// irender-ui-kit: the `ui-timeline` behaviour, for a timeline used as an activity feed.
// From each entry's `when` (a date and time) it puts a heading over each day's first
// entry (`group: day`), writes how long ago it was (`relative`, every minute), marks
// entries after `since` as New, and holds back all but the first `limit` behind a
// button. With `filters` it shows one person's entries or one kind's (or several, set
// from state), and says so (`filter`); with `hasMore`, a Load more button once all are
// shown fires `more`; `bindSeen` gets when the feed was last seen, as the page is left. Entries from state are the runtime's:
// this only fills in and shows what each already has, and does it again whenever they change.
(window.__irenderBehaviors ||= {})["ui-timeline"] = (el, api) => {
  const cfg = api.config, K = window.__irenderKit, L = (t, v) => window.__irender.label(t, v);
  const loc = document.documentElement.lang || undefined;
  const at = (v) => {
    if (v === null || v === undefined || v === "") return null;
    const t = typeof v === "number" ? v : Date.parse(v);
    return Number.isFinite(t) ? t : null;
  };
  const dayOf = (t) => {
    const d = new Date(t);
    return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
  };
  const F = {};
  const fmt = (k, o) => (F[k] ||= new Intl.DateTimeFormat(loc, o));
  const rel = (F.rel ||= new Intl.RelativeTimeFormat(loc, { numeric: "auto", style: "short" }));
  // Labels: "Today", "Yesterday"
  function heading(t, now) {
    const days = Math.round((dayOf(now) - dayOf(t)) / 864e5);
    if (days === 0) return L("Today");
    if (days === 1) return L("Yesterday");
    if (days > 1 && days < 7) return fmt("weekday", { weekday: "long" }).format(t);
    return fmt("date", { day: "numeric", month: "long", year: new Date(t).getFullYear() === new Date(now).getFullYear() ? undefined : "numeric" }).format(t);
  }
  // How long ago, for the last day; past that the time (under a day's heading) or the date.
  function ago(t, now) {
    const s = Math.round((now - t) / 1000);
    if (s >= 0 && s < 45) return L("just now");
    if (s >= 0 && s < 3600) return rel.format(-Math.round(s / 60), "minute");
    if (s >= 0 && s < 86400) return rel.format(-Math.round(s / 3600), "hour");
    return cfg.group === "day" ? fmt("time", { timeStyle: "short" }).format(t) : fmt("when", { day: "numeric", month: "short", hour: "numeric", minute: "2-digit" }).format(t);
  }

  let open = false, picked = { who: null, kind: null };
  const bar = el.querySelector(":scope > [data-tl-filters]");
  // What's picked: from `bindFilter` when it's bound, else this feed's own.
  // Each of `who` and `kind` is one value, a list of them (any of which), or null for all.
  const filter = () => {
    const f = cfg.bindFilter ? api.state[cfg.bindFilter] : picked;
    const one = (v) => (Array.isArray(v) ? (v.length ? v.map(String) : null) : v === null || v === undefined || v === "" ? null : String(v));
    return { who: one(f?.who), kind: one(f?.kind) };
  };
  const passes = (v, f) => f === null || (Array.isArray(f) ? f.includes(v) : v === f);
  // A change of filter is told (`filter`), so a page can ask its server for what's picked.
  const setFilter = (f) => {
    picked = f;
    api.emit("filter", { who: f.who, kind: f.kind });
    if (cfg.bindFilter) api.set(cfg.bindFilter, f.who === null && f.kind === null ? null : f);
    else draw();
  };
  // Labels: "Everyone", "Every kind", "Who", "Kind"
  function menus(entries) {
    if (!bar) return;
    const f = filter(), distinct = (a) => [...new Set(entries.map((li) => li.getAttribute(a)).filter(Boolean))].sort((x, y) => x.localeCompare(y));
    const sig = JSON.stringify([distinct("data-who"), distinct("data-kind"), f]);
    if (bar.__sig === sig) return;
    bar.__sig = sig;
    const menu = (name, all, list, cur, key) => {
      const s = document.createElement("select");
      s.setAttribute("aria-label", L(name));
      s.setAttribute(`data-tl-${key}`, "");
      // Several picked (from state): the menu says how many, and picking one replaces them.
      // Labels: "{n} picked"
      const many = Array.isArray(cur) && cur.length > 1;
      s.append(new Option(L(all), ""), ...(many ? [new Option(L("{n} picked", { n: cur.length }), "\u0000", false, true)] : []), ...list.map((v) => new Option(v, v, false, !many && (Array.isArray(cur) ? cur[0] === v : v === cur))));
      s.addEventListener("change", () => s.value !== "\u0000" && setFilter({ ...filter(), [key]: s.value || null }));
      return s;
    };
    const kids = [];
    if (distinct("data-who").length > 1 || f.who) kids.push(menu("Who", "Everyone", distinct("data-who"), f.who, "who"));
    if (distinct("data-kind").length > 1 || f.kind) kids.push(menu("Kind", "Every kind", distinct("data-kind"), f.kind, "kind"));
    const keep = bar.contains(document.activeElement) ? [...document.activeElement.attributes].find((a) => a.name.startsWith("data-tl-"))?.name : null;
    bar.replaceChildren(...kids);
    bar.hidden = !kids.length;
    if (keep) bar.querySelector(`[${keep}]`)?.focus();
  }
  function draw() {
    const now = Date.now(), since = at(K.value(api, cfg.since));
    const all = [...el.querySelectorAll("[data-tl-item]")], f = filter();
    menus(all);
    // Left out by the filters: not shown, and not counted against `limit`.
    const entries = all.filter((li) => {
      const out = !passes(li.getAttribute("data-who"), f.who) || !passes(li.getAttribute("data-kind"), f.kind);
      li.toggleAttribute("data-tl-out", out);
      return !out;
    });
    let none = el.querySelector(":scope > [data-tl-none]");
    if (bar && !entries.length && all.length) {
      // Labels: "Nothing matches these filters"
      if (!none) el.insertBefore((none = Object.assign(document.createElement("li"), { textContent: L("Nothing matches these filters") })), bar.nextSibling).setAttribute("data-tl-none", "");
    } else none?.remove();
    let last = null, shown = 0;
    for (const li of entries) {
      const t = at(li.getAttribute("data-when"));
      const time = li.querySelector(":scope > [data-body] > [data-line] > time");
      const day = li.querySelector(":scope > [data-tl-day]");
      const isNew = li.querySelector(":scope > [data-body] > [data-line] > [data-tl-new]");
      const held = cfg.limit && !open && shown >= cfg.limit;
      li.toggleAttribute("data-tl-hidden", !!held);
      if (!held) shown++;
      if (time && t !== null) {
        time.setAttribute("datetime", new Date(t).toISOString());
        const full = fmt("full", { dateStyle: "full", timeStyle: "short" }).format(t);
        if (cfg.relative) {
          time.textContent = ago(t, now);
          time.title = full;
        } else if (cfg.group === "day") {
          time.textContent = fmt("time", { timeStyle: "short" }).format(t);
          time.title = full;
        }
      }
      if (day) {
        const d = t === null ? null : dayOf(t);
        const first = cfg.group === "day" && d !== null && d !== last && !held;
        day.hidden = !first;
        if (first) day.textContent = heading(t, now);
        if (d !== null && !held) last = d;
      }
      if (isNew) {
        const on = since !== null && t !== null && t > since;
        isNew.hidden = !on;
        isNew.textContent = on ? L("New") : "";
      }
    }
    const more = el.querySelector(":scope > [data-tl-show]");
    if (more) {
      const rest = entries.length - shown, held = !!cfg.limit && !open && rest > 0;
      // All here are shown, and the server has more: Load more.
      const server = !held && !!K.value(api, cfg.hasMore), busy = server && !!K.value(api, cfg.loading);
      more.hidden = !held && !server;
      const b = more.querySelector("button");
      // Labels: "Load more", "Loading…"
      b.textContent = held ? L(["Show {n} more", "Show {n} more"], { n: rest }) : busy ? L("Loading…") : L("Load more");
      b.toggleAttribute("data-tl-load", server);
      b.disabled = busy;
      b.setAttribute("aria-busy", String(busy));
    }
  }
  let before = 0;
  el.querySelector(":scope > [data-tl-show] button")?.addEventListener("click", (e) => {
    if (e.currentTarget.hasAttribute("data-tl-load")) {
      // The page adds them; focus goes to the first new one when they come.
      before = el.querySelectorAll("[data-tl-item]").length;
      return api.emit("more", { count: before, filter: filter() });
    }
    const first = el.querySelector("[data-tl-item][data-tl-hidden]");
    open = true;
    draw();
    // Focus goes to the first entry that was held back, so reading carries on there.
    if (first) {
      first.tabIndex = -1;
      first.focus();
    }
  });
  draw();
  // `bindSeen`: when this feed was last seen, written as the page is left or hidden (the
  // time of its newest entry, or now), for the page to keep as next visit's `since`.
  if (cfg.bindSeen) {
    const seen = (e) => {
      if (e.type === "visibilitychange" && document.visibilityState !== "hidden") return;
      const newest = Math.max(...[...el.querySelectorAll("[data-tl-item]")].map((li) => at(li.getAttribute("data-when")) ?? -Infinity));
      api.set(cfg.bindSeen, new Date(Number.isFinite(newest) ? Math.max(newest, Date.now()) : Date.now()).toISOString());
    };
    addEventListener("visibilitychange", seen);
    addEventListener("pagehide", seen);
  }
  // Times ago keep up, a minute at a time; a new day moves the headings.
  if (cfg.relative || cfg.group) {
    const t = setInterval(() => (el.isConnected ? draw() : clearInterval(t)), 60000);
  }
  return {
    update() {
      draw();
      const items = el.querySelectorAll("[data-tl-item]:not([data-tl-out])");
      if (before && items.length > before) {
        const first = el.querySelectorAll("[data-tl-item]")[before];
        before = 0;
        if (first && !first.hasAttribute("data-tl-out")) (first.tabIndex = -1), first.focus();
      }
    },
  };
};
