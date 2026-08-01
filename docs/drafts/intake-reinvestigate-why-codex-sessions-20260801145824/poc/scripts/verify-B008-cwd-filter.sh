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

mkdir -p "$SMOKE_DIR/repo1" "$SMOKE_DIR/repo2" "$SMOKE_DIR/repo3"
git -C "$SMOKE_DIR/repo1" init -q
git -C "$SMOKE_DIR/repo2" init -q
git -C "$SMOKE_DIR/repo3" init -q

# 会话在 repo1 创建
if (cd "$SMOKE_DIR/repo1" && "$CCT_BIN" run smoke-a) </dev/null >"$SMOKE_DIR/run-a.log" 2>&1; then :; else
  echo "[FAIL] B008: repo1 中 cct run smoke-a 失败 — $(tail -3 "$SMOKE_DIR/run-a.log" | tr '\n' ' ')"
  exit 1
fi
grep -q "POC_STUB_LAST_MESSAGE" "$SMOKE_DIR/out-a.txt" || {
  echo "[FAIL] B008: smoke-a 会话未创建成功"
  exit 1
}

# codex 0.146 可见性可观测量: resume 复用既有会话时 rollout 文件数不变;
# 无匹配会话（被过滤）时新建会话 → rollout 文件数 +1。
# 注意: resume 会把最新 turn 的 cwd 写回会话（latest_turn_context_cwd），
# 因此 --all 验证（repo2）与默认过滤验证（repo3）必须用不同目录，避免互相干扰。
rollout_count() { find "$CODEX_HOME/sessions" -type f -name 'rollout-*.jsonl' 2>/dev/null | wc -l | tr -d ' '; }
[ "$(rollout_count)" -eq 1 ] || { echo "[FAIL] B008: smoke-a 应恰好产生 1 个 rollout（实际 $(rollout_count)）"; exit 1; }

# repo2 中 resume --last --all: 跨目录 → A 可见 → 复用既有会话（rollout 数不变）
if (cd "$SMOKE_DIR/repo2" && "$CCT_BIN" run smoke-c) </dev/null >"$SMOKE_DIR/run-c.log" 2>&1; then :; else
  echo "[FAIL] B008: cct run smoke-c (--all) 失败 — $(tail -3 "$SMOKE_DIR/run-c.log" | tr '\n' ' ')"
  exit 1
fi
if ! grep -q "POC_STUB_LAST_MESSAGE" "$SMOKE_DIR/out-c.txt"; then
  echo "[FAIL] B008: --all 后 resume --last 未恢复跨仓库会话"
  exit 1
fi
[ "$(rollout_count)" -eq 1 ] || { echo "[FAIL] B008: --all resume 应复用既有会话（rollout 数应为 1, 实际 $(rollout_count)）"; exit 1; }

# repo3 中 resume --last（默认）: cwd 过滤 → A 不可见 → 新建会话（rollout 数 +1）
if (cd "$SMOKE_DIR/repo3" && "$CCT_BIN" run smoke-b) </dev/null >"$SMOKE_DIR/run-b.log" 2>&1; then :; fi
if [ "$(rollout_count)" -eq 1 ]; then
  echo "[FAIL] B008: 跨仓库 resume --last 未新建独立会话（rollout 数仍为 1）— cwd 过滤未生效"
  exit 1
fi

echo "[PASS] B008: 默认跨仓库不可见, --all 后可见（cwd 过滤维度）"
