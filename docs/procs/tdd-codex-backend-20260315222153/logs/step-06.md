---
title: Step 6 — Update cli.rs for codex add flow
doc_type: proc
brief: Step 6 — Update cli.rs for codex add flow
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
---
# Step 6 — Update cli.rs for codex add flow

## Status: SUCCESS

## Summary

Verified that the CLI `run_add_with()` flow correctly sets `backend: Backend::Claude` and `full_auto: None` for profiles added via `cct add`.

## Changes

### `src/cli.rs`
- Added test `cli_add_sets_claude_backend_and_no_full_auto`: creates a profile via `run_add_with()`, verifies `p.backend == Backend::Claude` and `p.full_auto.is_none()`.
- The CLI code already had the correct `backend: config::Backend::Claude` and `full_auto: None` fields from Step 1, so the test was immediately GREEN. This is a confirmation test.

## Test Results

All 65 tests pass (54 lib + 2 main + 5 integration + 4 live).
