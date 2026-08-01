#!/usr/bin/env bash
# PoC B012: L2 冒烟前置条件 — 旧版死锁实例已终止 + proxy 端口空闲
# Source: spec.md Acceptance Criterion #12
# Target: process
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
[ -f "$SCRIPT_DIR/../config.env" ] && source "$SCRIPT_DIR/../config.env"

command -v nc >/dev/null 2>&1 || { echo "[SKIP] nc not installed"; exit 77; }
PROXY_PORT="${PROXY_PORT:-19191}"

echo "[PoC B012] L2 冒烟前置条件检查"
FAILS=0

# 旧版死锁实例（spec 记录 PID 29182）必须已终止
if kill -0 29182 2>/dev/null; then
  echo "[FAIL] B012: 旧版死锁实例 PID 29182 仍在运行 — 先手动终止（释放 TCP 端口 + 删遗留 socket）"
  FAILS=$((FAILS + 1))
else
  echo "[OK] B012: 旧实例 PID 29182 未在运行"
fi

# 测试期间端口必须空闲（避免互踩真实 proxy）
if nc -z -w 2 127.0.0.1 "$PROXY_PORT" 2>/dev/null; then
  echo "[FAIL] B012: 端口 $PROXY_PORT 被占用 — 先迁移/停止现有 proxy"
  FAILS=$((FAILS + 1))
else
  echo "[OK] B012: 端口 $PROXY_PORT 空闲"
fi

if [ "$FAILS" -eq 0 ]; then
  echo "[PASS] B012: 前置条件满足（旧实例已终止 + 端口空闲）"
  exit 0
else
  echo "[FAIL] B012: 前置条件未满足 — 按上方提示迁移后重试"
  exit 1
fi
