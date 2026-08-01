---
title: "double_start_race_one_wins — Red Phase"
brief: "double_start_race_one_wins — Red: exit 101"
doc_type: proc
created: 2026-08-01T11:12:43Z
case: "double_start_race_one_wins"
phase: red
---
Exit code: 101
Full output: `cargo test --test proxy_contract double_start_race_one_wins`（工作树根目录执行，完整输出；rtk tee 日志 ~/Library/Application Support/rtk/tee/1785582740_cargo_test.log）

```
running 1 test
test double_start_race_one_wins ... FAILED

failures:

---- double_start_race_one_wins stdout ----

thread 'double_start_race_one_wins' (6816033) panicked at tests/proxy_contract.rs:795:5:
double-start race must not panic, combined stderr:
[A]
[cct-proxy] starting on 127.0.0.1:63994, control socket "/var/folders/8t/7x4hxj395mv4fzw_hf0jd29m0000gn/T/.tmpKGMzPQ/race.sock"
[cct-proxy] control socket bound
[cct-proxy] ctl << {"cmd":"status","base_url":null,"api_key":null,"model":null}
[cct-proxy] ctl >> status (base_url=, model=)

[B]
[cct-proxy] starting on 127.0.0.1:63994, control socket "/var/folders/8t/7x4hxj395mv4fzw_hf0jd29m0000gn/T/.tmpKGMzPQ/race.sock"

thread 'main' (6816035) panicked at src/proxy.rs:219:61:
bind proxy control socket: Os { code: 17, kind: AlreadyExists, message: "File exists" }
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/panicking.rs:80:14
   2: core::result::unwrap_failed
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/result.rs:1867:5
   3: core::result::Result<T,E>::expect
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/result.rs:1185:23
   4: cct::proxy::run_proxy::{{closure}}
             at ./src/proxy.rs:219:61
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
   2: proxy_contract::double_start_race_one_wins::{{closure}}
             at ./tests/proxy_contract.rs:795:5
   3: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
   4: serial_test::serial_code_lock::local_serial_core
             at /Users/zhengjiaye/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/serial_test-3.4.0/src/serial_code_lock.rs:36:5
   5: proxy_contract::double_start_race_one_wins
             at ./tests/proxy_contract.rs:736:1
   6: proxy_contract::double_start_race_one_wins::{{closure}}
             at ./tests/proxy_contract.rs:737:32
   7: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
   8: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.


failures:
    double_start_race_one_wins

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 8 filtered out; finished in 2.08s


    Blocking waiting for file lock on artifact directory
   Compiling cct v0.5.0 (/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 4.01s
     Running tests/proxy_contract.rs (target/debug/deps/proxy_contract-37538df7d2e1326e)
error: test failed, to rerun pass `--test proxy_contract`
```

Red 确认：**真实 Red**——断言 3（tests/proxy_contract.rs:795，stderr 合计无 "panic"）失败，子进程 [B]（thread 'main' 6816035）在 `src/proxy.rs:219:61` 的 `TokioUnixListener::bind(socket_path).expect("bind proxy control socket")` 处 panic（`Os { code: 17, kind: AlreadyExists, message: "File exists" }`，EADDRINUSE）：

1. **竞态时序（本轮实际发生）**：两进程的 `remove_file(socket_path)` 都在对方 bind 之前执行（各自 no-op），随后 [A] 先 bind 成功（stderr "control socket bound"），[B] 再 bind 时 socket 文件已存在 → EADDRINUSE → `.expect()` panic → exit 101。这正好命中 Red 分析的第二个分支：控制 socket 段无"先探测再删"且 bind 失败走 panic。
2. **断言 1 通过**（非 vacuous 证据）：[A] stderr 的 `ctl << {"cmd":"status",...}` / `ctl >> status` 行证明 `check_proxy_running` 应用层探测真实成功——恰一个存活语义成立，但幸存者的健康不足以掩盖 panic 路径。
3. **断言 2 通过**：恰一个退出（[B]，非 0 → 101），一个存活（[A]）；2s 预算内有界收敛（try_wait 轮询，无死锁）。
4. **断言 3 红**：两 stderr 合计含 "panicked at src/proxy.rs:219:61: bind proxy control socket" → `!combined.contains("panic")` 失败，cargo test 总退出码 101。
5. **Red 分析的另一分支（幸存者 socket 被删）本轮未出现**：若 [B] 的 remove_file 在 [A] bind 之后执行，[A] 的 socket 会被 unlink 导致 check_proxy_running 失败（断言 1 红）——两种时序下本测试都是 Red，只是失败断言不同。Green 修复（先探测、EADDRINUSE 重新探测收敛、不删活 socket、失败 exit(1) + 诊断而非 panic）落地后断言 1/2/3 将同时转绿。
