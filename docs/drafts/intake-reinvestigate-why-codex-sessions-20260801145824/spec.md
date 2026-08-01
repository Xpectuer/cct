---
title: "Spec: cct proxy 死锁修复 + Codex 会话可见性验证与文档收尾"
doc_type: spec
brief: "Part A 修复 proxy current_thread 死锁（异步 accept + 应用层健康探测 + 僵尸自愈重启 + 并发防护 + 日志脱敏）；Part B 实测验证同 provider 会话可见性 + AC7 文档收尾"
confidence: speculative
created: 2026-08-01
updated: 2026-08-01
revision: 4
yields_from:
  - requirements.md
  - terminology.md
  - domain-model.md
---

# Spec

## Solution Summary

**Part A** 修复 cct proxy 的 runtime 死锁：控制 socket 从 `std::os::unix::net::UnixListener`（同步阻塞）改为 `tokio::net::UnixListener` 异步 accept（tokio UnixStream 经 `into_std()` 转 std 后交 `spawn_blocking` 执行同步 `handle_control`），与 HTTP 服务共享同一 current_thread runtime。健康检查升级为应用层探测（`send_control(status)` + 超时，探测超时写死 500ms × 3 次重试，stop 超时写死 2s）：**端口空闲判定用试探 bind（父进程 ensure_proxy_running 内执行，先 drop 再 spawn）**——探测失败且端口空闲（僵尸场景）→ 自动重新 spawn（**子进程启动时"先探测再删"统一负责 socket 清理，父进程不直接删 socket 文件**，避免 TOCTOU（Time-of-Check to Time-of-Use）检查-使用竞态下 unlink 并发启动的活 proxy），spawn 后循环探测就绪（**复用 500ms × 3 次常量**，重试耗尽 → 明确报错退出）；探测失败但端口被占（进程存活的旧版遗留/第三方进程）→ **明确报错退出**——报错信息经**只读 lsof 诊断**（`lsof -tiTCP:<port> -sTCP:LISTEN` 单次调用取占用者 PID，仅报错展示用，端口取自 `proxy_port()` 实际绑定值；**lsof 不可用或调用失败**（缺失/无权限/busybox 语法差异）时降级为"运行 `lsof -iTCP:<port>` 查看占用者"的纯建议文本；**子进程 TCP bind 失败同样输出 lsof 诊断文本**），不自动终止进程、**不引入 lsof/pidfile 自动终止机制**。`run_proxy` 启动时先探测再删 socket：有活 proxy → 报错退出（不破坏其控制通道）；**Unix 控制 socket 与 TCP 的 bind 失败同等处理**（报错退出而非 panic；控制 socket EADDRINUSE → 视为已有实例重新探测，**探测未响应则重试 3 次、每次 500ms，耗尽后报错退出**，覆盖双启动竞态并保证收敛）。`shutdown` 命令退出前清理 socket 文件（修复"每次 stop 留下死 socket"的稳态缺陷）。控制命令与请求日志打印前对 api_key 脱敏（遵守 mask-secrets-on-every-display-path）。`proxy_socket_path()` 增加 `CCT_PROXY_SOCKET` env 覆盖（仿 `CCT_CONFIG` / `CCT_KIMI_CONFIG` 既有先例，不违反接口冻结——未改既有接口），契约测试与 fake spawn 目标经此绑定临时路径。

**Part B** 修复后通过 codex exec 非交互链路（TUI picker 无法由 agent 操作）实测验证——测试会话写入**临时 CODEX_HOME**（不碰真实 `~/.codex`）：**smoke 用临时 profiles.toml（`CCT_CONFIG` 覆盖）定义 profile，`extra_args` 嵌入 `exec` 子命令**（如 `["exec", "-o", "<out>", "--dangerously-bypass-approvals-and-sandbox", "<prompt>"]`，`build_shared_codex_args` 将 extra_args 追加在旗标之后；**冒烟 profile 不设 full_auto**——非交互批准由 extra_args 内嵌的 bypass 旗标单点承担，避免重复旗标）——无 tty 下裸 `codex` 报错（本机 0.146.0 实测 `Error: stdin is not a terminal`）而 `codex exec` 正常，且 6 个 `--config` 旗标由真实函数生成、零 drift（满足 single-source-of-truth）；stub 上游按 responses-API SSE（Server-Sent Events）契约（`response.created` / `response.output_text.delta` / `response.completed` 事件流）实现，`-o` 文件断言末尾文本。同 provider（proxy/custom）会话在 `codex resume` 中互相可见（官方语义本就如此，若不符则定义为 cct 层 bug 追加修复）；跨 provider 不可见、显式 `codex resume <session-id>` 可恢复；cwd 过滤维度（`--all` 跨目录）一并验证。最后完成 AC7 文档收尾：消除 per-profile CODEX_HOME / `generate_codex_config` 陈旧叙述，新增"resume 按 model_provider ∩ cwd 过滤"语义说明。

