---
title: "port_occupied_bails_with_diagnosis — Red Phase"
brief: "port_occupied_bails_with_diagnosis — Red: exit 0"
doc_type: proc
created: 2026-08-01T10:55:58Z
case: "port_occupied_bails_with_diagnosis"
phase: red
---
Exit code: 0
Full output: `rtk proxy cargo test --test launch_proxy_contract port_occupied_bails_with_diagnosis`（工作树根目录执行，rtk proxy 取完整输出）

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.09s
     Running tests/launch_proxy_contract.rs (target/debug/deps/launch_proxy_contract-11aa5be2c88374b7)

running 1 test
test port_occupied_bails_with_diagnosis ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.06s
```

Red 确认：**本用例无法在 Red 阶段演示失败——端口预检已被 TC-15 Green 交付**（vacuous Red，与 TC-16/TC-17/TC-18 同因）。`src/launch.rs:140-141` 的 `ensure_proxy_running` 流程为：应用层探测 → 失败 → **试探 bind 端口**（`TcpListener::bind(("127.0.0.1", port)).is_err()`）→ 被占则 `bail!(port_conflict_message(port))`（不含 spawn）。本用例的测试进程 bind 一个动态端口并保持监听（listener 存活至测试结束），`ensure_proxy_running` 首轮应用层探测（socket 不存在 → false）后 bind 试探立即失败 → 直接 Err，0.06s 返回（无 spawn、无探测等待）。

测试断言全部非空洞（若端口预检被破坏，任一断言都会失败），覆盖端口冲突语义全链：

1. **Err 契约**：`result.is_err()` —— 端口被占时 `ensure_proxy_running` 必须返回 Err（若实现忽略占用、照常 spawn fake，则 fake 会 touch READY 标记 → 断言 3 红；若实现返回 Ok，此处红）；
2. **占用诊断信息**：`msg.contains("port {port} already in use")` —— 错误必须含 `port_conflict_message` 的占用诊断文本（当前实现经 `tcp_port_owner` 区分 "by PID {pid}" / "运行 lsof -iTCP:{port} 查看占用者" 两个变体，二者均含该前缀；若实现返回无信息量的裸错误/无关错误，此处红）；
3. **未 spawn 证据**：`!ready.exists()` —— fake 仅在启动时 touch READY 标记，标记不存在即 `ensure_proxy_running` 未拉起任何进程（若实现先 spawn 再发现端口冲突，fake 已 touch READY → 此处红）。

实际运行结果：0.06s 完成，与"bind 试探失败即 bail"路径吻合——无 spawn、无探测循环。

结论：本用例作为"端口冲突必须报错且不得 spawn"类实现的回归防护网有效（对"忽略占用强行 spawn"或"返回非诊断性错误"类实现会红），但当前实现已满足该契约——Green 阶段将无需改动 src/ 即为全绿（已实现行为 + 回归测试落位）。
