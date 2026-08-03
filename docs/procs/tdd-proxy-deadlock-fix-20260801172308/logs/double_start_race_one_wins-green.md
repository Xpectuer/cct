---
title: "double_start_race_one_wins — Green Phase"
brief: "double_start_race_one_wins — Green: exit 0"
doc_type: proc
created: 2026-08-01T11:17:27Z
case: "double_start_race_one_wins"
phase: green
---
Exit code: 0
Full output: `cargo test --test proxy_contract double_start_race_one_wins`（工作树根目录执行，完整输出；rtk proxy 原始输出）

```
   Compiling ring v0.17.14
   Compiling rustls v0.23.41
   Compiling rustls-webpki v0.103.13
   Compiling tokio-rustls v0.26.4
   Compiling hyper-rustls v0.27.9
   Compiling reqwest v0.12.28
   Compiling cct v0.5.0 (/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 6.13s
     Running tests/proxy_contract.rs (target/debug/deps/proxy_contract-37538df7d2e1326e)

running 1 test
test double_start_race_one_wins ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out; finished in 2.04s
```

Green 确认：**exit 0，断言 1/2/3 全通过**。实现按 plan code-spec Step 6 控制 socket 段落地（src/proxy.rs `run_proxy` 内，TCP bind 段未触碰）：

```rust
    // 先探测再删：有活 proxy → 报错退出，不破坏其控制通道（约束 #5）。
    if check_proxy_running(socket_path) {
        eprintln!(
            "[cct-proxy] another live proxy already owns control socket {socket_path:?} — exiting"
        );
        std::process::exit(1);
    }
    let _ = std::fs::remove_file(socket_path); // 探测失败后才删（约束 #3）

    let ctl_listener = match TokioUnixListener::bind(socket_path) {
        Ok(l) => l,
        Err(e) if e.kind() == io::ErrorKind::AddrInUse => {
            // 双启动竞态：已有实例并发启动 → 重新探测，耗尽报错（保证收敛）。
            for _ in 0..PROBE_RETRIES {
                if check_proxy_running(socket_path) {
                    eprintln!(
                        "[cct-proxy] another live proxy owns control socket {socket_path:?} — exiting"
                    );
                    std::process::exit(1);
                }
                std::thread::sleep(PROBE_TIMEOUT);
            }
            eprintln!("[cct-proxy] control socket bind {socket_path:?} failed: {e}");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("[cct-proxy] control socket bind {socket_path:?} failed: {e}");
            std::process::exit(1);
        }
    };
    log_proxy!("control socket bound");
```

1. **断言 1（恰一个存活）**：`check_proxy_running` 应用层探测成功——双启动收敛为恰一个进程持有控制通道。
2. **断言 2（输家非 0 退出）**：输家不再 panic，改走 `eprintln!` 诊断 + `std::process::exit(1)`（非 0）。
3. **断言 3（无 "panic"）**：两子进程 stderr 合计不再含 "panic"（旧实现 `bind(socket_path).expect(...)` 的 panic 路径已移除）。
4. **先探测再删（约束 #5/#3）**：有活 proxy 时直接报错退出、不 `remove_file` 破坏其控制通道；探测失败后才删。
5. **EADDRINUSE 收敛**：bind 失败（已有实例并发启动）→ 重新探测 `PROBE_RETRIES`×`PROBE_TIMEOUT`（3×500ms），探测到活实例报错退出，耗尽报错退出；均非 panic。
6. **平台注意**：本机（macOS）Unix socket bind 到已存在路径报 `Os { code: 17, kind: AlreadyExists }`（EEXIST），实际走 catch-all `Err(e)` 分支（同样 exit(1) + 诊断，无 panic，收敛语义不变）；`AddrInUse`（EADDRINUSE）分支主要服务 Linux，保持与 plan 一致。
7. **范围**：仅改控制 socket 段（先探测再删 + bind 失败处理）；TCP bind 段（TC-11 已做）未触碰。
8. **稳定性**：竞态测试连跑 5 次（1 次原始 + 4 次复跑）均 exit 0、`finished in ~2.0s`，无 flake。

前置 Red 日志: docs/procs/tdd-proxy-deadlock-fix-20260801172308/logs/double_start_race_one_wins-red.md
