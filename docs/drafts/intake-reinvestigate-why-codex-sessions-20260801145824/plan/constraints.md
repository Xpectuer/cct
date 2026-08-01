---
title: "Constraints: cct proxy 死锁修复 + Codex 会话可见性验证与文档收尾"
doc_type: proc
brief: "Acceptance criteria and hard constraints from spec"
confidence: verified
yields_from:
  - spec.md
created: 2026-08-01
updated: 2026-08-01
revision: 1
---

# Constraints

## Acceptance Criteria

| # | Criterion | Verifiable Check |
|---|-----------|-----------------|
| 1 | [Smoke] 控制 socket 有并发命令时 HTTP 请求得到响应而非无限挂起 | 死锁回归契约测试：并发控制命令 + HTTP 请求均完成 |
| 2 | [Smoke] 僵尸死 proxy（进程退出、socket 残留、端口空闲）→ 应用层探测失败 → 父进程试探 bind 判端口空闲 → 子进程"先探测再删"清理 → 重新 spawn → 就绪探测（500ms×3，耗尽明确报错）→ codex 收到第一个 Response | 契约测试制造僵尸场景后 ensure_proxy_running 自愈重启；探测重试耗尽报错退出非 panic |
| 3 | 进程存活占 19191 → 明确报错退出（只读 lsof 诊断 PID；lsof 缺失降级为定位命令文本），不 panic、不自动终止 | 契约测试断言退出码 + 报错含占用信息 + 占用者进程未被 kill |
| 4 | [Smoke] cct→proxy→stub SSE 上游整条链路：Bearer key 转发 + 流式返回（`-o` 文件断言末尾文本） | 契约测试（stub 转发）+ L2 smoke（responses-API SSE stub，`-o` 含末尾文本 + stub.log 含 Bearer） |
| 5 | CCT_PROXY_LOG 开启时控制命令（含 api_key 的 switch）与请求日志不含 api_key 明文 | 契约测试 grep 日志无测试 key 明文 |
| 6 | [Smoke] 同 provider（proxy/custom）多 profile 会话 resume --last 互相可见（临时 CODEX_HOME） | L2 smoke：profile B 的 `-o` 输出含 profile A 会话末尾文本；不符则定义为 cct bug 追加修复 |
| 7 | [Smoke] 跨 provider 不可见；显式 `codex exec resume <id>` 可恢复 | L2 smoke：跨 provider `-o` 不含另一 provider session 文本；显式 resume 含之 |
| 8 | [Smoke] cwd 过滤：跨仓库默认不可见，追加 `--all` 可见 | L2 smoke：repo2 默认 `-o` 无标记；`--all` 后有标记 |
| 9 | 活 proxy 时手动 `cct proxy start` → 报错退出，不删活 proxy socket，不 panic；ensure_proxy_running 自动路径探测成功即复用 | 契约测试断言退出码 + socket 文件完好 + 原 proxy 仍响应 |
| 10 | 契约测试覆盖：并发、僵尸、占端口、双启动竞态（EADDRINUSE 重新探测非 panic）、stub 转发、日志脱敏、stop 2s 超时返回错误；全部用临时 socket（CCT_PROXY_SOCKET）+ 动态端口；launch 重启经 CCT_PROXY_BIN 注入 fake 目标 | cargo test proxy + integration 全绿；fake 目标经 CCT_PROXY_SOCKET 应答 status 探测 |
| 11 | 升级迁移说明写入 docs/references/install-script.md：旧实例 socket 仍响应会被视为健康复用（唯一修复路径=手动终止）+ 手动终止 + 删除遗留 socket 兜底 | install-script.md 含迁移段落（一次性） |
| 12 | L2 冒烟前置：先迁移（PID 29182 终止）、测试期间不并行运行 cct/proxy、启动时按 proxy_port() 实际端口预检占用并给迁移提示 | smoke 脚本启动时端口预检，占用则 FAIL + 迁移提示 |
| 13 | 5 份文档（CLAUDE.md / ARCHITECTURE.md / docs/modules/launch.md / docs/references/codex-home-storage-layout.md / docs/references/codex-backend-development-guide.md）无 per-profile CODEX_HOME 与 generate_codex_config 陈旧叙述 + 新增"resume 按 model_provider ∩ cwd 过滤"语义说明；session-cards / procs / context-* 历史快照不改 | grep 5 文档无陈旧模式 + 语义说明存在 |
| 14 | 不写任何 Codex 配置文件（config.toml / auth.json / profile-*.config.toml）+ 接口不变（CCT_PROXY_PORT / CCT_PROXY_LOG / proxy start|stop / run） | 配置快照回归测试（对比前后文件集合）+ 既有接口测试仍绿 |
| 15 | [Smoke] live 分层诊断：先 curl --noproxy '*' 验证 proxy 层（502/404 亦存活），再 codex 对话验证上游层 | L2 smoke 步骤顺序：proxy 层诊断通过后进入 codex 对话 |

