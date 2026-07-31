---
title: "Verification: Codex history shared across profiles"
doc_type: proc
brief: "Verification strategy, test plan, and self-review checklist"
confidence: verified
yields_from:
  - spec.md
created: 2026-08-01
updated: 2026-08-01
revision: 2
---

# Verification

## Per-Step Verify Commands

| Step | Verify Command | Expected Result |
|------|---------------|-----------------|
| 1 | `cargo test resolve_codex_layout` | 2 个新测试通过；overlay 路径含 `{name}.config.toml` |
| 2 | `cargo test migration` | 4 个新测试通过（plan 只列历史 artifact、skip 已存在目标、marker 幂等） |
| 3 | `cargo test generate_codex_config` | 更新后 5 个测试通过；base 无 model/model_providers.custom 断言 |
| 4 | `cargo test overlay` | 3 个新测试通过；overlay 含 env_key 且无 requires_openai_auth |
| 5 | `cargo build && cargo test` | 编译 + 全绿（`write_codex_auth` 保留但不再被调用；旧契约测试仍引用它） |
| 6 | `cargo test build_codex_args` | 5 个测试更新后通过，args 前插 `--profile` |
| 7 | `rg "write_codex_auth" src/` + `cargo build && cargo test` | 无残留符号；编译 + 全绿 |
| 8 | `cargo test apply_overlay_winner` | 3 个新测试通过；回写保留其他字段 |
| 9 | `cargo test diff_cct_owned_keys` + `cargo test apply_on_disk_winner` | 4 个 diff 测试 + 回写重载闭环测试通过 |
| 10 | `cargo test enter_conflict` + `cargo build && cargo test` | 模式转换测试（`enter_conflict_holds_profile_idx_and_diffs`）通过；app 现有测试全绿 |
| 11 | `cargo test` | ui 测试含 footer `[p]`/`[d]` 断言 |
| 12 | `cargo build && cargo test` | 全绿；`rg "launch_and_exit\(" src/main.rs` = 3 处调用（另含 1 处 `fn launch_and_exit` 定义） |
| 13 | `cargo test` | 3 个契约测试通过（含回写闭环） |
| 14 | `rg "write_codex_auth" docs/` 空；`rg -l "overlay" docs/modules/launch.md docs/references/` | 文档已更新 |
| 15 | `cargo fmt --check` | 无格式差异 |
| 16 | `cargo test` | 全绿（最终回归） |

## Test Strategy

- **单元（纯）**：`resolve_codex_layout`、`diff_cct_owned_keys`、`plan_migration` 无 I/O 或只读决策，直接断言。
- **单元（tempdir）**：`write_codex_profile_overlay`、`run_migration`、`apply_overlay_winner` 用 `tempfile::tempdir()` 隔离文件系统副作用。
- **跨模块契约**：`update_profile → overlay 产物`（Group C）覆盖 `config → launch` 数据流；`overlay_winner_writeback_closes_diff` 覆盖反向（launch → config）闭环。
- **TUI 行为**：渲染/按键通过 ui.rs footer 文本断言 + app.rs 模式转换测试覆盖；main.rs dispatch 不可单测（exec-replace），由契约测试 + 最终手动 smoke 兜底。
- **序列化隔离**：涉及 `CCT_CONFIG` 的测试用 `#[serial]`（沿用现有模式）。
- **无 live codex 依赖**：所有测试不调用真实 codex 二进制；spec Open Questions 的 3 项实测在 Step 1 前手动完成。

## Self-Review Checklist

| Check | Pass Condition |
|-------|---------------|
| All acceptance criteria covered | 7 条验收标准各自映射 ≥1 步（见 code-spec.md Step 16 表格） |
| No step references later steps | Execution Order YAML 无前向依赖；validate-dag.sh 退出 0 |
| Every step has executable Verify | 各步 Verify 均为可运行命令/断言，无人工判断词 |
| Old anchors unique | 每步 old 锚文本在目标文件中唯一（实现时若重复需精化） |
| Surgical edits only | 无整文件重写步骤；toml_edit merge 贯穿 |
| Pure/effectful separation | 纯函数（layout/diff/plan）与 I/O（prepare/run/write）分步 |
| Secrets masked | diff 对话框只显示 model/base_url（非密钥）；env_key 值不显示 |
| Hotkey discoverable | 冲突对话框 p/d 键在 footer 提示 + 有测试断言 |
| Cross-module contract covered | update_profile → overlay、回写闭环两条数据流均有测试 |
| Docs updated in same changeset | Step 14 覆盖 4 个文档 |
