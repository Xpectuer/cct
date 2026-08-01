#!/usr/bin/env bash
# PoC B013: 文档收尾 — 5 份文档无 per-profile CODEX_HOME / generate_codex_config 陈旧叙述
#           + 新增 "resume 按 model_provider ∩ cwd 过滤" 语义说明
# Source: spec.md Acceptance Criterion #13
# Target: filesystem
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
[ -f "$SCRIPT_DIR/../config.env" ] && source "$SCRIPT_DIR/../config.env"

REPO_ROOT="${REPO_ROOT:-$(git -C "$SCRIPT_DIR/../.." rev-parse --show-toplevel 2>/dev/null || echo)}"
DOCS=(CLAUDE.md ARCHITECTURE.md docs/modules/launch.md
      docs/references/codex-home-storage-layout.md
      docs/references/codex-backend-development-guide.md)
STALE_PAT='per-profile CODEX_HOME|per profile CODEX_HOME|generate_codex_config'

echo "[PoC B013] 文档收尾检查（陈旧叙述 + resume 过滤语义）"
FAILS=0

for d in "${DOCS[@]}"; do
  if [ ! -f "$REPO_ROOT/$d" ]; then
    echo "[FAIL] B013: 缺少文档 $d"
    FAILS=$((FAILS + 1))
    continue
  fi
  if grep -qE "$STALE_PAT" "$REPO_ROOT/$d"; then
    echo "[FAIL] B013: $d 仍含陈旧叙述（per-profile CODEX_HOME / generate_codex_config）"
    FAILS=$((FAILS + 1))
  fi
done

if ! grep -qE "resume.*(model_provider|cwd)|model_provider.*cwd" \
  "$REPO_ROOT/docs/references/codex-backend-development-guide.md"; then
  echo "[FAIL] B013: 缺 resume 按 model_provider ∩ cwd 过滤的语义说明"
  FAILS=$((FAILS + 1))
fi

if [ "$FAILS" -eq 0 ]; then
  echo "[PASS] B013: 5 份文档无陈旧叙述, resume 过滤语义已说明"
  exit 0
else
  echo "[FAIL] B013: $FAILS 处文档收尾未完成"
  exit 1
fi
