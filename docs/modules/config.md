---
doc_type: module
module_name: "config"
module_path: "src/config.rs"
generated_by: mci-phase-2
revision: 5
updated: 2026-07-17
---

# config Module Documentation

> **Purpose**: Deserializes `profiles.toml` into typed Rust structs via `serde`/`toml`, bootstraps a default config file on first run, resolves the config path, validates profile field combinations, and writes new profiles for the Claude, Codex, and Kimi backends.
> **Path**: `src/config.rs`

---

<!-- BEGIN:interface -->
## 1. Interface

### Exported Types

- `enum Backend` — discriminates the three supported launch backends:
  - `Claude` (default, via `#[derive(Default)]`) — profiles launched with the `claude` CLI.
  - `Codex` — profiles launched with the `codex` CLI.
  - `Kimi` — profiles launched with the `kimi` CLI (Kimi Code).
  - Attributes: `#[serde(rename_all = "lowercase")]` so TOML uses `backend = "claude"` / `"codex"` / `"kimi"`.
  - Derives: `Debug`, `Default`, `Deserialize`, `Clone`, `PartialEq`.

- `struct Profile` — Represents one launch profile loaded from TOML. All fields except `name` are optional:
  - `name: String` — Unique display name shown in the TUI list.
  - `description: Option<String>` — Human-readable description shown in the detail panel.
  - `backend: Backend` — `#[serde(default)]` so existing configs without this field default to `Backend::Claude`.
  - `base_url: Option<String>` — First-class profile field. For Claude: written as `ANTHROPIC_BASE_URL` in `[profiles.env]`. For Codex: written as `base_url` in the profile block and passed to `generate_codex_config`.
  - `full_auto: Option<bool>` — Codex-only. When `true`, adds `--full-auto` to the codex invocation.
  - `env: Option<HashMap<String, String>>` — Environment variables injected before exec.
  - `extra_args: Option<Vec<String>>` — Additional CLI arguments appended verbatim.
  - `skip_permissions: Option<bool>` — Claude-only. When `true`, adds `--dangerously-skip-permissions`.
  - `auth_type: Option<String>` — Claude only. When `"token"`, uses `ANTHROPIC_AUTH_TOKEN` env var. Default `None` means `ANTHROPIC_API_KEY`.
  - `model: Option<String>` — Passed via `--model` for Claude; via `config.toml` for Codex; via `-m <name>/<model>` for Kimi.
  - `max_context_size: Option<String>` — Kimi-only (`#[serde(default)]`). `"1m"` (1,000,000) or `"260k"` (262,144); `None` means auto-detect from the model (`k3*` → `1m`, otherwise `260k`). Toggled with the `Space` key.
  - Derives: `Debug`, `Deserialize`, `Clone`.

- `struct NewProfile` — Input type for creating a new profile via `append_profile`. All fields except `name` are optional:
  - `name: String` — Required profile name (must be unique, case-insensitive).
  - `description: Option<String>` — Human-readable description (Claude only; ignored for Codex).
  - `base_url: Option<String>` — For Claude: `ANTHROPIC_BASE_URL` in env. For Codex: written as `base_url =` in the profile block.
  - `api_key: Option<String>` — For Claude: `ANTHROPIC_API_KEY` in env (or `ANTHROPIC_AUTH_TOKEN` if `auth_type = "token"`). For Codex: `OPENAI_API_KEY` in env.
  - `auth_type: Option<String>` — Claude only. When `"token"`, writes `ANTHROPIC_AUTH_TOKEN` instead of `ANTHROPIC_API_KEY`.
  - `model: Option<String>` — For Claude: `model =` field + 5 model alias env vars. For Codex: `model =` field only (no env vars).
  - `backend: Backend` — Which backend this profile targets.
  - `full_auto: Option<bool>` — Codex-only. Written as `full_auto =` in the profile block.
  - `max_context_size: Option<String>` — Kimi-only. Written as `max_context_size = "1m"|"260k"` in the profile block when set.

### Exported Functions

- `config_path() -> PathBuf`
  - Returns the resolved path to `profiles.toml`.
  - Priority: `CCT_CONFIG` env var (if set and non-empty) → `dirs::config_dir()/cc-tui/profiles.toml` → fallback `~/.config/cc-tui/profiles.toml` if `dirs` returns `None`.
  - No I/O performed; pure path resolution.

- `ensure_default_config() -> Result<()>`
  - Checks whether the file at `config_path()` exists.
  - If absent: creates all parent directories with `fs::create_dir_all`, then writes `DEFAULT_CONFIG` to the path.
  - If present: no-op (idempotent).
  - Returns `anyhow::Result<()>`; propagates errors with context messages.

