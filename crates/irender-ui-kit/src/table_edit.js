// irender-ui-kit: `ui-data-table`'s editable cells, used by data_table.js when a column
// has `edit`. Cells are moved through with the arrow keys and edited in place with Enter,
// F2, a double click or by typing; ranges are picked with Shift, copied and pasted as a
// spreadsheet does, filled down (⌘D, or by dragging the picked cells' corner) and cleared. Edits go to the state the rows come from
// (or, with `url`, to the rows loaded), each with a `celledit` event; undo takes them back.
// The editor floats over its cell, since the runtime writes the cell's own text.
window.__irenderTableEdit = (t) => {
  const { el, api, cfg, $, $$, body, scroller, table, at, blank, rowKey, keyOf, indexOf, side, L } = t;
  const C = window.__irenderCells, B = cfg.bind, E = cfg.edit;
  const columns = new Map(cfg.columns.filter((c) => c.edit).map((c) => [c.key, { ...c.edit, field: c.field, key: c.key, label: c.label }]));
  const loc = cfg.locale || undefined;
  let cur = null, anchor = null, open = null, undo = [], redo = [], wrote = null, held = false;

  // ---- where things are
  const colKeys = () => [...table.tHead.rows].find((r) => !r.hasAttribute("data-dt-bands")).querySelectorAll("th[data-dt-col]:not([hidden])");
  const keys = () => [...colKeys()].map((th) => th.getAttribute("data-dt-col"));
  const shownKeys = () => t.shown().map((i) => keyOf(rowKey(t.data()[i], i)));
  const trAt = (pos) => {
    const i = t.shown()[pos];
    return i === undefined ? null : $$("[data-dt-body] > [data-dt-row]:not([hidden])").find((tr) => indexOf(tr) === i) ?? null;
  };
  const cellAt = (pos, ck) => trAt(pos)?.querySelector(`:scope > [data-dt-col="${CSS.escape(ck)}"]`) ?? null;
  const posOf = (k) => shownKeys().indexOf(k);
  const spot = (td) => {
    const tr = td.closest("[data-dt-row]"), i = indexOf(tr);
    return { k: keyOf(rowKey(t.data()[i], i)), c: td.getAttribute("data-dt-col") };
  };
  // The cell of this table an event came from (not of a table inside a detail row).
  const cellOf = (n) => {
    const td = n.closest?.("td[data-dt-col]");
    return td && td.closest("[data-dt-row]")?.parentElement === body ? td : null;
  };
  // The picked cells: rows and columns from the anchor to the cursor.
  function range() {
    if (!cur) return null;
    const ks = keys(), a = anchor ?? cur;
    const r = [posOf(a.k), posOf(cur.k)].sort((x, y) => x - y), c = [ks.indexOf(a.c), ks.indexOf(cur.c)].sort((x, y) => x - y);
    return r[0] < 0 || c[0] < 0 ? null : { r0: r[0], r1: r[1], c0: c[0], c1: c[1], ks };
  }

  // ---- writing
  const source = () => (t.remote ? t.remoteRows() : Array.isArray(api.state[E.rows]) ? api.state[E.rows] : []);
  const changes = () => (B.changes && api.state[B.changes] && typeof api.state[B.changes] === "object" ? api.state[B.changes] : {});
  function store(rows, changed) {
    wrote = rows;
    if (B.changes && changed) api.set(B.changes, changed);
    if (t.remote) t.setRemote(rows);
    else api.set(E.rows, rows);
  }
  const setIn = (row, path, v) => {
    const [head, ...rest] = path;
    return { ...(row ?? {}), [head]: rest.length ? setIn(row?.[head], rest, v) : v };
  };
  // `records`: the rows are a source's records, and what's changed in them is sent
  // through its `write` (data.js), a field at a time, once it's right. A row added here
  // is made there, and known by the id it's given.
  const D = E.records ? window.__irenderData : null, made = new Map();
  const idOf = (row, key) => made.get(keyOf(key)) ?? at(row, D.key(E.records)) ?? key;
  function through(events) {
    if (!D) return;
    validate();
    for (const e of events) {
      const c = [...columns.values()].find((x) => x.field === e.field);
      if (c && faults.has(keyOf(e.key) + "\u0000" + c.key)) continue;
      const head = e.field.split(".")[0];
      D.save(E.records, idOf(e.row, e.key), { [head]: e.row[head] ?? null });
    }
  }
  // Edits, `[{ i, c, value }]`, as one step to undo.
  function write(list) {
    const before = source(), rows = [...before], events = [], changed = { ...changes() };
    for (const { i, c, value } of list) {
      const was = at(rows[i], c.field);
      if (was === value || (blank(was) && blank(value))) continue;
      rows[i] = setIn(rows[i], c.field.split("."), value);
      const key = rowKey(rows[i], i);
      changed[key] = { ...changed[key], [c.field]: value };
      events.push({ key, row: rows[i], field: c.field, value, previous: was ?? null });
    }
    if (!events.length) return false;
    step(before);
    store(rows, changed);
    for (const e of events) api.emit("celledit", e);
    through(events);
    return true;
  }
  // Remembers the rows as they were; rows changed by something else can't be undone to.
  function step(before) {
    if (wrote !== null && before !== wrote) undo = [];
    undo.push({ rows: before, changed: changes() });
    if (undo.length > 100) undo.shift();
    redo = [];
  }
  function back(from, to) {
    if (!from.length || source() !== wrote) return t.announce(L("Nothing to undo"));
    const s = from.pop();
    to.push({ rows: source(), changed: changes() });
    store(s.rows, s.changed);
    api.emit("rowschange", { rows: s.rows });
  }
  function addRow() {
    const before = source(), used = new Set(before.map((r, i) => keyOf(rowKey(r, i))));
    let row = { ...(E.newRow ?? {}) };
    if (blank(at(row, cfg.rowKey))) {
      let n = before.length + 1;
      while (used.has(keyOf(`new-${n}`))) n++;
      row = setIn(row, cfg.rowKey.split("."), `new-${n}`);
    }
    const rows = [...before, row], key = rowKey(row, rows.length - 1);
    step(before);
    store(rows, { ...changes(), [key]: { ...row } });
    api.emit("rowadd", { key, row });
    if (D) {
      const { [cfg.rowKey]: _, ...values } = row;
      D.create(E.records, values).then((r) => r?.id && made.set(keyOf(key), r.id));
    }
    const k = keyOf(key);
    t.reveal(rows.length - 1);
    const c = keys().find((c) => columns.has(c));
    if (posOf(k) < 0 || !c) return t.announce(L("Row added"));
    go(posOf(k), c);
    begin();
  }
  function removeRows(ks) {
    const gone = new Set(ks.map(keyOf)), before = source();
    const out = before.filter((r, i) => gone.has(keyOf(rowKey(r, i))));
    if (!out.length) return;
    const rows = before.filter((r, i) => !gone.has(keyOf(rowKey(r, i))));
    step(before);
    const changed = { ...changes() };
    for (const k of ks) changed[k] = null; // gone
    store(rows, changed);
    if (D) out.forEach((r) => D.remove(E.records, idOf(r, rowKey(r, before.indexOf(r)))));
    t.clearSelection();
    api.emit("rowdelete", { keys: ks, rows: out });
    t.announce(L(["{n} row deleted", "{n} rows deleted"], { n: out.length }));
  }

  // ---- checking
  let checked = null, faults = new Map(), outside = null;
  function validate() {
    const rows = source(), given = t.resolve(cfg.errors) ?? null;
    if (checked === rows && JSON.stringify(given) === outside) return;
    checked = rows;
    outside = JSON.stringify(given);
    faults = new Map();
    const list = [];
    const uses = new Map([...columns.values()].filter((c) => c.unique).map((c) => [c.key, C.count(rows.map((r) => at(r, c.field)))]));
    rows.forEach((r, i) => {
      const key = rowKey(r, i);
      for (const c of columns.values()) {
        const m = C.check(c, at(r, c.field), uses.get(c.key)) ?? given?.[key]?.[c.field] ?? null;
        if (!m) continue;
        faults.set(keyOf(key) + "\u0000" + c.key, String(m));
        list.push({ key, field: c.field, message: String(m) });
      }
    });
    if (B.errors && JSON.stringify(api.state[B.errors] ?? []) !== JSON.stringify(list)) api.set(B.errors, list);
  }

  // ---- drawing
  const tip = Object.assign(document.createElement("div"), { hidden: true });
  tip.setAttribute("data-dt-tip", "");
  scroller.append(tip);
  // The fill handle: the picked cells' corner, dragged up or down to repeat them.
  const handle = Object.assign(document.createElement("div"), { hidden: true });
  handle.setAttribute("data-dt-fill", "");
  handle.setAttribute("aria-hidden", "true");
  scroller.append(handle);
  let filling = null;
  // The table, and the corner's cell: columns sharing out a full-width table change the cell.
  // Placed in the next frame, not in the observer's own turn, which placing would make loop.
  let queued = 0;
  const sized = typeof ResizeObserver === "function" ? new ResizeObserver(() => (queued ||= requestAnimationFrame(() => ((queued = 0), placeHandle())))) : null;
  sized?.observe(table);
  let watched = null;
  function draw() {
    validate();
    const rows = t.data(), changed = changes(), ks = shownKeys();
    if (cur && ks.length && !ks.includes(cur.k)) cur = anchor = null;
    const r = range(), first = keys()[0];
    const multi = r && (r.r0 !== r.r1 || r.c0 !== r.c1);
    for (const tr of $$("[data-dt-body] > [data-dt-row]:not([hidden])")) {
      const i = indexOf(tr), key = rowKey(rows[i], i), k = keyOf(key), pos = ks.indexOf(k);
      for (const td of tr.querySelectorAll(":scope > td[data-dt-col]")) {
        const ck = td.getAttribute("data-dt-col"), c = columns.get(ck), here = cur && cur.k === k && cur.c === ck;
        td.tabIndex = cur ? (here ? 0 : -1) : pos === 0 && ck === first ? 0 : -1;
        td.toggleAttribute("data-dt-cur", !!here);
        td.toggleAttribute("data-dt-in", !!multi && pos >= r.r0 && pos <= r.r1 && r.ks.indexOf(ck) >= r.c0 && r.ks.indexOf(ck) <= r.c1);
        td.toggleAttribute("data-dt-fill-to", !!filling && pos >= filling.p0 && pos <= filling.p1 && r.ks.indexOf(ck) >= r.c0 && r.ks.indexOf(ck) <= r.c1);
        if (!c) {
          td.setAttribute("aria-readonly", "true");
          continue;
        }
        td.setAttribute("data-dt-editable", c.type);
        const bad = faults.get(k + "\u0000" + ck);
        td.toggleAttribute("data-dt-bad", !!bad);
        if (bad) td.setAttribute("aria-invalid", "true"), td.setAttribute("aria-description", bad);
        else td.removeAttribute("aria-invalid"), td.removeAttribute("aria-description");
        const ch = changed[key];
        td.toggleAttribute("data-dt-dirty", !!ch && Object.hasOwn(ch, c.field));
      }
    }
    const td = cur && cellAt(posOf(cur.k), cur.c), bad = td && faults.get(cur.k + "\u0000" + cur.c);
    tip.hidden = !bad || !!open || !td.contains(document.activeElement);
    if (!tip.hidden) {
      tip.textContent = bad;
      place(tip, td, td.offsetHeight);
    }
    if (open) placeEditor();
    placeHandle();
  }
  // The fill handle on the last picked cell's corner, while the cells have the focus. Placed
  // again whenever the table's size changes (columns settling as fonts load, say).
  function placeHandle() {
    const r = range(), end = r && cellAt(r.r1, r.ks[r.c1]);
    handle.hidden = !end || !!open || (!filling && !el.contains(document.activeElement)) || ![...Array(r.c1 - r.c0 + 1)].some((_, x) => columns.has(r.ks[r.c0 + x]));
    if (handle.hidden) return;
    if (sized && watched !== end) {
      if (watched) sized.unobserve(watched);
      sized.observe((watched = end));
    }
    const a = place(handle, end, 0);
    handle.style.left = parseFloat(handle.style.left) + a.width + "px";
    handle.style.top = parseFloat(handle.style.top) + a.height + "px";
  }
  // Over a cell, in the scroll area's own coordinates, so it scrolls with the rows.
  function place(n, td, below = 0) {
    const a = td.getBoundingClientRect(), b = scroller.getBoundingClientRect();
    n.style.left = a.left - b.left - scroller.clientLeft + scroller.scrollLeft + "px";
    n.style.top = a.top - b.top - scroller.clientTop + scroller.scrollTop + below + "px";
    return a;
  }
  function placeEditor() {
    const td = cellAt(posOf(open.k), open.c.key);
    if (!td) return;
    const a = place(open.input, td);
    open.input.style.width = a.width + "px";
    open.input.style[open.c.type === "multiline" ? "minHeight" : "height"] = a.height + "px";
  }

  // ---- moving
  function go(pos, c, extend = false) {
    const n = t.shown().length, ks = keys();
    if (!n) return;
    pos = Math.max(0, Math.min(n - 1, pos));
    if (!trAt(pos)) t.scrollTo(pos);
    const k = shownKeys()[pos];
    if (extend) anchor ??= cur;
    else anchor = null;
    cur = { k, c };
    draw();
    const td = cellAt(pos, ks.includes(c) ? c : ks[0]);
    if (!td) return;
    td.focus({ preventScroll: true });
    reveal(td);
    draw();
  }
  // Into view, clear of the header and the pinned columns.
  function reveal(td) {
    const head = table.tHead.getBoundingClientRect().height, b = scroller.getBoundingClientRect(), a = td.getBoundingClientRect();
    if (a.top < b.top + head) scroller.scrollTop -= b.top + head - a.top;
    else if (a.bottom > b.bottom) scroller.scrollTop += a.bottom - b.bottom;
    if (!td.hasAttribute("data-pin")) {
      const pins = [...td.parentElement.querySelectorAll(":scope > [data-pin]:not([hidden])")].map((p) => [p.getAttribute("data-pin") === "left", p.offsetWidth]);
      const rtl = getComputedStyle(el).direction === "rtl";
      const sum = (left) => pins.filter(([l]) => l === left).reduce((s, [, w]) => s + w, 0);
      const l = b.left + (rtl ? sum(false) : sum(true)), r = b.right - (rtl ? sum(true) : sum(false));
      if (a.left < l) scroller.scrollLeft -= l - a.left;
      else if (a.right > r) scroller.scrollLeft += a.right - r;
    }
    td.scrollIntoView({ block: "nearest", inline: "nearest" });
  }
  // The next editable cell, across rows.
  function nextCell(dir) {
    const ks = keys(), n = t.shown().length;
    let p = posOf(cur.k), c = ks.indexOf(cur.c);
    for (let guard = 0; guard < ks.length * n; guard++) {
      c += dir;
      if (c < 0 || c >= ks.length) {
        c = dir > 0 ? 0 : ks.length - 1;
        p += dir;
      }
      if (p < 0 || p >= n) return null;
      if (columns.has(ks[c])) return [p, ks[c]];
    }
    return null;
  }

  // ---- editing
  const rowIndex = (k) => t.shown()[posOf(k)];
  const may = (i) => {
    const w = E.when ? t.test(E.when, t.data()[i]) : true;
    return w && t.isDataRow(t.data()[i]);
  };
  function begin(text) {
    const c = cur && columns.get(cur.c), i = cur ? rowIndex(cur.k) : undefined;
    if (!c || i === undefined || open || !may(i)) return;
    const was = at(t.data()[i], c.field);
    if (c.type === "checkbox") return void (write([{ i, c, value: !(was === true || was === "true" || was === 1) }]) && refocus());
    const input = document.createElement(c.type === "select" ? "select" : c.type === "multiline" ? "textarea" : "input");
    input.setAttribute("data-dt-editor", "");
    input.setAttribute("aria-label", c.label || c.field);
    if (c.type === "select") {
      const all = C.choices(c), list = all.length ? all : [...new Set(source().map((r) => at(r, c.field)).filter((v) => !blank(v)))].sort().map((v) => ({ value: v, label: String(v) }));
      if (!c.required || blank(was)) list.unshift({ value: null, label: "" });
      if (!blank(was) && !list.some((o) => o.value === was)) list.unshift({ value: was, label: String(was) });
      list.forEach((o, n) => input.append(new Option(o.label, String(n), false, o.value === was || (blank(o.value) && blank(was)))));
      input.__list = list;
    } else {
      if (c.type === "date") input.type = /^\d{4}-\d{2}-\d{2}$/.test(String(was ?? "")) || blank(was) ? "date" : "text";
      if (c.type === "number") input.inputMode = "decimal";
      input.value = text ?? (blank(was) ? "" : typeof was === "object" ? JSON.stringify(was) : c.type === "number" && typeof was === "number" ? was.toLocaleString(loc, { useGrouping: false, maximumFractionDigits: 20 }) : String(was));
      if (c.type === "multiline") input.rows = 4;
    }
    open = { input, c, k: cur.k, i };
    scroller.append(input);
    placeEditor();
    tip.hidden = true;
    input.focus({ preventScroll: true });
    if (text === undefined && input.select) input.select();
    if (c.type === "select" || input.type === "date") {
      try {
        input.showPicker?.();
      } catch { /* only right after a person's own click or key */ }
    }
    if (c.type === "select") input.addEventListener("change", () => end(true));
    input.addEventListener("blur", () => open?.input === input && end(true, false));
  }
  function end(commit, focus = true) {
    if (!open) return;
    const { input, c, k } = open, i = rowIndex(k);
    open = null;
    let value;
    if (commit && i !== undefined) {
      value = c.type === "select" ? input.__list[Number(input.value)]?.value ?? null : C.parse(c, input.value, cfg.locale);
    }
    input.remove();
    if (commit && i !== undefined) write([{ i, c, value }]);
    if (focus) refocus();
    else draw();
  }
  // The cursor's cell again, wherever its row went; the nearest when it's filtered out.
  function refocus(pos = null) {
    const p = cur ? posOf(cur.k) : -1;
    if (p >= 0) go(p, cur.c, !!anchor);
    else if (pos !== null) go(pos, keys()[0]);
  }

  // ---- ranges
  const cellText = (i, ck) => {
    const c = cfg.columns.find((c) => c.key === ck), v = c?.field ? at(t.data()[i], c.field) : null;
    return blank(v) ? "" : typeof v === "object" ? JSON.stringify(v) : String(v);
  };
  const quote = (s) => (/[\t\n"]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s);
  function copyText() {
    const r = range();
    if (!r) return "";
    const lines = [];
    for (let p = r.r0; p <= r.r1; p++) lines.push(r.ks.slice(r.c0, r.c1 + 1).map((ck) => quote(cellText(t.shown()[p], ck))).join("\t"));
    return lines.join("\n");
  }
  // Tab-separated text, quoted fields included, as spreadsheets copy it.
  function parseTsv(s) {
    const out = [[]];
    let field = "", quoted = false;
    for (let i = 0; i < s.length; i++) {
      const ch = s[i];
      if (quoted) {
        if (ch === '"' && s[i + 1] === '"') field += '"', i++;
        else if (ch === '"') quoted = false;
        else field += ch;
      } else if (ch === '"' && field === "") quoted = true;
      else if (ch === "\t") out.at(-1).push(field), (field = "");
      else if (ch === "\n" || ch === "\r") {
        if (ch === "\r" && s[i + 1] === "\n") i++;
        out.at(-1).push(field), (field = "");
        out.push([]);
      } else field += ch;
    }
    out.at(-1).push(field);
    if (out.length > 1 && out.at(-1).length === 1 && out.at(-1)[0] === "") out.pop();
    return out;
  }
  function paste(text) {
    const r = range(), lines = parseTsv(text);
    if (!r) return;
    const list = [], one = lines.length === 1 && lines[0].length === 1;
    const h = one ? r.r1 - r.r0 + 1 : lines.length, w = one ? r.c1 - r.c0 + 1 : Math.max(...lines.map((l) => l.length));
    for (let y = 0; y < h; y++) {
      for (let x = 0; x < w; x++) {
        const i = t.shown()[r.r0 + y], c = columns.get(r.ks[r.c0 + x]), s = one ? lines[0][0] : lines[y][x];
        if (i === undefined || !c || s === undefined || !may(i)) continue;
        list.push({ i, c, value: C.parse(c, s, cfg.locale) });
      }
    }
    // More rows than the table shows are new ones, when it takes them.
    const extra = one || !E.addRow ? [] : lines.slice(Math.max(0, t.shown().length - r.r0));
    if (write(list)) t.announce(L(["{n} cell pasted", "{n} cells pasted"], { n: list.length }));
    if (extra.length) {
      const before = source(), rows = [...before];
      let n = rows.length;
      const used = new Set(before.map((x, i) => keyOf(rowKey(x, i)))), changed = { ...changes() };
      for (const line of extra) {
        let row = { ...(E.newRow ?? {}) };
        line.forEach((s, x) => {
          const c = columns.get(r.ks[r.c0 + x]);
          if (c) row = setIn(row, c.field.split("."), C.parse(c, s, cfg.locale));
        });
        if (blank(at(row, cfg.rowKey))) {
          do n++;
          while (used.has(keyOf(`new-${n}`)));
          row = setIn(row, cfg.rowKey.split("."), `new-${n}`);
        }
        rows.push(row);
        changed[rowKey(row, rows.length - 1)] = { ...row };
      }
      // One step with the cells pasted before them.
      if (!list.length) step(before);
      store(rows, changed);
      api.emit("rowschange", { rows });
    }
    refocus();
  }
  function fill(value) {
    const r = range(), list = [];
    if (!r) return;
    for (let p = r.r0; p <= r.r1; p++) {
      for (let x = r.c0; x <= r.c1; x++) {
        const i = t.shown()[p], c = columns.get(r.ks[x]);
        if (c && may(i)) list.push({ i, c, value: value(c, p, r) });
      }
    }
    write(list);
    refocus();
  }

  // ---- the fill handle
  handle.addEventListener("pointerdown", (e) => {
    const r = range();
    if (!r || e.button !== 0) return;
    e.preventDefault();
    filling = { id: e.pointerId, r, p0: r.r0, p1: r.r1 };
    try {
      handle.setPointerCapture(e.pointerId);
    } catch { /* a pointer that's already gone */ }
  });
  handle.addEventListener("pointermove", (e) => {
    if (!filling || e.pointerId !== filling.id) return;
    // The row under the pointer, by height: the handle has the pointer, not the cells.
    const trs = t.shown().map((_, p) => trAt(p));
    let p = trs.findIndex((tr) => tr && e.clientY < tr.getBoundingClientRect().bottom);
    if (p < 0) p = trs.length - 1;
    const { r } = filling;
    [filling.p0, filling.p1] = p > r.r1 ? [r.r1 + 1, p] : p < r.r0 ? [p, r.r0 - 1] : [r.r0, r.r1];
    draw();
  });
  const fillEnd = (e) => {
    if (!filling || e.pointerId !== filling.id) return;
    const { r, p0, p1 } = filling, h = r.r1 - r.r0 + 1, list = [];
    filling = null;
    if (p0 >= r.r0 && p1 <= r.r1) return draw();
    // The picked rows repeat, in order, away from them: down from the first, up from the last.
    for (let p = p0; p <= p1; p++) {
      const from = t.shown()[p > r.r1 ? r.r0 + ((p - r.r1 - 1) % h) : r.r1 - ((r.r0 - 1 - p) % h)], i = t.shown()[p];
      for (let x = r.c0; x <= r.c1; x++) {
        const c = columns.get(r.ks[x]);
        if (c && may(i)) list.push({ i, c, value: at(t.data()[from], c.field) ?? null });
      }
    }
    write(list);
    anchor = { k: shownKeys()[Math.min(p0, r.r0)], c: r.ks[r.c0] };
    cur = { k: shownKeys()[Math.max(p1, r.r1)], c: r.ks[r.c1] };
    refocus();
    t.announce(L(["{n} row filled", "{n} rows filled"], { n: p1 - p0 + 1 }));
  };
  handle.addEventListener("pointerup", fillEnd);
  handle.addEventListener("pointercancel", (e) => {
    if (filling?.id === e.pointerId) (filling = null), draw();
  });

  // ---- events
  body.addEventListener("pointerdown", (e) => {
    const td = cellOf(e.target);
    if (!td || e.button !== 0 || e.target.closest("a, button, input, select, textarea, label")) return;
    held = !!cur && spot(td).k === cur.k && spot(td).c === cur.c && td.contains(document.activeElement);
    if (e.shiftKey && cur) e.preventDefault();
  });
  body.addEventListener("click", (e) => {
    const td = cellOf(e.target);
    if (!td || e.target.closest("a, button, input, select, textarea, label")) return;
    if (open) end(true, false);
    const s = spot(td), c = columns.get(s.c);
    go(posOf(s.k), s.c, e.shiftKey);
    // A click on the cell that's already picked edits it, a checkbox at once.
    if (!e.shiftKey && c && (held || (c.type === "checkbox" && e.target.closest("[data-dt-check]")))) begin();
  });
  body.addEventListener("dblclick", (e) => {
    const td = cellOf(e.target);
    if (td && columns.get(td.getAttribute("data-dt-col"))?.type !== "checkbox") begin();
  });
  body.addEventListener("focusin", (e) => {
    const td = e.target === cellOf(e.target) ? e.target : null;
    if (!td || open) return;
    const s = spot(td);
    if (cur?.k === s.k && cur.c === s.c) return;
    cur = s;
    anchor = null;
    draw();
  });
  body.addEventListener("focusout", (e) => !body.contains(e.relatedTarget) && !open && (tip.hidden = true));
  scroller.addEventListener("scroll", () => open && placeEditor(), { passive: true });

  // Before the table's own keys.
  el.addEventListener("keydown", (e) => {
    if (open && e.target === open.input) {
      const multi = open.c.type === "multiline";
      if (e.isComposing) return;
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        end(false);
      } else if (e.key === "Enter" && !(multi && e.shiftKey)) {
        e.preventDefault();
        e.stopPropagation();
        // The row below as shown before the edit, in case it moves.
        const p = posOf(cur.k), next = shownKeys()[p + (e.shiftKey ? -1 : 1)], c = cur.c;
        end(true, false);
        const to = next === undefined ? -1 : posOf(next);
        to >= 0 ? go(to, c) : refocus(p);
      } else if (e.key === "Tab") {
        e.preventDefault();
        e.stopPropagation();
        const next = nextCell(e.shiftKey ? -1 : 1), k = next && shownKeys()[next[0]];
        end(true, false);
        const p = next ? posOf(k) : -1;
        p >= 0 ? go(p, next[1]) : refocus();
      } else e.stopPropagation();
      return;
    }
    // A key while the scroll area holds the focus of a row scrolled out of the window.
    if (e.target === scroller && cur && posOf(cur.k) >= 0) go(posOf(cur.k), cur.c, !!anchor);
    const td = cellOf(document.activeElement);
    if (!td || !cur || document.activeElement !== td) return;
    const mod = e.metaKey || e.ctrlKey, ks = keys(), p = posOf(cur.k), c = ks.indexOf(cur.c), last = t.shown().length - 1;
    const done = (f) => (e.preventDefault(), e.stopPropagation(), f());
    const key = side(e.key);
    if (e.altKey) return;
    if (mod) {
      switch (e.key.toLowerCase()) {
        case "z": return done(() => (e.shiftKey ? back(redo, undo) : back(undo, redo), refocus(p)));
        case "y": return done(() => (back(redo, undo), refocus(p)));
        case "a": return done(() => ((anchor = { k: shownKeys()[0], c: ks[0] }), (cur = { k: shownKeys()[last], c: ks.at(-1) }), draw()));
        case "d": return done(() => {
          const r = range();
          const top = t.shown()[r.r0 === r.r1 ? Math.max(0, r.r0 - 1) : r.r0];
          fill((c) => at(t.data()[top], c.field) ?? null);
        });
        case "home": return done(() => go(0, ks[0], e.shiftKey));
        case "end": return done(() => go(last, ks.at(-1), e.shiftKey));
        case "arrowdown": return done(() => go(last, cur.c, e.shiftKey));
        case "arrowup": return done(() => go(0, cur.c, e.shiftKey));
        case "arrowleft":
        case "arrowright": return done(() => go(p, key === "ArrowRight" ? ks.at(-1) : ks[0], e.shiftKey));
      }
      return;
    }
    switch (key) {
      case "ArrowDown": return done(() => go(p + 1, cur.c, e.shiftKey));
      case "ArrowUp": return done(() => go(p - 1, cur.c, e.shiftKey));
      case "ArrowRight": return done(() => c < ks.length - 1 && go(p, ks[c + 1], e.shiftKey));
      case "ArrowLeft": return done(() => c > 0 && go(p, ks[c - 1], e.shiftKey));
      case "Home": return done(() => go(p, ks[0], e.shiftKey));
      case "End": return done(() => go(p, ks.at(-1), e.shiftKey));
      case "PageDown": return done(() => go(p + 10, cur.c, e.shiftKey));
      case "PageUp": return done(() => go(p - 10, cur.c, e.shiftKey));
      case "Tab": {
        const next = nextCell(e.shiftKey ? -1 : 1);
        return next ? done(() => go(...next)) : undefined;
      }
      case "Enter":
      case "F2":
        return columns.has(cur.c) ? done(() => begin()) : undefined;
      case "Escape": return anchor ? done(() => ((anchor = null), draw())) : undefined;
      case "Delete":
      case "Backspace": return done(() => fill((c) => (c.type === "checkbox" ? false : null)));
      case " ":
        if (columns.get(cur.c)?.type === "checkbox") return done(() => begin());
        break;
    }
    // Typing replaces what's there.
    if (e.key.length === 1 && columns.has(cur.c)) {
      const ty = columns.get(cur.c).type;
      if (ty === "checkbox") return;
      done(() => begin(ty === "select" || ty === "date" ? undefined : e.key));
    }
  }, true);
  const onCells = () => !open && cur && cellOf(document.activeElement) === document.activeElement;
  el.addEventListener("copy", (e) => {
    if (!onCells()) return;
    e.preventDefault();
    e.clipboardData.setData("text/plain", copyText());
  });
  el.addEventListener("cut", (e) => {
    if (!onCells()) return;
    e.preventDefault();
    e.clipboardData.setData("text/plain", copyText());
    fill((c) => (c.type === "checkbox" ? false : null));
  });
  el.addEventListener("paste", (e) => {
    if (!onCells()) return;
    e.preventDefault();
    paste(e.clipboardData.getData("text/plain"));
  });
  el.addEventListener("click", (e) => {
    const b = e.target.closest?.("[data-dt-add-row], [data-dt-delete-rows]");
    if (!b || b.closest("[data-i-x=ui-data-table]") !== el) return;
    if (b.hasAttribute("data-dt-add-row")) addRow();
    else removeRows(t.selection());
  });

  return {
    draw,
    check: validate,
    problems: () => faults.size,
    wrong: (k) => cur?.k === k || [...faults.keys()].some((f) => f.startsWith(k + "\u0000")),
    holds: () => !!cur && !open,
  };
};
