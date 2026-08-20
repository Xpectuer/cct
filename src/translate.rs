//! Responses ↔ Chat Completions protocol translation.
//!
//! `cct`'s codex backend forwards `/v1/responses` verbatim to the upstream.
//! Gateways that only implement OpenAI's Chat Completions API
//! (`/v1/chat/completions`) cannot be used that way — codex ≥ 0.148 removed
//! `wire_api = "chat"` support (openai/codex#7782). When a profile sets
//! `wire_api = "chat"`, the proxy rewrites responses requests into chat
//! completions and translates the upstream chat SSE stream back into
//! responses SSE events.
//!
//! Pure module — no IO. All mapping logic lives here (pure-builders,
//! thin-effectful-edges rule); proxy.rs only wires it in.

use serde_json::{json, Map, Value};
use std::collections::HashMap;

/// Rewrite a `/v1/responses` request body into a `/v1/chat/completions` body.
///
/// Output contains exactly: `model`, `messages`, `stream: true`,
/// `stream_options: {"include_usage": true}`, and `tools` (only when the
/// request carries function tools). Responses-only fields (`store`,
/// `include`, `tool_choice`, `parallel_tool_calls`, `reasoning`, `text`,
/// `conversation`, …) are dropped — chat gateways do not understand them.
pub fn responses_to_chat_request(body: &Value) -> Value {
    let mut out = Map::new();
    if let Some(model) = body.get("model").and_then(|m| m.as_str()) {
        out.insert("model".into(), Value::String(model.into()));
    }

    // First message is always the instructions system message.
    let instructions = body
        .get("instructions")
        .and_then(|i| i.as_str())
        .unwrap_or("");
    let mut messages: Vec<Value> = vec![json!({"role": "system", "content": instructions})];

    let mut last_assistant_text: Option<String> = None;
    let mut last_emitted_role: Option<&str> = None;
    // Reasoning with no previous assistant message attaches to the next
    // assistant/function_call message (old reasoning_by_anchor_index logic).
    let mut pending_reasoning: Option<String> = None;

    if let Some(input) = body.get("input").and_then(|i| i.as_array()) {
        for item in input {
            match item.get("type").and_then(|t| t.as_str()) {
                Some("message") => {
                    let role = item.get("role").and_then(|r| r.as_str()).unwrap_or("user");
                    let parts = item
                        .get("content")
                        .and_then(|c| c.as_array())
                        .cloned()
                        .unwrap_or_default();
                    let text: String = parts
                        .iter()
                        .filter(|p| {
                            matches!(
                                p.get("type").and_then(|t| t.as_str()),
                                Some("input_text") | Some("output_text")
                            )
                        })
                        .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
                        .collect();
                    if role == "assistant" {
                        // Consecutive assistant messages with identical text
                        // are duplicates — skip the whole message.
                        if last_assistant_text.as_deref() == Some(text.as_str()) {
                            continue;
                        }
                        last_assistant_text = Some(text.clone());
                        let mut msg = json!({"role": "assistant", "content": text});
                        attach_pending_reasoning(&mut msg, &mut pending_reasoning);
                        messages.push(msg);
                        last_emitted_role = Some("assistant");
                    } else {
                        let msg = if let Some(img) = parts
                            .iter()
                            .find(|p| p.get("type").and_then(|t| t.as_str()) == Some("input_image"))
                        {
                            let url = img
                                .get("image_url")
                                .and_then(|u| u.get("url"))
                                .or_else(|| img.get("url"))
                                .and_then(|u| u.as_str())
                                .unwrap_or("");
                            json!({
                                "role": role,
                                "content": [
                                    {"type": "text", "text": text},
                                    {"type": "image_url", "image_url": {"url": url}}
                                ]
                            })
                        } else {
                            json!({"role": role, "content": text})
                        };
                        messages.push(msg);
                        last_emitted_role = Some(role);
                        last_assistant_text = None;
                    }
                }
                Some("function_call") => {
                    let call_id = item
                        .get("call_id")
                        .and_then(|c| c.as_str())
                        .unwrap_or("")
                        .to_string();
                    let name = item
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string();
                    let arguments = match item.get("arguments") {
                        Some(Value::String(s)) => s.clone(),
                        Some(v) => v.to_string(),
                        None => "{}".to_string(),
                    };
                    let call = json!({
                        "id": call_id,
                        "type": "function",
                        "function": {"name": name, "arguments": arguments}
                    });
                    // Merge into the current assistant tool-call message, or
                    // start a fresh one (old push_tool_call_message semantics).
                    let last_is_tool_msg = messages
                        .last()
                        .map(|last| {
                            last.get("role").and_then(|r| r.as_str()) == Some("assistant")
                                && last.get("content").map(|c| c.is_null()).unwrap_or(false)
                                && last.get("tool_calls").is_some()
                        })
                        .unwrap_or(false);
                    if last_is_tool_msg {
                        let last = messages.last_mut().expect("last message exists");
                        last["tool_calls"]
                            .as_array_mut()
                            .expect("tool_calls array")
                            .push(call);
                        attach_pending_reasoning(last, &mut pending_reasoning);
                    } else {
                        let mut msg = json!({
                            "role": "assistant",
                            "content": null,
                            "tool_calls": [call]
                        });
                        attach_pending_reasoning(&mut msg, &mut pending_reasoning);
                        messages.push(msg);
                    }
                    last_emitted_role = Some("assistant");
                    last_assistant_text = None;
                }
                Some("function_call_output") => {
                    let call_id = item
                        .get("call_id")
                        .and_then(|c| c.as_str())
                        .unwrap_or("")
                        .to_string();
                    let content: Value = match item.get("output") {
                        Some(Value::String(s)) => Value::String(s.clone()),
                        Some(v) if v.is_object() => {
                            if let Some(s) = v.get("content").and_then(|c| c.as_str()) {
                                Value::String(s.to_string())
                            } else if let Some(items) =
                                v.get("content_items").and_then(|c| c.as_array())
                            {
                                Value::Array(
                                    items
                                        .iter()
                                        .map(|part| {
                                            match part.get("type").and_then(|t| t.as_str()) {
                                                Some("output_image") | Some("input_image") => {
                                                    let url = part
                                                        .get("image_url")
                                                        .and_then(|u| u.get("url"))
                                                        .and_then(|u| u.as_str())
                                                        .unwrap_or("");
                                                    json!({
                                                        "type": "image_url",
                                                        "image_url": {"url": url}
                                                    })
                                                }
                                                _ => json!({
                                                    "type": "text",
                                                    "text": part
                                                        .get("text")
                                                        .and_then(|t| t.as_str())
                                                        .unwrap_or("")
                                                }),
                                            }
                                        })
                                        .collect(),
                                )
                            } else {
                                Value::String(String::new())
                            }
                        }
                        _ => Value::String(String::new()),
                    };
                    messages.push(json!({
                        "role": "tool",
                        "tool_call_id": call_id,
                        "content": content
                    }));
                    last_emitted_role = Some("tool");
                    last_assistant_text = None;
                }
                Some("reasoning") => {
                    // Old adapter's last_emitted_role guard: reasoning that
                    // follows a user turn has no assistant anchor — skip.
                    if last_emitted_role == Some("user") {
                        continue;
                    }
                    let text: String = item
                        .get("content")
                        .and_then(|c| c.as_array())
                        .map(|parts| {
                            parts
                                .iter()
                                .filter(|p| {
                                    matches!(
                                        p.get("type").and_then(|t| t.as_str()),
                                        Some("reasoning_text") | Some("text")
                                    )
                                })
                                .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
                                .collect()
                        })
                        .unwrap_or_default();
                    if text.is_empty() {
                        continue;
                    }
                    // Attach to the previous assistant message; with none yet,
                    // hold it for the next assistant/function_call message.
                    if let Some(idx) = messages
                        .iter()
                        .rposition(|m| m.get("role").and_then(|r| r.as_str()) == Some("assistant"))
                    {
                        merge_reasoning(&mut messages[idx], &text);
                    } else {
                        pending_reasoning = Some(match pending_reasoning.take() {
                            Some(prev) => format!("{prev}\n{text}"),
                            None => text,
                        });
                    }
                }
                // custom_tool_call / local_shell_call / web_search_call /
                // ghost / compaction markers: no chat-completions equivalent.
                _ => {}
            }
        }
    }

    out.insert("messages".into(), Value::Array(messages));
    out.insert("stream".into(), Value::Bool(true));
    out.insert("stream_options".into(), json!({"include_usage": true}));

    // Only function tools survive — custom/local tools have no chat
    // equivalent. Shape: {"type":"function","name":…,"function":{…}}.
    let tools: Vec<Value> = body
        .get("tools")
        .and_then(|t| t.as_array())
        .map(|tools| {
            tools
                .iter()
                .filter(|t| t.get("type").and_then(|x| x.as_str()) == Some("function"))
                .map(|t| {
                    let name = t.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    let mut function = t.as_object().cloned().unwrap_or_default();
                    function.remove("type");
                    json!({"type": "function", "name": name, "function": function})
                })
                .collect()
        })
        .unwrap_or_default();
    if !tools.is_empty() {
        out.insert("tools".into(), Value::Array(tools));
    }

    Value::Object(out)
}

