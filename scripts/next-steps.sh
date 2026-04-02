#!/usr/bin/env bash
# next-steps.sh — Emit correct next command based on draft session status.
# Usage: bash scripts/next-steps.sh <draft-dir>
# Exit 0: prints next step
# Exit 1: error (missing session-log.md or unknown status)

set -euo pipefail

DRAFT_DIR="${1:-}"

if [ -z "$DRAFT_DIR" ]; then
  echo "[ERROR] Usage: next-steps.sh <draft-dir>"
  exit 1
fi

SESSION_LOG="$DRAFT_DIR/session-log.md"

if [ ! -f "$SESSION_LOG" ]; then
  echo "[ERROR] No session-log.md in $DRAFT_DIR"
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
STATUS=$("$SCRIPT_DIR/fm.sh" get "$SESSION_LOG" status 2>/dev/null) || {
  echo "[ERROR] Could not extract status from $SESSION_LOG"
  exit 1
}

case "$STATUS" in
  ready)
    echo "Draft status: ready"
    echo "Next: /tdd $DRAFT_DIR  or  /progress $DRAFT_DIR"
    ;;
  active)
    echo "Draft status: active"
    echo "Next: /idea $DRAFT_DIR  or  /interview $DRAFT_DIR"
    ;;
  activated)
    echo "Draft status: activated — already executing."
    echo "Check /dashboard for your active proc."
    ;;
  abandoned)
    echo "Draft status: abandoned."
    ;;
  *)
    echo "[ERROR] Unknown or missing status '${STATUS}' in $SESSION_LOG"
    exit 1
    ;;
esac
