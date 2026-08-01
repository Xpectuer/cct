---
title: "probe_exhaustion_reports_error — Green Phase"
brief: "probe_exhaustion_reports_error — Green: exit 0"
doc_type: proc
created: 2026-08-01T10:51:02Z
case: "probe_exhaustion_reports_error"
phase: green
---
Exit code: 0
Full output: `rtk proxy cargo test --test launch_proxy_contract`（工作树根目录执行，完整重启契约套件；rtk proxy 取完整输出）

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.10s
     Running tests/launch_proxy_contract.rs (target/debug/deps/launch_proxy_contract-11aa5be2c88374b7)

running 4 tests
test reuses_live_proxy ... ok
test zombie_socket_triggers_restart ... ok
test probe_exhaustion_reports_error ... ok
test spawns_fake_when_none_running ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.67s
```

备注：vacuous Red —— 就绪耗尽语义已由 TC-15 Green 实现，本步无 src 改动（Red 阶段本用例即 exit 0，耗尽路径 `ensure_proxy_running` 的"探测耗尽 → Err + 'did not become healthy'"契约由 src 既有实现交付）。本次为完整重启契约套件全绿（4/4 通过，2.67s，无挂起）：`probe_exhaustion_reports_error`（1.5s 级耗尽返回）与 `reuses_live_proxy` / `zombie_socket_triggers_restart` / `spawns_fake_when_none_running` 同批通过，确认新增回归守卫未破坏既有重启行为。本步仅运行测试 + 记录日志，未修改任何 src/ 与 tests/ 文件。
