#!/usr/bin/env bash
# 在 arm64 Debian 容器内构建 Rust 产物（cardshotd / cardshot-ui）
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
docker run --rm --platform linux/arm64 \
    -v "$ROOT/rust:/work" \
    -v cardshot-cargo:/usr/local/cargo \
    -w /work \
    rust:1-trixie \
    bash -c 'export PATH=/usr/local/cargo/bin:$PATH; cargo build --release --locked'
