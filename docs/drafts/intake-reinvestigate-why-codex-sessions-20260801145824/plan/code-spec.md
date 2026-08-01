---
title: "Code Spec: cct proxy 死锁修复 + Codex 会话可见性验证与文档收尾"
doc_type: proc
brief: "Step-by-step implementation across 4 groups: G1 proxy 修复 → G2 契约测试 → G3 L2 实测 → G4 文档收尾"
confidence: verified
yields_from:
  - spec.md
created: 2026-08-01
updated: 2026-08-01
revision: 1
---

# Code Spec

**上下文**：接受准则与硬约束见 [constraints.md](constraints.md)；领域术语见 [domain-knowledge.md](domain-knowledge.md)；依赖 DAG 见 [architecture.md](architecture.md)。本文件不复述约束，步骤引用其编号。

## Files Changed

| File | Change Type | Group |
|------|-------------|-------|
| src/proxy.rs | Major edit | G1 |
| src/launch.rs | Major edit | G1 |
| tests/proxy_contract.rs | New file | G2 |
| tests/launch_proxy_contract.rs | New file | G2 |
| src/proxy.rs `#[cfg(test)]` | Major edit（新增测试） | G2 |
| docs/references/install-script.md | Minor edit | G4 |
| CLAUDE.md / ARCHITECTURE.md / docs/modules/launch.md / docs/references/codex-home-storage-layout.md / docs/references/codex-backend-development-guide.md | Major edit | G4 |
| poc/scripts/* | 只执行（含 run-all.sh） | G3 |

---

# Group: G1 — Proxy 死锁修复

**Checkpoint（组内输入）**：约束 #1–9、#14、#15（constraints.md）；domain-knowledge 的"生命周期/启动顺序/socket 清理责任/探测常量"业务规则。

## Step 1 — proxy_socket_path() 支持 CCT_PROXY_SOCKET 覆盖

**File**: `src/proxy.rs`
**What**: 仿 CCT_CONFIG 先例，env 优先（约束 #8）。

**Old**:
```rust
pub fn proxy_socket_path() -> PathBuf {
    dirs::config_dir()
```
**New**:
```rust
pub fn proxy_socket_path() -> PathBuf {
    if let Ok(p) = std::env::var("CCT_PROXY_SOCKET") {
        return PathBuf::from(p);
    }
    dirs::config_dir()
```
**Verify**: 在 `#[cfg(test)]` 中新增 `proxy_socket_path_override` 测试（set_var 临时路径 → 返回该路径，remove_var 后还原）；`cargo test proxy_socket_path`

## Step 2 — 控制 socket 异步 accept（死锁修复核心）

**File**: `src/proxy.rs`
**What**: `run_control_socket` 从 `std::os::unix::net::UnixListener` 同步 `incoming()` 阻塞（饿死 current_thread runtime）改为 `tokio::net::UnixListener` 异步 accept + `into_std()` + `spawn_blocking`（约束 #1）。本步仅做异步化，bind 失败行为保持现状（Step 6 处理）。

**Old**:
```rust
use std::os::unix::net::{UnixListener, UnixStream};
```
```rust
async fn run_control_socket(listener: UnixListener, state: Arc<ProxyState>) {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let st = state.clone();
                tokio::task::spawn_blocking(move || handle_control(stream, st));
            }
            Err(e) => {
                eprintln!("[cct-proxy] control socket accept error: {e}");
                break;
            }
        }
    }
}
```
**New**:
```rust
use std::os::unix::net::UnixStream;
use tokio::net::UnixListener as TokioUnixListener;
```
```rust
async fn run_control_socket(listener: TokioUnixListener, state: Arc<ProxyState>) {
    loop {
        match listener.accept().await {
            Ok((stream, _peer)) => {
                let st = state.clone();
                let std_stream = match stream.into_std() {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("[cct-proxy] control stream into_std error: {e}");
                        continue;
                    }
                };
                tokio::task::spawn_blocking(move || handle_control(std_stream, st));
            }
            Err(e) => {
                eprintln!("[cct-proxy] control socket accept error: {e}");
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
    }
}
```
同时将 `run_proxy` 内绑定改为 tokio 版本：
**Old**: `let ctl_listener = UnixListener::bind(socket_path).expect("bind proxy control socket");`
**New**: `let ctl_listener = TokioUnixListener::bind(socket_path).expect("bind proxy control socket");`

**Verify**: `cargo build` 通过；`cargo test`（现有 7 个 proxy 单测 + 全量）全绿

## Step 3 — 应用层健康探测常量与 check_proxy_running 升级

**File**: `src/proxy.rs`
**What**: 新增 500ms × 3 / 2s 常量；`check_proxy_running` 从内核 connect 升级为发送 `status` 命令并等待响应（应用层探测，死 proxy 连上但无响应 ≠ 健康）。`send_control` 增加带超时内部变体，公共 `send_control` 保持原签名（内部转调，接口冻结约束 #9）。

**Old**:
```rust
/// Check whether the proxy daemon is currently running.
pub fn check_proxy_running(socket_path: &Path) -> bool {
    UnixStream::connect(socket_path).is_ok()
}
```
**New**:
```rust
pub const PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(500);
pub const PROBE_RETRIES: u32 = 3;
pub const STOP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

/// Check whether the proxy daemon is healthy — application-level probe:
/// sends `status` over the control socket and expects a response within
/// PROBE_TIMEOUT. (Kernel-level connect alone cannot detect a dead proxy.)
pub fn check_proxy_running(socket_path: &Path) -> bool {
    let cmd = ControlCommand {
        cmd: "status".into(),
        base_url: None,
        api_key: None,
        model: None,
    };
    send_control_timeout(socket_path, &cmd, PROBE_TIMEOUT).is_ok()
}
```
`send_control` 改为转调（保持公共签名）：
```rust
pub fn send_control(socket_path: &Path, cmd: &ControlCommand) -> io::Result<ControlResponse> {
    send_control_timeout(socket_path, cmd, PROBE_TIMEOUT)
}

fn send_control_timeout(
    socket_path: &Path,
    cmd: &ControlCommand,
    timeout: std::time::Duration,
) -> io::Result<ControlResponse> {
    let mut stream = UnixStream::connect(socket_path)?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    // …原有 write payload + shutdown + read_line 逻辑不变…
}
```
（`control_command_parse_*` 等既有测试不受影响；`send_control_timeout` 需被后续步骤与测试使用，设为 `pub(crate)` 或保持私有并仅内部调用。）

**Verify**: `cargo test` 全绿；新增单测：不存在路径 → `check_proxy_running` false；对线程应答的临时 socket → true

## Step 4 — ensure_proxy_running 重写（父进程：探测→试探 bind→spawn→就绪探测）

**File**: `src/launch.rs`
**What**: 僵尸自愈 + 占端口报错的父进程路径（约束 #2、#3、#4、#8）：应用层探测失败后试探 bind 判端口空闲（先 drop 再 spawn），spawn 目标经 `CCT_PROXY_BIN` 注入，就绪探测复用 500ms×3 常量，耗尽明确报错。

**Old**（`src/launch.rs:132-163`）:
```rust
pub fn ensure_proxy_running(_port: u16, socket_path: &Path) -> Result<()> {
    if crate::proxy::check_proxy_running(socket_path) {
        return Ok(());
    }
    let exe = std::env::current_exe().context("cannot find own executable")?;
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("proxy")
        .arg("start")
```
**New**:
```rust
pub fn ensure_proxy_running(port: u16, socket_path: &Path) -> Result<()> {
    if crate::proxy::check_proxy_running(socket_path) {
        return Ok(());
    }
    // 端口空闲判定：试探 bind（先 drop 再 spawn），避免 TOCTOU 竞态下
    // unlink 并发启动的活 proxy。探测失败 + 端口被占 → 报错退出（lsof 诊断）。
    if std::net::TcpListener::bind(("127.0.0.1", port)).is_err() {
        anyhow::bail!("{}", crate::proxy::port_conflict_message(port));
    }
    let exe = std::env::var("CCT_PROXY_BIN").map(String::from).unwrap_or_else(|_| {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.to_str().map(String::from))
            .unwrap_or_else(|| "cct".to_string())
    });
    let mut cmd = std::process::Command::new(&exe);
    cmd.arg("proxy")
        .arg("start")
```
就绪轮询段（原 5s/100ms 循环）替换为：
**Old**:
```rust
    // Wait up to 5s for the socket to appear.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        if crate::proxy::check_proxy_running(socket_path) {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    anyhow::bail!("proxy did not start within 5 seconds")
```
**New**:
```rust
    // 就绪探测：复用 PROBE_TIMEOUT × PROBE_RETRIES 常量。
    for _ in 0..crate::proxy::PROBE_RETRIES {
        if crate::proxy::check_proxy_running(socket_path) {
            return Ok(());
        }
        std::thread::sleep(crate::proxy::PROBE_TIMEOUT);
    }
    anyhow::bail!("proxy did not become healthy after {} probes", crate::proxy::PROBE_RETRIES)
```
（注：`CCT_PROXY_BIN` 取 `String`；`current_exe()` 失败（ok 链）或路径非 UTF-8（to_str None）时回退 `"cct"`——不传播错误，spawn 失败由 `cmd.spawn().context(...)` 兜底。）

**Verify**: `cargo build` + `cargo test`；G2 Step 12 launch 契约测试覆盖

## Step 5 — 占端口诊断辅助（只读 lsof + 降级文本）

**File**: `src/proxy.rs`
**What**: `lsof -tiTCP:<port> -sTCP:LISTEN` 单次调用取占用者 PID；lsof 缺失/失败/无输出 → None → 降级为定位命令建议文本（约束 #4）。

**New**（`handle_request` 之后、`run_control_socket` 之前新增）:
```rust
/// Read-only diagnosis: PID listening on `port` via lsof. Returns None when
/// lsof is unavailable or nothing is listening (caller falls back to advice text).
pub fn tcp_port_owner(port: u16) -> Option<String> {
    let out = std::process::Command::new("lsof")
        .args([format!("-tiTCP:{port}"), "-sTCP:LISTEN".to_string()])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8(out.stdout)
        .ok()?
        .lines()
        .next()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Port-conflict error message for report-only display. Never kills anything.
pub fn port_conflict_message(port: u16) -> String {
    match tcp_port_owner(port) {
        Some(pid) => format!(
            "port {port} already in use by PID {pid} (lsof -tiTCP:{port} -sTCP:LISTEN). \
             若为旧版本遗留实例或第三方进程, 手动终止后重试; cct 不会自动终止进程."
        ),
        None => format!(
            "port {port} already in use. 运行 `lsof -iTCP:{port}` 查看占用者."
        ),
    }
}
```
**Verify**: 新增单测（构造 PATH 无 lsof 的场景 → `tcp_port_owner` None → `port_conflict_message` 含 "lsof -iTCP" 建议文本）；`cargo test tcp_port_owner`

## Step 6 — run_proxy 启动：先探测再删 + bind 失败报错（TCP + 控制 socket EADDRINUSE）

**File**: `src/proxy.rs`
**What**: 子进程启动时序（约束 #3、#5）：有活 proxy → 报错退出不破坏其控制通道；探测失败才删 socket（先探测再删）；控制 socket EADDRINUSE → 视为已有实例重新探测 3 次 × 500ms，耗尽报错退出；TCP bind 失败 → lsof 诊断文本报错。均非 panic。

**Old**:
```rust
    let _ = std::fs::remove_file(socket_path);

    let ctl_listener = TokioUnixListener::bind(socket_path).expect("bind proxy control socket");
    log_proxy!("control socket bound");
```
**New**:
```rust
    // 先探测再删：有活 proxy → 报错退出，不破坏其控制通道（约束 #5）。
    if crate::proxy::check_proxy_running(socket_path) {
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
            for _ in 0..crate::proxy::PROBE_RETRIES {
                if crate::proxy::check_proxy_running(socket_path) {
                    eprintln!(
                        "[cct-proxy] another live proxy owns control socket {socket_path:?} — exiting"
                    );
                    std::process::exit(1);
                }
                std::thread::sleep(crate::proxy::PROBE_TIMEOUT);
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
TCP bind 段：
**Old**:
```rust
    let addr = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("proxy bind {addr}: {e}"));
```
**New**:
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
**Verify**: `cargo build`；G2 Step 11 占端口/双启动契约测试；PoC B003/B009 脚本

## Step 7 — shutdown 命令退出前清理 socket 文件

**File**: `src/proxy.rs`
**What**: 修复"每次 stop 留下死 socket"稳态缺陷（约束 #6）。`handle_control` 增加 socket 路径参数，shutdown 分支写响应后删文件再 exit。

**Old**:
```rust
async fn run_control_socket(listener: TokioUnixListener, state: Arc<ProxyState>) {
```
```rust
                tokio::task::spawn_blocking(move || handle_control(std_stream, st));
```
**New**:
```rust
async fn run_control_socket(listener: TokioUnixListener, state: Arc<ProxyState>, socket_path: PathBuf) {
```
```rust
                let sp = socket_path.clone();
                tokio::task::spawn_blocking(move || handle_control(std_stream, st, sp));
```
（`run_proxy` 内调用点同步传 `socket_path.to_path_buf()`。）

**Old**:
```rust
fn handle_control(mut stream: UnixStream, state: Arc<ProxyState>) {
```
**New**:
```rust
fn handle_control(mut stream: UnixStream, state: Arc<ProxyState>, socket_path: PathBuf) {
```
shutdown 分支：
**Old**:
```rust
            std::process::exit(0);
```
（"shutdown" 分支内）
**New**:
```rust
            let _ = std::fs::remove_file(&socket_path); // 退出前清理死 socket 缺陷
            std::process::exit(0);
```
**Verify**: G2 契约测试：启动 → shutdown → 断言 socket 文件不存在

## Step 8 — 控制命令与请求日志 api_key 脱敏

**File**: `src/proxy.rs`
**What**: 控制命令日志（switch 含 api_key 明文）与请求日志 path 打印前脱敏（约束 #7，mask-secrets-on-every-display-path）。控制命令为结构化 JSON——**按字段名脱敏**（`api_key` 值无条件掩码，不依赖值形态/前缀）；请求日志 path 无字段名可依，按 `sk-` 前缀值扫描兜底。两 helper 集中在本模块，不散落多处 open-code。

**New**（`write_control_response` 之前新增 2 个 helper）:
```rust
/// Redact the api_key field value from a control-command JSON line.
/// Field-name based（约束 #7）：任何 api_key 值均掩码，不依赖 sk- 前缀。
fn mask_ctl_line(line: &str, api_key: Option<&str>) -> String {
    match api_key {
        Some(key) if !key.is_empty() => line.replace(key, "***"),
        _ => line.to_string(),
    }
}

/// Redact sk-... secret values from a request path/query log line
///（请求日志无字段名可依，值前缀扫描兜底）。
fn mask_request_path(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(b"sk-") {
            i += 3;
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_')
            {
                i += 1;
            }
            out.push_str("sk-***");
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}
```
应用点（2 处，利用已解析的 `cmd.api_key`）：
**Old**:
```rust
    log_proxy!("ctl << {}", line.trim());
```
**New**:
```rust
    log_proxy!("ctl << {}", mask_ctl_line(line.trim(), cmd.api_key.as_deref()));
```
**Old**: `log_proxy!("<< {method} {path_and_query}");`
**New**: `log_proxy!("<< {method} {}", mask_request_path(&path_and_query));`

**Verify**: 新增单测——`mask_ctl_line`（含 `sk-abc123` 与不含 sk- 前缀的 `custom-token-xyz` 两种 api_key 均被掩码为 `***`）；`mask_request_path`（`?key=sk-xyz` → 含 `sk-***` 无明文）；`cargo test mask_`；G2 Step 11 脱敏契约

## Step 9 — shutdown_proxy stop 2s 超时返回错误 + main.rs stop_proxy 区分"无 socket / 无响应"

**File**: `src/proxy.rs`、`src/main.rs`
**What**: 无响应 proxy 上 `cct proxy stop` 在 STOP_TIMEOUT（2s）后返回错误而非永久挂起（约束 #1、#10 覆盖项）。`shutdown_proxy` 不再吞错。**配套**：`main.rs stop_proxy` 把"socket 文件不存在"（快速 exit 0 "not running"）与"socket 存在但无响应"（2s 超时报错非 0，死锁进程仍持端口时不得误报 not running）区分开——spec AC 的超时契约经二进制路径可交付。

**Old**:
```rust
pub fn shutdown_proxy(socket_path: &Path) -> io::Result<()> {
    let cmd = ControlCommand {
        cmd: "shutdown".into(),
        base_url: None,
        api_key: None,
        model: None,
    };
    let _ = send_control(socket_path, &cmd);
    Ok(())
}
```
**New**:
```rust
pub fn shutdown_proxy(socket_path: &Path) -> io::Result<()> {
    let cmd = ControlCommand {
        cmd: "shutdown".into(),
        base_url: None,
        api_key: None,
        model: None,
    };
    let resp = send_control_timeout(socket_path, &cmd, STOP_TIMEOUT)?;
    if resp.status == "ok" {
        Ok(())
    } else {
        Err(io::Error::other(
            resp.message.unwrap_or_else(|| "unknown error".into()),
        ))
    }
}
```
`main.rs stop_proxy` 段：
**Old**:
```rust
fn stop_proxy() -> Result<()> {
    let socket_path = proxy::proxy_socket_path();
    if !proxy::check_proxy_running(&socket_path) {
        println!("Proxy is not running.");
        return Ok(());
    }
    proxy::shutdown_proxy(&socket_path)?;
    println!("Proxy shut down.");
    Ok(())
}
```
**New**:
```rust
fn stop_proxy() -> Result<()> {
    let socket_path = proxy::proxy_socket_path();
    // 无 socket 文件 = 无实例 → 快速 exit 0；socket 存在但无响应 →
    // shutdown_proxy 2s 超时传播错误（死锁进程持端口时不得误报 not running）。
    if !socket_path.exists() {
        println!("Proxy is not running.");
        return Ok(());
    }
    proxy::shutdown_proxy(&socket_path)?;
    println!("Proxy shut down.");
    Ok(())
}
```
**Verify**: G2 Step 11 stop-超时契约（无响应控制 socket → 2s 内返回 Err 而非挂起；socket 不存在 → exit 0）；`cargo test`

## Group: G1 — 完成

**产出清单（供 G2 使用）**：`src/proxy.rs` 完成 CCT_PROXY_SOCKET 覆盖、异步 accept、应用层探测（PROBE_TIMEOUT×3 / STOP_TIMEOUT）、lsof 诊断辅助（`tcp_port_owner`/`port_conflict_message`）、先探测再删、EADDRINUSE 重新探测、shutdown 清理 socket、日志脱敏（`mask_sensitive`）、stop 2s 超时；`src/launch.rs` `ensure_proxy_running` 重写（试探 bind 判定 + CCT_PROXY_BIN 注入 + 就绪探测）。

---

# Group: G2 — 契约测试

**Checkpoint（G1 产出）**：proxy.rs 新接口 `PROBE_TIMEOUT`/`PROBE_RETRIES`/`STOP_TIMEOUT`、`send_control_timeout`、`tcp_port_owner`、`port_conflict_message`、`mask_sensitive`、`handle_control(stream, state, socket_path)`；launch.rs `ensure_proxy_running(port, socket_path)`（CCT_PROXY_BIN 注入、试探 bind、就绪探测）。

**组内输入**：约束 #1–5、#9、#10、#14；dev-deps 已有 `tempfile`/`serial_test`；lib target 存在（`cct::proxy`/`cct::launch` 可直接调用）；`CARGO_BIN_EXE_cct` 可用。

## Step 10 — tests/proxy_contract.rs 基础设施：stub 上游 + 动态端口 + 临时 socket 启动器

**File**: `tests/proxy_contract.rs`（New file）
**What**: 协议无关 stub 上游（std TcpListener + thread，记录请求到共享 Vec，对 /v1/* 返回 SSE 流）；动态端口获取（bind 0 取端口后 drop）；`spawn_proxy` helper 用 `CARGO_BIN_EXE_cct` + env（CCT_PROXY_SOCKET 临时路径 / CCT_PROXY_PORT 动态端口 / CCT_PROXY_LOG 临时日志）启动真实 proxy。

**New**（文件骨架，完整内容按此展开）:
```rust
//! Proxy 契约测试：真实二进制 + 临时 socket + 动态端口（约束 #10）。
//! 与用户实例隔离：全部走 CCT_PROXY_SOCKET / CCT_PROXY_PORT env。

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cct::proxy::{ControlCommand, ControlResponse, PROBE_TIMEOUT, STOP_TIMEOUT};
use serial_test::serial;
use tempfile::TempDir;

/// 协议无关 stub 上游：记录 (method, path, authorization)，SSE 流式返回。
struct StubUpstream {
    log: Arc<Mutex<Vec<(String, String, String)>>>,
    port: u16,
}

impl StubUpstream {
    fn start() -> Self { /* TcpListener::bind(("127.0.0.1", 0)) + thread accept loop;
                           每连接读首行请求行 + Authorization header；
                           响应: 200 + content-type text/event-stream +
                           "event: response.created\n..." + "event: response.completed\n" + fixed DELTA */
    }
    fn requests(&self) -> Vec<(String, String, String)> { self.log.lock().unwrap().clone() }
}

