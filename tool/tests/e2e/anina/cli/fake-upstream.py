"""A stand-in for the Anthropic API, so the proxy has somewhere to send requests in a recording.

Answers POST /v1/messages (JSON, or SSE when the body asks to stream) and
POST /v1/messages/count_tokens with about one token per four bytes. It listens on 127.0.0.1
only and never reaches a network.
    python3 fake-upstream.py <port>
"""

import json
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


def tokens(body: bytes) -> int:
    return max(1, len(body) // 4)


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args):  # quiet: the recording shows mandrake, not this server
        pass

    def reply(self, payload: dict, status: int = 200) -> None:
        data = json.dumps(payload).encode()
        self.send_response(status)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_POST(self):  # noqa: N802 (http.server names it)
        body = self.rfile.read(int(self.headers.get("content-length", 0)))
        path = self.path.split("?")[0]
        if path.endswith("/count_tokens"):
            return self.reply({"input_tokens": tokens(body)})
        if not path.endswith("/v1/messages"):
            return self.reply({"type": "error", "error": {"type": "not_found_error"}}, 404)
        usage = {
            "input_tokens": tokens(body),
            "output_tokens": 12,
            "cache_read_input_tokens": 0,
            "cache_creation_input_tokens": 0,
        }
        msg = {
            "id": "msg_fake",
            "type": "message",
            "role": "assistant",
            "model": "fake",
            "content": [{"type": "text", "text": "ok"}],
            "stop_reason": "end_turn",
            "usage": usage,
        }
        try:
            stream = json.loads(body or b"{}").get("stream", False)
        except ValueError:
            stream = False
        if not stream:
            return self.reply(msg)
        self.send_response(200)
        self.send_header("content-type", "text/event-stream")
        self.end_headers()
        start = dict(msg, content=[], usage=dict(usage, output_tokens=1))
        text = {"type": "text", "text": ""}
        events = [
            ("message_start", {"type": "message_start", "message": start}),
            ("content_block_start", {"type": "content_block_start", "index": 0, "content_block": text}),
            ("content_block_delta", {"type": "content_block_delta", "index": 0,
                                     "delta": {"type": "text_delta", "text": "ok"}}),
            ("content_block_stop", {"type": "content_block_stop", "index": 0}),
            ("message_delta", {"type": "message_delta", "delta": {"stop_reason": "end_turn"},
                               "usage": {"output_tokens": 12}}),
            ("message_stop", {"type": "message_stop"}),
        ]
        for name, data in events:
            self.wfile.write(f"event: {name}\ndata: {json.dumps(data)}\n\n".encode())
        self.wfile.flush()


if __name__ == "__main__":
    ThreadingHTTPServer(("127.0.0.1", int(sys.argv[1])), Handler).serve_forever()
