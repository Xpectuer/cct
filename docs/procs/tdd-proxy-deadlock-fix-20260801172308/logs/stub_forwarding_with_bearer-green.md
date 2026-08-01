---
title: "stub_forwarding_with_bearer — Green Phase"
brief: "stub_forwarding_with_bearer — Green: exit 0"
doc_type: proc
created: 2026-08-01T10:32:27Z
case: "stub_forwarding_with_bearer"
phase: green
---
Exit code: 0
Full output: `cargo test --test proxy_contract`（工作树根目录执行，`rtk proxy` 原始输出，完整如下）

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.09s
     Running tests/proxy_contract.rs (target/debug/deps/proxy_contract-37538df7d2e1326e)

running 3 tests
test smoke_stub_receives_request ... ok
test stub_forwarding_with_bearer ... ok
test concurrent_control_and_http ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
```

备注：**vacuous Red** —— 既有实现已满足 AC4（switch 分支 + Bearer 注入 + SSE 流式转发均在 HEAD 基线存在，Red 阶段已确认无法制造真实失败），本步为 tests-only：**无任何 src/ 改动**。Green 阶段运行完整契约套件（smoke + concurrent_control_and_http + stub_forwarding_with_bearer 共 3 个用例）exit 0，回归守卫生效：无 TC-6 或本步之外的改动破坏转发路径。
