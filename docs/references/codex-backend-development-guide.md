---
title: "Reference: Codex Backend Development Guide"
doc_type: reference
brief: "Implementation contract for the Codex backend in cct: config schema, validation, UI behavior, launch flow, and full_auto toggling"
confidence: verified
created: 2026-03-22
updated: 2026-08-02
revision: 2
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
| `base_url` | profile | Upstream URL passed to the local proxy (also encoded in the `model_providers.custom.base_url` `--config` flag) |
| `model` | profile | Passed via `--config model=<model>`; defaults to `gpt-4.1` at launch time if omitted |
| `full_auto` | profile | Approval level (`untrusted`/`never`/`danger`) — maps to `--ask-for-approval` or `--dangerously-bypass-approvals-and-sandbox` |
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
- `full_auto` is written as a profile-level string — `untrusted`/`never`/`danger` — when
  present (legacy boolean values still deserialize: `true` → `danger`, `false` → unset)
- `[profiles.env]` is created only when an API key is present
- the only auto-generated Codex env var is `OPENAI_API_KEY`

For Codex, `base_url` is not mirrored into env vars because launch passes it to the proxy
via the control socket (`proxy::switch_profile`) and encodes it in the
`model_providers.custom.base_url` `--config` flag.

Example:

```toml
[[profiles]]
name = "openai-prod"
backend = "codex"
model = "gpt-5"
base_url = "https://api.openai.com/v1"
full_auto = "never"

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
["Name *", "Base URL", "API Key", "Model", "Approval"]
```

Codex field-index mapping:

| Index | Label | Output field |
|------|-------|--------------|
| 0 | Name | `name` |
| 1 | Base URL | `base_url` |
| 2 | API Key | `api_key` |
| 3 | Model | `model` |
| 4 | Approval | `full_auto` |

Codex add-form specifics:

- `description` is always `None`
- `"untrusted"`, `"never"`, and `"danger"` map to the matching approval level
- `"y"` and `"yes"` map to `danger` (backward compatibility)
- any other value maps to `None`
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

- profile rows are colored by approval level: `untrusted` green → `never` yellow →
  `danger` red (unset renders white)
- the detail panel shows `approval: <level>` for Codex profiles (from
  `approval_label`; unset shows `approval: on-request`)
- the footer hint changes to `s: Approval` on the Codex tab

Claude-only hotkeys are intentionally not shared with Codex:

- `c` resume applies only to Claude
- `skip_permissions` toggling applies only to Claude

## Runtime Launch Flow

Codex launch is handled by `launch::exec_codex(profile)`, which dispatches on `auth_type`.

Proxy mode (API key):

1. Confirm `codex` is available in `PATH`
2. Ensure the `cct proxy` daemon is running (`ensure_proxy_running`; spawns `cct proxy start` if unhealthy)
3. Switch the proxy to this profile's upstream via the control socket (`proxy::switch_profile`: `base_url`, `OPENAI_API_KEY`, `model`)
4. Inject `profile.env` into the process environment
5. Exec-replace with `codex` plus the 6 inline `--config` flags from `build_codex_proxy_config_args(model, port)` (custom provider pointing at `http://127.0.0.1:<port>/v1`) and the shared approval/extra args

Subscription mode (`auth_type = "subscription"`):

1. Confirm `codex` is available in `PATH`
2. Set `DISABLE_AUTOUPDATER=1` and inject `profile.env`
3. Exec-replace with `codex --config model_provider=openai [--config model=<model>]` plus the shared approval/extra args — no proxy, native OAuth

`CODEX_HOME` is never set in either mode; all profiles share the default `~/.codex`
history/sessions, and no `config.toml` or `auth.json` is written.

`launch::build_codex_args()` (via `build_shared_codex_args`) is intentionally narrow:

- adds the approval flag matching `profile.full_auto` (`--ask-for-approval never|untrusted` or `--dangerously-bypass-approvals-and-sandbox`)
- appends `extra_args`
- does not add `--model` or provider config — those arrive via `--config` flags

## Codex Provider Configuration (inline --config)

No Codex config file is written. Proxy mode passes 6 inline `--config` flags built by
`launch::build_codex_proxy_config_args(model, port)`:

```text
--config model_provider=custom
--config model=<profile.model or gpt-4.1>
--config model_providers.custom.name=cct-proxy
--config model_providers.custom.base_url=http://127.0.0.1:<port>/v1
--config model_providers.custom.wire_api=responses
--config model_providers.custom.env_key=OPENAI_API_KEY
```

Important implementation details:

- `CODEX_HOME` is left at its default (`~/.codex`) — all profiles share one history/session store, and no per-profile config directory is created
- the old `config.toml` generation and `auth.json` writing were removed; the API key lives inside the proxy process (switched per profile over the control socket) and is injected as the `Authorization` header when forwarding
- the codex API key can be written either in the env block (`env.OPENAI_API_KEY`) or at the profile top level (`api_key = "..."`); both are honored, the top-level shorthand being injected into the env map at deserialization time

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

## Session Visibility (resume)

Codex session state lives in the shared `~/.codex`, so `codex resume` follows the CLI's
own filtering rules:

- `codex resume` lists only sessions whose `model_provider_id` matches the current provider
  **and** whose `cwd` matches the current working directory
- `codex resume --all` bypasses the cwd filter but cannot disable the provider filter
- an explicit `codex resume <session-id>` bypasses both filters
- same-provider sessions are visible across `cct` profiles — all profiles physically share
  `~/.codex` (no per-profile `CODEX_HOME`)

## Test Coverage Expectations

The Codex backend work established these regression boundaries:

- config parsing and backward compatibility for omitted `backend`
- validation failures for illegal field combinations
- Codex-specific env generation in `append_profile()`
- backend-filtered navigation and backend switching
- backend-specific field labels and form mapping
- tab bar rendering and Codex detail rendering
- `build_codex_args()` combinations
- `build_codex_proxy_config_args()` content and default model behavior
- launch-command dispatch by backend
- Codex `full_auto` persistence and `s`-key dispatch

Any future Codex backend change should preserve those boundaries or replace them with stricter
coverage.
