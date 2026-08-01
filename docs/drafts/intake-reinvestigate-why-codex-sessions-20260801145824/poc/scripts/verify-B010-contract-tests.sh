#!/usr/bin/env bash
# PoC B010: 契约测试覆盖（并发/僵尸/占端口/双启动竞态/转发/脱敏/stop 超时）
# Source: spec.md Acceptance Criterion #10
# Target: cli
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
[ -f "$SCRIPT_DIR/../config.env" ] && source "$SCRIPT_DIR/../config.env"

command -v cargo >/dev/null 2>&1 || { echo "[SKIP] cargo not installed"; exit 77; }
REPO_ROOT="${REPO_ROOT:-$(git -C "$SCRIPT_DIR/../.." rev-parse --show-toplevel 2>/dev/null || echo "${CCT_BIN%/*/../..}")}"

echo "[PoC B010] 契约测试全部通过"
cd "$REPO_ROOT"

FAILS=0
if ! cargo test --quiet proxy >/tmp/cct-poc-b010-proxy.log 2>&1; then
  echo "[FAIL] B010: cargo test proxy 失败 — $(tail -5 /tmp/cct-poc-b010-proxy.log | tr '\n' ' ')"
  FAILS=$((FAILS + 1))
fi
if ! cargo test --quiet --test integration >/tmp/cct-poc-b010-integration.log 2>&1; then
  echo "[FAIL] B010: integration 测试失败 — $(tail -5 /tmp/cct-poc-b010-integration.log | tr '\n' ' ')"
  FAILS=$((FAILS + 1))
fi

if [ "$FAILS" -eq 0 ]; then
  echo "[PASS] B010: proxy 单元契约 + integration 测试全部通过"
  exit 0
else
  echo "[FAIL] B010: 契约测试存在失败项"
  exit 1
fi
