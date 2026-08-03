---
title: "stub_forwarding_with_bearer — Refactor Phase"
brief: "stub_forwarding_with_bearer — Refactor: exit 0"
doc_type: proc
created: 2026-08-01T18:35:40+0800
case: "stub_forwarding_with_bearer"
phase: refactor
---
Changes made（全部在 tests/proxy_contract.rs，本用例无 src 改动；纯重构，断言强度零变化）:

1. **提取 `start_proxy()` helper（最大重复点）**：三个测试（`smoke_stub_receives_request` / `concurrent_control_and_http` / `stub_forwarding_with_bearer`）各有一段 7 行相同的启动样板 —— `TempDir` + `sock = dir.path().join("proxy.sock")` + `free_port()` + `spawn_proxy` + `wait_healthy` 断言。达到 KISS"三处重复才提取"阈值，提取为 `start_proxy() -> (TempDir, PathBuf, u16, ProxyChild)`，三个调用点各缩为一行解构。健康断言消息统一为 "within 3s"（消息文本非契约，语义不变）。`TempDir` 通过元组绑定存活到测试结束，socket 目录不提前删除。
2. **`http_request` 命名澄清**：`rest` → `without_scheme`、`host_port` → `authority`（该值实际是 host:port 权威段，原名歧义）。纯重命名，行为零变化。
3. **`switch_cmd` / `status_cmd` 改用 `serde_json::json!`**：去掉 `format!` 手拼 JSON 的 `{{`/`}}` 转义噪音，`json!` 字面量更清晰且免手写转义（测试值含引号时更稳）。序列化语义等价（serde_json 默认 Map 排序与反序列化无关），注释保持。两个同族 helper 一并改，避免风格分裂。
4. **SSE 头断言大小写不敏感化（`body_lower = body.to_ascii_lowercase()`）**：`content-type: text/event-stream` 与 `transfer-encoding: chunked` 是契约值，但**头名**大小写由代理生成方式决定（HTTP 协议不敏感，属实现细节）。原断言硬编码代理当前小写头的偶然格式，违反 "assert-contracts-not-incidental-platform-strings" 规则；小写化匹配同一契约、鲁棒性更优，不削弱断言。`HTTP/1.1 200`（协议语法大写 token）与 DELTA/事件顺序断言仍走原始 body（DELTA 内容大小写敏感）。
5. **stale 注释修正**：`DELTA` 常量注释 "后续 stub_forwarding_with_bearer 也复用"（Green 阶段落笔时的未来式）改为 "smoke 与 stub_forwarding_with_bearer 共用"。

测试质量观察（未改动的判断）：

- **AC4 断言强度审查（Bearer 注入 + SSE 流式 + DELTA）**：三项契约均被强断言覆盖且未削弱 —— (a) Bearer 注入：stub 记录精确等于 `("POST", "/v1/chat", "Bearer sk-contract-key")` 且 `reqs.len() == 1`（不多不少）; (b) SSE 流式：200 + SSE content-type + `transfer-encoding: chunked`（非 Content-Length 缓冲）; (c) DELTA：内容存在 + 事件顺序 created → delta → completed（用 `find` 位置比较）。事件类型本身（`response.output_text.delta`）未单独断言，但 DELTA 位于 created 与 completed 之间已蕴含该事件序列，强度足够。
- **未削弱点确认**：`stub.requests()` 精确匹配、`reqs.len() == 1`、3s 预算、SSE 事件顺序断言全部原样保留。
- **有意保留**：`http_get` 单次使用 wrapper（GET 语义自明，内联反而降低 smoke 可读性）；`StubUpstream` 的 `Vec<(String,String,String)>` 三元组（有 doc 注释文档化字段顺序，换 struct 属偏好而非简化）；`read_request` 的 EOF → 空 method 约定（stub 协议的一部分）。
- **潜在可加强点（未做，超出"不改变断言强度"边界）**：stub 目前只记录 method/path/auth，不记录请求体 —— 若未来要断言 proxy 转发的 POST body 完整性，需扩展 `read_request` 读 body；当前 AC4 未要求 body 契约，不改。

test_cmd exit code: 0
output: 按任务要求执行 `cargo test --test proxy_contract`（工作树根目录；为记录完整输出使用 `rtk proxy cargo test --test proxy_contract` 原始模式运行，语义等价），完整输出如下

```
$ rtk proxy cargo test --test proxy_contract; echo "EXIT_CODE=$?"
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.09s
     Running tests/proxy_contract.rs (target/debug/deps/proxy_contract-37538df7d2e1326e)

running 3 tests
test smoke_stub_receives_request ... ok
test stub_forwarding_with_bearer ... ok
test concurrent_control_and_http ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s

EXIT_CODE=0
```
