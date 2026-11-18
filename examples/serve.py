#!/usr/bin/env python3
"""Serves the rendered examples, plus the live endpoints the chart gallery reads.

    examples/build.sh
    python3 examples/serve.py                 # http://localhost:8000, from target/site
    python3 examples/serve.py 8080 out/site

Live endpoints, under any folder (the gallery asks for charts/api/live/...):
  api/live/metrics  JSON: the current cpu, memory and requests, smooth over time
  api/live/stream   server-sent events: an order every second
  api/chat          POST: the chats in the UI kit gallery and the chat block. Streams a canned
                    reply as server-sent events. "chart …" also gets UI as JSON (`ui`), which the
                    page renders: in the browser with irender-wasm, or through api/render.
                    "photos …" gets UI rendered here (`html`, with `irender render --embed`).
  api/render        POST { ui }: a message's UI rendered to markup, under the untrusted policy
  api/budget        JSON: the projects under a team in the budget block (`?parent=<team id>`),
                    for ui-data-table's `childrenUrl`.
  api/shipments     JSON: 5,000 made-up shipments for the data table blocks, a batch at a time.
                    Takes ui-data-table's query: offset, limit, sort (`-eta,id`), q (search),
                    filters (JSON conditions) and delay (ms); answers { rows, total }.
UI is rendered by the CLI: set IRENDER to the binary (default target/debug/irender).
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
REPLIES = os.path.join(HERE, "blocks", "chat-replies")
# Prompts that get UI: a word in the message picks the reply. "chart" sends the UI as
# JSON for the page to render; "photos" renders it here and sends the markup.
UI_REPLIES = {
    "chart": ("Here are this year's sales by month. Switch the range above the chart.", "chart.json"),
    "photos": ("Here are a few of Lisbon's sights. Scroll or use the arrows for more.", "photos.yaml"),
}
_rendered = {}


def make_shipments(n=5000):
    rng = random.Random(42)
    carriers = ["DHL", "FedEx", "UPS", "Maersk", "DB Schenker", "Kuehne+Nagel"]
    cities = ["Rotterdam", "Hamburg", "Shanghai", "Singapore", "Los Angeles", "Dubai", "Mumbai", "Santos", "Antwerp", "Busan"]
    out = []
    for i in range(n):
        origin, dest = rng.sample(cities, 2)
        status = rng.choices(["In transit", "Delivered", "Delayed", "Customs", "Booked"], [5, 6, 1, 1, 2])[0]
        out.append({
            "id": f"SHP-{100000 + i}",
            "carrier": rng.choice(carriers),
            "origin": origin,
            "destination": dest,
            "mode": rng.choice(["Sea", "Air", "Road", "Rail"]),
            "status": status,
            "weight": round(rng.uniform(40, 24000), 1),
            "value": round(rng.uniform(800, 250000), 2),
            "eta": f"2026-{rng.randint(1, 12):02d}-{rng.randint(1, 28):02d}",
            "progress": 100 if status == "Delivered" else 0 if status == "Booked" else rng.randint(5, 95),
        })
    return out


SHIPMENTS = make_shipments()


def budget_projects(parent):
    """A team's projects for the budget block: the same ones every time for a team."""
    rng = random.Random(parent)
    names = ["Platform", "Migration", "Launch", "Research", "Hiring", "Tooling", "Compliance", "Partnerships"]
    rows = []
    for i, name in enumerate(rng.sample(names, rng.randint(2, 4))):
        months = {f"m{m}": rng.randint(2, 30) * 500 for m in range(12)}
        rows.append({"id": f"{parent}-P{i + 1}", "name": f"{name} project", "owner": rng.choice(["Ada", "Grace", "Alan", "Katherine"]), **months, "total": sum(months.values())})
    time.sleep(0.4)
    return rows


