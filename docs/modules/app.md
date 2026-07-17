---
doc_type: module
module_name: "app"
module_path: "src/app.rs"
generated_by: mci-phase-2
revision: 3
updated: 2026-03-15
---

# app Module Documentation

> **Purpose**: Owns the mutable cursor state (selected profile index), provides backend-filtered circular navigation, manages TUI mode transitions, and contains `FormState` with `to_new_profile()` as the single source of truth for form-field-to-semantic mapping.
> **Path**: src/app.rs

---

<!-- BEGIN:interface -->
## 1. Interface

### Exported Constants

- `pub const FIELD_LABELS: [&str; 6]` — Claude-specific ordered labels for the 6 add-form fields. Retained for backward compatibility; prefer `field_labels(backend)` for backend-aware rendering.

### Exported Free Functions

- `pub fn field_labels(backend: &Backend) -> [&'static str; 6]`
  - Returns backend-specific field label arrays for the 6-slot add form:
    - `Backend::Claude` → `["Name *", "Description", "Base URL", "API Key", "Pro Model", "Fast Model"]`
    - `Backend::Codex` → `["Name *", "Base URL", "API Key", "Model", "Full Auto (y/n)", ""]`
    - `Backend::Kimi` → `["Name *", "Description", "Base URL", "API Key", "Model", "Context (1m/260k)"]`
  - Used by `ui::build_form_lines` and internally by `FormState::to_new_profile` to keep label order and field mapping in sync.

### Exported Enums

- `pub enum AppMode` — discriminates between the two runtime UI modes:
  - `Normal` — standard profile-list view; navigation and launch are active.
  - `AddForm(FormState)` — inline profile add form is visible; keyboard input goes to the form.

### Exported Structs

- `pub struct FormState` — holds the transient state of the inline add form:
  - `pub fields: [String; 6]` — one string buffer per field. The semantic meaning of each index depends on `backend`:
    - Claude: `[0]=Name, [1]=Description, [2]=Base URL, [3]=API Key, [4]=Pro Model, [5]=Fast Model`
    - Codex: `[0]=Name, [1]=Base URL, [2]=API Key, [3]=Model, [4]=Full Auto (y/n), [5]=""`
    - Kimi: `[0]=Name, [1]=Description, [2]=Base URL, [3]=API Key, [4]=Model, [5]=Context (max_context_size)`
  - `pub active_field: usize` — index of the currently focused field (0–5); clamped by `next_field`/`prev_field`.
  - `pub confirming: bool` — when `true`, the form shows the confirmation summary view.
  - `pub error: Option<String>` — inline validation error displayed below the form.
  - `pub backend: Backend` — determines which field layout is in use.
  - `pub auth_type: Option<String>` — persisted auth type; `"token"` means `ANTHROPIC_AUTH_TOKEN`. Set from `Profile.auth_type` in `from_profile`, passed to `NewProfile` in `to_new_profile`.
  - `pub is_edit: bool` — `true` when editing an existing profile, `false` when adding new.
  - `pub original_name: Option<String>` — original profile name for rename detection during edit.

- `pub struct App` — sole owner of all runtime TUI state.

### App Fields

- `pub profiles: Vec<Profile>` — full unfiltered profile list loaded from disk; may be replaced on hot-reload.
- `pub selected: usize` — logical cursor within the **filtered** profile list for `active_backend`.
- `pub mode: AppMode` — current UI mode; `Normal` on construction.
- `pub active_backend: Backend` — which backend tab is currently visible; `Backend::Claude` on construction.

### App Methods

- `App::new(profiles: Vec<Profile>) -> Self` — constructs with `selected = 0`, `mode = AppMode::Normal`, `active_backend = Backend::Claude`.
- `app.filtered_indices(&self) -> Vec<usize>` — returns the indices (into `self.profiles`) of all profiles whose `backend == self.active_backend`. Used by `ui` for list rendering and by `next`/`prev` for filtered navigation.
- `app.switch_backend(&mut self, backend: Backend)` — sets `active_backend` and resets `selected` to `0` so the cursor lands on the first profile in the new backend's subset.
- `app.next(&mut self)` — advances `selected` within `filtered_indices()`, wrapping circularly; no-op when the filtered list is empty.
- `app.prev(&mut self)` — retreats `selected` within `filtered_indices()`, wrapping circularly; no-op when the filtered list is empty.

### FormState Methods

- `FormState::new() -> Self` — constructs with all fields empty, `active_field = 0`, `confirming = false`, `error = None`, `backend = Backend::Claude`.
- `form.next_field(&mut self)` — advances `active_field` by one, clamped at `4`.
- `form.prev_field(&mut self)` — retreats `active_field` by one, clamped at `0` via `saturating_sub`.
- `form.to_new_profile(&self) -> NewProfile` — **single source of truth** for form-field-to-semantic mapping. Reads `self.fields` according to `self.backend`'s index convention and constructs a `NewProfile`. Codex path additionally parses `fields[4]` as `"y"/"yes"` → `full_auto = true`. Kimi path maps `fields[5]` (trimmed, empty → `None`) to `max_context_size`, and leaves `fast_model`/`full_auto`/`auth_type` unset.
<!-- END:interface -->

---

<!-- BEGIN:dependency_graph -->
## 2. Dependency Graph

- **Imports from `crate::config`** → `Backend` enum, `NewProfile`, and `Profile` struct. The `app` module now imports `Backend` to type `active_backend` and `FormState.backend`, and imports `NewProfile` for `to_new_profile()`.
- **No `std` imports beyond language primitives** — no I/O, threading, or collections beyond `Vec`.
- **No external crates** — zero third-party dependencies.
- **Does NOT depend on**: `ui`, `launch`, `config::load_profiles`, or any OS APIs.

