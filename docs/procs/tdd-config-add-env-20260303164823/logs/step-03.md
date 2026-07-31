---
title: "Step 3 — Case 3: FormState 5-field expansion"
doc_type: proc
brief: "Status: SUCCESS"
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
---
# Step 3 — Case 3: FormState 5-field expansion

**Status**: SUCCESS
**Date**: 2026-03-03
**File**: `src/app.rs` (primary), `src/ui.rs` (test update)

## Summary

Expanded `FormState` from 3 fields (Name, Description, Model) to 5 fields (Name, Description, Base URL, API Key, Model) to support env var input in the TUI add-profile form.

## RED Phase

Added test `form_state_five_fields` in `src/app.rs` asserting:
- `FIELD_LABELS.len() == 5`
- `FIELD_LABELS` contains `["Name *", "Description", "Base URL", "API Key", "Model"]`
- `FormState::new().fields.len() == 5`
- `next_field()` clamps at index 4

**Result**: FAILED — `FIELD_LABELS.len()` was 3, expected 5.

## GREEN Phase

Four changes in `src/app.rs`:
1. `FIELD_LABELS`: `[&str; 3]` with 3 labels -> `[&str; 5]` with `["Name *", "Description", "Base URL", "API Key", "Model"]`
2. `FormState.fields`: `[String; 3]` -> `[String; 5]`
3. `FormState::new()`: 3 `String::new()` -> 5 `String::new()`
4. `next_field()`: `.min(2)` -> `.min(4)`

Two test updates required for GREEN:
- `src/app.rs` `form_state_field_navigation`: Updated to navigate through all 5 fields (0->1->2->3->4) and clamp at 4 instead of 2.
- `src/ui.rs` `ui_renders_add_form`: Changed `assert_eq!(lines.len(), 3)` to `assert_eq!(lines.len(), 5)` since form now renders 5 field lines.

**Result**: All 34 tests pass.

## REFACTOR Phase

No refactoring needed. Code is clean, `cargo clippy` reports zero warnings.

**Result**: All 34 tests pass.

## Known Side Effects (deferred to later cases)

- `src/ui.rs` `build_form_lines` confirmation summary (lines 149-165): Still references `form.fields[0]` (Name), `form.fields[1]` (Description), `form.fields[2]` (now Base URL, was Model). The confirmation display will show "Model: (base_url_value)" — this is expected and will be fixed in **Case 5** (UI form rendering).
- `src/main.rs` lines 110-111: Reads `form.fields[1]` (desc) and `form.fields[2]` (now Base URL, was Model) for the save logic. Will be fixed in **Case 6** (Main TUI save logic).
- `src/main.rs` line 154: `if form.active_field < 2` Enter-advances logic still uses old max — will be updated in **Case 6**.

## Test Count

| Suite | Count | Status |
|-------|-------|--------|
| lib (unit) | 23 | PASS |
| main (unit) | 2 | PASS |
| integration | 5 | PASS |
| live | 4 | PASS |
| **Total** | **34** | **ALL PASS** |
