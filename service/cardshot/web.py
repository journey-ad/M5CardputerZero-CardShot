"""远程 Web 管理：HTTP 服务与 REST API"""
import json
import mimetypes
import os
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import urlparse, parse_qs

STATIC_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "web")
mimetypes.add_type("video/mp4", ".mp4")
mimetypes.add_type("image/webp", ".webp")


class Handler(BaseHTTPRequestHandler):
    server_version = "CardShot/0.2"

    def log_message(self, fmt, *args):
        pass

    # ---- 响应工具 ----
    def _json(self, obj, code=200):
        body = json.dumps(obj, ensure_ascii=False).encode("utf-8")
        self.send_response(code)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(body)

    def _read_body(self):
        n = int(self.headers.get("Content-Length") or 0)
        if not n:
            return {}
        return json.loads(self.rfile.read(n).decode("utf-8") or "{}")

    # ---- GET ----
    def do_GET(self):
        svc = self.server.svc
        u = urlparse(self.path)
        q = parse_qs(u.query)
        try:
            if u.path in ("/", "/index.html"):
                return self._send_file(os.path.join(STATIC_DIR, "index.html"), "text/html; charset=utf-8")
            if u.path == "/favicon.ico":
                self.send_response(204)
                self.end_headers()
                return
            if u.path == "/api/status":
                return self._json({"ok": True, "status": svc.status()})
            if u.path == "/api/items":
                return self._json({"ok": True, "items": svc.library.items()})
            if u.path == "/api/config":
                return self._json({"ok": True, "config": svc.public_config()})
            if u.path == "/api/thumb":
                item_id = (q.get("id") or [""])[0]
                p = svc.library.ensure_thumb(item_id)
                if not p or not os.path.exists(p):
                    return self._json({"ok": False, "error": "无缩略图"}, 404)
                return self._send_file(p, "image/png")
            if u.path == "/api/file":
                item_id = (q.get("id") or [""])[0]
                kind, name, path = svc.library.resolve_id(item_id)
                if not os.path.exists(path):
                    return self._json({"ok": False, "error": "文件不存在"}, 404)
                dl = (q.get("download") or ["0"])[0] in ("1", "true")
                return self._send_file(path, None, download=name if dl else None)
            return self._json({"ok": False, "error": "未找到"}, 404)
        except Exception as e:
            return self._json({"ok": False, "error": str(e)}, 500)

    # ---- POST ----
    def do_POST(self):
        svc = self.server.svc
        u = urlparse(self.path)
        try:
            body = self._read_body()
            if u.path == "/api/shot":
                return self._json({"ok": True, "path": svc.capture.screenshot()})
            if u.path == "/api/record":
                action = body.get("action", "toggle")
                if action == "start":
                    return self._json({"ok": True, "path": svc.capture.start_record(),
                                       "status": svc.capture.status()})
                if action == "stop":
                    return self._json({"ok": True, "path": svc.capture.stop_record(),
                                       "status": svc.capture.status()})
                a, p = svc.capture.toggle_record()
                return self._json({"ok": True, "action": a, "path": p,
                                   "status": svc.capture.status()})
            if u.path == "/api/delete":
                return self._json({"ok": True, "result": svc.library.delete(body.get("ids") or [])})
            if u.path == "/api/config":
                return self._json({"ok": True, "config": svc.apply_config(body.get("patch") or body)})
            if u.path == "/api/remote":
                if "enabled" in body and not body["enabled"]:
                    return self._json({"ok": True, "remote": svc.remote_stop()})
                if "port" in body:
                    svc.cfg["remote"]["port"] = int(body["port"])
                return self._json({"ok": True, "remote": svc.remote_start()})
            return self._json({"ok": False, "error": "未找到"}, 404)
        except Exception as e:
            return self._json({"ok": False, "error": str(e)}, 500)

    # ---- 文件发送（支持 Range）----
    def _send_file(self, path, ctype, download=None):
        if not os.path.exists(path):
            return self._json({"ok": False, "error": "文件不存在"}, 404)
        size = os.path.getsize(path)
        ctype = ctype or mimetypes.guess_type(path)[0] or "application/octet-stream"
        start, end = 0, size - 1
        rng = self.headers.get("Range")
        partial = False
        if rng and rng.startswith("bytes="):
            try:
                a, b = rng[6:].split("-", 1)
                start = int(a) if a else 0
                end = int(b) if b else size - 1
                start = max(0, start)
                end = min(size - 1, end)
                partial = True
            except Exception:
                start, end, partial = 0, size - 1, False
        length = end - start + 1
        self.send_response(206 if partial else 200)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(length))
        self.send_header("Accept-Ranges", "bytes")
        if partial:
            self.send_header("Content-Range", "bytes %d-%d/%d" % (start, end, size))
        if download:
            self.send_header("Content-Disposition", 'attachment; filename="%s"' % download)
        self.end_headers()
        with open(path, "rb") as f:
            f.seek(start)
            remaining = length
            while remaining > 0:
                chunk = f.read(min(65536, remaining))
                if not chunk:
                    break
                self.wfile.write(chunk)
                remaining -= len(chunk)


class _Server(ThreadingHTTPServer):
    allow_reuse_address = True
    daemon_threads = True


class RemoteManager:
    """可运行时启停的 HTTP 服务"""

    def __init__(self, svc):
        self.svc = svc
        self._httpd = None
        self._thread = None
        self._lock = threading.Lock()

    def start(self, bind=None, port=None):
        with self._lock:
            bind = bind or self.svc.cfg["remote"].get("bind", "0.0.0.0")
            port = int(port or self.svc.cfg["remote"].get("port", 8420))
            if self._httpd is not None:
                return self.status()
            httpd = _Server((bind, port), Handler)
            httpd.svc = self.svc
            self._httpd = httpd
            self._thread = threading.Thread(target=httpd.serve_forever, daemon=True)
            self._thread.start()
            self.svc.cfg["remote"]["enabled"] = True
            self.svc.cfg["remote"]["port"] = port
            return self.status()

    def stop(self):
        with self._lock:
            if self._httpd is None:
                return self.status()
            self._httpd.shutdown()
            self._httpd.server_close()
            self._httpd = None
            self._thread = None
            self.svc.cfg["remote"]["enabled"] = False
            return self.status()

    def status(self):
        running = self._httpd is not None
        port = self.svc.cfg["remote"].get("port", 8420)
        return {"running": running, "port": port,
                "url": "http://%s:%d/" % (_lan_ip(), port) if running else None}


def _lan_ip():
    try:
        import socket
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        s.connect(("8.8.8.8", 80))
        ip = s.getsockname()[0]
        s.close()
        return ip
    except Exception:
        return "127.0.0.1"
