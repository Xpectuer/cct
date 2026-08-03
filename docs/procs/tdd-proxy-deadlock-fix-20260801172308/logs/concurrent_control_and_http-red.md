---
title: "concurrent_control_and_http — Red Phase"
brief: "concurrent_control_and_http — Red: exit 101"
doc_type: proc
created: 2026-08-01T10:15:03Z
case: "concurrent_control_and_http"
phase: red
---
Exit code: 101
Full output: `cargo test --test proxy_contract`（工作树根目录执行，完整输出如下）

```
running 2 tests
test smoke_stub_receives_request ... ok
test concurrent_control_and_http ... FAILED

failures:

---- concurrent_control_and_http stdout ----

thread 'concurrent_control_and_http' (6563276) panicked at tests/proxy_contract.rs:281:19:
HTTP GET did not complete within the 3s budget (elapsed 2.001948542s): Resource temporarily unavailable (os error 35)
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/panicking.rs:80:14
   2: proxy_contract::concurrent_control_and_http::{{closure}}
             at ./tests/proxy_contract.rs:281:19
   3: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
   4: serial_test::serial_code_lock::local_serial_core
             at /Users/zhengjiaye/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/serial_test-3.4.0/src/serial_code_lock.rs:36:5
   5: proxy_contract::concurrent_control_and_http
             at ./tests/proxy_contract.rs:227:1
   6: proxy_contract::concurrent_control_and_http::{{closure}}
             at ./tests/proxy_contract.rs:228:33
   7: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
   8: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.


failures:
    concurrent_control_and_http

test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.14s


    Blocking waiting for file lock on artifact directory
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3.95s
     Running tests/proxy_contract.rs (target/debug/deps/proxy_contract-37538df7d2e1326e)
error: test failed, to rerun pass `--test proxy_contract`
```