- `validate_profiles(profiles: &[Profile]) -> Result<()>`
  - Called immediately after deserialization inside `load_profiles()`.
  - Rejects illegal combinations:
    - `backend == Codex && skip_permissions == Some(true)` — codex does not have a skip-permissions flag.
    - `backend == Claude && full_auto == Some(true)` — full_auto is codex-only.
    - `backend == Kimi && skip_permissions == Some(true)` — kimi does not have a skip-permissions flag.
    - `backend == Kimi && full_auto.is_some()` — full_auto is codex-only.
    - `backend == Kimi && auth_type.is_some()` — kimi always uses `ANTHROPIC_API_KEY`.
  - Returns `Err` with a descriptive message naming the offending profile on first violation found.

- `load_profiles() -> Result<Vec<Profile>>`
  - Reads the file at `config_path()` to a `String`.
  - Parses the full TOML document; then calls `validate_profiles` before returning.
  - Returns `anyhow::Result<Vec<Profile>>`; propagates I/O, parse, and validation errors.

- `profile_name_exists(name: &str) -> Result<bool>`
  - Calls `load_profiles()` and returns `true` if any profile's name matches `name` case-insensitively.
  - Used by both `cli::run_add_with` and the TUI AddForm to guard against duplicate names before appending.

- `append_profile(profile: &NewProfile) -> Result<()>`
  - Appends a new `[[profiles]]` block (and optional `[profiles.env]` block) to the existing config file.
  - **Backend-specific behaviour**:
    - Claude: `base_url` → `ANTHROPIC_BASE_URL`; `api_key` → `ANTHROPIC_API_KEY`; `model` → `ANTHROPIC_MODEL` + 4 alias vars + `API_TIMEOUT_MS` + `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC`.
    - Codex: `base_url` written as profile-level `base_url =` field (not in env); `full_auto` written as `full_auto =` field; only `OPENAI_API_KEY` goes into `[profiles.env]`.
    - Kimi: only `ANTHROPIC_BASE_URL`, `ANTHROPIC_API_KEY`, and `ANTHROPIC_MODEL` go into `[profiles.env]` (no Claude-specific aliases, no timeouts); `max_context_size` written as a profile-level field when set.
    - `backend = "codex"` / `"kimi"` is written explicitly; `backend = "claude"` is omitted (it is the serde default).
  - Reads then appends (never rewrites) the config file to preserve comments and ordering.

- `ensure_kimi_profile() -> Result<()>`
  - If no `Backend::Kimi` profile exists, appends a `default-kimi` profile (`model = "kimi-k2"`, `base_url = "https://api.kimi.com/v1"`).
  - Mirrors `ensure_codex_profile()`; called from `main` at startup (errors ignored, non-fatal).

- `default_max_context_size(model: Option<&str>) -> &'static str`
  - Pure helper: model starting with `k3` → `"1m"`, anything else (including `None`) → `"260k"`.

- `resolve_max_context_size(size: Option<&str>) -> u64`
  - Pure helper: `Some("1m")` → `1_000_000`, anything else → `262_144`. Used by `launch::generate_kimi_config`.

- `toggle_skip_permissions(profile_name: &str, new_value: bool) -> Result<()>`
  - Surgically updates the `skip_permissions` field of the named profile using `toml_edit::DocumentMut`.
  - Preserves all comments, whitespace, and key ordering for other profiles.
  - Callers in `main.rs` reflect the change optimistically in `app.profiles[app.selected]` after a successful return.

- `toggle_auth_type(profile_name: &str) -> Result<()>`
  - Toggles between `ANTHROPIC_API_KEY` and `ANTHROPIC_AUTH_TOKEN` for Claude profiles.
  - Renames the env var key and sets/removes the `auth_type = "token"` field using `toml_edit::DocumentMut`.
  - Does nothing to the key value — only renames the env var name.
  - Bound to `t` key in the TUI. Equivalent CLI: `cct add --auth-type token` for new profiles.

- `toggle_kimi_max_context_size(profile_name: &str) -> Result<()>`
  - Flips the `max_context_size` field of the named Kimi profile between `"1m"` and `"260k"`.
  - Computes the current effective value (explicit field, else the model-based default from `default_max_context_size`) and writes the opposite explicit value.
  - Surgical `toml_edit::DocumentMut` edit — preserves comments, whitespace, and key ordering.
  - Bound to the `Space` key in the TUI (Kimi tab only); callers reload the profile in place after a successful return.

### Private Constants

