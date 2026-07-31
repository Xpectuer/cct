---
title: "Step 4 + 7: CLI Add Prompts & Test Update"
doc_type: proc
brief: "Status: SUCCESS"
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
---
# Step 4 + 7: CLI Add Prompts & Test Update

**Status**: SUCCESS
**Date**: 2026-03-03
**Tests**: 34 pass (unchanged count; test updated in place)

## What Changed

### RED Phase

Updated `cli_run_add_rejects_duplicate` test in `src/cli.rs`:
- Changed input from 3 fields (`name\ndesc\nmodel\ny`) to 5 fields (`name\ndesc\nbase_url\napi_key\nmodel\ny`)
- Added assertions verifying env var generation: `[profiles.env]`, `ANTHROPIC_BASE_URL`, `ANTHROPIC_MODEL = "MiniMax-M2.1"`
- Test failed: `run_add_with` only read 3 lines, so the extra input was misinterpreted (base_url consumed as model, api_key as confirm), resulting in only 1 profile instead of 2.

### GREEN Phase

Four changes to `src/cli.rs`:

1. **Added `mask_key` helper** (lines 6-12): Masks API keys for display. Keys <= 8 chars are fully masked with `*`. Longer keys show first 4 and last 4 chars with `...` in between.

2. **Added Base URL prompt** (lines 47-55): `Base URL (optional): ` prompt between Description and Model. Empty input maps to `None`.

3. **Added API Key prompt** (lines 57-65): `API Key (optional): ` prompt after Base URL. Empty input maps to `None`.

4. **Updated summary display** (lines 77-103): Now shows all 5 fields including masked API key via `mask_key`.

5. **Updated `NewProfile` construction** (lines 116-122): Changed `base_url: None, api_key: None` placeholders to `base_url, api_key` from the prompts.

### REFACTOR Phase

- No structural changes needed; code is clean.
- `cargo clippy` passes with no warnings.
- All 34 tests pass.

## Files Modified

- `src/cli.rs` — added `mask_key` helper, 2 new prompts, 5-field summary, real `NewProfile` construction, updated test input/assertions
