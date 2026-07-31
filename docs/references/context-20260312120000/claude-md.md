---
title: CLAUDE.md Snapshot
doc_type: reference
brief: cct is a terminal UI launcher for Claude Code. It reads named profiles from a TOML config file
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
claude_md_coverage: false
---
# CLAUDE.md Snapshot

`cct` is a terminal UI launcher for Claude Code. It reads named profiles from a TOML config file
(`~/.config/cc-tui/profiles.toml`), displays them in a ratatui TUI, and exec-replaces the process
with `claude <args>` when the user selects one.

Five focused modules: config, app, ui, launch, cli. Data flow: main → load profiles → App → draw
loop → Enter → launch::exec_claude (exec-replace, no return on success).

Key bindings (Normal mode): ↑↓/jk navigate, Enter launch, s toggle skip_permissions, a add form,
e edit config, q/Ctrl-C quit.
