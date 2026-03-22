---
title: "Reference: Codex Backend Development Guide"
doc_type: reference
brief: "Implementation contract for the Codex backend in cct: config schema, validation, UI behavior, launch flow, and full_auto toggling"
confidence: verified
created: 2026-03-22
updated: 2026-03-22
revision: 1
---

# Reference: Codex Backend Development Guide

## Purpose

This guide consolidates the Codex-backend design and implementation rules that were developed in:

- `docs/drafts/intake-20260314120000`
- `docs/procs/tdd-codex-backend-20260315222153`
- `docs/drafts/intake-codex-fullauto-toggle-20260315235754`
- `docs/procs/tdd-codex-fullauto-toggle-20260316000832`

The claims below were checked against the current implementation in `src/config.rs`,
`src/app.rs`, `src/ui.rs`, `src/launch.rs`, `src/main.rs`, and `src/cli.rs`.

## Backend Model

`cct` supports two backends:

- `claude` is the default backend when `backend` is omitted from `profiles.toml`
- `codex` is explicitly selected with `backend = "codex"`

Relevant profile fields for Codex:

| Field | Location | Meaning |
|------|----------|---------|
| `backend` | profile | Must be `"codex"` |
| `base_url` | profile | Written into generated Codex `config.toml` |
| `model` | profile | Written into generated Codex `config.toml`; defaults to `gpt-4.1` at launch time if omitted |
| `full_auto` | profile | Enables `codex --full-auto` |
| `extra_args` | profile | Passed through to the `codex` CLI |
| `env.OPENAI_API_KEY` | env block | Injected into the process environment before exec |

Codex deliberately does not use:

- `skip_permissions`
- Claude-specific `ANTHROPIC_*` environment generation
- `--continue`

## Config Invariants

`config::validate_profiles()` enforces backend-specific field legality after TOML deserialization:

- Codex profiles must not set `skip_permissions`
- Claude profiles must not set `full_auto`

This keeps invalid combinations from reaching UI or launch code.

## Profile Append Rules

`config::append_profile()` treats Claude and Codex differently.

For Codex profiles:

- `backend = "codex"` is written because Claude is the implicit default
- `base_url` is written as a profile-level field
- `full_auto` is written as a profile-level boolean when present
- `[profiles.env]` is created only when an API key is present
- the only auto-generated Codex env var is `OPENAI_API_KEY`

For Codex, `base_url` is not mirrored into env vars because launch reads it from the generated
Codex config file instead.

Example:

```toml
[[profiles]]
name = "openai-prod"
backend = "codex"
model = "gpt-5"
base_url = "https://api.openai.com/v1"
full_auto = true

[profiles.env]
OPENAI_API_KEY = "sk-..."
```

## Add-Form Mapping

The add form stays fixed at 5 fields, but the field semantics depend on backend.
`app::FormState::to_new_profile()` is the single source of truth.

Claude field labels:

```text
["Name *", "Description", "Base URL", "API Key", "Model"]
```

Codex field labels:

```text
["Name *", "Base URL", "API Key", "Model", "Full Auto (y/n)"]
```

Codex field-index mapping:

| Index | Label | Output field |
|------|-------|--------------|
| 0 | Name | `name` |
| 1 | Base URL | `base_url` |
| 2 | API Key | `api_key` |
| 3 | Model | `model` |
| 4 | Full Auto (y/n) | `full_auto` |

Codex add-form specifics:

- `description` is always `None`
- `"y"` and `"yes"` map to `Some(true)`
- any other value maps to `Some(false)`
- the form backend is initialized from `app.active_backend` when entering add mode

The standalone CLI flow `cct add` remains Claude-only and always creates:

- `backend = Claude`
- `full_auto = None`

## TUI Behavior

The normal-mode UI is backend-aware.

- The left pane shows a `[Claude] [Codex]` tab bar
- `Tab` toggles active backend
- `1` switches directly to Claude
- `2` switches directly to Codex
- list navigation operates only on profiles matching `app.active_backend`
- the selected cursor is remapped to the first matching profile when switching backends

Codex-specific UI behavior:

- profile rows with `full_auto = true` are rendered in yellow
- the detail panel shows `full_auto: ✓` for Codex profiles
- the footer hint changes to `s: Full-auto` on the Codex tab

Claude-only hotkeys are intentionally not shared with Codex:

- `c` resume applies only to Claude
- `skip_permissions` toggling applies only to Claude

## Runtime Launch Flow

Codex launch is handled by `launch::exec_codex(profile)`.

Sequence:

1. Confirm `codex` is available in `PATH`
2. Build a per-profile Codex home directory at `~/.config/cc-tui/codex/<profile-name>`
3. Generate `config.toml` inside that directory
4. Set `CODEX_HOME` to that directory
5. Inject `profile.env` into the process environment
6. Exec-replace the current process with `codex`

`launch::build_codex_args()` is intentionally narrow:

- adds `--full-auto` when `profile.full_auto == Some(true)`
- appends `extra_args`
- does not add `--model`

## Generated Codex Config

`launch::generate_codex_config(profile, codex_home)` writes:

```toml
model_provider = "custom"
model = "<profile.model or gpt-4.1>"

[model_providers.custom]
name = "<profile.name>"
base_url = "<profile.base_url or empty string>"
requires_openai_auth = true
```

Important implementation detail:

- each profile gets its own `CODEX_HOME` directory under `~/.config/cc-tui/codex/<profile-name>`
- this avoids cross-profile config clobbering

## Persisted Full-Auto Toggle

The initial Codex backend work added `full_auto` support to profile creation and launch.
The follow-up change made `s` symmetric across backends:

- on Claude profiles, `s` toggles `skip_permissions`
- on Codex profiles, `s` toggles `full_auto`

Persistence rules:

- `config::toggle_full_auto(profile_name, new_value)` uses `toml_edit`
- the edit is surgical and preserves comments and surrounding formatting
- after persistence succeeds, the in-memory selected profile is updated immediately so the
  detail panel refreshes in the same session

## Test Coverage Expectations

The Codex backend work established these regression boundaries:

- config parsing and backward compatibility for omitted `backend`
- validation failures for illegal field combinations
- Codex-specific env generation in `append_profile()`
- backend-filtered navigation and backend switching
- backend-specific field labels and form mapping
- tab bar rendering and Codex detail rendering
- `build_codex_args()` combinations
- `generate_codex_config()` content and default model behavior
- launch-command dispatch by backend
- Codex `full_auto` persistence and `s`-key dispatch

Any future Codex backend change should preserve those boundaries or replace them with stricter
coverage.
