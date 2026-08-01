---
title: "port_occupied_bails_with_diagnosis — Green Phase"
brief: "port_occupied_bails_with_diagnosis — Green: exit 0"
doc_type: proc
created: 2026-08-01T11:00:32Z
case: "port_occupied_bails_with_diagnosis"
phase: green
---
Exit code: 0
Full output: `rtk proxy cargo test --test launch_proxy_contract`（工作树根目录执行，完整重启契约套件；rtk proxy 取完整输出）

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.09s
     Running tests/launch_proxy_contract.rs (target/debug/deps/launch_proxy_contract-11aa5be2c88374b7)

running 5 tests
test reuses_live_proxy ... ok
test probe_exhaustion_reports_error ... ok
test spawns_fake_when_none_running ... ok
test port_occupied_bails_with_diagnosis ... ok
test zombie_socket_triggers_restart ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.72s
```

备注：vacuous Red —— 端口预检已由 TC-15 Green 实现，本步无 src 改动（Red 阶段本用例即 exit 0，0.06s：测试进程 bind 动态端口并保持监听 → 应用层探测失败后 `ensure_proxy_running` 的 bind 试探立即失败 → `bail!(port_conflict_message(port))`，无 spawn）。本次为完整重启契约套件全绿（5/5 通过，2.72s，无挂起）：`port_occupied_bails_with_diagnosis` 与 `reuses_live_proxy` / `probe_exhaustion_reports_error` / `spawns_fake_when_none_running` / `zombie_socket_triggers_restart` 同批通过，确认新增回归守卫未破坏既有重启行为。本步仅运行测试 + 记录日志，未修改任何 src/ 与 tests/ 文件。
