---
title: "Plan: Codex API Key Edit Sync"
doc_type: plan
brief: "Implementation plan for write_codex_auth() fix"
confidence: verified
created: 2026-03-25
updated: 2026-03-25
revision: 1
---

# Plan: Codex API Key Edit Sync

## Overview

修复 Issue #11：Codex profile 编辑 API key 后，启动时 auth.json 未同步更新。

## Steps

### Step 1: 新增 `write_codex_auth()` 函数
**File**: `src/launch.rs`
**Action**: 在 `generate_codex_config` 函数之后新增 `write_codex_auth`
**Details**:
- 签名: `pub fn write_codex_auth(profile: &Profile, codex_home: &Path) -> Result<()>`
- 从 `profile.env` 中取 `OPENAI_API_KEY`
- 如果 key 存在: 写 `{"openai_api_key":"<key>"}` 到 `{codex_home}/auth.json`
- 如果 key 不存在: 返回 `Ok(())`

### Step 2: 在 `exec_codex()` 中调用 `write_codex_auth()`
**File**: `src/launch.rs`
**Action**: 在 `generate_codex_config` 调用之后、`set_var("CODEX_HOME")` 之前插入调用
**Details**:
- 错误处理与 `generate_codex_config` 一致: `anyhow::anyhow!("failed to write codex auth: {e}")`

### Step 3: 新增单元测试
**File**: `src/launch.rs` (tests module)
**Action**: 新增 3 个测试
**Details**:
- `write_codex_auth_writes_correct_json` — 验证 JSON 内容
- `write_codex_auth_skips_when_no_key` — 验证无 key 时不创建文件
- `write_codex_auth_overwrites_existing` — 验证覆盖旧值

### Step 4: 验证
**Action**: `cargo test` + `cargo clippy`

## Execution Order

```
Step 1 -> Step 2 -> Step 3 -> Step 4
```

所有步骤串行执行，无并行依赖。

## Acceptance Criteria

- [ ] `write_codex_auth()` 写入正确 JSON
- [ ] `exec_codex()` 在启动前调用 `write_codex_auth()`
- [ ] 3 个单元测试通过
- [ ] `cargo test` 全通过
- [ ] `cargo clippy` 无警告
