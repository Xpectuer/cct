#!/usr/bin/env bash
# PoC B015: 分层诊断第一步 — proxy 层存活（curl --noproxy '*' 直连本地端口）
# Source: spec.md Acceptance Criterion #15
# Target: network
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
[ -f "$SCRIPT_DIR/../config.env" ] && source "$SCRIPT_DIR/../config.env"

command -v curl >/dev/null 2>&1 || { echo "[SKIP] curl not installed"; exit 77; }
command -v nc >/dev/null 2>&1 || { echo "[SKIP] nc not installed"; exit 77; }
[ -x "${CCT_BIN:?Must be set in config.env}" ] || { echo "[SKIP] cct binary not built: $CCT_BIN"; exit 77; }

echo "[PoC B015] proxy 层存活诊断（无上游时 502/404 亦为存活证据）"

TMP=$(mktemp -d)
SOCK="${CCT_PROXY_SOCKET:-$TMP/proxy.sock}"
export CCT_PROXY_SOCKET="$SOCK" CCT_PROXY_PORT="${PROXY_PORT:-19191}"
cleanup() { [ -n "${PROXY_PID:-}" ] && kill "$PROXY_PID" 2>/dev/null || true; rm -f "$SOCK"; rm -rf "$TMP"; }
trap cleanup EXIT

# 自起 proxy 实例（gate 下前置脚本均已清理 daemon, 无监听可测）:
# 分层诊断第一步仍为"proxy 层存活"——对自起实例同样成立
"$CCT_BIN" proxy start >/dev/null 2>&1 & PROXY_PID=$!
for _ in $(seq 1 50); do
  [ -S "$SOCK" ] && break
  sleep 0.1
done
if ! nc -z -w 2 127.0.0.1 "$CCT_PROXY_PORT" 2>/dev/null; then
  echo "[FAIL] B015: 端口 $CCT_PROXY_PORT 无监听 — proxy 未启动"
  exit 1
fi

HTTP=$(curl -s --noproxy '*' --max-time 3 -o /dev/null -w "%{http_code}" \
  "http://127.0.0.1:$CCT_PROXY_PORT/v1/models" 2>&1) && RC=0 || RC=$?

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
