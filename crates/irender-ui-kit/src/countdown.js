// irender-ui-kit: the `ui-countdown` behaviour. Every second it works out the time left
// to `to` (which may follow state), shows it as units or a clock, as text or tiles, and
// sets the status: ok, warning (under `warn`), danger (under `danger`), over. The status
// goes to `bindStatus` and is said once when it changes; the seconds left go to
// `bindLeft`. At zero it fires `end`. Past it, it counts up as overdue, or shows `endText`.
// While `paused`, the time left stands still; resumed, the deadline moves out by the pause.
// With `workdays`, `hours` or `holidays` it counts working time only, still outside it.
(window.__irenderBehaviors ||= {})["ui-countdown"] = (el, api) => {
  const cfg = api.config, K = window.__irenderKit, L = (t, v) => window.__irender.label(t, v);
  const text = el.querySelector("[data-cd-text]"), bar = el.querySelector("[data-cd-bar]");
  const live = el.querySelector("[data-cd-live]"), tiles = el.querySelector("[data-cd-tiles]");
  // A time from a date and time, a date, milliseconds, or `+7h 40m` / `-30m` from when the
  // page opened; null if it isn't one.
  const opened = Date.now();
  const UNIT = { d: 86400, h: 3600, m: 60, s: 1 };
  const ms = (v) => {
    if (v === null || v === undefined || v === "") return null;
    if (typeof v === "number") return v;
    const rel = /^\s*([+-])\s*((?:\d+(?:\.\d+)?\s*[dhms]\s*)+)$/i.exec(v);
    if (rel) {
      let s = 0;
      for (const [, n, u] of rel[2].matchAll(/(\d+(?:\.\d+)?)\s*([dhms])/gi)) s += Number(n) * UNIT[u.toLowerCase()];
      return opened + (rel[1] === "-" ? -s : s) * 1000;
    }
    const t = Date.parse(v);
    return isNaN(t) ? null : t;
  };
  const p2 = (n) => String(n).padStart(2, "0");
  // Labels: "{n}d", "{n}h", "{n}m", "{n}s"
  const part = (n, u) => L(`{n}${u}`, { n });
  function units(s) {
    const d = Math.floor(s / 86400), h = Math.floor(s / 3600) % 24, m = Math.floor(s / 60) % 60, sec = s % 60;
    // The two largest units that matter: 2d 4h, 4h 13m, 13m 05s, 42s.
    if (d) return `${part(d, "d")} ${part(h, "h")}`;
    if (h) return `${part(h, "h")} ${part(m, "m")}`;
    if (m) return `${part(m, "m")} ${part(p2(sec), "s")}`;
    return part(sec, "s");
  }
  function clock(s) {
    const d = Math.floor(s / 86400), h = Math.floor(s / 3600) % 24, m = Math.floor(s / 60) % 60;
    const hms = `${d ? p2(h) : h}:${p2(m)}:${p2(s % 60)}`;
    return d ? `${part(d, "d")} ${hms}` : hms;
  }
  // Said in full for screen readers: "2 days 4 hours", "13 minutes".
  function spoken(s) {
    const d = Math.floor(s / 86400), h = Math.floor(s / 3600) % 24, m = Math.floor(s / 60) % 60;
    const out = [];
    if (d) out.push(L(["{n} day", "{n} days"], { n: d }));
    if (h) out.push(L(["{n} hour", "{n} hours"], { n: h }));
    if (!d && m) out.push(L(["{n} minute", "{n} minutes"], { n: m }));
    if (!out.length) out.push(L(["{n} second", "{n} seconds"], { n: s % 60 }));
    return out.join(" ");
  }

  // Working time: the weekdays, the minutes of the day and the dates off; null when every
  // second counts.
  const ALL = [0, 1, 2, 3, 4, 5, 6];
  const cal = cfg.workdays || cfg.hours || cfg.holidays?.length
    ? { days: new Set(cfg.workdays === true ? [1, 2, 3, 4, 5] : Array.isArray(cfg.workdays) ? cfg.workdays : ALL), hours: cfg.hours ?? [0, 1440], off: new Set((cfg.holidays ?? []).map(String)) }
    : null;
  const ymd = (d) => `${d.getFullYear()}-${p2(d.getMonth() + 1)}-${p2(d.getDate())}`;
  // Working milliseconds from `a` to `b`, below zero when `b` is earlier; local days.
  function worked(a, b) {
    if (b < a) return -worked(b, a);
    let sum = 0;
    const d = new Date(a);
    d.setHours(0, 0, 0, 0);
    for (let n = 0; d.getTime() < b && n < 3700; n++) {
      if (cal.days.has(d.getDay()) && !cal.off.has(ymd(d))) {
        const start = new Date(d).setHours(0, cal.hours[0]), end = new Date(d).setHours(0, cal.hours[1]);
        sum += Math.max(0, Math.min(end, b) - Math.max(start, a));
      }
      d.setDate(d.getDate() + 1);
    }
    return sum;
  }
  const span = (a, b) => (cal ? worked(a, b) : b - a);

  let status = null, lastLeft;
  // Pauses on this page: since when (while paused), and how long in all, which a new `to`
  // (the page's own, with the pause in it) clears.
  let since = null, shift = 0, wasTo;
  function draw() {
    const raw = K.value(api, cfg.to), paused = !!K.value(api, cfg.paused), real = Date.now();
    if (wasTo !== undefined && JSON.stringify(raw) !== wasTo) shift = 0;
    wasTo = JSON.stringify(raw);
    if (paused && since === null) since = real;
    if (!paused && since !== null) (shift += real - since), (since = null);
    // The time as it's counted: stood still while paused.
    const now = (since ?? real) - shift;
    el.toggleAttribute("data-paused", paused);
    const to = ms(raw), from = ms(K.value(api, cfg.from));
    if (to === null) {
      text.textContent = "";
      return;
    }
    const left = Math.round(span(now, to) / 1000), s = Math.abs(left);
    // Out of hours: standing still until work starts again.
    const shut = !!cal && !paused && worked(now, now + 1) === 0;
    el.toggleAttribute("data-closed", shut);
    // The deadline as it now falls, moved out by the pauses.
    const due = to + (real - now);
    // Labels: "Out of hours"
    const why = paused ? L("Paused") : shut ? L("Out of hours") : null;
    const next = left <= 0 ? "over" : cfg.danger !== undefined && left <= cfg.danger ? "danger" : cfg.warn !== undefined && left <= cfg.warn ? "warning" : "ok";
    el.setAttribute("data-status", next);
    el.setAttribute("data-tone", { ok: "neutral", warning: "warning", danger: "danger", over: "danger" }[next]);
    const shown = cfg.format === "clock" ? clock(s) : units(s);
    if (left > 0) text.textContent = tiles ? L("Until {when}", { when: new Date(due).toLocaleString(document.documentElement.lang || undefined, { dateStyle: "medium", timeStyle: "short" }) }) : shown;
    else if (cfg.overdue) text.textContent = L("{time} overdue", { time: shown });
    else text.textContent = cfg.endText;
    // Labels: "Paused"
    if (why) text.textContent = `${text.textContent} · ${why}`;
    if (tiles) {
      const v = Math.max(0, left), n = { days: Math.floor(v / 86400), hours: Math.floor(v / 3600) % 24, minutes: Math.floor(v / 60) % 60, seconds: v % 60 };
      for (const t of tiles.querySelectorAll("[data-cd-tile]")) t.querySelector("b").textContent = p2(n[t.getAttribute("data-cd-tile")]);
      if (left <= 0) text.textContent = cfg.overdue ? L("{time} overdue", { time: shown }) : cfg.endText;
      if (why) text.textContent = left > 0 ? why : `${text.textContent} · ${why}`;
    }
    text.setAttribute("datetime", new Date(due).toISOString());
    const said = left > 0 ? L("{time} left", { time: spoken(s) }) : cfg.overdue ? L("{time} overdue", { time: spoken(s) }) : cfg.endText;
    // Labels: "out of hours"
    el.setAttribute("aria-description", paused ? `${said}, ${L("paused")}` : shut ? `${said}, ${L("out of hours")}` : said);
    if (bar && from !== null && to > from) el.style.setProperty("--used", String(Math.min(100, Math.max(0, (span(from, now) / (span(from, to) || 1)) * 100))));
    if (bar) requestAnimationFrame(() => el.setAttribute("data-cd-ready", ""));
    // Noted before it's written: writing it calls `update`, which draws again.
    if (left !== lastLeft) {
      lastLeft = left;
      if (cfg.bindLeft) api.set(cfg.bindLeft, left);
    }
    // `end` when it reaches zero on the page, not for a deadline already past on arrival.
    if (next === "over" && status !== null && status !== "over") api.emit("end", { to: new Date(due).toISOString() });
    if (next !== status) {
      // Said when it changes, not on every tick; nothing on the first draw unless it's late.
      const name = el.getAttribute("aria-label");
      if (status !== null || next !== "ok") {
        const what = next === "over" ? (cfg.overdue ? L("Overdue") : cfg.endText) : next === "ok" ? null : L("{time} left", { time: spoken(s) });
        if (what && live) live.textContent = name ? `${name}: ${what}` : what;
      }
      status = next;
      if (cfg.bindStatus) api.set(cfg.bindStatus, next);
    }
  }
  // Ticks on the second, while it's on the page.
  let timer = 0, drawing = false;
  const tick = () => {
    if (!el.isConnected || drawing) return;
    drawing = true;
    try {
      draw();
    } finally {
      drawing = false;
    }
    timer = setTimeout(tick, 1000 - (Date.now() % 1000) + 5);
  };
  tick();
  return {
    update() {
      // A change of state it wrote itself, while drawing, is already drawn.
      if (drawing) return;
      clearTimeout(timer);
      tick();
    },
  };
};
