use std::io::{self, BufRead, BufReader, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use bytes::Bytes;
use futures_util::TryStreamExt;
use http_body_util::{BodyExt, Full, StreamBody};
use hyper::body::{Frame, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, UnixListener as TokioUnixListener};

// ── shared state ──────────────────────────────────────────────────────────

#[derive(Default)]
struct ActiveProfile {
    base_url: String,
    api_key: String,
    model: String,
}

struct ProxyState {
    active: RwLock<ActiveProfile>,
}

// ── control protocol ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
pub struct ControlCommand {
    cmd: String,
    #[serde(default)]
    base_url: Option<String>,
    #[serde(default)]
    api_key: Option<String>,
    #[serde(default)]
    model: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ControlResponse {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
}

// ── public API ────────────────────────────────────────────────────────────

/// Default proxy listen port. Override with `CCT_PROXY_PORT`.
pub fn proxy_port() -> u16 {
    std::env::var("CCT_PROXY_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(19191)
}

/// Path to the Unix domain socket used for control commands. Override with
/// `CCT_PROXY_SOCKET`.
pub fn proxy_socket_path() -> PathBuf {
    if let Ok(p) = std::env::var("CCT_PROXY_SOCKET") {
        return PathBuf::from(p);
    }
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("cc-tui")
        .join("proxy.sock")
}

/// Path to the proxy log file (only used when `CCT_PROXY_LOG` is set).
pub fn proxy_log_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("cc-tui")
        .join("proxy.log")
}

/// Log a message to stderr when `CCT_PROXY_LOG` is set.
macro_rules! log_proxy {
    ($($arg:tt)*) => {
        if std::env::var("CCT_PROXY_LOG").is_ok() {
            eprintln!("[cct-proxy] {}", format!($($arg)*));
        }
    };
}

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

/// Send a JSON control command to the proxy and return the response.
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
    let payload =
        serde_json::to_vec(cmd).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    stream.write_all(&payload)?;
    stream.write_all(b"\n")?;
    stream.shutdown(Shutdown::Write)?;

    let mut reader = BufReader::new(&stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    if line.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "proxy closed connection without responding",
        ));
    }
    serde_json::from_str(line.trim()).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Ask the proxy to switch to a new active profile.
pub fn switch_profile(
    socket_path: &Path,
    base_url: &str,
    api_key: &str,
    model: &str,
) -> io::Result<()> {
    let cmd = ControlCommand {
        cmd: "switch".into(),
        base_url: Some(base_url.into()),
        api_key: Some(api_key.into()),
        model: Some(model.into()),
    };
    let resp = send_control(socket_path, &cmd)?;
    status_to_result(resp)
}

/// Convert a control response into a `Result`: `"ok"` → Ok, anything else →
/// Err carrying the response message.
fn status_to_result(resp: ControlResponse) -> io::Result<()> {
    if resp.status == "ok" {
        Ok(())
    } else {
        Err(io::Error::other(
            resp.message.unwrap_or_else(|| "unknown error".into()),
        ))
    }
}

/// Ask the proxy to shut down.
pub fn shutdown_proxy(socket_path: &Path) -> io::Result<()> {
    let cmd = ControlCommand {
        cmd: "shutdown".into(),
        base_url: None,
        api_key: None,
        model: None,
    };
    let resp = send_control_timeout(socket_path, &cmd, STOP_TIMEOUT)?;
    status_to_result(resp)
}

// ── proxy internals ───────────────────────────────────────────────────────

/// Start the proxy daemon in a new OS thread with its own tokio runtime.
pub fn start_proxy(port: u16, socket_path: PathBuf) -> io::Result<std::thread::JoinHandle<()>> {
    let handle = std::thread::Builder::new()
        .name("cct-proxy".into())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("build proxy tokio runtime");
            rt.block_on(run_proxy(port, &socket_path));
        })?;
    Ok(handle)
}

