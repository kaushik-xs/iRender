// irender-ui-kit: the `ui-event-calendar` behaviour. Draws `events` in a month grid, a
// week of hours, or a list by day, and moves through time with the toolbar or the keys.
// Events: { id, title, start, end, allDay, category, tone, description }; `start` and
// `end` are dates ("2026-11-04") or local times ("2026-11-04T09:30"). A click on an
// event writes it to `bindEvent` (and fires `open`); on a day, its date to `bindDay`
// (and fires `day`), for adding one. With `bindEvents`, events move: dragged to another
// time or day (or resized by a timed one's foot), or with Alt and the arrow keys; the
// list goes back to state with their new dates, written as they were. `days` marks days
// apart from events: a shade in a tone, a label beside the date, badges under it.
(window.__irenderBehaviors ||= {})["ui-event-calendar"] = (el, api) => {
  const cfg = api.config;
  const $ = (s, from = el) => from.querySelector(s);
  const $$ = (s, from = el) => [...from.querySelectorAll(s)];
  const body = $("[data-ev-body]");
  const live = $("[data-ev-live]");
  const isObj = (v) => v !== null && typeof v === "object" && !Array.isArray(v);
  // Interface text in the document's words (its `labels`), by the English.
  const L = (text, vars) => window.__irender.label(text, vars);
  // Right to left, Left and Right swap: the next one is on the left.
  const side = (k) => (getComputedStyle(el).direction === "rtl" ? { ArrowLeft: "ArrowRight", ArrowRight: "ArrowLeft" }[k] ?? k : k);

  const resolve = (v) => (isObj(v) && "$irender" in v && Object.keys(v).length === 1 ? window.__irender.evaluate(v.$irender, api.state) : v);
  const say = (text) => {
    live.textContent = "";
    setTimeout(() => (live.textContent = text), 30);
  };
  const picked = () => (cfg.bindDay ? api.state[cfg.bindDay] ?? null : null);
  const locale = cfg.locale || undefined;
  const weekStart = cfg.weekStart ?? 1;
  const [hourFrom, hourTo] = cfg.hours ?? [7, 20];

  // ---- dates, as local days
  const pad = (n) => String(n).padStart(2, "0");
  const iso = (d) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
  const parse = (s) => {
    const m = /^(\d{4})-(\d{2})-(\d{2})(?:[T ](\d{2}):(\d{2}))?/.exec(String(s ?? ""));
    return m ? new Date(+m[1], +m[2] - 1, +m[3], +(m[4] ?? 0), +(m[5] ?? 0)) : null;
  };
  const day = (d) => new Date(d.getFullYear(), d.getMonth(), d.getDate());
  const addDays = (d, n) => new Date(d.getFullYear(), d.getMonth(), d.getDate() + n);
  const addMonths = (d, n) => new Date(d.getFullYear(), d.getMonth() + n, Math.min(d.getDate(), 28));
  const startOfWeek = (d) => addDays(day(d), -((d.getDay() - weekStart + 7) % 7));
  const sameDay = (a, b) => a && b && iso(a) === iso(b);
  const today = () => (cfg.today ? parse(cfg.today) : day(new Date()));
  const fmt = (o) => new Intl.DateTimeFormat(locale, o);
  const time = (d) => fmt({ hour: "numeric", minute: "2-digit" }).format(d);
  const timed = (s) => /T\d{2}:\d{2}/.test(String(s ?? ""));

  // ---- events, with their days and colour
  function events() {
    const list = resolve(cfg.events);
    // One that repeats is each of its days near what's shown.
    const near = (e, i) => {
      const at = e?.repeat ? parse(e.start) : null;
      if (!at) return [[e, i, null]];
      const first = day(at), off = Math.round((day(cursor) - first) / 864e5);
      return window.__irenderKit.repeats(e.repeat, first, off - 400, off + 400).map((n) => {
        const go = (s) => {
          const d = parse(s);
          if (!d) return s;
          const m = new Date(d.getFullYear(), d.getMonth(), d.getDate() + n, d.getHours(), d.getMinutes());
          return timed(s) ? `${iso(m)}T${pad(m.getHours())}:${pad(m.getMinutes())}` : iso(m);
        };
        return [{ ...e, start: go(e.start), end: e.end ? go(e.end) : e.end }, i, iso(addDays(first, n))];
      });
    };
    return (Array.isArray(list) ? list : [])
      .flatMap(near)
      .map(([e, i, date]) => {
        const start = parse(e.start);
        if (!start) return null;
        let end = parse(e.end) ?? start;
        const allDay = e.allDay ?? !timed(e.start);
        // An all-day event's end date is its last day; a timed one ending at midnight ends the day before.
        const lastDay = allDay ? day(end) : end.getHours() || end.getMinutes() || sameDay(start, end) ? day(end) : addDays(day(end), -1);
        if (end < start) end = start;
        const cats = resolve(cfg.categories) ?? {};
        const tone = e.tone ?? (isObj(cats) ? cats[e.category] : null) ?? "primary";
        return { ...e, key: date ? `${e.id ?? i}@${date}` : String(e.id ?? i), series: date ? String(e.id ?? i) : null, occurrence: date, startAt: start, endAt: end, first: day(start), last: lastDay < day(start) ? day(start) : lastDay, allDay, tone };
      })
      .filter(Boolean)
      .sort((a, b) => a.startAt - b.startAt || (b.allDay ? 1 : 0) - (a.allDay ? 1 : 0) || String(a.title).localeCompare(String(b.title)));
  }
  const on = (list, d) => list.filter((e) => e.first <= d && d <= e.last);

  // ---- marked days: date → { tone, labels, items }, from `[{ date, end, tone, label, items }]`
  // or a map of date → `{ tone, label, items }`. Two on one day share it; the later tone wins.
  function marks() {
    const v = resolve(cfg.days);
    const list = Array.isArray(v) ? v : isObj(v) ? Object.entries(v).map(([date, m]) => ({ ...(isObj(m) ? m : { label: m }), date })) : [];
    const out = new Map();
    for (const m of list) {
      const from = isObj(m) ? parse(m.date) : null;
      if (!from) continue;
      const to = parse(m.end) ?? from, long = Math.round((day(to) - day(from)) / 864e5);
      // One that repeats is each of its days (and runs) near what's shown.
      const first = day(from), off = Math.round((day(cursor) - first) / 864e5);
      const starts = m.repeat ? window.__irenderKit.repeats(m.repeat, first, off - 400, off + 400).map((n) => addDays(first, n)) : [first];
      for (const s of starts) for (let d = s, n = 0; n <= long && n < 400; d = addDays(d, 1), n++) {
        const at = out.get(iso(d)) ?? { tone: null, labels: [], items: [] };
        if (m.tone) at.tone = m.tone;
        if (m.label != null && m.label !== "") at.labels.push(String(m.label));
        for (const it of Array.isArray(m.items) ? m.items : m.items != null ? [m.items] : []) at.items.push(isObj(it) ? { text: String(it.text ?? it.label ?? ""), tone: it.tone ?? null, id: it.id ?? null, item: it } : { text: String(it), tone: null, id: null });
        out.set(iso(d), at);
      }
    }
    return out;
  }
  // A mark's words for a day's name: "No shifts, Morning, Night".
  const said = (m) => (m ? [...m.labels, ...m.items.map((i) => i.text)].filter(Boolean).map((t) => `, ${t}`).join("") : "");
  // Its label beside the date (or the date alone), and its badges.
  function dated(m, date) {
    if (!m?.labels.length) return [date];
    const head = make("div", { "data-ev-day-head": "" });
    head.append(date, make("span", { "data-ev-note": "", "aria-hidden": "true", title: m.labels.join(", ") }, m.labels.join(", ")));
    return [head];
  }
  // A badge with an `id` is a button, for a pointer: the day's name already says it, and
  // the keys move between days.
  function badges(m) {
    if (!m?.items.length) return [];
    const live = m.items.some((it) => it.id != null);
    const box = make("div", { "data-ev-badges": "", "aria-hidden": live ? null : "true" });
    for (const it of m.items) {
      box.append(it.id != null
        ? make("button", { type: "button", tabindex: -1, "data-ev-badge": "", "data-ev-badge-id": String(it.id), "data-tone": it.tone, title: it.text }, it.text)
        : make("span", { "data-ev-badge": "", "data-tone": it.tone, title: it.text, "aria-hidden": live ? "true" : null }, it.text));
    }
    return [box];
  }

  // ---- where we are
  let view = cfg.view ?? "month";
  let cursor = parse(resolve(cfg.date)) ?? today(); // the day keyboard focus is on, and what's shown around it
  const make = (tag, attrs = {}, text) => {
    const e = document.createElement(tag);
    for (const [k, v] of Object.entries(attrs)) if (v !== null && v !== undefined && v !== false) e.setAttribute(k, v === true ? "" : v);
    if (text !== undefined) e.textContent = text;
    return e;
  };
  const chip = (e, d) => {
    const b = make("button", { type: "button", "data-ev-event": e.key, "data-tone": e.tone, tabindex: -1 });
    const continues = e.first < d, goesOn = e.last > d;
    if (continues) b.setAttribute("data-from-before", "");
    if (goesOn) b.setAttribute("data-on-after", "");
    const when = e.allDay ? L("all day") : `${time(e.startAt)}${e.endAt > e.startAt ? `–${time(e.endAt)}` : ""}`;
    b.setAttribute("aria-label", `${e.title}, ${when}${e.category ? `, ${e.category}` : ""}`);
    if (!e.allDay && !continues) b.append(make("time", {}, time(e.startAt)));
    b.append(make("span", {}, String(e.title ?? "")));
    return b;
  };

  function title() {
    if (view === "day") return fmt({ weekday: "long", day: "numeric", month: "long", year: "numeric" }).format(cursor);
    if (view === "year") return String(cursor.getFullYear());
    if (view === "week") {
      // As the locale writes a range: "2–8 November 2026", "November 2 – 8, 2026".
      const a = startOfWeek(cursor), b = addDays(a, 6);
      const f = fmt({ day: "numeric", month: a.getMonth() === b.getMonth() ? "long" : "short", year: "numeric" });
      return f.formatRange ? f.formatRange(a, b) : `${f.format(a)} – ${f.format(b)}`;
    }
    return fmt({ month: "long", year: "numeric" }).format(cursor);
  }

  function month(list, marked) {
    const first = new Date(cursor.getFullYear(), cursor.getMonth(), 1);
    const start = startOfWeek(first);
    const weeks = Math.ceil(((first - start) / 864e5 + new Date(cursor.getFullYear(), cursor.getMonth() + 1, 0).getDate()) / 7);
    const grid = make("div", { role: "grid", "data-ev-month": "", "aria-label": title() });
    const head = make("div", { role: "row", "data-ev-weekdays": "" });
    for (let i = 0; i < 7; i++) {
      const d = addDays(start, i);
      head.append(make("div", { role: "columnheader", "aria-label": fmt({ weekday: "long" }).format(d) }, fmt({ weekday: "short" }).format(d)));
    }
    grid.append(head);
    const max = cfg.maxPerDay ?? 3;
    for (let w = 0; w < weeks; w++) {
      const row = make("div", { role: "row", "data-ev-week": "" });
      for (let i = 0; i < 7; i++) {
        const d = addDays(start, w * 7 + i), here = on(list, d), m = marked.get(iso(d));
        const cell = make("div", {
          role: "gridcell",
          "data-ev-day": iso(d),
          tabindex: sameDay(d, cursor) ? 0 : -1,
          "data-shade": !!m?.tone,
          "data-tone": m?.tone ?? null,
          "data-outside": d.getMonth() !== cursor.getMonth(),
          "data-today": sameDay(d, today()),
          "data-picked": iso(d) === picked(),
          "aria-selected": iso(d) === picked() ? "true" : "false",
          "aria-label": `${fmt({ weekday: "long", day: "numeric", month: "long" }).format(d)}${said(m)}${here.length ? `, ${L(["{n} event", "{n} events"], { n: here.length })}` : ""}`,
        });
        cell.append(...dated(m, make("span", { "data-ev-date": "", "aria-hidden": "true" }, String(d.getDate()))), ...badges(m));
        const chips = make("div", { "data-ev-chips": "" });
        here.slice(0, here.length > max ? max - 1 : max).forEach((e) => chips.append(chip(e, d)));
        if (here.length > max) {
          chips.append(make("button", { type: "button", "data-ev-more": iso(d), tabindex: -1, "aria-label": L("{n} more on {date}", { n: here.length - max + 1, date: fmt({ day: "numeric", month: "long" }).format(d) }) }, L("+{n} more", { n: here.length - max + 1 })));
        }
        cell.append(chips);
        row.append(cell);
      }
      grid.append(row);
    }
    return grid;
  }

  // The week's days, or (the day view) just the one: hours down, days across.
  function week(list, marked, days = Array.from({ length: 7 }, (_, i) => addDays(startOfWeek(cursor), i))) {
    const hours = Math.max(1, hourTo - hourFrom);
    const wrap = make("div", { "data-ev-weekview": "", style: `--hours: ${hours}; --days: ${days.length}` });
    const head = make("div", { "data-ev-week-head": "" });
    head.append(make("div", { "aria-hidden": "true" }));
    for (const d of days) {
      const h = make("button", { type: "button", "data-ev-day": iso(d), "data-today": sameDay(d, today()), tabindex: sameDay(d, cursor) ? 0 : -1, "aria-label": fmt({ weekday: "long", day: "numeric", month: "long" }).format(d) + said(marked.get(iso(d))) });
      h.append(make("span", {}, fmt({ weekday: "short" }).format(d)), make("b", {}, String(d.getDate())));
      head.append(h);
    }
    wrap.append(head);
    // All day, and events over more than a day.
    const allDay = make("div", { "data-ev-allday": "" });
    allDay.append(make("div", { "data-ev-hour": "" }, L("All day")));
    for (const d of days) {
      const m = marked.get(iso(d));
      const cell = make("div", { "data-ev-allday-cell": "", "data-ev-cell-date": iso(d), "data-shade": !!m?.tone, "data-tone": m?.tone ?? null });
      if (m?.labels.length) cell.append(make("span", { "data-ev-note": "", "aria-hidden": "true" }, m.labels.join(", ")));
      cell.append(...badges(m));
      on(list, d).filter((e) => e.allDay || e.first < e.last).forEach((e) => cell.append(chip(e, d)));
      allDay.append(cell);
    }
    wrap.append(allDay);
    const grid = make("div", { "data-ev-hours": "" });
    const labels = make("div", { "data-ev-labels": "", "aria-hidden": "true" });
    for (let h = hourFrom; h < hourTo; h++) labels.append(make("div", { "data-ev-hour": "" }, time(new Date(2000, 0, 1, h))));
    grid.append(labels);
    for (const d of days) {
      const tone = marked.get(iso(d))?.tone;
      const col = make("div", { "data-ev-col": "", "data-ev-date-col": iso(d), "data-today": sameDay(d, today()), "data-shade": !!tone, "data-tone": tone ?? null });
      // Timed events on the day, laid side by side where they overlap.
      const items = on(list, d).filter((e) => !e.allDay && e.first.getTime() === e.last.getTime());
      // Events that overlap, directly or through others, share the width in lanes; the
      // rest get the whole column.
      const endOf = (e) => (e.endAt > e.startAt ? e.endAt : new Date(e.startAt.getTime() + 30 * 60000));
      let group = [], lanes = [], groupEnd = null;
      const close = () => group.forEach((x) => (x.lanes = lanes.length));
      for (const e of items) {
        if (groupEnd && e.startAt >= groupEnd) close(), (group = []), (lanes = []), (groupEnd = null);
        let lane = lanes.findIndex((end) => end <= e.startAt);
        if (lane < 0) lane = lanes.push(0) - 1;
        lanes[lane] = endOf(e);
        e.lane = lane;
        group.push(e);
        groupEnd = !groupEnd || endOf(e) > groupEnd ? endOf(e) : groupEnd;
      }
      close();
      for (const e of items) {
        const top = (e.startAt.getHours() + e.startAt.getMinutes() / 60 - hourFrom) / hours;
        const end = e.endAt > e.startAt ? e.endAt : new Date(e.startAt.getTime() + 30 * 60000);
        const len = (end - e.startAt) / 36e5 / hours;
        const c = chip(e, d);
        c.setAttribute("data-ev-timed", "");
        if (cfg.bindEvents) c.append(make("span", { "data-ev-resize": "", "aria-hidden": "true" }));
        // Under 45 minutes: time and title on one line, so both fit.
        if ((end - e.startAt) / 6e4 < 45) c.setAttribute("data-short", "");
        const pc = (n) => `${+(n * 100).toFixed(3)}%`;
        c.style.cssText = `top: ${pc(Math.max(0, top))}; height: max(22px, ${pc(len)}); left: calc(${pc(e.lane / e.lanes)} + 2px); width: calc(${pc(1 / e.lanes)} - 4px)`;
        col.append(c);
      }
      if (sameDay(d, today()) && !cfg.today) {
        const now = new Date(), at = (now.getHours() + now.getMinutes() / 60 - hourFrom) / hours;
        if (at >= 0 && at <= 1) col.append(make("div", { "data-ev-now": "", style: `top: ${at * 100}%`, "aria-hidden": "true" }));
      }
      grid.append(col);
    }
    wrap.append(grid);
    return wrap;
  }

  // An event's time on day `d`: its hours, "All day", or for one running on from the day
  // before, "Until" its end on its last day.
  function when(e, d) {
    if (e.allDay) return L("All day");
    if (e.first < d) return sameDay(e.last, d) ? L("Until {time}", { time: time(e.endAt) }) : L("All day");
    return `${time(e.startAt)}${e.endAt > e.startAt ? ` – ${time(e.endAt)}` : ""}`;
  }

  // The year: twelve small months, the days with events marked in their colour.
  // With `days`, a day marked with a tone is tinted in it; `yearEvents: false` leaves only those.
  function year(list, marked) {
    const y = cursor.getFullYear();
    const wrap = make("div", { "data-ev-year": "" });
    const busy = new Map();
    if (cfg.yearEvents !== false) for (const e of list) for (let d = e.first; d <= e.last; d = addDays(d, 1)) if (d.getFullYear() === y) busy.set(iso(d), [...(busy.get(iso(d)) ?? []), e]);
    for (let m = 0; m < 12; m++) {
      const first = new Date(y, m, 1), start = startOfWeek(first), days = new Date(y, m + 1, 0).getDate();
      const box = make("section", { "data-ev-mini": "" });
      box.append(make("button", { type: "button", "data-ev-month-open": `${y}-${pad(m + 1)}`, "aria-label": `Open ${fmt({ month: "long", year: "numeric" }).format(first)}` }, fmt({ month: "long" }).format(first)));
      const grid = make("div", { role: "grid", "aria-label": fmt({ month: "long", year: "numeric" }).format(first) });
      const head = make("div", { role: "row", "data-ev-mini-head": "" });
      for (let i = 0; i < 7; i++) head.append(make("span", { role: "columnheader", "aria-label": fmt({ weekday: "long" }).format(addDays(start, i)) }, fmt({ weekday: "narrow" }).format(addDays(start, i))));
      grid.append(head);
      const weeks = Math.ceil(((first - start) / 864e5 + days) / 7);
      for (let w = 0; w < weeks; w++) {
        const row = make("div", { role: "row" });
        for (let i = 0; i < 7; i++) {
          const d = addDays(start, w * 7 + i), here = busy.get(iso(d)) ?? [], mark = marked.get(iso(d));
          if (d.getMonth() !== m) {
            row.append(make("span", { role: "gridcell", "aria-hidden": "true" }));
            continue;
          }
          const cell = make("span", {
            role: "gridcell",
            "data-ev-day": iso(d),
            tabindex: sameDay(d, cursor) ? 0 : -1,
            "data-today": sameDay(d, today()),
            "data-busy": here.length > 0,
            "data-shade": !!mark?.tone,
            "data-tone": mark?.tone ?? here[0]?.tone ?? null,
            title: mark?.labels.length ? mark.labels.join(", ") : null,
            "aria-label": `${fmt({ weekday: "long", day: "numeric", month: "long" }).format(d)}${said(mark)}${here.length ? `, ${L(["{n} event", "{n} events"], { n: here.length })}` : ""}`,
          }, String(d.getDate()));
          row.append(cell);
        }
        grid.append(row);
      }
      box.append(grid);
      wrap.append(box);
    }
    return wrap;
  }

  function agenda(list, marked) {
    // The month around the cursor, day by day, only days with events or marks.
    const first = new Date(cursor.getFullYear(), cursor.getMonth(), 1), last = new Date(cursor.getFullYear(), cursor.getMonth() + 1, 0);
    const box = make("div", { "data-ev-list": "" });
    for (let d = first; d <= last; d = addDays(d, 1)) {
      const here = on(list, d), m = marked.get(iso(d));
      if (!here.length && !m) continue;
      const group = make("section", { "data-ev-group": iso(d), "data-today": sameDay(d, today()), "data-tone": m?.tone ?? null, "aria-label": fmt({ weekday: "long", day: "numeric", month: "long" }).format(d) + said(m) });
      const h = make("h4", {});
      h.append(make("b", {}, String(d.getDate())), make("span", {}, fmt({ weekday: "long", month: "short" }).format(d)));
      if (m?.labels.length) h.append(make("span", { "data-ev-note": "" }, m.labels.join(", ")));
      const side = make("div", {});
      side.append(h, ...badges(m));
      group.append(side);
      const ul = make("ul", {});
      for (const e of here) {
        const li = make("li", {});
        const b = chip(e, d);
        b.tabIndex = 0;
        b.replaceChildren(
          make("time", {}, when(e, d)),
          make("span", { "data-ev-dot": "", "aria-hidden": "true" }),
          make("span", { "data-ev-name": "" }, String(e.title ?? "")),
        );
        if (e.category) b.append(make("small", {}, String(e.category)));
        li.append(b);
        ul.append(li);
      }
      group.append(ul);
      box.append(group);
    }
    if (!box.children.length) box.append(make("p", { "data-ev-empty": "" }, cfg.emptyText ?? L("Nothing in {month}.", { month: fmt({ month: "long" }).format(cursor) })));
    return box;
  }

  function draw(focus) {
    const list = events();
    $("[data-ev-title]").textContent = title();
    for (const b of $$("[data-ev-view]")) b.setAttribute("aria-pressed", b.getAttribute("data-ev-view") === view ? "true" : "false");
    body.setAttribute("data-view", view);
    const marked = marks();
    body.replaceChildren(view === "week" ? week(list, marked) : view === "day" ? week(list, marked, [cursor]) : view === "year" ? year(list, marked) : view === "list" ? agenda(list, marked) : month(list, marked));
    // The focused day's events join the tab order after it.
    const at = iso(cursor);
    for (const c of $$(`[data-ev-day="${at}"] :is([data-ev-event], [data-ev-more]), :is([data-ev-date-col="${at}"], [data-ev-cell-date="${at}"]) [data-ev-event]`)) c.tabIndex = 0;
    if (focus) $(`[data-ev-day="${iso(cursor)}"]`)?.focus();
    const legend = $("[data-ev-legend]");
    if (legend) {
      const cats = resolve(cfg.categories);
      legend.replaceChildren(...(isObj(cats) ? Object.entries(cats) : []).map(([name, tone]) => {
        const s = make("span", { "data-tone": tone });
        s.append(make("i", { "aria-hidden": "true" }), make("span", {}, name));
        return s;
      }));
    }
  }

  // ---- moving
  function switchTo(next, focus) {
    view = next;
    if (cfg.bindView) api.set(cfg.bindView, view);
    draw(focus);
    // Labels: "Day view, {title}", "Week view, {title}", "Month view, {title}", "Year view, {title}", "List view, {title}"
    say(L(`${next[0].toUpperCase() + next.slice(1)} view, {title}`, { title: title() }));
  }
  // What's on screen, to tell whether a move changed it.
  const page = () => (view === "week" ? iso(startOfWeek(cursor)) : view === "day" ? iso(cursor) : view === "year" ? String(cursor.getFullYear()) : `${cursor.getFullYear()}-${cursor.getMonth()}`);
  function go(to, focus = false) {
    const before = page();
    cursor = day(to);
    const after = page();
    draw(focus);
    if (before !== after) say(title());
    api.emit("navigate", { date: iso(cursor), view });
  }
  function step(n) {
    go(view === "week" ? addDays(cursor, 7 * n) : view === "day" ? addDays(cursor, n) : view === "year" ? addMonths(cursor, 12 * n) : addMonths(cursor, n));
  }
  function pick(d) {
    cursor = day(d);
    if (cfg.bindDay) api.set(cfg.bindDay, iso(cursor));
    api.emit("day", { date: iso(cursor) });
    draw(true);
  }
  function open(key) {
    const e = events().find((x) => x.key === key);
    if (!e) return;
    // A day of one that repeats: the event, with that day's own times.
    const whole = (resolve(cfg.events) ?? []).find((x, i) => String(x.id ?? i) === (e.series ?? key)) ?? e;
    const original = e.series ? { ...whole, start: e.start, end: e.end, occurrence: e.occurrence } : whole;
    if (cfg.bindEvent) api.set(cfg.bindEvent, original);
    api.emit("open", { event: original });
  }

  el.addEventListener("click", (e) => {
    const t = e.target;
    // A badge's own click, not its day's.
    const badge = t.closest("[data-ev-badge-id]");
    if (badge) {
      const date = badge.closest("[data-ev-day]")?.getAttribute("data-ev-day") ?? badge.closest("[data-ev-group]")?.getAttribute("data-ev-group") ?? badge.closest("[data-ev-cell-date]")?.getAttribute("data-ev-cell-date") ?? null;
      const it = marks().get(date)?.items.find((x) => String(x.id) === badge.getAttribute("data-ev-badge-id"));
      const hit = { date, item: it?.item ?? { id: badge.getAttribute("data-ev-badge-id"), text: badge.textContent } };
      if (cfg.bindBadge) api.set(cfg.bindBadge, hit);
      return api.emit("badge", hit);
    }
    if (t.closest("[data-ev-prev]")) return step(-1);
    if (t.closest("[data-ev-next]")) return step(1);
    if (t.closest("[data-ev-today]")) return go(today(), false);
    const v = t.closest("[data-ev-view]");
    if (v) return switchTo(v.getAttribute("data-ev-view"), false);
    const ev = t.closest("[data-ev-event]");
    if (ev) return open(ev.getAttribute("data-ev-event"));
    const monthOpen = t.closest("[data-ev-month-open]");
    if (monthOpen) {
      cursor = parse(monthOpen.getAttribute("data-ev-month-open") + "-01");
      return switchTo("month", true);
    }
    const more = t.closest("[data-ev-more]");
    if (more) {
      cursor = parse(more.getAttribute("data-ev-more"));
      view = "week";
      return draw(true);
    }
    const d = t.closest("[data-ev-day]");
    if (d && body.contains(d)) {
      pick(parse(d.getAttribute("data-ev-day")));
      // In the year, a day opens on its own.
      if (view === "year" && cfg.views.includes("day")) switchTo("day", true);
    }
  });
  body.addEventListener("keydown", (e) => {
    const d = e.target.closest?.("[data-ev-day]");
    if (!d || e.target !== d) return;
    // From the day that has focus (a click or Tab may have moved it).
    cursor = parse(d.getAttribute("data-ev-day"));
    const across = view === "week" || view === "day";
    const moves = { ArrowLeft: -1, ArrowRight: 1, ArrowUp: across ? 0 : -7, ArrowDown: across ? 0 : 7 };
    const k = side(e.key);
    if (k in moves && moves[k]) return e.preventDefault(), go(addDays(cursor, moves[k]), true);
    if (e.key === "Home") return e.preventDefault(), go(startOfWeek(cursor), true);
    if (e.key === "End") return e.preventDefault(), go(addDays(startOfWeek(cursor), 6), true);
    if (e.key === "PageUp" || e.key === "PageDown") return e.preventDefault(), go(view === "week" ? addDays(cursor, e.key === "PageUp" ? -7 : 7) : addMonths(cursor, e.key === "PageUp" ? -1 : 1), true);
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      pick(cursor);
      if (view === "year" && cfg.views.includes("day")) switchTo("day", true);
    }
  });

  // ---- moving events
  const snap = cfg.snap ?? 15;
  const MIN = 6e4, DAY = 864e5;
  const stamp = (d, allDay) => (allDay ? iso(d) : `${iso(d)}T${pad(d.getHours())}:${pad(d.getMinutes())}`);
  // Moves event `key` by `days` and `minutes`, and its end by `longer` minutes more;
  // writes the list back. Days are counted on the calendar, so a move over a clock change
  // keeps the time of day.
  function move(key, { days = 0, minutes = 0, longer = 0 }) {
    const e = events().find((x) => x.key === key);
    if (!e || !(days || minutes || longer)) return null;
    const at = (d, n) => new Date(d.getFullYear(), d.getMonth(), d.getDate() + days, d.getHours(), d.getMinutes() + n);
    const start = at(e.startAt, minutes);
    const shown = e.endAt > e.startAt ? e.endAt : new Date(e.startAt.getTime() + 30 * MIN);
    let end = at(shown, minutes + longer);
    if (!e.allDay && end - start < snap * MIN) end = new Date(start.getTime() + snap * MIN);
    const list = resolve(cfg.events) ?? [];
    let moved = null;
    const next = list.flatMap((x, i) => {
      // One day of an event that repeats becomes an event of its own, and the series
      // leaves that day out.
      if (e.series && String(x.id ?? i) === e.series) {
        const { repeat, ...rest } = x, rule = isObj(repeat) ? repeat : { every: repeat };
        moved = { ...rest, id: window.__irenderKit.id(), start: stamp(start, e.allDay) };
        if (x.end || longer) moved.end = stamp(e.allDay ? at(e.endAt, 0) : end, e.allDay);
        return [{ ...x, repeat: { ...rule, except: [...(Array.isArray(rule.except) ? rule.except : []), e.occurrence] } }, moved];
      }
      if (String(x.id ?? i) !== key) return x;
      moved = { ...x, start: stamp(start, e.allDay) };
      // An end it had moves with it; one it didn't stays unwritten unless it was resized.
      if (x.end !== undefined && x.end !== null && x.end !== "") moved.end = stamp(e.allDay ? at(e.endAt, 0) : end, e.allDay);
      else if (longer) moved.end = stamp(end, false);
      return moved;
    });
    if (!moved) return null;
    api.set(cfg.bindEvents, next);
    api.emit("move", { event: moved, from: list.find((x, i) => String(x.id ?? i) === (e.series ?? key)) });
    cursor = day(start);
    return moved;
  }
  const tell = (m) => say(`${L("{title} moved to {date}", { title: String(m.title), date: fmt({ weekday: "long", day: "numeric", month: "long" }).format(parse(m.start)) })}${timed(m.start) ? `, ${time(parse(m.start))}` : ""}${m.end && timed(m.end) ? ` ${L("to {time}", { time: time(parse(m.end)) })}` : ""}`);
  const refocus = (key) => {
    draw(false);
    const c = $(`[data-ev-event="${CSS.escape(key)}"]`);
    if (c) (c.tabIndex = 0), c.focus();
  };

  // Dragging. A press becomes a drag after a few pixels, so a click still opens the event.
  let drag = null;
  const dateAt = (x, y) => {
    const t = document.elementFromPoint(x, y)?.closest("[data-ev-date-col], [data-ev-cell-date], [data-ev-day]");
    return t && body.contains(t) ? t.getAttribute("data-ev-date-col") || t.getAttribute("data-ev-cell-date") || t.getAttribute("data-ev-day") : null;
  };
  body.addEventListener("pointerdown", (e) => {
    if (!cfg.bindEvents || e.button !== 0 || view === "list" || view === "year") return;
    const chipEl = e.target.closest("[data-ev-event]");
    if (!chipEl) return;
    const ev = events().find((x) => x.key === chipEl.getAttribute("data-ev-event"));
    if (!ev) return;
    const col = chipEl.closest("[data-ev-date-col]");
    drag = {
      chip: chipEl, ev, x: e.clientX, y: e.clientY, id: e.pointerId, on: false,
      resize: !!e.target.closest("[data-ev-resize]"),
      timed: !!col,
      from: col?.getAttribute("data-ev-date-col") ?? chipEl.closest("[data-ev-cell-date], [data-ev-day]")?.getAttribute("data-ev-cell-date") ?? chipEl.closest("[data-ev-day]")?.getAttribute("data-ev-day"),
      pxPerMin: col ? col.getBoundingClientRect().height / ((hourTo - hourFrom) * 60) : 0,
      top: chipEl.style.top, height: chipEl.style.height, parent: chipEl.parentElement, next: chipEl.nextSibling,
      result: null,
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("pointercancel", onCancel);
  });
  function onMove(e) {
    if (!drag || e.pointerId !== drag.id) return;
    const dx = e.clientX - drag.x, dy = e.clientY - drag.y;
    if (!drag.on) {
      if (Math.hypot(dx, dy) < 5) return;
      drag.on = true;
      drag.chip.setAttribute("data-dragging", "");
    }
    e.preventDefault();
    const { ev } = drag;
    const steps = (px) => Math.round(px / drag.pxPerMin / snap) * snap;
    if (drag.timed) {
      const hours = (hourTo - hourFrom) * 60;
      const startMin = ev.startAt.getHours() * 60 + ev.startAt.getMinutes() - hourFrom * 60;
      const length = ((ev.endAt > ev.startAt ? ev.endAt : new Date(ev.startAt.getTime() + 30 * MIN)) - ev.startAt) / MIN;
      const pc = (n) => `${+((n / hours) * 100).toFixed(3)}%`;
      if (drag.resize) {
        const longer = Math.max(snap - length, Math.min(hours - startMin - length, steps(dy)));
        drag.result = { longer };
        drag.chip.style.height = `max(22px, ${pc(length + longer)})`;
      } else {
        const minutes = Math.max(-startMin, Math.min(hours - startMin - length, steps(dy)));
        const to = dateAt(e.clientX, Math.min(Math.max(e.clientY, drag.parent.getBoundingClientRect().top + 1), drag.parent.getBoundingClientRect().bottom - 1)) || drag.from;
        const col = $(`[data-ev-date-col="${to}"]`) || drag.parent;
        if (drag.chip.parentElement !== col) col.append(drag.chip);
        drag.chip.style.top = pc(startMin + minutes);
        drag.chip.style.left = "2px";
        drag.chip.style.width = "calc(100% - 4px)";
        drag.result = { minutes, days: Math.round((parse(to) - parse(drag.from)) / DAY) };
      }
    } else {
      // A day's event: it follows the pointer, and drops on another day.
      drag.chip.style.transform = `translate(${dx}px, ${dy}px)`;
      const to = dateAt(e.clientX, e.clientY);
      $$("[data-ev-drop]").forEach((c) => c.removeAttribute("data-ev-drop"));
      if (to) {
        $(`[data-ev-day="${to}"]:not([data-ev-week-head] *), [data-ev-cell-date="${to}"]`)?.setAttribute("data-ev-drop", "");
        drag.result = { days: Math.round((parse(to) - parse(drag.from)) / DAY) };
      }
    }
  }
  function stopDrag() {
    window.removeEventListener("pointermove", onMove);
    window.removeEventListener("pointerup", onUp);
    window.removeEventListener("pointercancel", onCancel);
    $$("[data-ev-drop]").forEach((c) => c.removeAttribute("data-ev-drop"));
  }
  function onUp(e) {
    if (!drag || e.pointerId !== drag.id) return;
    const d = drag;
    drag = null;
    stopDrag();
    if (!d.on) return;
    // A drag isn't a click on the event.
    const stop = (c) => (c.stopPropagation(), c.preventDefault());
    window.addEventListener("click", stop, { capture: true, once: true });
    setTimeout(() => window.removeEventListener("click", stop, { capture: true }), 0);
    const m = d.result && move(d.ev.key, d.result);
    if (m) tell(m);
    draw(false);
  }
  function onCancel(e) {
    if (!drag || e.pointerId !== drag.id) return;
    drag = null;
    stopDrag();
    draw(false);
  }

  // The keys: Alt and the arrows move the focused event (a day across, a snap or a week
  // down); with Shift, Up and Down make a timed one shorter or longer.
  body.addEventListener("keydown", (e) => {
    if (!cfg.bindEvents || !e.altKey) return;
    const c = e.target.closest?.("[data-ev-event]");
    if (!c || view === "list" || view === "year") return;
    const key = c.getAttribute("data-ev-event");
    const ev = events().find((x) => x.key === key);
    if (!ev) return;
    const hours = view === "week" || view === "day";
    const timedHere = hours && !!c.closest("[data-ev-date-col]");
    const v = { ArrowUp: -1, ArrowDown: 1 }[e.key], h = { ArrowLeft: -1, ArrowRight: 1 }[side(e.key)];
    let change = null;
    if (h) change = { days: h };
    else if (v && timedHere) change = e.shiftKey ? { longer: v * snap } : { minutes: v * snap };
    else if (v && !hours) change = { days: v * 7 };
    if (!change) return;
    e.preventDefault();
    const m = move(key, change);
    if (m) {
      refocus(events().some((x) => x.key === key) ? key : String(m.id));
      tell(m);
    }
  });

  let last = null;
  draw(false);
  return {
    update() {
      const now = [resolve(cfg.events), resolve(cfg.categories), resolve(cfg.days), picked()];
      if (last && now.every((v, i) => v === last[i])) return;
      last = now;
      const focused = body.contains(document.activeElement) && document.activeElement.matches("[data-ev-day]");
      draw(focused);
    },
  };
};
