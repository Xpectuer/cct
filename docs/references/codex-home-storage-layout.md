---
title: "Reference: Codex CODEX_HOME Storage Layout"
doc_type: reference
brief: "What Codex stores under the shared default CODEX_HOME (~/.codex) in cct, with special focus on sqlite state and log databases"
confidence: verified
created: 2026-04-03
updated: 2026-08-02
revision: 2
---

# Reference: Codex CODEX_HOME Storage Layout

## Purpose

This document records what Codex keeps under its default `CODEX_HOME` (`~/.codex`) —
`cct` no longer sets a per-profile `CODEX_HOME` — and what the observed sqlite files
there most likely do.

The goal is to make future Codex-backend work less guessy when it needs to inspect,
preserve, or reason about Codex state shared across profiles.

## Verified Launch Boundary

`cct` itself does not implement sqlite persistence for Codex state, and no longer writes
any Codex config or auth files.

What `cct` does at launch time is:

1. Ensure the local `cct proxy` daemon is running (spawn it if unhealthy)
2. Switch the proxy to the profile's upstream (`base_url` / `OPENAI_API_KEY` / `model`)
3. Pass the custom provider via 6 inline `--config` flags (`build_codex_proxy_config_args`)
4. `exec` into `codex`

This is verified in `src/launch.rs`:

- `exec_codex()` dispatches to `exec_codex_proxy` (API key) or `exec_codex_subscription` (OAuth)
- `build_codex_proxy_config_args(model, port)` injects `model_provider=custom`,
  `model=<model>`, and the `model_providers.custom.*` settings as `--config` flags
- `CODEX_HOME` is never set — Codex uses its default directory, so the effective path is:

```text
~/.codex
```

## Observed Files Under ~/.codex

A sampled Codex home directory (shared by all profiles) contained:

```text
config.toml
auth.json
history.jsonl
session_index.jsonl
logs_1.sqlite
state_5.sqlite
version.json
log/codex-tui.log
agents/
rules/
skills/
sessions/
memories/
shell_snapshots/
tmp/
```

This layout shows that the shared `CODEX_HOME` is a full Codex workspace containing
conversation metadata, logs, agent assets, and local state — shared by every `cct`
profile (same provider, same directory).

## SQLite Files

### `logs_1.sqlite`

Verified facts:

- Contains `_sqlx_migrations` and `logs`
- `logs` columns include:
  - `ts`, `ts_nanos`
  - `level`
  - `target`
  - `feedback_log_body`
  - `module_path`, `file`, `line`
  - `thread_id`
  - `process_uuid`

Most likely role:

- Structured runtime log store for Codex internals
- Supports filtering by thread, process, and time
- Used for diagnostics, debugging, or TUI log views rather than user conversation content

Why this interpretation is strong:

- The schema is log-shaped rather than message-shaped
- There are indexes on timestamp and `thread_id`
- The same-named database exists in global `~/.codex/`

### `state_5.sqlite`

Verified facts:

- Contains:
  - `threads`
  - `thread_spawn_edges`
  - `thread_dynamic_tools`
  - `jobs`
  - `agent_jobs`
  - `agent_job_items`
  - `stage1_outputs`
  - `backfill_state`
  - `logs`
  - `_sqlx_migrations`
- `threads` columns include:
  - `rollout_path`
  - `cwd`
  - `title`
  - `sandbox_policy`
  - `approval_mode`
  - `tokens_used`
  - `archived`
  - `git_sha`, `git_branch`, `git_origin_url`
  - `cli_version`
  - `first_user_message`
  - `agent_nickname`, `agent_role`, `agent_path`
  - `memory_mode`
  - `model`, `reasoning_effort`

Most likely role:

- Primary structured state database for Codex threads and agent execution metadata
- Tracks thread-level context that complements, rather than replaces, `history.jsonl`
- Persists parent/child thread relationships for spawned agents
- Persists dynamic tool registrations attached to a thread
- Reserves tables for queued or batched job execution

Why this interpretation is strong:

- `threads` is clearly a session metadata table, not a raw transcript table
- `thread_spawn_edges` directly encodes subthread lineage
- `thread_dynamic_tools` matches dynamic tool injection semantics
- `jobs` and `agent_jobs` are orchestration-oriented tables

## Relationship To JSONL Files

The adjacent flat files suggest a split storage model:

- `history.jsonl` likely stores append-only conversation or event history
- `session_index.jsonl` likely stores a lightweight lookup/index layer
- `state_5.sqlite` stores normalized thread and orchestration metadata
- `logs_1.sqlite` stores structured runtime diagnostics

This means future tooling should not assume that conversation history lives only in one
place. Codex appears to split transcript-like data and stateful metadata across JSONL and
sqlite.

## Important Constraint For cct Work

`cct` should treat these sqlite files as Codex-owned internal state unless a change is
explicitly designed around a verified Codex contract.

Practical implications:

- Do not hand-edit these sqlite files to implement user-facing features
- Prefer `profiles.toml` as the source of truth for launcher-owned settings
- Do not write Codex config/auth files — provider config is injected via `--config` flags
  and the API key lives inside the proxy (switched over the control socket), so there is
  nothing to persist
- Treat sqlite schema details as implementation clues, not a stable external API

This follows the same rule already learned for `auth.json`: external-tool on-disk formats
must be verified from real behavior, not guessed.

## Session Visibility (resume)

Because `cct` never sets `CODEX_HOME`, session state lives in the shared `~/.codex` and
`codex resume` follows the CLI's own filtering rules:

- `codex resume` lists only sessions whose `model_provider_id` matches the current
  provider **and** whose `cwd` matches the current working directory
- `codex resume --all` bypasses the cwd filter but cannot disable the provider filter
- an explicit `codex resume <session-id>` bypasses both filters
- same-provider sessions are visible across `cct` profiles — all profiles physically
  share `~/.codex` (no per-profile `CODEX_HOME`)

## Related

- `src/launch.rs` — Codex launch boundary (proxy mode `--config` flags; `CODEX_HOME` untouched)
- `docs/references/codex-backend-development-guide.md` — Codex backend launch contract
- `docs/lessons/external-tool-config-schema-must-be-verified.md` — why Codex file formats
  should be verified from working examples
