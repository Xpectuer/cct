---
title: "check_proxy_running_app_probe — Red Phase"
brief: "check_proxy_running_app_probe — Red: exit 101"
doc_type: proc
created: 2026-08-01T09:43:47Z
case: "check_proxy_running_app_probe"
phase: red
---
Exit code: 101
Full output: `cargo test check_proxy_running`（工作树根目录执行，完整输出如下）

```
running 3 tests
test proxy::tests::check_proxy_running_false_when_socket_absent ... ok
test proxy::tests::check_proxy_running_false_when_socket_silent ... FAILED
test proxy::tests::check_proxy_running_true_when_daemon_responds ... FAILED

failures:

---- proxy::tests::check_proxy_running_false_when_socket_silent stdout ----

thread 'proxy::tests::check_proxy_running_false_when_socket_silent' (6446778) panicked at src/proxy.rs:653:9:
silent socket must not be reported as running
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/panicking.rs:80:14
   2: cct::proxy::tests::check_proxy_running_false_when_socket_silent
             at ./src/proxy.rs:653:9
   3: cct::proxy::tests::check_proxy_running_false_when_socket_silent::{{closure}}
             at ./src/proxy.rs:641:54
   4: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
   5: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.

---- proxy::tests::check_proxy_running_true_when_daemon_responds stdout ----

thread '<unnamed>' (6446780) panicked at src/proxy.rs:614:13:
app-level probe must send a control command, got EOF
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/panicking.rs:80:14
   2: cct::proxy::tests::check_proxy_running_true_when_daemon_responds::{{closure}}
             at ./src/proxy.rs:614:13
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.

thread 'proxy::tests::check_proxy_running_true_when_daemon_responds' (6446779) panicked at src/proxy.rs:636:23:
responder thread panicked: Any { .. }
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/panicking.rs:80:14
   2: core::result::unwrap_failed
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/result.rs:1867:5
   3: core::result::Result<T,E>::expect
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/result.rs:1185:23
   4: cct::proxy::tests::check_proxy_running_true_when_daemon_responds
             at ./src/proxy.rs:636:23
   5: cct::proxy::tests::check_proxy_running_true_when_daemon_responds::{{closure}}
             at ./src/proxy.rs:603:55
   6: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
   7: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.


failures:
    proxy::tests::check_proxy_running_false_when_socket_silent
    proxy::tests::check_proxy_running_true_when_daemon_responds

test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 134 filtered out; finished in 0.01s


    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.13s
     Running unittests src/lib.rs (target/debug/deps/cct-c695630c4374c597)
error: test failed, to rerun pass `--lib`
```
