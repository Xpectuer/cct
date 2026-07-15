# cc-switch Codex 协议转换逻辑

> 分析对象：`docs/references/cc-switch/src-tauri/src/proxy/providers/`
> 分析日期：2026-07-13

## 一、背景

cc-switch 是一个本地代理，位于 Codex CLI 与上游 AI 提供商之间。Codex CLI 始终使用 **OpenAI Responses API** 协议与代理通信，但上游提供商可能使用不同的协议：

| 上游协议 | 转换路径 | 说明 |
|----------|----------|------|
| OpenAI Responses（原生） | **直接透传** | 不需要转换，请求/响应原样转发 |
| OpenAI Chat Completions | **Responses ↔ Chat** | 上游只提供 Chat API，需要双向转换 |
| Anthropic Messages | **Responses ↔ Anthropic** | 上游是 Anthropic 网关，需要双向转换 |

## 二、配置如何落到 Codex

在理解协议转换之前，首先要理解 cc-switch 如何将自己的 provider 配置"注入"到 Codex CLI 的文件系统中。

### 2.1 Codex 的本地配置体系

Codex CLI 从 `~/.codex/` 目录读取配置：

```
~/.codex/
  ├─ auth.json         — 认证信息 (ChatGPT OAuth token 或 OPENAI_API_KEY)
  ├─ config.toml       — 主配置：model_provider, model, base_url, wire_api 等
  └─ models_cache.json  — Codex 首次连接 OpenAI 时缓存的模型目录
```

`config.toml` 的关键字段：

```toml
model_provider = "custom"               # 激活的 provider ID
model = "claude-sonnet-5"               # 当前模型

[model_providers.custom]
name = "Anthropic Gateway"
base_url = "https://api.anthropic.com"   # 上游地址
wire_api = "anthropic"                   # ★ 决定协议转换路径
api_key = "sk-..."                       # API 密钥
```

### 2.2 cc-switch 的接管流程

```
用户选择 provider（UI / DB）
  │
  ├─ 1. 读取 provider.settings_config
  │     ├─ auth:   { OPENAI_API_KEY: "sk-..." }
  │     ├─ config: "model_provider = \"custom\"\n[model_providers.custom]\n..."
  │     ├─ base_url / baseURL
  │     ├─ modelCatalog: { models: [...] }
  │     └─ apiFormat: "anthropic" | "openai_chat" | ...
  │
  ├─ 2. 从 provider.settings_config 提取 apiFormat（决定转换路径）
  │     探测优先级（codex_provider_uses_anthropic / codex_provider_uses_chat_completions）：
  │     ├─ ① meta.api_format（provider 注册时的元数据）
  │     ├─ ② settings_config.api_format / settings_config.apiFormat
  │     ├─ ③ settings_config.config TOML 内 [model_providers.<id>].wire_api
  │     └─ ④ settings_config.config TOML 内 base_url 后缀 (/chat/completions)
  │
  ├─ 3. 生成模型目录文件 → ~/.codex/cc-switch-model-catalog.json
  │     └─ 根据 apiFormat 决定 CodexCatalogToolProfile：
  │          anthropic         → Anthropic    (过滤 apply_patch/web_search)
  │          openai_responses  → NativeResponses (过滤 custom tools)
  │          其他              → ProxyChat    (保留全部 Codex tools)
  │
  ├─ 4. 写入 ~/.codex/config.toml
  │     ├─ model_provider = "custom"
  │     ├─ [model_providers.custom]
  │     │     base_url = "http://127.0.0.1:<port>"  → 指向 cc-switch 本地代理
  │     │     wire_api = "responses"                  → Codex 始终用 Responses 协议对话
  │     ├─ model = <modelCatalog 中选中的模型>
  │     └─ model_catalog_json = "cc-switch-model-catalog.json"
  │
  └─ 5. Codex CLI 启动 → 读取 config.toml → 向 http://127.0.0.1:<port>/v1/responses 发请求
       → cc-switch 代理拦截 → 根据原始 provider.settings_config 决定转换路径
```

### 2.3 关键：两套配置的分离

cc-switch **写给 Codex** 的 `~/.codex/config.toml` 和 cc-switch **自己用来做路由决策** 的 provider 配置是两套不同的东西：

