#!/bin/bash
# 部署 CardShot 服务到设备：上传程序包/配置/服务单元，注册并重启 systemd user 服务
set -euo pipefail
ROOT=$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
. "$ROOT/scripts/lib.sh"

echo "==> 创建远端目录"
ssh_run 'mkdir -p ~/.local/share/cardshot ~/.config/cardshot ~/.config/systemd/user ~/Pictures/CardShot ~/Videos/CardShot ~/.cache/cardshot/thumbs'

echo "==> 上传程序包与 Web 资源"
ssh_run 'rm -rf ~/.local/share/cardshot/cardshot ~/.local/share/cardshot/web'
scp_r "$ROOT/service/cardshot" ".local/share/cardshot/"
scp_r "$ROOT/service/web" ".local/share/cardshot/"
scp_to "$ROOT/scripts/ctl_client.py" ".local/share/cardshot/ctl_client.py"

echo "==> 上传配置（已存在则保留）"
if ssh_run 'test -f ~/.config/cardshot/config.json'; then
    echo "配置已存在，保留远端版本"
else
    scp_to "$ROOT/service/config.example.json" ".config/cardshot/config.json"
fi

echo "==> 安装服务单元"
scp_to "$ROOT/service/cardshot.service" ".config/systemd/user/cardshot.service"

echo "==> 重载并重启服务"
ssh_run 'export XDG_RUNTIME_DIR=/run/user/$(id -u); systemctl --user daemon-reload; systemctl --user enable cardshot.service >/dev/null 2>&1; systemctl --user restart cardshot.service; sleep 1.5; systemctl --user --no-pager status cardshot.service | head -12'
