// irender-ui-kit: the `ui-filters` behaviour. What's applied lives in state under `bind`
// as `{ field: condition }`, in ui-chart's `filter` format:
//   list      [a, b] (is any of), { not: [a, b] } (is none of)
//   number    5 (equals), { gt }, { lt }, { from, to } (between, either end may be left out)
//   date      { from: d, to: d } (on), { lt: d } (before), { gt: d } (after), { from, to }
//   text      { contains }, "x" (is)
//   any       { empty: true } (is empty), { empty: false } (is not empty)
// While the panel is open it edits a draft, written on Apply (or at once with `live`).
(window.__irenderBehaviors ||= {})["ui-filters"] = (el, api) => {
  const cfg = api.config;
  const defs = cfg.defs;
  const $ = (s, from = el) => from.querySelector(s);
  const $$ = (s, from = el) => [...from.querySelectorAll(s)];
  const panel = $("[data-fl-panel]");
  const panes = $$("[data-fl-pane]");
  const tabs = $$("[data-fl-tab]");

  const isObj = (v) => v !== null && typeof v === "object" && !Array.isArray(v);
  const announcer = $("[data-fl-live]");
  const say = (text) => {
    if (!announcer) return;
    announcer.textContent = "";
    setTimeout(() => (announcer.textContent = text), 30);
  };
  const blank = (v) => v === null || v === undefined || v === "";
  const same = (a, b) => a === b || (!blank(a) && !blank(b) && String(a) === String(b));
  const clone = (v) => (v === undefined ? v : JSON.parse(JSON.stringify(v)));

  // ---- state and rows, resolved against the state and the list item it's in
  const env = () => {
    const s = api.scope?.();
    return s ? { ...api.state, ...s } : api.state;
  };
  // Values with nothing to resolve come back as they are, so plain rows keep their identity.
  const live = (v) => (Array.isArray(v) ? v.some(live) : isObj(v) && (("$irender" in v && Object.keys(v).length === 1) || Object.values(v).some(live)));
  const resolve = (v) => {
    if (!live(v)) return v;
    if (Array.isArray(v)) return v.map(resolve);
    if (!isObj(v)) return v;
    if ("$irender" in v && Object.keys(v).length === 1) return window.__irender.evaluate(v.$irender, env());
    return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, resolve(x)]));
  };
  const applied = () => (isObj(api.state[cfg.bind]) ? api.state[cfg.bind] : {});
  const rows = () => {
    const r = resolve(cfg.rows);
    return Array.isArray(r) ? r : [];
  };
  const at = (r, f) => (r === null || typeof r !== "object" ? undefined : f.split(".").reduce((o, k) => (o == null ? o : o[k]), r));

  // ---- formatting, for chips
  const locale = cfg.locale || undefined;
  const number = (d, n) => {
    const x = Number(n);
    if (!Number.isFinite(x)) return String(n);
    try {
      const o = d.type === "currency" ? { style: "currency", currency: d.currency || cfg.currency || "USD", ...(Number.isInteger(x) && { minimumFractionDigits: 0 }) } : {};
      return new Intl.NumberFormat(locale, o).format(x) + (d.unit ? " " + d.unit : "");
    } catch {
      return String(n);
    }
  };
  const date = (s) => {
    const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(String(s));
    const v = m ? new Date(+m[1], +m[2] - 1, +m[3]) : new Date(s);
    return isNaN(v) ? String(s) : new Intl.DateTimeFormat(locale, { day: "numeric", month: "short", year: "numeric" }).format(v);
  };
  const show = (d, v) => (d.type === "date" ? date(v) : d.type === "number" || d.type === "currency" ? number(d, v) : String(v));

  // ---- a list filter's values: `options`, or the field's values in `rows`, with counts
  function options(d) {
    const given = resolve(d.options);
    const counts = new Map();
    for (const r of rows()) {
      const v = at(r, d.field);
      for (const x of Array.isArray(v) ? v : [v]) if (!blank(x)) counts.set(String(x), (counts.get(String(x)) ?? 0) + 1);
    }
    const list = Array.isArray(given)
      ? given.map((o) => (isObj(o) ? { value: o.value, label: String(o.label ?? o.value) } : { value: o, label: String(o) }))
      : [...counts.keys()].sort((a, b) => a.localeCompare(b, locale, { numeric: true })).map((v) => ({ value: v, label: v }));
    const known = cfg.rows != null;
    return list.map((o) => ({ ...o, count: known ? counts.get(String(o.value)) ?? 0 : null }));
  }
  const labelOf = (d, v) => options(d).find((o) => same(o.value, v))?.label ?? String(v);

  // ---- conditions <-> what the controls show
  // What a condition means for a field: its operator, values `a` and `b`, or the values picked.
  function read(d, c) {
    const first = d.ops[0];
    const none = { op: first, a: "", b: "", picked: [] };
    if (blank(c) || (Array.isArray(c) && !c.length)) return none;
    if (isObj(c) && "empty" in c) return { ...none, op: c.empty ? "empty" : "set" };
    if (d.type === "list") {
      // Plain values only: a condition the panel can't show is left as its default.
      const plain = (vs) => [].concat(vs ?? []).filter((v) => !isObj(v) && !Array.isArray(v));
      if (isObj(c) && "not" in c) return { ...none, op: "none", picked: plain(c.not) };
      if (isObj(c)) return none;
      return { ...none, op: "any", picked: plain(c) };
    }
    if (isObj(c)) {
      if ("contains" in c) return { op: "contains", a: c.contains ?? "", b: "" };
      if ("gt" in c) return { op: d.type === "date" ? "after" : "gt", a: c.gt ?? "", b: "" };
      if ("lt" in c) return { op: d.type === "date" ? "before" : "lt", a: c.lt ?? "", b: "" };
      if (d.type === "date" && !blank(c.from) && same(c.from, c.to)) return { op: "on", a: c.from, b: "" };
      return { op: "between", a: c.from ?? "", b: c.to ?? "" };
    }
    return { op: d.type === "text" ? "is" : d.type === "date" ? "on" : "eq", a: Array.isArray(c) ? c[0] : c, b: "" };
  }
  // The condition for an operator and values; null when there's nothing to filter by.
  function write(d, { op, a, b, picked }) {
    const num = (v) => (d.type === "number" || d.type === "currency" ? Number(v) : v);
    const has = (v) => !blank(v) && String(v).trim() !== "";
    switch (op) {
      case "any": return picked.length ? picked : null;
      case "none": return picked.length ? { not: picked } : null;
      case "empty": return { empty: true };
      case "set": return { empty: false };
      case "contains": return has(a) ? { contains: a } : null;
      case "is": return has(a) ? a : null;
      case "eq": return has(a) ? num(a) : null;
      case "on": return has(a) ? { from: a, to: a } : null;
      case "gt": case "after": return has(a) ? { gt: num(a) } : null;
      case "lt": case "before": return has(a) ? { lt: num(a) } : null;
      case "between": {
        if (!has(a) && !has(b)) return null;
        const c = {};
        if (has(a)) c.from = num(a);
        if (has(b)) c.to = num(b);
        return c;
      }
    }
    return null;
  }
  // Why a between can't be applied, or "".
  function problem(d, s) {
    if (s.op !== "between" || blank(s.a) || blank(s.b) || s.a === "" || s.b === "") return "";
    const bad = d.type === "date" ? String(s.a) > String(s.b) : Number(s.a) > Number(s.b);
    return !bad ? "" : d.type === "date" ? "The start date can't be after the end date." : "The first value can't be more than the second.";
  }
  const takesValue = (op) => !["empty", "set", "any", "none"].includes(op);
  const takesList = (op) => op === "any" || op === "none";

  // ---- the chip text for a condition
  function describe(d, c) {
    const s = read(d, c);
    const many = (vs) => {
      const names = vs.map((v) => labelOf(d, v));
      return names.length > 2 ? `${names.slice(0, 2).join(", ")} +${names.length - 2}` : names.join(", ");
    };
    switch (s.op) {
      case "any": return many(s.picked);
      case "none": return `not ${many(s.picked)}`;
      case "empty": return "is empty";
      case "set": return "is not empty";
      case "contains": return `contains “${s.a}”`;
      case "is": return `“${s.a}”`;
      case "eq": case "on": return show(d, s.a);
      case "gt": return `> ${show(d, s.a)}`;
      case "lt": return `< ${show(d, s.a)}`;
      case "after": return `after ${show(d, s.a)}`;
      case "before": return `before ${show(d, s.a)}`;
      case "between":
        if (blank(s.b) || s.b === "") return d.type === "date" ? `from ${show(d, s.a)}` : `≥ ${show(d, s.a)}`;
        if (blank(s.a) || s.a === "") return d.type === "date" ? `until ${show(d, s.b)}` : `≤ ${show(d, s.b)}`;
        return `${show(d, s.a)} – ${show(d, s.b)}`;
    }
    return "";
  }
  const active = (f, d) => write(d, read(d, f[d.field])) !== null;
  // How many filters a field adds: each value picked in a list, otherwise one.
  const count = (f, d) => {
    if (!active(f, d)) return 0;
    const s = read(d, f[d.field]);
    return d.type === "list" ? s.picked.length : 1;
  };

  // ---- date presets: a range, from `today`
  const pad = (n) => String(n).padStart(2, "0");
  const iso = (d) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
  const todayDate = () => {
    const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(cfg.today ?? "");
    return m ? new Date(+m[1], +m[2] - 1, +m[3]) : new Date(new Date().getFullYear(), new Date().getMonth(), new Date().getDate());
  };
  function preset(name) {
    const t = todayDate(), y = t.getFullYear(), mo = t.getMonth(), q = Math.floor(mo / 3) * 3;
    const back = (n) => new Date(y, mo, t.getDate() - n);
    const range = {
      today: [t, t],
      yesterday: [back(1), back(1)],
      last7: [back(6), t],
      last30: [back(29), t],
      thisMonth: [new Date(y, mo, 1), new Date(y, mo + 1, 0)],
      lastMonth: [new Date(y, mo - 1, 1), new Date(y, mo, 0)],
      thisQuarter: [new Date(y, q, 1), new Date(y, q + 3, 0)],
      thisYear: [new Date(y, 0, 1), new Date(y, 11, 31)],
    }[name];
    return range && range.map(iso);
  }

  // ---- the chips and counts beside the button
  const chips = $("[data-fl-chips]"), clearAll = $("[data-fl-clear]"), total = $("[data-fl-count]");
  function drawBar() {
    const f = applied();
    const on = defs.filter((d) => active(f, d));
    const n = defs.reduce((sum, d) => sum + count(f, d), 0);
    total.hidden = !n;
    total.textContent = n;
    const button = $("[data-fl-open]"), name = $("span:not([data-fl-count])", button).textContent;
    if (n) button.setAttribute("aria-label", `${name}, ${n} applied`);
    else button.removeAttribute("aria-label");
    if (!chips) return;
    clearAll.hidden = !on.length;
    chips.replaceChildren(
      ...on.map((d) => {
        const chip = document.createElement("span");
        chip.setAttribute("data-fl-chip", "");
        const edit = document.createElement("button");
        edit.type = "button";
        edit.setAttribute("data-fl-edit", defs.indexOf(d));
        const name = document.createElement("b");
        name.textContent = d.label + ":";
        const what = document.createElement("span");
        what.textContent = describe(d, f[d.field]);
        edit.title = `${d.label}: ${what.textContent}`;
        edit.append(name, what);
        const x = document.createElement("button");
        x.type = "button";
        x.setAttribute("data-fl-remove", d.field);
        x.setAttribute("aria-label", `Remove the ${d.label} filter`);
        x.textContent = "✕";
        chip.append(edit, x);
        return chip;
      }),
    );
  }

  // ---- the panel
  let draft = {};
  let current = 0;
  const instant = !!cfg.live;
  const pane = (i) => panes[i];
  const stateOf = (i) => {
    const p = pane(i), d = defs[i];
    if (d.type === "list") return { op: $("[data-fl-op]", p).value, picked: $$("[data-fl-pick]:checked", p).map((c) => c.__value), a: "", b: "" };
    return { op: $("[data-fl-op]", p).value, a: $("[data-fl-a]", p).value, b: $("[data-fl-b]", p).value };
  };

  // Shows field `i`'s draft condition in its pane.
  function fill(i) {
    const p = pane(i), d = defs[i], s = read(d, draft[d.field]);
    $("[data-fl-op]", p).value = d.ops.includes(s.op) ? s.op : d.ops[0];
    if (d.type === "list") drawOptions(i, s.picked);
    else {
      $("[data-fl-a]", p).value = s.a ?? "";
      $("[data-fl-b]", p).value = s.b ?? "";
    }
    shape(i);
  }
  // Shows the inputs the operator takes.
  function shape(i) {
    const p = pane(i), d = defs[i], op = $("[data-fl-op]", p).value;
    const presets = $("[data-fl-presets]", p);
    if (presets) {
      const s0 = stateOf(i);
      presets.hidden = !d.ops.includes("between") || !takesValue(op);
      for (const b of $$("[data-fl-preset]", presets)) {
        const r = preset(b.getAttribute("data-fl-preset"));
        b.setAttribute("aria-pressed", r && s0.op === "between" && s0.a === r[0] && s0.b === r[1] ? "true" : "false");
      }
    }
    if (d.type === "list") {
      const values = takesList(op);
      $("[data-fl-options]", p).hidden = !values;
      $("[data-fl-search]", p).hidden = !values || options(d).length <= 5;
    } else {
      $("[data-fl-values]", p).hidden = !takesValue(op);
      $("[data-fl-b]", p).hidden = $("[data-fl-and]", p).hidden = op !== "between";
      $("[data-fl-a]", p).setAttribute("aria-label", `${d.label} ${op === "between" ? "from" : "value"}`);
      $("[data-fl-b]", p).setAttribute("aria-label", `${d.label} to`);
    }
    const why = problem(d, stateOf(i));
    const err = $("[data-fl-error]", p);
    err.hidden = !why;
    err.textContent = why;
    for (const input of $$("[data-fl-a], [data-fl-b]", p)) input.setAttribute("aria-invalid", why ? "true" : "false");
    const apply = $("[data-fl-apply]", panel);
    if (apply) apply.disabled = defs.some((d, j) => problem(d, stateOf(j)));
  }
  function drawOptions(i, picked) {
    const p = pane(i), d = defs[i];
    const list = options(d);
    const search = $("[data-fl-search]", p);
    search.hidden = list.length <= 5 || !takesList($("[data-fl-op]", p).value);
    const q = search.value.trim().toLowerCase();
    const shown = list.filter((o) => !q || o.label.toLowerCase().includes(q) || picked.some((v) => same(v, o.value)));
    const box = $("[data-fl-options]", p);
    // Picked values that aren't options (any more) stay, so nothing is dropped unseen.
    for (const v of picked) if (!list.some((o) => same(o.value, v))) shown.push({ value: v, label: String(v), count: null });
    const choice = $("template[data-fl-choice]", p);
    box.replaceChildren(
      ...shown.map((o) => {
        const row = choice.content.firstElementChild.cloneNode(true);
        const cb = $("[data-fl-pick]", row);
        cb.__value = o.value;
        cb.checked = picked.some((v) => same(v, o.value));
        const [name, n] = $$("span", row);
        name.textContent = o.label;
        if (o.count === null) n.remove();
        else n.textContent = o.count;
        return row;
      }),
    );
    if (!shown.length) {
      const none = document.createElement("p");
      none.setAttribute("data-fl-none", "");
      none.textContent = list.length ? "No matches" : "No values";
      box.append(none);
    }
  }
  function badges() {
    tabs.forEach((t, i) => {
      const n = count(draft, defs[i]);
      const b = $("[data-fl-n]", t);
      b.hidden = !n;
      b.textContent = n;
    });
  }
  // Reads pane `i` into the draft, and writes it out at once when `live`.
  function take(i) {
    const d = defs[i], s = stateOf(i);
    const c = problem(d, s) ? draft[d.field] : write(d, s);
    if (c === null || c === undefined) delete draft[d.field];
    else draft[d.field] = c;
    badges();
    if (instant && !problem(d, s)) commit();
  }
  function commit() {
    api.set(cfg.bind, clone(draft));
  }
  function select(i, focus) {
    current = i;
    tabs.forEach((t, j) => {
      t.setAttribute("aria-selected", j === i ? "true" : "false");
      t.tabIndex = j === i ? 0 : -1;
      panes[j].hidden = j !== i;
    });
    if (focus) tabs[i].focus();
  }
  function open(i = current) {
    draft = clone(applied());
    defs.forEach((_, j) => fill(j));
    badges();
    select(i);
    if (!panel.open) panel.showModal();
    // Into the first control of the field, so typing starts at once.
    const p = pane(i);
    ($("[data-fl-search]:not([hidden])", p) || $("[data-fl-pick]", p) || $("[data-fl-a]:not([hidden])", p) || tabs[i]).focus();
  }

  // ---- saved views: `bindViews` holds [{ name, filters }]
  const menu = $("[data-fl-views]");
  const views = () => (cfg.bindViews && Array.isArray(api.state[cfg.bindViews]) ? api.state[cfg.bindViews] : []);
  const sameFilters = (a, b) => JSON.stringify(a ?? {}) === JSON.stringify(b ?? {});
  function drawViews() {
    if (!menu) return;
    const list = $("[data-fl-view-list]", menu), f = applied();
    const vs = views();
    list.replaceChildren(
      ...vs.map((v, i) => {
        const li = document.createElement("li");
        li.setAttribute("data-index", i);
        const apply = document.createElement("button");
        apply.type = "button";
        apply.setAttribute("data-fl-view-apply", "");
        apply.setAttribute("aria-current", sameFilters(v.filters, f) ? "true" : "false");
        apply.textContent = v.name;
        const del = document.createElement("button");
        del.type = "button";
        del.setAttribute("data-fl-view-delete", "");
        del.setAttribute("aria-label", `Delete the ${v.name} view`);
        del.textContent = "✕";
        li.append(apply, del);
        return li;
      }),
    );
    if (!vs.length) {
      const li = document.createElement("li");
      li.setAttribute("data-fl-view-empty", "");
      li.textContent = "No saved views yet.";
      list.append(li);
    }
  }
  function toggleViews(open = menu.hidden) {
    menu.hidden = !open;
    $("[data-fl-views-open]").setAttribute("aria-expanded", open ? "true" : "false");
    if (open) {
      drawViews();
      ($("[data-fl-view-apply]", menu) || $("[data-fl-view-name]", menu)).focus();
    }
  }
  function saveView() {
    const box = $("[data-fl-view-name]", menu), name = box.value.trim();
    if (!name) return box.focus();
    // A view of the same name is replaced.
    const rest = views().filter((v) => v.name !== name);
    api.set(cfg.bindViews, [...rest, { name, filters: clone(applied()) }]);
    box.value = "";
    say(`Saved the ${name} view`);
    queueMicrotask(drawViews);
  }
  function applyView(i) {
    const v = views()[i];
    if (!v) return;
    api.set(cfg.bind, clone(v.filters ?? {}));
    say(`${v.name} applied`);
    toggleViews(false);
    $("[data-fl-views-open]").focus();
  }
  function deleteView(i) {
    const v = views()[i];
    if (!v) return;
    api.set(cfg.bindViews, views().filter((_, j) => j !== i));
    say(`Deleted the ${v.name} view`);
    queueMicrotask(() => {
      drawViews();
      ($("[data-fl-view-apply]", menu) || $("[data-fl-view-name]", menu)).focus();
    });
  }
  if (menu) {
    menu.addEventListener("keydown", (e) => {
      if (e.key === "Escape") {
        e.preventDefault();
        toggleViews(false);
        $("[data-fl-views-open]").focus();
      }
      if (e.key === "Enter" && e.target.matches("[data-fl-view-name]")) e.preventDefault(), saveView();
    });
    document.addEventListener("pointerdown", (e) => !menu.hidden && !e.target.closest("[data-fl-views-wrap]") && toggleViews(false));
  }

  // ---- events
  el.addEventListener("click", (e) => {
    const t = e.target.closest("button");
    if (!t || !el.contains(t)) return;
    if (t.hasAttribute("data-fl-preset")) {
      const p = t.closest("[data-fl-pane]"), i = Number(p.getAttribute("data-fl-pane"));
      const r = preset(t.getAttribute("data-fl-preset"));
      $("[data-fl-op]", p).value = "between";
      $("[data-fl-a]", p).value = r[0];
      $("[data-fl-b]", p).value = r[1];
      shape(i);
      take(i);
      return;
    }
    if (t.hasAttribute("data-fl-views-open")) return toggleViews();
    if (t.hasAttribute("data-fl-view-save")) return saveView();
    if (t.hasAttribute("data-fl-view-apply")) return applyView(Number(t.closest("li").getAttribute("data-index")));
    if (t.hasAttribute("data-fl-view-delete")) return deleteView(Number(t.closest("li").getAttribute("data-index")));
    if (t.hasAttribute("data-fl-open")) open();
    else if (t.hasAttribute("data-fl-edit")) open(Number(t.getAttribute("data-fl-edit")));
    else if (t.hasAttribute("data-fl-remove")) {
      const f = { ...applied() };
      delete f[t.getAttribute("data-fl-remove")];
      api.set(cfg.bind, f);
      // Focus stays in the bar: the next chip, or the button.
      queueMicrotask(() => ($("[data-fl-edit]") || $("[data-fl-open]")).focus());
    } else if (t.hasAttribute("data-fl-clear")) {
      api.set(cfg.bind, {});
      queueMicrotask(() => $("[data-fl-open]").focus());
    } else if (t.hasAttribute("data-fl-tab")) select(Number(t.getAttribute("data-fl-tab")));
    else if (t.hasAttribute("data-fl-reset")) {
      draft = {};
      defs.forEach((_, j) => fill(j));
      badges();
      if (instant) commit();
    } else if (t.hasAttribute("data-fl-apply")) {
      if (defs.some((d, j) => problem(d, stateOf(j)))) return;
      commit();
      panel.close();
      $("[data-fl-open]").focus();
    }
  });
  panel.addEventListener("change", (e) => {
    const p = e.target.closest("[data-fl-pane]");
    if (!p) return;
    const i = Number(p.getAttribute("data-fl-pane"));
    if (e.target.matches("[data-fl-op]")) {
      // Picks stay when switching between is any of and is none of.
      if (defs[i].type === "list") drawOptions(i, stateOf(i).picked);
      shape(i);
    }
    if (!e.target.matches("[data-fl-search]")) take(i);
  });
  panel.addEventListener("input", (e) => {
    const p = e.target.closest("[data-fl-pane]");
    if (!p) return;
    const i = Number(p.getAttribute("data-fl-pane"));
    if (e.target.matches("[data-fl-search]")) drawOptions(i, stateOf(i).picked);
    else if (e.target.matches("[data-fl-a], [data-fl-b]")) {
      shape(i);
      take(i);
    }
  });
  // Up and down (and Home, End) move between fields.
  $("[role=tablist]", panel).addEventListener("keydown", (e) => {
    const i = tabs.indexOf(e.target);
    if (i < 0) return;
    const to = { ArrowDown: i + 1, ArrowUp: i - 1, Home: 0, End: tabs.length - 1 }[e.key];
    if (to === undefined) return;
    e.preventDefault();
    select((to + tabs.length) % tabs.length, true);
  });
  // Escape, ✕ or Cancel: the draft is dropped; focus goes back to the button.
  panel.addEventListener("close", () => {
    if (document.activeElement === document.body || !document.activeElement) $("[data-fl-open]").focus();
  });

  drawBar();
  let lastRows = null, lastApplied = null;
  return {
    update() {
      const f = JSON.stringify(applied());
      const r = cfg.rows != null ? rows() : null;
      if (f !== lastApplied) {
        lastApplied = f;
        drawBar();
        if (menu && !menu.hidden) drawViews();
        // Changed from outside while open (or by `live`): the panel follows.
        if (panel.open && instant) {
          draft = clone(applied());
          badges();
        }
      }
      if (r !== lastRows) {
        lastRows = r;
        if (panel.open) defs.forEach((d, j) => d.type === "list" && drawOptions(j, stateOf(j).picked));
        drawBar();
      }
    },
  };
};