| | 写到 `~/.codex/config.toml` 的 | cc-switch 路由决策用的 |
|---|---|---|
| `wire_api` | `"responses"`（写死，让 Codex 始终用 Responses 协议） | 真正的上游 apiFormat（`"anthropic"` / `"chat"` 等） |
| `base_url` | `http://127.0.0.1:<port>`（指向 cc-switch 代理） | 真正的上游地址（从 provider.settings_config 读取） |
| `model_provider` | `"custom"`（统一使用 custom provider） | 无对应关系 |

**核心设计**：写给 Codex 的配置**故意欺骗** Codex，让它以为在和一个标准的 OpenAI Responses 端点通信。实际的上游类型（Anthropic / Chat / 原生 Responses）只存在 cc-switch 内部的 provider 配置里，Codex 对此一无所知。

### 2.4 apiFormat 探测的完整优先级

`codex_provider_uses_anthropic()` 和 `codex_provider_uses_chat_completions()` 共享同一套配置读取逻辑：

```
探测 apiFormat（按优先级）:
  │
  ├─ ① meta.api_format（provider 元数据）
  │     例: provider.meta = { api_format: "anthropic" }
  │
  ├─ ② settings_config.api_format / settings_config.apiFormat
  │     例: provider.settings_config = { "apiFormat": "openai_chat" }
  │
  ├─ ③ settings_config.config TOML 内的 wire_api 字段
  │     读取 [model_providers.<active_provider>].wire_api
  │     或顶层 wire_api
  │     例: wire_api = "anthropic"  → 触发 Anthropic 转换
  │
  └─ ④ (仅 Chat 路径) settings_config.config TOML 内的 base_url
        如果 base_url 以 /chat/completions 结尾 → 触发 Chat 转换
        如果 settings_config.base_url 以 /chat/completions 结尾 → 同上
```

**有效的 apiFormat 取值**：

| 值 | 触发路径 | 说明 |
|----|---------|------|
| `"anthropic"`, `"anthropic_messages"`, `"claude"`, `"messages"` | Anthropic 转换 | 上游是 Anthropic Messages API |
| `"chat"`, `"chat_completions"`, `"openai_chat"`, `"openai_chat_completions"` | Chat 转换 | 上游是 OpenAI Chat Completions API |
| `"responses"`, `"openai_responses"` | 透传 | 上游是原生 OpenAI Responses API |
| 未设置 + base_url 以 `/chat/completions` 结尾 | Chat 转换 | 通过 URL 模式推断 |
| 未设置 + 其他情况 | 透传 | 默认行为 |

## 三、请求路由

### 入口点

Codex CLI 将请求发到 cc-switch 的 `/v1/responses` 或 `/v1/responses/compact` 端点。handler 在接收到请求后按以下逻辑决策：

```
body (Responses 请求)
  │
  ├─ should_convert_codex_responses_to_anthropic()
  │     └─ 上游 apiFormat = "anthropic" / "anthropic_messages" / "claude" / "messages"
  │        → handle_codex_anthropic_to_responses_transform()
  │
  ├─ should_convert_codex_responses_to_chat()
  │     └─ 上游 apiFormat = "chat" / "openai_chat" / ...
  │        或 base_url 以 /chat/completions 结尾
  │        → handle_codex_chat_to_responses_transform()
  │
  └─ 其他情况
        → 直接透传（passthrough）
```

### 转发前的模型替换

无论走哪个转换路径，在转发到上游之前都会执行模型替换（`apply_codex_upstream_model`）：

1. 如果请求中携带的 model 在 provider 的 `modelCatalog` 中，保留请求中的 model
2. 否则，替换为 provider 配置中的 `model`（TOML 中 `model_provider.<name>` 对应的模型）

## 四、核心数据结构：CodexToolContext

两个转换路径共用同一个 `CodexToolContext`，它是工具映射的单一真相来源。

### Codex 的五种工具类型

