---
title: "Proc: Codex P0 fixes — auth.json 残留 / config.toml 覆盖 / features 默认值"
doc_type: proc
status: completed
brief: "P0 变更集:修复 #11 api key 编辑不生效、#10 features 默认值、config.toml 用户手改被覆盖"
confidence: verified
created: 2026-08-01
updated: 2026-08-01
revision: 2
---

# Codex P0 Fixes

**Started**: 2026-08-01 00:48
**Related issues**: #11 (bug), #10
**test_cmd**: `cargo test`

## Scope

一个变更集解决三件事(都集中在 `src/launch.rs` 的 Codex 文件生成逻辑):

| # | 问题 | 根因(代码审计) | 修复 |
|---|------|----------------|------|
| 1 | #11 edit api key 后 auth.json 不变 | 候选:① `write_codex_auth` 在 key 缺失时不删除旧 auth.json(残留);② 需验证 edit 数据流(update_profile 的 Codex 分支确实写 `env.OPENAI_API_KEY`,但需确认 main.rs edit 分支的调用链) | ① 空 key 时删除 auth.json;② 契约测试覆盖 edit → auth.json 链路 |
| 2 | #10 `default_mode_request_user_input` 应默认开启 | `generate_codex_config` 生成模板不含 `[features]` | 新文件/缺失时写入 `default_mode_request_user_input = true`;用户显式设置则尊重 |
| 3 | config.toml 用户手改被覆盖(launch.rs:82 无条件 `fs::write`) | 每次 launch 全量重写,用户手加的 `[features]`/`[projects]` 丢失 | 改为 surgical merge:`toml_edit` 只更新 cct 管理的 5 个键(model_provider, model, model_providers.custom.{name,base_url,requires_openai_auth}),保留其余内容 |

顺带修复:auth.json / config.toml 的 `format!` 拼接不转义 → 改用 `serde_json` / `toml_edit` 生成(profile 字段是用户输入边界)。

## Constraints(来自仓库规则)

- `preserve-user-edited-config-structure` — merge 而非覆盖,与 `toggle_full_auto` 的 toml_edit 实践一致
- `assert-contracts-not-incidental-platform-strings` — 测试断言解析后的 JSON/TOML 值,不比字符串
- `pure-builders-thin-effectful-edges` — 生成函数保持可测(接受 codex_home 路径参数)
- `cross-module-features-need-contract-tests` — 必须覆盖 update_profile → launch 生成物的数据流
- KISS — 不引入新依赖(serde_json/toml_edit 已在),create 与 merge 共用一条代码路径

## Execution Order

1. **根因确认** — 读 `main.rs` edit 分支调用链;写复现测试(编辑 key → auth.json 不变)确认 #11 的真实触发路径
2. **write_codex_auth** — serde_json 生成 + 空 key 删除旧文件;单元测试
3. **generate_codex_config** — toml_edit merge + features 默认值;单元测试(新文件、merge 保留、用户显式 false 尊重)
4. **契约测试** — update_profile(edit codex key/model)→ auth.json / config.toml 产物一致
5. **验证** — `cargo test` 全绿 + `cargo clippy` 无新警告
6. **文档** — `docs/modules/launch.md` 更新两个函数接口描述;`docs/dashboard.md` 加本 proc 行

## Verification Checklist

- [x] #11:编辑 codex api key → 重新 launch 生成的 auth.json 含新 key;清空 key → auth.json 被删除
- [x] #10:新 profile 的 config.toml 含 `default_mode_request_user_input = true`
- [x] merge:已有 config.toml 中用户手写的 `[features]`/`[projects]` 保留,model 更新
- [x] 全量测试通过,无回归(122 passed,clippy 干净)

## Outcome

**根因确认(#11)**:`Profile` 无顶层 `api_key` 字段 — `cct edit` 手改顶层 `api_key = "..."` 被 serde 静默忽略,launch 时 `write_codex_auth` 只读 `env.OPENAI_API_KEY`,auth.json 保持旧值。次要根因:`write_codex_auth` 在 key 移除后不删残留 auth.json;JSON 用 `format!` 拼接不转义。

**变更**:
- `config.rs` — `Profile` 改为手写 `Deserialize`:顶层 `api_key` 注入 `env.OPENAI_API_KEY`(仅 Codex;Claude 由 auth_type 体系管理,不处理)
- `launch.rs` `write_codex_auth` — `serde_json` 生成;key 缺失时删除残留 auth.json
- `launch.rs` `generate_codex_config` — `toml_edit` surgical merge(只刷 cct 拥有的键,保留用户 `[features]`/`[projects]`);新文件含 `[features] default_mode_request_user_input = true`(缺失才写,尊重用户显式值);`ensure_subtable` helper 保证表头格式输出,值由 toml_edit 转义
- 新增 9 个测试:3 个 #11 复现、4 个 generate_codex_config 行为、2 个跨模块契约(update_profile → auth.json/config.toml)

**未做(后续)**:#9 历史共享、#12 Response API、#7 worktree、#4 Windows、config.toml 中 `requires_openai_auth` 的 docs 样例同步见 codex-backend-development-guide.md 更新。