/// Merge pending reasoning text into a message that is about to become the
/// anchor (assistant or function_call message).
fn attach_pending_reasoning(msg: &mut Value, pending: &mut Option<String>) {
    if let Some(text) = pending.take() {
        merge_reasoning(msg, &text);
    }
}

/// Append reasoning text to a message's `"reasoning"` field, joining with
/// `\n` when the anchor already carries reasoning.
fn merge_reasoning(msg: &mut Value, text: &str) {
    let existing = msg.get("reasoning").and_then(|r| r.as_str());
    let joined = match existing {
        Some(prev) => format!("{prev}\n{text}"),
        None => text.to_string(),
    };
    msg["reasoning"] = Value::String(joined);
}

/// Buffered state of one in-flight function tool call.
pub struct ToolCallState {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// Translates an upstream chat-completions SSE stream into responses SSE
/// events (mirrors codex's deleted chat adapter / process_chat_sse). The
/// translator buffers tool calls and only emits their `output_item.done`
/// events once the model finishes with `finish_reason == "tool_calls"`; text
/// and reasoning are streamed through as added/delta/done events.
#[derive(Default)]
pub struct ChatSseTranslator {
    tool_calls: HashMap<usize, ToolCallState>,
    tool_call_order: Vec<usize>,
    tool_call_index_by_id: HashMap<String, usize>,
    last_tool_call_index: Option<usize>,
    assistant_text: Option<String>,
    reasoning_text: Option<String>,
    reasoning_segment_count: i64,
    usage: Option<Value>,
    response_id: String,
    terminal_emitted: bool,
}

impl ChatSseTranslator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed one upstream SSE `data:` payload; returns the responses events to
    /// emit (each becomes one `data: {json}\n\n` frame downstream).
    pub fn feed(&mut self, data: &str) -> Vec<String> {
        let trimmed = data.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }
        if trimmed == "[DONE]" || trimmed == "DONE" {
            return self.finish();
        }
        let chunk: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };
        let mut events = Vec::new();
        if let Some(usage) = chunk.get("usage") {
            self.usage = Some(chat_usage(usage));
        }
        if self.response_id.is_empty() {
            if let Some(id) = chunk.get("id").and_then(|i| i.as_str()) {
                if !id.is_empty() {
                    self.response_id = id.to_string();
                }
            }
        }
        if let Some(choices) = chunk.get("choices").and_then(|c| c.as_array()) {
            for choice in choices {
                events.extend(self.feed_choice(choice));
            }
        }
        events
    }

    fn feed_choice(&mut self, choice: &Value) -> Vec<String> {
        let mut events = Vec::new();
        let delta = choice.get("delta").unwrap_or(&Value::Null);

        if let Some(seg) = delta.get("reasoning").and_then(reasoning_segment) {
            if !seg.is_empty() {
                if self.reasoning_text.is_none() {
                    events.push(
                        json!({
                            "type": "response.output_item.added",
                            "item": {"type": "reasoning", "summary": [], "content": []}
                        })
                        .to_string(),
                    );
                }
                events.push(
                    json!({
                        "type": "response.reasoning_text.delta",
                        "delta": seg,
                        "content_index": self.reasoning_segment_count
                    })
                    .to_string(),
                );
                self.reasoning_segment_count += 1;
                self.reasoning_text = Some(match self.reasoning_text.take() {
                    Some(prev) => prev + &seg,
                    None => seg,
                });
            }
        }

        if let Some(content) = delta.get("content") {
            for part in content_parts(content) {
                if part.is_empty() {
                    continue;
                }
                if self.assistant_text.is_none() {
                    events.push(
                        json!({
                            "type": "response.output_item.added",
                            "item": {"type": "message", "role": "assistant", "content": []}
                        })
                        .to_string(),
                    );
                }
                events
                    .push(json!({"type": "response.output_text.delta", "delta": part}).to_string());
                self.assistant_text = Some(match self.assistant_text.take() {
                    Some(prev) => prev + &part,
                    None => part,
                });
            }
        }

        if let Some(tool_calls) = delta.get("tool_calls").and_then(|t| t.as_array()) {
            for tc in tool_calls {
                let index = match tc.get("index").and_then(|i| i.as_u64()) {
                    Some(i) => i as usize,
                    None => match self.last_tool_call_index {
                        Some(i) => i,
                        None => self.tool_calls.keys().max().map(|k| k + 1).unwrap_or(0),
                    },
                };
                self.last_tool_call_index = Some(index);
                if let Some(id) = tc.get("id").and_then(|i| i.as_str()) {
                    if !id.is_empty() {
                        self.tool_call_index_by_id.insert(id.to_string(), index);
                    }
                }
                let entry = self
                    .tool_calls
                    .entry(index)
                    .or_insert_with(|| ToolCallState {
                        id: String::new(),
                        name: String::new(),
                        arguments: String::new(),
                    });
                if entry.id.is_empty() {
                    if let Some(id) = tc.get("id").and_then(|i| i.as_str()) {
                        entry.id = id.to_string();
                    }
                }
                if entry.name.is_empty() {
                    if let Some(name) = tc
                        .get("function")
                        .and_then(|f| f.get("name"))
                        .and_then(|n| n.as_str())
                    {
                        entry.name = name.to_string();
                    }
                }
                if let Some(arg) = tc
                    .get("function")
                    .and_then(|f| f.get("arguments"))
                    .and_then(|a| a.as_str())
                {
                    entry.arguments.push_str(arg);
                }
                if !self.tool_call_order.contains(&index) {
                    self.tool_call_order.push(index);
                }
            }
        }

        match choice.get("finish_reason").and_then(|f| f.as_str()) {
            Some("stop") => {
                // Flush done items now; response.completed waits for finish()
                // so the usage chunk (if any) can ride along.
                events.extend(self.flush_reasoning_done());
                events.extend(self.flush_message_done());
            }
            Some("length") => {
                events.push(
                    json!({
                        "type": "response.failed",
                        "response": {
                            "error": {
                                "code": "context_length_exceeded",
                                "message": "The model's context window was exceeded"
                            }
                        }
                    })
                    .to_string(),
                );
                self.terminal_emitted = true;
            }
            Some("tool_calls") => {
                events.extend(self.flush_reasoning_done());
                let order = std::mem::take(&mut self.tool_call_order);
                for index in order {
                    let state = self
                        .tool_calls
                        .get(&index)
                        .expect("tool call state for ordered index");
                    let call_id = if state.id.is_empty() {
                        format!("tool-call-{index}")
                    } else {
                        state.id.clone()
                    };
                    events.push(
                        json!({
                            "type": "response.output_item.done",
                            "item": {
                                "type": "function_call",
                                "name": state.name,
                                "arguments": state.arguments,
                                "call_id": call_id
                            }
                        })
                        .to_string(),
                    );
                }
                self.tool_calls.clear();
                self.tool_call_index_by_id.clear();
                self.last_tool_call_index = None;
            }
            _ => {}
        }
        events
    }

    fn flush_reasoning_done(&mut self) -> Vec<String> {
        match self.reasoning_text.take() {
            Some(text) => vec![json!({
                "type": "response.output_item.done",
                "item": {
                    "type": "reasoning",
                    "summary": [],
                    "content": [{"type": "reasoning_text", "text": text}]
                }
            })
            .to_string()],
            None => Vec::new(),
        }
    }

    fn flush_message_done(&mut self) -> Vec<String> {
        match self.assistant_text.take() {
            Some(text) => vec![json!({
                "type": "response.output_item.done",
                "item": {
                    "type": "message",
                    "role": "assistant",
                    "content": [{"type": "output_text", "text": text}]
                }
            })
            .to_string()],
            None => Vec::new(),
        }
    }

    /// Call after the upstream stream ends (EOF or `[DONE]`): flushes any
    /// unclosed reasoning/message items and emits `response.completed`.
    /// Idempotent — once a terminal state was reached it returns empty.
    pub fn finish(&mut self) -> Vec<String> {
        if self.terminal_emitted {
            return Vec::new();
        }
        let mut events = Vec::new();
        events.extend(self.flush_reasoning_done());
        events.extend(self.flush_message_done());
        let mut response = Map::new();
        response.insert("id".into(), Value::String(self.response_id.clone()));
        if let Some(usage) = self.usage.take() {
            response.insert("usage".into(), usage);
        }
        events.push(json!({"type": "response.completed", "response": response}).to_string());
        self.terminal_emitted = true;
        events
    }

    /// Synthesize the full responses event sequence from a single (non-
    /// streamed) `chat.completion` body. `response.created` is optional for
    /// codex, so it is not emitted.
    pub fn from_chat_completion(body: &Value) -> Vec<String> {
        let mut events = Vec::new();
        let Some(choice) = body
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|c| c.first())
        else {
            return events;
        };
        let message = choice.get("message").unwrap_or(&Value::Null);

        if let Some(seg) = message.get("reasoning").and_then(reasoning_segment) {
            if !seg.is_empty() {
                events.push(
                    json!({
                        "type": "response.output_item.added",
                        "item": {"type": "reasoning", "summary": [], "content": []}
                    })
                    .to_string(),
                );
                events.push(
                    json!({
                        "type": "response.reasoning_text.delta",
                        "delta": seg,
                        "content_index": 0
                    })
                    .to_string(),
                );
                events.push(
                    json!({
                        "type": "response.output_item.done",
                        "item": {
                            "type": "reasoning",
                            "summary": [],
                            "content": [{"type": "reasoning_text", "text": seg}]
                        }
                    })
                    .to_string(),
                );
            }
        }

        let text: String = message
            .get("content")
            .map(|c| content_parts(c).join(""))
            .unwrap_or_default();
        if !text.is_empty() {
            events.push(
                json!({
                    "type": "response.output_item.added",
                    "item": {"type": "message", "role": "assistant", "content": []}
                })
                .to_string(),
            );
            events.push(json!({"type": "response.output_text.delta", "delta": text}).to_string());
            events.push(
                json!({
                    "type": "response.output_item.done",
                    "item": {
                        "type": "message",
                        "role": "assistant",
                        "content": [{"type": "output_text", "text": text}]
                    }
                })
                .to_string(),
            );
        }

        if let Some(tool_calls) = message.get("tool_calls").and_then(|t| t.as_array()) {
            for tc in tool_calls {
                let name = tc
                    .get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string();
                let arguments = match tc.get("function").and_then(|f| f.get("arguments")) {
                    Some(Value::String(s)) => s.clone(),
                    Some(v) => v.to_string(),
                    None => String::new(),
                };
                let call_id = tc
                    .get("id")
                    .and_then(|i| i.as_str())
                    .unwrap_or("")
                    .to_string();
                events.push(
                    json!({
                        "type": "response.output_item.done",
                        "item": {
                            "type": "function_call",
                            "name": name,
                            "arguments": arguments,
                            "call_id": call_id
                        }
                    })
                    .to_string(),
                );
            }
        }

        let mut response = Map::new();
        response.insert(
            "id".into(),
            Value::String(
                body.get("id")
                    .and_then(|i| i.as_str())
                    .unwrap_or("")
                    .to_string(),
            ),
        );
        if let Some(usage) = body.get("usage") {
            response.insert("usage".into(), chat_usage(usage));
        }
        events.push(json!({"type": "response.completed", "response": response}).to_string());
        events
    }
}

