---
title: Tech Stack Snapshot
doc_type: reference
brief: "- Language: Rust (edition 2021)"
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
claude_md_coverage: false
---
# Tech Stack Snapshot

- Language: Rust (edition 2021)
- Build: Cargo
- Dependencies: clap 4, ratatui 0.29, crossterm 0.28, serde 1, toml 0.8, toml_edit 0.22, dirs 5, anyhow 1
- CI/CD: GitHub Actions (ci.yml + release.yml)
- Release targets: aarch64-apple-darwin, x86_64-apple-darwin, x86_64-unknown-linux-gnu
- Linux build runner: ubuntu-latest (glibc 2.35+)
