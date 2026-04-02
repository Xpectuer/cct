---
title: "Plan: Codex conversation history shared across profiles"
doc_type: proc
brief: "Implementation plan for shared Codex history with isolated profile config"
confidence: verified
created: 2026-04-02
updated: 2026-04-02
revision: 1
---

# Plan: Codex conversation history shared across profiles

## Overview

修复 issue #9：当前 `exec_codex()` 为每个 Codex profile 使用独立 `CODEX_HOME`，导致 conversation history 被 profile 隔离。目标是在不修改 UI 或 profile schema 的前提下，让所有 Codex profile 共享一份历史，同时保留 profile 自身的配置生成与认证隔离。

## Files Changed

| File | Change Type |
|------|-------------|
| `src/launch.rs` | Functional refactor + behavior change |
| `docs/modules/launch.md` | Documentation update |
| `docs/references/codex-backend-development-guide.md` | Documentation update |
| `docs/drafts/intake-codex-conversation-history-20260402144823/spec.md` | Design record |
| `docs/drafts/intake-codex-conversation-history-20260402144823/review.md` | Review record |

## Step 1 — Confirm the shared-history artifact boundary

**File**: `src/launch.rs` design notes + local validation

**What**: Derive the initial list of Codex history artifacts that must be shared across profiles.

**Details**:
- Start from verified local Codex home contents
- Treat `history.jsonl`, `session_index.jsonl`, `sessions/`, and `archived_sessions/` as the initial explicit shared set
- If implementation or manual verification shows user-visible history also depends on SQLite state, extend the artifact set deliberately rather than sharing the whole home

**Verify**:
- Local inspection notes match the final artifact list in code comments or docs

## Step 2 — Introduce a pure Codex layout helper

**File**: `src/launch.rs`

**What**: Add a pure helper such as `resolve_codex_layout(profile_name: &str) -> CodexLayout`.

**Details**:
- Return profile runtime path, shared history path, and active home path
- Remove direct path stitching from `exec_codex()`
- Keep the helper pure and unit-testable

**Verify**:
- New unit tests assert the returned path structure without touching the filesystem

## Step 3 — Add composed-home preparation logic

**File**: `src/launch.rs`

**What**: Add a side-effectful helper such as `prepare_codex_home(profile, &layout) -> Result<()>`.

**Details**:
- Create required directories
- Write profile-specific `config.toml`
- Write profile-specific `auth.json`
- Establish shared mappings for the explicit history artifact list
- Make repeated launches idempotent when the expected mappings already exist
- Fail on unsafe conflicts instead of overwriting unknown files

**Verify**:
- Tempdir tests cover correct file creation, shared mapping, idempotency, and conflict failure

## Step 4 — Refactor `exec_codex()` orchestration

**File**: `src/launch.rs`

**What**: Update `exec_codex()` to orchestrate the new flow.

**Details**:
- Keep installed-binary check unchanged
- Resolve layout
- Prepare the composed home
- Set `CODEX_HOME` to the active home
- Inject env vars and exec
- Preserve current `write_codex_auth()` behavior when no API key exists

**Verify**:
- Existing launch tests still pass
- New tests confirm the new helper sequence indirectly through side-effect boundaries

## Step 5 — Update launch and backend documentation

**Files**:
- `docs/modules/launch.md`
- `docs/references/codex-backend-development-guide.md`

**What**: Rewrite outdated statements that say every Codex profile gets its own complete `CODEX_HOME`.

**Details**:
- Describe the new split between profile runtime config and shared history artifacts
- Document the explicit shared boundary
- Call out any remaining uncertainty if SQLite artifacts turn out to be required

**Verify**:
- `rg -n 'each profile gets its own .*CODEX_HOME|per-profile Codex home directory' docs/modules/launch.md docs/references/codex-backend-development-guide.md`

## Step 6 — Run verification

**What**: Run targeted and full Rust validation.

**Verify**:
- `cargo test launch`
- `cargo test`
- `cargo clippy`

## Step 7 — Review

Write `review.md` for this draft, checking:

- design still follows pure-builder / thin-effectful-edge rule
- shared-history boundary is explicit and test-backed
- no UI or schema changes leaked into the implementation
- conflict behavior is fail-fast, not silent overwrite

## Step 8 — Commit

Use `/commit` after implementation and verification.

Suggested message:

`fix: share codex conversation history across profiles`

## Execution Order

Step 1 → Step 2 → Step 3 → Step 4 → Step 5 → Step 6 → Step 7 → Step 8

## Acceptance Criteria

- [ ] Two Codex profiles can see the same conversation history
- [ ] Profile-specific config generation remains isolated
- [ ] Shared-history artifact list is explicit in code and docs
- [ ] Unsafe conflicts in active-home history paths fail fast
- [ ] Launch tests cover layout, mapping, and failure behavior
- [ ] Documentation no longer describes Codex history as per-profile isolated
