"""配置读写"""
import copy
import json
import os

DEFAULT_CONFIG = {
    "output_dir": "~/Pictures/CardShot",
    "video_dir": "~/Videos/CardShot",
    "thumb_dir": "~/.cache/cardshot/thumbs",
    "image_format": "png",          # png | jpeg | webp
    "image_quality": 90,             # 1-100，对 jpeg/webp 生效
    "record": {
        "fps": 20,
        "bitrate": "2M",
        "encoder": "h264_v4l2m2m",   # 回退 libx264
        "audio": False,
    },
    "hotkeys": {
        "enabled": False,
        "shot": ["shift+space"],
        "record": ["shift+enter"],
    },
    "feedback": {"led": True},
    "remote": {
        "enabled": False,
        "bind": "0.0.0.0",
        "port": 8420,
    },
    "socket_name": "cardshot.sock",
}


def _expand(p):
    return os.path.expanduser(p)


def _deep_merge(base, patch):
    for k, v in patch.items():
        if isinstance(v, dict) and isinstance(base.get(k), dict):
            _deep_merge(base[k], v)
        else:
            base[k] = v
    return base


def load(path):
    """读配置文件并与默认值合并，返回 dict"""
    cfg = copy.deepcopy(DEFAULT_CONFIG)
    if path and os.path.exists(path):
        with open(path, "r", encoding="utf-8") as f:
            _deep_merge(cfg, json.load(f))
    for key in ("output_dir", "video_dir", "thumb_dir"):
        cfg[key] = _expand(cfg[key])
    cfg["_path"] = path
    return cfg


def save(cfg):
    """把可持久化字段写回配置文件"""
    path = cfg.get("_path")
    if not path:
        return False
    data = {k: v for k, v in cfg.items() if not k.startswith("_")}
    tmp = path + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
    os.replace(tmp, path)
    return True