/// Run the proxy in the foreground (blocking). Used by `cct proxy` subcommand.
pub fn run_foreground(port: u16) -> io::Result<()> {
    let socket_path = proxy_socket_path();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build proxy tokio runtime");
    rt.block_on(run_proxy(port, &socket_path));
    Ok(())
}

/// Print "another live proxy owns the control socket" and exit(1) — never panic.
fn exit_socket_owned(socket_path: &Path) -> ! {
    eprintln!("[cct-proxy] another live proxy owns control socket {socket_path:?} — exiting");
    std::process::exit(1);
}

/// Print the control-socket bind failure and exit(1) — never panic.
fn exit_bind_failed(socket_path: &Path, err: &io::Error) -> ! {
    eprintln!("[cct-proxy] control socket bind {socket_path:?} failed: {err}");
    std::process::exit(1);
}

/// True when `bind` failed because the socket path is already taken: Linux
/// reports EADDRINUSE, macOS/BSD EEXIST（实测双启动竞态本机 macOS 走 EEXIST，
/// os error 17）——两分支同一收敛语义。
fn is_bind_conflict(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::AddrInUse | io::ErrorKind::AlreadyExists
    )
}

async fn run_proxy(port: u16, socket_path: &Path) {
    log_proxy!("starting on 127.0.0.1:{port}, control socket {socket_path:?}");

    let state = Arc::new(ProxyState {
        active: RwLock::new(ActiveProfile::default()),
    });

    // TCP bind 先行——双启动竞态的唯一仲裁者（方向 A，见 findings/
    // double_start_race_one_wins-analysis.md）：败者在 TCP EADDRINUSE 处直接
    // exit(1)（port_conflict_message 诊断），根本走不到控制 bind——不重绑、不删除、
    // 不留下任何 socket 文件。活 proxy 必然持有 TCP，因此控制段探测-删除只会对
    // 真僵尸文件执行（约束 #3/#5 意图）。AC3 占端口行为与消息文本不变。
    let addr = format!("127.0.0.1:{port}");
    let listener = match TcpListener::bind(&addr).await {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("[cct-proxy] TCP bind {addr} failed: {e}");
            eprintln!("[cct-proxy] {}", port_conflict_message(port));
            // exit(1) 而非 panic/return：panic 输出违反占用诊断契约（不 panic + 报错
            // 文本），return 则 `cct proxy start` 静默以 0 退出。
            std::process::exit(1);
        }
    };

    // 先绑后删（delete-on-conflict）：不预先删除路径——预删除可能破坏并发启动者
    // 刚绑定的活 socket（约束 #5 意图：不破坏活 proxy 控制通道）。删除只发生在
    // 探测确认死 socket（僵尸）之后，与父进程 ensure_proxy_running 的试探 bind
    // 同一模式（约束 #3）。TCP 先行后，双启动败者已在上一步退出，本分支只对
    // 僵尸文件/异例（非 proxy 进程占路径）触发——exit_socket_owned 保留作防御。
    let ctl_listener = match TokioUnixListener::bind(socket_path) {
        Ok(l) => l,
        Err(e) if is_bind_conflict(&e) => {
            // 冲突 = 路径被占用：活 proxy → 报错退出（不删其控制通道）；
            // 探测无应答（僵尸 socket）→ 删除后重绑一次。
            if check_proxy_running(socket_path) {
                exit_socket_owned(socket_path);
            }
            let _ = std::fs::remove_file(socket_path);
            match TokioUnixListener::bind(socket_path) {
                Ok(l) => l,
                Err(e2) if is_bind_conflict(&e2) => {
                    // 重绑仍冲突 → 并发竞态加剧 → 重新探测，耗尽报错（保证收敛）。
                    for _ in 0..PROBE_RETRIES {
                        if check_proxy_running(socket_path) {
                            exit_socket_owned(socket_path);
                        }
                        std::thread::sleep(PROBE_TIMEOUT);
                    }
                    exit_bind_failed(socket_path, &e2);
                }
                Err(e2) => exit_bind_failed(socket_path, &e2),
            }
        }
        Err(e) => exit_bind_failed(socket_path, &e),
    };
    log_proxy!("control socket bound");

    let ctl_state = state.clone();
    // 阴影为 owned PathBuf：tokio::spawn 要求 'static（借用 &Path 不能进闭包）。
    let socket_path = socket_path.to_path_buf();
    tokio::spawn(async move {
        // run_control_socket 永不返回（唯一出口是 shutdown 分支的 process::exit，
        // 那里已自行删 socket 文件）——await 后无需清理。
        run_control_socket(ctl_listener, ctl_state, socket_path).await;
    });

    loop {
        let (stream, _peer) = match listener.accept().await {
            Ok(conn) => conn,
            Err(_) => continue,
        };
        let io = TokioIo::new(stream);
        let svc_state = state.clone();

        tokio::spawn(async move {
            let svc = service_fn(move |req| handle_request(req, svc_state.clone()));
            if let Err(e) = http1::Builder::new().serve_connection(io, svc).await {
                let msg = format!("{e}");
                if !msg.contains("connection closed")
                    && !msg.contains("broken pipe")
                    && !msg.contains("Connection reset")
                {
                    eprintln!("[cct-proxy] connection error: {e}");
                }
            }
        });
    }
}

