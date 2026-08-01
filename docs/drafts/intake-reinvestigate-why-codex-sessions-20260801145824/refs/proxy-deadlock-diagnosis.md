# 诊断取证：cct proxy 启动即"半死"（runtime 死锁）

调查日期：2026-08-01。来源：本地实测（macOS，codex-cli 0.146.0）+ 源码审阅。

## 结论（一句话）

**`src/proxy.rs` 的控制 socket 用标准库同步阻塞 `std::os::unix::net::UnixListener` 跑在 tokio `current_thread` runtime 上；`run_control_socket` 任务一旦被 poll，`accept()` 同步阻塞整个线程，runtime 死锁，TCP HTTP 服务（主 future）永不被调度。proxy 从启动起就"半死"：端口在监听、连接能建立，但任何 HTTP 请求无限挂起。**

## 用户现象

用 cct 启动 codex（proxy 模式）→ codex 连上 proxy（TCP 连接建立）→ **无限卡住，等待第一个 Response**。

## 证据链

### 1. 源码（src/proxy.rs，HEAD）

```rust
use std::os::unix::net::{UnixListener, UnixStream};   // ← std 同步阻塞 listener

pub fn run_foreground(port: u16) -> io::Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()   // ← current_thread runtime
        .enable_all().build().expect("build proxy tokio runtime");
    rt.block_on(run_proxy(port, &socket_path));
    Ok(())
}

async fn run_proxy(port: u16, socket_path: &Path) {
    ...
    tokio::spawn(async move { run_control_socket(ctl_listener, ctl_state).await; ... });
    let listener = TcpListener::bind(&addr).await...;   // tokio 异步 listener
    loop { let (stream, _) = listener.accept().await...; tokio::spawn(...serve_connection...); }
}

async fn run_control_socket(listener: UnixListener, state: Arc<ProxyState>) {
    for stream in listener.incoming() {   // ← std 同步阻塞迭代器，async 上下文中阻塞线程
        ... spawn_blocking(handle_control) ...
    }
}
```

死锁机制：block_on 驱动主 future（TCP accept loop）至 pending 后，调度队列中的 `run_control_socket` 任务被 poll → `listener.incoming().next()` 同步 `accept()` → **阻塞整个线程（含 runtime 调度器）** → TCP accept loop 与所有连接处理任务永不再被调度。控制 socket 无连接时，线程永久卡在 `__accept`。

### 2. 实测（本机运行中的旧 proxy，PID 29182，14:42 启动）

- `lsof -iTCP:19191`：端口 LISTEN（fd 10），进程 `~/.local/bin/cct proxy start`
- `sample 29182`（curl 挂起期间采样 1718 样本）：**100% 在 `__accept`（UnixListener::accept）**——控制 socket 的同步阻塞 accept 占死线程
- `netstat -an | grep 19191`：**ESTABLISHED 连接存在但无人处理；多个 CLOSE_WAIT 残留**（应用层从不关闭连接）
- `curl --noproxy '*' http://127.0.0.1:19191/v1/models`：8s 超时无响应（TCP 握手成功，应用无响应）

### 3. 附带缺陷：健康检查误判

`check_proxy_running` 用 `UnixStream::connect(proxy.sock)`——TCP 握手由内核完成，死 proxy 也能连上 → 返回 true → cct 复用死 proxy，永不重启。用户第二次起的 proxy 因 19191 被占用直接 panic（`Address already in use`）。

### 4. 引入时间

- `b18cc4e`（2026-07-13）引入 proxy 架构时即含此代码；`17912cc`（同日）仅改流式转发
- 存在于 HEAD 与用户安装的 `~/.local/bin/cct`

## 修复方向（供后续 debate/tdd，本轮不实施）

1. **最小修复**：控制 socket 改用 `tokio::net::UnixListener`（异步 accept），或把 `run_control_socket` 移到独立 `std::thread`（阻塞 accept 在该线程，不影响 runtime）
2. **健康检查硬化**：`check_proxy_running` 加应用层探测（如发送 `status` 控制命令并等待响应，带超时），死 proxy 时自动重启
3. **启动互斥**：`TcpListener::bind` 失败（端口被占）时，不应 panic，而是探测既有 proxy 是否健康/或优雅处理
4. 测试：stub 一个死 socket 场景 + 控制 socket 与 HTTP 并发请求契约测试（遵守 test-boundaries-with-stubs 规则）

## 对 intake 议题的影响

此 bug **阻塞了"同 provider 会话可见性"的用户验证前提**（用户原话：前提是"该 provider 下打开 codex 可以正常对话"）。修复 proxy 后，才能实测：两个同为 proxy（`model_provider=custom`）的 profile 会话在 `codex resume` 中是否互相可见。
