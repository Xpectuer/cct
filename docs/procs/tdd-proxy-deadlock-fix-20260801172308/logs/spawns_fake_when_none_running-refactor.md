---
title: "spawns_fake_when_none_running — Refactor Phase"
brief: "spawns_fake_when_none_running — Refactor: exit 0"
doc_type: proc
created: 2026-08-01T10:20:17Z
case: "spawns_fake_when_none_running"
phase: refactor
---
Changes made: `src/launch.rs` `ensure_proxy_running` 四处清理，行为零变化：

1. 删除 `std::env::var("CCT_PROXY_BIN").map(String::from)` 中的 `.map(String::from)`——`env::var` 本身返回 `Result<String, VarError>`，该映射是恒等函数，属死操作。
2. 路径风格统一：`std::process::Command` / `std::env::var` / `std::fs::File` 改为文件顶部已导入的 `Command` / `env` / `fs`（`std::thread::sleep` 与 `std::net::TcpListener` 未导入，保留全限定）。
3. stderr 处理由嵌套 if/else 拍平为同一层级的两分支：`if env::var("CCT_PROXY_LOG").is_err() { cmd.stderr(null) } else if let Ok(file) = fs::File::create(...) { cmd.stderr(file) }`。三分支行为逐一等价：env 未设 → null；env 已设且建文件成功 → file；env 已设但建文件失败 → stderr 保持默认 inherit（原代码同样如此，此边界情形未变）。
4. 变量 `exe` 更名为 `proxy_bin`（语义更明确）；doc 注释补充 CCT_PROXY_BIN 注入约定（spawn 目标取自该 env、测试可注入 fake），与测试文件契约对齐。

`tests/launch_proxy_contract.rs` 未改动。

具体观察（tests/launch_proxy_contract.rs）：
- 命名清晰：`free_port` / `write_fake_proxy` / `restore_env` 语义直接，assert 消息（"READY 标记缺失：fake 未被 CCT_PROXY_BIN 启动（注入未生效）"）在失败时能定位违约面。
- 重复逻辑：`free_port` 与 `tests/proxy_contract.rs` 同名 helper 重复，但注释已显式声明"同约定"；两个测试文件共享一个 4 行 helper 不足以支撑提取 `tests/common/mod.rs`（KISS：单次使用逻辑内联）。
- env 保存/恢复：4 个 save + 4 个 restore 是显式展开而非循环，与仓库既有风格（`claude_default_env_is_injected` 同样显式恢复）一致；契约测试以可读性优先，维持显式。
- `restore_env` 无 Drop guard：若断言失败则 env 残留。`#[serial]` + 进程隔离（独立测试进程）使残留无害，与仓库既有测试模式一致，不加抽象。
- 复杂条件：无。fake 脚本的 `while os.path.exists(sock)` + settimeout 循环是"socket 被 TempDir 清理后自终止"这一非显而易见行为的必要实现，注释已说明。

死代码：无。`ensure_proxy_running` 中 `CCT_PROXY_LOG` 设置但建文件失败的 inherit 分支无法用测试覆盖（无 spawn 后读取 stderr 的路径），但保留以维持既有行为。

核心不变量核验：CCT_PROXY_BIN 注入（`env::var("CCT_PROXY_BIN")` 取值逻辑未动）、端口预检（`TcpListener::bind(("127.0.0.1", port))` 失败即 bail）、就绪探测（`PROBE_RETRIES` 次 `check_proxy_running` + `PROBE_TIMEOUT` sleep 循环原样保留）均未变弱。
test_cmd exit code: 0
output: `rtk proxy cargo test --test launch_proxy_contract`（工作树根目录执行；rtk 对 cargo 输出做摘要压缩，已用 `rtk proxy` 绕过过滤器恢复完整日志，完整输出如下）

```
    Blocking waiting for file lock on artifact directory
   Compiling ring v0.17.14
   Compiling rustls v0.23.41
   Compiling rustls-webpki v0.103.13
   Compiling tokio-rustls v0.26.4
   Compiling hyper-rustls v0.27.9
   Compiling reqwest v0.12.28
   Compiling cct v0.5.0 (/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 9.16s
     Running tests/launch_proxy_contract.rs (target/debug/deps/launch_proxy_contract-11aa5be2c88374b7)

running 1 test
test spawns_fake_when_none_running ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.51s

EXIT_CODE=0
```
