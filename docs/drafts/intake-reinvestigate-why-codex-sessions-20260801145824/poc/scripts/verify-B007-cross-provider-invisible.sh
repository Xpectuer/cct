#!/usr/bin/env bash
# PoC B007: 跨 provider 会话不可见; 显式 `codex exec resume <session-id>` 可恢复
# Source: spec.md Acceptance Criterion #7
# Target: cli
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
[ -f "$SCRIPT_DIR/../config.env" ] && source "$SCRIPT_DIR/../config.env"

echo "[PoC B007] 跨 provider 不可见 + 显式 resume <session-id> 可恢复"
source "$SCRIPT_DIR/setup-smoke.sh"

# profile A: 创建 custom provider 会话
if "$CCT_BIN" run smoke-a >"$SMOKE_DIR/run-a.log" 2>&1; then :; else
  echo "[FAIL] B007: cct run smoke-a 失败 — $(tail -3 "$SMOKE_DIR/run-a.log" | tr '\n' ' ')"
  exit 1
fi
grep -q "POC_STUB_LAST_MESSAGE" "$SMOKE_DIR/out-a.txt" || {
  echo "[FAIL] B007: smoke-a 会话未创建成功"
  exit 1
}

# 切换 provider（subscription/openai）后 resume --last: 不应见 custom 会话
if "$CCT_BIN" run smoke-sub >"$SMOKE_DIR/run-sub.log" 2>&1; then :; fi
if [ -f "$SMOKE_DIR/out-sub.txt" ] && grep -q "POC_STUB_LAST_MESSAGE" "$SMOKE_DIR/out-sub.txt"; then
  echo "[FAIL] B007: 跨 provider resume --last 错误可见 custom 会话"
  exit 1
fi

# 显式恢复: 从临时 CODEX_HOME 的 sessions 目录取 session-id
SESSION_ID=$(ls -1t "$CODEX_HOME"/sessions/*.jsonl 2>/dev/null | head -1 \
  | xargs -n1 basename 2>/dev/null | sed 's/\.jsonl$//')
[ -n "$SESSION_ID" ] || { echo "[FAIL] B007: 未找到测试会话 (sessions 目录为空)"; exit 1; }

if codex exec resume "$SESSION_ID" -o "$SMOKE_DIR/out-explicit.txt" >"$SMOKE_DIR/run-exp.log" 2>&1; then :; else
  echo "[FAIL] B007: 显式 resume $SESSION_ID 失败 — $(tail -3 "$SMOKE_DIR/run-exp.log" | tr '\n' ' ')"
  exit 1
fi
if grep -q "POC_STUB_LAST_MESSAGE" "$SMOKE_DIR/out-explicit.txt"; then
  echo "[PASS] B007: 跨 provider 不可见; 显式 resume <session-id> 可恢复"
  exit 0
else
  echo "[FAIL] B007: 显式 resume 未恢复会话内容"
  exit 1
fi
