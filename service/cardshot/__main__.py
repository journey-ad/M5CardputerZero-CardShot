"""CardShot 守护进程入口"""
import argparse
import fcntl
import os
import signal
import sys
import time

from . import config as config_mod
from .capture import Capture
from .control import ControlServer
from .feedback import Feedback
from .library import Library
from .web import RemoteManager


class Service:
    def __init__(self, cfg_path):
        self.cfg_path = cfg_path
        self.cfg = config_mod.load(cfg_path)
        self.feedback = Feedback(self.cfg.get("feedback", {}).get("led", True))
        self.capture = Capture(self.cfg, self.feedback)
        self.library = Library(self.cfg)
        self.remote = RemoteManager(self)

    # ---- 状态 ----
    def status(self):
        st = self.capture.status()
        st.update({
            "counts": self.library.counts(),
            "total_bytes": self.library.total_bytes(),
            "remote": self.remote.status(),
            "config": self.public_config(),
        })
        return st

    def public_config(self):
        return {
            "image_format": self.cfg.get("image_format", "png"),
            "image_quality": self.cfg.get("image_quality", 90),
            "record": dict(self.cfg.get("record", {})),
            "hotkeys": dict(self.cfg.get("hotkeys", {})),
            "remote": self.remote.status(),
            "output_dir": self.cfg["output_dir"],
            "video_dir": self.cfg["video_dir"],
        }

    # ---- 远程服务控制 ----
    def remote_start(self):
        return self.remote.start()

    def remote_stop(self):
        return self.remote.stop()

    def remote_status(self):
        return self.remote.status()

    # ---- 配置变更 ----
    def apply_config(self, patch):
        if not isinstance(patch, dict):
            return self.public_config()
        known = ("image_format", "image_quality", "record", "hotkeys", "feedback")
        for key in known:
            if key in patch:
                if isinstance(patch[key], dict) and isinstance(self.cfg.get(key), dict):
                    self.cfg[key].update(patch[key])
                else:
                    self.cfg[key] = patch[key]
        if "remote" in patch and isinstance(patch["remote"], dict):
            r = patch["remote"]
            if "port" in r:
                self.cfg["remote"]["port"] = int(r["port"])
            if "enabled" in r:
                if r["enabled"]:
                    self.remote.start()
                else:
                    self.remote.stop()
        config_mod.save(self.cfg)
        return self.public_config()

    def reload_config(self):
        self.cfg = config_mod.load(self.cfg_path)
        self.capture.cfg = self.cfg
        self.library.img_dir = self.cfg["output_dir"]
        self.library.vid_dir = self.cfg["video_dir"]
        os.makedirs(self.cfg["output_dir"], exist_ok=True)
        os.makedirs(self.cfg["video_dir"], exist_ok=True)

    # ---- 生命周期 ----
    def start_background(self):
        socket_path = os.path.join(
            os.environ.get("XDG_RUNTIME_DIR") or "/run/user/%d" % os.getuid(),
            self.cfg.get("socket_name", "cardshot.sock"),
        )
        self.control = ControlServer(self, socket_path)
        self.control.start()
        if self.cfg["remote"].get("enabled") and self.cfg["remote"].get("autostart", True):
            try:
                self.remote.start()
            except Exception as e:
                print("远程服务启动失败: %s" % e, file=sys.stderr)
        return socket_path


def acquire_lock(path):
    f = open(path, "w")
    try:
        fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except OSError:
        return None
    f.write(str(os.getpid()))
    f.flush()
    return f


def main():
    ap = argparse.ArgumentParser(description="CardShot 后台截图/录屏服务")
    ap.add_argument("--config", default=os.path.expanduser("~/.config/cardshot/config.json"))
    args = ap.parse_args()

    os.makedirs(os.path.dirname(args.config), exist_ok=True)
    lock_path = os.path.join(
        os.environ.get("XDG_RUNTIME_DIR") or "/tmp",
        "cardshot.lock",
    )
    lock = acquire_lock(lock_path)
    if lock is None:
        print("已有 CardShot 实例在运行", file=sys.stderr)
        return 1

    svc = Service(args.config)
    sock = svc.start_background()
    print("CardShot 已启动 socket=%s remote=%s" % (sock, svc.remote.status()), flush=True)

    stop = {"flag": False}

    def _sig(_s, _f):
        stop["flag"] = True

    signal.signal(signal.SIGTERM, _sig)
    signal.signal(signal.SIGINT, _sig)
    while not stop["flag"]:
        time.sleep(0.5)
    if svc.capture.is_recording():
        svc.capture.stop_record()
    svc.remote.stop()
    return 0


if __name__ == "__main__":
    sys.exit(main())
