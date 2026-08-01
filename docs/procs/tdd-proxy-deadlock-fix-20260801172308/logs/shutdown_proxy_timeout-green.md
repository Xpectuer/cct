---
title: "shutdown_proxy_timeout — Green Phase"
brief: "shutdown_proxy_timeout — Green: exit 0"
doc_type: proc
created: 2026-08-01T10:07:25Z
case: "shutdown_proxy_timeout"
phase: green
---
Exit code: 0
Full output: `cargo test shutdown_proxy`（工作树根目录执行，完整输出如下）

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.09s
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
```

配套 main.rs（stop_proxy 用例名不含 `shutdown_proxy` 子串，单独命令验证；`cargo test stop_proxy`，完整输出如下）：

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.08s
     Running unittests src/lib.rs (target/debug/deps/cct-c695630c4374c597)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 147 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/debug/deps/cct-a415d99fd63277d3)

running 2 tests
test tests::stop_proxy_errs_on_unresponsive_socket ... ok
test tests::stop_proxy_ok_when_socket_absent ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 19 filtered out; finished in 2.00s

     Running tests/integration.rs (target/debug/deps/integration-7dbc2c9fd2903748)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s

     Running tests/live.rs (target/debug/deps/live-080beb1530291df0)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.00s
```

Green 确认：两条命令均 exit 0。`shutdown_proxy` 改为 `send_control_timeout(.., STOP_TIMEOUT)` + 校验 `status == "ok"`，无响应 socket 在 2s 读超时后返回 Err（不再吞错）；`stop_proxy` 改为 `socket_path.exists()` 区分"无 socket → 快速 exit 0"与"socket 存在但无响应 → shutdown 错误传播"。4 个测试（proxy 2 + main 2）全绿，`shutdown_proxy_errs_on_unresponsive_socket` 2.01s / `stop_proxy_errs_on_unresponsive_socket` 2.00s 内返回（均为 STOP_TIMEOUT 量级，无挂起）。
