#!/usr/bin/env bash
# PoC B009: 活 proxy 时手动 `cct proxy start` 报错退出, 不删 socket、不破坏原 proxy
# Source: spec.md Acceptance Criterion #9
# Target: process
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
[ -f "$SCRIPT_DIR/../config.env" ] && source "$SCRIPT_DIR/../config.env"

command -v nc >/dev/null 2>&1 || { echo "[SKIP] nc not installed"; exit 77; }
[ -x "${CCT_BIN:?Must be set in config.env}" ] || { echo "[SKIP] cct binary not built: $CCT_BIN"; exit 77; }

echo "[PoC B009] 双启动防护: 报错退出, 不删活 proxy 的 socket"

TMP=$(mktemp -d)
SOCK="${CCT_PROXY_SOCKET:-$TMP/proxy.sock}"
export CCT_PROXY_SOCKET="$SOCK" CCT_PROXY_PORT="${PROXY_PORT:-19191}"
cleanup() { [ -n "${PROXY_PID:-}" ] && kill "$PROXY_PID" 2>/dev/null || true; rm -f "$SOCK"; rm -rf "$TMP"; }
trap cleanup EXIT

probe() { printf '{"cmd":"status"}\n' | nc -U -w 2 "$SOCK" >/dev/null 2>&1; }

# 1) 启动第一个 proxy
"$CCT_BIN" proxy start >/dev/null 2>&1 & PROXY_PID=$!
for _ in $(seq 1 50); do
  probe && break
  sleep 0.1
done
probe || { echo "[FAIL] B009: 首个 proxy 未就绪"; exit 1; }

# 2) 第二个手动 start（同步）应报错退出且不 panic
OUT="$TMP/second.out"
if "$CCT_BIN" proxy start >"$OUT" 2>&1; then
  echo "[FAIL] B009: 活 proxy 时手动 start 未报错 (exit 0)"
  exit 1
fi
grep -qi "panicked" "$OUT" && { echo "[FAIL] B009: 第二次 start panic 而非明确报错 — $(head -3 "$OUT")"; exit 1; }
[ -S "$SOCK" ] || { echo "[FAIL] B009: 活 proxy 的 socket 被删除"; exit 1; }
probe || { echo "[FAIL] B009: 原 proxy 失去响应（被破坏）"; exit 1; }

echo "[PASS] B009: 双启动报错退出, 原 proxy socket 与响应完好"
