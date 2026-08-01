---
title: "log_masks_api_key — Green Phase"
brief: "log_masks_api_key — Green: exit 0"
doc_type: proc
created: 2026-08-01T10:41:20Z
case: "log_masks_api_key"
phase: green
---
Exit code: 0
Full output: `cargo test --test proxy_contract`（工作树根目录执行，`rtk proxy cargo test` 原始输出，完整如下）

```
   Compiling cct v0.5.0 (/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.39s
     Running tests/proxy_contract.rs (target/debug/deps/proxy_contract-37538df7d2e1326e)

running 4 tests
test smoke_stub_receives_request ... ok
test concurrent_control_and_http ... ok
test stub_forwarding_with_bearer ... ok
test log_masks_api_key ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
```

（备注：修复 outbound 日志泄漏——mask_request_path 应用于 -> upstream 日志行。最小改动：src/proxy.rs handle_request 内 `log_proxy!("-> upstream {method} {upstream_url} (model={})", active.model)` 改为 `log_proxy!("-> upstream {method} {} (model={})", mask_request_path(&upstream_url), active.model)`——只掩码 sk- 值，base_url/host 不受影响；inbound 行（src/proxy.rs:274）原已脱敏，本轮未动。）
