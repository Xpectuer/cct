#!/usr/bin/env bash
# PoC B011: install-script.md 含旧版死锁实例迁移说明（一次性升级指引）
# Source: spec.md Acceptance Criterion #11
# Target: filesystem
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
[ -f "$SCRIPT_DIR/../config.env" ] && source "$SCRIPT_DIR/../config.env"

REPO_ROOT="${REPO_ROOT:-$(git -C "$SCRIPT_DIR/../.." rev-parse --show-toplevel 2>/dev/null || echo)}"
DOC="$REPO_ROOT/docs/references/install-script.md"

echo "[PoC B011] 迁移说明已写入 install-script.md"

if [ ! -f "$DOC" ]; then
  echo "[FAIL] B011: 缺少 install-script.md ($DOC)"
  exit 1
fi
if grep -qE "29182|手动终止|遗留.*socket|死锁实例" "$DOC"; then
  echo "[PASS] B011: install-script.md 含旧实例迁移说明"
  exit 0
else
  echo "[FAIL] B011: install-script.md 无迁移说明（旧版实例处理步骤缺失）"
  exit 1
fi
