//! Proxy 契约测试：真实二进制 + 临时 socket + 动态端口（约束 #10）。
//! 与用户实例隔离：全部走 CCT_PROXY_SOCKET / CCT_PROXY_PORT env。

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use cct::launch::ensure_proxy_running;
use cct::proxy::{check_proxy_running, send_control, switch_profile, ControlCommand};
use serial_test::serial;
use tempfile::TempDir;

/// 固定 DELTA：SSE 响应体断言锚点（smoke 与 stub_forwarding_with_bearer 共用）。
const DELTA: &str = "CONTRACT_STUB_DELTA";

/// 协议无关 stub 上游：记录 (method, path, authorization)，SSE 流式返回。
struct StubUpstream {
    log: Arc<Mutex<Vec<(String, String, String)>>>,
    port: u16,
    shutdown: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl StubUpstream {
    fn start() -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("stub bind");
        let port = listener.local_addr().expect("stub addr").port();
        let log = Arc::new(Mutex::new(Vec::new()));
        let shutdown = Arc::new(AtomicBool::new(false));
        let thread_log = log.clone();
        let thread_shutdown = shutdown.clone();
        let handle = std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { break };
                if thread_shutdown.load(Ordering::Relaxed) {
                    break;
                }
                let hlog = thread_log.clone();
                std::thread::spawn(move || {
                    if let Ok((method, path, auth)) = read_request(&mut stream) {
                        if !method.is_empty() {
                            hlog.lock().unwrap().push((method, path, auth));
                        }
                    }
                    let _ = stream.write_all(sse_response().as_bytes());
                    let _ = stream.shutdown(Shutdown::Both);
                });
            }
        });
        StubUpstream {
            log,
            port,
            shutdown,
            handle: Some(handle),
        }
    }

    fn requests(&self) -> Vec<(String, String, String)> {
        self.log.lock().unwrap().clone()
    }
}

impl Drop for StubUpstream {
    fn drop(&mut self) {
        // 关停：置标志 + 自连一次让阻塞中的 accept 返回并退出。
        self.shutdown.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

/// 读请求行 + 头，返回 (method, path, authorization)。EOF → 空 method。
fn read_request(stream: &mut TcpStream) -> std::io::Result<(String, String, String)> {
    let mut reader = BufReader::new(stream.try_clone().expect("clone stub stream"));
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("").to_string();
    let mut auth = String::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if line.to_ascii_lowercase().starts_with("authorization:") {
            auth = line
                .split_once(':')
                .map(|(_, v)| v.trim().to_string())
                .unwrap_or_default();
        }
    }
    Ok((method, path, auth))
}

/// SSE 事件流：response.created → response.output_text.delta → response.completed。
fn sse_response() -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n\
         event: response.created\n\
         data: {{\"type\":\"response.created\",\"response\":{{\"id\":\"resp_test_0001\",\"status\":\"in_progress\"}}}}\n\n\
         event: response.output_text.delta\n\
         data: {{\"type\":\"response.output_text.delta\",\"delta\":\"{DELTA}\"}}\n\n\
         event: response.completed\n\
         data: {{\"type\":\"response.completed\",\"response\":{{\"id\":\"resp_test_0001\",\"status\":\"completed\"}}}}\n\n"
    )
}

/// 动态端口：bind 0 取端口后 drop（返回即空闲）。
fn free_port() -> u16 {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind free port");
    let port = listener.local_addr().expect("local addr").port();
    drop(listener);
    port
}

/// 子进程守卫：测试失败（panic）时也确保 kill，不遗留孤儿 proxy。
struct ProxyChild(std::process::Child);

impl Drop for ProxyChild {
    fn drop(&mut self) {
        let _ = self.kill_and_wait();
    }
}

impl ProxyChild {
    /// SIGKILL + wait 回收子进程（Drop / read_stderr / 僵尸场景共用同一回收路径）。
    fn kill_and_wait(&mut self) -> std::io::Result<()> {
        self.0.kill()?;
        self.0.wait()?;
        Ok(())
    }

