---
title: "port_occupied_reports_error_keeps_occupant — Refactor Phase"
brief: "port_occupied_reports_error_keeps_occupant — Refactor: exit 0"
doc_type: proc
created: 2026-08-01T19:13:19+0800
case: "port_occupied_reports_error_keeps_occupant"
phase: refactor
---
Changes made:

1. **src/proxy.rs — run_proxy TCP bind 段去掉同模块自限定路径**：`crate::proxy::port_conflict_message(port)` → `port_conflict_message(port)`。`crate::proxy::` 在 src/proxy.rs 内部是模块对自己的自引用，且与文件内其余 intra-module 调用（`run_control_socket` / `send_control_timeout` / `status_to_result` 等均为裸调用）约定不一致；launch.rs 里的 `crate::proxy::...` 是跨模块调用（正确保留）。
2. **src/proxy.rs — bind 成功分支绑定名 `l` → `listener`**：`Ok(l) => l` → `Ok(listener) => listener`（阴影绑定惯用写法）。消除 1 字母缩写，与文件内 accept 循环 `Ok(conn) => conn` 的具名绑定习惯一致。
3. **src/proxy.rs — exit(1) 前补一行 why 注释**：`exit(1) 而非 panic/return`——panic 输出违反占用诊断契约（不 panic + 报错文本）、return 则 `cct proxy start` 静默以 0 退出，两种替代方案的后果均非显而易见，符合 KISS"只在 why 不显而易见时写注释"。
4. **tests/proxy_contract.rs — 4 处 Red 阶段遗留注释更新为当前实现语义**（纯注释改动，无行为变化，断言文本与 `exit(1)` 行为原样保留）：
   - 测试 doc comment 尾句 `当前实现 run_proxy TCP bind 失败 panic（exit 101）——断言 2/3 红（真实 Red）` → `回归守卫：若 bind 失败路径回退为 panic（exit 101）或丢失诊断文本，本测试红`（与 concurrent_control_and_http refactor 同一处理先例）。
   - 有界等待注释 `（当前 panic → 101；修复后 exit(1) + 诊断）` → `（bind 失败 → exit(1) + 诊断）`。
   - 断言 1 注释 `（当前 panic → 101；修复后 exit(1)）` → `（bind 失败路径 → exit(1)）`。
   - 断言 3 注释 `（当前实现 TCP bind 失败 panic → 本断言红）` → `（exit(1) 路径不得在 stderr 输出 panic 文本）`。

其余观察（未改动，含理由）：

- **有界退出轮询循环与 `stop_times_out_on_unresponsive_socket` 重复**（2 处相同 try_wait + 预算断言模式），低于 KISS"三处重复才提取"阈值，未提取。
- **stderr 读取块与 `ProxyChild::read_stderr` 相似但语义不同**：`read_stderr` 先 kill+wait 再读（活进程 pipe 需 EOF），本用例子进程已自然退出，复用会引入"kill 已退出进程"的误导语义，保持内联显式写法。
- **match 显式分支保留**，不换 `unwrap_or_else`：错误分支含两条 eprintln + exit，显式 match 更清晰，符合项目"显式优于紧凑"风格。
- **诊断行不用 `log_proxy!` 宏**：`log_proxy!` 受 `CCT_PROXY_LOG` 门控，占用诊断属于必须无条件输出的错误路径，保持 `eprintln!`。
- **`127.0.0.1:{port}` 字面量两处**（启动日志行 + `addr` 变量），低于提取阈值，未改。
- **行为零变化**：`exit(1)` 非 0 退出、两行诊断文本（`TCP bind {addr} failed: {e}` + `port_conflict_message` 原文）、断言锚点 `port {port} already in use` / `lsof -iTCP` / 不含 `panic` 全部原样保留。

test_cmd exit code:
- `cargo test --test proxy_contract`（全量）: **exit 101** — 唯一失败 `double_start_race_one_wins` 为并行 TC-12 的 Red 阶段（控制 socket 竞态，tests/proxy_contract.rs:767 断言，与本次 TCP bind 段改动无关）；其余 8 个（含 port_occupied_reports_error_keeps_occupant）全过。
- `cargo test --test proxy_contract port_occupied_reports_error_keeps_occupant`（本用例）: **exit 0**

output: 全量 + 用例过滤两次运行完整输出如下

```
$ cargo test --test proxy_contract; echo "EXIT_CODE=$?"
    Blocking waiting for file lock on artifact directory
   Compiling ring v0.17.14
   Compiling rustls v0.23.41
   Compiling rustls-webpki v0.103.13
   Compiling tokio-rustls v0.26.4
   Compiling hyper-rustls v0.27.9
   Compiling reqwest v0.12.28
   Compiling cct v0.5.0 (/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 10.84s
     Running tests/proxy_contract.rs (target/debug/deps/proxy_contract-37538df7d2e1326e)

running 9 tests
test smoke_stub_receives_request ... ok
test port_occupied_reports_error_keeps_occupant ... ok
test stop_times_out_on_unresponsive_socket ... ok
test log_masks_api_key ... ok
test stub_forwarding_with_bearer ... ok
test concurrent_control_and_http ... ok
test double_start_race_one_wins ... FAILED
test log_masks_api_key_upstream_error ... ok
test zombie_recovery_restarts_proxy ... ok

failures:

---- double_start_race_one_wins stdout ----

thread 'double_start_race_one_wins' (6825829) panicked at tests/proxy_contract.rs:767:5:
exactly one proxy must survive the double-start race — check_proxy_running("/var/folders/8t/7x4hxj395mv4fzw_hf0jd29m0000gn/T/.tmpEsUKER/race.sock") was false after 2.018358958s (status_a=None, status_b=Some(ExitStatus(unix_wait_status(256))))
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/panicking.rs:80:14
   2: proxy_contract::double_start_race_one_wins::{{closure}}
             at ./tests/proxy_contract.rs:767:5
   3: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
   4: serial_test::serial_code_lock::local_serial_core
             at /Users/zhengjiaye/.cargo/registry/src/index.crates.io-1949cf8c6e5b557f/serial_test-3.4.0/src/serial_code_lock.rs:36:5
   5: proxy_contract::double_start_race_one_wins
             at ./tests/proxy_contract.rs:736:1
   6: proxy_contract::double_start_race_one_wins::{{closure}}
             at ./tests/proxy_contract.rs:737:32
   7: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
   8: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.


failures:
    double_start_race_one_wins

test result: FAILED. 8 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.39s

EXIT_CODE=101

$ cargo test --test proxy_contract port_occupied_reports_error_keeps_occupant; echo "EXIT_CODE=$?"
cargo test: 1 passed, 8 filtered out (1 suite, 0.16s)
EXIT_CODE=0
```
