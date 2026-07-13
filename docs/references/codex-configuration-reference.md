---
title: "Reference: Codex CLI Configuration"
doc_type: reference
brief: "Codex CLI 配置体系完整参考：CODEX_HOME、config.toml、auth.json、model_providers、profiles、配置优先级"
confidence: verified
created: 2026-07-13
updated: 2026-07-13
revision: 1
source:
  - https://developers.openai.com/codex/config-reference
  - https://developers.openai.com/codex/config-basic
  - https://developers.openai.com/codex/config-advanced
  - https://developers.openai.com/codex/environment-variables
  - https://developers.openai.com/codex/config-sample
  - https://www.morphllm.com/codex-provider-configuration
  - https://github.com/openai/codex/issues/2760
---

# Reference: Codex CLI Configuration

## Purpose

记录 Codex CLI 的配置体系，为 cct 的 Codex 后端实现提供准确的参考基准。
来源为 OpenAI 官方文档及社区验证资料（2026-07 当前版本）。

---

## 1. CODEX_HOME

| 项目 | 值 |
|------|-----|
| 默认值 | `~/.codex` |
| 覆盖方式 | `CODEX_HOME` 环境变量 |
| 要求 | 如果手动设置，目录必须已存在 |

**CODEX_HOME 目录完整结构：**

```text
~/.codex/
├── config.toml              # 用户级配置（主配置）
├── auth.json                # 认证凭据（file 模式，权限 0600）
├── .credentials.json        # MCP OAuth 凭据
├── .env                     # 环境变量覆盖（启动时加载）
├── history.jsonl            # 命令/会话历史
├── session_index.jsonl      # 会话索引
├── state_*.sqlite           # SQLite 状态数据库（threads, jobs, agents 等）
├── logs_*.sqlite            # SQLite 日志数据库
├── version.json             # 版本信息
├── AGENTS.md                # 全局指令
├── AGENTS.override.md       # 全局指令覆盖
├── rules/                   # 自定义规则
├── skills/                  # 技能目录
├── agents/                  # 子代理配置
├── memories/                # 记忆存储
├── prompts/                 # 自定义提示
├── sessions/                # 会话 rollout JSONL
│   └── YYYY/MM/DD/
├── shell_snapshots/         # Shell 快照
├── tmp/                     # 临时文件
├── log/
│   └── codex-tui.log        # TUI 日志
└── <profile-name>.config.toml  # Profile 文件
```

**关键结论**：`CODEX_HOME` 不仅仅存放启动配置，它是 Codex 的完整工作空间——包含对话历史、会话状态、日志、记忆、技能等。**修改 `CODEX_HOME` 会隔离所有这些状态**。

---

## 2. config.toml — 主配置文件

### 位置与优先级（从高到低）

| 优先级 | 层级 | 说明 |
|--------|------|------|
| 1 | CLI flags / `--config key=value` | 单次调用覆盖 |
| 2 | Project `.codex/config.toml` | 可信项目，从根到 CWD 层层叠加，最近者胜 |
| 3 | Profile 文件 `--profile <name>` | `~/.codex/<name>.config.toml` |
| 4 | User `~/.codex/config.toml` | 用户个人默认 |
| 5 | System `/etc/codex/config.toml` | 系统级/管理配置 |
| 6 | Built-in defaults | 代码内置默认值 |

### 安全边界

**Project `.codex/config.toml` 不能覆盖以下 key**（防止 clone 仓库劫持 API 调用）：

- `openai_base_url`, `chatgpt_base_url`
- `model_provider`, `model_providers`
- `profile`, `profiles`
- `notify`
- `apps_mcp_product_sku`
- `experimental_realtime_ws_base_url`
- `otel`

Provider、认证、通知、遥测相关配置必须放在用户级 `~/.codex/config.toml`。

### 核心 Top-Level Keys

#### Model & Provider

