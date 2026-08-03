#!/usr/bin/env bash
# PoC B003: 进程存活占端口时启动新 proxy → 明确报错退出（lsof 诊断）, 不 panic、不自动终止
# Source: spec.md Acceptance Criterion #3
# Target: process
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
[ -f "$SCRIPT_DIR/../config.env" ] && source "$SCRIPT_DIR/../config.env"

command -v python3 >/dev/null 2>&1 || { echo "[SKIP] python3 not installed"; exit 77; }
[ -x "${CCT_BIN:?Must be set in config.env}" ] || { echo "[SKIP] cct binary not built: $CCT_BIN"; exit 77; }

echo "[PoC B003] 端口被占时明确报错, 不 panic 不自动终止"

TMP=$(mktemp -d)
export CCT_PROXY_PORT="${PROXY_PORT:-19191}"
export CCT_PROXY_SOCKET="${CCT_PROXY_SOCKET:-$TMP/proxy.sock}"
cleanup() { [ -n "${OCCUPY_PID:-}" ] && kill "$OCCUPY_PID" 2>/dev/null || true; [ -n "${PROXY_PID:-}" ] && kill "$PROXY_PID" 2>/dev/null || true; rm -rf "$TMP"; }
trap cleanup EXIT

# 假占用者: 绑定 TCP 端口但不响应控制 socket（模拟旧版死锁/第三方进程）
python3 -c "import socket,time; s=socket.socket(); s.setsockopt(socket.SOL_SOCKET,socket.SO_REUSEADDR,1); s.bind(('127.0.0.1',$CCT_PROXY_PORT)); s.listen(1); time.sleep(30)" & OCCUPY_PID=$!
sleep 0.3

OUT="$TMP/start.out"
"$CCT_BIN" proxy start >"$OUT" 2>&1 && RC=0 || RC=$?

if [ "$RC" -eq 0 ]; then
  echo "[FAIL] B003: 端口被占时启动未报错 (exit 0)"
  exit 1
fi
kill -0 "$OCCUPY_PID" 2>/dev/null || { echo "[FAIL] B003: 占用进程被自动终止"; exit 1; }
grep -qi "panicked" "$OUT" && { echo "[FAIL] B003: proxy panic 而非明确报错 — $(head -3 "$OUT")"; exit 1; }
grep -qE "端口|占用|EADDRINUSE|Address already" "$OUT" || {
  echo "[FAIL] B003: 报错消息不含端口占用信息 — $(head -3 "$OUT")"; exit 1
}
if command -v lsof >/dev/null 2>&1; then
  grep -q "$OCCUPY_PID" "$OUT" || { echo "[FAIL] B003: lsof 可用但报错未含占用者 PID"; exit 1; }
fi
echo "[PASS] B003: 端口被占时明确报错退出 (RC=$RC), 占用进程存活, 未 panic"
