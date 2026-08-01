---
title: "stub_forwarding_with_bearer — Red Phase"
brief: "stub_forwarding_with_bearer — Red: exit 0（真空 Red：AC4 行为在 HEAD 基线已实现，无法制造失败）"
doc_type: proc
created: 2026-08-01T10:29:31Z
case: "stub_forwarding_with_bearer"
phase: red
---
Exit code: 0

Full output: `cargo test --test proxy_contract stub_forwarding_with_bearer`（工作树根目录执行，`rtk proxy` 原始输出，完整如下）

```
    Blocking waiting for file lock on artifact directory
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.40s
     Running tests/proxy_contract.rs (target/debug/deps/proxy_contract-37538df7d2e1326e)

running 1 test
test stub_forwarding_with_bearer ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.12s
```

## 分析：真空 Red（测试通过，AC4 行为已实现）

任务要求"测试必须失败"，但实测 **exit 0、1 passed**。按"如果通过了，说明断言不够强——加强"要求，已把断言加强到 AC4 契约的完整表面（约束 #4："Bearer key 转发 + 流式返回"），加强后仍全绿。逐一核实任务假设的三类失败原因，均不成立：

1. **switch 未实现**？不成立。`src/proxy.rs` `handle_control` 的 `"switch"` 分支在 HEAD 基线（本会话开始前）即存在（`git show HEAD:src/proxy.rs` 第 423 行），写入 `state.active`（base_url/api_key/model），返回 `status: "ok"`。测试经 serde_json 构造 `{"cmd":"switch","base_url":...,"api_key":...}`（ControlCommand 字段私有、Deserialize 公开），`send_control` 应答正常。
2. **转发不带 Bearer**？不成立。`handle_request`（HEAD 基线即有）在 `!active.api_key.is_empty()` 时注入 `Authorization: Bearer {api_key}`（src/proxy.rs:333-335）。测试客户端**不携带任何 Authorization 头**，stub 记录到的 authorization 恰为 `"Bearer sk-contract-key"`——证明 Bearer 由 proxy 从 switch 状态注入，而非客户端透传。
3. **响应无 DELTA**？不成立。proxy 经 reqwest 转发 stub 的 SSE（`upstream_resp.bytes_stream()` 逐块流式），响应体含 `CONTRACT_STUB_DELTA` 且事件顺序 created → delta → completed。

加强后的完整断言面（全部通过）：
- stub 记录恰 1 条 `("POST", "/v1/chat", "Bearer sk-contract-key")`
- 响应 `HTTP/1.1 200` + `content-type: text/event-stream` + `transfer-encoding: chunked`（流式非缓冲）+ 含 DELTA + SSE 事件顺序

**交叉验证（手动 E2E，真实二进制 + 临时 socket/端口 + curl --noproxy）**：proxy 转发请求头为 `authorization: Bearer sk-contract-key`、`content-length: 66`、body 完整；响应 200 + chunked + 三个 SSE 事件齐全。与契约测试结论一致。

**结论**：AC4 的转发行为在 HEAD 基线即已完整实现（switch 分支 + reqwest 转发 + Bearer 注入 + SSE 流式均为既有代码），本会话 G1 的改动（异步 accept/探测/脱敏/超时）不涉及转发路径。契约测试对已实现行为无法制造真实失败——再"加强"将被迫断言契约之外的行为（如透传任意客户端头），违反 KISS 与"assert contracts, not incidental platform strings"。故本用例为**真空 Red**：不伪造失败，测试即最终契约守护（与 TC-6 之后的其余契约用例同样性质）。建议 Orchestrator：本用例直接标记 Red 通过/免 Green（无 src 改动可做），或按 TDD 严格语义将其视为已满足的回归契约。

附注（测试文件内 helper 改动，均在 tests/proxy_contract.rs）：
- `http_get` 泛化为 `http_request(method, url, auth, body, timeout)`，GET 保留原签名包装
- `spawn_proxy` 增加 `NO_PROXY=127.0.0.1,localhost`：本机 shell 有 `http_proxy=socks5://127.0.0.1:7892`（且 7892 未监听），reqwest 默认读环境代理会把 stub 上游请求打向死代理 → 502；加 NO_PROXY 保证测试隔离（对 TC-8 log_masks_api_key 等后续转发类契约同样必要）
