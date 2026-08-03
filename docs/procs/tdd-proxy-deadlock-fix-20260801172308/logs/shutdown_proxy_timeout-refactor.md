---
title: "shutdown_proxy_timeout — Refactor Phase"
brief: "shutdown_proxy_timeout — Refactor: exit 0"
doc_type: proc
created: 2026-08-01T18:09:11+0800
case: "shutdown_proxy_timeout"
phase: refactor
---
Changes made: 提取共享 helper `status_to_result(resp: ControlResponse) -> io::Result<()>`（src/proxy.rs）——`switch_profile` 与 `shutdown_proxy` 各自重复了同一段 8 行 `if resp.status == "ok" { Ok(()) } else { Err(io::Error::other(resp.message.unwrap_or_else(|| "unknown error".into()))) }` 检查（正是任务点名的 send_control_timeout + status 检查重复模式）。两个调用点改为 `let resp = send_control_timeout(...)?; status_to_result(resp)`。行为零变化：同一 status 比较、同一错误构造、同一超时窗口（switch 走 PROBE_TIMEOUT、shutdown 走 STOP_TIMEOUT）、错误传播方式不变。两处重复、helper 单行语义自明，符合"只在简单明确时做"的标准。其余改动不做：

- 未给 `stop_proxy`（src/main.rs）动刀：`if !socket_path.exists()` 快速返回 + `shutdown_proxy` 传播错误的两分支已是最小形态，注释说明"死锁进程持端口不得误报 not running"这一非显而易见的原因，符合 KISS 注释准则，无重复、无死代码。
- 未提取两个 "silent listener" 测试的公共模式（shutdown_proxy_errs_on_unresponsive_socket 与 stop_proxy_errs_on_unresponsive_socket）：跨文件（proxy.rs / main.rs 各自测试模块）、夹具机制不同（fresh_test_socket vs CCT_PROXY_SOCKET 环境变量），仅 2 处，低于 KISS"三处重复"阈值。
- 未给 `ControlCommand` 加 bare-command 构造器：`check_proxy_running` / `switch_profile` / `shutdown_proxy` 各自构造命令（2-3 处），但字段形状不同（switch 带 3 个 Some），code-spec 原样给出 literal，提取收益边际。

具体观察：
- 死代码：无。`send_control`（pub，被 switch_profile 使用）与 `send_control_timeout`（内部，被 check_proxy_running / shutdown_proxy / send_control 使用）均有调用方。`PROBE_RETRIES` 仍未被使用——plan 有意为之（后续步骤消费），未处理。
- 命名：`send_control` 与 `send_control_timeout` 一公共一内部、以 timeout 参数区分，清晰。`status_to_result` 命名直接对应其行为。
- 复杂条件：`send_control_timeout` 为线性流程（connect → 设读写超时 → 写 payload → shutdown(Write) → read_line → 反序列化），无嵌套分支；超时语义（STOP_TIMEOUT 2s）与错误传播（`?` 上抛）在提取后原样保留。
- 测试健壮性：`shutdown_proxy_errs_on_unresponsive_socket` 断言 `elapsed() < 3s`（超时 2s，留 1.5× 余量）；`stop_proxy_errs_on_unresponsive_socket` 断言 `< 5s`（2s 超时 + 测试自身 setup，留更大余量）。两处均未改动。
- 测试重复：proxy.rs 与 main.rs 各有一个"hold 连接不回包"测试，行为语义一致但位于不同模块、不同夹具路径，已在上面说明未合并理由。
test_cmd exit code: 0
output: 按任务要求以 `cargo test shutdown_proxy && cargo test stop_proxy` 执行（cargo 单次只接受一个 TESTNAME filter，故分两条命令运行，语义等价），工作树根目录执行，完整输出如下

```
$ cargo test shutdown_proxy
   Compiling ring v0.17.14
   Compiling rustls v0.23.41
   Compiling rustls-webpki v0.103.13
   Compiling tokio-rustls v0.26.4
   Compiling hyper-rustls v0.27.9
   Compiling reqwest v0.12.28
   Compiling cct v0.5.0 (/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 6.03s
     Running unittests src/lib.rs (target/debug/deps/cct-c695630c4374c597)

running 2 tests
test proxy::tests::shutdown_proxy_ok_when_daemon_responds ... ok
test proxy::tests::shutdown_proxy_errs_on_unresponsive_socket ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 145 filtered out; finished in 2.01s

     Running unittests src/main.rs (target/debug/deps/cct-a415d99fd63277d3)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 21 filtered out; finished in 0.00s

     Running tests/integration.rs (target/debug/deps/integration-7dbc2c9fd2903748)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s

     Running tests/live.rs (target/debug/deps/live-080beb1530291df0)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.00s

exit code: 0

$ cargo test stop_proxy
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.09s
     Running unittests src/lib.rs (target/debug/deps/cct-c695630c4374c597)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 147 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/debug/deps/cct-a415d99fd63277d3)

running 2 tests
test tests::stop_proxy_ok_when_socket_absent ... ok
test tests::stop_proxy_errs_on_unresponsive_socket ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 19 filtered out; finished in 2.01s

     Running tests/integration.rs (target/debug/deps/integration-7dbc2c9fd2903748)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s

     Running tests/live.rs (target/debug/deps/live-080beb1530291df0)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.00s

exit code: 0
```
