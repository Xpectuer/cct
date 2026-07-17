---
doc_type: module
module_name: "launch"
module_path: "src/launch.rs"
generated_by: mci-phase-2
revision: 3
updated: 2026-07-17
---

# launch Module Documentation

> **Purpose**: Handles all process-lifecycle concerns for `cct`: builds CLI argument lists for the Claude, Codex, and Kimi backends, generates Codex config files, surgically writes Kimi provider/model entries into `~/.kimi-code/config.toml`, exec-replaces the current process, restores terminal state, and opens `$EDITOR` for config hot-reload.
> **Path**: src/launch.rs

---

<!-- BEGIN:interface -->
## 1. Interface

### Exported Functions

- `pub fn restore_terminal()`
  - Disables crossterm raw mode and emits `LeaveAlternateScreen` to stdout.
  - Returns: `()` (errors from crossterm are silently discarded with `let _ = ...`).
  - Must be called before any exec or editor invocation to ensure the terminal is returned to cooked mode.

- `pub fn build_args(profile: &Profile, with_continue: bool) -> Vec<String>`
  - Pure function with no side effects.
  - Constructs the ordered CLI argument list for the `claude` binary from a `Profile`.
  - Argument ordering: `--continue` (if `with_continue` is `true`), then `--model <value>` (if `profile.model` is `Some`), then `--dangerously-skip-permissions` (if `profile.skip_permissions` is `Some(true)`), then each element of `profile.extra_args` in order.
  - Returns: `Vec<String>` — may be empty if `with_continue` is false and all profile fields are absent or false.

- `pub fn build_launch_command(profile: &Profile, with_continue: bool) -> (String, Vec<String>)`
  - Pure dispatch function; chooses the correct binary and arg builder based on `profile.backend`.
  - `Backend::Claude` → `("claude", build_args(profile, with_continue))`
  - `Backend::Codex` → `("codex", build_codex_args(profile))` (ignores `with_continue`)
  - `Backend::Kimi` → `("kimi", build_kimi_args(profile))` (ignores `with_continue`)
  - Used by integration tests to verify dispatch without exec-replacing the process.

- `pub fn exec_claude(profile: &Profile, with_continue: bool) -> anyhow::Error`
  - Applies a default set of Claude launch environment variables (`CLAUDE_DEFAULT_ENV`) — e.g. telemetry opt-outs, auto-updater disable, and attribution header off — then injects all key-value pairs from `profile.env` on top, so profile entries override defaults.
  - Calls `build_args(profile, with_continue)` then exec-replaces the current process with `claude <args>` using `std::os::unix::process::CommandExt::exec`.
  - `with_continue=true` prepends `--continue` to the arg list, resuming the last Claude Code session.
  - **Never returns on success** — the process image is replaced.
  - Returns: `anyhow::Error` only when `exec` itself fails.

- `pub fn check_codex_installed() -> bool`
  - Runs `which codex` to test whether the `codex` binary is available in `$PATH`.
  - Returns `true` if `which` exits with status 0; `false` on non-zero exit or any error.

- `pub fn generate_codex_config(profile: &Profile, codex_home: &Path) -> anyhow::Result<()>`
  - Writes `<codex_home>/config.toml` with the following content derived from the profile:
    ```toml
    model_provider = "custom"
    model = "<profile.model or gpt-4.1>"
    [model_providers.custom]
    name = "<profile.name>"
    base_url = "<profile.base_url or empty string>"
    ```
  - Creates parent directories if they do not exist.
  - Multiple codex profiles share one config file; it is fully rewritten before each codex launch.
  - `codex_home` is separated from the function body (testable with a temp dir).

- `pub fn build_codex_args(profile: &Profile) -> Vec<String>`
  - Pure function with no side effects.
  - Argument ordering: `--full-auto` (if `profile.full_auto` is `Some(true)`), then each element of `profile.extra_args` in order.
  - Does NOT include `--model`; the model is passed to codex via `config.toml` (through `CODEX_HOME`).

- `pub fn exec_codex(profile: &Profile) -> anyhow::Error`
  - Steps performed before exec-replace:
    1. Checks `codex` binary is installed via `check_codex_installed()`; returns error if not.
    2. Resolves `codex_home` to `~/.config/cct-tui/codex/` via `dirs::config_dir()`.
    3. Calls `generate_codex_config(profile, &codex_home)` to write `config.toml`; returns error on failure.
    4. Sets `CODEX_HOME` environment variable to `codex_home`.
    5. Injects all key-value pairs from `profile.env` (contains `OPENAI_API_KEY`).
    6. Exec-replaces with `codex <build_codex_args(profile)>`.
  - **Never returns on success**.

