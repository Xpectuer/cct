---
title: "Reference: Codex CODEX_HOME Storage Layout"
doc_type: reference
brief: "What Codex stores under per-profile CODEX_HOME in cct, with special focus on sqlite state and log databases"
confidence: verified
created: 2026-04-03
updated: 2026-04-03
revision: 1
---

# Reference: Codex CODEX_HOME Storage Layout

## Purpose

This document records what `cct` places in a Codex profile's `CODEX_HOME` and what the
observed sqlite files there most likely do.

The goal is to make future Codex-backend work less guessy when it needs to inspect,
preserve, or reason about profile-local Codex state.

## Verified Launch Boundary

`cct` itself does not implement sqlite persistence for Codex state.

What `cct` does at launch time is:

1. Compute a per-profile `CODEX_HOME`
2. Write `config.toml`
3. Write `auth.json`
4. Export `CODEX_HOME`
5. `exec` into `codex`

This is verified in `src/launch.rs`:

- `exec_codex()` builds `dirs::config_dir()/cc-tui/codex/<profile-name>`
- `generate_codex_config()` writes the profile-derived Codex config
- `write_codex_auth()` writes `auth.json`
- `env::set_var("CODEX_HOME", ...)` hands the directory to Codex before `exec`

On macOS, `dirs::config_dir()` resolves to `~/Library/Application Support`, so the
effective path is:

```text
~/Library/Application Support/cc-tui/codex/<profile-name>
```

## Observed Per-Profile Files

A sampled profile directory contained:

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

This layout shows that a profile-local `CODEX_HOME` is not just launch config; it is a
full Codex workspace containing conversation metadata, logs, agent assets, and local
state.

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
- Let launch derive `config.toml` and `auth.json`
- Treat sqlite schema details as implementation clues, not a stable external API

This follows the same rule already learned for `auth.json`: external-tool on-disk formats
must be verified from real behavior, not guessed.

## Related

- `src/launch.rs` — Codex launch boundary and `CODEX_HOME` setup
- `docs/references/codex-backend-development-guide.md` — Codex backend launch contract
- `docs/lessons/external-tool-config-schema-must-be-verified.md` — why Codex file formats
  should be verified from working examples
