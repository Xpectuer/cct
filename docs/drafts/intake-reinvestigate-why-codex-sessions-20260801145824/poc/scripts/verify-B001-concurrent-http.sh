#!/usr/bin/env bash
# PoC B001: 并发控制命令 + HTTP 请求时 proxy 不挂起（死锁回归）
# Source: spec.md Acceptance Criterion #1
# Target: api
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
[ -f "$SCRIPT_DIR/../config.env" ] && source "$SCRIPT_DIR/../config.env"

command -v curl >/dev/null 2>&1 || { echo "[SKIP] curl not installed"; exit 77; }
command -v nc >/dev/null 2>&1 || { echo "[SKIP] nc not installed"; exit 77; }
[ -x "${CCT_BIN:?Must be set in config.env}" ] || { echo "[SKIP] cct binary not built: $CCT_BIN"; exit 77; }

echo "[PoC B001] 并发控制命令 + HTTP 请求不挂起（死锁回归）"

TMP=$(mktemp -d)
SOCK="${CCT_PROXY_SOCKET:-$TMP/proxy.sock}"
export CCT_PROXY_SOCKET="$SOCK"
export CCT_PROXY_PORT="${PROXY_PORT:-19191}"
cleanup() { [ -n "${PROXY_PID:-}" ] && kill "$PROXY_PID" 2>/dev/null || true; rm -f "$SOCK"; rm -rf "$TMP"; }
trap cleanup EXIT

"$CCT_BIN" proxy start >/dev/null 2>&1 & PROXY_PID=$!
for _ in $(seq 1 50); do
  [ -S "$SOCK" ] && break
  sleep 0.1
done
[ -S "$SOCK" ] || { echo "[FAIL] B001: proxy socket 未出现 — 启动失败"; exit 1; }

# 并发: 5 个 status 控制命令 + 1 个 HTTP 请求
NCS=()
for _ in $(seq 1 5); do
  printf '{"cmd":"status"}\n' | nc -U -w 2 "$SOCK" >/dev/null 2>&1 & NCS+=($!)
done
HTTP=$(curl -s --noproxy '*' --max-time 3 -o /dev/null -w "%{http_code}" \
  "http://127.0.0.1:$CCT_PROXY_PORT/v1/models" 2>&1) && RC=0 || RC=$?
# 只等并发任务 PID — 无参 wait 会连带等待长驻 proxy daemon 导致挂起
wait "${NCS[@]}" 2>/dev/null || true

if [ "$RC" = 0 ]; then
  echo "[PASS] B001: HTTP 在 3s 内得到响应 (HTTP $HTTP)"
  exit 0
else
  echo "[FAIL] B001: HTTP 请求超时挂起 (curl exit $RC) — 死锁未修复"
  exit 1
fi
