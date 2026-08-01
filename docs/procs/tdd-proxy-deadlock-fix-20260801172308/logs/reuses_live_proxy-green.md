---
title: "reuses_live_proxy — Green Phase"
brief: "reuses_live_proxy — Green: exit 0"
doc_type: proc
created: 2026-08-01T10:31:17Z
case: "reuses_live_proxy"
phase: green
---
Exit code: 0
Full output: `cargo test --test launch_proxy_contract`（工作树根目录执行；首跑被 rtk 压缩为 `cargo test: 2 passed (1 suite, 0.57s)` 且 `~/Library/Application Support/rtk/tee/` 无对应日志，改用 `rtk proxy cargo test ...` 取完整输出，如下）

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.09s
     Running tests/launch_proxy_contract.rs (target/debug/deps/launch_proxy_contract-11aa5be2c88374b7)

running 2 tests
test reuses_live_proxy ... ok
test spawns_fake_when_none_running ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.57s
```

（备注：vacuous Red —— 复用路径既有实现已满足 AC9，本步无 src 改动。`src/launch.rs` 的 `ensure_proxy_running` 首行复用路径 `if check_proxy_running(socket_path) { return Ok(()) }` 在 pre-fix 已存在，TC-15 Green 已把探测升级为应用层，因此 reuses_live_proxy 在 Red 阶段即全绿；本步仅跑完整重启契约套件确认回归守卫生效：reuses_live_proxy + spawns_fake_when_none_running 双双通过，exit 0，无 src/ 改动。）
