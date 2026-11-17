#!/usr/bin/env python3
"""Serves the rendered examples, plus the live endpoints the chart gallery reads.

    examples/build.sh
    python3 examples/serve.py                 # http://localhost:8000, from target/site
    python3 examples/serve.py 8080 out/site

Live endpoints, under any folder (the gallery asks for charts/api/live/...):
  api/live/metrics  JSON: the current cpu, memory and requests, smooth over time
  api/live/stream   server-sent events: an order every second
  api/chat          POST: the chats in the UI kit gallery and the chat block. Streams a canned
                    reply as server-sent events; "chart …" and "photos …" also get UI, an iRender
                    document from blocks/chat-replies rendered with `irender render --embed`
                    (set IRENDER to the binary; default target/debug/irender)
"""

import json
import math
import os
import random
import subprocess
import sys
import time
from datetime import datetime, timezone
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer

REGIONS = ["Europe", "Asia", "Americas"]
HERE = os.path.dirname(os.path.abspath(__file__))
IRENDER = os.environ.get("IRENDER", os.path.join(HERE, "..", "target", "debug", "irender"))
# Prompts that get UI: the first word in the message picks the reply.
UI_REPLIES = {
    "chart": ("Here are this year's sales by month. Switch the range above the chart.", "chart.yaml", "--chart-kit"),
    "photos": ("Here are a few of Lisbon's sights. Scroll or use the arrows for more.", "photos.yaml", None),
}
_rendered = {}


def render_reply(name, kit):
    """A reply's UI document as markup for the page, rendered once."""
    if name not in _rendered:
        args = [IRENDER, "render", os.path.join(HERE, "blocks", "chat-replies", name), "--ui-kit", "--embed"]
        _rendered[name] = subprocess.run(args + ([kit] if kit else []), capture_output=True, text=True, check=True).stdout
    return _rendered[name]


def metrics():
    t = time.time()
    return {
        "time": datetime.fromtimestamp(int(t), timezone.utc).isoformat().replace("+00:00", "Z"),
        "cpu": round(55 + 25 * math.sin(t / 20) + random.uniform(-6, 6), 1),
        "memory": round(62 + 8 * math.sin(t / 45 + 1) + random.uniform(-2, 2), 1),
        "requests": int(400 + 150 * math.sin(t / 30) + random.uniform(-40, 40)),
    }


class Handler(SimpleHTTPRequestHandler):
    def do_GET(self):
        path = self.path.split("?", 1)[0]
        if path.endswith("/api/live/metrics"):
            return self.send_json(metrics())
        if path.endswith("/api/live/stream"):
            return self.stream()
        return super().do_GET()

    def do_POST(self):
        if self.path.split("?", 1)[0].endswith("/api/chat"):
            return self.chat()
        self.send_error(404)

    def chat(self):
        body = json.loads(self.rfile.read(int(self.headers.get("Content-Length") or 0)) or b"{}")
        messages = body.get("messages") or []
        said = next((m.get("content", "") for m in reversed(messages) if m.get("role") == "user"), "")
        ui = next((v for k, v in UI_REPLIES.items() if k in said.lower()), None)
        html = None
        if ui:
            reply = ui[0]
            try:
                html = render_reply(ui[1], ui[2])
            except (OSError, subprocess.CalledProcessError) as e:
                reply += f" (The UI didn't render: {e}. Build the CLI or set IRENDER.)"
        else:
            reply = (
                f"You said “{said}”. This is a canned reply from examples/serve.py, streamed a word at a "
                f"time as server-sent events, like a model's would be. Ask for a chart or photos to get "
                f"a reply with UI. The conversation so far has {len(messages)} messages."
            )
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        try:
            time.sleep(0.6)
            for word in reply.split(" "):
                self.wfile.write(f"data: {json.dumps({'content': word + ' '})}\n\n".encode())
                self.wfile.flush()
                time.sleep(0.04)
            if html:
                self.wfile.write(f"data: {json.dumps({'html': html})}\n\n".encode())
            self.wfile.write(b"data: [DONE]\n\n")
        except (BrokenPipeError, ConnectionResetError):
            pass

    def send_json(self, value):
        body = json.dumps(value).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Cache-Control", "no-store")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def stream(self):
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        n = int(time.time())
        try:
            while True:
                n += 1
                order = {
                    "id": n,
                    "time": datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z"),
                    "region": random.choice(REGIONS),
                    "amount": round(random.lognormvariate(5.5, 0.6)),
                }
                self.wfile.write(f"data: {json.dumps(order)}\n\n".encode())
                self.wfile.flush()
                time.sleep(1)
        except (BrokenPipeError, ConnectionResetError):
            pass

    def log_message(self, *args):
        pass


def main():
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8000
    root = sys.argv[2] if len(sys.argv) > 2 else "target/site"
    server = ThreadingHTTPServer(("127.0.0.1", port), partial(Handler, directory=root))
    server.daemon_threads = True
    print(f"Serving {root} on http://localhost:{port}")
    server.serve_forever()


if __name__ == "__main__":
    main()
