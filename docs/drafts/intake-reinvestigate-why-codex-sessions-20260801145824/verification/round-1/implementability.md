# Verifier Report: Implementability (Angle 3/7)

## Score: 6/10
## Verdict: FAIL

## Findings

### BLOCKER: 死 proxy 自动重启 AC 在"进程存活但死锁"场景下不可实现——机制缺失（无法终止仍占用 19191 端口的旧进程）
- Location: spec.md:Solution Summary (Part A) + AC 2 + Terminology（死 proxy 定义）
- Evidence: 死锁 proxy（PID 29182）仍持有 TCP 19191；socket 删除杀不掉进程、shutdown 对死锁进程无应答；proxy.rs:199-200 bind 失败 panic；修复后首次升级运行的真实路径 = 探测超时 → 删 socket → spawn → bind AddrInUse → panic → 5s 超时 bail
- Fix: 二选一：(a) pidfile（proxy.sock 旁 proxy.pid）+ 校验 PID 归属后 SIGTERM；(b) 按端口定位（lsof -tiTCP:19191 -sTCP:LISTEN / fuser）验证后终止。同时 proxy.rs:199 panic 改可恢复路径。附带：check_proxy_running 升级后 cct proxy stop（main.rs:237）对死锁 proxy 会误报"Proxy is not running"

### ADVISORY: `codex exec resume --last` 断言格式未定义
- Location: spec.md:Open Questions 2 + AC 4/5
- Evidence: 本机 0.146.0 实测 `codex exec resume` 帮助含 `-o/--output-last-message <FILE>` 可确定性捕获；会话 id 可从 rollout 首条 session_meta 读取
- Fix: 用 `-o <file>` 捕获输出 + rollout 文件对比 session id（同 provider 命中 / 跨 provider 变化 / 显式 id 绕过）

### ADVISORY: Part B codex exec 链路的环境复现条件未写明
- Location: spec.md:Terminology（codex exec 链路）
- Evidence: codex exec 直接运行走用户 config.toml（本机 model_provider=deepseek），不会自动带 model_provider=custom；不复刻 build_codex_proxy_config_args 的 6 个 --config 旗标 + OPENAI_API_KEY 注入则会话归错 provider，可见性断言失真
- Fix: Part B 验证步骤显式写"使用 build_codex_proxy_config_args 等价旗标 + env 注入"作为前置条件

### ADVISORY: 控制 socket 改造的机械细节未列（不影响可行性）
- Location: spec.md:Decisions 4
- Evidence: handle_control 签名是 std::os::unix::net::UnixStream（proxy.rs:386）；tokio UnixListener::accept() 产出 tokio UnixStream 需 .into_std() 后再 spawn_blocking；启动时无条件 remove_file（proxy.rs:185）需改"先探测再删"
- Fix: 实施时注意 into_std() 与文件清理语义（无需改 spec 决策）

## 已验证通过的关键点
1. 异步 accept + spawn_blocking 在 current_thread runtime 下成立（tokio full features 含 net/rt；spawn_blocking 有先例 proxy.rs:376）✓
2. 应用层探测可实现（send_control 已实现 status 协议；set_read_timeout 加超时无新依赖）✓
3. codex exec 链路真实且非交互（--dangerously-bypass-approvals-and-sandbox、exec resume --last/--all/显式 id 均实测存在）✓
4. 文档收尾清单与真实文件对应（CLAUDE.md/launch.md/layout/guide 均含陈旧叙述；ARCHITECTURE.md 基本干净）✓
