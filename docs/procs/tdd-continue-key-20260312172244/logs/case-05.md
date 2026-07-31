---
title: Case 05
doc_type: proc
brief: Case 05
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
---
## Case 5 — ui_footer_shows_resume_hint

### Actions Taken

1. **RED**: Added `assert!(normal_footer.contains("[c] Resume"));` to existing test `ui_footer_shows_add_hint` without updating the footer string. Ran `cargo test ui_footer_shows` — failed as expected with `assertion failed: normal_footer.contains("[c] Resume")`.

2. **GREEN**: Made two edits:
   - Updated production footer in `src/ui.rs` `draw` function (line ~102): inserted `[c] Resume` between `[Enter] Launch` and `[s] Skip-perms`.
   - Updated test `ui_footer_shows_add_hint` string to match the new footer.
   - Ran `cargo test ui_footer_shows` — passed.

3. **REFACTOR**: No refactoring needed.

### Verify Result

- `cargo test` — all 44 tests pass (33 lib + 2 main + 5 integration + 4 live).
- **STATUS: SUCCESS**
