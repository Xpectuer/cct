#!/usr/bin/env bash
# PoC B008: 同 provider 跨仓库（cwd）会话不可见; 追加 --all 后可见
# Source: spec.md Acceptance Criterion #8
# Target: cli
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
[ -f "$SCRIPT_DIR/../config.env" ] && source "$SCRIPT_DIR/../config.env"

command -v git >/dev/null 2>&1 || { echo "[SKIP] git not installed"; exit 77; }

echo "[PoC B008] cwd 过滤: 跨仓库不可见, --all 可见"
source "$SCRIPT_DIR/setup-smoke.sh"

mkdir -p "$SMOKE_DIR/repo1" "$SMOKE_DIR/repo2"
git -C "$SMOKE_DIR/repo1" init -q
git -C "$SMOKE_DIR/repo2" init -q

# 会话在 repo1 创建
if (cd "$SMOKE_DIR/repo1" && "$CCT_BIN" run smoke-a) >"$SMOKE_DIR/run-a.log" 2>&1; then :; else
  echo "[FAIL] B008: repo1 中 cct run smoke-a 失败 — $(tail -3 "$SMOKE_DIR/run-a.log" | tr '\n' ' ')"
  exit 1
fi
grep -q "POC_STUB_LAST_MESSAGE" "$SMOKE_DIR/out-a.txt" || {
  echo "[FAIL] B008: smoke-a 会话未创建成功"
  exit 1
}

# repo2 中 resume --last: 默认 cwd 过滤 → 不可见
if (cd "$SMOKE_DIR/repo2" && "$CCT_BIN" run smoke-b) >"$SMOKE_DIR/run-b.log" 2>&1; then :; fi
if [ -f "$SMOKE_DIR/out-b.txt" ] && grep -q "POC_STUB_LAST_MESSAGE" "$SMOKE_DIR/out-b.txt"; then
  echo "[FAIL] B008: 跨仓库 resume --last 错误可见"
  exit 1
fi

# repo2 中 resume --last --all: 跨目录 → 可见
if (cd "$SMOKE_DIR/repo2" && "$CCT_BIN" run smoke-c) >"$SMOKE_DIR/run-c.log" 2>&1; then :; else
  echo "[FAIL] B008: cct run smoke-c (--all) 失败 — $(tail -3 "$SMOKE_DIR/run-c.log" | tr '\n' ' ')"
  exit 1
fi
if grep -q "POC_STUB_LAST_MESSAGE" "$SMOKE_DIR/out-c.txt"; then
  echo "[PASS] B008: 默认跨仓库不可见, --all 后可见（cwd 过滤维度）"
  exit 0
else
  echo "[FAIL] B008: --all 后仍不可见"
  exit 1
fi
