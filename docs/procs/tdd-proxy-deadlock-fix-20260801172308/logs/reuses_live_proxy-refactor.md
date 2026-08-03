---
title: "reuses_live_proxy — Refactor Phase"
brief: "reuses_live_proxy — Refactor: exit 0"
doc_type: proc
created: 2026-08-01T10:35:40Z
case: "reuses_live_proxy"
phase: refactor
---
Changes made: `tests/launch_proxy_contract.rs` 三处改动，行为与断言强度零变化：

1. **env 保存/恢复去重**：`reuses_live_proxy` 与 `spawns_fake_when_none_running` 各有一份逐字重复的 12 行 env 块（4 个 prev 保存 + 4 个 set_var + 4 个 restore_env）。提取为 `ProxyEnvGuard`（构造时保存 CCT_PROXY_BIN/SOCKET/PORT/READY_MARKER 原值并覆写，Drop 时恢复），删除了因此不再使用的 `restore_env` helper。两测试各收敛为一行 `let _proxy_env = ProxyEnvGuard::set(&fake, &socket, port, &ready);`。
   - 行为等价性：guard 在 `ensure_proxy_running` 调用前创建（set 顺序与原代码一致，fake child 仍能继承 env），恢复点从"调用后、断言前"移到作用域末尾；断言及其消息不依赖 env 状态，通过/失败判定不变。
   - 额外收益：原代码若在就绪等待循环或调用中 panic，env 恢复不执行（restore 语句在调用之后才运行）；guard 的 Drop 恢复对 panic 路径同样生效，serial 测试间更不泄漏。此改动未引入任何断言，也未改变任何既有断言。
2. **删除恒真断言**：`assert_eq!(child.id(), pid)` 中 `pid` 即 `child.id()`（`Child::id` 返回不可变存储值，两者永远相等），该断言不可能失败，属防御性死代码（违反仓库 KISS"无防御性代码"）。真正的 PID 级信号是同一 handle 的存活断言 `child.try_wait().unwrap().is_none()`（若实现盲目重启，新 fake 的 `rm -f $SOCK` 会使原 fake 的 accept 循环退出、进程终止，try_wait 即返回 Some）。删除后 AC9"复用且不再 spawn"的判定能力不变：Ok 复用 + liveness + READY 标记 mtime 双信号保留，测试 doc 注释与内联注释同步改写（"原 fake PID 未变"→"原 fake 进程仍存活"）。
3. **命名清晰化**：手动拉起的 fake 进程句柄 `child` 更名为 `proxy_child`（与文档注释中"原 fake"语义一致，避免与通用 child 混淆）；`pid` 保留（用于就绪等待超时的失败消息）。

未改动：fake 脚本、就绪等待循环（单次使用，按 KISS 内联）、`check_proxy_running` 应用层探测、liveness/mtime/Ok 三重断言本身、`#[serial]` 纪律。

具体观察（测试质量）：
- 断言强度 vs AC9：三信号互相独立——`result.is_ok()`（复用路径返回 Ok）、liveness（进程未重启，PID 级最强信号）、mtime（重启会 touch 出新 mtime，秒级以下分辨率足够区分，且仅作 liveness 之外的 belt-and-suspenders；即便某文件系统 mtime 粒度粗导致误放行，liveness 仍独立兜底）。原代码中"PID 未变"断言恒真、无信息量，已删除；"PID/liveness/mtime 三断言"的实际覆盖由 liveness+mtime 完整承担。
- 就绪等待循环的失败消息带 pid，超时时可直接定位进程，好。
- `write_fake_proxy` 的"socket 被 TempDir 清理后自终止"依赖 `while os.path.exists(sock)`，注释已说明，非显而易见行为的必要实现。
test_cmd exit code: 0
output: `rtk proxy cargo test --test launch_proxy_contract`（工作树根目录执行；rtk 对 cargo 输出做摘要压缩，已用 `rtk proxy` 绕过过滤器恢复完整日志，完整输出如下）

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.09s
     Running tests/launch_proxy_contract.rs (target/debug/deps/launch_proxy_contract-11aa5be2c88374b7)

running 2 tests
test spawns_fake_when_none_running ... ok
test reuses_live_proxy ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.57s

EXIT_CODE=0
```