fn free_port() -> u16 { /* bind 0 → local_addr().port() → drop */ }

fn spawn_proxy(sock: &std::path::Path, port: u16) -> std::process::Child {
    let bin = env!("CARGO_BIN_EXE_cct");
    std::process::Command::new(bin)
        .args(["proxy", "start"])
        .env("CCT_PROXY_SOCKET", sock)
        .env("CCT_PROXY_PORT", port.to_string())
        .env("CCT_PROXY_LOG", "1")
        .stderr(std::process::Stdio::piped()) // 捕获 stderr 断言脱敏（log_proxy! 走 eprintln）
        .spawn()
        .expect("spawn cct proxy")
}
```
（注：CCT_PROXY_LOG 开启时 `log_proxy!` 宏输出到进程 stderr（`eprintln!`）；测试经 `Stdio::piped()` 捕获 stderr 行断言脱敏。`ensure_proxy_running` 内把 stderr 重定向到 `proxy_log_path()` 是父进程 spawn 路径的行为，契约测试直接 spawn 二进制不经过它——**不新增 CCT_PROXY_LOG_PATH 接口**（约束 #9 接口冻结）。）
**Verify**: `cargo test --test proxy_contract` 编译通过（空骨架 + 1 个 smoke 测试：stub 收到请求）

## Step 11 — tests/proxy_contract.rs：7 个行为契约（并发/转发/脱敏/stop 超时/僵尸/占端口/双启动）

**File**: `tests/proxy_contract.rs`（续）
**What**: 每个行为一个 `#[test] #[serial]` 测试（env 隔离）。全部用临时 socket + 动态端口。

