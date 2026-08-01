---
title: "Domain Model: Codex conversation history isolation re-evaluation"
doc_type: reference
brief: "Entities, relationships, and invariants of the Codex session storage domain"
confidence: speculative
created: 2026-08-01
updated: 2026-08-01
revision: 1
source_skill: intake
---

# Domain Model

## Entities

### cct profile
`profiles.toml` 中的启动配置单元（name、model、base_url、env、extra_args、auth_type）。它决定 cct 用哪些 `--config` flags 启动 codex，但 07-13 后**不决定会话存到哪里**。

**Key Attributes**:
- **auth_type**: proxy（API key 经 cct proxy）或 subscription（openai OAuth）
- **env / model / base_url**: 决定注入的 `--config` 值与代理 upstream

### CODEX_HOME
Codex 本地状态根（默认 `~/.codex`），承载全部 per-user state（sessions/、state_5.sqlite、history.jsonl、logs、memories）。07-13 后所有 cct profile 指向同一实例——会话物理存储的唯一位置。

**Key Attributes**:
- **sessions/**: rollout 会话文件的物理位置（按年/月/日分目录）
- **config.toml**: 用户级配置（cct 不写）

### Codex profile（官方配置层）
`--profile <name>` 选择的配置叠加文件 `~/.codex/profile-name.config.toml`（Codex 0.134.0+ 机制）。当前 cct 不使用。纯配置层实体，不参与会话存储。 [UNCERTAIN — 未来 cct 是否启用取决于需求确认]

**Key Attributes**:
- **层位置**: 项目级之下、用户级之上（配置优先级第 3 位）
- **命名**: `<name>.config.toml`，字母/数字/连字符/下划线

### Session（rollout）
一次对话记录，物理文件 `sessions/<YYYY>/<MM>/<DD>/rollout-<uuid>.jsonl`；首条 session_meta 记录 `cwd`、git origin、model_provider、history_mode。拥有者语义：属于**某个工作目录（仓库）**，不属于任何 profile。

**Key Attributes**:
- **cwd / git.repository_url**: 会话绑定的仓库
- **model_provider**: 创建时的供应商标识（无 profile 字段）

### Thread（threads 记录）
state_5.sqlite 中的会话元数据行，经 `rollout_path` 与 Session 一一对应；字段含 `cwd`、`git_branch`、`git_origin_url`、`model_provider`、`history_mode`（实测 legacy）。

**Key Attributes**:
- **model_provider**: 有独立索引（idx_threads_provider），本机 4 种 provider 会话共存
- **archived**: 已归档标记（本机 49/268 归档）

### cct proxy
cct 的本地 HTTP 转发代理（单实例，127.0.0.1:19191/v1）：将 codex 的 OpenAI 兼容请求转发至 cct profile 的 upstream 并注入 Bearer key；Unix 控制 socket（proxy.sock）接收 status/switch/shutdown 命令；由 cct 拥有生命周期（ensure_proxy_running 启动、僵尸自愈重启、并发启动防护）。

**Key Attributes**:
- **控制 socket**: `~/.config/cc-tui/proxy.sock`（Unix domain socket，应用层探测通道）
- **状态**: 内存态 ActiveProfile（base_url/api_key/model），无持久化
- **生命周期归属**: cct（spawn 独立进程，exec 前 ensure + switch）

### Repository（工作目录/项目）
会话可见性的分组维度。`codex resume` 默认只呈现当前 cwd（及其 git origin）下的会话。

**Key Attributes**:
- **git origin URL**: 会话分组/过滤的键

### model_provider
会话元数据中的供应商标识（openai / custom / deepseek 等）。**已证实是 resume 视图的过滤维度**（官方 picker 按当前 config.model_provider_id 过滤，`--all` 关不掉；显式 id 可绕过）。是 threads 索引字段；本地实测多 provider 会话共存于同一 threads 表——**物理共存、视图过滤**。

## Relationships

| From | To | Type | Description |
|------|----|------|-------------|
| cct profile | Codex 进程 | launches | 通过 `--config` flags + exec 启动；不设 CODEX_HOME、不写配置文件 |
| Codex 进程 | Session | produces | 写入 `CODEX_HOME/sessions/`——所有 profile 启动的进程写同一位置（共享 by construction） |
| Session | Repository | binds-to | session_meta/threads 记录 `cwd` + `git.repository_url`（多对一：一个仓库多个会话） |
| `codex resume` | Repository | filters-by | 默认只呈现当前仓库的会话；`--all` 放宽到全部 |
| Codex profile（官方层） | 用户 config | overlays-on | `~/.codex/profile-name.config.toml` 叠加于 `config.toml`，纯配置优先级，不引入新存储位置 |
| Session | model_provider | records | threads.model_provider（多对一：一个 provider 对应多个会话） |
| cct profile | model_provider | injects | 通过 `--config model_provider=<id>` 注入（proxy=custom / subscription=openai），直接决定该启动下 resume 可见的会话集合 |
| cct profile（auth_type=proxy） | cct proxy | routes-through | 启动时 ensure + switch，将 profile 的 base_url/api_key/model 注入 proxy（多对一：多个 proxy profile 共用单实例） |
| cct proxy | upstream | forwards-to | 将 codex 的 /v1 请求转发至 profile 的 base_url 并注入 Bearer key（一对一：单实例对应当前 active profile 的 upstream） |
| Codex 进程 | cct proxy | connects-to | HTTP 127.0.0.1:19191/v1（base_url 由 `--config model_providers.custom.base_url` 指定） |
| cct profile | Codex profile（官方层） | none | 当前无关系（cct 不传 `--profile`）；未来是否建立取决于需求确认 |

## Domain Invariants

1. **会话物理存储唯一**：所有 codex 会话存在于当前 CODEX_HOME（`~/.codex/sessions/`）才能被任何 profile 恢复；不存在"按 profile 分桶"的官方存储机制。
2. **会话绑定仓库**：每个会话记录创建时的 cwd / git origin；resume 默认按此过滤是官方语义，不能通过 cct 配置绕过（除非 `--all`）。
3. **配置层与状态层正交**：官方 `--profile`（配置叠加）不影响会话存储；cct 的配置注入方式（`--config` flags 或未来 `--profile`）不应改变会话可见性。
4. **cct 不拥有 Codex 内部状态**：config.toml、auth.json、sqlite、rollout 文件均为 Codex/用户资产；cct 不得改写（上一轮 review 与 `9a09b39` 已确立）。
5. **仓库间会话默认不可见**：跨仓库查看必须显式 `--all`——这是官方特性而非缺陷；任何"自动统一会话列表"方案都违反此不变量并引入隐私/噪音问题。
6. **同 provider 同仓库会话可见（官方默认）**：`codex resume` 默认可见集合 = {model_provider == 当前} ∩ {cwd == 当前}；跨 provider 必须显式 session-id。
7. **resume 过滤不可被 `--all` 完全关闭**：`--all` 仅关 cwd 过滤；provider 过滤在本地 TUI picker 中恒定，跨 provider 恢复只能显式 `codex resume <id>`。
