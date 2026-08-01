---
title: "spawns_fake_when_none_running — Green Phase"
brief: "spawns_fake_when_none_running — Green: exit 0"
doc_type: proc
created: 2026-08-01T10:17:10Z
case: "spawns_fake_when_none_running"
phase: green
---
Exit code: 0
Full output: `cargo test --test launch_proxy_contract`（工作树根目录执行；rtk proxy 原样执行并落盘完整日志）

```
    Blocking waiting for file lock on artifact directory
   Compiling ring v0.17.14
   Compiling rustls v0.23.41
   Compiling rustls-webpki v0.103.13
   Compiling tokio-rustls v0.26.4
   Compiling hyper-rustls v0.27.9
   Compiling reqwest v0.12.28
   Compiling cct v0.5.0 (/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 7.79s
     Running tests/launch_proxy_contract.rs (target/debug/deps/launch_proxy_contract-11aa5be2c88374b7)

running 1 test
test spawns_fake_when_none_running ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.51s
```

Green 确认：按 code-spec Step 4 重写 `src/launch.rs` `ensure_proxy_running`（仅三处替换，spawn 其余参数/env 逻辑不动）：① `_port` → `port` 并新增端口空闲判定——应用层探测失败后 `TcpListener::bind(("127.0.0.1", port))` 试探 bind（is_err 时临时 listener 即 drop），端口被占 → `bail!(port_conflict_message(port))`（lsof 诊断，约束 #4）；② spawn 目标改为 `CCT_PROXY_BIN` env 注入（取 String），回退链 `current_exe().ok()` → `to_str()` 失败时回退 `"cct"`（不传播错误，spawn 失败由 `cmd.spawn().context(...)` 兜底，约束 #8）；③ 就绪轮询从 5s/100ms 循环替换为 `PROBE_RETRIES × PROBE_TIMEOUT`（500ms×3）循环，耗尽 `bail!("proxy did not become healthy after {} probes")`。fake 经 CCT_PROXY_BIN 被拉起（READY 标记断言通过），fake 可应答应用层 status 探测 → `ensure_proxy_running` 返回 Ok（exit 0，0.51s）。
