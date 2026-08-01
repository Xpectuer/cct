---
title: "zombie_socket_triggers_restart — Red Phase"
brief: "zombie_socket_triggers_restart — Red: exit 0"
doc_type: proc
created: 2026-08-01T10:37:36Z
case: "zombie_socket_triggers_restart"
phase: red
---
Exit code: 0
Full output: `rtk proxy cargo test --test launch_proxy_contract zombie_socket_triggers_restart`（工作树根目录执行，rtk proxy 取完整输出）

```
    Blocking waiting for file lock on artifact directory
    Finished `test` profile [unoptimized + debuginfo] target(s) in 6.56s
     Running tests/launch_proxy_contract.rs (target/debug/deps/launch_proxy_contract-11aa5be2c88374b7)

running 1 test
test zombie_socket_triggers_restart ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.57s
```

Red 确认：**该用例无法在 Red 阶段演示失败——AC2 重启行为已被现有实现交付**（vacuous Red，与 TC-16 reuses_live_proxy 同因）。`src/launch.rs:134` 的 `ensure_proxy_running` 流程为：应用层探测 → 失败 → 试探 bind 端口 → spawn（CCT_PROXY_BIN）→ 就绪探测（PROBE_TIMEOUT × PROBE_RETRIES）→ Ok。本用例的关键事实是 **fake 不监听 TCP 端口**（它只是应答 status 的 Unix socket 服务），因此 zombie 场景下端口空闲、bind 预检成功 → 正常 spawn fake → 就绪探测通过 → Ok。重启路径本身已由 TC-15 Green（Step 4 重写）实现，本测试是 AC2 重启语义的回归守卫。

测试断言全部非空洞（若重启路径被破坏，任一断言都会失败），覆盖完整 AC2 语义链：
1. **前置-僵尸成立**：SIGKILL（`child.kill()`，无清理路径可执行）→ `child.wait()` 确认进程退出 → `socket.exists()` 断言 socket 文件残留；
2. **前置-探测失败**：删除 READY 标记后 `!check_proxy_running(&socket)` —— 残留 socket 无进程应答（UnixStream connect 拒绝）→ 应用层探测 false（内核 connect 探测无法发现死 proxy，这正是 TC-2 引入应用层探测的原因）；
3. **重启生效**：`ensure_proxy_running(port, &socket)` 返回 `Ok`；
4. **重启证据（核心）**：`ready.exists()` —— READY 标记在调用前被显式删除，调用后重新出现只能来自新 spawn 的 fake（fake 仅在启动时 touch 一次）→ 证明 fake 确实被重新 spawn，而非假 Ok；
5. **恢复健康**：`check_proxy_running(&socket)` 为 true —— 重启后能应答应用层 status 探测。

实际运行结果：0.57s 完成，无挂起（对比 Step 4 "Old" 的死锁场景：僵尸 socket + 恢复路径若缺失会在就绪探测处耗尽 → Err）。

结论：本用例作为 AC2 僵尸自愈重启的回归防护网有效（对"不重启就返回 Ok"或"就绪探测耗尽"类实现会红），但当前实现已满足该契约——Green 阶段将无需改动 src/ 即为全绿（已实现行为 + 回归测试落位）。