def matches(v, c):
    """ui-data-table's (and ui-chart's) filter conditions, enough of them for the blocks."""
    if c is None or c == "" or c == []:
        return True
    if isinstance(c, list):
        return any(str(v) == str(x) for x in c)
    if isinstance(c, dict):
        if "contains" in c:
            return str(c["contains"]).lower() in str(v).lower()
        num = isinstance(v, (int, float))
        conv = (lambda x: float(x)) if num else str
        lo, hi = c.get("from"), c.get("to")
        if lo not in (None, "") and conv(v) < conv(lo):
            return False
        if hi not in (None, "") and conv(v) > conv(hi):
            return False
        return True
    return str(v) == str(c)


def shipments(query):
    rows = SHIPMENTS
    q = (query.get("q") or "").strip().lower()
    if q:
        rows = [r for r in rows if all(any(w in str(r[f]).lower() for f in ("id", "carrier", "origin", "destination")) for w in q.split())]
    for field, cond in json.loads(query.get("filters") or "{}").items():
        rows = [r for r in rows if matches(r.get(field), cond)]
    for key in reversed([k for k in (query.get("sort") or "").split(",") if k]):
        field = key.lstrip("-")
        rows = sorted(rows, key=lambda r: (r.get(field) is None, r.get(field)), reverse=key.startswith("-"))
    offset = max(0, int(query.get("offset") or 0))
    limit = min(500, max(1, int(query.get("limit") or 50)))
    time.sleep(min(2000, int(query.get("delay") or 300)) / 1000)
    return {"rows": rows[offset:offset + limit], "total": len(rows)}


def render_ui(ui):
    """A message's UI (a document or a node) as markup, as irender_ui_kit::render_ui does it."""
    doc = dict(ui, version=ui.get("version", 1)) if "root" in ui else {"version": 1, "root": ui}
    args = [IRENDER, "render", "-", "--embed", "--untrusted", "--ui-kit", "--chart-kit"]
    done = subprocess.run(args, input=json.dumps(doc), capture_output=True, text=True)
    if done.returncode != 0:
        raise ValueError(done.stderr.strip() or done.stdout.strip())
    return done.stdout


def render_reply(name):
    """A reply's UI document as markup for the page, rendered once."""
    if name not in _rendered:
        args = [IRENDER, "render", os.path.join(REPLIES, name), "--ui-kit", "--chart-kit", "--embed"]
        _rendered[name] = subprocess.run(args, capture_output=True, text=True, check=True).stdout
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
        if path.endswith("/api/budget"):
            from urllib.parse import parse_qsl
            query = dict(parse_qsl(self.path.split("?", 1)[1] if "?" in self.path else ""))
            return self.send_json(budget_projects(query.get("parent", "")))
        if path.endswith("/api/shipments"):
            from urllib.parse import parse_qsl
            query = dict(parse_qsl(self.path.split("?", 1)[1] if "?" in self.path else ""))
            try:
                return self.send_json(shipments(query))
            except (ValueError, TypeError) as e:
                return self.send_error(400, str(e))
        return super().do_GET()

    def do_POST(self):
        path = self.path.split("?", 1)[0]
        if path.endswith("/api/chat"):
            return self.chat()
        if path.endswith("/api/render"):
            return self.render()
        self.send_error(404)

    def render(self):
        try:
            ui = json.loads(self.rfile.read(int(self.headers.get("Content-Length") or 0)) or b"{}")["ui"]
            body, status = render_ui(ui).encode(), 200
        except (KeyError, TypeError, ValueError, OSError) as e:
            body, status = str(e).encode(), 422
        self.send_response(status)
        self.send_header("Content-Type", "text/html; charset=utf-8" if status == 200 else "text/plain; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def chat(self):
        body = json.loads(self.rfile.read(int(self.headers.get("Content-Length") or 0)) or b"{}")
        messages = body.get("messages") or []
        said = next((m.get("content", "") for m in reversed(messages) if m.get("role") == "user"), "")
        ui = next((v for k, v in UI_REPLIES.items() if k in said.lower()), None)
        extra = None
        if ui:
            reply = ui[0]
            try:
                if ui[1].endswith(".json"):
                    with open(os.path.join(REPLIES, ui[1])) as f:
                        extra = {"ui": json.load(f)}
                else:
                    extra = {"html": render_reply(ui[1])}
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
            if extra:
                self.wfile.write(f"data: {json.dumps(extra)}\n\n".encode())
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
