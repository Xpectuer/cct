---
title: "zombie_socket_triggers_restart — Refactor Phase"
brief: "zombie_socket_triggers_restart — Refactor: exit 0"
doc_type: proc
created: 2026-08-01T10:45:57Z
case: "zombie_socket_triggers_restart"
phase: refactor
---
Changes made: `tests/launch_proxy_contract.rs` 三处改动，测试行为与断言强度零变化（zombie 用例 5 个断言逐字保留）：

1. **夹具提取 `setup_proxy_env`**：6 行 setup 块（tempdir 路径 + write_fake_proxy + free_port + ProxyEnvGuard::set）在三个测试（spawns_fake_when_none_running / reuses_live_proxy / zombie_socket_triggers_restart）中逐字重复 3 次，按仓库 KISS「三处重复代码才考虑提取」提取为 `setup_proxy_env(dir) -> (fake, socket, ready, port, env guard)`。行为等价：调用点顺序不变，guard 仍存活至测试结束。
2. **就绪等待提取 `wait_fake_ready`**：zombie 用例 17 行 spawn+deadline 等待循环是 reuses_live_proxy 的逐字拷贝（reuses_live_proxy 前次 refactor 因单次使用按 KISS 内联；本用例落位后成两处，提取）。签名取 `&Child`、不返回子进程——若从 helper 返回 `Child`，clippy `zombie_processes` 会告警（lint 无法跨函数追踪 wait），保持 kill/wait 回收在调用方可见。竞态说明（「标记已写但 socket 未 bind」）与 5s 超时（消息含 pid）随之单点化。
3. **命名清晰化**：zombie 用例中被 SIGKILL 的 victim 句柄 `proxy_child` 更名为 `old_proxy`，与测试自身文档术语「旧 proxy 被 SIGKILL」一致（reuses_live_proxy 的 `proxy_child` 保留——它是存活的「原 fake」，语义不同）。

未改动：fake 脚本、`setup_proxy_env` 之外的 env guard 机制、zombie 用例全部 5 个断言及其消息（SIGKILL 后 socket 残留前置 / 删 READY 标记后探测失败前置 / ensure_proxy_running 返回 Ok / READY 标记重现 = 重新 spawn 硬证据 / 重启后应用层探测健康）、`#[serial]` 纪律。

具体观察（测试质量，vs AC2 重启语义：kill → socket 残留 → 重启 → 健康）：
- 5 断言覆盖完整 AC2 语义链且全部非空洞：去掉重启路径 → 「READY 标记缺失」红；不重新 spawn 只假 Ok → 「READY 标记缺失」红；探测未恢复 → 「重启后必须恢复健康」红；前置两个断言防止僵尸场景未建立时测试空转。
- marker 重现断言未削弱且证据力强：标记在调用前被显式 `remove_file`，调用后重现只能来自新 spawn 的 fake（fake 仅在启动时 touch 一次），构成「确实重新 spawn」而非「假 Ok」的硬证据。zombie 场景无法使用 reuses_live_proxy 的 PID 级 liveness 断言（旧进程按设计被杀死），marker 重现即为此处设计的重启信号，与 AC2 语义一致。
- 两处「前置」断言（socket 残留 + 探测失败）是场景成立的前提，保留必要；若环境异常（如 TempDir 竞态导致 socket 未残留）会在此红掉而非误报后续失败，诊断定位清晰。
- 就绪等待循环提取后超时消息仍带 pid，超时可直接定位进程；等待逻辑单点化避免了将来两测试间语义漂移（e.g. 只改一处的超时或竞态处理）。
test_cmd exit code: 0
output: `rtk proxy cargo test --test launch_proxy_contract`（工作树根目录执行；rtk proxy 取完整输出），另附 `cargo clippy --test launch_proxy_contract` 验证无 lint（clippy 首次运行发现 `zombie_processes` 告警，源自 helper 返回 `Child`，已按上述第 2 点重构消除）

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.13s
     Running tests/launch_proxy_contract.rs (target/debug/deps/launch_proxy_contract-11aa5be2c88374b7)

running 3 tests
test zombie_socket_triggers_restart ... ok
test reuses_live_proxy ... ok
test spawns_fake_when_none_running ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.14s

EXIT_CODE=0
```

```
    Checking cct v0.5.0 (/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.28s
```
