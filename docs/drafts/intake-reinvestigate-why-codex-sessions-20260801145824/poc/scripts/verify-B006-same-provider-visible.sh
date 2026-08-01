#!/usr/bin/env bash
# PoC B006: 同 provider（proxy/custom）两 profile, resume --last 互相可见
# Source: spec.md Acceptance Criterion #6
# Target: cli
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
[ -f "$SCRIPT_DIR/../config.env" ] && source "$SCRIPT_DIR/../config.env"

echo "[PoC B006] 同 provider 会话在 resume --last 中互相可见"
source "$SCRIPT_DIR/setup-smoke.sh"

# 可证伪可观测量（codex 0.146，B008 已验证同语义）:
# - resume 复用既有会话 → rollout 文件数不变（同一会话文件被续写）
# - 无匹配会话（不可见/被过滤）→ 新建会话 → rollout 文件数 +1
# 因此本脚本的判别断言是 session-id 对比 + rollout 复用计数（spec AC-6:
# "输出中出现 profile A 会话的 session-id（-o 捕获 + rollout session_meta 对比）"），
# 而非固定标记文本（stub 对任何会话都返回同一 DELTA，标记断言不可证伪）。
rollout_count() { find "$CODEX_HOME/sessions" -type f -name 'rollout-*.jsonl' 2>/dev/null | wc -l | tr -d ' '; }
# codex 0.146 rollout 文件名: sessions/<Y>/<M>/<D>/rollout-<ts>-<session-id>.jsonl
session_id_of() { basename "$1" | sed -E 's/.*-([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})\.jsonl$/\1/'; }

# profile A: 创建会话
if "$CCT_BIN" run smoke-a </dev/null >"$SMOKE_DIR/run-a.log" 2>&1; then :; else
  echo "[FAIL] B006: cct run smoke-a 失败 — $(tail -3 "$SMOKE_DIR/run-a.log" | tr '\n' ' ')"
  exit 1
fi
grep -q "POC_STUB_LAST_MESSAGE" "$SMOKE_DIR/out-a.txt" || {
  echo "[FAIL] B006: smoke-a 会话未创建成功（-o 无 stub 末尾文本）"
  exit 1
}
[ "$(rollout_count)" -eq 1 ] || { echo "[FAIL] B006: smoke-a 应恰好产生 1 个 rollout（实际 $(rollout_count)）"; exit 1; }
SESSION_FILE_A=$(find "$CODEX_HOME/sessions" -type f -name 'rollout-*.jsonl' | head -1)
SESSION_ID_A=$(session_id_of "$SESSION_FILE_A")

# profile B（同 provider）: resume --last 应复用 profile A 的会话——
# 复用 → rollout 文件数不变且 session-id 与 A 一致；新建会话 → 数 +1（FAIL）
if "$CCT_BIN" run smoke-b </dev/null >"$SMOKE_DIR/run-b.log" 2>&1; then :; else
  echo "[FAIL] B006: cct run smoke-b 失败 — $(tail -3 "$SMOKE_DIR/run-b.log" | tr '\n' ' ')"
  exit 1
fi
grep -q "POC_STUB_LAST_MESSAGE" "$SMOKE_DIR/out-b.txt" || {
  echo "[FAIL] B006: resume --last 未产生输出（-o 无 stub 末尾文本）"
  exit 1
}
[ "$(rollout_count)" -eq 1 ] || {
  echo "[FAIL] B006: resume --last 新建了会话（rollout 数 $(rollout_count)，应为 1）— profile B 未看到 profile A 的会话"
  exit 1
}
SESSION_ID_B=$(session_id_of "$(find "$CODEX_HOME/sessions" -type f -name 'rollout-*.jsonl' | head -1)")
[ "$SESSION_ID_B" = "$SESSION_ID_A" ] || {
  echo "[FAIL] B006: resume --last 会话 id ($SESSION_ID_B) 与 profile A 会话 id ($SESSION_ID_A) 不一致 — 同 provider 可见性不符"
  exit 1
}

echo "[PASS] B006: profile B 经 resume --last 复用 profile A 的会话（session-id $SESSION_ID_A 一致, rollout 数不变）"
exit 0