| 类型 | Codex 中的表示 | 说明 |
|------|---------------|------|
| `function` | `{"type":"function","name":"x","parameters":{...}}` | 标准函数工具 |
| `custom` | `{"type":"custom","name":"apply_patch"}` | 自由格式工具（无 JSON schema），如 patch 应用 |
| `tool_search` | `{"type":"tool_search"}` | Codex 专用：动态搜索和加载工具 / MCP 命名空间 |
| `namespace` | `{"type":"namespace","name":"mcp__x","tools":[...]}` | MCP 命名空间，包含子工具列表 |
| `web_search` 等托管工具 | `{"type":"web_search"}` | 被过滤掉，不传给上游 |

### 转换时的处理

```
build_codex_tool_context_from_request(body)
  │
  ├─ 遍历 body.tools[]
  │   ├─ function → 直接转换为 Chat 的 function tool
  │   ├─ namespace → 展开子工具，name 变为 "namespace__name"
  │   ├─ custom   → 包装为 string-input 的 function tool
  │   │              参数: {"input": "string"}
  │   │              描述: 嵌入原始 tool 定义的 JSON
  │   ├─ tool_search → 注册 proxy 工具 "tool_search"
  │   └─ web_search 等 → 丢弃
  │
  └─ 遍历 body.input[]
      └─ tool_search_output.tools → 动态加载的工具也注册到 context
```

**关键设计**：`CodexToolContext` 维护三张映射表，用于响应时还原：

- `chat_name_to_spec`: Chat 工具名 → `CodexToolSpec`（含 kind、原始 name、namespace）
- `namespace_name_to_chat_name`: `(namespace, name)` → Chat 工具名
- `seen_chat_names`: 已注册的 Chat 工具名（去重）

**命名空间工具名称扁平化**：
- `mcp__files` + `read` → `mcp__files__read`
- 超过 64 字符时截断并追加 SHA256 哈希：
  - `very_long_namespace_name__very_long_function_name` → `very_long_n…__a1b2c3d4`

## 五、路径 A：Responses → Chat Completions 转换

### 文件

- `transform_codex_chat.rs` — 请求转换（非流式）
- `streaming_codex_chat.rs` — 流式 SSE 转换
- `codex_chat_common.rs` — 共享工具（reasoning 提取等）
- `codex_chat_history.rs` — 会话历史记录

### 请求转换流程

```
responses_to_chat_completions_with_reasoning(body, reasoning_config)
  │
  ├─ 1. 构建 CodexToolContext
  │
  ├─ 2. 转换 instructions → system message
  │
  ├─ 3. 转换 input[] → messages[]
  │     ├─ message 项 → 直接映射为 chat message
  │     ├─ function_call → assistant 的 tool_calls
  │     ├─ function_call_output → tool role message
  │     ├─ reasoning → 附加到最近一条 assistant 消息的 reasoning_content
  │     ├─ input_text → user message 的 content
  │     ├─ input_image → content 中的 image_url 块
  │     └─ custom_tool_call / tool_search_call → 类似的 tool_calls 映射
  │
  ├─ 4. 合并 system messages 到首位（MiniMax 兼容）
  │     └─ collapse_system_messages_to_head()
  │
  ├─ 5. 映射生成参数
  │     ├─ max_output_tokens → max_tokens（非 o-series）/ max_completion_tokens
  │     ├─ temperature, top_p, stream → 直传
  │     └─ reasoning.effort → 见下方推理配置
  │
  ├─ 6. 注入 stream_options.include_usage
  │     └─ 确保流式响应包含 token 统计
  │
  └─ 7. 输出 Chat Completions 请求 JSON
```

### Responses input 到 Chat messages 的关键映射

| Responses Item Type | Chat Role | Chat Content |
|---------------------|-----------|--------------|
| `message` (role=user) | user | 内容数组 → text/image_url/file 块 |
| `message` (role=assistant) | assistant | 文本 + reasoning_content |
| `message` (role=system/developer) | system | 文本（合并到首条） |
| `message` (role=latest_reminder) | user | 文本 |
| `function_call` | assistant | tool_calls[{id, function:{name, arguments}}] |
| `function_call_output` | tool | content + tool_call_id |
| `custom_tool_call` | assistant | tool_calls（arguments 包装为 `{"input":"..."}`) |
| `tool_search_call` | assistant | tool_calls（name="tool_search"） |
| `reasoning` | (不创建消息) | 附加到上一条 assistant 的 reasoning_content |
| `input_text` | user | text 块 |
| `input_image` | user | image_url 块 |

