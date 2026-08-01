# Verifier Report: Implementability (Angle 3/7) — Round 2

## Score: 7/10
## Verdict: PASS（以 INTERVIEW_NEEDED 先于 Part A 实施解决为前提）

## Findings

### ADVISORY: "报错含占用者 PID" 与 "无 pidfile/lsof 机制" 在 macOS 上自相矛盾
- Location: spec.md:Solution Summary + Decisions 4 + 占端口 AC
- Evidence: macOS 无 /proc；bind AddrInUse 错误不含 PID；无 lsof/pidfile 则进程内无法得知占用者 PID
- Fix: 二选一（需用户定夺，见 INTERVIEW_NEEDED）

### ADVISORY: "契约测试全部使用临时目录 socket 路径" 在 spawn 层不可实现
- Location: spec.md:契约测试 AC
- Evidence: proxy_socket_path() 走 dirs::config_dir()（macOS 忽略 XDG_CONFIG_HOME），无覆盖机制；spawn 子进程永远绑定真实 proxy.sock；CCT_PROXY_PORT 两端生效但 socket 路径无对应机制
- Fix: proxy_socket_path() 加 CCT_PROXY_SOCKET env 覆盖（先例 CCT_CONFIG/CCT_KIMI_CONFIG），不违反接口冻结（未改既有接口）

### ADVISORY: 僵尸 vs 占端口的"端口空闲"判定机制未指定，且必须落在父进程
- Location: spec.md:Solution Summary + Decisions 5
- Evidence: 试探 bind（成功=空闲/AddrInUse=被占）必须放 ensure_proxy_running 父进程内（spawn 子进程 stderr 被丢弃，用户看不到报错）；父进程不做 remove_file（由子进程"探测→再删"统一负责，避免 TOCTOU unlink 并发启动的活 proxy）
- Fix: spec 补"端口空闲判定用试探 bind（父进程内），子进程启动时先探测再删 socket；父进程不直接删 socket 文件"

### ADVISORY: 首个 Smoke AC 的 `cct run <profile>` "发起对话"缺非交互机制说明
- Location: spec.md:Smoke 2
- Evidence: cct run 对 codex 执行无子命令（交互式 TUI），脚本无法驱动；需 extra_args 携带 exec 子命令（launch.rs:199-202 逐字追加）或直接复刻旗标跑 codex exec
- Fix: Terminology 或 AC 注明"smoke 用 extra_args 携带 exec 子命令，或按 AC 4-6 直接复刻旗标跑 codex exec"

## INTERVIEW_NEEDED
"报错含占用者 PID"与"无 pidfile/lsof 机制"取哪个（或允许只读 lsof 诊断调用）？
Context: 两者在 macOS 上不能同时满足——无 lsof/pidfile 则进程内拿不到占用者 PID。round-1 用户已拍板"不自动 kill、无 pidfile/lsof"；rev 2 AC 又要求"含占用者 PID"

## Round-1 闭合核查
- BLOCKER（占端口无法终止）→ 已闭合（报错手动）✓
- ADVISORY 1-3 → 全部闭合 ✓

## 已验证通过的关键点
1. 异步 accept + spawn_blocking 成立 ✓
2. 僵尸自愈流程闭环（每分支有已定义结局）✓
3. CCT_PROXY_BIN 注入可实现 ✓（与 socket 路径覆盖联动）
4. 脱敏只需改一处（proxy.rs:420）✓
5. shutdown 清 socket 机械改动 ✓
6. Part B 链路与 6 个 --config 旗标一致 ✓
