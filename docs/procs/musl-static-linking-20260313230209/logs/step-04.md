---
title: Step 04
doc_type: proc
brief: Step 04
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
---
## Step 4 — Add verification steps to release workflow

### Actions Taken

Inserted two new workflow steps in `.github/workflows/release.yml` immediately before the existing "Upload artifact" step:

1. **Verify static linking** — runs `file ... | grep -i "statically linked"` on all `linux-musl` targets to confirm the binary is statically linked.
2. **Smoke test on Ubuntu 20.04** — runs the built `cct --help` inside a `ubuntu:20.04` Docker container, but only for the `x86_64-unknown-linux-musl` target (aarch64 skipped since GitHub runners are x86_64).

Both steps use `if:` conditions so they only execute for the relevant matrix entries.

### Verify Result

- `grep -c 'statically linked' .github/workflows/release.yml` returned **1** (pass)
- `grep -c 'ubuntu:20.04' .github/workflows/release.yml` returned **1** (pass)
