#!/bin/bash
# 通过设备本地 Unix socket 向 CardShot 服务发命令
# 用法: ctl.sh status|shot|record_start|record_stop|record_toggle|list [path]
set -euo pipefail
ROOT=$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
. "$ROOT/scripts/lib.sh"
CMD=${1:-status}
shift || true
ssh_run "python3 ~/.local/share/cardshot/ctl_client.py $CMD $*"
