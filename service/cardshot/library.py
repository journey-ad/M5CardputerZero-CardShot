"""相册库：扫描截图/录屏、生成缩略图、删除条目"""
import hashlib
import json
import os
import subprocess

from PIL import Image

IMG_EXT = (".png", ".jpg", ".jpeg", ".webp")
VID_EXT = (".mp4", ".webm", ".mkv")
THUMB_W = 240


class Library:
    def __init__(self, cfg):
        self.cfg = cfg
        self.img_dir = cfg["output_dir"]
        self.vid_dir = cfg["video_dir"]
        self.thumb_dir = cfg["thumb_dir"]
        os.makedirs(self.thumb_dir, exist_ok=True)
        self._dur_cache = {}

    # ---- 路径映射 ----
    def path_for(self, kind, name):
        base = self.img_dir if kind == "image" else self.vid_dir
        if os.sep in name or (os.altsep and os.altsep in name) or name in (".", ".."):
            raise ValueError("非法文件名")
        return os.path.join(base, name)

    def resolve_id(self, item_id):
        if ":" not in item_id:
            raise ValueError("非法 id")
        kind, name = item_id.split(":", 1)
        if kind not in ("image", "video"):
            raise ValueError("非法类型")
        return kind, name, self.path_for(kind, name)

    # ---- 扫描 ----
    def _scan(self, kind, base, exts):
        out = []
        if not os.path.isdir(base):
            return out
        for name in os.listdir(base):
            if not name.lower().endswith(exts):
                continue
            p = os.path.join(base, name)
            if not os.path.isfile(p):
                continue
            st = os.stat(p)
            out.append({
                "id": "%s:%s" % (kind, name),
                "kind": kind,
                "name": name,
                "size": st.st_size,
                "mtime": int(st.st_mtime),
            })
        return out

    def items(self):
        rows = self._scan("image", self.img_dir, IMG_EXT)
        rows += self._scan("video", self.vid_dir, VID_EXT)
        rows.sort(key=lambda r: r["mtime"], reverse=True)
        for r in rows:
            if r["kind"] == "video":
                r["duration"] = self._duration(r["id"])
        return rows

    def counts(self):
        return {
            "images": len(self._scan("image", self.img_dir, IMG_EXT)),
            "videos": len(self._scan("video", self.vid_dir, VID_EXT)),
        }

    def total_bytes(self):
        total = 0
        for r in self._scan("image", self.img_dir, IMG_EXT) + self._scan("video", self.vid_dir, VID_EXT):
            total += r["size"]
        return total

    def _duration(self, item_id):
        if item_id in self._dur_cache:
            return self._dur_cache[item_id]
        try:
            kind, name, path = self.resolve_id(item_id)
            out = subprocess.run(
                ["ffprobe", "-v", "error", "-show_entries", "format=duration",
                 "-of", "default=nw=1:nk=1", path],
                capture_output=True, text=True, timeout=5,
            )
            dur = round(float(out.stdout.strip()), 1)
        except Exception:
            dur = None
        self._dur_cache[item_id] = dur
        return dur

    # ---- 缩略图 ----
    def thumb_path(self, item_id):
        kind, name, src = self.resolve_id(item_id)
        try:
            st = os.stat(src)
            key = "%s:%d:%d" % (item_id, int(st.st_mtime), st.st_size)
        except OSError:
            key = item_id
        digest = hashlib.md5(key.encode("utf-8")).hexdigest()
        return os.path.join(self.thumb_dir, digest + ".png")

    def ensure_thumb(self, item_id):
        out = self.thumb_path(item_id)
        if os.path.exists(out):
            return out
        kind, name, src = self.resolve_id(item_id)
        if kind == "image":
            img = Image.open(src).convert("RGB")
            ratio = THUMB_W / img.width
            img = img.resize((THUMB_W, max(1, int(img.height * ratio))))
            img.save(out, "PNG")
        else:
            # 视频取第 1 秒一帧
            tmp = out + ".tmp.png"
            subprocess.run(
                ["ffmpeg", "-hide_banner", "-loglevel", "error", "-y",
                 "-ss", "1", "-i", src, "-frames:v", "1",
                 "-vf", "scale=%d:-1" % THUMB_W, tmp],
                capture_output=True, timeout=15,
            )
            if not os.path.exists(tmp):
                # 极短视频回退到 0 秒
                subprocess.run(
                    ["ffmpeg", "-hide_banner", "-loglevel", "error", "-y",
                     "-i", src, "-frames:v", "1",
                     "-vf", "scale=%d:-1" % THUMB_W, tmp],
                    capture_output=True, timeout=15,
                )
            if os.path.exists(tmp):
                os.replace(tmp, out)
            else:
                return None
        return out

    # ---- 删除 ----
    def delete(self, item_ids):
        removed = 0
        errors = []
        for item_id in item_ids:
            try:
                kind, name, path = self.resolve_id(item_id)
                thumb = self.thumb_path(item_id)
                if os.path.exists(path):
                    os.remove(path)
                    removed += 1
                if os.path.exists(thumb):
                    os.remove(thumb)
            except Exception as e:
                errors.append({"id": item_id, "error": str(e)})
        self._dur_cache.clear()
        return {"removed": removed, "errors": errors}
