# Verifier Report: Test Depth (Angle 6/7)

## Score: 7/10
## Verdict: FAIL

## Findings

### BLOCKER: AC2「死 proxy 自动重启」无可执行的测试路径，launch↔proxy 跨模块重启契约未定义
- Location: spec.md:Acceptance Criteria（AC 2 Smoke + AC 6）
- Evidence: ensure_proxy_running（launch.rs:132）用 current_exe() 派生 `proxy start`——cargo test 下是测试二进制；既有 stub 模式（PATH fake binary、CCT_CLAUDE_BIN、exec_profile example）全部失效，无注入点
- Fix: 三层拆分——(a) proxy 层契约测试：直调 run_proxy + 临时路径，造"只 bind 不响应 status"假死 listener + stale 文件，断言探测判死；(b) launch 层契约测试：新增可注入 spawn 目标（CCT_PROXY_BIN env 或 exec_profile example 带 proxy start 分派），断言死 socket → 清理 → 重新 spawn → 探测成功；(c) "codex 收到第一个 Response"归 live 层用户配合

### BLOCKER: AC3「codex 经 cct 启动」无 agent 可执行的自动化路径——cct 非交互启动面不存在
- Location: spec.md:AC 3；session-log:[Smoke 自动化方案]
- Evidence: cct CLI 只有 add / proxy start|stop / env <profile> -- <cmd>；run_env 只注入 env；TUI 需 Enter；exec_codex_proxy 是 exec-replace 测试内不可调用。session-log 的自动化方案是 codex exec 直连，绕开 cct 启动链路 → ensure_proxy_running → switch_profile → env 注入 → --config flags 整条链路 L2 零覆盖
- Fix: 二选一——(a) 新增非交互 `cct launch <profile>` 子命令（产品面变更，需用户确认）；(b) 扩展 exec_profile example 支持 codex proxy 模式（spawn 非 exec）供 smoke 脚本调用。两者都拒绝则把 AC3 定义为"codex + cct 生成的 flags（build_codex_proxy_config_args 契约测试背书）→ 真实 proxy → stub 上游"，"经 cct 启动"降级为 Open Question 3 用户 live 验证

### ADVISORY: 契约测试硬编码 19191 / proxy.sock 会与用户真实 proxy 冲突
- Location: spec.md:AC 1
- Fix: 契约测试全部用临时目录 socket 路径 + 随机/动态端口（run_proxy 直调 + CCT_PROXY_PORT 覆盖），与用户实例隔离

### ADVISORY: L2 只测 provider 维度，cwd/`--all` 维度无对应 AC
- Location: spec.md:Terminology「resume 仓库过滤」 vs AC 4/5
- Fix: 新增/并入 Smoke：同 provider 跨仓库会话，resume --last（当前 cwd）不可见、--all 可见

### ADVISORY: 关键 L2 断言目标未钉死（--last 输出格式待确认）
- Location: spec.md:Open Question 2；AC 4
- Fix: 测试编写前做只读探测，按 assert-contracts 规则钉在语义契约（会话 id/消息存在性）

### ADVISORY: AC8「不写 Codex 配置文件」是约束而非测试
- Location: spec.md:AC 8
- Fix: 追加 config 文件快照对比断言（跑完 cct 启动路径后 ~/.codex 无写入）——9a09b39 修复过的回归类 bug

## INTERVIEW_NEEDED
是否允许为 L2 冒烟新增非交互启动路径（`cct launch <profile>` CLI 子命令 vs 仅测试用 example binary），以及 L2 可见性测试是否可向真实 `~/.codex` 写入测试会话（或用临时 CODEX_HOME 隔离）？
Context: 前者是产品 CLI 面变更（tui-cli-sync 规则管辖），后者涉及用户真实状态数据安全边界
