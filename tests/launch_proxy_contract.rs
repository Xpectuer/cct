//! ensure_proxy_running 重启契约（CCT_PROXY_BIN 注入 fake 目标）。
//!
//! 与用户实例隔离：全部走 CCT_PROXY_SOCKET / CCT_PROXY_PORT / CCT_PROXY_BIN env
//! （约束 #8、#10），fake spawn 目标仿 CCT_CLAUDE_BIN 注入先例。

use std::path::{Path, PathBuf};

use cct::launch::ensure_proxy_running;
use serial_test::serial;

/// 动态空闲端口：bind 0 取端口后 drop（与 tests/proxy_contract.rs free_port 同约定）。
fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind ephemeral port");
    listener.local_addr().expect("local addr").port()
}

/// fake spawn 目标：启动时 rm 残留 socket、写 READY 标记，随后经 python3 循环
/// accept 控制 socket 并应答 `{"status":"ok"}`（应用层探测协议，见 proxy.rs
/// check_proxy_running 的 ControlResponse 格式）。READY 标记路径经
/// CCT_PROXY_READY_MARKER 传入——断言 fake 确实被 ensure_proxy_running
/// 经 CCT_PROXY_BIN 拉起。socket 文件被删（TempDir 清理）后 python 循环
/// 自终止，不留孤儿进程。
fn write_fake_proxy(dir: &Path) -> PathBuf {
    let script = dir.join("fake-proxy.sh");
    std::fs::write(
        &script,
        r#"#!/bin/bash
set -e
SOCK="${CCT_PROXY_SOCKET:?}"
READY="${CCT_PROXY_READY_MARKER:?}"
rm -f "$SOCK"
: > "$READY"
exec python3 - "$SOCK" <<'PY'
import os
import socket
import sys

sock = sys.argv[1]
server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
server.bind(sock)
server.listen(16)
server.settimeout(0.5)
while os.path.exists(sock):
    try:
        conn, _ = server.accept()
    except socket.timeout:
        continue
    except OSError:
        break
    try:
        data = b""
        while b"\n" not in data:
            chunk = conn.recv(4096)
            if not chunk:
                break
            data += chunk
        conn.sendall(b'{"status":"ok"}\n')
    except OSError:
        pass
    finally:
        conn.close()
PY
"#,
    )
    .unwrap();
    let mut perms = std::fs::metadata(&script).unwrap().permissions();
    use std::os::unix::fs::PermissionsExt;
    perms.set_mode(0o755);
    std::fs::set_permissions(&script, perms).unwrap();
    script
}

/// 代理相关 env（CCT_PROXY_BIN/SOCKET/PORT/READY_MARKER）的保存、覆写与恢复。
/// Drop 时恢复原值——断言失败/panic 也恢复，serial 测试之间不泄漏 env。
struct ProxyEnvGuard {
    prev_bin: Option<String>,
    prev_socket: Option<String>,
    prev_port: Option<String>,
    prev_ready: Option<String>,
}

impl ProxyEnvGuard {
    fn set(fake: &Path, socket: &Path, port: u16, ready: &Path) -> Self {
        let guard = Self {
            prev_bin: std::env::var("CCT_PROXY_BIN").ok(),
            prev_socket: std::env::var("CCT_PROXY_SOCKET").ok(),
            prev_port: std::env::var("CCT_PROXY_PORT").ok(),
            prev_ready: std::env::var("CCT_PROXY_READY_MARKER").ok(),
        };
        std::env::set_var("CCT_PROXY_BIN", fake);
        std::env::set_var("CCT_PROXY_SOCKET", socket);
        std::env::set_var("CCT_PROXY_PORT", port.to_string());
        std::env::set_var("CCT_PROXY_READY_MARKER", ready);
        guard
    }
}