**New**:
```rust
#[test]
#[serial]
fn concurrent_control_and_http() {
    // 起 proxy → 线程 A 循环 send status（≥20 次）+ 主线程并发 HTTP GET /v1/models
    // → 全部在 3s 内完成（死锁回归，约束 #1）
}

#[test]
#[serial]
fn stub_forwarding_with_bearer() {
    // stub 上游 → send switch(base_url=stub, api_key="sk-contract-key") → HTTP POST /v1/chat
    // → stub 记录含 Bearer sk-contract-key + 响应体含固定 DELTA（约束 #4）
}

#[test]
#[serial]
fn log_masks_api_key() {
    // CCT_PROXY_LOG=1 + 捕获 stderr → switch(sk-contract-key) + HTTP 请求
    // → 断言 stderr 不含 "sk-contract-key"（约束 #7，mask-secrets-on-every-display-path）
}

#[test]
#[serial]
fn stop_times_out_on_unresponsive_socket() {
    // ① socket 文件存在 + thread 接受连接但不回包（无响应控制 socket）→
    //    spawn `cct proxy stop`（CCT_PROXY_SOCKET 临时路径）→ ≤2.5s 内退出且非 0，
    //    stderr 含错误（Step 9：main.rs stop_proxy 经 shutdown_proxy 2s 超时传播）
    // ② socket 文件不存在 → spawn `cct proxy stop` → 快速 exit 0 + "Proxy is not running."
}

#[test]
#[serial]
fn zombie_recovery_restarts_proxy() {
    // spawn_proxy → 等健康（check_proxy_running）→ kill 子进程（SIGKILL）
    // → socket 文件残留 → ensure_proxy_running(port, socket)（lib 直调）→ Ok
    // → check_proxy_running 为 true（约束 #2 自愈）
    // 注意：ensure_proxy_running 会 spawn current_exe()（测试二进制自身）——
    // 本测试须设 CCT_PROXY_BIN=env!("CARGO_BIN_EXE_cct") 指向真实 proxy 入口，
    // 与 Step 12 launch 契约同一注入约定（否则就绪探测耗尽、测试按原文失败）。
}

#[test]
#[serial]
fn port_occupied_reports_error_keeps_occupant() {
    // 测试进程先 bind 动态端口 → spawn_proxy 同端口 → 子进程退出码非 0
    // → stderr 含占用信息（lsof PID 或降级建议文本）→ 不 panic
    // → 占用者（测试自己的 listener）仍存活（约束 #3）
}

#[test]
#[serial]
fn double_start_race_one_wins() {
    // 同时 spawn 两个 proxy（同 socket + 同端口）→ 恰一个存活（check_proxy_running true）
    // → 另一个在 ≤2s 内退出非 0 且 stderr 无 "panic"（EADDRINUSE 重新探测收敛，约束 #5/#10）
}
```
**Verify**: `cargo test --test proxy_contract` 全绿；PoC B001/B002/B003/B009 前置脚本逻辑与其一致

