---
title: "Spec: Codex conversation history shared across profiles"
doc_type: proc
brief: "Design spec for sharing Codex conversation history via a shared CODEX_HOME and the official --profile mechanism, with two-way binding between on-disk config and profiles.toml"
confidence: verified
created: 2026-04-02
updated: 2026-08-01
revision: 2
---

# Spec: Codex conversation history shared across profiles

## Chosen Approach

All Codex profiles share a single `CODEX_HOME` directory. Per-profile launch
configuration moves into Codex's official profile layer (`--profile <name>` +
`$CODEX_HOME/<name>.config.toml`, the mechanism Codex 0.134.0+ supports), and API
keys flow through `model_providers.custom.env_key` instead of a cct-written
`auth.json`. Conversation history, session metadata (SQLite), and Memories are
therefore shared by construction — no file linking, no copying, no artifact list
to maintain.

Two-way binding between on-disk Codex config and `profiles.toml`:

- **Forward**: `profiles.toml` is the source of truth for the keys cct owns. Every
  launch refreshes those keys in the on-disk config (surgical merge).
- **Reverse**: if a user hand-edits a cct-owned key in the on-disk config so it
  diverges from `profiles.toml`, the TUI presents a conflict dialog before launch
  and the chosen side is written back to the other.

Legacy per-profile `CODEX_HOME` directories from the previous layout are migrated
automatically: history-bearing artifacts are moved into the shared home once, then
the old directory is left in place (never deleted).

## Alternatives Considered

### 1. Share the entire `CODEX_HOME`

Rejected. This is what the chosen approach does for state, but the launch config
(provider, model, base URL) must stay profile-specific — hence the official
`--profile` overlay instead of one shared `config.toml`.

### 2. Keep per-profile `CODEX_HOME` and copy history on launch

Rejected. Codex history is not one file; it spans `history.jsonl`,
`session_index.jsonl`, `state_*.sqlite`, `sessions/`, and `archived_sessions/`.
Copy-based sync creates drift and ordering problems around indexes and session
stores.

### 3. Per-profile `CODEX_HOME` with symlinked history artifacts (earlier revision)

Rejected in revision 2. Local inspection of a real Codex home (0.144.6) showed
this design is unsound:

- `state_5.sqlite.threads.rollout_path` stores **absolute paths** rooted at the
  launching profile's `CODEX_HOME`. With per-profile homes plus a shared symlinked
  `archived_sessions/`, threads indexed by profile A point into A's home path,
  which profile B's home does not resolve.
- SQLite WAL mode (`state_5.sqlite-wal` / `-shm`) is not multi-home-safe.
- The shared artifact list is not exhaustive: a real home also contains
  `memories_1.sqlite`, `goals_1.sqlite`, `sqlite/`, `.codex-global-state.json`,
  and more — any future Codex file silently breaks history sharing.
