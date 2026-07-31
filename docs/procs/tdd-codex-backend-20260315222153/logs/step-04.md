---
title: Step 4 — Codex launch functions in launch.rs
doc_type: proc
brief: "Status: SUCCESS"
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
---
# Step 4 — Codex launch functions in launch.rs

**Status**: SUCCESS
**Date**: 2026-03-15

## What was done

Added codex backend launch support to `src/launch.rs`:

1. **`check_codex_installed()`** — checks if `codex` binary is in PATH via `which`.
2. **`generate_codex_config(profile, codex_home)`** — writes a `config.toml` with `model_provider`, `model`, `name`, and `base_url` derived from the profile. Takes `&Path` for testability with tempdir.
3. **`build_codex_args(profile)`** — pure function that builds CLI args: `--full-auto` when enabled, plus any `extra_args`.
4. **`exec_codex(profile)`** — orchestrates config generation, env injection, `CODEX_HOME` setup, and exec-replace with `codex`.

Added imports: `fs`, `PathBuf`.

## TDD Cases

| # | Test | Result |
|---|------|--------|
| 12 | `build_codex_args` — empty, full_auto only, extra only, both | PASS |
| 13 | `generate_codex_config` — writes correct toml, defaults model to gpt-4.1 | PASS |

## Test count

- 50 unit tests, 2 main tests, 5 integration, 4 live — all pass.
- No new warnings introduced (1 pre-existing `field_labels` warning).