## Step 12 — tests/launch_proxy_contract.rs：ensure_proxy_running 重启契约（CCT_PROXY_BIN fake）

**File**: `tests/launch_proxy_contract.rs`（New file）
**What**: launch 层重启路径（约束 #2、#8、#10）：fake spawn 目标经 CCT_PROXY_BIN 注入（仿 CCT_CLAUDE_BIN 先例），fake 经 CCT_PROXY_SOCKET 接收临时路径并应答 status。

**New**（fake 用测试内 shell 脚本文件 + lib 直调）:
```rust
//! ensure_proxy_running 重启契约（CCT_PROXY_BIN 注入 fake 目标）。

use std::process::Command;
use cct::launch::ensure_proxy_running;
use serial_test::serial;
use tempfile::TempDir;

/// fake spawn 目标：启动时 rm 残留 socket，循环 accept 并应答 {"status":"ok"}
fn write_fake_proxy(dir: &std::path::Path) -> std::path::PathBuf {
    let script = dir.join("fake-proxy.sh");
    std::fs::write(
        &script,
        r#"#!/bin/bash
set -e
SOCK="${CCT_PROXY_SOCKET:?}"
rm -f "$SOCK"
# 循环 accept：用 python3 或 bash /dev/tcp 应答 status
"#,
    )
    .unwrap();
    // chmod +x
    let mut perms = std::fs::metadata(&script).unwrap().permissions();
    use std::os::unix::fs::PermissionsExt;
    perms.set_mode(0o755);
    std::fs::set_permissions(&script, perms).unwrap();
    script
}

#[test]
#[serial]
fn spawns_fake_when_none_running() {
    // CCT_PROXY_BIN=fake 脚本 + CCT_PROXY_SOCKET 临时路径 + CCT_PROXY_PORT 动态端口
    // → ensure_proxy_running → Ok；fake 已启动（进程存活 or READY 标记文件存在）
}

#[test]
#[serial]
fn reuses_live_proxy() {
    // 先起 fake → ensure_proxy_running → Ok 且未再 spawn（进程数不变）
}

#[test]
#[serial]
fn zombie_socket_triggers_restart() {
    // 起 fake → kill → socket 残留 → ensure_proxy_running → 重新 spawn fake → Ok
}

#[test]
#[serial]
fn probe_exhaustion_reports_error() {
    // CCT_PROXY_BIN=立即退出的脚本（不监听）→ ensure_proxy_running
    // → Err（就绪探测 500ms×3 耗尽），≤2s 返回，不挂起
}

#[test]
#[serial]
fn port_occupied_bails_with_diagnosis() {
    // 测试进程 bind 动态端口 → ensure_proxy_running → Err 含 "port" 与占用信息
    // → 未 spawn 任何进程
}
```
**Verify**: `cargo test --test launch_proxy_contract` 全绿（5 个测试 ≤2s 各自完成）