- Symlinks are a poor fit for the Windows support goal (#4).

### 4. Official profile mechanism with shared `CODEX_HOME`

Chosen. One shared home makes all state shared by construction; the official
`--profile` overlay keeps launch config per-profile. Verified against the Codex
config reference (learn.chatgpt.com/docs/config-file/config-advanced): profile
files live at `$CODEX_HOME/<name>.config.toml`, selected via `--profile <name>`;
the legacy `[profiles.<name>]` tables and `profile = "..."` selector are
unsupported in 0.134.0+.

## Design

### Layout

```text
~/Library/Application Support/cc-tui/codex/        ← CODEX_HOME (shared; Linux: ~/.config/cc-tui/codex)
├── config.toml            ← shared base config, cct-owned keys + user edits preserved
├── <name>.config.toml     ← per-profile overlay, rewritten from profiles.toml each launch
├── auth.json              ← shared; cct no longer writes it (key comes from env_key)
├── history.jsonl          ← shared by construction
├── state_*.sqlite         ← shared by construction
├── sessions/              ← shared by construction
└── archived_sessions/     ← shared by construction
```

### launch.rs — split layout resolution from side effects

Add a pure helper `resolve_codex_layout(profile_name: &str) -> CodexLayout`
returning:

- `shared_home` — the single `CODEX_HOME` (no profile name segment)
- `overlay_path` — `shared_home/<profile_name>.config.toml`
- `legacy_home` — the old per-profile directory `shared_home/<profile_name>/` if
  it still exists (migration source)

This helper stays pure so the directory contract is testable without filesystem
writes.

### launch.rs — per-profile overlay instead of per-home config

`generate_codex_config` keeps writing the shared base `config.toml` (surgical
merge; `model_provider = "custom"`, `[features] default_mode_request_user_input`
when absent, as today).

New effectful helper `write_codex_profile_overlay(profile, shared_home)` writes
`<name>.config.toml`:

```toml
model = "<model>"

[model_providers.custom]
name = "<profile name>"
base_url = "<base_url>"
env_key = "OPENAI_API_KEY"
```

- Surgical merge via `toml_edit`: cct-owned keys (`model`,
  `model_providers.custom.{name,base_url,env_key}`) are refreshed; user edits to
  other keys survive.
- `requires_openai_auth` is dropped; the API key is read from the environment
  (`OPENAI_API_KEY`, already injected from `profile.env` by `exec_codex`).
- `write_codex_auth` is deleted — cct no longer manages `auth.json`. Codex falls
  back to shared `auth.json`/keychain when no key is present, which also makes
  ChatGPT login state shared (desirable).

### launch.rs — exec_codex orchestration

1. Check `codex` is installed
2. Resolve layout
3. Migrate legacy history artifacts (once)
4. Write base config + per-profile overlay
5. Inject profile env
6. Set `CODEX_HOME` to the shared home
7. Append `--profile <name>` to the launch args (`build_codex_args`)
8. Exec-replace with `codex`

### Two-way binding — conflict detection and dialog

Pure helper `diff_cct_owned_keys(profile, overlay_doc) -> Vec<KeyDiff>` compares
the cct-owned overlay keys against the current `profiles.toml` values. Called from
the TUI **before** dispatch on Enter (not inside `exec_codex`, which exec-replaces
and cannot return to the UI).

If any `KeyDiff` exists, the app enters a new mode `AppMode::ConflictConfirm`
instead of launching, showing each divergent key with both values:

| Key | profiles.toml | on-disk overlay |
|-----|---------------|-----------------|
| model | gpt-4.1 | gpt-5.6 |

Key choices (footer-hinted, per the hotkey discoverability rule):

- `p` — **profiles.toml wins**: overlay is regenerated from `profiles.toml`;
  launch proceeds immediately.
- `d` — **on-disk wins**: overlay value is written back into `profiles.toml`
  (via the existing `toml_edit` update path in `config.rs`), then launch proceeds.

### Migration of legacy per-profile homes

When `legacy_home` exists, move history-bearing artifacts into `shared_home`
once:

- Move: `history.jsonl`, `session_index.jsonl`, `state_*.sqlite`
  (with `-wal`/`-shm`), `memories_*.sqlite`, `goals_*.sqlite`, `sqlite/`,
  `sessions/`, `archived_sessions/`, `memories/`
- Skip (stay in legacy dir, harmless): `config.toml`, `auth.json`, `log/`,
  `version.json`, `agents/`, `rules/`, `skills/`, `shell_snapshots/`, `tmp/`,
  caches
- If a target already exists in `shared_home`, skip that artifact (never
  overwrite). The old directory is left in place; a marker file
  `shared_home/.cct-migrated-v1` records completed profiles so migration runs at
  most once per profile.
- Migration decisions live in a pure helper (`plan_migration(legacy_home,
  shared_home) -> Vec<Move>`), side effects in `run_migration`.

## Data Flow

1. User selects a Codex profile and presses Enter
2. TUI runs `diff_cct_owned_keys`; on divergence, shows `ConflictConfirm` and the
   user picks a winner (write-back to the losing side)
3. `exec_codex()` resolves the shared-home layout
4. `plan_migration`/`run_migration` move legacy history once
5. Base `config.toml` + `<name>.config.toml` overlay are (re)generated
6. `CODEX_HOME` is set to the shared home; profile env injected; `--profile`
   appended
7. Codex reads shared history/state but profile-specific launch config

Invariant: shared history artifacts are never rewritten per profile; cct-owned
overlay keys are refreshed from `profiles.toml` on every launch.

## Error Handling

- Failure to create the shared home directory is fatal to launch
- Failure to write base config or overlay is fatal to launch
- Migration: a target that already exists in the shared home is skipped, never
  overwritten; migration errors are fatal (no silent partial state)
- Missing `OPENAI_API_KEY`: no overlay `env_key` change needed — Codex falls back
  to shared `auth.json`/keychain, matching current skip behavior
- No silent fallback to per-profile isolated history

## Testing

### Pure layout / diff / migration-plan tests

- `resolve_codex_layout_returns_shared_and_overlay_paths`
- `resolve_codex_layout_keeps_profile_name_in_overlay_only`
- `diff_cct_owned_keys_reports_model_divergence`
- `diff_cct_owned_keys_is_empty_when_in_sync`
- `plan_migration_lists_history_artifacts_only`
- `plan_migration_skips_existing_targets`

### Overlay / migration effectful tests (tempdir)

- `write_codex_profile_overlay_writes_model_provider_env_key`
- `write_codex_profile_overlay_preserves_user_edits`
- `run_migration_moves_history_and_marks_profile`
- `run_migration_is_idempotent`

### Conflict dialog (TUI) tests

- `conflict_confirm_renders_both_values`
- `conflict_confirm_p_uses_profiles_toml_and_launches`
- `conflict_confirm_d_writes_back_overlay_value_to_profiles_toml`

Note: main.rs dispatch is exec-replace and not unit-testable; the p/d decision
logic is covered by equivalent seams instead — `apply_on_disk_winner_reloads_and_regenerates`
(reload+regenerate chain), `overlay_winner_writeback_closes_diff` (write-back
round trip), and footer-text assertions for discoverability.

### Cross-module contract tests

- `update_profile → overlay regeneration keeps shared home stable`
- `config write-back (on-disk wins) → profiles.toml updated → next diff empty`

### Docs contract update

- `docs/modules/launch.md`
- `docs/references/codex-backend-development-guide.md`
- `docs/references/codex-home-storage-layout.md`
- any project overview text that still says every Codex profile owns a complete
  `CODEX_HOME`

## Open Questions

- Verify on codex 0.144.6 before implementation: (1) `env_key` + third-party
  `base_url` authenticates without `requires_openai_auth`; (2) nested
  `model_providers.custom` table works in a profile overlay; (3) `--profile`
  composes with `--full-auto`.
- Whether `history.jsonl` and `session_index.jsonl` are still maintained when the
  shared home already has state — expected yes, as they are per-CODEX_HOME files.

## References

- learn.chatgpt.com/docs/customization/overview
- learn.chatgpt.com/docs/config-file/config-basic
- learn.chatgpt.com/docs/config-file/config-advanced (profiles, `env_key`,
  `sqlite_home`, `[history]`)
- learn.chatgpt.com/docs/config-file/config-reference
- learn.chatgpt.com/docs/config-file/environment-variables (`CODEX_HOME`,
  `CODEX_SQLITE_HOME`)
- `docs/references/codex-home-storage-layout.md` — observed `state_5.sqlite`
  schema and `rollout_path` absolute paths
