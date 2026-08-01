---
title: "Terminology: Codex conversation history isolation re-evaluation"
doc_type: reference
brief: "Canonical domain glossary for intake-20260801145824"
confidence: speculative
created: 2026-08-01
updated: 2026-08-01
revision: 1
source_skill: intake
---

# Terminology

## Core Concepts

**CODEX_HOME**:
Codex 官方定义的本地状态根目录（默认 `~/.codex`），承载全部 per-user state：`config.toml`、`auth.json`、`history.jsonl`、`sessions/`、`state_5.sqlite`、logs、caches、memories 等。cct 07-13 起不设置此变量，所有 profile 共享默认值。
_Avoid_: "per-profile CODEX_HOME"（旧架构，已废弃）、"codex-homes/<name>"（遗留目录名）

**cct profile**:
`profiles.toml`（`~/Library/Application Support/cc-tui/profiles.toml`）中定义的启动配置单元：name、model、base_url、env、extra_args、auth_type 等；cct 据此构建 `--config` flags 并 exec codex。
_Avoid_: 与 "Codex profile" 混用——两者是不同层级的实体，本 draft 必须区分表述。

**Codex profile（官方配置层）**:
Codex 0.134.0+ 的官方配置叠加机制：`--profile <name>` 加载 `~/.codex/profile-name.config.toml`（叠加在 `~/.codex/config.toml` 之上，位于项目级与 CLI flags 之间）。它是**配置层，不是状态隔离层**——不改变会话存储位置。旧版 `[profiles.<name>]` 表与顶层 `profile = "<name>"` 选择器已废弃。
_Avoid_: "profiles 表配置"（旧机制）、"profile selector"

**配置优先级**:
Codex 解析配置的顺序（高→低）：CLI flags / `--config` > 项目 `.codex/config.toml`（仅受信任项目）> `--profile` 文件 > 用户 `~/.codex/config.toml` > 系统 `/etc/codex/config.toml` > 内置默认。项目级配置不能覆盖 `model_provider` / `model_providers` / `profile` / `profiles` / `notify` 等键。
_Avoid_: "配置文件覆盖规则"（无歧义名）

**会话（session / rollout）**:
codex 的一次对话记录，以 `rollout-<uuid>.jsonl` 存储在 `CODEX_HOME/sessions/<YYYY>/<MM>/<DD>/`；`session_meta` 首条记录含 `cwd`、`git{commit_hash,branch,repository_url}`、`model_provider`、`history_mode`。**无 profile 字段**；threads 表（state_5.sqlite）同样按 `cwd` / `git_origin_url` / `model_provider` 索引。
_Avoid_: "codex 会话按 profile 存储"（无证据）

**resume 仓库过滤**:
`codex resume` 的默认语义："Reopen a recent chat from the current repository"——只列出/选择当前工作目录（及其 git origin）下的会话；`--last` 取当前 cwd 最近会话，`--all` 才跨目录搜索。这是官方产品语义，也是"会话看起来被隔离"的最可能机制。
_Avoid_: "resume 按 profile 过滤"（官方无此语义）

**cct proxy**:
cct 的本地 HTTP 转发代理（单实例，127.0.0.1:19191/v1）：将 codex 的 OpenAI 兼容请求转发至 profile 的 upstream 并注入 Bearer key；Unix 控制 socket（proxy.sock）接收 status/switch/shutdown 命令；生命周期由 cct 拥有（ensure_proxy_running 启动、僵尸自愈重启、并发启动防护）。与 auth_type 值 "proxy"、用户系统代理（如 Clash）区分。
_Avoid_: "proxy 进程"（无歧义但非 canonical）、将 cct proxy 与 Clash 等系统代理混为一谈

**model_provider**:
codex 配置中当前使用的模型供应商标识（如 `openai`、`custom`、`deepseek`）。**已证实是 resume 的过滤维度**：官方 TUI resume picker 本地运行时按 `config.model_provider_id` 过滤会话（源码取证，见 refs/codex-resume-filtering-source.md），`--all` 关不掉。threads 表有 `idx_threads_provider` 索引；本机实测 4 种 provider 会话共存于同一 threads 表（物理共享但视图隔离）。
_Avoid_: "provider 隔离会话"（物理上不隔离；但"resume 视图按 provider 过滤"是官方行为，表述时用"视图过滤"而非"隔离"）

**resume provider 过滤**:
Codex 0.146.0 官方行为：`codex resume` picker（本地）与 `codex exec resume --last` 仅列出当前 config `model_provider_id` 的会话；`--all` 只关 cwd 过滤；显式 `codex resume <session-id>`（UUID/名称）可绕过恢复任意会话。cct proxy profile 的 `model_provider=custom` 是其会话对直接运行（如 deepseek）不可见的原因。
_Avoid_: "会话丢失"（会话未丢，仅被过滤）

**history_mode**:
会话元数据中的持久化模式字段，本机实测值为 `legacy`。官方文档提及 `history.jsonl` 仅在启用 history persistence 时存在。
_Avoid_: 将 legacy 解读为"隔离模式"（未证实）

## 工作流 / 生命周期

**共享 ~/.codex 架构**:
cct 07-13 重构（`afc1d11`）后的 Codex 启动方式：不设 `CODEX_HOME`、不写任何 codex 配置文件，provider 配置经 `--config` CLI flags 注入（`model_provider=custom` 代理路径或 `model_provider=openai` 订阅路径）。所有 profile 共享同一状态根。
_Avoid_: "generate_codex_config"、"write_codex_auth"、"codex_home_for_profile"（已删除的函数）

**会话可见性**:
用户从 cct 启动 codex 后，通过 `codex resume` / TUI 会话列表能看到的会话集合——受**双重过滤**影响：model_provider 过滤（官方 picker 恒定，`--all` 关不掉）+ cwd（仓库）过滤（`--all` 可关）。物理共享 ≠ 可见性共享，这是本 draft 的核心区分。
_Avoid_: "会话共享"（易与物理共享混淆；表述时区分"物理共享"与"可见性"）
