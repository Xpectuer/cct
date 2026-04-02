---
title: "Spec: Codex conversation history shared across profiles"
doc_type: proc
brief: "Design spec for sharing Codex conversation history while keeping profile-specific launch config isolated"
confidence: verified
created: 2026-04-02
updated: 2026-04-02
revision: 1
---

# Spec: Codex conversation history shared across profiles

## Chosen Approach

Keep profile-specific Codex runtime configuration isolated, but make all Codex profiles point at the same history artifacts. The implementation introduces a shared history directory and keeps per-profile generated `config.toml` and `auth.json` separate. The launch path composes these two concerns at runtime instead of treating `CODEX_HOME` as a single all-or-nothing ownership boundary.

## Alternatives Considered

### 1. Share the entire `CODEX_HOME`

Rejected. This would guarantee shared history, but it would also merge auth, caches, and other runtime state that the current requirements explicitly keep profile-specific.

### 2. Keep per-profile `CODEX_HOME` and copy history on launch

Rejected. Local inspection shows Codex history is not one file; it spans multiple files and directories. Copy-based sync would create drift, conflict handling, and ordering problems around indexes and session stores.

### 3. Isolated runtime home plus shared history artifacts

Chosen. This keeps the existing launch architecture intact, limits the change to `launch.rs`, and gives an explicit, testable contract for what "shared history" means.

## Design

### launch.rs — split layout resolution from side effects

Add a layout helper, for example `resolve_codex_layout(profile_name: &str) -> CodexLayout`, that returns:

- `profile_runtime_dir`
- `shared_history_dir`
- `active_codex_home`

`profile_runtime_dir` holds profile-owned generated files. `shared_history_dir` holds the common conversation history artifacts. `active_codex_home` is the runtime directory exposed to Codex through `CODEX_HOME`.

This helper must stay pure so the directory contract can be tested without filesystem writes.

### launch.rs — prepare a composed Codex home

Add a side-effectful helper, for example `prepare_codex_home(profile, &layout) -> Result<()>`, responsible for:

1. Creating the shared and active directories
2. Writing profile-specific `config.toml`
3. Writing profile-specific `auth.json`
4. Mapping history-related paths inside `active_codex_home` to `shared_history_dir`

The shared-history mapping must be driven by an explicit artifact list in code, not by sharing the whole directory. Initial candidates, based on local Codex home inspection, are:

- `history.jsonl`
- `session_index.jsonl`
- `sessions/`
- `archived_sessions/`

If implementation or validation proves that additional files such as SQLite state are required for user-visible history continuity, the shared artifact list must be extended deliberately and documented.

### launch.rs — keep `exec_codex()` as orchestration only

`exec_codex()` should continue to do orchestration work only:

1. Check `codex` is installed
2. Resolve layout
3. Prepare the composed home
4. Set `CODEX_HOME`
5. Inject profile env
6. Exec-replace with `codex`

This preserves the repo rule that pure builders stay separate from effectful edges.

## Data Flow

1. User selects a Codex profile and launches it
2. `exec_codex()` resolves the three-path layout
3. `prepare_codex_home()` writes profile config/auth into the active runtime home
4. Shared history artifacts are created or linked into the active home
5. `CODEX_HOME` is set to the active home
6. Codex reads one profile's launch config but the shared conversation history

The important invariant is that profile-specific config is overwritten on every launch, but shared history artifacts are never rewritten per profile.

## Error Handling

- Failure to create the shared history directory is fatal to launch
- Failure to create or map a shared history artifact is fatal to launch
- If an active-home history path already exists but is not the expected shared mapping, launch should fail rather than silently overwrite user data
- Missing `OPENAI_API_KEY` keeps the current behavior: `auth.json` is skipped and Codex decides whether auth is sufficient

The implementation should not silently fall back to per-profile isolated history, because that would make the feature nondeterministic and hard to debug.

## Testing

### launch.rs — pure layout tests

- `resolve_codex_layout_returns_profile_and_shared_paths`
- `resolve_codex_layout_keeps_profile_name_out_of_shared_dir`

### launch.rs — composed-home preparation tests

- `prepare_codex_home_writes_profile_config_and_auth`
- `prepare_codex_home_maps_history_artifacts_to_shared_dir`
- `prepare_codex_home_is_idempotent_when_links_already_exist`
- `prepare_codex_home_keeps_two_profiles_on_same_shared_history`

### launch.rs — conflict and failure tests

- `prepare_codex_home_fails_when_history_file_conflicts_with_existing_regular_file`
- `write_codex_auth_still_skips_when_no_key`
- `prepare_codex_home_fails_when_shared_mapping_cannot_be_established`

### docs — contract update

Update:

- `docs/modules/launch.md`
- `docs/references/codex-backend-development-guide.md`
- any project overview text that still says every Codex profile has its own complete `CODEX_HOME`

## Open Questions

One implementation-time question remains: whether user-visible conversation continuity depends only on the JSONL/session paths above or also on one or more SQLite files. This must be confirmed before finalizing the shared artifact list.