## Step 13 — 配置快照回归（AC14：启动链路不写任何 Codex 配置文件）

**File**: `tests/proxy_contract.rs`（追加一个测试）
**What**: 临时 CODEX_HOME 下跑 proxy 启动 + switch 前置路径，断言目录内无 config.toml / auth.json / profile-*.config.toml（约束 #14）。

**New**:
```rust
#[test]
#[serial]
fn launch_path_writes_no_codex_config() {
    // TempDir: codex_home + profiles.toml
    // env: CODEX_HOME=codex_home, CCT_CONFIG=profiles.toml, CCT_PROXY_SOCKET/端口 临时
    // 经 launch::exec_codex 的 proxy 前置不可达（exec-replace）→ 改为：
    //   ensure_proxy_running + switch_profile（exec_codex_proxy 的 1-2 步等价路径）
    // 断言 codex_home 下递归 glob 无 config.toml / auth.json / profile-*.config.toml
    // （snapshot：写入前后文件集合一致）
}
```
**Verify**: `cargo test --test proxy_contract launch_path_writes_no_codex_config` 通过；`cargo test` 全量绿

## Step 14 — G2 收尾：全量测试 + clippy + PoC 冒烟前置脚本

**File**: （无）
**What**: 契约测试全部合入后的整体质量门（约束 #15 基线对比）。

