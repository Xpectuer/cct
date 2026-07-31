---
title: CLAUDE.md Snapshot
doc_type: reference
brief: cct is a terminal UI launcher for Claude Code. Reads named profiles from ~/.config/cc-tui/profiles.toml, displays th
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
claude_md_coverage: false
---
# CLAUDE.md Snapshot

`cct` is a terminal UI launcher for Claude Code. Reads named profiles from `~/.config/cc-tui/profiles.toml`, displays them in a ratatui TUI, and exec-replaces the process with `claude <args>` when the user selects one.

Five modules: config / app / ui / launch / cli. No shared mutable state.

Current key bindings (Normal mode):
- q / Ctrl+C: quit
- j/k / ↓↑: navigate
- Enter: launch selected profile
- c: launch with --continue
- e: open editor (hot-reload config)
- s: toggle skip_permissions on selected profile
- a: open add-form