**Quality Check**: Single internal dependency clearly stated; absence of external dependencies confirmed.
<!-- END:dependency_graph -->

---

<!-- BEGIN:state_management -->
## 3. State Management

**Type**: Stateful — `App` is the single mutable owner of all TUI runtime state.

- **`selected` field** — mutated in-place by `next()` and `prev()` on every keypress within the filtered subset. Its lifecycle begins at `0` and ends when the process is exec-replaced or exits.
- **`active_backend` field** — set by `switch_backend()`; drives which profiles are visible in `filtered_indices()`. Switching backend resets `selected` to `0`.
- **`profiles` field** — unfiltered; may be replaced entirely on hot-reload. `main.rs` clamps `selected` to the new filtered length after replacement.
- **`mode` field** — transitions between `AppMode::Normal` and `AppMode::AddForm(FormState)`. When `AddForm` is created, it inherits `active_backend` so the form renders the correct field labels. When saved or cancelled, `mode` returns to `Normal` and `FormState` is dropped.
- **`FormState.fields`** — five `String` buffers edited in-place. Semantic meaning of each index is determined by `FormState.backend` and enforced by `to_new_profile()`.
- **No interior mutability** — no `Mutex`, `RefCell`, or `Arc`; single `&mut App` driven by the event loop.

**Quality Check**: State lifecycle, mutation points, and ownership model fully documented.
<!-- END:state_management -->

---

<!-- BEGIN:edge_cases -->
## 4. Edge Cases

### Empty Profile List

Both `next()` and `prev()` open with an `if !self.profiles.is_empty()` guard and return immediately without touching `selected`. This prevents a panic from `% 0` in `next()` and from underflowing `usize` in `prev()`. The UI and launch code also guard on `!app.profiles.is_empty()` before accessing `app.profiles[app.selected]`.

### Cursor Out-of-Bounds After Hot-Reload

`App` itself does not clamp `selected` when `profiles` is replaced. The caller (`main.rs`, lines 50-51) performs the clamp:

```rust
if app.selected >= app.profiles.len() {
    app.selected = app.profiles.len().saturating_sub(1);
}
```

`saturating_sub(1)` on a zero-length vec yields `0`, so after a reload that empties the list, `selected` lands at `0` and the `is_empty` guards in `next`/`prev` keep it safe.

### Single-Profile List

When `profiles.len() == 1`, both `next()` and `prev()` leave `selected` at `0` — `(0 + 1) % 1 == 0` for next, and the `else` branch of prev computes `selected -= 1` which would underflow, but the `selected == 0` arm is taken first and wraps to `len - 1 == 0`. Navigation appears to do nothing visually, which is the correct behaviour.

### `selected` Type Is `usize`

`selected` cannot be negative. The `prev()` implementation avoids underflow by checking `selected == 0` before subtracting. Any future refactor that changes this check must preserve that guard.

**Quality Check**: 4 edge cases identified with specific code references.
<!-- END:edge_cases -->

---

<!-- BEGIN:usage_example -->
## 5. Usage Example

```rust
// src/main.rs — how the event loop uses App

use cct::{app::App, config, launch, ui};

fn main() -> anyhow::Result<()> {
    // 1. Load profiles from disk once at startup
    let profiles = config::load_profiles()?;

    // 2. Construct App — selected starts at 0
    let mut app = App::new(profiles);

    loop {
        // 3. Pass an immutable reference to the renderer
        tui.draw(|f| ui::draw(&app, f))?;

        match event::read()? {
            // 4. Navigate down (wraps at end of list)
            KeyCode::Down | KeyCode::Char('j') => app.next(),

            // 5. Navigate up (wraps at beginning of list)
            KeyCode::Up | KeyCode::Char('k') => app.prev(),

            // 6. Launch — profiles[selected] is always a valid index here
            KeyCode::Enter if !app.profiles.is_empty() => {
                launch::exec_claude(&app.profiles[app.selected]);
            }

            // 7. Hot-reload: replace profiles, then clamp cursor
            KeyCode::Char('e') => {
                launch::open_editor(&config::config_path())?;
                if let Ok(updated) = config::load_profiles() {
                    app.profiles = updated;
                    // Clamp — App does not do this internally
                    if app.selected >= app.profiles.len() {
                        app.selected = app.profiles.len().saturating_sub(1);
                    }
                }
            }

            _ => {}
        }
    }
}
```

**Quality Check**: Example covers construction, both navigation methods, field access for launch, and the hot-reload cursor-clamp pattern.
<!-- END:usage_example -->

---

## Quality Gate Checklist

- [x] **Interface**: 10 public interface points documented (constant, enum, 2 structs, 5 methods)
- [x] **Dependencies**: Single internal dependency (`crate::config::Profile`) listed with reasoning; no external crates
- [x] **State Management**: Stateful; mutation points, ownership, mode transitions, and hot-reload lifecycle documented
- [x] **Edge Cases**: 4 special cases identified (empty list, out-of-bounds after reload, single-item list, usize underflow guard)
- [x] **Usage Example**: Rust pseudocode mirrors actual `main.rs` patterns with annotations
- [x] **YAML Frontmatter**: `doc_type`, `module_name`, `module_path` present

---

**Template Version**: 2.0
**Last Updated**: 2026-03-15 (revision 3 — added Backend, active_backend, filtered_indices, switch_backend, field_labels, FormState.backend, FormState.to_new_profile)