**Verify**:
```bash
cargo test 2>&1 | tail -5                 # 全绿（含既有 integration/live 排除 CCT_LIVE_TESTS）
cargo clippy --all-targets 2>&1 | tail -5 # 无新增警告
bash poc/scripts/verify-B014-interface-frozen.sh  # PASS：接口冻结 + 快照回归
bash poc/scripts/verify-B010-contract-tests.sh     # PASS：cargo test 契约全绿
```

## Group: G2 — 完成

**产出清单（供 G3 使用）**：`tests/proxy_contract.rs`（7+1 行为契约）、`tests/launch_proxy_contract.rs`（5 重启契约）；Step 11 僵尸自愈/占端口/双启动、Step 12 重启、Step 13 快照全部可独立执行。

---

# Group: G3 — L2 实测

**Checkpoint（G1+G2 产出）**：proxy 修复已落地且有契约测试防护网；PoC 脚本（B001-B015）已在 `poc/`；修复前基线证据在 [refs/proxy-deadlock-diagnosis.md](../refs/proxy-deadlock-diagnosis.md)（死锁复现 curl 超时 + 采样）与 session-log（PID 29182 占端口、AC7 文档残留盘点）；4 个只读脚本（B011/B012/B013/B015）修复前 FAIL 已实测（结果未落盘，Step 15 补录进 Results Log）。

**组内输入**：约束 #4、#6、#7、#8、#12、#15；poc/poc.md 矩阵 + config.env.example；smoke 脚本已存在。

## Step 15 — 迁移前置 + 全量 PoC 运行

**File**: （执行操作）
**What**: 约束 #12 前置：旧实例 PID 29182 终止（spec AC11 明确唯一修复路径是用户手动终止旧进程——执行 `kill 29182` 前向用户确认；若控制 socket 仍响应，新版本探测会视为健康复用）+ 端口 19191 释放；然后修复后全量跑 PoC。

**Steps**:
0. 补录基线行：poc.md Results Log 增加一行"修复前基线（只读脚本 B011/B012/B013/B015 FAIL；证据 refs/proxy-deadlock-diagnosis.md + session-log）"
1. 终止 PID 29182（`kill 29182`；先 `ps -p 29182` 确认仍是旧版 cct proxy 实例再执行）
2. `bash poc/scripts/verify-B012-l2-prereqs.sh` → PASS（29182 已终止 + 端口空闲）
3. `cd poc && ./run-all.sh` → 修复后全量

**Verify**: `run-all.sh` 输出 `Total: 15 | Pass: 15 | Fail: 0 | Skip: 0`；B001（死锁回归）/B002（僵尸自愈）/B003（占端口报错）/B005（脱敏）/B009（双启动）从修复前 FAIL 转 PASS

## Step 16 — B006-B008 会话可见性结果判定（条件分支）

**File**: （无）
**What**: 约束 #6/#7/#8 实测结果解读：B006 同 provider 可见、B007 跨 provider 不可见 + 显式恢复、B008 cwd 过滤 + `--all`。

**分支**：
- **B006/B007/B008 全 PASS** → 官方语义与 cct 链路一致，无 cct 层 bug；进入 Step 17。
- **任一 FAIL** → 实测不符 → 定义为 cct 层 bug（[spec.md](../spec.md) AC6 兜底条款）→ 追加修复任务（回到 G1/G2 模式补步），修复后重跑对应脚本。

**Verify**: `bash poc/scripts/verify-B006-same-provider-visible.sh && bash poc/scripts/verify-B007-cross-provider-invisible.sh && bash poc/scripts/verify-B008-cwd-filter.sh` 三者 PASS

## Step 17 — 分层诊断确认 + poc.md Results Log 落账

**File**: `poc/poc.md`
**What**: 约束 #15 分层诊断顺序确认（proxy 层 curl --noproxy '*' 先行 → 上游层 codex 对话）；结果写入 Results Log。

**Steps**:
1. `bash poc/scripts/verify-B015-layered-diag.sh` → PASS（proxy 层存活；修复前 FAIL 已复现死锁，修复后转 PASS 即分层证据）
2. 将 `run-all.sh` 输出填入 `poc/poc.md` Results Log 表（Date/Total/Pass/Fail/Skip/Notes），Notes 注明"修复后全量"

**Verify**: `poc/poc.md` Results Log 有当日记录行；B015 输出 `[PASS] B015: proxy 层存活`

## Step 18 — TUI picker 可视化确认（OQ3）🖐️ MANUAL（可选）

**File**: （无）
**What**: spec Open Question 3：修复后用户自愿打开 TUI 手动 `codex resume` 做一次可视化确认（agent 无法操作 TUI）。可选，不阻塞任何 AC。

**Steps**（用户自愿）:
1. 任一 profile 启动 codex（TUI 正常使用路径）
2. `codex resume` 打开 picker → 确认同 provider 会话可见、跨 provider 不可见

**Verify**: 用户视觉确认；结果可记录到 [poc.md](../poc/poc.md) Manual Checks（如执行）

