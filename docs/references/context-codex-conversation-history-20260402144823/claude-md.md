---
title: CLAUDE.md Snapshot
doc_type: reference
brief: cct is a terminal UI launcher for Claude Code and OpenAI Codex. It reads named profiles from a TOML config file, displ
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
claude_md_coverage: false
---
# CLAUDE.md Snapshot

`cct` is a terminal UI launcher for Claude Code and OpenAI Codex. It reads named profiles from a TOML config file, displays them in a ratatui TUI organized into Claude/Codex backend tabs, and exec-replaces the process with `claude <args>` or `codex [--full-auto]` when the user selects a profile.

Relevant architectural notes for this intake:

- `launch` owns Codex launch orchestration
- `generate_codex_config` writes profile-derived Codex config
- `CODEX_HOME` is set before `exec`
- the project prefers narrow side-effect boundaries and single-purpose modules
