---
title: "port_occupied_bails_with_diagnosis — Refactor Phase"
brief: "port_occupied_bails_with_diagnosis — Refactor: exit 0"
doc_type: proc
created: 2026-08-01T19:03:24+0800
case: "port_occupied_bails_with_diagnosis"
phase: refactor
---
Changes made: `tests/launch_proxy_contract.rs` 三处改动，测试行为与断言强度零变化（3 个断言逐字保留：is_err / 错误信息含 "port {port} already in use" / READY 标记不存在）：

1. **重复逻辑去重**：用例内联的 fake/socket/ready/guard 建立（4 行）与本文件 `setup_proxy_env` 夹具几乎逐字重复，唯一差异是端口来源（占用 vs `free_port()`）。提取 `setup_proxy_env_with_port(dir, port)` 作为底层构造函数，原 `setup_proxy_env` 改为委托它并取 `free_port()`——占用端口与空闲端口场景共用同一套 env 建立逻辑，端口来源差异显式化到调用方。用例主体收敛为一行 `let (_fake, socket, ready, _, _proxy_env) = setup_proxy_env_with_port(dir.path(), port);`。
2. **断言内重复格式化消除**：`msg.contains(&format!("port {port} already in use"))` 把 needle 格式化了两遍（条件 + 断言消息里再次内联）。绑定 `let needle = format!("port {port} already in use");` 后条件与消息共用同一变量，诊断消息引用 `{needle}` 不再重复。
3. **冗余注释替换**：原行内注释（"占住一个动态端口并保持监听——listener 存活至测试结束，端口冲突现场成立"）与用例 doc 注释（"bind 一个动态端口并**保持监听**（listener 存活至测试结束）"）逐字重复。替换为补充新信息的注释：`// 保持监听使端口持续被占：free_port() 会立即释放，制造不了冲突现场。`——点明"为何不用 free_port()"这一非显而易见的原因，符合 KISS「只在'为什么'不显而易见时写注释」。

未改动：`write_fake_proxy`、`ProxyEnvGuard`、`free_port`、`setup_proxy_env` 的既有 3 个调用点（spawns_fake / reuses_live / zombie）、3 个断言及其消息、`#[serial]` 纪律。

具体观察（测试质量，vs AC3 未 spawn：Err + 含 port + marker 不存在）：
- 断言强度与 AC3 一一对应且互不冗余：`result.is_err()` 覆盖「直接返回 Err，不尝试拉起」；错误信息含 `"port {port} already in use"` 是占用诊断的精确契约（与 `src/launch.rs` 端口占用路径错误文案匹配），防止实现用无关错误糊弄 Err；`!ready.exists()` 覆盖「未 spawn」半边——fake 仅在启动时 touch 一次 READY 标记，标记不存在即从未被拉起，与本文件其余用例（spawns_fake 断言标记存在、zombie 断言标记被重新 touch）共用同一证据约定。三重断言无一处可削弱或需要加强。
- 占用现场由 `TcpListener::bind(("127.0.0.1", 0))` + 保持监听制造，端口经 `local_addr()` 取自 OS 而非硬编码——与 `free_port()` 同约定，无端口冲突假阳性；listener 绑定存活至测试结束，Drop 时 OS 释放，无泄漏。
- 已评估但不做：`port` 参数化后的返回元组仍含 `port` 字段（调用方已知其值）——保留是为与 `setup_proxy_env` 返回元组形状完全一致，避免两个夹具函数签名漂移；`free_port`/bind+local_addr 两处取端口逻辑不提取（`free_port` 释放、本用例持有 listener，语义相反，合并反而制造抽象）。
test_cmd exit code: 0
output: `rtk proxy cargo test --test launch_proxy_contract`（工作树根目录执行，rtk proxy 取完整输出）

```
    Blocking waiting for file lock on artifact directory
    Finished `test` profile [unoptimized + debuginfo] target(s) in 9.25s
     Running tests/launch_proxy_contract.rs (target/debug/deps/launch_proxy_contract-11aa5be2c88374b7)

running 5 tests
test reuses_live_proxy ... ok
test probe_exhaustion_reports_error ... ok
test port_occupied_bails_with_diagnosis ... ok
test spawns_fake_when_none_running ... ok
test zombie_socket_triggers_restart ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.71s

EXIT_CODE=0
```
