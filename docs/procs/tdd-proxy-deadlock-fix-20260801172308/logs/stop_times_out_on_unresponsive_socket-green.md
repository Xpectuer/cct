---
title: "stop_times_out_on_unresponsive_socket — Green Phase"
brief: "stop_times_out_on_unresponsive_socket — Green: exit 0"
doc_type: proc
created: 2026-08-01T10:51:38Z
case: "stop_times_out_on_unresponsive_socket"
phase: green
---
Exit code: 0
Full output: `cargo test --test proxy_contract`（工作树根目录执行，完整输出如下）

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.09s
     Running tests/proxy_contract.rs (target/debug/deps/proxy_contract-37538df7d2e1326e)

running 6 tests
test smoke_stub_receives_request ... ok
test concurrent_control_and_http ... ok
test stop_times_out_on_unresponsive_socket ... ok
test log_masks_api_key ... ok
test log_masks_api_key_upstream_error ... ok
test stub_forwarding_with_bearer ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.56s
```

（备注：vacuous Red —— stop 超时语义已由 TC-5 Green 实现，本步无 src 改动）

Green 确认：完整契约套件 exit 0，6 个测试全绿（计划标注 5 个，实际 proxy_contract.rs 含 `log_masks_api_key_upstream_error` 共 6 个 `#[test]`）。`stop_times_out_on_unresponsive_socket` 通过且套件总耗时 2.56s —— 与 Red 阶段实测（单测 2.16s ≈ 2s STOP_TIMEOUT 等待）一致，证明无响应 socket 场景真实等待 2s 读超时后报错退出，无挂起死锁。①（无响应 socket → 非 0 退出 + ≤2.5s + stderr 含错误 + 无误报 "not running"）与 ②（无 socket 文件 → 快速 exit 0 + "Proxy is not running."）均满足。
