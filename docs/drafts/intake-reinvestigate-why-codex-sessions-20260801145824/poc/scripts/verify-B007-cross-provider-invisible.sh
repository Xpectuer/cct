#!/usr/bin/env bash
# PoC B007: 跨 provider 会话不可见; 显式 `codex exec resume <session-id>` 可恢复
# Source: spec.md Acceptance Criterion #7
# Target: cli
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
[ -f "$SCRIPT_DIR/../config.env" ] && source "$SCRIPT_DIR/../config.env"

echo "[PoC B007] 跨 provider 不可见 + 显式 resume <session-id> 可恢复"
source "$SCRIPT_DIR/setup-smoke.sh"

# 可证伪可观测量（codex 0.146）: resume 复用既有会话 → rollout 文件数不变;
# 无匹配会话（被 provider 过滤）→ 新建会话 → rollout 文件数 +1。
# "跨 provider 不可见"的判别断言 = 新建会话存在（count 2）且其 session-id 与
# profile A 的会话 id 无交集（id 级核对，spec AC-7: "输出中不包含另一 provider
# 的任何 session-id"），而非固定标记文本（smoke-sub 即使直连失败也会在会话
# 创建时生成 rollout 文件，续接是否可见在 rollout 层可区分）。
rollout_count() { find "$CODEX_HOME/sessions" -type f -name 'rollout-*.jsonl' 2>/dev/null | wc -l | tr -d ' '; }
# codex 0.146 rollout 文件名: sessions/<Y>/<M>/<D>/rollout-<ts>-<session-id>.jsonl
session_id_of() { basename "$1" | sed -E 's/.*-([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})\.jsonl$/\1/'; }

# profile A: 创建 custom provider 会话
if "$CCT_BIN" run smoke-a </dev/null >"$SMOKE_DIR/run-a.log" 2>&1; then :; else
  echo "[FAIL] B007: cct run smoke-a 失败 — $(tail -3 "$SMOKE_DIR/run-a.log" | tr '\n' ' ')"
  exit 1
fi
grep -q "POC_STUB_LAST_MESSAGE" "$SMOKE_DIR/out-a.txt" || {
  echo "[FAIL] B007: smoke-a 会话未创建成功"
  exit 1
}
[ "$(rollout_count)" -eq 1 ] || { echo "[FAIL] B007: smoke-a 应恰好产生 1 个 rollout（实际 $(rollout_count)）"; exit 1; }
SESSION_FILE_A=$(find "$CODEX_HOME/sessions" -type f -name 'rollout-*.jsonl' | head -1)
SESSION_ID_A=$(session_id_of "$SESSION_FILE_A")
[ -n "$SESSION_ID_A" ] || { echo "[FAIL] B007: 未找到测试会话 (sessions 目录为空)"; exit 1; }

# 切换 provider（subscription/openai，--config model_provider=openai 由 cct 真实
# 函数 build_codex_subscription_args 生成）后 resume --last: 不应见 custom 会话。
# smoke-sub 直连 api.openai.com（假 key → 请求必然失败、out-sub.txt 不生成），
# 但 codex 在会话创建时即写 rollout 文件——可见性以 rollout 层可证伪断言，
# 不依赖上游连通。
if "$CCT_BIN" run smoke-sub </dev/null >"$SMOKE_DIR/run-sub.log" 2>&1; then :; fi
# ① 若 resume 复用（跨 provider 错误可见）→ count 仍为 1 → FAIL
[ "$(rollout_count)" -eq 2 ] || {
  echo "[FAIL] B007: 跨 provider resume --last 复用了 custom 会话（rollout 数 $(rollout_count)，应为 2）— provider 过滤未生效"
  exit 1
}
# ② id 级核对: 新会话的 session-id 不得与 profile A 的会话 id 一致
SESSION_FILE_B=$(find "$CODEX_HOME/sessions" -type f -name 'rollout-*.jsonl' | grep -vxF "$SESSION_FILE_A" | head -1)
SESSION_ID_B=$(session_id_of "$SESSION_FILE_B")
[ "$SESSION_ID_B" != "$SESSION_ID_A" ] || {
  echo "[FAIL] B007: 新会话 session-id ($SESSION_ID_B) 与 profile A 会话 id 一致 — 跨 provider 错误可见"
  exit 1
}

# 显式恢复: 显式 resume <session-id> 可绕过 provider 过滤。
# 旗标禁止手工复刻（spec: "6 旗标由真实函数生成"）——追加临时 profile
# smoke-explicit 并经 `cct run` 启动, 6 个 --config 旗标由 cct 真实函数
# build_codex_proxy_config_args 生成（extra_args 嵌入 exec resume <id>, 指向
# proxy→stub 链路, 与 smoke-a 同 provider）。
cat >> "$CCT_CONFIG" << TOML_EOF

[[profiles]]
name = "smoke-explicit"
backend = "codex"
base_url = "http://127.0.0.1:$STUB_PORT/v1"
model = "gpt-4.1"
extra_args = ["exec", "resume", "$SESSION_ID_A", "-o", "$SMOKE_DIR/out-explicit.txt", "hello"]
[profiles.env]
OPENAI_API_KEY = "$TEST_API_KEY"
TOML_EOF
if "$CCT_BIN" run smoke-explicit </dev/null >"$SMOKE_DIR/run-exp.log" 2>&1; then :; else
  echo "[FAIL] B007: 显式 resume $SESSION_ID_A 失败 — $(tail -3 "$SMOKE_DIR/run-exp.log" | tr '\n' ' ')"
  exit 1
fi
grep -q "POC_STUB_LAST_MESSAGE" "$SMOKE_DIR/out-explicit.txt" || {
  echo "[FAIL] B007: 显式 resume 未恢复会话内容"
  exit 1
}
# ③ 显式 resume 必须复用既有会话（rollout 数仍为 2）；新建会话 → count 3 → FAIL
[ "$(rollout_count)" -eq 2 ] || {
  echo "[FAIL] B007: 显式 resume 新建了会话（rollout 数 $(rollout_count)，应为 2）— 未恢复 profile A 的会话"
  exit 1
}

echo "[PASS] B007: 跨 provider 不可见（新会话 $SESSION_ID_B 与 A 的 $SESSION_ID_A 不同）; 显式 resume $SESSION_ID_A 可恢复"
exit 0
