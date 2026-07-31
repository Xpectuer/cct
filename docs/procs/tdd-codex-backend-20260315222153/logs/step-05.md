---
title: Step 5 — Update main.rs event handling
doc_type: proc
brief: Step 5 — Update main.rs event handling
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
---
# Step 5 — Update main.rs event handling

## Status: SUCCESS

## Summary

Added backend-aware dispatch in `main.rs` and a testable `build_launch_command()` function in `launch.rs`.

## Changes

### `src/launch.rs`
- Added `build_launch_command(profile, with_continue) -> (String, Vec<String>)` — returns `("claude", claude_args)` or `("codex", codex_args)` based on `profile.backend`. The `with_continue` flag is ignored for Codex.
- 3 new tests: `build_launch_command_dispatches_claude`, `build_launch_command_dispatches_claude_with_continue`, `build_launch_command_dispatches_codex`

### `src/main.rs`
- **Tab** key: toggles between Claude and Codex backend tabs
- **1/2** keys: switch directly to Claude (1) or Codex (2)
- **Enter**: dispatches to `exec_claude` or `exec_codex` based on selected profile's backend
- **c** (continue): only works for Claude-backend profiles
- **s** (toggle skip_permissions): only works for Claude-backend profiles
- **a** (add form): initializes `FormState.backend` from `app.active_backend`
- **y** (confirm add): field mapping is backend-aware:
  - Claude: fields = [name, description, base_url, api_key, model]
  - Codex: fields = [name, base_url, api_key, model, full_auto] — parses "y"/"yes" as `true`

## Test Results

All 65 tests pass (54 lib + 2 main + 5 integration + 4 live).