| Key | 类型 | 默认 | 说明 |
|-----|------|------|------|
| `model` | string | — | 默认模型 ID，如 `"gpt-5.5"` |
| `model_provider` | string | `"openai"` | 从 `[model_providers]` 中选择 provider |
| `model_reasoning_effort` | string | — | 推理力度：`"low"` / `"medium"` / `"high"` / `"xhigh"` |
| `model_verbosity` | string | — | 回复详细度：`"low"` / `"medium"` / `"high"` |
| `model_context_window` | number | — | 模型上下文窗口 token 数 |
| `model_catalog_json` | string (path) | — | 自定义模型目录 JSON 文件路径 |

#### Approval & Sandbox

| Key | 类型 | 默认 | 说明 |
|-----|------|------|------|
| `approval_policy` | string | — | `"never"` / `"on-request"` / `"always"` |
| `sandbox_mode` | string | — | `"workspace-write"` / `"danger-full-access"` 等 |
| `default_permissions` | string | — | 默认权限策略 |

#### Auth & Credentials

| Key | 类型 | 默认 | 说明 |
|-----|------|------|------|
| `preferred_auth_method` | string | — | `"apikey"` / `"chatgpt"` |
| `cli_auth_credentials_store` | string | — | `"file"` / `"keyring"` / `"auto"` |
| `openai_base_url` | string | — | 覆盖内置 OpenAI provider 的 base URL |

#### Feature Flags

位于 `[features]` 表下，包括 `hooks`, `personality`, `unified_exec`, `browser_use`, `computer_use`, `codex_git_commit` 等。

#### Tools

位于 `[tools]` 表下，包括 `web_search` 等工具开关。

---

## 3. auth.json — 认证凭据

### 位置

`$CODEX_HOME/auth.json`（仅当 `cli_auth_credentials_store = "file"` 时使用）

### 格式

#### API Key 模式

```json
{
  "auth_mode": "apikey",
  "OPENAI_API_KEY": "sk-..."
}
```

#### ChatGPT 登录模式

```json
{
  "auth_mode": "chatgpt",
  "tokens": {
    "id_token": "...",
    "access_token": "...",
    "refresh_token": "...",
    "account_id": "..."
  },
  "last_refresh": "2025-01-01T00:00:00Z"
}
```

#### 多认证模式

```json
{
  "auth_mode": "chatgptAuthTokens",
  "OPENAI_API_KEY": "sk-...",
  "tokens": { ... }
}
```

### 安全特性

- Unix 上写入权限为 `0o600`
- `auth.json` 不应提交到版本控制

---

## 4. model_providers — 自定义 Provider 配置

### 定义方式

```toml
model = "gpt-5.4"
model_provider = "my-provider"

[model_providers.my-provider]
name = "My Provider"
base_url = "https://api.example.com/v1"
env_key = "MY_API_KEY"
wire_api = "responses"
```

### Provider Block 完整字段

| Key | 类型 | 必须 | 说明 |
|-----|------|------|------|
| `name` | string | 否 | 显示名称 |
| `base_url` | string | 是 | API endpoint URL |
| `env_key` | string | 否 | 存放 API Key 的环境变量名，作为 Bearer token 发送 |
| `wire_api` | string | 否 | 协议，目前仅 `"responses"` |
| `requires_openai_auth` | bool | 否 | 使用 ChatGPT 登录（仅对 OpenAI 自身有效） |
| `experimental_bearer_token` | string | 否 | 直接在 TOML 中写 API key（不推荐，用 env_key） |
| `http_headers` | map | 否 | 每个请求附加的静态 HTTP 头 |
| `env_http_headers` | map | 否 | 从环境变量读取的 HTTP 头 |
| `query_params` | map | 否 | URL query 参数（如 Azure 的 `api-version`） |
| `request_max_retries` | number | 否 | 请求最大重试次数 |
| `stream_max_retries` | number | 否 | 流式最大重试次数 |
| `stream_idle_timeout_ms` | number | 否 | 流空闲超时 |

