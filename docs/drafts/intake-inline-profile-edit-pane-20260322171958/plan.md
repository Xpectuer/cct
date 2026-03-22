---
title: "Plan: Inline profile edit pane for selected profiles"
doc_type: proc
brief: "Extend AddForm with edit metadata, add update_profile persistence, and replace [e] with inline edit"
confidence: verified
created: 2026-03-22
updated: 2026-03-22
revision: 1
---

# Plan: Inline profile edit pane for selected profiles

## Files Changed

| File | Change Type |
|------|-------------|
| `src/app.rs` | Moderate edit — extend `FormState`, add prefill helpers, update tests |
| `src/config.rs` | Moderate edit — add `update_profile()` and persistence tests |
| `src/main.rs` | Moderate edit — change `[e]` entry path and add edit-mode save logic |
| `src/ui.rs` | Minor edit — adjust titles, confirmation text, and footer copy |
| `README.md` | Minor edit — update hot-reload/edit keybinding docs |

## Step 1 — Extend `FormState` for edit metadata and prefill helpers

**File**: `src/app.rs`

**What**:
- add `is_edit: bool`
- add `original_name: Option<String>`
- add `FormState::new_for_backend(backend: Backend) -> Self`
- add `FormState::from_profile(profile: &Profile) -> Self`

`from_profile()` should reverse current backend form semantics and populate:

- Claude: name, description, base_url, `ANTHROPIC_API_KEY`, model
- Codex: name, base_url, `OPENAI_API_KEY`, model, `y`/`n` for `full_auto`

**Verify**:
- `cargo test from_profile -- --test-threads=1`

## Step 2 — Add config-layer update support for existing profiles

**File**: `src/config.rs`

**What**:
- add `update_profile(original_name: &str, updated: &NewProfile) -> Result<()>`
- use `toml_edit::DocumentMut` to find the matching `[[profiles]]` table
- update in place rather than rebuilding the file
- preserve untouched fields including `extra_args` and unknown env keys

**Implementation rules**:
- update `name` directly
- for Claude:
  - update/remove `description`, `model`, `base_url`
  - update/remove known Anthropic env keys derived from base URL, API key, and model
- for Codex:
  - update/remove `base_url`, `model`, `full_auto`
  - update/remove `OPENAI_API_KEY`
- keep unknown env entries and comments intact

**Verify**:
- `cargo test update_profile -- --test-threads=1`

## Step 3 — Replace `[e]` external editor path with inline edit entry

**File**: `src/main.rs`

**What**:
- remove the `launch::open_editor(config::config_path())` behavior from `[e]`
- when a profile exists, build `FormState::from_profile(&app.profiles[app.selected])`
- enter `AppMode::AddForm(form)`

**Verify**:
- code search should show no `open_editor` call from the `[e]` branch

## Step 4 — Split add-mode and edit-mode save behavior

**File**: `src/main.rs`

**What**:
- keep the current add flow for `!form.is_edit`
- for `form.is_edit`:
  - require non-empty name
  - permit unchanged name
  - reject rename only if another profile already uses the target name
  - call `config::update_profile(original_name, &form.to_new_profile())`
- after successful save, reload profiles and select the updated profile by final name

**Verify**:
- add tests for duplicate validation logic if practical
- manual smoke via `cargo test`

## Step 5 — Update UI copy for inline edit mode

**File**: `src/ui.rs`

**What**:
- form panel title becomes `Add Profile` or `Edit Profile`
- confirmation copy becomes add-specific or edit-specific
- normal footer uses `[e] Edit`
- no remaining `Edit config` text in UI

**Verify**:
- `cargo test ui_ -- --test-threads=1`
- `rg -n "Edit config" src README.md` should return nothing after docs update

## Step 6 — Update README to match the new interaction

**File**: `README.md`

**What**:
- remove the “press `e` to open `$EDITOR`” hot-reload description
- document `[e]` as editing the selected profile inline
- adjust keybinding table and quick-start text

**Verify**:
- `rg -n "Edit config|\\$EDITOR|hot-reload" README.md`

## Step 7 — Add focused tests for preservation and rename behavior

**Files**: `src/app.rs`, `src/config.rs`, `src/ui.rs`

**What**:
- `from_profile_claude_prefills_fields`
- `from_profile_codex_prefills_fields`
- `update_profile_preserves_extra_args`
- `update_profile_preserves_unknown_env_keys`
- `update_profile_renames_in_place`
- `update_profile_missing_original_errors`
- `ui_form_title_and_confirmation_reflect_edit_mode`

**Verify**:
- `cargo test -- --test-threads=1`

## Step 8 — Final verification

Run:

```bash
cargo test -- --test-threads=1
cargo clippy
```

Manual checks if needed:

- Claude tab: press `e`, edit and save
- Codex tab: press `e`, edit and save
- rename to an existing profile name should fail inline
- rename to a new name should preserve selection on saved profile

## Execution Order

1. Step 1
2. Step 2
3. Step 3
4. Step 4
5. Step 5
6. Step 6
7. Step 7
8. Step 8

## Commit Message

`feat: edit selected profiles inline with e`
