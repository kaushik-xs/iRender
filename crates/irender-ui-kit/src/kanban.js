// irender-ui-kit: the `ui-kanban` behaviour. Draws the board from `columns` and `cards`
// whenever they change (the server drew the first one the same way), and moves cards:
// - dragging a card onto a column, between cards (a line shows where it lands);
// - Alt with the arrow keys on a focused card: Up and Down within its column, Left and
//   Right to the next column, keeping its place as near as it can;
// - the arrow keys alone move focus between cards, Enter opens one (`bindCard`, `open`).
// A move rewrites `bindCards` (the card's `column`, and the order), and fires `move`.
// "Add card" puts a name box at the bottom of a column: Enter adds the card.
//
// With `swimlanes` (a field), the board is a row of cells per lane, one per column: a
// card's cell is its column and its lane (the field's value), and moving it to another
// lane writes the field. Up and Down go on into the next lane at a cell's ends, and Alt
// with them moves a card there. With `reorderable`, a grip on each column's header moves
// the column (dragged, by touch after a hold, or Left and Right on it), writing the order to `bindColumns`.
// With `filter` (`{ field: condition }`, as the data table's and `ui-chart`'s), only the
// cards that pass are drawn; moves place a card among the ones shown and keep the rest.
(window.__irenderBehaviors ||= {})["ui-kanban"] = (el, api) => {
  const cfg = api.config;
  // Interface text in the document's words (its `labels`), by the English.
  const L = (text, vars) => window.__irender.label(text, vars);
  // Right to left, Left and Right swap: the next one is on the left.
  const side = (k) => (getComputedStyle(el).direction === "rtl" ? { ArrowLeft: "ArrowRight", ArrowRight: "ArrowLeft" }[k] ?? k : k);

  const $ = (s, from = el) => from.querySelector(s);
  const $$ = (s, from = el) => [...from.querySelectorAll(s)];
  const board = $("[data-kb-board]");
  const live = $("[data-kb-live]");
  const isObj = (v) => v !== null && typeof v === "object" && !Array.isArray(v);
  const env = () => {
    const s = api.scope?.();
    return s ? { ...api.state, ...s } : api.state;
  };
  const resolve = (v) => (isObj(v) && "$irender" in v && Object.keys(v).length === 1 ? window.__irender.evaluate(v.$irender, env()) : v);
  const columns = () => (Array.isArray(resolve(cfg.columns)) ? resolve(cfg.columns) : []).map((c) => (isObj(c) ? c : { id: c, title: c }));
  const allCards = () => (Array.isArray(resolve(cfg.cards)) ? resolve(cfg.cards) : []);
  const key = (v) => (v == null ? "" : String(v));
  const say = (text) => {
    live.textContent = "";
    setTimeout(() => (live.textContent = text), 30);
  };
  const collapsed = new Set((cfg.collapsed ?? []).map(key));
  const PRIORITY = { urgent: "danger", critical: "danger", high: "warning", medium: "info", normal: "info", low: "neutral" };
  const initialsOf = (name) => String(name ?? "").split(/\s+/).filter(Boolean).slice(0, 2).map((w) => w[0].toUpperCase()).join("");
  const at = (r, f) => String(f).split(".").reduce((x, k) => (x !== null && typeof x === "object" ? x[k] : undefined), r);

  // ---- `filter`: the data table's conditions (`matches` in data_table.js), so one
  // `ui-filters` panel can narrow a board, a table and a chart alike
  const blank = (v) => v === null || v === undefined || v === "";
  const isNum = (v) => typeof v === "number" || (typeof v === "string" && v.trim() !== "" && !isNaN(v));
  const looksLikeDate = (v) => typeof v === "string" && /^\d{4}-\d{2}/.test(v) && !isNaN(Date.parse(v));
  const fold = (s) => String(s ?? "").normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase();
  const same = (a, b) => a === b || (!blank(a) && !blank(b) && typeof a !== "object" && typeof b !== "object" && String(a) === String(b));
  const cmp = (a, b) => {
    if (isNum(a) && isNum(b)) return Number(a) - Number(b);
    if (looksLikeDate(a) && looksLikeDate(b)) return Date.parse(a) - Date.parse(b);
    return String(a ?? "").localeCompare(String(b ?? ""), cfg.locale || undefined, { numeric: true, sensitivity: "base" });
  };
  const endOfDay = (b) => (typeof b === "string" && /^\d{4}-\d{2}-\d{2}$/.test(b) ? b + "T23:59:59.999" : b);
  const OPS = {
    from: (v, b) => cmp(v, b) >= 0,
    to: (v, b) => cmp(v, looksLikeDate(v) ? endOfDay(b) : b) <= 0,
    gt: (v, b) => cmp(v, b) > 0,
    gte: (v, b) => cmp(v, b) >= 0,
    lt: (v, b) => cmp(v, b) < 0,
    lte: (v, b) => cmp(v, b) <= 0,
    contains: (v, b) => fold(v).includes(fold(b)),
  };
  function matches(v, c, field) {
    if (blank(c)) return true;
    if (Array.isArray(v)) return !v.length ? matches(null, c, field) || (Array.isArray(c) && !c.length) : v.some((x) => matches(x, c, field));
    if (Array.isArray(c)) return !c.length || c.some((x) => same(v, isObj(x) ? x[field] : x));
    if (!isObj(c)) return same(v, c);
    if (Object.hasOwn(c, "not")) return blank(c.not) || !matches(v, c.not, field);
    if (Object.hasOwn(c, "empty")) return blank(c.empty) || (c.empty ? blank(v) : !blank(v));
    const ops = Object.keys(c).filter((k) => Object.hasOwn(OPS, k));
    if (!ops.length) return same(v, c[field]);
    return ops.every((k) => blank(c[k]) || (!blank(v) && OPS[k](v, c[k])));
  }
  // The filter, with any state it uses read (it may be `{ owner: "{{ who }}" }`).
  const deep = (v) => (Array.isArray(v) ? v.map(deep) : isObj(v) ? ("$irender" in v && Object.keys(v).length === 1 ? resolve(v) : Object.fromEntries(Object.entries(v).map(([k, x]) => [k, deep(x)]))) : v);
  const filter = () => {
    const f = deep(cfg.filter);
    return isObj(f) ? f : null;
  };
  const passes = (c, f = filter()) => !f || Object.entries(f).every(([field, cond]) => matches(at(c, field), cond, field));
  // The cards shown: every card, or those that pass the filter.
  const cards = () => {
    const all = allCards(), f = filter();
    return f ? all.filter((c) => passes(c, f)) : all;
  };

  // ---- lanes: one per value of the `swimlanes` field, in `lanes`' order or as they come.
  // The field may follow state, so it's read again on each draw.
  let laneField = resolve(cfg.swimlanes) || null;
  // A lane's key from a card's value: a name (or label, id) for an object, else the text.
  const laneKey = (v) => (v == null || v === "" ? "" : isObj(v) ? key(v.name ?? v.label ?? v.id) : key(v));
  const laneName = (v) => (v == null || v === "" ? L("No {field}", { field: String(laneField) }) : isObj(v) ? String(v.name ?? v.label ?? v.id) : String(v));
  function lanes() {
    if (!laneField) return [{ key: null }];
    const out = new Map();
    for (const l of Array.isArray(resolve(cfg.lanes)) ? resolve(cfg.lanes) : []) {
      const v = isObj(l) && "value" in l ? l.value : l;
      out.set(laneKey(v), { key: laneKey(v), value: v, name: isObj(l) && l.label != null ? String(l.label) : laneName(v) });
    }
    for (const c of cards()) {
      const v = at(c, laneField), k = laneKey(v);
      if (!out.has(k)) out.set(k, { key: k, value: v, name: laneName(v) });
    }
    // Cards without a value last.
    const list = [...out.values()];
    return [...list.filter((l) => l.key !== ""), ...list.filter((l) => l.key === "")];
  }
  const laneOf = (c) => (laneField ? laneKey(at(c, laneField)) : null);
  const foldedLanes = new Set();

  // ---- drawing (the same markup as kanban.rs)
  const make = (tag, attrs = {}, text) => {
    const e = document.createElement(tag);
    for (const [k, v] of Object.entries(attrs)) if (v !== null && v !== undefined && v !== false) e.setAttribute(k, v === true ? "" : v);
    if (text !== undefined) e.textContent = text;
    return e;
  };
  const svg = {
    plus: '<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" aria-hidden="true"><path d="M8 3v10M3 8h10"/></svg>',
    fold: '<svg data-flip width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M10 3.5 5.5 8 10 12.5"/></svg>',
    lane: '<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 6l4 4 4-4"/></svg>',
    grip: '<svg width="10" height="14" viewBox="0 0 10 14" fill="currentColor" aria-hidden="true"><circle cx="2.5" cy="2.5" r="1.2"/><circle cx="7.5" cy="2.5" r="1.2"/><circle cx="2.5" cy="7" r="1.2"/><circle cx="7.5" cy="7" r="1.2"/><circle cx="2.5" cy="11.5" r="1.2"/><circle cx="7.5" cy="11.5" r="1.2"/></svg>',
  };
  // `fields`: more of a card, each a line: text, a badge, a progress bar or a date.
  function fieldEl(f, c) {
    const v = at(c, f.field);
    if (v == null || v === "") return null;
    const row = make("div", { "data-kb-field": f.field });
    if (f.label) row.append(make("span", { "data-kb-field-label": "" }, String(f.label)));
    if (f.cell === "badge") row.append(make("span", { "data-kb-badge": "", "data-tone": (isObj(f.tones) ? f.tones[String(v)] : null) ?? "neutral" }, String(v)));
    else if (f.cell === "progress") {
      const n = Math.max(0, Math.min(100, Number(v) || 0));
      const bar = make("span", { "data-kb-progress": "", role: "progressbar", "aria-valuenow": n, "aria-valuemin": 0, "aria-valuemax": 100, "aria-label": f.label ? String(f.label) : String(f.field) });
      bar.append(make("span", { style: `width: ${n}%` }));
      row.append(bar, make("span", { "data-kb-field-value": "" }, `${n}%`));
    } else if (f.cell === "date") {
      const d = /^\d{4}-\d{2}-\d{2}$/.test(String(v)) ? new Date(`${v}T00:00`) : new Date(v);
      row.append(make("span", { "data-kb-field-value": "" }, Number.isNaN(+d) ? String(v) : d.toLocaleDateString(cfg.locale || undefined, { day: "numeric", month: "short" })));
    } else row.append(make("span", { "data-kb-field-value": "" }, String(v)));
    return row;
  }
  function card(c) {
    const a = make("article", { "data-kb-card": key(c.id), tabindex: -1, draggable: cfg.movable ? "true" : null, "aria-roledescription": L("card") });
    const label = [c.title, c.priority && L("{priority} priority", { priority: String(c.priority) }), c.due && L("due {date}", { date: String(c.due) }), assignee(c)?.name && L("assigned to {name}", { name: assignee(c).name })].filter(Boolean).join(", ");
    a.setAttribute("aria-label", label);
    a.append(make("div", { "data-kb-title": "" }, String(c.title ?? "")));
    if (c.description) a.append(make("p", { "data-kb-desc": "" }, String(c.description)));
    const list = resolve(cfg.fields);
    const fields = (Array.isArray(list) ? list : []).map((f) => fieldEl(isObj(f) ? f : { field: f }, c)).filter(Boolean);
    if (fields.length) {
      const box = make("div", { "data-kb-fields": "" });
      box.append(...fields);
      a.append(box);
    }
    const tags = Array.isArray(c.tags) ? c.tags : [];
    if (tags.length) {
      const t = make("div", { "data-kb-tags": "" });
      for (const tag of tags) t.append(make("span", { "data-tone": isObj(tag) ? tag.tone ?? "neutral" : "neutral" }, String(isObj(tag) ? tag.label ?? tag.text ?? "" : tag)));
      a.append(t);
    }
    const who = assignee(c);
    if (c.priority || c.due || who) {
      const f = make("div", { "data-kb-foot": "" });
      if (c.priority) f.append(make("span", { "data-kb-priority": "", "data-tone": PRIORITY[String(c.priority).toLowerCase()] ?? "neutral" }, String(c.priority)));
      if (c.due) f.append(make("span", { "data-kb-due": "" }, String(c.due)));
      if (who) {
        const av = make("span", { "data-kb-avatar": "", title: who.name ?? "" });
        if (who.avatar) av.append(make("img", { src: who.avatar, alt: "" }));
        else av.textContent = who.initials || initialsOf(who.name);
        f.append(av);
      }
      a.append(f);
    }
    return a;
  }
  const assignee = (c) => (c.assignee == null || c.assignee === "" ? null : isObj(c.assignee) ? c.assignee : { name: String(c.assignee) });
  let focusId = null;
  let adding = null; // the cell with a name box open, as "column|lane"
  const cellKey = (col, lane) => `${col}|${lane ?? ""}`;
  // A column's header: its dot, title, count (of every lane), fold button and grip.
  function header(col, mine) {
    // Filtered, "2 of 5"; a limit counts every card in the column.
    const total = allCards().filter((c) => key(c.column) === key(col.id)).length;
    const id = key(col.id), title = String(col.title ?? id);
    const head = make("header", { "data-kb-head": "" });
    if (cfg.reorderable) {
      const g = make("button", { type: "button", "data-kb-grip": id, draggable: "true", "aria-label": L("Move column {name}", { name: title }), title: L("Drag, or press Left or Right, to move the column") });
      g.innerHTML = svg.grip;
      head.append(g);
    }
    head.append(make("span", { "data-kb-dot": "", "aria-hidden": "true" }), make("h3", {}, title));
    head.append(make("span", { "data-kb-count": "", title: col.limit != null ? L("Limit {limit}", { limit: String(col.limit) }) : null }, col.limit != null ? `${total} / ${col.limit}` : mine.length < total ? L("{n} of {total}", { n: mine.length, total }) : String(mine.length)));
    if (cfg.collapsible) {
      const b = make("button", { type: "button", "data-kb-fold": "", "aria-expanded": collapsed.has(id) ? "false" : "true", "aria-label": L(collapsed.has(id) ? "Show {name}" : "Hide {name}", { name: title }) });
      b.innerHTML = svg.fold;
      head.append(b);
    }
    return head;
  }
  // A column's cards in one lane (or all of them): the list, the empty note and Add card.
  function cell(col, lane, mine, withHead) {
    const id = key(col.id), title = String(col.title ?? id);
    const all = allCards().filter((c) => key(c.column) === id);
    const over = col.limit != null && all.length > Number(col.limit);
    const name = lane?.key == null ? title : `${title}, ${lane.name}`;
    const section = make("section", { "data-kb-column": id, "data-kb-lane": lane?.key ?? null, "data-tone": col.tone ?? null, "data-over": over, "data-collapsed": collapsed.has(id), "aria-label": `${name}, ${L(["{n} card", "{n} cards"], { n: mine.length })}` });
    if (withHead) section.append(header(col, mine));
    const list = make("div", { "data-kb-cards": "", role: "list" });
    for (const c of mine) {
      const item = card(c);
      item.setAttribute("role", "listitem");
      list.append(item);
    }
    const here = cellKey(id, lane?.key);
    if (!mine.length && adding !== here) list.append(make("p", { "data-kb-empty": "" }, cfg.emptyText ?? L("No cards")));
    section.append(list);
    if (cfg.addable) {
      if (adding === here) {
        section.append(make("textarea", { "data-kb-name": "", rows: 2, placeholder: L("Card title, then Enter"), "aria-label": L("New card in {name}", { name }) }));
      } else {
        const b = make("button", { type: "button", "data-kb-add": "" });
        b.innerHTML = svg.plus;
        b.append(make("span", {}, cfg.addLabel ?? L("Add card")));
        section.append(b);
      }
    }
    return section;
  }
  function draw() {
    laneField = resolve(cfg.swimlanes) || null;
    const cols = columns(), all = cards();
    const hadFocus = el.contains(document.activeElement) && document.activeElement.matches("[data-kb-card]");
    const grip = el.contains(document.activeElement) ? document.activeElement.getAttribute("data-kb-grip") : null;
    board.toggleAttribute("data-kb-lanes", !!laneField);
    if (!laneField) {
      board.replaceChildren(...cols.map((col) => cell(col, null, all.filter((c) => key(c.column) === key(col.id)), true)));
    } else {
      // A row of the columns' headers, then a row of cells per lane under its header.
      const heads = make("div", { "data-kb-heads": "" });
      for (const col of cols) {
        const h = make("div", { "data-kb-colhead": key(col.id), "data-tone": col.tone ?? null, "data-collapsed": collapsed.has(key(col.id)) });
        h.append(header(col, all.filter((c) => key(c.column) === key(col.id))));
        heads.append(h);
      }
      const rows = lanes().map((lane) => {
        const mine = all.filter((c) => laneOf(c) === lane.key);
        const folded = foldedLanes.has(lane.key);
        const row = make("section", { "data-kb-lane-row": lane.key, "data-collapsed": folded, "aria-label": `${lane.name}, ${L(["{n} card", "{n} cards"], { n: mine.length })}` });
        const head = make("header", { "data-kb-lane-head": "" });
        const b = make("button", { type: "button", "data-kb-lane-fold": lane.key, "aria-expanded": folded ? "false" : "true" });
        b.innerHTML = svg.lane;
        b.append(make("span", {}, lane.name));
        head.append(b, make("span", { "data-kb-count": "" }, String(mine.length)));
        row.append(head);
        if (!folded) {
          const cells = make("div", { "data-kb-cells": "" });
          cells.append(...cols.map((col) => cell(col, lane, mine.filter((c) => key(c.column) === key(col.id)), false)));
          row.append(cells);
        }
        return row;
      });
      board.replaceChildren(heads, ...rows);
    }
    // One tab stop: the focused card, else the first.
    const stop = (focusId && $(`[data-kb-card="${CSS.escape(focusId)}"]`)) || $("[data-kb-card]");
    if (stop) stop.tabIndex = 0;
    if (hadFocus && focusId) $(`[data-kb-card="${CSS.escape(focusId)}"]`)?.focus();
    if (grip) $(`[data-kb-grip="${CSS.escape(grip)}"]`)?.focus();
    const box = $("[data-kb-name]");
    if (box && document.activeElement !== box) box.focus();
  }

  // ---- reading and writing the cards
  const cardEl = (id) => $(`[data-kb-card="${CSS.escape(id)}"]`);
  const cardOf = (id) => allCards().find((c) => key(c.id) === id);
  const columnOf = (id) => key(cardOf(id)?.column);
  // The cards in column `col` (and lane `lane`, with lanes).
  const inCell = (col, lane, all = cards()) => all.filter((c) => key(c.column) === col && (!laneField || laneOf(c) === lane));
  const titleOf = (col) => columns().find((c) => key(c.id) === col)?.title ?? col;
  const laneByKey = (k) => lanes().find((l) => l.key === k);
  // Moves card `id` to place `pos` (among the cards in column `to`, and lane `lane`,
  // without it).
  function move(id, to, pos, lane) {
    const all = allCards(), f = filter();
    const c = all.find((x) => key(x.id) === id);
    if (!c) return false;
    const from = key(c.column), fromLane = laneOf(c);
    lane = laneField ? (lane === undefined ? fromLane : lane) : null;
    const before = inCell(from, fromLane, all.filter((x) => passes(x, f))).findIndex((x) => key(x.id) === id);
    if (from === to && lane === fromLane && before === pos) return false;
    const rest = all.filter((x) => key(x.id) !== id);
    let moved = { ...c, column: columns().find((x) => key(x.id) === to)?.id ?? to };
    // Into another lane: the lane's value in the field.
    if (laneField && lane !== fromLane) {
      const v = laneByKey(lane)?.value;
      const path = String(laneField).split(".");
      const set = (o, i) => (i === path.length - 1 ? { ...o, [path[i]]: v ?? null } : { ...o, [path[i]]: set(isObj(o[path[i]]) ? o[path[i]] : {}, i + 1) });
      moved = set(moved, 0);
    }
    // `pos` is among the cards shown; filtered out ones keep their places.
    const target = inCell(to, lane, rest.filter((x) => passes(x, f))), whole = inCell(to, lane, rest);
    // Into the whole list just before the card that will follow it (or after the cell's last).
    const next = target[pos];
    let i = next ? rest.indexOf(next) : whole.length ? rest.indexOf(whole.at(-1)) + 1 : rest.length;
    rest.splice(i, 0, moved);
    focusId = id;
    if (cfg.bindCards) api.set(cfg.bindCards, rest);
    api.emit("move", { id: c.id, from: c.column, to: moved.column, index: pos, ...(laneField ? { fromLane: at(c, laneField) ?? null, toLane: at(moved, laneField) ?? null } : {}) });
    const where = laneField && lane !== fromLane ? `${titleOf(to)}, ${laneByKey(lane)?.name ?? ""}` : titleOf(to);
    say(from === to && lane === fromLane ? L("{title} moved to position {n} in {column}", { title: String(c.title), n: pos + 1, column: where }) : L("{title} moved to {column}, position {n}", { title: String(c.title), column: where, n: pos + 1 }));
    queueMicrotask(() => cardEl(id)?.focus());
    return true;
  }
  function open(id) {
    const c = cardOf(id);
    if (!c) return;
    if (cfg.bindCard) api.set(cfg.bindCard, c);
    api.emit("open", { card: c });
  }
  function addCard(here, title) {
    adding = null;
    const [col, lane] = here.split("|");
    const text = title.trim();
    if (!text || !cfg.bindCards) return draw();
    const id = globalThis.crypto?.randomUUID?.() ?? `k${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;
    let c = { id, column: columns().find((x) => key(x.id) === col)?.id ?? col, title: text };
    if (laneField) {
      const v = laneByKey(lane)?.value;
      if (v != null && v !== "") c = { ...c, [laneField]: v };
    }
    const all = allCards();
    const mine = inCell(col, laneField ? lane : null, all);
    const i = mine.length ? all.indexOf(mine.at(-1)) + 1 : all.length;
    focusId = id;
    api.set(cfg.bindCards, [...all.slice(0, i), c, ...all.slice(i)]);
    api.emit("add", { card: c });
    say(L("Added {title} to {column}", { title: text, column: titleOf(col) }));
    queueMicrotask(() => cardEl(id)?.focus());
  }
  // Column `id` moved to place `to` among the columns, written to `bindColumns`.
  function moveColumn(id, to) {
    const cols = resolve(cfg.columns);
    if (!Array.isArray(cols)) return;
    const from = cols.findIndex((c) => key(isObj(c) ? c.id : c) === id);
    to = Math.max(0, Math.min(cols.length - 1, to));
    if (from < 0 || from === to) return;
    const next = cols.slice();
    const [c] = next.splice(from, 1);
    next.splice(to, 0, c);
    if (cfg.bindColumns) api.set(cfg.bindColumns, next);
    api.emit("columns", { id: isObj(c) ? c.id : c, from, to, columns: next });
    say(L("{name} moved to position {n} of {count}", { name: String(titleOf(id)), n: to + 1, count: next.length }));
  }

  // ---- events
  const cellOf = (t) => t.closest("[data-kb-column]");
  el.addEventListener("click", (e) => {
    const t = e.target;
    const col = t.closest("[data-kb-column]")?.getAttribute("data-kb-column") ?? t.closest("[data-kb-colhead]")?.getAttribute("data-kb-colhead");
    if (t.closest("[data-kb-fold]")) {
      collapsed.has(col) ? collapsed.delete(col) : collapsed.add(col);
      return draw();
    }
    const lf = t.closest("[data-kb-lane-fold]");
    if (lf) {
      const k = lf.getAttribute("data-kb-lane-fold");
      foldedLanes.has(k) ? foldedLanes.delete(k) : foldedLanes.add(k);
      return draw();
    }
    if (t.closest("[data-kb-add]")) {
      adding = cellKey(col, cellOf(t)?.getAttribute("data-kb-lane"));
      return draw();
    }
    const c = t.closest("[data-kb-card]");
    if (c) {
      focusId = c.getAttribute("data-kb-card");
      for (const x of $$("[data-kb-card]")) x.tabIndex = x === c ? 0 : -1;
      c.focus();
      open(focusId);
    }
  });
  board.addEventListener("keydown", (e) => {
    if (e.target.matches("[data-kb-name]")) {
      const s = cellOf(e.target), here = cellKey(s.getAttribute("data-kb-column"), s.getAttribute("data-kb-lane"));
      if (e.key === "Enter" && !e.shiftKey) e.preventDefault(), addCard(here, e.target.value);
      if (e.key === "Escape") {
        e.preventDefault();
        adding = null;
        draw();
        $(`[data-kb-column="${CSS.escape(s.getAttribute("data-kb-column"))}"]${s.hasAttribute("data-kb-lane") ? `[data-kb-lane="${CSS.escape(s.getAttribute("data-kb-lane"))}"]` : ""} [data-kb-add]`)?.focus();
      }
      return;
    }
    // A column's grip: Left and Right move the column.
    const g = e.target.closest?.("[data-kb-grip]");
    if (g) {
      const dir = { ArrowLeft: -1, ArrowRight: 1 }[side(e.key)];
      if (!dir) return;
      e.preventDefault();
      const id = g.getAttribute("data-kb-grip");
      moveColumn(id, columns().findIndex((c) => key(c.id) === id) + dir);
      return;
    }
    const c = e.target.closest("[data-kb-card]");
    if (!c || e.target !== c) return;
    const id = c.getAttribute("data-kb-card"), col = columnOf(id), lane = laneOf(cardOf(id) ?? {});
    const cols = columns().map((x) => key(x.id)).filter((x) => !collapsed.has(x));
    const lanesOpen = lanes().map((l) => l.key).filter((k) => !foldedLanes.has(k));
    const ci = cols.indexOf(col), li = lanesOpen.indexOf(lane), mine = inCell(col, lane).map((x) => key(x.id)), i = mine.indexOf(id);
    if (e.altKey && cfg.movable) {
      const step = { ArrowUp: [0, -1], ArrowDown: [0, 1], ArrowLeft: [-1, 0], ArrowRight: [1, 0] }[side(e.key)];
      if (!step) return;
      e.preventDefault();
      if (step[0]) {
        const to = cols[ci + step[0]];
        if (to) move(id, to, Math.min(i, inCell(to, lane).length), lane);
      } else if (mine[i + step[1]] !== undefined) move(id, col, i + step[1], lane);
      else if (laneField && lanesOpen[li + step[1]] !== undefined) {
        // Past a cell's end, into the next lane: at its top going down, its end going up.
        const to = lanesOpen[li + step[1]];
        move(id, col, step[1] > 0 ? 0 : inCell(col, to).length, to);
      }
      return;
    }
    const focus = (to) => {
      if (!to) return;
      e.preventDefault();
      focusId = to;
      for (const x of $$("[data-kb-card]")) x.tabIndex = x.getAttribute("data-kb-card") === to ? 0 : -1;
      cardEl(to)?.focus();
    };
    // Up and Down past a cell's ends: the nearest lane with cards in this column.
    const lane_ = (dir) => {
      for (let j = li + dir; j >= 0 && j < lanesOpen.length; j += dir) {
        const there = inCell(col, lanesOpen[j]).map((x) => key(x.id));
        if (there.length) return dir > 0 ? there[0] : there.at(-1);
      }
    };
    if (e.key === "ArrowDown") focus(mine[i + 1] ?? (laneField ? lane_(1) : undefined));
    else if (e.key === "ArrowUp") focus(mine[i - 1] ?? (laneField ? lane_(-1) : undefined));
    else if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
      // The nearest column with cards that way, at the same height or its last card.
      const dir = side(e.key) === "ArrowLeft" ? -1 : 1;
      for (let j = ci + dir; j >= 0 && j < cols.length; j += dir) {
        const there = inCell(cols[j], lane).map((x) => key(x.id));
        if (there.length) return focus(there[Math.min(i, there.length - 1)]);
      }
    } else if (e.key === "Home") focus(mine[0]);
    else if (e.key === "End") focus(mine.at(-1));
    else if (e.key === "Enter" || e.key === " ") e.preventDefault(), open(id);
  });
  board.addEventListener("focusout", (e) => {
    if (e.target.matches("[data-kb-name]") && adding) {
      const here = adding;
      // Leaving the box keeps what was typed.
      setTimeout(() => adding === here && addCard(here, e.target.value));
    }
  });

  // ---- dragging cards (and, with `reorderable`, columns by their grip)
  let dragId = null, dragCol = null;
  const clear = () => {
    $$("[data-kb-drop]").forEach((x) => x.remove());
    $$("[data-kb-column][data-over-drop], [data-kb-colhead][data-over-drop], [data-kb-head][data-over-drop]").forEach((x) => x.removeAttribute("data-over-drop"));
  };
  // Where in `list` a pointer at `y` would drop: the index among its cards (not the dragged one).
  const slot = (list, y) => {
    const items = $$("[data-kb-card]", list).filter((x) => x.getAttribute("data-kb-card") !== dragId);
    const i = items.findIndex((x) => {
      const r = x.getBoundingClientRect();
      return y < r.top + r.height / 2;
    });
    return { items, at: i < 0 ? items.length : i };
  };
  // The column a dragged column would go before: the one whose header the pointer is over.
  const headAt = (target) => target?.closest?.("[data-kb-colhead], [data-kb-column]:not([data-kb-lane])");
  const headId = (h) => h.getAttribute("data-kb-colhead") ?? h.getAttribute("data-kb-column");
  board.addEventListener("dragstart", (e) => {
    const g = e.target.closest?.("[data-kb-grip]");
    if (g && cfg.reorderable) {
      dragCol = g.getAttribute("data-kb-grip");
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", String(titleOf(dragCol)));
      return;
    }
    const c = e.target.closest?.("[data-kb-card]");
    if (!cfg.movable || !c) return;
    dragId = c.getAttribute("data-kb-card");
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData("text/plain", c.querySelector("[data-kb-title]").textContent);
    requestAnimationFrame(() => c.setAttribute("data-dragging", ""));
  });
  // The cell a card would drop into from over `target`, if it takes cards.
  const columnAt = (target) => {
    const col = target?.closest?.("[data-kb-column]");
    return col && board.contains(col) && !col.hasAttribute("data-collapsed") ? col : null;
  };
  // Shows where a card dropped at height `y` in `col` lands.
  function showDrop(col, y) {
    const list = $("[data-kb-cards]", col);
    const { items, at } = slot(list, y);
    clear();
    col.setAttribute("data-over-drop", "");
    const line = make("div", { "data-kb-drop": "", "aria-hidden": "true" });
    items[at] ? list.insertBefore(line, items[at]) : list.append(line);
  }
  const dropIn = (col, y) => move(dragId, col.getAttribute("data-kb-column"), slot($("[data-kb-cards]", col), y).at, laneField ? col.getAttribute("data-kb-lane") ?? "" : undefined);
  // A dragged column over the header `h`: marked as where it goes.
  const showColDrop = (h) => {
    clear();
    (h.querySelector("[data-kb-head]") ?? h).setAttribute("data-over-drop", "");
  };
  // A dragged column let go at `x` over the header `h`: before it, or after it past its middle.
  function dropCol(h, x) {
    const to = columns().findIndex((c) => key(c.id) === headId(h)), r = h.getBoundingClientRect();
    const past = (side("ArrowRight") === "ArrowRight") === x > r.left + r.width / 2;
    const from = columns().findIndex((c) => key(c.id) === dragCol);
    moveColumn(dragCol, to + (past ? 1 : 0) - (from < to + (past ? 1 : 0) ? 1 : 0));
  }
  board.addEventListener("dragover", (e) => {
    if (dragCol !== null) {
      const h = headAt(e.target);
      if (!h) return;
      e.preventDefault();
      showColDrop(h);
      return;
    }
    if (dragId === null) return;
    const col = columnAt(e.target);
    if (!col) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
    showDrop(col, e.clientY);
  });
  board.addEventListener("dragleave", (e) => !board.contains(e.relatedTarget) && clear());
  board.addEventListener("drop", (e) => {
    if (dragCol !== null) {
      const h = headAt(e.target);
      clear();
      if (!h) return;
      e.preventDefault();
      dropCol(h, e.clientX);
      return;
    }
    if (dragId === null) return;
    const col = e.target.closest?.("[data-kb-column]");
    if (!col) return;
    e.preventDefault();
    clear();
    dropIn(col, e.clientY);
  });
  const endDrag = () => {
    clear();
    $$("[data-dragging]").forEach((x) => x.removeAttribute("data-dragging"));
    dragId = dragCol = null;
  };
  board.addEventListener("dragend", endDrag);
  // By touch: hold a card, then drag it.
  window.__irenderTouchDrag(board, {
    scroller: board,
    pick: (t) => (cfg.movable && !t.closest("button, input, textarea, a[href]") ? t.closest("[data-kb-card]") : null),
    start(c) {
      dragId = c.getAttribute("data-kb-card");
      c.setAttribute("data-dragging", "");
    },
    over(x, y, under) {
      const col = columnAt(under);
      col ? showDrop(col, y) : clear();
    },
    drop(x, y, under) {
      const col = columnAt(under);
      if (col && dragId !== null) dropIn(col, y);
    },
    end: endDrag,
  });
  // And a column by its grip, with `reorderable`: the header follows the finger.
  window.__irenderTouchDrag(board, {
    scroller: board,
    pick: (t) => {
      const g = cfg.reorderable ? t.closest("[data-kb-grip]") : null;
      return g ? g.closest("[data-kb-head]") ?? g : null;
    },
    start(h) {
      dragCol = h.querySelector("[data-kb-grip]")?.getAttribute("data-kb-grip") ?? null;
    },
    over(x, y, under) {
      const h = headAt(under);
      h ? showColDrop(h) : clear();
    },
    drop(x, y, under) {
      const h = headAt(under);
      if (h && dragCol !== null) dropCol(h, x);
    },
    end: endDrag,
  });

  // `bindCard` follows its card as the cards change (moved, edited), or empties if it goes.
  function syncOpen() {
    const open = cfg.bindCard ? api.state[cfg.bindCard] : null;
    if (!isObj(open) || open.id == null) return;
    const now = allCards().find((c) => key(c.id) === key(open.id)) ?? null;
    if (now !== open) api.set(cfg.bindCard, now);
  }
  let last = null;
  draw();
  return {
    update() {
      const now = [resolve(cfg.columns), resolve(cfg.cards), resolve(cfg.lanes), resolve(cfg.swimlanes), resolve(cfg.fields), JSON.stringify(filter())];
      if (last && now.every((x, n) => x === last[n])) return;
      last = now;
      draw();
      syncOpen();
    },
  };
};
