// irender-ui-kit: the `ui-mindmap` behaviour. The map is a tree in state, `{ id, label,
// children, collapsed, color }`; this lays it out round the central topic (its first
// branches shared between both sides by size, or all on the right), draws it as SVG and
// writes every change back. The keys: arrows go to the parent (towards the middle), the
// first child (away) and the siblings (up and down); Tab adds a child, Enter a sibling;
// F2, a double click or just typing writes in a topic; Space folds; Delete deletes;
// ⌘/Ctrl+Z undoes, with Shift redoes. Drag a topic onto another to move it there; drag
// the space to pan; ⌘/Ctrl and the wheel (or a pinch) zoom. Everything it writes is text.
(window.__irenderBehaviors ||= {})["ui-mindmap"] = (el, api) => {
  const cfg = api.config, L = (t, v) => window.__irender.label(t, v);
  const bar = el.querySelector("[data-mm-bar]"), stage = el.querySelector("[data-mm-stage]"), live = el.querySelector("[data-mm-live]");
  const say = (msg) => live && (live.textContent = msg);
  const NS = "http://www.w3.org/2000/svg";
  const isObj = (v) => v !== null && typeof v === "object" && !Array.isArray(v);
  const clamp = (n, a, b) => Math.min(b, Math.max(a, n));
  const s = (tag, attrs = {}, ...kids) => {
    const e = document.createElementNS(NS, tag);
    for (const [k, v] of Object.entries(attrs)) if (v !== null && v !== undefined && v !== false) e.setAttribute(k, v === true ? "" : String(v));
    e.append(...kids.filter(Boolean));
    return e;
  };
  const h = (tag, attrs = {}, ...kids) => {
    const e = document.createElement(tag);
    for (const [k, v] of Object.entries(attrs)) {
      if (v === null || v === undefined || v === false) continue;
      if (k.startsWith("on")) e.addEventListener(k.slice(2), v);
      else e.setAttribute(k, v === true ? "" : v);
    }
    e.append(...kids.filter((k) => k !== null && k !== undefined && k !== false));
    return e;
  };
  const ICONS = {
    child: "M5 12h6m0 0V6h8M11 12v6h8", sibling: "M6 6h12M6 12h12M6 18h7", trash: "M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3",
    fold: "M7 10l5 5 5-5", undo: "M9 14L4 9l5-5M4 9h10a6 6 0 0 1 0 12h-3", redo: "M15 14l5-5-5-5M20 9H10a6 6 0 0 0 0 12h3",
    minus: "M5 12h14", plus: "M12 5v14M5 12h14", fit: "M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5", download: "M12 4v11M7 10l5 5 5-5M5 20h14",
  };
  const icon = (name) => s("svg", { viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", "stroke-width": "1.8", "stroke-linecap": "round", "stroke-linejoin": "round", "aria-hidden": "true" }, s("path", { d: ICONS[name] }));

  // ---- the map
  const map = () => (isObj(api.state[cfg.bind]) ? api.state[cfg.bind] : null);
  const kids = (t) => (Array.isArray(t?.children) ? t.children.filter(isObj) : []);
  // Every topic: its parent, its level, its first branch (for the colour) and its side.
  function index(root) {
    const out = new Map();
    const walk = (t, parent, depth, branch) => {
      out.set(String(t.id), { t, parent, depth, branch });
      kids(t).forEach((c, i) => walk(c, String(t.id), depth + 1, depth === 0 ? i : branch));
    };
    if (root) walk(root, null, 0, -1);
    return out;
  }
  // The tree with the topic `id` changed by `f` (which returns the new topic, or null to drop it).
  function change(t, id, f) {
    if (String(t.id) === id) return f(t);
    const cs = kids(t);
    let changed = false;
    const next = cs.map((c) => {
      const n = change(c, id, f);
      if (n !== c) changed = true;
      return n;
    }).filter(Boolean);
    return changed ? { ...t, children: next } : t;
  }
  const newId = () => `t${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;
  let selected = null, past = [], future = [];
  function commit(next, msg) {
    past.push(map());
    if (past.length > 100) past.shift();
    future = [];
    api.set(cfg.bind, next);
    api.emit("change", { value: next });
    if (msg) say(msg);
    draw();
  }
  // ---- the selected topic's note, edited under the map
  const notes = h("div", { "data-mm-note": true, hidden: true });
  const noteId = `${cfg.id}-note`;
  // Labels: "Note"
  const noteBox = h("textarea", { id: noteId, rows: "3" });
  notes.append(h("label", { for: noteId }, L("Note")), noteBox);
  if (!cfg.readOnly) stage.after(notes);
  let noteFor = null;
  function drawNote() {
    const it = selected ? index(map()).get(selected) : null;
    notes.hidden = !it;
    if (!it) return;
    if (noteFor !== selected || document.activeElement !== noteBox) noteBox.value = it.t.note ?? "";
    noteFor = selected;
  }
  noteBox.addEventListener("change", () => {
    const it = noteFor ? index(map()).get(noteFor) : null;
    const v = noteBox.value.trim();
    if (it && v !== (it.t.note ?? "")) commit(change(map(), noteFor, (t) => { const { note, ...rest } = t; return v ? { ...rest, note: v } : rest; }), v ? L("Note saved") : L("Note removed"));
  });
  function select(id, quiet) {
    selected = id;
    if (cfg.bindSelected) api.set(cfg.bindSelected, id);
    paintSelection();
    drawBar();
    drawNote();
    if (!quiet && id) {
      const it = index(map()).get(id);
      if (it) say(`${it.t.label}, ${L("level {n}", { n: it.depth + 1 })}${kids(it.t).length ? `, ${it.t.collapsed ? L("folded") : L("open")}` : ""}`);
    }
  }

  // ---- laying it out
  const ctx = document.createElement("canvas").getContext("2d");
  const font = (depth) => (depth === 0 ? "600 17px system-ui, sans-serif" : depth === 1 ? "600 14px system-ui, sans-serif" : "14px system-ui, sans-serif");
  const MAXW = 240, HGAP = 48, VGAP = 12;
  // What a topic shows: its icon, its label, and a mark when it has a note.
  const shownText = (t) => `${t.icon ? `${t.icon} ` : ""}${t.label ?? ""}`;
  // A link of its own (`href`, http(s) or a path) opens with ⌘/Ctrl+Enter or its mark.
  const linkOf = (t) => {
    const u = typeof t?.href === "string" ? t.href.trim() : "";
    return /^(https?:\/\/|\/(?!\/)|\.{0,2}\/)/i.test(u) || (u && !/^[a-z][a-z0-9+.-]*:/i.test(u)) ? u : null;
  };
  // The marks at a topic's end: a note's and a link's.
  const marks = (t) => (t.note ? 1 : 0) + (linkOf(t) ? 1 : 0);
  const NOTE = 16;
  function size(t, depth) {
    ctx.font = font(depth);
    const pad = depth === 0 ? 22 : 14, hh = depth === 0 ? 46 : depth === 1 ? 36 : 30;
    const w = clamp(Math.ceil(ctx.measureText(shownText(t)).width) + 2 * pad + marks(t) * NOTE, 56, MAXW);
    return { w, h: hh, pad };
  }
  // Top down (`layout: down`): each topic over its children, side by side under it.
  function layoutDown(root) {
    const boxes = new Map(), HG = 16, VG = 56;
    const width = (t, depth) => {
      const b = size(t, depth);
      const cs = t.collapsed ? [] : kids(t);
      const sub = cs.reduce((sum, c, i) => sum + width(c, depth + 1) + (i ? HG : 0), 0);
      boxes.set(String(t.id), { ...b, sub: Math.max(b.w, sub), depth, side: 2 });
      return Math.max(b.w, sub);
    };
    width(root, 0);
    const place = (t, cx, y) => {
      const b = boxes.get(String(t.id));
      b.x = cx - b.w / 2;
      b.y = y;
      const cs = t.collapsed ? [] : kids(t);
      const all = cs.reduce((sum, c, i) => sum + boxes.get(String(c.id)).sub + (i ? HG : 0), 0);
      let x = cx - all / 2;
      for (const c of cs) {
        const cb = boxes.get(String(c.id));
        place(c, x + cb.sub / 2, y + b.h + VG);
        x += cb.sub + HG;
      }
    };
    place(root, 0, -size(root, 0).h / 2);
    return boxes;
  }
  function layout(root) {
    if (cfg.layout === "down") return layoutDown(root);
    const boxes = new Map();
    const height = (t, depth) => {
      const b = size(t, depth);
      const cs = t.collapsed ? [] : kids(t);
      const sub = cs.reduce((sum, c, i) => sum + height(c, depth + 1) + (i ? VGAP : 0), 0);
      const total = Math.max(b.h, sub);
      boxes.set(String(t.id), { ...b, sub: total });
      return total;
    };
    height(root, 0);
    const place = (t, depth, x, cy, side) => {
      const b = boxes.get(String(t.id));
      b.x = side > 0 ? x : x - b.w;
      b.y = cy - b.h / 2;
      b.side = side;
      b.depth = depth;
      const cs = t.collapsed ? [] : kids(t);
      let y = cy - (b.sub - 0) / 2 + (b.sub - cs.reduce((sum, c, i) => sum + boxes.get(String(c.id)).sub + (i ? VGAP : 0), 0)) / 2;
      for (const c of cs) {
        const cb = boxes.get(String(c.id));
        place(c, depth + 1, side > 0 ? b.x + b.w + HGAP : b.x - HGAP, y + cb.sub / 2, side);
        y += cb.sub + VGAP;
      }
    };
    const rb = boxes.get(String(root.id));
    rb.x = -rb.w / 2;
    rb.y = -rb.h / 2;
    rb.side = 0;
    rb.depth = 0;
    // The first branches: each on whichever side is shorter so far (the right, when even).
    const first = root.collapsed ? [] : kids(root);
    const right = [], left = [];
    let r = 0, l = 0;
    for (const c of first) {
      const hgt = boxes.get(String(c.id)).sub + VGAP;
      if (cfg.layout === "right" || r <= l) {
        right.push(c);
        r += hgt;
      } else {
        left.push(c);
        l += hgt;
      }
    }
    for (const [list, side] of [[right, 1], [left, -1]]) {
      const sub = list.reduce((sum, c, i) => sum + boxes.get(String(c.id)).sub + (i ? VGAP : 0), 0);
      let y = -sub / 2;
      for (const c of list) {
        const cb = boxes.get(String(c.id));
        place(c, 1, side > 0 ? rb.x + rb.w + HGAP : rb.x - HGAP, y + cb.sub / 2, side);
        y += cb.sub + VGAP;
      }
    }
    return boxes;
  }

  // ---- drawing
  const svg = s("svg", { "aria-hidden": "false" });
  const world = s("g");
  svg.append(world);
  stage.append(svg);
  let view = null, boxes = new Map();
  const colorOf = (it) => (it.depth === 0 ? null : it.t.color ?? (it.depth === 1 ? cfg.colors[it.branch % cfg.colors.length] : null));
  // A topic's colour: its own, else its first branch's.
  function branchColor(id, idx) {
    for (let at = idx.get(id); at; at = idx.get(at.parent)) {
      if (at.t.color) return at.t.color;
      if (at.depth === 1) return cfg.colors[at.branch % cfg.colors.length];
    }
    return null;
  }
  const domId = (id) => `${cfg.id}-t-${String(id).replace(/[^\w-]/g, "_")}`;
  let ink = "#1f2937", paper = "#ffffff";
  function draw() {
    const root = map();
    const cs = getComputedStyle(el);
    ink = cs.color || ink;
    paper = cs.backgroundColor && cs.backgroundColor !== "rgba(0, 0, 0, 0)" ? cs.backgroundColor : paper;
    world.replaceChildren();
    if (!root) return drawBar();
    const idx = index(root);
    boxes = layout(root);
    // Nothing picked (or what was is folded away or gone): the middle, in state too.
    if (selected === null || !boxes.has(selected)) {
      selected = String(root.id);
      if (cfg.bindSelected && api.state[cfg.bindSelected] !== selected) queueMicrotask(() => api.set(cfg.bindSelected, selected));
    }
    // The lines first, under the topics.
    const lines = s("g", { "aria-hidden": "true" });
    for (const [id, b] of boxes) {
      const it = idx.get(id);
      if (!it.parent || !boxes.has(it.parent)) continue;
      const p = boxes.get(it.parent), c = branchColor(id, idx) ?? ink;
      let d;
      if (b.side === 2) {
        // Down: from the parent's foot to the child's head.
        const x1 = p.x + p.w / 2, y1 = p.y + p.h, x2 = b.x + b.w / 2, y2 = b.y, my = (y1 + y2) / 2;
        d = `M${x1} ${y1}C${x1} ${my} ${x2} ${my} ${x2} ${y2}`;
      } else {
        const x1 = b.side > 0 ? p.x + p.w : p.x, y1 = p.y + p.h / 2, x2 = b.side > 0 ? b.x : b.x + b.w, y2 = b.y + b.h / 2, mx = (x1 + x2) / 2;
        d = `M${x1} ${y1}C${mx} ${y1} ${mx} ${y2} ${x2} ${y2}`;
      }
      lines.append(s("path", { d, fill: "none", stroke: c, "stroke-width": it.depth === 1 ? 3 : 2, "stroke-linecap": "round", opacity: 0.85 }));
    }
    world.append(lines);
    // Lines between topics of different branches (`links` on the central topic): dashed,
    // arched between their nearest sides, with a label in the middle.
    const cross = s("g", { "data-mm-links": true, "aria-hidden": "true" });
    for (const l of Array.isArray(root.links) ? root.links.filter(isObj) : []) {
      const a = boxes.get(String(l.from)), z = boxes.get(String(l.to));
      if (!a || !z || a === z) continue;
      const ax = a.x + a.w / 2, ay = a.y + a.h / 2, zx = z.x + z.w / 2, zy = z.y + z.h / 2;
      const bend = Math.max(40, Math.hypot(zx - ax, zy - ay) * 0.25);
      const mx = (ax + zx) / 2 - ((zy - ay) / Math.max(1, Math.hypot(zx - ax, zy - ay))) * bend, my = (ay + zy) / 2 + ((zx - ax) / Math.max(1, Math.hypot(zx - ax, zy - ay))) * bend;
      // From and to the boxes' edges, towards the bend, so the arrow shows.
      const edge = (b, cx, cy) => {
        const dx = mx - cx, dy = my - cy, k = Math.min((b.w / 2 + 3) / Math.max(1e-6, Math.abs(dx)), (b.h / 2 + 3) / Math.max(1e-6, Math.abs(dy)));
        return [cx + dx * Math.min(1, k), cy + dy * Math.min(1, k)];
      };
      const [sx1, sy1] = edge(a, ax, ay), [ex1, ey1] = edge(z, zx, zy);
      cross.append(s("path", { "data-mm-link": `${l.from}>${l.to}`, d: `M${sx1} ${sy1}Q${mx} ${my} ${ex1} ${ey1}`, fill: "none", stroke: l.color ?? ink, "stroke-width": 1.5, "stroke-dasharray": "6 5", opacity: 0.6, "marker-end": `url(#${cfg.id}-arrow)` }));
      if (l.label) {
        const t = s("text", { x: (ax + 2 * mx + zx) / 4, y: (ay + 2 * my + zy) / 4, "text-anchor": "middle", "font-size": 12, fill: ink, opacity: 0.8, stroke: paper, "stroke-width": 4, "paint-order": "stroke" });
        t.textContent = String(l.label);
        cross.append(t);
      }
    }
    if (cross.children.length) {
      cross.prepend(s("defs", {}, s("marker", { id: `${cfg.id}-arrow`, viewBox: "0 0 10 10", refX: 9, refY: 5, markerWidth: 7, markerHeight: 7, orient: "auto-start-reverse" }, s("path", { d: "M0 0L10 5L0 10z", fill: ink, opacity: 0.6 }))));
      world.append(cross);
    }
    // Then the topics, as tree items.
    for (const [id, b] of boxes) {
      const it = idx.get(id), c = branchColor(id, idx), n = kids(it.t).length;
      const siblings = it.parent ? kids(idx.get(it.parent).t) : [it.t];
      const g = s("g", {
        id: domId(id), "data-mm-topic": id, role: "treeitem", "aria-level": it.depth + 1, "aria-selected": selected === id ? "true" : "false",
        "aria-expanded": n ? (it.t.collapsed ? "false" : "true") : null, "aria-setsize": siblings.length, "aria-posinset": siblings.findIndex((x) => String(x.id) === id) + 1,
        "aria-label": shownText(it.t), "aria-description": it.t.note ? String(it.t.note) : null,
      });
      const fill = it.depth === 0 ? ink : it.depth === 1 ? `color-mix(in srgb, ${c} 16%, ${paper})` : paper;
      g.append(s("rect", { "data-mm-ring": true, x: b.x - 4, y: b.y - 4, width: b.w + 8, height: b.h + 8, rx: b.h / 2 + 4, fill: "none", stroke: "transparent", "stroke-width": 2.5 }));
      g.append(s("rect", { x: b.x, y: b.y, width: b.w, height: b.h, rx: it.depth === 0 ? 12 : b.h / 2, fill, stroke: it.depth === 0 ? ink : c ?? ink, "stroke-width": it.depth === 0 ? 0 : it.depth === 1 ? 2 : 1.5 }));
      // The label, cut short with … when it's longer than a topic is wide.
      ctx.font = font(it.depth);
      const room = b.w - 2 * b.pad - marks(it.t) * NOTE;
      let text = shownText(it.t);
      if (ctx.measureText(text).width > room) {
        while (text.length > 1 && ctx.measureText(`${text}…`).width > room) text = text.slice(0, -1);
        text = `${text}…`;
      }
      const t = s("text", { x: b.x + (b.w - marks(it.t) * NOTE) / 2, y: b.y + b.h / 2, "text-anchor": "middle", fill: it.depth === 0 ? paper : ink, "font-size": it.depth === 0 ? 17 : 14, "font-weight": it.depth < 2 ? 600 : 400 });
      t.textContent = text;
      g.append(t, s("title", {}, document.createTextNode(shownText(it.t) + (it.t.note ? `\n\n${it.t.note}` : "") + (linkOf(it.t) ? `\n\n${linkOf(it.t)}` : ""))));
      // A topic with a link has a mark at its end that opens it.
      if (linkOf(it.t)) {
        const lx = b.x + b.w - b.pad / 2 - NOTE / 2 - (it.t.note ? NOTE : 0);
        const m = s("text", { "data-mm-href": id, x: lx, y: b.y + b.h / 2, "text-anchor": "middle", fill: it.depth === 0 ? paper : ink, "font-size": 13, style: "cursor: pointer" });
        m.textContent = "↗";
        g.append(m);
        g.setAttribute("aria-description", [it.t.note, L("Has a link: ⌘ or Ctrl and Enter opens it")].filter(Boolean).join(". "));
      }
      // A topic with a note has a mark at its end.
      if (it.t.note) {
        const m = s("text", { "data-mm-has-note": true, x: b.x + b.w - b.pad / 2 - NOTE / 2, y: b.y + b.h / 2, "text-anchor": "middle", fill: it.depth === 0 ? paper : ink, "font-size": 12, opacity: 0.7 });
        m.textContent = "≡";
        g.append(m);
      }
      // A folded topic says how many are inside.
      if (n && it.t.collapsed) {
        const cx = b.side < 0 ? b.x - 12 : b.x + b.w + 12, cy = b.y + b.h / 2;
        const badge = s("g", { "data-mm-fold": id, style: "cursor: pointer" }, s("circle", { cx, cy, r: 10, fill: paper, stroke: c ?? ink, "stroke-width": 1.5 }));
        const nt = s("text", { x: cx, y: cy, "text-anchor": "middle", "font-size": 11, fill: ink, "font-weight": 600 });
        nt.textContent = String(n);
        badge.append(nt);
        g.append(badge);
      }
      world.append(g);
    }
    // To start with: the central topic in the middle, or the whole map if it won't fit.
    if (!view) {
      centre();
      // A tree from the top starts with its top near the top.
      if (cfg.layout === "down") setView({ ...view, y: (boxes.get(String(root.id))?.y ?? 0) - 24 / view.zoom });
      const r = stage.getBoundingClientRect(), bs = [...boxes.values()];
      const wide = Math.max(...bs.map((b) => b.x + b.w)) - Math.min(...bs.map((b) => b.x)), tall = Math.max(...bs.map((b) => b.y + b.h)) - Math.min(...bs.map((b) => b.y));
      if (r.width && (wide + 64 > r.width || tall + 64 > r.height)) fit();
    }
    paint();
    drawBar();
    drawNote();
  }
  function paintSelection() {
    for (const g of world.querySelectorAll("[data-mm-topic]")) {
      const on = g.getAttribute("data-mm-topic") === selected;
      g.setAttribute("aria-selected", on ? "true" : "false");
      g.querySelector("[data-mm-ring]").setAttribute("stroke", on ? "var(--color-primary)" : "transparent");
    }
    if (selected) stage.setAttribute("aria-activedescendant", domId(selected));
  }
  function paint() {
    if (!view) return;
    world.setAttribute("transform", `scale(${view.zoom}) translate(${-view.x} ${-view.y})`);
    const z = bar.querySelector("[data-mm-zoom]");
    if (z) z.textContent = `${Math.round(view.zoom * 100)}%`;
    paintSelection();
    if (editing) placeEditor();
  }
  function setView(v) {
    view = { x: v.x, y: v.y, zoom: clamp(v.zoom, 0.2, 4) };
    paint();
  }
  // The central topic in the middle.
  function centre() {
    const r = stage.getBoundingClientRect(), z = view?.zoom ?? 1;
    setView({ zoom: z, x: -(r.width || 600) / (2 * z), y: -(r.height || 400) / (2 * z) });
  }
  function fit() {
    if (!boxes.size) return;
    const r = stage.getBoundingClientRect(), bs = [...boxes.values()];
    const x0 = Math.min(...bs.map((b) => b.x)), y0 = Math.min(...bs.map((b) => b.y)), x1 = Math.max(...bs.map((b) => b.x + b.w)), y1 = Math.max(...bs.map((b) => b.y + b.h));
    const pad = 32, zoom = clamp(Math.min((r.width - 2 * pad) / (x1 - x0), (r.height - 2 * pad) / (y1 - y0)), 0.2, 1.5);
    setView({ zoom, x: x0 - (r.width / zoom - (x1 - x0)) / 2, y: y0 - (r.height / zoom - (y1 - y0)) / 2 });
  }
  function zoomBy(f, sx, sy) {
    const r = stage.getBoundingClientRect(), cx = sx ?? r.width / 2, cy = sy ?? r.height / 2;
    const bx = cx / view.zoom + view.x, by = cy / view.zoom + view.y, zoom = clamp(view.zoom * f, 0.2, 4);
    setView({ zoom, x: bx - cx / zoom, y: by - cy / zoom });
  }
  // The selected topic into view.
  function reveal(id) {
    const b = boxes.get(id), r = stage.getBoundingClientRect();
    if (!b || !view) return;
    const sx = (b.x - view.x) * view.zoom, sy = (b.y - view.y) * view.zoom, sw = b.w * view.zoom, sh = b.h * view.zoom, m = 24;
    let { x, y } = view;
    if (sx < m) x += (sx - m) / view.zoom;
    else if (sx + sw > r.width - m) x += (sx + sw - r.width + m) / view.zoom;
    if (sy < m) y += (sy - m) / view.zoom;
    else if (sy + sh > r.height - m) y += (sy + sh - r.height + m) / view.zoom;
    setView({ ...view, x, y });
  }

  // ---- the toolbar
  function drawBar() {
    const it = selected ? index(map()).get(selected) : null, ro = cfg.readOnly;
    const btn = (name, label, onclick, disabled) => h("button", { type: "button", "aria-label": label, title: label, disabled: disabled ? true : null, onclick }, icon(name));
    const parts = [];
    if (!ro) {
      parts.push(h("button", { type: "button", title: `${L("Add a topic inside")} (Tab)`, disabled: it ? null : true, onclick: () => addChild() }, icon("child"), L("Topic")));
      parts.push(btn("sibling", `${L("Add a topic beside")} (Enter)`, () => addSibling(), !it));
      parts.push(btn("trash", `${L("Delete")} (Delete)`, () => remove(), !it || it.depth === 0));
    }
    parts.push(btn("fold", `${L("Fold or unfold")} (Space)`, () => fold(), !it || !kids(it.t).length));
    if (!ro) {
      parts.push(h("span", { "data-mm-sep": true }));
      parts.push(btn("undo", L("Undo"), undo, !past.length));
      parts.push(btn("redo", L("Redo"), redo, !future.length));
    }
    parts.push(h("span", { "data-mm-sep": true }));
    parts.push(btn("minus", L("Zoom out"), () => zoomBy(1 / 1.25)));
    parts.push(h("span", { "data-mm-zoom": true }, `${Math.round((view?.zoom ?? 1) * 100)}%`));
    parts.push(btn("plus", L("Zoom in"), () => zoomBy(1.25)));
    parts.push(btn("fit", L("Fit everything"), fit));
    parts.push(btn("download", L("Export as SVG"), exportSvg));
    const had = bar.contains(document.activeElement) ? document.activeElement.title : null;
    bar.replaceChildren(...parts);
    if (had) [...bar.querySelectorAll("button")].find((b) => b.title === had)?.focus();
  }

  // ---- changing the map
  function addChild(id = selected) {
    const root = map();
    if (cfg.readOnly || !root || id === null) return;
    const t = { id: newId(), label: L("New topic") };
    commit(change(root, id, (p) => ({ ...p, collapsed: false, children: [...kids(p), t] })), L("Topic added"));
    select(t.id, true);
    reveal(t.id);
    edit(t.id, null);
  }
  function addSibling() {
    const root = map(), idx = index(root), it = selected && idx.get(selected);
    if (cfg.readOnly || !it) return;
    if (!it.parent) return addChild(selected);
    const t = { id: newId(), label: L("New topic") };
    commit(change(root, it.parent, (p) => {
      const cs = kids(p), at = cs.findIndex((c) => String(c.id) === selected);
      return { ...p, children: [...cs.slice(0, at + 1), t, ...cs.slice(at + 1)] };
    }), L("Topic added"));
    select(t.id, true);
    reveal(t.id);
    edit(t.id, null);
  }
  function remove() {
    const root = map(), idx = index(root), it = selected && idx.get(selected);
    if (cfg.readOnly || !it || !it.parent) return;
    const siblings = kids(idx.get(it.parent).t), at = siblings.findIndex((c) => String(c.id) === selected);
    const next = siblings[at + 1] ?? siblings[at - 1];
    const gone = it.t.label, inside = index(it.t).size - 1;
    commit(change(root, selected, () => null), inside ? L("{name} and {n} inside it deleted", { name: gone, n: inside }) : L("{name} deleted", { name: gone }));
    select(next ? String(next.id) : it.parent, true);
  }
  function fold(id = selected) {
    const root = map(), it = id && index(root).get(id);
    if (!it || !kids(it.t).length) return;
    const collapsed = !it.t.collapsed;
    // Folding is looking, not changing: allowed read only, and not undone.
    api.set(cfg.bind, change(root, id, (t) => ({ ...t, collapsed })));
    draw();
    say(collapsed ? L("{name} folded, {n} inside", { name: it.t.label, n: kids(it.t).length }) : L("{name} unfolded", { name: it.t.label }));
  }
  // A topic's link, in a new tab.
  function open(id = selected) {
    const it = id && index(map()).get(id), u = it && linkOf(it.t);
    if (!u) return;
    window.open(new URL(u, location.href).href, "_blank", "noopener");
    api.emit("open", { id, href: u });
  }
  // Topic `id` among its siblings, `by` places up or down; or before or after `target`
  // (a topic dragged onto another's top or foot), into that one's parent.
  function reorder(id, by) {
    const root = map(), idx = index(root), it = idx.get(id);
    if (cfg.readOnly || !it?.parent) return;
    const cs = kids(idx.get(it.parent).t), at = cs.findIndex((c) => String(c.id) === id), to = at + by;
    if (to < 0 || to >= cs.length) return;
    const next = cs.slice();
    next.splice(at, 1);
    next.splice(to, 0, it.t);
    // Labels: "{name} moved to {n} of {total}"
    commit(change(root, it.parent, (p) => ({ ...p, children: next })), L("{name} moved to {n} of {total}", { name: it.t.label, n: to + 1, total: cs.length }));
    select(id, true);
  }
  function place(id, target, after) {
    const root = map(), idx = index(root), it = idx.get(id), tg = idx.get(target);
    if (cfg.readOnly || !it?.parent || !tg?.parent || id === target || index(it.t).has(target)) return;
    const without = change(root, id, () => null);
    const placed = change(without, tg.parent, (p) => {
      const cs = kids(p), at = cs.findIndex((c) => String(c.id) === target);
      return { ...p, children: [...cs.slice(0, at + (after ? 1 : 0)), it.t, ...cs.slice(at + (after ? 1 : 0))] };
    });
    // Labels: "{name} moved before {target}", "{name} moved after {target}"
    commit(placed, after ? L("{name} moved after {target}", { name: it.t.label, target: tg.t.label }) : L("{name} moved before {target}", { name: it.t.label, target: tg.t.label }));
    select(id, true);
  }
  // Moves topic `id` (and what's inside it) into `target`, last.
  function move(id, target) {
    const root = map(), idx = index(root), it = idx.get(id);
    if (!it || !it.parent || id === target || index(it.t).has(target) || it.parent === target) return;
    const without = change(root, id, () => null);
    commit(change(without, target, (p) => ({ ...p, collapsed: false, children: [...kids(p), it.t] })), L("{name} moved into {target}", { name: it.t.label, target: idx.get(target).t.label }));
    select(id, true);
  }
  function undo() {
    if (!past.length) return;
    future.push(map());
    api.set(cfg.bind, past.pop());
    say(L("Undone"));
    draw();
  }
  function redo() {
    if (!future.length) return;
    past.push(map());
    api.set(cfg.bind, future.pop());
    say(L("Redone"));
    draw();
  }

  // ---- writing in a topic: an input over it
  let editing = null, editor = null;
  function placeEditor() {
    const b = boxes.get(editing);
    if (!b || !editor) return;
    Object.assign(editor.style, { left: `${(b.x - view.x) * view.zoom}px`, top: `${(b.y - view.y) * view.zoom}px`, width: `${Math.max(b.w, 160)}px`, height: `${b.h}px`, transform: `scale(${view.zoom})` });
  }
  function edit(id, first) {
    if (cfg.readOnly) return;
    finishEdit(true);
    const it = index(map()).get(id);
    if (!it) return;
    editing = id;
    editor = h("input", { type: "text", "data-mm-edit": true, "aria-label": L("Topic"), value: first ?? it.t.label ?? "" });
    stage.append(editor);
    placeEditor();
    editor.focus();
    if (first === null || first === undefined) editor.select();
    else editor.setSelectionRange(editor.value.length, editor.value.length);
    editor.addEventListener("keydown", (e) => {
      e.stopPropagation();
      if (e.key === "Enter" || e.key === "Tab") {
        e.preventDefault();
        finishEdit(true);
        stage.focus();
        // Enter writes it; Tab writes it and goes on to a new topic inside it.
        if (e.key === "Tab") addChild();
      } else if (e.key === "Escape") {
        e.preventDefault();
        finishEdit(false);
        stage.focus();
      }
    });
    editor.addEventListener("blur", () => finishEdit(true));
  }
  function finishEdit(keep) {
    if (!editing || !editor) return;
    const id = editing, label = editor.value.trim(), ed = editor;
    editing = null;
    editor = null;
    ed.remove();
    const it = index(map()).get(id);
    if (keep && it && label && label !== it.t.label) commit(change(map(), id, (t) => ({ ...t, label })), L("Renamed to {name}", { name: label }));
  }

  // ---- the keys
  stage.addEventListener("keydown", (e) => {
    if (e.target !== stage) return;
    const root = map();
    if (!root) return;
    const idx = index(root), it = idx.get(selected), mod = e.metaKey || e.ctrlKey;
    if (!it) return;
    const go = (id) => {
      if (!id) return;
      e.preventDefault();
      select(id);
      reveal(id);
    };
    const b = boxes.get(selected);
    const visibleKids = it.t.collapsed ? [] : kids(it.t).map((c) => String(c.id)).filter((c) => boxes.has(c));
    const siblings = it.parent ? kids(idx.get(it.parent).t).map((c) => String(c.id)) : [];
    const at = siblings.indexOf(selected);
    const towards = (side) => (it.depth === 0 ? visibleKids.find((c) => boxes.get(c)?.side === side) : b.side === side ? visibleKids[0] : it.parent);
    if (mod && e.key === "Enter") return e.preventDefault(), open();
    // Alt and the arrows along the siblings: the topic moves among them.
    const along = cfg.layout === "down" ? { ArrowLeft: -1, ArrowRight: 1 } : { ArrowUp: -1, ArrowDown: 1 };
    if (e.altKey && along[e.key]) return e.preventDefault(), reorder(selected, along[e.key]);
    if (cfg.layout === "down") {
      // Down: ↓ into the first child, ↑ to the parent, ← and → along the siblings.
      const nav = { ArrowDown: visibleKids[0], ArrowUp: it.parent, ArrowLeft: siblings[at - 1], ArrowRight: siblings[at + 1] }[e.key];
      if (e.key in { ArrowDown: 1, ArrowUp: 1, ArrowLeft: 1, ArrowRight: 1 }) return go(nav);
    }
    if (mod && e.key.toLowerCase() === "z") return e.preventDefault(), e.shiftKey ? redo() : undo();
    if (mod && e.key.toLowerCase() === "y") return e.preventDefault(), redo();
    switch (e.key) {
      case "ArrowRight": return go(towards(1));
      case "ArrowLeft": return go(towards(-1));
      case "ArrowUp": return go(it.depth === 0 ? null : siblings[at - 1]);
      case "ArrowDown": return go(it.depth === 0 ? null : siblings[at + 1]);
      case "Home": return go(String(root.id));
      case "Tab":
        if (cfg.readOnly) return;
        e.preventDefault();
        return addChild();
      case "Enter":
        if (cfg.readOnly) return;
        e.preventDefault();
        return addSibling();
      case "F2":
        e.preventDefault();
        return edit(selected, null);
      case " ":
        e.preventDefault();
        return fold();
      case "Delete":
      case "Backspace":
        e.preventDefault();
        return remove();
    }
    if (e.key === "+" || e.key === "=") return zoomBy(1.25);
    if (e.key === "-") return zoomBy(1 / 1.25);
    if (e.key === "0") return fit();
    // Typing writes over the topic, as in a spreadsheet's cell.
    if (e.key.length === 1 && !mod && !e.altKey && !cfg.readOnly) {
      e.preventDefault();
      edit(selected, e.key);
    }
  });

  // ---- the pointer: a topic selects (and drags onto another to move); the space pans
  const pointers = new Map();
  let drag = null, pinch = null;
  const local = (e) => {
    const r = stage.getBoundingClientRect();
    return { sx: e.clientX - r.left, sy: e.clientY - r.top };
  };
  const topicAt = (x, y, except) => {
    for (const n of document.elementsFromPoint(x, y)) {
      const g = n.closest?.("[data-mm-topic]");
      if (g && stage.contains(g) && g.getAttribute("data-mm-topic") !== except) return g.getAttribute("data-mm-topic");
    }
    return null;
  };
  stage.addEventListener("pointerdown", (e) => {
    if (editing) finishEdit(true);
    if (e.target.closest?.("[data-mm-edit]")) return;
    stage.focus({ preventScroll: true });
    pointers.set(e.pointerId, local(e));
    try {
      stage.setPointerCapture(e.pointerId);
    } catch {}
    if (pointers.size === 2) {
      drag = null;
      const [a, b] = [...pointers.values()];
      pinch = { d: Math.hypot(a.sx - b.sx, a.sy - b.sy), mid: { sx: (a.sx + b.sx) / 2, sy: (a.sy + b.sy) / 2 }, view: { ...view } };
      return;
    }
    const foldBadge = e.target.closest?.("[data-mm-fold]");
    if (foldBadge) return fold(foldBadge.getAttribute("data-mm-fold"));
    const link = e.target.closest?.("[data-mm-href]");
    if (link) return select(link.getAttribute("data-mm-href")), open(link.getAttribute("data-mm-href"));
    const id = e.target.closest?.("[data-mm-topic]")?.getAttribute("data-mm-topic");
    const { sx, sy } = local(e);
    if (id) {
      select(id);
      drag = { kind: "topic", id, sx, sy, moved: false, target: null };
    } else {
      drag = { kind: "pan", sx, sy, view: { ...view } };
      stage.setAttribute("data-panning", "");
    }
  });
  stage.addEventListener("pointermove", (e) => {
    if (!pointers.has(e.pointerId)) return;
    pointers.set(e.pointerId, local(e));
    if (pinch && pointers.size >= 2) {
      const [a, b] = [...pointers.values()];
      const d = Math.hypot(a.sx - b.sx, a.sy - b.sy), mid = { sx: (a.sx + b.sx) / 2, sy: (a.sy + b.sy) / 2 };
      const zoom = clamp(pinch.view.zoom * (d / Math.max(1, pinch.d)), 0.2, 4);
      const bx = pinch.mid.sx / pinch.view.zoom + pinch.view.x, by = pinch.mid.sy / pinch.view.zoom + pinch.view.y;
      return setView({ zoom, x: bx - mid.sx / zoom, y: by - mid.sy / zoom });
    }
    if (!drag) return;
    const { sx, sy } = local(e);
    if (drag.kind === "pan") return setView({ ...drag.view, x: drag.view.x - (sx - drag.sx) / view.zoom, y: drag.view.y - (sy - drag.sy) / view.zoom });
    // A topic dragged: it follows the pointer, and the one under it is marked.
    if (cfg.readOnly || !index(map()).get(drag.id)?.parent) return;
    if (!drag.moved && Math.hypot(sx - drag.sx, sy - drag.sy) < 5) return;
    drag.moved = true;
    const g = world.querySelector(`[data-mm-topic="${CSS.escape(drag.id)}"]`);
    g?.setAttribute("transform", `translate(${(sx - drag.sx) / view.zoom} ${(sy - drag.sy) / view.zoom})`);
    g?.setAttribute("opacity", "0.7");
    g?.setAttribute("pointer-events", "none");
    const target = topicAt(e.clientX, e.clientY, drag.id);
    const ok = target && !index(index(map()).get(drag.id).t).has(target);
    for (const t of world.querySelectorAll("[data-mm-topic] [data-mm-ring]")) t.removeAttribute("stroke-dasharray");
    world.querySelector("[data-mm-slot]")?.remove();
    drag.target = ok ? target : null;
    drag.zone = "into";
    if (ok) {
      // Its first or last quarter (across, going down): before or after it, as a sibling;
      // the middle: into it.
      const tb = boxes.get(target), tg = index(map()).get(target);
      const r = world.querySelector(`[data-mm-topic="${CSS.escape(target)}"] rect:not([data-mm-ring])`).getBoundingClientRect();
      const f = cfg.layout === "down" ? (e.clientX - r.left) / r.width : (e.clientY - r.top) / r.height;
      drag.zone = tg.parent && f < 0.25 ? "before" : tg.parent && f > 0.75 ? "after" : "into";
      if (drag.zone === "into") {
        const ring = world.querySelector(`[data-mm-topic="${CSS.escape(target)}"] [data-mm-ring]`);
        ring?.setAttribute("stroke", "var(--color-primary)");
        ring?.setAttribute("stroke-dasharray", "5 4");
      } else {
        // A bar where it would go.
        const down = cfg.layout === "down", before = drag.zone === "before";
        const bar = down
          ? s("rect", { "data-mm-slot": true, x: before ? tb.x - 9 : tb.x + tb.w + 5, y: tb.y, width: 4, height: tb.h, rx: 2, fill: "var(--color-primary)" })
          : s("rect", { "data-mm-slot": true, x: tb.x, y: before ? tb.y - 8 : tb.y + tb.h + 4, width: tb.w, height: 4, rx: 2, fill: "var(--color-primary)" });
        world.append(bar);
        paintSelection();
      }
    } else paintSelection();
  });
  function end(e) {
    pointers.delete(e.pointerId);
    if (pinch) {
      if (pointers.size < 2) pinch = null;
      return;
    }
    stage.removeAttribute("data-panning");
    const d = drag;
    drag = null;
    world.querySelector("[data-mm-slot]")?.remove();
    if (d?.kind === "topic" && d.moved) {
      if (d.target && d.zone !== "into") place(d.id, d.target, d.zone === "after");
      else if (d.target) move(d.id, d.target);
      else draw();
    }
  }
  stage.addEventListener("pointerup", end);
  stage.addEventListener("pointercancel", end);
  stage.addEventListener("dblclick", (e) => {
    const id = e.target.closest?.("[data-mm-topic]")?.getAttribute("data-mm-topic");
    if (id) edit(id, null);
  });
  stage.addEventListener("wheel", (e) => {
    e.preventDefault();
    const { sx, sy } = local(e);
    if (e.ctrlKey || e.metaKey) zoomBy(Math.exp(-e.deltaY / 300), sx, sy);
    else setView({ ...view, x: view.x + e.deltaX / view.zoom, y: view.y + e.deltaY / view.zoom });
  }, { passive: false });

  // ---- export: the map as an SVG file, as drawn
  function exportSvg() {
    if (!boxes.size) return;
    const bs = [...boxes.values()], pad = 24;
    const x0 = Math.min(...bs.map((b) => b.x)) - pad, y0 = Math.min(...bs.map((b) => b.y)) - pad;
    const w = Math.max(...bs.map((b) => b.x + b.w)) - x0 + pad, hh = Math.max(...bs.map((b) => b.y + b.h)) - y0 + pad;
    const out = s("svg", { xmlns: NS, viewBox: `${x0} ${y0} ${w} ${hh}`, width: Math.round(w), height: Math.round(hh), "font-family": "system-ui, sans-serif" });
    out.append(s("rect", { x: x0, y: y0, width: w, height: hh, fill: paper }));
    const copy = world.cloneNode(true);
    copy.removeAttribute("transform");
    copy.querySelectorAll("[data-mm-ring]").forEach((r) => r.remove());
    copy.querySelectorAll("[role]").forEach((n) => ["role", "aria-selected", "aria-expanded", "aria-level", "aria-setsize", "aria-posinset", "aria-label", "id"].forEach((a) => n.removeAttribute(a)));
    out.append(copy);
    const blob = new Blob([new XMLSerializer().serializeToString(out)], { type: "image/svg+xml" });
    const a = h("a", { href: URL.createObjectURL(blob), download: `${String(map()?.label ?? "mind map").replace(/[\\/:*?"<>|]/g, "-")}.svg` });
    document.body.append(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(a.href), 1000);
    say(L("Exported"));
  }

  new ResizeObserver(() => paint()).observe(stage);
  draw();
  let seen = JSON.stringify(map());
  return {
    update() {
      // The map changed elsewhere (a page's own button, a server).
      const now = JSON.stringify(map());
      if (now === seen || drag || editing) return;
      seen = now;
      draw();
    },
  };
};
