# Verifier Report: Test Depth (Angle 6/7) — Round 2

## Score: 7/10
## Verdict: FAIL

## Findings

### BLOCKER: AC 2/3 的「发起对话 / codex 收到第一个 Response」在无 tty 子进程下不可达——裸 codex TUI 直接失败
- Location: spec.md:AC 2/3；launch.rs:242（exec_codex_proxy 无子命令 exec）
- Evidence: 本机实测 codex-cli 0.146.0：无 tty 下裸 `codex` 报 `Error: stdin is not a terminal`；`codex exec` 无 tty 正常。`cct run <profile>` exec-replace 成裸 codex → smoke 子进程调用时没有任何 HTTP 请求流过 proxy
- Fix: 钉死机制——smoke 用临时 profiles.toml（CCT_CONFIG 覆盖），extra_args 嵌入 exec 子命令（`["exec", "-o", "<out>", "--dangerously-bypass-approvals-and-sandbox", "<prompt>"]`，build_shared_codex_args 追加在旗标之后）；stub 上游补 responses-API SSE 契约；或将 AC 2 "第一个 Response" 显式归用户 live 层
- 已实证：codex exec resume --last/--all/-o 均存在于 0.146.0；临时 CODEX_HOME 下 codex exec 无 tty 正常

### ADVISORY: launch 层 fake spawn 目标如何绑定被测临时 socket 路径未定义
- Location: spec.md:AC 6
- Fix: fake 目标经 env（CCT_PROXY_SOCKET）接收临时路径，测试端同时设置

### ADVISORY: AC 4-6「复刻 6 个 --config 旗标」与 build_codex_proxy_config_args 存在 drift 风险
- Location: spec.md:Terminology「codex exec 链路」、AC 4
- Fix: 两腿均走 cct run（extra_args 嵌入 exec）；或单元测试钉住「脚本旗标模板 == 函数输出」

### ADVISORY: smoke 前置条件未声明——旧版死锁实例（PID 29182）迁移前 AC 2/3 必失败
- Location: spec.md:AC 7 vs AC 2/3
- Fix: AC 写明 smoke 运行前置条件（先迁移、不并行运行 cct/proxy），smoke 脚本预检 19191

无 INTERVIEW_NEEDED（tty 不确定性已实测消除）。
