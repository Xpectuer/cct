# CLAUDE.md Snapshot

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`cct` is a terminal UI launcher for Claude Code and OpenAI Codex. It reads named profiles from a TOML config file (`~/.config/cc-tui/profiles.toml`), displays them in a ratatui TUI organized into Claude/Codex backend tabs, and exec-replaces the process with `claude <args>` or `codex [--full-auto]` when the user selects a profile.

## Build & Test Commands

```bash
cargo build           # debug build
cargo build --release # release build
cargo test            # run all tests
cargo test <name>     # run a single test by name (e.g. cargo test build_args_full)
cargo clippy          # lint
cargo run             # run the TUI locally

# E2E (mock — no real claude needed)
cargo test --test integration

# E2E (live — requires `claude` binary installed)
CCT_LIVE_TESTS=1 cargo test --test live

# Shell tests for install.sh (requires bats-core)
bats tests/install.bats
```

## Architecture

The app is five focused modules with no shared mutable state:

| Module | File | Responsibility |
|--------|------|----------------|
| `config` | `src/config.rs` | Deserialize `profiles.toml`; `Backend` enum; `validate_profiles`; write default config; append new profiles with backend-specific env generation; toggle skip_permissions via toml_edit |
| `app` | `src/app.rs` | Cursor state, `active_backend`, `filtered_indices()`, `switch_backend()`, `AppMode` (Normal/AddForm), `FormState` with `to_new_profile()` as single source of truth |
| `ui` | `src/ui.rs` | ratatui rendering — tab bar + 35/65 split filtered list+detail panel + footer; backend-aware `build_form_lines`; masks sensitive env vars |
| `launch` | `src/launch.rs` | `build_launch_command` dispatch; `exec_claude`/`exec_codex`; `generate_codex_config`; `exec()` process replace; open `$EDITOR`; check/install claude binary |
| `cli` | `src/cli.rs` | `cct add` interactive CLI flow — 5 prompts, masked summary, duplicate guard (Claude profiles only) |

**Data flow:** `main` checks backend binaries → loads + validates profiles → creates `App` → draw loop → on Enter dispatches to `launch::exec_claude` or `launch::exec_codex` based on `profile.backend`.

**Key design choices:**
- `exec` (not `spawn`) is used so the target CLI inherits the terminal cleanly; there is no return path on success.
- `ui::mask_value` redacts any env key containing `TOKEN`, `KEY`, or `SECRET`.
- Config hot-reload on `e`: editor opens, then profiles are re-parsed in-place without restart.
- `FormState::to_new_profile()` is the single source of truth for form-field-index → semantic mapping per backend.
- Codex launch: `generate_codex_config` writes `~/.config/cct-tui/codex/config.toml` from profile fields; `CODEX_HOME` is set before exec.
- `s` key toggles `skip_permissions` on the selected Claude profile; persisted via `toml_edit` to preserve comments.
- On startup, if `claude` is missing, `prompt_install()` offers to run the official installer before entering raw mode.
