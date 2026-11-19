// irender-ui-kit: the `ui-tree` behaviour. It draws the tree from `nodes` whenever they
// change (the server drew the first one the same way), and handles:
// - the keyboard, as the ARIA tree pattern: Up, Down, Home, End; Right opens a node or
//   steps into it, Left closes it or steps out; Enter selects; Space checks;
// - checking: a node checks the nodes under it, and shows "some" when partly checked;
// - search: matches with the nodes above them, the rest hidden;
// - add and delete, as buttons, Delete on the keyboard, and Delete for the checked.
//   With `bindNodes`, Add puts a name box where the node will go: Enter adds it with a
//   new id, Escape cancels. Without it, Add only reports the parent, for the host;
// - renaming in place (F2, a double click on the name), with `renamable`;
// - children loaded from `childrenUrl` when a node with `children: true` first opens;
// - moving, by dragging (onto a node: into it; near its top or bottom edge: before or
//   after it) or with Alt and the arrows (up, down; right: into the node above; left:
//   out to after its parent).
// Moves and deletions are written to `bindNodes` when given, and fire `move` and
// `delete` either way, so a host can save them.
(window.__irenderBehaviors ||= {})["ui-tree"] = (el, api) => {
  // Right to left, Left and Right swap: the next one is on the left.
  const side = (k) => (getComputedStyle(el).direction === "rtl" ? { ArrowLeft: "ArrowRight", ArrowRight: "ArrowLeft" }[k] ?? k : k);
  const cfg = api.config;
  const $ = (s, from = el) => from.querySelector(s);
  const $$ = (s, from = el) => [...from.querySelectorAll(s)];
  const list = $("[role=tree]");
  const live = $("[data-tree-live]");
  const search = $("[data-tree-search]");
  const isObj = (v) => v !== null && typeof v === "object" && !Array.isArray(v);
  // Interface text in the document's words (its `labels`), by the English.
  const L = (text, vars) => window.__irender.label(text, vars);

  // ---- the nodes, resolved against state and the list item the tree is in
  const env = () => {
    const s = api.scope?.();
    return s ? { ...api.state, ...s } : api.state;
  };
  const resolve = (v) => (isObj(v) && "$irender" in v && Object.keys(v).length === 1 ? window.__irender.evaluate(v.$irender, env()) : v);
  const roots = () => {
    const n = resolve(cfg.nodes);
    return Array.isArray(n) ? n : [];
  };
  const idOf = (n) => (n == null || n.id == null ? null : String(n.id));
  const labelOf = (n) => String(n.label ?? n.name ?? "");
  // A node's children: its own, or those loaded from `childrenUrl` for `children: true`.
  const loaded = new Map(), loading = new Set(), failed = new Set();
  const kids = (n) => (Array.isArray(n?.children) ? n.children : loaded.get(idOf(n)) ?? []);
  // Has children, or will load some when opened.
  const lazy = (n) => !!cfg.childrenUrl && (n?.children === true || n?.hasChildren === true) && !loaded.has(idOf(n));
  const hasKids = (n) => kids(n).length > 0 || lazy(n);

  // id → { node, parent id, depth, index among its siblings }
  let index = new Map();
  function reindex(nodes) {
    index = new Map();
    const walk = (list, parent, depth) =>
      list.forEach((n, i) => {
        const id = idOf(n);
        if (id === null) return;
        index.set(id, { node: n, parent, depth, i });
        walk(kids(n), id, depth + 1);
      });
    walk(nodes, null, 0);
  }
  const under = (id) => {
    const out = [];
    const walk = (n) => kids(n).forEach((c) => (idOf(c) !== null && out.push(idOf(c)), walk(c)));
    walk(index.get(id)?.node);
    return out;
  };
  const path = (id) => {
    const out = [];
    for (let p = index.get(id)?.parent; p != null; p = index.get(p)?.parent) out.unshift(p);
    return out;
  };

  // ---- what's open, selected and checked
  const selected = () => (cfg.bind && api.state[cfg.bind] != null ? String(api.state[cfg.bind]) : localSel);
  let localSel = null;
  let localChecked = [];
  const checked = () => {
    const v = cfg.bindChecked ? api.state[cfg.bindChecked] : localChecked;
    return new Set((Array.isArray(v) ? v : []).map(String));
  };
  const open = new Set();
  // Open to start with: as `expanded` says, and the selected node's parents.
  reindex(roots());
  for (const [id, { depth, node }] of index) {
    const e = cfg.expanded;
    if (!kids(node).length) continue;
    if (e === true || (typeof e === "number" && depth < e) || (Array.isArray(e) && e.map(String).includes(id))) open.add(id);
  }
  if (selected()) path(selected()).forEach((p) => open.add(p));

  function state(id, on) {
    const n = index.get(id)?.node;
    const k = kids(n).map(idOf).filter((x) => x !== null);
    if (!k.length) return on.has(id) ? "true" : "false";
    const s = k.map((c) => state(c, on));
    if (s.every((x) => x === "true")) return "true";
    return on.has(id) || s.some((x) => x !== "false") ? "mixed" : "false";
  }

  // ---- search
  const fold = (s) => String(s ?? "").normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
  const query = () => fold(search?.value.trim());
  // With a query: the ids to show (matches and the nodes above them).
  function matching(q) {
    if (!q) return null;
    const show = new Set();
    for (const [id, { node }] of index) if (fold(labelOf(node)).includes(q)) [id, ...path(id)].forEach((x) => show.add(x));
    return show;
  }

  // ---- drawing (the same markup as tree.rs)
  const svg = {
    chevron: '<svg data-flip width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M6 3.5 10.5 8 6 12.5"/></svg>',
    check: '<svg data-on width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3.5 8.5 6.5 11.5 12.5 4.5"/></svg><svg data-some width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" aria-hidden="true"><path d="M4 8h8"/></svg>',
    plus: '<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" aria-hidden="true"><path d="M8 3v10M3 8h10"/></svg>',
    trash: '<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M2.5 4.5h11M6.5 4.5V3h3v1.5M4 4.5l.7 8.5h6.6l.7-8.5"/></svg>',
  };
  let focusId = null, renaming = null;
  // A node's `icon`: the icon kit's drawing the server left in a template, else the text
  // (an emoji, say).
  function iconEl(name) {
    const span = make("span", { "data-tree-icon": "", "aria-hidden": "true" });
    const tpl = el.querySelector(`template[data-tree-icon="${CSS.escape(name)}"]`);
    if (tpl) span.append(tpl.content.cloneNode(true));
    else span.textContent = name;
    return span;
  }
  const make = (tag, attrs = {}, html) => {
    const e = document.createElement(tag);
    for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
    if (html !== undefined) e.innerHTML = html;
    return e;
  };
  function draw() {
    const nodes = roots();
    reindex(nodes);
    const sel = selected(), on = checked(), q = query(), show = matching(q);
    if (focusId && !index.has(focusId)) focusId = null;
    const stop = focusId ?? (sel && index.has(sel) ? sel : null);
    let first = true;
    const items = (list, depth) => {
      const shown = list.filter((n) => idOf(n) !== null && (!show || show.has(idOf(n))));
      return shown.map((n, i) => {
        const id = idOf(n), label = labelOf(n), k = kids(n).filter((c) => idOf(c) !== null && (!show || show.has(idOf(c))));
        const isOpen = (k.length > 0 || (lazy(n) && loading.has(id))) && (open.has(id) || !!show);
        const li = make("li", {
          role: "treeitem", "data-tree-id": id, "aria-level": depth + 1, "aria-setsize": shown.length, "aria-posinset": i + 1,
          "aria-selected": id === sel ? "true" : "false", tabindex: (stop ? id === stop : first) ? 0 : -1,
        });
        first = false;
        if (hasKids(n)) li.setAttribute("aria-expanded", isOpen ? "true" : "false");
        if (loading.has(id)) li.setAttribute("aria-busy", "true");
        if (cfg.checkable) li.setAttribute("aria-checked", state(id, on));
        const row = make("div", { "data-tree-row": "", style: `--depth: ${depth}` });
        if (cfg.movable && !q) row.draggable = true;
        row.append(make("span", { "data-tree-twisty": "", "aria-hidden": "true", ...(loading.has(id) ? { "data-loading": "" } : {}) }, hasKids(n) ? svg.chevron : ""));
        if (cfg.checkable) row.append(make("span", { "data-tree-check": "", "aria-hidden": "true" }, svg.check));
        if (n.icon) row.append(iconEl(String(n.icon)));
        // Renaming: a name box in place of the label.
        if (renaming === id) {
          const box = make("input", { type: "text", "data-tree-rename": "", "aria-label": L("Rename {name}", { name: label }) });
          box.value = label;
          box.className = $("[data-tree-search]")?.className ?? "";
          row.append(box);
          li.append(row);
          return li;
        }
        const text = make("span", { "data-tree-label": "" });
        const at = q ? fold(label).indexOf(q) : -1;
        // The match, marked (positions hold: folding keeps one character per letter).
        if (at >= 0) text.append(label.slice(0, at), make("mark", {}), label.slice(at + q.length)), (text.querySelector("mark").textContent = label.slice(at, at + q.length));
        else text.textContent = label;
        row.append(text);
        if (cfg.counts && kids(n).length) {
          const c = make("span", { "data-tree-count": "", "aria-hidden": "true" });
          c.textContent = under(id).length;
          row.append(c);
        }
        if (cfg.addable || cfg.deletable) {
          const a = make("span", { "data-tree-actions": "" });
          if (cfg.addable) a.append(make("button", { type: "button", "data-tree-add": "", tabindex: -1, "aria-label": L("Add under {name}", { name: label }), title: L("Add under {name}", { name: label }) }, svg.plus));
          if (cfg.deletable) a.append(make("button", { type: "button", "data-tree-delete": "", tabindex: -1, "aria-label": L("Delete {name}", { name: label }), title: L("Delete {name}", { name: label }) }, svg.trash));
          row.append(a);
        }
        li.append(row);
        if (k.length) {
          const g = make("ul", { role: "group" });
          g.hidden = !isOpen;
          g.append(...items(kids(n), depth + 1));
          li.append(g);
        }
        return li;
      });
    };
    const hadFocus = el.contains(document.activeElement) && document.activeElement.matches("[role=treeitem]");
    list.replaceChildren(...items(nodes, 0));
    if (editing) {
      // The name box, last among the parent's children.
      const depth = editing.parent === null ? 0 : index.get(editing.parent).depth + 1;
      const holder = editing.parent === null ? list : item(editing.parent);
      let group = editing.parent === null ? list : $(":scope > ul", holder);
      if (!group) holder.append((group = make("ul", { role: "group" })));
      group.hidden = false;
      const li = make("li", { role: "none", "data-tree-new": "" });
      const row = make("div", { "data-tree-row": "", style: `--depth: ${depth}` });
      row.append(make("span", { "data-tree-twisty": "", "aria-hidden": "true" }));
      const input = make("input", { type: "text", "data-tree-name": "", "aria-label": editing.parent === null ? L("New item's name") : L("New item under {name}", { name: labelOf(index.get(editing.parent).node) }), placeholder: L("Name, then Enter") });
      input.className = $("[data-tree-search]")?.className ?? "";
      row.append(input);
      li.append(row);
      group.append(li);
      list.hidden = false;
      $("[data-tree-empty]").hidden = true;
    }
    list.hidden = !list.children.length;
    const empty = $("[data-tree-empty]");
    empty.hidden = !!list.children.length;
    if (q && !list.children.length) empty.textContent = L("No matches.");
    else empty.textContent = empty.dataset.text ?? (empty.dataset.text = empty.textContent);
    const bulk = $("[data-tree-bulk]");
    if (bulk) {
      bulk.hidden = !on.size;
      $("[data-tree-checked]", bulk).textContent = L("{n} selected", { n: on.size });
    }
    if (hadFocus) item(focusId)?.focus();
  }

  // ---- reading the drawn tree
  const item = (id) => (id == null ? null : list.querySelector(`[data-tree-id="${CSS.escape(id)}"]`));
  const visible = () => $$("[role=treeitem]", list).filter((li) => !li.closest("[hidden]"));
  const idAt = (e) => e?.closest?.("[role=treeitem]")?.getAttribute("data-tree-id") ?? null;
  const say = (text) => {
    live.textContent = "";
    setTimeout(() => (live.textContent = text), 30);
  };
  function focus(id) {
    focusId = id;
    for (const li of $$("[role=treeitem]", list)) li.tabIndex = li.getAttribute("data-tree-id") === id ? 0 : -1;
    const li = item(id);
    li?.focus();
    li?.scrollIntoView?.({ block: "nearest" });
  }

  // ---- actions
  function select(id) {
    localSel = id;
    if (cfg.bind) api.set(cfg.bind, index.get(id) ? index.get(id).node.id : id);
    else draw(), syncNode();
    api.emit("select", { id, node: index.get(id)?.node });
  }
  function toggleOpen(id, to) {
    const n = index.get(id)?.node;
    if (!hasKids(n)) return;
    const now = to ?? !open.has(id);
    now ? open.add(id) : open.delete(id);
    focusId = id;
    if (now && lazy(n) && !loading.has(id)) load(id, n);
    draw();
  }
  // Children from `childrenUrl` (`?parent=<id>`), answering a list or `{ nodes }` /
  // `{ rows }`. They load once; failing, the node closes and opening it tries again.
  async function load(id, n) {
    loading.add(id);
    failed.delete(id);
    try {
      const u = String(resolve(cfg.childrenUrl));
      const res = await fetch(u + (u.includes("?") ? "&" : "?") + new URLSearchParams({ parent: String(n.id) }), { headers: { accept: "application/json" } });
      if (!res.ok) throw new Error(String(res.status));
      const a = await res.json();
      loaded.set(id, Array.isArray(a) ? a : Array.isArray(a?.nodes) ? a.nodes : Array.isArray(a?.rows) ? a.rows : []);
      say(L(["{name}: {n} item", "{name}: {n} items"], { name: labelOf(n), n: loaded.get(id).length }));
    } catch {
      failed.add(id);
      open.delete(id);
      say(L("Couldn't load what's in {name}", { name: labelOf(n) }));
    }
    loading.delete(id);
    draw();
  }
  // Renames node `id` (`label`, or `name` where the node uses that).
  function rename(id, text) {
    const n = index.get(id)?.node;
    renaming = null;
    const name = text.trim();
    if (!n || !name || name === labelOf(n)) {
      draw();
      return focus(id);
    }
    if (cfg.bindNodes) {
      const nodes = copy(roots());
      const where = locate(nodes, id);
      const x = where.list[where.i];
      if ("label" in x || !("name" in x)) x.label = name;
      else x.name = name;
      focusId = id;
      write(nodes);
    }
    api.emit("rename", { id: n.id, label: name, node: n });
    say(L("Renamed to {name}", { name }));
    queueMicrotask(() => focus(id));
  }
  function setChecked(ids) {
    const out = [...ids];
    localChecked = out;
    if (cfg.bindChecked) api.set(cfg.bindChecked, out);
    else draw();
  }
  function toggleCheck(id) {
    const on = checked();
    const all = [id, ...under(id)];
    const was = state(id, on) === "true";
    for (const x of all) was ? on.delete(x) : on.add(x);
    // A parent is checked only while everything under it is.
    for (const p of path(id).reverse()) state(p, on) === "true" ? on.add(p) : on.delete(p);
    focusId = id;
    setChecked(on);
    say(was ? L("{name} unchecked", { name: labelOf(index.get(id).node) }) : L("{name} checked", { name: labelOf(index.get(id).node) }));
  }
  let editing = null;
  function add(parent) {
    const id = parent === null ? null : index.get(parent)?.node.id ?? parent;
    if (cfg.bindParent) api.set(cfg.bindParent, id);
    if (parent !== null) open.add(parent);
    if (!cfg.bindNodes) {
      draw();
      return api.emit("add", { parent: id });
    }
    editing = { parent };
    draw();
    $("[data-tree-name]")?.focus();
  }
  // Adds the named node, or drops the name box when it's empty or cancelled.
  function finish(name) {
    const at = editing;
    editing = null;
    const text = (name ?? "").trim();
    if (!at || !text) {
      // Back where the add began: the parent, or the Add button.
      draw();
      if (at?.parent != null) return focus(at.parent);
      return $("[data-tree-add-root]")?.focus();
    }
    const node = { id: globalThis.crypto?.randomUUID?.() ?? `n${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`, label: text };
    const nodes = copy(roots());
    childrenOf(nodes, at.parent).push(node);
    focusId = node.id;
    write(nodes);
    api.emit("add", { parent: at.parent === null ? null : index.get(at.parent)?.node.id ?? at.parent, node });
    say(L("Added {name}", { name: text }));
    queueMicrotask(() => focus(node.id));
  }

  // A copy of the nodes with `fn` applied to the list holding `id` (or the top list).
  // A copy, with the children loaded so far in place of `children: true`.
  const copy = (nodes) => {
    const fill = (list) => list.map((n) => {
      const c = { ...n };
      if (loaded.has(idOf(n)) && !Array.isArray(n.children)) c.children = loaded.get(idOf(n));
      if (Array.isArray(c.children)) c.children = fill(c.children);
      return c;
    });
    return JSON.parse(JSON.stringify(fill(nodes)));
  };
  function locate(nodes, id) {
    for (const [i, n] of nodes.entries()) {
      if (idOf(n) === id) return { list: nodes, i };
      const found = kids(n).length ? locate(n.children, id) : null;
      if (found) return found;
    }
    return null;
  }
  const childrenOf = (nodes, id) => {
    if (id === null) return nodes;
    const at = locate(nodes, id);
    const n = at.list[at.i];
    return (n.children ??= []);
  };
  function write(nodes) {
    if (cfg.bindNodes) api.set(cfg.bindNodes, nodes);
  }
  function remove(ids) {
    const top = ids.filter((id) => !path(id).some((p) => ids.includes(p)));
    if (!top.length) return;
    // Read before writing: the write redraws, and these nodes are gone from the index.
    const names = top.map((id) => labelOf(index.get(id).node));
    const original = top.map((id) => index.get(id).node.id);
    // Focus goes to the next node shown, or the one before.
    const shown = visible().map((li) => li.getAttribute("data-tree-id"));
    const gone = new Set(top.flatMap((id) => [id, ...under(id)]));
    const at = shown.indexOf(focusId ?? top[0]);
    focusId = shown.slice(at + 1).find((x) => !gone.has(x)) ?? shown.slice(0, at).reverse().find((x) => !gone.has(x)) ?? null;
    if (cfg.bindNodes) {
      const nodes = copy(roots());
      for (const id of top) {
        const where = locate(nodes, id);
        if (where) where.list.splice(where.i, 1);
      }
      write(nodes);
    }
    const on = checked();
    if (on.size) setChecked([...on].filter((x) => !gone.has(x)));
    if (selected() && gone.has(selected())) {
      localSel = null;
      if (cfg.bind) api.set(cfg.bind, null);
    }
    api.emit("delete", { ids: original });
    say(L("Deleted {names}", { names: names.join(", ") }));
    queueMicrotask(() => focusId && focus(focusId));
  }
  // Moves `id` to be child number `at` of `parent` (null: the top).
  function move(id, parent, at) {
    if (parent === id || (parent !== null && path(parent).includes(id))) return false;
    const from = index.get(id);
    if (!from) return false;
    if (from.parent === parent && (at === from.i || at === from.i + 1)) return false;
    if (cfg.bindNodes) {
      const nodes = copy(roots());
      const where = locate(nodes, id);
      const [n] = where.list.splice(where.i, 1);
      const to = childrenOf(nodes, parent);
      // Taking it out first shifts later siblings up.
      const i = from.parent === parent && at > from.i ? at - 1 : at;
      to.splice(Math.min(i, to.length), 0, n);
      write(nodes);
    }
    if (parent !== null) open.add(parent);
    focusId = id;
    const parentNode = parent === null ? null : index.get(parent).node;
    api.emit("move", { id: from.node.id, parent: parentNode ? parentNode.id : null, index: from.parent === parent && at > from.i ? at - 1 : at });
    say(parentNode ? L("Moved {name} into {parent}", { name: labelOf(from.node), parent: labelOf(parentNode) }) : L("Moved {name} to the top", { name: labelOf(from.node) }));
    queueMicrotask(() => focus(id));
    return true;
  }

  // ---- events
  el.addEventListener("click", (e) => {
    const t = e.target;
    if (t.closest("[data-tree-add-root]")) return add(null);
    if (t.closest("[data-tree-delete-checked]")) return remove([...checked()]);
    if (t.closest("[data-tree-uncheck]")) return setChecked([]);
    const id = idAt(t);
    if (!id || !list.contains(t) || t.closest("[data-tree-new]")) return;
    if (t.closest("[data-tree-add]")) return add(id);
    if (t.closest("[data-tree-delete]")) return (focusId = id), remove([id]);
    focusId = id;
    if (t.closest("[data-tree-check]")) return toggleCheck(id);
    if (t.closest("[data-tree-twisty]")) return toggleOpen(id);
    select(id);
    item(id)?.focus();
  });
  list.addEventListener("dblclick", (e) => {
    const id = idAt(e.target);
    if (!id || e.target.closest("button, [data-tree-check], [data-tree-twisty], [data-tree-rename]")) return;
    // On the name, with `renamable`: rename it; elsewhere, open or close it.
    if (cfg.renamable && e.target.closest("[data-tree-label]")) return startRename(id);
    toggleOpen(id);
  });
  function startRename(id) {
    if (!cfg.renamable || !index.has(id)) return;
    renaming = id;
    focusId = id;
    draw();
    const box = $("[data-tree-rename]");
    box?.focus();
    box?.select();
  }
  list.addEventListener("focusin", (e) => {
    const id = idAt(e.target);
    if (id && e.target.matches("[role=treeitem]")) {
      focusId = id;
      for (const li of $$("[role=treeitem]", list)) li.tabIndex = li === e.target ? 0 : -1;
    }
  });
  list.addEventListener("keydown", (e) => {
    const li = e.target.closest("[role=treeitem]");
    if (!li || e.target !== li) return;
    const id = li.getAttribute("data-tree-id"), at = index.get(id);
    const rows = visible(), i = rows.indexOf(li);
    const go = (to) => to && (e.preventDefault(), focus(to.getAttribute("data-tree-id")));
    const hasKids = li.hasAttribute("aria-expanded"), isOpen = li.getAttribute("aria-expanded") === "true";
    if (e.altKey && cfg.movable && !query()) {
      const siblings = at.parent === null ? roots() : kids(index.get(at.parent).node);
      const ids = siblings.map(idOf);
      let done = false;
      if (e.key === "ArrowUp" && at.i > 0) done = move(id, at.parent, at.i - 1);
      else if (e.key === "ArrowDown" && at.i < ids.length - 1) done = move(id, at.parent, at.i + 2);
      else if (side(e.key) === "ArrowRight" && at.i > 0) {
        const into = ids[at.i - 1];
        done = move(id, into, kids(index.get(into).node).length);
      } else if (side(e.key) === "ArrowLeft" && at.parent !== null) done = move(id, index.get(at.parent).parent, index.get(at.parent).i + 1);
      if (done || e.key.startsWith("Arrow")) e.preventDefault();
      return;
    }
    switch (side(e.key)) {
      case "ArrowDown": return go(rows[i + 1]);
      case "ArrowUp": return go(rows[i - 1]);
      case "Home": return go(rows[0]);
      case "End": return go(rows.at(-1));
      case "ArrowRight":
        e.preventDefault();
        if (hasKids && !isOpen) return toggleOpen(id, true);
        if (hasKids) return go(rows[i + 1]);
        return;
      case "ArrowLeft":
        e.preventDefault();
        if (hasKids && isOpen) return toggleOpen(id, false);
        return at.parent !== null && focus(at.parent);
      case "Enter":
        e.preventDefault();
        return select(id);
      case " ":
        e.preventDefault();
        return cfg.checkable ? toggleCheck(id) : select(id);
      case "F2":
        if (!cfg.renamable) return;
        e.preventDefault();
        return startRename(id);
      case "Delete":
        if (!cfg.deletable) return;
        e.preventDefault();
        return remove(cfg.checkable && checked().has(id) ? [...checked()] : [id]);
      case "*": {
        // Opens every sibling of the focused node, as the pattern suggests.
        e.preventDefault();
        const siblings = at.parent === null ? roots() : kids(index.get(at.parent).node);
        siblings.forEach((n) => {
          if (!hasKids(n)) return;
          open.add(idOf(n));
          if (lazy(n) && !loading.has(idOf(n))) load(idOf(n), n);
        });
        focusId = id;
        return draw();
      }
    }
  });
  list.addEventListener("keydown", (e) => {
    if (e.target.matches("[data-tree-rename]")) {
      const id = idAt(e.target);
      if (e.key === "Enter") e.preventDefault(), rename(id, e.target.value);
      else if (e.key === "Escape") e.preventDefault(), e.stopPropagation(), rename(id, "");
      return;
    }
    if (!e.target.matches("[data-tree-name]")) return;
    if (e.key === "Enter") e.preventDefault(), finish(e.target.value);
    else if (e.key === "Escape") e.preventDefault(), e.stopPropagation(), finish("");
  });
  list.addEventListener("focusout", (e) => {
    // Leaving the box keeps what was typed, as a file browser does.
    if (e.target.matches("[data-tree-rename]") && renaming) return rename(renaming, e.target.value);
    if (e.target.matches("[data-tree-name]") && editing) finish(e.target.value);
  });
  search?.addEventListener("input", () => draw());
  search?.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && search.value) {
      e.preventDefault();
      search.value = "";
      draw();
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      const first = visible()[0];
      first && focus(first.getAttribute("data-tree-id"));
    }
  });

  // ---- dragging
  let dragId = null;
  const clear = () => $$("[data-drop]", list).forEach((x) => x.removeAttribute("data-drop"));
  const zone = (li, y) => {
    const r = $("[data-tree-row]", li).getBoundingClientRect();
    const f = (y - r.top) / r.height;
    return f < 0.25 ? "before" : f > 0.75 && !(li.getAttribute("aria-expanded") === "true") ? "after" : "inside";
  };
  list.addEventListener("dragstart", (e) => {
    const id = idAt(e.target);
    if (!cfg.movable || !id) return;
    dragId = id;
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData("text/plain", labelOf(index.get(id).node));
    item(id).setAttribute("data-dragging", "");
  });
  // The node a dragged one would drop on from over `target`: not itself or inside it.
  const dropOn = (target) => {
    const li = target?.closest?.("[role=treeitem]");
    const id = li && list.contains(li) ? idAt(li) : null;
    return id && id !== dragId && !path(id).includes(dragId) ? li : null;
  };
  // Drops the dragged node at height `y` on `li`: before it, after it or inside it.
  function dropAt(li, y) {
    const id = idAt(li), where = zone(li, y), t = index.get(id);
    if (where === "inside") move(dragId, id, kids(t.node).length);
    else move(dragId, t.parent, where === "before" ? t.i : t.i + 1);
  }
  list.addEventListener("dragover", (e) => {
    if (dragId === null) return;
    const li = dropOn(e.target);
    clear();
    if (!li) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
    li.setAttribute("data-drop", zone(li, e.clientY));
  });
  list.addEventListener("dragleave", (e) => !list.contains(e.relatedTarget) && clear());
  list.addEventListener("drop", (e) => {
    if (dragId === null) return;
    e.preventDefault();
    const li = dropOn(e.target);
    clear();
    if (li) dropAt(li, e.clientY);
  });
  const endDrag = () => {
    clear();
    if (dragId !== null) item(dragId)?.removeAttribute("data-dragging");
    dragId = null;
  };
  list.addEventListener("dragend", endDrag);
  // By touch: hold a node's row, then drag it.
  window.__irenderTouchDrag(list, {
    pick: (t) => {
      const row = cfg.movable && !t.closest("button, input, a[href]") ? t.closest("[data-tree-row]") : null;
      return row && row.draggable ? row : null;
    },
    start(row) {
      dragId = idAt(row);
      item(dragId)?.setAttribute("data-dragging", "");
    },
    over(x, y, under) {
      const li = dropOn(under);
      clear();
      if (li) li.setAttribute("data-drop", zone(li, y));
    },
    drop(x, y, under) {
      const li = dropOn(under);
      clear();
      if (li && dragId !== null) dropAt(li, y);
    },
    end: endDrag,
  });

  // `bindNode`: the selected node itself, kept in step as the nodes change.
  function syncNode() {
    if (!cfg.bindNode) return;
    const sel = selected();
    const node = sel && index.has(sel) ? index.get(sel).node : null;
    if (api.state[cfg.bindNode] !== node) api.set(cfg.bindNode, node);
  }
  let last = null;
  draw();
  return {
    update() {
      const key = [roots(), selected(), [...checked()].join("\u0000")];
      if (last && key.every((v, i) => v === last[i])) return;
      last = key;
      if (selected()) path(selected()).forEach((p) => open.add(p));
      draw();
      syncNode();
    },
  };
};
