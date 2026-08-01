---
title: "reuses_live_proxy — Red Phase"
brief: "reuses_live_proxy — Red: exit 0"
doc_type: proc
created: 2026-08-01T10:25:27Z
case: "reuses_live_proxy"
phase: red
---
Exit code: 0
Full output: `cargo test --test launch_proxy_contract reuses_live_proxy`（工作树根目录执行；首跑被 rtk 压缩为 `cargo test: 1 passed, 1 filtered out (1 suite, 0.06s)`，且 `~/Library/Application Support/rtk/tee/` 无对应日志，改用 `rtk proxy cargo test ...` 取完整输出，如下）

```
   Compiling ring v0.17.14
   Compiling rustls v0.23.41
   Compiling rustls-webpki v0.103.13
   Compiling tokio-rustls v0.26.4
   Compiling hyper-rustls v0.27.9
   Compiling reqwest v0.12.28
   Compiling cct v0.5.0 (/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 5.88s
     Running tests/launch_proxy_contract.rs (target/debug/deps/launch_proxy_contract-11aa5be2c88374b7)

running 1 test
test reuses_live_proxy ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.06s
```

Red 确认：**该用例无法在 Red 阶段演示失败——AC9 复用行为已被现有实现交付**。`src/launch.rs:135-137` 的 `ensure_proxy_running` 首行即复用路径：`if crate::proxy::check_proxy_running(socket_path) { return Ok(()) }`——活 proxy 应用层探测（status 协议，TC-2/TC-15 Green 后）命中即直接返回 Ok，不再 spawn。该检查在修复前旧实现中即已存在（plan Step 4 "Old" 首行同形），TC-15 Green（Step 4 重写）只是把内核 connect 探测升级为应用层探测并保留复用语义，因此本用例断言全部通过、exit 0。

为满足"核心断言必须是**进程未重启**"（不重蹈"仅断言 Ok"的弱断言），本测试从第一版即写入最强断言集，全部非空洞（若实现盲目重启，任一断言都会失败）：
1. `ensure_proxy_running(port, &socket)` 返回 `Ok`（复用而非报错）；
2. 手动拉起的 fake PID 未变且 `child.try_wait()` 为 None（仍存活）——若实现探测失败后盲目重启，新 fake 启动时 `rm -f $SOCK` 会令原 fake 的 python accept 循环退出 → 此断言失败；
3. READY 标记 mtime 未被重写——fake 仅在启动时 touch 一次，重启会 touch 出新 mtime → 此断言失败；
4. 就绪同步：等待 READY 标记 **且** `check_proxy_running` 通过后才记录稳态（避免"标记已写但 socket 未 bind"竞态被误判为重启）。

实际运行结果：0.06s 内完成，fake 首轮轮询即健康，ensure_proxy_running 探测命中走复用路径 → Ok；原 fake PID 未变且存活、mtime 未变。测试 0.06s 内返回，无挂起。

结论：本用例作为 AC9 复用回归防护网有效（对"盲目重启"类实现会红），但当前实现已满足该契约——Green 阶段将无需改动 src/ 即为全绿（已实现行为 + 回归测试落位）。
