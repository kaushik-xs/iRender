// irender-ui-kit: the `ui-timeline` behaviour, for a timeline used as an activity feed.
// From each entry's `when` (a date and time) it puts a heading over each day's first
// entry (`group: day`), writes how long ago it was (`relative`, every minute), marks
// entries after `since` as New, and holds back all but the first `limit` behind a
// button. Entries from state are the runtime's: this only fills in and shows what each
// already has, and does it again whenever they change.
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

  let open = false;
  function draw() {
    const now = Date.now(), since = at(K.value(api, cfg.since));
    const entries = [...el.querySelectorAll("[data-tl-item]")];
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
      const rest = entries.length - shown;
      more.hidden = !cfg.limit || open || rest <= 0;
      more.querySelector("button").textContent = L(["Show {n} more", "Show {n} more"], { n: rest });
    }
  }
  el.querySelector(":scope > [data-tl-show] button")?.addEventListener("click", () => {
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
  // Times ago keep up, a minute at a time; a new day moves the headings.
  if (cfg.relative || cfg.group) {
    const t = setInterval(() => (el.isConnected ? draw() : clearInterval(t)), 60000);
  }
  return { update: draw };
};
