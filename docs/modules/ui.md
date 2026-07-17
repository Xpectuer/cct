---
doc_type: module
module_name: "ui"
module_path: "src/ui.rs"
generated_by: mci-phase-2
revision: 4
updated: 2026-07-17
---

# ui Module Documentation

> **Purpose**: Renders the full terminal UI for `cct` using ratatui — a tab bar for backend switching, a 35/65 split filtered list and detail panel, a 1-line footer, and redacts sensitive environment variable values before display.
> **Path**: src/ui.rs

---

<!-- BEGIN:interface -->
## 1. Interface

### Exported Constants

- `const SENSITIVE: &[&str] = &["TOKEN", "KEY", "SECRET"]`
  - Private to the module; drives masking logic in `mask_value`.
  - Substring-matched case-insensitively against env var key names.

### Exported Functions

- `pub fn mask_value<'a>(key: &str, val: &'a str) -> &'a str`
  - Parameters: `key` — the environment variable name (any case); `val` — the original value (lifetime-tied to caller's string).
  - Returns: `"***"` (a `'static` str coerced to `'a`) if the uppercased `key` contains any of `TOKEN`, `KEY`, or `SECRET`; otherwise returns `val` unchanged.
  - No heap allocation on the masking path; the `"***"` literal satisfies the `'a` bound because the caller's lifetime is at least as long as `'static`.
  - Also used internally by `build_form_lines` to mask the API Key field in the confirmation summary.

- `pub fn draw(app: &App, frame: &mut Frame)`
  - The single entry point called each render tick from the event loop in `main`.
  - Internally performs four rendering passes in order:
    1. Tab bar (top 1 line) — `build_tab_bar()` renders `[Claude] [Codex] [Kimi]` tabs with the active backend highlighted.
    2. Profile list (left 35%) — `app.filtered_indices()` is used; only profiles matching `app.active_backend` are shown. Stateful `List` widget with blue highlight and `"> "` symbol.
    3. Detail panel (right 65%) — dispatched on `app.mode`:
       - `AppMode::Normal` → `build_detail` for the selected profile.
       - `AppMode::AddForm(form)` → `build_form_lines` for the inline add form.
    4. Footer (bottom 1 line) — key-binding hint in `DarkGray`; content reflects current mode and `form.confirming`. Includes `[Tab/1/2/3] Backend` hint.
  - Has no return value; all output goes through `frame.render_widget` / `frame.render_stateful_widget`.

### Private Functions (documented for maintainers)

- `pub fn build_tab_bar(active: &Backend) -> Vec<Line<'static>>`
  - Renders `[Claude] [Codex] [Kimi]` where the active backend is shown in bold/highlighted style.
  - Integrated into `draw()` at the top of the layout.

- `fn build_detail(profile: &Profile) -> Vec<Line<'static>>`
  - Constructs the detail panel text from a single `Profile`.
  - For Codex profiles: shows `approval` (derived from `full_auto`) and `auth: subscription` fields instead of Claude-specific fields.
  - For Kimi profiles: shows `max_context_size` — the explicit value (`1m` / `260k`), or `auto (1m)` / `auto (260k)` derived from the model when unset.
  - For Claude profiles with `auth_type = "token"`: shows `auth: token`.
  - Footer: Claude tab shows `[s] Skip-perms` / `[t] Auth` hints; Kimi tab shows a `[Space] Context` hint for the `max_context_size` toggle (no `[c] Resume`, `[s]`, or `[t]`); hints say `[Tab/1/2/3]`.
  - Returns owned `Vec<Line<'static>>`.

- `fn build_form_lines(form: &FormState) -> Vec<Line<'static>>`
  - Constructs the add-form panel content from `FormState`.
  - Uses `field_labels(&form.backend)` from `app` module to get backend-specific labels; this keeps label order and `to_new_profile()` field-index convention in sync.
  - **Edit view** (`form.confirming == false`): renders one line per field prefixing the active field with `"> "` (cyan + bold).
  - **Confirmation view** (`form.confirming == true`): renders a summary with the API Key masked via `mask_value`.
  - Appends a red error line if `form.error` is `Some`.
  - Returns owned `Vec<Line<'static>>`.
<!-- END:interface -->

---

<!-- BEGIN:dependency_graph -->
## 2. Dependency Graph

- **Imports from `crate::app`** — uses `App`, `AppMode`, `FormState`, `FIELD_LABELS`, `field_labels`, and `Backend`. `field_labels(&form.backend)` is called dynamically inside `build_form_lines` to select the correct label set per backend.
- **Imports from `crate::config`** — uses the `Profile` struct (fields: `name`, `description`, `model`, `skip_permissions`, `extra_args`, `env`) to build detail lines; `Profile` is passed by reference through `App`.
- **Imports from `ratatui`** (external crate, not a project module):
  - `layout::{Constraint, Direction, Layout}` — geometric splitting of the terminal area.
  - `style::{Color, Modifier, Style}` — color and bold styling for highlight, form active-field, and footer.
  - `text::Line` — the line type used in `build_detail` and `build_form_lines` return values.
  - `widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap}` — all rendered widget types.
  - `Frame` — the render target passed in from `main`.
- **Does NOT depend on**: `crate::config` directly for I/O (no file reads), `crate::launch` (no process exec), or any async runtime. The module performs zero I/O.
<!-- END:dependency_graph -->

---

<!-- BEGIN:state_management -->
## 3. State Management

**Type**: Stateless (pure render function)

The `ui` module owns no persistent state. Every call to `draw` constructs all intermediate data structures from scratch:

- `ListState` is allocated per frame (`ListState::default()`) and mutated only to set the selected index before being consumed by `render_stateful_widget`. It is dropped at the end of `draw`.
- `Vec<ListItem>` and `Vec<Line<'static>>` are built fresh each frame from the `&App` snapshot.
- `mask_value` is a pure function — it performs a substring scan and returns a pointer; no allocations or side effects.

All mutable state (selected profile index, profile list) lives in `App` in `main` and is passed in by shared reference. The UI layer never mutates `App`.
<!-- END:state_management -->

---

<!-- BEGIN:edge_cases -->
## 4. Edge Cases

### Empty Profile List

When `app.profiles.is_empty()` is `true`:
- The list panel renders a single `ListItem` with the text `"No profiles. Press 'e' to edit config."`.
- `list_state.select(...)` is skipped entirely, so no item is highlighted.
- The detail panel renders `"Select a profile to see details."` instead of calling `build_detail`.
- This guards against an out-of-bounds index on `app.profiles[app.selected]`.

### Sensitive Value Masking

`mask_value` uses substring matching (not exact match) on the uppercased key:
- `"ANTHROPIC_AUTH_TOKEN"` → contains `"TOKEN"` → masked.
- `"OPENAI_API_KEY"` → contains `"KEY"` → masked.
- `"MY_SECRET"` → contains `"SECRET"` → masked.
- `"ANTHROPIC_BASE_URL"` → none of the three substrings present → value is returned as-is.
- Mixed-case keys like `"Api_Key"` are normalized via `.to_uppercase()` before matching, so masking is case-insensitive.

### ENV Key Sort Order

In `build_detail`, env entries are sorted alphabetically by key (`pairs.sort_by_key(|(k, _)| k.as_str())`) before rendering. This ensures deterministic display order regardless of `HashMap` iteration order, which is randomized by Rust's default hasher (SipHash with a random seed).

### Layout Split Percentages

The horizontal split uses `Percentage(35)` and `Percentage(65)`. Ratatui distributes any remainder pixel (from odd terminal widths) to the last constraint. The detail panel may receive one extra column in those cases — this is handled transparently by ratatui.

### List Item Two-Line Format

When a profile has a `description`, its `ListItem` is formatted as `"{name}\n  {description}"`, producing a two-line item. The `List` widget does not enforce fixed item height, so highlighted items will span two rows correctly, but the overall list height is consumed faster when descriptions are present.

### skip_permissions Display

`skip_permissions` is only rendered when explicitly set to `true`. A value of `false` or `None` produces no output in the detail panel — absence of the line means "not set / false".
<!-- END:edge_cases -->

---

<!-- BEGIN:usage_example -->
## 5. Usage Example

```rust
// Typical call site in main.rs — inside the crossterm/ratatui event loop:

use cct::app::App;
use cct::config::load_profiles;
use cct::ui;

fn main() -> anyhow::Result<()> {
    let profiles = load_profiles()?;
    let mut app = App::new(profiles);

    // ratatui Terminal<CrosstermBackend<Stdout>>
    let mut terminal = /* ... setup crossterm backend ... */;

    loop {
        // draw() is called once per event-loop tick
        terminal.draw(|frame| {
            ui::draw(&app, frame);
        })?;

        // Handle keyboard input — ui module is not involved here
        match read_key_event()? {
            KeyCode::Down | KeyCode::Char('j') => app.next(),
            KeyCode::Up   | KeyCode::Char('k') => app.prev(),
            KeyCode::Enter => {
                // ui module is done; launch module takes over
                launch::exec_claude(&app.profiles[app.selected])?;
            }
            KeyCode::Char('q') => break,
            _ => {}
        }
    }
    Ok(())
}

// Standalone masking utility — can be used in tests or other display contexts:
use cct::ui::mask_value;

let display = mask_value("ANTHROPIC_AUTH_TOKEN", "sk-ant-abc123");
assert_eq!(display, "***");

let display = mask_value("ANTHROPIC_BASE_URL", "https://api.anthropic.com");
assert_eq!(display, "https://api.anthropic.com");
```
<!-- END:usage_example -->

---

## Quality Gate Checklist

- [x] **Interface**: 4 public/private interface points documented (SENSITIVE const, mask_value, draw, build_form_lines) plus private build_detail for maintainers
- [x] **Dependencies**: All internal module dependencies listed with field-level reasoning; ratatui imports enumerated; new `AppMode`, `FormState`, `FIELD_LABELS` imports documented
- [x] **State Management**: Clearly stateless; ListState lifecycle explained per-frame
- [x] **Edge Cases**: 6 special cases identified — empty profiles, masking substring logic, case-insensitivity, env sort order, layout split remainder, skip_permissions omission behavior
- [x] **Usage Example**: Rust pseudocode shows realistic event-loop integration and standalone mask_value usage
- [x] **YAML Frontmatter**: doc_type, module_name, module_path present

---

**Template Version**: 2.0
**Last Updated**: 2026-07-17 (revision 4 — Kimi backend: 3-tab bar, `[Tab/1/2/3]` hints, Kimi footer with `[Space] Context`, `max_context_size` in detail panel)
