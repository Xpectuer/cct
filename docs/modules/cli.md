---
doc_type: module
module_name: "cli"
module_path: "src/cli.rs"
generated_by: mci-phase-2
revision: 3
updated: 2026-07-17
---

# cli Module Documentation

> **Purpose**: Implements the `cct add` and `cct edit` subcommands. `cct add` runs an interactive 6-prompt CLI flow that collects profile fields, shows a masked summary, and calls `config::append_profile` on user confirmation; `--backend claude|codex|kimi` selects the target backend (default claude). `cct edit` directly opens `profiles.toml` in `$EDITOR` (dispatched from `main.rs` via `launch::open_editor`).
> **Path**: src/cli.rs (add); main.rs — `Some(Commands::Edit)` arm (edit)

---

<!-- BEGIN:interface -->
## 1. Interface

### Exported Functions

- `pub fn run_add(auth_type: Option<String>, backend: Option<String>) -> Result<()>`
  - Entry point for the `cct add` subcommand; called from `main` when `Commands::Add { auth_type, backend }` is matched.
  - Accepts `--auth-type token` flag to use `ANTHROPIC_AUTH_TOKEN` instead of `ANTHROPIC_API_KEY`.
  - Accepts `--backend claude|codex|kimi` flag (case-insensitive, default claude) via `resolve_backend`.
  - Delegates to `run_add_with(io::stdin().lock(), io::stdout(), auth_type, backend)`.
  - Returns: `anyhow::Result<()>`.

- `pub fn run_add_with<R: BufRead, W: Write>(reader: R, writer: W, auth_type: Option<String>, backend: config::Backend) -> Result<()>`
  - Testable, I/O-generic version of the add flow.
  - Accepts any `BufRead` reader and `Write` writer, enabling unit tests to inject fake stdin/stdout without touching real file descriptors.
  - **6-prompt sequence** (in order):
    1. **Name** (required, loops until non-empty; exits with code 1 if duplicate detected)
    2. **Description** (optional — empty input → `None`)
    3. **Base URL** (optional)
    4. **API Key** (optional)
    5. **Model** (optional)
    6. **Fast Model** (optional, for Haiku/SmallFast tier)
  - After prompts: prints a summary table with the API key masked via `mask_key`.
  - Prompts `"Save? (y/n): "` — any response other than `"y"` (case-insensitive) prints `"Cancelled."` and returns `Ok(())`.
  - On confirmation: calls `config::append_profile(&NewProfile { ..., auth_type, backend, max_context_size: None })` then prints `"Profile '<name>' added."`. For Kimi profiles this writes `backend = "kimi"` plus an env block with only `ANTHROPIC_BASE_URL` / `ANTHROPIC_API_KEY` / `ANTHROPIC_MODEL`; `max_context_size` stays auto (TUI-only field).
  - Returns `Ok(())` on success or user cancellation; `Err` only on I/O failures.

- `pub fn run_pick_profile(profiles: &[Profile]) -> Result<usize>` / `pub fn run_pick_profile_with<R: BufRead, W: Write>(...) -> Result<usize>`
  - Interactive numbered picker used by `cct env` / `cct run` when no profile name is given.
  - Each profile line carries a backend tag: `[claude]`, `[codex] `, or `[kimi]  ` (padded to equal width).

### Private Functions

- `fn mask_key(key: &str) -> String`
  - Masks an API key for display in the CLI summary (not the TUI — that uses `ui::mask_value`).
  - If `key.len() <= 8`: returns `"*".repeat(key.len())` (all stars).
  - If `key.len() > 8`: returns `"<first4>...<last4>"` format (e.g., `"sk-1...key4"`).
  - This format gives the user enough visual confirmation to verify the key without exposing it.

- `fn resolve_backend(backend: Option<String>) -> Result<config::Backend>`
  - Parses the `--backend` flag: `None` → `Claude` (default); `"claude"` / `"codex"` / `"kimi"` (case-insensitive) → the matching variant.
  - Returns `Err("Unknown backend: '<x>'. Expected one of: claude, codex, kimi")` for anything else.
<!-- END:interface -->

---

<!-- BEGIN:dependency_graph -->
## 2. Dependency Graph

