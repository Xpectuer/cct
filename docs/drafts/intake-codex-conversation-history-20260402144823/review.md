---
title: "Review: Codex conversation history shared across profiles"
doc_type: review
brief: "Design review checklist for shared Codex history with isolated profile config"
confidence: verified
created: 2026-04-02
updated: 2026-04-02
revision: 1
---

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