- `pub fn check_kimi_installed() -> bool`
  - Runs `which kimi` to test whether the `kimi` binary is available in `$PATH`. Same pattern as `check_codex_installed`.

- `pub fn prompt_install_kimi() -> Result<()>`
  - Mirrors `prompt_install()` for claude; runs the official installer `curl -fsSL https://code.kimi.com/kimi-code/install.sh | bash`.

- `pub fn kimi_config_path() -> PathBuf`
  - Returns the Kimi Code CLI config path: `$CCT_KIMI_CONFIG` if set (test override, mirrors `CCT_CONFIG` in `config::config_path`), else `~/.kimi-code/config.toml`.

- `pub fn generate_kimi_config(profile: &Profile) -> Result<()>`
  - Surgically writes this profile's entries into the kimi config via `toml_edit::DocumentMut`, creating the parent dir/file if missing and preserving all pre-existing tables (e.g. `managed:kimi-code` providers created by `kimi login`, `services.*`, `default_model`, `thinking`).
  - Writes `[providers."<profile.name>"]` with `type = "kimi"`, `base_url` (from `profile.base_url`, else env `ANTHROPIC_BASE_URL`; normalized to `https://` scheme + `/v1` suffix with duplicate slashes collapsed), and `api_key` (env `ANTHROPIC_AUTH_TOKEN` or `ANTHROPIC_API_KEY`).
  - Writes `[models."<name>/<model>"]` (skipped when the profile has no model) with `provider`, `model`, `max_context_size` (explicit `profile.max_context_size` resolved via `config::resolve_max_context_size`, else model-based default: `k3*` → 1,000,000, otherwise 262,144), `capabilities = ["thinking", "always_thinking", "image_in", "video_in", "tool_use"]`, `display_name` (uppercased model), and — for `k3*` models only — `support_efforts = ["max"]` / `default_effort = "max"` (both keys are removed on re-generation when the model is not `k3*`).

- `pub fn build_kimi_args(profile: &Profile) -> Vec<String>`
  - Pure function with no side effects.
  - `["-m", "<profile.name>/<model>"]` (model from `profile.model`, else env `ANTHROPIC_MODEL`; omitted entirely when no model), followed by `extra_args` verbatim.

- `pub fn exec_kimi(profile: &Profile) -> anyhow::Error`
  - Steps before exec-replace: (1) `check_kimi_installed()` — error if not found; (2) `generate_kimi_config(profile)`; (3) inject all `profile.env` pairs; (4) exec-replace with `kimi <build_kimi_args(profile)>`.
  - **Never returns on success**.

- `pub fn command_exists(cmd: &str) -> bool`
  - Runs `which <cmd>` to test whether an arbitrary command is available in `$PATH`.
  - Returns `true` if `which` exits with status 0; `false` on non-zero exit or any error.
  - Used by `run_env` to validate the user-supplied command before exec.

- `pub fn exec_with_env(profile: &Profile, cmd: &str, args: &[String]) -> anyhow::Error`
  - Applies the same default Claude launch environment variables as `exec_claude`, then injects all key-value pairs from `profile.env` on top.
  - Exec-replaces the current process with `<cmd> <args...>` using `std::os::unix::process::CommandExt::exec`.
  - **No shell is involved** — `$VAR` expansion, globs, and pipes do not work. Use `sh -c '...'` when shell features are needed.
  - **Never returns on success**.
  - Returns: `anyhow::Error` only when `exec` itself fails.

- `pub fn check_claude_installed() -> bool`
  - Runs `which <bin>` (or the value of `CCT_CLAUDE_BIN` env var when set) to test whether the target binary is available in `$PATH`.
  - The `CCT_CLAUDE_BIN` override is used exclusively in unit tests.

- `pub fn prompt_install() -> Result<()>`
  - Must be called **before** `enable_raw_mode` / `EnterAlternateScreen`.
  - Prints `"Claude CLI not found in PATH."` and prompts `"Install now? [Y/n]"`.
  - If user answers `"n"` or `"no"`: prints manual install instructions and calls `std::process::exit(0)`.
  - Otherwise: runs `curl -fsSL https://claude.ai/install.sh | bash`.
  - Returns `Err` if the installer exits non-zero or if `claude` is still not found after install.

- `pub fn open_editor(path: &Path) -> Result<()>`
  - Reads `$EDITOR`; falls back to `"vi"` if unset or empty.
  - Spawns the editor as a child process, blocking until it exits.
  - Returns: `Ok(())` on clean editor exit; `Err(anyhow::Error)` with context message `"spawn editor \"<editor>\""` if spawn fails.

