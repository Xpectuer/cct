---
title: "shutdown_proxy_timeout — Red Phase"
brief: "shutdown_proxy_timeout — Red: exit 101"
doc_type: proc
created: 2026-08-01T10:05:20Z
case: "shutdown_proxy_timeout"
phase: red
---
Exit code: 101
Full output: `cargo test shutdown_proxy`（工作树根目录执行，完整输出如下）

```
running 2 tests
test proxy::tests::shutdown_proxy_ok_when_daemon_responds ... ok
test proxy::tests::shutdown_proxy_errs_on_unresponsive_socket ... FAILED

failures:

---- proxy::tests::shutdown_proxy_errs_on_unresponsive_socket stdout ----

thread 'proxy::tests::shutdown_proxy_errs_on_unresponsive_socket' (6527453) panicked at src/proxy.rs:769:9:
shutdown on silent socket must return Err, got: Ok(())
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/panicking.rs:80:14
   2: cct::proxy::tests::shutdown_proxy_errs_on_unresponsive_socket
             at ./src/proxy.rs:769:9
   3: cct::proxy::tests::shutdown_proxy_errs_on_unresponsive_socket::{{closure}}
             at ./src/proxy.rs:759:52
   4: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
   5: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.


failures:
    proxy::tests::shutdown_proxy_errs_on_unresponsive_socket

test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 145 filtered out; finished in 0.54s


    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.09s
     Running unittests src/lib.rs (target/debug/deps/cct-c695630c4374c597)
error: test failed, to rerun pass `--lib`
```

配套 main.rs（stop_proxy 用例名不含 `shutdown_proxy` 子串，单独命令验证；`cargo test stop_proxy`，完整输出如下）：

```
running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 147 filtered out; finished in 0.00s


running 2 tests
test tests::stop_proxy_ok_when_socket_absent ... ok
test tests::stop_proxy_errs_on_unresponsive_socket ... FAILED

failures:

---- tests::stop_proxy_errs_on_unresponsive_socket stdout ----
Proxy is not running.

thread 'tests::stop_proxy_errs_on_unresponsive_socket' (6527805) panicked at src/main.rs:877:9:
stop on unresponsive socket must propagate error, got: Ok(())
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/panicking.rs:80:14
   2: cct::tests::stop_proxy_errs_on_unresponsive_socket::{{closure}}
             at ./src/main.rs:877:9
   3: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
   4: serial_test::serial_code_lock::local_serial_core
             at /Users/zhengjiaye/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/serial_test-3.4.0/src/serial_code_lock.rs:36:5
   5: cct::tests::stop_proxy_errs_on_unresponsive_socket
             at ./src/main.rs:862:5
   6: cct::tests::stop_proxy_errs_on_unresponsive_socket::{{closure}}
             at ./src/main.rs:863:48
   7: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
   8: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.


failures:
    tests::stop_proxy_errs_on_unresponsive_socket

test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 19 filtered out; finished in 0.53s


    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.08s
     Running unittests src/lib.rs (target/debug/deps/cct-c695630c4374c597)
     Running unittests src/main.rs (target/debug/deps/cct-a415d99fd63277d3)
error: test failed, to rerun pass `--bin cct`
```

Red 确认：核心断言 `shutdown_proxy` 无响应 socket 必须返回 Err，当前实现 `let _ = send_control(...)` 吞错返回 `Ok(())` → 断言失败（exit 101）。main.rs 检查：`src/main.rs` 有 `#[cfg(test)]` 模块但无 stop_proxy 测试先例 → 单元层可测，已补两个测试；`stop_proxy_ok_when_socket_absent`（无 socket → 快速 Ok）旧/新实现均通过（guard 测试），`stop_proxy_errs_on_unresponsive_socket`（socket 存在但无响应 → Err）旧实现经 check_proxy_running 探测失败后误报 "Proxy is not running." 返回 Ok → 断言失败（exit 101）。两测试均 0.53-0.54s 内返回（无挂起）。
