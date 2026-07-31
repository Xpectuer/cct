---
title: Case 04
doc_type: proc
brief: Case 04
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
---
## Case 4 — main_c_key_launches_with_continue

### Actions Taken

1. **RED**: No unit test possible for the key handler in `run_tui` (requires terminal). Verified via grep that no `Char('c')` launch arm existed before the change.

2. **GREEN**: Added a `(KeyCode::Char('c'), _)` arm in `src/main.rs` between the Enter handler (line 63) and the `e` editor handler (line 69). The new arm:
   - Guards with `if !app.profiles.is_empty()` (same as Enter)
   - Calls `launch::restore_terminal()`
   - Calls `launch::exec_claude(&app.profiles[app.selected], true)` — passes `true` for `with_continue`
   - Falls through to error/exit on failure (same pattern as Enter)

3. **No conflict with Ctrl-C**: The existing `(KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL)` arm on line 57 matches Ctrl-C specifically. The new `(KeyCode::Char('c'), _)` arm on line 69 catches plain `c` only, since Ctrl-C is matched first.

4. **REFACTOR**: No changes needed.

### Verify Result

- `cargo test`: 44 tests passed (33 lib + 2 main + 5 integration + 4 live), 0 failures.
- `cargo clippy`: clean, no warnings.
- `grep Char('c') src/main.rs` shows both lines:
  - Line 57: `(KeyCode::Char('c'), KeyModifiers::CONTROL)` — Ctrl-C quit
  - Line 69: `(KeyCode::Char('c'), _) if !app.profiles.is_empty()` — new continue arm

**Result: SUCCESS**