## User Stories

1. As a cct Codex 用户（proxy 模式），I want cct 启动 codex 后能正常收到第一个 Response，so that 我才能验证并依赖会话连续性（当前 proxy 死锁直接阻断）。
2. As a cct Codex 用户（同 provider 多 profile），I want 同一 model_provider 的会话在 resume 中互相可见，so that 会话连续性不依赖 profile 名。
3. As a cct 维护者，I want 文档（CLAUDE.md / ARCHITECTURE.md / launch.md / codex 参考文档）准确描述共享 `~/.codex` 架构与官方 resume 的 provider/cwd 过滤语义，so that 后续开发者与用户不会基于过时叙述或错误预期做出错误决策。

## Terminology

| Term | Definition | Source |
|------|-----------|--------|
| CODEX_HOME | Codex 本地状态根（默认 `~/.codex`），承载 sessions/、state_5.sqlite 等；cct 07-13 起不设置，所有 profile 共享。_Avoid_: "per-profile CODEX_HOME"（旧架构已废弃） | [debate] 范围确认 |
| cct profile | `profiles.toml` 中的启动配置单元；决定 cct 注入的 `--config` flags，不决定会话存储位置。_Avoid_: 与 Codex profile（官方配置层）混用 | [debate] 范围确认 |
| cct proxy | cct 的本地 HTTP 转发代理（单实例，127.0.0.1:19191/v1），将 codex 请求转发至 profile 的 upstream 并注入 Bearer key；Unix 控制 socket 接收 status/switch/shutdown 命令；生命周期由 cct 拥有（ensure_proxy_running 启动、僵尸自愈重启）。与 auth_type 值 "proxy"、用户系统代理（如 Clash）区分 | [verify-round-1] terminology BLOCKER 2 → 修复 |
| model_provider | codex 配置中当前模型供应商标识（openai/custom/deepseek）；resume 的过滤维度之一（`ProviderFilter::MatchDefault`），`--all` 关不掉。**"provider" 为本文中 model_provider 的简称**。_Avoid_: "provider 隔离会话"（物理共享，仅视图过滤） | [debate] 可见性期望 |
| resume provider 过滤 | Codex 0.146.0 官方行为：`codex resume` picker 与 `codex exec resume --last` 仅列当前 `model_provider_id` 的会话；显式 `codex resume <id>` 可绕过。_Avoid_: "会话丢失"（会话未丢，仅被过滤） | [debate] 机制确认 |
| resume 仓库过滤 | `codex resume` 默认只列当前 cwd（当前工作目录）及其 git origin 的会话；`--all` 才跨目录 | [debate] 机制确认 |
| 会话可见性 | 用户从 cct 启动 codex 后 resume 能看到的会话集合 = {model_provider == 当前} ∩ {cwd == 当前}；物理共享 ≠ 可见性共享 | [debate] 机制确认 |
| 应用层探测 | 健康检查方式：通过 Unix 控制 socket 发送 `status` 命令并等待响应（带超时）；区别于内核层 socket connect（死 proxy 也能连上） | [debate] 死 proxy 恢复策略 |
| 死 proxy | 应用层无响应的 proxy；分为两类——(a) 僵尸：进程已退出、socket 文件残留、端口空闲；(b) 占端口：进程存活仍持有端口但无响应（修复前旧版本死锁遗留）。**死 socket**：指向已退出/无响应 proxy 的 Unix socket 文件 | [verify-round-1] risk BLOCKER 1 → 修复 |
| codex exec 链路 | 非交互自动化路径：`codex exec --dangerously-bypass-approvals-and-sandbox "<prompt>"` 创建会话、`codex exec resume --last` 验证可见性、`codex exec resume <session-id>` 显式恢复；断言用 `-o/--output-last-message <FILE>` 确定性捕获 + rollout 首条 session_meta 对比 session id。与 TUI picker 的**过滤语义一致**（源码取证：exec 用 resume_lookup_model_providers，TUI 用 ProviderFilter::MatchDefault，两条独立代码路径） | [debate] Smoke 自动化方案 |
| stub 上游 | 本地假 HTTP 服务充当 proxy 的 upstream，用于解耦"proxy 转发正确性"与"真实上游连通性"；两层契约——proxy 层契约测试的 stub 可协议无关（断言转发/Bearer key），L2（live 实测层）冒烟的 stub 按 responses-API SSE 契约实现（`response.created` / `response.output_text.delta` / `response.completed` 事件流） | [debate] pre-mortem mitigation |
| `cct run <profile>` | 既有 CLI 子命令：非交互启动 profile（走完整 proxy 链路 ensure→switch→env→flags→exec），无 profile 名时交互式选择；smoke 脚本以子进程方式调用（exec-replace 仅影响子进程本身）。不新增命令。**smoke 中经临时 profiles.toml（CCT_CONFIG 覆盖）+ extra_args 嵌入 `exec` 子命令驱动非交互对话** | [verify-interview] 非交互启动路径 |
| 临时 CODEX_HOME | L2（live 实测层）可见性测试专用状态根（临时目录），承载测试会话，不碰真实 `~/.codex`；测试后清理。产品运行时共享约束不变 | [verify-interview] 测试数据位置 |
| CCT_PROXY_SOCKET / CCT_PROXY_BIN | 新增测试注入 env（均仿既有先例）：`CCT_PROXY_SOCKET` 覆盖 `proxy_socket_path()`（仿 CCT_CONFIG / CCT_KIMI_CONFIG），契约测试与 fake spawn 目标经此绑定临时 socket 路径；`CCT_PROXY_BIN` 覆盖 `ensure_proxy_running` 的 spawn 目标（仿 CCT_CLAUDE_BIN），launch 层重启契约测试注入 fake 目标。两者均为新增、不改变既有接口 | [verify-round-2] implementability ADVISORY → 修复 |
| AC7 | 上轮 review 遗留的文档收尾工作：CLAUDE.md / ARCHITECTURE.md / docs/modules/launch.md / docs/references/codex-home-storage-layout.md / docs/references/codex-backend-development-guide.md 中 per-profile CODEX_HOME 与 `generate_codex_config` 陈旧叙述（README 无陈旧叙述，实测 0 处） | [debate] AC7 文档残留盘点 |

