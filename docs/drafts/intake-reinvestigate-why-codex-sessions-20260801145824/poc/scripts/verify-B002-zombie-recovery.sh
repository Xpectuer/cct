#!/usr/bin/env bash
# PoC B002: 僵尸死 proxy（进程退出、socket 残留、端口空闲）自愈重启
# Source: spec.md Acceptance Criterion #2
# Target: process
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
[ -f "$SCRIPT_DIR/../config.env" ] && source "$SCRIPT_DIR/../config.env"

command -v nc >/dev/null 2>&1 || { echo "[SKIP] nc not installed"; exit 77; }
[ -x "${CCT_BIN:?Must be set in config.env}" ] || { echo "[SKIP] cct binary not built: $CCT_BIN"; exit 77; }

echo "[PoC B002] 僵尸死 proxy 自愈重启"

TMP=$(mktemp -d)
SOCK="${CCT_PROXY_SOCKET:-$TMP/proxy.sock}"
export CCT_PROXY_SOCKET="$SOCK" CCT_PROXY_PORT="${PROXY_PORT:-19191}"
cleanup() { kill "${PROXY_PID:-}" 2>/dev/null; rm -f "$SOCK"; rm -rf "$TMP"; }
trap cleanup EXIT

probe() { printf '{"cmd":"status"}\n' | nc -U -w 2 "$SOCK" >/dev/null 2>&1; }

# 1) 正常启动并确认健康
"$CCT_BIN" proxy start >/dev/null 2>&1 & PROXY_PID=$!
for _ in $(seq 1 50); do
  probe && break
  sleep 0.1
done
probe || { echo "[FAIL] B002: 首次启动后应用层探测失败"; exit 1; }

# 2) 制造僵尸: kill -9, 留下 socket 文件与空闲端口
kill -9 "$PROXY_PID"; PROXY_PID=""
wait 2>/dev/null || true
sleep 0.3
[ -S "$SOCK" ] || { echo "[FAIL] B002: 僵尸场景未建立（socket 被清理）"; exit 1; }

# 3) 重新启动 → 应清理死 socket 并自愈（应用层探测成功）
"$CCT_BIN" proxy start >/dev/null 2>&1 & PROXY_PID=$!
for _ in $(seq 1 20); do
  probe && { echo "[PASS] B002: 死 socket 被清理, proxy 自愈并响应"; exit 0; }
  sleep 0.5
done
echo "[FAIL] B002: 重启后 10s 内应用层探测未成功 — 自愈未实现"
exit 1
