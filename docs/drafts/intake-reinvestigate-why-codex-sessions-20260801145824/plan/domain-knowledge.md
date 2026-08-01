---
title: "Domain Knowledge: cct proxy 死锁修复 + Codex 会话可见性验证与文档收尾"
doc_type: proc
brief: "Domain entities, terminology, and business rules for implementation"
confidence: verified
yields_from:
  - spec.md
created: 2026-08-01
updated: 2026-08-01
revision: 1
---

# Domain Knowledge

## Entities

| Entity | Description | Key Attributes |
|--------|-------------|----------------|
| cct proxy | cct 的本地 HTTP 转发代理（单实例 127.0.0.1:19191/v1）；Unix 控制 socket 接收 status/switch/shutdown 命令；生命周期由 cct 拥有 | TCP 端口（CCT_PROXY_PORT，默认 19191）；控制 socket 路径（proxy_socket_path()，CCT_PROXY_SOCKET 可覆盖）；CCT_PROXY_LOG 日志；current_thread tokio runtime（异步 accept + 同步 handle_control via spawn_blocking） |
| 控制 socket | Unix domain socket，控制命令通道（status/switch/shutdown） | 当前实现 `std::os::unix::net::UnixListener` 同步阻塞 → 改为 tokio 异步 accept；start 时"先探测再删"；shutdown 时清理文件 |
| 僵尸死 proxy | 进程已退出、socket 文件残留、端口空闲 | 应用层探测失败 + 端口空闲 → 自愈路径（重启） |
| 占端口 proxy | 进程存活仍持有端口但无响应（旧版本死锁遗留/第三方） | 应用层探测失败 + 端口被占 → 报错退出 + lsof 诊断（只读展示） |
| model_provider | codex 配置中当前模型供应商标识（openai/custom/deepseek）；resume 的过滤维度之一 | proxy 模式 → "custom"；subscription 模式 → "openai"（build_codex_proxy_config_args 的 model_provider 旗标决定） |
| codex session | 本地会话记录（sessions/*.jsonl + state_5.sqlite），存于 CODEX_HOME（默认 ~/.codex，cct 07-13 起不设置，所有 profile 共享） | session-id（rollout 首条 session_meta）；provider 归属；cwd/git-origin 归属 |
| resume | codex 恢复会话机制：picker（TUI）与 `exec resume --last`/`<id>`（非交互） | 过滤 = {model_provider == 当前} ∩ {cwd == 当前}；`--all` 绕过 cwd 过滤但关不掉 provider 过滤；显式 `<session-id>` 绕过全部过滤 |
| stub 上游 | 本地假 HTTP 服务充当 proxy 的 upstream | 两层：契约测试 stub 可协议无关（断言转发/Bearer）；L2 smoke stub 按 responses-API SSE 契约（response.created / response.output_text.delta / response.completed） |
| L2 smoke profile | 临时 profiles.toml 中定义的测试 profile | extra_args 嵌入 `exec` 子命令（含 -o 输出文件 + bypass 旗标）；不设 full_auto |

## Terminology

| Term | Definition | Avoid |
|------|------------|-------|
| CODEX_HOME | Codex 本地状态根（默认 ~/.codex），承载 sessions/、state_5.sqlite；cct 07-13 起不设置，所有 profile 共享 | "per-profile CODEX_HOME"（旧架构已废弃） |
| model_provider | 当前模型供应商标识；"provider" 为其简称 | "provider 隔离会话"（物理共享，仅视图过滤） |
| 应用层探测 | 通过控制 socket 发送 status 命令并等待响应（带超时）；区别于内核层 connect（死 proxy 也能连上） | — |
| 死 socket | 指向已退出/无响应 proxy 的 Unix socket 文件 | — |
| 会话可见性 | resume 能看到的会话集合 = {model_provider == 当前} ∩ {cwd == 当前} | "会话丢失"（未丢，仅被过滤） |
| resume provider 过滤 | Codex 0.146.0 官方行为：仅列当前 model_provider_id 的会话；显式 resume <id> 可绕过 | — |
| resume 仓库过滤 | 默认只列当前 cwd 及其 git origin 的会话；`--all` 跨目录 | — |
| cct run <profile> | 既有 CLI 子命令：非交互启动 profile（ensure→switch→env→flags→exec），无 profile 名时交互式选择 | 与 TUI 启动路径区分 |
| 试探 bind | 父进程端口空闲判定：尝试 bind 目标 TCP 端口，成功=空闲（drop 后再 spawn），失败=被占 | — |

## Business Rules

- **生命周期**：cct 拥有 proxy 生命周期；ensure_proxy_running 启动、僵尸自愈重启；单实例。
- **启动顺序（父进程 ensure_proxy_running）**：应用层探测 status → 成功=复用返回；失败 → 试探 bind 判端口 → 空闲=spawn 新进程（先 drop bind）→ spawn 后就绪探测（500ms×3）→ 成功返回/耗尽报错；被占=报错退出（lsof 诊断展示）。
- **启动顺序（子进程 run_proxy）**：先探测再删 socket（有活 proxy → 报错退出不破坏其控制通道）；TCP bind 失败 → lsof 诊断文本报错；控制 socket EADDRINUSE → 重新探测 3 次 × 500ms，耗尽报错退出（双启动竞态收敛）。
- **socket 清理责任**：子进程"先探测再删"统一负责 socket 清理；父进程不直接 unlink（避免 TOCTOU）；shutdown 命令退出前清理 socket 文件。
- **探测/超时常量**：探测 500ms × 3 次重试；stop 2s 超时返回错误。均写死，不可配置。
- **日志脱敏**：控制命令（含 api_key 的 switch）与请求日志打印前脱敏，遵守 mask-secrets-on-every-display-path。
- **接口冻结**：CCT_PROXY_PORT / CCT_PROXY_LOG / proxy start|stop / run 接口不变；新增 env（CCT_PROXY_SOCKET / CCT_PROXY_BIN）为测试注入点。
- **Codex 配置隔离**：cct 层不写任何 Codex 配置文件（config.toml / auth.json / profile-*.config.toml）；仅通过 exec 前 env + --config 旗标注入。
- **配置生成单一来源**：6 个 --config 旗标只经 build_codex_proxy_config_args 生成；smoke/测试禁止手工复刻旗标。
- **非交互批准**：smoke profile 不设 full_auto；bypass 旗标经 extra_args 单点嵌入。
- **测试隔离**：L2 冒烟用临时 CODEX_HOME + 临时 profiles.toml（CCT_CONFIG）；契约测试用 CCT_PROXY_SOCKET 临时路径 + 动态端口；不碰真实状态。
- **可见性语义**：同 provider 会话 resume 互相可见（官方语义）；跨 provider 不可见；显式 resume <id> 可绕过全部过滤；--all 仅绕过 cwd 过滤。
