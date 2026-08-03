---
title: "port_occupied_reports_error_keeps_occupant — Red Phase"
brief: "port_occupied_reports_error_keeps_occupant — Red: exit 101"
doc_type: proc
created: 2026-08-01T11:06:50Z
case: "port_occupied_reports_error_keeps_occupant"
phase: red
---
Exit code: 101
Full output: `cargo test --test proxy_contract port_occupied_reports_error_keeps_occupant`（工作树根目录执行，完整输出）

```
running 1 test
test port_occupied_reports_error_keeps_occupant ... FAILED

failures:

---- port_occupied_reports_error_keeps_occupant stdout ----

thread 'port_occupied_reports_error_keeps_occupant' (6797521) panicked at tests/proxy_contract.rs:709:5:
stderr must carry the port-conflict diagnosis, got:
[cct-proxy] starting on 127.0.0.1:63580, control socket "/var/folders/8t/7x4hxj395mv4fzw_hf0jd29m0000gn/T/.tmpDJdy9o/proxy-occupied.sock"
[cct-proxy] control socket bound

thread 'main' (6797522) panicked at src/proxy.rs:232:29:
proxy bind 127.0.0.1:63580: Address already in use (os error 48)
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/panicking.rs:80:14
   2: cct::proxy::run_proxy::{{closure}}::{{closure}}
             at ./src/proxy.rs:232:29
   3: core::result::Result<T,E>::unwrap_or_else
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/result.rs:1622:23
   4: cct::proxy::run_proxy::{{closure}}
             at ./src/proxy.rs:232:10
   5: <core::pin::Pin<P> as core::future::future::Future>::poll
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/future/future.rs:133:9
   6: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}::{{closure}}
             at /Users/zhengjiaye/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.50.0/src/runtime/scheduler/current_thread/mod.rs:769:70
   7: tokio::task::coop::with_budget
             at /Users/zhengjiaye/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.50.0/src/task/coop/mod.rs:167:5
   8: tokio::task::coop::budget
             at /Users/zhengjiaye/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.50.0/src/task/coop/mod.rs:133:5
   9: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}
             at /Users/zhengjiaye/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.50.0/src/runtime/scheduler/current_thread/mod.rs:769:25
  10: tokio::runtime::scheduler::current_thread::Context::enter
             at /Users/zhengjiaye/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.50.0/src/runtime/scheduler/current_thread/mod.rs:446:19
  11: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}
             at /Users/zhengjiaye/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.50.0/src/runtime/scheduler/current_thread/mod.rs:768:44
  12: tokio::runtime::scheduler::current_thread::CoreGuard::enter::{{closure}}
             at /Users/zhengjiaye/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.50.0/src/runtime/scheduler/current_thread/mod.rs:856:68
  13: tokio::runtime::context::scoped::Scoped<T>::set
             at /Users/zhengjiaye/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.50.0/src/runtime/context/scoped.rs:40:9
  14: tokio::runtime::context::set_scheduler::{{closure}}
             at /Users/zhengjiaye/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.50.0/src/runtime/context.rs:176:38
  15: std::thread::local::LocalKey<T>::try_with
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/std/src/thread/local.rs:513:12
  16: std::thread::local::LocalKey<T>::with
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/std/src/thread/local.rs:477:20
  17: tokio::runtime::context::set_scheduler
             at /Users/zhengjiaye/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.50.0/src/runtime/context.rs:176:17
  18: tokio::runtime::scheduler::current_thread::CoreGuard::enter
             at /Users/zhengjiaye/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.50.0/src/runtime/scheduler/current_thread/mod.rs:856:27
  19: tokio::runtime::scheduler::current_thread::CoreGuard::block_on
             at /Users/zhengjiaye/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.50.0/src/runtime/scheduler/current_thread/mod.rs:756:24
  20: tokio::runtime::scheduler::current_thread::CurrentThread::block_on::{{closure}}
             at /Users/zhengjiaye/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.50.0/src/runtime/scheduler/current_thread/mod.rs:200:33
  21: tokio::runtime::context::runtime::enter_runtime
             at /Users/zhengjiaye/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.50.0/src/runtime/context/runtime.rs:65:16
  22: tokio::runtime::scheduler::current_thread::CurrentThread::block_on
             at /Users/zhengjiaye/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.50.0/src/runtime/scheduler/current_thread/mod.rs:188:9
  23: tokio::runtime::runtime::Runtime::block_on_inner
             at /Users/zhengjiaye/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.50.0/src/runtime/runtime.rs:371:52
  24: tokio::runtime::runtime::Runtime::block_on
             at /Users/zhengjiaye/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.50.0/src/runtime/runtime.rs:345:18
  25: cct::proxy::run_foreground
             at ./src/proxy.rs:206:8
  26: cct::run_proxy_start
             at ./src/main.rs:231:5
  27: cct::main
             at ./src/main.rs:92:36
  28: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.

stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/panicking.rs:80:14
   2: proxy_contract::port_occupied_reports_error_keeps_occupant::{{closure}}
             at ./tests/proxy_contract.rs:709:5
   3: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
   4: serial_test::serial_code_lock::local_serial_core
             at /Users/zhengjiaye/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/serial_test-3.4.0/src/serial_code_lock.rs:36:5
   5: proxy_contract::port_occupied_reports_error_keeps_occupant
             at ./tests/proxy_contract.rs:674:1
   6: proxy_contract::port_occupied_reports_error_keeps_occupant::{{closure}}
             at ./tests/proxy_contract.rs:675:48
   7: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
   8: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.


failures:
    port_occupied_reports_error_keeps_occupant

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.18s


    Blocking waiting for file lock on artifact directory
    Finished `test` profile [unoptimized + debuginfo] target(s) in 7.42s
     Running tests/proxy_contract.rs (target/debug/deps/proxy_contract-37538df7d2e1326e)
error: test failed, to rerun pass `--test proxy_contract`
```

Red 确认：**真实 Red**——`src/proxy.rs:232:29` 的 `TcpListener::bind(&addr).await.unwrap_or_else(|e| panic!("proxy bind {addr}: {e}"))` 在端口被占（测试进程的 occupant listener 持有 127.0.0.1:63580）时走 panic 路径：

1. **子进程 panic 证据**：子进程（thread 'main' 6797522）`panicked at src/proxy.rs:232:29: proxy bind 127.0.0.1:63580: Address already in use (os error 48)`——panic 文本仅含裸错误串，无 `port_conflict_message` 的占用诊断（"port {port} already in use" / "lsof -iTCP" 两分支任一）。
2. **断言 2 红**（tests/proxy_contract.rs:709）：stderr 缺占用诊断 → `assert!(stderr.contains("port {port} already in use") || stderr.contains("lsof -iTCP"))` 失败，测试在断言 2 处终止（断言 3「stderr 不含 panic」因断言 2 先行失败未执行——子进程 stderr 含 "panicked at"，修复前必然同样红）。
3. **退出码**：子进程 panic → exit 101（非 0，断言 1 通过）；cargo test 总退出码 101。
4. **占用者存活**（约束 #3）：测试进程自己的 occupant listener 全程存活（子进程 panic 退出未触碰占用者）——断言 4 语义成立，Green 阶段 `TcpListener::bind` 再试必须仍失败。

本用例为真实 Red（非 vacuous）：修复（plan Step 6：TCP bind 失败 → `eprintln!` 诊断 + `port_conflict_message` + `exit(1)`）落地后断言 2/3 将转绿。
