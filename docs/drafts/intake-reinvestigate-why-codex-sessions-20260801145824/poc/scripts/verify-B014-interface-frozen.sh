#!/usr/bin/env bash
# PoC B014: 不写任何 Codex 配置文件（快照回归测试）+ 既有接口不变
# Source: spec.md Acceptance Criterion #14
# Target: cli
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
[ -f "$SCRIPT_DIR/../config.env" ] && source "$SCRIPT_DIR/../config.env"

command -v cargo >/dev/null 2>&1 || { echo "[SKIP] cargo not installed"; exit 77; }
REPO_ROOT="${REPO_ROOT:-$(git -C "$SCRIPT_DIR/../.." rev-parse --show-toplevel 2>/dev/null || echo)}"

echo "[PoC B014] 配置快照回归 + 接口冻结"
cd "$REPO_ROOT"

FAILS=0
if ! cargo test --quiet >/tmp/cct-poc-b014.log 2>&1; then
  echo "[FAIL] B014: 回归测试失败 — $(tail -5 /tmp/cct-poc-b014.log | tr '\n' ' ')"
  FAILS=$((FAILS + 1))
fi
grep -qE "CCT_PROXY_PORT" src/proxy.rs || { echo "[FAIL] B014: CCT_PROXY_PORT 接口缺失"; FAILS=$((FAILS + 1)); }
grep -qE "CCT_PROXY_LOG" src/proxy.rs || { echo "[FAIL] B014: CCT_PROXY_LOG 接口缺失"; FAILS=$((FAILS + 1)); }
grep -qE "proxy start|proxy stop|Run \{" src/main.rs || { echo "[FAIL] B014: cct proxy start|stop / run 命令接口缺失"; FAILS=$((FAILS + 1)); }

if [ "$FAILS" -eq 0 ]; then
  echo "[PASS] B014: 回归测试通过, 既有接口未变"
  exit 0
else
  echo "[FAIL] B014: $FAILS 处失败"
  exit 1
fi