### Exported Types

None — all public surface is functions. The module consumes `crate::config::Profile` and `crate::config::Backend` from the `config` module.

<!-- END:interface -->

---

<!-- BEGIN:dependency_graph -->
## 2. Dependency Graph

- **Imports from `crate::config`** → `Profile` struct and `Backend` enum. `Backend` is used in `build_launch_command` to dispatch to the correct arg builder.
- **Imports from `std::os::unix::process::CommandExt`** → Provides the `.exec()` method on `std::process::Command`. Unix-only; the module will not compile on Windows.
- **Imports from `std::process::Command`** → Used to construct the exec targets and the `which` check.
- **Imports from `std::env`** → `env::set_var` (inject env vars) and `env::var` (read `$EDITOR`).
- **Imports from `std::{fs, path::Path, path::PathBuf}`** → Used by `generate_codex_config` to create directories and write the config file.
- **Imports from `crossterm`** → `terminal::disable_raw_mode` and `execute!(stdout, LeaveAlternateScreen)` for terminal cleanup in `restore_terminal`.
- **Imports from `anyhow`** → `Context` trait and `Result` alias.
- **Imports from `dirs`** → `dirs::config_dir()` in `prompt_install` (claude fallback) and in `exec_codex` (to resolve `codex_home` path).
- **Does NOT depend on**: `app`, `ui`, or any async runtime.

<!-- END:dependency_graph -->

---

<!-- BEGIN:state_management -->
## 3. State Management

- **`build_args` / `build_codex_args`** — Purely functional. Take a `&Profile` reference, perform no I/O, and return a `Vec<String>`. `build_launch_command` is similarly pure; it just dispatches to one of these.

- **`open_editor`** — Spawns a child process and blocks. Reads `$EDITOR` at call time but retains no state.

- **`exec_claude`** — Two permanent side effects: (1) env mutation via `env::set_var` (default Claude env vars first, then `profile.env` overrides); (2) process replacement via `CommandExt::exec()`. Terminal cleanup (`restore_terminal`) must be called by the caller before `exec_claude`.

- **`exec_codex`** — Four side effects before exec: (1) writes `~/.config/cct-tui/codex/config.toml`; (2) sets `CODEX_HOME` env var; (3) sets `OPENAI_API_KEY` from `profile.env`; (4) process replacement. `restore_terminal` must be called before `exec_codex`.

- **`generate_codex_config`** — File I/O side effect: creates directory and writes config.toml. It is separated from `exec_codex` to allow unit testing against a temp directory without exec-replacing the process.

- **`generate_kimi_config`** — File I/O side effect: creates `~/.kimi-code/` and surgically edits `config.toml` in place. Path is overridable via `CCT_KIMI_CONFIG` so unit tests run against a temp dir and never touch the real file. `build_kimi_args` is pure, like the other arg builders.

- **`restore_terminal`** — Interacts with global terminal state. Errors suppressed intentionally.

<!-- END:state_management -->

---

<!-- BEGIN:edge_cases -->
## 4. Edge Cases

### Hardcoded Values and Fallbacks

- **Editor fallback**: `open_editor` defaults to `"vi"` when `$EDITOR` is unset. There is no validation that `vi` exists on the system; a missing `vi` will produce an `Err` with the context message `spawn editor "vi"`.
- **`--dangerously-skip-permissions` flag**: Only appended when `profile.skip_permissions` is explicitly `Some(true)`. A missing field (`None`) is treated identically to `Some(false)` via `unwrap_or(false)`.

### Error Handling Quirks

- **`exec_claude` return type is `anyhow::Error`, not `Result<!, anyhow::Error>`**: Rust's stable toolchain does not support the never type (`!`) as a return value in all positions. The function signature signals intent through its doc comment ("Returns only on error") but cannot enforce it statically. Callers must treat the return value as always representing failure.
- **`restore_terminal` swallows errors**: Both `disable_raw_mode()` and `execute!(...)` return `Result`s that are explicitly discarded. This is intentional — if the terminal is already in cooked mode, the call is a no-op and failing silently is correct.
- **`exec` error wrapping**: The error from `CommandExt::exec()` is wrapped in an `anyhow::anyhow!("exec claude: {err}")` string rather than using `.context()`, because `exec()` returns `io::Error` directly (not a `Result` with a success arm to chain from).

### Argument Ordering Contract