### 推理（Reasoning）配置推断

由于不同上游提供商用不同参数控制推理/思考功能，cc-switch 维护了一套自动推断逻辑 (`infer_codex_chat_reasoning_config`)：

```
推断逻辑（按优先级）:
  │
  ├─ 平台优先（避免模型名干扰）
  │   ├─ OpenRouter → reasoning:{effort:"xhigh"|"high"|"medium"|"low"|"minimal"}
  │   └─ SiliconFlow → enable_thinking:true
  │
  └─ 按模型/提供商名称关键字推断
      ├─ DeepSeek → thinking:{type:"enabled"} + reasoning_effort
      │              effort_value_mode: "deepseek" (max/xhigh → "max")
      ├─ Kimi/Moonshot → thinking:{type:"enabled"}，无 effort
      ├─ GLM/Zhipu → thinking:{type:"enabled"}，无 effort
      ├─ Qwen/DashScope → enable_thinking:true，无 effort
      ├─ MiniMax → reasoning_split:true，无 effort
      ├─ StepFun → thinking 关闭路径 (thinking_param=none)
      └─ Mimo → thinking:{type:"enabled"}，无 effort
```

**显式 provider meta 配置** 会覆盖自动推断。

### 响应转换（非流式）

```
chat_completion_to_response_with_context(body, tool_context)
  │
  ├─ 提取 reasoning_content → 生成 reasoning item
  ├─ 提取 content / refusal → 生成 message item
  ├─ 提取 tool_calls → 根据 tool_context 还原：
  │     ├─ tool_search → tool_search_call item
  │     ├─ custom       → custom_tool_call item
  │     └─ function     → function_call item（含 namespace 还原）
  ├─ 映射 finish_reason → status（"length"→"incomplete"）
  └─ 转换 usage（prompt_tokens→input_tokens, completion_tokens→output_tokens）
```

### 流式 SSE 转换

`streaming_codex_chat.rs` 中的 `ChatToResponsesState` 维护状态机：

1. **文本内容**：`content` delta → `response.output_text.delta` 事件
2. **推理内容**：`reasoning_content` / `reasoning_details` delta → `response.reasoning_summary_text.delta` 事件
3. **内联 `<think>` 标签**（MiniMax 特有）：检测 `<think>...</think>` 包裹的推理内容，剥离标签后作为 reasoning 事件发送
4. **工具调用**：`tool_calls` delta → `response.function_call_arguments.delta` 事件（完成后 `response.function_call_arguments.done`）
5. **自定义工具**：`response.custom_tool_call_input.delta/done` 事件

### 流结束处理

- 正常完成（有 `finish_reason`）：发出 `response.completed`
- 截断但有输出：设为 `incomplete` + `reason:"max_output_tokens"`
- 无输出截断：发出 `response.failed`
- 错误事件：发出 `response.failed`

## 六、路径 B：Responses → Anthropic Messages 转换

### 文件

- `transform_codex_anthropic.rs` — 请求/响应格式转换（非流式）
- `streaming_codex_anthropic.rs` — Anthropic SSE → Responses SSE
- 复用了 `transform_codex_chat.rs` 中的 `CodexToolContext`

### 请求转换流程

