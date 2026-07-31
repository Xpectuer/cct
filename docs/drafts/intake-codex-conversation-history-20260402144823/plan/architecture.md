---
title: "Architecture: Codex history shared across profiles"
doc_type: proc
brief: "File layout, dependency DAG, and execution order for shared CODEX_HOME feature"
confidence: verified
yields_from:
  - spec.md
created: 2026-08-01
updated: 2026-08-01
revision: 1
---

# Architecture

**Strategy**: 所有 Codex profile 共享单一 `CODEX_HOME`（`cc-tui/codex`）；per-profile 配置移入官方 `--profile` 叠加层；key 走 `env_key`；TUI 在 Enter dispatch 前做双向绑定冲突检测（`ConflictConfirm` 模式）；旧 per-profile home 一次性自动迁移。

## Files Changed

| File | Change Type | Group |
|------|-------------|-------|
| `src/launch.rs` | Major edit — layout 解析、迁移、base config 改共享根、overlay 生成、`diff_cct_owned_keys`/`read_overlay_diffs`（纯）、auth 删除、`--profile`、exec_codex 编排、契约测试改造 | A 共享 HOME 启动链 + C 契约 |
| `src/config.rs` | Major edit — `KeyDiff` 结构、落盘胜出回写 `apply_overlay_winner` | B 双向绑定 |
| `src/app.rs` | Major edit — `AppMode::ConflictConfirm` + 冲突状态（KeyDiff 列表、两侧选择） | B 双向绑定 |
| `src/ui.rs` | Major edit — 冲突对话框渲染、footer 键（p/d）提示 | B 双向绑定 |
| `src/main.rs` | Minor edit — Enter dispatch 前冲突预检、对话框按键分发 | B 双向绑定 |
| `src/launch.rs`（tests 模块） | Major edit — 契约测试改造：update_profile → overlay 产物、落盘回写闭环 | C 契约与文档 |
| `docs/modules/launch.md` | Major edit — 新函数接口、布局变化 | C 契约与文档 |
| `docs/references/codex-backend-development-guide.md` | Major edit — 启动流程、auth 机制变化 | C 契约与文档 |
| `docs/references/codex-home-storage-layout.md` | Major edit — 布局从 per-profile 改共享 | C 契约与文档 |
| `CLAUDE.md` / `ARCHITECTURE.md` / `README.md` | Minor edit — 架构表、关键设计点、Launch Codex 流程 | C 契约与文档 |

## Execution Order (overview)

```
Group A (launch.rs 核心)  →  Group B (双向绑定 UI)  →  Group C (契约测试 + 文档)
```

- **A 必须最先**：`diff_cct_owned_keys`（B）依赖 overlay 生成语义；迁移逻辑被 `exec_codex` 编排调用。
- **B 依赖 A**：冲突对话框基于 overlay 生成逻辑与 `build_codex_args` 的新输出；回写 `apply_overlay_winner` 独立于 A（仅 config.rs）。
- **C 依赖 A+B**：契约测试覆盖 A 的产物与 B 的回写闭环；文档描述最终行为。
- 组内步骤线性（每步一个逻辑编辑）；`diff_cct_owned_keys`/`read_overlay_diffs` 是纯函数，定义在 `src/launch.rs`（与 overlay 生成同模块保持键定义单一来源），被 main.rs 在 Enter 预检时调用。
