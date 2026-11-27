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
                    With facets=<fields>, also { facets: { <field>: [{ value, count }] } },
                    each counted under every filter but the field's own; with
                    summary=<field>:sum,…, also { summary: { "<field>:sum": n } }.
                    With groupBy=<field>, answers a row per group ({ <field>, count, value });
                    with group=<value> too, that group's rows, a batch at a time.
  api/upload        POST multipart (field `file`): keeps the file in memory for this run and
                    answers { id, url } for ui-file-upload. Files over 10 MB get 413.
  api/available JSON: whether a name is free (`?value=<name>`), for ui-form's `remote` rule:
                { valid } or { valid: false, message }. admin, root, support, help, imran and
                ada are taken.
  api/twin/plant         JSON: the packing hall's machines, vehicles and sensors (block twin-01), by id:
                         { "cv-1": { status, speed, temp }, …, "plant": { output, oee, … } }. A motor
                         on line 1 overheats and line 2 stops for a while every few minutes.
  api/twin/plant/stream  the same, as server-sent events, once a second
  api/twin/coldstore     JSON: the cold store's sensors and doors (block twin-02); freezer B's door
                         is left open now and then and its sensors warm up.
  api/twin/bottling       JSON: the bottling line in block twin-04, by id: silos, the pasteuriser,
                         the mixer, filler, capper, checkweigher, conveyors, the bottles on the belt,
                         cold store and room sensors, and "line" (product, batch, speed, counts).
                         The pasteuriser dips under 72 °C and diverts now and then, which stops the
                         line; a cold store sensor warms up later in the same five minutes.
  api/twin/bottling/stream  the same, as server-sent events, once a second
  api/tickets   JSON: 400 made-up support tickets (block data-table-09), with ui-data-table's
                query as api/shipments (offset, limit, sort, q, filters, facets). Each names
                its requester and assignee (`requesterId`, `assigneeId`: api/people) and
                its account (`accountId`: api/accounts) by id only.
  api/people    JSON: people by id, many at a time (`?ids=u-01,u-02`, a list) after 600 ms,
                or one (`api/people/u-01`); an unknown id is left out, or 404.
  api/accounts/<id>  JSON: one account, after 250 ms: for a source that fetches one at a time.
                With an ETag: asked again with If-None-Match, it answers 304 when unchanged.
  api/appointments   JSON: a clinic's day (block clinic-01), `?day=YYYY-MM-DD&clinician=<id>`:
                { rows, total, cursor }, each naming its patient and clinician by id only.
                Every 20 seconds something changes (a status moves on, or a visit is
                cancelled); `since=<cursor>` answers only that: { rows, deleted, cursor }.
  api/clinicians     JSON: clinicians by id, `?ids=c-1,c-2`, after 400 ms.
  api/patients/batch POST { ids }: patients by id after 700 ms, as { data: { <id>: patient } }.
                Their details are personal: the page fetches them, nothing keeps them.
  api/records   POST anything as JSON: answers { id, size }, the same for the same x-queued-id.
  api/uploads/<id>/<name>  GET: an uploaded file back.
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
# Asked for a "table": formatting of every kind, streamed a few characters at a time, so
# each piece stops partway through something.
TABLE_REPLY = (
    "Here are the **three plans** side by side, from the [pricing page](https://example.com/pricing):\n\n"
    "| Plan | Price | Best for |\n| --- | --- | --- |\n"
    "| **Starter** | $0 | Trying it _out_ |\n"
    "| **Team** | $12 | Groups of up to `20` |\n"
    "| **Business** | $30 | ~~Everyone~~ Larger companies |\n\n"
    "Most teams your size pick **Team**. To switch:\n\n"
    "1. Open *Settings*, then **Billing**\n"
    "2. Pick the plan, or run `acme plan set team`\n\n"
    "```bash\nacme plan set team --seats 12\n```\n"
)
_rendered = {}
# Uploaded files, in memory for this run: id -> (name, type, bytes).
UPLOADS = {}
# Records sent by the app blocks: queued id -> record id.
TAKEN = {"admin", "root", "support", "help", "imran", "ada"}
RECORDS = {}
MAX_UPLOAD = 10 * 1024 * 1024


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


