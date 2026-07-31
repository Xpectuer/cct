---
title: "Constraints: Codex history shared across profiles"
doc_type: proc
brief: "Acceptance criteria and hard constraints for shared CODEX_HOME + --profile + two-way binding"
confidence: verified
yields_from:
  - spec.md
created: 2026-08-01
updated: 2026-08-01
revision: 1
---

# Constraints

## Acceptance Criteria

| # | Criterion | Verifiable Check |
|---|-----------|-----------------|
| 1 | 两个 Codex profile 启动后看到同一对话历史 | launch profile A 再 launch profile B，两者的 `CODEX_HOME` 相同（`cc-tui/codex`，无 profile 名段）；`history.jsonl`/`state_*.sqlite`/`sessions/` 位于同一目录 |
| 2 | 每 profile 仍用各自 model/provider/base_url | 生成的 `<name>.config.toml` 含该 profile 的 `model`、`model_providers.custom.{name,base_url}`；launch 参数含 `--profile <name>` |
| 3 | 双向绑定：手改 cct-owned overlay 键 → 冲突对话框；选择后回写另一侧 | `diff_cct_owned_keys` 返回分歧；TUI 进入 `ConflictConfirm`；`p` 重新生成 overlay 并 launch；`d` 回写 `profiles.toml` 后 launch；单测覆盖两条路径 |
| 4 | 旧 per-profile 历史自动迁移一次，不覆盖共享根已有目标 | `plan_migration` 只列历史 artifact；`run_migration` 移动后写 `.cct-migrated-v1`；二次运行幂等；共享根已存在目标 → skip |
| 5 | `auth.json` 不再由 cct 写；key 走 `env_key` | `write_codex_auth` 已删除；overlay 含 `env_key = "OPENAI_API_KEY"` 且无 `requires_openai_auth`；无 key 的 profile 不写 auth.json |
| 6 | 自动化测试覆盖全部行为 | `cargo test` 含：layout 纯函数、overlay merge、diff、对话框 dispatch、migration、跨模块 write-back 契约；全部 stub/纯，无 live codex 依赖 |
| 7 | 文档不再声称历史按 profile 隔离 | grep 仓库文档无 "history is isolated per profile" 类表述；launch.md / codex-backend-development-guide.md / codex-home-storage-layout.md / CLAUDE.md 已更新 |

## Hard Constraints

| # | Constraint | Type | Detail |
|---|-----------|------|--------|
| 1 | 所有 Codex profile 共享**恰好一个** `CODEX_HOME` | scope | 新状态只落在 `dirs::config_dir()/cc-tui/codex`；不得再为 profile 建子目录 home |
| 2 | per-profile 配置走官方 overlay：`$CODEX_HOME/<name>.config.toml` + `--profile <name>` | tech | Codex ≥ 0.134.0 机制；`[profiles.*]` table 与 `profile = "..."` selector 已废弃，不得使用 |
| 3 | API key 走 `env_key = "OPENAI_API_KEY"`；cct 停止写 `auth.json` | tech | `requires_openai_auth` 移除；key 由 `exec_codex` 注入的 profile env 提供 |
| 4 | 双向绑定不静默覆盖任何一侧 | behavior | 分歧时必须在 launch 前弹 `ConflictConfirm`；`p`/`d` 二选一后回写另一侧；无第三条静默路径 |
| 5 | TUI 变更限于冲突对话框 | scope | 仅 `AppMode::ConflictConfirm` + 两个决策键（`p`/`d`）+ `Esc` 返回 Normal；不新增其他 UI 行为；新键必须 footer 提示 + 测试（hotkey discoverability 规则） |
| 6 | profile schema（`profiles.toml`）不变 | compat | 不改 `Profile` 字段、不加新字段；`config.rs` 的 update 路径复用 |
| 7 | 纯 helper 与 effectful 分离 | tech | `resolve_codex_layout`/`diff_cct_owned_keys`/`plan_migration` 纯（只读）；`write_codex_profile_overlay`/`generate_codex_config`/`run_migration`/`exec_codex` 做 I/O |
| 8 | surgical merge（toml_edit）保留用户手改 | tech | overlay 与共享 base 都只刷新 cct-owned 键；`[features] default_mode_request_user_input` 缺失才写（#10 行为保持） |
| 9 | 迁移不删除旧目录、不覆盖共享根目标 | behavior | 移动后旧 `<name>/` 原样保留；同名目标存在 → skip 该 artifact；错误 → fail fast |
| 10 | 迁移只跑一次 | behavior | `.cct-migrated-v1` marker 记录已完成 profile |
| 11 | 不引入新依赖 | tech | serde_json / toml_edit 已存在；无新 crate |
| 12 | 只影响 Codex 后端 | scope | Claude launch 路径（`exec_claude`、`generate_claude_env` 等）零改动 |
| 13 | 冲突处理必须在 exec 前 | tech | `exec_codex` 是 exec-replace 无返回路径；diff/对话框在 main.rs Enter dispatch 处完成 |
| 14 | 测试不依赖 live codex | tech | stub/临时目录；真实 codex 行为验证单独列为 MANUAL 前验证（spec Open Questions） |
