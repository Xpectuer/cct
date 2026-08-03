---
title: "shutdown_removes_socket_file — Red Phase"
brief: "shutdown_removes_socket_file — Red: exit 101"
doc_type: proc
created: 2026-08-01T11:54:54Z
case: "shutdown_removes_socket_file"
phase: red
---
Exit code: 101
Full output: `cargo test --test proxy_contract shutdown_removes_socket_file`（工作树根目录执行，完整输出如下）

```
running 1 test
test shutdown_removes_socket_file ... FAILED

failures:

---- shutdown_removes_socket_file stdout ----

thread 'shutdown_removes_socket_file' (6984499) panicked at tests/proxy_contract.rs:835:5:
shutdown must remove the socket file — dead socket file left behind at "/var/folders/8t/7x4hxj395mv4fzw_hf0jd29m0000gn/T/.tmpHccvKP/proxy.sock"
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/panicking.rs:80:14
   2: proxy_contract::shutdown_removes_socket_file::{{closure}}
             at ./tests/proxy_contract.rs:835:5
   3: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
   4: serial_test::serial_code_lock::local_serial_core
             at /Users/zhengjiaye/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/serial_test-3.4.0/src/serial_code_lock.rs:36:5
   5: proxy_contract::shutdown_removes_socket_file
             at ./tests/proxy_contract.rs:810:1
   6: proxy_contract::shutdown_removes_socket_file::{{closure}}
             at ./tests/proxy_contract.rs:811:34
   7: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
   8: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.


failures:
    shutdown_removes_socket_file

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 9 filtered out; finished in 0.25s


    Blocking waiting for file lock on artifact directory
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.42s
     Running tests/proxy_contract.rs (target/debug/deps/proxy_contract-37538df7d2e1326e)
error: test failed, to rerun pass `--test proxy_contract`
```
