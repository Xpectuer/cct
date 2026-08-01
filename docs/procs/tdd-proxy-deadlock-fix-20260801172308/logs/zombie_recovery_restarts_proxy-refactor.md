---
title: "zombie_recovery_restarts_proxy — Refactor Phase"
brief: "zombie_recovery_restarts_proxy — Refactor: exit 0"
doc_type: proc
created: 2026-08-01T19:03:56+0800
case: "zombie_recovery_restarts_proxy"
phase: refactor
---
Changes made:

1. **tests/proxy_contract.rs — 提取 `ProxyChild::kill_and_wait()`**：`kill + wait` 回收序列出现 3 处（`Drop`、`read_stderr`、僵尸测试中直接访问 `proxy.0`），达到 KISS"三处重复才提取"阈值，提取为 `fn kill_and_wait(&mut self) -> std::io::Result<()>`。`Drop` 与 `read_stderr` 改为调用它（错误忽略语义原样保留：`let _ = ...`）；僵尸测试改为 `proxy.kill_and_wait().expect("SIGKILL proxy and reap it")`——`expect` 保留 panic-on-error 断言强度，与原先两行各自 `.expect()` 等价（kill 或 wait 失败都会 panic），同时消除测试对 `ProxyChild.0` 内部字段的直接访问（封装改善）。
2. **tests/proxy_contract.rs — 变量名 `_restart` → `_env_guard`**：`RestartEnvGuard` 不做重启动作，只负责覆写/恢复 env；`_restart` 会误导读者以为守卫自身重启 proxy。`_env_guard` 与守卫文档注释（"重启路径的 env 守卫"）一致。下划线前缀保留（值须存活到作用域结束以触发 Drop 恢复 env，不能写成 `let _ = ...`）。

**RestartEnvGuard 本身未改动**，评估如下：`prev: [(&'static str, Option<String>); 4]` 的魔数 `4` 由编译器强制（数组长度与 `set` 中字面量不匹配即编译错误），不是隐性不变量；`set` 中逐 key 快照 + 逐 key 覆写/移除的线性写法是惯用形态，改为 const keys 列表 + `map` 只是把字面量重复换成 const 声明，无实质增益，违反 KISS，不改。

其余观察（含断言强度 vs AC2）：

- **AC2 全链条断言完整，未削弱**：① kill + wait 回收（expect panic-on-error）→ ② `sock.exists()` 断言 socket 残留 → ③ `!check_proxy_running` 断言应用层探测失败 → ④ `ensure_proxy_running` 返回 Ok → ⑤ `check_proxy_running` 断言重启后健康。每一步都有对应断言，前置条件（②③）保证僵尸场景真实成立（若 kill 失败，③ 会因 proxy 仍存活而失败，不会空转通过）。
- **⑤ 与 ④ 的关系**：`ensure_proxy_running`（src/launch.rs:164-169）契约上只在就绪探测成功后返回 Ok，故 ⑤ 在 ④ Ok 时已被蕴含；保留 ⑤ 作为 AC2 终态（"重启 → 健康"）的显式断言，防未来实现偏离契约（如提前返回 Ok 不探测），属正确取舍而非冗余。
- **健康定义匹配 AC2**：重启后的健康 = 控制通道应用层 status 应答（`check_proxy_running`），与 AC2"健康"定义一致；重启后的 HTTP 转发链路由 `concurrent_control_and_http` 覆盖，本用例无需追加转发断言。
- **env 隔离完整**：`RestartEnvGuard` 覆写 `CCT_PROXY_BIN`（真实入口，与 Step 12 launch 契约同一注入约定）、`CCT_PROXY_SOCKET/PORT`（本测试临时路径），并移除 `CCT_PROXY_LOG`（防重启 proxy 写用户 proxy.log）；Drop 时先 shutdown 新拉起的 daemon（`send_control` shutdown 分支 exit(0)，失败忽略）再恢复原 env，panic 路径也不遗留孤儿进程，无需改动。
- **测试与其他用例的串行隔离**：`#[serial]` 保证 env 覆写期间无其他测试并发读取进程 env，guard 恢复发生在测试作用域结束时，顺序正确。
test_cmd exit code: 0
output: 按任务要求执行 `cargo test --test proxy_contract`（工作树根目录；为记录完整输出使用 `rtk proxy` 原始模式运行，语义等价），完整输出如下

```
$ rtk proxy cargo test --test proxy_contract; echo "EXIT_CODE=$?"
    Blocking waiting for file lock on artifact directory
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.48s
     Running tests/proxy_contract.rs (target/debug/deps/proxy_contract-37538df7d2e1326e)

running 7 tests
test smoke_stub_receives_request ... ok
test log_masks_api_key ... ok
test stub_forwarding_with_bearer ... ok
test log_masks_api_key_upstream_error ... ok
test concurrent_control_and_http ... ok
test stop_times_out_on_unresponsive_socket ... ok
test zombie_recovery_restarts_proxy ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.17s

EXIT_CODE=0
```
