---
title: Step 02
doc_type: proc
brief: Step 02
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
---
## Step 2 — Add musl toolchain, cross install, and conditional build steps

### Actions Taken
- Replaced the single `cargo build --release` step in `.github/workflows/release.yml` with three steps:
  1. **Install musl toolchain** (conditional on `linux-musl` target): installs `musl-tools`, adds the rustup target.
  2. **Install cross** (conditional on `matrix.use_cross == true`): installs cross from git.
  3. **Build release binary**: conditionally uses `cross build` or `cargo build` based on `matrix.use_cross`, always passing `--target`.

### Verify Result
- `grep -c 'musl-tools' .github/workflows/release.yml` returned **1** — PASS
- `grep -c 'cross build' .github/workflows/release.yml` returned **1** — PASS
