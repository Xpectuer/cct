#!/usr/bin/env bash
# PoC B004: cct→proxy→stub 上游整条链路（SSE 流式返回 + Bearer key 转发）
# Source: spec.md Acceptance Criterion #4
# Target: api
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
[ -f "$SCRIPT_DIR/../config.env" ] && source "$SCRIPT_DIR/../config.env"

echo "[PoC B004] proxy 指向 stub 上游: 转发带 Bearer key, SSE 流式返回"
source "$SCRIPT_DIR/setup-smoke.sh"

if "$CCT_BIN" run smoke-a </dev/null >"$SMOKE_DIR/run-a.log" 2>&1; then :; else
  echo "[FAIL] B004: cct run smoke-a 失败 — $(tail -3 "$SMOKE_DIR/run-a.log" | tr '\n' ' ')"
  exit 1
fi

grep -q "POC_STUB_LAST_MESSAGE" "$SMOKE_DIR/out-a.txt" || {
  echo "[FAIL] B004: -o 文件不含 stub 末尾文本（流式转发失败）"
  exit 1
}
grep -q "Authorization: Bearer $TEST_API_KEY" "$SMOKE_DIR/stub.log" || {
  echo "[FAIL] B004: stub 未收到带 Bearer key 的请求"
  exit 1
}
echo "[PASS] B004: cct→proxy→stub 链路正常 — Bearer key 转发 + SSE 流式返回"
