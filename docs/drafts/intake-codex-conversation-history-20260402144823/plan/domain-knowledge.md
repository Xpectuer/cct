---
title: "Domain Knowledge: Codex history shared across profiles"
doc_type: proc
brief: "Domain entities, terminology, and business rules for shared CODEX_HOME implementation"
confidence: verified
yields_from:
  - spec.md
created: 2026-08-01
updated: 2026-08-01
revision: 1
---

# Domain Knowledge

## Entities

| Entity | Description | Key Attributes |
|--------|-------------|----------------|
| `Profile` | 一个 cct 启动配置（`profiles.toml` 中的 `[[profiles]]` 块） | `name`（唯一，作 overlay 文件名）、`model`、`base_url`、`env`（含 `OPENAI_API_KEY`）、`full_auto`、`extra_args` |
| `CodexLayout`（新） | 路径解析结果，纯函数产物 | `shared_home`（`cc-tui/codex`）、`overlay_path`（`shared_home/<name>.config.toml`）、`legacy_home`（`shared_home/<name>/`，可能不存在） |
| 共享 base `config.toml` | 所有 profile 共用的 Codex 配置 | cct-owned：`model_provider = "custom"`、`[features] default_mode_request_user_input`（缺失才写）；其余键用户手改保留 |
| Profile overlay `<name>.config.toml` | 每 profile 的 Codex 叠加层（`--profile` 选择） | cct-owned：`model`、`model_providers.custom.{name, base_url, env_key}`；每次 launch 从 `profiles.toml` 刷新 |
| 历史 artifact | 共享 HOME 下承载会话状态的路径 | `history.jsonl`、`session_index.jsonl`、`state_*.sqlite`（+`-wal`/`-shm`）、`sessions/`、`archived_sessions/`、`memories/`、`memories_*.sqlite`、`goals_*.sqlite`、`sqlite/` |
| `KeyDiff`（新） | 一个 cct-owned 键的双侧取值 | `key`、`profiles_value`、`overlay_value` |
| `AppMode::ConflictConfirm`（新） | TUI 冲突对话框模式 | 持有 `Vec<KeyDiff>`；`p` = profiles.toml 胜、`d` = 落盘胜 |

## Terminology

| Term | Definition | Avoid |
|------|------------|-------|
| `CODEX_HOME` | Codex 状态根目录（config/auth/history/sessions/sqlite/memories 全部在此）；本设计中所有 profile 共享 | — |
| profile overlay | `$CODEX_HOME/<name>.config.toml`，经 `--profile <name>` 加载的叠加配置层 | 不要把 `[profiles.*]` table（Codex 0.134.0+ 已废弃）与 cct 的 `profiles.toml` 混淆 |
| cct-owned keys | cct 每次 launch 从 `profiles.toml` 刷新的键：`model`、`model_providers.custom.{name,base_url,env_key}`（overlay）；`model_provider`、`[features] default_mode_request_user_input`（base） | 用户手改这些键会触发双向绑定冲突 |
| `env_key` | Codex 官方机制：provider 从命名的环境变量读 API key | 不要与 `experimental_bearer_token`（直接嵌 token）混用 |
| legacy home | 旧布局 `cc-tui/codex/<name>/`（带 profile 名段的 per-profile home），迁移源 | 迁移后保留不删 |
| `.cct-migrated-v1` | 共享根下的 marker 文件，记录已完成迁移的 profile 名，保证迁移只跑一次 | — |
| `requires_openai_auth` | 旧方案标志（provider 用 OpenAI 认证体系）；新方案移除，改 `env_key` | — |

## Business Rules

- **正向绑定**：`profiles.toml` 是 cct-owned 键的事实源；每次 launch 刷新 overlay，共享 base 的 cct-owned 键同样刷新。
- **反向绑定**：overlay 中 cct-owned 键被手改且与 `profiles.toml` 分歧 → 进入 `ConflictConfirm`；`p`（profiles.toml 胜）重新生成 overlay 后 launch，`d`（落盘胜）把 overlay 值写回 `profiles.toml` 后 launch。**任何方向都不静默覆盖**。
- **共享天然性**：同一 `CODEX_HOME` 下 `history.jsonl`/`state_*.sqlite`/`sessions/` 等全部天然共享，不需要文件操作；`state_*.sqlite` 的 `threads.rollout_path` 是绝对路径（根于 `CODEX_HOME`），因此**绝不能**用符号链接做部分共享（路径会错乱）。
- **auth 降级**：`OPENAI_API_KEY` 缺失时 overlay 仍写 `env_key`，Codex 自行退回共享 `auth.json`/keychain（ChatGPT 登录态共享是期望行为）。
- **迁移**：`plan_migration` 只列历史 artifact（见实体表），不列 `config.toml`/`auth.json`/`log/`/`agents/` 等运行时内容；共享根已有同名目标 → skip；迁移错误 → launch 失败（不静默降级为隔离历史）；marker 写完后幂等。
- **exec 无返回**：`exec_codex` 是进程替换，冲突处理（diff + 对话框 + 回写）必须在 main.rs Enter dispatch 阶段、exec 之前完成。
- **`--profile` 组合**：`--profile <name>` 与 `--full-auto`、`extra_args` 可共存（`build_codex_args` 输出序：`--profile` 在前）。
- **旧值废弃**：`write_codex_auth`、`requires_openai_auth = true` 属旧机制，直接删除不留兼容补丁（KISS）。