// ── HTTP handler ──────────────────────────────────────────────────────────

type ProxyBody =
    http_body_util::combinators::BoxBody<Bytes, Box<dyn std::error::Error + Send + Sync>>;

async fn handle_request(
    req: Request<Incoming>,
    state: Arc<ProxyState>,
) -> Result<Response<ProxyBody>, hyper::Error> {
    let method = req.method().clone();
    let path_and_query = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/")
        .to_string();

    log_proxy!("<< {method} {}", mask_request_path(&path_and_query));

    if !req.uri().path().starts_with("/v1") {
        log_proxy!(">> 404 (path not /v1)");
        return Ok(plain_response(
            StatusCode::NOT_FOUND,
            "cct proxy — no upstream configured for this path\n",
        ));
    }

    let active = {
        let guard = state.active.read().unwrap();
        if guard.base_url.is_empty() {
            log_proxy!(">> 502 (no active profile)");
            return Ok(plain_response(
                StatusCode::BAD_GATEWAY,
                "cct proxy — no active profile. Launch a profile from cct first.\n",
            ));
        }
        ActiveProfile {
            base_url: guard.base_url.clone(),
            api_key: guard.api_key.clone(),
            model: guard.model.clone(),
        }
    };

    let upstream_url = format!(
        "{}{}",
        active.base_url.trim_end_matches('/'),
        path_and_query
    );

    log_proxy!(
        "-> upstream {method} {} (model={})",
        mask_request_path(&upstream_url),
        active.model
    );

    // Snapshot Content-Type before req is consumed by body collection.
    let content_type = req
        .headers()
        .get("Content-Type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Collect incoming body (consumes req).
    let body_bytes = req
        .collect()
        .await
        .map(|b| b.to_bytes())
        .unwrap_or_default();

    // Forward via reqwest.
    let client = reqwest::Client::new();
    let reqwest_method =
        reqwest::Method::from_bytes(method.as_str().as_bytes()).unwrap_or(reqwest::Method::POST);
    let mut upstream_req = client
        .request(reqwest_method, &upstream_url)
        .body(body_bytes.to_vec());

    if !active.api_key.is_empty() {
        upstream_req = upstream_req.header("Authorization", format!("Bearer {}", active.api_key));
    }

    if let Some(ct) = &content_type {
        upstream_req = upstream_req.header("Content-Type", ct.as_str());
    }

    match upstream_req.send().await {
        Ok(upstream_resp) => {
            let upstream_status = upstream_resp.status().as_u16();
            let status =
                StatusCode::from_u16(upstream_status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

            log_proxy!("<< upstream {upstream_status} (streaming)");

            // Snapshot response headers before streaming the body.
            let headers: Vec<(String, String)> = upstream_resp
                .headers()
                .iter()
                .filter(|(name, _)| name.as_str().to_lowercase() != "transfer-encoding")
                .map(|(name, value)| {
                    (
                        name.as_str().to_string(),
                        value.to_str().unwrap_or("").to_string(),
                    )
                })
                .collect();

            // Stream upstream response body chunk-by-chunk (critical for SSE).
            let byte_stream = upstream_resp.bytes_stream();
            let frame_stream = byte_stream
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                .map_ok(Frame::data);
            let body = StreamBody::new(frame_stream).boxed();

            let mut resp = Response::builder().status(status);
            for (name, value) in &headers {
                resp = resp.header(name.as_str(), value.as_str());
            }
            Ok(resp.body(body).expect("build proxy response"))
        }
        Err(e) => {
            // reqwest errors embed the full request URL (with query) — the log
            // path must run the same sk- value scan as the inbound/outbound lines.
            log_proxy!("<< upstream error: {}", mask_request_path(&format!("{e}")));
            Ok(plain_response(
                StatusCode::BAD_GATEWAY,
                format!("cct proxy — upstream unreachable: {e}\n"),
            ))
        }
    }
}

fn plain_response(status: StatusCode, body: impl Into<String>) -> Response<ProxyBody> {
    let bytes = Bytes::from(body.into().into_bytes());
    let body = Full::new(bytes)
        .map_err(
            |_: std::convert::Infallible| -> Box<dyn std::error::Error + Send + Sync> {
                unreachable!()
            },
        )
        .boxed();
    Response::builder()
        .status(status)
        .header("Content-Type", "text/plain")
        .body(body)
        .expect("build error response")
}

// ── port diagnostics (read-only lsof) ─────────────────────────────────────

/// Read-only diagnosis: PID listening on `port` via lsof. Returns None when
/// lsof is unavailable or nothing is listening (caller falls back to advice text).
pub fn tcp_port_owner(port: u16) -> Option<String> {
    let out = std::process::Command::new("lsof")
        .arg(format!("-tiTCP:{port}"))
        .arg("-sTCP:LISTEN")
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
        None => format!("port {port} already in use. 运行 `lsof -iTCP:{port}` 查看占用者."),
    }
}

// ── unix-socket control handler ───────────────────────────────────────────

async fn run_control_socket(
    listener: TokioUnixListener,
    state: Arc<ProxyState>,
    socket_path: PathBuf,
) {
    loop {
        match listener.accept().await {
            Ok((stream, _peer)) => {
                let ctl_state = state.clone();
                let std_stream = match stream.into_std() {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("[cct-proxy] control stream into_std error: {e}");
                        continue;
                    }
                };
                let socket_path = socket_path.clone();
                tokio::task::spawn_blocking(move || {
                    handle_control(std_stream, ctl_state, socket_path)
                });
            }
            Err(e) => {
                eprintln!("[cct-proxy] control socket accept error: {e}");
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
    }
}

fn handle_control(mut stream: UnixStream, state: Arc<ProxyState>, socket_path: PathBuf) {
    let mut reader = BufReader::new(&stream);
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() || line.trim().is_empty() {
        log_proxy!("ctl << empty command");
        let _ = write_control_response(&mut stream, &error_response("empty command"));
        return;
    }

    let cmd: ControlCommand = match serde_json::from_str(line.trim()) {
        Ok(c) => c,
        Err(e) => {
            log_proxy!("ctl << invalid JSON: {e}");
            let _ =
                write_control_response(&mut stream, &error_response(format!("invalid JSON: {e}")));
            return;
        }
    };

    log_proxy!(
        "ctl << {}",
        mask_ctl_line(line.trim(), cmd.api_key.as_deref())
    );

    match cmd.cmd.as_str() {
        "switch" => {
            let base_url = cmd.base_url.unwrap_or_default();
            let api_key = cmd.api_key.unwrap_or_default();
            let model = cmd.model.unwrap_or_default();
            {
                let mut active = state.active.write().unwrap();
                active.base_url = base_url.clone();
                active.api_key = api_key;
                active.model = model.clone();
            }
            log_proxy!("ctl >> ok (switched to base_url={base_url}, model={model})");
            let _ = write_control_response(
                &mut stream,
                &ControlResponse {
                    status: "ok".into(),
                    message: None,
                    base_url: Some(base_url),
                    model: Some(model),
                },
            );
        }
        "status" => {
            let active = state.active.read().unwrap();
            log_proxy!(
                "ctl >> status (base_url={}, model={})",
                active.base_url,
                active.model
            );
            let _ = write_control_response(
                &mut stream,
                &ControlResponse {
                    status: "ok".into(),
                    message: None,
                    base_url: if active.base_url.is_empty() {
                        None
                    } else {
                        Some(active.base_url.clone())
                    },
                    model: if active.model.is_empty() {
                        None
                    } else {
                        Some(active.model.clone())
                    },
                },
            );
        }
        "shutdown" => {
            log_proxy!("ctl >> ok (shutting down)");
            let _ = write_control_response(
                &mut stream,
                &ControlResponse {
                    status: "ok".into(),
                    message: Some("shutting down".into()),
                    base_url: None,
                    model: None,
                },
            );
            // Unix socket 文件不会随进程退出自动删除，process::exit 又跳过析构：
            // exit 前必须显式清理，否则每次 stop 留下死 socket 文件（约束 #6 稳态缺陷）。
            let _ = std::fs::remove_file(&socket_path);
            std::process::exit(0);
        }
        other => {
            log_proxy!("ctl >> err (unknown command: {other})");
            let _ = write_control_response(
                &mut stream,
                &error_response(format!("unknown command: {other}")),
            );
        }
    }
}

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
    let mut rest = s;
    while let Some((before, after)) = rest.split_once("sk-") {
        out.push_str(before);
        out.push_str("sk-***");
        let token_end = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
            .unwrap_or(after.len());
        rest = &after[token_end..];
    }
    out.push_str(rest);
    out
}

/// Build an error ControlResponse carrying `message`.
fn error_response(message: impl Into<String>) -> ControlResponse {
    ControlResponse {
        status: "err".into(),
        message: Some(message.into()),
        base_url: None,
        model: None,
    }
}

fn write_control_response(stream: &mut UnixStream, resp: &ControlResponse) -> io::Result<()> {
    let mut payload =
        serde_json::to_vec(resp).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    payload.push(b'\n');
    stream.write_all(&payload)?;
    Ok(())
}

// ── tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::os::unix::net::UnixListener;

    #[test]
    fn control_command_parse_switch() {
        let json = r#"{"cmd":"switch","base_url":"https://api.example.com/v1","api_key":"sk-test","model":"gpt-4"}"#;
        let cmd: ControlCommand = serde_json::from_str(json).unwrap();
        assert_eq!(cmd.cmd, "switch");
        assert_eq!(cmd.base_url.as_deref(), Some("https://api.example.com/v1"));
        assert_eq!(cmd.api_key.as_deref(), Some("sk-test"));
        assert_eq!(cmd.model.as_deref(), Some("gpt-4"));
    }

    #[test]
    fn control_command_parse_status() {
        let json = r#"{"cmd":"status"}"#;
        let cmd: ControlCommand = serde_json::from_str(json).unwrap();
        assert_eq!(cmd.cmd, "status");
        assert!(cmd.base_url.is_none());
    }

    #[test]
    fn control_response_serialize_ok() {
        let resp = ControlResponse {
            status: "ok".into(),
            message: None,
            base_url: Some("https://api.example.com/v1".into()),
            model: Some("gpt-4".into()),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"status\":\"ok\""));
        assert!(json.contains("https://api.example.com/v1"));
    }

    #[test]
    fn control_response_serialize_err() {
        let resp = ControlResponse {
            status: "err".into(),
            message: Some("profile not found".into()),
            base_url: None,
            model: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"status\":\"err\""));
        assert!(json.contains("profile not found"));
    }

    #[test]
    fn proxy_port_default() {
        std::env::remove_var("CCT_PROXY_PORT");
        assert_eq!(proxy_port(), 19191);
    }

    #[test]
    fn proxy_port_from_env() {
        std::env::set_var("CCT_PROXY_PORT", "12345");
        assert_eq!(proxy_port(), 12345);
        std::env::remove_var("CCT_PROXY_PORT");
    }

    #[test]
    fn proxy_socket_path_ends_with_proxy_sock() {
        let path = proxy_socket_path();
        assert!(path.ends_with("proxy.sock"), "got: {path:?}");
    }

    #[test]
    fn proxy_socket_path_override() {
        let temp = std::env::temp_dir().join("cct-proxy-test.proxy.sock");
        std::env::set_var("CCT_PROXY_SOCKET", &temp);
        assert_eq!(proxy_socket_path(), temp);
        std::env::remove_var("CCT_PROXY_SOCKET");
        let restored = proxy_socket_path();
        assert!(restored.ends_with("proxy.sock"), "got: {restored:?}");
    }

    fn test_socket(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("cct-proxy-{name}.sock"))
    }

    /// A socket path guaranteed not to exist: a stale socket left behind by a
    /// crashed run would make the "absent" tests false-negative.
    fn fresh_test_socket(name: &str) -> std::path::PathBuf {
        let path = test_socket(name);
        let _ = std::fs::remove_file(&path);
        path
    }

    #[test]
    fn check_proxy_running_false_when_socket_absent() {
        let path = fresh_test_socket("check-proxy-running-absent");
        assert!(
            !check_proxy_running(&path),
            "absent socket must not be reported as running"
        );
    }

    #[test]
    fn check_proxy_running_true_when_daemon_responds() {
        // A live daemon answers the app-level `status` probe with a
        // ControlResponse within the probe timeout.
        let path = fresh_test_socket("check-proxy-running-responds");
        let listener = UnixListener::bind(&path).expect("bind test control socket");
        let handle = std::thread::spawn(move || {
            let (mut stream, _peer) = listener.accept().expect("accept probe connection");
            let mut reader = BufReader::new(&stream);
            let mut line = String::new();
            reader.read_line(&mut line).expect("read probe command");
            assert!(
                !line.trim().is_empty(),
                "app-level probe must send a control command, got EOF"
            );
            let cmd: ControlCommand =
                serde_json::from_str(line.trim()).expect("parse probe command JSON");
            assert_eq!(
                cmd.cmd, "status",
                "probe must send a status command, got: {line}"
            );
            write_control_response(
                &mut stream,
                &ControlResponse {
                    status: "ok".into(),
                    message: None,
                    base_url: None,
                    model: None,
                },
            )
            .expect("write control response");
        });
        assert!(
            check_proxy_running(&path),
            "responding daemon must be reported as running"
        );
        handle.join().expect("responder thread panicked");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn check_proxy_running_false_when_socket_silent() {
        // A dead proxy accepts the connection but never answers: the app-level
        // probe must give up within a bounded time, not hang forever.
        let path = fresh_test_socket("check-proxy-running-silent");
        let listener = UnixListener::bind(&path).expect("bind test control socket");
        let handle = std::thread::spawn(move || {
            let (_stream, _peer) = listener.accept().expect("accept probe connection");
            // Hold the connection open without responding.
            std::thread::sleep(std::time::Duration::from_secs(2));
        });
        let started = std::time::Instant::now();
        assert!(
            !check_proxy_running(&path),
            "silent socket must not be reported as running"
        );
        assert!(
            started.elapsed() < std::time::Duration::from_secs(5),
            "probe must return in bounded time, took {:?}",
            started.elapsed()
        );
        handle.join().expect("silent listener thread panicked");
        let _ = std::fs::remove_file(&path);
    }

    // ── shutdown 2s 超时（Step 9：约束 #1/#10——无响应 proxy 不得挂起、不得吞错）──────

    /// 无响应 socket（accept 后 hold 住不回包）→ shutdown_proxy 必须在 STOP_TIMEOUT
    /// 量级内返回 Err。错误必须传播：旧实现 `let _ = send_control(...)` 吞错返回 Ok。
    #[test]
    fn shutdown_proxy_errs_on_unresponsive_socket() {
        let path = fresh_test_socket("shutdown-proxy-silent");
        let listener = UnixListener::bind(&path).expect("bind test control socket");
        let handle = std::thread::spawn(move || {
            let (_stream, _peer) = listener.accept().expect("accept shutdown connection");
            // Hold the connection open without responding (hung/dead proxy).
            std::thread::sleep(std::time::Duration::from_secs(2));
        });
        let started = std::time::Instant::now();
        let result = shutdown_proxy(&path);
        assert!(
            result.is_err(),
            "shutdown on silent socket must return Err, got: {result:?}"
        );
        assert!(
            started.elapsed() < std::time::Duration::from_secs(3),
            "shutdown must return in bounded time, took {:?}",
            started.elapsed()
        );
        handle.join().expect("silent listener thread panicked");
        let _ = std::fs::remove_file(&path);
    }

    /// 正常应答 `{"status":"ok"}` 的 socket → shutdown_proxy 返回 Ok。
    #[test]
    fn shutdown_proxy_ok_when_daemon_responds() {
        let path = fresh_test_socket("shutdown-proxy-responds");
        let listener = UnixListener::bind(&path).expect("bind test control socket");
        let handle = std::thread::spawn(move || {
            let (mut stream, _peer) = listener.accept().expect("accept shutdown connection");
            let mut reader = BufReader::new(&stream);
            let mut line = String::new();
            reader.read_line(&mut line).expect("read shutdown command");
            let cmd: ControlCommand =
                serde_json::from_str(line.trim()).expect("parse shutdown command JSON");
            assert_eq!(
                cmd.cmd, "shutdown",
                "expected shutdown command, got: {line}"
            );
            write_control_response(
                &mut stream,
                &ControlResponse {
                    status: "ok".into(),
                    message: Some("shutting down".into()),
                    base_url: None,
                    model: None,
                },
            )
            .expect("write control response");
        });
        let result = shutdown_proxy(&path);
        assert!(
            result.is_ok(),
            "shutdown with ok response must succeed, got: {result:?}"
        );
        handle.join().expect("responder thread panicked");
        let _ = std::fs::remove_file(&path);
    }

    // ── 占端口诊断（Step 5：只读 lsof + 降级文本，约束 #4）──────────────

    /// lsof 缺失（PATH 不含 lsof）→ tcp_port_owner 返回 None，
    /// port_conflict_message 降级为定位命令建议文本（含 "lsof -iTCP"）。
    #[test]
    #[serial]
    fn tcp_port_owner_fallback_when_lsof_missing() {
        std::env::set_var("PATH", "/nonexistent");
        let owner = tcp_port_owner(19191);
        std::env::remove_var("PATH");
        assert!(
            owner.is_none(),
            "tcp_port_owner must return None when lsof is not on PATH, got: {owner:?}"
        );
        let msg = port_conflict_message(19191);
        assert!(
            msg.contains("lsof -iTCP"),
            "degraded message must suggest the locating command, got: {msg}"
        );
    }

    /// lsof 可用且端口有 LISTEN 占用者 → 返回 PID，消息含 "PID"。
    /// 环境敏感（CI 可能无 lsof）：无 lsof 时跳过，缺失场景由上一测试覆盖。
    #[test]
    #[serial]
    fn tcp_port_owner_reports_pid_when_lsof_available() {
        if std::process::Command::new("lsof")
            .arg("-v")
            .output()
            .is_err()
        {
            return;
        }
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind test port");
        let port = listener.local_addr().expect("test port").port();
        let owner = tcp_port_owner(port);
        assert!(
            owner.is_some(),
            "lsof present + listener on port {port} must yield a PID"
        );
        let msg = port_conflict_message(port);
        assert!(
            msg.contains("PID"),
            "owner message must include PID, got: {msg}"
        );
    }

    // ── 控制命令与请求日志脱敏（Step 8：约束 #7，mask-secrets-on-every-display-path）──────

    /// api_key 为 sk- 前缀形态：按字段名脱敏，行内所有出现掩码为 ***。
    #[test]
    fn mask_ctl_line_masks_sk_prefix_api_key() {
        let line =
            r#"{"cmd":"switch","base_url":"https://api.example.com/v1","api_key":"sk-abc123"}"#;
        let masked = mask_ctl_line(line, Some("sk-abc123"));
        assert!(
            !masked.contains("sk-abc123"),
            "plaintext api_key must be gone, got: {masked}"
        );
        assert!(
            masked.contains("\"api_key\":\"***\""),
            "api_key field value must be masked, got: {masked}"
        );
    }

    /// api_key 无 sk- 前缀（自定义 token 形态）：同样按字段名脱敏为 ***，
    /// 不依赖值形态——这是按字段名脱敏的核心断言。
    #[test]
    fn mask_ctl_line_masks_custom_token_api_key() {
        let line = r#"{"cmd":"switch","base_url":"https://api.example.com/v1","api_key":"custom-token-xyz"}"#;
        let masked = mask_ctl_line(line, Some("custom-token-xyz"));
        assert!(
            !masked.contains("custom-token-xyz"),
            "plaintext custom token must be gone, got: {masked}"
        );
        assert!(
            masked.contains("\"api_key\":\"***\""),
            "custom token api_key field value must be masked, got: {masked}"
        );
    }

    /// 无 api_key（如 status 命令）：原样返回。
    #[test]
    fn mask_ctl_line_no_key_passthrough() {
        let line = r#"{"cmd":"status"}"#;
        assert_eq!(mask_ctl_line(line, None), line);
    }

    /// 请求路径含 ?key=sk-xyz：sk- 值掩码为 sk-***，无明文。
    #[test]
    fn mask_request_path_masks_query_key() {
        let path = "/v1/messages?key=sk-xyz";
        let masked = mask_request_path(path);
        assert!(
            masked.contains("sk-***"),
            "sk- value must be masked, got: {masked}"
        );
        assert!(
            !masked.contains("sk-xyz"),
            "plaintext sk- value must be gone, got: {masked}"
        );
    }

    /// sk- 值含 -/_ 分隔字符（如 sk-ab_c-d）：整体掩码。
    #[test]
    fn mask_request_path_masks_key_with_separators() {
        let path = "/v1/messages?key=sk-ab_c-d";
        let masked = mask_request_path(path);
        assert!(
            masked.contains("sk-***"),
            "sk- value must be masked, got: {masked}"
        );
        assert!(
            !masked.contains("sk-ab_c-d"),
            "plaintext sk- value must be gone, got: {masked}"
        );
    }

    /// 非 sk- 内容原样保留。
    #[test]
    fn mask_request_path_preserves_non_secret() {
        let path = "/v1/messages?model=gpt-4";
        assert_eq!(mask_request_path(path), path);
    }
}
