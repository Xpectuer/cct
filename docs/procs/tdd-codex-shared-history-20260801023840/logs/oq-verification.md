---
title: "OQ Verification — codex 0.144.6"
brief: "3 项 Open Questions 实测结果"
doc_type: proc
created: 2026-08-01T02:49:51+0800
---

# OQ Verification — codex 0.144.6

**环境**: `codex-cli 0.144.6`（`/Users/zhengjiaye/.nvm/versions/node/v20.20.0/bin/codex`，npm 全局安装），macOS。
**隔离**: 全部实验使用 `mktemp -d` 临时 CODEX_HOME（`/tmp/oq-codex-R0f0Yo`），`CODEX_HOME` + `CODEX_SQLITE_HOME` 指向该目录；实验后已清理。未读写任何真实配置（`~/Library/Application Support/cc-tui/`、`~/.config/*` 均未触碰，目录 mtime 无变化）。

**实验配置**（模拟 plan Step 3/4 产物）：

`config.toml`（base）:
```toml
model_provider = "custom"

[features]
default_mode_request_user_input = true
```

`oqtest.config.toml`（overlay — 即 `write_codex_profile_overlay` 的产物形状，**无** `requires_openai_auth`）:
```toml
model = "gpt-5.4"

[model_providers.custom]
name = "oqtest"
base_url = "https://clauddy.com/v1"
env_key = "OPENAI_API_KEY"
```

鉴权 key 从真实 profiles.toml（`clauddy-codex` codex profile）**进程内读取**（51 字符），仅注入子进程 env，任何输出中未出现明文。

---

## OQ-1 — `env_key` 鉴权（无需 `requires_openai_auth`）: **PASS**

**方法**: 真实最小会话 + 无 key 负向对照。

**命令**（key 进程内注入，输出经脱敏）:
```bash
CODEX_HOME=<tmp> codex exec --profile oqtest --full-auto --skip-git-repo-check -C <tmp>/work \
  "Reply with exactly one word: OK. Do not use any tools or shell commands."
```

**观察到的事实**:
1. 会话成功（exit 0，模型回复 `OK`），模型请求 `POST https://clauddy.com/v1/responses` 返回 `status=200 OK`。
2. 会话配置日志（codex 自身打点）:
   - `provider=ModelProviderInfo { name: "oqtest", base_url: Some("https://clauddy.com/v1"), env_key: Some("OPENAI_API_KEY"), ..., requires_openai_auth: false, ... }` — **overlay 中未写 `requires_openai_auth`，生效值即为 `false`**，鉴权照常完成。
   - `auth.env_provider_key_present=true auth.env_provider_key_name="configured"`
   - `auth.header_attached=true auth.header_name="authorization"` — key 经 Authorization header 送达。
3. **负向对照**（同配置、env 中移除 `OPENAI_API_KEY`）: 无任何网络请求，直接报错
   `ERROR: Missing environment variable: OPENAI_API_KEY.`（exit 1）— 证明 key 的来源就是 `env_key` 指定的环境变量。
4. 成功会话后 CODEX_HOME 中**未生成 `auth.json`** — `env_key` 机制完全替代 auth.json，Step 7 删除 `write_codex_auth` 安全。

**结论**: `env_key = "OPENAI_API_KEY"` + 第三方 `base_url`，无 `requires_openai_auth`，鉴权可用。

---

## OQ-2 — overlay 中的嵌套 `model_providers.custom` 表: **PASS**

**方法**: 同一临时 CODEX_HOME + overlay，flag 解析 + 真实会话双证据。

**命令**:
```bash
codex --profile oqtest --version        # flag 解析 + 无 TOML 解析错误
codex exec --profile oqtest --full-auto ...   # 真实加载（上述 OQ-1 会话）
```

**观察到的事实**:
1. `codex --profile oqtest --version` 正常输出 `codex-cli 0.144.6`（无配置解析报错）。
2. 真实会话日志确认 overlay 的嵌套表被解析并使用：
   - `model=gpt-5.4`（overlay 顶层的 `model`）;
   - `provider=ModelProviderInfo { name: "oqtest", base_url: Some("https://clauddy.com/v1"), env_key: Some("OPENAI_API_KEY"), ... }` — name/base_url/env_key 全部来自 overlay 嵌套表;
   - 模型请求实际发往 `https://clauddy.com/`（provider base_url），非官方端点。
