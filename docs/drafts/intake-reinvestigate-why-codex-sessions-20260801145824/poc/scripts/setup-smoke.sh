#!/usr/bin/env bash
# PoC 冒烟 fixture — 起 stub 上游 + 生成临时 profiles.toml + export 隔离 env。
# 由 verify-B004/B006/B007/B008 source 使用；退出时自动清理（trap EXIT）。
set -euo pipefail

: "${TEST_API_KEY:?Must be set in config.env}"
: "${STUB_PORT:?Must be set in config.env}"
command -v python3 >/dev/null 2>&1 || { echo "[SKIP] python3 not installed"; exit 77; }
command -v codex >/dev/null 2>&1 || { echo "[SKIP] codex CLI not installed"; exit 77; }
[ -x "${CCT_BIN:?Must be set in config.env}" ] || { echo "[SKIP] cct binary not built: $CCT_BIN"; exit 77; }

SMOKE_DIR=$(mktemp -d)
STUB_LOG="$SMOKE_DIR/stub.log"
STUB_PID=""
export SMOKE_DIR STUB_LOG TEST_API_KEY STUB_PORT
export CCT_CONFIG="$SMOKE_DIR/profiles.toml"
export CODEX_HOME="$SMOKE_DIR/codex-home"
export CCT_PROXY_SOCKET="${CCT_PROXY_SOCKET:-$SMOKE_DIR/proxy.sock}"
export CCT_PROXY_PORT="${PROXY_PORT:-19191}"
export OPENAI_API_KEY="$TEST_API_KEY"

cleanup_smoke() {
  # 停掉 cct run 内部 ensure_proxy_running 拉起的 proxy daemon（经控制 socket），
  # 否则遗留 daemon 占住 TCP 端口 → B002/B009 首跑级联失败。
  "$CCT_BIN" proxy stop >/dev/null 2>&1 || true
  [ -n "${STUB_PID:-}" ] && kill "$STUB_PID" 2>/dev/null || true
  rm -rf "$SMOKE_DIR"
}
trap cleanup_smoke EXIT

python3 "$SCRIPT_DIR/stub-sse-upstream.py" "$STUB_PORT" "$STUB_LOG" & STUB_PID=$!
for _ in $(seq 1 50); do
  grep -q "LISTENING" "$STUB_LOG" 2>/dev/null && break
  sleep 0.1
done
grep -q "LISTENING" "$STUB_LOG" || {
  echo "[FAIL] stub 上游未就绪（端口 $STUB_PORT 被占用?）"
  exit 1
}

mkdir -p "$CODEX_HOME"
# api_key 必须写在 [profiles.env] OPENAI_API_KEY（exec_codex_proxy 只从这里读）;
# 顶层 api_key 字段不存在于 Profile schema, 会被 serde 静默忽略 → Bearer 转发断言失败。
cat > "$CCT_CONFIG" << EOF
[[profiles]]
name = "smoke-a"
backend = "codex"
base_url = "http://127.0.0.1:$STUB_PORT/v1"
model = "gpt-4.1"
extra_args = ["exec", "-o", "$SMOKE_DIR/out-a.txt", "--dangerously-bypass-approvals-and-sandbox", "hello"]
[profiles.env]
OPENAI_API_KEY = "$TEST_API_KEY"

[[profiles]]
name = "smoke-b"
backend = "codex"
base_url = "http://127.0.0.1:$STUB_PORT/v1"
model = "gpt-4.1"
extra_args = ["exec", "resume", "--last", "-o", "$SMOKE_DIR/out-b.txt", "hello"]
[profiles.env]
OPENAI_API_KEY = "$TEST_API_KEY"

[[profiles]]
name = "smoke-c"
backend = "codex"
base_url = "http://127.0.0.1:$STUB_PORT/v1"
model = "gpt-4.1"
extra_args = ["exec", "resume", "--last", "--all", "-o", "$SMOKE_DIR/out-c.txt", "hello"]
[profiles.env]
OPENAI_API_KEY = "$TEST_API_KEY"

[[profiles]]
name = "smoke-sub"
backend = "codex"
auth_type = "subscription"
extra_args = ["exec", "resume", "--last", "-o", "$SMOKE_DIR/out-sub.txt", "hello"]
EOF
