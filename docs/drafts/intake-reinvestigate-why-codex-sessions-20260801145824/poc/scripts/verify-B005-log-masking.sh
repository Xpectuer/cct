#!/usr/bin/env bash
# PoC B005: CCT_PROXY_LOG 开启时, 控制命令与 HTTP 请求日志不含 api_key 明文
# Source: spec.md Acceptance Criterion #5
# Target: filesystem
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
[ -f "$SCRIPT_DIR/../config.env" ] && source "$SCRIPT_DIR/../config.env"

command -v curl >/dev/null 2>&1 || { echo "[SKIP] curl not installed"; exit 77; }
command -v nc >/dev/null 2>&1 || { echo "[SKIP] nc not installed"; exit 77; }
[ -x "${CCT_BIN:?Must be set in config.env}" ] || { echo "[SKIP] cct binary not built: $CCT_BIN"; exit 77; }
[ -n "${TEST_API_KEY:?Must be set in config.env}" ] || { echo "[SKIP] TEST_API_KEY 为空"; exit 77; }

echo "[PoC B005] 日志打印不含 api_key 明文（脱敏）"

TMP=$(mktemp -d)
SOCK="${CCT_PROXY_SOCKET:-$TMP/proxy.sock}"
LOG="$TMP/proxy.log"
export CCT_PROXY_SOCKET="$SOCK" CCT_PROXY_PORT="${PROXY_PORT:-19191}" CCT_PROXY_LOG=1
cleanup() { [ -n "${PROXY_PID:-}" ] && kill "$PROXY_PID" 2>/dev/null || true; rm -f "$SOCK"; rm -rf "$TMP"; }
trap cleanup EXIT

# CCT_PROXY_LOG 是开关: 置位后 log_proxy! 写 stderr — 捕获到 $LOG 供脱敏断言
"$CCT_BIN" proxy start >"$LOG" 2>&1 & PROXY_PID=$!
for _ in $(seq 1 50); do
  [ -S "$SOCK" ] && break
  sleep 0.1
done
[ -S "$SOCK" ] || { echo "[FAIL] B005: proxy socket 未出现"; exit 1; }

# 控制命令（switch 含 api_key）+ HTTP 请求, 触发日志
printf '{"cmd":"switch","base_url":"http://127.0.0.1:9/v1","api_key":"%s","model":"gpt-4.1"}\n' \
  "$TEST_API_KEY" | nc -U -w 2 "$SOCK" >/dev/null 2>&1 || true
curl -s --noproxy '*' --max-time 2 -o /dev/null \
  "http://127.0.0.1:$CCT_PROXY_PORT/v1/responses" || true
kill "$PROXY_PID"; PROXY_PID=""
sleep 0.3

if [ ! -f "$LOG" ]; then
  echo "[SKIP] B005: 日志文件未生成（CCT_PROXY_LOG 未生效?）"
  exit 77
fi
if grep -Fq "$TEST_API_KEY" "$LOG"; then
  echo "[FAIL] B005: 日志含 api_key 明文 — 未脱敏"
  exit 1
fi
echo "[PASS] B005: 日志不含 api_key 明文"
