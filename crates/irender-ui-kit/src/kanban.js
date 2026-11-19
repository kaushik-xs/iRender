// irender-ui-kit: the `ui-kanban` behaviour. Draws the board from `columns` and `cards`
// whenever they change (the server drew the first one the same way), and moves cards:
// - dragging a card onto a column, between cards (a line shows where it lands);
// - Alt with the arrow keys on a focused card: Up and Down within its column, Left and
//   Right to the next column, keeping its place as near as it can;
// - the arrow keys alone move focus between cards, Enter opens one (`bindCard`, `open`).
// A move rewrites `bindCards` (the card's `column`, and the order), and fires `move`.
// "Add card" puts a name box at the bottom of a column: Enter adds the card.
(window.__irenderBehaviors ||= {})["ui-kanban"] = (el, api) => {
  const cfg = api.config;
  // Interface text in the document's words (its `labels`), by the English.
  const L = (text, vars) => window.__irender.label(text, vars);
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
  const cards = () => (Array.isArray(resolve(cfg.cards)) ? resolve(cfg.cards) : []);
  const key = (v) => (v == null ? "" : String(v));
  const say = (text) => {
    live.textContent = "";
    setTimeout(() => (live.textContent = text), 30);
  };
  const collapsed = new Set((cfg.collapsed ?? []).map(key));
  const PRIORITY = { urgent: "danger", critical: "danger", high: "warning", medium: "info", normal: "info", low: "neutral" };
  const initialsOf = (name) => String(name ?? "").split(/\s+/).filter(Boolean).slice(0, 2).map((w) => w[0].toUpperCase()).join("");

  // ---- drawing (the same markup as kanban.rs)
  const make = (tag, attrs = {}, text) => {
    const e = document.createElement(tag);
    for (const [k, v] of Object.entries(attrs)) if (v !== null && v !== undefined && v !== false) e.setAttribute(k, v === true ? "" : v);
    if (text !== undefined) e.textContent = text;
    return e;
  };
  const svg = {
    plus: '<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" aria-hidden="true"><path d="M8 3v10M3 8h10"/></svg>',
    fold: '<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M10 3.5 5.5 8 10 12.5"/></svg>',
  };
  function card(c) {
    const a = make("article", { "data-kb-card": key(c.id), tabindex: -1, draggable: cfg.movable ? "true" : null, "aria-roledescription": L("card") });
    const label = [c.title, c.priority && L("{priority} priority", { priority: String(c.priority) }), c.due && L("due {date}", { date: String(c.due) }), assignee(c)?.name && L("assigned to {name}", { name: assignee(c).name })].filter(Boolean).join(", ");
    a.setAttribute("aria-label", label);
    a.append(make("div", { "data-kb-title": "" }, String(c.title ?? "")));
    if (c.description) a.append(make("p", { "data-kb-desc": "" }, String(c.description)));
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
  let adding = null; // the column with a name box open
  function draw() {
    const cols = columns(), all = cards();
    const hadFocus = el.contains(document.activeElement) && document.activeElement.matches("[data-kb-card]");
    board.replaceChildren(
      ...cols.map((col) => {
        const id = key(col.id);
        const mine = all.filter((c) => key(c.column) === id);
        const over = col.limit != null && mine.length > Number(col.limit);
        const section = make("section", { "data-kb-column": id, "data-tone": col.tone ?? null, "data-over": over, "data-collapsed": collapsed.has(id), "aria-label": `${col.title ?? id}, ${L(["{n} card", "{n} cards"], { n: mine.length })}` });
        const head = make("header", { "data-kb-head": "" });
        head.append(make("span", { "data-kb-dot": "", "aria-hidden": "true" }), make("h3", {}, String(col.title ?? id)));
        head.append(make("span", { "data-kb-count": "", title: col.limit != null ? L("Limit {limit}", { limit: String(col.limit) }) : null }, col.limit != null ? `${mine.length} / ${col.limit}` : String(mine.length)));
        if (cfg.collapsible) {
          const b = make("button", { type: "button", "data-kb-fold": "", "aria-expanded": collapsed.has(id) ? "false" : "true", "aria-label": L(collapsed.has(id) ? "Show {name}" : "Hide {name}", { name: String(col.title ?? id) }) });
          b.innerHTML = svg.fold;
          head.append(b);
        }
        section.append(head);
        const list = make("div", { "data-kb-cards": "", role: "list" });
        for (const c of mine) {
          const item = card(c);
          item.setAttribute("role", "listitem");
          list.append(item);
        }
        if (!mine.length && adding !== id) list.append(make("p", { "data-kb-empty": "" }, cfg.emptyText ?? L("No cards")));
        section.append(list);
        if (cfg.addable) {
          if (adding === id) {
            const box = make("textarea", { "data-kb-name": "", rows: 2, placeholder: L("Card title, then Enter"), "aria-label": L("New card in {name}", { name: String(col.title ?? id) }) });
            section.append(box);
          } else {
            const b = make("button", { type: "button", "data-kb-add": "" });
            b.innerHTML = svg.plus;
            b.append(make("span", {}, cfg.addLabel ?? L("Add card")));
            section.append(b);
          }
        }
        return section;
      }),
    );
    // One tab stop: the focused card, else the first.
    const stop = (focusId && $(`[data-kb-card="${CSS.escape(focusId)}"]`)) || $("[data-kb-card]");
    if (stop) stop.tabIndex = 0;
    if (hadFocus && focusId) $(`[data-kb-card="${CSS.escape(focusId)}"]`)?.focus();
    const box = $("[data-kb-name]");
    if (box && document.activeElement !== box) box.focus();
  }

  // ---- reading and writing the cards
  const cardEl = (id) => $(`[data-kb-card="${CSS.escape(id)}"]`);
  const columnOf = (id) => key(cards().find((c) => key(c.id) === id)?.column);
  const inColumn = (col, all = cards()) => all.filter((c) => key(c.column) === col);
  const titleOf = (col) => columns().find((c) => key(c.id) === col)?.title ?? col;
  // Moves card `id` to place `at` (among the cards in column `to`, without it).
  function move(id, to, at) {
    const all = cards();
    const c = all.find((x) => key(x.id) === id);
    if (!c) return false;
    const from = key(c.column);
    const before = inColumn(from, all).findIndex((x) => key(x.id) === id);
    if (from === to && before === at) return false;
    const rest = all.filter((x) => key(x.id) !== id);
    const target = inColumn(to, rest);
    const moved = { ...c, column: columns().find((x) => key(x.id) === to)?.id ?? to };
    // Into the whole list just before the card that will follow it (or after the column's last).
    const next = target[at];
    let i = next ? rest.indexOf(next) : target.length ? rest.indexOf(target.at(-1)) + 1 : rest.length;
    rest.splice(i, 0, moved);
    focusId = id;
    if (cfg.bindCards) api.set(cfg.bindCards, rest);
    api.emit("move", { id: c.id, from: c.column, to: moved.column, index: at });
    say(from === to ? L("{title} moved to position {n} in {column}", { title: String(c.title), n: at + 1, column: titleOf(to) }) : L("{title} moved to {column}, position {n}", { title: String(c.title), column: titleOf(to), n: at + 1 }));
    queueMicrotask(() => cardEl(id)?.focus());
    return true;
  }
  function open(id) {
    const c = cards().find((x) => key(x.id) === id);
    if (!c) return;
    if (cfg.bindCard) api.set(cfg.bindCard, c);
    api.emit("open", { card: c });
  }
  function addCard(col, title) {
    adding = null;
    const text = title.trim();
    if (!text || !cfg.bindCards) return draw();
    const id = globalThis.crypto?.randomUUID?.() ?? `k${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;
    const c = { id, column: columns().find((x) => key(x.id) === col)?.id ?? col, title: text };
    const all = cards();
    const mine = inColumn(col, all);
    const i = mine.length ? all.indexOf(mine.at(-1)) + 1 : all.length;
    focusId = id;
    api.set(cfg.bindCards, [...all.slice(0, i), c, ...all.slice(i)]);
    api.emit("add", { card: c });
    say(L("Added {title} to {column}", { title: text, column: titleOf(col) }));
    queueMicrotask(() => cardEl(id)?.focus());
  }

  // ---- events
  el.addEventListener("click", (e) => {
    const t = e.target;
    const col = t.closest("[data-kb-column]")?.getAttribute("data-kb-column");
    if (t.closest("[data-kb-fold]")) {
      collapsed.has(col) ? collapsed.delete(col) : collapsed.add(col);
      return draw();
    }
    if (t.closest("[data-kb-add]")) {
      adding = col;
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
      const col = e.target.closest("[data-kb-column]").getAttribute("data-kb-column");
      if (e.key === "Enter" && !e.shiftKey) e.preventDefault(), addCard(col, e.target.value);
      if (e.key === "Escape") e.preventDefault(), (adding = null), draw(), $(`[data-kb-column="${CSS.escape(col)}"] [data-kb-add]`)?.focus();
      return;
    }
    const c = e.target.closest("[data-kb-card]");
    if (!c || e.target !== c) return;
    const id = c.getAttribute("data-kb-card"), col = columnOf(id);
    const cols = columns().map((x) => key(x.id)).filter((x) => !collapsed.has(x));
    const ci = cols.indexOf(col), mine = inColumn(col).map((x) => key(x.id)), i = mine.indexOf(id);
    if (e.altKey && cfg.movable) {
      const step = { ArrowUp: [0, -1], ArrowDown: [0, 1], ArrowLeft: [-1, 0], ArrowRight: [1, 0] }[e.key];
      if (!step) return;
      e.preventDefault();
      if (step[0]) {
        const to = cols[ci + step[0]];
        if (to) move(id, to, Math.min(i, inColumn(to).length));
      } else if (mine[i + step[1]] !== undefined) move(id, col, i + step[1]);
      return;
    }
    const focus = (to) => {
      if (!to) return;
      e.preventDefault();
      focusId = to;
      for (const x of $$("[data-kb-card]")) x.tabIndex = x.getAttribute("data-kb-card") === to ? 0 : -1;
      cardEl(to)?.focus();
    };
    if (e.key === "ArrowDown") focus(mine[i + 1]);
    else if (e.key === "ArrowUp") focus(mine[i - 1]);
    else if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
      // The nearest column with cards that way, at the same height or its last card.
      const dir = e.key === "ArrowLeft" ? -1 : 1;
      for (let j = ci + dir; j >= 0 && j < cols.length; j += dir) {
        const there = inColumn(cols[j]).map((x) => key(x.id));
        if (there.length) return focus(there[Math.min(i, there.length - 1)]);
      }
    } else if (e.key === "Home") focus(mine[0]);
    else if (e.key === "End") focus(mine.at(-1));
    else if (e.key === "Enter" || e.key === " ") e.preventDefault(), open(id);
  });
  board.addEventListener("focusout", (e) => {
    if (e.target.matches("[data-kb-name]") && adding) {
      const col = adding;
      // Leaving the box keeps what was typed.
      setTimeout(() => adding === col && addCard(col, e.target.value));
    }
  });

  // ---- dragging
  let dragId = null;
  const clear = () => {
    $$("[data-kb-drop]").forEach((x) => x.remove());
    $$("[data-kb-column][data-over-drop]").forEach((x) => x.removeAttribute("data-over-drop"));
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
  board.addEventListener("dragstart", (e) => {
    const c = e.target.closest?.("[data-kb-card]");
    if (!cfg.movable || !c) return;
    dragId = c.getAttribute("data-kb-card");
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData("text/plain", c.querySelector("[data-kb-title]").textContent);
    requestAnimationFrame(() => c.setAttribute("data-dragging", ""));
  });
  board.addEventListener("dragover", (e) => {
    if (dragId === null) return;
    const col = e.target.closest?.("[data-kb-column]");
    if (!col || col.hasAttribute("data-collapsed")) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
    const list = $("[data-kb-cards]", col);
    const { items, at } = slot(list, e.clientY);
    clear();
    col.setAttribute("data-over-drop", "");
    const line = make("div", { "data-kb-drop": "", "aria-hidden": "true" });
    items[at] ? list.insertBefore(line, items[at]) : list.append(line);
  });
  board.addEventListener("dragleave", (e) => !board.contains(e.relatedTarget) && clear());
  board.addEventListener("drop", (e) => {
    if (dragId === null) return;
    const col = e.target.closest?.("[data-kb-column]");
    if (!col) return;
    e.preventDefault();
    const { at } = slot($("[data-kb-cards]", col), e.clientY);
    clear();
    move(dragId, col.getAttribute("data-kb-column"), at);
  });
  board.addEventListener("dragend", () => {
    clear();
    $$("[data-dragging]").forEach((x) => x.removeAttribute("data-dragging"));
    dragId = null;
  });

  // `bindCard` follows its card as the cards change (moved, edited), or empties if it goes.
  function syncOpen() {
    const open = cfg.bindCard ? api.state[cfg.bindCard] : null;
    if (!isObj(open) || open.id == null) return;
    const now = cards().find((c) => key(c.id) === key(open.id)) ?? null;
    if (now !== open) api.set(cfg.bindCard, now);
  }
  let last = null;
  draw();
  return {
    update() {
      const now = [resolve(cfg.columns), resolve(cfg.cards)];
      if (last && now[0] === last[0] && now[1] === last[1]) return;
      last = now;
      draw();
      syncOpen();
    },
  };
};
