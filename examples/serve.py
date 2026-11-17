#!/usr/bin/env python3
"""Serves the rendered examples, plus the live endpoints the chart gallery reads.

    examples/build.sh
    python3 examples/serve.py                 # http://localhost:8000, from target/site
    python3 examples/serve.py 8080 out/site

Live endpoints, under any folder (the gallery asks for charts/api/live/...):
  api/live/metrics  JSON: the current cpu, memory and requests, smooth over time
  api/live/stream   server-sent events: an order every second
"""

import json
import math
import random
import sys
import time
from datetime import datetime, timezone
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer

REGIONS = ["Europe", "Asia", "Americas"]


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
