---
title: "Decision Record: Codex session isolation + proxy deadlock fix"
doc_type: proc
brief: "Debate 决策记录 — 范围、可见性方案、proxy 修复机制、死 proxy 恢复策略"
confidence: verified
created: 2026-08-01
updated: 2026-08-01
revision: 2
---

# Decision Record

## Step 1 — Debate 范围

### Decision: 两个一起（proxy 修复 + 会话可见性）（Accepted）

**Status**: Accepted
**Context**: intake 调查确认两个独立问题——① cct proxy 死锁（P0 bug，阻塞同 provider 可见性验证前提）；② issue #9 会话可见性（机制 = Codex 官方 resume 按 model_provider 过滤，方案待定）。
**Considered**: 仅 proxy 死锁修复（rejected: 可见性议题仍悬置，用户明确关注"被隔离的会话"）；仅会话可见性（rejected: proxy bug 不修则用户无法正常对话，可见性也无法实测验证）
**Rationale**: 两条线有依赖关系——proxy 修好才能实测同 provider 可见性，实测结果影响可见性方案细节；一个 spec 两条线避免两个流程重复收集上下文。
**Consequences**: + 一次辩论解决全部已知问题，依赖链清晰；- spec 范围较大，需明确分区（Part A / Part B）。

## Step 1 — 可见性期望行为

### Decision: 同 provider 可见 + 文档澄清（Accepted）

**Status**: Accepted
**Context**: 官方机制已确认——resume 按 model_provider 过滤（`--all` 关不掉），同 provider 同仓库本应可见（被 proxy 死锁阻塞验证），跨 provider 只能显式 `codex resume <id>`。
**Considered**: 跨 provider 恢复入口（rejected: 需 cct 读取 codex 外部会话索引（只读外部状态）+ 自建 resume UI，工作量大且与官方语义不一致，KISS 反对）；仅文档收尾（rejected: 不满足用户对"当前 HEAD 仍存在被隔离会话"的关注）
**Rationale**: 与官方语义一致、改动最小；先实测验证同 provider 可见性（若不符才是 cct 真 bug），差异写入文档（AC7）。
**Consequences**: + 无 cct 功能改动或极小，KISS；- 跨 provider 会话仍需显式 `codex resume <id>` 恢复（官方限制，cct 只做文档说明）。

## Step 1 — 死 proxy 恢复策略

### Decision: 自动重启（Accepted）

**Status**: Accepted
**Context**: 死锁 proxy 会被 `check_proxy_running`（仅内核 socket connect）误判为健康并复用于启动 codex。
**Considered**: 报错让用户手动（rejected: 每次遇到死 proxy 都要手动干预，体验差）；仅修死锁不动检查（rejected: 升级后遗留的旧死进程仍会误判健康）
**Rationale**: 健康检查升级为应用层探测（send status + 超时），失败自动清理死 socket 并重新 spawn——与 ensure_proxy_running 现有 spawn-if-needed 逻辑自然衔接，用户无感知。
**Consequences**: + 死 proxy 场景自动自愈；- check_proxy_running 多一次应用层探测的开销（毫秒级，可接受）。

## Step 3 — Part A 修复机制

### Decision: 异步 accept（tokio::net::UnixListener）（Accepted）

**Status**: Accepted
**Context**: `run_control_socket` 用 std 同步阻塞 `UnixListener` 跑在 tokio current_thread runtime 上，`accept()` 阻塞整个 runtime，HTTP 服务永不调度。
**Considered**: 独立线程阻塞（rejected: 引入第二线程与 socket 清理时机的新问题，且 handle_control 同步逻辑仍需 spawn_blocking）；独立 runtime（rejected: 两个 runtime 两套调度，过度设计，KISS 反对）
**Rationale**: 控制 socket 改 `tokio::net::UnixListener` 异步 accept，与 HTTP 服务同一 runtime；handle_control 保持同步经 spawn_blocking（tokio UnixStream 经 `into_std()` 转换；current_thread runtime 下 spawn_blocking 使用独立线程池，不受影响）。tokio 已含 `full` features，无新依赖。
**Consequences**: + 最小 diff、风格一致、无新依赖；- 机械细节：`into_std()` 转换与 socket 清理语义（启动时"先探测再删"）需保持。

### Decision: 占端口死 proxy 处理：僵尸自愈 + 报错手动（Accepted，修订 2）

**Status**: Accepted
**Context**: [verify-round-1] risk/implementability BLOCKER——自动重启对"进程存活占端口"的死锁 proxy 不成立（bind EADDRINUSE → panic）；修复后新 proxy 永不产生死锁，占端口场景仅来自旧版遗留或第三方进程。[verify-round-2] implementability INTERVIEW_NEEDED——"报错含占用者 PID" 与 "无 lsof/pidfile" 在 macOS 上矛盾（无 /proc、bind 错误不含 PID）。
**Considered**: pidfile + SIGTERM 自动终止（rejected: 引入新状态文件与 kill 机制，归属验证逻辑，违反 KISS）；lsof/fuser 按端口定位终止（rejected: 依赖外部命令，macOS/Linux 差异）
**Rationale**: 自动自愈仅限僵尸场景（进程已退出、端口空闲；端口空闲判定 = 父进程试探 bind）；占端口 → 明确报错（PID 经**只读 lsof 诊断**单次调用展示用，Alpine/musl 缺失时降级为定位命令文本；"无 lsof 机制"解释为"无 lsof **自动终止**机制"）。升级路径上用户手动处理一次（迁移步骤写入文档）。
**Consequences**: + 无自动终止机制、无误杀第三方进程风险；- 升级首次运行需用户手动干预一次（一次性）。