- **Imports from `crate::config`** → `config` module reference, `NewProfile` struct, `config::profile_name_exists` (duplicate check), `config::append_profile` (persistence). The `cli` module does not own any config I/O itself — it delegates entirely to `config`.
- **Imports from `std::io`** → `self` (for `io::stdin().lock()` and `io::stdout()`), `BufRead` trait (for `reader.read_line`), `Write` trait (for `writer.write!/writeln!`).
- **Does NOT depend on**: `app`, `ui`, `launch`, ratatui, or crossterm. The CLI flow is entirely text-based and terminal-state-agnostic.
<!-- END:dependency_graph -->

---

<!-- BEGIN:state_management -->
## 3. State Management

**Type**: Stateless between calls. All transient state (the five field values, the confirmation response) lives in local `String` variables on the call stack and is dropped when `run_add_with` returns.

The only persistent side effect is the file write performed by `config::append_profile`. If the function returns `Err` before reaching `append_profile`, no disk mutation occurs.

**Duplicate guard**: `config::profile_name_exists` reads the config file at call time (not cached) to check for collisions. If the user's config was modified concurrently during the prompt loop, the check reflects the on-disk state.
<!-- END:state_management -->

---

<!-- BEGIN:edge_cases -->
## 4. Edge Cases

### Duplicate Profile Name
- `profile_name_exists(name)` performs a **case-insensitive** check against all existing profile names.
- On detection: prints `"Error: profile '<name>' already exists."` to `stderr` and calls `std::process::exit(1)` — not a graceful `Err` return.
- This is a deliberate fail-fast design: if the config already contains the name, further prompts are pointless.

### Empty Name Loop
- The Name prompt loops with `"Name is required."` until the user enters a non-empty string.
- Other fields (2–5) accept empty input as `None` and do not loop.

### API Key Masking in Summary
- `mask_key` is distinct from `ui::mask_value`. `mask_key` shows `first4...last4` to help the user verify the correct key was entered. `mask_value` shows `"***"` unconditionally for any key containing TOKEN/KEY/SECRET.
- The summary's API Key line uses `mask_key`; if the key is empty, `"(none)"` is printed instead.

### Cancellation at Save Prompt
- Any response other than `"y"` (trimmed, lowercase) is treated as cancellation — no error, no write.
- `"Y"`, `"yes"`, `"n"`, `""` (Enter), or anything else all result in `"Cancelled."` and `Ok(())`.

### I/O Error Propagation
- All `write!`, `writeln!`, and `read_line` calls propagate errors via `?`. A broken pipe or closed writer causes the function to return `Err` immediately.
<!-- END:edge_cases -->

---

<!-- BEGIN:usage_example -->
## 5. Usage Example

```rust
// In main.rs — routing for CLI subcommands:
match args.command {
    Some(Commands::Add { auth_type, backend }) => cli::run_add(auth_type, backend),
    Some(Commands::Edit) => launch::open_editor(&config::config_path()),
    None => run_tui(),
}

// In tests — inject deterministic input/output:
let input = b"my-profile\nA description\nhttps://api.example.com\nsk-test-key\nkimi-k2\n\ny\n";
let mut output: Vec<u8> = Vec::new();
cli::run_add_with(&input[..], &mut output, None, config::Backend::Claude).unwrap();

// Verify the profile was created:
let profiles = config::load_profiles().unwrap();
assert!(profiles.iter().any(|p| p.name == "my-profile"));

// Inspect the summary output (contains masked key):
let text = String::from_utf8(output).unwrap();
assert!(text.contains("sk-t...key\n") || text.contains("***"));
assert!(text.contains("Profile 'my-profile' added."));
```
<!-- END:usage_example -->

---

## Quality Gate Checklist

- [x] **Interface**: 2 public + 1 private function documented with signatures and semantics
- [x] **Dependencies**: `config` module dependencies listed with reasoning; stdlib and no external crate deps noted
- [x] **State Management**: Stateless; delegation to `config::append_profile` for persistence described
- [x] **Edge Cases**: 5 cases identified — duplicate guard (exit 1), empty name loop, masking distinction, cancellation, I/O errors
- [x] **Usage Example**: Shows real `main.rs` routing and test injection pattern
- [x] **YAML Frontmatter**: `doc_type`, `module_name`, `module_path`, `generated_by` present

---

**Template Version**: 2.0
**Last Updated**: 2026-03-15 (revision 1 — initial doc)
