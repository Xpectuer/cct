---
title: "tcp_port_owner_fallback — Green Phase"
brief: "tcp_port_owner_fallback — Green: exit 0"
doc_type: proc
created: 2026-08-01T09:56:14Z
case: "tcp_port_owner_fallback"
phase: green
---
Exit code: 0

Full output: `cargo test tcp_port_owner`（工作树根目录执行；rtk 对 cargo 输出做摘要压缩，已用 `rtk proxy cargo test tcp_port_owner` 绕过过滤器恢复完整日志，完整输出如下）

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.10s
     Running unittests src/lib.rs (target/debug/deps/cct-c695630c4374c597)

running 2 tests
test proxy::tests::tcp_port_owner_reports_pid_when_lsof_available ... ok
test proxy::tests::tcp_port_owner_fallback_when_lsof_missing ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 137 filtered out; finished in 0.00s

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

复验：同命令在沙箱开启/关闭（`dangerouslyDisableSandbox`）以及直接运行 lib 测试二进制
（`target/debug/deps/cct-c695630c4374c597 tcp_port_owner`）共 4 种方式均 exit 0、2 passed。

环境备注：端口 19191 仍被旧实例 PID 29182（`cct proxy start`）占用（shell 层 `lsof -tiTCP:19191 -sTCP:LISTEN` 可见）。
本机测试进程内 lsof 能看到测试自身 listener（test 2 非空返回），但看不到 29182 的 listener——
test 1 的降级断言（`msg.contains("lsof -iTCP")`）因此成立。若在 lsof 可跨进程可见且 19191 被占的
机器上运行，test 1 会走到 PID 分支而失败；该耦合与 Step 15 迁移前置（用户确认后终止 29182）相关，G3 处理。
