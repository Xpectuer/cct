---
title: "Constraints: Codex conversation history isolation re-evaluation"
doc_type: reference
brief: "Expanded constraints for intake-20260801145824"
confidence: speculative
created: 2026-08-01
updated: 2026-08-01
revision: 1
source_skill: intake
---

# Constraints

## Tech Stack

| Component | Version / Detail | Source |
|-----------|-----------------|--------|
| cct | Rust 0.5.0，edition 2021，五模块扁平结构（config/app/ui/launch/cli + proxy） | Cargo.toml + CLAUDE.md |
| codex-cli | 0.146.0（用户本机实测） | `codex --version` |
| profiles.toml | 顶层 `[[profiles]]` + `[profiles.env]`，cct 唯一配置事实源 | CLAUDE.md / config.rs |
| 官方文档 | learn.chatgpt.com 2026-08 版：config-basic / config-advanced#profiles / config-reference / developer-commands#codex-resume | browser-harness 调查（用户给的 `config-file/` URL 已 404 改版） |
| 本地状态 | `~/.codex/`：sessions/（rollout jsonl）、state_5.sqlite（threads 表）、history.jsonl、session_index.jsonl | 只读实测 |

## Hard Boundaries

| Boundary | Detail | Why Non-Negotiable |
|----------|--------|--------------------|
| 共享 `~/.codex` 不回退 | 所有 codex profile 继续共享默认 CODEX_HOME；不得恢复 per-profile CODEX_HOME 或引入新 home | 07-13 架构既定；回退即重新引入 issue #9 原始物理隔离 |
| cct 不写 Codex 配置文件 | 不写 `config.toml` / `profile-*.config.toml` / `auth.json`；配置一律 `--config` flags | `9a09b39` 修复的"覆盖用户 config"bug 的教训；用户 config 是用户资产 |
| 不手动编辑 Codex 内部状态 | 不编辑/合并/迁移 sqlite、rollout、history.jsonl | 外部工具内部契约未公开且版本变动频繁（codex-home-storage-layout.md 已确立） |
| 尊重官方 resume 语义 | 不改变"resume 按仓库过滤"；跨仓库查看走官方 `--all` | 官方产品语义；违反即引入隐私泄漏与噪音 |
| 不新增 schema | profiles.toml schema 不变 | KISS + 上轮 review 结论 |

## Domain Invariants

See [domain-model.md §Domain Invariants](./domain-model.md) for the full list. 直接影响实现的：
1. 会话物理存储唯一（共享 `~/.codex` 已满足，任何改动不得破坏）；
2. 配置层与状态层正交（若未来启用 `--profile`，不得以此改变会话可见性）；
4. cct 不拥有 Codex 内部状态。

## Relevant Rules

| Rule | Why Relevant |
|------|-------------|
| test-boundaries-with-stubs-before-manual-verification | 任何 cct 层改动（如 resume 入口）先 stub 测试，manual 仅作最后兜底 |
| external-tool-config-schema-must-be-verified | 本轮官方机制全部经官方文档 + 本地实测验证，未凭猜测（教训：auth.json 大小写猜错过） |
| preserve-user-edited-config-structure | 用户 `~/.codex/config.toml` 与 `profiles.toml` 均不得整文件重写 |
| pure-builders-thin-effectful-edges | 若新增 resume 参数构建，保持纯函数 + 窄 exec 边界 |
| cross-module-features-need-contract-tests | 若改动跨 launch/app/ui（如新热键），需契约测试 |
| hotkey-ui-changes-must-be-discoverable | 若新增 resume 热键，必须有 footer 提示 + 测试 |
| update-docs-after-new-feature | 文档收尾（AC7）属本议题范围，须同步 CLAUDE.md / launch.md / codex 参考文档 |
| KISS | 上轮 review 已否决 --profile overlay / 冲突对话框 / 迁移等复杂设计，本轮方案必须更简 |