### Command-Backed Auth

```toml
[model_providers.my-provider.auth]
command = "/usr/local/bin/fetch-token"
args = ["--audience", "codex"]
timeout_ms = 5000
refresh_interval_ms = 300000
```

| Key | 类型 | 说明 |
|-----|------|------|
| `command` | string | 获取 token 的命令 |
| `args` | array | 命令参数 |
| `cwd` | string | 工作目录 |
| `timeout_ms` | number | 命令超时（默认 5000） |
| `refresh_interval_ms` | number | 刷新间隔（默认 300000，0 = 仅失败时刷新） |

**注意**：`[auth]` 不能与 `env_key`、`experimental_bearer_token`、`requires_openai_auth` 混用。

### 内置 Provider IDs（不可覆盖）

`openai`, `ollama`, `lmstudio`, `amazon-bedrock`

---

## 5. Profiles

### 当前机制（Codex 0.134.0+）

Profile 文件放在 `$CODEX_HOME/<profile-name>.config.toml`，与 `config.toml` 并列：

```toml
# ~/.codex/deep-review.config.toml
model = "gpt-5.5"
model_reasoning_effort = "xhigh"
approval_policy = "on-request"
```

使用：`codex --profile deep-review`

**重要变更**：不再支持在 `config.toml` 中用 `[profiles.name]` 表定义，也不支持 `profile = "name"` selector。Profile 文件只包含与 base config 不同的 key。

### Profile 优先级

Profile 文件位于用户 config 之上、项目 config 之下：

```
CLI flags > Project .codex/ > Profile 文件 > User ~/.codex/config.toml > System > Defaults
```

---

## 6. 环境变量

### Core Locations

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `CODEX_HOME` | `~/.codex` | Codex 状态根目录（config, auth, logs, sessions, skills） |
| `CODEX_SQLITE_HOME` | `CODEX_HOME` | SQLite 状态存储位置 |

### Provider API Keys

不通过固定环境变量名指定，而是在 `[model_providers.<id>]` 中用 `env_key` 声明变量名。Codex 读取该变量值作为 Bearer token。

### `.env` 文件

Codex 启动时自动加载 `$CODEX_HOME/.env` 中的环境变量。

---

## 7. 对 cct 的设计启示

### 当前问题

cct 将 `CODEX_HOME` 设置为 `~/.config/cc-tui/codex/<profile-name>/`，导致：
- `history.jsonl` — 对话历史被隔离
- `state_*.sqlite` — 会话状态丢失
- `sessions/` — 历史会话不可见
- `memories/` — 记忆被隔离
- `skills/`, `rules/`, `agents/` — 自定义内容丢失

### 正确做法

cct 只应覆盖 **provider 配置**（两个文件），不应改变 Codex 的整体状态目录：

1. **不设置 `CODEX_HOME`**（或设为用户已有的值，即不改变）
2. **只写 `config.toml`**：设置 `model`, `model_provider`, `[model_providers.<id>]` 块
3. **只写 `auth.json`**：当 `cli_auth_credentials_store = "file"` 时

### 推荐实现

```
exec_codex:
  1. 解析 codex home = $CODEX_HOME 或 ~/.codex
  2. 备份 ~/.codex/config.toml（如果存在）← 可选，因 exec 不返回
  3. 写入 ~/.codex/config.toml（provider 配置）
  4. 写入 ~/.codex/auth.json（API key）
  5. 注入 profile.env 到进程环境
  6. exec codex <args>
```

这样切换 profile 只改变供应商连接方式，用户的对话历史、记忆、技能等全部保留。

## Related

- `docs/references/codex-backend-development-guide.md` — cct Codex 后端开发指南
- `docs/references/codex-home-storage-layout.md` — 当前 per-profile CODEX_HOME 布局（待更新）
- `src/launch.rs` — Codex 启动边界实现
