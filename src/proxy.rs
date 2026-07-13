use std::io::{self, BufRead, BufReader, Write};
use std::net::Shutdown;
use std::os::unix::net::{UnixListener, UnixStream};
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
use tokio::net::TcpListener;

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

/// Path to the Unix domain socket used for control commands.
pub fn proxy_socket_path() -> PathBuf {
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

/// Check whether the proxy daemon is currently running.
pub fn check_proxy_running(socket_path: &Path) -> bool {
    UnixStream::connect(socket_path).is_ok()
}

/// Send a JSON control command to the proxy and return the response.
pub fn send_control(socket_path: &Path, cmd: &ControlCommand) -> io::Result<ControlResponse> {
    let mut stream = UnixStream::connect(socket_path)?;
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
    let _ = send_control(socket_path, &cmd);
    Ok(())
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

async fn run_proxy(port: u16, socket_path: &Path) {
    log_proxy!("starting on 127.0.0.1:{port}, control socket {socket_path:?}");

    let state = Arc::new(ProxyState {
        active: RwLock::new(ActiveProfile::default()),
    });

    let _ = std::fs::remove_file(socket_path);

    let ctl_listener = UnixListener::bind(socket_path).expect("bind proxy control socket");
    log_proxy!("control socket bound");

    let ctl_state = state.clone();
    let ctl_path = socket_path.to_path_buf();
    tokio::spawn(async move {
        run_control_socket(ctl_listener, ctl_state).await;
        let _ = std::fs::remove_file(&ctl_path);
    });

    let addr = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("proxy bind {addr}: {e}"));

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

    log_proxy!("<< {method} {path_and_query}");

    if req.uri().path().is_empty() || !req.uri().path().starts_with("/v1") {
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
        "-> upstream {method} {upstream_url} (model={})",
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
            log_proxy!("<< upstream error: {e}");
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

// ── unix-socket control handler ───────────────────────────────────────────

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

fn handle_control(mut stream: UnixStream, state: Arc<ProxyState>) {
    let mut reader = BufReader::new(&stream);
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() || line.trim().is_empty() {
        log_proxy!("ctl << empty command");
        let _ = write_control_response(
            &mut stream,
            &ControlResponse {
                status: "err".into(),
                message: Some("empty command".into()),
                base_url: None,
                model: None,
            },
        );
        return;
    }

    let cmd: ControlCommand = match serde_json::from_str(line.trim()) {
        Ok(c) => c,
        Err(e) => {
            log_proxy!("ctl << invalid JSON: {e}");
            let _ = write_control_response(
                &mut stream,
                &ControlResponse {
                    status: "err".into(),
                    message: Some(format!("invalid JSON: {e}")),
                    base_url: None,
                    model: None,
                },
            );
            return;
        }
    };

    log_proxy!("ctl << {}", line.trim());

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
            std::process::exit(0);
        }
        other => {
            log_proxy!("ctl >> err (unknown command: {other})");
            let _ = write_control_response(
                &mut stream,
                &ControlResponse {
                    status: "err".into(),
                    message: Some(format!("unknown command: {other}")),
                    base_url: None,
                    model: None,
                },
            );
        }
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
}