## Hard Constraints

| # | Constraint | Type | Detail |
|---|-----------|------|--------|
| 1 | 异步 accept 用 `tokio::net::UnixListener` + `spawn_blocking`（`into_std()` 转 std 后执行同步 handle_control），与 HTTP 服务同一 current_thread runtime | tech | tokio 已含 full features，无新依赖 |
| 2 | 探测超时写死 500ms × 3 次重试；stop 超时写死 2s | tech | 不可配置，直接常量 |
| 3 | 端口空闲判定 = 父进程 ensure_proxy_running 内试探 bind，先 drop 再 spawn；子进程启动时"先探测再删"统一负责 socket 清理，父进程不直接 unlink | tech | 避免 TOCTOU 下 unlink 并发启动的活 proxy |
| 4 | 占端口报错：只读 lsof 诊断（`lsof -tiTCP:<port> -sTCP:LISTEN` 单次调用，仅报错展示用），lsof 不可用降级为"运行 `lsof -iTCP:<port>`"建议文本；不自动 kill、无 lsof/pidfile 自动终止机制 | scope | 端口取自 proxy_port() 实际绑定值；子进程 TCP bind 失败同样输出诊断 |
| 5 | run_proxy 启动时先探测再删：有活 proxy → 报错退出不破坏其控制通道；控制 socket EADDRINUSE → 视为已有实例重新探测（3 次 × 500ms，耗尽报错退出），不 panic | tech | 覆盖双启动竞态并保证收敛 |
| 6 | shutdown 命令退出前清理 socket 文件 | tech | 修复"每次 stop 留下死 socket"稳态缺陷 |
| 7 | 日志脱敏：控制命令与请求日志打印前 api_key 脱敏 | security | 遵守 mask-secrets-on-every-display-path；控制命令为结构化 JSON——按 api_key 字段名脱敏（不依赖值前缀）；请求日志 path 按 sk- 值扫描兜底；两 helper 集中于 proxy 模块（Step 8），不散落 open-code |
| 8 | `proxy_socket_path()` 增加 CCT_PROXY_SOCKET env 覆盖（仿 CCT_CONFIG / CCT_KIMI_CONFIG）；新增 CCT_PROXY_BIN 覆盖 spawn 目标（仿 CCT_CLAUDE_BIN） | compat | 均为新增 env，不改既有接口 |
| 9 | 接口冻结：不改变 CCT_PROXY_PORT / CCT_PROXY_LOG / `cct proxy start|stop` / `cct run` 接口；回退 = revert 上一发布版 | compat | 配置快照回归断言不写 Codex 配置 |
| 10 | 6 个 `--config` 旗标由真实函数 build_codex_proxy_config_args 生成，禁止手工复刻 | tech | single-source-of-truth；smoke 经 extra_args 嵌入 exec 子命令 |
| 11 | smoke profile 不设 full_auto；非交互批准由 extra_args 内嵌 `--dangerously-bypass-approvals-and-sandbox` 单点承担 | tech | 避免重复旗标 |
| 12 | 测试用临时 CODEX_HOME + 临时 profiles.toml（CCT_CONFIG 覆盖），不碰真实 ~/.codex / 真实配置 | scope | 产品运行时共享 CODEX_HOME 约束不变 |
| 13 | smoke 以子进程方式调用 `cct run <profile>`（exec-replace 仅影响子进程本身），不新增命令 | scope | 非交互启动路径复用既有 CLI |
| 14 | session-cards / procs / context-* 历史快照不改动 | scope | 只改 5 份活跃文档 |
| 15 | 兼容性基线：修复前实测复现死锁（B015 FAIL）+ 旧实例 PID 29182 占端口 | baseline | 修复后 B015 PASS + B012 通过（迁移完成） |
