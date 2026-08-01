---
title: "Review: Codex conversation history shared across profiles"
doc_type: review
brief: "Design review checklist for shared Codex history with isolated profile config"
confidence: verified
created: 2026-04-02
updated: 2026-08-01
revision: 3
---

> **⚠️ SUPERSEDED (2026-08-01)**: 本文是 revision 1 设计（per-profile `CODEX_HOME` + 符号链接共享 artifact）的评审记录，该设计已被 revision 2（共享 `CODEX_HOME` + 官方 `--profile` 叠加层）否决。以下内容仅作历史参考。

## Re-review vs Current Code (revision 3, 2026-08-01)

**起因**: 本 draft 的 spec/plan（revision 2）在编写时基于旧版 `src/launch.rs`（仍含 `generate_codex_config` / `write_codex_auth` / per-profile `CODEX_HOME`）。此后仓库发生了大幅代码演进（2026-07-13 的 client(TUI)+proxy 架构重构与共享 home 重构、2026-08-01 的 P0 修复），plan 的 old anchor 与设计前提已不再成立。本次对 master @ `514c646` 重新评审。

### 当前代码实际状态（2026-08-01，master）

| Commit | 日期 | 变更 |
|--------|------|------|
| `b18cc4e` | 07-13 | 引入 client(TUI)+server(proxy) 架构，`generate_codex_config` 仍存在 |
| `107cecf` | 07-13 | 删除 `write_codex_auth`（cct 不再写 `auth.json`） |
| `afc1d11` | 07-13 | **删除 `generate_codex_config` 与 `codex_home_for_profile`**；改用 `build_codex_proxy_config_args` 通过 `--config` CLI flags 传 provider 配置；所有 profile 共享默认 `~/.codex` |
| `9a09b39` | 08-01 | P0 修复：尊重顶层 `api_key`（经 env → proxy `switch_profile`）、不再写/覆盖用户 `config.toml` |

当前 `src/launch.rs` 的 Codex 路径：`exec_codex` → `exec_codex_proxy`（`--config model_provider=custom` 等 flags + proxy `switch_profile`）或 `exec_codex_subscription`（`--config model_provider=openai`）；**不设置 `CODEX_HOME`，不写任何 Codex 配置文件**。

### 逐项比对结果

| # | Draft（spec rev 2 / code-spec）假设 | 当前代码实际 | 影响 |
|---|--------------------------------------|--------------|------|
| 1 | `generate_codex_config(profile, codex_home)` 存在（code-spec 锚定"当前文件 72-111 行"），写共享 base `config.toml` | 已被 `afc1d11` 删除，函数不存在 | code-spec Step 1-13 的 old anchor 全部失效；Step 3"整函数替换"无目标 |
| 2 | `write_codex_auth` 写 `auth.json`，需删除 | `107cecf` 已删除；key 经 env → proxy `switch_profile` | Step 7/13 的"删除 auth"已由现状完成 |
| 3 | 旧布局为 per-profile `CODEX_HOME`（`~/.config/cc-tui/codex-homes/<name>`），需自动迁移 | `afc1d11` 已共享默认 `~/.codex`；**但 07-13 前用户遗留的 `codex-homes/<name>` 目录未迁移** | 迁移价值缩水但未归零（历史孤儿数据仍在） |
| 4 | cct 拥有 on-disk `config.toml`/overlay，用户手改后与 `profiles.toml` 分歧 → 冲突对话框前提 | cct 完全不写 Codex 配置文件，配置全部走 `--config` flags | 冲突检测的"on-disk 值"载体已不存在，双向绑定设计需重新推导（或放弃） |
| 5 | 启动时设置 `CODEX_HOME` + 前插 `--profile <name>` | 当前不设 `CODEX_HOME`、无 `--profile`；arg 为 `--config` flags | code-spec Step 6 的 `build_codex_args` 改造目标不存在 |
| 6 | 核心问题："history 按 profile 隔离" | **已解决**：07-13 起所有 profile 共享 `~/.codex` | issue #9 的首要目标在现行代码中已经达成 |

### 结论

1. **核心目标已达成**：共享 Codex conversation history（issue #9 的首要诉求）在现行代码中已实现（`afc1d11`，共享 `~/.codex`）。
2. **plan 不可按原样执行**：code-spec 的 old anchor 指向已被删除的函数，Step 1-13 无法落地；spec 的冲突检测前提（on-disk overlay）在 `--config` flags 架构下不存在载体。
3. **draft 剩余有效增量**（均未实现，但需基于现行架构重新推导）：
   - 遗留 `codex-homes/<name>` 历史数据的一次性导入（迁移）——只对 07-13 前的老用户有意义；
   - 双向绑定冲突对话框（`AppMode::ConflictConfirm`、p/d 键）——前提需重新设计（如：`profile.env` 与什么对比、或删除该特性）。
4. **建议**：本 draft 不再直接进入实现；如需继续，应基于当前架构重新推导 spec/plan（revision 3），或将剩余增量拆为独立小需求。评审记录：`plan/review.md` 的 READY 结论随本次重审失效。

## 历史评审（revision 1→2）

# Review: Codex conversation history shared across profiles

## Design Review Checklist

| # | Check | Status |
|---|-------|--------|
| 1 | 改动边界限制在 `launch` 与文档，不扩散到 UI / schema | pass |
| 2 | 遵循 pure-builder / thin-effectful-edge 规则 | pass |
| 3 | 共享边界由显式 artifact 列表定义，而不是共享整个 `CODEX_HOME` | pass |
| 4 | profile 专属 `config.toml` / `auth.json` 仍保持独立 | pass |
| 5 | 历史映射建立失败时会阻断启动，而不是静默降级 | pass |
| 6 | 冲突文件不会被隐式覆盖 | pass |
| 7 | 测试覆盖 layout、共享映射、幂等、失败路径 | pass |
| 8 | 文档会同步更新，不保留旧的 per-profile history 叙述 | pass |

## Rule Compliance

| Rule | Compliant | Notes |
|------|-----------|-------|
| KISS | Yes | 仅调整 Codex 启动目录组织，不加新 UI 或新配置项 |
| pure-builders-thin-effectful-edges | Yes | 路径解析纯化，文件系统与 exec 保持在窄边界 |
| single-source-of-truth-variant-mappings | N/A | 不涉及 form 或 backend variant mapping |
| mask-secrets-on-every-display-path | N/A | 不新增任何 UI 展示路径 |
| preserve-user-edited-config-structure | Yes | 不修改 `profiles.toml` schema |
| cross-module-features-need-contract-tests | Yes | 共享边界和失败行为由 `launch.rs` 契约测试覆盖 |

## Risks

- **中风险**: Codex 的“可见历史”可能依赖除 JSONL/session 目录之外的 SQLite 文件，若边界判断不完整，第一版可能出现“部分共享”。
- **低风险**: 不同平台上的链接语义可能有实现差异；如果当前项目明确只支持 Unix，这个风险可控，但测试仍需覆盖。
- **低风险**: 老版本 runtime 目录中若已存在普通文件，迁移策略如果不够保守，容易误覆盖用户数据，因此必须 fail-fast。

## Open Issues

- 在最终实现前，需要确认 SQLite 文件是否属于会话连续性的最小闭包；若是，文档与 artifact 列表都要同步补充。
