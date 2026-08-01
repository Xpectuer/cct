---
title: "shutdown_removes_socket_file — Refactor Phase"
brief: "shutdown_removes_socket_file — Refactor: exit 0"
doc_type: proc
created: 2026-08-01T20:01:58+0800
case: "shutdown_removes_socket_file"
phase: refactor
---
Changes made:

1. **src/proxy.rs — run_proxy 控制 socket 传递链统一命名 + 删除不可达清理**：原 `let ctl_path = socket_path.to_path_buf();` + `run_control_socket(..., ctl_path.clone()).await; let _ = std::fs::remove_file(&ctl_path);` 改为阴影绑定 `let socket_path = socket_path.to_path_buf();` 后直接 move 进 `tokio::spawn` 闭包（`run_control_socket(ctl_listener, ctl_state, socket_path).await`）。两处变化：
   - 删除了 await 后的 `remove_file(&ctl_path)`——**不可达死代码**：`run_control_socket` 是无限循环永不返回（accept 错误分支 sleep 后 continue），唯一出口是 `handle_control` shutdown 分支的 `process::exit(0)`（那里已自行删 socket 文件）或 run_proxy TCP bind 失败的 `process::exit(1)`；runtime 关停时 spawned task 在 await 点被 drop，也不会执行其后代码。删除后 `ctl_path.clone()` 不再需要，传递链简化为单次 move。
   - 全链统一命名 `socket_path`：`run_proxy(&Path)` → `run_control_socket(PathBuf)` → `handle_control(PathBuf)` 原为 `socket_path` → `ctl_path` → `socket_path` → `sp` → `socket_path` 四段交替命名（Green agent 的 `ctl_path` 是 E0521 `'static` 借用的 workaround，见 green 日志），现统一为单一名称，阴影绑定带一行 why 注释（`tokio::spawn` 要求 `'static`，借用 `&Path` 不能进闭包）。行为零变化。
2. **src/proxy.rs — run_control_socket 每连接传递 `sp` → `socket_path` 阴影**：`let sp = socket_path.clone(); spawn_blocking(move || handle_control(std_stream, ctl_state, sp))` 改为 `let socket_path = socket_path.clone();` + move 进闭包（clone-then-move 惯用法，PathBuf 非 Copy，循环下轮仍需使用外层值）。消除 1-2 字母缩写名，与全链统一命名一致。
3. **src/proxy.rs — shutdown 分支删文件注释改写为 why 注释**：`// 退出前清理死 socket 缺陷`（TODO 式残留）→ 两行 why：Unix socket 文件不会随进程退出自动删除、`process::exit` 跳过析构、不删则每次 stop 留下死 socket（约束 #6 稳态缺陷）。行为零变化（仍 `let _ =` 吞错）。
4. **tests/proxy_contract.rs — 提取 `wait_child_exit` helper**：有界 try_wait 轮询循环在 stop_times_out_on_unresponsive_socket ①、port_occupied_reports_error_keeps_occupant、shutdown_removes_socket_file 三处重复（port_occupied refactor 日志明确记过"2 处低于 KISS 三处重复提取阈值，未提取"——本用例成为第 3 处，阈值达成）。提取为 `wait_child_exit(child: &mut std::process::Child, budget: Duration, context: &str) -> std::process::ExitStatus`（try_wait 轮询 + 预算断言 + 50ms sleep，断言消息 `"{context} — never exited"`），三个调用点各缩为 1 行。预算与上下文消息逐处保留：stop 4s / port_occupied 5s / shutdown 3s 均与原来一致；panic 路径 expect 消息统一为 `"try_wait child"`（原 "try_wait stop child" / "try_wait proxy child" 为 panic-only 文本，非行为契约）。
5. **tests/proxy_contract.rs — 两处轮询调用的 `let started = Instant::now()` 移除**：port_occupied 与 shutdown_removes 中 `started` 仅被循环内断言使用，改 helper 后成未使用变量，删除；stop_times_out 保留（其后 `let elapsed = started.elapsed()` 仍用）。

其余观察（未改动，含理由）：

- **`handle_control` shutdown 分支 `process::exit(0)` 保留**：在 spawn_blocking 工作线程中终止整个 proxy 进程是既有设计（外部 STOP_TIMEOUT=2s 语义依赖它），且 `process::exit` 前的顺序（先写响应再删文件再退出）正确——客户端在进程终止前必收到 ok 应答，本测试 `status.success()` 断言依赖此顺序。
- **shutdown 分支删文件 `let _ =` 吞错保留**：删文件失败（如权限/竞态）不改变 exit 语义，向客户端报错反而可能让 stop 命令误报非 0；既有行为，不改。
- **`handle_control` 三个响应构造块未提取 helper**：switch/status/shutdown 各分支的 `ControlResponse` 字面量字段集互不相同（base_url/model 的 None/Some 条件各异），提取为共享构造会引入条件参数，净可读性为负，维持内联（与 status_to_result/error_response 既有 helper 分层一致）。
- **`run_control_socket` 参数名与 `run_proxy` 绑定名统一为 `socket_path`**：阴影绑定（`let socket_path = socket_path.to_path_buf()`）为 Rust 惯用写法，文件内已有先例（port_occupied refactor 日志 `Ok(l) => l` → `Ok(listener) => listener` 注释确认该写法约定）。
- **`wait_child_exit` 未覆盖 double_start_race_one_wins 的双子进程轮询**：该循环在同一循环体内轮询两个 child（status_a/status_b 双 Option），形状与单 child helper 不同，提取反而复杂化，保持内联。
- **测试断言强度逐条核对（未削弱）**：shutdown_removes 的 `sock.exists()` 前置、`status.success()`、`!sock.exists()` 三条断言原样保留；stop_times_out 的 8 条断言、port_occupied 的 4 条断言全部原样保留（仅 panic 消息文本因 helper 合并微调，非契约断言）。

test_cmd exit code: 0
output:

```
$ rtk proxy cargo test --test proxy_contract; echo "EXIT_CODE=$?"
   Compiling ring v0.17.14
   Compiling rustls v0.23.41
   Compiling rustls-webpki v0.103.13
   Compiling tokio-rustls v0.26.4
   Compiling hyper-rustls v0.27.9
   Compiling reqwest v0.12.28
   Compiling cct v0.5.0 (/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 8.61s
     Running tests/proxy_contract.rs (target/debug/deps/proxy_contract-37538df7d2e1326e)

running 10 tests
test smoke_stub_receives_request ... ok
test log_masks_api_key ... ok
test shutdown_removes_socket_file ... ok
test port_occupied_reports_error_keeps_occupant ... ok
test concurrent_control_and_http ... ok
test double_start_race_one_wins ... ok
test log_masks_api_key_upstream_error ... ok
test stop_times_out_on_unresponsive_socket ... ok
test stub_forwarding_with_bearer ... ok
test zombie_recovery_restarts_proxy ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.46s

EXIT_CODE=0
```