impl Drop for ProxyEnvGuard {
    fn drop(&mut self) {
        for (key, prev) in [
            ("CCT_PROXY_BIN", self.prev_bin.take()),
            ("CCT_PROXY_SOCKET", self.prev_socket.take()),
            ("CCT_PROXY_PORT", self.prev_port.take()),
            ("CCT_PROXY_READY_MARKER", self.prev_ready.take()),
        ] {
            match prev {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

/// 测试夹具：在临时目录中建立 fake proxy 脚本与代理 env（CCT_PROXY_BIN/SOCKET/
/// PORT/READY_MARKER），返回 (fake, socket, ready, port, env guard)。guard 必须
/// 存活至测试结束（Drop 时恢复 env）。端口由调用方指定——占用端口与空闲端口
/// 场景共用同一套 env 建立逻辑。
fn setup_proxy_env_with_port(
    dir: &Path,
    port: u16,
) -> (PathBuf, PathBuf, PathBuf, u16, ProxyEnvGuard) {
    let fake = write_fake_proxy(dir);
    let socket = dir.join("proxy.sock");
    let ready = dir.join("fake.ready");
    let guard = ProxyEnvGuard::set(&fake, &socket, port, &ready);
    (fake, socket, ready, port, guard)
}

/// 同 setup_proxy_env_with_port，但端口取动态空闲端口（free_port）。
fn setup_proxy_env(dir: &Path) -> (PathBuf, PathBuf, PathBuf, u16, ProxyEnvGuard) {
    setup_proxy_env_with_port(dir, free_port())
}

/// 等待 fake 就绪（READY 标记 + 应用层探测通过）。等待避免"标记已写但 socket 未
/// bind"的竞态误判；5s 超时 panic，消息含 pid。子进程回收（kill/wait）由调用方负责。
fn wait_fake_ready(child: &std::process::Child, socket: &Path, ready: &Path) {
    let pid = child.id();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if ready.exists() && cct::proxy::check_proxy_running(socket) {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "fake 未在 5s 内就绪（pid={pid}）"
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

/// 无 proxy 运行 → ensure_proxy_running 必须经 CCT_PROXY_BIN 拉起 fake，
/// 就绪探测通过后返回 Ok，fake 可应答应用层 status 探测。
#[test]
#[serial]
fn spawns_fake_when_none_running() {
    let dir = tempfile::tempdir().unwrap();
    let (_fake, socket, ready, port, _proxy_env) = setup_proxy_env(dir.path());

    let result = ensure_proxy_running(port, &socket);

    assert!(
        result.is_ok(),
        "ensure_proxy_running 必须经 CCT_PROXY_BIN 拉起 fake 并返回 Ok: {result:?}"
    );
    assert!(
        ready.exists(),
        "READY 标记缺失：fake 未被 CCT_PROXY_BIN 启动（注入未生效）"
    );
    assert!(
        cct::proxy::check_proxy_running(&socket),
        "fake 必须能应答应用层 status 探测"
    );
}

/// 已有一个活的 fake proxy（手动拉起、就绪且健康）→ ensure_proxy_running 必须
/// 复用：探测命中即 Ok，不得再次 spawn。核心断言是**进程未重启**——原 fake
/// 进程仍存活（若实现盲目重启，新 fake 会 `rm -f $SOCK` 使原 fake 的 accept
/// 循环退出），且 READY 标记 mtime 未被重写（fake 仅在启动时 touch 一次）。
#[test]
#[serial]
fn reuses_live_proxy() {
    let dir = tempfile::tempdir().unwrap();
    let (fake, socket, ready, port, _proxy_env) = setup_proxy_env(dir.path());
    let mut proxy_child = std::process::Command::new(&fake)
        .spawn()
        .expect("手动 spawn fake proxy");
    wait_fake_ready(&proxy_child, &socket, &ready);

    let mtime_before = std::fs::metadata(&ready)
        .expect("READY 标记存在")
        .modified()
        .expect("READY 标记 mtime");

    let result = ensure_proxy_running(port, &socket);

    assert!(
        result.is_ok(),
        "活 proxy 在场时 ensure_proxy_running 必须复用返回 Ok: {result:?}"
    );
    // 核心断言（PID 级）：原 fake 进程仍存活——若实现盲目重启，新 fake 会
    // `rm -f $SOCK` 终止原 fake 的 accept 循环，进程随之退出。
    assert!(
        proxy_child.try_wait().unwrap().is_none(),
        "fake 已退出：ensure_proxy_running 重启了活 proxy（原 fake 的 socket 被新实例删除）"
    );
    // READY 标记未被重写：fake 只在启动时写一次，重启会 touch 出新 mtime。
    let mtime_after = std::fs::metadata(&ready)
        .expect("READY 标记存在")
        .modified()
        .expect("READY 标记 mtime");
    assert_eq!(
        mtime_after, mtime_before,
        "READY 标记被重写：ensure_proxy_running 再次 spawn 了 fake"
    );

    let _ = proxy_child.kill();
    let _ = proxy_child.wait();
}

/// 僵尸场景（AC2 重启）：旧 proxy 被 SIGKILL、socket 文件残留 → 应用层探测失败 →
/// ensure_proxy_running 必须经 CCT_PROXY_BIN 重新 spawn fake，就绪后恢复健康。
#[test]
#[serial]
fn zombie_socket_triggers_restart() {
    let dir = tempfile::tempdir().unwrap();
    let (fake, socket, ready, port, _proxy_env) = setup_proxy_env(dir.path());
    let mut old_proxy = std::process::Command::new(&fake)
        .spawn()
        .expect("手动 spawn fake proxy");
    wait_fake_ready(&old_proxy, &socket, &ready);

    // SIGKILL 旧 fake：无清理路径可执行 → socket 文件残留（僵尸场景）。
    old_proxy.kill().expect("SIGKILL fake proxy");
    old_proxy.wait().expect("等 fake 退出");
    assert!(
        socket.exists(),
        "前置：SIGKILL 后 socket 文件必须残留（僵尸场景）"
    );

    // 删掉 READY 标记：重新 spawn 的唯一证据（fake 仅在启动时 touch 一次）。
    std::fs::remove_file(&ready).expect("移除 READY 标记");

    // 应用层探测必须失败：socket 残留但无进程应答（connect 拒绝）。
    assert!(
        !cct::proxy::check_proxy_running(&socket),
        "前置：残留 socket 无响应 → check_proxy_running 应为 false"
    );

    let result = ensure_proxy_running(port, &socket);

    assert!(
        result.is_ok(),
        "僵尸 socket 场景 ensure_proxy_running 必须重启 fake 并返回 Ok: {result:?}"
    );
    // 重启证据：READY 标记被重新 touch（fake 被重新 spawn），且恢复健康。
    assert!(
        ready.exists(),
        "READY 标记缺失：fake 未被重新 spawn（重启路径未生效）"
    );
    assert!(
        cct::proxy::check_proxy_running(&socket),
        "重启后 fake 必须恢复健康（能应答应用层 status 探测）"
    );
}

/// 就绪耗尽：CCT_PROXY_BIN 指向立即退出的脚本（不监听）→ ensure_proxy_running
/// 就绪探测（PROBE_TIMEOUT 500ms × PROBE_RETRIES 3）耗尽后必须返回 Err，
/// ≤2s 返回、不挂起——否则 spawn 永不健康的 proxy 会卡死 codex 启动链路。
#[test]
#[serial]
fn probe_exhaustion_reports_error() {
    let dir = tempfile::tempdir().unwrap();
    let fake = dir.path().join("exit-immediately.sh");
    std::fs::write(&fake, "#!/bin/bash\nexit 0\n").unwrap();
    let mut perms = std::fs::metadata(&fake).unwrap().permissions();
    use std::os::unix::fs::PermissionsExt;
    perms.set_mode(0o755);
    std::fs::set_permissions(&fake, perms).unwrap();
    let socket = dir.path().join("proxy.sock");
    let ready = dir.path().join("unused.ready");
    let port = free_port();
    let _proxy_env = ProxyEnvGuard::set(&fake, &socket, port, &ready);

    let start = std::time::Instant::now();
    let result = ensure_proxy_running(port, &socket);
    let elapsed = start.elapsed();

    assert!(
        result.is_err(),
        "立即退出的 CCT_PROXY_BIN 目标必须令 ensure_proxy_running 返回 Err: {result:?}"
    );
    let msg = format!("{}", result.unwrap_err());
    assert!(
        msg.contains("did not become healthy"),
        "错误信息必须指明就绪探测耗尽（含 \"did not become healthy\"）: {msg}"
    );
    assert!(
        elapsed <= std::time::Duration::from_secs(2),
        "就绪耗尽必须 ≤2s 返回（实际 {elapsed:?}），不得挂起"
    );
}

/// 端口被占：测试进程 bind 一个动态端口并**保持监听**（listener 存活至测试
/// 结束）→ ensure_proxy_running 必须直接返回 Err（错误信息含
/// "port {port} already in use" 占用诊断），**不得 spawn** CCT_PROXY_BIN
/// 目标——否则会在他人占用的端口上强行拉起 proxy。
#[test]
#[serial]
fn port_occupied_bails_with_diagnosis() {
    let dir = tempfile::tempdir().unwrap();
    // 保持监听使端口持续被占：free_port() 会立即释放，制造不了冲突现场。
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind ephemeral port");
    let port = listener.local_addr().expect("local addr").port();
    let (_fake, socket, ready, _, _proxy_env) = setup_proxy_env_with_port(dir.path(), port);

    let result = ensure_proxy_running(port, &socket);

    assert!(
        result.is_err(),
        "端口被占时 ensure_proxy_running 必须返回 Err: {result:?}"
    );
    let msg = format!("{}", result.unwrap_err());
    let needle = format!("port {port} already in use");
    assert!(
        msg.contains(&needle),
        "错误信息必须含占用诊断（\"{needle}\"）: {msg}"
    );
    // 未 spawn 证据：fake 仅在启动时 touch READY 标记——标记不存在即未被拉起。
    assert!(
        !ready.exists(),
        "READY 标记存在：端口被占时 ensure_proxy_running 仍 spawn 了 fake"
    );
}
