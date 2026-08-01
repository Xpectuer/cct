---
title: "spawns_fake_when_none_running — Red Phase"
brief: "spawns_fake_when_none_running — Red: exit 101"
doc_type: proc
created: 2026-08-01T10:12:49Z
case: "spawns_fake_when_none_running"
phase: red
---
Exit code: 101
Full output: `cargo test --test launch_proxy_contract`（工作树根目录执行；rtk 压缩后从 `~/Library/Application Support/rtk/tee/` 恢复完整日志，如下）

```
running 1 test
test spawns_fake_when_none_running ... FAILED

failures:

---- spawns_fake_when_none_running stdout ----

thread 'spawns_fake_when_none_running' (6557046) panicked at tests/launch_proxy_contract.rs:107:5:
ensure_proxy_running 必须经 CCT_PROXY_BIN 拉起 fake 并返回 Ok: Err(proxy did not start within 5 seconds

Stack backtrace:
   0: std::backtrace_rs::backtrace::libunwind::trace
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/std/src/../../backtrace/src/backtrace/libunwind.rs:117:9
   1: std::backtrace_rs::backtrace::trace_unsynchronized
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/std/src/../../backtrace/src/backtrace/mod.rs:66:14
   2: std::backtrace::Backtrace::create
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/std/src/backtrace.rs:331:13
   3: anyhow::error::<impl anyhow::Error>::msg
             at /Users/zhengjiaye/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/anyhow-1.0.102/src/backtrace.rs:10:14
   4: anyhow::__private::format_err
             at /Users/zhengjiaye/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/anyhow-1.0.102/src/lib.rs:687:13
   5: cct::launch::ensure_proxy_running
             at ./src/launch.rs:162:5
   6: launch_proxy_contract::spawns_fake_when_none_running::{{closure}}
             at ./tests/launch_proxy_contract.rs:100:18
   7: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
   8: serial_test::serial_code_lock::local_serial_core
             at /Users/zhengjiaye/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/serial_test-3.4.0/src/serial_code_lock.rs:36:5
   9: launch_proxy_contract::spawns_fake_when_none_running
             at ./tests/launch_proxy_contract.rs:83:1
  10: launch_proxy_contract::spawns_fake_when_none_running::{{closure}}
             at ./tests/launch_proxy_contract.rs:84:35
  11: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
  12: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
  13: test::__rust_begin_short_backtrace
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/test/src/lib.rs:663:18
  14: test::run_test_in_process::{{closure}}
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/test/src/lib.rs:686:74
  15: <core::panic::unwind_safe::AssertUnwindSafe<F> as core::ops::function::FnOnce<()>>::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/panic/unwind_safe.rs:274:9
  16: std::panicking::catch_unwind::do_call
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/std/src/panicking.rs:581:40
  17: std::panicking::catch_unwind
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/std/src/panicking.rs:544:19
  18: std::panic::catch_unwind
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/std/src/panic.rs:359:14
  19: test::run_test_in_process
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/test/src/lib.rs:686:27
  20: test::run_test::{{closure}}
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/test/src/lib.rs:607:43
  21: test::run_test::{{closure}}
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/test/src/lib.rs:637:41
  22: std::sys::backtrace::__rust_begin_short_backtrace
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/std/src/sys/backtrace.rs:166:18
  23: std::thread::lifecycle::spawn_unchecked::{{closure}}::{{closure}}
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/std/src/thread/lifecycle.rs:91:13
  24: <core::panic::unwind_safe::AssertUnwindSafe<F> as core::ops::function::FnOnce<()>>::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/panic/unwind_safe.rs:274:9
  25: std::panicking::catch_unwind::do_call
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/std/src/panicking.rs:581:40
  26: std::panicking::catch_unwind
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/std/src/panicking.rs:544:19
  27: std::panic::catch_unwind
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/std/src/panic.rs:359:14
  28: std::thread::lifecycle::spawn_unchecked::{{closure}}
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/std/src/thread/lifecycle.rs:89:26
  29: core::ops::function::FnOnce::call_once{{vtable.shim}}
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
  30: <alloc::boxed::Box<F,A> as core::ops::function::FnOnce<Args>>::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/alloc/src/boxed.rs:2206:9
  31: std::sys::thread::unix::Thread::new::thread_start
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/std/src/sys/thread/unix.rs:127:17
  32: __pthread_cond_wait)
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/panicking.rs:80:14
   2: launch_proxy_contract::spawns_fake_when_none_running::{{closure}}
             at ./tests/launch_proxy_contract.rs:107:5
   3: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
   4: serial_test::serial_code_lock::local_serial_core
             at /Users/zhengjiaye/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/serial_test-3.4.0/src/serial_code_lock.rs:36:5
   5: launch_proxy_contract::spawns_fake_when_none_running
             at ./tests/launch_proxy_contract.rs:83:1
   6: launch_proxy_contract::spawns_fake_when_none_running::{{closure}}
             at ./tests/launch_proxy_contract.rs:84:35
   7: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
   8: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.


failures:
    spawns_fake_when_none_running

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.08s


    Blocking waiting for file lock on artifact directory
    Finished `test` profile [unoptimized + debuginfo] target(s) in 6.86s
     Running tests/launch_proxy_contract.rs (target/debug/deps/launch_proxy_contract-11aa5be2c88374b7)
error: test failed, to rerun pass `--test launch_proxy_contract`
```

Red 确认：核心断言 `ensure_proxy_running(port, socket)` 必须返回 Ok——当前实现（src/launch.rs:132-163）spawn 目标是 `current_exe()`（测试二进制）而非 `CCT_PROXY_BIN` 注入的 fake，fake 从未被拉起（READY 标记不存在的第二断言同样会失败），就绪探测 5s 耗尽 → `Err(proxy did not start within 5 seconds)`（src/launch.rs:162 bail）→ 断言失败（exit 101）。测试 5.08s 内返回，无挂起。fake 脚本为 bash + python3 Unix socket 应答循环，真实可应答 `{"status":"ok"}` 应用层探测协议（约束 #8/#10：全部走 CCT_PROXY_SOCKET / CCT_PROXY_PORT / CCT_PROXY_BIN env，与用户实例隔离）。