3. 唯一相关警告为 `--full-auto` deprecation（见 OQ-3）与 `default_mode_request_user_input` 的 under-development 提示，均与 TOML 解析无关。

**结论**: 嵌套表写在 per-profile overlay 中可被 codex 0.144.6 正确解析并驱动会话。

---

## OQ-3 — `--profile` 与 `--full-auto` 组合: **PARTIAL（exec 子命令下 PASS；顶层 FAIL）**

**方法**: 三组 flag 形态实测。

**命令与观察到的事实**:

| # | 命令形态 | 结果 |
|---|----------|------|
| 1 | `codex --profile oqtest --version`（顶层，仅 profile） | 接受（exit 0）。顶层 help 中 `-p, --profile <CONFIG_PROFILE_V2>` 描述：*Layer $CODEX_HOME/<name>.config.toml on top of the base user config* — 与 plan 的 `<name>.config.toml` 命名约定完全一致 |
| 2 | `codex exec --profile oqtest --full-auto --version`（exec 子命令下组合） | **接受**（exit 0，打印 `codex-cli-exec 0.144.6`）；真实会话（OQ-1）完整跑通，`approval_policy=never` |
| 3 | `codex --profile oqtest --full-auto`（顶层组合 — **plan Step 6 当前产物形状**） | **FAIL**: `error: unexpected argument '--full-auto' found`（clap 解析错误，提示 "use '-- --full-auto'"） |

**其他观察到的事实**:
- **`--full-auto` 已弃用**：每次使用均输出 `warning: \`--full-auto\` is deprecated; use \`--sandbox workspace-write\` instead.`（仍可用，语义映射为 approval never + workspace-write sandbox）。
- `--full-auto` 是 `exec` 子命令的隐藏 flag（help 中不显示），顶层未声明。
- 当前 cct（plan 之前）`build_codex_args` 产出 `codex --full-auto`（无 `exec`）在 0.144.6 同样被拒 — 该问题在 plan 之前已存在于 full_auto profile 上，Step 6 会把同一问题带入 `--profile` 形态。
- 附加观察（missing-profile 行为）：`codex exec --profile missing-profile`（overlay 文件不存在）**不报 profile 未找到**，静默忽略并继续用 base config，随后因 base 只有 `model_provider = "custom"` 而无 custom 表报 `Error: Model provider 'custom' not found`。即：overlay 缺失的失败形态是 provider 解析错误而非"profile 不存在"，Step 5 先写 overlay 再 exec 的顺序可避免。

**结论**: 两 flag 组合在 `codex exec` 子命令下可用；在顶层不可用。

---

## 汇总与 plan 影响

| OQ | 结论 | 影响 |
|----|------|------|
| OQ-1 env_key 鉴权 | **PASS** | Step 4 overlay 内容**无需改**（`env_key` + `base_url`，无 `requires_openai_auth`，验证正确）；Step 7 删除 `write_codex_auth` 安全（codex 不生成/不依赖 auth.json） |
| OQ-2 嵌套表 overlay | **PASS** | Step 4 overlay 的 `[model_providers.custom]` 嵌套表写法正确，无需改 |
| OQ-3 `--profile` + `--full-auto` | **PARTIAL** | **Step 6 需要回改**：`build_codex_args` 产出 `["--profile", name, "--full-auto", ...]` 在顶层被 clap 拒绝。full_auto 时须前置 `exec` 子命令（`["exec", "--profile", name, "--full-auto", ...]`）或改用非弃用 flag（`--sandbox workspace-write`）。`build_codex_args_full_auto_only` 等测试期望同步更新。`--full-auto` 的 deprecation warning 不影响功能但建议记录 |

**关键文件**: `docs/procs/tdd-codex-shared-history-20260801023840/tdd.md`（Step 6 需回改）、`ref/plan/code-spec.md`（Step 6/Test Case 24）。