## Decisions

完整决策记录（含被拒方案）见 [decisions.md](decisions.md)。摘要：

1. **范围：两个一起** — Part A（proxy 死锁修复）+ Part B（会话可见性验证 + 文档收尾）；依赖链：proxy 修好才能实测同 provider 可见性。
2. **可见性期望：同 provider 可见 + 文档澄清** — 与官方语义一致、改动最小；拒绝跨 provider 统一列表（需自建 resume UI，违反 KISS）与仅文档收尾。
3. **Part A 机制：异步 accept** — `tokio::net::UnixListener` + `spawn_blocking`（`into_std()` 转换），与 HTTP 服务同一 runtime；tokio 已含 `full` features，无新依赖。
4. **死 proxy 恢复：僵尸自愈 + 占端口报错** — 自动自愈仅限进程已退出/端口空闲（端口空闲判定 = 父进程试探 bind；子进程"先探测再删"统一负责 socket 清理 + 重新 spawn + 就绪探测，重试耗尽明确报错）；进程存活占端口 → 明确报错（PID 经只读 lsof 诊断展示用，Alpine/musl 缺失时降级为定位命令文本），不自动 kill、**无 lsof/pidfile 自动终止机制**；旧版本遗留实例一次性手动迁移。
5. **并发启动防护：探测 + 报错** — 手动 `cct proxy start` 遇活 proxy → 报错退出且不删 socket 文件；`ensure_proxy_running` 自动路径 → 探测成功即复用返回成功；死 socket 清理重建；bind 失败报错退出不 panic。
6. **日志脱敏** — 控制命令与请求日志打印前对 api_key 脱敏（mask-secrets-on-every-display-path）。
7. **验收路径：stub/契约测试先行 + 用户 live 实测** — 契约测试（含 stub 上游、CCT_PROXY_BIN 注入、临时 socket/动态端口）解耦 proxy 正确性与上游连通；实测脚本分层诊断（curl --noproxy '*' → codex 对话）。
8. **Smoke 自动化：codex exec 链路 + 临时 CODEX_HOME** — TUI picker 无法由 agent 操作；非交互启动用既有 `cct run <profile>`（子进程调用），不新增命令。
9. **接口冻结** — 不改变 `CCT_PROXY_PORT` / `CCT_PROXY_LOG` / `cct proxy start|stop` / `cct run` 命令接口；回退 = revert 到上一发布版本（proxy 状态为内存态，无迁移负担）。

