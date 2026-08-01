---
title: "stop_times_out_on_unresponsive_socket — Red Phase"
brief: "stop_times_out_on_unresponsive_socket — Red: exit 0"
doc_type: proc
created: 2026-08-01T10:49:14Z
case: "stop_times_out_on_unresponsive_socket"
phase: red
---
Exit code: 0
Full output: `cargo test --test proxy_contract stop_times_out_on_unresponsive_socket`（工作树根目录执行，完整输出如下）

```
   Compiling ring v0.17.14
   Compiling rustls v0.23.41
   Compiling rustls-webpki v0.103.13
   Compiling tokio-rustls v0.26.4
   Compiling hyper-rustls v0.27.9
   Compiling reqwest v0.12.28
   Compiling cct v0.5.0 (/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 6.01s
     Running tests/proxy_contract.rs (target/debug/deps/proxy_contract-37538df7d2e1326e)

running 1 test
test stop_times_out_on_unresponsive_socket ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 2.16s
```

**Result: vacuous Red（预期内）** — TC-5 Green 已实现 stop_proxy 区分（无 socket → 快速 Ok "Proxy is not running."）与 shutdown_proxy STOP_TIMEOUT=2s 错误传播，故 ① ② 均通过。断言覆盖超时语义的证据：
- 测试总耗时 2.16s ≈ ① 的 2s 读超时 + 进程退出 + ② 快速路径 —— 时间线证明子进程真实等待了 ~2s 超时，而非瞬时误报（若 stop 误报 "Proxy is not running." 瞬时退出，`!status.success()` / `!stderr.contains("Proxy is not running.")` 会失败）。
- ① 断言：try_wait 有界轮询（4s 挂起检测，挂死 → 断言失败而非无限阻塞）+ 非 0 退出码 + elapsed ≤2.5s + stderr 含 "Error"（anyhow Termination 输出）+ 不误报 not running + stdout 空（错误路径不打印成功文案）。
- ② 断言：exit 0 + elapsed <1s + stdout 含 "Proxy is not running."。
- 挂起（死锁）场景若复现，① 会在 4s 轮询截止时失败 —— Red 语义仍被断言覆盖。
