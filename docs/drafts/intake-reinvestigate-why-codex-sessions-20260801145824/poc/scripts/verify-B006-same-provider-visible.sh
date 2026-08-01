#!/usr/bin/env bash
# PoC B006: 同 provider（proxy/custom）两 profile, resume --last 互相可见
# Source: spec.md Acceptance Criterion #6
# Target: cli
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
[ -f "$SCRIPT_DIR/../config.env" ] && source "$SCRIPT_DIR/../config.env"

echo "[PoC B006] 同 provider 会话在 resume --last 中互相可见"
source "$SCRIPT_DIR/setup-smoke.sh"

# profile A: 创建会话; profile B（同 provider）: resume --last 应看到 A 的会话
if "$CCT_BIN" run smoke-a >"$SMOKE_DIR/run-a.log" 2>&1; then :; else
  echo "[FAIL] B006: cct run smoke-a 失败 — $(tail -3 "$SMOKE_DIR/run-a.log" | tr '\n' ' ')"
  exit 1
fi
grep -q "POC_STUB_LAST_MESSAGE" "$SMOKE_DIR/out-a.txt" || {
  echo "[FAIL] B006: smoke-a 会话未创建成功（-o 无 stub 末尾文本）"
  exit 1
}

if "$CCT_BIN" run smoke-b >"$SMOKE_DIR/run-b.log" 2>&1; then :; else
  echo "[FAIL] B006: cct run smoke-b 失败 — $(tail -3 "$SMOKE_DIR/run-b.log" | tr '\n' ' ')"
  exit 1
fi

if grep -q "POC_STUB_LAST_MESSAGE" "$SMOKE_DIR/out-b.txt"; then
  echo "[PASS] B006: profile B 经 resume --last 看到 profile A 的会话"
  exit 0
else
  echo "[FAIL] B006: resume --last 未见 profile A 会话 — 同 provider 可见性不符"
  exit 1
fi
