// irender-ui-kit: the `ui-calendar` and `ui-date-picker` behaviours. The server renders
// the frame; this draws the calendar (days, months, quarters or years), keeps the value
// in state, and handles the mouse and keyboard as the WAI-ARIA date picker pattern
// describes. Everything it writes is text, never HTML, apart from its own icons.
//
// Dates are UTC midnights, so daylight saving never moves a day, and they're formatted
// in UTC for the same reason.
(() => {
  const B = (window.__irenderBehaviors ||= {});
  if (B["ui-calendar"]) return;

  const DAY = 864e5;
  const D = (y, m, d) => new Date(Date.UTC(y, m - 1, d));
  const Y = (d) => d.getUTCFullYear(), M = (d) => d.getUTCMonth() + 1, Dd = (d) => d.getUTCDate();
  const pad = (n) => String(n).padStart(2, "0");
  const isoDay = (d) => `${String(Y(d)).padStart(4, "0")}-${pad(M(d))}-${pad(Dd(d))}`;
  const addDays = (d, n) => new Date(d.getTime() + n * DAY);
  const addMonths = (d, n) => {
    const first = D(Y(d), M(d) + n, 1);
    const last = D(Y(first), M(first) + 1, 0);
    return D(Y(first), M(first), Math.min(Dd(d), Dd(last)));
  };
  const today = () => {
    const n = new Date();
    return D(n.getFullYear(), n.getMonth() + 1, n.getDate());
  };
  const isObj = (v) => v !== null && typeof v === "object" && !Array.isArray(v);
  // Interface text in the document's words (its `labels`), by the English.
  const L = (text, vars) => window.__irender.label(text, vars);
  const blank = (v) => v === null || v === undefined || v === "";

  // ---- the picker kinds: each has keys (`2026-03-05`, `2026-03`, `2026-Q1`, `2026`),
  // the dates they cover, and how they step
  function units(fs) {
    // A financial year is named by the calendar year it starts in.
    const fy = (d) => (M(d) >= fs ? Y(d) : Y(d) - 1);
    return {
      day: {
        key: isoDay,
        start: (k) => D(+k.slice(0, 4), +k.slice(5, 7), +k.slice(8, 10)),
        end: (k) => D(+k.slice(0, 4), +k.slice(5, 7), +k.slice(8, 10)),
        step: (k, n) => isoDay(addDays(D(+k.slice(0, 4), +k.slice(5, 7), +k.slice(8, 10)), n)),
      },
      month: {
        key: (d) => `${Y(d)}-${pad(M(d))}`,
        start: (k) => D(+k.slice(0, 4), +k.slice(5, 7), 1),
        end: (k) => D(+k.slice(0, 4), +k.slice(5, 7) + 1, 0),
        step: (k, n) => {
          const d = D(+k.slice(0, 4), +k.slice(5, 7) + n, 1);
          return `${Y(d)}-${pad(M(d))}`;
        },
      },
      quarter: {
        key: (d) => `${fy(d)}-Q${Math.floor(((M(d) - fs + 12) % 12) / 3) + 1}`,
        start: (k) => D(+k.slice(0, 4), fs + (+k.slice(6) - 1) * 3, 1),
        end: (k) => D(+k.slice(0, 4), fs + (+k.slice(6) - 1) * 3 + 3, 0),
        step: (k, n) => {
          const d = D(+k.slice(0, 4), fs + (+k.slice(6) - 1 + n) * 3, 1);
          return `${fy(d)}-Q${Math.floor(((M(d) - fs + 12) % 12) / 3) + 1}`;
        },
      },
      year: {
        key: (d) => String(fy(d)),
        start: (k) => D(+k, fs, 1),
        end: (k) => D(+k + 1, fs, 0),
        step: (k, n) => String(+k + n),
      },
    };
  }

  function start(el, api, popover) {
    const cfg = api.config;
    // A week is picked from the days: the grid is a month's, and a pick is its whole week.
    const week = cfg.picker === "week", kind = week ? "day" : cfg.picker, mode = cfg.mode, fs = cfg.fiscalStart || 1;
    const U = units(fs), u = U[kind];
    const loc = cfg.locale || undefined;
    const $ = (s) => el.querySelector(s);
    const body = $("[data-cal]"), live = $("[data-cal-live]");
    const trigger = $("[data-cal-trigger]"), pop = $("[data-cal-pop]"), clearX = $("[data-cal-x]");
    // With `input`, a text field to type the value in, beside a button for the calendar.
    const field = $("[data-cal-input]");
    const home = () => (field || trigger)?.focus();

    // ---- parsing: ISO dates and keys, and `today`, `today-30d`, `today+1y`
    function parseDate(v) {
      if (blank(v)) return null;
      const s = String(v).trim();
      let m = s.match(/^today(?:\s*([+-])\s*(\d+)\s*([dwmy]))?$/i);
      if (m) {
        let d = today();
        if (m[1]) {
          const n = (m[1] === "-" ? -1 : 1) * Number(m[2]), unit = m[3].toLowerCase();
          d = unit === "d" ? addDays(d, n) : unit === "w" ? addDays(d, 7 * n) : addMonths(d, unit === "m" ? n : 12 * n);
        }
        return d;
      }
      if ((m = s.match(/^(\d{4})-(\d{2})-(\d{2})/))) return D(+m[1], +m[2], +m[3]);
      if ((m = s.match(/^(\d{4})-W(\d{2})$/i))) return isoMonday(+m[1], +m[2]);
      if ((m = s.match(/^(\d{4})-(\d{2})$/))) return D(+m[1], +m[2], 1);
      if ((m = s.match(/^(\d{4})-Q([1-4])$/i))) return U.quarter.start(`${m[1]}-Q${m[2]}`);
      if ((m = s.match(/^(\d{4})$/))) return U.year.start(m[1]);
      return null;
    }
    // ISO weeks: Monday to Sunday, the first one holding the year's first Thursday.
    function isoMonday(y, w) {
      const jan4 = D(y, 1, 4);
      return addDays(jan4, (w - 1) * 7 - ((jan4.getUTCDay() + 6) % 7));
    }
    const mondayOf = (k) => {
      const d = u.start(k);
      return addDays(d, -((d.getUTCDay() + 6) % 7));
    };
    function isoWeek(k) {
      const thu = addDays(mondayOf(k), 3), y = thu.getUTCFullYear();
      return `${y}-W${pad(Math.floor((thu - isoMonday(y, 1)) / (7 * DAY)) + 1)}`;
    }
    const keyOf = (v) => {
      const d = parseDate(v);
      return d ? u.key(d) : null;
    };
    const timeOf = (v) => (typeof v === "string" && /T(\d{2}:\d{2})/.exec(v)?.[1]) || null;

    // ---- props that may use state
    const env = () => {
      const s = api.scope();
      return s ? { ...api.state, ...s } : api.state;
    };
    const resolve = (v) => {
      if (Array.isArray(v)) return v.map(resolve);
      if (!isObj(v)) return v;
      if ("$irender" in v && Object.keys(v).length === 1) return window.__irender.evaluate(v.$irender, env());
      return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, resolve(x)]));
    };

    // ---- the value: in state when bound, here otherwise
    let local = cfg.value ?? null;
    const raw = () => (cfg.bind ? api.state[cfg.bind] : local);
    function read() {
      const v = raw();
      if (mode === "range") return { from: keyOf(isObj(v) ? v.from : null), to: keyOf(isObj(v) ? v.to : null) };
      if (mode === "multiple") return (Array.isArray(v) ? v : []).map(keyOf).filter(Boolean);
      return keyOf(v);
    }
    let time = timeOf(raw()) || "";
    // A range's start and end times, with `time`. Not picked, they're 00:00 and 23:59,
    // so the range takes in the whole of its last day.
    let fromTime = "", toTime = "";
    function readTimes() {
      const v = raw();
      if (mode === "range" && isObj(v)) {
        fromTime = timeOf(v.from) || fromTime;
        toTime = timeOf(v.to) || toTime;
      } else time = timeOf(v) || time;
    }
    readTimes();
    const withTime = (k, t, dflt) => `${k}T${t || dflt}`;
    // A range's ends as written: with their times, when there are times.
    const ends = (v) => (cfg.time ? { from: v.from && withTime(v.from, fromTime, "00:00"), to: v.to && withTime(v.to, toTime, "23:59") } : { from: v.from, to: v.to });
    // The dates a value covers, for `bindRange`.
    function rangeOf(v) {
      if (week) return v ? { from: isoDay(mondayOf(v)), to: isoDay(addDays(mondayOf(v), 6)) } : null;
      if (mode === "range" && cfg.time) return v.from ? { ...ends(v), to: v.to ? ends(v).to : null } : null;
      if (mode === "range") return v.from ? { from: isoDay(u.start(v.from)), to: v.to ? isoDay(u.end(v.to)) : null } : null;
      if (mode === "multiple") return v.map((k) => ({ from: isoDay(u.start(k)), to: isoDay(u.end(k)) }));
      if (!v) return null;
      if (cfg.time && time) return { from: `${v}T${time}`, to: `${v}T${time}` };
      return { from: isoDay(u.start(v)), to: isoDay(u.end(v)) };
    }
    function write(v) {
      const out = week ? (v ? isoWeek(v) : null) : mode === "range" ? (v.from ? { ...ends(v), to: v.to ? ends(v).to : null } : null) : mode === "multiple" ? v : v && cfg.time ? `${v}T${time || "00:00"}` : v;
      // The previous period and last year's follow the range.
      const c = readCompare();
      if (c && c.type !== "custom") storeCompare({ type: c.type, ...derive(c.type, v) });
      if (cfg.bindRange) api.set(cfg.bindRange, rangeOf(v));
      if (cfg.bind) api.set(cfg.bind, out);
      else {
        local = out;
        render();
      }
      api.emit("change", { value: out, range: rangeOf(v), compare: kinds ? rawCompare() ?? null : undefined });
    }

    // ---- a period to compare with (`compare`, ranges only): the one before, the same
    // one a year earlier, or one picked on the calendar. Kept as the dates it covers,
    // `{ type, from, to }`, or null while it's off.
    const kinds = mode === "range" && Array.isArray(cfg.compare) && cfg.compare.length ? cfg.compare : null;
    let localCompare = null, picking = false;
    const rawCompare = () => (cfg.bindCompare ? api.state[cfg.bindCompare] : localCompare);
    function readCompare() {
      const c = rawCompare();
      if (!kinds || !isObj(c) || !kinds.includes(c.type)) return null;
      return { type: c.type, from: keyOf(c.from), to: keyOf(c.to) };
    }
    // The period `type` makes of a whole range, as keys.
    function derive(type, v) {
      if (!v.from || !v.to) return { from: null, to: null };
      if (type === "lastYear") {
        const back = (k) => (kind === "day" ? isoDay(addMonths(u.start(k), -12)) : u.step(k, { month: -12, quarter: -4, year: -1 }[kind]));
        return { from: back(v.from), to: back(v.to) };
      }
      if (type !== "previous") return { from: null, to: null };
      // As many days (months, quarters, years) as the range, just before it.
      let n = 1;
      if (kind === "day") n = Math.round((u.start(v.to) - u.start(v.from)) / DAY) + 1;
      else for (let k = v.from; k !== v.to && n < 1e4; k = u.step(k, 1)) n++;
      return { from: u.step(v.from, -n), to: u.step(v.to, -n) };
    }
    function storeCompare(c) {
      const out = c ? { type: c.type, from: c.from ? isoDay(u.start(c.from)) : null, to: c.to ? isoDay(u.end(c.to)) : null } : null;
      const was = rawCompare() ?? null;
      if (JSON.stringify(was) === JSON.stringify(out)) return false;
      if (cfg.bindCompare) api.set(cfg.bindCompare, out);
      else localCompare = out;
      return true;
    }
    // A change made from the comparison's own controls.
    function writeCompare(c) {
      if (!storeCompare(c)) return render(true);
      if (!cfg.bindCompare) render(true);
      const v = read();
      api.emit("change", { value: mode === "range" && v.from ? { ...ends(v), to: v.to ? ends(v).to : null } : null, range: rangeOf(v), compare: rawCompare() ?? null });
    }
    // The previous period and last year's kept in step with a range set elsewhere.
    function followCompare() {
      const c = readCompare();
      if (!c || c.type === "custom") return;
      const d = derive(c.type, read());
      if (d.from !== c.from || d.to !== c.to) storeCompare({ type: c.type, ...d });
    }

    // ---- limits
    const limits = () => ({ min: parseDate(resolve(cfg.min)), max: parseDate(resolve(cfg.max)) });
    function disabled(k, lim) {
      if (lim.min && u.end(k) < lim.min) return true;
      if (lim.max && u.start(k) > lim.max) return true;
      if (kind !== "day") return false;
      const d = u.start(k);
      if (Array.isArray(cfg.disabledDays) && cfg.disabledDays.map(Number).includes(d.getUTCDay())) return true;
      const list = resolve(cfg.disabledDates);
      if (!Array.isArray(list)) return false;
      return list.some((x) => {
        if (isObj(x)) {
          const a = parseDate(x.from), b = parseDate(x.to);
          return (!a || d >= a) && (!b || d <= b);
        }
        const a = parseDate(x);
        return a && +a === +d;
      });
    }

    // ---- words
    const fyName = (y) => (fs === 1 ? String(y) : `FY${y}–${String(y + 1).slice(-2)}`);
    const F = {};
    const fmt = (name, o) => (F[name] ||= new Intl.DateTimeFormat(loc, { timeZone: "UTC", ...o }));
    function label(k, long) {
      if (!k) return "";
      if (kind === "day") return fmt(long ? "dayLong" : "day", { dateStyle: long ? "full" : "medium" }).format(u.start(k));
      if (kind === "month") return fmt(long ? "monthLong" : "month", { month: long ? "long" : "short", year: "numeric" }).format(u.start(k));
      if (kind === "quarter") {
        const q = `Q${k.slice(6)} ${fyName(+k.slice(0, 4))}`;
        if (!long) return q;
        const f = fmt("span", { month: "long", year: "numeric" });
        return `${q}, ${f.format(u.start(k))} – ${f.format(u.end(k))}`;
      }
      return fyName(+k);
    }
    const timeText = (t) => (t ? fmt("time", { timeStyle: "short" }).format(new Date(Date.UTC(1970, 0, 1, +t.slice(0, 2), +t.slice(3, 5)))) : "");
    function display(v) {
      if (mode === "range") {
        if (!v.from) return "";
        if (!v.to) return `${label(v.from)} – …`;
        if (cfg.time) {
          // One day: its times once; two: each day with its time.
          const a = timeText(fromTime || "00:00"), b = timeText(toTime || "23:59");
          return v.from === v.to ? `${label(v.from)}, ${a} – ${b}` : `${label(v.from)}, ${a} – ${label(v.to)}, ${b}`;
        }
        return v.from === v.to ? label(v.from) : `${label(v.from)} – ${label(v.to)}`;
      }
      if (mode === "multiple") return v.length === 0 ? "" : v.length === 1 ? label(v[0]) : L("{n} selected", { n: v.length });
      if (!v) return "";
      // Labels: "Week {n}, {year}"
      if (week) {
        const w = isoWeek(v), f = fmt("weekSpan", { month: "short", day: "numeric" });
        return `${L("Week {n}, {year}", { n: +w.slice(6), year: w.slice(0, 4) })} (${f.format(mondayOf(v))} – ${f.format(addDays(mondayOf(v), 6))})`;
      }
      return cfg.time && time ? `${label(v)}, ${timeText(time)}` : label(v);
    }

    // A comparison as text: `Feb 1, 2026 – Feb 28, 2026`.
    const spanText = (r) => (!r.from ? "" : !r.to ? `${label(r.from)} – …` : r.from === r.to ? label(r.from) : `${label(r.from)} – ${label(r.to)}`);
    // Labels: "Previous period", "Same period last year", "Custom"
    const KINDS = { previous: "Previous period", lastYear: "Same period last year", custom: "Custom" };

    // ---- presets: names, or `{ label, from, to }` with dates or `today±N`
    // Labels: "Today", "Yesterday", "Tomorrow", "Last 7 days", "Last 30 days", "Last 90 days", "Next 7 days", "Next 30 days", "This week", "Last week", "This month", "Last month", "Next month", "This quarter", "Last quarter", "This year", "Last year", "Year to date", "Month to date"
    const NAMES = {
      today: "Today", yesterday: "Yesterday", tomorrow: "Tomorrow", last7: "Last 7 days", last30: "Last 30 days",
      last90: "Last 90 days", next7: "Next 7 days", next30: "Next 30 days", thisWeek: "This week", lastWeek: "Last week",
      thisMonth: "This month", lastMonth: "Last month", nextMonth: "Next month", thisQuarter: "This quarter",
      lastQuarter: "Last quarter", thisYear: "This year", lastYear: "Last year", yearToDate: "Year to date", monthToDate: "Month to date",
    };
    function preset(p) {
      if (isObj(p)) return { label: String(p.label), from: parseDate(resolve(p.from)), to: parseDate(resolve(p.to ?? p.from)) };
      const t = today(), ws = weekStart();
      const week = addDays(t, -((t.getUTCDay() - ws + 7) % 7));
      const q = (d) => U.quarter.key(d), y = (d) => U.year.key(d);
      const span = (a, b) => ({ from: a, to: b });
      const r = {
        today: span(t, t), yesterday: span(addDays(t, -1), addDays(t, -1)), tomorrow: span(addDays(t, 1), addDays(t, 1)),
        last7: span(addDays(t, -6), t), last30: span(addDays(t, -29), t), last90: span(addDays(t, -89), t),
        next7: span(t, addDays(t, 6)), next30: span(t, addDays(t, 29)),
        thisWeek: span(week, addDays(week, 6)), lastWeek: span(addDays(week, -7), addDays(week, -1)),
        thisMonth: span(U.month.start(U.month.key(t)), U.month.end(U.month.key(t))),
        lastMonth: span(U.month.start(U.month.step(U.month.key(t), -1)), U.month.end(U.month.step(U.month.key(t), -1))),
        nextMonth: span(U.month.start(U.month.step(U.month.key(t), 1)), U.month.end(U.month.step(U.month.key(t), 1))),
        thisQuarter: span(U.quarter.start(q(t)), U.quarter.end(q(t))),
        lastQuarter: span(U.quarter.start(U.quarter.step(q(t), -1)), U.quarter.end(U.quarter.step(q(t), -1))),
        thisYear: span(U.year.start(y(t)), U.year.end(y(t))),
        lastYear: span(U.year.start(String(+y(t) - 1)), U.year.end(String(+y(t) - 1))),
        yearToDate: span(U.year.start(y(t)), t), monthToDate: span(U.month.start(U.month.key(t)), t),
      }[p];
      // `quarterN`: the quarter N before today's, named by its (financial) year.
      const back = /^quarter(\d)$/.exec(p);
      if (back) {
        const k = U.quarter.step(q(t), -Number(back[1])), [year, n] = k.split("-Q");
        return { label: quarterName(year, n), ...span(U.quarter.start(k), U.quarter.end(k)) };
      }
      return { label: NAMES[p] && L(NAMES[p]), ...r };
    }
    // "Q2 2026", or on a financial year "Q2 FY2026–27", as the calendar names quarters.
    const quarterName = (year, n) => L("Q{n} {year}", { n, year: fyName(+year) });
    function applyPreset(p) {
      const { from, to } = preset(p);
      if (!from) return;
      if (mode === "range") {
        const v = { from: u.key(from), to: u.key(to || from) };
        focus = v.from;
        view = pageOf(v.from);
        write(v);
      } else {
        focus = u.key(from);
        view = pageOf(focus);
        write(mode === "multiple" ? [...new Set([...read(), u.key(from)])] : u.key(from));
      }
      if (popover && cfg.closeOnSelect) close();
    }

    // ---- what a page of the calendar shows
    function weekStart() {
      if (cfg.weekStart != null) return cfg.weekStart;
      try {
        const l = new Intl.Locale(loc || navigator.language);
        const info = l.getWeekInfo?.() ?? l.weekInfo;
        if (info?.firstDay) return info.firstDay % 7;
      } catch {}
      return 0;
    }
    // A page is a month (days), a year (months, quarters) or twelve years.
    const pageOf = (k) => (kind === "day" ? k.slice(0, 7) : kind === "year" ? String(+k - (((+k % 12) + 12) % 12)) : k.slice(0, 4));
    const pageStep = (p, n) => (kind === "day" ? U.month.step(p, n) : kind === "year" ? String(+p + 12 * n) : String(+p + n));
    // The dates a page is about, without a month's spill-over days.
    const pageStart = (p) => (kind === "day" ? U.month.start(p) : u.start(cells(p)[0]));
    const pageEnd = (p) => (kind === "day" ? U.month.end(p) : u.end(cells(p).at(-1)));
    const pageCount = kind === "day" ? Math.max(1, cfg.months || 1) : 1;
    const cols = { day: 7, month: 3, quarter: 4, year: 3 }[kind];
    function cells(p) {
      if (kind === "day") {
        const first = U.month.start(p), ws = weekStart();
        const from = addDays(first, -((first.getUTCDay() - ws + 7) % 7));
        return Array.from({ length: 42 }, (_, i) => isoDay(addDays(from, i)));
      }
      if (kind === "month") return Array.from({ length: 12 }, (_, i) => `${p}-${pad(i + 1)}`);
      if (kind === "quarter") return [1, 2, 3, 4].map((q) => `${p}-Q${q}`);
      return Array.from({ length: 12 }, (_, i) => String(+p + i));
    }
    function caption(p) {
      if (kind === "day") return fmt("caption", { month: "long", year: "numeric" }).format(U.month.start(p));
      if (kind === "year") return `${fyName(+p)} – ${fyName(+p + 11)}`;
      return kind === "quarter" ? fyName(+p) : p;
    }
    const cellText = (k) =>
      kind === "day" ? String(+k.slice(8, 10))
      : kind === "month" ? fmt("short", { month: "short" }).format(u.start(k))
      : kind === "quarter" ? `Q${k.slice(6)}`
      : fyName(+k);

    // ---- state of the view
    const first = () => {
      const v = read();
      return (mode === "range" ? v.from : mode === "multiple" ? v[0] : v) || u.key(today());
    };
    let focus = first(), view = pageOf(focus), hover = null;
    const pages = () => Array.from({ length: pageCount }, (_, i) => pageStep(view, i));
    const shownPages = () => new Set(pages());
    function reveal(k) {
      const p = pageOf(k), all = pages();
      if (all.includes(p)) return;
      // Moving past the last page shows the next one; before the first, the one before.
      view = p > all.at(-1) ? pageStep(p, -(pageCount - 1)) : p;
    }

    // ---- drawing
    const h = (tag, attrs = {}, ...kids) => {
      const e = document.createElement(tag);
      for (const [k, v] of Object.entries(attrs)) if (v !== null && v !== undefined && v !== false) e.setAttribute(k, v === true ? "" : v);
      e.append(...kids.filter((k) => k !== null && k !== undefined));
      return e;
    };
    const ARROW = (d) => {
      const s = document.createElementNS("http://www.w3.org/2000/svg", "svg");
      s.setAttribute("width", "14");
      s.setAttribute("height", "14");
      s.setAttribute("viewBox", "0 0 16 16");
      s.setAttribute("fill", "none");
      s.setAttribute("stroke", "currentColor");
      s.setAttribute("stroke-width", "1.6");
      s.setAttribute("stroke-linecap", "round");
      s.setAttribute("stroke-linejoin", "round");
      s.setAttribute("aria-hidden", "true");
      const p = document.createElementNS("http://www.w3.org/2000/svg", "path");
      p.setAttribute("d", d === "prev" ? "M10 3.5 5.5 8l4.5 4.5" : "M6 3.5 10.5 8 6 12.5");
      s.setAttribute("data-flip", "");
      s.append(p);
      return s;
    };
    const uid = el.id || el.querySelector("[id]")?.id || "cal";
    // Labels: "Previous month", "Next month", "Previous year", "Next year", "Previous 12 years", "Next 12 years"
    const unitWord = { day: "month", month: "year", quarter: "year", year: "12 years" }[kind];

    function render(keepFocus) {
      const v = read(), lim = limits(), t = u.key(today());
      const focused = keepFocus ?? body.contains(document.activeElement);
      const sameWeek = (k) => week && !!v && isoWeek(k) === isoWeek(v);
      const chosen = (k) => (week ? sameWeek(k) : mode === "range" ? k === v.from || k === v.to : mode === "multiple" ? v.includes(k) : k === v);
      // A range in progress shows up to the cell under the pointer.
      const span = (r, live) => (r?.from ? [r.from, r.to ?? (live ? hover : null) ?? r.from].sort((x, y) => u.start(x) - u.start(y)) : [null, null]);
      const [a, b] = mode === "range" ? span(v, !picking) : [null, null];
      const between = (k) => a && u.start(k) > u.start(a) && u.start(k) < u.start(b);
      // The comparison, drawn over the range, and while it's picked up to the pointer.
      const c = readCompare();
      const [ca, cb] = span(c, picking);
      const compared = (k) => ca && u.start(k) >= u.start(ca) && u.start(k) <= u.start(cb);
      // A control of the comparison's that had focus gets it back.
      const ctl = body.contains(document.activeElement) ? document.activeElement.getAttribute("data-cal-ctl") : null;

      const wrap = h("div", { "data-cal-wrap": true });
      if (cfg.presets?.length) {
        const list = h("div", { "data-cal-presets": true, role: "group", "aria-label": L("Presets") });
        for (const [i, p] of cfg.presets.entries()) {
          const r = preset(p), keys = r.from && { from: u.key(r.from), to: u.key(r.to || r.from) };
          const on = keys && (mode === "range" ? v.from === keys.from && v.to === keys.to : mode === "single" ? v === keys.from && keys.from === keys.to : false);
          list.append(h("button", { type: "button", "data-preset": i, "aria-pressed": on ? "true" : "false" }, r.label));
        }
        wrap.append(list);
      }
      const main = h("div", { "data-cal-main": true });
      const shown = pages();
      const head = h("div", { "data-cal-head": true },
        // A page wholly outside the limits can't be reached.
        h("button", { type: "button", "data-cal-step": "-1", "aria-label": L(`Previous ${unitWord}`), disabled: lim.min && pageEnd(pageStep(shown[0], -1)) < lim.min ? true : null }, ARROW("prev")),
      );
      const captions = h("div", { "data-cal-captions": true });
      shown.forEach((p, i) => {
        const id = `${uid}-cap-${i}`;
        if (cfg.dropdowns && kind === "day" && i === 0) {
          const ty = Y(today());
          const lo = lim.min ? Y(lim.min) : ty - 100, hi = lim.max ? Y(lim.max) : ty + 10;
          const month = h("select", { "data-cal-month": true, "aria-label": L("Month") });
          for (let m = 1; m <= 12; m++) month.append(h("option", { value: m, selected: m === +p.slice(5, 7) }, fmt("mname", { month: "long" }).format(D(2024, m, 1))));
          const year = h("select", { "data-cal-year": true, "aria-label": L("Year") });
          for (let y = hi; y >= lo; y--) year.append(h("option", { value: y, selected: y === +p.slice(0, 4) }, String(y)));
          captions.append(h("div", { "data-cal-caption": true, id }, month, year));
        } else captions.append(h("div", { "data-cal-caption": true, id, "aria-live": i === 0 ? "polite" : null }, caption(p)));
      });
      head.append(captions, h("button", { type: "button", "data-cal-step": "1", "aria-label": L(`Next ${unitWord}`), disabled: lim.max && pageStart(pageStep(shown.at(-1), 1)) > lim.max ? true : null }, ARROW("next")));
      main.append(head);

      const grids = h("div", { "data-cal-grids": true });
      shown.forEach((p, gi) => {
        const table = h("table", { role: "grid", "aria-labelledby": `${uid}-cap-${gi}` });
        if (kind === "day") {
          const ws = weekStart(), tr = h("tr");
          for (let i = 0; i < 7; i++) {
            const d = D(2024, 1, 7 + ((ws + i) % 7)); // 7 January 2024 was a Sunday
            tr.append(h("th", { scope: "col", abbr: fmt("wlong", { weekday: "long" }).format(d) }, fmt("wshort", { weekday: "short" }).format(d).slice(0, 2)));
          }
          table.append(h("thead", {}, tr));
        }
        const tbody = h("tbody");
        const list = cells(p);
        for (let r = 0; r < list.length / cols; r++) {
          const tr = h("tr");
          for (const k of list.slice(r * cols, r * cols + cols)) {
            const outside = kind === "day" && k.slice(0, 7) !== p;
            const off = disabled(k, lim), sel = chosen(k);
            // Days outside the month are left for the month they belong to when two show.
            if (outside && pageCount > 1) {
              tr.append(h("td", { role: "gridcell" }));
              continue;
            }
            const btn = h("button", {
              type: "button",
              "data-key": k,
              tabindex: k === focus && !(outside && pageCount > 1) ? "0" : "-1",
              "aria-label": compared(k) ? `${label(k, true)}, ${L("compared")}` : label(k, true),
              "aria-pressed": sel ? "true" : "false",
              "aria-disabled": off ? "true" : null,
              "aria-current": k === t ? "date" : null,
              "data-today": k === t || null,
              "data-outside": outside || null,
              "data-in-range": between(k) || (sameWeek(k) && isoDay(mondayOf(k)) !== k && isoDay(addDays(mondayOf(k), 6)) !== k) || null,
              "data-range-start": (a && k === a) || (sameWeek(k) && isoDay(mondayOf(k)) === k) || null,
              "data-range-end": (b && k === b) || (sameWeek(k) && isoDay(addDays(mondayOf(k), 6)) === k) || null,
              "data-compare": compared(k) || null,
              "data-compare-start": ca && k === ca ? true : null,
              "data-compare-end": cb && k === cb ? true : null,
            }, cellText(k));
            tr.append(h("td", { role: "gridcell", "aria-selected": sel ? "true" : "false" }, btn));
          }
          tbody.append(tr);
        }
        table.append(tbody);
        grids.append(table);
      });
      main.append(grids);

      if (cfg.time && mode === "range") {
        // A start and an end time, side by side.
        const one = (end) => {
          const input = h("input", { type: "time", step: String(Math.round(cfg.step * 60)), value: end ? toTime : fromTime, "aria-label": end ? L("End time") : L("Start time"), "data-cal-time-input": end ? "to" : "from" });
          return h("label", { "data-cal-time": true }, end ? L("End") : L("Start"), input);
        };
        main.append(h("div", { "data-cal-times": true }, one(false), one(true)));
      } else if (cfg.time) {
        const input = h("input", { type: "time", step: String(Math.round(cfg.step * 60)), value: time, "aria-label": L("Time"), "data-cal-time-input": true });
        main.append(h("label", { "data-cal-time": true }, L("Time"), input));
      }
      if (kinds) {
        const box = h("div", { "data-cal-compare": true });
        box.append(h("label", {}, h("input", { type: "checkbox", "data-cal-ctl": "compare", checked: !!c }), L("Compare")));
        if (c) {
          const group = h("div", { "data-cal-compare-kinds": true, role: "group", "aria-label": L("Compare with") });
          for (const t of kinds) group.append(h("button", { type: "button", "data-compare-kind": t, "data-cal-ctl": t, "aria-pressed": c.type === t ? "true" : "false" }, L(KINDS[t])));
          if (kinds.length > 1 || kinds[0] === "custom") box.append(group);
          const text = picking ? (c.from && !c.to ? L("Pick the comparison's last day") : L("Pick the comparison's first day"))
            : c.from ? L("vs {range}", { range: spanText(c) }) : L("Pick a range to compare");
          box.append(h("div", { "data-cal-compare-text": true }, text));
        }
        main.append(box);
      }
      const foot = h("div", { "data-cal-foot": true });
      if (cfg.clearable && !popover) foot.append(h("button", { type: "button", "data-cal-clear": true }, L("Clear")));
      main.append(foot);
      wrap.append(main);
      body.replaceChildren(wrap);
      if (ctl) body.querySelector(`[data-cal-ctl="${ctl}"]`)?.focus({ preventScroll: true });
      else if (focused) body.querySelector(`[data-key="${focus}"]`)?.focus({ preventScroll: true });
      sync(v);
    }

    // The trigger's text (or the field's), the clear button and the hidden inputs.
    function sync(v) {
      const text = display(v);
      if (field) {
        // Not over text still being typed, or not yet a value.
        if (!field.__dirty) field.value = typed(v);
        el.toggleAttribute("data-empty", !text);
      } else if (trigger) {
        const span = trigger.querySelector("[data-cal-text]");
        span.textContent = text || span.getAttribute("data-placeholder");
        // The whole value on hover, for when it's cut short.
        trigger.title = text;
        el.toggleAttribute("data-empty", !text);
      }
      if (clearX) clearX.hidden = !text;
      const vs = $("[data-cal-vs]"), c = readCompare();
      if (vs) {
        vs.hidden = !(text && c?.from && c.to);
        vs.textContent = vs.hidden ? "" : L("vs {range}", { range: spanText(c) });
        if (trigger && !field && !vs.hidden) trigger.title = `${text} ${vs.textContent}`;
      }
      const hidden = $("[data-cal-hidden]");
      if (hidden && cfg.name) {
        const inputs = mode === "range" ? [[`${cfg.name}[from]`, ends(v).from ?? ""], [`${cfg.name}[to]`, ends(v).to ?? ""]]
          : mode === "multiple" ? v.map((k) => [cfg.name, k])
          : [[cfg.name, v ? (cfg.time ? `${v}T${time || "00:00"}` : v) : ""]];
        const cv = c && rawCompare();
        if (cv) inputs.push([`${cfg.name}[compareFrom]`, cv.from ?? ""], [`${cfg.name}[compareTo]`, cv.to ?? ""]);
        hidden.replaceChildren(...inputs.map(([n, val]) => h("input", { type: "hidden", name: n, value: val })));
      }
    }
    const say = (msg) => live && (live.textContent = msg);

    // ---- choosing
    function choose(k) {
      if (disabled(k, limits())) return;
      focus = k;
      const v = read();
      if (mode === "single") {
        write(week ? isoDay(mondayOf(k)) : k);
        say(week ? L("{date} selected", { date: display(k) }) : L("{date} selected", { date: label(k, true) }));
        if (popover && cfg.closeOnSelect) close();
      } else if (mode === "multiple") {
        write(v.includes(k) ? v.filter((x) => x !== k) : [...v, k].sort((x, y) => u.start(x) - u.start(y)));
      } else if (picking) {
        // A custom comparison: two picks, as a range takes.
        const c = readCompare();
        hover = null;
        if (!c?.from || c.to) {
          writeCompare({ type: "custom", from: k, to: null });
          say(L("Comparison starts {date}; pick the end", { date: label(k, true) }));
        } else {
          const [from, to] = [c.from, k].sort((x, y) => u.start(x) - u.start(y));
          picking = false;
          writeCompare({ type: "custom", from, to });
          say(L("Comparing with {from} to {to}", { from: label(from, true), to: label(to, true) }));
          if (popover && cfg.closeOnSelect && read().to) close();
        }
      } else if (!v.from || v.to) {
        hover = null;
        write({ from: k, to: null });
        say(L("Range starts {date}; pick the end", { date: label(k, true) }));
      } else {
        const [from, to] = [v.from, k].sort((x, y) => u.start(x) - u.start(y));
        hover = null;
        write({ from, to });
        say(L("{from} to {to} selected", { from: label(from, true), to: label(to, true) }));
        if (popover && cfg.closeOnSelect) close();
      }
      render(true);
    }
    function clear() {
      hover = null;
      time = fromTime = toTime = "";
      picking = false;
      if (kinds) storeCompare(null);
      write(mode === "range" ? { from: null, to: null } : mode === "multiple" ? [] : null);
      say(L("Cleared"));
    }

    // ---- the keyboard, on the grid's buttons
    function move(k) {
      focus = k;
      reveal(k);
      const r = picking ? readCompare() : read();
      if (mode === "range" && r?.from && !r.to) hover = k;
      render(true);
    }
    body.addEventListener("keydown", (e) => {
      const k = e.target.getAttribute?.("data-key");
      if (!k) return;
      const d = u.start(k);
      let next = null;
      // Right to left, the next day is on the left.
      const fwd = getComputedStyle(body).direction === "rtl" ? -1 : 1;
      switch (e.key) {
        case "ArrowLeft": next = u.step(k, -fwd); break;
        case "ArrowRight": next = u.step(k, fwd); break;
        case "ArrowUp": next = u.step(k, -cols); break;
        case "ArrowDown": next = u.step(k, cols); break;
        case "Home": {
          const at = kind === "day" ? (d.getUTCDay() - weekStart() + 7) % 7 : cells(pageOf(k)).indexOf(k) % cols;
          next = u.step(k, -at);
          break;
        }
        case "End": {
          const at = kind === "day" ? (d.getUTCDay() - weekStart() + 7) % 7 : cells(pageOf(k)).indexOf(k) % cols;
          next = u.step(k, cols - 1 - at);
          break;
        }
        case "PageUp":
        case "PageDown": {
          const n = e.key === "PageUp" ? -1 : 1;
          next = kind === "day" ? u.key(addMonths(d, e.shiftKey ? 12 * n : n)) : u.step(k, n * (kind === "year" ? 12 : kind === "month" ? 12 : 4));
          break;
        }
        case "Enter":
        case " ":
          e.preventDefault();
          return choose(k);
        default:
          return;
      }
      e.preventDefault();
      move(next);
    });

    body.addEventListener("click", (e) => {
      const b = e.target.closest("button");
      if (!b || !body.contains(b)) return;
      if (b.hasAttribute("data-key")) {
        const k = b.getAttribute("data-key");
        if (kind === "day" && k.slice(0, 7) !== pageOf(k) && !pages().includes(pageOf(k))) reveal(k);
        return choose(k);
      }
      if (b.hasAttribute("data-cal-step")) {
        view = pageStep(view, Number(b.getAttribute("data-cal-step")));
        // Focus follows the page, on the same day of a month or the same place.
        const target = kind === "day" ? u.key(addMonths(u.start(focus), Number(b.getAttribute("data-cal-step")))) : u.step(focus, Number(b.getAttribute("data-cal-step")) * (kind === "year" ? 12 : kind === "month" ? 12 : 4));
        focus = target;
        render(false);
        return;
      }
      if (b.hasAttribute("data-preset")) return applyPreset(cfg.presets[Number(b.getAttribute("data-preset"))]);
      if (b.hasAttribute("data-cal-clear")) return clear();
      if (b.hasAttribute("data-compare-kind")) {
        const t = b.getAttribute("data-compare-kind"), c = readCompare();
        hover = null;
        // Custom starts from the dates compared now, and takes the next two picks.
        picking = t === "custom";
        if (t === "custom") {
          say(L("Pick the comparison's first day"));
          return writeCompare({ type: t, from: c?.from ?? null, to: c?.to ?? null });
        }
        return writeCompare({ type: t, ...derive(t, read()) });
      }
    });
    body.addEventListener("change", (e) => {
      const t = e.target;
      if (t.matches('[data-cal-ctl="compare"]')) {
        hover = null;
        if (!t.checked) {
          picking = false;
          return writeCompare(null);
        }
        const k = kinds[0];
        picking = k === "custom";
        return writeCompare({ type: k, ...derive(k, read()) });
      }
      if (t.matches("[data-cal-month], [data-cal-year]")) {
        const y = body.querySelector("[data-cal-year]").value, m = body.querySelector("[data-cal-month]").value;
        view = `${y}-${pad(m)}`;
        focus = u.key(D(+y, +m, Math.min(Dd(u.start(focus)), Dd(D(+y, +m + 1, 0)))));
        render(false);
      } else if (t.matches("[data-cal-time-input]")) {
        const v = read();
        if (mode === "range") {
          if (t.getAttribute("data-cal-time-input") === "to") toTime = t.value;
          else fromTime = t.value;
          if (v.from) write(v);
          return;
        }
        time = t.value;
        if (v) write(v);
        else write(focus);
      }
    });
    // A range in progress follows the pointer.
    body.addEventListener("mouseover", (e) => {
      const k = e.target.closest?.("[data-key]")?.getAttribute("data-key");
      if (!k || mode !== "range") return;
      const v = picking ? readCompare() : read();
      if (!v?.from || v.to || hover === k) return;
      hover = k;
      render(false);
    });

    // ---- the popover
    function close() {
      if (!pop) return;
      try {
        pop.hidePopover();
      } catch {}
      home();
    }
    if (pop) {
      pop.addEventListener("toggle", (e) => {
        if (e.newState !== "open") return;
        picking = false;
        focus = first();
        view = pageOf(focus);
        render(true);
      });
    }
    clearX?.addEventListener("click", () => {
      clear();
      setError(null);
      home();
    });

    // ---- typing a value
    // The locale's numeric date, its order of day, month and year, and a pattern for the
    // placeholder: 15/03/2026, [day, month, year], DD/MM/YYYY.
    const numeric = fmt("numeric", { day: "2-digit", month: "2-digit", year: "numeric" });
    const sample = numeric.formatToParts(D(2026, 11, 23));
    const order = sample.map((p) => p.type).filter((t) => t === "day" || t === "month" || t === "year");
    const pattern = sample.map((p) => ({ day: "DD", month: "MM", year: "YYYY" })[p.type] ?? p.value).join("");
    // Month names in the locale and in English, long and short.
    const monthNames = new Map();
    for (const l of [loc, "en"]) {
      for (const style of ["long", "short"]) {
        const f = new Intl.DateTimeFormat(l, { month: style, timeZone: "UTC" });
        for (let m = 1; m <= 12; m++) monthNames.set(f.format(D(2024, m, 1)).toLowerCase().replace(/\.$/, ""), m);
      }
    }
    // Text the field shows for a value, which reads back as the same value.
    function typed(v) {
      const one = (k) => (kind === "day" ? numeric.format(u.start(k)) : label(k));
      if (mode === "range" && cfg.time) return v.from ? `${one(v.from)} ${timeText(fromTime || "00:00")} – ${v.to ? `${one(v.to)} ${timeText(toTime || "23:59")}` : ""}`.trim() : "";
      if (mode === "range") return v.from ? `${one(v.from)} – ${v.to ? one(v.to) : ""}`.trim() : "";
      if (!v) return "";
      return cfg.time && time ? `${one(v)} ${timeText(time)}` : one(v);
    }
    // A two-digit year is within 20 years ahead of this one, or else in the century before.
    const fullYear = (y, digits) => {
      if (digits > 2) return y;
      const now = Y(today()), base = now - (now % 100);
      return base + y <= now + 20 ? base + y : base - 100 + y;
    };
    const exists = (y, m, d) => m >= 1 && m <= 12 && d >= 1 && Dd(D(y, m, d)) === d && M(D(y, m, d)) === m;
    // One date, month, quarter or year from text; a key, or null when it isn't one.
    function parseOne(text) {
      let s = text.trim().toLowerCase();
      if (!s) return null;
      const rel = s.match(/^(today|tomorrow|yesterday)$/);
      if (rel) return u.key(addDays(today(), { today: 0, tomorrow: 1, yesterday: -1 }[rel[1]]));
      if (/^today\s*[+-]/.test(s)) {
        const d = parseDate(s);
        return d ? u.key(d) : null;
      }
      if (kind === "quarter" || kind === "year") {
        // `Q1 FY2026–27`, `2026-Q1`, `q1 2026`, `FY2026-27`, `2026`.
        const q = s.match(/q\s*([1-4])/);
        const y = s.replace(/q\s*[1-4]/, "").match(/(\d{4})/);
        if (!y) return null;
        if (kind === "year") return String(+y[1]);
        return q ? `${+y[1]}-Q${q[1]}` : null;
      }
      const iso = s.match(/^(\d{4})-(\d{1,2})(?:-(\d{1,2}))?$/);
      const tokens = s.match(/\p{L}+|\d+/gu) ?? [];
      const word = tokens.find((t) => /\p{L}/u.test(t) && monthNames.has(t));
      const nums = tokens.filter((t) => /^\d+$/.test(t));
      let y, m, d;
      if (iso) [y, m, d] = [+iso[1], +iso[2], iso[3] ? +iso[3] : 1];
      else if (word) {
        m = monthNames.get(word);
        // A four-digit number (or one over 31) is the year; the other is the day.
        const yi = nums.findIndex((n) => n.length > 2 || +n > 31);
        y = yi >= 0 ? fullYear(+nums[yi], nums[yi].length) : kind === "month" && nums.length === 1 ? fullYear(+nums[0], nums[0].length) : Y(today());
        const rest = nums.filter((_, i) => i !== yi && !(kind === "month" && nums.length === 1));
        d = kind === "month" ? 1 : rest.length ? +rest[0] : null;
        if (kind === "day" && d === null) return null;
      } else if (kind === "month") {
        // `03/2026`, `2026/03`.
        if (nums.length !== 2) return null;
        const [a, b] = nums;
        [y, m] = a.length > 2 ? [+a, +b] : [fullYear(+b, b.length), +a];
        d = 1;
      } else {
        if (nums.length === 3 && nums[0].length > 2) [y, m, d] = nums.map(Number);
        else if (nums.length === 3) {
          const by = Object.fromEntries(order.map((t, i) => [t, nums[i]]));
          [y, m, d] = [fullYear(+by.year, by.year.length), +by.month, +by.day];
        } else if (nums.length === 2) {
          // Day and month in the locale's order, this year.
          const dm = order.filter((t) => t !== "year");
          const by = Object.fromEntries(dm.map((t, i) => [t, +nums[i]]));
          [y, m, d] = [Y(today()), by.month, by.day];
        } else return null;
      }
      if (!exists(y, m, d)) return null;
      return u.key(D(y, m, d));
    }
    // A time at the end of the text, as `HH:MM`, and the text without it.
    function splitTime(text) {
      const t = text.match(/(\d{1,2})[:.](\d{2})\s*([ap])\.?\s*m\.?\s*$|(\d{1,2})[:.](\d{2})\s*$/i);
      if (!t) return [text, null];
      let hh = +(t[1] ?? t[4]), mm = +(t[2] ?? t[5]);
      const ap = t[3]?.toLowerCase();
      if (ap === "p" && hh < 12) hh += 12;
      if (ap === "a" && hh === 12) hh = 0;
      if (hh > 23 || mm > 59) return [text, "bad"];
      return [text.slice(0, t.index), `${pad(hh)}:${pad(mm)}`];
    }
    // What to type, in error messages: the locale's pattern for days, an example otherwise.
    const example = () => (kind === "day" ? pattern : label(u.key(today())));
    function check(k, lim) {
      if (lim.min && u.end(k) < lim.min) return L("Pick a date on or after {date}", { date: label(u.key(lim.min)) });
      if (lim.max && u.start(k) > lim.max) return L("Pick a date on or before {date}", { date: label(u.key(lim.max)) });
      if (disabled(k, lim)) return L("{date} can't be picked", { date: label(k) });
      return null;
    }
    // The field's text as a value, or an error to show.
    function parseField(text) {
      const lim = limits();
      // Labels: "Enter a date like {example}", "Enter a month like {example}", "Enter a quarter like {example}", "Enter a year like {example}"
      const bad = { error: L(`Enter a ${kind === "day" ? "date" : kind} like {example}`, { example: example() }) };
      if (mode === "range") {
        const parts = text.split(/\s+(?:–|—|-|to|until)\s+|\s*[–—]\s*/i).filter((p) => p.trim());
        if (parts.length !== 2) return { error: L("Enter two dates, like {example} – {example}", { example: example() }) };
        // With `time`, each end may have one: `02/03/2026 09:00 – 06/03/2026 17:30`.
        const split = parts.map((p) => (cfg.time ? splitTime(p.trim()) : [p, null]));
        if (split.some(([, t]) => t === "bad")) return { error: L("Enter a time like 14:30") };
        const keys = split.map(([p]) => parseOne(p));
        if (keys.some((k) => !k)) return bad;
        const sorted = keys.map((k, n) => [k, split[n][1]]).sort(([a, ta], [b, tb]) => u.start(a) - u.start(b) || String(ta).localeCompare(String(tb)));
        const [[from, t1], [to, t2]] = sorted;
        const error = check(from, lim) || check(to, lim);
        return error ? { error } : { value: { from, to }, times: [t1, t2] };
      }
      let rest = text, t = null;
      if (cfg.time) {
        [rest, t] = splitTime(text);
        if (t === "bad") return { error: L("Enter a time like 14:30") };
      }
      const k = parseOne(rest);
      if (!k) return bad;
      const error = check(k, lim);
      return error ? { error } : { value: k, time: t };
    }
    let error = null;
    function setError(msg) {
      if (!field) return;
      if (msg) field.setAttribute("aria-invalid", "true");
      else field.removeAttribute("aria-invalid");
      field.title = msg ?? "";
      if (msg !== error) {
        error = msg;
        if (cfg.bindError) api.set(cfg.bindError, msg);
        if (msg) say(msg);
      }
    }
    function commit() {
      if (!field || !field.__dirty) return;
      const text = field.value.trim();
      // Text that isn't a value stays as typed, to be fixed; the rest is now the value.
      const done = () => (field.__dirty = false);
      if (!text) {
        done();
        setError(null);
        const v = read();
        if (mode === "range" ? v.from : v) clear();
        return;
      }
      const r = parseField(text);
      if (r.error) return setError(r.error);
      done();
      setError(null);
      if (r.time) time = r.time;
      if (r.times) [fromTime, toTime] = [r.times[0] || "", r.times[1] || ""];
      focus = mode === "range" ? r.value.from : r.value;
      view = pageOf(focus);
      write(r.value);
      sync(read());
    }
    if (field) {
      const hm = cfg.time ? " HH:MM" : "";
      field.placeholder ||= mode === "range" ? `${kind === "day" ? pattern + hm : example()} – ${kind === "day" ? pattern + hm : example()}` : kind === "day" ? pattern + hm : example();
      field.addEventListener("input", () => (field.__dirty = true));
      field.addEventListener("change", commit);
      field.addEventListener("blur", commit);
      field.addEventListener("keydown", (e) => {
        if (e.key === "Enter") {
          e.preventDefault();
          commit();
        } else if (e.key === "Escape" && field.__dirty) {
          // Back to the value as it was.
          e.preventDefault();
          field.__dirty = false;
          setError(null);
          sync(read());
        } else if (e.key === "ArrowDown" && pop && !pop.matches(":popover-open")) {
          e.preventDefault();
          commit();
          pop.showPopover();
        }
      });
    }

    // Other components (the data table's date filter) set the value through this.
    const handle = {
      update() {
        readTimes();
        followCompare();
        if (!popover || pop?.matches(":popover-open")) render();
        else sync(read());
      },
      set(v) {
        local = v;
        readTimes();
        focus = first();
        view = pageOf(focus);
        render(false);
      },
    };
    followCompare();
    if (popover) sync(read());
    else render(false);
    return handle;
  }

  B["ui-calendar"] = (el, api) => start(el, api, false);
  B["ui-date-picker"] = (el, api) => start(el, api, true);
})();