### Decision: 并发启动防护：探测 + 报错（Accepted，修订 2）

**Status**: Accepted
**Context**: [verify-round-1] consistency/risk ADVISORY——"报错/复用"表述二义；[verify-round-2] risk ADVISORY——Unix 控制 socket bind 竞态（并发双启动时败者 `expect` panic，proxy.rs:187）未被 TCP bind 修复覆盖。
**Considered**: 仅报错（rejected: ensure_proxy_running 自动路径本就需复用）
**Rationale**: 拆两条路径——手动 `cct proxy start` 遇活 proxy → 报错退出（不删 socket 文件）；`ensure_proxy_running` 自动路径 → 探测成功即复用返回成功。**控制 socket 与 TCP 的 bind 失败同等处理**：报错退出而非 panic；控制 socket EADDRINUSE（双启动竞态）→ 视为已有实例重新探测。
**Consequences**: + 语义唯一，无无人消费的复用分支；- 两条路径行为不同需文档说明。

### Decision: 日志脱敏（Accepted）

**Status**: Accepted
**Context**: [verify-round-1] risk BLOCKER——CCT_PROXY_LOG 开启时 `log_proxy!("ctl << {}", line.trim())` 原样打印含 api_key 的 switch 命令 JSON。
**Rationale**: 控制命令与 HTTP 请求日志打印前对 api_key 脱敏（mask-secrets-on-every-display-path 规则）；契约测试断言日志不含 api_key。
**Consequences**: + 消除泄漏面；- 日志可读性略降（key 已 mask）。

### Decision: 验收路径：stub/契约测试先行 + 用户 live 实测（Accepted）

**Status**: Accepted
**Context**: [debate] delivery 维度确认。
**Considered**: 仅自动化实测自选（rejected: 同 provider 可见性结论滞后）；先最小验证再 TDD（rejected: 已是第二次调查，契约测试先行同样稳）
**Rationale**: 契约测试（stub 上游、CCT_PROXY_BIN 注入、临时 socket/动态端口）解耦 proxy 正确性与上游连通；用户配合 live 实测同 provider 可见性。
**Consequences**: + 分层可诊断；- 需要用户配合一次实测。

### Decision: Smoke 自动化：codex exec 链路 + 临时 CODEX_HOME + 既有 cct run（Accepted）

**Status**: Accepted
**Context**: [verify-round-1] testdepth BLOCKER（AC3 无 agent 可执行机制）+ INTERVIEW_NEEDED。
**Considered**: 新增 `cct launch <profile>` 子命令（rejected: 用户确认既有 `cct run <profile>` 已存在且走完整 proxy 链路）；example binary（rejected: 用户选择既有命令）
**Rationale**: TUI picker 无法由 agent 操作 → codex exec 非交互链路（断言用 `-o/--output-last-message` + rollout session_meta 对比）；启动入口用既有 `cct run <profile>`（子进程调用，exec-replace 仅影响子进程），6 个 `--config` 旗标由 cct run 内部真实函数（build_codex_proxy_config_args）生成、禁止手工复刻（single-source-of-truth）+ OPENAI_API_KEY 注入；测试会话写临时 CODEX_HOME（用户确认，不碰真实 ~/.codex）。
**Consequences**: + 零新增命令、隔离用户真实状态、零旗标 drift；- 冒烟脚本需构造临时 profiles.toml + extra_args 嵌入 exec（脚本稍复杂）。

### Decision: 探测/stop 超时语义（Accepted）

**Status**: Accepted
**Context**: [verify-round-1] risk ADVISORY（send_control read_line 无超时，死 proxy 会让 stop 永久挂起）+ [verify-round-2] completeness ADVISORY（stop 超时行为缺决策溯源）。
**Rationale**: 探测超时写死 500ms × 3 次重试（过短会把响应慢的健康 proxy 误判为僵尸并破坏其控制通道——"不破坏控制通道"承诺的前提是超时值宽容）；stop 超时写死 2s；契约测试固化同一常量。数值写死不引入配置项（KISS）。
**Consequences**: + stop 对死 proxy 不再永久挂起；- 常量硬编码（如需调整改代码）。

### Decision: 接口冻结与回退（Accepted）

**Status**: Accepted
**Context**: [verify-round-1] consistency ADVISORY——AC8 后半句为孤儿约束。
**Rationale**: 不改变 `CCT_PROXY_PORT` / `CCT_PROXY_LOG` / `cct proxy start|stop` / `cct run` 命令接口；回退 = revert 到上一发布版本（proxy 状态为内存态，无迁移负担）。
**Consequences**: + 兼容性明确；- 无。
