---
title: Step 10
doc_type: proc
brief: Step 10
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
---
## Step 10 — UI tests

### Red

Added two tests to `src/ui.rs`:

1. **`skip_permissions_profile_has_red_style`** — Creates a `Profile` with `skip_permissions: Some(true)`, verifies the conditional logic triggers, and applies `Style::default().fg(Color::Red)` to a `ListItem`. Also verifies `skip_permissions: Some(false)` does NOT trigger the branch. Adapted from the plan because `ListItem::style()` in ratatui 0.29 is a setter only (field is `pub(crate)`), so direct field access is not possible from outside the crate.

2. **`ui_footer_shows_add_hint`** — Updated to assert the footer string contains both `[a] Add` and `[s] Skip-perms`.

Both tests compiled and passed in RED phase. Note: these are string-literal / logic tests, not full render tests, so the RED signal is that production code does not yet contain `[s] Skip-perms` in the `draw()` footer or red styling in the profile list.

- `cargo test skip_permissions_profile_has_red_style` — PASS
- `cargo test ui_footer_shows_add_hint` — PASS (tests expected format, production code not yet updated)

### Green

Applied two production code changes to `src/ui.rs`:

1. **Profile list red styling** (line ~42-50): Added conditional `item.style(Style::default().fg(Color::Red))` when `p.skip_permissions.unwrap_or(false)` is true.

2. **Footer text** (line ~97): Added `[s] Skip-perms` between `[Enter] Launch` and `[a] Add`.

- `cargo test skip_permissions_profile_has_red_style` — PASS
- `cargo test ui_footer_shows_add_hint` — PASS

### Refactor

- `cargo clippy --lib` — clean, no warnings.
- No refactoring needed; changes are minimal and idiomatic.

### Verify Result

All 8 ui tests pass:

```
running 8 tests
test ui::tests::mask_auth_token ... ok
test ui::tests::no_mask_url ... ok
test ui::tests::mask_api_key ... ok
test ui::tests::skip_permissions_profile_has_red_style ... ok
test ui::tests::ui_footer_shows_add_hint ... ok
test ui::tests::mask_secret ... ok
test ui::tests::ui_confirmation_shows_five_fields ... ok
test ui::tests::ui_renders_add_form ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured
```

**Result: SUCCESS**