def make_people(n=60):
    rng = random.Random(7)
    first = ["Ada", "Grace", "Alan", "Katherine", "Linus", "Margaret", "Dennis", "Barbara", "Ken", "Frances", "Tim", "Radia", "Guido", "Hedy", "John", "Sophie"]
    last = ["Okafor", "Lindqvist", "Tanaka", "Moreau", "Silva", "Novak", "Haddad", "Iyer", "Kowalski", "Mensah", "Rossi", "Byrne", "Nakamura", "Duarte", "Fischer"]
    teams = ["Support", "Billing", "Onboarding", "Engineering", "Success"]
    out = {}
    for i in range(1, n + 1):
        name = f"{rng.choice(first)} {rng.choice(last)}"
        pid = f"u-{i:02d}"
        out[pid] = {
            "id": pid, "name": name, "email": name.lower().replace(" ", ".") + "@example.com",
            "title": rng.choice(["Agent", "Senior agent", "Lead", "Customer", "Admin", "Manager"]),
            "team": rng.choice(teams), "phone": f"+1 555 {rng.randint(100, 999)} {rng.randint(1000, 9999)}",
        }
    return out


PEOPLE = make_people()


def make_accounts(n=20):
    rng = random.Random(9)
    names = ["Northwind", "Contoso", "Fabrikam", "Globex", "Initech", "Umbrella", "Soylent", "Hooli", "Vandelay", "Wonka", "Stark", "Wayne", "Acme", "Tyrell", "Cyberdyne", "Aperture", "Massive", "Oscorp", "Gringotts", "Monarch"]
    return {f"acc-{i:02d}": {"id": f"acc-{i:02d}", "name": names[i - 1], "plan": rng.choice(["Free", "Team", "Business", "Enterprise"]), "region": rng.choice(REGIONS)} for i in range(1, n + 1)}


ACCOUNTS = make_accounts()


def make_tickets(n=400):
    rng = random.Random(11)
    subjects = ["Can't sign in", "Invoice shows the wrong amount", "Export times out", "Add a seat", "Webhook stopped firing", "Password reset email missing",
                "Change the billing contact", "Slow dashboard", "Import fails on row 12", "Two-factor codes rejected", "Cancel the trial", "API rate limit"]
    agents = [f"u-{i:02d}" for i in range(1, 11)]
    out = []
    for i in range(n):
        status = rng.choices(["Open", "Pending", "On hold", "Solved"], [5, 3, 1, 4])[0]
        out.append({
            "id": f"T-{4100 + i}", "subject": rng.choice(subjects), "status": status,
            "priority": rng.choices(["Low", "Normal", "High", "Urgent"], [3, 6, 3, 1])[0],
            "opened": f"2026-{rng.randint(9, 11):02d}-{rng.randint(1, 28):02d}",
            "requesterId": f"u-{rng.randint(11, 60):02d}", "assigneeId": None if status == "Open" and rng.random() < 0.2 else rng.choice(agents),
            "accountId": f"acc-{rng.randint(1, 20):02d}",
        })
    return out


TICKETS = make_tickets()


def listing(rows, query, searched_fields):
    """ui-data-table's query over `rows`: search, filters, facets, sort and a batch."""
    q = (query.get("q") or "").strip().lower()
    if q:
        rows = [r for r in rows if all(any(w in str(r.get(f) or "").lower() for f in searched_fields) for w in q.split())]
    searched = rows
    filters = json.loads(query.get("filters") or "{}")
    for field, cond in filters.items():
        rows = [r for r in rows if matches(r.get(field), cond)]
    extras = {}
    if query.get("facets"):
        extras["facets"] = {}
        for f in query["facets"].split(","):
            counts = {}
            for r in searched:
                if all(matches(r.get(k), c) for k, c in filters.items() if k != f):
                    counts[r.get(f)] = counts.get(r.get(f), 0) + 1
            extras["facets"][f] = [{"value": v, "count": n} for v, n in sorted(counts.items(), key=lambda x: str(x[0]))]
    for key in reversed([k for k in (query.get("sort") or "").split(",") if k]):
        field = key.lstrip("-")
        rows = sorted(rows, key=lambda r: (r.get(field) is None, str(r.get(field))), reverse=key.startswith("-"))
    offset = max(0, int(query.get("offset") or 0))
    limit = min(500, max(1, int(query.get("limit") or 50)))
    time.sleep(0.25)
    return {"rows": rows[offset:offset + limit], "total": len(rows), **extras}


