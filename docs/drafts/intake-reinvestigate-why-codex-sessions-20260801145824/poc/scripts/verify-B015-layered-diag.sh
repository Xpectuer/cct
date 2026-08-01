#!/usr/bin/env bash
# PoC B015: 分层诊断第一步 — proxy 层存活（curl --noproxy '*' 直连本地端口）
# Source: spec.md Acceptance Criterion #15
# Target: network
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
[ -f "$SCRIPT_DIR/../config.env" ] && source "$SCRIPT_DIR/../config.env"

command -v curl >/dev/null 2>&1 || { echo "[SKIP] curl not installed"; exit 77; }
command -v nc >/dev/null 2>&1 || { echo "[SKIP] nc not installed"; exit 77; }
PROXY_PORT="${PROXY_PORT:-19191}"

echo "[PoC B015] proxy 层存活诊断（无上游时 502/404 亦为存活证据）"

if ! nc -z -w 2 127.0.0.1 "$PROXY_PORT" 2>/dev/null; then
  echo "[SKIP] B015: 端口 $PROXY_PORT 无监听 — 先启动 proxy（cct proxy start 或 cct run）"
  exit 77
fi

HTTP=$(curl -s --noproxy '*' --max-time 3 -o /dev/null -w "%{http_code}" \
  "http://127.0.0.1:$PROXY_PORT/v1/models" 2>&1) && RC=0 || RC=$?

if [ "$RC" = 0 ]; then
  echo "[PASS] B015: proxy 层存活 (HTTP $HTTP) — 可进入上游层诊断"
  exit 0
elif [ "$RC" = 28 ]; then
  echo "[FAIL] B015: curl 3s 超时 — proxy 挂起（死锁未修复）"
  exit 1
else
  echo "[FAIL] B015: proxy 层不可达 (curl exit $RC)"
  exit 1
fi
