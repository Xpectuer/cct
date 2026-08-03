---
title: "zombie_recovery_restarts_proxy — Red Phase"
brief: "zombie_recovery_restarts_proxy — Red: exit 0"
doc_type: proc
created: 2026-08-01T10:58:08Z
case: "zombie_recovery_restarts_proxy"
phase: red
---
Exit code: 0
Full output: `cargo test --test proxy_contract zombie_recovery_restarts_proxy`（工作树根目录执行，rtk proxy 取完整输出）

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.08s
     Running tests/proxy_contract.rs (target/debug/deps/proxy_contract-37538df7d2e1326e)

running 1 test
test zombie_recovery_restarts_proxy ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.63s
```

Red 确认：**本用例无法在 Red 阶段演示失败——AC2 僵尸自愈语义已被现有实现交付**（vacuous Red，与 TC-16/TC-17/TC-18 同因）。`src/launch.rs:134` 的 `ensure_proxy_running` 完整路径（探测→bind 预检→spawn→就绪）已由 TC-15 Green（Step 4 重写）实现：应用层探测失败 → 试探 bind 端口（旧 proxy 已 SIGKILL，端口已释放）→ 经 `CCT_PROXY_BIN` spawn（本测试指向真实 cct 入口 `env!("CARGO_BIN_EXE_cct")`）→ 就绪探测（PROBE_TIMEOUT 500ms × PROBE_RETRIES 3）→ Ok。真实 proxy 启动时 `remove_file(socket_path)`（`src/proxy.rs:217`）清理残留 socket 并重新 bind → 健康恢复。本测试是这条自愈链的回归守卫。

测试断言全部非空洞，覆盖完整僵尸自愈语义链（若任一环节被破坏都会红）：
1. **前置残留**：`sock.exists()` —— SIGKILL 后 socket 文件必须残留（若实现有清理路径删除 socket，此处红）；
2. **前置死亡**：`!check_proxy_running(&sock)` —— 残留 socket 无进程应答时应用层探测必须失败（若探测误判健康，ensure_proxy_running 走复用路径、测试后续断言失去意义）；
3. **重启契约**：`ensure_proxy_running(port, &sock)` 返回 Ok —— 若实现无重启路径（返回 Err 或挂起），此处红；
4. **恢复健康**：`check_proxy_running(&sock)` 为 true —— 重启后的真实 proxy 必须能应答应用层 status 探测。

测试自身正确性验证：
- **CCT_PROXY_BIN 注入生效**：`RestartEnvGuard::set` 设 `CCT_PROXY_BIN=env!("CARGO_BIN_EXE_cct")`（否则 ensure_proxy_running 会 spawn 测试二进制自身 → 就绪探测耗尽，0.63s 实测时长证明未发生）；SOCKET/PORT 指向本测试临时路径，`ensure_proxy_running` 的 spawn 子进程继承父进程 env。
- **无孤儿泄漏**：`RestartEnvGuard` Drop 时向重启的 proxy 发送 `{"cmd":"shutdown"}`（proxy.rs "shutdown" 分支 `std::process::exit(0)`）并恢复被覆写的 `CCT_PROXY_BIN/SOCKET/PORT/LOG` 进程 env。运行后 `pgrep -fl "target/debug/cct proxy start"` 无残留进程；唯一在跑的是用户 14:42 启动的 `~/.local/bin/cct proxy start` 实例（安装版、默认 socket，与本测试无关）。
- **并发注意事项**：运行期间发现同工作树有并行 agent 正在重构 `tests/launch_proxy_contract.rs`（rtk tee 日志 1785581254 显示其中间态编译错误）；本测试 `--test proxy_contract` 只构建 proxy_contract 目标，不受影响。

结论：本用例作为"僵尸 socket 重启"类实现的回归防护网有效（对"残留 socket 不清理导致 bind 失败""重启后不就绪""返回 Err"类实现会红），但当前实现已满足该契约——Green 阶段将无需改动 src/ 即为全绿（已实现行为 + 回归测试落位）。