CLINICIANS = {
    "c-1": {"id": "c-1", "name": "Dr. Amara Osei", "specialty": "Family medicine", "room": "1"},
    "c-2": {"id": "c-2", "name": "Dr. Lukas Weber", "specialty": "Paediatrics", "room": "2"},
    "c-3": {"id": "c-3", "name": "Dr. Priya Raman", "specialty": "Dermatology", "room": "3"},
    "c-4": {"id": "c-4", "name": "Dr. Tomás Ortega", "specialty": "Cardiology", "room": "4"},
    "c-5": {"id": "c-5", "name": "Nurse Hana Sato", "specialty": "Vaccinations", "room": "5"},
}


def make_patients(n=90):
    rng = random.Random(13)
    first = ["Mia", "Noah", "Zara", "Omar", "Lena", "Kofi", "Ines", "Ravi", "Elsa", "Yusuf", "Chloe", "Mateo", "Aiko", "Sven", "Leila", "Dev"]
    last = ["Brennan", "Achebe", "Castillo", "Dubois", "Eriksen", "Farouk", "Gallo", "Horvat", "Ito", "Jansen", "Kaur", "Larsen", "Mbeki"]
    out = {}
    for i in range(1, n + 1):
        pid = f"pt-{i:03d}"
        out[pid] = {
            "id": pid, "name": f"{rng.choice(first)} {rng.choice(last)}", "mrn": f"MRN-{rng.randint(100000, 999999)}",
            "born": f"{rng.randint(1945, 2022)}-{rng.randint(1, 12):02d}-{rng.randint(1, 28):02d}",
            "phone": f"+44 7{rng.randint(100, 999)} {rng.randint(100000, 999999)}",
            "allergies": rng.choice([[], [], ["Penicillin"], ["Latex"], ["Peanuts", "Shellfish"]]),
        }
    return out


PATIENTS = make_patients()


STARTED = time.time()
NEXT = {"Booked": "Checked in", "Checked in": "Seen"}