```
responses_request_to_anthropic(body, default_max_tokens)
  │
  ├─ 1. 构建 CodexToolContext
  │
  ├─ 2. model → 直传
  │
  ├─ 3. instructions → system
  │
  ├─ 4. input[] → messages[]
  │     ├─ input_text → user 的 text block
  │     ├─ input_image → user 的 image block
  │     ├─ function_call → assistant 的 tool_use block
  │     ├─ function_call_output → user 的 tool_result block
  │     ├─ custom_tool_call → assistant 的 tool_use（input 包装）
  │     ├─ tool_search_call → assistant 的 tool_use（name="tool_search"）
  │     └─ reasoning.encrypted_content → 解码还原 signed thinking block
  │
  ├─ 5. 消息规范化（Anthropic 严格要求）
  │     ├─ drop_incomplete_tool_turns() —— 移除不完整的 tool_use/tool_result 对
  │     ├─ drop_empty_messages() —— 移除空内容消息
  │     ├─ ensure_leading_user_message() —— 首条必须是 user
  │     └─ trim_trailing_assistant_text() —— 去掉尾随空白
  │
  ├─ 6. max_output_tokens → max_tokens（必填，缺失时注入默认值）
  │
  ├─ 7. reasoning.effort → thinking
  │     ├─ 自适应模型（Sonnet 5, Fable 5）：使用 thinking:{type:"adaptive"}
  │     ├─ effort → budget_tokens: 见映射表
  │     ├─ max_tokens/2 预算上限（为 visible answer 保留空间）
  │     └─ budget < 1024 → 禁用 thinking, 恢复 temperature/top_p
  │
  ├─ 8. tools → Anthropic tools（过滤掉 web_search 等不支持的）
  │
  ├─ 9. tool_choice 映射
  │     "required" → {"type":"any"}
  │     {"type":"function"} → {"type":"tool","name":"..."}
  │     forced tool_choice + thinking → 禁用 thinking（Anthropic 冲突）
  │
  └─ 10. 输出 Anthropic Messages 请求 JSON
```

### Reasoning Effort → Anthropic Thinking Budget

| Codex Effort | Thinking Budget |
|-------------|-----------------|
| minimal / low | 2048 |
| medium | 8192 |
| high | 16384 |
| xhigh / max | 24576 |

**自适应模型**（claude-sonnet-5 等）：默认启用 `thinking:{type:"adaptive"}`， 可叠加 `output_config:{effort:"low"/"medium"/"high"/"max"}`。

### Anthropic Signed Thinking 的加密桥接

Codex 使用 Anthropic 模型时，工具调用的多轮对话需要 replay 前一轮的 signed thinking block。cc-switch 的方案：

```
发送方向（Anthropic response → Codex）:
  Anthropic thinking/redacted_thinking block
    → base64 编码 + "ccswitch-anthropic-thinking-v1:" 前缀
    → 存入 Responses reasoning.encrypted_content 字段

接收方向（Codex 下一次请求 → Anthropic）:
  reasoning.encrypted_content
    → 剥离前缀 + base64 解码
    → 还原为 Anthropic thinking/redacted_thinking block
    → 插入 assistant 消息的 content 列表
```

### Tool 工具映射

| Codex Tool | Anthropic Tool |
|------------|---------------|
| `function` (name="x") | `{name:"x", input_schema: parameters}` |
| `custom` (name="apply_patch") | `{name:"apply_patch", input_schema: {type:"object", properties: {input: {type:"string"}}}}` |
| `namespace` (展开子工具) | 每个子工具独立注册（如 `mcp__files__read`） |
| `tool_search` | `{name:"tool_search", input_schema: {query, limit}}` |
| `web_search` 等 | **丢弃**（Anthropic 不支持） |

### 响应转换（非流式）

```
anthropic_response_to_responses_with_context(body, tool_context)
  │
  ├─ content[] 遍历：
  │     ├─ text → output_text 块
  │     ├─ tool_use → function_call/custom_tool_call/tool_search_call item
  │     │             并用 tool_context 还原 namespace
  │     └─ thinking/redacted_thinking → reasoning item（加密存储）
  │
  ├─ stop_reason → status
  │     end_turn → "completed"
  │     tool_use → "completed"
  │     max_tokens → "incomplete" + reason:"max_output_tokens"
  │     refusal → "incomplete" + reason:"content_filter"
  │
  └─ usage 转换
      input_tokens = anthropic.input_tokens + cache_read_input_tokens
      output_tokens = anthropic.output_tokens
      reasoning_tokens = anthropic.output_tokens_details.thinking_tokens
```

### 流式 SSE 转换

`streaming_codex_anthropic.rs` 中的 `AnthropicToResponsesState` 处理 Anthropic SSE 事件：

