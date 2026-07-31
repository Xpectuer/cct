---
title: Step 6 — Main TUI Save Logic
doc_type: proc
brief: "Case: 6 — Update AddForm confirmation handler and Enter-to-advance threshold"
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
---
# Step 6 — Main TUI Save Logic

**Case**: 6 — Update AddForm confirmation handler and Enter-to-advance threshold
**Status**: SUCCESS
**Tests**: 35 pass (24 lib + 2 main + 5 integration + 4 live)

## RED Phase

No new failing test was written for this case. The TUI event loop in `main.rs` is not covered by unit tests (it requires terminal interaction). However, the existing code had a **field-index mismatch bug**:

- `form.fields[2]` was read as `model`, but after Case 5 expanded `FormState` to 5 fields, index 2 is actually **Base URL**.
- `base_url` and `api_key` were hardcoded to `None`, ignoring user input.
- The Enter-to-advance threshold `form.active_field < 2` only allowed advancing through 3 fields (Name, Description, Model), but there are now 5 fields.

These bugs would cause incorrect data to be saved and two fields to be unreachable via Enter key navigation.

## GREEN Phase

### Change 1 — Field reading in 'y' confirmation handler

Updated field index mapping from:
```rust
let desc = form.fields[1].trim().to_string();
let model = form.fields[2].trim().to_string();
// base_url: None, api_key: None
```

To:
```rust
let desc = form.fields[1].trim().to_string();
let base_url = form.fields[2].trim().to_string();
let api_key = form.fields[3].trim().to_string();
let model = form.fields[4].trim().to_string();
// base_url and api_key now use Some(value) when non-empty
```

### Change 2 — Enter-to-advance threshold

Updated from `form.active_field < 2` to `form.active_field < 4` so Enter advances through all 5 fields (indices 0-4) before triggering confirmation.

### Verification

- `cargo test` — 35 pass, 0 fail
- `cargo clippy` — clean, no warnings

## REFACTOR Phase

No refactoring needed. The code is clear and follows the same pattern used for `desc` and `model`. Clippy clean.

## Files Changed

| File | Change |
|------|--------|
| `src/main.rs` | Lines 110-128: read 5 fields with correct indices; Lines 163-164: threshold `< 2` to `< 4` |
