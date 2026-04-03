---
title: "Intake Session Log"
doc_type: proc
status: ready
brief: "Q&A transcript for codex conversation history shared across profiles"
confidence: verified
created: 2026-04-02
updated: 2026-04-02
revision: 1
context_ref: "docs/references/context-codex-conversation-history-20260402144823/"
---

# Intake Session Log

**Session**: intake-codex-conversation-history-20260402144823
**Status**: ready

## Context Snapshot

- **Project**: `cct`, a Rust ratatui launcher for Claude Code and OpenAI Codex
- **Tech stack**: Rust 2021, ratatui, crossterm, serde, toml, toml_edit, dirs, anyhow
- **Relevant module**: `src/launch.rs` owns Codex launch orchestration, generated config, environment injection, and `CODEX_HOME` setup
- **Current product behavior**: Codex launch writes generated config into a Codex home path and then execs `codex`, which causes history to remain isolated per profile under the current layout
- **Relevant docs**: `CLAUDE.md`, `README.md`, and `docs/modules/launch.md` still describe Codex launch in a way that assumes a single profile-owned home boundary
- **Primary issue**: users want shared Codex conversation history across profiles without collapsing all per-profile runtime state into one shared directory

## Q&A Transcript

### Round 1
**Category**: Goal
**Question**: 对 issue #9 来说，真正要修复的用户问题是什么？
**Answer**: Codex conversation history 不应该按 profile 被隔离。不同 Codex profile 应共享可见历史，但 profile 自己的启动配置仍要保持隔离。
**Files Read**: CLAUDE.md, README.md

### Round 2
**Category**: Constraints
**Question**: 修复时应该遵守什么技术边界？
**Answer**: 只调整 Codex 启动目录组织，不改 UI、不改 profile schema，并继续遵守 pure builder / thin effectful edge 的实现方式。
**Files Read**: CLAUDE.md, docs/modules/launch.md

### Round 3
**Category**: Existing State
**Question**: 当前实现为什么会把 conversation history 隔离到每个 profile？
**Answer**: `exec_codex()` 目前按 profile 生成和使用各自的运行目录语义，`CODEX_HOME` 没有明确拆分出“共享历史”和“profile 专属配置”这两个边界，所以历史自然被 profile 绑定。
**Files Read**: docs/modules/launch.md, spec.md

### Round 4
**Category**: Scope
**Question**: 第一版修复的范围应该控制在哪里？
**Answer**: 聚焦 `src/launch.rs` 和相关文档，增加 layout/helper 与 composed-home 准备逻辑，用显式 artifact 列表共享历史；不扩散到 UI、config schema 或 Claude 路径。
**Files Read**: plan.md, spec.md, review.md

### Round 5
**Category**: Acceptance
**Question**: 这项工作完成后，怎样判断它达标？
**Answer**: 至少两个 Codex profile 能看到同一份 history；profile 配置仍互相隔离；共享边界在代码和文档中显式定义；冲突路径 fail-fast；测试覆盖 layout、映射、幂等和失败行为。
**Files Read**: plan.md, spec.md

## Summary
**Rounds**: 5
**Stop Reason**: Existing draft artifacts already answered all 5 intake categories with high confidence
**Gaps**: 仓库缺少 intake skill 默认依赖的 `scripts/fm.sh` 和 `scripts/next-steps.sh`，因此本次仅做手工规范化补档，未运行技能中的自动校验/next-steps 脚本
**Note**: 本文件基于现有 `spec.md`、`plan.md`、`review.md` 与当前仓库上下文重建，用于补齐缺失的 intake 文档

## Supplement Session — 2026-04-02

- **Action**: 补建缺失的 `requirements.md` 与 `session-log.md`
- **Reason**: 该 draft 已进入 spec/plan/review 阶段，但 intake 基础文档缺失，导致流程链条不完整
- **Result**: 已补齐 requirements、session log 与 context snapshot 引用，draft 可继续用于后续流程
