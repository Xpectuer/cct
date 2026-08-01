#!/usr/bin/env python3
"""PoC stub 上游 — responses-API SSE 契约（spec.md AC4）。

用法: stub-sse-upstream.py <port> <logfile>

行为:
- 启动成功时写一行 LISTENING 到 logfile（供 setup 检测就绪）
- 每个请求: 记录 method/path/Authorization/body 前 500 字符到 logfile
- 返回 item-based SSE 事件流: response.created -> response.output_item.added
  -> response.output_text.delta -> response.output_item.done -> response.completed
  (delta 固定为 POC_STUB_LAST_MESSAGE, 供 -o 文件末尾文本断言;
  codex 0.146 缺 output_item.added 会报 OutputTextDelta without active item)

契约细节若与 codex 客户端不匹配, PoC 结果将直接揭示 — 这正是验证目标。
"""
import sys
import socket
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer

PORT, LOG = int(sys.argv[1]), sys.argv[2]

DELTA = "POC_STUB_LAST_MESSAGE"

CREATED = {
    "type": "response.created",
    "response": {"id": "resp_poc_0001", "object": "response", "status": "in_progress", "output": []},
}
COMPLETED = {
    "type": "response.completed",
    "response": {
        "id": "resp_poc_0001", "object": "response", "status": "completed",
        "output": [{"type": "message", "role": "assistant",
                    "content": [{"type": "output_text", "text": DELTA, "annotations": []}]}],
    },
}
# codex 0.146 需要 item-based 事件序列: output_item.added 建立 active item 后,
# output_text.delta 才会被接受（否则 "OutputTextDelta without active item",
# -o 输出为空）。形状对齐 codex 自身测试 fixture（codex-rs/codex-api sse/responses.rs）。
OUTPUT_ITEM_ADDED = {
    "type": "response.output_item.added",
    "output_index": 0,
    "item": {"type": "message", "role": "assistant", "status": "in_progress", "content": []},
}
DELTA_EV = {"type": "response.output_text.delta", "delta": DELTA, "sequence_number": 1}
OUTPUT_ITEM_DONE = {
    "type": "response.output_item.done",
    "output_index": 0,
    "item": {"type": "message", "role": "assistant", "status": "completed",
            "content": [{"type": "output_text", "text": DELTA, "annotations": []}]},
}


def sse_frame(evt, obj):
    import json
    return f"event: {evt}\ndata: {json.dumps(obj)}\n\n".encode()


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *a):
        pass

    def _log(self):
        with open(LOG, "a") as f:
            auth = self.headers.get("Authorization", "")
            f.write(f"{self.command} {self.path} Authorization: {auth}\n")
            n = int(self.headers.get("content-length") or 0)
            if n:
                f.write("body=" + self.rfile.read(n).decode(errors="replace")[:500] + "\n")

    def _sse(self):
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.end_headers()
        for evt, obj in (("response.created", CREATED),
                         ("response.output_item.added", OUTPUT_ITEM_ADDED),
                         ("response.output_text.delta", DELTA_EV),
                         ("response.output_item.done", OUTPUT_ITEM_DONE),
                         ("response.completed", COMPLETED)):
            self.wfile.write(sse_frame(evt, obj))
            self.wfile.flush()

    def do_POST(self):
        self._log()
        self._sse()

    def do_GET(self):
        self._log()
        self._sse()


def main():
    with open(LOG, "a") as f:
        f.write("LISTENING\n")
    try:
        HTTPServer(("127.0.0.1", PORT), Handler).serve_forever()
    except OSError as e:
        with open(LOG, "a") as f:
            f.write(f"BIND_FAILED {e}\n")
        sys.exit(1)


if __name__ == "__main__":
    main()
