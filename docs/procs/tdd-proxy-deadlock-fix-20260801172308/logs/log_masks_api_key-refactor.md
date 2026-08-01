---
title: "log_masks_api_key — Refactor Phase"
brief: "log_masks_api_key — Refactor: exit 0"
doc_type: proc
created: 2026-08-01T18:45:25+0800
case: "log_masks_api_key"
phase: refactor
---
Changes made:

1. **src/proxy.rs — 修复最后一处未脱敏日志路径（outbound 上游错误行）**：`Err(e)` 分支的 `log_proxy!("<< upstream error: {e}")` 改为 `log_proxy!("<< upstream error: {}", mask_request_path(&format!("{e}")))`。原因：reqwest 错误文本内嵌完整请求 URL（含 query，形如 `error sending request for url (http://…/v1/models?key=sk-xyz)`）；当上游不可达且请求 query 携带 sk- 值时，明文会原样进入 debug 日志（CCT_PROXY_LOG=1 的 stderr）——这是审计中唯一一处直接打印可能含敏感值内容的 log_proxy! 行。修复后错误日志行与 inbound/outbound 请求行共用同一 `mask_request_path` sk- 值扫描（约束 #7 的值前缀兜底）。**未削弱脱敏语义**：客户端可见的 502 body（`cct proxy — upstream unreachable: {e}`）保持完整错误文本——那是调用方自己的请求 URL，非 UI/日志显示路径；仅日志路径脱敏。附注：对超时类错误（"operation timed out for url (…)"）同样生效，不依赖具体错误措辞（assert-contracts-not-incidental-platform-strings）。

2. **tests/proxy_contract.rs — 新增契约测试 `log_masks_api_key_upstream_error`**：switch 到必定连接被拒的上游（127.0.0.1:1 → ECONNREFUSED 立即返回）→ GET `/v1/models?key=sk-error-query` → 断言 502 + stderr 全文不含 api_key（sk-contract-key）与 query 明文（sk-error-query），且含 `sk-***` 反真空守卫（outbound 日志行在 send 前写出，必触发）。已验证测试非空转：临时还原未脱敏版本后该测试红（stderr 含 sk-error-query），恢复修复后绿。

3. **无重复逻辑可合并**：`mask_ctl_line`（按字段名，对任意 api_key 值形态生效）与 `mask_request_path`（按 sk- 值前缀扫描，无字段名可依的路径兜底）服务于两种不同上下文，合并会削弱字段名脱敏（自定义 token 形态）或引入 JSON 解析复杂度，有意保留为两个 helper，各自注释已说明分工。

脱敏覆盖面的完整审计（本次逐行过一遍全部日志出口）：

- **已脱敏路径（3 处 log_proxy! 请求/控制行）**：inbound `<< {method} {path}`（mask_request_path）、outbound `-> upstream {method} {url}`（mask_request_path）、ctl `<< {line}`（mask_ctl_line）。本次新增第 4 处：upstream error 行（mask_request_path）。
- **安全路径（直接打印无敏感值）**：启动/绑定日志（211/220）；404/502/streaming 固定串（277/287/348）；`ctl >> ok (switched to base_url=…, model=…)` 与 status（491/504，base_url/model 非敏感字段，api_key 不打印）；`ctl << invalid JSON: {e}`（serde_json 错误不回显输入内容）；`ctl >> err (unknown command: {other})`（仅 cmd 字段，api_key 不在其中）；`ctl << empty command`（464）。
- **已接受的设计边界（未改动，记录在案）**：(a) 请求 path/query 中非 sk- 前缀形态的密钥（如 `?key=custom-token`）不被掩码——这是文档化的"值前缀扫描兜底"（约束 #7），扩展到 query 参数名识别属新脱敏特性而非重构，超出本用例范围；(b) 非 CCT_PROXY_LOG 门控的 `eprintln!("[cct-proxy] connection error: {e}")`（约 250 行）为 hyper http1 帧级错误，不回显请求内容，且属既有代码（本次未触碰）。
- **反真空守卫强度确认**：原 `log_masks_api_key` 的 `sk-***` 断言由 ctl 行（mask_ctl_line 产出 `***`）与 inbound/outbound 行（产出 `sk-***`）共同保障，不会空转；新测试的 `sk-***` 断言由 outbound 行保障。

test_cmd exit code: 0
output: 按任务要求执行 `cargo test --test proxy_contract`（工作树根目录；为记录完整输出使用 `rtk proxy cargo test --test proxy_contract` 原始模式运行，语义等价），完整输出如下

```
$ rtk proxy cargo test --test proxy_contract; echo "EXIT_CODE=$?"
   Compiling cct v0.5.0 (/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.46s
     Running tests/proxy_contract.rs (target/debug/deps/proxy_contract-37538df7d2e1326e)

running 5 tests
test smoke_stub_receives_request ... ok
test concurrent_control_and_http ... ok
test stub_forwarding_with_bearer ... ok
test log_masks_api_key_upstream_error ... ok
test log_masks_api_key ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.46s

EXIT_CODE=0
```
