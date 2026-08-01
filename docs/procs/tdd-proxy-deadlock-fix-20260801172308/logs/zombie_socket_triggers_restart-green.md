---
title: "zombie_socket_triggers_restart — Green Phase"
brief: "zombie_socket_triggers_restart — Green: exit 0"
doc_type: proc
created: 2026-08-01T10:40:04Z
case: "zombie_socket_triggers_restart"
phase: green
---
Exit code: 0
Full output: `rtk proxy cargo test --test launch_proxy_contract`（工作树根目录执行，rtk proxy 取完整输出）

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.09s
     Running tests/launch_proxy_contract.rs (target/debug/deps/launch_proxy_contract-11aa5be2c88374b7)

running 3 tests
test reuses_live_proxy ... ok
test zombie_socket_triggers_restart ... ok
test spawns_fake_when_none_running ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.14s
```

（备注：vacuous Red —— 重启路径既有实现已满足 AC2，本步无 src 改动）

完整重启契约套件（spawns_fake_when_none_running + reuses_live_proxy + zombie_socket_triggers_restart）全绿，exit 0，1.14s 完成。zombie_socket_triggers_restart 的 5 个非空断言（SIGKILL 后 socket 残留 → 删 READY 标记后探测失败 → ensure_proxy_running Ok → READY 重新出现 = 重新 spawn 的硬证据 → 重启后探测健康）全部通过，与本步预期一致：重启路径已由 TC-15 Green 实现，本用例作为 AC2 僵尸自愈重启语义的回归守卫落位，无 src 改动。