## Group: G3 — 完成

**产出清单（供 G4 使用）**：迁移完成 + 全量 PASS 实测证据（poc.md Results Log）+ 会话可见性语义实测确认（B006-B008 PASS）+（可选）OQ3 TUI 可视化确认。

---

# Group: G4 — 文档收尾

**Checkpoint（G1 产出 + G3 结果）**：CCT_PROXY_SOCKET 已实现、shutdown 清理已实现、探测语义已落地（G1）；可见性实测确认（G3）；PoC B011/B013/B014 当前 FAIL（文档缺口基线，证据见 [refs/proxy-deadlock-diagnosis.md](../refs/proxy-deadlock-diagnosis.md) 与 session-log AC7 盘点）。

**组内输入**：约束 #11、#13、#14；`docs/` 目录结构；历史快照（session-cards / procs / context-*）不改动。

## Step 19 — install-script.md 迁移说明（AC11）

**File**: `docs/references/install-script.md`（Minor edit）
**What**: 一次性升级指引：旧版死锁实例若控制 socket 仍响应会被视为健康复用（HTTP 仍挂起，唯一修复路径是手动终止）；用户手动终止 + 删除遗留 socket 兜底（顺序无关均安全）；新版本不再产生死锁进程。

**New**（追加小节，标题建议 `## Upgrading from pre-fix versions (deadlock) 迁移说明`）:
```markdown
### 旧版死锁实例迁移（一次性）

旧版本（0.5.0 之前修复版）proxy 可能遗留死锁进程与死 socket 文件：

1. 若 `lsof -iTCP:19191` 显示 PID 仍存活 → `kill <PID>`（新版探测会将其视为健康并复用，
   唯一修复路径是手动终止旧进程）。
2. 遗留 socket 文件（`~/.config/cc-tui/proxy.sock`）→ 可手动删除兜底；新版本启动时
   探测失败会自动清理，删除顺序无关均安全。
3. 新版本不再产生死锁进程，此迁移一次性。
```
**Verify**: `bash poc/scripts/verify-B011-migration-docs.sh` → PASS（文档含 29182/手动终止/遗留 socket 说明）

## Step 20 — 五文档 AC13 清理：per-profile CODEX_HOME / generate_codex_config 陈旧叙述 + resume 过滤语义

**File**: `CLAUDE.md`、`ARCHITECTURE.md`、`docs/modules/launch.md`、`docs/references/codex-home-storage-layout.md`、`docs/references/codex-backend-development-guide.md`（Major edit）
**What**: 消除 per-profile CODEX_HOME 与 `generate_codex_config` 陈旧叙述；新增"resume 按 model_provider ∩ cwd 过滤"语义说明（约束 #13）。历史快照（session-cards / procs / context-*）不动。

**具体**（逐文件，先 grep 定位再改）:
1. 每文件 `grep -n "per-profile CODEX_HOME\|per profile CODEX_HOME\|generate_codex_config"` 定位陈旧叙述 → 改写为：CODEX_HOME 不设置（默认 `~/.codex`），所有 profile 共享；6 个 `--config` 旗标经 `build_codex_proxy_config_args` 注入（不再写 config.toml）
2. 在 codex 相关文档（codex-home-storage-layout.md + codex-backend-development-guide.md）新增语义段落：`codex resume` 仅列当前 `model_provider_id` ∩ 当前 cwd 的会话；`--all` 绕过 cwd 过滤但关不掉 provider 过滤；显式 `resume <session-id>` 可绕过全部过滤；同 provider 会话跨 profile 可见（物理共享 `~/.codex`）

**Verify**: `bash poc/scripts/verify-B013-doc-cleanup.sh` → PASS（5 文档零陈旧叙述 + resume 语义说明存在）

## Step 21 — 文档终审：B011/B013/B014 全 PASS

**File**: （无）
**What**: G4 收尾质量门：文档缺口全部补齐 + 接口冻结回归。

**Verify**:
```bash
bash poc/scripts/verify-B011-migration-docs.sh  # PASS
bash poc/scripts/verify-B013-doc-cleanup.sh     # PASS
bash poc/scripts/verify-B014-interface-frozen.sh # PASS
```

## Group: G4 — 完成

**产出清单**：install-script.md 迁移段落 + 5 文档陈旧叙述消除 + resume 过滤语义说明；B011/B013/B014 全 PASS。

---

# 终端步骤

## Step 22 — Proof-Read End-to-End

Read each changed file in full (`src/proxy.rs`、`src/launch.rs`、`tests/proxy_contract.rs`、`tests/launch_proxy_contract.rs`、`docs/references/install-script.md`、5 份文档改动段)。Check: formatting, no leftover TODOs, spec intent preserved, 与 [constraints.md](constraints.md) 逐条对照。

## Step 23 — Cross-Check Acceptance Criteria

| Criterion | Addressed in Step |
|-----------|------------------|
| AC1 并发响应（死锁回归） | Step 2 + Step 11 `concurrent_control_and_http` |
| AC2 僵尸自愈重启 | Step 3/4/6 + Step 11 `zombie_recovery_restarts_proxy` + Step 12 `zombie_socket_triggers_restart` + Step 15 B002 |
| AC3 占端口报错（lsof 诊断、不 kill） | Step 4/5/6 + Step 11 `port_occupied_reports_error_keeps_occupant` |
| AC4 stub 转发链路 | Step 11 `stub_forwarding_with_bearer` + Step 15 B004 |
| AC5 日志脱敏 | Step 8 + Step 11 `log_masks_api_key` |
| AC6 同 provider 可见 | Step 16 B006（无 cct 层 bug 时确认） |
| AC7 跨 provider 不可见 + 显式恢复 | Step 16 B007 |
| AC8 cwd 过滤 + --all | Step 16 B008 |
| AC9 活 proxy 双启动报错 + 复用 | Step 6 + Step 11 `double_start_race_one_wins` + Step 12 `reuses_live_proxy` |
| AC10 契约测试覆盖 7 场景 + 隔离 | Step 10/11/12 |
| AC11 迁移说明 | Step 19 + Step 15（手动迁移）+ Step 21 B011 |
| AC12 L2 前置条件 | Step 15（B012 预检 + 迁移） |
| AC13 五文档清理 + 语义说明 | Step 20 + Step 21 B013 |
| AC14 不写 Codex 配置 + 接口冻结 | Step 13 + Step 14/21 B014 |
| AC15 分层诊断 | Step 17 B015 |

