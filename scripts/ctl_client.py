#!/usr/bin/env python3
"""通过本地 Unix socket 向 CardShot 服务发送单条命令并打印 JSON 响应"""
import json
import os
import socket
import sys

cmd = sys.argv[1] if len(sys.argv) > 1 else "status"
rest = sys.argv[2:]
runtime = os.environ.get("XDG_RUNTIME_DIR") or "/run/user/%d" % os.getuid()
req = {"cmd": cmd}
if rest:
    if rest[0].lstrip().startswith("{"):
        req.update(json.loads(rest[0]))
    elif cmd == "delete":
        req["ids"] = rest
    else:
        req["path"] = rest[0]
sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
sock.connect(os.path.join(runtime, "cardshot.sock"))
sock.sendall((json.dumps(req) + "\n").encode("utf-8"))
print(sock.recv(1 << 20).decode("utf-8").strip())
