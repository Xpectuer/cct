---
title: "proxy_socket_path_override — Refactor Phase"
brief: "proxy_socket_path_override — Refactor: exit 0"
doc_type: proc
created: 2026-08-01T09:41:23Z
case: "proxy_socket_path_override"
phase: refactor
---
Changes made: 仅补充 `proxy_socket_path()` 的 doc comment 提及 `CCT_PROXY_SOCKET` 覆盖（与 `proxy_port()` 的 "Override with `CCT_PROXY_PORT`." 及 `kimi_config_path()` 的覆盖文档惯例对齐）。函数体未改动——它已经与代码库既定 env-override 模式逐字一致（`config_path()` 的 `CCT_CONFIG`、`kimi_config_path()` 的 `CCT_KIMI_CONFIG` 均为同一形状：`if let Ok(p) = env::var(...)` 提前返回 + `dirs::config_dir()` 回退），改写成 `.ok().map().unwrap_or_else()` 链反而会偏离主导风格。测试 `proxy_socket_path_override` 未改动。

具体观察：
- 重复逻辑：`dirs::config_dir().unwrap_or_else(|| PathBuf::from("~/.config")).join("cc-tui")` 前缀在全仓库出现 3 次（`config.rs::config_path`、`proxy.rs::proxy_socket_path`、`proxy.rs::proxy_log_path`），恰好达到 KISS 规则"三处重复才考虑提取"的阈值。但提取共享 helper 会跨模块改动 `config.rs` 和 `proxy_log_path`，超出本用例 scope，留作观察项，不在本 phase 处理。
- 死代码 / 复杂条件：无。函数为单一提前返回 + 单一回退链，无冗余分支。
- 测试健壮性：`proxy_socket_path_override` 的临时路径以 `proxy.sock` 结尾，与并行测试 `proxy_socket_path_ends_with_proxy_sock` 的断言碰撞安全；且测试自证 env 清理后回退路径恢复，无需改动。
- 风格一致性：全仓 4 个 env 覆盖函数中 3 个（config_path / kimi_config_path / proxy_socket_path）使用 `if let Ok` 提前返回形态，仅 `proxy_port()` 用链式（因其需要 `.parse()` 转换，链式合理），故保持现状。
test_cmd exit code: 0
output: `cargo test proxy_socket_path`（工作树根目录执行，完整输出如下）

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.09s
     Running unittests src/lib.rs (target/debug/deps/cct-c695630c4374c597)

running 2 tests
test proxy::tests::proxy_socket_path_override ... ok
test proxy::tests::proxy_socket_path_ends_with_proxy_sock ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 132 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/debug/deps/cct-a415d99fd63277d3)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 19 filtered out; finished in 0.00s

     Running tests/integration.rs (target/debug/deps/integration-7dbc2c9fd2903748)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s

     Running tests/live.rs (target/debug/deps/live-080beb1530291df0)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.00s
```
