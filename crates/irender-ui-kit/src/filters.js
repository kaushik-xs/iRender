// irender-ui-kit: the `ui-filters` behaviour. What's applied lives in state under `bind`
// as `{ field: condition }`, in ui-chart's `filter` format:
//   list      [a, b] (is any of), { not: [a, b] } (is none of)
//   number    5 (equals), { gt }, { lt }, { from, to } (between, either end may be left out)
//   date      { from: d, to: d } (on), { lt: d } (before), { gt: d } (after), { from, to }
//   text      { contains }, "x" (is)
//   any       { empty: true } (is empty), { empty: false } (is not empty)
// While the panel is open it edits a draft, written on Apply (or at once with `live`).
// With many fields, a box over them finds one by its name or section.
(window.__irenderBehaviors ||= {})["ui-filters"] = (el, api) => {
  const cfg = api.config;
  const defs = cfg.defs;
  const $ = (s, from = el) => from.querySelector(s);
  const $$ = (s, from = el) => [...from.querySelectorAll(s)];
  const panel = $("[data-fl-panel]");
  const panes = $$("[data-fl-pane]");
  const tabs = $$("[data-fl-tab]");

  const isObj = (v) => v !== null && typeof v === "object" && !Array.isArray(v);
  // Interface text in the document's words (its `labels`), by the English.
  const L = (text, vars) => window.__irender.label(text, vars);
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
  // Numbers in the panel are grouped as they're typed, in the locale's way.
  const sepParts = new Intl.NumberFormat(locale).formatToParts(12345.6);
  const groupSep = sepParts.find((x) => x.type === "group")?.value ?? ",";
  const decSep = sepParts.find((x) => x.type === "decimal")?.value ?? ".";
  const toNumberText = (text) => {
    let t = String(text ?? "").split(groupSep).join("").replace(/[\s\u00a0\u202f]/g, "");
    if (decSep !== ".") t = t.replace(decSep, ".");
    t = t.replace(/[^\d.-]/g, "");
    return t;
  };
  const grouped = (raw) => {
    const t = toNumberText(raw);
    if (!t || t === "-") return t;
    const neg = t.startsWith("-"), [whole, frac] = t.replace(/-/g, "").split(".");
    const w = whole ? new Intl.NumberFormat(locale, { maximumFractionDigits: 0 }).format(BigInt(whole)) : "0";
    return (neg ? "-" : "") + w + (frac !== undefined ? decSep + frac : "");
  };
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
  // With `optionsUrl`, the values found for what's searched; every one seen keeps its label.
  const remote = new Map(); // field → { q, list, seen, loading, failed, seq }
  const remoteOf = (d) => remote.get(d.field) ?? remote.set(d.field, { q: null, list: [], seen: new Map(), loading: false, failed: false, seq: 0, timer: 0 }).get(d.field);
  function lookUp(i, q, wait = 0) {
    const d = defs[i], r = remoteOf(d), base = resolve(d.optionsUrl);
    if (typeof base !== "string" || r.q === q) return;
    r.q = q;
    clearTimeout(r.timer);
    r.timer = setTimeout(async () => {
      const n = ++r.seq;
      r.loading = true;
      r.failed = false;
      try {
        const res = await fetch(base + (base.includes("?") ? "&" : "?") + new URLSearchParams({ q }), { headers: { accept: "application/json" } });
        if (!res.ok) throw new Error(String(res.status));
        const got = await res.json();
        if (n !== r.seq) return;
        r.list = (Array.isArray(got) ? got : got?.options ?? []).map((o) => (isObj(o) ? { value: o.value, label: String(o.label ?? o.value), count: o.count ?? null } : { value: o, label: String(o), count: null }));
        for (const o of r.list) r.seen.set(String(o.value), o);
      } catch {
        if (n !== r.seq) return;
        r.list = [];
        r.failed = true;
        r.q = null;
      }
      r.loading = false;
      drawOptions(i, stateOf(i).picked);
      drawBar();
    }, wait);
  }
  function options(d) {
    if (d.optionsUrl) return [...remoteOf(d).seen.values()];
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
    return !bad ? "" : d.type === "date" ? L("The start date can't be after the end date.") : L("The first value can't be more than the second.");
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
      case "none": return L("not {values}", { values: many(s.picked) });
      case "empty": return L("is empty");
      case "set": return L("is not empty");
      case "contains": return L("contains “{value}”", { value: s.a });
      case "is": return `“${s.a}”`;
      case "eq": case "on": return show(d, s.a);
      case "gt": return `> ${show(d, s.a)}`;
      case "lt": return `< ${show(d, s.a)}`;
      case "after": return L("after {value}", { value: show(d, s.a) });
      case "before": return L("before {value}", { value: show(d, s.a) });
      case "between":
        if (blank(s.b) || s.b === "") return d.type === "date" ? L("from {value}", { value: show(d, s.a) }) : `≥ ${show(d, s.a)}`;
        if (blank(s.a) || s.a === "") return d.type === "date" ? L("until {value}", { value: show(d, s.b) }) : `≤ ${show(d, s.b)}`;
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
  // Quarters and years follow the financial year, which starts in month `fiscalStart`.
  const fs = (cfg.fiscalStart || 1) - 1;
  // The (financial) year `t` is in, by the calendar year it starts in, and its quarter, 0–3.
  const fyOf = (t) => (t.getMonth() >= fs ? t.getFullYear() : t.getFullYear() - 1);
  const quarterOf = (t) => Math.floor(((t.getMonth() - fs + 12) % 12) / 3);
  // A quarter `n` steps from today's, as [first day, last day].
  const quarterSpan = (t, n) => {
    const m = fs + (quarterOf(t) + n) * 3;
    return [new Date(fyOf(t), m, 1), new Date(fyOf(t), m + 3, 0)];
  };
  function preset(name) {
    const t = todayDate(), y = t.getFullYear(), mo = t.getMonth(), fy = fyOf(t);
    const back = (n) => new Date(y, mo, t.getDate() - n);
    const ago = /^quarter(\d)$/.exec(name);
    const range = ago ? quarterSpan(t, -Number(ago[1])) : {
      today: [t, t],
      yesterday: [back(1), back(1)],
      last7: [back(6), t],
      last30: [back(29), t],
      thisMonth: [new Date(y, mo, 1), new Date(y, mo + 1, 0)],
      lastMonth: [new Date(y, mo - 1, 1), new Date(y, mo, 0)],
      thisQuarter: quarterSpan(t, 0),
      lastQuarter: quarterSpan(t, -1),
      thisYear: [new Date(fy, fs, 1), new Date(fy + 1, fs, 0)],
      lastYear: [new Date(fy - 1, fs, 1), new Date(fy, fs, 0)],
      yearToDate: [new Date(fy, fs, 1), t],
    }[name];
    return range && range.map(iso);
  }
  // "Q2 2026", or on a financial year "Q2 FY2026–27", as the calendar names quarters.
  for (const b of $$("[data-fl-preset^=quarter]")) {
    const n = Number(b.getAttribute("data-fl-preset").slice(7)), [from] = quarterSpan(todayDate(), -n), y = fyOf(from);
    b.textContent = L("Q{n} {year}", { n: String(quarterOf(from) + 1), year: fs ? `FY${y}–${String(y + 1).slice(-2)}` : String(y) });
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
    if (n) button.setAttribute("aria-label", L("{name}, {n} applied", { name, n }));
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
        x.setAttribute("aria-label", L("Remove the {name} filter", { name: d.label }));
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
    const val = (sel) => {
      const input = $(sel, p);
      return input.hasAttribute("data-fl-num") ? toNumberText(input.value) : input.value;
    };
    return { op: $("[data-fl-op]", p).value, a: val("[data-fl-a]"), b: val("[data-fl-b]") };
  };

  // Shows field `i`'s draft condition in its pane.
  function fill(i) {
    const p = pane(i), d = defs[i], s = read(d, draft[d.field]);
    $("[data-fl-op]", p).value = d.ops.includes(s.op) ? s.op : d.ops[0];
    if (d.type === "list") drawOptions(i, s.picked);
    else {
      for (const [sel, v] of [["[data-fl-a]", s.a], ["[data-fl-b]", s.b]]) {
        const input = $(sel, p);
        input.value = input.hasAttribute("data-fl-num") ? grouped(v ?? "") : v ?? "";
      }
      syncCalendars(i);
    }
    shape(i);
  }
  // The pane's calendars show its dates: the range, or the one day.
  function syncCalendars(i) {
    const p = pane(i), a = $("[data-fl-a]", p).value, b = $("[data-fl-b]", p).value;
    const range = $('[data-fl-cal="range"]', p), single = $('[data-fl-cal="single"]', p);
    range?.__x?.set?.(a || b ? { from: a || null, to: b || null } : null);
    single?.__x?.set?.(a || null);
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
      $("[data-fl-search]", p).hidden = !values || (!d.optionsUrl && options(d).length <= 5);
    } else {
      $("[data-fl-values]", p).hidden = !takesValue(op);
      $("[data-fl-b]", p).hidden = $("[data-fl-and]", p).hidden = op !== "between";
      const fieldB = $("[data-fl-field-b]", p);
      if (fieldB) fieldB.hidden = op !== "between";
      const range = $('[data-fl-cal="range"]', p), single = $('[data-fl-cal="single"]', p);
      if (range) range.hidden = op !== "between";
      if (single) single.hidden = !["on", "before", "after"].includes(op);
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
    const search = $("[data-fl-search]", p);
    const far = d.optionsUrl ? remoteOf(d) : null;
    if (far) lookUp(i, search.value.trim(), far.q === null ? 0 : 250);
    const list = far ? [...far.list, ...picked.filter((v) => !far.list.some((o) => same(o.value, v))).map((v) => far.seen.get(String(v)) ?? { value: v, label: String(v), count: null })] : options(d);
    search.hidden = (!far && list.length <= 5) || !takesList($("[data-fl-op]", p).value);
    const q = far ? "" : search.value.trim().toLowerCase();
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
      none.textContent = far?.loading ? L("Loading…") : far?.failed ? L("Couldn't load the values") : list.length || (far && search.value.trim()) ? L("No matches") : far && far.q !== null && far.seq === 0 ? L("Loading…") : L("No values");
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
  // ---- finding a field: every word typed is in its name or section
  const find = $("[data-fl-find]");
  const unfound = $("[data-fl-unfound]");
  const fold = (t) => String(t ?? "").normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase();
  const shown = () => tabs.filter((t) => !t.hidden);
  function narrow() {
    const words = fold(find?.value).split(/\s+/).filter(Boolean);
    tabs.forEach((t, i) => {
      const text = fold(`${defs[i].label} ${defs[i].section ?? ""}`);
      t.hidden = !words.every((w) => text.includes(w));
    });
    // A section's heading goes with its fields.
    for (const h of $$("[data-fl-section]", panel)) {
      let n = h.nextElementSibling, any = false;
      while (n && !n.hasAttribute("data-fl-section")) {
        if (n.matches("[data-fl-tab]") && !n.hidden) any = true;
        n = n.nextElementSibling;
      }
      h.hidden = !any;
    }
    if (unfound) unfound.hidden = shown().length > 0;
    // The first match is the one Tab reaches.
    const first = shown()[0];
    if (first && tabs[current].hidden) tabs.forEach((t) => (t.tabIndex = t === first ? 0 : -1));
  }
  find?.addEventListener("input", narrow);
  find?.addEventListener("keydown", (e) => {
    const first = shown()[0];
    if (e.key === "Escape" && find.value) {
      // Clears the box; a second Escape closes the panel.
      e.preventDefault();
      find.value = "";
      narrow();
    } else if ((e.key === "ArrowDown" || e.key === "Enter") && first) {
      e.preventDefault();
      const i = tabs.indexOf(first);
      select(i, e.key === "ArrowDown");
      // Enter goes on into the field's controls, as opening the panel does.
      if (e.key === "Enter") focusPane(i);
    }
  });

  function select(i, focus) {
    current = i;
    tabs.forEach((t, j) => {
      t.setAttribute("aria-selected", j === i ? "true" : "false");
      t.tabIndex = j === i ? 0 : -1;
      panes[j].hidden = j !== i;
    });
    if (focus) tabs[i].focus();
  }
  // The field's first control, so typing starts at once.
  function focusPane(i) {
    const p = pane(i);
    ($("[data-fl-search]:not([hidden])", p) || $("[data-fl-pick]", p) || $("[data-fl-a]:not([hidden])", p) || tabs[i]).focus();
  }
  function open(i = current) {
    draft = clone(applied());
    defs.forEach((_, j) => fill(j));
    badges();
    if (find) {
      find.value = "";
      narrow();
    }
    select(i);
    if (!panel.open) panel.showModal();
    focusPane(i);
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
        del.setAttribute("aria-label", L("Delete the {name} view", { name: v.name }));
        del.textContent = "✕";
        li.append(apply, del);
        return li;
      }),
    );
    if (!vs.length) {
      const li = document.createElement("li");
      li.setAttribute("data-fl-view-empty", "");
      li.textContent = L("No saved views yet.");
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
    say(L("Saved the {name} view", { name }));
    queueMicrotask(drawViews);
  }
  function applyView(i) {
    const v = views()[i];
    if (!v) return;
    api.set(cfg.bind, clone(v.filters ?? {}));
    say(L("{name} applied", { name: v.name }));
    toggleViews(false);
    $("[data-fl-views-open]").focus();
  }
  function deleteView(i) {
    const v = views()[i];
    if (!v) return;
    api.set(cfg.bindViews, views().filter((_, j) => j !== i));
    say(L("Deleted the {name} view", { name: v.name }));
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
      syncCalendars(i);
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
    // A calendar's pick: a whole range (or one day) fills the dates.
    const cal = e.target.closest?.("[data-fl-cal]");
    if (cal) {
      if (!e.detail) return;
      if (cal.getAttribute("data-fl-cal") === "range") {
        const r = e.detail.range ?? e.detail.value;
        if (r?.from && !r.to) return;
        $("[data-fl-a]", p).value = r?.from ?? "";
        $("[data-fl-b]", p).value = r?.to ?? "";
      } else {
        const v = e.detail.value;
        $("[data-fl-a]", p).value = typeof v === "string" ? v.slice(0, 10) : "";
      }
      shape(i);
      take(i);
      return;
    }
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
      // Grouped as typed, the caret kept after the same digits.
      const t = e.target;
      if (t.hasAttribute("data-fl-num")) {
        const at = t.selectionStart ?? t.value.length, digits = (t.value.slice(0, at).match(/\d/g) ?? []).length;
        t.value = grouped(t.value);
        let pos = 0, seen = 0;
        while (pos < t.value.length && seen < digits) if (/\d/.test(t.value[pos++])) seen++;
        t.setSelectionRange(pos, pos);
      }
      shape(i);
      take(i);
    }
  });
  // Up and down (and Home, End) move between fields.
  // Only the fields the box shows; Up from the first goes back to the box.
  $("[role=tablist]", panel).addEventListener("keydown", (e) => {
    const list = shown(), i = list.indexOf(e.target);
    if (i < 0) return;
    const to = { ArrowDown: i + 1, ArrowUp: i - 1, Home: 0, End: list.length - 1 }[e.key];
    if (to === undefined) return;
    e.preventDefault();
    if (to < 0 && find) return find.focus();
    select(tabs.indexOf(list[(to + list.length) % list.length]), true);
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
