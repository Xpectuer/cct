---
title: "check_proxy_running_app_probe — Refactor Phase"
brief: "check_proxy_running_app_probe — Refactor: exit 0"
doc_type: proc
created: 2026-08-01T09:48:04Z
case: "check_proxy_running_app_probe"
phase: refactor
---
Changes made: 提取 `fresh_test_socket(name)` 测试 helper——3 个 check_proxy_running 测试重复了 `test_socket(name)` + `std::fs::remove_file(&path)` 两步（恰好达到 KISS"三处重复才考虑提取"的阈值）。helper 附带 doc comment 说明"保证路径不存在"这一不变量（崩溃残留的 stale socket 会让 absent 测试假阴性）。3 处调用点改为单行 `let path = fresh_test_socket(...)`；每个测试末尾的 `remove_file` 清理保持不变。行为零变化。其余改动不做：

- 未重命名 `send_control_timeout` → `send_control_with_timeout`：code-spec（Step 3）明确固定该名称，后续 G2 步骤（Step 5/9 等）会引用它，重命名会造成 plan 漂移。
- 未提取 `serde_json` → `io::Error(InvalidData)` 映射 helper：该 idiom 在 proxy.rs 出现 3 次，但第 3 处在未改动的 `write_control_response`（既有代码，超出本用例 scope），且内联形式是标准 Rust idiom，提取收益边际。
- 未给 `ControlCommand` 加 bare-command 构造器：`check_proxy_running` 与 `shutdown_proxy` 各建一次全 None 命令（2 处，低于三处阈值），且 code-spec 原样给出该 literal。

具体观察：
- 死代码：无。`PROBE_RETRIES` / `STOP_TIMEOUT` 未使用系 plan 有意为之（后续步骤消费），未处理。`send_control`（公共签名冻结）转调 `send_control_timeout`，被 `switch_profile` / `shutdown_proxy` 使用。
- 命名：`PROBE_TIMEOUT` 目前也被 `send_control` 用作通用控制命令超时（switch/shutdown 走同一 500ms 窗口）——命名略窄于实际语义，但本地 unix socket 上响应恒为毫秒级，且 code-spec 固定该常量名，留作观察项。
- 复杂条件：`check_proxy_running` 为单表达式 `send_control_timeout(...).is_ok()`，`send_control_timeout` 线性流程（connect → 设超时 → 写 payload → shutdown Write → read_line → 反序列化），无嵌套分支，无需简化。
- 测试健壮性：`check_proxy_running_true_when_daemon_responds` 中若 `check_proxy_running` 断言失败，responder 线程会永久阻塞在 `accept()` 上（进程不退出）——仅失败路径受影响，未改动，留作观察项。
- 超时边界：silent 测试断言 `started.elapsed() < 5s`（probe 超时 500ms），留 4× 余量，CI 慢机器上仍安全。
test_cmd exit code: 0
output: `cargo test proxy::tests::check_proxy_running`（工作树根目录执行，完整输出如下）

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.09s
     Running unittests src/lib.rs (target/debug/deps/cct-c695630c4374c597)

running 3 tests
test proxy::tests::check_proxy_running_false_when_socket_absent ... ok
test proxy::tests::check_proxy_running_true_when_daemon_responds ... ok
test proxy::tests::check_proxy_running_false_when_socket_silent ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 134 filtered out; finished in 2.01s

     Running unittests src/main.rs (target/debug/deps/cct-a415d99fd63277d3)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 19 filtered out; finished in 0.00s

     Running tests/integration.rs (target/debug/deps/integration-7dbc2c9fd2903748)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s

     Running tests/live.rs (target/debug/deps/live-080beb1530291df0)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.00s
```