def appointments(query):
    """The day's appointments, with what's changed since the server started: every 20
    seconds a visit's status moves on, or now and then it's cancelled. The cursor is the
    number of changes; `since` answers the rows changed after it and the ids deleted."""
    full = day_appointments(query)
    rows = {r["id"]: dict(r) for r in full}
    order = [r["id"] for r in full]
    now = int((time.time() - STARTED) // 20)
    since = query.get("since")
    since = int(since) if since and since.isdigit() and int(since) <= now else None
    changed, deleted = set(), set()
    for k in range(1, now + 1):
        rng = random.Random(f"{query.get('day')}-{query.get('clinician')}-{k}")
        live = [i for i in order if i in rows]
        if not live:
            break
        pick = rng.choice(live)
        if rng.random() < 0.15:
            del rows[pick]
            if since is not None and k > since:
                deleted.add(pick)
                changed.discard(pick)
        elif rows[pick]["status"] in NEXT:
            rows[pick]["status"] = NEXT[rows[pick]["status"]]
            if since is not None and k > since:
                changed.add(pick)
    if since is not None:
        return {"rows": [rows[i] for i in order if i in changed], "deleted": sorted(deleted), "cursor": str(now)}
    live = [rows[i] for i in order if i in rows]
    return {"rows": live, "total": len(live), "cursor": str(now)}


def day_appointments(query):
    """The day's appointments as booked: the same ones every time for a day."""
    day = query.get("day") or "2026-11-27"
    rng = random.Random(day)
    reasons = ["Check-up", "Follow-up", "Rash", "Chest pain", "Flu shot", "Blood pressure", "Ear ache", "Results", "Mole check", "Travel vaccine"]
    rows = []
    for c in CLINICIANS:
        for slot in sorted(rng.sample(range(16), rng.randint(3, 7))):
            hour, minute = 8 + slot // 2, 30 * (slot % 2)
            rows.append({
                "id": f"{day}-{c}-{slot}", "time": f"{day}T{hour:02d}:{minute:02d}", "at": f"{hour:02d}:{minute:02d}", "clinicianId": c,
                "patientId": rng.choice(list(PATIENTS)), "reason": rng.choice(reasons),
                "status": rng.choices(["Booked", "Checked in", "Seen", "No-show"], [5, 2, 3, 1])[0],
            })
    rows.sort(key=lambda r: r["time"])
    if query.get("clinician"):
        rows = [r for r in rows if r["clinicianId"] == query["clinician"]]
    time.sleep(0.3)
    return rows


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


SUMMARIES = {
    "sum": lambda vs: round(sum(vs), 2),
    "avg": lambda vs: round(sum(vs) / len(vs), 2) if vs else None,
    "min": lambda vs: min(vs, default=None),
    "max": lambda vs: max(vs, default=None),
}


def shipments(query):
    rows = SHIPMENTS
    q = (query.get("q") or "").strip().lower()
    if q:
        rows = [r for r in rows if all(any(w in str(r[f]).lower() for f in ("id", "carrier", "origin", "destination")) for w in q.split())]
    searched = rows
    filters = json.loads(query.get("filters") or "{}")
    for field, cond in filters.items():
        rows = [r for r in rows if matches(r.get(field), cond)]
    extras = {}
    if query.get("facets"):
        extras["facets"] = {}
        for f in query["facets"].split(","):
            counts = {}
            for r in searched:
                if all(matches(r.get(k), c) for k, c in filters.items() if k != f):
                    counts[r.get(f)] = counts.get(r.get(f), 0) + 1
            extras["facets"][f] = [{"value": v, "count": n} for v, n in sorted(counts.items(), key=lambda x: str(x[0]))]
    if query.get("summary"):
        extras["summary"] = {}
        for ask in query["summary"].split(","):
            f, _, how = ask.partition(":")
            if how in SUMMARIES:
                extras["summary"][ask] = SUMMARIES[how]([r[f] for r in rows if isinstance(r.get(f), (int, float))])
    for key in reversed([k for k in (query.get("sort") or "").split(",") if k]):
        field = key.lstrip("-")
        rows = sorted(rows, key=lambda r: (r.get(field) is None, r.get(field)), reverse=key.startswith("-"))
    offset = max(0, int(query.get("offset") or 0))
    limit = min(500, max(1, int(query.get("limit") or 50)))
    time.sleep(min(2000, int(query.get("delay") or 300)) / 1000)
    group_by = query.get("groupBy")
    if group_by and "group" in query:
        mine = [r for r in rows if str(r.get(group_by)) == query["group"]]
        return {"rows": mine[offset:offset + limit], "total": len(mine)}
    if group_by:
        groups = {}
        for r in rows:
            g = groups.setdefault(r.get(group_by), {group_by: r.get(group_by), "count": 0, "value": 0})
            g["count"] += 1
            g["value"] = round(g["value"] + r["value"], 2)
        ordered = sorted(groups.values(), key=lambda g: str(g[group_by]))
        return {"rows": ordered[offset:offset + limit], "total": len(ordered), **extras}
    return {"rows": rows[offset:offset + limit], "total": len(rows), **extras}


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


def wave(t, period, lo, hi, phase=0.0):
    """Between lo and hi and back, once a period."""
    return lo + (hi - lo) * (0.5 + 0.5 * math.sin(2 * math.pi * (t / period + phase)))


def plant():
    """The packing hall in block twin-01, now. Every value follows the clock, so a page
    polling and one streaming agree."""
    t = time.time()
    out = {}
    cycle = t % 240
    line2_stopped = 150 <= cycle < 195
    for i in range(1, 9):
        line = 1 if i <= 4 else 2
        stopped = line == 2 and line2_stopped
        temp = wave(t, 90, 52, 62, i / 8)
        if i == 3:
            # Its bearing heats up for two minutes, then it's put right.
            heat = min(1, max(0, (cycle - 40) / 80)) if cycle < 160 else 0
            temp += 33 * heat
        if stopped:
            temp -= 8
        out[f"cv-{i}"] = {
            "status": "stopped" if stopped else "fault" if temp >= 85 else "running",
            "speed": 0 if stopped else round(wave(t, 30, 23, 25, i / 5), 1),
            "temp": round(temp, 1),
        }
    for i, phase in ((1, 0.0), (2, 0.4)):
        level = 100 - ((t / 1.5 + phase * 100) % 95)
        out[f"hp-{i}"] = {"status": "running", "level": round(level), "temp": round(wave(t, 60, 18, 21, phase), 1)}
    for i in range(1, 4):
        level = 100 - ((t / 6 + i * 23) % 92)
        out[f"tk-{i}"] = {"status": "running", "level": round(level), "temp": round(wave(t, 120, 6, 8, i / 3), 1)}
    for vid, period, phase in (("fl-1", 600, 0.1), ("amr-1", 420, 0.5), ("amr-2", 360, 0.8)):
        battery = wave(t, period, 12, 96, phase)
        charging = battery < 22
        out[vid] = {"status": "charging" if charging else "running", "battery": round(battery), "speed": 0 if charging else 1.2 if vid == "fl-1" else 0.8}
    for i in (1, 2):
        out[f"fan-{i}"] = {"status": "stopped" if line2_stopped and i == 2 else "running", "effect": "none" if line2_stopped and i == 2 else "spin", "rpm": 0 if line2_stopped and i == 2 else round(wave(t, 50, 58, 62, i / 2))}
    for i, base in ((1, 24), (2, 27), (3, 29)):
        out[f"env-{i}"] = {"air": round(base + wave(t, 300, -1.5, 1.5, i / 3), 1), "humidity": round(wave(t, 400, 42, 58, i / 3))}
    running = sum(1 for v in out.values() if v.get("status") == "running")
    out["plant"] = {
        "output": round(wave(t, 120, 1180, 1320) * (0.55 if line2_stopped else 1)),
        "oee": round(wave(t, 200, 78, 86) * (0.8 if line2_stopped else 1), 1),
        "running": running,
        "time": datetime.fromtimestamp(int(t), timezone.utc).isoformat().replace("+00:00", "Z"),
    }
    return out


def bottling():
    """The bottling line in block twin-04, now. A five-minute story: the pasteuriser's
    holding temperature dips under 72 °C and it diverts (the line stops for want of
    milk), then a cold store sensor warms up while its door is left open."""
    t = time.time()
    cycle = t % 300
    dip = 60 <= cycle < 100
    stopped = 70 <= cycle < 115
    warm = min(1, max(0, (cycle - 180) / 40)) if 180 <= cycle < 260 else 0
    hold = 75 + wave(t, 40, -0.6, 0.6) - (4.2 * math.sin(math.pi * (cycle - 60) / 40) if dip else 0)
    bpm = 0 if stopped else round(wave(t, 60, 118, 124))
    made = int(t / 60 * 120) % 40000
    out = {
        "pas-1": {"status": "diverting" if hold < 72 else "running", "temp": round(hold, 1), "flow": round(wave(t, 90, 7600, 8100))},
        "mix-1": {"status": "running", "brix": round(wave(t, 120, 11.6, 12.3), 1), "ph": round(wave(t, 150, 6.62, 6.74), 2), "level": round(100 - (t / 3 % 80))},
        "fil-1": {"status": "stopped" if stopped else "running", "bpm": bpm, "fill": round(500 + wave(t, 20, -1.2, 1.4), 1)},
        "cap-1": {"status": "stopped" if stopped else "running", "torque": round(wave(t, 25, 17, 20))},
        "chk-1": {"status": "stopped" if stopped else "running", "rejects": int(made / 2100) % 60, "weight": round(wave(t, 15, 528, 534), 1)},
        "cs-1": {"temp": round(wave(t, 200, 2.4, 3.4), 1), "humidity": round(wave(t, 300, 78, 86))},
        "cs-2": {"temp": round(wave(t, 220, 2.6, 3.6) + 5 * warm, 1), "humidity": round(wave(t, 260, 76, 84))},
        "env-1": {"air": round(wave(t, 400, 21, 24), 1), "humidity": round(wave(t, 350, 48, 60))},
    }
    for i, base in ((1, 3.1), (2, 3.6)):
        out[f"tk-{i}"] = {"status": "running", "level": round(100 - ((t / 5 + i * 31) % 85)), "temp": round(base + wave(t, 180, -0.3, 0.3, i / 2), 1)}
    out["tk-3"] = {"status": "running", "level": round(100 - (t / 7 % 85)), "temp": round(6.2 + wave(t, 200, -0.4, 0.4), 1), "brix": round(wave(t, 300, 10.8, 11.4), 1)}
    for i in range(1, 6):
        out[f"cv-{i}"] = {"status": "stopped" if stopped else "running"}
    for i in range(1, 11):
        out[f"b-{i}"] = {"speed": 0 if stopped else 0.5}
    ccp = sum(1 for v in (hold < 72, out["cs-2"]["temp"] >= 7, out["tk-1"]["temp"] >= 6, out["tk-2"]["temp"] >= 6) if v)
    out["line"] = {
        "product": "Whole milk 500 ml", "batch": f"B-{int(t // 3600) % 1000:03d}", "bpm": bpm, "made": made,
        "rejects": out["chk-1"]["rejects"], "ccp": ccp,
        "time": datetime.fromtimestamp(int(t), timezone.utc).isoformat().replace("+00:00", "Z"),
    }
    return out


COLD_ROOMS = {"freezer-a": (-21, 4), "freezer-b": (-20, 4), "chiller": (3, 4), "dock": (11, 2)}


def coldstore():
    """The cold store in block twin-02, now: sensors s-01 to s-14 and doors d-1 to d-3."""
    t = time.time()
    cycle = t % 300
    # Freezer B's door is left open for a minute and a half, and its air warms up.
    open_b = 60 <= cycle < 150
    warm = min(1, max(0, (cycle - 60) / 70)) if cycle < 210 else 0
    warm = warm if cycle < 150 else max(0, 1 - (cycle - 150) / 60)
    rooms = ["freezer-a"] * 4 + ["freezer-b"] * 4 + ["chiller"] * 4 + ["dock"] * 2
    out = {}
    for i, room in enumerate(rooms, 1):
        base, spread = COLD_ROOMS[room]
        temp = base + spread * (wave(t, 180, -0.5, 0.5, i / 7))
        if room == "freezer-b":
            temp += 13 * warm * (1 - 0.15 * (i - 5))
        out[f"s-{i:02d}"] = {"temp": round(temp, 1), "humidity": round(wave(t, 240, 70, 85, i / 5)), "battery": 100 - (i * 7) % 40}
    for i, room in ((1, "freezer-a"), (2, "freezer-b"), (3, "chiller")):
        out[f"d-{i}"] = {"door": "open" if (i == 2 and open_b) else "closed"}
    return out


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
        if "/api/uploads/" in path:
            return self.uploaded(path)
        if path.endswith("/api/twin/plant/stream"):
            return self.events(plant)
        if path.endswith("/api/twin/plant"):
            return self.send_json(plant())
        if path.endswith("/api/twin/bottling/stream"):
            return self.events(bottling)
        if path.endswith("/api/twin/bottling"):
            return self.send_json(bottling())
        if path.endswith("/api/twin/coldstore"):
            return self.send_json(coldstore())
        if path.endswith("/api/available"):
            from urllib.parse import parse_qsl
            query = dict(parse_qsl(self.path.split("?", 1)[1] if "?" in self.path else ""))
            name = query.get("value", "").strip().lower()
            if name in TAKEN:
                return self.send_json({"valid": False, "message": f"@{name} is taken"})
            return self.send_json({"valid": True})
        if path.endswith("/api/tickets"):
            return self.send_json(listing(TICKETS, self.query(), ("id", "subject", "status", "priority")))
        if path.endswith("/api/people"):
            ids = [i for i in self.query().get("ids", "").split(",") if i]
            time.sleep(0.6)
            return self.send_json([PEOPLE[i] for i in ids if i in PEOPLE])
        if "/api/people/" in path:
            person = PEOPLE.get(path.rsplit("/", 1)[1])
            return self.send_json(person) if person else self.send_error(404)
        if "/api/accounts/" in path:
            time.sleep(0.25)
            account = ACCOUNTS.get(path.rsplit("/", 1)[1])
            return self.send_tagged(account) if account else self.send_error(404)
        if path.endswith("/api/appointments"):
            return self.send_json(appointments(self.query()))
        if path.endswith("/api/clinicians"):
            time.sleep(0.4)
            return self.send_json([CLINICIANS[i] for i in self.query().get("ids", "").split(",") if i in CLINICIANS])
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
        if path.endswith("/api/upload"):
            return self.upload(path)
        if path.endswith("/api/records"):
            return self.record()
        if path.endswith("/api/patients/batch"):
            return self.patients()
        self.send_error(404)

    def query(self):
        from urllib.parse import parse_qsl
        return dict(parse_qsl(self.path.split("?", 1)[1] if "?" in self.path else ""))

    def patients(self):
        """Patients by id, from a POST of { ids }: as { data: { id: patient } }."""
        try:
            ids = json.loads(self.rfile.read(int(self.headers.get("Content-Length") or 0)) or b"{}").get("ids") or []
        except (ValueError, AttributeError):
            return self.send_error(400, "not JSON")
        time.sleep(0.7)
        self.send_json({"data": {i: PATIENTS[i] for i in ids if isinstance(i, str) and i in PATIENTS}})

    def record(self):
        """Takes anything as JSON and answers with an id; one sent again (the same
        x-queued-id) gets the same answer."""
        body = self.rfile.read(int(self.headers.get("Content-Length") or 0))
        try:
            json.loads(body or b"{}")
        except ValueError:
            return self.send_error(400, "not JSON")
        queued = self.headers.get("x-queued-id")
        if queued not in RECORDS:
            RECORDS[queued or f"r{len(RECORDS)}"] = f"R-{1001 + len(RECORDS)}"
        self.send_json({"id": RECORDS.get(queued) or list(RECORDS.values())[-1], "size": len(body)})

    def upload(self, path):
        from email.parser import BytesParser
        from email.policy import HTTP
        from urllib.parse import quote
        length = int(self.headers.get("Content-Length") or 0)
        if length > MAX_UPLOAD + 64 * 1024:
            self.rfile.read(length)
            body = json.dumps({"error": "Files can be up to 10 MB here"}).encode()
            self.send_response(413)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            return self.wfile.write(body)
        raw = self.rfile.read(length)
        message = BytesParser(policy=HTTP).parsebytes(b"Content-Type: " + self.headers["Content-Type"].encode() + b"\r\n\r\n" + raw)
        part = next((p for p in message.iter_parts() if p.get_param("name", header="content-disposition") == "file"), None)
        if part is None:
            return self.send_error(400, "no file field")
        name = os.path.basename(part.get_filename() or "file")
        file_id = f"u{len(UPLOADS) + 1}{random.randrange(16**4):04x}"
        UPLOADS[file_id] = (name, part.get_content_type(), part.get_payload(decode=True) or b"")
        self.send_json({"id": file_id, "url": path.replace("/api/upload", f"/api/uploads/{file_id}/{quote(name)}")})

    def uploaded(self, path):
        file_id = path.split("/api/uploads/", 1)[1].split("/", 1)[0]
        if file_id not in UPLOADS:
            return self.send_error(404)
        name, kind, data = UPLOADS[file_id]
        self.send_response(200)
        self.send_header("Content-Type", kind)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

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
        pieces, pause = None, 0.04
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
        elif "table" in said.lower():
            pieces, pause = [TABLE_REPLY[i : i + 4] for i in range(0, len(TABLE_REPLY), 4)], 0.03
        else:
            # Markdown, which the chat shows formatted as it streams in.
            reply = (
                f"You said **“{said}”**. This is a canned reply from `examples/serve.py`, streamed a word "
                f"at a time as server-sent events, like a model's would be.\n\n"
                f"Things to try:\n\n"
                f"- Ask for a **chart** to get a reply with a chart in it\n"
                f"- Ask for a **table** to see a table, links and code format as they arrive\n"
                f"- Ask for **photos** to get a carousel\n\n"
                f"The conversation so far has {len(messages)} messages."
            )
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        try:
            time.sleep(0.6)
            for piece in pieces or [word + " " for word in reply.split(" ")]:
                self.wfile.write(f"data: {json.dumps({'content': piece})}\n\n".encode())
                self.wfile.flush()
                time.sleep(pause)
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

    def send_tagged(self, value):
        """JSON with an ETag, or 304 Not Modified when the page already has it."""
        import hashlib
        body = json.dumps(value).encode()
        tag = '"' + hashlib.sha1(body).hexdigest()[:16] + '"'
        if self.headers.get("If-None-Match") == tag:
            self.send_response(304)
            self.send_header("ETag", tag)
            self.end_headers()
            return
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Cache-Control", "no-store")
        self.send_header("ETag", tag)
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

    def events(self, make):
        """What `make` gives, as a server-sent event once a second."""
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        try:
            while True:
                self.wfile.write(f"data: {json.dumps(make())}\n\n".encode())
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
