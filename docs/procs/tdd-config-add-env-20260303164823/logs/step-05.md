---
title: "Step 5: UI Form Rendering — Confirmation Summary"
doc_type: proc
brief: Update build_form_lines confirmation summary to show all 5 fields with masked API key.
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
---
# Step 5: UI Form Rendering — Confirmation Summary

## Case
Update `build_form_lines` confirmation summary to show all 5 fields with masked API key.

## RED Phase
**Test**: `ui_confirmation_shows_five_fields` in `src/ui.rs`

Created a test that:
- Sets up `FormState` with `confirming: true` and populated fields (Name, Base URL, API Key, Model)
- Calls `build_form_lines(&form)` and joins the output lines
- Asserts all 5 field labels are present: Name, Description, Base URL, API Key, Model
- Asserts the API key is masked (`***`) and does NOT appear in cleartext
- Asserts Model shows the actual value (`kimi-k2`)
- Asserts empty Description shows `(none)`

**Failure**: Test failed with `Expected 'Base URL:' in confirmation` because the old code only had 3 lines and `fields[2]` (Base URL) was mislabelled as "Model:".

Output before fix:
```
  Name:        test-profile
  Description: (none)
  Model:       https://api.example.com
```

## GREEN Phase
**Change**: Updated the `if form.confirming` block in `build_form_lines` (`src/ui.rs` lines 149-181):

- `fields[0]` -> Name (unchanged)
- `fields[1]` -> Description (unchanged)
- `fields[2]` -> Base URL (NEW — was previously mislabelled as Model)
- `fields[3]` -> API Key (NEW — masked via `mask_value("API_KEY", ...)`)
- `fields[4]` -> Model (was `fields[2]`, now correctly at index 4)

Key design choice: Reused existing `mask_value` function for API key masking — the key "API_KEY" contains "KEY" which triggers masking to return `"***"`.

**Result**: All 35 tests pass (24 unit + 2 main + 5 integration + 4 live).

## REFACTOR Phase
No refactoring needed. Code is clean, clippy passes with no warnings.

## Files Modified
- `src/ui.rs`: Updated confirmation block (5 fields), added test `ui_confirmation_shows_five_fields`

## Test Count
- Before: 34 tests passing
- After: 35 tests passing (+1 new test)

## Status: SUCCESS