/// Extract a reasoning segment from a `delta.reasoning` / `message.reasoning`
/// value: a string, or an object's `.text` / `.content` string.
fn reasoning_segment(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Object(_) => v
            .get("text")
            .and_then(|t| t.as_str())
            .or_else(|| v.get("content").and_then(|c| c.as_str()))
            .map(String::from),
        _ => None,
    }
}

/// Extract text parts from a `delta.content` / `message.content` value: a
/// plain string, or each `.text` field of an array of content parts.
fn content_parts(v: &Value) -> Vec<String> {
    match v {
        Value::String(s) => vec![s.clone()],
        Value::Array(parts) => parts
            .iter()
            .filter_map(|p| p.get("text").and_then(|t| t.as_str()).map(String::from))
            .collect(),
        _ => Vec::new(),
    }
}

/// Map a chat-completions usage object into the responses
/// `ResponseCompletedUsage` wire shape.
fn chat_usage(usage: &Value) -> Value {
    json!({
        "input_tokens": usage.get("prompt_tokens").and_then(|t| t.as_u64()).unwrap_or(0),
        "output_tokens": usage.get("completion_tokens").and_then(|t| t.as_u64()).unwrap_or(0),
        "total_tokens": usage.get("total_tokens").and_then(|t| t.as_u64()).unwrap_or(0),
    })
}

