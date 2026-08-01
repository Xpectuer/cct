---
title: "tcp_port_owner_fallback — Refactor Phase"
brief: "tcp_port_owner_fallback — Refactor: exit 0"
doc_type: proc
created: 2026-08-01T09:57:50Z
case: "tcp_port_owner_fallback"
phase: refactor
---
Changes made: `src/proxy.rs` `tcp_port_owner` 的 lsof 参数构造由 `.args([format!("-tiTCP:{port}"), "-sTCP:LISTEN".to_string()])` 改为两个 `.arg(...)` 调用（`.arg(format!("-tiTCP:{port}"))` + `.arg("-sTCP:LISTEN")`）。原写法在同一数组中混用 `format!` 与对字符串字面量冗余的 `.to_string()`；两个 `.arg()` 更直接，且生成的 argv 完全相同（`lsof -tiTCP:<port> -sTCP:LISTEN`），行为零变化。`port_conflict_message` 与两个测试未改动。

具体观察：
- 消息文本未动：`port_conflict_message` 的两条分支文本（含 "lsof -tiTCP:{port} -sTCP:LISTEN" 的 PID 文本、含 "lsof -iTCP:{port}" 的降级文本）被计划 spec（`plan/code-spec.md` Step 5）与后续契约测试断言依赖，逐字保留。注意两条文本恰好可被 "lsof -iTCP" 子串区分（PID 分支是 "-tiTCP" 带前导 t），现有降级断言有效。
- 重复逻辑：无。`tcp_port_owner`/`port_conflict_message` 单次使用、无跨函数重复；lsof 命令本身在函数内与消息文本中各出现一次，但消息文本受契约约束不可提取为共享常量（提取会改动文本拼接形状）。
- 复杂条件：解析链 `String::from_utf8(out.stdout).ok()?.lines().next().map(trim).filter(!empty)` 已是扁平单链；`.filter(|s| !s.is_empty())` 是对外部工具输出（系统边界）的防御，符合 KISS"只在系统边界做校验"，保留。
- 死代码：无。`if !out.status.success() { return None; }` 承担"lsof 存在但无监听者/非零退出"的降级路径，两个测试分别覆盖缺失（spawn 失败）与可用（有监听者）两端。
- 测试健壮性：`#[serial]` 必要（fallback 测试改写 PATH，PID 测试占用随机端口）。fallback 测试用固定 19191 端口无碍——PATH 无效时 lsof spawn 必然失败，与端口实际占用无关。PID 测试的 lsof 可用性跳过检查（`lsof -v` spawn 失败即跳过）与注释"CI 可能无 lsof"匹配，无需改动。
test_cmd exit code: 0
output: `rtk proxy cargo test tcp_port_owner`（工作树根目录执行；rtk 对 cargo 输出做摘要压缩，已用 `rtk proxy` 绕过过滤器恢复完整日志，完整输出如下）

```
    Blocking waiting for file lock on artifact directory
   Compiling ring v0.17.14
   Compiling rustls v0.23.41
   Compiling rustls-webpki v0.103.13
   Compiling tokio-rustls v0.26.4
   Compiling hyper-rustls v0.27.9
   Compiling reqwest v0.12.28
   Compiling cct v0.5.0 (/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 8.09s
     Running unittests src/lib.rs (target/debug/deps/cct-c695630c4374c597)

running 2 tests
test proxy::tests::tcp_port_owner_reports_pid_when_lsof_available ... ok
test proxy::tests::tcp_port_owner_fallback_when_lsof_missing ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 137 filtered out; finished in 0.13s

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
