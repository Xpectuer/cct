---
title: "Spec: Inline profile edit pane for selected profiles"
doc_type: proc
brief: "Design spec for replacing [e] external config edit with inline profile editing"
confidence: verified
created: 2026-03-22
updated: 2026-03-22
revision: 1
---

# Spec: Inline profile edit pane for selected profiles

## Chosen Approach

Keep a single `AppMode::AddForm(FormState)` and extend `FormState` with edit metadata. Add one config-layer update function that surgically edits an existing `[[profiles]]` table with `toml_edit` while preserving unrelated fields, comments, and formatting.

## Architecture

The feature should remain a narrow extension of the existing add-form flow. `FormState` becomes the one form model for both create and edit. It needs two extra fields:

- `is_edit: bool`
- `original_name: Option<String>`

That keeps field navigation, backend-specific labels, confirmation handling, and error rendering in one place. `[a]` creates a blank form with `is_edit = false`. `[e]` preloads the selected profile into the same form and sets `is_edit = true`. UI copy should switch based on `is_edit`, but the keyboard model stays identical.

This is preferable to a separate edit mode because the current TUI already has a stable add-form path and only needs different entry and save semantics. The risk of scattered branching is contained by centralizing mode-specific decisions in a few helper methods rather than across the whole event loop.

## Components

### `src/app.rs`

Add edit metadata to `FormState` and helper constructors:

- `FormState::new_for_backend(backend)`
- `FormState::from_profile(profile: &Profile)`

`from_profile()` is the reverse mapping of `to_new_profile()`. It should fill fields using current backend semantics:

- Claude: `name`, `description`, `base_url`, API key from `ANTHROPIC_API_KEY`, `model`
- Codex: `name`, `base_url`, API key from `OPENAI_API_KEY`, `model`, `full_auto` as `y` or `n`

This keeps field-index mapping and prefill mapping close together instead of scattering them into `main.rs`.

### `src/config.rs`

Keep `append_profile()` for add flow. Add:

```rust
pub fn update_profile(original_name: &str, updated: &NewProfile) -> Result<()>
```

Implementation should:

1. Read `profiles.toml`
2. Parse as `toml_edit::DocumentMut`
3. Find the `[[profiles]]` entry whose `name` matches `original_name`
4. Update only the form-backed fields for that backend
5. Preserve untouched fields such as `extra_args`, custom env keys, and comments
6. Write the document back

For Claude, `description`, `model`, and `base_url` should be updated directly, and env changes should be applied surgically:

- set/remove `ANTHROPIC_BASE_URL` from the form base URL
- set/remove `ANTHROPIC_API_KEY` from the form API key
- set/remove model alias env vars based on the form model
- keep unknown env keys intact

For Codex, update `base_url`, `model`, `full_auto`, and `OPENAI_API_KEY`, again preserving unknown env keys.

### `src/main.rs`

Replace the current `[e]` external-editor path with inline edit entry:

- if there is no profile for the current tab, do nothing
- build `FormState::from_profile(&app.profiles[app.selected])`
- set `app.mode = AppMode::AddForm(form)`

On confirmation:

- add mode: validate duplicate name globally, then call `append_profile()`
- edit mode: allow unchanged name; if renamed, reject only when another profile already uses the target name; then call `update_profile(original_name, &new_profile)`

After a successful save, reload profiles, keep selection on the edited profile by its new name, and return to `Normal`.

### `src/ui.rs`

Change copy, not layout:

- detail title: `Add Profile` vs `Edit Profile`
- confirmation heading: `Save this profile?` vs `Save changes to this profile?`
- footer in normal mode: `[e] Edit`
- no more `Edit config` wording

## Data Flow

Entry flow:

1. User selects a profile in the active backend tab
2. User presses `[e]`
3. `main.rs` converts the selected `Profile` into `FormState`
4. `ui.rs` renders the existing form pane with prefilled values

Save flow:

1. User confirms
2. `main.rs` trims `fields[0]` and validates name presence
3. Add flow still uses existing duplicate-name behavior
4. Edit flow compares `original_name` and proposed `name`
5. `config::update_profile()` surgically edits the matching TOML table
6. `load_profiles()` refreshes in-memory state
7. `App` reselects the edited profile by final name

This keeps file persistence in `config.rs`, state decisions in `main.rs`, and display logic in `ui.rs`.

## Error Handling

- Empty name: show existing inline form error and stay in form mode
- Duplicate rename: show inline error and stay in form mode
- Missing original profile during update: surface `Save failed: ...` in form error
- Invalid TOML or write failure: same inline error path
- Profile reload failure after save: keep warning behavior consistent with current code, but successful save should still exit form mode only after reload succeeds

The key rule is that edit failures should never drop the user out of the form or silently append a new profile.

## Testing

### `src/app.rs`

- `from_profile()` pre-fills Claude fields correctly
- `from_profile()` pre-fills Codex fields correctly
- edit metadata is set correctly for edit entry

### `src/config.rs`

- editing Claude profile updates form-backed keys but preserves `extra_args`
- editing Claude profile preserves unknown env keys
- editing Codex profile updates `full_auto` and `OPENAI_API_KEY`
- renaming updates the `name` field in place rather than appending a second profile
- update fails when original profile is missing

### `src/ui.rs`

- panel title reflects add vs edit mode
- footer strings use `[e] Edit`
- confirmation copy reflects edit mode when `is_edit = true`

### `src/main.rs`

- save path dispatches to update logic in edit mode
- post-save reload reselects the renamed profile

## Decisions

- **Single form mode, extra metadata**: chosen for smallest safe patch
- **Dedicated update function**: required because append semantics cannot preserve untouched fields
- **No new editable fields**: first version only edits what the current form already models
- **No replacement external-editor key**: explicitly out of scope for this change

## Open Questions

None.
