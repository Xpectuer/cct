---
title: "stop_times_out_on_unresponsive_socket — Refactor Phase"
brief: "stop_times_out_on_unresponsive_socket — Refactor: exit 0"
doc_type: proc
created: 2026-08-01T18:54:03+0800
case: "stop_times_out_on_unresponsive_socket"
phase: refactor
---
Changes made:

1. **tests/proxy_contract.rs — 提取 `spawn_stop(sock: &Path) -> std::process::Child` helper**：测试两处 `cct proxy stop` 子进程构造（①无响应 socket 场景、②absent socket 场景）重复 5 行 Command 构造（bin / args / CCT_PROXY_SOCKET env / stdout+stderr piped / spawn）。提取为 `spawn_stop`，与既有 `spawn_proxy` helper 并列于文件顶部，两个调用点各缩为 1 行。行为零变化：相同参数、相同 env、相同 Stdio 配置。
2. **tests/proxy_contract.rs — 场景①改用 `spawn_stop(&sock)`**：保留原有界 `try_wait` 轮询（4s 硬上限）——这是挂起保护的承重结构（对可能挂起的子进程用 `.output()` 会无限阻塞），轮询循环、断言、计时原样保留。
3. **tests/proxy_contract.rs — 场景②`.output()` → `spawn_stop(&absent).wait_with_output()`**：std 中 `Command::output()` 与 `Child::wait_with_output()` 语义等价（piped 收集 + wait + 读到 EOF），对快速退出的 absent-socket 场景行为零变化。

其余观察（未改动，含理由）：

- **断言强度与超时契约逐条核对（未削弱）**：无响应 → `!status.success()`（非 0）、`elapsed <= 2500ms`（≤2.5s）、`stderr.contains("Error")`（stderr 报错）、`!stderr.contains("Proxy is not running.")`（不误报 not running）、`stdout.is_empty()`（错误路径不打印成功消息）；absent → `output.status.success()`（exit 0）、`elapsed < 1s`（快速）、stdout 含 "Proxy is not running."（成功消息）。8 条断言全部与契约一一对应，无一条被削弱或放宽。
- **`stderr.contains("Error")` 保留**：该断言是自有二进制（shutdown_proxy 错误传播）的消息形状，非外部工具输出，跨平台稳定，符合 assert-contracts-not-incidental-platform-strings 规则；契约本身要求"stderr 报错"，此即其直接表达。
- **hold 线程 + release channel 结构保留**：accept 后不回包、由主线程释放，是"模拟挂死 proxy 控制通道"的最小实现；panic 路径下 hold 线程阻塞在 `release_rx.recv()` 不影响测试结果（harness 报告 panic），无需 Drop 守卫。
- **4s 轮询上限与 2.5s 契约断言是两层有界，未合并**：轮询上限防止测试自身无限阻塞，契约断言表达超时要求，职责分离明确。
- **两处 `String::from_utf8_lossy` 转换与 `Instant::now()` 计时未提取**：各属不同场景的作用域，提取为共享 helper 反而降低局部可读性，净收益为负。
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
    Finished `test` profile [unoptimized + debuginfo] target(s) in 6.10s
     Running tests/proxy_contract.rs (target/debug/deps/proxy_contract-37538df7d2e1326e)

running 6 tests
test smoke_stub_receives_request ... ok
test stub_forwarding_with_bearer ... ok
test stop_times_out_on_unresponsive_socket ... ok
test concurrent_control_and_http ... ok
test log_masks_api_key_upstream_error ... ok
test log_masks_api_key ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.59s

EXIT_CODE=0
```
