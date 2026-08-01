---
title: "TDD: Codex shared history across profiles"
doc_type: proc
status: abandoned
source: "docs/drafts/intake-codex-conversation-history-20260402144823"
brief: "TDD session for shared CODEX_HOME + --profile overlay + two-way binding + migration"
test_cmd: "cargo test"
full_test_cmd: "cargo test"
yields_from:
  - tdd-codex-shared-history-20260801023840_plan.md
created: 2026-08-01
updated: 2026-08-01
revision: 2
---

# Codex Shared History - TDD Session

> **⚠️ ABANDONED (2026-08-01)**: 本 TDD 会话所依赖的 plan/spec 基于旧版代码编写（含 `generate_codex_config` / `write_codex_auth` / per-profile `CODEX_HOME`）。代码库大幅更新（2026-07-13 共享 `~/.codex` 重构 + `--config` flags、2026-08-01 P0 修复）后，plan 的 old anchor 全部失效，且核心目标（共享 history）已在现行代码中实现。重审结论见 `docs/drafts/intake-codex-conversation-history-20260402144823/review.md`（revision 3）。测试 0/25，未进入 Green 阶段，无代码变更需要回滚。

**Started**: 2026-08-01 02:39
**Abandoned**: 2026-08-01
**Plan**: `./tdd-codex-shared-history-20260801023840_plan.md`

## Test Cases

Depends On 列引用 **plan 步骤号**（继承自 Execution Order YAML 的 DAG 依赖）。
测试策略遵循 plan/verification.md（权威）：纯函数直接断言、tempdir 隔离 I/O、
`CCT_CONFIG` + `#[serial]` 序列化隔离、无 live codex 依赖（spec Open Questions
3 项实测在 Step 1 前手动完成）。

| # | Test Case | Tier | Plan Section | Target File(s) | Depends On | Red | Green | Refactor |
|---|-----------|------|--------------|----------------|------------|-----|-------|----------|
| 1 | apply_on_disk_winner_reloads_and_regenerates（回写→重载→重生成闭环，launch→config→launch） | integration | Step 9 | `src/launch.rs` | [4, 8] | [ ] | [ ] | [ ] |
| 2 | update_codex_api_key_reaches_env（update_profile → profile.env 数据流） | integration | Step 13 | `src/launch.rs` | [8, 9] | [ ] | [ ] | [ ] |
| 3 | update_codex_model_reaches_overlay（update_profile → overlay 产物） | integration | Step 13 | `src/launch.rs` | [8, 9] | [ ] | [ ] | [ ] |
| 4 | overlay_winner_writeback_closes_diff（diffs→回写→重生成→diff 空） | integration | Step 13 | `src/launch.rs` | [8, 9] | [ ] | [ ] | [ ] |
| 5 | resolve_codex_layout_returns_shared_and_overlay_paths | unit | Step 1 | `src/launch.rs` | [] | [ ] | [ ] | [ ] |
| 6 | resolve_codex_layout_keeps_profile_name_in_overlay_only | unit | Step 1 | `src/launch.rs` | [] | [ ] | [ ] | [ ] |
| 7 | plan_migration_lists_history_artifacts_only | unit | Step 2 | `src/launch.rs` | [1] | [ ] | [ ] | [ ] |
| 8 | plan_migration_skips_existing_targets | unit | Step 2 | `src/launch.rs` | [1] | [ ] | [ ] | [ ] |
| 9 | run_migration_moves_history_and_marks_profile | unit | Step 2 | `src/launch.rs` | [1] | [ ] | [ ] | [ ] |
| 10 | run_migration_is_idempotent | unit | Step 2 | `src/launch.rs` | [1] | [ ] | [ ] | [ ] |
| 11 | write_codex_profile_overlay_writes_model_provider_env_key | unit | Step 4 | `src/launch.rs` | [3] | [ ] | [ ] | [ ] |
| 12 | write_codex_profile_overlay_preserves_user_edits | unit | Step 4 | `src/launch.rs` | [3] | [ ] | [ ] | [ ] |
| 13 | write_codex_profile_overlay_escapes_profile_values | unit | Step 4 | `src/launch.rs` | [3] | [ ] | [ ] | [ ] |
| 14 | apply_overlay_winner_writes_model_and_base_url | unit | Step 8 | `src/config.rs` | [] | [ ] | [ ] | [ ] |
| 15 | apply_overlay_winner_preserves_other_fields | unit | Step 8 | `src/config.rs` | [] | [ ] | [ ] | [ ] |
| 16 | apply_overlay_winner_unknown_key_ignored | unit | Step 8 | `src/config.rs` | [] | [ ] | [ ] | [ ] |
| 17 | diff_cct_owned_keys_reports_model_divergence | unit | Step 9 | `src/launch.rs` | [4, 8] | [ ] | [ ] | [ ] |
| 18 | diff_cct_owned_keys_is_empty_when_in_sync | unit | Step 9 | `src/launch.rs` | [4, 8] | [ ] | [ ] | [ ] |
| 19 | diff_cct_owned_keys_ignores_fixed_keys | unit | Step 9 | `src/launch.rs` | [4, 8] | [ ] | [ ] | [ ] |
| 20 | read_overlay_diffs_none_when_missing | unit | Step 9 | `src/launch.rs` | [4, 8] | [ ] | [ ] | [ ] |
| 21 | enter_conflict_holds_profile_idx_and_diffs（AppMode 模式转换） | unit | Step 10 | `src/app.rs` | [8] | [ ] | [ ] | [ ] |
| 22 | conflict_confirm_renders_both_values（footer 含 `[p]`/`[d]`） | unit | Step 11 | `src/ui.rs` | [10] | [ ] | [ ] | [ ] |
| 23 | generate_codex_config_* 5 个既有测试改新签名（base 无 model/custom 断言） | unit | Step 3 | `src/launch.rs` | [1] | [ ] | [ ] | [ ] |
| 24 | build_codex_args_* 5 个既有测试期望前插 `--profile` | unit | Step 6 | `src/launch.rs` | [1] | [ ] | [ ] | [ ] |
| 25 | write_codex_auth_* 8 个既有测试删除（Step 13 重建/新增 3 个契约测试） | unit | Step 7 | `src/launch.rs` | [5] | [ ] | [ ] | [ ] |

## Agent Tool Log

| # | Case | Outcome | Notes | Timestamp |
|---|------|---------|-------|-----------|

## Status

**Status**: ABANDONED (2026-08-01) — 上游 plan 因代码演进失效，见文首说明
**Current case**: 1 / 25
**Progress**: 0% (0/25 complete)
**Blocked**: None

---
**Updated**: 2026-08-01
