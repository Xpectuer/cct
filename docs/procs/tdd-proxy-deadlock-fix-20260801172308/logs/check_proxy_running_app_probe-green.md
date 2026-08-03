---
title: "check_proxy_running_app_probe — Green Phase"
brief: "check_proxy_running_app_probe — Green: exit 0"
doc_type: proc
created: 2026-08-01T09:45:32Z
case: "check_proxy_running_app_probe"
phase: green
---
Exit code: 0
Full output: `cargo test proxy::tests::check_proxy_running`（工作树根目录执行，完整输出如下）

```
   Compiling ring v0.17.14
   Compiling rustls v0.23.41
   Compiling rustls-webpki v0.103.13
   Compiling tokio-rustls v0.26.4
   Compiling hyper-rustls v0.27.9
   Compiling reqwest v0.12.28
   Compiling cct v0.5.0 (/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 6.60s
     Running unittests src/lib.rs (target/debug/deps/cct-c695630c4374c597)

running 3 tests
test proxy::tests::check_proxy_running_false_when_socket_absent ... ok
test proxy::tests::check_proxy_running_true_when_daemon_responds ... ok
test proxy::tests::check_proxy_running_false_when_socket_silent ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 134 filtered out; finished in 2.01s

     Running unittests src/main.rs (target/debug/deps/cct-a415d99fd63277d3)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 19 filtered out; finished in 0.00s

     Running tests/integration.rs (target/debug/deps/integration-7dbc2c9fd2903748)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s

     Running tests/live.rs (target/debug/deps/live-080beb1530291df0)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.00s
```
