---
title: "Requirements: Codex conversation history shared across profiles"
doc_type: proc
brief: "Share Codex conversation history across profiles via shared CODEX_HOME + official --profile mechanism, with two-way binding to profiles.toml"
confidence: verified
created: 2026-04-02
updated: 2026-08-01
revision: 3
source_skill: intake
---

# Requirements: Codex conversation history shared across profiles

> **⚖️ RE-EVAL (2026-08-01, revision 3)** — 本文基于 2026-07-13 之前的代码布局撰写。仓库大幅演进后重新评估（详见 [review.md](review.md) revision 3）：
>
> **问题已被现行代码解决**：所有 Codex profile 共享默认 `~/.codex`（`afc1d11`，07-13 重构，配置经 `--config` CLI flags 传入），conversation history 不再按 profile 隔离；cct 不再写 `auth.json`（`107cecf`）；API key 经 profile env → proxy 注入（`9a09b39`）。
>
> **剩余增量逐项评估**：
>
> | 增量 | 现状 | 评估 | 决定 |
> |------|------|------|------|
> | 遗留 `codex-homes/<name>` 历史迁移 | 07-13 前 per-profile CODEX_HOME 布局 | 本机无任何遗留目录（`~/.config/cc-tui/codex-homes/` 不存在），无用户数据可迁移 | **不做** |
> | 双向绑定冲突对话框 | cct 已完全不写 Codex 配置文件，配置走 `--config` flags | "on-disk 值与 profiles.toml 分歧"的前提载体不存在；`profiles.toml` 是唯一事实源，手改走 `e` 热重载 | **不做** |
> | 官方 `--profile` 叠加层 | 当前 `--config` flags 实现同目的 | 每次启动从 `profiles.toml` 生成，无 on-disk 文件需管理；引入 overlay 反而会重新引入 `9a09b39` 修复的"覆盖用户 config"问题 | **不做** |
>
> **验收标准逐条状态**（下文 §5）：AC1（共享 history）、AC2（per-profile 启动配置）、AC5（不再写 auth.json）**已达成**；AC3（冲突对话框）、AC4（遗留迁移）、AC6（对应测试）**前提消失，不适用**；AC7（文档不再声称 history 按 profile 隔离）**未达成**——`docs/modules/launch.md`、`docs/references/codex-home-storage-layout.md`、`docs/references/codex-backend-development-guide.md`、`CLAUDE.md` 仍描述已删除的 per-profile `CODEX_HOME` / `generate_codex_config` 行为。
>
> **结论**：代码增量全部判定不做（KISS），本 draft **关闭**；唯一剩余工作为上述文档陈旧叙述清理（待确认执行）。下文为历史设计文档，仅作参考。

## 1. Problem Statement

`cct` currently treats each Codex profile as if it owns a fully separate `CODEX_HOME`. In practice that isolates conversation history per profile, so a user switching between Codex profiles cannot continue seeing the same prior conversations even when they expect history to be global. Issue #9 documents this as a product mismatch: profile-specific launch settings should stay isolated, but conversation history should not fragment by profile.

## 2. Desired Outcome

All Codex profiles share one conversation-history store while still keeping each profile's launch configuration isolated. Launching profile A and then profile B should expose the same visible Codex history. The shared store is the single `CODEX_HOME`; per-profile launch config moves to Codex's official `--profile` overlay layer; `profiles.toml` and the on-disk Codex config stay two-way bound (divergence is surfaced to the user, never silently resolved). Legacy per-profile homes migrate automatically.

## 3. Constraints
- **Tech stack**: Rust, primarily `src/launch.rs` and `src/app.rs` (conflict dialog), with documentation updates in `docs/modules/` and `docs/references/`
- **Hard boundaries**:
  - All Codex profiles share exactly one `CODEX_HOME`; no per-profile home directories for new state
  - Per-profile launch config lives in `$CODEX_HOME/<name>.config.toml`, selected via `--profile <name>` (Codex 0.134.0+ official mechanism; legacy `[profiles.*]` tables are unsupported)
  - API keys flow through `model_providers.custom.env_key` from profile env; cct stops writing `auth.json`
  - Two-way binding: cct-owned overlay keys are refreshed from `profiles.toml` each launch; when a hand-edited on-disk value diverges from `profiles.toml`, the TUI shows a conflict dialog and the chosen side is written back to the other — no silent overwrite in either direction
  - Migration of legacy per-profile homes is automatic and runs at most once per profile; existing shared-home targets are never overwritten
  - TUI changes are limited to the conflict dialog (`AppMode::ConflictConfirm`, two footer-hinted keys); profile schema unchanged
  - Keep pure path/diff/migration-plan helpers separate from effectful filesystem preparation

## 4. Scope
### In Scope
- One shared `CODEX_HOME` for all Codex profiles
- Per-profile overlay generation (`write_codex_profile_overlay`) with surgical `toml_edit` merge
- `--profile <name>` appended to launch args; `write_codex_auth` removed
- Conflict detection (`diff_cct_owned_keys`) and the TUI `ConflictConfirm` dialog with write-back to `profiles.toml`
- Automatic one-time migration of legacy per-profile history artifacts into the shared home
- Tests: layout, diff, migration-plan (pure); overlay/migration (tempdir); dialog dispatch; cross-module contracts
- Documentation updates (launch.md, codex-backend-development-guide.md, codex-home-storage-layout.md, CLAUDE.md)

### Out of Scope
- Any change to the Claude backend
- Changes to the profile schema in `profiles.toml`
- Symlink/copy-based history sharing
- Broad runtime-state unification beyond what sharing `CODEX_HOME` already provides (caches, logs, skills stay shared by construction)
- Backporting the official profile mechanism to Codex versions < 0.134.0

## 5. Acceptance Criteria
- [ ] Two Codex profiles see the same conversation history after launch
- [ ] Each profile still launches with its own model/provider/base URL (via overlay + `--profile`)
- [ ] Editing a cct-owned key in the on-disk overlay and launching shows a conflict dialog; choosing "on-disk wins" writes the value back to `profiles.toml`, and choosing "profiles.toml wins" regenerates the overlay
- [ ] Legacy per-profile history artifacts migrate to the shared home once, without overwriting existing targets
- [ ] `auth.json` is no longer written by cct; profiles with `OPENAI_API_KEY` authenticate via `env_key`
- [ ] Automated tests cover layout resolution, overlay merge, conflict diff, dialog dispatch, migration, and cross-module write-back
- [ ] Documentation no longer states that Codex history is isolated per profile

---
*Generated by intake skill on 2026-04-02; revised 2026-08-01 (design revision 2: shared CODEX_HOME + official `--profile`)*
*Session log: ./session-log.md*