The ordering of arguments appended by `build_args` is deterministic and tested:
1. `--continue` (flag, only when `with_continue=true`) — must be first
2. `--model <value>` (positional pair, only when `model` is `Some`)
3. `--dangerously-skip-permissions` (flag, only when `skip_permissions` is `Some(true)`)
4. Elements of `extra_args` in their original TOML order (appended verbatim)

Callers must not assume any other ordering. Unit tests pin this contract: `build_args_empty`, `build_args_model_only`, `build_args_full`, `build_args_continue_only`, `build_args_continue_with_flags`.

### Unix-Only Constraint

`std::os::unix::process::CommandExt` is gated to Unix targets by the standard library. Compiling `cct` on Windows will fail at this import. There is no `#[cfg(unix)]` guard or Windows fallback; this is an intentional design constraint (terminal-based `exec` semantics are Unix-specific).

### Environment Variable Injection Race

`env::set_var` is not thread-safe in a multi-threaded program (it is `unsafe` in Rust editions that expose that). `cct` is single-threaded in its event loop, so this is safe in practice, but care must be taken if the architecture is ever extended to use background threads before the `exec` call.

### Default Claude Env Vars vs. Profile Overrides

`exec_claude` and `exec_with_env` first set a fixed list of default environment variables (`DISABLE_AUTOUPDATER=1`, telemetry opt-outs, attribution header off, etc.) and then apply `profile.env`. Any key present in both the default list and the profile's `[profiles.env]` table uses the profile value, so users can opt back into a default-disabled behavior on a per-profile basis.

<!-- END:edge_cases -->

---

<!-- BEGIN:usage_example -->
## 5. Usage Example

The following reproduces the actual call pattern from `src/main.rs`:

```rust
// --- Enter key pressed: launch selected profile (fresh session) ---
// Step 1: restore terminal BEFORE exec (mandatory ordering)
launch::restore_terminal();

// Step 2: exec_claude replaces the process; only returns on error
let err = launch::exec_claude(&app.profiles[app.selected], false);

// Step 3: exec failed — print error and exit with non-zero code
eprintln!("Error: {err:#}");
std::process::exit(1);

// --- 'c' key pressed: resume last Claude Code session (--continue) ---
launch::restore_terminal();
let err = launch::exec_claude(&app.profiles[app.selected], true);
eprintln!("Error: {err:#}");
std::process::exit(1);

// --- 'e' key pressed: hot-reload config via $EDITOR ---
// Step 1: restore terminal so the editor gets a clean cooked-mode terminal
launch::restore_terminal();

// Step 2: open editor on the config file path; blocks until editor exits
launch::open_editor(&config::config_path())?;

// Step 3: re-enter raw mode and re-draw the TUI
enable_raw_mode()?;
execute!(io::stdout(), EnterAlternateScreen)?;
tui.clear()?;

// --- Inspecting what args would be built (e.g., for logging or testing) ---
let profile = Profile {
    name: "prod".into(),
    description: Some("Production endpoint".into()),
    model: Some("claude-opus-4-6".into()),
    skip_permissions: Some(false),
    extra_args: Some(vec!["--verbose".into()]),
    env: Some([
        ("ANTHROPIC_BASE_URL".into(), "https://api.example.com".into()),
        ("ANTHROPIC_AUTH_TOKEN".into(), "sk-ant-...".into()),
    ].into()),
};

let args = launch::build_args(&profile, false);
// args == ["--model", "claude-opus-4-6", "--verbose"]

let args_continue = launch::build_args(&profile, true);
// args_continue == ["--continue", "--model", "claude-opus-4-6", "--verbose"]
```

<!-- END:usage_example -->

---

## Quality Gate Checklist

- [x] **Interface**: 8 public functions documented with signatures, return types, and semantics
- [x] **Dependencies**: All internal and external module dependencies listed with reasoning (added `dirs`)
- [x] **State Management**: Clearly distinguishes pure functions from process-mutating functions; lifecycle of env mutation explained
- [x] **Edge Cases**: Editor fallback, error-type quirk, argument ordering contract, Unix-only constraint, env set_var threading note
- [x] **Usage Example**: Concrete Rust pseudocode mirroring actual `main.rs` call patterns for both Enter (exec) and 'e' (editor) flows
- [x] **YAML Frontmatter**: `doc_type`, `module_name`, `module_path` present

---

**Template Version**: 2.0
**Last Updated**: 2026-07-17 (revision 3 — Kimi backend: check_kimi_installed, prompt_install_kimi, kimi_config_path, generate_kimi_config, build_kimi_args, exec_kimi, build_launch_command Kimi arm)
