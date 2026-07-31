---
title: Step 08
doc_type: proc
brief: Step 08
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
---
## Case 8 — path_hint_shown_when_not_in_path

### RED
- Added test setting INSTALL_DIR to a nonexistent path
- Asserts output contains PATH hint message

### GREEN
- path_hint() implemented with case pattern matching on PATH
- Test passes

### REFACTOR
- No changes needed

### Result
SUCCESS
