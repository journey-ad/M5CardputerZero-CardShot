#!/bin/bash
# 安装 CardShot 到 APPLaunch 并注册桌面入口
set -euo pipefail
ROOT=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
. "$ROOT/scripts/lib.sh"
UI_BIN="$ROOT/crates/target/release/cardshot-ui"
[ -f "$UI_BIN" ] || { echo "缺少构建产物: $UI_BIN"; exit 1; }

REMOTE=/tmp/cardshot-pack
ssh_run "rm -rf $REMOTE; mkdir -p $REMOTE"
scp_to "$UI_BIN" "$REMOTE/cardshot-ui"
scp_r "$ROOT/packaging" "$REMOTE/"

ssh_run "printf '%s\n' \"\$DEVICE_SUDO_PASS\" >/dev/null" 2>/dev/null || true
# 通过 sudo 安装到 APPLaunch 目录
SCP_PASS="$DEVICE_SUDO_PASS"
ssh_run "echo '$SCP_PASS' | sudo -S bash -c '
set -e
install -d /usr/share/APPLaunch/bin /usr/share/APPLaunch/applications /usr/share/APPLaunch/share/images
install -m 0755 $REMOTE/cardshot-ui /usr/share/APPLaunch/bin/cardshot-ui
install -m 0755 $REMOTE/packaging/bin/run-cardshot /usr/share/APPLaunch/bin/run-cardshot
install -m 0644 $REMOTE/packaging/applications/cardshot.desktop /usr/share/APPLaunch/applications/cardshot.desktop
install -m 0644 $REMOTE/packaging/share/images/cardshot_100.png /usr/share/APPLaunch/share/images/cardshot_100.png
install -m 0644 $REMOTE/packaging/share/images/cardshot_256.png /usr/share/APPLaunch/share/images/cardshot_256.png
'"
# 重启 APPLaunch 使用户服务加载新条目
ssh_run 'export XDG_RUNTIME_DIR=/run/user/$(id -u); systemctl --user restart APPLaunch.service 2>/dev/null || true'
echo "已安装 CardShot 到 APPLaunch"
