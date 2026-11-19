// irender-ui-kit: the `ui-notifications` behaviour. Draws the notifications from `items`
// ({ id, title, body, who, when, read, archived, tone }), in the Unread or All tab; a
// click opens one (`bindItem`, `open`) and marks it read; each has a read toggle and an
// archive button; "Mark all as read" does them all. Changes go to `bindItems`, with
// `read` and `archive` events. The bell's count follows.
(window.__irenderBehaviors ||= {})["ui-notifications"] = (el, api) => {
  const cfg = api.config;
  const $ = (s, from = el) => from.querySelector(s);
  const $$ = (s, from = el) => [...from.querySelectorAll(s)];
  const list = $("[data-nt-list]");
  const live = $("[data-nt-live]");
  // Interface text in the document's words (its `labels`), by the English.
  const L = (text, vars) => window.__irender.label(text, vars);
  const isObj = (v) => v !== null && typeof v === "object" && !Array.isArray(v);
  const resolve = (v) => (isObj(v) && "$irender" in v && Object.keys(v).length === 1 ? window.__irender.evaluate(v.$irender, api.state) : v);
  const items = () => (Array.isArray(resolve(cfg.items)) ? resolve(cfg.items) : []);
  const key = (n, i) => String(n.id ?? i);
  const say = (text) => {
    live.textContent = "";
    setTimeout(() => (live.textContent = text), 30);
  };
  let tab = cfg.tab ?? "unread";
  const initialsOf = (name) => String(name ?? "").split(/\s+/).filter(Boolean).slice(0, 2).map((w) => w[0].toUpperCase()).join("");
  const who = (n) => (n.who == null || n.who === "" ? null : isObj(n.who) ? n.who : { name: String(n.who) });
  const make = (tag, attrs = {}, text) => {
    const e = document.createElement(tag);
    for (const [k, v] of Object.entries(attrs)) if (v !== null && v !== undefined && v !== false) e.setAttribute(k, v === true ? "" : v);
    if (text !== undefined) e.textContent = text;
    return e;
  };
  const svg = {
    archive: '<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="2" y="3" width="12" height="3" rx="1"/><path d="M3 6v6.5A1.5 1.5 0 0 0 4.5 14h7a1.5 1.5 0 0 0 1.5-1.5V6M6.5 9h3"/></svg>',
  };

  function draw() {
    const all = items(), shown = all.map((n, i) => [n, i]).filter(([n]) => !n.archived && (tab === "all" || !n.read));
    const unread = all.filter((n) => !n.read && !n.archived).length;
    // The bell and the header follow the count.
    const count = $("[data-nt-count]");
    if (count) {
      count.hidden = !unread;
      count.textContent = unread > 99 ? "99+" : unread;
    }
    const bell = $("[data-nt-bell]");
    if (bell) bell.setAttribute("aria-label", unread ? L("{label}, {n} unread", { label: cfg.label ?? L("Notifications"), n: unread }) : cfg.label ?? L("Notifications"));
    $("[data-nt-mark-all]").disabled = !unread;
    for (const b of $$("[data-nt-tab]")) {
      b.setAttribute("aria-pressed", b.getAttribute("data-nt-tab") === tab ? "true" : "false");
      if (b.getAttribute("data-nt-tab") === "unread") b.textContent = unread ? L("Unread ({n})", { n: unread }) : L("Unread");
    }
    const focused = document.activeElement?.closest?.("[data-nt-item]")?.getAttribute("data-nt-item");
    list.replaceChildren(
      ...shown.map(([n, i]) => {
        const id = key(n, i), w = who(n);
        const li = make("li", { "data-nt-item": id, "data-unread": !n.read, "data-tone": n.tone ?? null });
        const open = make("button", { type: "button", "data-nt-open": "", "aria-label": [n.read ? null : L("Unread"), n.title, n.body, n.when].filter(Boolean).join(". ") });
        const mark = make("span", { "data-nt-mark": "", "aria-hidden": "true" });
        if (w?.avatar) mark.append(make("img", { src: w.avatar, alt: "" }));
        else if (w) mark.textContent = w.initials || initialsOf(w.name);
        else mark.setAttribute("data-dot", "");
        const text = make("span", { "data-nt-text": "" });
        text.append(make("span", { "data-nt-title": "" }, String(n.title ?? "")));
        if (n.body) text.append(make("span", { "data-nt-body": "" }, String(n.body)));
        if (n.when) text.append(make("time", {}, String(n.when)));
        open.append(mark, text);
        li.append(open);
        const actions = make("span", { "data-nt-actions": "" });
        const toggle = make("button", { type: "button", "data-nt-toggle": "", "aria-label": n.read ? L("Mark {name} as unread", { name: n.title }) : L("Mark {name} as read", { name: n.title }), title: n.read ? L("Mark as unread") : L("Mark as read") });
        toggle.append(make("span", { "aria-hidden": "true" }));
        const arch = make("button", { type: "button", "data-nt-archive": "", "aria-label": L("Archive {name}", { name: n.title }), title: L("Archive") });
        arch.innerHTML = svg.archive;
        actions.append(toggle, arch);
        li.append(actions);
        return li;
      }),
    );
    const empty = $("[data-nt-empty]");
    empty.hidden = !!shown.length;
    empty.textContent = tab === "unread" && all.some((n) => !n.archived) ? (cfg.caughtUp ?? L("You're all caught up.")) : (cfg.emptyText ?? L("No notifications."));
    if (focused) ($(`[data-nt-item="${CSS.escape(focused)}"] [data-nt-open]`) ?? $("[data-nt-open]"))?.focus();
  }

  // Rewrites `bindItems` with `change` applied to the items `ids` (all when null).
  function update(ids, change) {
    const all = items();
    const next = all.map((n, i) => (ids === null || ids.includes(key(n, i)) ? { ...n, ...change } : n));
    if (cfg.bindItems) api.set(cfg.bindItems, next);
    return all.filter((n, i) => ids === null || ids.includes(key(n, i)));
  }
  const find = (id) => items().find((n, i) => key(n, i) === id);

  el.addEventListener("click", (e) => {
    const t = e.target;
    const tabBtn = t.closest("[data-nt-tab]");
    if (tabBtn) {
      tab = tabBtn.getAttribute("data-nt-tab");
      draw();
      return;
    }
    if (t.closest("[data-nt-mark-all]")) {
      // The unread ones still shown; archived ones are left as they were.
      const ids = items().map((n, i) => [n, key(n, i)]).filter(([n]) => !n.read && !n.archived).map(([, id]) => id);
      const changed = update(ids, { read: true });
      if (changed.length) api.emit("read", { items: changed, read: true });
      say(L("All marked as read"));
      return;
    }
    const li = t.closest("[data-nt-item]");
    if (!li) return;
    const id = li.getAttribute("data-nt-item"), n = find(id);
    if (!n) return;
    // Focus moves to the next item when this one leaves the list.
    const next = () => {
      const rows = $$("[data-nt-item]").map((x) => x.getAttribute("data-nt-item")), at = rows.indexOf(id);
      return rows[at + 1] ?? rows[at - 1] ?? null;
    };
    if (t.closest("[data-nt-toggle]")) {
      const after = next();
      update([id], { read: !n.read });
      api.emit("read", { items: [n], read: !n.read });
      say(n.read ? L("Marked as unread") : L("Marked as read"));
      queueMicrotask(() => ($(`[data-nt-item="${CSS.escape(id)}"] [data-nt-toggle]`) ?? (after && $(`[data-nt-item="${CSS.escape(after)}"] [data-nt-open]`)) ?? $("[data-nt-mark-all]"))?.focus());
    } else if (t.closest("[data-nt-archive]")) {
      const after = next();
      update([id], { archived: true });
      api.emit("archive", { item: n });
      say(L("{name} archived", { name: n.title }));
      queueMicrotask(() => ((after && $(`[data-nt-item="${CSS.escape(after)}"] [data-nt-open]`)) ?? $("[data-nt-mark-all]"))?.focus());
    } else if (t.closest("[data-nt-open]")) {
      if (!n.read) update([id], { read: true });
      if (cfg.bindItem) api.set(cfg.bindItem, { ...n, read: true });
      api.emit("open", { item: n });
      if (cfg.closeOnOpen !== false) $("[data-nt-panel][popover]")?.hidePopover?.();
    }
  });
  // Up and Down move between notifications; Home and End to the ends.
  list.addEventListener("keydown", (e) => {
    const rows = $$("[data-nt-open]"), i = rows.indexOf(document.activeElement);
    if (i < 0) return;
    const to = { ArrowDown: i + 1, ArrowUp: i - 1, Home: 0, End: rows.length - 1 }[e.key];
    if (to === undefined || !rows[to]) return;
    e.preventDefault();
    rows[to].focus();
  });
  // Opening the panel: focus goes to the first notification, or Mark all as read.
  const panel = $("[data-nt-panel][popover]");
  panel?.addEventListener("toggle", (e) => {
    if (e.newState === "open") {
      draw();
      ($("[data-nt-open]") ?? $("[data-nt-tab][aria-pressed=true]"))?.focus();
    }
  });

  let last = null;
  draw();
  return {
    update() {
      const now = resolve(cfg.items);
      if (now === last) return;
      last = now;
      draw();
    },
  };
};
