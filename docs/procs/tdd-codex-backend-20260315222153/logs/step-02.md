---
title: "Step 2: Filtered navigation in app.rs"
doc_type: proc
brief: "Status: SUCCESS"
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
---
# Step 2: Filtered navigation in app.rs

**Status**: SUCCESS
**Date**: 2026-03-15

## Changes Made

### `src/app.rs`
- Added `use crate::config::{Backend, Profile}` (was only `Profile`)
- Added `pub fn field_labels(backend: &Backend) -> [&'static str; 5]` returning backend-specific labels
  - Claude: `["Name *", "Description", "Base URL", "API Key", "Model"]`
  - Codex: `["Name *", "Base URL", "API Key", "Model", "Full Auto (y/n)"]`
- Kept `FIELD_LABELS` constant for backward compat (ui.rs still references it; will be updated in Step 3)
- Added `backend: Backend` field to `FormState` (defaults to `Backend::Claude` in `new()`)
- Added `active_backend: Backend` field to `App` (defaults to `Backend::Claude` in `new()`)
- Added `App::filtered_indices(&self) -> Vec<usize>` - returns indices matching `active_backend`
- Added `App::switch_backend(&mut self, backend: Backend)` - sets backend + resets `selected`
- Rewrote `App::next()` and `App::prev()` to navigate only within filtered indices (circular)

## Test Cases (4/4 passing)

| # | Test | Result |
|---|------|--------|
| 6 | `filtered_indices_returns_correct_backend_subset` | PASS |
| 7 | `switch_backend_resets_selected_to_first_matching` | PASS |
| 8 | `next_prev_navigate_within_filtered_backend` | PASS |
| 9 | `field_labels_returns_backend_specific_labels` | PASS |

## Final Test Run

All 53 tests passing (42 lib + 2 main + 5 integration + 4 live).
