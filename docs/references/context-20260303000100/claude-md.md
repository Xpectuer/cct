---
title: CLAUDE.md Snapshot
doc_type: reference
brief: "- Project: cct — terminal UI launcher for Claude Code"
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
claude_md_coverage: false
---
# CLAUDE.md Snapshot

- **Project**: `cct` — terminal UI launcher for Claude Code
- **Config**: TOML file at `~/.config/cc-tui/profiles.toml`
- **Modules**: config (deserialize TOML), app (cursor state), ui (ratatui rendering), launch (exec-replace)
- **Data flow**: main loads profiles → App → draw loop → Enter → exec_claude
- **Key design**: exec (not spawn), env var masking, config hot-reload on `e` key
