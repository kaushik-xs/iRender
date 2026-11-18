// irender-ui-kit: the `ui-event-calendar` behaviour. Draws `events` in a month grid, a
// week of hours, or a list by day, and moves through time with the toolbar or the keys.
// Events: { id, title, start, end, allDay, category, tone, description }; `start` and
// `end` are dates ("2026-11-04") or local times ("2026-11-04T09:30"). A click on an
// event writes it to `bindEvent` (and fires `open`); on a day, its date to `bindDay`
// (and fires `day`), for adding one.
(window.__irenderBehaviors ||= {})["ui-event-calendar"] = (el, api) => {
  const cfg = api.config;
  const $ = (s, from = el) => from.querySelector(s);
  const $$ = (s, from = el) => [...from.querySelectorAll(s)];
  const body = $("[data-ev-body]");
  const live = $("[data-ev-live]");
  const isObj = (v) => v !== null && typeof v === "object" && !Array.isArray(v);
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
    return (Array.isArray(list) ? list : [])
      .map((e, i) => {
        const start = parse(e.start);
        if (!start) return null;
        let end = parse(e.end) ?? start;
        const allDay = e.allDay ?? !timed(e.start);
        // An all-day event's end date is its last day; a timed one ending at midnight ends the day before.
        const lastDay = allDay ? day(end) : end.getHours() || end.getMinutes() || sameDay(start, end) ? day(end) : addDays(day(end), -1);
        if (end < start) end = start;
        const cats = resolve(cfg.categories) ?? {};
        const tone = e.tone ?? (isObj(cats) ? cats[e.category] : null) ?? "primary";
        return { ...e, key: String(e.id ?? i), startAt: start, endAt: end, first: day(start), last: lastDay < day(start) ? day(start) : lastDay, allDay, tone };
      })
      .filter(Boolean)
      .sort((a, b) => a.startAt - b.startAt || (b.allDay ? 1 : 0) - (a.allDay ? 1 : 0) || String(a.title).localeCompare(String(b.title)));
  }
  const on = (list, d) => list.filter((e) => e.first <= d && d <= e.last);

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
    const when = e.allDay ? "all day" : `${time(e.startAt)}${e.endAt > e.startAt ? `–${time(e.endAt)}` : ""}`;
    b.setAttribute("aria-label", `${e.title}, ${when}${e.category ? `, ${e.category}` : ""}`);
    if (!e.allDay && !continues) b.append(make("time", {}, time(e.startAt)));
    b.append(make("span", {}, String(e.title ?? "")));
    return b;
  };

  function title() {
    if (view === "week") {
      // As the locale writes a range: "2–8 November 2026", "November 2 – 8, 2026".
      const a = startOfWeek(cursor), b = addDays(a, 6);
      const f = fmt({ day: "numeric", month: a.getMonth() === b.getMonth() ? "long" : "short", year: "numeric" });
      return f.formatRange ? f.formatRange(a, b) : `${f.format(a)} – ${f.format(b)}`;
    }
    return fmt({ month: "long", year: "numeric" }).format(cursor);
  }

  function month(list) {
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
        const d = addDays(start, w * 7 + i), here = on(list, d);
        const cell = make("div", {
          role: "gridcell",
          "data-ev-day": iso(d),
          tabindex: sameDay(d, cursor) ? 0 : -1,
          "data-outside": d.getMonth() !== cursor.getMonth(),
          "data-today": sameDay(d, today()),
          "data-picked": iso(d) === picked(),
          "aria-selected": iso(d) === picked() ? "true" : "false",
          "aria-label": `${fmt({ weekday: "long", day: "numeric", month: "long" }).format(d)}${here.length ? `, ${here.length} ${here.length === 1 ? "event" : "events"}` : ""}`,
        });
        cell.append(make("span", { "data-ev-date": "", "aria-hidden": "true" }, String(d.getDate())));
        const chips = make("div", { "data-ev-chips": "" });
        here.slice(0, here.length > max ? max - 1 : max).forEach((e) => chips.append(chip(e, d)));
        if (here.length > max) {
          chips.append(make("button", { type: "button", "data-ev-more": iso(d), tabindex: -1, "aria-label": `${here.length - max + 1} more on ${fmt({ day: "numeric", month: "long" }).format(d)}` }, `+${here.length - max + 1} more`));
        }
        cell.append(chips);
        row.append(cell);
      }
      grid.append(row);
    }
    return grid;
  }

  function week(list) {
    const start = startOfWeek(cursor), days = Array.from({ length: 7 }, (_, i) => addDays(start, i));
    const hours = Math.max(1, hourTo - hourFrom);
    const wrap = make("div", { "data-ev-weekview": "", style: `--hours: ${hours}` });
    const head = make("div", { "data-ev-week-head": "" });
    head.append(make("div", { "aria-hidden": "true" }));
    for (const d of days) {
      const h = make("button", { type: "button", "data-ev-day": iso(d), "data-today": sameDay(d, today()), tabindex: sameDay(d, cursor) ? 0 : -1, "aria-label": fmt({ weekday: "long", day: "numeric", month: "long" }).format(d) });
      h.append(make("span", {}, fmt({ weekday: "short" }).format(d)), make("b", {}, String(d.getDate())));
      head.append(h);
    }
    wrap.append(head);
    // All day, and events over more than a day.
    const allDay = make("div", { "data-ev-allday": "" });
    allDay.append(make("div", { "data-ev-hour": "" }, "All day"));
    for (const d of days) {
      const cell = make("div", { "data-ev-allday-cell": "" });
      on(list, d).filter((e) => e.allDay || e.first < e.last).forEach((e) => cell.append(chip(e, d)));
      allDay.append(cell);
    }
    wrap.append(allDay);
    const grid = make("div", { "data-ev-hours": "" });
    const labels = make("div", { "data-ev-labels": "", "aria-hidden": "true" });
    for (let h = hourFrom; h < hourTo; h++) labels.append(make("div", { "data-ev-hour": "" }, time(new Date(2000, 0, 1, h))));
    grid.append(labels);
    for (const d of days) {
      const col = make("div", { "data-ev-col": "", "data-ev-date-col": iso(d), "data-today": sameDay(d, today()) });
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
    if (e.allDay) return "All day";
    if (e.first < d) return sameDay(e.last, d) ? `Until ${time(e.endAt)}` : "All day";
    return `${time(e.startAt)}${e.endAt > e.startAt ? ` – ${time(e.endAt)}` : ""}`;
  }

  function agenda(list) {
    // The month around the cursor, day by day, only days with events.
    const first = new Date(cursor.getFullYear(), cursor.getMonth(), 1), last = new Date(cursor.getFullYear(), cursor.getMonth() + 1, 0);
    const box = make("div", { "data-ev-list": "" });
    for (let d = first; d <= last; d = addDays(d, 1)) {
      const here = on(list, d);
      if (!here.length) continue;
      const group = make("section", { "data-ev-group": iso(d), "data-today": sameDay(d, today()), "aria-label": fmt({ weekday: "long", day: "numeric", month: "long" }).format(d) });
      const h = make("h4", {});
      h.append(make("b", {}, String(d.getDate())), make("span", {}, fmt({ weekday: "long", month: "short" }).format(d)));
      group.append(h);
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
    if (!box.children.length) box.append(make("p", { "data-ev-empty": "" }, cfg.emptyText ?? `Nothing in ${fmt({ month: "long" }).format(cursor)}.`));
    return box;
  }

  function draw(focus) {
    const list = events();
    $("[data-ev-title]").textContent = title();
    for (const b of $$("[data-ev-view]")) b.setAttribute("aria-pressed", b.getAttribute("data-ev-view") === view ? "true" : "false");
    body.setAttribute("data-view", view);
    body.replaceChildren(view === "week" ? week(list) : view === "list" ? agenda(list) : month(list));
    // The focused day's events join the tab order after it.
    for (const c of $$(`[data-ev-day="${iso(cursor)}"] [data-ev-event], [data-ev-day="${iso(cursor)}"] [data-ev-more]`)) c.tabIndex = 0;
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
  function go(to, focus = false) {
    const before = view === "week" ? iso(startOfWeek(cursor)) : `${cursor.getFullYear()}-${cursor.getMonth()}`;
    cursor = day(to);
    const after = view === "week" ? iso(startOfWeek(cursor)) : `${cursor.getFullYear()}-${cursor.getMonth()}`;
    draw(focus);
    if (before !== after) say(title());
    api.emit("navigate", { date: iso(cursor), view });
  }
  function step(n) {
    go(view === "week" ? addDays(cursor, 7 * n) : addMonths(cursor, n));
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
    const original = (resolve(cfg.events) ?? []).find((x, i) => String(x.id ?? i) === key) ?? e;
    if (cfg.bindEvent) api.set(cfg.bindEvent, original);
    api.emit("open", { event: original });
  }

  el.addEventListener("click", (e) => {
    const t = e.target;
    if (t.closest("[data-ev-prev]")) return step(-1);
    if (t.closest("[data-ev-next]")) return step(1);
    if (t.closest("[data-ev-today]")) return go(today(), false);
    const v = t.closest("[data-ev-view]");
    if (v) {
      view = v.getAttribute("data-ev-view");
      if (cfg.bindView) api.set(cfg.bindView, view);
      draw(false);
      return say(`${v.textContent} view, ${title()}`);
    }
    const ev = t.closest("[data-ev-event]");
    if (ev) return open(ev.getAttribute("data-ev-event"));
    const more = t.closest("[data-ev-more]");
    if (more) {
      cursor = parse(more.getAttribute("data-ev-more"));
      view = "week";
      return draw(true);
    }
    const d = t.closest("[data-ev-day]");
    if (d && body.contains(d)) pick(parse(d.getAttribute("data-ev-day")));
  });
  body.addEventListener("keydown", (e) => {
    const d = e.target.closest?.("[data-ev-day]");
    if (!d || e.target !== d) return;
    // From the day that has focus (a click or Tab may have moved it).
    cursor = parse(d.getAttribute("data-ev-day"));
    const moves = { ArrowLeft: -1, ArrowRight: 1, ArrowUp: view === "week" ? 0 : -7, ArrowDown: view === "week" ? 0 : 7 };
    if (e.key in moves && moves[e.key]) return e.preventDefault(), go(addDays(cursor, moves[e.key]), true);
    if (e.key === "Home") return e.preventDefault(), go(startOfWeek(cursor), true);
    if (e.key === "End") return e.preventDefault(), go(addDays(startOfWeek(cursor), 6), true);
    if (e.key === "PageUp" || e.key === "PageDown") return e.preventDefault(), go(view === "week" ? addDays(cursor, e.key === "PageUp" ? -7 : 7) : addMonths(cursor, e.key === "PageUp" ? -1 : 1), true);
    if (e.key === "Enter" || e.key === " ") return e.preventDefault(), pick(cursor);
  });

  let last = null;
  draw(false);
  return {
    update() {
      const now = [resolve(cfg.events), resolve(cfg.categories), picked()];
      if (last && now.every((v, i) => v === last[i])) return;
      last = now;
      const focused = body.contains(document.activeElement) && document.activeElement.matches("[data-ev-day]");
      draw(focused);
    },
  };
};
