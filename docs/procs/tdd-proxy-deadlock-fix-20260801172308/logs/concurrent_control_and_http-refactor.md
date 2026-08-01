---
title: "concurrent_control_and_http — Refactor Phase"
brief: "concurrent_control_and_http — Refactor: exit 0"
doc_type: proc
created: 2026-08-01T18:21:30+0800
case: "concurrent_control_and_http"
phase: refactor
---
Changes made:

1. **src/proxy.rs — `handle_request` 404 条件去掉冗余子句**：`req.uri().path().is_empty() || !req.uri().path().starts_with("/v1")` → `!req.uri().path().starts_with("/v1")`。空串不可能 `starts_with("/v1")`，`is_empty()` 子句被第二个条件蕴含，逻辑上纯冗余；删除后对空 path 的请求仍走同一 404 分支。
2. **src/proxy.rs — 提取 `error_response(message: impl Into<String>) -> ControlResponse` helper**：`handle_control` 中三处重复构造 `ControlResponse { status: "err", message: Some(...), base_url: None, model: None }`（empty command / invalid JSON / unknown command 三个分支）改为 `error_response(...)` 单行调用，净删约 20 行。行为零变化：同一 status 字面量、同一 message 内容、同一 None 字段。三处重复达到 KISS"三处才提取"阈值，helper 语义自明。
3. **src/proxy.rs — `run_control_socket` 命名统一**：`let st = state.clone()` → `let ctl_state`（两处），与 `run_proxy` 中同一模式（`ctl_state`）命名一致，消除 2 字母缩写。
4. **tests/proxy_contract.rs — 两处 stale doc comment 修正**：`smoke_stub_receives_request` 与 `concurrent_control_and_http` 的文档注释仍把"控制 socket 同步阻塞 accept 饿死 current_thread runtime"描述为"当前实现"（Red 阶段遗留，Green 已改为异步 accept，注释与代码事实不符）。改写为回归守卫语义：smoke 注释改为"HTTP 转发契约由 concurrent_control_and_http 覆盖"；AC1 注释改为"若控制 socket 的 accept 退化为同步阻塞，将饿死 runtime，HTTP 请求挂起 → 读超时 → 本测试失败（修复前状态）"。纯注释改动，无行为变化。

其余观察（未改动，含理由）：

- **不可达但防御性的 socket 清理**：`run_proxy` 中控制任务 `run_control_socket(...).await` 之后的 `let _ = std::fs::remove_file(&ctl_path)` 当前不可达（accept 循环在错误时 sleep 100ms 重试、永不返回）。但它是防御性清理（若未来循环有退出路径则保证 socket 文件不残留），且下次启动时 `remove_file`（bind 前）已覆盖陈旧文件场景，删除属判断偏好而非简化，保留。
- **runtime 构建重复**：`start_proxy` 与 `run_foreground` 各有一段相同的 current_thread runtime 构建块（2 处），低于 KISS"三处重复才提取"阈值，未提取。
- **config_dir 表达式重复**：`proxy_socket_path` / `proxy_log_path` 重复 `dirs::config_dir().unwrap_or_else(...).join("cc-tui")`（2 处），低于阈值，未提取。
- **两个 accept 循环不合并**：HTTP（TCP、`tokio::spawn`）与 control（Unix、`into_std` + `spawn_blocking`）循环结构相似但协议与任务模型不同，抽象成共享 loop 会降低清晰度，保持现状。
- **异步 accept 语义未变弱**：`run_control_socket` 的 accept 错误 → 100ms sleep 重试、`into_std` 失败 → continue 的流程原样保留；HTTP accept 错误 → continue 原样保留。
- **`PROBE_RETRIES` 常量仍未使用**：plan 后续步骤消费（先前 refactor 已注明），非本次范围。
- **保持显式的两处**：`mask_ctl_line` 的 `match` 形态、status 分支 `if is_empty { None } else { Some(clone) }` 均为显式写法，优于 `Some(x).filter(...)` 类紧凑技巧，未改动。
test_cmd exit code: 0
output: 按任务要求执行 `cargo test --test proxy_contract`（工作树根目录；为记录完整输出使用 `rtk proxy cargo test --test proxy_contract` 原始模式运行，语义等价），完整输出如下

```
$ rtk proxy cargo test --test proxy_contract; echo "EXIT_CODE=$?"
    Blocking waiting for file lock on artifact directory
   Compiling ring v0.17.14
   Compiling rustls v0.23.41
   Compiling rustls-webpki v0.103.13
   Compiling tokio-rustls v0.26.4
   Compiling hyper-rustls v0.27.9
   Compiling reqwest v0.12.28
   Compiling cct v0.5.0 (/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 7.93s
     Running tests/proxy_contract.rs (target/debug/deps/proxy_contract-37538df7d2e1326e)

running 2 tests
test smoke_stub_receives_request ... ok
test concurrent_control_and_http ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s

EXIT_CODE=0
```