/// Incremental SSE frame decoder: splits a byte stream on blank lines and
/// extracts the joined `data:` payload of each frame. `event:` / `id:` /
/// `retry:` lines and `:` comments are ignored — downstream only parses
/// `data:` JSON.
#[derive(Default)]
pub struct SseFrameDecoder {
    buf: Vec<u8>,
}

impl SseFrameDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed raw bytes; returns the complete `data:` payloads — each was one
    /// SSE frame, possibly split across multiple `push()` calls.
    pub fn push(&mut self, bytes: &[u8]) -> Vec<String> {
        self.buf.extend_from_slice(bytes);
        let mut frames = Vec::new();
        while let Some(end) = find_frame_end(&self.buf) {
            let frame: Vec<u8> = self.buf.drain(..end).collect();
            if let Some(data) = parse_frame(&frame) {
                frames.push(data);
            }
        }
        frames
    }

    /// EOF: emit whatever trailing bytes remain (an unterminated final
    /// frame).
    pub fn flush(&mut self) -> Vec<String> {
        if self.buf.is_empty() {
            return Vec::new();
        }
        let frame = std::mem::take(&mut self.buf);
        parse_frame(&frame).into_iter().collect()
    }
}

/// Find the byte index just past the first blank line (`\n\n` or `\r\n\r\n`).
fn find_frame_end(buf: &[u8]) -> Option<usize> {
    let mut i = 0;
    while i + 1 < buf.len() {
        if buf[i] == b'\n' {
            if buf[i + 1] == b'\n' {
                return Some(i + 2);
            }
            if i + 2 < buf.len() && buf[i + 1] == b'\r' && buf[i + 2] == b'\n' {
                return Some(i + 3);
            }
        }
        i += 1;
    }
    None
}

