---
title: "proxy_socket_path_override — Red Phase"
brief: "proxy_socket_path_override — Red: exit 101"
doc_type: proc
created: 2026-08-01T09:38:18Z
case: "proxy_socket_path_override"
phase: red
---
Exit code: 101
Full output: `cargo test proxy_socket_path`（工作树根目录执行，完整输出如下）

```
running 2 tests
test proxy::tests::proxy_socket_path_ends_with_proxy_sock ... ok
test proxy::tests::proxy_socket_path_override ... FAILED

failures:

---- proxy::tests::proxy_socket_path_override stdout ----

thread 'proxy::tests::proxy_socket_path_override' (6425870) panicked at src/proxy.rs:578:9:
assertion `left == right` failed
  left: "/Users/zhengjiaye/Library/Application Support/cc-tui/proxy.sock"
 right: "/var/folders/8t/7x4hxj395mv4fzw_hf0jd29m0000gn/T/cct-proxy-test.proxy.sock"
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/panicking.rs:80:14
   2: core::panicking::assert_failed_inner
   3: core::panicking::assert_failed
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/panicking.rs:394:5
   4: cct::proxy::tests::proxy_socket_path_override
             at ./src/proxy.rs:578:9
   5: cct::proxy::tests::proxy_socket_path_override::{{closure}}
             at ./src/proxy.rs:575:36
   6: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
   7: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.


failures:
    proxy::tests::proxy_socket_path_override

test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 132 filtered out; finished in 0.01s


    Blocking waiting for file lock on artifact directory
    Finished `test` profile [unoptimized + debuginfo] target(s) in 19.70s
     Running unittests src/lib.rs (target/debug/deps/cct-c695630c4374c597)
error: test failed, to rerun pass `--lib`
```
