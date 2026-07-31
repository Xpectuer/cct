---
title: Tech Stack Snapshot
doc_type: reference
brief: - Cargo.toml
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
claude_md_coverage: false
---
# Tech Stack Snapshot

- `Cargo.toml`
  - package: `cct`
  - edition: `2021`
  - dependencies: `clap`, `ratatui`, `crossterm`, `serde`, `toml`, `toml_edit`, `dirs`, `anyhow`
  - dev-dependencies: `tempfile`, `serial_test`

This confirms the feature should be implemented as a Rust-side launch-path change with unit/integration coverage.
