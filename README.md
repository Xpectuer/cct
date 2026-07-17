<p align="center">
  <img src="./logo.png" alt="cct logo" width="160">
</p>

# cct — Claude Code TUI Launcher

A terminal UI for managing and launching [Claude Code](https://claude.ai/code), [OpenAI Codex](https://github.com/openai/codex), and [Kimi Code](https://code.kimi.com) with named profiles. Define multiple configurations in a single TOML file, pick one from a TUI, and `cct` exec-replaces itself with the target CLI — no wrapper process, clean terminal inheritance.

## Features

- **Three backends** — supports Claude Code, OpenAI Codex, and Kimi Code, with per-backend profile fields, env vars, and launch flags
- **Profile management** — store model, env vars, and CLI flags per profile
- **TUI selector** — ratatui-based list+detail panel with keyboard navigation, organized into Claude/Codex/Kimi tabs
- **Inline profile creation** — press `a` in the TUI to open a 6-field add form (backend-aware: Claude gets Pro Model + Fast Model; Codex gets Model + Approval; Kimi gets Model + Context), or run `cct add` from the CLI
- **Inline profile editing** — press `e` on a selected profile to open a prefilled edit form, update the fields inline, and save back to `profiles.toml`
- **Profile duplication** — press `d` on a selected profile to duplicate it (appends `_copy` to the name), edit any fields, and save as a new profile
- **Auto-populated env vars** — providing `base_url`, `api_key`, or `model` in the add flow generates a `[profiles.env]` section with all relevant env vars pre-filled (Anthropic vars for Claude, `OPENAI_API_KEY` for Codex, a minimal 3-var set for Kimi)
- **Claude launch defaults** — `cct` injects privacy/telemetry defaults (auto-updater off, telemetry opt-outs, attribution header off, etc.) before applying profile env vars, so every Claude profile starts with the same defensive baseline
- **Sensitive value masking** — env keys containing `TOKEN`, `KEY`, or `SECRET` are redacted in the UI
- **skip_permissions / full_auto toggle** — press `s` to toggle `--dangerously-skip-permissions` on Claude profiles or `--full-auto` on Codex profiles; the change is persisted immediately to `profiles.toml`
- **Auth type toggle** — press `t` to switch between `ANTHROPIC_API_KEY` and `ANTHROPIC_AUTH_TOKEN` on Claude profiles; persisted immediately. Use `cct add --auth-type token` to create token-auth profiles from the CLI
- **Kimi max_context_size toggle** — press `Space` on a Kimi profile to flip `max_context_size` between `1m` and `260k`; persisted immediately. When unset, the default is auto-detected from the model (`k3*` → `1m`, otherwise `260k`)
- **Kimi config generation** — before launching, `cct` surgically writes the profile's provider/model entries into `~/.kimi-code/config.toml` (preserving existing providers such as `managed:kimi-code` from `kimi login`)
- **One-shot continue** — press `c` to launch the selected Claude profile with `--continue`, resuming the last conversation in a single turn
- **Autoinstall** — if `claude` is not found in PATH on startup, `cct` offers to install it via `curl -fsSL https://claude.ai/install.sh | bash`; a missing `kimi` binary produces a non-blocking install hint
- **Zero overhead** — `exec()` replaces the process; no parent lingers

## Subcommands

| Command | Action |
|---------|--------|
| `cct` (no args) | Launch the TUI |
| `cct add` | Add a new profile interactively. `--auth-type token` writes `ANTHROPIC_AUTH_TOKEN` instead of `ANTHROPIC_API_KEY`; `--backend codex\|kimi` selects the backend (default `claude`) |
| `cct edit` | Open `profiles.toml` in `$EDITOR` (or `vi`) |
| `cct run [name]` | Launch a profile by name (case-insensitive). Without a name, shows an interactive numbered picker |
| `cct env` | List all profiles (non-interactive) |
| `cct env <profile> -- <cmd> [args...]` | Run any command with a profile's environment variables injected |

## Install

**Option A — curl|bash (GitHub, recommended)**:
```bash
curl -fsSL https://raw.githubusercontent.com/Xpectuer/cc_starter/refs/heads/master/install.sh | bash
```

**Option B — curl|bash (self-hosted GitLab, internal network)**:
```bash
curl -fsSL https://gitlab.clounix.com/zhengjy/cc_starter/-/raw/master/install.sh | GITLAB_URL=https://gitlab.clounix.com GITLAB_PROJECT=zhengjy/cc_starter bash
```

Installs the latest release binary to `~/.local/bin/cct`. Requires `curl` and `tar`.
Supported platforms: Linux x86_64 (both), Linux aarch64 (both), macOS arm64 (GitHub only), macOS x86_64 (GitHub only).

**Option C — cargo**:
```bash
cargo install --path .
```

Requires Rust 1.70+ and a Unix-like OS (uses `exec`).

## Quick Start

1. Run `cct` once to generate the default config at `~/Library/Application Support/cc-tui/profiles.toml` (macOS) or `~/.config/cc-tui/profiles.toml` (Linux/other Unix-like).
2. Add profiles interactively:

   **Option A — TUI form**: Run `cct`, then press `a` to open the inline add form. Use `Tab` to switch between the Claude/Codex/Kimi tab to choose which backend the new profile uses. Fill in the fields and confirm.

   **Option B — CLI**: Run `cct add` and answer the prompts (Claude backend by default; pass `--backend codex` or `--backend kimi` to choose another).

   **Option C — Manual edit**: Run `cct edit` to open the config in your editor, or edit the file directly:

```toml
# Claude profile
[[profiles]]
name = "third-party"
description = "Third-party Claude endpoint"
model = "kimi-k2"                  # optional — maps to --model
skip_permissions = false            # optional — adds --dangerously-skip-permissions
auth_type = "token"                 # optional — "token" uses ANTHROPIC_AUTH_TOKEN instead of ANTHROPIC_API_KEY
extra_args = ["--verbose"]          # optional — appended verbatim

# Auto-generated by `cct add` when base_url / api_key / model are provided:
[profiles.env]
ANTHROPIC_BASE_URL = "https://api.example.com/v1"
ANTHROPIC_API_KEY = "sk-..."
ANTHROPIC_MODEL = "kimi-k2"
ANTHROPIC_DEFAULT_SONNET_MODEL = "kimi-k2"
ANTHROPIC_DEFAULT_OPUS_MODEL = "kimi-k2"
CLAUDE_CODE_SUBAGENT_MODEL = "kimi-k2"
API_TIMEOUT_MS = "600000"
CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC = "1"
CLAUDE_CODE_DISABLE_NONSTREAMING_FALLBACK = "1"
CLAUDE_CODE_EFFORT_LEVEL = "max"

# Codex profile
[[profiles]]
name = "codex-profile"
backend = "codex"
model = "gpt-4.1"
base_url = "https://api.openai.com/v1"
full_auto = false

[profiles.env]
OPENAI_API_KEY = "sk-..."

# Kimi profile
[[profiles]]
name = "kimi-profile"
backend = "kimi"
model = "kimi-k2"
base_url = "https://api.kimi.com/v1"
max_context_size = "1m"             # optional — "1m" or "260k" (default: auto from model)

[profiles.env]
ANTHROPIC_BASE_URL = "https://api.kimi.com/v1"
ANTHROPIC_API_KEY = "sk-..."
ANTHROPIC_MODEL = "kimi-k2"
```

3. Run `cct`, select a profile, and press `e` to edit it inline or `Enter` to launch it (or `c` to launch Claude with `--continue`). Switch backend tabs with `Tab` / `1` / `2` / `3`.
4. Launch from the command line:
   - `cct run <name>` — launch a profile by name directly
   - `cct run` — interactive numbered picker
   - `cct env <profile> -- <command>` — run a command with a profile's env vars
   - `cct env` — list all profiles non-interactively

## Tips

**Pass arbitrary flags to Claude.** `cct env` injects a profile's environment and runs whatever command you give it — the command doesn't need to be aware of `cct` at all. To launch Claude with extra flags that `cct` doesn't natively expose:

```bash
cct env my-profile claude --model haiku --verbose --continue
```

This follows the Unix principle: design interfaces that compose with software you've never heard of. `cct env` doesn't know about `claude` flags, and `claude` doesn't know about `cct` — they just work together through the shell.

## Keybindings

### Normal Mode

| Key | Action |
|-----|--------|
| `j` / `Down` | Next profile (within active backend tab) |
| `k` / `Up` | Previous profile (within active backend tab) |
| `Tab` | Switch backend tab (Claude → Codex → Kimi) |
| `1` | Switch to Claude tab |
| `2` | Switch to Codex tab |
| `3` | Switch to Kimi tab |
| `Enter` | Launch selected profile |
| `c` | Launch selected Claude profile with `--continue` (one-shot resume) |
| `s` | Toggle `skip_permissions` (Claude) or `full_auto` (Codex) on selected profile |
| `t` | Toggle auth type between `ANTHROPIC_API_KEY` and `ANTHROPIC_AUTH_TOKEN` (Claude only) |
| `Space` | Toggle `max_context_size` between `1m` and `260k` (Kimi only) |
| `a` | Open inline add-profile form (uses active backend tab) |
| `d` | Duplicate the selected profile (appends `_copy` to name) |
| `e` | Edit the selected profile inline |
| `q` / `Ctrl-C` | Quit |

### Add Form Mode

| Key | Action |
|-----|--------|
| `Tab` / `Down` | Next field |
| `Shift-Tab` / `Up` | Previous field |
| `Enter` | Advance to next field / confirm on last field |
| `Esc` | Cancel and return to normal mode |
| `y` | Save (when confirming) |
| `n` / `Esc` | Back (when confirming) |

## Build & Test

```bash
cargo build                    # debug build
cargo build --release          # release build
cargo test                     # all tests
cargo clippy                   # lint
cargo test --test integration  # E2E (mock)
CCT_LIVE_TESTS=1 cargo test --test live  # E2E (live, needs claude binary)
bats tests/install.bats        # shell tests for install.sh (requires bats-core)
```

## Architecture

Five focused modules, no shared mutable state:

| Module | Responsibility |
|--------|----------------|
| `config` | TOML deserialization, default config bootstrap, profile append/update with backend-specific env-var generation, `toggle_skip_permissions` / `toggle_auth_type` / `toggle_full_auto` / `toggle_kimi_max_context_size` via `toml_edit` |
| `app` | Cursor state, backend-filtered navigation, `AppMode` (Normal / AddForm), `FormState` with backend-aware `to_new_profile()` as single source of truth |
| `ui` | ratatui rendering, tab bar + 35/65 split layout, value masking, backend-aware form labels |
| `launch` | CLI arg building for Claude, Codex, and Kimi, exec-replace, codex proxy config, kimi `config.toml` generation, editor open, autoinstall checks |
| `cli` | `cct add` interactive flow (6 prompts, masked summary, duplicate guard, `--backend` flag), `cct run` interactive picker, `cct env` profile env injection |

See [ARCHITECTURE.md](ARCHITECTURE.md) for details.

## License

MIT
