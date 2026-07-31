---
title: Step 06
doc_type: proc
brief: Step 06
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
---
## Step 6 — Update README for inline edit
### Actions Taken
- Replaced the stale feature bullet that described `e` as an external-editor hot-reload path.
- Updated Quick Start so inline editing is described as the `e` flow and direct file editing is positioned as the manual alternative.
- Changed the normal-mode keybinding entry to describe inline editing of the selected profile.

### Verify Result
- `rg -n "Edit config|\\$EDITOR|hot-reload" README.md` only returned the expected empty result after the README update.
