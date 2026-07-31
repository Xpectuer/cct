---
title: CLAUDE.md Snapshot
doc_type: reference
brief: cct is a terminal UI launcher for Claude Code. It reads named profiles from a TOML config file (~/.config/cc-tui/prof
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
claude_md_coverage: false
---
# CLAUDE.md Snapshot

`cct` is a terminal UI launcher for Claude Code. It reads named profiles from a TOML config file (`~/.config/cc-tui/profiles.toml`), displays them in a ratatui TUI, and exec-replaces the process with `claude <args>` when the user selects one.

Five modules: config, app, ui, launch, cli. Rust/Cargo project with ratatui TUI.
