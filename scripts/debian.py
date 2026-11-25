"""在本机或 Docker 中调用 Debian 打包工具"""

import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parent.parent


def dpkg_deb(*args, text=True):
    command = ["dpkg-deb", *map(str, args)]
    if not shutil.which("dpkg-deb"):
        if not shutil.which("docker"):
            raise SystemExit("请安装 dpkg 或启动 Docker 后重试")
        command = [
            "docker", "run", "--rm",
            "--user", f"{os.getuid()}:{os.getgid()}",
            "--volume", f"{ROOT}:{ROOT}", "--workdir", str(ROOT),
            "ubuntu:22.04", *command,
        ]
    return subprocess.check_output(command, cwd=ROOT, text=text)
