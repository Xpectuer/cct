---
doc_type: module
module_name: "launch"
module_path: "src/launch.rs"
generated_by: mci-phase-2
revision: 4
updated: 2026-08-02
---

# launch Module Documentation

> **Purpose**: Handles all process-lifecycle concerns for `cct`: builds CLI argument lists for the Claude and Codex backends, builds inline `--config` flags for the Codex provider (proxy/subscription modes), exec-replaces the current process, restores terminal state, and opens `$EDITOR` for config hot-reload.
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
  - Used by integration tests to verify dispatch without exec-replacing the process.

- `pub fn exec_claude(profile: &Profile, with_continue: bool) -> anyhow::Error`
  - Injects all key-value pairs from `profile.env` into the current process environment via `env::set_var`.
  - Calls `build_args(profile, with_continue)` then exec-replaces the current process with `claude <args>` using `std::os::unix::process::CommandExt::exec`.
  - `with_continue=true` prepends `--continue` to the arg list, resuming the last Claude Code session.
  - **Never returns on success** — the process image is replaced.
  - Returns: `anyhow::Error` only when `exec` itself fails.

- `pub fn check_codex_installed() -> bool`
  - Runs `which codex` to test whether the `codex` binary is available in `$PATH`.
  - Returns `true` if `which` exits with status 0; `false` on non-zero exit or any error.

- `pub fn build_codex_proxy_config_args(model: &str, port: u16) -> Vec<String>`
  - Pure function with no side effects.
  - Builds 6 inline `--config` flags that configure the custom provider pointing at the local proxy: `model_provider=custom`, `model=<model>`, `model_providers.custom.name=cct-proxy`, `model_providers.custom.base_url=http://127.0.0.1:<port>/v1`, `model_providers.custom.wire_api=responses`, `model_providers.custom.env_key=OPENAI_API_KEY`.
  - Replaces the old `config.toml` approach: `CODEX_HOME` is left at its default (`~/.codex`) so all profiles share history/sessions.

- **No `config.toml` or `auth.json` is written for Codex**: the API key travels to the local proxy over the control socket via `proxy::switch_profile`, and the proxy injects it as the `Authorization` header when forwarding. The profile top-level `api_key = "..."` shorthand is deserialized into `env.OPENAI_API_KEY` for Codex profiles.

- `pub fn build_codex_args(profile: &Profile) -> Vec<String>`
  - Pure function with no side effects.
  - Builds the shared approval/extra args (`build_shared_codex_args`): the approval flag matching `profile.full_auto` (`--ask-for-approval never|untrusted`, `--dangerously-bypass-approvals-and-sandbox`, or nothing), then each element of `profile.extra_args` in order.
  - Does NOT include `--model` or provider config — those arrive via `--config` flags built by `build_codex_proxy_config_args` (proxy mode) or `build_codex_subscription_args` (subscription mode).

- `pub fn exec_codex(profile: &Profile) -> anyhow::Error`
  - Checks `codex` is installed via `check_codex_installed()`; returns error if not. Then dispatches on `auth_type`:
    - **Proxy mode** (default, API key): (1) `ensure_proxy_running` spawns the `cct proxy` daemon if it is not healthy; (2) `proxy::switch_profile` switches the daemon's upstream to the profile's `base_url`/`OPENAI_API_KEY`/`model` over the control socket; (3) injects `profile.env`; (4) exec-replaces with `codex <build_codex_proxy_config_args(model, port)> <approval/extra args>`.
    - **Subscription mode** (`auth_type = "subscription"`): sets `DISABLE_AUTOUPDATER=1`, injects `profile.env`, and exec-replaces with `codex --config model_provider=openai [--config model=<model>] <approval/extra args>` — no proxy, Codex's built-in OpenAI provider with OAuth.
  - `CODEX_HOME` is never set in either mode — Codex uses its default `~/.codex`, shared by all profiles (no `config.toml` or `auth.json` is written).
  - **Never returns on success**.

- `pub fn command_exists(cmd: &str) -> bool`
  - Runs `which <cmd>` to test whether an arbitrary command is available in `$PATH`.
  - Returns `true` if `which` exits with status 0; `false` on non-zero exit or any error.
  - Used by `run_env` to validate the user-supplied command before exec.

- `pub fn exec_with_env(profile: &Profile, cmd: &str, args: &[String]) -> anyhow::Error`
  - Injects all key-value pairs from `profile.env` into the current process environment via `env::set_var`.
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
  - Otherwise: delegates to `install_claude`.
  - Returns `Err` if the installer exits non-zero or if `claude` is still not found after install.

- `pub fn install_claude() -> Result<()>`
  - Non-interactive install used by `cct run` so agents and scripts can bootstrap Claude Code on first use instead of hitting a bare "not found" exec error.
  - Runs `curl -fsSL https://claude.ai/install.sh | bash`, then re-checks `check_claude_installed`, falling back to `~/.local/bin/claude` existing on disk.
  - Returns `Err` with the manual install command in the message if the installer fails or `claude` is still not found.

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
- **Imports from `std::{fs, path::Path, path::PathBuf}`** → `fs` used by `ensure_proxy_running` to redirect proxy stderr to the log file (`CCT_PROXY_LOG`); `Path`/`PathBuf` used for the proxy control socket path.
- **Imports from `crossterm`** → `terminal::disable_raw_mode` and `execute!(stdout, LeaveAlternateScreen)` for terminal cleanup in `restore_terminal`.
- **Imports from `anyhow`** → `Context` trait and `Result` alias.
- **Imports from `dirs`** → `dirs::home_dir()` for the claude autoinstall prompt and `exec_with_env` resolution.
- **Does NOT depend on**: `app`, `ui`, or any async runtime.

<!-- END:dependency_graph -->

---

<!-- BEGIN:state_management -->
## 3. State Management

- **`build_args` / `build_codex_args`** — Purely functional. Take a `&Profile` reference, perform no I/O, and return a `Vec<String>`. `build_launch_command` is similarly pure; it just dispatches to one of these.

- **`open_editor`** — Spawns a child process and blocks. Reads `$EDITOR` at call time but retains no state.

- **`exec_claude`** — Two permanent side effects: (1) env mutation via `env::set_var`; (2) process replacement via `CommandExt::exec()`. Terminal cleanup (`restore_terminal`) must be called by the caller before `exec_claude`.

- **`exec_codex`** — Side effects before exec: proxy mode ensures/spawns the `cct proxy` daemon and switches its upstream over the control socket; both modes inject `profile.env` via `env::set_var`; process replacement. No Codex config file is written and `CODEX_HOME` is never set. `restore_terminal` must be called before `exec_codex`.

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
**Last Updated**: 2026-08-02 (revision 4 — Codex launch via local proxy: config.toml/auth.json generation removed, build_codex_proxy_config_args inline `--config` flags, CODEX_HOME never set)