- `DEFAULT_CONFIG: &str` — A `const` string literal containing a commented example `profiles.toml` with one minimal `[[profiles]]` block. Written to disk only when no config file exists. Verified by the `default_config_is_valid_toml` unit test to be parseable TOML.

- `struct Config` — Private deserialization wrapper with a single field `profiles: Vec<Profile>`. Exists only to satisfy TOML's top-level table requirement; not exposed outside this module.
<!-- END:interface -->

---

<!-- BEGIN:dependency_graph -->
## 2. Dependency Graph

### External Crate Dependencies

- **`serde`** (feature `derive`) — Provides the `Deserialize` derive macro applied to `Profile` and `Config`. No `Serialize` is used; config is read-only from Rust's perspective.
- **`toml`** — `toml::from_str::<Config>(&content)` performs the TOML-to-struct deserialization.
- **`toml_edit`** — `toml_edit::DocumentMut` is used by the toggle functions (`toggle_skip_permissions`, `toggle_auth_type`, `toggle_codex_auth_type`, `toggle_full_auto`, `toggle_kimi_max_context_size`) and by `update_profile` for surgical
  in-place edits that preserve comments and formatting. Read paths continue to use the simpler `toml` crate.
- **`anyhow`** — `anyhow::Result` and the `.with_context(|| ...)` combinator are used for all error propagation, giving callers human-readable error chains.
- **`dirs`** — `dirs::config_dir()` maps to the OS-appropriate XDG config directory (`~/.config` on Linux, `~/Library/Application Support` on macOS). Falls back to `PathBuf::from("~/.config")` if `dirs` returns `None`.

### Standard Library Dependencies

- **`std::collections::HashMap`** — Type for `Profile.env`; maps env var names to values.
- **`std::fs`** — `fs::read_to_string`, `fs::write`, `fs::create_dir_all` for all disk I/O.
- **`std::path::PathBuf`** — Return type of `config_path()` and intermediate path construction.
- **`std::env::var`** — Used inside `config_path()` to read `CCT_CONFIG`.

### Internal Module Dependencies

- **None.** The `config` module is a leaf in the internal dependency graph. It does not import from `app`, `ui`, or `launch`. All other modules that need config data receive a `Vec<Profile>` from `main` rather than calling into this module directly.
<!-- END:dependency_graph -->

---

<!-- BEGIN:state_management -->
## 3. State Management

**Type**: Stateless at runtime.

The `config` module holds no heap-allocated state between calls. Every function is a standalone I/O operation or pure path computation:

- `config_path()` reads one environment variable and constructs a `PathBuf`; the result is not cached.
- `ensure_default_config()` performs file-system side effects (create dirs, write file) and then returns, retaining nothing.
- `load_profiles()` reads the file, parses it into an owned `Vec<Profile>`, and transfers ownership to the caller. No reference to the parsed data remains in this module.

**State on disk** (not in memory):

| Location | Format | Lifecycle |
|---|---|---|
| `$CCT_CONFIG` or `~/.config/cc-tui/profiles.toml` | TOML | Persistent; created once by `ensure_default_config`, mutated only by the user's `$EDITOR` via `launch::open_editor` |

**Hot-reload pattern**: `main.rs` calls `config::load_profiles()` a second time after the editor closes (key `e`). Because this module is stateless, the second call reads the freshly-saved file without any cache invalidation step.
<!-- END:state_management -->

---

<!-- BEGIN:edge_cases -->
## 4. Edge Cases

### CCT_CONFIG Environment Variable Override

- When `CCT_CONFIG` is set to a non-empty value, `config_path()` returns it unconditionally without consulting `dirs`. This allows test harnesses and CI pipelines to supply a fixture file without touching the user's real config directory.
- If `CCT_CONFIG` contains a path whose parent directory does not exist, `ensure_default_config()` will attempt to create it via `fs::create_dir_all`. Failure produces an `anyhow` error with a descriptive context string.

### Missing or Unresolvable XDG Config Directory

- `dirs::config_dir()` returns `None` on platforms where no home directory is configured. The code guards this with `.unwrap_or_else(|| PathBuf::from("~/.config"))`. Note: the fallback is a literal tilde string, which is not automatically expanded by `fs`; on such a system the path would be relative to the working directory and likely fail at the I/O call site.

### TOML Parse Errors

- `load_profiles()` wraps `toml::from_str` with `.with_context(|| format!("parse TOML in {path:?}"))`. A malformed `profiles.toml` (e.g., after a user edit) surfaces as an `anyhow::Error` in `main`. The hot-reload path in `main.rs` handles this gracefully with a `match` that prints a warning and retains the previously-loaded profiles rather than crashing.
- Missing required field: `Profile.name` is the only non-optional field. A `[[profiles]]` block without `name` will fail deserialization with a serde error.

