# A local stand-in for the Anthropic API, so the proxy, `audit`, `hex` and `watch` get traffic with no
# network and no account. `count_tokens` answers bytes/4; `/v1/messages` answers a short SSE stream.
#   python3 upstream.py <port>     then [proxy] upstream = "http://127.0.0.1:<port>" in the profile
import json
import sys
from http.server import BaseHTTPRequestHandler, HTTPServer


class H(BaseHTTPRequestHandler):
    def log_message(self, *a):
        pass

    def do_POST(self):
        body = self.rfile.read(int(self.headers.get("content-length", 0)))
        if self.path.startswith("/v1/messages/count_tokens"):
            out = json.dumps({"input_tokens": len(body) // 4}).encode()
            self.send_response(200)
            self.send_header("content-type", "application/json")
            self.send_header("content-length", str(len(out)))
            self.end_headers()
            self.wfile.write(out)
            return
        n = len(body) // 4
        ev = [
            (
                "message_start",
                {
                    "type": "message_start",
                    "message": {
                        "id": "msg_local",
                        "type": "message",
                        "role": "assistant",
                        "model": "claude-sonnet-4-5",
                        "content": [],
                        "usage": {
                            "input_tokens": n,
                            "output_tokens": 1,
                            "cache_read_input_tokens": 0,
                            "cache_creation_input_tokens": 0,
                        },
                    },
                },
            ),
            (
                "content_block_start",
                {
                    "type": "content_block_start",
                    "index": 0,
                    "content_block": {"type": "text", "text": ""},
                },
            ),
            (
                "content_block_delta",
                {
                    "type": "content_block_delta",
                    "index": 0,
                    "delta": {"type": "text_delta", "text": "ok"},
                },
            ),
            ("content_block_stop", {"type": "content_block_stop", "index": 0}),
            (
                "message_delta",
                {
                    "type": "message_delta",
                    "delta": {"stop_reason": "end_turn"},
                    "usage": {"output_tokens": 1},
                },
            ),
            ("message_stop", {"type": "message_stop"}),
        ]
        out = "".join(f"event: {e}\ndata: {json.dumps(d)}\n\n" for e, d in ev).encode()
        self.send_response(200)
        self.send_header("content-type", "text/event-stream")
        self.send_header("content-length", str(len(out)))
        self.end_headers()
        self.wfile.write(out)


HTTPServer(("127.0.0.1", int(sys.argv[1])), H).serve_forever()