    /// 读子进程 stderr 全文（AC5 脱敏断言用）。先 kill+wait 使 stderr pipe 关闭
    /// （EOF）再 read_to_string——活进程的 pipe 上读取会阻塞等 EOF。所有相关日志
    /// 行（ctl / inbound / outbound）都在 HTTP 响应完成前写出（eprintln 无缓冲），
    /// kill 不丢行。
    fn read_stderr(&mut self) -> String {
        let _ = self.kill_and_wait();
        let mut out = String::new();
        if let Some(mut stderr) = self.0.stderr.take() {
            let _ = stderr.read_to_string(&mut out);
        }
        out
    }
}

/// env 变量守卫：记录旧值 → set/remove → Drop 时还原（panic 时也执行——
/// 测试进程内 env 共享，泄漏会污染后续测试）。RestartEnvGuard 与本测试共用。
struct EnvVarsGuard(Vec<(&'static str, Option<std::ffi::OsString>)>);

impl EnvVarsGuard {
    fn set(vars: &[(&'static str, Option<std::ffi::OsString>)]) -> Self {
        let prev = vars
            .iter()
            .map(|(name, value)| {
                let old = std::env::var_os(name);
                match value {
                    Some(v) => std::env::set_var(name, v),
                    None => std::env::remove_var(name),
                }
                (*name, old)
            })
            .collect();
        EnvVarsGuard(prev)
    }
}

impl Drop for EnvVarsGuard {
    fn drop(&mut self) {
        for (name, prev) in &self.0 {
            match prev {
                Some(v) => std::env::set_var(*name, v),
                None => std::env::remove_var(*name),
            }
        }
    }
}

/// ensure_proxy_running 重启路径的 env 守卫：覆写 CCT_PROXY_BIN/SOCKET/PORT
///（并移除 CCT_PROXY_LOG，防止子进程写用户 proxy.log）；Drop 时恢复原 env，
/// 并尽力 shutdown 被拉起的 proxy（panic 时也不遗留孤儿 daemon）。
struct RestartEnvGuard {
    socket: std::path::PathBuf,
    // 仅用于 Drop 副作用（还原 env）——字段名加下划线抑制 dead_code。
    _env: EnvVarsGuard,
}

impl RestartEnvGuard {
    fn set(socket: &Path, port: u16) -> Self {
        let env = EnvVarsGuard::set(&[
            ("CCT_PROXY_BIN", Some(env!("CARGO_BIN_EXE_cct").into())),
            ("CCT_PROXY_SOCKET", Some(socket.as_os_str().into())),
            ("CCT_PROXY_PORT", Some(port.to_string().into())),
            ("CCT_PROXY_LOG", None),
        ]);
        Self {
            socket: socket.to_path_buf(),
            _env: env,
        }
    }
}

impl Drop for RestartEnvGuard {
    fn drop(&mut self) {
        // shutdown 命令令 proxy 进程 exit(0)（见 proxy.rs "shutdown" 分支）——
        // 尽力回收 ensure_proxy_running 拉起的 daemon，失败（socket 已死）则忽略。
        let cmd = serde_json::from_value(serde_json::json!({"cmd": "shutdown"}))
            .expect("parse shutdown command");
        if send_control(&self.socket, &cmd).is_ok() {
            // shutdown 分支在 exit(0) 前 remove_file（shutdown_removes_socket_file
            // 契约）——轮询 socket 消失，确认 daemon 已处理 shutdown 并退出
            // （有界等待，避免残留 daemon 逃逸到测试进程退出之后）。
            let deadline = Instant::now() + Duration::from_secs(2);
            while self.socket.exists() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

/// 用真实二进制 + 临时 socket + 动态端口启动 proxy（约束 #10 隔离）。
/// NO_PROXY=127.0.0.1,localhost：proxy 内部 reqwest 直连 stub 上游，
/// 不经过用户 shell 的 http_proxy 环境变量（测试隔离，避免环境依赖）。
fn spawn_proxy(sock: &Path, port: u16) -> ProxyChild {
    let bin = env!("CARGO_BIN_EXE_cct");
    let child = std::process::Command::new(bin)
        .args(["proxy", "start"])
        .env("CCT_PROXY_SOCKET", sock)
        .env("CCT_PROXY_PORT", port.to_string())
        .env("CCT_PROXY_LOG", "1")
        .env("NO_PROXY", "127.0.0.1,localhost")
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn cct proxy");
    ProxyChild(child)
}

/// spawn `cct proxy stop`（stdout/stderr piped；stop 测试专用）。
fn spawn_stop(sock: &Path) -> std::process::Child {
    std::process::Command::new(env!("CARGO_BIN_EXE_cct"))
        .args(["proxy", "stop"])
        .env("CCT_PROXY_SOCKET", sock)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn cct proxy stop")
}

/// 有界等待子进程自行退出：try_wait 轮询（预算内未退出 → panic）。
/// 避免对可能挂起的子进程用 `.wait()` 无限阻塞。
fn wait_child_exit(
    child: &mut std::process::Child,
    budget: Duration,
    context: &str,
) -> std::process::ExitStatus {
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().expect("try_wait child") {
            return status;
        }
        assert!(started.elapsed() < budget, "{context} — never exited");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// 轮询应用层探测直至健康（check_proxy_running 发送 status 等响应）。
fn wait_healthy(sock: &Path, budget: Duration) -> bool {
    let deadline = Instant::now() + budget;
    while Instant::now() < deadline {
        if check_proxy_running(sock) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

/// 一次完成 proxy 启动：临时 socket + 动态端口 + NO_PROXY 隔离 + 健康等待。
/// 返回 (TempDir, sock 路径, port, ProxyChild)——TempDir 存活到测试结束。
fn start_proxy() -> (TempDir, std::path::PathBuf, u16, ProxyChild) {
    let dir = TempDir::new().expect("tempdir");
    let sock = dir.path().join("proxy.sock");
    let port = free_port();
    let child = spawn_proxy(&sock, port);
    assert!(
        wait_healthy(&sock, Duration::from_secs(3)),
        "proxy control channel must become healthy within 3s"
    );
    (dir, sock, port, child)
}

/// 手写 HTTP 请求（std TcpStream，Connection: close，可选 Authorization + body）。
fn http_request(
    method: &str,
    url: &str,
    auth: Option<&str>,
    body: Option<&str>,
    timeout: Duration,
) -> std::io::Result<String> {
    let without_scheme = url.strip_prefix("http://").unwrap_or(url);
    let (authority, path) = match without_scheme.find('/') {
        Some(i) => (&without_scheme[..i], &without_scheme[i..]),
        None => (without_scheme, "/"),
    };
    let mut stream = TcpStream::connect(authority)?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: {authority}\r\nConnection: close\r\n"
    )?;
    if let Some(a) = auth {
        write!(stream, "Authorization: {a}\r\n")?;
    }
    if let Some(b) = body {
        write!(stream, "Content-Type: application/json\r\n")?;
        write!(stream, "Content-Length: {}\r\n", b.len())?;
    }
    write!(stream, "\r\n")?;
    if let Some(b) = body {
        write!(stream, "{b}")?;
    }
    let mut out = String::new();
    stream.read_to_string(&mut out)?;
    Ok(out)
}

/// 手写 HTTP GET（std TcpStream，Connection: close，可选 Authorization）。
fn http_get(url: &str, auth: Option<&str>, timeout: Duration) -> std::io::Result<String> {
    http_request("GET", url, auth, None, timeout)
}

/// ControlCommand 字段私有：经 serde_json 构造（Deserialize 公开）。
fn status_cmd() -> ControlCommand {
    serde_json::from_value(serde_json::json!({"cmd": "status"})).expect("parse status command")
}

/// ControlCommand 字段私有：经 serde_json 构造（Deserialize 公开）。
/// base_url/api_key 为可选字段（serde default）——仅传 cmd + 两个字段。
fn switch_cmd(base_url: &str, api_key: &str) -> ControlCommand {
    serde_json::from_value(serde_json::json!({
        "cmd": "switch",
        "base_url": base_url,
        "api_key": api_key,
    }))
    .expect("parse switch command")
}

// ── smoke ───────────────────────────────────────────────────────────────

/// 基础设施 smoke：stub 直接收请求（记录 method/path/auth + SSE 响应体），
/// spawn_proxy 起真实二进制且控制通道健康（check_proxy_running）。
/// 注意：不走 proxy 的 HTTP 转发路径——HTTP 转发契约由 concurrent_control_and_http 覆盖。
#[test]
fn smoke_stub_receives_request() {
    let stub = StubUpstream::start();
    let url = format!("http://127.0.0.1:{}/v1/models", stub.port);
    let body = http_get(&url, Some("Bearer sk-smoke-key"), Duration::from_secs(2))
        .expect("stub must respond");
    assert!(
        body.contains("response.created"),
        "SSE body missing created event"
    );
    assert!(body.contains(DELTA), "SSE body missing delta");
    let reqs = stub.requests();
    assert_eq!(
        reqs.len(),
        1,
        "stub must record exactly one request, got {reqs:?}"
    );
    assert_eq!(
        reqs[0],
        (
            "GET".to_string(),
            "/v1/models".to_string(),
            "Bearer sk-smoke-key".to_string()
        )
    );

    let (_dir, _sock, _port, _proxy) = start_proxy();
}

// ── 7 行为契约（Step 11）──────────────────────────────────────────────

/// AC1 死锁回归守卫：控制通道并发 ≥20 次 status + 主线程 HTTP GET /v1/models，
/// 两者都必须在 3s 预算内完成。若控制 socket 的 accept 退化为同步阻塞，将饿死
/// current_thread runtime，HTTP 请求挂起 → 读超时 → 本测试失败（修复前状态）。
#[test]
#[serial]
fn concurrent_control_and_http() {
    const ITERATIONS: u32 = 20;
    const BUDGET: Duration = Duration::from_secs(3);

    let (_dir, sock, port, _proxy) = start_proxy();

    // 线程 A：循环 status 控制命令（send_control 内部自带 PROBE_TIMEOUT 读超时，
    // 单次调用有界）。
    let (tx, rx) = mpsc::channel();
    let thread_sock = sock.clone();
    let ctl_handle = std::thread::spawn(move || {
        for i in 0..ITERATIONS {
            if let Err(e) = send_control(&thread_sock, &status_cmd()) {
                let _ = tx.send(Err(format!("iteration {i}: {e}")));
                return;
            }
        }
        let _ = tx.send(Ok(()));
    });

    // 主线程：并发 HTTP GET /v1/models（2s 读超时——挂起时有限失败而非无限等）。
    let http_started = Instant::now();
    let http_result = http_get(
        &format!("http://127.0.0.1:{port}/v1/models"),
        None,
        Duration::from_secs(2),
    );
    let http_elapsed = http_started.elapsed();

    // 控制线程必须在 3s 预算内完成全部 20 次。
    let ctl_result = rx
        .recv_timeout(BUDGET)
        .expect("control thread did not finish within the 3s budget");
    ctl_handle.join().expect("control thread panicked");

    assert!(
        http_elapsed < BUDGET,
        "HTTP GET took {http_elapsed:?} — exceeded the 3s budget"
    );
    match http_result {
        Ok(body) => {
            assert!(
                body.contains("HTTP/1.1"),
                "HTTP GET must return an HTTP response, got: {body:?}"
            );
        }
        Err(e) => {
            panic!("HTTP GET did not complete within the 3s budget (elapsed {http_elapsed:?}): {e}")
        }
    }
    if let Err(e) = ctl_result {
        panic!("control command loop failed: {e}");
    }
}

/// AC4 转发链路（约束 #4）：stub 上游 → switch 控制命令（base_url=stub,
/// api_key="sk-contract-key"）→ HTTP POST /v1/chat（经 proxy 转发）。
/// 客户端不携带 Authorization——Bearer 必须由 proxy 按 switch 注入的 api_key 生成；
/// stub 记录 (method, path, authorization) + SSE 响应体含固定 DELTA（流式返回）。
#[test]
#[serial]
fn stub_forwarding_with_bearer() {
    let stub = StubUpstream::start();
    let stub_url = format!("http://127.0.0.1:{}", stub.port);

    let (_dir, sock, port, _proxy) = start_proxy();

    // ControlResponse 字段私有：send_control 返回 Ok 即代表有应答；
    // switch 失败（如 unknown command → status "err"）会经后续 stub 断言暴露。
    send_control(&sock, &switch_cmd(&stub_url, "sk-contract-key"))
        .expect("switch command must be answered");

    // 客户端不带 Authorization 头：Bearer 必须由 proxy 注入（转发契约的核心）。
    let body = http_request(
        "POST",
        &format!("http://127.0.0.1:{port}/v1/chat"),
        None,
        Some(r#"{"model":"test-model","messages":[{"role":"user","content":"hi"}]}"#),
        Duration::from_secs(3),
    )
    .expect("proxy must forward the POST and stream the response back");

    // 1) stub 记录到 Bearer sk-contract-key（约束 #4：Bearer key 转发）。
    let reqs = stub.requests();
    assert_eq!(
        reqs.len(),
        1,
        "stub must record exactly one request, got {reqs:?}"
    );
    assert_eq!(
        reqs[0],
        (
            "POST".to_string(),
            "/v1/chat".to_string(),
            "Bearer sk-contract-key".to_string()
        ),
        "stub must see POST /v1/chat with the switched Bearer key"
    );

    // 2) 响应体含固定 DELTA（SSE 流式返回，约束 #4）。
    assert!(
        body.contains("HTTP/1.1 200"),
        "proxy must return 200, got: {body:?}"
    );
    // 头名大小写由代理的生成方式决定（HTTP 协议不敏感）：小写化后匹配契约值。
    let body_lower = body.to_ascii_lowercase();
    assert!(
        body_lower.contains("content-type: text/event-stream"),
        "proxy must relay the SSE content type, got: {body:?}"
    );
    assert!(
        body_lower.contains("transfer-encoding: chunked"),
        "SSE must be streamed chunked (not buffered with Content-Length), got: {body:?}"
    );
    assert!(
        body.contains(DELTA),
        "SSE body missing delta, got: {body:?}"
    );
    // SSE 事件顺序：response.created → delta → response.completed。
    let created = body
        .find("response.created")
        .expect("missing response.created event");
    let delta = body.find(DELTA).expect("missing delta");
    let completed = body
        .find("response.completed")
        .expect("missing response.completed event");
    assert!(
        created < delta && delta < completed,
        "SSE event order must be created → delta → completed"
    );
}

/// AC5 日志脱敏（约束 #7，mask-secrets-on-every-display-path）：CCT_PROXY_LOG=1
/// 下捕获 proxy 进程 stderr → switch 携带 api_key="sk-contract-key"（ctl 日志路径）
/// + HTTP 请求 path/query 含 sk- 值（inbound + outbound 日志路径）→ stderr 全文
/// 不得含任何 sk- 明文；掩码文本 sk-*** 必须出现（反真空守卫：日志路径真实触发过）。
#[test]
#[serial]
fn log_masks_api_key() {
    let stub = StubUpstream::start();
    let stub_url = format!("http://127.0.0.1:{}", stub.port);

    let (_dir, sock, port, mut proxy) = start_proxy();

    // 1) 控制通道：switch 携带 api_key="sk-contract-key"（ctl 日志路径）。
    send_control(&sock, &switch_cmd(&stub_url, "sk-contract-key"))
        .expect("switch command must be answered");

    // 2) HTTP 请求：path/query 含 sk- 值（inbound / outbound 日志路径）。
    let body = http_get(
        &format!("http://127.0.0.1:{port}/v1/models?key=sk-xyz-query"),
        None,
        Duration::from_secs(3),
    )
    .expect("proxy must forward the GET");
    assert!(
        body.contains("HTTP/1.1"),
        "proxy must return an HTTP response, got: {body:?}"
    );

    // 3) 读 stderr 全文（kill+wait 后 pipe EOF，见 ProxyChild::read_stderr）。
    let stderr = proxy.read_stderr();

    assert!(
        !stderr.contains("sk-contract-key"),
        "stderr must not contain the switched api_key plaintext, got:\n{stderr}"
    );
    assert!(
        !stderr.contains("sk-xyz-query"),
        "stderr must not contain the request query secret plaintext, got:\n{stderr}"
    );
    // 反真空守卫：若日志路径根本没触发（如 CCT_PROXY_LOG 未生效），上面断言会空转通过。
    assert!(
        stderr.contains("sk-***"),
        "stderr must contain masked sk-*** form (ctl/inbound log paths must have fired), got:\n{stderr}"
    );
}

/// AC5 扩展：上游不可达时，reqwest 错误文本内嵌完整请求 URL（含 query 中的 sk- 值），
/// 该错误日志路径同样不得泄露出明文（连接被拒 → 502 + stderr 脱敏）。
#[test]
#[serial]
fn log_masks_api_key_upstream_error() {
    let (_dir, sock, port, mut proxy) = start_proxy();

    // 指向必定连接被拒的端口（127.0.0.1:1 无服务监听，ECONNREFUSED 立即返回）。
    send_control(&sock, &switch_cmd("http://127.0.0.1:1", "sk-contract-key"))
        .expect("switch command must be answered");

    let body = http_get(
        &format!("http://127.0.0.1:{port}/v1/models?key=sk-error-query"),
        None,
        Duration::from_secs(3),
    )
    .expect("proxy must answer 502, not hang");
    assert!(
        body.contains("HTTP/1.1 502"),
        "dead upstream must yield 502, got: {body:?}"
    );

    let stderr = proxy.read_stderr();
    assert!(
        !stderr.contains("sk-contract-key"),
        "stderr must not contain the switched api_key plaintext, got:\n{stderr}"
    );
    assert!(
        !stderr.contains("sk-error-query"),
        "stderr must not contain the request query secret inside the upstream error line, got:\n{stderr}"
    );
    // 反真空守卫：outbound 日志行在 send 前写出，必含掩码形式。
    assert!(
        stderr.contains("sk-***"),
        "stderr must contain masked sk-*** form (outbound/error log paths must have fired), got:\n{stderr}"
    );
}

/// AC1/AC10 stop 超时（约束 #1/#10）：`cct proxy stop` 对无响应控制 socket 必须
/// 有界超时（main.rs stop_proxy 经 shutdown_proxy STOP_TIMEOUT=2s 传播错误 → 非 0
/// 退出 + stderr 报错），不得无限挂起；socket 不存在时快速 exit 0。三态全覆盖
/// （tdd.md TC-9 case 1-3）：① 本测试（无响应 → 有界超时非 0）+ ② 本测试（无 socket
/// → 快速 exit 0 + "Proxy is not running."）+ ③ stop_rejects_stale_socket（文件残留
/// 但 connect 立即拒绝 → 快速非 0，不得误报 not running）。
#[test]
#[serial]
fn stop_times_out_on_unresponsive_socket() {
    // ① socket 存在但无响应：UnixListener bind 临时路径，thread accept 后 hold 住
    //    不回包（模拟死锁 proxy）→ spawn `cct proxy stop` → ≤2.5s 内非 0 退出，
    //    stderr 含错误信息（不得误报 "Proxy is not running."）。
    let dir = TempDir::new().expect("tempdir");
    let sock = dir.path().join("stop-silent.sock");
    let listener = std::os::unix::net::UnixListener::bind(&sock).expect("bind control socket");
    let (release_tx, release_rx) = mpsc::channel();
    let hold = std::thread::spawn(move || {
        let (_stream, _peer) = listener.accept().expect("accept stop connection");
        // 不读不回包，hold 到主线程释放（模拟挂死的 proxy 控制通道）。
        let _ = release_rx.recv();
    });

    let mut child = spawn_stop(&sock);

    // 有界等待：挂起 → 预算内未退出即断言失败，而非无限阻塞。
    let started = Instant::now();
    let status = wait_child_exit(
        &mut child,
        Duration::from_secs(4),
        "cct proxy stop hung on unresponsive socket",
    );
    let output = child.wait_with_output().expect("collect stop output");
    let elapsed = started.elapsed();

    let _ = release_tx.send(());
    hold.join().expect("hold thread panicked");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !status.success(),
        "stop on unresponsive socket must exit non-zero (STOP_TIMEOUT propagates), got: {status:?}"
    );
    assert!(
        elapsed <= Duration::from_millis(2500),
        "stop must time out within 2.5s (STOP_TIMEOUT=2s + margin), took {elapsed:?}"
    );
    assert!(
        stderr.contains("Error"),
        "stderr must carry the shutdown error, got: {stderr:?}"
    );
    assert!(
        !stderr.contains("Proxy is not running."),
        "must NOT misreport 'not running' on an unresponsive socket, stderr: {stderr:?}"
    );
    assert!(
        stdout.is_empty(),
        "error path must not print the success message, stdout: {stdout:?}"
    );

    // ② socket 不存在 → 快速（≤1s）exit 0 + "Proxy is not running."
    let absent = dir.path().join("stop-absent.sock");
    let started = Instant::now();
    let output = spawn_stop(&absent)
        .wait_with_output()
        .expect("collect stop output (absent socket)");
    let elapsed = started.elapsed();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "absent socket must exit 0, got: {:?}",
        output.status
    );
    assert!(
        elapsed < Duration::from_secs(1),
        "absent socket stop must return quickly, took {elapsed:?}"
    );
    assert!(
        stdout.contains("Proxy is not running."),
        "stdout must say 'Proxy is not running.', got: {stdout:?}"
    );
}

/// TC-9 case ③（tdd.md:50）：stale socket 快错误路径——socket 文件存在但 connect
/// 立即拒绝（旧版遗留死 socket，进程已死、文件无人清理）。`cct proxy stop` 因文件
/// 存在不走 not-running 分支，而是 shutdown_proxy 传播 ECONNREFUSED → 快速（≤1s）
/// 非 0 退出，stdout 不得误报 "Proxy is not running."。旧实现把一切 connect 错误
/// 当 not running 吞掉 exit 0——本断言即该语义变更的回归守卫。
#[test]
#[serial]
fn stop_rejects_stale_socket() {
    let dir = TempDir::new().expect("tempdir");
    let stale = dir.path().join("stop-stale.sock");
    {
        let listener = std::os::unix::net::UnixListener::bind(&stale).expect("bind stale socket");
        drop(listener); // 文件残留、无人 accept → 后续 connect 立即被拒
    }
    assert!(
        stale.exists(),
        "前置：drop 后 socket 文件必须残留（stale socket 场景）"
    );

    let started = Instant::now();
    let output = spawn_stop(&stale)
        .wait_with_output()
        .expect("collect stop output (stale socket)");
    let elapsed = started.elapsed();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "stale socket (connect refused) must exit non-zero (ECONNREFUSED propagates), got: {:?}",
        output.status
    );
    assert!(
        elapsed < Duration::from_secs(1),
        "stale socket stop must return quickly (<1s, connect refused is immediate), took {elapsed:?}"
    );
    assert!(
        !stdout.contains("Proxy is not running."),
        "must NOT misreport 'not running' on a stale socket, stdout: {stdout:?}"
    );
    assert!(
        stderr.contains("Error"),
        "stderr must carry the shutdown connect error, got: {stderr:?}"
    );
}

/// AC2 僵尸自愈（约束 #2）：真实 proxy 被 SIGKILL 后 socket 文件残留（无清理路径），
/// 应用层探测失败 → ensure_proxy_running（lib 直调）必须重启 proxy 并恢复健康。
/// 重启路径依赖进程 env：ensure_proxy_running spawn 的子进程继承父进程 env——
/// 本测试设 CCT_PROXY_BIN=真实入口（与 Step 12 launch 契约同一注入约定，否则
/// spawn 测试二进制自身 → 就绪探测耗尽）、CCT_PROXY_SOCKET/PORT=本测试临时路径。
#[test]
#[serial]
fn zombie_recovery_restarts_proxy() {
    let (_dir, sock, port, mut proxy) = start_proxy();

    // SIGKILL 旧 proxy：无清理路径可执行 → socket 文件残留（僵尸场景）。
    proxy.kill_and_wait().expect("SIGKILL proxy and reap it");
    assert!(
        sock.exists(),
        "前置：SIGKILL 后 socket 文件必须残留（僵尸场景）"
    );
    assert!(
        !check_proxy_running(&sock),
        "前置：残留 socket 无进程应答 → check_proxy_running 应为 false"
    );

    let _env_guard = RestartEnvGuard::set(&sock, port);
    let result = cct::launch::ensure_proxy_running(port, &sock);
    assert!(
        result.is_ok(),
        "僵尸 socket 场景 ensure_proxy_running 必须重启真实 proxy 并返回 Ok: {result:?}"
    );
    assert!(
        check_proxy_running(&sock),
        "重启后 proxy 必须恢复健康（能应答应用层 status 探测）"
    );
}

/// AC3 占端口报错（约束 #3，不 kill）：测试进程先 bind 动态端口并保持监听（占用者），
/// spawn_proxy 同端口 → 子进程必须自行退出（非 0），stderr 含占用诊断（port_conflict_message
/// 的 lsof-PID / 降级建议两种分支之一）且不得 panic；占用者（测试自己的 listener）必须
/// 仍存活（cct 不自动终止进程）。回归守卫：若 bind 失败路径回退为 panic（exit 101）
/// 或丢失诊断文本，本测试红。
#[test]
#[serial]
fn port_occupied_reports_error_keeps_occupant() {
    // 测试进程先占住动态端口（listener keep alive 至测试结束）。
    let occupant = TcpListener::bind(("127.0.0.1", 0)).expect("bind occupant listener");
    let port = occupant.local_addr().expect("occupant addr").port();

    let dir = TempDir::new().expect("tempdir");
    let sock = dir.path().join("proxy-occupied.sock");
    let mut child = spawn_proxy(&sock, port);

    // 子进程必须在预算内自行退出（bind 失败 → exit(1) + 诊断）——
    // 有界轮询而非无限 wait，回归挂起时快速失败。
    let status = wait_child_exit(
        &mut child.0,
        Duration::from_secs(5),
        "proxy must exit on its own when the port is occupied",
    );
    // stderr pipe：进程已退出（EOF）→ 直接 read_to_string。
    let mut stderr = String::new();
    if let Some(mut err) = child.0.stderr.take() {
        let _ = err.read_to_string(&mut stderr);
    }

    // 1) 退出码非 0（bind 失败路径 → exit(1)）。
    assert!(
        !status.success(),
        "occupied port must yield non-zero exit, got: {status:?}"
    );
    // 2) stderr 含占用信息：port_conflict_message 两分支（lsof-PID / 降级建议）任一。
    assert!(
        stderr.contains(&format!("port {port} already in use")) || stderr.contains("lsof -iTCP"),
        "stderr must carry the port-conflict diagnosis, got:\n{stderr}"
    );
    // 3) 不得走 panic 路径（exit(1) 路径不得在 stderr 输出 panic 文本）。
    assert!(
        !stderr.contains("panic"),
        "port conflict must not panic (no 'panic' on stderr), got:\n{stderr}"
    );
    // 4) 占用者仍存活：同端口再 bind 必须失败（若占用者被杀，端口释放 → bind 成功）。
    assert!(
        TcpListener::bind(("127.0.0.1", port)).is_err(),
        "occupant listener must still own the port (constraint #3: cct never kills it)"
    );
    assert!(
        occupant.local_addr().is_ok(),
        "occupant listener handle must still be alive"
    );
}

/// AC9 双启动竞态（约束 #5/#10）：同 socket + 同端口几乎同时 spawn 两个 proxy →
/// 恰一个存活（check_proxy_running true），输家 ≤2s 内退出非 0，两 stderr 合计无
/// "panic"（EADDRINUSE/EEXIST 收敛，绝不允许双活）。实现为"先绑后删"
/// （delete-on-conflict，见 src/proxy.rs run_proxy / is_bind_conflict）：bind 冲突
/// （Linux EADDRINUSE / macOS EEXIST）→ 探测活 proxy → 报错退出（不删其控制
/// 通道）；探测失败（僵尸 socket）→ 删后重绑。均非 panic。注：探测误判残差
/// （对启动中实例）仍存在，见 refactor 日志分析。
#[test]
#[serial]
fn double_start_race_one_wins() {
    let dir = TempDir::new().expect("tempdir");
    let sock = dir.path().join("race.sock");
    let port = free_port();

    // 两个 spawn 背靠背发出（尽力同时），同 socket + 同端口。
    let mut child_a = spawn_proxy(&sock, port);
    let mut child_b = spawn_proxy(&sock, port);

    // 有界等待：≤2s 内两子进程都应退出（try_wait 轮询，绝不死锁）。
    // 时序注意：可能先双活、输家随后退出——预算内必须收敛为恰一个退出。
    let started = Instant::now();
    let mut status_a = None;
    let mut status_b = None;
    while started.elapsed() < Duration::from_secs(2) {
        if status_a.is_none() {
            status_a = child_a.0.try_wait().expect("try_wait proxy A");
        }
        if status_b.is_none() {
            status_b = child_b.0.try_wait().expect("try_wait proxy B");
        }
        if status_a.is_some() && status_b.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let elapsed = started.elapsed();

    // 1) 恰一个存活：胜利者持有控制通道（应用层探测）。双启动必亡其一（TCP
    //    EADDRINUSE / unix bind 互斥），此断言确认幸存者的通道未被破坏。
    assert!(
        check_proxy_running(&sock),
        "exactly one proxy must survive the double-start race — check_proxy_running({sock:?}) \
         was false after {elapsed:?} (status_a={status_a:?}, status_b={status_b:?})"
    );

    // 2) 预算内恰一个退出（绝不双活），输家退出码非 0（bind 冲突收敛）。
    let loser_code = match (status_a, status_b) {
        (Some(sa), None) => sa.code(),
        (None, Some(sb)) => sb.code(),
        (Some(_), Some(_)) => panic!(
            "both proxies exited after {elapsed:?} — exactly one must survive \
             (status_a={status_a:?}, status_b={status_b:?})"
        ),
        (None, None) => panic!(
            "both proxies still alive after {elapsed:?} (2s budget) — double-start must \
             converge to exactly one winner"
        ),
    };
    assert!(
        loser_code.map(|c| c != 0).unwrap_or(false),
        "the losing proxy must exit non-zero (socket bind conflict convergence), got code {loser_code:?}"
    );

    // 3) 两子进程 stderr 合计不得含 "panic"（当前实现 unix bind 失败走 expect panic）。
    let stderr_a = child_a.read_stderr();
    let stderr_b = child_b.read_stderr();
    let combined = format!("[A]\n{stderr_a}\n[B]\n{stderr_b}");
    assert!(
        !combined.contains("panic"),
        "double-start race must not panic, combined stderr:\n{combined}"
    );
}

/// 控制 socket 重探测耗尽（约束 #3/#5 防御路径的直接测试，audit-edge_cases
/// Item 4）：socket 路径被"不可清除的异例占用者"持续占据 → 首次 bind EEXIST →
/// 探测（connect 立即失败）→ 删除失败（目录不可 unlink，`let _ =` 吞掉）→
/// 重绑仍冲突 → 重探测 3×500ms（PROBE_RETRIES×PROBE_TIMEOUT）耗尽 →
/// exit_bind_failed exit(1)，stderr 含 "control socket bind" 且无 panic。
///
/// 占用者形态的选择：审计原建议"循环 bind→sleep→drop 抢绑线程"，但该形态
/// 与 proxy 的 remove→rebind（µs 级窗口）存在竞态——抢绑线程的下一次 bind 需
/// 恰落在此 µs 窗口内才赢，proxy 几乎总是重绑成功 → 测试 flake（"偶发 proxy
/// 恰好 rebind 成功"实为主流时序）。目录占用者无此竞态：remove_file 对目录
/// 必然失败（macOS EPERM / Linux EISDIR），路径在重绑瞬间仍被占——正是
/// proxy.rs 注释定义的"僵尸文件/异例（非 proxy 进程占路径）"场景（TCP 先行后
/// 该分支仅对这类占路径者触发），且确定性成立（本机实测 bind 目录 → EADDRINUSE
/// os error 48；connect 目录 → ENOTSOCK 立即失败；Linux 同为 EADDRINUSE）。
#[test]
#[serial]
fn control_socket_rebind_exhaustion_exits() {
    let dir = TempDir::new().expect("tempdir");
    let sock = dir.path().join("exhaustion.sock");
    std::fs::create_dir(&sock).expect("create directory occupant at socket path");

    let port = free_port();
    let mut child = spawn_proxy(&sock, port);

    // 子进程必须自行退出（重探测耗尽 → exit_bind_failed exit(1)），预算 5s
    // （算法最坏 ~1.5s：3×500ms sleep + 探测均瞬时失败；预算含调度抖动）。
    let started = Instant::now();
    let status = wait_child_exit(
        &mut child.0,
        Duration::from_secs(5),
        "proxy must exit on its own when the control-socket rebind keeps failing",
    );
    let mut stderr = String::new();
    if let Some(mut err) = child.0.stderr.take() {
        let _ = err.read_to_string(&mut stderr);
    }
    let elapsed = started.elapsed();

    // 1) 退出码非 0（耗尽路径 → exit_bind_failed → exit(1)，非 panic 101）。
    assert!(
        !status.success(),
        "control-socket rebind exhaustion must yield non-zero exit, got: {status:?}"
    );
    // 2) stderr 含控制 bind 失败信息（exit_bind_failed 的诊断文本契约）。
    assert!(
        stderr.contains("control socket bind"),
        "stderr must carry the control-socket bind failure, got:\n{stderr}"
    );
    // 3) 不得 panic。
    assert!(
        !stderr.contains("panic"),
        "control-socket rebind exhaustion must not panic, got:\n{stderr}"
    );
    // 4) 有界收敛：3×500ms 重探测 sleep（~1.5s）+ 余量，不得挂起（审计 ≤3s）。
    assert!(
        elapsed <= Duration::from_secs(3),
        "exhaustion must converge within ~1.5s (3×500ms probe sleeps) + margin, took {elapsed:?}"
    );
}

// ── 7 行为契约（Step 7 / 约束 #6）──────────────────────────────────────

/// 约束 #6 稳态缺陷回归：shutdown 退出前必须清理 socket 文件。启动真实 proxy →
/// 健康 → spawn `cct proxy stop`（真实二进制路径，CCT_PROXY_SOCKET 注入）→
/// proxy 退出（exit 0）→ 断言 socket 文件不存在。当前实现（Step 7 未做）
/// shutdown 分支 exit(0) 不删文件 → 每次 stop 留下死 socket（僵尸自愈场景源头之一）。
#[test]
#[serial]
fn shutdown_removes_socket_file() {
    let (_dir, sock, _port, _proxy) = start_proxy();
    assert!(sock.exists(), "前置：健康 proxy 必须持有 socket 文件");

    let mut child = spawn_stop(&sock);
    let status = wait_child_exit(
        &mut child,
        Duration::from_secs(3),
        "cct proxy stop must exit within 3s (STOP_TIMEOUT=2s + margin)",
    );
    let output = child.wait_with_output().expect("collect stop output");

    assert!(
        status.success(),
        "stop on healthy proxy must exit 0, got: {status:?} stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !sock.exists(),
        "shutdown must remove the socket file — dead socket file left behind at {sock:?}"
    );
}

// ── AC14 配置快照回归（Step 13 / 约束 #14）────────────────────────────

/// 递归收集 root 下全部文件（相对 root 的路径，排序后返回）。
fn snapshot_codex_home(root: &Path) -> Vec<std::path::PathBuf> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, root, out);
            } else {
                out.push(
                    path.strip_prefix(root)
                        .expect("strip codex_home root")
                        .to_path_buf(),
                );
            }
        }
    }
    let mut files = Vec::new();
    walk(root, root, &mut files);
    files.sort();
    files
}

/// 约束 #14 禁止名单：config.toml / auth.json / profile-*.config.toml。
fn is_codex_config_file(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    name == "config.toml"
        || name == "auth.json"
        || (name.starts_with("profile-") && name.ends_with(".config.toml"))
}

/// AC14 配置快照回归（约束 #14）：启动链路不得在 CODEX_HOME 下写任何 Codex
/// 配置文件（config.toml / auth.json / profile-*.config.toml）。
/// exec_codex_proxy 第 3 步 exec-replace 不可达（会替换测试进程），故覆盖其
/// 前置 1-2 步等价路径：ensure_proxy_running + switch_profile；CODEX_HOME /
/// CCT_CONFIG 注入测试进程 env，ensure_proxy_running spawn 的 proxy 子进程
/// 继承之（真实启动链路的 env 面）。snapshot：写入前后 codex_home 文件集合必须一致。
#[test]
#[serial]
fn launch_path_writes_no_codex_config() {
    let dir = TempDir::new().expect("tempdir");
    let codex_home = dir.path().join("codex_home");
    std::fs::create_dir_all(&codex_home).expect("create codex_home");
    let sock = dir.path().join("proxy.sock");
    let port = free_port();

    // env 面：CODEX_HOME + CCT_CONFIG 临时化（CCT_PROXY_SOCKET/PORT/BIN 由
    // RestartEnvGuard 注入）。守卫 Drop 时还原——断言 panic 也不泄漏到后续测试。
    let profiles_path = dir.path().join("profiles.toml");
    std::fs::write(&profiles_path, "").expect("write temp profiles.toml");
    let _codex_env = EnvVarsGuard::set(&[
        ("CODEX_HOME", Some(codex_home.as_os_str().into())),
        ("CCT_CONFIG", Some(profiles_path.as_os_str().into())),
    ]);

    // snapshot：写入前文件集合。
    let before = snapshot_codex_home(&codex_home);

    // 启动链路前置两步（exec_codex_proxy 的 1-2 步）：起 proxy + switch 上游。
    let _restart_guard = RestartEnvGuard::set(&sock, port);
    ensure_proxy_running(port, &sock).expect("ensure_proxy_running must start the real proxy");
    // switch 紧连竞态加固（Red 阶段 1/12 次 ENOTCONN，macOS unix socket 偶发）：
    // 仅重试连接级瞬时错误（NotConnected / ConnectionRefused）；状态级错误
    // （proxy 应答 err）立即失败——不削弱契约断言。
    let mut switched = false;
    for _ in 0..3 {
        match switch_profile(&sock, "http://127.0.0.1:1", "sk-snapshot-key", "gpt-4.1") {
            Ok(()) => {
                switched = true;
                break;
            }
            Err(e)
                if !matches!(
                    e.kind(),
                    std::io::ErrorKind::NotConnected | std::io::ErrorKind::ConnectionRefused
                ) =>
            {
                panic!("switch_profile must be answered by the proxy: {e}")
            }
            Err(_) => std::thread::sleep(Duration::from_millis(100)),
        }
    }
    assert!(
        switched,
        "switch_profile must be answered by the proxy (3 transient connect attempts failed)"
    );

    // snapshot：写入后文件集合 → 与写入前一致（无任何 Codex 配置文件被创建）。
    let after = snapshot_codex_home(&codex_home);
    assert_eq!(
        before, after,
        "launch path must not write any file under CODEX_HOME — before: {before:?} after: {after:?}"
    );
    assert!(
        !after.iter().any(|f| is_codex_config_file(f)),
        "launch path must not write codex config files (config.toml / auth.json / \
         profile-*.config.toml), found: {after:?}"
    );
}
