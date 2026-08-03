---
title: "probe_exhaustion_reports_error — Refactor Phase"
brief: "probe_exhaustion_reports_error — Refactor: exit 0"
doc_type: proc
created: 2026-08-01T10:53:41Z
case: "probe_exhaustion_reports_error"
phase: refactor
---
Changes made: `tests/launch_proxy_contract.rs` probe_exhaustion_reports_error 一处小改动，测试行为与断言强度零变化（3 个断言逐字保留）：

1. **命名清晰化**：setup 中 `dir.path().join("unused.ready")` 的临时借用改为先绑定局部变量 `let ready = dir.path().join("unused.ready");` 再传入 `ProxyEnvGuard::set`。对齐本文件 `setup_proxy_env` 夹具的既有约定（socket/ready/port 均显式绑定为局部变量），并消除 `dir.path()` 的第三次重复调用（前两次在 fake 脚本路径与 socket 路径）。纯语法层面重排，env 设置值完全一致。

未改动：`exit-immediately.sh` fake 脚本（`exit 0` 干净退出，隔离耗尽路径与崩溃处理路径）、`ProxyEnvGuard::set` env 注入、3 个断言及其消息（is_err / 错误信息含 "did not become healthy" / elapsed ≤ 2s）、`#[serial]` 纪律。

具体观察（测试质量，vs 耗尽契约：Err + ≤2s 不挂起）：
- 断言强度与耗尽契约一一对应：`result.is_err()` 覆盖「返回 Err」半边；`elapsed <= 2s` 覆盖「不挂起」半边；错误信息含 "did not become healthy" 是第三重契约——与 `src/launch.rs:171`（`"proxy did not become healthy after {} probes"`）精确匹配，防止实现用无关错误糊弄 Err。三重断言互不冗余，无需削弱或加强。
- `unused.ready` 命名自明：本用例的 marker 不参与任何断言（立即退出的 fake 从不读它），与其余用例「marker = 重新 spawn 硬证据」的角色区分清楚；guard 仍须持有它是因为 `ProxyEnvGuard::set` 签名固定要求 4 个 env，且 `_proxy_env` 绑定必须存活至函数末尾（Drop 恢复 env）。
- 就绪耗尽时序未硬编码 magic number 于断言中（`2s` 直接来自契约），不依赖 PROBE_TIMEOUT(500ms) × PROBE_RETRIES(3) 的实现细节——实现方调整探测节奏时测试契约依旧成立，符合「断言契约而非实现」。
- 已评估但不做：chmod 块（`metadata + set_mode(0o755) + set_permissions`）与 `write_fake_proxy` 内逐字重复——仅 2 处，按仓库 KISS「三处重复代码才考虑提取」暂不提取；复用 `setup_proxy_env` 亦被否决——它硬编码完整 fake（python accept 循环），换成它会悄悄改变本用例「立即退出目标」的场景，测试将失效。
test_cmd exit code: 0
output: `rtk proxy cargo test --test launch_proxy_contract`（工作树根目录执行，rtk proxy 取完整输出）

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.09s
     Running tests/launch_proxy_contract.rs (target/debug/deps/launch_proxy_contract-11aa5be2c88374b7)

running 4 tests
test reuses_live_proxy ... ok
test spawns_fake_when_none_running ... ok
test zombie_socket_triggers_restart ... ok
test probe_exhaustion_reports_error ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.67s

EXIT_CODE=0
```
