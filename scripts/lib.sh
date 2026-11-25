#!/bin/bash
# 公共 SSH 辅助：读取 .secrets/device.env，用 askpass 方式免交互
set -euo pipefail
ROOT=$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
ENV_FILE=${DEVICE_ENV_FILE:-$ROOT/.secrets/device.env}
if [ -f "$ENV_FILE" ]; then
    set -a
    # shellcheck disable=SC1090
    source "$ENV_FILE"
    set +a
fi
: "${DEVICE_IP:?请配置 DEVICE_IP}"
DEVICE_USER=${DEVICE_USER:?缺少 DEVICE_USER}
HOST="${DEVICE_USER}@${DEVICE_IP}"
export DEVICE_PASS
ASKPASS="$ROOT/scripts/askpass.sh"
export SSH_ASKPASS="$ASKPASS" SSH_ASKPASS_REQUIRE=force

ssh_run() { ssh -o StrictHostKeyChecking=accept-new "$HOST" "$@"; }
scp_to()  { scp -o StrictHostKeyChecking=accept-new "$1" "$HOST:$2" </dev/null; }
scp_from(){ scp -o StrictHostKeyChecking=accept-new "$HOST:$1" "$2" </dev/null; }
scp_r()   { scp -r -o StrictHostKeyChecking=accept-new "$1" "$HOST:$2" </dev/null; }