### DEFAULT_CONFIG Bootstrap

- `ensure_default_config()` is idempotent: it only writes the file if it does not already exist. A partially-written or corrupted file that exists on disk will NOT be overwritten; `load_profiles()` will return a parse error instead.
- The `DEFAULT_CONFIG` constant intentionally comments out all optional fields so users see the available knobs without having them take effect. The `default_config_is_valid_toml` unit test guarantees this string is always parseable, preventing template drift.

### Empty Profiles List

- A valid `profiles.toml` containing `profiles = []` (or simply no `[[profiles]]` blocks) passes TOML parsing and returns an empty `Vec<Profile>`. The TUI renders an empty list; the Enter key is guarded by `!app.profiles.is_empty()` in `main.rs`, so no panic occurs.

### File Permissions

- `fs::write` and `fs::create_dir_all` use the process's default umask. No explicit permission bits are set, so the config file and directory inherit the user's umask (typically `0644` / `0755`). Sensitive values like `ANTHROPIC_AUTH_TOKEN` are stored in plaintext; the `ui` module masks them on display, but the file itself is not encrypted.
<!-- END:edge_cases -->

---

<!-- BEGIN:usage_example -->
## 5. Usage Example

The following pseudocode mirrors the actual call sites in `src/main.rs`:

```rust
use cct::{config, app, launch, ui};
use app::App;

fn main() -> anyhow::Result<()> {
    // Step 1: Ensure ~/.config/cc-tui/profiles.toml exists.
    // Creates parent dirs and writes DEFAULT_CONFIG on first run.
    // No-op on subsequent runs. Fails fast with a descriptive error
    // if the directory cannot be created (e.g., permission denied).
    config::ensure_default_config()?;

    // Step 2: Read and deserialize all profiles.
    // Returns Vec<Profile>; ownership transfers entirely to the caller.
    // Errors here mean the file is unreadable or contains invalid TOML.
    let profiles: Vec<config::Profile> = config::load_profiles()?;

    // Step 3: Hand profiles to the App state machine.
    let mut app = App::new(profiles);

    // --- Main event loop ---
    loop {
        // ... draw TUI, read key events ...

        // On Enter: exec-replace with claude using the selected profile.
        // Profile fields (name, model, skip_permissions, extra_args, env)
        // are read by launch::exec_claude — config module is not called again.
        if user_pressed_enter {
            launch::exec_claude(&app.profiles[app.selected]);
        }

        // On 'e': open editor, then hot-reload profiles without restart.
        // config_path() is called to pass the file path to the editor.
        if user_pressed_e {
            launch::open_editor(&config::config_path())?;

            // Second call to load_profiles() picks up edits.
            // Errors are warned rather than propagated, preserving the
            // previous valid state in app.profiles.
            match config::load_profiles() {
                Ok(updated) => {
                    app.profiles = updated;
                    // Clamp cursor if list shrank.
                    if app.selected >= app.profiles.len() {
                        app.selected = app.profiles.len().saturating_sub(1);
                    }
                }
                Err(e) => eprintln!("Warning: profile reload failed: {e:#}"),
            }
        }
    }
}

// In tests or CI: override the config path via environment variable.
// CCT_CONFIG=/tmp/fixture.toml cargo test
// config::config_path() will return PathBuf::from("/tmp/fixture.toml").
```
<!-- END:usage_example -->

---

## Quality Gate Checklist

- [x] **Interface**: 3 exported types (`Profile`, `NewProfile`; `Config` private) + 5 public functions + 1 constant documented
- [x] **Dependencies**: All external crates (`serde`, `toml`, `anyhow`, `dirs`) and std modules listed with reasoning; internal leaf status stated
- [x] **State Management**: Clearly stateless at runtime; on-disk state lifecycle documented with hot-reload pattern explained
- [x] **Edge Cases**: 6 cases identified — CCT_CONFIG override, missing XDG dir, TOML parse errors, DEFAULT_CONFIG bootstrap, empty profiles list, file permissions
- [x] **Usage Example**: Rust pseudocode mirrors real `main.rs` call sites, covers initial load and hot-reload paths
- [x] **YAML Frontmatter**: `doc_type`, `module_name`, `module_path`, `generated_by` all present

---

**Template Version**: 2.0
**Last Updated**: 2026-07-17 (revision 5 — added `Backend::Kimi`, `max_context_size` fields, `ensure_kimi_profile`, `default_max_context_size`/`resolve_max_context_size`, `toggle_kimi_max_context_size`, Kimi validate/append rules)