## Acceptance Criteria

- [Smoke] **Given** 修复后的 cct proxy 运行中且控制 socket 有并发命令，**when** HTTP 请求到达 127.0.0.1:19191/v1，**then** 请求得到响应而非无限挂起（死锁回归契约测试）。
- [Smoke] **Given** 存在僵尸死 proxy（进程已退出：socket 文件残留、端口空闲），**when** 用户从 cct 启动 codex（proxy 模式），**then** cct 应用层探测失败 → 端口空闲判定（父进程试探 bind）→ 子进程"先探测再删"清理死 socket → 自动重新 spawn → 就绪探测成功（重试耗尽则明确报错）→ codex 正常收到第一个 Response。
- **Given** 进程存活但占用 19191 端口（旧版死锁遗留或第三方进程），**when** 启动新 proxy，**then** 明确报错退出（PID 经只读 lsof 诊断展示；lsof 缺失时给出定位命令文本），不 panic、不自动终止进程。
- [Smoke] **Given** proxy 指向 stub 上游（本地假 HTTP 服务，responses-API SSE 契约），**when** smoke 脚本以子进程运行 `cct run <profile>`（临时 profiles.toml + extra_args 嵌入 `exec` 子命令）发起对话，**then** 请求经 proxy 正确转发（带 Bearer key）且流式返回（`-o` 文件断言末尾文本）——证明 cct→proxy→上游整条链路正确，与真实上游连通性解耦。
- **Given** CCT_PROXY_LOG 开启，**when** 控制命令（含 api_key 的 switch）与 HTTP 请求日志打印，**then** 日志输出不含 api_key 明文（契约测试断言）。
- [Smoke] **Given** 两个同 provider（proxy/custom）cct profile 在同一仓库（临时 CODEX_HOME），**when** profile A 下经 `cct run <profile-A>`（extra_args 嵌入 `exec`）对话后、再经 `cct run <profile-B>`（extra_args 嵌入 `exec resume --last -o <out>`，6 旗标由真实函数生成、禁止手工复刻）运行，**then** 输出中出现 profile A 会话的 session-id（`-o` 捕获 + rollout session_meta 对比）；若实测不符，定义为 cct 层 bug 并追加修复。
- [Smoke] **Given** 同一仓库存在不同 provider 的会话（临时 CODEX_HOME），**when** 切换 provider 后经 `cct run <smoke-profile>`（extra_args 嵌入 `exec resume --last -o <out>`）运行，**then** 输出中不包含另一 provider 的任何 session-id；显式 `codex exec resume <session-id>` 可恢复。
- [Smoke] **Given** 同 provider 跨仓库会话（临时 CODEX_HOME），**when** 当前 cwd 下经 `cct run <smoke-profile>`（extra_args 嵌入 `exec resume --last -o <out>`，cwd 由 smoke 脚本控制）运行，**then** 不可见；追加 `--all` 后可见（cwd 过滤维度）。
- **Given** 已有活 proxy 运行，**when** 手动执行 `cct proxy start`，**then** 应用层探测成功 → 报错退出，不删除活 proxy 的 socket 文件，不 panic；**when** `ensure_proxy_running` 自动路径，**then** 探测成功即复用返回成功。
- **Given** 修复后的 proxy，**when** 契约测试运行，**then** 覆盖：控制 socket 与 HTTP 并发请求、僵尸死 socket 场景、TCP 端口占用场景（断言报错退出码与消息）、**双启动竞态（控制 socket bind EADDRINUSE → 重新探测而非 panic）**、stub 上游转发、CCT_PROXY_LOG 脱敏、无响应 proxy 上 `cct proxy stop` 在 2s 超时后返回错误而非永久挂起；契约测试全部使用临时目录 socket 路径（经 `CCT_PROXY_SOCKET` 覆盖）+ 动态端口（与用户实例隔离）；launch 层重启契约经 `CCT_PROXY_BIN` env 注入（仿 CCT_CLAUDE_BIN 既有先例）测试 spawn 目标，fake 目标经 `CCT_PROXY_SOCKET` 接收临时路径并应答 status 探测。
- **Given** 用户本机存在旧版死锁实例（PID 29182）与遗留死 socket 文件，**when** 升级到修复版本，**then** 迁移步骤明确：**旧版死锁实例若控制 socket 仍响应，新版本应用层探测会将其视为健康并复用（此时 codex HTTP 请求仍挂起）——此场景唯一修复路径是用户手动终止旧进程**；用户手动终止（释放 TCP 端口）+ 删除遗留 socket（兜底；新版本启动时探测失败会自行清理，顺序无关均安全），作为升级说明写入 `docs/references/install-script.md`（一次性；新版本不再产生死锁进程）。占端口报错路径实际覆盖的是"控制 socket 不可达 + TCP 被占"（旧实例 socket 已死/被删，或第三方进程）。
- **Given** L2 冒烟测试执行，**when** 启动，**then** 前置条件满足：先完成迁移（PID 29182 终止）、测试期间不并行运行 cct/proxy（避免互踩真实 socket），smoke 脚本启动时按 `proxy_port()` 实际端口预检占用并给出迁移提示。
- **Given** 文档收尾改动，**when** 完成，**then** CLAUDE.md / ARCHITECTURE.md / docs/modules/launch.md / docs/references/codex-home-storage-layout.md / docs/references/codex-backend-development-guide.md 不再存在 per-profile CODEX_HOME 与 `generate_codex_config` 的陈旧叙述，并新增"resume 按 model_provider ∩ cwd 过滤"语义说明；session-cards / procs / context-* 历史快照不改动。
- **Given** 任何 cct 层改动，**when** 合入，**then** 不写任何 Codex 配置文件（config.toml / auth.json / profile-*.config.toml，配置快照对比回归测试断言），不改变 `CCT_PROXY_PORT` / `CCT_PROXY_LOG` / `cct proxy start|stop` / `cct run` 命令接口。
- **Given** live 实测，**when** 分层诊断执行，**then** 先 `curl --noproxy '*'` 验证 proxy 层响应（无上游时 502/404 亦为 proxy 存活的证据），再经 codex 对话验证上游层，失败时能定位到具体层（pre-mortem 防护）。

## Open Questions

1. **issue #9 关闭时机**：文档收尾完成后是否关闭 issue #9？（用户是 repo owner，由用户决定）
2. **`codex exec resume --last` 的 session-id 打印行为**：`-o/--output-last-message` 与 rollout session_meta 对比断言已定，但 `--last` 是否直接打印 session-id 待实测第一步确认——验证脚本按语义契约（会话 id 存在性）断言，不依赖逐字输出。
3. **TUI picker 手动验证**：agent 无法操作 TUI；用户是否自愿在修复后手动打开 `codex resume` 做一次可视化确认（可选，不阻塞 AC）。
4. **Clash 代理环境下的上游连通**：用户本机 shell 有 `http_proxy=127.0.0.1:7892`，reqwest 默认读系统代理设置；若上游 base_url 需代理访问属预期行为，若被意外劫持需在实测时确认（不属本 spec 修复范围）。
