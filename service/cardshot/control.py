"""Unix socket 控制服务：本机应用与 CLI 的接口"""
import json
import os
import socket
import threading
import traceback


class ControlServer(threading.Thread):
    def __init__(self, svc, sock_path):
        super().__init__(daemon=True)
        self.svc = svc
        self.sock_path = sock_path

    def run(self):
        if os.path.exists(self.sock_path):
            try:
                os.unlink(self.sock_path)
            except OSError:
                pass
        srv = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        srv.bind(self.sock_path)
        os.chmod(self.sock_path, 0o660)
        srv.listen(8)
        while True:
            try:
                conn, _ = srv.accept()
            except OSError:
                break
            threading.Thread(target=self._process_conn, args=(conn,), daemon=True).start()

    def _process_conn(self, conn):
        try:
            data = b""
            while b"\n" not in data and len(data) < 1 << 20:
                chunk = conn.recv(65536)
                if not chunk:
                    break
                data += chunk
            req = json.loads(data.decode("utf-8").strip() or "{}")
            resp = self.dispatch(req)
        except Exception as e:
            resp = {"ok": False, "error": str(e), "trace": traceback.format_exc()[-400:]}
        try:
            conn.sendall((json.dumps(resp, ensure_ascii=False) + "\n").encode("utf-8"))
        finally:
            conn.close()

    def dispatch(self, req):
        svc = self.svc
        cmd = (req.get("cmd") or "status").lower()
        if cmd == "status":
            return {"ok": True, "status": svc.status()}
        if cmd == "shot":
            return {"ok": True, "path": svc.capture.screenshot()}
        if cmd == "record_start":
            p = svc.capture.start_record()
            return {"ok": p is not None, "path": p, "recording": True}
        if cmd == "record_stop":
            return {"ok": True, "path": svc.capture.stop_record(), "recording": False}
        if cmd == "record_toggle":
            action, path = svc.capture.toggle_record()
            return {"ok": True, "action": action, "path": path,
                    "recording": svc.capture.is_recording()}
        if cmd in ("list", "items"):
            return {"ok": True, "items": svc.library.items()}
        if cmd == "counts":
            return {"ok": True, "counts": svc.library.counts(),
                    "total_bytes": svc.library.total_bytes()}
        if cmd == "delete":
            ids = req.get("ids")
            if ids is None and req.get("id"):
                ids = [req["id"]]
            if not ids and req.get("path"):
                ids = [req["path"]]
            return {"ok": True, "result": svc.library.delete(ids or [])}
        if cmd == "config_get":
            return {"ok": True, "config": svc.public_config()}
        if cmd == "config_set":
            return {"ok": True, "config": svc.apply_config(req.get("patch") or {})}
        if cmd == "remote_start":
            return {"ok": True, "remote": svc.remote_start()}
        if cmd == "remote_stop":
            return {"ok": True, "remote": svc.remote_stop()}
        if cmd == "remote_status":
            return {"ok": True, "remote": svc.remote_status()}
        if cmd == "reload":
            svc.reload_config()
            return {"ok": True}
        return {"ok": False, "error": "未知命令: %s" % cmd}
