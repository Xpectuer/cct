---
title: "port_occupied_reports_error_keeps_occupant — Green Phase"
brief: "port_occupied_reports_error_keeps_occupant — Green: exit 0"
doc_type: proc
created: 2026-08-01T11:09:30Z
case: "port_occupied_reports_error_keeps_occupant"
phase: green
---
Exit code: 0
Full output: `cargo test --test proxy_contract port_occupied_reports_error_keeps_occupant`（工作树根目录执行，完整输出）

```
   Compiling ring v0.17.14
   Compiling rustls v0.23.41
   Compiling rustls-webpki v0.103.13
   Compiling tokio-rustls v0.26.4
   Compiling hyper-rustls v0.27.9
   Compiling reqwest v0.12.28
   Compiling cct v0.5.0 (/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 6.04s
     Running tests/proxy_contract.rs (target/debug/deps/proxy_contract-37538df7d2e1326e)

running 1 test
test port_occupied_reports_error_keeps_occupant ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.12s
```

Green 确认：**exit 0，全断言通过**。实现按 plan code-spec Step 6 落地（src/proxy.rs `run_proxy` 内 TCP bind 段）：

```rust
let addr = format!("127.0.0.1:{port}");
let listener = match TcpListener::bind(&addr).await {
    Ok(l) => l,
    Err(e) => {
        eprintln!("[cct-proxy] TCP bind {addr} failed: {e}");
        eprintln!("[cct-proxy] {}", crate::proxy::port_conflict_message(port));
        std::process::exit(1);
    }
};
```

1. **退出码**：子进程 TCP bind 失败 → 非 panic，`exit(1)`（非 0，断言 1 通过）。
2. **诊断文本**（断言 2）：stderr 含 `[cct-proxy] TCP bind 127.0.0.1:<port> failed: Address already in use (os error 48)` + `port_conflict_message`（本机 lsof 可用 → "port <port> already in use by PID ..." 分支）。
3. **无 panic**（断言 3）：stderr 不再含 "panic" 文本。
4. **占用者存活**（断言 4）：测试进程的 occupant listener 仍持有端口，同端口再 bind 失败。
5. **范围**：仅改 TCP bind 段；控制 socket 的 remove_file/bind（TC-12 范畴）未触碰。
