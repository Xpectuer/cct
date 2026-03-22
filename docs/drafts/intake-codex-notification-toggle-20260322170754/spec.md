---
title: "Spec: Codex notification toggle via [n] key"
doc_type: proc
brief: "Design spec for toggling Codex tui notifications with [n] key in TUI"
confidence: verified
created: 2026-03-22
updated: 2026-03-22
revision: 1
---

# Spec: Codex notification toggle via [n] key

## Chosen Approach

Store notification state on the Codex profile, persist it in `profiles.toml`, and always emit `[tui] notifications = true|false` into generated `CODEX_HOME/config.toml`. Keep the UI change limited to a new Codex-only `n` hotkey plus a footer hint.

## Alternatives Considered

### 1. Write only generated `CODEX_HOME/config.toml`

Rejected. `generate_codex_config()` rewrites the file on every launch, so direct edits would be transient and drift from the profile model.

### 2. Footer-discoverable profile-backed toggle

Chosen. This preserves the current architecture, follows the existing `skip_permissions` / `full_auto` toggle pattern, and complies with the repo rule that new hotkeys must be visible in the UI.

### 3. Expand scope to include `notification_method` / `notify`

Rejected for now. Those settings are valid Codex features but were explicitly excluded from this slice and would force more profile surface area and TUI form work than needed.

## Design

### config.rs — profile field and toggle function

Add a new optional boolean field on `Profile` and `NewProfile` for Codex notification state, named `notifications`. Validation remains backend-specific: Claude profiles must not use Codex-only fields, and Codex profiles may omit `notifications`, which defaults to `false` at read time via `unwrap_or(false)` in behavior paths.

Add `toggle_notifications(profile_name: &str, new_value: bool) -> Result<()>` using the same `toml_edit::DocumentMut` pattern as `toggle_skip_permissions()` and `toggle_full_auto()`. This function performs a targeted edit on the named `[[profiles]]` entry and preserves formatting/comments.

### main.rs — `n` key handler

In `AppMode::Normal`, add a new `KeyCode::Char('n')` branch guarded by `!app.profiles.is_empty()`. The handler should no-op for Claude profiles and toggle `profile.notifications` for Codex profiles:

1. Compute `old_val = profile.notifications.unwrap_or(false)`
2. Compute `new_val = !old_val`
3. Call `config::toggle_notifications(&profile.name, new_val)`
4. On success, update `profile.notifications = Some(new_val)`
5. On failure, print the same warning pattern used by existing toggles

### launch.rs — deterministic Codex config generation

Extend `generate_codex_config()` to always write:

```toml
[tui]
notifications = true
```

or

```toml
[tui]
notifications = false
```

The `[tui]` table should always be present for Codex-generated config so tests can assert exact emitted state. The function should continue to omit `notification_method` and `notify`.

### ui.rs — footer discoverability only

Update the Codex Normal-mode footer to advertise the new key, for example:

`[s] Full-auto  [n] Notifications`

Claude footer remains unchanged. No detail panel or add-form changes are required in this design.

## Data Flow

1. User selects a Codex profile in TUI
2. User presses `n`
3. `main.rs` dispatches to `config::toggle_notifications`
4. `profiles.toml` is updated in place via `toml_edit`
5. In-memory `app.profiles[selected].notifications` is updated on success
6. Later, on Enter, `exec_codex()` calls `generate_codex_config()`
7. Generated `config.toml` includes `[tui].notifications = <profile value>`

## Error Handling

- Missing profile in `profiles.toml` returns an error from `toggle_notifications`, matching existing toggle behavior
- A failed write leaves in-memory state unchanged because mutation happens only after `Ok(())`
- Claude profiles ignore `n`; this avoids accidental cross-backend behavior changes

## Testing

### config.rs

- `toggle_notifications_insert`
- `toggle_notifications_flip`
- `toggle_notifications_not_found`

### launch.rs

- Extend Codex config generation test to assert emitted `[tui]` table
- Assert `notifications = true` and `notifications = false` cases
- Assert generated config does not contain `notification_method` or `notify`

### ui.rs

- Update footer discoverability test so Codex footer includes `[n] Notifications`
- Keep Claude footer expectation unchanged

## Open Questions

None.
