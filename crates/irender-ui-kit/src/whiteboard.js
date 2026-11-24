// irender-ui-kit: the `ui-whiteboard` behaviour. The items are in state, in the board's own
// units; this draws them as SVG in a view (`{ x, y, zoom }`) and handles the tools:
// select (click, Shift-click, a box; drag to move; the corner to resize), hand (or Space,
// the middle button, two fingers), pen, sticky note, rectangle, ellipse, arrow and text.
// Double-click a note or text to edit it. The keys: V H P N R O A T pick tools; ⌘/Ctrl+Z
// undoes, with Shift redoes; Delete deletes; ⌘/Ctrl+D duplicates; arrows nudge (Shift ten);
// Tab goes to the next item while one's selected; Escape lets go. The wheel scrolls, with
// ⌘/Ctrl (or a pinch) it zooms. Everything it writes is text or SVG it builds, never HTML.
(window.__irenderBehaviors ||= {})["ui-whiteboard"] = (el, api) => {
  const cfg = api.config, L = (t, v) => window.__irender.label(t, v);
  const bar = el.querySelector("[data-wb-bar]"), stage = el.querySelector("[data-wb-stage]"), live = el.querySelector("[data-wb-live]");
  const say = (msg) => live && (live.textContent = msg);
  const NS = "http://www.w3.org/2000/svg";
  const isObj = (v) => v !== null && typeof v === "object" && !Array.isArray(v);
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
  const clamp = (n, a, b) => Math.min(b, Math.max(a, n));
  const NOTE_COLORS = ["#fef08a", "#fbcfe8", "#bfdbfe", "#bbf7d0", "#ddd6fe", "#fed7aa"];
  // Labels: "Select", "Hand", "Pen", "Sticky note", "Rectangle", "Ellipse", "Arrow", "Text"
  const TOOLS = { select: ["Select", "V"], hand: ["Hand", "H"], pen: ["Pen", "P"], note: ["Sticky note", "N"], rect: ["Rectangle", "R"], ellipse: ["Ellipse", "O"], arrow: ["Arrow", "A"], text: ["Text", "T"] };
  const KIND = { pen: "Drawing", note: "Sticky note", rect: "Rectangle", ellipse: "Ellipse", arrow: "Arrow", text: "Text" };
  const ICONS = {
    select: "M5 3l13 7-6 1.5L9 18z", hand: "M8 13V5.5a1.5 1.5 0 0 1 3 0V11m0-1V4.5a1.5 1.5 0 0 1 3 0V11m0-4.5a1.5 1.5 0 0 1 3 0V14a6 6 0 0 1-6 6h-1a6 6 0 0 1-5-2.7L3 15a1.5 1.5 0 0 1 2.5-1.7L8 16",
    pen: "M4 20l4-1 11-11-3-3L5 16zM14 6l3 3", note: "M5 4h14v10l-6 6H5zM13 20v-6h6", rect: "M4 6h16v12H4z", ellipse: "M12 5c4.4 0 8 3.1 8 7s-3.6 7-8 7-8-3.1-8-7 3.6-7 8-7z",
    arrow: "M5 19L19 5M10 5h9v9", text: "M5 6V4h14v2M12 4v16M9 20h6", undo: "M9 14L4 9l5-5M4 9h10a6 6 0 0 1 0 12h-3", redo: "M15 14l5-5-5-5M20 9H10a6 6 0 0 0 0 12h3",
    trash: "M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3", minus: "M5 12h14", plus: "M12 5v14M5 12h14", fit: "M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5", download: "M12 4v11M7 10l5 5 5-5M5 20h14",
  };
  const icon = (name) => s("svg", { viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", "stroke-width": "1.8", "stroke-linecap": "round", "stroke-linejoin": "round", "aria-hidden": "true" }, s("path", { d: ICONS[name] }));

  // ---- the items and the view
  const items = () => (Array.isArray(api.state[cfg.bind]) ? api.state[cfg.bind].filter((i) => isObj(i) && i.id != null) : []);
  // A little room round the board's corner to start with.
  let view = { x: -24, y: -24, zoom: 1 };
  if (cfg.bindView && isObj(api.state[cfg.bindView])) view = { x: 0, y: 0, zoom: 1, ...api.state[cfg.bindView] };
  let tool = cfg.readOnly ? "hand" : "select", color = cfg.colors[0], selected = new Set(), past = [], future = [];
  const newId = () => `i${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`;
  function commit(next, msg) {
    past.push(items());
    if (past.length > 100) past.shift();
    future = [];
    api.set(cfg.bind, next);
    api.emit("change", { value: next });
    if (msg) say(msg);
    draw();
  }
  function setView(v, write = true) {
    view = { x: v.x, y: v.y, zoom: clamp(v.zoom, 0.1, 8) };
    if (write && cfg.bindView) api.set(cfg.bindView, { ...view });
    paint();
  }
  // From a point of the stage to the board, and back.
  const toBoard = (sx, sy) => ({ x: sx / view.zoom + view.x, y: sy / view.zoom + view.y });
  const local = (e) => {
    const r = stage.getBoundingClientRect();
    return { sx: e.clientX - r.left, sy: e.clientY - r.top };
  };
  // An item's box, in board units.
  function box(it) {
    if (it.type === "pen" || it.type === "arrow") {
      const pts = it.points ?? [];
      if (!pts.length) return { x: it.x ?? 0, y: it.y ?? 0, w: 0, h: 0 };
      const xs = pts.map((p) => p[0] + (it.x ?? 0)), ys = pts.map((p) => p[1] + (it.y ?? 0));
      const x = Math.min(...xs), y = Math.min(...ys);
      return { x, y, w: Math.max(...xs) - x, h: Math.max(...ys) - y };
    }
    return { x: it.x ?? 0, y: it.y ?? 0, w: it.w ?? 0, h: it.h ?? 0 };
  }
  const named = (it) => {
    const words = it.text ? `: ${String(it.text).slice(0, 60)}` : "";
    return `${L(KIND[it.type] ?? "Item")}${words}`;
  };

  // ---- drawing the board
  const svg = s("svg", { "aria-hidden": "true" });
  const world = s("g");
  const overlay = s("g");
  svg.append(world, overlay);
  stage.append(svg);
  const empty = h("p", { "data-wb-empty": true }, cfg.readOnly ? L("Nothing on the board yet") : L("Pick a tool and draw, or press N for a sticky note"));
  stage.append(empty);
  function smooth(pts) {
    if (pts.length < 2) return pts.length ? `M${pts[0][0]} ${pts[0][1]}l0.01 0` : "";
    let d = `M${pts[0][0]} ${pts[0][1]}`;
    for (let i = 1; i < pts.length - 1; i++) {
      const [x, y] = pts[i], [nx, ny] = pts[i + 1];
      d += `Q${x} ${y} ${(x + nx) / 2} ${(y + ny) / 2}`;
    }
    const [lx, ly] = pts[pts.length - 1];
    return `${d}L${lx} ${ly}`;
  }
  function shape(it) {
    const c = it.color ?? cfg.colors[0];
    const g = s("g", { "data-wb-item": it.id, "data-type": it.type });
    if (it.type === "pen") {
      g.setAttribute("transform", `translate(${it.x ?? 0} ${it.y ?? 0})`);
      g.append(s("path", { d: smooth(it.points ?? []), fill: "none", stroke: c, "stroke-width": it.width ?? 3, "stroke-linecap": "round", "stroke-linejoin": "round" }));
      // A wider, clear stroke to click on.
      g.append(s("path", { d: smooth(it.points ?? []), fill: "none", stroke: "transparent", "stroke-width": 14 }));
    } else if (it.type === "arrow") {
      const [[x1, y1], [x2, y2]] = it.points?.length >= 2 ? it.points : [[0, 0], [0, 0]];
      const a = Math.atan2(y2 - y1, x2 - x1), head = 14;
      const tip = (t) => `${x2 - head * Math.cos(a + t)} ${y2 - head * Math.sin(a + t)}`;
      g.setAttribute("transform", `translate(${it.x ?? 0} ${it.y ?? 0})`);
      g.append(s("path", { d: `M${x1} ${y1}L${x2} ${y2}M${tip(0.45)}L${x2} ${y2}L${tip(-0.45)}`, fill: "none", stroke: c, "stroke-width": 3, "stroke-linecap": "round", "stroke-linejoin": "round" }));
      g.append(s("path", { d: `M${x1} ${y1}L${x2} ${y2}`, stroke: "transparent", "stroke-width": 14 }));
    } else if (it.type === "rect") {
      g.append(s("rect", { x: it.x, y: it.y, width: Math.max(1, it.w), height: Math.max(1, it.h), rx: 6, fill: it.fill ?? "transparent", stroke: c, "stroke-width": 3 }));
    } else if (it.type === "ellipse") {
      g.append(s("ellipse", { cx: it.x + it.w / 2, cy: it.y + it.h / 2, rx: Math.max(1, it.w / 2), ry: Math.max(1, it.h / 2), fill: it.fill ?? "transparent", stroke: c, "stroke-width": 3 }));
    } else if (it.type === "note" || it.type === "text") {
      if (it.type === "note") g.append(s("rect", { x: it.x, y: it.y, width: it.w, height: it.h, rx: 4, fill: it.color ?? NOTE_COLORS[0], filter: `url(#${cfg.id}-shadow)` }));
      const fo = s("foreignObject", { x: it.x, y: it.y, width: Math.max(1, it.w), height: Math.max(1, it.h), [it.type === "note" ? "data-wb-note" : "data-wb-text"]: true });
      const div = document.createElement("div");
      div.textContent = it.text ?? "";
      if (it.type === "text") div.style.setProperty("color", c);
      fo.append(div);
      g.append(fo);
    }
    return g;
  }
  function paint() {
    world.setAttribute("transform", `scale(${view.zoom}) translate(${-view.x} ${-view.y})`);
    overlay.setAttribute("transform", `scale(${view.zoom}) translate(${-view.x} ${-view.y})`);
    // The dots of the grid move with the board.
    const g = 24 * view.zoom;
    stage.style.setProperty("--wb-grid", `${g}px`);
    stage.style.setProperty("--wb-gx", `${-view.x * view.zoom}px`);
    stage.style.setProperty("--wb-gy", `${-view.y * view.zoom}px`);
    const z = bar.querySelector("[data-wb-zoom]");
    if (z) z.textContent = `${Math.round(view.zoom * 100)}%`;
    drawSelection();
    if (editing) placeEditor();
  }
  function draw() {
    const list = items();
    const ids = new Set(list.map((i) => String(i.id)));
    selected = new Set([...selected].filter((id) => ids.has(id)));
    const defs = s("defs", {}, s("filter", { id: `${cfg.id}-shadow`, x: "-10%", y: "-10%", width: "130%", height: "140%" }, s("feDropShadow", { dx: 0, dy: 2, stdDeviation: 2, "flood-opacity": 0.18 })));
    world.replaceChildren(defs, ...list.map(shape));
    empty.hidden = list.length > 0;
    drawBar();
    paint();
  }
  // The selection: a dashed box round each item, and a corner to resize one.
  function drawSelection() {
    overlay.replaceChildren();
    const k = 1 / view.zoom;
    for (const it of items().filter((i) => selected.has(String(i.id)))) {
      const b = box(it);
      overlay.append(s("rect", { x: b.x - 4 * k, y: b.y - 4 * k, width: b.w + 8 * k, height: b.h + 8 * k, fill: "none", stroke: "var(--color-primary)", "stroke-width": 1.5 * k, "stroke-dasharray": `${4 * k} ${3 * k}`, "pointer-events": "none" }));
      if (selected.size === 1 && !cfg.readOnly && ["note", "rect", "ellipse", "text"].includes(it.type)) {
        overlay.append(s("rect", { "data-wb-handle": it.id, x: b.x + b.w - 2 * k, y: b.y + b.h - 2 * k, width: 10 * k, height: 10 * k, rx: 2 * k, fill: "var(--ui-bg, #fff)", stroke: "var(--color-primary)", "stroke-width": 1.5 * k, style: "cursor: nwse-resize" }));
      }
    }
    if (marquee) overlay.append(s("rect", { x: Math.min(marquee.x0, marquee.x1), y: Math.min(marquee.y0, marquee.y1), width: Math.abs(marquee.x1 - marquee.x0), height: Math.abs(marquee.y1 - marquee.y0), fill: "color-mix(in srgb, var(--color-primary) 10%, transparent)", stroke: "var(--color-primary)", "stroke-width": k }));
    if (sketch) overlay.append(sketch);
  }

  // ---- the toolbar
  function drawBar() {
    const parts = [];
    for (const t of usable()) {
      const [name, key] = TOOLS[t];
      parts.push(h("button", { type: "button", "data-wb-tool": t, "aria-pressed": tool === t ? "true" : "false", "aria-label": `${L(name)} (${key})`, title: `${L(name)} (${key})`, onclick: () => pickTool(t) }, icon(t)));
    }
    if (!cfg.readOnly) {
      parts.push(h("span", { "data-wb-sep": true }));
      for (const c of cfg.colors) {
        const sw = h("span", { "data-wb-swatch": true });
        sw.style.setProperty("background", c);
        parts.push(h("button", { type: "button", "data-wb-color": c, "aria-pressed": color === c ? "true" : "false", "aria-label": L("Colour {c}", { c }), title: c, onclick: () => pickColor(c) }, sw));
      }
      parts.push(h("span", { "data-wb-sep": true }));
      parts.push(h("button", { type: "button", "aria-label": L("Undo"), title: L("Undo (⌘Z)"), disabled: past.length ? null : true, onclick: undo }, icon("undo")));
      parts.push(h("button", { type: "button", "aria-label": L("Redo"), title: L("Redo (⇧⌘Z)"), disabled: future.length ? null : true, onclick: redo }, icon("redo")));
      parts.push(h("button", { type: "button", "data-wb-delete": true, "aria-label": L("Delete"), title: L("Delete"), disabled: selected.size ? null : true, onclick: removeSelected }, icon("trash")));
    }
    parts.push(h("span", { "data-wb-sep": true }));
    parts.push(h("button", { type: "button", "aria-label": L("Zoom out"), title: L("Zoom out"), onclick: () => zoomBy(1 / 1.25) }, icon("minus")));
    parts.push(h("span", { "data-wb-zoom": true, "aria-live": "off" }, `${Math.round(view.zoom * 100)}%`));
    parts.push(h("button", { type: "button", "aria-label": L("Zoom in"), title: L("Zoom in"), onclick: () => zoomBy(1.25) }, icon("plus")));
    parts.push(h("button", { type: "button", "aria-label": L("Fit everything"), title: L("Fit everything"), onclick: fit }, icon("fit")));
    parts.push(h("button", { type: "button", "aria-label": L("Export as SVG"), title: L("Export as SVG"), onclick: exportSvg }, icon("download")));
    const had = bar.contains(document.activeElement) ? document.activeElement.getAttribute("aria-label") : null;
    bar.replaceChildren(...parts);
    if (had) [...bar.querySelectorAll("button")].find((b) => b.getAttribute("aria-label") === had)?.focus();
  }
  // Read only, only looking: select and hand.
  const usable = () => (cfg.readOnly ? cfg.tools.filter((t) => t === "select" || t === "hand") : cfg.tools);
  function pickTool(t) {
    if (!usable().includes(t)) return;
    tool = t;
    el.setAttribute("data-tool", t);
    drawBar();
    say(L(TOOLS[t][0]));
  }
  function pickColor(c) {
    color = c;
    // A colour picked with items selected colours them.
    if (selected.size) commit(items().map((i) => (selected.has(String(i.id)) ? { ...i, color: i.type === "note" ? i.color : c } : i)));
    drawBar();
  }

  // ---- the view
  function zoomBy(f, sx, sy) {
    const r = stage.getBoundingClientRect();
    const cx = sx ?? r.width / 2, cy = sy ?? r.height / 2;
    const before = toBoard(cx, cy), zoom = clamp(view.zoom * f, 0.1, 8);
    setView({ zoom, x: before.x - cx / zoom, y: before.y - cy / zoom });
  }
  function fit() {
    const list = items();
    const r = stage.getBoundingClientRect();
    if (!list.length) return setView({ x: 0, y: 0, zoom: 1 });
    const bs = list.map(box);
    const x0 = Math.min(...bs.map((b) => b.x)), y0 = Math.min(...bs.map((b) => b.y));
    const x1 = Math.max(...bs.map((b) => b.x + b.w)), y1 = Math.max(...bs.map((b) => b.y + b.h));
    const pad = 40, zoom = clamp(Math.min((r.width - 2 * pad) / Math.max(1, x1 - x0), (r.height - 2 * pad) / Math.max(1, y1 - y0)), 0.1, 2);
    setView({ zoom, x: x0 - (r.width / zoom - (x1 - x0)) / 2, y: y0 - (r.height / zoom - (y1 - y0)) / 2 });
  }
  stage.addEventListener("wheel", (e) => {
    e.preventDefault();
    const { sx, sy } = local(e);
    if (e.ctrlKey || e.metaKey) zoomBy(Math.exp(-e.deltaY / 300), sx, sy);
    else setView({ ...view, x: view.x + e.deltaX / view.zoom, y: view.y + e.deltaY / view.zoom });
  }, { passive: false });

  // ---- the pointer: one at a time does the tool's work; two pan and zoom
  const pointers = new Map();
  let action = null, marquee = null, sketch = null, space = false, pinch = null;
  const hitItem = (target) => target.closest?.("[data-wb-item]")?.getAttribute("data-wb-item") ?? null;
  stage.addEventListener("pointerdown", (e) => {
    if (editing) finishEdit();
    stage.focus({ preventScroll: true });
    pointers.set(e.pointerId, local(e));
    try {
      stage.setPointerCapture(e.pointerId);
    } catch {}
    if (pointers.size === 2) {
      // A second finger: whatever the first began gives way to moving the board.
      action = null;
      sketch = null;
      const [a, b] = [...pointers.values()];
      pinch = { d: Math.hypot(a.sx - b.sx, a.sy - b.sy), mid: { sx: (a.sx + b.sx) / 2, sy: (a.sy + b.sy) / 2 }, view: { ...view } };
      return paint();
    }
    const { sx, sy } = local(e), p = toBoard(sx, sy);
    if (e.button === 1 || tool === "hand" || space) {
      stage.setAttribute("data-panning", "on");
      action = { kind: "pan", sx, sy, view: { ...view } };
      return;
    }
    if (e.button !== 0) return;
    if (cfg.readOnly) return;
    const handle = e.target.closest?.("[data-wb-handle]")?.getAttribute("data-wb-handle");
    if (handle) {
      const it = items().find((i) => String(i.id) === handle);
      action = { kind: "resize", id: handle, start: p, box: box(it), before: items() };
      return;
    }
    if (tool === "select") {
      const id = hitItem(e.target);
      if (id) {
        if (e.shiftKey) selected.has(id) ? selected.delete(id) : selected.add(id);
        else if (!selected.has(id)) selected = new Set([id]);
        action = { kind: "move", start: p, moved: false, before: items() };
        announceSelection();
        drawBar();
        drawSelection();
      } else {
        if (!e.shiftKey) selected = new Set();
        marquee = { x0: p.x, y0: p.y, x1: p.x, y1: p.y, add: e.shiftKey ? new Set(selected) : new Set() };
        action = { kind: "marquee" };
        drawBar();
        drawSelection();
      }
      return;
    }
    if (tool === "pen") {
      action = { kind: "pen", points: [[p.x, p.y]] };
      sketch = s("path", { fill: "none", stroke: color, "stroke-width": 3, "stroke-linecap": "round", "stroke-linejoin": "round" });
      return drawSelection();
    }
    action = { kind: "shape", type: tool, start: p, end: p };
    sketch = s("g");
    drawSelection();
  });
  stage.addEventListener("pointermove", (e) => {
    if (!pointers.has(e.pointerId)) return;
    pointers.set(e.pointerId, local(e));
    if (pinch && pointers.size >= 2) {
      const [a, b] = [...pointers.values()];
      const d = Math.hypot(a.sx - b.sx, a.sy - b.sy), mid = { sx: (a.sx + b.sx) / 2, sy: (a.sy + b.sy) / 2 };
      const zoom = clamp(pinch.view.zoom * (d / Math.max(1, pinch.d)), 0.1, 8);
      const at = { x: pinch.mid.sx / pinch.view.zoom + pinch.view.x, y: pinch.mid.sy / pinch.view.zoom + pinch.view.y };
      return setView({ zoom, x: at.x - mid.sx / zoom, y: at.y - mid.sy / zoom });
    }
    if (!action) return;
    const { sx, sy } = local(e), p = toBoard(sx, sy);
    if (action.kind === "pan") return setView({ ...action.view, x: action.view.x - (sx - action.sx) / view.zoom, y: action.view.y - (sy - action.sy) / view.zoom });
    if (action.kind === "move") {
      const dx = p.x - action.start.x, dy = p.y - action.start.y;
      if (!action.moved && Math.hypot(dx, dy) * view.zoom < 3) return;
      action.moved = true;
      api.set(cfg.bind, action.before.map((i) => (selected.has(String(i.id)) ? { ...i, x: (i.x ?? 0) + dx, y: (i.y ?? 0) + dy } : i)));
      return draw();
    }
    if (action.kind === "resize") {
      const w = Math.max(20, action.box.w + (p.x - action.start.x)), hh = Math.max(20, action.box.h + (p.y - action.start.y));
      api.set(cfg.bind, action.before.map((i) => (String(i.id) === action.id ? { ...i, w, h: hh } : i)));
      return draw();
    }
    if (action.kind === "marquee") {
      marquee.x1 = p.x;
      marquee.y1 = p.y;
      const [x0, x1] = [Math.min(marquee.x0, p.x), Math.max(marquee.x0, p.x)], [y0, y1] = [Math.min(marquee.y0, p.y), Math.max(marquee.y0, p.y)];
      selected = new Set([...marquee.add, ...items().filter((i) => {
        const b = box(i);
        return b.x >= x0 && b.y >= y0 && b.x + b.w <= x1 && b.y + b.h <= y1;
      }).map((i) => String(i.id))]);
      return drawSelection();
    }
    if (action.kind === "pen") {
      const last = action.points[action.points.length - 1];
      if (Math.hypot(p.x - last[0], p.y - last[1]) * view.zoom >= 2) action.points.push([p.x, p.y]);
      sketch.setAttribute("d", smooth(action.points));
      return drawSelection();
    }
    if (action.kind === "shape") {
      action.end = p;
      const it = build(action, true);
      sketch = it ? shape(it) : null;
      if (sketch) sketch.setAttribute("opacity", "0.7");
      drawSelection();
    }
  });
  // An item from a drag (or a click: the type's usual size), or null.
  function build(a, preview) {
    const { start: p0, end: p1, type } = a;
    const dragged = Math.hypot(p1.x - p0.x, p1.y - p0.y) * view.zoom > 6;
    const id = preview ? "sketch" : newId();
    if (type === "arrow") {
      if (!dragged) return preview ? null : { id, type, x: p0.x, y: p0.y, points: [[0, 0], [120, 0]], color };
      return { id, type, x: p0.x, y: p0.y, points: [[0, 0], [p1.x - p0.x, p1.y - p0.y]], color };
    }
    const size = { note: [180, 140], rect: [140, 90], ellipse: [120, 120], text: [220, 40] }[type];
    const x = dragged ? Math.min(p0.x, p1.x) : p0.x, y = dragged ? Math.min(p0.y, p1.y) : p0.y;
    const w = dragged ? Math.max(20, Math.abs(p1.x - p0.x)) : size[0], hh = dragged ? Math.max(20, Math.abs(p1.y - p0.y)) : size[1];
    if (!dragged && preview) return null;
    const it = { id, type, x, y, w, h: hh };
    if (type === "note") it.color = NOTE_COLORS[items().filter((i) => i.type === "note").length % NOTE_COLORS.length];
    else it.color = color;
    if (type === "note" || type === "text") it.text = "";
    return it;
  }
  function end(e) {
    pointers.delete(e.pointerId);
    if (pinch) {
      if (pointers.size < 2) pinch = null;
      return;
    }
    stage.toggleAttribute("data-panning", space);
    const a = action;
    action = null;
    if (!a) return;
    if (a.kind === "move" && a.moved) {
      past.push(a.before);
      future = [];
      api.emit("change", { value: items() });
      say(L(["{n} item moved", "{n} items moved"], { n: selected.size }));
      drawBar();
    } else if (a.kind === "resize") {
      past.push(a.before);
      future = [];
      api.emit("change", { value: items() });
      drawBar();
    } else if (a.kind === "marquee") {
      marquee = null;
      drawSelection();
      drawBar();
      announceSelection();
    } else if (a.kind === "pen") {
      sketch = null;
      if (a.points.length) {
        const x = Math.min(...a.points.map((p) => p[0])), y = Math.min(...a.points.map((p) => p[1]));
        const it = { id: newId(), type: "pen", x, y, points: a.points.map(([px, py]) => [Math.round((px - x) * 10) / 10, Math.round((py - y) * 10) / 10]), color };
        commit([...items(), it], L("Drawing added"));
      } else drawSelection();
    } else if (a.kind === "shape") {
      sketch = null;
      const it = build(a, false);
      if (!it) return drawSelection();
      commit([...items(), it], L("{kind} added", { kind: L(KIND[it.type]) }));
      selected = new Set([String(it.id)]);
      // Notes and text are written in straight away; the tool goes back to Select.
      pickTool("select");
      drawSelection();
      drawBar();
      if (it.type === "note" || it.type === "text") edit(String(it.id));
    }
  }
  stage.addEventListener("pointerup", end);
  stage.addEventListener("pointercancel", end);
  stage.addEventListener("dblclick", (e) => {
    const id = hitItem(e.target);
    const it = id && items().find((i) => String(i.id) === id);
    if (it && (it.type === "note" || it.type === "text") && !cfg.readOnly) edit(id);
  });

  // ---- writing in a note or text: a textarea over it, the board's size
  let editing = null, editor = null;
  function placeEditor() {
    const it = items().find((i) => String(i.id) === editing);
    if (!it || !editor) return;
    const z = view.zoom;
    Object.assign(editor.style, {
      left: `${(it.x - view.x) * z}px`, top: `${(it.y - view.y) * z}px`, width: `${it.w}px`, height: `${it.h}px`, transform: `scale(${z})`,
      padding: it.type === "note" ? "8px 10px" : "0", font: it.type === "note" ? "15px/1.35 var(--font-body, system-ui, sans-serif)" : "18px/1.3 var(--font-body, system-ui, sans-serif)",
      background: it.type === "note" ? (it.color ?? NOTE_COLORS[0]) : "transparent", color: it.type === "note" ? "#1f1f1f" : (it.color ?? "inherit"),
    });
  }
  function edit(id) {
    finishEdit();
    editing = id;
    const it = items().find((i) => String(i.id) === id);
    editor = h("textarea", { "data-wb-edit": true, "aria-label": L("{kind} text", { kind: L(KIND[it.type]) }), spellcheck: "true" });
    editor.value = it.text ?? "";
    stage.append(editor);
    placeEditor();
    editor.focus();
    editor.select();
    editor.addEventListener("keydown", (e) => {
      e.stopPropagation();
      if (e.key === "Escape" || (e.key === "Enter" && (e.metaKey || e.ctrlKey))) {
        e.preventDefault();
        finishEdit();
        stage.focus();
      }
    });
    editor.addEventListener("blur", () => finishEdit());
  }
  function finishEdit() {
    if (!editing || !editor) return;
    const id = editing, text = editor.value, it = items().find((i) => String(i.id) === id);
    editing = null;
    const ed = editor;
    editor = null;
    ed.remove();
    if (it && text !== (it.text ?? "")) commit(items().map((i) => (String(i.id) === id ? { ...i, text } : i)), L("Text changed"));
  }

  // ---- the keys
  function announceSelection() {
    const list = items(), picked = list.filter((i) => selected.has(String(i.id)));
    if (picked.length === 1) say(L("{name}, {n} of {total}", { name: named(picked[0]), n: list.indexOf(picked[0]) + 1, total: list.length }));
    else if (picked.length) say(L("{n} selected", { n: picked.length }));
  }
  function removeSelected() {
    if (!selected.size || cfg.readOnly) return;
    const n = selected.size;
    const next = items().filter((i) => !selected.has(String(i.id)));
    selected = new Set();
    commit(next, L(["{n} item deleted", "{n} items deleted"], { n }));
  }
  function undo() {
    if (!past.length) return;
    future.push(items());
    api.set(cfg.bind, past.pop());
    say(L("Undone"));
    draw();
  }
  function redo() {
    if (!future.length) return;
    past.push(items());
    api.set(cfg.bind, future.pop());
    say(L("Redone"));
    draw();
  }
  stage.addEventListener("keydown", (e) => {
    if (e.target !== stage) return;
    const mod = e.metaKey || e.ctrlKey, k = e.key.toLowerCase();
    if (e.key === " ") {
      e.preventDefault();
      space = true;
      stage.setAttribute("data-panning", "");
      return;
    }
    if (mod && k === "z") {
      e.preventDefault();
      return e.shiftKey ? redo() : undo();
    }
    if (mod && k === "y") {
      e.preventDefault();
      return redo();
    }
    if (mod && k === "a") {
      e.preventDefault();
      selected = new Set(items().map((i) => String(i.id)));
      drawSelection();
      drawBar();
      return announceSelection();
    }
    if (mod && k === "d" && selected.size && !cfg.readOnly) {
      e.preventDefault();
      const copies = items().filter((i) => selected.has(String(i.id))).map((i) => ({ ...structuredClone(i), id: newId(), x: (i.x ?? 0) + 20, y: (i.y ?? 0) + 20 }));
      selected = new Set(copies.map((c) => c.id));
      return commit([...items(), ...copies], L(["{n} item duplicated", "{n} items duplicated"], { n: copies.length }));
    }
    if ((e.key === "Delete" || e.key === "Backspace") && selected.size) {
      e.preventDefault();
      return removeSelected();
    }
    if (e.key === "Escape") {
      if (selected.size) {
        e.preventDefault();
        selected = new Set();
        drawSelection();
        drawBar();
      }
      return;
    }
    if (e.key === "Enter" && selected.size === 1) {
      const id = [...selected][0], it = items().find((i) => String(i.id) === id);
      if (it && (it.type === "note" || it.type === "text") && !cfg.readOnly) {
        e.preventDefault();
        return edit(id);
      }
    }
    // Tab goes along the items while one's selected (Escape first lets Tab leave).
    if (e.key === "Tab" && selected.size) {
      const list = items();
      if (!list.length) return;
      e.preventDefault();
      const at = list.findIndex((i) => selected.has(String(i.id)));
      const next = list[(at + (e.shiftKey ? -1 : 1) + list.length) % list.length];
      selected = new Set([String(next.id)]);
      drawSelection();
      drawBar();
      return announceSelection();
    }
    const arrows = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] }[e.key];
    if (arrows) {
      e.preventDefault();
      const n = e.shiftKey ? 10 : 1;
      if (selected.size && !cfg.readOnly) commit(items().map((i) => (selected.has(String(i.id)) ? { ...i, x: (i.x ?? 0) + arrows[0] * n, y: (i.y ?? 0) + arrows[1] * n } : i)));
      else setView({ ...view, x: view.x + (arrows[0] * n * 20) / view.zoom, y: view.y + (arrows[1] * n * 20) / view.zoom });
      return;
    }
    if (mod || e.altKey) return;
    if (k === "+" || k === "=") return zoomBy(1.25);
    if (k === "-") return zoomBy(1 / 1.25);
    if (k === "0") return fit();
    const t = Object.entries(TOOLS).find(([, [, key]]) => key.toLowerCase() === k)?.[0];
    if (t && usable().includes(t)) {
      e.preventDefault();
      pickTool(t);
      // N and T at the keyboard add the item in the middle of the view, to write in.
      if ((t === "note" || t === "text") && !cfg.readOnly) {
        const r = stage.getBoundingClientRect(), c = toBoard(r.width / 2 - 90, r.height / 2 - 70);
        const it = build({ type: t, start: c, end: c }, false);
        commit([...items(), it], L("{kind} added", { kind: L(KIND[t]) }));
        selected = new Set([String(it.id)]);
        pickTool("select");
        drawSelection();
        edit(String(it.id));
      }
    }
  });
  stage.addEventListener("keyup", (e) => {
    if (e.key === " ") {
      space = false;
      stage.removeAttribute("data-panning");
    }
  });
  stage.addEventListener("focus", () => {
    if (!items().length) say(L("Empty whiteboard"));
  });

  // ---- export: the board as an SVG file
  function exportSvg() {
    const list = items();
    if (!list.length) return say(L("Nothing to export"));
    const bs = list.map(box), pad = 20;
    const x0 = Math.min(...bs.map((b) => b.x)) - pad, y0 = Math.min(...bs.map((b) => b.y)) - pad;
    const w = Math.max(...bs.map((b) => b.x + b.w)) - x0 + pad, hh = Math.max(...bs.map((b) => b.y + b.h)) - y0 + pad;
    const out = s("svg", { xmlns: NS, viewBox: `${x0} ${y0} ${w} ${hh}`, width: Math.round(w), height: Math.round(hh) });
    out.append(s("rect", { x: x0, y: y0, width: w, height: hh, fill: "#ffffff" }));
    // Notes and text as SVG text (an image viewer may not draw HTML inside SVG).
    for (const it of list) {
      if (it.type === "note" || it.type === "text") {
        const g = s("g");
        if (it.type === "note") g.append(s("rect", { x: it.x, y: it.y, width: it.w, height: it.h, rx: 4, fill: it.color ?? NOTE_COLORS[0] }));
        const size = it.type === "note" ? 15 : 18;
        String(it.text ?? "").split("\n").forEach((line, i) => {
          const t = s("text", { x: it.x + (it.type === "note" ? 12 : 0), y: it.y + (it.type === "note" ? 10 : 0) + size * 1.3 * (i + 1) - 4, "font-size": size, "font-family": "system-ui, sans-serif", fill: it.type === "note" ? "#1f1f1f" : (it.color ?? "#1f2937") });
          t.textContent = line;
          g.append(t);
        });
        out.append(g);
      } else out.append(shape(it));
    }
    const blob = new Blob([new XMLSerializer().serializeToString(out)], { type: "image/svg+xml" });
    const a = h("a", { href: URL.createObjectURL(blob), download: "whiteboard.svg" });
    document.body.append(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(a.href), 1000);
    say(L("Exported"));
  }

  el.setAttribute("data-tool", tool);
  draw();
  let seen = JSON.stringify(items());
  return {
    update() {
      // Items or the view changed elsewhere (another person's, from a server).
      const now = JSON.stringify(items());
      if (cfg.bindView && isObj(api.state[cfg.bindView])) {
        const v = api.state[cfg.bindView];
        if (v.x !== view.x || v.y !== view.y || v.zoom !== view.zoom) setView({ x: 0, y: 0, zoom: 1, ...v }, false);
      }
      if (now === seen || action || editing) return;
      seen = now;
      draw();
    },
  };
};
