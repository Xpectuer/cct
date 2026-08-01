---
title: "shutdown_removes_socket_file — Green Phase"
brief: "shutdown_removes_socket_file — Green: exit 0"
doc_type: proc
created: 2026-08-01T11:56:49Z
case: "shutdown_removes_socket_file"
phase: green
---
Exit code: 0
Full output: `cargo test --test proxy_contract shutdown_removes_socket_file`（工作树根目录执行，完整输出如下）

```
   Compiling ring v0.17.14
   Compiling rustls v0.23.41
   Compiling rustls-webpki v0.103.13
   Compiling tokio-rustls v0.26.4
   Compiling hyper-rustls v0.27.9
   Compiling reqwest v0.12.28
   Compiling cct v0.5.0 (/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 6.40s
     Running tests/proxy_contract.rs (target/debug/deps/proxy_contract-37538df7d2e1326e)

running 1 test
test shutdown_removes_socket_file ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out; finished in 0.17s
```

（备注：首轮编译曾报 E0521 —— `run_proxy` 内 `socket_path.to_path_buf()` 直接写在 `tokio::spawn(async move)` 闭包内，借用越过 `'static` 边界；改为闭包外既有 `ctl_path`（即 `socket_path.to_path_buf()` 的值）clone 传入后编译通过。）

Green 确认：`shutdown_removes_socket_file` exit 0 —— stop 命令退出前由 `handle_control` shutdown 分支 `std::fs::remove_file(&socket_path)` 清理 socket 文件（约束 #6 稳态缺陷修复），测试断言 `!sock.exists()` 通过。
