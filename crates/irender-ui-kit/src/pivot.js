// irender-ui-kit: the `ui-pivot` behaviour. Groups the rows into a tree by the fields down
// the side and another by those across the top, sums each pair of groups, and draws the
// table. The layout (which fields, where, how they're summed, the sort, what's folded) is
// kept in state when bound.
(window.__irenderBehaviors ||= {})["ui-pivot"] = (el, api) => {
  const cfg = api.config, K = window.__irenderKit, L = (t, v) => window.__irender.label(t, v);
  const { make, button, icon, isObj, blank } = K;
  const at = (r, f) => String(f).split(".").reduce((x, k) => (x !== null && typeof x === "object" ? x[k] : undefined), r);
  const collate = new Intl.Collator(cfg.locale || undefined, { numeric: true, sensitivity: "base" }).compare;
  const AGGS = ["sum", "avg", "min", "max", "count", "distinct", "median"];
  // Labels: "sum", "avg", "min", "max", "count", "distinct", "median"
  let local = null, closed = new Set(), lastCell = null;

  const layout = () => {
    const given = (cfg.bind ? api.state[cfg.bind] : local) ?? {};
    const name = (x) => String(isObj(x) ? x.field : x);
    const vals = given.values ?? cfg.values ?? [];
    return {
      down: (given.down ?? cfg.down ?? []).map(name),
      across: (given.across ?? cfg.across ?? []).map(name),
      values: vals.map((v) => (isObj(v) ? v : { field: v })).map((v) => ({ agg: "sum", ...v })),
      sort: given.sort ?? null,
      closed: given.closed ?? null,
    };
  };
  function save(change) {
    const next = { ...layout(), ...change };
    if (next.closed === null) next.closed = [...closed];
    if (cfg.bind) api.set(cfg.bind, next);
    else {
      local = next;
      draw(true);
    }
    api.emit("layout", { layout: next });
  }

  // ---- fields
  const fields = (rows) => {
    const own = Array.isArray(cfg.fields) ? cfg.fields.map((f) => (isObj(f) ? f : { field: f })) : Object.keys(rows[0] ?? {}).filter((k) => !isObj(rows[0][k]) && !Array.isArray(rows[0][k])).map((k) => ({ field: k }));
    return new Map(own.map((f) => [String(f.field), { label: String(f.label ?? f.field), ...f }]));
  };
  // A date's part: what it's grouped by, in order, and how it reads.
  function part(v, p) {
    const d = new Date(String(v).length === 10 ? v + "T00:00:00Z" : v);
    if (blank(v) || isNaN(d)) return null;
    const y = d.getUTCFullYear(), m = d.getUTCMonth() + 1;
    if (p === "year") return { k: String(y), t: String(y) };
    if (p === "quarter") return { k: `${y}-${Math.ceil(m / 3)}`, t: `Q${Math.ceil(m / 3)} ${y}` };
    if (p === "month") return { k: `${y}-${String(m).padStart(2, "0")}`, t: new Intl.DateTimeFormat(cfg.locale || undefined, { month: "short", year: "numeric", timeZone: "UTC" }).format(d) };
    if (p === "week") {
      const t = new Date(d);
      t.setUTCDate(t.getUTCDate() + 4 - (t.getUTCDay() || 7));
      const w = Math.ceil(((t - Date.UTC(t.getUTCFullYear(), 0, 1)) / 86400000 + 1) / 7);
      return { k: `${t.getUTCFullYear()}-${String(w).padStart(2, "0")}`, t: L("Week {n}, {year}", { n: String(w), year: String(t.getUTCFullYear()) }) };
    }
    return { k: d.toISOString().slice(0, 10), t: new Intl.DateTimeFormat(cfg.locale || undefined, { dateStyle: "medium", timeZone: "UTC" }).format(d) };
  }
  const groupOf = (f, v) => {
    if (f?.part) return part(v, f.part) ?? { k: "￿", t: L("(blank)") };
    if (blank(v)) return { k: "￿", t: L("(blank)") };
    return { k: typeof v === "object" ? JSON.stringify(v) : v, t: f?.options?.find?.((o) => isObj(o) && o.value === v)?.label ?? (typeof v === "boolean" ? (v ? L("Yes") : L("No")) : String(v)) };
  };

  // ---- sums
  function agg(rows, v) {
    // A calculated value: its expression over the rows' sums by field, and how many.
    if (v.calc !== undefined) {
      const c = cfg.calcs?.[v.calc];
      if (!c) return null;
      const sums = {};
      for (const r of rows) for (const [f, x] of Object.entries(r ?? {})) if (typeof x === "number" || (!blank(x) && !isNaN(x) && typeof x !== "boolean")) sums[f] = Math.round(((sums[f] ?? 0) + Number(x)) * 1e8) / 1e8;
      const x = window.__irender.evaluate(c, { sum: sums, count: rows.length });
      return typeof x === "number" && Number.isFinite(x) ? Math.round(x * 1e8) / 1e8 : x === "" || x === undefined ? null : x;
    }
    if (v.agg === "count") return rows.length;
    const xs = rows.map((r) => at(r, v.field)).filter((x) => !blank(x));
    if (v.agg === "distinct") return new Set(xs.map((x) => JSON.stringify(x))).size;
    if (v.agg === "first") return xs[0] ?? null;
    if (v.agg === "last") return xs.at(-1) ?? null;
    const ns = xs.map(Number).filter((x) => !isNaN(x));
    if (!ns.length) return null;
    switch (v.agg) {
      case "avg": return ns.reduce((a, b) => a + b, 0) / ns.length;
      case "min": return ns.reduce((a, b) => Math.min(a, b));
      case "max": return ns.reduce((a, b) => Math.max(a, b));
      case "median": {
        const s = ns.sort((a, b) => a - b), m = s.length >> 1;
        return s.length % 2 ? s[m] : (s[m - 1] + s[m]) / 2;
      }
      default: return Math.round(ns.reduce((a, b) => a + b, 0) * 1e8) / 1e8;
    }
  }
  // Groups nested by `names`: `{ key, text, rows, kids, depth, path }`.
  function tree(rows, names, defs) {
    const root = { rows, kids: [], depth: -1, path: [], text: L("Total") };
    const split = (n) => {
      const f = names[n.depth + 1];
      if (f === undefined) return;
      const by = new Map();
      for (const r of n.rows) {
        const g = groupOf(defs.get(f), at(r, f));
        if (!by.has(g.k)) by.set(g.k, { key: g.k, text: g.t, rows: [], kids: [], depth: n.depth + 1, path: [...n.path, g.t], field: f, parent: n });
        by.get(g.k).rows.push(r);
      }
      n.kids = [...by.values()].sort((a, b) => (a.key === "￿") - (b.key === "￿") || (typeof a.key === "number" && typeof b.key === "number" ? a.key - b.key : collate(String(a.key), String(b.key))));
      n.kids.forEach(split);
    };
    split(root);
    return root;
  }
  const id = (n) => JSON.stringify(n.path);
  const leaves = (n) => (n.kids.length ? n.kids.flatMap(leaves) : [n]);

  // Which rows of `a` are in `b`, without going through all of them for every pair.
  const sets = new Map();
  const both = (a, b) => {
    if (b.depth === -1) return a.rows;
    if (a.depth === -1) return b.rows;
    if (!sets.has(b)) sets.set(b, new Set(b.rows));
    const s = sets.get(b);
    return a.rows.filter((r) => s.has(r));
  };
  const passes = (r, filter) => Object.entries(filter).every(([f, c]) => {
    const v = at(r, f);
    if (blank(c) || (Array.isArray(c) && !c.length)) return true;
    if (Array.isArray(c)) return c.some((x) => x === v || String(x) === String(v));
    if (isObj(c)) {
      if (c.contains != null && !String(v ?? "").toLowerCase().includes(String(c.contains).toLowerCase())) return false;
      const cmp = (a, b) => (typeof a === "number" || !isNaN(a) ? Number(a) - Number(b) : String(a) < String(b) ? -1 : String(a) > String(b) ? 1 : 0);
      if (!blank(c.from) && (blank(v) || cmp(v, c.from) < 0)) return false;
      if (!blank(c.to) && (blank(v) || cmp(String(c.to).length === 10 ? String(v).slice(0, String(c.to).length) : v, c.to) > 0)) return false;
      return true;
    }
    return c === v || String(c) === String(v);
  });

  // ---- the table, as cells: used for the page and for Export
  function build() {
    sets.clear();
    const all = K.value(api, cfg.rows) ?? [], filter = K.value(api, cfg.filter);
    const rows = isObj(filter) ? all.filter((r) => passes(r, filter)) : all;
    const defs = fields(all), lay = layout();
    const vals = lay.values.length ? lay.values : [{ field: "", agg: "count", label: L("Count") }];
    const label = (v) => v.label ?? (v.calc !== undefined ? String(v.calc) : v.agg === "count" ? L("Count") : `${defs.get(v.field)?.label ?? v.field} (${L(v.agg)})`);
    const side = tree(rows, lay.down, defs), top = tree(rows, lay.across, defs);
    const cols = lay.across.length ? leaves(top) : [];
    const totals = cfg.totals ?? "both";
    const rowTotals = !cols.length || totals === "both" || totals === "rows";
    if (lay.sort) {
      // By a column's value (the totals', or a group's).
      const v = vals[lay.sort.value] ?? vals[0], col = lay.sort.column ? cols.find((c) => id(c) === lay.sort.column) ?? top : top;
      const go = (n) => {
        const m = new Map(n.kids.map((k) => [k, agg(both(k, col), v)]));
        n.kids.sort((a, b) => (m.get(a) === null) - (m.get(b) === null) || (lay.sort.desc ? -1 : 1) * (typeof m.get(a) === "number" ? m.get(a) - m.get(b) : collate(String(m.get(a)), String(m.get(b)))));
        n.kids.forEach(go);
      };
      go(side);
    }
    const spots = [...cols.map((c) => ({ c, total: false })), ...(rowTotals ? [{ c: top, total: cols.length > 0 }] : [])];
    const value = (n, s, v) => {
      // Running: this group and the ones above it among its parent's, together.
      if (v.show === "running" && n.parent) {
        const sibs = n.parent.kids, upTo = sibs.slice(0, sibs.indexOf(n) + 1).flatMap((k) => both(k, s.c));
        return agg(upTo, v);
      }
      const own = agg(both(n, s.c), v);
      if (!v.show || v.show === "value" || v.show === "running" || typeof own !== "number") return own;
      const of = v.show === "row" ? agg(n.rows, v) : v.show === "column" ? agg(both(side, s.c), v) : agg(rows, v);
      return of ? own / of : null;
    };
    const share = (v) => v.show && v.show !== "value" && v.show !== "running";
    const say = (x, v) => (x === null ? "" : typeof x !== "number" ? String(x) : share(v) ? K.format(x, { format: "percent", decimals: 1, locale: cfg.locale }) : K.format(x, { format: v.format, currency: v.currency, decimals: v.decimals ?? (Number.isInteger(x) ? undefined : 2), locale: cfg.locale }));
    return { rows, defs, lay, vals, label, side, top, cols, spots, value, say, totals };
  }
  const folded = (n) => cfg.subtotals !== false && n.kids.length > 0 && closed.has(id(n));
  function lines(n) {
    return n.kids.flatMap((k) => (k.kids.length ? (cfg.subtotals === false ? lines(k) : [k, ...(folded(k) ? [] : lines(k))]) : [k]));
  }

  let was = null;
  function draw(force) {
    const lay0 = layout();
    if (lay0.closed) closed = new Set(lay0.closed);
    const sig = JSON.stringify([lay0, [...closed], K.value(api, cfg.filter), cfg.bindCell ? null : 0, K.value(api, cfg.emptyText)]);
    const rowsNow = K.value(api, cfg.rows);
    if (!force && was && was.sig === sig && was.rows === rowsNow) return mark();
    was = { sig, rows: rowsNow };
    const m = build(), { vals, side, top, cols, spots, lay } = m;
    const focus = el.contains(document.activeElement) ? document.activeElement.getAttribute("data-key") : null;
    const table = make("table", { "aria-label": cfg.title });
    const head = make("thead");
    // Headings: a row per field across, then the values' names when there's more than one.
    const corner = lay.down.map((f) => m.defs.get(f)?.label ?? f).join(" / ");
    const levels = cols.length ? lay.across.length : 0, depth = levels + (vals.length > 1 || !cols.length ? 1 : 0);
    for (let d = 0; d < Math.max(1, depth); d++) {
      const tr = make("tr");
      if (d === 0) tr.append(make("th", { "data-corner": "", rowspan: Math.max(1, depth), scope: "col" }, corner));
      if (d < levels) {
        const groups = [];
        for (const c of cols) {
          const key = JSON.stringify(c.path.slice(0, d + 1));
          if (groups.at(-1)?.key === key) groups.at(-1).n++;
          else groups.push({ key, n: 1, text: c.path[d], c });
        }
        for (const g of groups) {
          const last = d === levels - 1 && vals.length === 1;
          tr.append(make("th", { scope: last ? "col" : "colgroup", colspan: g.n * vals.length, "data-span": !last, "data-col": last }, last ? sorter(g.c, 0, g.text) : g.text));
        }
        if (d === 0 && spots.at(-1)?.total) tr.append(make("th", { scope: "colgroup", colspan: vals.length, rowspan: vals.length > 1 ? levels : depth, "data-total": "", "data-span": vals.length > 1, "data-col": vals.length === 1 }, vals.length === 1 ? sorter(top, 0, L("Total")) : L("Total")));
      } else for (const s of spots) vals.forEach((v, i) => tr.append(make("th", { scope: "col", "data-col": "", "data-total": s.total }, sorter(s.c, i, m.label(v)))));
      head.append(tr);
    }
    function sorter(c, i, text) {
      const key = c === top ? null : id(c), on = lay.sort && (lay.sort.column ?? null) === key && (lay.sort.value ?? 0) === i;
      return button({ "data-pv-sort": "", "data-key": `sort-${key}-${i}`, "aria-label": L("Sort by {name}", { name: text }), click: () => save({ sort: on ? (lay.sort.desc ? { ...lay.sort, desc: false } : null) : { column: key, value: i, desc: true } }) }, text, on ? icon(lay.sort.desc ? "down" : "up", 11) : null);
    }
    const body = make("tbody"), rows = lines(side);
    // The heat: each value's own range, over the cells that aren't totals.
    const range = vals.map(() => [Infinity, -Infinity]);
    const data = rows.map((n) => spots.map((s) => vals.map((v) => m.value(n, s, v))));
    if (cfg.heat) rows.forEach((n, y) => !n.kids.length && spots.forEach((s, x) => !s.total && vals.forEach((_, i) => {
      const x0 = data[y][x][i];
      if (typeof x0 === "number") range[i] = [Math.min(range[i][0], x0), Math.max(range[i][1], x0)];
    })));
    const cellsOf = (n, values) => spots.flatMap((s, x) => vals.map((v, i) => {
      const val = values[x][i], heat = cfg.heat && !s.total && !n.kids.length && n.depth >= 0 && typeof val === "number" && range[i][1] > range[i][0] ? (val - range[i][0]) / (range[i][1] - range[i][0]) : null;
      const td = make("td", { "data-cell": "", "data-total": s.total, "data-click": !!cfg.bindCell, tabindex: cfg.bindCell ? -1 : null, style: heat === null ? null : `--h:${heat.toFixed(3)}` }, m.say(val, v));
      td.__cell = { down: n.path, across: s.c.path, value: val, field: v.field, rows: () => both(n, s.c) };
      return td;
    }));
    rows.forEach((n, y) => {
      const open = !folded(n);
      const th = make("th", { scope: "row" }, n.kids.length ? button({ "data-pv-fold": "", "data-key": `fold-${id(n)}`, "aria-expanded": String(open), click: () => {
        open ? closed.add(id(n)) : closed.delete(id(n));
        save({ closed: [...closed] });
      } }, icon("right", 12), n.text) : n.text);
      const tr = make("tr", { "data-depth": n.depth, "data-sub": n.kids.length > 0, style: `--d:${n.depth}` }, th, cellsOf(n, data[y]));
      body.append(tr);
    });
    table.append(head, body);
    if (m.totals === "both" || m.totals === "columns") {
      const values = spots.map((s) => vals.map((v) => m.value(side, s, v)));
      if (lay.down.length) table.append(make("tfoot", null, make("tr", null, make("th", { scope: "row" }, L("Total")), cellsOf(side, values))));
      else body.append(make("tr", null, make("th", { scope: "row" }, L("Total")), cellsOf(side, values)));
    }
    const parts = [];
    if (cfg.editable || cfg.export) parts.push(bar(m));
    parts.push(m.rows.length ? make("div", { "data-pv-scroll": "", tabindex: 0, role: "region", "aria-label": cfg.title ?? L("Pivot table") }, table) : make("p", { "data-pv-none": "" }, K.value(api, cfg.emptyText) ?? L("No rows.")));
    const scroll = el.querySelector("[data-pv-scroll]"), pos = scroll && [scroll.scrollLeft, scroll.scrollTop];
    el.replaceChildren(...parts);
    const s2 = el.querySelector("[data-pv-scroll]");
    if (pos && s2) [s2.scrollLeft, s2.scrollTop] = pos;
    // The rows of headings stick below one another.
    let y = 0;
    for (const tr of head.rows) {
      for (const th of tr.cells) th.style.top = y + "px";
      y += tr.getBoundingClientRect().height;
    }
    if (focus) el.querySelector(`[data-key="${CSS.escape(focus)}"]`)?.focus();
    const firstCell = el.querySelector("td[data-click]");
    if (firstCell) firstCell.tabIndex = 0;
    mark();
  }
  function mark() {
    if (!cfg.bindCell) return;
    const c = api.state[cfg.bindCell];
    for (const td of el.querySelectorAll("td[data-click]")) td.setAttribute("aria-selected", String(!!c && JSON.stringify([td.__cell.down, td.__cell.across, td.__cell.field]) === JSON.stringify([c.down, c.across, c.field])));
  }

  // ---- the panel of fields
  function bar(m) {
    const { lay, defs } = m, kids = [];
    if (cfg.editable) {
      const zone = (name, title, list, chip) => {
        const used = new Set(name === "values" ? [] : [...lay.down, ...lay.across]);
        const add = make("select", { "aria-label": L("Add a field to {name}", { name: title }), "data-key": `add-${name}` }, new Option("+ " + L("Add"), ""), [...defs.values()].filter((f) => !used.has(String(f.field))).map((f) => new Option(f.label, String(f.field))));
        add.addEventListener("change", () => add.value && save({ [name]: name === "values" ? [...lay.values, { field: add.value, agg: isNum(m.rows, add.value) ? "sum" : "count" }] : [...lay[name], add.value] }));
        const z = make("div", { "data-pv-zone": name, role: "group", "aria-label": title }, make("small", null, title), list.map(chip), add);
        z.addEventListener("dragover", (e) => drag && (e.preventDefault(), z.setAttribute("data-over", "")));
        z.addEventListener("dragleave", () => z.removeAttribute("data-over"));
        z.addEventListener("drop", (e) => {
          e.preventDefault();
          z.removeAttribute("data-over");
          if (!drag) return;
          const chips = [...z.querySelectorAll("[data-pv-chip]")];
          let i = chips.findIndex((c) => { const b = c.getBoundingClientRect(); return e.clientX < b.left + b.width / 2; });
          if (i < 0) i = chips.length;
          const d = drag, next = { down: [...lay.down], across: [...lay.across], values: [...lay.values] };
          drag = null;
          const [x] = next[d.zone].splice(d.i, 1);
          if (d.zone === name && d.i < i) i--;
          const field = isObj(x) ? x.field : x;
          next[name].splice(i, 0, name === "values" ? (isObj(x) ? x : { field, agg: isNum(m.rows, field) ? "sum" : "count" }) : field);
          if (name !== "values") for (const o of ["down", "across"]) next[o] = next[o].filter((f, k, a) => a.indexOf(f) === k && (o === name || f !== field));
          save(next);
        });
        return z;
      };
      const chipOf = (name) => (x, i) => {
        const field = String(isObj(x) ? x.field : x), text = defs.get(field)?.label ?? field;
        const c = make("span", { "data-pv-chip": "", draggable: "true" }, name === "values" && x.agg === "count" && !field ? L("Count") : text);
        if (name === "values") {
          const s = make("select", { "aria-label": L("How {name} is summed", { name: text }), "data-key": `agg-${i}` }, AGGS.map((a) => new Option(L(a), a, false, a === x.agg)));
          s.addEventListener("change", () => save({ values: lay.values.map((v, k) => (k === i ? { ...v, agg: s.value, label: undefined } : v)) }));
          c.append(s);
        }
        c.append(button({ "aria-label": L("Remove {name}", { name: text }), "data-key": `rm-${name}-${i}`, click: () => save({ [name]: lay[name].filter((_, k) => k !== i) }) }, icon("x", 12)));
        c.addEventListener("dragstart", (e) => {
          drag = { zone: name, i };
          e.dataTransfer.setData("text/plain", text);
          e.dataTransfer.effectAllowed = "move";
        });
        c.addEventListener("dragend", () => (drag = null));
        return c;
      };
      kids.push(zone("down", L("Down"), lay.down, chipOf("down")), zone("across", L("Across"), lay.across, chipOf("across")), zone("values", L("Values"), lay.values, chipOf("values")));
    }
    if (cfg.export) kids.push(button({ "data-pv-export": "", "data-key": "export", click: exportIt }, L("Export")));
    return make("div", { "data-pv-bar": "" }, kids);
  }
  let drag = null;
  const isNum = (rows, f) => rows.slice(0, 50).some((r) => typeof at(r, f) === "number");
  function exportIt() {
    const m = build(), n = Math.max(1, m.lay.down.length);
    const head = [...(m.lay.down.length ? m.lay.down.map((f) => m.defs.get(f)?.label ?? f) : [""]), ...m.spots.flatMap((s) => m.vals.map((v) => [s.total || !m.cols.length ? (m.cols.length ? L("Total") : null) : s.c.path.join(" / "), m.vals.length > 1 || !m.cols.length ? m.label(v) : null].filter(Boolean).join(" · ")))];
    const line = (x, names) => [...names, ...m.spots.flatMap((s) => m.vals.map((v) => m.value(x, s, v)))];
    // Every innermost group, whatever is folded, with its outer groups' names beside it.
    const rows = [head, ...(m.lay.down.length ? leaves(m.side).map((x) => line(x, Array.from({ length: n }, (_, i) => x.path[i] ?? ""))) : []), line(m.side, [L("Total"), ...Array(n - 1).fill("")])];
    const Z = window.__irenderSheets, file = cfg.export;
    const formats = [...Array(n).fill(null), ...m.spots.flatMap(() => m.vals.map((v) => (v.show && v.show !== "value" && v.show !== "running" ? "percent" : v.format === "currency" ? "#,##0.00" : v.format === "percent" ? "percent" : null)))];
    if (/\.xlsx$/.test(file)) Z.save(Z.workbook(rows, cfg.title ?? "Pivot", formats), file);
    else Z.save(new Blob(["﻿" + Z.toCsv(rows) + "\r\n"], { type: "text/csv;charset=utf-8" }), file);
    api.emit("export", { rows: rows.length - 1, file });
  }

  // ---- cells
  const pick = (td) => {
    const c = td.__cell, rows = c.rows();
    lastCell = td;
    if (cfg.bindCell) api.set(cfg.bindCell, { down: c.down, across: c.across, field: c.field, value: c.value, rows });
    api.emit("cell", { down: c.down, across: c.across, field: c.field, value: c.value, rows });
  };
  el.addEventListener("click", (e) => {
    const td = e.target.closest("td[data-click]");
    if (td) pick(td);
  });
  el.addEventListener("keydown", (e) => {
    const td = e.target.closest("td[data-click]");
    if (!td) return;
    const tr = td.parentElement, i = td.cellIndex, rows = [...el.querySelectorAll("tbody tr, tfoot tr")], y = rows.indexOf(tr), next = K.rtl(el) ? -1 : 1;
    const go = (r, c) => {
      const t = rows[K.clamp(r, 0, rows.length - 1)].cells[c];
      if (!t?.matches("td")) return;
      e.preventDefault();
      td.tabIndex = -1;
      t.tabIndex = 0;
      t.focus();
    };
    if (e.key === "ArrowRight") go(y, i + next);
    else if (e.key === "ArrowLeft") go(y, i - next);
    else if (e.key === "ArrowDown") go(y + 1, i);
    else if (e.key === "ArrowUp") go(y - 1, i);
    else if (e.key === "Home") go(y, 1);
    else if (e.key === "End") go(y, tr.cells.length - 1);
    else if (e.key === "Enter" || e.key === " ") e.preventDefault(), pick(td);
  });
  draw();
  return { update: () => draw() };
};
