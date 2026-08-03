---
title: "proxy_socket_path_override — Green Phase"
brief: "proxy_socket_path_override — Green: exit 0"
doc_type: proc
created: 2026-08-01T09:39:35Z
case: "proxy_socket_path_override"
phase: green
---
Exit code: 0
Full output: `cargo test proxy_socket_path`（工作树根目录执行，完整输出如下）

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.10s
     Running unittests src/lib.rs (target/debug/deps/cct-c695630c4374c597)

running 2 tests
test proxy::tests::proxy_socket_path_ends_with_proxy_sock ... ok
test proxy::tests::proxy_socket_path_override ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 132 filtered out; finished in 0.00s

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
