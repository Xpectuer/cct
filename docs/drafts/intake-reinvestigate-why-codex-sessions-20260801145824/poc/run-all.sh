#!/usr/bin/env bash
# PoC Verification Runner
# Generated from: Spec: cct proxy 死锁修复 + Codex 会话可见性验证与文档收尾
# Run all PoC scripts and report consolidated results.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
POC_SCRIPTS="$SCRIPT_DIR/scripts"

TOTAL=0; PASS=0; FAIL=0; SKIP=0
FAILURES=()

echo "=== PoC Runner ==="
echo "Draft: $(basename "$(dirname "$SCRIPT_DIR")")"
echo "Started: $(date '+%Y-%m-%d %H:%M:%S')"
echo ""

for script in "$POC_SCRIPTS"/verify-*.sh; do
  [ -f "$script" ] || continue
  TOTAL=$((TOTAL + 1))
  echo "--- $(basename "$script") ---"
  if "$script" 2>&1; then
    PASS=$((PASS + 1))
  else
    RC=$?
    if [ "$RC" -eq 77 ]; then
      SKIP=$((SKIP + 1))
    else
      FAIL=$((FAIL + 1))
      FAILURES+=("$(basename "$script")")
    fi
  fi
  echo ""
done

echo "=== Results ==="
echo "Total: $TOTAL | Pass: $PASS | Fail: $FAIL | Skip: $SKIP"
if [ "${#FAILURES[@]}" -gt 0 ]; then
  echo "Failures:"
  for f in "${FAILURES[@]}"; do printf '  - %s\n' "$f"; done
  exit 1
fi
echo "All checks passed."
