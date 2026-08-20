---
title: Agent guide
doc_type: guide
brief: Minimal repository map and working rules for coding agents.
updated: 2026-08-20
---
# Agent Guide

`AGENTS.md` is a symlink to this file. Keep this entry point short; follow links only when the task needs them.

## Project

`cct` is a Rust terminal UI and CLI for managing named Claude Code and OpenAI Codex profiles. It reads `profiles.toml`, lets users select or edit profiles, and exec-replaces itself with the selected CLI. Codex profiles can use a local proxy, including Responses API to Chat Completions translation.

## Start Here

1. Read [README.md](README.md) for user-visible behavior and commands.
2. Read [ARCHITECTURE.md](ARCHITECTURE.md) before changing cross-module behavior.
3. Use [docs/README.md](docs/README.md) as the canonical documentation index.
4. Read only the relevant module guide and rules for the task.

Historical material under `docs/drafts/`, `docs/procs/`, `docs/session-cards/`, and timestamped `docs/references/context-*` is evidence, not current specification. Do not bulk-load or update it unless the task explicitly targets history.

Routine `rg` searches skip imported and generated history via `.ignore`. Use `rg --no-ignore` when a task explicitly needs those files.

## Repository Map

| Path | Purpose |
|------|---------|
| `src/` | Product code |
| `tests/` | Integration, live, proxy-contract, and installer tests |
| `scripts/` | Documentation workflow and config-validation helpers |
| `examples/` | Small API usage examples |
| `docs/modules/` | Stable module contracts |
| `docs/rules/` | Project-specific engineering rules |
| `docs/lessons/` | Reusable findings from previous work |
| `docs/drafts/`, `docs/procs/` | Historical design and execution records |
| `docs/references/` | External or captured reference material |

Current Rust modules:

| Module | Responsibility |
|--------|----------------|
| `config` | Profile schema, validation, migration, and surgical TOML edits |
| `app` | TUI state and backend-aware form mapping |
| `ui` | Rendering and secret masking |
| `cli` | Interactive and non-interactive CLI flows |
| `launch` | Command construction, process replacement, and proxy startup |
| `proxy` | Local Codex proxy daemon and control socket |
| `translate` | Responses API / Chat Completions translation |

`main.rs` is the effectful entry point; `lib.rs` exposes the modules for tests.

## Commands

Use `rtk` for compact output.

```bash
rtk cargo test
rtk cargo test --test integration
rtk cargo test --test proxy_contract
rtk cargo clippy --all-targets --all-features
rtk cargo build
```

Optional checks:

```bash
bats tests/install.bats                  # requires bats-core
CCT_LIVE_TESTS=1 cargo test --test live  # requires a real claude binary
```

For commands that access external hosts, first source `~/.zsh/user.zsh`. It enables the local proxy by default; use `unset_proxy` for direct/internal access or `set_clx_proxy` for the Clounix proxy.

## Change Rules

- Read a file before editing it. Preserve unrelated local changes.
- Make the smallest change that satisfies the request; match existing style.
- Do not skip Git pre-commit hooks unless the user explicitly allows it.
- Preserve user-edited TOML structure with `toml_edit`; do not rewrite the whole config.
- Mask values whose keys contain `TOKEN`, `KEY`, or `SECRET` on every display path.
- Backend-dependent mappings need one authoritative function.
- TUI hotkeys require a visible footer hint and a test; review CLI parity for every TUI feature.
- Test external-command boundaries with stubs before live verification.
- Cross-module behavior needs a contract test.
- New feature work must update `README.md`, `ARCHITECTURE.md`, and the relevant module guide.
- Put new documentation under `docs/` and add it to [docs/README.md](docs/README.md).
- Move files to Trash instead of permanently deleting them.

## Verification

Define the observable success condition before coding. Run the narrowest relevant test while iterating, then run the broader affected suite. If a verification method is unclear, stop and ask rather than claiming success.
