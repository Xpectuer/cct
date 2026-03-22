---
title: "TDD: Codex notification toggle via [n] key"
doc_type: proc
status: active
source: "docs/drafts/intake-codex-notification-toggle-20260322170754"
brief: "TDD session for Codex notification toggle"
test_cmd: "cargo test"
created: 2026-03-22
updated: 2026-03-22
revision: 1
---

# Codex notification toggle via [n] key - TDD Session

**Started**: 2026-03-22 17:37
**Plan**: `./plan.md`

## Test Cases

| # | Test Case | Plan Section | Target File(s) | Red | Green | Refactor |
|---|-----------|--------------|----------------|-----|-------|----------|
| 1 | toggle_notifications_insert | Step 1, Step 2 | src/config.rs | [ ] | [ ] | [ ] |
| 2 | toggle_notifications_flip | Step 1, Step 2 | src/config.rs | [ ] | [ ] | [ ] |
| 3 | toggle_notifications_not_found | Step 1, Step 2 | src/config.rs | [ ] | [ ] | [ ] |
| 4 | n_key_dispatches_by_backend | Step 3 | src/main.rs | [ ] | [ ] | [ ] |
| 5 | generate_codex_config_writes_tui_notifications | Step 4 | src/launch.rs | [ ] | [ ] | [ ] |
| 6 | footer_backend_aware_notifications_hint | Step 5 | src/ui.rs | [ ] | [ ] | [ ] |
| 7 | focused_notification_regression_pass | Step 6, Step 7 | src/config.rs, src/main.rs, src/launch.rs, src/ui.rs | [ ] | [ ] | [ ] |

## Subagent Log

| # | Case | Outcome | Notes | Timestamp |
|---|------|---------|-------|-----------|

## Status

**Current case**: 1 / 7
**Progress**: 0% (0/7 complete)
**Blocked**: None

---
**Updated**: 2026-03-22 17:37
