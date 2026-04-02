---
title: "Intake Session Log"
doc_type: proc
status: active
brief: "Q&A transcript for agent-first complete CLI in cct"
confidence: speculative
created: 2026-04-03
updated: 2026-04-03
revision: 1
context_ref: "docs/references/context-agentfirst-complete-cli-cct-20260403003214/"
---

# Intake Session Log

**Session**: intake-20260403003214
**Status**: active

## Context Snapshot

- **Project**: `cct`，Rust 编写的 Claude Code / OpenAI Codex TUI launcher
- **Tech stack**: Rust 2021, clap, ratatui, crossterm, serde, toml, toml_edit
- **Current CLI state**: 现有 `src/cli.rs` 仅提供 `cct add` 的逐步交互式问答流程，默认创建 Claude profile
- **Current launcher model**: 配置与启动逻辑已支持 backend 概念，项目内已有 Codex backend 支持相关历史 intake / TDD
- **Relevant prior session**: `docs/drafts/intake-20260314120000` 已完成 Codex backend 基础支持 intake，但不覆盖本次 agent-first CLI 契约设计
- **User intent**: 新开 intake，聚焦完整 CLI，要求 agent 可一次性定义全部必要参数

## Q&A Transcript

### Round 1
**Category**: Goal
**Question**: 这次应该基于旧会话继续扩展，还是为这个新 CLI 流程单独开一个 intake？
**Answer**: 选项 1，新开 intake，聚焦 CLI 完整流程。
**Files Read**: `CLAUDE.md`, `README.md`, `Cargo.toml`, `docs/drafts/intake-20260314120000/session-log.md`, `docs/drafts/intake-20260314120000/requirements.md`, `docs/procs/tdd-codex-backend-20260315222153/tdd.md`

### Round 2
**Category**: Goal
**Question**: 你说的 “complete CLI, support backend selection -> profile selection”，第一版最核心的交互入口你希望是哪一种？
**Answer**: 用户补充目标不是传统交互入口，而是 “CLI 一次交互可以定义清楚所有必要参数，适合 agent 调用”。
**Files Read**: `src/cli.rs`

### Round 3
**Category**: Constraints
**Question**: 既然你要适合 agent 调用，第一版对调用方式的要求更偏向哪种？
**Answer**: 选项 1，单条命令全参数化，零交互。
**Files Read**: None

### Round 4
**Category**: Scope
**Question**: 第一版的命令面你希望怎么设计，才能让 agent 一次性把必要参数说清楚？
**Answer**: 选项 1，新增显式子命令，按动作拆分：`cct launch ...`、`cct add ...`、`cct list ...`。
**Files Read**: None

### Round 5
**Category**: Constraints
**Question**: 对 `launch` 这个 agent-friendly 子命令，第一版你希望它如何定位要启动哪个 profile？
**Answer**: 选项 1，必须显式给出 `--backend` 和 `--profile` 两个参数。
**Files Read**: None

### Round 6
**Category**: Scope
**Question**: 除了 `launch` 以外，第一版 agent CLI 里你希望哪些动作必须一起到位？
**Answer**: 选项 1，`list` + `launch` + `add` 一起到位，形成最小闭环。
**Files Read**: None

### Round 7
**Category**: Acceptance
**Question**: 为了让 agent 稳定调用，`list` / `add` / `launch` 的输出格式第一版你希望怎么定？
**Answer**: 选项 1，默认机器可读 JSON，并提供可选人类可读格式。
**Files Read**: None

### Round 8
**Category**: Acceptance
**Question**: 第一版完成后，你最看重哪组 done 信号？
**Answer**: 选项 1，命令契约稳定 + JSON 可机读 + 测试覆盖 `list/add/launch` 全流程。
**Files Read**: None

## Summary
**Rounds**: 8
**Stop Reason**: All five categories answered with confidence
**Gaps**: None

## Synthesis

本次 intake 明确了一个独立于现有 TUI 的 agent-first CLI 方向。目标不是把现有交互式 `cct add` 补丁式增强，而是为 `cct` 增加一套显式、稳定、适合自动化调用的子命令契约。第一版至少需要覆盖 `list`、`add`、`launch`，形成 “发现 profile -> 创建 profile -> 启动 profile” 的完整闭环。调用方式必须是单条命令全参数化、零交互，尤其 `launch` 需要显式要求 `--backend` 和 `--profile`，以保留清晰的 backend selection -> profile selection 语义。

输出契约需要默认 JSON，且成功和失败结果都必须可机器解析，避免 agent 因文本格式漂移而失效。完成标准不是“能跑通一次”而已，而是命令契约、帮助信息、错误处理和测试覆盖都要足够稳定，能让 agent 长期依赖。基于现有代码结构，这项工作应复用当前 `config`、`launch` 和 backend/profile 模型，同时尽量不破坏现有 TUI 主路径与已有配置兼容性。
