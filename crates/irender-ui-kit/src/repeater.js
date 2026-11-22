// irender-ui-kit: the `ui-repeater` behaviour. The runtime draws a row per item; this puts
// the items' values in the controls (`data-field`), writes what's typed back to the list in
// state, works out computed fields, totals and what's wrong (cells.js), and adds, copies,
// moves and removes rows.
(window.__irenderBehaviors ||= {})["ui-repeater"] = (el, api) => {
  const cfg = api.config, C = window.__irenderCells, L = (t, v) => window.__irender.label(t, v);
  const loc = cfg.locale || undefined;
  const mine = (n) => n?.closest("[data-i-x=ui-repeater]") === el;
  const $$ = (s) => [...el.querySelectorAll(s)].filter(mine);
  const body = $$("[data-rep-rows]")[0], live = $$("[data-rep-live]")[0];
  const fields = new Map(cfg.fields.map((f) => [f.field, f]));
  // What cells.js checks: an amount is a number, a computed field isn't checked.
  const KIND = { currency: "number", number: "number", date: "date", select: "select", checkbox: "checkbox", multiline: "multiline" };
  const asCell = (f) => ({ ...f, type: KIND[f.type] ?? "text", pattern: f.pattern ?? (f.type === "email" ? "[^\\s@]+@[^\\s@]+\\.[^\\s@]+" : f.type === "url" ? "https?://\\S+" : undefined) });
  const isObj = (v) => v !== null && typeof v === "object" && !Array.isArray(v);
  const list = () => (Array.isArray(api.state[cfg.bind]) ? api.state[cfg.bind] : []);
  const env = () => ({ ...api.state, ...(api.scope() ?? {}) });
  const resolve = (v) => (isObj(v) && "$irender" in v ? window.__irender.evaluate(v.$irender, env()) : v);
  const locked = () => !!resolve(cfg.disabled);
  const touched = new Set(); // "index\u0000field"
  const say = (t) => {
    live.textContent = "";
    setTimeout(() => (live.textContent = t), 30);
  };

  // ---- items
  function computed(item) {
    let out = item;
    for (const f of cfg.fields) {
      if (f.type !== "computed") continue;
      let v = window.__irender.evaluate(f.compute, { ...env(), ...out });
      if (typeof v === "number") v = Math.round(v * 1e8) / 1e8;
      if (out[f.field] !== v) out = { ...out, [f.field]: v };
    }
    return out;
  }
  const set = (items, event) => {
    api.set(cfg.bind, items.map(computed));
    api.emit("rowschange", { rows: list(), ...event });
  };
  function fresh() {
    const item = {};
    for (const f of cfg.fields) {
      if (f.default !== undefined) item[f.field] = f.default;
      else if (f.type === "checkbox") item[f.field] = false;
    }
    return { ...item, ...(cfg.newItem ?? {}) };
  }
  const rowsEls = () => [...body.children].filter((n) => n.hasAttribute("data-rep-row"));
  const controls = (row) => [...row.querySelectorAll("[data-field]")].filter(mine);
  const at = (n) => rowsEls().indexOf(n.closest("[data-rep-row]"));
  const focusRow = (i, sel = "input, select, textarea") => rowsEls()[i]?.querySelector(sel)?.focus();
  function add(i = list().length, item = fresh(), focus = true) {
    if (locked() || (cfg.max != null && list().length >= cfg.max)) return;
    touched.clear();
    set(list().toSpliced(i, 0, item), { added: i });
    if (focus) focusRow(i);
    say(L("{name} added", { name: cfg.itemLabel }));
  }
  function remove(i) {
    if (locked() || list().length <= cfg.min) return;
    touched.clear();
    set(list().toSpliced(i, 1), { removed: i });
    const n = list().length;
    // The row that took its place, or the one before, or Add.
    if (n) focusRow(Math.min(i, n - 1), "[data-rep-remove]");
    else $$("[data-rep-add]")[0].focus();
    say(L("{name} removed", { name: cfg.itemLabel }));
  }
  function move(i, to) {
    const n = list().length;
    if (locked() || to < 0 || to >= n || to === i) return false;
    const next = [...list()], [x] = next.splice(i, 1);
    next.splice(to, 0, x);
    touched.clear();
    set(next, { moved: to, from: i });
    say(L("{name} moved to {n} of {total}", { name: cfg.itemLabel, n: to + 1, total: n }));
    return true;
  }

  // ---- values
  function read(c, f) {
    if (c.type === "checkbox") return c.checked;
    if (f?.type === "select") return c.value === "" ? null : C.choices(f)[Number(c.value)]?.value ?? null;
    if (f?.type === "number" || f?.type === "currency") return C.parse({ type: "number" }, c.value, cfg.locale);
    return c.value === "" ? null : c.value;
  }
  const money = (f) => f.format === "currency" || f.type === "currency";
  function fmt(f, v) {
    if (v === null || v === undefined || v === "") return "";
    if (typeof v !== "number") return typeof v === "object" ? JSON.stringify(v) : String(v);
    const o = money(f) && f.currency ? { style: "currency", currency: f.currency } : f.format === "percent" ? { style: "percent" } : {};
    if (f.decimals != null) o.minimumFractionDigits = o.maximumFractionDigits = f.decimals;
    return v.toLocaleString(loc, o);
  }
  function write(c, f, v) {
    const blank = v === null || v === undefined;
    if (c.tagName === "OUTPUT") c.textContent = f ? fmt(f, v) : blank ? "" : String(v);
    else if (c.type === "checkbox") c.checked = v === true || v === "true" || v === 1;
    else if (f?.type === "select") {
      const k = blank ? -1 : C.choices(f).findIndex((o) => o.value === v);
      c.value = k < 0 ? "" : String(k);
    } else if (c !== document.activeElement) {
      // An amount shows its decimals until it's edited.
      const n = typeof v === "number" && f && (f.type === "currency" || f.decimals != null)
        ? v.toLocaleString(loc, { minimumFractionDigits: f.decimals ?? 2, maximumFractionDigits: f.decimals ?? 2 })
        : typeof v === "number" ? v.toLocaleString(loc, { maximumFractionDigits: 20, useGrouping: f?.type === "number" }) : blank ? "" : String(v);
      if (c.value !== n) c.value = n;
    }
  }
  function changed(c) {
    const i = at(c), name = c.getAttribute("data-field"), items = list();
    if (i < 0 || i >= items.length) return;
    const v = read(c, fields.get(name));
    if (items[i]?.[name] === v) return;
    set(items.with(i, { ...items[i], [name]: v }), { edited: i, field: name });
  }

  // ---- drawing
  let ids = 0;
  const base = el.id || `rep-${(window.__irenderIds = (window.__irenderIds ?? 0) + 1)}`;
  function draw() {
    const items = list();
    // Too few, or a computed field that's out of date: put right first.
    if (items.length < cfg.min) {
      return void api.set(cfg.bind, [...items, ...Array.from({ length: cfg.min - items.length }, fresh)].map(computed));
    }
    const now = items.map(computed);
    if (now.some((x, i) => x !== items[i])) return void api.set(cfg.bind, now);
    const off = locked(), all = !!resolve(cfg.showErrors);
    const uses = new Map(cfg.fields.filter((f) => f.unique).map((f) => [f.field, C.count(items.map((x) => x?.[f.field]))]));
    const wrong = [];
    rowsEls().forEach((row, i) => {
      const item = items[i] ?? {};
      row.setAttribute("aria-label", L("{name} {n} of {total}", { name: cfg.itemLabel, n: i + 1, total: items.length }));
      for (const c of controls(row)) {
        const name = c.getAttribute("data-field"), f = fields.get(name);
        write(c, f, item[name]);
        if (c.tagName !== "OUTPUT") c.disabled = off;
        if (!f || f.type === "computed") continue;
        const cell = c.closest("[data-rep-cell]"), msg = cell.querySelector("[data-rep-error]"), label = cell.querySelector("label");
        if (!c.id) {
          c.id = `${base}-f${++ids}`;
          label.htmlFor = c.id;
          msg.id = c.id + "-error";
        }
        const m = C.check(asCell(f), item[name], uses.get(name));
        if (m) wrong.push({ index: i, field: name, message: m });
        const show = m && (all || touched.has(i + "\u0000" + name));
        msg.textContent = show ? m : "";
        if (show) c.setAttribute("aria-invalid", "true"), c.setAttribute("aria-describedby", msg.id);
        else c.removeAttribute("aria-invalid"), c.removeAttribute("aria-describedby");
      }
      for (const b of row.querySelectorAll("[data-rep-remove]")) if (mine(b)) b.disabled = off || items.length <= cfg.min;
      for (const b of row.querySelectorAll("[data-rep-copy]")) if (mine(b)) b.disabled = off || (cfg.max != null && items.length >= cfg.max);
      for (const b of row.querySelectorAll("[data-rep-grip]")) if (mine(b)) b.disabled = off || items.length < 2;
    });
    const addB = $$("[data-rep-add]")[0];
    addB.disabled = off || (cfg.max != null && items.length >= cfg.max);
    const empty = $$("[data-rep-empty]")[0];
    if (empty) empty.hidden = items.length > 0;
    const head = $$("[data-rep-head]")[0];
    if (head) head.style.display = items.length ? "" : "none";
    const totals = {};
    for (const f of cfg.fields) {
      if (!f.summary) continue;
      const vs = items.map((x) => x?.[f.field]).filter((v) => v !== null && v !== undefined && v !== "" && !isNaN(v)).map(Number);
      const sum = vs.reduce((a, b) => a + b, 0);
      const v = f.summary === "count" ? items.length : f.summary === "sum" ? sum : !vs.length ? null : f.summary === "avg" ? sum / vs.length : Math[f.summary](...vs);
      totals[f.field] = v === null ? null : Math.round(v * 1e8) / 1e8;
      const out = $$(`[data-total="${CSS.escape(f.field)}"] b`)[0];
      const text = v === null ? "—" : f.summary === "count" ? String(v) : fmt(f, totals[f.field]);
      if (out && out.textContent !== text) out.textContent = text;
    }
    if (cfg.totals && JSON.stringify(api.state[cfg.totals]) !== JSON.stringify(totals)) api.set(cfg.totals, totals);
    if (cfg.errors && JSON.stringify(api.state[cfg.errors] ?? []) !== JSON.stringify(wrong)) api.set(cfg.errors, wrong);
  }

  // ---- events
  el.addEventListener("input", (e) => mine(e.target) && e.target.matches("[data-field]") && e.target.tagName !== "SELECT" && changed(e.target));
  el.addEventListener("change", (e) => mine(e.target) && e.target.matches?.("[data-field]") && changed(e.target));
  el.addEventListener("focusout", (e) => {
    if (!e.target.matches?.("[data-field]") || !mine(e.target) || !e.target.isConnected) return;
    const i = at(e.target);
    // A row just moved or removed wasn't left by the person.
    if (i < 0 || el.contains(e.relatedTarget) && e.relatedTarget.matches("[data-rep-remove]")) return;
    touched.add(i + "\u0000" + e.target.getAttribute("data-field"));
    setTimeout(draw);
  });
  el.addEventListener("click", (e) => {
    const b = e.target.closest("button");
    if (!b || !mine(b)) return;
    if (b.hasAttribute("data-rep-add")) add();
    else if (b.hasAttribute("data-rep-remove")) remove(at(b));
    else if (b.hasAttribute("data-rep-copy")) add(at(b) + 1, { ...list()[at(b)] });
  });
  el.addEventListener("keydown", (e) => {
    const t = e.target;
    if (!mine(t)) return;
    if (t.matches("[data-rep-grip]") && (e.key === "ArrowUp" || e.key === "ArrowDown") && e.altKey) {
      e.preventDefault();
      const i = at(t), to = i + (e.key === "ArrowDown" ? 1 : -1);
      if (move(i, to)) focusRow(to, "[data-rep-grip]");
    } else if (t.matches("input[data-field]") && e.key === "Enter" && !e.isComposing && t.type !== "checkbox") {
      // Enter goes to the same field of the next row, adding one after the last.
      e.preventDefault();
      const i = at(t), name = t.getAttribute("data-field");
      if (i === list().length - 1) add(undefined, undefined, false);
      focusRow(i + 1, `[data-field="${CSS.escape(name)}"]`);
    }
  });
  // Dragging a row by its grip: it moves as the pointer passes its neighbours.
  let drag = null;
  el.addEventListener("pointerdown", (e) => {
    const g = e.target.closest("[data-rep-grip]");
    if (!g || !mine(g) || e.button !== 0 || locked()) return;
    drag = { id: e.pointerId, i: at(g) };
    rowsEls()[drag.i].setAttribute("data-moving", "");
    try {
      g.setPointerCapture(e.pointerId);
    } catch { /* a pointer that's already gone */ }
  });
  el.addEventListener("pointermove", (e) => {
    if (!drag || e.pointerId !== drag.id) return;
    const rs = rowsEls();
    const to = rs.findIndex((r) => {
      const b = r.getBoundingClientRect();
      return e.clientY >= b.top && e.clientY <= b.bottom;
    });
    if (to < 0 || to === drag.i) return;
    // Past the middle, so rows of different heights don't swap back and forth.
    const b = rs[to].getBoundingClientRect(), mid = b.top + b.height / 2;
    if (to > drag.i ? e.clientY < mid : e.clientY > mid) return;
    if (move(drag.i, to)) {
      rs[drag.i].removeAttribute("data-moving");
      rowsEls()[to].setAttribute("data-moving", "");
      drag.i = to;
    }
  });
  const drop = () => {
    if (!drag) return;
    rowsEls().forEach((r) => r.removeAttribute("data-moving"));
    drag = null;
  };
  el.addEventListener("pointerup", drop);
  el.addEventListener("pointercancel", drop);

  // The first draw waits for the runtime to have drawn the rows.
  return { update: draw };
};