| Anthropic Event | Responses Event |
|----------------|-----------------|
| `message_start` | `response.created` + `response.in_progress` |
| `content_block_start` (text) | `response.output_item.added` + `response.content_part.added` (message) |
| `content_block_start` (tool_use) | `response.output_item.added` (function_call/custom_tool_call) |
| `content_block_start` (thinking) | `response.output_item.added` (reasoning) |
| `content_block_delta` (text_delta) | `response.output_text.delta` |
| `content_block_delta` (input_json_delta) | `response.function_call_arguments.delta` |
| `content_block_delta` (thinking_delta) | `response.reasoning_summary_text.delta` |
| `content_block_delta` (signature_delta) | (缓冲，不发送) |
| `content_block_stop` | `response.content_part.done` / `response.function_call_arguments.done` / `response.reasoning_summary_text.done` |
| `message_delta` | (记录 stop_reason + usage) |
| `message_stop` | `response.completed` |
| `error` | `response.failed` |

## 七、错误处理

### Chat 错误 → Responses 错误

上游 Chat Completions API 返回的错误格式不一（OpenAI 标准 `{"error":{...}}`、MiniMax `{"base_resp":{...}}`、纯文本等），需要统一转换为 Responses API 风格：

```json
{
  "error": {
    "message": "...",
    "type": "...",
    "code": "...",
    "param": null
  }
}
```

转换函数 `chat_error_to_response_error()` 兼容：
- 标准 OpenAI 错误体
- MiniMax base_resp 格式
- 顶层 message/detail 字段
- 裸字符串

### 代理层错误

当 cc-switch 自身出错（无可用 provider、超时、熔断等），构造富化的错误体：

```json
{
  "error": {
    "message": "CC Switch local proxy failed while handling Codex endpoint ...",
    "type": "proxy_error",
    "code": "cc_switch_no_available_provider",
    "provider": "...",
    "model": "...",
    "endpoint": "/v1/responses"
  }
}
```

## 八、特殊处理逻辑

### 压缩请求体

Codex Desktop 客户端可能对请求体使用 zstd 压缩，handler 在接收到请求时自动检测 `Content-Encoding` 头并解压。

### SSE 嗅探兜底（#2234）

部分上游对 `stream:false` 的请求仍返回 SSE 流，且 Content-Type 未标记为 `text/event-stream`。当 JSON 解析失败且 body 以 `data:`/`event:` 等 SSE 前缀开头时，自动按 SSE 聚合后再走非流式转换器。

### 流截断处理

- 上游在 `message_stop` 之前断开连接，但产生了部分输出 → 报告 `incomplete`（而非正常完成）
- 上游在产生任何输出之前断开 → 报告 `failed`
- `message_delta` 已携带 `stop_reason` 但流提前结束 → 正常完成（turn 已完整）

### 内联 `<think>` 标签处理

MiniMax M2.7 等模型在 `content` 中内联输出 `\<think\>推理内容\</think\>回答内容`。流式转换器实时检测并分离为 reasoning 和 text 事件。

## 九、文件索引

| 文件 | 职责 |
|------|------|
| `proxy/handlers.rs` | HTTP 请求入口，路由到正确的转换处理器 |
| `proxy/providers/codex.rs` | Codex 适配器：路由决策、apiFormat 检测、reasoning 配置推断 |
| `proxy/providers/transform_codex_chat.rs` | Responses ↔ Chat Completions 格式转换（非流式） |
| `proxy/providers/transform_codex_anthropic.rs` | Responses ↔ Anthropic Messages 格式转换（非流式） |
| `proxy/providers/streaming_codex_chat.rs` | Chat SSE → Responses SSE 流式转换 |
| `proxy/providers/streaming_codex_anthropic.rs` | Anthropic SSE → Responses SSE 流式转换 |
| `proxy/providers/codex_chat_common.rs` | 共享工具函数（reasoning 提取、think 标签解析） |
| `proxy/providers/codex_chat_history.rs` | Codex Chat 会话历史记录 |
| `proxy/providers/codex_responses_sse.rs` | Responses SSE 事件构造工具函数 |
| `proxy/providers/codex_oauth_auth.rs` | Codex OAuth 认证 |
| `proxy/thinking_optimizer.rs` | 自适应推理模型判断 |
| `proxy/thinking_budget_rectifier.rs` | 推理预算调整 |
