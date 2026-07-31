---
title: Tech Stack Snapshot
doc_type: reference
brief: "Detected from Cargo.toml (Rust project):"
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
claude_md_coverage: false
---
# Tech Stack Snapshot

Detected from Cargo.toml (Rust project):

- Language: Rust (edition 2021)
- TUI: ratatui + crossterm
- Config: serde + toml + toml_edit
- CLI parsing: clap
- Error handling: anyhow
- Process exec: std::os::unix::process::CommandExt
- Testing: cargo test (unit + integration), bats-core (shell tests for install.sh)
