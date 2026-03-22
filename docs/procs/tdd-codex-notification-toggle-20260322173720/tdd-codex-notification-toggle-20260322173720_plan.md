---
title: "Plan: Codex notification toggle via [n] key"
doc_type: proc
brief: "Add profile-backed Codex notifications toggle, generated tui config, and footer hint"
confidence: verified
created: 2026-03-22
updated: 2026-03-22
revision: 1
---

# Plan: Codex notification toggle via [n] key

## Files Changed

| File | Change Type |
|------|-------------|
| `src/config.rs` | Minor edit — add `notifications` field wiring and `toggle_notifications()` plus unit tests |
| `src/main.rs` | Minor edit — add Codex-only `n` key handler |
| `src/launch.rs` | Minor edit — emit `[tui].notifications` in generated Codex config plus tests |
| `src/ui.rs` | Minor edit — advertise `n` in Codex footer and update discoverability tests |

## Step 1 — Add profile field and toggle helper in `src/config.rs`

**What**

- Add `pub notifications: Option<bool>` to `Profile`
- Add `pub notifications: Option<bool>` to `NewProfile`
- Keep add-form behavior unchanged by setting `notifications: None` in current constructors
- Add `toggle_notifications(profile_name: &str, new_value: bool) -> Result<()>`

**Implementation Notes**

- Mirror `toggle_full_auto()` exactly, but target `entry["notifications"]`
- Preserve comments/formatting via `toml_edit`

**Verify**

- `rg "pub notifications: Option<bool>" src/config.rs` returns 2 matches
- `rg "fn toggle_notifications" src/config.rs` returns 1 match

## Step 2 — Add config unit tests in `src/config.rs`

**What**

Add tests covering:

- field insertion when `notifications` is absent
- flip `true -> false -> true`
- missing profile error

**Implementation Notes**

- Reuse the same temporary-config pattern used by existing toggle tests
- Verify comment preservation in at least one test

**Verify**

- `cargo test toggle_notifications -- --test-threads=1`

## Step 3 — Add Normal-mode `n` handler in `src/main.rs`

**What**

Add a new branch for `KeyCode::Char('n')`:

- Claude: no-op
- Codex: toggle `notifications`

**Implementation Notes**

- Follow the same warning-on-error pattern used for `s`
- Only mutate in-memory profile state after successful persistence

**Verify**

- `rg "Char\\('n'\\)" src/main.rs`
- Code read confirms Claude path does not change behavior

## Step 4 — Emit `[tui].notifications` in `src/launch.rs`

**What**

Extend `generate_codex_config()` so generated config always includes:

```toml
[tui]
notifications = true|false
```

**Implementation Notes**

- Use `profile.notifications.unwrap_or(false)`
- Continue writing `model_provider`, `model`, and `[model_providers.custom]` unchanged
- Do not write `notification_method` or `notify`

**Verify**

- Unit tests assert exact generated content for both boolean states

## Step 5 — Update discoverability in `src/ui.rs`

**What**

Update Codex footer text to include `[n] Notifications`.

**Implementation Notes**

- Claude footer string remains unchanged
- Codex footer becomes the only visible affordance required by project rule

**Verify**

- Footer test checks Codex string contains `[n] Notifications`
- Footer test keeps Claude expectations intact

## Step 6 — Proof-read and run focused tests

**What**

Run targeted tests for touched modules, then a full test pass if the focused set succeeds.

**Verify**

- `cargo test toggle_notifications -- --test-threads=1`
- `cargo test generate_codex_config`
- `cargo test ui_footer_shows_add_hint`
- `cargo test`

## Step 7 — Cross-check acceptance criteria

| Criterion | Addressed in Step |
|-----------|-------------------|
| Codex profile stores notification bool in `profiles.toml` | 1, 2 |
| Codex `n` hotkey toggles and persists value | 3 |
| Reload path reads the value back correctly | 2 |
| Generated Codex config always includes `[tui].notifications` | 4 |
| Claude behavior remains unchanged | 3, 5 |
| `notification_method` / `notify` are not emitted | 4 |
| Tests cover new behavior and discoverability | 2, 4, 5, 6 |

## Execution Order

1. Step 1
2. Step 2
3. Step 3
4. Step 4
5. Step 5
6. Step 6
7. Step 7

## Commit Message

`feat: press [n] to toggle Codex notifications`
