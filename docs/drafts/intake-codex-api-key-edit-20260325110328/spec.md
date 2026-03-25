---
title: "Spec: Codex API Key Edit Sync"
doc_type: spec
brief: "Design spec for write_codex_auth() in launch.rs"
confidence: verified
created: 2026-03-25
updated: 2026-03-25
revision: 1
source_skill: idea
---

# Spec: Codex API Key Edit Sync

## Chosen Approach

**独立 `write_codex_auth()` 纯函数** — 复用 `generate_codex_config` 的模式（`&Profile` + `&Path` -> `Result<()>`），在 `exec_codex()` 中紧跟 config 生成之后调用。

### 决策理由

- 完全遵循项目 pure-builder / thin-effectful-edge 规则
- 与 `generate_codex_config` 签名一致，测试模式一致
- 改动 < 20 行代码 + < 20 行测试
- 符合 KISS 原则，不过度抽象

### 排除的方案

- **合并进 `generate_codex_config()`**: 违反单一职责，语义不清，逻辑分支不同
- **编辑时写入 auth.json**: 超出 scope（intake 已排除）
- **环境变量替代**: codex CLI 只读 auth.json，不可行

## Architecture

### 新增函数

```rust
/// Write auth.json to codex_home with the profile's OPENAI_API_KEY.
/// Skips silently if no API key is present in profile.env.
pub fn write_codex_auth(profile: &Profile, codex_home: &Path) -> Result<()>
```

### 调用链

```
exec_codex(profile)
  ├── check_codex_installed()
  ├── codex_home = ~/.config/cc-tui/codex/{profile.name}
  ├── generate_codex_config(profile, &codex_home)   // 写 config.toml
  ├── write_codex_auth(profile, &codex_home)         // 新增：写 auth.json
  ├── set_var("CODEX_HOME", &codex_home)
  ├── set_var(profile.env...)
  └── exec("codex", args)
```

## Data Flow

1. `profile.env["OPENAI_API_KEY"]` -> 提取 key 值
2. 如果 key 存在: 序列化为 `{"openai_api_key": "<key>"}` -> `fs::write` 到 `{codex_home}/auth.json`
3. 如果 key 不存在: 返回 `Ok(())` (静默跳过)

## Error Handling

- `fs::write` 失败 -> 返回 `Err`
- `exec_codex()` 捕获错误 -> 返回 `anyhow::anyhow!("failed to write codex auth: {e}")`
- 与 `generate_codex_config` 的错误处理路径完全一致

## Testing Plan

| Test | Input | Expected |
|------|-------|----------|
| `write_codex_auth_writes_correct_json` | Profile with `OPENAI_API_KEY="sk-test"` | auth.json 内容为 `{"openai_api_key":"sk-test"}` |
| `write_codex_auth_skips_when_no_key` | Profile without env | auth.json 不存在 |
| `write_codex_auth_overwrites_existing` | 先写旧文件，再调用新 key | auth.json 内容为新 key |

## Open Questions

无。需求明确，格式已确认，实现路径清晰。
