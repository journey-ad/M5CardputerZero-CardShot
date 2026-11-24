#!/bin/sh
case "${1:-}" in
    *[Pp]assword*) printf '%s\n' "${DEVICE_PASS:?缺少 DEVICE_PASS}" ;;
    *) exit 1 ;;
esac
