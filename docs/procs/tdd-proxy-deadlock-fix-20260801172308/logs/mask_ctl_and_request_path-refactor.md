---
title: "mask_ctl_and_request_path — Refactor Phase"
brief: "mask_ctl_and_request_path — Refactor: exit 0"
doc_type: proc
created: 2026-08-01T10:02:50Z
case: "mask_ctl_and_request_path"
phase: refactor
---
Changes made: `src/proxy.rs` 的 `mask_request_path` 由手写字节扫描改写为基于 `str::split_once` + 谓词 `find` 的标准库实现。原实现用 `s.as_bytes()` + 逐字节 `while` 循环：每步检查 `starts_with(b"sk-")`，命中则消耗 token 字符集（ASCII 字母数字、`-`、`_`）后输出 `sk-***`，否则 `out.push(bytes[i] as char)` 逐字节拷贝。新实现用 `split_once("sk-")` 定位每个 `sk-` 标记，`before` 原样保留、标记输出 `sk-***`，再用 `after.find(|c| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))` 找到 token 末尾并跳过，循环直至无更多标记。脱敏语义逐字符等价：token 字符集完全相同（`is_ascii_alphanumeric | '-' | '_'`），每个 `sk-` 值（含 `-`/`_` 分隔形态，如 `sk-ab_c-d`）整体掩码为 `sk-***`，非 `sk-` 内容原样保留——任何 sk- 明文不可能泄漏，约束 #7 不弱化。附带修复：旧实现 `push(bytes[i] as char)` 会把多字节 UTF-8 的续字节按 Latin-1 单个字符推入，非 ASCII 文本会变 mojibake；新实现全程在 `str` 边界上切片，任意非 ASCII 内容原样保留（`http::Uri` 实际仅接受 ASCII，真实请求路径不会触发，但纯函数本身现在对任意 `&str` 正确）。`mask_ctl_line` 与两处应用点（`handle_request` 入站行、`handle_control` 控制行）未改动。

具体观察：
- `mask_ctl_line` 已是最简：`match api_key { Some(key) if !key.is_empty() => line.replace(key, "***"), _ => line.to_string() }`。注意 `replace` 会掩码整行中该 key 值的每一次出现（若 key 恰为 `base_url` 或 `model` 子串也会被掩码）——是比"仅字段名定位"更强的脱敏，非弱化，保留。命名字面清晰（ctl = control），注释保留约束 #7 上下文，无改动。
- 命名：`mask_ctl_line`（按字段名）与 `mask_request_path`（无字段名可依，值前缀扫描兜底）策略不同、各有注释说明，无重复逻辑可合并，均无死代码（各恰有一处应用点）。
- 复杂条件：新 `mask_request_path` 消除了逐字节索引算术（`i += 3` / 嵌套 while）；`find` 谓词单表达式。对不含 `sk-` 的常见路径（如 `/v1/messages?model=gpt-4`），`split_once` 一次未命中即原样返回，性能优于逐字节扫描。
- 潜在泄漏观察（超出本次 case 范围，未改动）：`handle_request` 的出站日志 `log_proxy!("-> upstream {method} {upstream_url} (model={})", ...)` 打印的 `upstream_url` 含完整 `path_and_query` 且未脱敏——若请求目标携带 `?key=sk-...`，入站行（`<< {method} {mask_request_path(...)}`）已掩码，但该出站行会泄出明文 token，违反 mask-secrets-on-every-display-path。建议后续 case 覆盖此应用点（本次按 surgical edits 规则不动）。
test_cmd exit code: 0
output: `rtk proxy cargo test mask_`（工作树根目录执行；rtk 会压缩 cargo 输出，已用 `rtk proxy` 绕过过滤器保留完整日志。另跑完整 `cargo test` 全量确认 exit 0，无回归。完整输出如下）

```
    Blocking waiting for file lock on artifact directory
   Compiling ring v0.17.14
   Compiling rustls v0.23.41
   Compiling rustls-webpki v0.103.13
   Compiling tokio-rustls v0.26.4
   Compiling hyper-rustls v0.27.9
   Compiling reqwest v0.12.28
   Compiling cct v0.5.0 (/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 8.03s
     Running unittests src/lib.rs (target/debug/deps/cct-c695630c4374c597)

running 10 tests
test proxy::tests::mask_ctl_line_no_key_passthrough ... ok
test ui::tests::mask_api_key ... ok
test ui::tests::mask_auth_token ... ok
test proxy::tests::mask_request_path_preserves_non_secret ... ok
test ui::tests::mask_secret ... ok
test proxy::tests::mask_request_path_masks_query_key ... ok
test proxy::tests::mask_request_path_masks_key_with_separators ... ok
test ui::tests::no_mask_url ... ok
test proxy::tests::mask_ctl_line_masks_custom_token_api_key ... ok
test proxy::tests::mask_ctl_line_masks_sk_prefix_api_key ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 135 filtered out; finished in 0.00s

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
（全量 `cargo test` 亦 exit 0：lib 145 测试 + main 19 + integration 5 + live 4 全绿。）