## Step 24 — Review

Follow the self-review checklist in [verification.md](verification.md). Writes `review.md`（draft 目录下）。

## Step 25 — Commit

Use /commit. Suggested message:
```
fix(proxy): async accept, app-level probe, zombie heal, log mask
- async accept (tokio UnixListener + spawn_blocking) fixes current_thread deadlock
- app-level status probe (500ms×3) replaces kernel connect check
- zombie self-heal via parent bind-probe + child probe-then-unlink
- port-conflict diagnostics via read-only lsof (fallback advice text)
- CCT_PROXY_SOCKET / CCT_PROXY_BIN test injection env
- shutdown cleans up socket file; stop times out after 2s
- contract tests (proxy_contract / launch_proxy_contract) + snapshot regression
```

## Execution Order

```yaml
steps:
  - id: 1
    title: "proxy_socket_path() 支持 CCT_PROXY_SOCKET 覆盖"
    files: ["src/proxy.rs"]
    depends_on: []
  - id: 2
    title: "控制 socket 异步 accept（死锁修复核心）"
    files: ["src/proxy.rs"]
    depends_on: [1]
  - id: 3
    title: "应用层健康探测常量与 check_proxy_running 升级"
    files: ["src/proxy.rs"]
    depends_on: [2]
  - id: 4
    title: "ensure_proxy_running 重写（父进程：探测→试探 bind→spawn→就绪探测）"
    files: ["src/launch.rs"]
    depends_on: [3]
  - id: 5
    title: "占端口诊断辅助（只读 lsof + 降级文本）"
    files: ["src/proxy.rs"]
    depends_on: [3]
  - id: 6
    title: "run_proxy 启动：先探测再删 + bind 失败报错（TCP + 控制 socket EADDRINUSE）"
    files: ["src/proxy.rs"]
    depends_on: [3, 5]
  - id: 7
    title: "shutdown 命令退出前清理 socket 文件"
    files: ["src/proxy.rs"]
    depends_on: [2]
  - id: 8
    title: "控制命令与请求日志 api_key 脱敏"
    files: ["src/proxy.rs"]
    depends_on: [2]
  - id: 9
    title: "shutdown_proxy stop 2s 超时返回错误 + main.rs stop_proxy 区分无 socket / 无响应"
    files: ["src/proxy.rs", "src/main.rs"]
    depends_on: [3]
  - id: 10
    title: "tests/proxy_contract.rs 基础设施：stub 上游 + 动态端口 + 临时 socket 启动器"
    files: ["tests/proxy_contract.rs"]
    depends_on: [6, 9]
  - id: 11
    title: "tests/proxy_contract.rs：7 个行为契约"
    files: ["tests/proxy_contract.rs"]
    depends_on: [10]
  - id: 12
    title: "tests/launch_proxy_contract.rs：ensure_proxy_running 重启契约"
    files: ["tests/launch_proxy_contract.rs"]
    depends_on: [4, 10]
  - id: 13
    title: "配置快照回归（启动链路不写 Codex 配置文件）"
    files: ["tests/proxy_contract.rs"]
    depends_on: [11]
  - id: 14
    title: "G2 收尾：全量测试 + clippy + PoC 冒烟前置脚本"
    files: []
    depends_on: [11, 12, 13]
  - id: 15
    title: "迁移前置 + 全量 PoC 运行 🖐️ MANUAL"
    files: []
    depends_on: [14]
  - id: 16
    title: "B006-B008 会话可见性结果判定（条件分支）"
    files: []
    depends_on: [15]
  - id: 17
    title: "分层诊断确认 + poc.md Results Log 落账"
    files: ["docs/drafts/intake-reinvestigate-why-codex-sessions-20260801145824/poc/poc.md"]
    depends_on: [15, 16]
  - id: 18
    title: "TUI picker 可视化确认（OQ3）🖐️ MANUAL（可选）"
    files: []
    depends_on: [17]
  - id: 19
    title: "install-script.md 迁移说明（AC11）"
    files: ["docs/references/install-script.md"]
    depends_on: [14]
  - id: 20
    title: "五文档 AC13 清理"
    files: ["CLAUDE.md", "ARCHITECTURE.md", "docs/modules/launch.md", "docs/references/codex-home-storage-layout.md", "docs/references/codex-backend-development-guide.md"]
    depends_on: [14, 16]
  - id: 21
    title: "文档终审：B011/B013/B014 全 PASS"
    files: []
    depends_on: [17, 19, 20]
  - id: 22
    title: "Proof-Read End-to-End"
    files: []
    depends_on: [21]
  - id: 23
    title: "Cross-Check Acceptance Criteria"
    files: []
    depends_on: [22]
  - id: 24
    title: "Review"
    files: []
    depends_on: [23]
  - id: 25
    title: "Commit"
    files: []
    depends_on: [24]
```

执行链：1→2→3→4/5→6→10→11→12→13→14→15→16→17→21→22→23→24→25；19/20（G4）与 15-18（G3）无 shared files，可并行；20 依赖 16（可见性结果影响文档叙述）；18 为可选 MANUAL（OQ3，不阻塞）。

（注：Step 22–25 为终端步骤；Step 15 含终止 PID 29182 的敏感动作（执行前向用户确认）；Step 18 含 🖐️ MANUAL 子步骤——TUI picker 可视化确认，用户自愿。）
