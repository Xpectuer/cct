---
title: Step 02
doc_type: proc
brief: Step 02
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
---
## Step 2 — Update CLAUDE.md platform-specific config path references

### Actions Taken
- Updated the project overview to mention the macOS config path alongside the Linux/Unix path.
- Made the config file format section describe both platform-specific locations.

### Verify Result
- `rg -n 'Library/Application Support/cc-tui/profiles.toml|~/.config/cc-tui/profiles.toml' CLAUDE.md` (exit 0)