/// Extract the joined `data:` payload from one frame's bytes.
fn parse_frame(frame: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(frame);
    let mut data_lines: Vec<&str> = Vec::new();
    for raw_line in text.split('\n') {
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if let Some(rest) = line.strip_prefix("data:") {
            data_lines.push(rest.strip_prefix(' ').unwrap_or(rest));
        }
    }
    if data_lines.is_empty() {
        None
    } else {
        Some(data_lines.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn messages_of(body: &Value) -> Vec<Value> {
        responses_to_chat_request(body)["messages"]
            .as_array()
            .expect("messages array")
            .clone()
    }

    fn parse(s: &str) -> Value {
        serde_json::from_str(s).expect("valid JSON")
    }

    // ── request conversion ────────────────────────────────────────────────

    #[test]
    fn request_instructions_map_to_system() {
        let body = json!({
            "model": "gpt-test",
            "instructions": "sys",
            "input": [{"type": "message", "role": "user", "content": [{"type": "input_text", "text": "hi"}]}],
            "stream": true
        });
        let messages = messages_of(&body);
        assert_eq!(messages[0], json!({"role": "system", "content": "sys"}));
    }

    #[test]
    fn request_instructions_default_to_empty_string() {
        let body = json!({"model": "m", "input": []});
        let messages = messages_of(&body);
        assert_eq!(messages[0], json!({"role": "system", "content": ""}));
    }

    #[test]
    fn request_user_text_concatenates_parts() {
        let body = json!({
            "model": "m",
            "input": [{"type": "message", "role": "user", "content": [
                {"type": "input_text", "text": "Hello "},
                {"type": "input_text", "text": "world"}
            ]}]
        });
        let messages = messages_of(&body);
        assert_eq!(
            messages[1],
            json!({"role": "user", "content": "Hello world"})
        );
    }

    #[test]
    fn request_assistant_dedupes_consecutive_text() {
        let body = json!({
            "model": "m",
            "input": [
                {"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "same"}]},
                {"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "same"}]},
                {"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "different"}]}
            ]
        });
        let messages = messages_of(&body);
        assert_eq!(messages.len(), 3, "system + 2 assistant messages");
        assert_eq!(messages[1], json!({"role": "assistant", "content": "same"}));
        assert_eq!(
            messages[2],
            json!({"role": "assistant", "content": "different"})
        );
    }

    #[test]
    fn request_function_calls_merge_into_one_assistant_message() {
        let body = json!({
            "model": "m",
            "input": [
                {"type": "message", "role": "user", "content": [{"type": "input_text", "text": "hi"}]},
                {"type": "function_call", "call_id": "c1", "name": "f1", "arguments": "{\"a\":1}"},
                {"type": "function_call", "call_id": "c2", "name": "f2", "arguments": "{}"}
            ]
        });
        let messages = messages_of(&body);
        assert_eq!(messages.len(), 3, "system + user + assistant tool msg");
        let tool_msg = &messages[2];
        assert_eq!(tool_msg["role"], "assistant");
        assert_eq!(tool_msg["content"], Value::Null);
        let calls = tool_msg["tool_calls"].as_array().expect("tool_calls");
        assert_eq!(calls.len(), 2);
        assert_eq!(
            calls[0],
            json!({"id": "c1", "type": "function", "function": {"name": "f1", "arguments": "{\"a\":1}"}})
        );
        assert_eq!(
            calls[1],
            json!({"id": "c2", "type": "function", "function": {"name": "f2", "arguments": "{}"}})
        );
    }

    #[test]
    fn request_function_call_output_string_form() {
        let body = json!({
            "model": "m",
            "input": [{"type": "function_call_output", "call_id": "c1", "output": "42"}]
        });
        let messages = messages_of(&body);
        assert_eq!(
            messages[1],
            json!({"role": "tool", "tool_call_id": "c1", "content": "42"})
        );
    }

    #[test]
    fn request_function_call_output_content_field() {
        let body = json!({
            "model": "m",
            "input": [{"type": "function_call_output", "call_id": "c2", "output": {"content": "result text"}}]
        });
        let messages = messages_of(&body);
        assert_eq!(
            messages[1],
            json!({"role": "tool", "tool_call_id": "c2", "content": "result text"})
        );
    }

    #[test]
    fn request_function_call_output_content_items() {
        let body = json!({
            "model": "m",
            "input": [{"type": "function_call_output", "call_id": "c3", "output": {"content_items": [
                {"type": "output_text", "text": "line1"},
                {"type": "output_image", "image_url": {"url": "https://x/y.png"}}
            ]}}]
        });
        let messages = messages_of(&body);
        assert_eq!(
            messages[1],
            json!({
                "role": "tool",
                "tool_call_id": "c3",
                "content": [
                    {"type": "text", "text": "line1"},
                    {"type": "image_url", "image_url": {"url": "https://x/y.png"}}
                ]
            })
        );
    }

    #[test]
    fn request_reasoning_after_user_is_skipped() {
        let body = json!({
            "model": "m",
            "input": [
                {"type": "message", "role": "user", "content": [{"type": "input_text", "text": "q"}]},
                {"type": "reasoning", "content": [{"type": "reasoning_text", "text": "think"}]},
                {"type": "message", "role": "user", "content": [{"type": "input_text", "text": "q2"}]}
            ]
        });
        let messages = messages_of(&body);
        assert_eq!(messages.len(), 3, "system + 2 user messages");
        assert!(messages.iter().all(|m| m.get("reasoning").is_none()));
    }

    #[test]
    fn request_reasoning_attaches_to_previous_assistant() {
        let body = json!({
            "model": "m",
            "input": [
                {"type": "message", "role": "user", "content": [{"type": "input_text", "text": "q"}]},
                {"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "ans"}]},
                {"type": "reasoning", "content": [{"type": "reasoning_text", "text": "hmm"}]},
                {"type": "message", "role": "user", "content": [{"type": "input_text", "text": "q2"}]}
            ]
        });
        let messages = messages_of(&body);
        assert_eq!(messages[2]["reasoning"], "hmm");
    }

    #[test]
    fn request_reasoning_joins_with_newline_on_same_anchor() {
        let body = json!({
            "model": "m",
            "input": [
                {"type": "message", "role": "user", "content": [{"type": "input_text", "text": "q"}]},
                {"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "ans"}]},
                {"type": "reasoning", "content": [{"type": "reasoning_text", "text": "a"}]},
                {"type": "reasoning", "content": [{"type": "text", "text": "b"}]}
            ]
        });
        let messages = messages_of(&body);
        assert_eq!(messages[2]["reasoning"], "a\nb");
    }

    #[test]
    fn request_reasoning_pending_attaches_to_next_function_call() {
        // Reasoning before any user message: the last_emitted_role guard
        // passes and there is no previous assistant — the text is held and
        // attached to the next function_call message.
        let body = json!({
            "model": "m",
            "input": [
                {"type": "reasoning", "content": [{"type": "reasoning_text", "text": "pre-think"}]},
                {"type": "function_call", "call_id": "c1", "name": "f1", "arguments": "{}"}
            ]
        });
        let messages = messages_of(&body);
        assert_eq!(messages.len(), 2, "system + assistant tool msg");
        assert_eq!(messages[1]["reasoning"], "pre-think");
    }

    #[test]
    fn request_reasoning_pending_attaches_to_next_assistant() {
        let body = json!({
            "model": "m",
            "input": [
                {"type": "reasoning", "content": [{"type": "reasoning_text", "text": "think"}]},
                {"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "ans"}]}
            ]
        });
        let messages = messages_of(&body);
        assert_eq!(
            messages[1],
            json!({"role": "assistant", "content": "ans", "reasoning": "think"})
        );
    }

    #[test]
    fn request_image_message_becomes_content_array() {
        let body = json!({
            "model": "m",
            "input": [{"type": "message", "role": "user", "content": [
                {"type": "input_text", "text": "what is this"},
                {"type": "input_image", "image_url": {"url": "https://x/y.png"}}
            ]}]
        });
        let messages = messages_of(&body);
        assert_eq!(
            messages[1],
            json!({
                "role": "user",
                "content": [
                    {"type": "text", "text": "what is this"},
                    {"type": "image_url", "image_url": {"url": "https://x/y.png"}}
                ]
            })
        );
    }

    #[test]
    fn request_filters_non_function_tools_and_shapes_functions() {
        let body = json!({
            "model": "m",
            "tools": [
                {"type": "function", "name": "f1", "description": "d", "parameters": {"type": "object"}, "strict": false},
                {"type": "custom_tool", "name": "ct", "parameters": {}}
            ]
        });
        let out = responses_to_chat_request(&body);
        let tools = out["tools"].as_array().expect("tools array");
        assert_eq!(tools.len(), 1);
        assert_eq!(
            tools[0],
            json!({
                "type": "function",
                "name": "f1",
                "function": {"name": "f1", "description": "d", "parameters": {"type": "object"}, "strict": false}
            })
        );
    }

    #[test]
    fn request_omits_tools_when_only_non_function() {
        let body = json!({
            "model": "m",
            "tools": [{"type": "web_search", "name": "ws"}]
        });
        let out = responses_to_chat_request(&body);
        assert!(out.get("tools").is_none());
    }

    #[test]
    fn request_drops_unknown_fields_and_keeps_exact_keys() {
        let body = json!({
            "model": "m",
            "instructions": "sys",
            "store": true,
            "include": ["reasoning"],
            "tool_choice": "auto",
            "parallel_tool_calls": true,
            "reasoning": {"effort": "high"},
            "text": {"format": {"type": "text"}},
            "conversation": "abc",
            "input": []
        });
        let out = responses_to_chat_request(&body);
        let mut keys: Vec<&str> = out
            .as_object()
            .unwrap()
            .keys()
            .map(|k| k.as_str())
            .collect();
        keys.sort_unstable();
        assert_eq!(keys, vec!["messages", "model", "stream", "stream_options"]);
        assert_eq!(out["stream"], true);
        assert_eq!(out["stream_options"], json!({"include_usage": true}));
    }

    #[test]
    fn request_skips_untranslatable_input_types() {
        let body = json!({
            "model": "m",
            "input": [
                {"type": "custom_tool_call", "name": "ct"},
                {"type": "local_shell_call", "command": "ls"},
                {"type": "web_search_call", "query": "x"},
                {"type": "ghost", "content": []}
            ]
        });
        let messages = messages_of(&body);
        assert_eq!(messages.len(), 1, "only the system message");
    }

    // ── SSE translator ────────────────────────────────────────────────────

    fn event_types(events: &[String]) -> Vec<String> {
        events
            .iter()
            .map(|e| parse(e)["type"].as_str().expect("event type").to_string())
            .collect()
    }

    #[test]
    fn sse_text_stream_event_sequence() {
        let mut t = ChatSseTranslator::new();
        let mut events: Vec<String> = Vec::new();
        events
            .extend(t.feed(r#"{"id":"cmpl-1","choices":[{"index":0,"delta":{"content":"Hel"}}]}"#));
        events.extend(t.feed(r#"{"choices":[{"index":0,"delta":{"content":"lo"}}]}"#));
        events.extend(t.feed(r#"{"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}"#));
        events.extend(t.finish());

        assert_eq!(
            event_types(&events),
            vec![
                "response.output_item.added",
                "response.output_text.delta",
                "response.output_text.delta",
                "response.output_item.done",
                "response.completed"
            ]
        );
        assert_eq!(
            parse(&events[0])["item"],
            json!({"type": "message", "role": "assistant", "content": []})
        );
        assert_eq!(parse(&events[1])["delta"], "Hel");
        assert_eq!(parse(&events[2])["delta"], "lo");
        assert_eq!(
            parse(&events[3])["item"],
            json!({"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "Hello"}]})
        );
        assert_eq!(parse(&events[4])["type"], "response.completed");
        assert_eq!(parse(&events[4])["response"]["id"], "cmpl-1");
        assert!(parse(&events[4])["response"].get("usage").is_none());
    }

    #[test]
    fn sse_reasoning_deltas_carry_content_index() {
        let mut t = ChatSseTranslator::new();
        let mut events: Vec<String> = Vec::new();
        events.extend(t.feed(r#"{"choices":[{"index":0,"delta":{"reasoning":"think"}}]}"#));
        events.extend(t.feed(r#"{"choices":[{"index":0,"delta":{"reasoning":" more"}}]}"#));
        events.extend(t.feed(r#"{"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}"#));
        events.extend(t.finish());

        assert_eq!(
            event_types(&events),
            vec![
                "response.output_item.added",
                "response.reasoning_text.delta",
                "response.reasoning_text.delta",
                "response.output_item.done",
                "response.completed"
            ]
        );
        assert_eq!(
            parse(&events[0])["item"],
            json!({"type": "reasoning", "summary": [], "content": []})
        );
        assert_eq!(parse(&events[1])["delta"], "think");
        assert_eq!(parse(&events[1])["content_index"], 0);
        assert_eq!(parse(&events[2])["delta"], " more");
        assert_eq!(parse(&events[2])["content_index"], 1);
        assert_eq!(
            parse(&events[3])["item"],
            json!({"type": "reasoning", "summary": [], "content": [{"type": "reasoning_text", "text": "think more"}]})
        );
    }

    #[test]
    fn sse_interleaved_tool_calls_flush_two_function_calls() {
        let mut t = ChatSseTranslator::new();
        let mut events: Vec<String> = Vec::new();
        events.extend(t.feed(
            r#"{"choices":[{"index":0,"delta":{"tool_calls":[
                {"index":0,"id":"call_1","type":"function","function":{"name":"get_weather","arguments":""}},
                {"index":1,"id":"call_2","type":"function","function":{"name":"get_time","arguments":"{}"}}
            ]}}]}"#,
        ));
        events.extend(t.feed(
            r#"{"choices":[{"index":0,"delta":{"tool_calls":[
                {"index":0,"function":{"arguments":"{\"city\":\"SF\"}"}}
            ]}}]}"#,
        ));
        events
            .extend(t.feed(r#"{"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}"#));
        events.extend(t.finish());

        // No events during buffering; only the two done events at the end.
        assert_eq!(
            event_types(&events),
            vec![
                "response.output_item.done",
                "response.output_item.done",
                "response.completed"
            ]
        );
        assert_eq!(
            parse(&events[0])["item"],
            json!({
                "type": "function_call",
                "name": "get_weather",
                "arguments": "{\"city\":\"SF\"}",
                "call_id": "call_1"
            })
        );
        assert_eq!(
            parse(&events[1])["item"],
            json!({
                "type": "function_call",
                "name": "get_time",
                "arguments": "{}",
                "call_id": "call_2"
            })
        );
        // finish() after tool_calls flush: no dangling items, plain completed.
        assert_eq!(parse(&events[2])["type"], "response.completed");
    }

    #[test]
    fn sse_tool_call_without_id_gets_generated_call_id() {
        let mut t = ChatSseTranslator::new();
        let mut events: Vec<String> = Vec::new();
        events.extend(t.feed(
            r#"{"choices":[{"index":0,"delta":{"tool_calls":[
                {"index":0,"function":{"name":"f","arguments":"{}"}}
            ]}}]}"#,
        ));
        events
            .extend(t.feed(r#"{"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}"#));
        assert_eq!(events.len(), 1);
        assert_eq!(parse(&events[0])["item"]["call_id"], "tool-call-0");
    }

    #[test]
    fn sse_length_finish_reason_emits_failed() {
        let mut t = ChatSseTranslator::new();
        let mut events: Vec<String> = Vec::new();
        events.extend(t.feed(r#"{"choices":[{"index":0,"delta":{"content":"partial"}}]}"#));
        events.extend(t.feed(r#"{"choices":[{"index":0,"delta":{},"finish_reason":"length"}]}"#));
        assert_eq!(
            event_types(&events),
            vec![
                "response.output_item.added",
                "response.output_text.delta",
                "response.failed"
            ]
        );
        let failed = parse(&events[2]);
        assert_eq!(
            failed["response"]["error"]["code"],
            "context_length_exceeded"
        );
        // Terminal state: finish() must emit nothing further.
        assert!(t.finish().is_empty());
        assert!(t.feed("[DONE]").is_empty());
    }

    #[test]
    fn sse_done_without_finish_reason_flushed_by_finish() {
        let mut t = ChatSseTranslator::new();
        let mut events: Vec<String> = Vec::new();
        events
            .extend(t.feed(r#"{"id":"cmpl-2","choices":[{"index":0,"delta":{"content":"hi"}}]}"#));
        events.extend(t.feed("[DONE]"));
        assert_eq!(
            event_types(&events),
            vec![
                "response.output_item.added",
                "response.output_text.delta",
                "response.output_item.done",
                "response.completed"
            ]
        );
        assert_eq!(parse(&events[3])["response"]["id"], "cmpl-2");
    }

    #[test]
    fn sse_usage_chunk_maps_into_completed() {
        let mut t = ChatSseTranslator::new();
        let mut events: Vec<String> = Vec::new();
        events.extend(t.feed(r#"{"choices":[{"index":0,"delta":{"content":"x"}}]}"#));
        events.extend(t.feed(
            r#"{"choices":[],"usage":{"prompt_tokens":10,"completion_tokens":5,"total_tokens":15}}"#,
        ));
        events.extend(t.finish());
        let completed = parse(events.last().expect("completed event"));
        assert_eq!(completed["type"], "response.completed");
        assert_eq!(
            completed["response"]["usage"],
            json!({"input_tokens": 10, "output_tokens": 5, "total_tokens": 15})
        );
    }

    #[test]
    fn sse_invalid_json_data_lines_are_skipped() {
        let mut t = ChatSseTranslator::new();
        assert!(t.feed("not json").is_empty());
        assert!(t.feed("").is_empty());
        assert!(t.feed("{}").is_empty(), "empty chunk emits nothing");
        assert!(t.feed("  ").is_empty());
    }

    #[test]
    fn sse_content_array_parts_each_emit_delta() {
        let mut t = ChatSseTranslator::new();
        let mut events: Vec<String> = Vec::new();
        events.extend(t.feed(
            r#"{"choices":[{"index":0,"delta":{"content":[{"type":"text","text":"a"},{"type":"text","text":"b"}]}}]}"#,
        ));
        events.extend(t.feed(r#"{"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}"#));
        events.extend(t.finish());
        assert_eq!(
            event_types(&events),
            vec![
                "response.output_item.added",
                "response.output_text.delta",
                "response.output_text.delta",
                "response.output_item.done",
                "response.completed"
            ]
        );
        assert_eq!(parse(&events[1])["delta"], "a");
        assert_eq!(parse(&events[2])["delta"], "b");
        assert_eq!(parse(&events[3])["item"]["content"][0]["text"], "ab");
    }

    // ── non-streamed completion ───────────────────────────────────────────

    #[test]
    fn from_chat_completion_builds_full_sequence() {
        let body = json!({
            "id": "cmpl-ns",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "reasoning": "ponder",
                    "content": "full answer",
                    "tool_calls": [
                        {"id": "call_9", "type": "function", "function": {"name": "f", "arguments": "{}"}}
                    ]
                },
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 3, "completion_tokens": 2, "total_tokens": 5}
        });
        let events = ChatSseTranslator::from_chat_completion(&body);
        assert_eq!(
            event_types(&events),
            vec![
                "response.output_item.added",
                "response.reasoning_text.delta",
                "response.output_item.done",
                "response.output_item.added",
                "response.output_text.delta",
                "response.output_item.done",
                "response.output_item.done",
                "response.completed"
            ]
        );
        // reasoning
        assert_eq!(
            parse(&events[0])["item"],
            json!({"type": "reasoning", "summary": [], "content": []})
        );
        assert_eq!(parse(&events[1])["delta"], "ponder");
        assert_eq!(parse(&events[2])["item"]["content"][0]["text"], "ponder");
        // content: single delta with full text
        assert_eq!(parse(&events[4])["delta"], "full answer");
        assert_eq!(
            parse(&events[5])["item"],
            json!({"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "full answer"}]})
        );
        // tool call
        assert_eq!(
            parse(&events[6])["item"],
            json!({"type": "function_call", "name": "f", "arguments": "{}", "call_id": "call_9"})
        );
        // completed with usage
        let completed = parse(&events[7]);
        assert_eq!(completed["response"]["id"], "cmpl-ns");
        assert_eq!(
            completed["response"]["usage"],
            json!({"input_tokens": 3, "output_tokens": 2, "total_tokens": 5})
        );
    }

    #[test]
    fn from_chat_completion_without_choices_is_empty() {
        let body = json!({"error": {"message": "boom"}});
        assert!(ChatSseTranslator::from_chat_completion(&body).is_empty());
    }

    #[test]
    fn from_chat_completion_without_usage_omits_usage() {
        let body = json!({
            "id": "cmpl-x",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "ok"}}]
        });
        let events = ChatSseTranslator::from_chat_completion(&body);
        let completed = parse(events.last().expect("completed event"));
        assert_eq!(completed["response"]["id"], "cmpl-x");
        assert!(completed["response"].get("usage").is_none());
    }
}
