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
    const kind = cfg.picker, mode = cfg.mode, fs = cfg.fiscalStart || 1;
    const U = units(fs), u = U[kind];
    const loc = cfg.locale || undefined;
    const $ = (s) => el.querySelector(s);
    const body = $("[data-cal]"), live = $("[data-cal-live]");
    const trigger = $("[data-cal-trigger]"), pop = $("[data-cal-pop]"), clearX = $("[data-cal-x]");

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
      if ((m = s.match(/^(\d{4})-(\d{2})$/))) return D(+m[1], +m[2], 1);
      if ((m = s.match(/^(\d{4})-Q([1-4])$/i))) return U.quarter.start(`${m[1]}-Q${m[2]}`);
      if ((m = s.match(/^(\d{4})$/))) return U.year.start(m[1]);
      return null;
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
    // The dates a value covers, for `bindRange`.
    function rangeOf(v) {
      if (mode === "range") return v.from ? { from: isoDay(u.start(v.from)), to: v.to ? isoDay(u.end(v.to)) : null } : null;
      if (mode === "multiple") return v.map((k) => ({ from: isoDay(u.start(k)), to: isoDay(u.end(k)) }));
      if (!v) return null;
      if (cfg.time && time) return { from: `${v}T${time}`, to: `${v}T${time}` };
      return { from: isoDay(u.start(v)), to: isoDay(u.end(v)) };
    }
    function write(v) {
      const out = mode === "range" ? (v.from ? { from: v.from, to: v.to } : null) : mode === "multiple" ? v : v && cfg.time ? `${v}T${time || "00:00"}` : v;
      if (cfg.bindRange) api.set(cfg.bindRange, rangeOf(v));
      if (cfg.bind) api.set(cfg.bind, out);
      else {
        local = out;
        render();
      }
      api.emit("change", { value: out, range: rangeOf(v) });
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
        return v.from === v.to ? label(v.from) : `${label(v.from)} – ${label(v.to)}`;
      }
      if (mode === "multiple") return v.length === 0 ? "" : v.length === 1 ? label(v[0]) : `${v.length} selected`;
      if (!v) return "";
      return cfg.time && time ? `${label(v)}, ${timeText(time)}` : label(v);
    }

    // ---- presets: names, or `{ label, from, to }` with dates or `today±N`
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
      return { label: NAMES[p], ...r };
    }
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
      s.append(p);
      return s;
    };
    const uid = el.id || el.querySelector("[id]")?.id || "cal";
    const unitWord = { day: "month", month: "year", quarter: "year", year: "12 years" }[kind];

    function render(keepFocus) {
      const v = read(), lim = limits(), t = u.key(today());
      const focused = keepFocus ?? body.contains(document.activeElement);
      const chosen = (k) => (mode === "range" ? k === v.from || k === v.to : mode === "multiple" ? v.includes(k) : k === v);
      // A range in progress shows up to the cell under the pointer.
      const [a, b] = mode === "range" && v.from ? [v.from, v.to ?? hover ?? v.from].sort((x, y) => u.start(x) - u.start(y)) : [null, null];
      const between = (k) => a && u.start(k) > u.start(a) && u.start(k) < u.start(b);

      const wrap = h("div", { "data-cal-wrap": true });
      if (cfg.presets?.length) {
        const list = h("div", { "data-cal-presets": true, role: "group", "aria-label": "Presets" });
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
        h("button", { type: "button", "data-cal-step": "-1", "aria-label": `Previous ${unitWord}`, disabled: lim.min && pageEnd(pageStep(shown[0], -1)) < lim.min ? true : null }, ARROW("prev")),
      );
      const captions = h("div", { "data-cal-captions": true });
      shown.forEach((p, i) => {
        const id = `${uid}-cap-${i}`;
        if (cfg.dropdowns && kind === "day" && i === 0) {
          const ty = Y(today());
          const lo = lim.min ? Y(lim.min) : ty - 100, hi = lim.max ? Y(lim.max) : ty + 10;
          const month = h("select", { "data-cal-month": true, "aria-label": "Month" });
          for (let m = 1; m <= 12; m++) month.append(h("option", { value: m, selected: m === +p.slice(5, 7) }, fmt("mname", { month: "long" }).format(D(2024, m, 1))));
          const year = h("select", { "data-cal-year": true, "aria-label": "Year" });
          for (let y = hi; y >= lo; y--) year.append(h("option", { value: y, selected: y === +p.slice(0, 4) }, String(y)));
          captions.append(h("div", { "data-cal-caption": true, id }, month, year));
        } else captions.append(h("div", { "data-cal-caption": true, id, "aria-live": i === 0 ? "polite" : null }, caption(p)));
      });
      head.append(captions, h("button", { type: "button", "data-cal-step": "1", "aria-label": `Next ${unitWord}`, disabled: lim.max && pageStart(pageStep(shown.at(-1), 1)) > lim.max ? true : null }, ARROW("next")));
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
              "aria-label": label(k, true),
              "aria-pressed": sel ? "true" : "false",
              "aria-disabled": off ? "true" : null,
              "aria-current": k === t ? "date" : null,
              "data-today": k === t || null,
              "data-outside": outside || null,
              "data-in-range": between(k) || null,
              "data-range-start": a && k === a ? true : null,
              "data-range-end": b && k === b ? true : null,
            }, cellText(k));
            tr.append(h("td", { role: "gridcell", "aria-selected": sel ? "true" : "false" }, btn));
          }
          tbody.append(tr);
        }
        table.append(tbody);
        grids.append(table);
      });
      main.append(grids);

      if (cfg.time) {
        const input = h("input", { type: "time", step: String(Math.round(cfg.step * 60)), value: time, "aria-label": "Time", "data-cal-time-input": true });
        main.append(h("label", { "data-cal-time": true }, "Time", input));
      }
      const foot = h("div", { "data-cal-foot": true });
      if (cfg.clearable && !popover) foot.append(h("button", { type: "button", "data-cal-clear": true }, "Clear"));
      main.append(foot);
      wrap.append(main);
      body.replaceChildren(wrap);
      if (focused) body.querySelector(`[data-key="${focus}"]`)?.focus({ preventScroll: true });
      sync(v);
    }

    // The trigger's text, the clear button and the hidden inputs.
    function sync(v) {
      const text = display(v);
      if (trigger) {
        const span = trigger.querySelector("[data-cal-text]");
        span.textContent = text || span.getAttribute("data-placeholder");
        el.toggleAttribute("data-empty", !text);
      }
      if (clearX) clearX.hidden = !text;
      const hidden = $("[data-cal-hidden]");
      if (hidden && cfg.name) {
        const inputs = mode === "range" ? [[`${cfg.name}[from]`, v.from ?? ""], [`${cfg.name}[to]`, v.to ?? ""]]
          : mode === "multiple" ? v.map((k) => [cfg.name, k])
          : [[cfg.name, v ? (cfg.time ? `${v}T${time || "00:00"}` : v) : ""]];
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
        write(k);
        say(`${label(k, true)} selected`);
        if (popover && cfg.closeOnSelect) close();
      } else if (mode === "multiple") {
        write(v.includes(k) ? v.filter((x) => x !== k) : [...v, k].sort((x, y) => u.start(x) - u.start(y)));
      } else if (!v.from || v.to) {
        hover = null;
        write({ from: k, to: null });
        say(`Range starts ${label(k, true)}; pick the end`);
      } else {
        const [from, to] = [v.from, k].sort((x, y) => u.start(x) - u.start(y));
        hover = null;
        write({ from, to });
        say(`${label(from, true)} to ${label(to, true)} selected`);
        if (popover && cfg.closeOnSelect) close();
      }
      render(true);
    }
    function clear() {
      hover = null;
      time = "";
      write(mode === "range" ? { from: null, to: null } : mode === "multiple" ? [] : null);
      say("Cleared");
    }

    // ---- the keyboard, on the grid's buttons
    function move(k) {
      focus = k;
      reveal(k);
      if (mode === "range" && read().from && !read().to) hover = k;
      render(true);
    }
    body.addEventListener("keydown", (e) => {
      const k = e.target.getAttribute?.("data-key");
      if (!k) return;
      const d = u.start(k);
      let next = null;
      switch (e.key) {
        case "ArrowLeft": next = u.step(k, -1); break;
        case "ArrowRight": next = u.step(k, 1); break;
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
    });
    body.addEventListener("change", (e) => {
      const t = e.target;
      if (t.matches("[data-cal-month], [data-cal-year]")) {
        const y = body.querySelector("[data-cal-year]").value, m = body.querySelector("[data-cal-month]").value;
        view = `${y}-${pad(m)}`;
        focus = u.key(D(+y, +m, Math.min(Dd(u.start(focus)), Dd(D(+y, +m + 1, 0)))));
        render(false);
      } else if (t.matches("[data-cal-time-input]")) {
        time = t.value;
        const v = read();
        if (v) write(v);
        else write(focus);
      }
    });
    // A range in progress follows the pointer.
    body.addEventListener("mouseover", (e) => {
      const k = e.target.closest?.("[data-key]")?.getAttribute("data-key");
      if (!k || mode !== "range") return;
      const v = read();
      if (!v.from || v.to || hover === k) return;
      hover = k;
      render(false);
    });

    // ---- the popover
    function close() {
      if (!pop) return;
      try {
        pop.hidePopover();
      } catch {}
      trigger?.focus();
    }
    if (pop) {
      pop.addEventListener("toggle", (e) => {
        if (e.newState !== "open") return;
        focus = first();
        view = pageOf(focus);
        render(true);
      });
    }
    clearX?.addEventListener("click", () => {
      clear();
      trigger?.focus();
    });

    // Other components (the data table's date filter) set the value through this.
    const handle = {
      update() {
        const t = timeOf(raw());
        if (t) time = t;
        if (!popover || pop?.matches(":popover-open")) render();
        else sync(read());
      },
      set(v) {
        local = v;
        time = timeOf(v) || time;
        focus = first();
        view = pageOf(focus);
        render(false);
      },
    };
    if (popover) sync(read());
    else render(false);
    return handle;
  }

  B["ui-calendar"] = (el, api) => start(el, api, false);
  B["ui-date-picker"] = (el, api) => start(el, api, true);
})();
