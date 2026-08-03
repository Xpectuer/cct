---
title: "log_masks_api_key — Red Phase"
brief: "log_masks_api_key — Red: exit 101"
doc_type: proc
created: 2026-08-01T10:38:31Z
case: "log_masks_api_key"
phase: red
---
Exit code: 101
Full output: `cargo test --test proxy_contract log_masks_api_key`（工作树根目录执行，`command cargo` 原始输出，完整如下）

```
   Compiling ring v0.17.14
   Compiling rustls v0.23.41
   Compiling rustls-webpki v0.103.13
   Compiling tokio-rustls v0.26.4
   Compiling hyper-rustls v0.27.9
   Compiling reqwest v0.12.28
   Compiling cct v0.5.0 (/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 6.12s
     Running tests/proxy_contract.rs (target/debug/deps/proxy_contract-37538df7d2e1326e)

running 1 test
test log_masks_api_key ... FAILED

failures:

---- log_masks_api_key stdout ----

thread 'log_masks_api_key' (6678802) panicked at tests/proxy_contract.rs:442:5:
stderr must not contain the request query secret plaintext, got:
[cct-proxy] starting on 127.0.0.1:60944, control socket "/var/folders/8t/7x4hxj395mv4fzw_hf0jd29m0000gn/T/.tmpeRRwIt/proxy.sock"
[cct-proxy] control socket bound
[cct-proxy] ctl << {"cmd":"status","base_url":null,"api_key":null,"model":null}
[cct-proxy] ctl >> status (base_url=, model=)
[cct-proxy] ctl << {"cmd":"switch","base_url":"http://127.0.0.1:60943","api_key":"***","model":null}
[cct-proxy] ctl >> ok (switched to base_url=http://127.0.0.1:60943, model=)
[cct-proxy] << GET /v1/models?key=sk-***
[cct-proxy] -> upstream GET http://127.0.0.1:60943/v1/models?key=sk-xyz-query (model=)
[cct-proxy] << upstream 200 (streaming)

stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/panicking.rs:80:14
   2: proxy_contract::log_masks_api_key::{{closure}}
             at ./tests/proxy_contract.rs:442:5
   3: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
   4: serial_test::serial_code_lock::local_serial_core
             at /Users/zhengjiaye/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/serial_test-3.4.0/src/serial_code_lock.rs:36:5
   5: proxy_contract::log_masks_api_key
             at ./tests/proxy_contract.rs:412:1
   6: proxy_contract::log_masks_api_key::{{closure}}
             at ./tests/proxy_contract.rs:413:23
   7: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
   8: core::ops::function::FnOnce::call_once
             at /rustc/4a4ef493e3a1488c6e321570238084b38948f6db/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.


failures:
    log_masks_api_key

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.18s

error: test failed, to rerun pass `--test proxy_contract`
```

## 分析：真实 Red（outbound 日志泄漏 sk- 明文）

测试在 `tests/proxy_contract.rs:442`（`sk-xyz-query` 明文断言）失败，失败证据正是任务预测的泄漏面。捕获到的 stderr 全文逐行核对：

| 日志行 | 路径 | 脱敏状态 |
|--------|------|---------|
| `ctl << {"cmd":"switch",...,"api_key":"***",...}` | handle_control ctl（src/proxy.rs:477） | ✅ 已脱敏（TC-4 mask_ctl_line） |
| `ctl >> ok (switched to base_url=..., model=)` | handle_control 应答（src/proxy.rs:490） | ✅ 无 api_key |
| `<< GET /v1/models?key=sk-***` | handle_request inbound（src/proxy.rs:274） | ✅ 已脱敏（TC-4 mask_request_path） |
| **`-> upstream GET http://127.0.0.1:60943/v1/models?key=sk-xyz-query (model=)`** | **handle_request outbound（src/proxy.rs:306-309）** | ❌ **明文泄漏** |
| `<< upstream 200 (streaming)` | 上游应答（src/proxy.rs:347） | ✅ 无密钥 |

泄漏点：`log_proxy!("-> upstream {method} {upstream_url} (model={})", active.model)` —— `upstream_url` 由 `active.base_url + path_and_query` 拼接（src/proxy.rs:300-304），**未经 `mask_request_path`**，`?key=sk-xyz-query` 明文落盘/落 stderr。与 TC-4 Refactor 阶段的预判一致（mask_ctl_and_request_path-refactor.md 已记录"outbound 日志潜在泄漏，留待 TC-8"）。

测试自身已验证：ctl 与 inbound 路径脱敏生效（`sk-contract-key` → `***`、`sk-xyz-query` → `sk-***`），仅 outbound 一条路径泄漏。反真空守卫（`stderr.contains("sk-***")`）确认日志路径真实触发，非空转。

**Green 方向**：outbound 日志行改用 `mask_request_path(&upstream_url)`（或对 `path_and_query` 先掩码再拼 URL）。本轮只写测试（tests/proxy_contract.rs 追加 `log_masks_api_key` + `ProxyChild::read_stderr` helper：kill+wait 使 stderr pipe EOF 后 read_to_string，活进程 pipe 上阻塞等 EOF 不可行），未改 src/。
