---
title: "zombie_recovery_restarts_proxy — Green Phase"
brief: "zombie_recovery_restarts_proxy — Green: exit 0"
doc_type: proc
created: 2026-08-01T10:59:54Z
case: "zombie_recovery_restarts_proxy"
phase: green
---
Exit code: 0
Full output: `cargo test --test proxy_contract`（工作树根目录执行，完整契约套件；rtk 压缩输出，改用 `rtk proxy` 取完整输出）

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.09s
     Running tests/proxy_contract.rs (target/debug/deps/proxy_contract-37538df7d2e1326e)

running 7 tests
test smoke_stub_receives_request ... ok
test stop_times_out_on_unresponsive_socket ... ok
test stub_forwarding_with_bearer ... ok
test concurrent_control_and_http ... ok
test log_masks_api_key ... ok
test zombie_recovery_restarts_proxy ... ok
test log_masks_api_key_upstream_error ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.22s
```

备注：vacuous Red —— 自愈路径（`ensure_proxy_running` 重启分支）已由 TC-15 Green 实现，本步无 src 改动（Red 阶段本用例即 exit 0：kill 后 socket 残留 + 探测 false + 重启 Ok + 恢复健康，断言非空泛；RestartEnvGuard 恢复 env + shutdown 清理无孤儿进程）。本次为完整 proxy_contract 契约套件全绿（7/7 通过，3.22s，无挂起）：`zombie_recovery_restarts_proxy` 与其余 6 个契约测试（smoke_stub_receives_request / stop_times_out_on_unresponsive_socket / stub_forwarding_with_bearer / concurrent_control_and_http / log_masks_api_key / log_masks_api_key_upstream_error）同批通过，确认新增回归守卫未破坏既有代理行为。本步仅运行测试 + 记录日志，未修改任何 src/ 与 tests/ 文件。
