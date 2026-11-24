"""帧缓冲抓取与截图/录屏"""
import os
import subprocess
import threading
import time

from PIL import Image

FB = "/dev/fb0"
WIDTH, HEIGHT = 320, 170
FRAME_BYTES = WIDTH * HEIGHT * 2

FMT_EXT = {"png": "png", "jpeg": "jpg", "jpg": "jpg", "webp": "webp"}


def read_frame():
    """读取一整帧 RGB565 原始字节"""
    with open(FB, "rb") as f:
        return f.read(FRAME_BYTES)


def frame_to_image(raw):
    """RGB565LE 原始字节转 Pillow RGB 图像"""
    return Image.frombytes("RGB", (WIDTH, HEIGHT), raw, "raw", "BGR;16")


def timestamp():
    return time.strftime("%Y%m%d_%H%M%S")


def save_image(img, path, fmt, quality):
    fmt = (fmt or "png").lower()
    if fmt == "png":
        img.save(path, "PNG", optimize=True)
    elif fmt in ("jpeg", "jpg"):
        img.save(path, "JPEG", quality=int(quality), optimize=True)
    elif fmt == "webp":
        img.save(path, "WEBP", quality=int(quality))
    else:
        img.save(path, "PNG", optimize=True)


class Capture:
    """截图与录屏状态机，线程安全地管理 ffmpeg 子进程"""

    def __init__(self, cfg, feedback=None):
        self.cfg = cfg
        self.feedback = feedback
        self._lock = threading.Lock()
        self._proc = None
        self._drain = None
        self._start_ts = 0.0
        self._frames = 0
        self._cur_file = None
        self._last_err = None
        os.makedirs(cfg["output_dir"], exist_ok=True)
        os.makedirs(cfg["video_dir"], exist_ok=True)

    # ---- 截图 ----
    def screenshot(self):
        raw = read_frame()
        img = frame_to_image(raw)
        fmt = (self.cfg.get("image_format") or "png").lower()
        ext = FMT_EXT.get(fmt, "png")
        name = "shot_%s.%s" % (timestamp(), ext)
        path = os.path.join(self.cfg["output_dir"], name)
        save_image(img, path, fmt, self.cfg.get("image_quality", 90))
        if self.feedback:
            self.feedback.blink()
        return path

    # ---- 录屏 ----
    def _cmd(self, path):
        rc = self.cfg.get("record", {})
        fps = str(rc.get("fps", 20))
        bitrate = rc.get("bitrate", "2M")
        encoder = rc.get("encoder", "h264_v4l2m2m")
        cmd = [
            "ffmpeg", "-hide_banner", "-loglevel", "error", "-y",
            "-f", "rawvideo", "-pix_fmt", "bgr565le",
            "-s", "%dx%d" % (WIDTH, HEIGHT), "-r", fps, "-i", "pipe:0",
            "-c:v", encoder,
        ]
        if encoder == "h264_v4l2m2m":
            cmd += ["-b:v", bitrate, "-pix_fmt", "yuv420p"]
        elif encoder == "libx264":
            cmd += ["-preset", "ultrafast", "-b:v", bitrate, "-pix_fmt", "yuv420p"]
        else:
            cmd += ["-b:v", bitrate]
        cmd += ["-movflags", "+faststart", path]
        return cmd

    def _drain_stderr(self, proc):
        try:
            for line in proc.stderr:
                text = line.decode("utf-8", "replace").strip()
                if text:
                    self._last_err = text[-200:]
        except Exception:
            pass

    def _feeder(self, proc, fps):
        period = 1.0 / max(1, int(fps))
        try:
            while True:
                t0 = time.time()
                proc.stdin.write(read_frame())
                self._frames += 1
                sleep = period - (time.time() - t0)
                if sleep > 0:
                    time.sleep(sleep)
        except (BrokenPipeError, ValueError, OSError):
            pass

    def start_record(self):
        with self._lock:
            if self._proc is not None:
                return None
            fps = int(self.cfg.get("record", {}).get("fps", 20))
            name = "rec_%s.mp4" % timestamp()
            path = os.path.join(self.cfg["video_dir"], name)
            proc = subprocess.Popen(
                self._cmd(path), stdin=subprocess.PIPE,
                stdout=subprocess.DEVNULL, stderr=subprocess.PIPE,
            )
            self._proc = proc
            self._cur_file = path
            self._start_ts = time.time()
            self._frames = 0
            self._last_err = None
            self._drain = threading.Thread(target=self._drain_stderr, args=(proc,), daemon=True)
            self._drain.start()
            threading.Thread(target=self._feeder, args=(proc, fps), daemon=True).start()
            if self.feedback:
                self.feedback.blink(times=2)
            return path

    def stop_record(self):
        with self._lock:
            if self._proc is None:
                return None
            proc = self._proc
            path = self._cur_file
            try:
                proc.stdin.close()
            except Exception:
                pass
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait(timeout=3)
            self._proc = None
            self._cur_file = None
            self._start_ts = 0.0
            if self.feedback:
                self.feedback.blink(times=2)
            return path

    def toggle_record(self):
        if self.is_recording():
            return ("stop", self.stop_record())
        return ("start", self.start_record())

    def is_recording(self):
        return self._proc is not None

    def status(self):
        rec = self.is_recording()
        return {
            "recording": rec,
            "elapsed": round(time.time() - self._start_ts, 1) if rec else 0,
            "frames": self._frames if rec else 0,
            "current_file": self._cur_file,
            "last_error": self._last_err,
        }
