#!/bin/bash
# 构建产物部署到设备：二进制 + web + systemd unit
set -euo pipefail
ROOT=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
. "$ROOT/scripts/lib.sh"
BIN="$ROOT/crates/target/release/cardshotd"
UI_BIN="$ROOT/crates/target/release/cardshot-ui"
[ -f "$BIN" ] || { echo "缺少构建产物: $BIN"; exit 1; }
[ -f "$UI_BIN" ] || { echo "缺少构建产物: $UI_BIN"; exit 1; }

ssh_run 'systemctl --user stop cardshot.service 2>/dev/null || true'
ssh_run 'mkdir -p ~/.local/share/cardshot ~/.config/cardshot ~/.config/systemd/user'
scp_to "$BIN" '.local/share/cardshot/cardshotd'
ssh_run 'pkill -x cardshot-ui 2>/dev/null || true'
scp_to "$UI_BIN" '.local/share/cardshot/cardshot-ui'
scp_r "$ROOT/service/web" '.local/share/cardshot/'
scp_to "$ROOT/service/cardshot.service" '.config/systemd/user/cardshot.service'

ssh_run 'systemctl --user stop cardshot.service 2>/dev/null || true; systemctl --user daemon-reload; systemctl --user enable cardshot.service >/dev/null 2>&1; systemctl --user restart cardshot.service; sleep 1.5; systemctl --user --no-pager --lines=0 status cardshot.service | head -n 10'
