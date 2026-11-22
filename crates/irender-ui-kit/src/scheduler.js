// irender-ui-kit: the `ui-scheduler` behaviour. Times are local, `YYYY-MM-DDTHH:MM`, and
// worked with as minutes. A day shows its hours; a week and a month, its days. Overlapping
// bookings of a row go under one another.
(window.__irenderBehaviors ||= {})["ui-scheduler"] = (el, api) => {
  const cfg = api.config, K = window.__irenderKit, L = (t, v) => window.__irender.label(t, v);
  const { make, button, icon, blank, isObj } = K;
  const loc = cfg.locale || undefined, MIN = 60000, DAY = 1440;
  const pad = (n) => String(n).padStart(2, "0");
  // Minutes since the epoch as if the local time were UTC, so days are always 1440 of them.
  const mins = (t) => {
    const m = /^(\d{4})-(\d{2})-(\d{2})(?:[T ](\d{2}):(\d{2}))?/.exec(String(t ?? ""));
    return m ? Date.UTC(+m[1], m[2] - 1, +m[3], +(m[4] ?? 0), +(m[5] ?? 0)) / MIN : NaN;
  };
  const stamp = (m) => {
    const d = new Date(m * MIN);
    return `${d.getUTCFullYear()}-${pad(d.getUTCMonth() + 1)}-${pad(d.getUTCDate())}T${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())}`;
  };
  const clock = (t, d) => {
    const m = /^(\d{1,2})(?::(\d{2}))?$/.exec(String(t ?? ""));
    return m ? Math.min(DAY, +m[1] * 60 + +(m[2] ?? 0)) : d;
  };
  const fmt = (m, o) => new Intl.DateTimeFormat(loc, { ...o, timeZone: "UTC" }).format(new Date(m * MIN));
  const nowMins = () => {
    const given = K.value(api, cfg.now);
    if (given === false) return NaN;
    if (!blank(given) && given !== true) return mins(given);
    const d = new Date();
    return Date.UTC(d.getFullYear(), d.getMonth(), d.getDate(), d.getHours(), d.getMinutes()) / MIN;
  };
  const views = Array.isArray(cfg.views) && cfg.views.length ? cfg.views : ["day", "week", "month"];
  let localView = cfg.view ?? views[0], localDate = null, selected = null;
  const view = () => {
    const v = cfg.bindView ? api.state[cfg.bindView] : localView;
    return views.includes(v) ? v : localView;
  };
  const date = () => {
    const d = mins(cfg.bindDate ? api.state[cfg.bindDate] : localDate);
    const t = isNaN(d) ? nowMins() : d;
    const day = Math.floor((isNaN(t) ? Date.now() / MIN : t) / DAY) * DAY;
    if (view() === "day") return day;
    const x = new Date(day * MIN);
    if (view() === "month") return Date.UTC(x.getUTCFullYear(), x.getUTCMonth(), 1) / MIN;
    // The week's first day, as the locale has it.
    let first = 1;
    try {
      const l = new Intl.Locale(loc ?? navigator.language);
      first = (l.getWeekInfo?.() ?? l.weekInfo)?.firstDay ?? 1;
    } catch { /* Monday */ }
    return day - ((x.getUTCDay() - first + 7) % 7) * DAY;
  };
  function setDate(d) {
    const s = stamp(d).slice(0, 10);
    localDate = s;
    if (cfg.bindDate) api.set(cfg.bindDate, s);
    else draw(true);
    api.emit("date", { date: s });
  }
  // What's shown: the first minute, how many minutes a row is across, and its columns.
  function span() {
    const d = date(), v = view();
    if (v === "day") {
      const from = clock(K.value(api, cfg.from), 0), to = Math.max(from + 60, clock(K.value(api, cfg.to), DAY));
      const cols = [];
      for (let m = Math.floor(from / 60) * 60; m < to; m += 60) cols.push(m);
      return { start: d + cols[0], len: cols.length * 60, cols: cols.map((m) => ({ at: d + m, text: fmt(d + m, { hour: "numeric" }) })), snap: cfg.snap ?? 15, width: 64 };
    }
    const n = v === "week" ? 7 : new Date(Date.UTC(new Date(d * MIN).getUTCFullYear(), new Date(d * MIN).getUTCMonth() + 1, 0)).getUTCDate();
    const today = Math.floor(nowMins() / DAY);
    return { start: d, len: n * DAY, cols: Array.from({ length: n }, (_, i) => ({ at: d + i * DAY, text: fmt(d + i * DAY, v === "week" ? { weekday: "short", day: "numeric", month: "short" } : { weekday: "narrow", day: "numeric" }), today: Math.floor((d + i * DAY) / DAY) === today })), snap: DAY, width: v === "week" ? 110 : 34 };
  }
  const resources = () => (K.value(api, cfg.resources) ?? []).filter(isObj);
  const each = (e) => {
    if (!e.repeat || !g0()) return [e];
    const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(String(e.start ?? ""));
    if (!m) return [e];
    const first = new Date(+m[1], m[2] - 1, +m[3]), base = Math.floor(mins(e.start) / DAY), [from, to] = g0();
    return K.repeats(e.repeat, first, Math.floor(from / DAY) - base - 1, Math.ceil(to / DAY) - base + 1).map((n) => ({ ...e, id: `${e.id}@${n}`, start: stamp(mins(e.start) + n * DAY), end: stamp(mins(e.end) + n * DAY), locked: true, series: e.id }));
  };
  // What's shown, for those that repeat.
  const g0 = () => (span0 ? [span0.start, span0.start + span0.len] : null);
  let span0 = null;
  const events = () => (K.value(api, cfg.events) ?? []).filter(isObj).flatMap(each).map((e) => {
    const s = mins(e.start), t = mins(e.end);
    return { e, id: String(e.id), r: String(e.resource), s, t: isNaN(t) || t <= s ? s + 60 : t };
  }).filter((x) => !isNaN(x.s));
  const may = (x) => !!cfg.bindEvents && !x.e.locked;

  // Bookings of a resource over its capacity at some moment.
  function clashes(all, res) {
    const bad = new Set();
    for (const r of res) {
      const cap = Math.max(1, Number(r.capacity) || 1), own = all.filter((x) => x.r === String(r.id));
      for (const x of own) {
        // The most at once while it lasts: checked where each of the others starts.
        for (const p of [x.s, ...own.filter((y) => y.s > x.s && y.s < x.t).map((y) => y.s)]) {
          const at = own.filter((y) => y.s <= p && y.t > p);
          if (at.length > cap) at.forEach((y) => bad.add(y.id));
        }
      }
    }
    return bad;
  }
  const collides = (id, r, s, t) => {
    const cap = Math.max(1, Number(resources().find((x) => String(x.id) === r)?.capacity) || 1);
    return events().filter((x) => x.id !== id && x.r === r && x.s < t && x.t > s).length >= cap;
  };
  function write(id, change, event) {
    const x = events().find((y) => y.id === id);
    if (!x) return false;
    const r = change.r ?? x.r, s = change.s ?? x.s, t = change.t ?? x.t;
    if (t <= s || (r === x.r && s === x.s && t === x.t)) return false;
    if (cfg.overlap === "prevent" && collides(id, r, s, t)) {
      K.say(el, L("That time is taken"));
      return false;
    }
    const res = resources().find((y) => String(y.id) === r);
    const next = { ...x.e, resource: res ? res.id : x.e.resource, start: stamp(s), end: stamp(t) };
    api.set(cfg.bindEvents, (api.state[cfg.bindEvents] ?? []).map((e) => (String(e.id) === id ? next : e)));
    api.emit(event, { event: next, previous: x.e });
    K.say(el, `${x.e.title ?? ""}: ${when(s, t)}${r !== x.r ? `, ${res?.title ?? r}` : ""}`);
    return true;
  }
  const when = (s, t) => (Math.floor(s / DAY) === Math.floor((t - 1) / DAY) ? `${fmt(s, { hour: "numeric", minute: "2-digit" })} – ${fmt(t, { hour: "numeric", minute: "2-digit" })}` : `${fmt(s, { dateStyle: "medium" })} – ${fmt(t, { dateStyle: "medium" })}`);

  // ---- drawing
  let was = null, g = null;
  function draw(force) {
    const now = nowMins(), sig = JSON.stringify([view(), date(), K.value(api, cfg.resources), K.value(api, cfg.events), K.value(api, cfg.from), K.value(api, cfg.to), Math.floor(now / 5), cfg.bindEvent ? api.state[cfg.bindEvent]?.id ?? null : selected]);
    if (!force && sig === was) return;
    if (drag) return; // redrawn when it's let go
    was = sig;
    g = span0 = span();
    const v = view(), all = events(), res = resources(), bad = cfg.overlap === "allow" ? new Set() : clashes(all, res);
    const cur = cfg.bindEvent ? String(api.state[cfg.bindEvent]?.id ?? "") : selected;
    const pct = (m) => `${((m - g.start) / g.len) * 100}%`;
    const end = g.start + g.len;
    const title = v === "day" ? fmt(g.start, { dateStyle: "full" }) : v === "month" ? fmt(g.start, { month: "long", year: "numeric" }) : `${fmt(g.start, { day: "numeric", month: "short" })} – ${fmt(end - DAY, { day: "numeric", month: "short", year: "numeric" })}`;
    const move = (n) => {
      const d = date();
      if (v === "month") { const x = new Date(d * MIN); return setDate(Date.UTC(x.getUTCFullYear(), x.getUTCMonth() + n, 1) / MIN); }
      setDate(d + n * (v === "week" ? 7 : 1) * DAY);
    };
    const names = { day: L("Day"), week: L("Week"), month: L("Month") };
    const bar = make("div", { "data-sc-bar": "" },
      button({ "aria-label": L("Previous"), "data-key": "prev", click: () => move(-1) }, icon("left", 15)),
      button({ "data-key": "today", click: () => setDate(Math.floor((isNaN(nowMins()) ? Date.now() / MIN : nowMins()) / DAY) * DAY) }, L("Today")),
      button({ "aria-label": L("Next"), "data-key": "next", click: () => move(1) }, icon("right", 15)),
      make("span", { "data-sc-title": "", "aria-live": "polite" }, title),
      views.length > 1 ? views.map((x) => button({ "data-key": `v-${x}`, "aria-pressed": String(x === v), click: () => {
        localView = x;
        cfg.bindView ? api.set(cfg.bindView, x) : draw(true);
      } }, names[x] ?? x)) : null);
    const grid = make("div", { "data-sc-grid": "", style: `--sc-w:${g.cols.length * g.width}px;--sc-step:${100 / g.cols.length}%` });
    grid.append(make("div", { "data-sc-corner": "" }, cfg.rowLabel ?? ""), make("div", { "data-sc-head": "", "aria-hidden": "true" }, g.cols.map((c) => make("span", { "data-today": c.today }, c.text))));
    let group = null;
    for (const r of res) {
      if (r.group && r.group !== group) grid.append(make("div", { "data-sc-group": "" }, make("span", null, (group = r.group))));
      const side = make("div", { "data-sc-side": "" }, r.avatar !== undefined || r.initials ? K.avatar({ ...r, name: r.title }) : null, make("span", { title: r.title }, r.title ?? r.id, r.subtitle ? make("small", null, r.subtitle) : null));
      const row = make("div", { "data-sc-row": String(r.id), role: "list", "aria-label": String(r.title ?? r.id) });
      // Outside its hours.
      if (v === "day" && Array.isArray(r.hours)) {
        let free = g.start;
        const open = r.hours.map((h) => [date() + clock(h.from, 0), date() + clock(h.to, DAY)]).sort((a, b) => a[0] - b[0]);
        for (const [a, b] of [...open, [end, end]]) {
          if (a > free) row.append(make("div", { "data-sc-off": "", style: `inset-inline-start:${pct(free)};width:${((Math.min(a, end) - free) / g.len) * 100}%` }));
          free = Math.max(free, Math.min(b, end));
        }
      }
      if (now >= g.start && now < end) row.append(make("div", { "data-sc-now": "", style: `inset-inline-start:${pct(now)}` }));
      // Those at the same time under one another.
      const own = all.filter((x) => x.r === String(r.id) && x.s < end && x.t > g.start).sort((a, b) => a.s - b.s || b.t - a.t), lanes = [];
      for (const x of own) {
        let lane = lanes.findIndex((t) => t <= (v === "day" ? x.s : Math.floor(x.s / DAY) * DAY));
        if (lane < 0) lane = lanes.length;
        lanes[lane] = v === "day" ? x.t : Math.ceil(x.t / DAY) * DAY;
        x.lane = lane;
      }
      const h = v === "day" ? 42 : 24, gapY = 3;
      row.style.height = `${Math.max(1, lanes.length) * (h + gapY) + 8}px`;
      for (const x of own) {
        const s = Math.max(g.start, v === "day" ? x.s : Math.floor(x.s / DAY) * DAY), t = Math.min(end, v === "day" ? x.t : Math.ceil(x.t / DAY) * DAY), m = may(x), clash = cfg.overlap !== "allow" && bad.has(x.id);
        row.append(make("div", {
          "data-sc-event": x.id, role: "listitem", tabindex: 0, "data-movable": m, "data-clash": clash, "data-status": x.e.status, "aria-current": cur === x.id ? "true" : null,
          "aria-label": `${x.e.title ?? ""}, ${when(x.s, x.t)}${clash ? `, ${L("clashes with another")}` : ""}`, title: `${x.e.title ?? ""}\n${when(x.s, x.t)}${x.e.note ? `\n${x.e.note}` : ""}`,
          style: `inset-inline-start:${pct(s)};width:${((t - s) / g.len) * 100}%;top:${4 + x.lane * (h + gapY)}px;height:${h}px;${typeof x.e.color === "string" && /^[#\w(),.%\s-]+$/.test(x.e.color) ? `--c:${clash ? "var(--color-destructive, #dc2626)" : x.e.color}` : ""}`,
        }, make("b", null, x.e.title ?? ""), v === "day" ? make("small", null, when(x.s, x.t)) : null, m && x.s >= g.start ? make("span", { "data-sc-edge": "start" }) : null, m && x.t <= end ? make("span", { "data-sc-edge": "end" }) : null));
      }
      grid.append(side, row);
    }
    const f = el.contains(document.activeElement) ? document.activeElement : null, key = f?.getAttribute("data-key"), ev = f?.getAttribute("data-sc-event");
    const old = el.querySelector("[data-sc-scroll]"), pos = old && [old.scrollLeft, old.scrollTop], first = !old || old.__view !== v;
    const scroll = make("div", { "data-sc-scroll": "" }, grid);
    scroll.__view = v;
    el.replaceChildren(...[bar, scroll, el.querySelector(":scope > [data-kit-live]")].filter(Boolean));
    if (pos && !first) [scroll.scrollLeft, scroll.scrollTop] = pos;
    else if (now >= g.start && now < end) scroll.scrollLeft = (K.rtl(el) ? -1 : 1) * Math.max(0, ((now - g.start) / g.len) * (grid.scrollWidth - 180) - 120);
    if (key) el.querySelector(`[data-key="${CSS.escape(key)}"]`)?.focus();
    else if (ev) el.querySelector(`[data-sc-event="${CSS.escape(ev)}"]`)?.focus({ preventScroll: true });
    if (cfg.bindClashes) {
      const list = [...bad].sort();
      if (JSON.stringify(api.state[cfg.bindClashes] ?? []) !== JSON.stringify(list)) api.set(cfg.bindClashes, list);
    }
  }
  function open(id) {
    const x = events().find((y) => y.id === id);
    if (!x) return;
    selected = id;
    if (cfg.bindEvent) api.set(cfg.bindEvent, x.e);
    else draw(true);
    api.emit("open", { event: x.e });
  }

  // ---- dragging: move, stretch, or make one on a free stretch
  let drag = null;
  const rowAt = (x, y) => document.elementsFromPoint(x, y).map((n) => n.closest?.("[data-sc-row]")).find((n) => n && el.contains(n));
  const minsAt = (row, x) => {
    const b = row.getBoundingClientRect(), p = (K.rtl(el) ? b.right - x : x - b.left) / b.width;
    return g.start + K.clamp(p, 0, 1) * g.len;
  };
  const snapTo = (m, how = Math.round) => g.start + how((m - g.start) / g.snap) * g.snap;
  el.addEventListener("pointerdown", (e) => {
    if (e.button !== 0) return;
    const ev = e.target.closest("[data-sc-event]"), row = e.target.closest("[data-sc-row]");
    if (ev) {
      const x = events().find((y) => y.id === ev.getAttribute("data-sc-event"));
      if (!x || !may(x)) return;
      const edge = e.target.closest("[data-sc-edge]")?.getAttribute("data-sc-edge");
      drag = { kind: edge ?? "move", x, ev, row, from: minsAt(row, e.clientX), px: e.clientX, py: e.clientY, id: e.pointerId, on: false, r: x.r, s: x.s, t: x.t };
    } else if (row && cfg.creatable && e.pointerType === "mouse") {
      const at = snapTo(minsAt(row, e.clientX), Math.floor);
      drag = { kind: "new", row, from: at, px: e.clientX, py: e.clientY, id: e.pointerId, on: false, r: row.getAttribute("data-sc-row"), s: at, t: at + g.snap };
    }
  });
  addEventListener("pointermove", (e) => {
    if (!drag || e.pointerId !== drag.id || !el.isConnected) return;
    if (!drag.on) {
      if (Math.hypot(e.clientX - drag.px, e.clientY - drag.py) < 4) return;
      drag.on = true;
      drag.ev?.setAttribute("data-dragging", "");
      if (drag.kind === "new") drag.row.append((drag.ghost = make("div", { "data-sc-new": "" })));
    }
    e.preventDefault();
    const at = minsAt(drag.row, e.clientX), d = drag;
    if (d.kind === "new") {
      const a = snapTo(at, at < d.from ? Math.floor : Math.ceil);
      d.s = Math.min(d.from, a);
      d.t = Math.max(d.from + g.snap, a);
    } else if (d.kind === "move") {
      const by = Math.round((at - d.from) / g.snap) * g.snap;
      d.s = d.x.s + by;
      d.t = d.x.t + by;
      const row = cfg.moveResources ? rowAt(e.clientX, e.clientY) : null;
      if (row) {
        el.querySelectorAll("[data-sc-row][data-over]").forEach((n) => n !== row && n.removeAttribute("data-over"));
        row.setAttribute("data-over", "");
        d.r = row.getAttribute("data-sc-row");
        if (d.ev.parentElement !== row) row.append(d.ev);
      }
    } else if (d.kind === "start") d.s = Math.min(view() === "day" ? snapTo(at) : d.x.s + Math.round((at - d.from) / DAY) * DAY, d.x.t - Math.min(g.snap, 15));
    else d.t = Math.max(view() === "day" ? snapTo(at) : d.x.t + Math.round((at - d.from) / DAY) * DAY, d.x.s + Math.min(g.snap, 15));
    const n = d.ghost ?? d.ev, s = Math.max(d.s, g.start), t = Math.min(d.t, g.start + g.len);
    n.style.insetInlineStart = `${((s - g.start) / g.len) * 100}%`;
    n.style.width = `${(Math.max(0, t - s) / g.len) * 100}%`;
    if (d.ghost) n.textContent = when(d.s, d.t);
    else if (n.querySelector("small")) n.querySelector("small").textContent = when(d.s, d.t);
    // Near an edge, the rest scrolls into view.
    const sc = el.querySelector("[data-sc-scroll]"), b = sc.getBoundingClientRect();
    if (e.clientX > b.right - 24) sc.scrollLeft += 12;
    else if (e.clientX < b.left + 200 && e.clientX > b.left + 160) sc.scrollLeft -= 12;
  });
  const finish = (e, keep) => {
    if (!drag || e.pointerId !== drag.id) return;
    const d = drag;
    drag = null;
    d.ghost?.remove();
    el.querySelectorAll("[data-over]").forEach((n) => n.removeAttribute("data-over"));
    if (!d.on) return;
    el.__dragged = true;
    setTimeout(() => (el.__dragged = false));
    if (keep && d.kind === "new") create(d.r, d.s, d.t);
    else if (keep && write(d.x.id, { r: d.r, s: d.s, t: d.t }, d.kind === "move" ? "move" : "resize")) return;
    draw(true);
  };
  addEventListener("pointerup", (e) => finish(e, true));
  addEventListener("pointercancel", (e) => finish(e, false));
  el.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && drag) {
      e.preventDefault();
      finish({ pointerId: drag.id }, false);
    }
  });
  function create(r, s, t) {
    if (cfg.overlap === "prevent" && collides(null, r, s, t)) return K.say(el, L("That time is taken"));
    const res = resources().find((x) => String(x.id) === r);
    const e = { title: L("New booking"), ...(cfg.newEvent ?? {}), id: K.id(), resource: res?.id ?? r, start: stamp(s), end: stamp(t) };
    if (cfg.bindEvents) api.set(cfg.bindEvents, [...(api.state[cfg.bindEvents] ?? []), e]);
    selected = String(e.id);
    if (cfg.bindEvent) api.set(cfg.bindEvent, e);
    api.emit("create", { event: e });
    K.say(el, L("Booking added"));
  }
  el.addEventListener("click", (e) => {
    const ev = e.target.closest("[data-sc-event]");
    if (ev && !el.__dragged) open(ev.getAttribute("data-sc-event"));
  });
  // Enter opens; the arrows go between bookings; with Alt they move one (Shift: its end).
  el.addEventListener("keydown", (e) => {
    const ev = e.target.closest?.("[data-sc-event]");
    if (!ev || e.target !== ev) return;
    const id = ev.getAttribute("data-sc-event"), x = events().find((y) => y.id === id);
    const k = K.rtl(el) ? { ArrowLeft: "ArrowRight", ArrowRight: "ArrowLeft" }[e.key] ?? e.key : e.key;
    if (e.key === "Enter" || e.key === " ") return e.preventDefault(), open(id);
    if (!k.startsWith("Arrow") || !x) return;
    e.preventDefault();
    if (e.altKey && may(x)) {
      const d = k === "ArrowRight" ? g.snap : k === "ArrowLeft" ? -g.snap : 0;
      if (d && e.shiftKey) write(id, { t: x.t + d }, "resize");
      else if (d) write(id, { s: x.s + d, t: x.t + d }, "move");
      else if (cfg.moveResources) {
        const ids = resources().map((r) => String(r.id)), to = ids[ids.indexOf(x.r) + (k === "ArrowDown" ? 1 : -1)];
        if (to !== undefined) write(id, { r: to }, "move");
      }
      return;
    }
    if (k === "ArrowRight" || k === "ArrowLeft") {
      const own = [...ev.parentElement.querySelectorAll("[data-sc-event]")];
      own[own.indexOf(ev) + (k === "ArrowRight" ? 1 : -1)]?.focus();
    } else {
      const rows = [...el.querySelectorAll("[data-sc-row]")];
      for (let i = rows.indexOf(ev.parentElement) + (k === "ArrowDown" ? 1 : -1); rows[i]; i += k === "ArrowDown" ? 1 : -1) {
        const own = [...rows[i].querySelectorAll("[data-sc-event]")];
        if (!own.length) continue;
        own.reduce((a, b) => (Math.abs(b.offsetLeft - ev.offsetLeft) < Math.abs(a.offsetLeft - ev.offsetLeft) ? b : a)).focus();
        break;
      }
    }
  });
  draw();
  // The mark for now keeps up.
  const tick = setInterval(() => (el.isConnected ? draw() : clearInterval(tick)), 60000);
  return { update: () => draw() };
};
