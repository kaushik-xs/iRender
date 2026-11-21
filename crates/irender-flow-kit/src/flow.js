// ui-flow: pan and zoom, layout, edge routing, selection and dragging for the
// nodes and edges irender-flow-kit renders. No dependencies.
(() => {
  if ((window.__irenderBehaviors ||= {})["ui-flow"]) return;
  const NS = "http://www.w3.org/2000/svg";
  const isObj = (v) => v !== null && typeof v === "object" && !Array.isArray(v);
  const num = (v, d) => (typeof v === "number" && Number.isFinite(v) ? v : typeof v === "string" && v.trim() !== "" && Number.isFinite(+v) ? +v : d);
  const str = (v) => (v === null || v === undefined ? "" : typeof v === "object" ? JSON.stringify(v) : String(v));
  const truthy = (v) => !(v === null || v === undefined || v === false || v === 0 || v === "" || Number.isNaN(v));
  const clamp = (v, lo, hi) => Math.min(hi, Math.max(lo, v));
  // Must agree with EDGE_ID in lib.rs.
  const edgeId = (e) => (truthy(e.id) ? str(e.id) : str(e.source) + "-" + str(e.target));
  const reducedMotion = matchMedia("(prefers-reduced-motion: reduce)");
  // What a palette item carries while it is dragged.
  const MIME = "application/x-irender-flow-node";
  let flows = 0;
  // What ⌘C / ⌘X last copied, shared by the page's flows.
  let clipboard = null;

  // Resolves a behaviour's `{ "$irender": template }` values against the state
  // (and the list item it is in).
  const resolver = (api) => {
    const env = () => {
      const s = api.scope();
      return s ? { ...api.state, ...s } : api.state;
    };
    const resolve = (v) => {
      if (Array.isArray(v)) return v.map(resolve);
      if (!isObj(v)) return v;
      if ("$irender" in v && Object.keys(v).length === 1) return window.__irender.evaluate(v.$irender, env());
      return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, resolve(x)]));
    };
    return resolve;
  };

  // ------------------------------------------------------------------ geometry
  const DIRS = { left: [-1, 0], right: [1, 0], top: [0, -1], bottom: [0, 1] };
  const OPPOSITE = { left: "right", right: "left", top: "bottom", bottom: "top" };
  // Where sources and targets go when a port names no side.
  const FLOW_SIDES = { LR: ["right", "left"], RL: ["left", "right"], TB: ["bottom", "top"], BT: ["top", "bottom"] };

  // React Flow's bezier: control points pulled out along each port's side.
  function bezier(s, t) {
    const pull = (a, b, side) => {
      const [dx, dy] = DIRS[side];
      const d = dx ? (b.x - a.x) * dx : (b.y - a.y) * dy;
      const off = d >= 0 ? 0.5 * d : 0.25 * 25 * Math.sqrt(-d);
      return { x: a.x + dx * off, y: a.y + dy * off };
    };
    const c1 = pull(s, t, s.side), c2 = pull(t, s, t.side);
    return {
      d: `M${s.x},${s.y} C${c1.x},${c1.y} ${c2.x},${c2.y} ${t.x},${t.y}`,
      x: (s.x + 3 * c1.x + 3 * c2.x + t.x) / 8,
      y: (s.y + 3 * c1.y + 3 * c2.y + t.y) / 8,
    };
  }

  // Right-angled routes: out of each port by `gap`, then across.
  function orthogonal(s, t, gap) {
    const [sx, sy] = DIRS[s.side], [tx, ty] = DIRS[t.side];
    const s0 = { x: s.x + sx * gap, y: s.y + sy * gap }, t0 = { x: t.x + tx * gap, y: t.y + ty * gap };
    const sh = sx !== 0, th = tx !== 0;
    let pts;
    if (sh && th) {
      const ahead = sx * (t.x - s.x) > 0 && tx * (s.x - t.x) > 0;
      if (ahead) { const mx = (s.x + t.x) / 2; pts = [s, { x: mx, y: s.y }, { x: mx, y: t.y }, t]; }
      else { const my = (s.y + t.y) / 2; pts = [s, s0, { x: s0.x, y: my }, { x: t0.x, y: my }, t0, t]; }
    } else if (!sh && !th) {
      const ahead = sy * (t.y - s.y) > 0 && ty * (s.y - t.y) > 0;
      if (ahead) { const my = (s.y + t.y) / 2; pts = [s, { x: s.x, y: my }, { x: t.x, y: my }, t]; }
      else { const mx = (s.x + t.x) / 2; pts = [s, s0, { x: mx, y: s0.y }, { x: mx, y: t0.y }, t0, t]; }
    } else if (sh) {
      pts = sx * (t.x - s.x) > 0 && ty * (s.y - t.y) > 0 ? [s, { x: t.x, y: s.y }, t] : [s, s0, { x: s0.x, y: t0.y }, t0, t];
    } else {
      pts = sy * (t.y - s.y) > 0 && tx * (s.x - t.x) > 0 ? [s, { x: s.x, y: t.y }, t] : [s, s0, { x: t0.x, y: s0.y }, t0, t];
    }
    // Drop repeated and collinear points.
    const out = [];
    for (const p of pts) {
      const a = out[out.length - 2], b = out[out.length - 1];
      if (b && Math.abs(b.x - p.x) < 0.01 && Math.abs(b.y - p.y) < 0.01) continue;
      if (a && ((Math.abs(a.x - b.x) < 0.01 && Math.abs(b.x - p.x) < 0.01) || (Math.abs(a.y - b.y) < 0.01 && Math.abs(b.y - p.y) < 0.01))) out.pop();
      out.push(p);
    }
    return out;
  }

  // A polyline with corners rounded to `r`, labelled at half its length.
  function polyline(pts, r) {
    let d = `M${pts[0].x},${pts[0].y}`;
    for (let i = 1; i < pts.length - 1; i++) {
      const a = pts[i - 1], b = pts[i], c = pts[i + 1];
      const k = Math.min(r, Math.hypot(b.x - a.x, b.y - a.y) / 2, Math.hypot(c.x - b.x, c.y - b.y) / 2);
      if (k < 0.5) { d += ` L${b.x},${b.y}`; continue; }
      const u = (p, q) => { const l = Math.hypot(q.x - p.x, q.y - p.y) || 1; return { x: (q.x - p.x) / l, y: (q.y - p.y) / l }; };
      const ba = u(b, a), bc = u(b, c);
      d += ` L${b.x + ba.x * k},${b.y + ba.y * k} Q${b.x},${b.y} ${b.x + bc.x * k},${b.y + bc.y * k}`;
    }
    const last = pts[pts.length - 1];
    d += ` L${last.x},${last.y}`;
    let total = 0;
    for (let i = 1; i < pts.length; i++) total += Math.hypot(pts[i].x - pts[i - 1].x, pts[i].y - pts[i - 1].y);
    let half = total / 2, x = pts[0].x, y = pts[0].y;
    for (let i = 1; i < pts.length; i++) {
      const l = Math.hypot(pts[i].x - pts[i - 1].x, pts[i].y - pts[i - 1].y);
      if (half <= l) { const f = l ? half / l : 0; x = pts[i - 1].x + (pts[i].x - pts[i - 1].x) * f; y = pts[i - 1].y + (pts[i].y - pts[i - 1].y) * f; break; }
      half -= l;
    }
    return { d, x, y };
  }

  function selfLoop(g) {
    const x = g.x + g.w, y = g.y, k = 36;
    return { d: `M${x - 24},${y} C${x - 24},${y - k * 1.4} ${x + k * 1.4},${y + 24} ${x},${y + 24}`, x: x + k * 0.55, y: y - k * 0.55 };
  }

  function route(kind, s, t) {
    if (kind === "straight") return { d: `M${s.x},${s.y} L${t.x},${t.y}`, x: (s.x + t.x) / 2, y: (s.y + t.y) / 2 };
    if (kind === "step") return polyline(orthogonal(s, t, 20), 0);
    if (kind === "smoothstep") return polyline(orthogonal(s, t, 20), 8);
    return bezier(s, t);
  }

  // ------------------------------------------------------------------ shapes
  // Outlines as corners in fractions of the box; the rounded ones meet edges as
  // their box does, which is where their curves are anyway at the middles.
  const POLYGONS = {
    rect: [[0, 0], [1, 0], [1, 1], [0, 1]],
    diamond: [[0.5, 0], [1, 0.5], [0.5, 1], [0, 0.5]],
    hexagon: [[0.2, 0], [0.8, 0], [1, 0.5], [0.8, 1], [0.2, 1], [0, 0.5]],
    parallelogram: [[0.15, 0], [1, 0], [0.85, 1], [0, 1]],
    triangle: [[0.5, 0], [1, 1], [0, 1]],
  };
  // `polygon`'s `points`: "x,y x,y …" or [[x, y], …], in percent of the box.
  const parsePoints = (v) => {
    const list = Array.isArray(v) ? v : str(v).trim().split(/\s+/).map((p) => p.split(","));
    const pts = list.map((p) => [num(p?.[0], NaN) / 100, num(p?.[1], NaN) / 100]).filter(([x, y]) => Number.isFinite(x) && Number.isFinite(y));
    return pts.length >= 3 ? pts : null;
  };
  // A node's outline in its own pixels: its SVG path, and its geometry for `hit`.
  function outline(kind, points, w, h) {
    if (kind === "circle" || kind === "ellipse") {
      const rx = kind === "circle" ? Math.min(w, h) / 2 : w / 2, ry = kind === "circle" ? Math.min(w, h) / 2 : h / 2;
      const cx = w / 2, cy = h / 2;
      return { ellipse: true, cx, cy, rx, ry, d: `M${cx - rx},${cy} a${rx},${ry} 0 1,0 ${2 * rx},0 a${rx},${ry} 0 1,0 ${-2 * rx},0 Z` };
    }
    const box = POLYGONS.rect.map(([x, y]) => [x * w, y * h]);
    if (kind === "rounded" || kind === "pill") {
      const k = kind === "pill" ? Math.min(w, h) / 2 : Math.min(10, w / 2, h / 2);
      return { pts: box, d: `M${k},0 H${w - k} A${k},${k} 0 0 1 ${w},${k} V${h - k} A${k},${k} 0 0 1 ${w - k},${h} H${k} A${k},${k} 0 0 1 0,${h - k} V${k} A${k},${k} 0 0 1 ${k},0 Z` };
    }
    if (kind === "cylinder") {
      const ry = Math.min(10, h / 4);
      return { pts: box, d: `M0,${ry} A${w / 2},${ry} 0 0 1 ${w},${ry} V${h - ry} A${w / 2},${ry} 0 0 1 0,${h - ry} Z M0,${ry} A${w / 2},${ry} 0 0 0 ${w},${ry}` };
    }
    const pts = (kind === "polygon" ? parsePoints(points) : POLYGONS[kind]) || POLYGONS.rect;
    const abs = pts.map(([x, y]) => [x * w, y * h]);
    return { pts: abs, d: "M" + abs.map((p) => p.join(",")).join(" L") + " Z" };
  }
  // Where the ray from (ox, oy) along (dx, dy) leaves an outline; null if it doesn't.
  function hit(o, ox, oy, dx, dy) {
    if (o.ellipse) {
      // ((ox + t·dx − cx) / rx)² + ((oy + t·dy − cy) / ry)² = 1, the far root.
      const px = (ox - o.cx) / o.rx, py = (oy - o.cy) / o.ry, qx = dx / o.rx, qy = dy / o.ry;
      const a = qx * qx + qy * qy, b = 2 * (px * qx + py * qy), c = px * px + py * py - 1, disc = b * b - 4 * a * c;
      if (!a || disc < 0) return null;
      const t = (-b + Math.sqrt(disc)) / (2 * a);
      return t >= 0 ? [ox + dx * t, oy + dy * t] : null;
    }
    let best = Infinity;
    for (let i = 0; i < o.pts.length; i++) {
      const [ax, ay] = o.pts[i], [bx, by] = o.pts[(i + 1) % o.pts.length];
      const ex = bx - ax, ey = by - ay, det = ex * dy - dx * ey;
      if (Math.abs(det) < 1e-9) continue;
      const t = (ex * (ay - oy) - ey * (ax - ox)) / det, u = (dx * (ay - oy) - dy * (ax - ox)) / det;
      if (t >= -1e-9 && u >= -1e-9 && u <= 1 + 1e-9) best = Math.min(best, t);
    }
    return Number.isFinite(best) ? [ox + dx * best, oy + dy * best] : null;
  }

  // -------------------------------------------------------------------- layout
  // A layered (Sugiyama-style) layout of the top-level nodes: cycles broken,
  // longest-path ranks, barycentre ordering, then positions that pull each node
  // towards its neighbours without letting nodes in a rank overlap. Components are
  // laid out separately, then packed into rows, largest first. Returns top-left
  // corners by id.
  function layered(ids, links, size, dir, nodeGap, rankGap) {
    const horizontal = dir === "LR" || dir === "RL";
    const along = (id) => (horizontal ? size(id).w : size(id).h); // rank axis
    const across = (id) => (horizontal ? size(id).h : size(id).w);
    const out = new Map(ids.map((id) => [id, []])), inn = new Map(ids.map((id) => [id, []]));
    const seen = new Set();
    for (const [a, b] of links) {
      const k = a + "\u0000" + b;
      if (a === b || seen.has(k) || !out.has(a) || !out.has(b)) continue;
      seen.add(k);
      out.get(a).push(b);
      inn.get(b).push(a);
    }
    // Weakly connected components, in the order their first node appears.
    const comp = new Map();
    const comps = [];
    for (const id of ids) {
      if (comp.has(id)) continue;
      const list = [], stack = [id];
      comp.set(id, comps.length);
      while (stack.length) {
        const n = stack.pop();
        list.push(n);
        for (const m of [...out.get(n), ...inn.get(n)]) if (!comp.has(m)) { comp.set(m, comps.length); stack.push(m); }
      }
      comps.push(list.sort((a, b) => ids.indexOf(a) - ids.indexOf(b)));
    }
    const pos = new Map();
    const boxes = []; // each component's nodes and size, to pack afterwards
    for (const list of comps) {
      // Break cycles: edges back to a node still on the DFS stack are reversed.
      const state = new Map(), succ = new Map(list.map((id) => [id, []]));
      const visit = (n) => {
        state.set(n, 1);
        for (const m of out.get(n)) {
          if (state.get(m) === 1) succ.get(m).push(n);
          else { succ.get(n).push(m); if (!state.has(m)) visit(m); }
        }
        state.set(n, 2);
      };
      for (const id of list) if (!inn.get(id).length && !state.has(id)) visit(id);
      for (const id of list) if (!state.has(id)) visit(id);
      const pred = new Map(list.map((id) => [id, []]));
      for (const [a, bs] of succ) for (const b of bs) pred.get(b).push(a);
      // Longest-path ranks, in topological order.
      const rank = new Map(), indeg = new Map(list.map((id) => [id, pred.get(id).length]));
      const queue = list.filter((id) => !indeg.get(id));
      for (const id of queue) rank.set(id, 0);
      for (let i = 0; i < queue.length; i++) {
        const n = queue[i];
        for (const m of succ.get(n)) {
          rank.set(m, Math.max(rank.get(m) ?? 0, rank.get(n) + 1));
          indeg.set(m, indeg.get(m) - 1);
          if (!indeg.get(m)) queue.push(m);
        }
      }
      // Pull sources towards their first successor, so they don't all sit in rank 0.
      for (const id of list) {
        if (pred.get(id).length || !succ.get(id).length) continue;
        rank.set(id, Math.min(...succ.get(id).map((m) => rank.get(m))) - 1);
      }
      const ranks = [];
      for (const id of list) (ranks[rank.get(id)] ||= []).push(id);
      const layers = ranks.filter(Boolean);
      // Order within ranks: barycentres of the neighbours in the rank before (down
      // sweeps) or after (up sweeps).
      const order = new Map();
      const renumber = () => layers.forEach((l) => l.forEach((id, i) => order.set(id, i)));
      renumber();
      const bary = (id, nbrs, fallback) => {
        const ns = nbrs.get(id);
        return ns.length ? ns.reduce((s, m) => s + order.get(m), 0) / ns.length : fallback;
      };
      for (let it = 0; it < 8; it++) {
        const down = it % 2 === 0;
        const seq = down ? layers.slice(1) : layers.slice(0, -1).reverse();
        for (const l of seq) {
          const b = new Map(l.map((id) => [id, bary(id, down ? pred : succ, order.get(id))]));
          l.sort((x, y) => b.get(x) - b.get(y) || order.get(x) - order.get(y));
          l.forEach((id, i) => order.set(id, i));
        }
      }
      // Rank axis: each rank as deep as its largest node.
      let at = 0;
      const rankAt = new Map();
      for (const l of layers) {
        const depth = Math.max(...l.map(along));
        for (const id of l) rankAt.set(id, at + (depth - along(id)) / 2);
        at += depth + rankGap;
      }
      // Cross axis: pack each rank, then pull nodes towards their neighbours' centres.
      const centre = new Map();
      for (const l of layers) {
        let c = 0;
        for (const id of l) { c += across(id) / 2; centre.set(id, c); c += across(id) / 2 + nodeGap; }
        const shift = (c - nodeGap) / 2;
        for (const id of l) centre.set(id, centre.get(id) - shift);
      }
      for (let it = 0; it < 6; it++) {
        const down = it % 2 === 0;
        for (const l of down ? layers : [...layers].reverse()) {
          const want = l.map((id) => {
            const ns = [...(down ? pred : succ).get(id), ...(it > 1 ? (down ? succ : pred).get(id) : [])];
            return ns.length ? ns.reduce((s, m) => s + centre.get(m), 0) / ns.length : centre.get(id);
          });
          place(l, want, across, nodeGap).forEach((c, i) => centre.set(l[i], c));
        }
      }
      let lo = Infinity, hi = -Infinity;
      for (const id of list) { lo = Math.min(lo, centre.get(id) - across(id) / 2); hi = Math.max(hi, centre.get(id) + across(id) / 2); }
      const own = new Map();
      let depth = 0;
      for (const id of list) {
        const a = rankAt.get(id), c = centre.get(id) - across(id) / 2 - lo;
        own.set(id, horizontal ? { x: a, y: c } : { x: c, y: a });
        depth = Math.max(depth, a + along(id));
      }
      boxes.push({ own, w: horizontal ? depth : hi - lo, h: horizontal ? hi - lo : depth, n: list.length });
    }
    // Pack the components in rows, largest first, wrapping at a width that keeps the
    // whole roughly 16:10, so many small, unconnected groups don't make one long strip.
    boxes.sort((a, b) => b.n - a.n || b.w * b.h - a.w * a.h);
    const gap = nodeGap * 2;
    const area = boxes.reduce((s, b) => s + (b.w + gap) * (b.h + gap), 0);
    const width = Math.max(Math.sqrt(area * 1.6), ...boxes.map((b) => b.w));
    let x = 0, y = 0, row = 0;
    for (const b of boxes) {
      if (x > 0 && x + b.w > width) { x = 0; y += row + gap; row = 0; }
      for (const [id, p] of b.own) pos.set(id, { x: p.x + x, y: p.y + y });
      x += b.w + gap;
      row = Math.max(row, b.h);
    }
    // RL and BT are LR and TB mirrored along the rank axis.
    if (dir === "RL" || dir === "BT") {
      let max = 0;
      for (const [id, p] of pos) max = Math.max(max, horizontal ? p.x + size(id).w : p.y + size(id).h);
      for (const [id, p] of pos) pos.set(id, horizontal ? { x: max - p.x - size(id).w, y: p.y } : { x: p.x, y: max - p.y - size(id).h });
    }
    return pos;
  }

  // Centres as close to `want` as possible (least squares) keeping the order and
  // the gaps: isotonic regression by pooling adjacent violators.
  function place(l, want, across, gap) {
    const off = [0];
    for (let i = 1; i < l.length; i++) off[i] = off[i - 1] + (across(l[i - 1]) + across(l[i])) / 2 + gap;
    const blocks = [];
    for (let i = 0; i < l.length; i++) {
      blocks.push({ sum: want[i] - off[i], n: 1 });
      while (blocks.length > 1) {
        const b = blocks[blocks.length - 1], a = blocks[blocks.length - 2];
        if (a.sum / a.n <= b.sum / b.n) break;
        a.sum += b.sum; a.n += b.n;
        blocks.pop();
      }
    }
    const out = [];
    let i = 0;
    for (const b of blocks) for (let k = 0; k < b.n; k++, i++) out.push(b.sum / b.n + off[i]);
    return out;
  }

  // --------------------------------------------------------------------- flow
  window.__irenderBehaviors["ui-flow"] = (el, api) => {
    const cfg = api.config;
    const resolve = resolver(api);
    let props = cfg.reactive ? resolve(cfg.props) : cfg.props;
    const pane = el.querySelector(":scope > [data-flow-pane]");
    const nodeLayer = pane.querySelector(":scope > [data-flow-nodes]");
    const edgeLayer = pane.querySelector(":scope > [data-flow-edges]");
    const minimap = el.querySelector(":scope > [data-flow-minimap]");
    const uid = "ui-flow-" + ++flows;

    // Arrowheads, once per flow, in the colour of the edge that uses them.
    const defs = document.createElementNS(NS, "svg");
    defs.setAttribute("aria-hidden", "true");
    defs.style.cssText = "position:absolute;width:0;height:0;overflow:hidden";
    defs.innerHTML =
      `<defs><marker id="${uid}-arrow" viewBox="-10 -10 20 20" markerWidth="12.5" markerHeight="12.5" orient="auto-start-reverse" refX="0" refY="0">` +
      `<polyline points="-5,-4 0,0 -5,4" fill="none" stroke="context-stroke" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></marker>` +
      `<marker id="${uid}-arrowclosed" viewBox="-10 -10 20 20" markerWidth="12.5" markerHeight="12.5" orient="auto-start-reverse" refX="0" refY="0">` +
      `<polyline points="-5,-4 0,0 -5,4 -5,-4" fill="context-stroke" stroke="context-stroke" stroke-width="1" stroke-linejoin="round"/></marker></defs>`;
    pane.prepend(defs);

    let view = { x: 0, y: 0, zoom: 1 };
    let selected = cfg.bind ? api.state[cfg.bind] ?? null : null;
    let picked = { nodes: new Set(selected === null ? [] : [str(selected)]), edges: new Set() };
    let pending = null; // a deletion waiting for `confirmDelete`
    let writing = false; // while the flow writes its selection to state
    let fitted = false, centred;
    const dragged = new Map(); // id → position set by dragging
    const sized = new Map(); // id → { w, h } set by resizing
    let history = [], future = [], batching = 0; // undo and redo
    let found = null, query = "", active = 0; // `search`'s matches
    let neighbours = new Set(); // nodes shown only as context for `filter`
    let laidOut = new Map(), layoutKey = "";
    const geo = new Map(); // id → { x, y, w, h }, absolute, in flow coordinates
    const ports = new Map(); // id → [{ id, type, side, x, y }], relative to the node
    const shapes = new Map(); // id → its outline, for nodes with a `shape`
    const dirty = new Set(); // node elements to measure again
    let nodes = [], byId = new Map(), edges = [];

    const opt = (name, d) => (props[name] === undefined || props[name] === null ? d : props[name]);
    const on = (name, d) => truthy(opt(name, d));
    // `draggable`, `connectable` and `deletable` follow `editable` unless set.
    const can = (name) => on(name, on("editable", false));
    const direction = () => (FLOW_SIDES[String(opt("direction", "LR")).toUpperCase()] ? String(opt("direction", "LR")).toUpperCase() : "LR");
    const zoomRange = () => [num(opt("minZoom"), 0.1), num(opt("maxZoom"), 2)];

    // The template config of a node's type (`container`, `resizable`, …), found the
    // way the server picks the template.
    const typeOf = (n) => {
      const k = str(n?.type);
      return cfg.nodeTypes[k === "default" ? "" : k] ?? cfg.nodeTypes[""] ?? {};
    };
    const depth = (id) => {
      let d = 0;
      for (let n = byId.get(id); n && byId.has(str(n.parentId)) && d < 64; n = byId.get(str(n.parentId))) d++;
      return d;
    };
    const nodeEls = () => {
      const m = new Map();
      for (const n of nodeLayer.children) if (n.hasAttribute?.("data-flow-node")) m.set(n.getAttribute("data-flow-node"), n);
      return m;
    };
    const edgeEls = () => {
      const m = new Map();
      for (const n of edgeLayer.children) {
        const id = n.getAttribute?.("data-flow-edge") ?? n.getAttribute?.("data-flow-label");
        if (id === null || id === undefined) continue;
        const r = m.get(id) || {};
        if (n.hasAttribute("data-flow-edge")) r.svg = n;
        else r.label = n;
        m.set(id, r);
      }
      return m;
    };

    // Node styles from the JSON (`style`, `className`, `width`, `height`): style
    // properties are set one by one, so a value can't escape its property, and URLs
    // are dropped for documents the policy doesn't trust.
    const styled = new WeakMap();
    function dress(nodeEl, n) {
      const before = styled.get(nodeEl) || { classes: [], props: [] };
      nodeEl.classList.remove(...before.classes);
      for (const p of before.props) nodeEl.style.removeProperty(p);
      // `nodeStyles: false` leaves the saved `style` and `className` out, for apps that
      // work the look out from `data` (the template does that instead).
      const saved = on("nodeStyles", true);
      const classes = saved ? str(n.className).split(/\s+/).filter(Boolean) : [];
      nodeEl.classList.add(...classes);
      const props = [];
      const set = (k, v) => {
        const name = k.startsWith("--") ? k : k.replace(/[A-Z]/g, (c) => "-" + c.toLowerCase());
        if (!/^-{0,2}[a-z][a-z0-9-]*$/.test(name)) return;
        const value = typeof v === "number" && !/^(opacity|z-index|flex|font-weight|line-height|order)$/.test(name) ? v + "px" : str(v);
        if (cfg.safe && /url\(|expression\(|javascript:/i.test(value)) return;
        nodeEl.style.setProperty(name, value);
        props.push(name);
      };
      if (saved && isObj(n.style)) for (const [k, v] of Object.entries(n.style)) set(k, v);
      // A container or resizable node is exactly its size; others grow with content.
      const t = typeOf(n), fixed = t.container || t.resizable, size = sized.get(str(n.id));
      const w = size?.w ?? n.width, h = size?.h ?? n.height;
      if (typeof w === "number") set(fixed ? "width" : "min-width", w);
      if (typeof h === "number") set(fixed ? "height" : "min-height", h);
      styled.set(nodeEl, { classes, props });
    }

    // A port placed against the node itself, rather than against a positioned
    // element inside it (such as a row); wrappers like a `list`'s don't count.
    const onNode = (p) => p.offsetParent === p.closest("[data-flow-node]");
    // Sizes and port offsets, from the DOM. Ports with no side get one from the
    // flow's direction, and ports sharing a side without an offset share it evenly.
    function measure(nodeEl, id) {
      const [out, inn] = FLOW_SIDES[direction()];
      const list = [...nodeEl.querySelectorAll("[data-flow-port]")].filter((p) => p.closest("[data-flow-node]") === nodeEl);
      const groups = {};
      for (const p of list) {
        const type = p.getAttribute("data-port-type") === "target" ? "target" : "source";
        // A side the runtime or the document set wins over the one set here.
        const attr = p.getAttribute("data-side");
        if (attr === null || attr === p.__auto) {
          p.__auto = type === "target" ? inn : out;
          p.setAttribute("data-side", p.__auto);
        }
        const offset = p.getAttribute("data-offset");
        if (offset) p.style.setProperty("--flow-port-offset", /^-?\d+(\.\d+)?$/.test(offset) ? offset + "px" : offset);
        else (groups[onNode(p) ? p.getAttribute("data-side") : "\u0000"] ||= []).push(p);
      }
      for (const [side, ps] of Object.entries(groups)) {
        if (side === "\u0000") continue;
        ps.forEach((p, i) => p.style.setProperty("--flow-port-offset", ((i + 1) / (ps.length + 1)) * 100 + "%"));
      }
      // The outline, at the node's size in pixels, so curves and rims don't stretch.
      const kind = nodeEl.getAttribute("data-shape");
      const svg = nodeEl.querySelector(":scope > [data-flow-shape]");
      const o = kind ? outline(kind, nodeEl.getAttribute("data-points"), nodeEl.offsetWidth, nodeEl.offsetHeight) : null;
      if (o) shapes.set(id, o);
      else shapes.delete(id);
      if (svg) {
        svg.setAttribute("viewBox", `0 0 ${nodeEl.offsetWidth} ${nodeEl.offsetHeight}`);
        svg.firstElementChild?.setAttribute("d", o ? o.d : "");
      }
      // Ports placed on a shape last time are measured afresh from their side.
      for (const p of list) if (p.__onOutline) { p.style.left = p.style.top = ""; p.__onOutline = false; }
      const box = nodeEl.getBoundingClientRect();
      const z = view.zoom || 1;
      geo.set(id, { ...(geo.get(id) || { x: 0, y: 0 }), w: nodeEl.offsetWidth, h: nodeEl.offsetHeight });
      const w = nodeEl.offsetWidth, h = nodeEl.offsetHeight;
      ports.set(id, list.map((p) => {
        const r = p.getBoundingClientRect();
        const side = DIRS[p.getAttribute("data-side")] ? p.getAttribute("data-side") : "right";
        let x = (r.left + r.width / 2 - box.left) / z, y = (r.top + r.height / 2 - box.top) / z;
        // A port on the node itself sits inside its border (`100%` is the padding
        // box), so its edges start from the border instead. Ports deeper inside,
        // e.g. on a row, keep their own point.
        if (onNode(p) && o) {
          // On a shape: where the line through the port, across its side, meets the outline.
          const [dx, dy] = DIRS[side];
          const at = hit(o, dx ? w / 2 : x, dy ? h / 2 : y, dx, dy);
          if (at) {
            [x, y] = at;
            p.style.left = x + "px";
            p.style.top = y + "px";
            p.__onOutline = true;
          }
        } else if (onNode(p)) {
          if (side === "left") x = 0;
          else if (side === "right") x = w;
          else if (side === "top") y = 0;
          else y = h;
        }
        return { el: p, id: p.getAttribute("data-flow-port") ?? "", type: p.getAttribute("data-port-type") === "target" ? "target" : "source", side, x, y };
      }));
    }

    const observer = new ResizeObserver((entries) => {
      let changed = false;
      for (const e of entries) {
        if (e.target === el) {
          if (!fitted && el.clientWidth && el.clientHeight && geo.size) fitView(false);
          drawMinimap();
          continue;
        }
        dirty.add(e.target);
        changed = true;
      }
      if (changed) requestAnimationFrame(() => refresh(false));
    });
    observer.observe(el);
    const watched = new WeakSet();
    // Entering or leaving fullscreen changes the view's size: fit the nodes to it.
    document.addEventListener("fullscreenchange", () => {
      if (el.isConnected && (document.fullscreenElement === el || !document.fullscreenElement)) requestAnimationFrame(() => fitView(false));
    });

    // Reads the nodes and edges, measures new or resized nodes, lays them out and
    // draws everything.
    function refresh(all) {
      nodes = (Array.isArray(props.nodes) ? props.nodes : []).filter((n) => isObj(n) && n.id !== null && n.id !== undefined && !n.hidden);
      edges = (Array.isArray(props.edges) ? props.edges : []).filter(isObj);
      // `filter`: only these nodes, with the nodes they connect to as context.
      neighbours = new Set();
      const shown = Array.isArray(props.filter) && props.filter.length ? new Set(props.filter.map(str)) : null;
      if (shown) {
        if (on("filterNeighbors", true))
          for (const e of edges) {
            const s = str(e.source), t = str(e.target);
            if (shown.has(s) && !shown.has(t)) neighbours.add(t);
            if (shown.has(t) && !shown.has(s)) neighbours.add(s);
          }
        // A shown node's container is shown too, or the node would have nowhere to sit.
        const all = new Map(nodes.map((n) => [str(n.id), n]));
        for (const id of [...shown, ...neighbours])
          for (let p = all.get(str(all.get(id)?.parentId)), i = 0; p && i < 64; p = all.get(str(p.parentId)), i++)
            if (!shown.has(str(p.id))) neighbours.add(str(p.id));
        nodes = nodes.filter((n) => shown.has(str(n.id)) || neighbours.has(str(n.id)));
      }
      byId = new Map(nodes.map((n) => [str(n.id), n]));
      for (const id of dragged.keys()) if (!byId.has(id)) dragged.delete(id);
      const els = nodeEls();
      for (const [id, nodeEl] of els) {
        const n = byId.get(id);
        if (!n) continue;
        show(nodeEl, true);
        if (nodeEl.__node !== n) { dress(nodeEl, n); nodeEl.__node = n; dirty.add(nodeEl); }
        const shape = nodeEl.getAttribute("data-shape") + " " + nodeEl.getAttribute("data-points");
        if (nodeEl.__shape !== shape) { nodeEl.__shape = shape; dirty.add(nodeEl); }
        if (!watched.has(nodeEl)) { watched.add(nodeEl); observer.observe(nodeEl); dirty.add(nodeEl); }
        if (all) dirty.add(nodeEl);
      }
      for (const nodeEl of dirty) {
        const id = nodeEl.getAttribute("data-flow-node");
        if (nodeEl.isConnected && byId.has(id)) measure(nodeEl, id);
      }
      dirty.clear();
      for (const id of geo.keys()) if (!byId.has(id)) { geo.delete(id); ports.delete(id); shapes.delete(id); }
      layout();
      search();
      place(els);
      if (!writing) exposeNode();
      if (!fitted && on("fit", true) && geo.size && el.clientWidth && el.clientHeight) fitView(false);
      else if (!fitted && !on("fit", true)) {
        const v = cfg.bindViewport && isObj(api.state[cfg.bindViewport]) ? api.state[cfg.bindViewport] : props.viewport;
        if (isObj(v)) setView({ x: num(v.x, 0), y: num(v.y, 0), zoom: num(v.zoom, 1) });
        fitted = true;
      }
      const c = props.center;
      if (c !== centred) { centred = c; if (c !== null && c !== undefined && fitted) centreOn(str(c)); }
    }

    // Top-level nodes when `layout` is `layered`; their children keep `position`.
    function layout() {
      if (opt("layout", "") !== "layered") { laidOut = new Map(); layoutKey = ""; return; }
      const top = nodes.filter((n) => !byId.has(str(n.parentId))).map((n) => str(n.id));
      const root = (id) => { let n = byId.get(id); while (n && byId.has(str(n.parentId))) n = byId.get(str(n.parentId)); return n ? str(n.id) : id; };
      const links = edges.map((e) => [root(str(e.source)), root(str(e.target))]);
      const dir = direction(), ng = num(opt("nodeGap"), 40), rg = num(opt("rankGap"), 80);
      const key = JSON.stringify([top.map((id) => [id, geo.get(id)?.w, geo.get(id)?.h]), links, dir, ng, rg]);
      if (key === layoutKey) return;
      // A new layout (new nodes, sizes or direction) replaces dragged positions.
      if (layoutKey && JSON.parse(layoutKey)[2] !== dir) dragged.clear();
      layoutKey = key;
      laidOut = layered(top, links, (id) => geo.get(id) || { w: 150, h: 40 }, dir, ng, rg);
    }

    // A node's position relative to its parent (or the flow), before `nodeOrigin`.
    const relative = (n) => {
      const id = str(n.id);
      if (dragged.has(id)) return dragged.get(id);
      if (laidOut.has(id)) return laidOut.get(id);
      return isObj(n.position) ? { x: num(n.position.x, 0), y: num(n.position.y, 0) } : { x: 0, y: 0 };
    };
    const origin = () => {
      const o = opt("nodeOrigin", [0, 0]);
      return Array.isArray(o) ? [num(o[0], 0), num(o[1], 0)] : [0, 0];
    };
    // Top-left corners, parents first (a child's position is relative to its parent's corner).
    function positions() {
      const [ox, oy] = origin();
      const done = new Map();
      const corner = (n, depth = 0) => {
        const id = str(n.id);
        if (done.has(id)) return done.get(id);
        const g = geo.get(id) || { w: 0, h: 0 };
        const p = relative(n);
        const laid = laidOut.has(id) && !dragged.has(id);
        let x = p.x - (laid ? 0 : ox * g.w), y = p.y - (laid ? 0 : oy * g.h);
        const parent = byId.get(str(n.parentId));
        if (parent && depth < 64) { const c = corner(parent, depth + 1); x += c.x; y += c.y; }
        const r = { x, y };
        done.set(id, r);
        return r;
      };
      for (const n of nodes) {
        const id = str(n.id), c = corner(n), g = geo.get(id);
        if (g) { g.x = c.x; g.y = c.y; }
      }
    }

    function place(els = nodeEls()) {
      positions();
      const sel = selected === null || selected === undefined ? null : str(selected);
      const near = new Set();
      const focusing = sel !== null && byId.has(sel) && opt("highlight", "") === "neighbors";
      if (focusing) {
        near.add(sel);
        for (const e of edges) {
          if (str(e.source) === sel) near.add(str(e.target));
          if (str(e.target) === sel) near.add(str(e.source));
        }
      }
      const selectable = on("selectable", true), draggable = can("draggable");
      flag(el, "data-connectable", connectable());
      const activeId = found?.[active] ?? null, hits = found && new Set(found);
      for (const [id, nodeEl] of els) {
        const g = geo.get(id);
        show(nodeEl, !!g);
        if (!g) continue;
        nodeEl.style.transform = `translate(${g.x}px, ${g.y}px)`;
        // Containers under their children; a dragged node above everything.
        const d = depth(id) * 3 + (typeOf(byId.get(id)).container ? 0 : 2);
        nodeEl.style.zIndex = nodeEl.hasAttribute("data-dragging") ? 1000 + d : d;
        flag(nodeEl, "data-selected", picked.nodes.has(id));
        flag(nodeEl, "data-dimmed", (focusing && !near.has(id)) || (hits && !hits.has(id)));
        flag(nodeEl, "data-match", !!hits?.has(id));
        flag(nodeEl, "data-neighbor", neighbours.has(id));
        flag(nodeEl, "data-active", id === activeId);
        flag(nodeEl, "data-selectable", selectable);
        flag(nodeEl, "data-draggable", draggable);
        if (selectable) nodeEl.tabIndex = 0;
        else nodeEl.removeAttribute("tabindex");
      }
      drawEdges(sel, focusing, hits);
      drawMinimap();
    }
    // `display` rather than `hidden`, which a template's own `display` would override.
    const show = (n, v) => (n.style.display = v ? "" : "none");
    const flag = (n, attr, v) => (v ? n.setAttribute(attr, "") : n.removeAttribute(attr));

    // Where the line between two nodes' centres leaves node `a`'s outline (its box,
    // without a shape), and on which side, for floating edges.
    function meet(a, b) {
      const g = geo.get(a), q = geo.get(b);
      const dx = q.x + q.w / 2 - (g.x + g.w / 2), dy = q.y + q.h / 2 - (g.y + g.h / 2);
      if (!dx && !dy) return { x: g.x + g.w / 2, y: g.y + g.h / 2, side: "right" };
      const [px, py] = hit(shapes.get(a) || outline("rect", null, g.w, g.h), g.w / 2, g.h / 2, dx, dy) || [g.w / 2, g.h / 2];
      const sx = dx ? g.w / 2 / Math.abs(dx) : Infinity, sy = dy ? g.h / 2 / Math.abs(dy) : Infinity;
      return { x: g.x + px, y: g.y + py, side: sx < sy ? (dx > 0 ? "right" : "left") : dy > 0 ? "bottom" : "top" };
    }

    // A port's point, by handle id, or the first port of the right type, or the middle
    // of the side the flow's direction gives.
    function end(id, handle, type) {
      const g = geo.get(id);
      const list = ports.get(id) || [];
      const h = handle === null || handle === undefined || handle === "" ? null : str(handle);
      const p = h !== null ? list.find((p) => p.id === h && p.type === type) || list.find((p) => p.id === h) : list.find((p) => p.type === type);
      if (p) return { x: g.x + p.x, y: g.y + p.y, side: p.side };
      const [out, inn] = FLOW_SIDES[direction()];
      const side = type === "source" ? out : inn;
      const [dx, dy] = DIRS[side];
      return { x: g.x + g.w / 2 + (dx * g.w) / 2, y: g.y + g.h / 2 + (dy * g.h) / 2, side };
    }

    const marker = (v, d) => {
      const t = String(isObj(v) ? v.type : v ?? d ?? "").toLowerCase();
      return t === "arrow" || t === "arrowclosed" ? `url(#${uid}-${t})` : null;
    };

    // Labels go at their edges' middles, unless one would cover a label placed before
    // it or a node: then at the nearest free point along its own edge, either way.
    // Containers don't count, as labels inside them are where they belong.
    function placeLabels(list) {
      const parents = new Set(nodes.map((n) => str(n.parentId)).filter((id) => byId.has(id)));
      const taken = [];
      for (const [id, g] of geo) if (!parents.has(id) && g.w) taken.push({ x: g.x, y: g.y, w: g.w, h: g.h });
      const free = (b) => !taken.some((o) => b.x < o.x + o.w + 2 && o.x < b.x + b.w + 2 && b.y < o.y + o.h + 2 && o.y < b.y + b.h + 2);
      for (const l of list) {
        const w = l.el.offsetWidth, h = l.el.offsetHeight;
        const box = (x, y) => ({ x: x - w / 2, y: y - h / 2, w, h });
        let best = box(l.x, l.y);
        if (!free(best)) {
          const len = l.path.getTotalLength();
          for (const f of [0.4, 0.6, 0.3, 0.7, 0.2, 0.8]) {
            const q = l.path.getPointAtLength(len * f);
            if (free(box(q.x, q.y))) { best = box(q.x, q.y); break; }
          }
        }
        taken.push(best);
        l.el.style.transform = `translate(${best.x + w / 2}px, ${best.y + h / 2}px) translate(-50%, -50%)`;
      }
    }

    function drawEdges(sel, focusing, hits) {
      const els = edgeEls();
      const seen = new Set();
      const labels = [];
      for (const e of edges) {
        const id = edgeId(e);
        const r = els.get(id);
        if (!r || seen.has(id)) continue;
        seen.add(id);
        const s = str(e.source), t = str(e.target);
        const visible = !e.hidden && geo.has(s) && geo.has(t);
        // An <svg> has no `hidden` property, so set the attribute (the CSS hides it).
        if (r.svg) r.svg.toggleAttribute("hidden", !visible);
        if (r.label) r.label.hidden = !visible;
        if (!visible || !r.svg) continue;
        const type = str(e.type);
        const tc = cfg.edgeTypes[type] || cfg.edgeTypes[""] || {};
        const builtin = { default: "bezier", simplebezier: "bezier", straight: "straight", step: "step", smoothstep: "smoothstep" }[type];
        const kind = tc.path || builtin || opt("edgePath", "bezier");
        let a, b;
        if (tc.floating || type === "floating") {
          a = meet(s, t);
          b = meet(t, s);
        } else {
          a = end(s, e.sourceHandle, "source");
          b = end(t, e.targetHandle, "target");
        }
        // A floating edge from a node to itself has no direction: a loop off its
        // top-right corner.
        const p = s === t && (tc.floating || type === "floating") ? selfLoop(geo.get(s)) : route(kind, a, b);
        const path = r.svg.querySelector("[data-flow-path]");
        path.setAttribute("d", p.d);
        // A wide, invisible copy of the path takes the clicks.
        const hit = r.svg.querySelector("[data-flow-hit]");
        if (hit) {
          hit.setAttribute("d", p.d);
          hit.style.pointerEvents = on("selectable", true) ? "stroke" : "none";
        }
        flag(path, "data-selected", picked.edges.has(id));
        const me = marker(e.markerEnd, tc.marker), ms = marker(e.markerStart, tc.markerStart);
        me ? path.setAttribute("marker-end", me) : path.removeAttribute("marker-end");
        ms ? path.setAttribute("marker-start", ms) : path.removeAttribute("marker-start");
        flag(path, "data-animated", e.animated ?? tc.animated ?? false);
        const touches = focusing && (s === sel || t === sel);
        const dim = (focusing && !touches) || (hits && !(hits.has(s) && hits.has(t)));
        flag(path, "data-highlighted", touches);
        flag(r.svg, "data-dimmed", dim);
        // Above the deeper end's container, below its nodes.
        const z = Math.max(depth(s), depth(t)) * 3 + 1;
        r.svg.style.zIndex = z;
        if (r.label) r.label.style.zIndex = z;
        if (r.svg.__style !== e.style) {
          for (const k of r.svg.__props || []) path.style.removeProperty(k);
          r.svg.__props = [];
          if (isObj(e.style)) {
            for (const [k, v] of Object.entries(e.style)) {
              const name = k.replace(/[A-Z]/g, (c) => "-" + c.toLowerCase());
              if (!/^(stroke|stroke-width|stroke-dasharray|stroke-linecap|opacity|stroke-opacity)$/.test(name)) continue;
              if (cfg.safe && /url\(/i.test(str(v))) continue;
              path.style.setProperty(name, str(v));
              r.svg.__props.push(name);
            }
          }
          r.svg.__style = e.style;
        }
        if (r.svg.__class !== e.className) {
          const old = str(r.svg.__class).split(/\s+/).filter(Boolean), next = str(e.className).split(/\s+/).filter(Boolean);
          for (const n of [r.svg, r.label]) { n?.classList.remove(...old); n?.classList.add(...next); }
          r.svg.__class = e.className;
        }
        if (r.label) {
          labels.push({ el: r.label, path, x: p.x, y: p.y });
          flag(r.label, "data-dimmed", dim);
        }
      }
      for (const [id, r] of els) if (!seen.has(id)) { if (r.svg) r.svg.setAttribute("hidden", ""); if (r.label) r.label.hidden = true; }
      placeLabels(labels);
    }

    // --------------------------------------------------------------- the view
    let viewTimer, anim;
    function setView(v, animate) {
      const [lo, hi] = zoomRange();
      const next = { x: num(v.x, 0), y: num(v.y, 0), zoom: clamp(num(v.zoom, 1), lo, hi) };
      cancelAnimationFrame(anim);
      anim = 0;
      if (animate && !reducedMotion.matches) {
        const from = { ...view }, t0 = performance.now();
        const step = (now) => {
          const k = Math.min(1, (now - t0) / 250), e = 1 - Math.pow(1 - k, 3);
          apply({ x: from.x + (next.x - from.x) * e, y: from.y + (next.y - from.y) * e, zoom: from.zoom + (next.zoom - from.zoom) * e });
          anim = k < 1 ? requestAnimationFrame(step) : 0;
        };
        anim = requestAnimationFrame(step);
      } else apply(next);
    }
    function apply(v, quiet) {
      view = v;
      pane.style.transform = `translate(${v.x}px, ${v.y}px) scale(${v.zoom})`;
      // The pattern doubles its spacing when zooming out would pack it too tight.
      let gap = 20 * v.zoom;
      while (gap < 10) gap *= 2;
      el.style.setProperty("--flow-gap", gap + "px");
      el.style.setProperty("--flow-bg-x", (v.x % gap) + "px");
      el.style.setProperty("--flow-bg-y", (v.y % gap) + "px");
      drawMinimap();
      if (cfg.bindViewport && !quiet) {
        clearTimeout(viewTimer);
        viewTimer = setTimeout(() => api.set(cfg.bindViewport, { x: view.x, y: view.y, zoom: view.zoom }), 150);
      }
    }
    function bounds(ids) {
      let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
      for (const id of ids || geo.keys()) {
        const g = geo.get(id);
        if (!g) continue;
        x0 = Math.min(x0, g.x); y0 = Math.min(y0, g.y); x1 = Math.max(x1, g.x + g.w); y1 = Math.max(y1, g.y + g.h);
      }
      return x0 === Infinity ? null : { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
    }
    function fitView(animate) {
      const b = bounds();
      const W = el.clientWidth, H = el.clientHeight;
      if (!b || !W || !H) return;
      fitted = true;
      const pad = num(opt("fitPadding"), 0.1);
      const [lo, hi] = zoomRange();
      const zoom = clamp(Math.min((W * (1 - 2 * pad)) / (b.w || 1), (H * (1 - 2 * pad)) / (b.h || 1), 1), lo, hi);
      setView({ x: W / 2 - (b.x + b.w / 2) * zoom, y: H / 2 - (b.y + b.h / 2) * zoom, zoom }, animate);
    }
    function centreOn(id) {
      const g = geo.get(id);
      if (!g) return;
      const z = view.zoom;
      setView({ x: el.clientWidth / 2 - (g.x + g.w / 2) * z, y: el.clientHeight / 2 - (g.y + g.h / 2) * z, zoom: z }, true);
    }
    function zoomBy(f, cx = el.clientWidth / 2, cy = el.clientHeight / 2, animate) {
      const [lo, hi] = zoomRange();
      const z = clamp(view.zoom * f, lo, hi);
      setView({ x: cx - ((cx - view.x) * z) / view.zoom, y: cy - ((cy - view.y) * z) / view.zoom, zoom: z }, animate);
    }

    // ----------------------------------------------------------------- minimap
    let mini;
    function drawMinimap() {
      if (!minimap) return;
      const W = minimap.clientWidth, H = minimap.clientHeight;
      if (!W || !H) return;
      if (!mini) {
        mini = document.createElementNS(NS, "svg");
        mini.setAttribute("width", "100%");
        mini.setAttribute("height", "100%");
        minimap.append(mini);
      }
      const vw = el.clientWidth / view.zoom, vh = el.clientHeight / view.zoom, vx = -view.x / view.zoom, vy = -view.y / view.zoom;
      const b = bounds() || { x: vx, y: vy, w: vw, h: vh };
      const x0 = Math.min(b.x, vx), y0 = Math.min(b.y, vy), x1 = Math.max(b.x + b.w, vx + vw), y1 = Math.max(b.y + b.h, vy + vh);
      const s = Math.max((x1 - x0) / W, (y1 - y0) / H) * 1.1 || 1;
      const ox = x0 - (W * s - (x1 - x0)) / 2, oy = y0 - (H * s - (y1 - y0)) / 2;
      mini.setAttribute("viewBox", `${ox} ${oy} ${W * s} ${H * s}`);
      mini.__map = { ox, oy, s };
      const sel = selected === null || selected === undefined ? null : str(selected);
      let html = "";
      for (const [id, g] of geo) {
        const mark = id === sel ? " data-selected" : "";
        const o = shapes.get(id);
        html += o ? `<path transform="translate(${g.x},${g.y})" d="${o.d}"${mark}/>` : `<rect x="${g.x}" y="${g.y}" width="${g.w}" height="${g.h}" rx="${4 * s}"${mark}/>`;
      }
      html += `<path fill-rule="evenodd" d="M${ox},${oy}h${W * s}v${H * s}h${-W * s}z M${vx},${vy}h${vw}v${vh}h${-vw}z"/>`;
      mini.innerHTML = html;
    }
    if (minimap) {
      const panTo = (e) => {
        const m = mini?.__map;
        if (!m) return;
        const r = minimap.getBoundingClientRect();
        const fx = m.ox + (e.clientX - r.left) * m.s, fy = m.oy + (e.clientY - r.top) * m.s;
        setView({ x: el.clientWidth / 2 - fx * view.zoom, y: el.clientHeight / 2 - fy * view.zoom, zoom: view.zoom });
      };
      minimap.addEventListener("pointerdown", (e) => {
        e.stopPropagation();
        try { minimap.setPointerCapture(e.pointerId); } catch {}
        panTo(e);
        const move = (ev) => panTo(ev);
        const up = () => { minimap.removeEventListener("pointermove", move); minimap.removeEventListener("pointerup", up); minimap.removeEventListener("pointercancel", up); };
        minimap.addEventListener("pointermove", move);
        minimap.addEventListener("pointerup", up);
        minimap.addEventListener("pointercancel", up);
      });
    }

    // -------------------------------------------------------------- selection
    // The node selected last is the primary one: `bind` holds it, and `highlight`
    // follows it. `bindSelection` holds every selected node and edge.
    const outId = (id) => byId.get(id)?.id ?? id; // as the node has it, so a number stays a number
    const selectionKey = () => JSON.stringify([[...picked.nodes], [...picked.edges]]);
    function choose(change, open) {
      if (!on("selectable", true)) return;
      const before = selectionKey();
      change();
      if (selectionKey() !== before) {
        const list = [...picked.nodes];
        selected = list.length ? list[list.length - 1] : null;
        place();
        const value = selected === null ? null : outId(selected);
        // Each write runs `update`, which mustn't take the other key's old value
        // for a selection made outside.
        writing = true;
        try {
          if (cfg.bind) api.set(cfg.bind, value);
          if (cfg.bindSelection) api.set(cfg.bindSelection, { nodes: list.map(outId), edges: [...picked.edges] });
        } finally {
          writing = false;
        }
        exposeNode();
        api.emit("nodeselect", { id: value });
      }
      if (open && selected !== null) api.emit("nodeopen", { id: outId(selected) });
    }
    const toggle = (set, id, add) => {
      if (!add) {
        picked.nodes.clear();
        picked.edges.clear();
        set.add(id);
      } else if (set.has(id)) set.delete(id);
      else set.add(id);
    };
    const selectNode = (id, add, open) => choose(() => toggle(picked.nodes, id, add), open);
    const selectEdge = (id, add) => choose(() => toggle(picked.edges, id, add));
    const clearSelection = () => choose(() => { picked.nodes.clear(); picked.edges.clear(); });

    // ---------------------------------------------------------------- writing
    // Edits go to the state keys in `bindNodes` and `bindEdges`; point `nodes` and
    // `edges` at the same keys to see them.
    const rawNodes = () => (Array.isArray(props.nodes) ? props.nodes : []);
    const rawEdges = () => (Array.isArray(props.edges) ? props.edges : []);
    const warned = new Set();
    const writes = (key, what) => {
      if (cfg[key]) return true;
      if (!warned.has(key)) { warned.add(key); console.warn(`ui-flow: set \`${key}\` to ${what}`); }
      return false;
    };
    // One undo step per user action, however many writes it makes. Only edits the
    // flow makes are recorded; undo puts back both arrays as they were.
    function edit(fn) {
      if (batching) return fn();
      const before = { nodes: rawNodes(), edges: rawEdges() };
      batching++;
      let out;
      try { out = fn(); } finally { batching--; }
      if (rawNodes() !== before.nodes || rawEdges() !== before.edges) {
        history.push(before);
        if (history.length > 100) history.shift();
        future = [];
        announce();
      }
      return out;
    }
    function restore(from, to) {
      const s = from.pop();
      if (!s) return;
      to.push({ nodes: rawNodes(), edges: rawEdges() });
      dragged.clear();
      sized.clear();
      if (cfg.bindNodes) api.set(cfg.bindNodes, s.nodes);
      if (cfg.bindEdges) api.set(cfg.bindEdges, s.edges);
      choose(() => {
        const ids = new Set(rawNodes().filter(isObj).map((n) => str(n.id))), es = new Set(rawEdges().filter(isObj).map(edgeId));
        for (const id of [...picked.nodes]) if (!ids.has(id)) picked.nodes.delete(id);
        for (const id of [...picked.edges]) if (!es.has(id)) picked.edges.delete(id);
      });
      announce();
    }
    const undo = () => restore(history, future);
    const redo = () => restore(future, history);
    const announce = () => cfg.bindHistory && api.set(cfg.bindHistory, { undo: history.length, redo: future.length });
    const toFlow = (e) => {
      const r = el.getBoundingClientRect();
      return { x: (e.clientX - r.left - el.clientLeft - view.x) / view.zoom, y: (e.clientY - r.top - el.clientTop - view.y) / view.zoom };
    };
    // `base` in snake_case plus the first free number: `serviceCall` → `service_call_1`.
    function freshId(base, also) {
      base = str(base || "node").replace(/([a-z0-9])([A-Z])/g, "$1_$2").toLowerCase().replace(/[^a-z0-9_-]/g, "_") || "node";
      const taken = new Set(rawNodes().filter(isObj).map((n) => str(n.id)));
      for (const id of also || []) taken.add(id);
      let i = 1;
      while (taken.has(`${base}_${i}`)) i++;
      return `${base}_${i}`;
    }
    // Adds a node made from `template` (`type`, `data`, … ; `id` if it's free) with
    // its `position` at `at`.
    function addNode(template, at) {
      if (!writes("bindNodes", "add nodes")) return null;
      const t = isObj(template) ? template : {};
      const raw = rawNodes();
      const free = t.id !== undefined && t.id !== null && !raw.some((n) => isObj(n) && str(n.id) === str(t.id));
      const node = { ...t, id: free ? t.id : freshId(t.type), position: { x: Math.round(at.x), y: Math.round(at.y) }, data: isObj(t.data) ? { ...t.data } : {} };
      edit(() => api.set(cfg.bindNodes, [...raw, node]));
      api.emit("nodeadd", { id: node.id });
      return node;
    }
    // In the middle of the view, nudged off any node already there.
    function addAtCentre(template, position) {
      let at = isObj(position) ? { x: num(position.x, 0), y: num(position.y, 0) }
        : { x: (el.clientWidth / 2 - view.x) / view.zoom, y: (el.clientHeight / 2 - view.y) / view.zoom };
      if (!isObj(position)) {
        const [ox, oy] = origin();
        at = { x: at.x - 75 + ox * 150, y: at.y - 20 + oy * 40 };
        for (let i = 0; i < 20 && nodes.some((n) => isObj(n.position) && Math.abs(num(n.position.x, 0) - at.x) < 8 && Math.abs(num(n.position.y, 0) - at.y) < 8); i++)
          at = { x: at.x + 24, y: at.y + 24 };
      }
      const node = addNode(template, at);
      if (node) selectNode(str(node.id));
      return node;
    }

    // ------------------------------------------------------------- connecting
    const connectable = () => can("connectable") && !!cfg.bindEdges;
    const rules = () => (isObj(props.connect) ? props.connect : {});
    const sameHandle = (a, b) => str(a ?? "") === str(b ?? "");
    const handleOf = (p) => (p.id === "" ? null : p.id);
    const count = (nodeId, handle, type) =>
      edges.filter((e) => (type === "source" ? str(e.source) === nodeId && sameHandle(e.sourceHandle, handle) : str(e.target) === nodeId && sameHandle(e.targetHandle, handle))).length;
    // A port's own rules, read when they're needed, since they may follow state.
    const rulesOf = (port) => {
      const p = port.el, accepts = p?.getAttribute("data-accepts"), max = p?.getAttribute("data-max");
      let list = null;
      if (accepts) {
        try { list = accepts.startsWith("[") ? JSON.parse(accepts).map(str) : accepts.split(",").map((a) => a.trim()); } catch {}
      }
      return { connectable: !p?.hasAttribute("data-noconnect"), accepts: list, max: max ? num(max, null) : null, scope: p?.getAttribute("data-scope") };
    };
    // Whether an edge may join `s` (a source port) to `t` (a target port), each
    // `{ node, port }`, under the flow's `connect` rules and the ports' own.
    function allowed(s, t) {
      if (!s?.port || !t?.port || s.port.type !== "source" || t.port.type !== "target") return false;
      const sr = rulesOf(s.port), tr = rulesOf(t.port);
      if (!sr.connectable || !tr.connectable) return false;
      const r = rules(), a = byId.get(s.node), b = byId.get(t.node);
      if (!a || !b) return false;
      if (s.node === t.node && !truthy(r.self)) return false;
      const sh = handleOf(s.port), th = handleOf(t.port);
      if (r.unique !== false && edges.some((e) => str(e.source) === s.node && str(e.target) === t.node && sameHandle(e.sourceHandle, sh) && sameHandle(e.targetHandle, th))) return false;
      // A `scope: children` port joins only the nodes inside its own node.
      if (sr.scope === "children" || tr.scope === "children") {
        if (sr.scope === "children" && str(b.parentId) !== s.node) return false;
        if (tr.scope === "children" && str(a.parentId) !== t.node) return false;
      } else if (truthy(r.sameParent) && str(a.parentId ?? "") !== str(b.parentId ?? "")) return false;
      const ta = str(a.type || "default"), tb = str(b.type || "default");
      if (sr.accepts && !sr.accepts.includes(tb)) return false;
      if (tr.accepts && !tr.accepts.includes(ta)) return false;
      if (sr.max !== null && count(s.node, sh, "source") >= sr.max) return false;
      if (tr.max !== null && count(t.node, th, "target") >= tr.max) return false;
      if (Array.isArray(r.pairs) && !r.pairs.some((p) => isObj(p) && (p.source == null || str(p.source) === ta) && (p.target == null || str(p.target) === tb))) return false;
      return true;
    }
    // Either way round: a drag may start at a target port.
    const fits = (from, to) => (from.port.type === "source" ? allowed(from, to) : allowed(to, from));
    function connect(from, to) {
      const [s, t] = from.port.type === "source" ? [from, to] : [to, from];
      if (!writes("bindEdges", "add edges")) return;
      const sh = handleOf(s.port), th = handleOf(t.port);
      const taken = new Set(edges.map(edgeId));
      let id = `${s.node}-${t.node}`;
      if (taken.has(id)) id = `${s.node}${sh === null ? "" : "." + sh}-${t.node}${th === null ? "" : "." + th}`;
      for (let i = 2; taken.has(id); i++) id = id.replace(/~\d+$/, "") + "~" + i;
      const edge = { ...(isObj(props.newEdge) ? props.newEdge : {}), id, source: outId(s.node), target: outId(t.node) };
      if (sh !== null) edge.sourceHandle = sh;
      if (th !== null) edge.targetHandle = th;
      edit(() => api.set(cfg.bindEdges, [...rawEdges(), edge]));
      api.emit("connect", { id, source: edge.source, target: edge.target });
    }
    // The port a dragged connection would join: the nearest one of the other type
    // within reach, or, over a node, the first of its ports that fits.
    function candidate(e, from) {
      const want = from.port.type === "source" ? "target" : "source";
      const at = toFlow(e);
      let best = null, reach = 24 / view.zoom;
      for (const [id, list] of ports) {
        const g = geo.get(id);
        if (!g) continue;
        for (const p of list) {
          if (p.type !== want) continue;
          const d = Math.hypot(g.x + p.x - at.x, g.y + p.y - at.y);
          if (d < reach) { reach = d; best = { node: id, port: p }; }
        }
      }
      if (best) return best;
      const over = document.elementFromPoint(e.clientX, e.clientY)?.closest?.("[data-flow-node]");
      if (!over || !nodeLayer.contains(over)) return null;
      const id = over.getAttribute("data-flow-node");
      for (const p of ports.get(id) || []) if (p.type === want && fits(from, { node: id, port: p })) return { node: id, port: p };
      return { node: id, port: null };
    }
    // The line drawn while connecting.
    const wire = document.createElementNS(NS, "svg");
    wire.setAttribute("aria-hidden", "true");
    wire.style.cssText = "position:absolute;z-index:9999;left:0;top:0;width:1px;height:1px;overflow:visible;pointer-events:none;display:none";
    wire.innerHTML = `<path fill="none" stroke="var(--color-primary, #18181b)" stroke-width="1.5" stroke-dasharray="5 4"/>`;
    pane.append(wire);
    let marked = null;
    const mark = (portEl, state) => {
      if (marked && marked !== portEl) marked.removeAttribute("data-connect");
      marked = portEl;
      if (portEl) portEl.setAttribute("data-connect", state);
    };
    function drawWire(e, from, to) {
      const g = geo.get(from.node);
      const a = { x: g.x + from.port.x, y: g.y + from.port.y, side: from.port.side };
      let b;
      if (to?.port) {
        const h = geo.get(to.node);
        b = { x: h.x + to.port.x, y: h.y + to.port.y, side: to.port.side };
      } else b = { ...toFlow(e), side: OPPOSITE[a.side] };
      const kind = str(isObj(props.newEdge) && props.newEdge.type) || opt("edgePath", "bezier");
      const path = route({ default: "bezier", simplebezier: "bezier" }[kind] || kind, a, b);
      wire.style.display = "";
      wire.firstChild.setAttribute("d", path.d);
    }
    // A connection dropped on empty canvas: with `newNode`, a node is made there
    // and joined to the port the drag started from.
    function dropped(e, from) {
      if (!isObj(props.newNode) || !cfg.bindNodes) return;
      const at = toFlow(e);
      // Next to the node it comes from: inside the same container.
      const parent = byId.get(from.node)?.parentId;
      const template = byId.has(str(parent)) ? { ...props.newNode, parentId: parent } : props.newNode;
      const pg = byId.has(str(parent)) ? geo.get(str(parent)) : null;
      edit(() => {
        const node = addNode(template, pg ? { x: at.x - pg.x, y: at.y - pg.y } : at);
        if (!node) return;
        const id = str(node.id), want = from.port.type === "source" ? "target" : "source";
        const port = (ports.get(id) || []).find((p) => p.type === want) || { id: "", type: want, side: OPPOSITE[from.port.side], x: 0, y: 0 };
        connect(from, { node: id, port });
        selectNode(id);
      });
    }

    // --------------------------------------------------------------- deleting
    // The selection, a deleted node's children and every edge touching a deleted node.
    function doomed() {
      const ns = new Set(picked.nodes);
      for (let grew = true; grew; ) {
        grew = false;
        for (const n of rawNodes()) if (isObj(n) && !ns.has(str(n.id)) && ns.has(str(n.parentId))) { ns.add(str(n.id)); grew = true; }
      }
      const es = new Set(picked.edges);
      for (const e of edges) if (ns.has(str(e.source)) || ns.has(str(e.target))) es.add(edgeId(e));
      return { nodes: [...ns], edges: [...es] };
    }
    // Deletes the selection, or with `confirmDelete` first writes what would go to
    // `bindDelete` and opens that dialog, whose button sends `flow.delete`.
    function requestDelete() {
      if (!can("deletable")) return;
      const d = doomed();
      if (!d.nodes.length && !d.edges.length) return;
      const dialog = opt("confirmDelete", null);
      if (!dialog) return remove(d);
      pending = d;
      if (cfg.bindDelete) api.set(cfg.bindDelete, { nodes: d.nodes.map(outId), edges: d.edges });
      try { document.getElementById(str(dialog))?.showModal?.(); } catch {}
      api.emit("deleterequest", { nodes: d.nodes.map(outId), edges: d.edges });
    }
    function remove(d) {
      const ns = new Set(d.nodes), es = new Set(d.edges);
      pending = null;
      edit(() => {
        if ((ns.size || es.size) && writes("bindEdges", "delete edges"))
          api.set(cfg.bindEdges, rawEdges().filter((e) => !isObj(e) || !(es.has(edgeId(e)) || ns.has(str(e.source)) || ns.has(str(e.target)))));
        if (ns.size && writes("bindNodes", "delete nodes")) api.set(cfg.bindNodes, rawNodes().filter((n) => !isObj(n) || !ns.has(str(n.id))));
      });
      if (cfg.bindDelete) api.set(cfg.bindDelete, null);
      choose(() => { ns.forEach((id) => picked.nodes.delete(id)); es.forEach((id) => picked.edges.delete(id)); });
      api.emit("delete", { nodes: d.nodes.map(outId), edges: d.edges });
    }
    const cancelDelete = () => {
      pending = null;
      if (cfg.bindDelete) api.set(cfg.bindDelete, null);
    };

    // ---------------------------------------------------------------- commands
    // `{ action: call, name: flow.fit }` and friends: from a button inside the flow
    // (a panel, a node, a dialog in either), or from anywhere with `args.flow` set
    // to the flow's element id. Without `args.flow`, a call from outside reaches
    // the page's only flow.
    function command(name, args) {
      if (name === "flow.fit") fitView(true);
      else if (name === "flow.zoomin") zoomBy(1.2, undefined, undefined, true);
      else if (name === "flow.zoomout") zoomBy(1 / 1.2, undefined, undefined, true);
      else if (name === "flow.delete") pending ? remove(pending) : requestDelete();
      else if (name === "flow.cancel") cancelDelete();
      else if (name === "flow.add") addAtCentre(args.node, args.position);
      else if (name === "flow.select") args.id === null || args.id === undefined ? clearSelection() : selectNode(str(args.id));
      else if (name === "flow.center") centreOn(str(args.id));
      else if (name === "flow.relayout") { dragged.clear(); layoutKey = ""; refresh(false); fitView(true); }
      else if (name === "flow.fullscreen") {
        if (document.fullscreenElement === el) document.exitFullscreen?.();
        else el.requestFullscreen?.().catch(() => {});
      } else if (name === "flow.update") updateNode(args);
      else if (name === "flow.undo") undo();
      else if (name === "flow.redo") redo();
      else if (name === "flow.copy") copy();
      else if (name === "flow.cut") cut();
      else if (name === "flow.paste") paste(null);
      else if (name === "flow.next" || name === "flow.prev") step(name === "flow.next" ? 1 : -1);
      else return false;
      return true;
    }
    const calls = (e) => (typeof e.detail?.name === "string" && e.detail.name.startsWith("flow.") ? e.detail : null);
    el.addEventListener("irender:call", (e) => {
      const d = calls(e);
      if (!d || (d.args?.flow != null && str(d.args.flow) !== el.id)) return;
      if (command(d.name, d.args || {})) e.stopPropagation();
    });
    const outside = (e) => {
      const d = calls(e);
      if (!d || !el.isConnected) return document.removeEventListener("irender:call", outside);
      const mine = d.args?.flow != null ? el.id !== "" && str(d.args.flow) === el.id : document.querySelectorAll('[data-i-x="ui-flow"]').length === 1;
      if (mine && !el.contains(e.target)) command(d.name, d.args || {});
    };
    document.addEventListener("irender:call", outside);

    // ------------------------------------------------- palette: drag and drop
    el.addEventListener("dragover", (e) => {
      if (!cfg.bindNodes || ![...(e.dataTransfer?.types || [])].includes(MIME)) return;
      e.preventDefault();
      e.dataTransfer.dropEffect = "copy";
    });
    el.addEventListener("drop", (e) => {
      const raw = e.dataTransfer?.getData(MIME);
      if (!raw || !cfg.bindNodes) return;
      e.preventDefault();
      let template;
      try { template = JSON.parse(raw); } catch { return; }
      const node = addNode(template, toFlow(e));
      if (node) selectNode(str(node.id));
    });

    // ------------------------------------------- pointer: pan, drag, connect
    const pointers = new Map();
    let gesture = null; // { kind: "pan" | "node" | "connect" | "box" | "pinch", … }
    let box = null; // the selection rectangle
    const interactive = (t) => t.closest("button, a[href], input, select, textarea, label, summary, [contenteditable], [data-flow-controls], [data-flow-minimap], [data-flow-nodrag]");
    el.addEventListener("pointerdown", (e) => {
      if (e.button !== 0 && e.pointerType === "mouse") return;
      const t = e.target;
      if (t.closest("[data-flow-controls], [data-flow-minimap]") || (t.closest("[data-position]") && !t.closest("[data-flow-pane]"))) return;
      pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
      if (pointers.size === 2 && on("zoom", true)) {
        const [p, q] = [...pointers.values()];
        if (gesture?.kind === "connect") stopConnecting();
        gesture = { kind: "pinch", d: Math.hypot(p.x - q.x, p.y - q.y), zoom: view.zoom, view: { ...view }, mid: { x: (p.x + q.x) / 2, y: (p.y + q.y) / 2 } };
        return;
      }
      if (pointers.size > 1) return;
      const base = { x: e.clientX, y: e.clientY, shift: e.shiftKey || e.metaKey || e.ctrlKey, moved: false };
      const handle = t.closest("[data-flow-resize]");
      if (handle && nodeLayer.contains(handle)) {
        const nodeEl = handle.closest("[data-flow-node]"), id = nodeEl.getAttribute("data-flow-node"), g = geo.get(id);
        if (g) {
          e.preventDefault();
          gesture = { ...base, kind: "resize", id, el: nodeEl, w: g.w, h: g.h };
          return;
        }
      }
      const portEl = t.closest("[data-flow-port]");
      const nodeEl = t.closest("[data-flow-node]");
      const inNode = nodeEl && nodeLayer.contains(nodeEl);
      if (portEl && inNode && connectable()) {
        const id = nodeEl.getAttribute("data-flow-node");
        const port = (ports.get(id) || []).find((p) => p.el === portEl);
        if (port && rulesOf(port).connectable) {
          e.preventDefault();
          gesture = { ...base, kind: "connect", from: { node: id, port } };
          return;
        }
      }
      // Controls inside a node work as usual.
      if (inNode && interactive(t)) return;
      if (inNode) {
        gesture = { ...base, kind: "node", id: nodeEl.getAttribute("data-flow-node"), el: nodeEl };
      } else if (t.closest("[data-flow-hit]") && edgeLayer.contains(t)) {
        gesture = { ...base, kind: "pan", view: { ...view }, edge: t.closest("svg[data-flow-edge]")?.getAttribute("data-flow-edge") };
      } else if (!t.closest("[data-flow-label]")) {
        gesture = { ...base, kind: base.shift && on("selectable", true) ? "box" : "pan", view: { ...view } };
      }
    });
    // Captured only once a drag starts, so that a plain click still reaches the
    // node it was on (and its `on.click` actions).
    const start = (e) => {
      gesture.moved = true;
      try {
        if (!el.hasPointerCapture(e.pointerId)) el.setPointerCapture(e.pointerId);
      } catch {} // not an active pointer, e.g. a synthetic event
    };
    // The nodes a drag moves: the selection when it includes the node, less any
    // node whose parent moves with it.
    function grab(id) {
      if (!picked.nodes.has(id)) selectNode(id);
      const ids = picked.nodes.has(id) ? [...picked.nodes] : [id];
      const set = new Set(ids);
      const under = (n) => { for (let p = byId.get(str(n.parentId)), i = 0; p && i < 64; p = byId.get(str(p.parentId)), i++) if (set.has(str(p.id))) return true; return false; };
      return ids.filter((x) => byId.has(x) && !under(byId.get(x))).map((x) => [x, { ...relative(byId.get(x)) }]);
    }
    let hover = null; // where the pointer is, in flow coordinates, for pasting
    el.addEventListener("pointerleave", () => (hover = null));
    el.addEventListener("pointermove", (e) => {
      if (!e.target.closest?.("[data-flow-controls], [data-flow-minimap], [data-position]") || e.target.closest("[data-flow-pane]")) hover = toFlow(e);
      if (!pointers.has(e.pointerId)) return;
      pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
      const g = gesture;
      if (!g) return;
      if (g.kind === "pinch") {
        const [p, q] = [...pointers.values()];
        const d = Math.hypot(p.x - q.x, p.y - q.y);
        const r = el.getBoundingClientRect();
        const [lo, hi] = zoomRange();
        const z = clamp((g.zoom * d) / (g.d || 1), lo, hi);
        const cx = g.mid.x - r.left, cy = g.mid.y - r.top;
        setView({ x: cx - ((cx - g.view.x) * z) / g.view.zoom, y: cy - ((cy - g.view.y) * z) / g.view.zoom, zoom: z });
        return;
      }
      const dx = e.clientX - g.x, dy = e.clientY - g.y;
      if (!g.moved && Math.hypot(dx, dy) < 4) return;
      if (g.kind === "pan") {
        if (!on("pan", true)) return;
        start(e);
        el.setAttribute("data-panning", "");
        setView({ x: g.view.x + dx, y: g.view.y + dy, zoom: view.zoom });
      } else if (g.kind === "node") {
        if (!can("draggable")) return;
        if (!g.moved) { g.group = grab(g.id); g.el.setAttribute("data-dragging", ""); }
        start(e);
        for (const [id, from] of g.group) dragged.set(id, { x: from.x + dx / view.zoom, y: from.y + dy / view.zoom });
        place();
        // The container it would drop into.
        const into = g.group.length ? landing(g.group[0][0]) : null;
        if (g.into !== into) {
          if (g.into) nodeEls().get(g.into)?.removeAttribute("data-drop-target");
          if (into) nodeEls().get(into)?.setAttribute("data-drop-target", "");
          g.into = into;
        }
      } else if (g.kind === "resize") {
        start(e);
        const t = typeOf(byId.get(g.id));
        const w = Math.max(num(t.minWidth, 80), g.w + dx / view.zoom), h = Math.max(num(t.minHeight, 40), g.h + dy / view.zoom);
        sized.set(g.id, { w: Math.round(w), h: Math.round(h) });
        g.el.style.width = Math.round(w) + "px";
        g.el.style.height = Math.round(h) + "px";
        measure(g.el, g.id);
        place();
      } else if (g.kind === "connect") {
        start(e);
        el.setAttribute("data-connecting", "");
        const to = candidate(e, g.from);
        g.to = to?.port && fits(g.from, to) ? to : null;
        mark(to?.port?.el || null, g.to ? "valid" : "invalid");
        drawWire(e, g.from, g.to);
      } else if (g.kind === "box") {
        start(e);
        const r = el.getBoundingClientRect();
        if (!box) {
          box = document.createElement("div");
          box.setAttribute("data-flow-box", "");
          box.style.cssText = "position:absolute;z-index:3;pointer-events:none;border:1px solid var(--color-primary, #18181b);background:color-mix(in srgb, var(--color-primary, #18181b) 8%, transparent);border-radius:2px";
          el.append(box);
        }
        const x0 = Math.min(g.x, e.clientX) - r.left, y0 = Math.min(g.y, e.clientY) - r.top;
        Object.assign(box.style, { left: x0 + "px", top: y0 + "px", width: Math.abs(dx) + "px", height: Math.abs(dy) + "px" });
        g.end = { clientX: e.clientX, clientY: e.clientY };
      }
    });
    function stopConnecting() {
      wire.style.display = "none";
      mark(null);
      el.removeAttribute("data-connecting");
    }
    const release = (e) => {
      pointers.delete(e.pointerId);
      const g = gesture;
      if (!g || pointers.size) { if (g?.kind === "pinch" && pointers.size === 1) gesture = null; return; }
      gesture = null;
      el.removeAttribute("data-panning");
      if (g.kind === "connect") stopConnecting();
      if (g.kind === "box") { box?.remove(); box = null; }
      if (e.type === "pointercancel") return;
      if (g.kind === "pan" && !g.moved) {
        if (g.edge !== undefined && g.edge !== null) selectEdge(g.edge, g.shift);
        else if (!g.shift) clearSelection();
      } else if (g.kind === "node" && !g.moved) selectNode(g.id, g.shift);
      else if (g.kind === "node") {
        g.el.removeAttribute("data-dragging");
        if (g.into) nodeEls().get(g.into)?.removeAttribute("data-drop-target");
        moved(g.group.map(([id]) => id));
      } else if (g.kind === "resize" && g.moved) {
        const size = sized.get(g.id);
        if (size && writes("bindNodes", "resize nodes"))
          edit(() => api.set(cfg.bindNodes, rawNodes().map((n) => (isObj(n) && str(n.id) === g.id ? { ...n, width: size.w, height: size.h } : n))));
        api.emit("noderesize", { id: outId(g.id) });
      } else if (g.kind === "connect") {
        if (!g.moved) return selectNode(g.from.node, g.shift);
        const to = candidate(e, g.from);
        if (to?.port && fits(g.from, to)) connect(g.from, to);
        else if (!to) dropped(e, g.from);
      } else if (g.kind === "box") {
        if (!g.moved || !g.end) return;
        const a = toFlow({ clientX: g.x, clientY: g.y }), b = toFlow(g.end);
        const x0 = Math.min(a.x, b.x), x1 = Math.max(a.x, b.x), y0 = Math.min(a.y, b.y), y1 = Math.max(a.y, b.y);
        const inside = [...geo].filter(([, r]) => r.x >= x0 && r.y >= y0 && r.x + r.w <= x1 && r.y + r.h <= y1).map(([id]) => id);
        choose(() => inside.forEach((id) => { picked.nodes.delete(id); picked.nodes.add(id); }));
      }
    };
    el.addEventListener("pointerup", release);
    el.addEventListener("pointercancel", release);
    el.addEventListener("dblclick", (e) => {
      const nodeEl = e.target.closest("[data-flow-node]");
      if (nodeEl && nodeLayer.contains(nodeEl) && !interactive(e.target)) selectNode(nodeEl.getAttribute("data-flow-node"), false, true);
    });

    // After a drag: the nodes, with the moved ones' new positions, go to `bindNodes`.
    // After a drag: the nodes, with the moved ones' new positions, go to `bindNodes`.
    // A node dropped onto a container moves into it, and one dragged out of its
    // container moves out; with `connect.sameParent`, edges that now cross from
    // one container to another are removed.
    function moved(ids) {
      const at = new Map(ids.filter((id) => dragged.has(id)).map((id) => [id, dragged.get(id)]));
      if (!at.size) return;
      if (cfg.bindNodes) {
        const [ox, oy] = origin();
        const parents = new Map();
        for (const id of at.keys()) {
          const into = landing(id), now = byId.has(str(byId.get(id)?.parentId)) ? str(byId.get(id).parentId) : null;
          if (into !== now) parents.set(id, into);
        }
        const position = (id) => {
          const g = geo.get(id) || { x: 0, y: 0, w: 0, h: 0 };
          if (parents.has(id)) {
            // The node stays where it is on screen: its corner, relative to the new parent.
            const pg = parents.get(id) === null ? { x: 0, y: 0 } : geo.get(parents.get(id));
            return { x: Math.round(g.x - pg.x + ox * g.w), y: Math.round(g.y - pg.y + oy * g.h) };
          }
          const p = at.get(id);
          // Laid-out positions are corners; saved positions follow `nodeOrigin`.
          const laid = laidOut.has(id);
          return { x: Math.round(p.x + (laid ? ox * g.w : 0)), y: Math.round(p.y + (laid ? oy * g.h : 0)) };
        };
        let list = rawNodes().map((n) => {
          if (!isObj(n) || !at.has(str(n.id))) return n;
          const id = str(n.id), next = { ...n, position: position(id) };
          if (parents.has(id)) {
            const parent = parents.get(id);
            if (parent === null) delete next.parentId;
            else next.parentId = byId.get(parent)?.id ?? parent;
            dragged.set(id, { x: next.position.x, y: next.position.y });
          }
          return next;
        });
        edit(() => {
          if (parents.size) list = parentsFirst(list);
          api.set(cfg.bindNodes, list);
          if (parents.size && truthy(rules().sameParent) && cfg.bindEdges) {
            const parentOf = new Map(list.filter(isObj).map((n) => [str(n.id), str(n.parentId ?? "")]));
            // A `scope: children` port keeps only edges into its own node; any other
            // edge needs both ends in the same container.
            const scoped = (id, handle, type) =>
              rulesOf((ports.get(id) || []).find((p) => p.type === type && sameHandle(p.id === "" ? null : p.id, handle)) || {}).scope === "children";
            const keep = (e) => {
              const s = str(e.source), t = str(e.target);
              if (scoped(s, e.sourceHandle, "source")) return parentOf.get(t) === s;
              if (scoped(t, e.targetHandle, "target")) return parentOf.get(s) === t;
              return parentOf.get(s) === parentOf.get(t);
            };
            const kept = rawEdges().filter((e) => !isObj(e) || keep(e));
            if (kept.length !== rawEdges().length) api.set(cfg.bindEdges, kept);
          }
        });
      }
      for (const id of at.keys()) api.emit("nodemove", { id: outId(id) });
    }
    // The innermost container under a node's centre, not counting the node itself
    // or anything inside it; null for none.
    function landing(id) {
      const g = geo.get(id);
      if (!g) return null;
      const cx = g.x + g.w / 2, cy = g.y + g.h / 2;
      const inside = (c) => { for (let n = byId.get(c), i = 0; n && i < 64; n = byId.get(str(n.parentId)), i++) if (str(n.id) === id) return true; return false; };
      let best = null, bestDepth = -1;
      for (const n of nodes) {
        const c = str(n.id);
        if (c === id || !typeOf(n).container || inside(c)) continue;
        const b = geo.get(c);
        if (!b || cx < b.x || cx > b.x + b.w || cy < b.y || cy > b.y + b.h) continue;
        const d = depth(c);
        if (d > bestDepth) { best = c; bestDepth = d; }
      }
      return best;
    }
    // React Flow wants a parent before its children.
    function parentsFirst(list) {
      const byKey = new Map(list.filter(isObj).map((n) => [str(n.id), n]));
      const out = [], done = new Set();
      const put = (n, d = 0) => {
        if (!isObj(n)) return out.push(n);
        const id = str(n.id);
        if (done.has(id)) return;
        done.add(id);
        const parent = byKey.get(str(n.parentId));
        if (parent && d < 64) put(parent, d + 1);
        out.push(n);
      };
      list.forEach((n) => put(n));
      return out;
    }

    // `flow.update`: merges the call's args into a node's `data`: the node `args.id`
    // names, or the selected one. `flow` and `id` themselves aren't merged.
    function updateNode(args) {
      const id = args.id !== undefined && args.id !== null ? str(args.id) : selected === null || selected === undefined ? null : str(selected);
      if (id === null || !writes("bindNodes", "update nodes")) return;
      const patch = Object.fromEntries(Object.entries(args).filter(([k]) => k !== "flow" && k !== "id"));
      let hit = false;
      const list = rawNodes().map((n) => {
        if (!isObj(n) || str(n.id) !== id) return n;
        hit = true;
        return { ...n, data: { ...(isObj(n.data) ? n.data : {}), ...patch } };
      });
      if (!hit) return;
      edit(() => api.set(cfg.bindNodes, list));
      api.emit("nodeupdate", { id: outId(id) });
    }
    // `bindNode`: the selected node as it is in `nodes`, kept up to date.
    let shownNode;
    function exposeNode() {
      if (!cfg.bindNode) return;
      const id = selected === null || selected === undefined ? null : str(selected);
      const n = id === null ? null : rawNodes().find((x) => isObj(x) && str(x.id) === id) ?? null;
      const json = JSON.stringify(n);
      if (json === shownNode) return;
      shownNode = json;
      api.set(cfg.bindNode, n);
    }

    // ---------------------------------------------------------- copy and paste
    // The selection with everything inside it, and the edges between them.
    function copy() {
      const ns = new Set(doomed().nodes);
      if (!ns.size) return false;
      const list = rawNodes().filter((n) => isObj(n) && ns.has(str(n.id)));
      const es = rawEdges().filter((e) => isObj(e) && ns.has(str(e.source)) && ns.has(str(e.target)));
      clipboard = { nodes: structuredClone(list), edges: structuredClone(es), pastes: 0 };
      return true;
    }
    function cut() {
      if (copy() && can("deletable")) remove(doomed());
    }
    // New ids for everything, at the pointer when it is over the flow, else a step
    // down and right of the original; the pasted nodes become the selection.
    function paste(at) {
      if (!clipboard || !writes("bindNodes", "paste nodes")) return;
      const map = new Map(), made = [];
      for (const n of clipboard.nodes) {
        // Named after the original: `check_stock` → `check_stock_1`.
        const id = freshId(str(n.id).replace(/_\d+$/, ""), made);
        made.push(id);
        map.set(str(n.id), id);
      }
      clipboard.pastes++;
      const top = (n) => !map.has(str(n.parentId));
      const tops = clipboard.nodes.filter(top);
      let dx = 24 * clipboard.pastes, dy = 24 * clipboard.pastes;
      if (at && tops.every((n) => !byId.has(str(n.parentId)))) {
        dx = at.x - Math.min(...tops.map((n) => num(n.position?.x, 0)));
        dy = at.y - Math.min(...tops.map((n) => num(n.position?.y, 0)));
      }
      const nodesOut = clipboard.nodes.map((n) => {
        const next = { ...structuredClone(n), id: map.get(str(n.id)) };
        delete next.selected;
        if (map.has(str(n.parentId))) next.parentId = map.get(str(n.parentId));
        else if (!byId.has(str(n.parentId))) delete next.parentId;
        if (top(n)) next.position = { x: Math.round(num(n.position?.x, 0) + dx), y: Math.round(num(n.position?.y, 0) + dy) };
        return next;
      });
      const taken = new Set(rawEdges().filter(isObj).map(edgeId));
      const edgesOut = clipboard.edges.map((e) => {
        const next = { ...structuredClone(e), source: map.get(str(e.source)), target: map.get(str(e.target)) };
        let id = `${next.source}-${next.target}`;
        for (let i = 2; taken.has(id); i++) id = `${next.source}-${next.target}~${i}`;
        taken.add(id);
        next.id = id;
        delete next.selected;
        return next;
      });
      edit(() => {
        api.set(cfg.bindNodes, [...rawNodes(), ...nodesOut]);
        if (edgesOut.length && cfg.bindEdges) api.set(cfg.bindEdges, [...rawEdges(), ...edgesOut]);
      });
      choose(() => { picked.nodes.clear(); picked.edges.clear(); for (const id of made) picked.nodes.add(id); });
      api.emit("nodeadd", { ids: nodesOut.map((n) => n.id) });
    }

    // ------------------------------------------------------------------ search
    // `search` matches a node's text or id, ignoring case and accents. The matches
    // go to `bindMatches`; `flow.next` / `flow.prev` step through them.
    const fold = (v) => str(v).normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase();
    function search() {
      const q = fold(opt("search", "")).trim();
      const els = nodeEls();
      const next = q ? nodes.map((n) => str(n.id)).filter((id) => fold(id).includes(q) || fold(els.get(id)?.textContent).includes(q)) : null;
      const changed = q !== query;
      if (changed) { query = q; active = 0; }
      if (JSON.stringify(next) !== JSON.stringify(found)) {
        found = next;
        if (active >= (found?.length ?? 0)) active = 0;
        if (cfg.bindMatches) api.set(cfg.bindMatches, (found || []).map(outId));
      }
      if (changed && found?.length) centreOn(found[active]);
    }
    function step(by) {
      if (!found?.length) return;
      active = (active + by + found.length) % found.length;
      place();
      centreOn(found[active]);
    }

    el.addEventListener("wheel", (e) => {
      if (!on("zoom", true) && !on("pan", true)) return;
      const r = el.getBoundingClientRect();
      const scale = e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? el.clientHeight : 1;
      const zooming = e.ctrlKey || e.metaKey || opt("wheel", "zoom") !== "pan";
      if (zooming && on("zoom", true)) {
        e.preventDefault();
        zoomBy(Math.exp(-e.deltaY * scale * (e.ctrlKey ? 0.01 : 0.002)), e.clientX - r.left, e.clientY - r.top);
      } else if (!zooming && on("pan", true)) {
        e.preventDefault();
        setView({ x: view.x - e.deltaX * scale, y: view.y - e.deltaY * scale, zoom: view.zoom });
      }
    }, { passive: false });

    el.querySelector(":scope > [data-flow-controls]")?.addEventListener("click", (e) => {
      const a = e.target.closest("[data-flow-action]")?.getAttribute("data-flow-action");
      if (a === "zoom-in") zoomBy(1.2, undefined, undefined, true);
      else if (a === "zoom-out") zoomBy(1 / 1.2, undefined, undefined, true);
      else if (a === "fit") fitView(true);
    });

    // ---------------------------------------------------------------- keyboard
    el.addEventListener("keydown", (e) => {
      const t = e.target;
      if (t !== el && interactive(t) && !t.matches("[data-flow-node]")) return;
      const nodeEl = t.closest?.("[data-flow-node]");
      const id = nodeEl && nodeLayer.contains(nodeEl) ? nodeEl.getAttribute("data-flow-node") : null;
      const arrows = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] };
      if ((e.key === "Enter" || e.key === " ") && id !== null) {
        e.preventDefault();
        const open = e.key === "Enter" && picked.nodes.has(id);
        selectNode(id, e.shiftKey && !open, open);
      } else if (e.key === "Escape") {
        if (gesture?.kind === "connect") { stopConnecting(); gesture = null; }
        else if (picked.nodes.size || picked.edges.size) clearSelection();
        else return;
      } else if ((e.key === "Delete" || e.key === "Backspace") && can("deletable")) {
        e.preventDefault();
        requestDelete();
      } else if ((e.metaKey || e.ctrlKey) && (e.key === "z" || e.key === "Z" || e.key === "y")) {
        e.preventDefault();
        e.key === "y" || e.shiftKey ? redo() : undo();
      } else if ((e.metaKey || e.ctrlKey) && (e.key === "c" || e.key === "x" || e.key === "v")) {
        // Text someone has selected in a node copies as text.
        if (e.key !== "v" && !window.getSelection()?.isCollapsed) return;
        e.preventDefault();
        if (e.key === "c") copy();
        else if (e.key === "x") cut();
        else if (cfg.bindNodes) paste(hover);
      } else if (e.key === "a" && (e.metaKey || e.ctrlKey) && on("selectable", true)) {
        e.preventDefault();
        choose(() => { for (const n of nodes) picked.nodes.add(str(n.id)); });
      } else if ((e.key === "+" || e.key === "=") && on("zoom", true)) {
        zoomBy(1.2, undefined, undefined, true);
      } else if (e.key === "-" && on("zoom", true)) {
        zoomBy(1 / 1.2, undefined, undefined, true);
      } else if (arrows[e.key]) {
        e.preventDefault();
        const [dx, dy] = arrows[e.key];
        const step = e.shiftKey ? 50 : 10;
        if (id !== null && can("draggable") && byId.has(id)) {
          const group = grab(id);
          for (const [x, from] of group) dragged.set(x, { x: from.x + dx * step, y: from.y + dy * step });
          place();
          clearTimeout(el.__moveTimer);
          el.__moveTimer = setTimeout(() => moved(group.map(([x]) => x)), 300);
        } else if (on("pan", true)) setView({ x: view.x - dx * step * 4, y: view.y - dy * step * 4, zoom: view.zoom });
      } else return;
    });

    refresh(true);
    return {
      // For `ui-flow-item` and host code: adds a node in the middle of the view.
      add: (template, position) => addAtCentre(template, position),
      // For `ui-flow-item` dragged by touch: adds a node where it was let go, in page
      // coordinates, as a drop does.
      drop(template, x, y) {
        if (!cfg.bindNodes) return null;
        const node = addNode(template, toFlow({ clientX: x, clientY: y }));
        if (node) selectNode(str(node.id));
        return node;
      },
      command: (name, args) => command(name, args || {}),
      update(state) {
        if (cfg.reactive) props = resolve(cfg.props);
        // Selections written from outside replace the flow's own.
        if (!writing && cfg.bindSelection && isObj(state[cfg.bindSelection])) {
          const v = state[cfg.bindSelection];
          const next = [Array.isArray(v.nodes) ? v.nodes.map(str) : [], Array.isArray(v.edges) ? v.edges.map(str) : []];
          if (JSON.stringify(next) !== selectionKey()) {
            picked = { nodes: new Set(next[0]), edges: new Set(next[1]) };
            selected = next[0].length ? next[0][next[0].length - 1] : null;
          }
        }
        if (!writing && cfg.bind) {
          const v = state[cfg.bind];
          if (str(v) !== str(selected)) {
            selected = v ?? null;
            picked = { nodes: new Set(selected === null ? [] : [str(selected)]), edges: new Set() };
          }
        }
        if (cfg.bindDelete && state[cfg.bindDelete] === null) pending = null;
        if (cfg.bindViewport && isObj(state[cfg.bindViewport])) {
          const v = state[cfg.bindViewport];
          if (Math.abs(num(v.x, 0) - view.x) > 0.5 || Math.abs(num(v.y, 0) - view.y) > 0.5 || Math.abs(num(v.zoom, 1) - view.zoom) > 0.001)
            if (!gesture && !anim) apply({ x: num(v.x, 0), y: num(v.y, 0), zoom: num(v.zoom, 1) }, true);
        }
        const dir = direction();
        refresh(dir !== this.dir);
        this.dir = dir;
      },
    };
  };

  // ----------------------------------------------------------- ui-flow-item
  // A palette entry: drag it onto a flow, or click it (Enter, Space) to add its
  // node in the middle of the view. By touch (or pen), where the browser's drag and
  // drop doesn't start, a press held still for 300ms picks it up, a copy follows the
  // finger, and letting go over a flow adds the node there.
  window.__irenderBehaviors["ui-flow-item"] = (el, api) => {
    const resolve = resolver(api);
    const node = () => resolve(api.config.node);
    const target = () => {
      const id = api.config.flow;
      if (id) return document.getElementById(id);
      const all = document.querySelectorAll('[data-i-x="ui-flow"]');
      return el.closest('[data-i-x="ui-flow"]') || (all.length === 1 ? all[0] : null);
    };
    el.addEventListener("dragstart", (e) => {
      e.dataTransfer.setData(MIME, JSON.stringify(node()));
      e.dataTransfer.effectAllowed = "copy";
    });
    const add = () => target()?.__x?.add?.(node());
    el.addEventListener("click", (e) => {
      if (dragged) {
        dragged = false;
        return e.preventDefault();
      }
      add();
    });

    let press = null, ghost = null, dragged = false;
    const flowAt = (x, y) => {
      const f = document.elementFromPoint(x, y)?.closest('[data-i-x="ui-flow"]');
      return f && (!api.config.flow || f.id === api.config.flow) ? f : null;
    };
    const finish = (drop) => {
      clearTimeout(press?.timer);
      if (ghost) {
        ghost.remove();
        ghost = null;
        el.removeAttribute("data-touch-dragging");
        dragged = true;
        setTimeout(() => (dragged = false), 100);
        if (drop) flowAt(press.x, press.y)?.__x?.drop?.(node(), press.x, press.y);
      }
      press = null;
    };
    el.addEventListener("pointerdown", (e) => {
      if (e.pointerType === "mouse" || !e.isPrimary || ghost) return;
      clearTimeout(press?.timer);
      press = {
        id: e.pointerId, x: e.clientX, y: e.clientY, x0: e.clientX, y0: e.clientY,
        timer: setTimeout(() => {
          const r = el.getBoundingClientRect();
          ghost = el.cloneNode(true);
          ghost.removeAttribute("id");
          ghost.setAttribute("aria-hidden", "true");
          Object.assign(ghost.style, {
            position: "fixed", left: `${r.left}px`, top: `${r.top}px`, width: `${r.width}px`, margin: "0", zIndex: "2147483000",
            pointerEvents: "none", opacity: "0.92", boxShadow: "0 12px 28px rgb(0 0 0 / 0.2)",
          });
          press.dx = press.x0 - r.left;
          press.dy = press.y0 - r.top;
          el.after(ghost);
          el.setAttribute("data-touch-dragging", "");
          navigator.vibrate?.(8);
        }, 300),
      };
    });
    el.addEventListener("pointermove", (e) => {
      if (!press || e.pointerId !== press.id) return;
      press.x = e.clientX;
      press.y = e.clientY;
      if (ghost) {
        ghost.style.left = `${press.x - press.dx}px`;
        ghost.style.top = `${press.y - press.dy}px`;
      } else if (Math.hypot(press.x - press.x0, press.y - press.y0) > 8) finish(false);
    });
    el.addEventListener("pointerup", (e) => press && e.pointerId === press.id && finish(true));
    el.addEventListener("pointercancel", (e) => press && e.pointerId === press.id && finish(false));
    el.addEventListener("touchmove", (e) => ghost && e.cancelable && e.preventDefault(), { passive: false });
    el.addEventListener("contextmenu", (e) => press && e.preventDefault());
    // A touch press isn't a mouse drag: the browser's own drag doesn't start.
    el.addEventListener("dragstart", (e) => press && e.preventDefault(), { capture: true });
    addEventListener("keydown", (e) => e.key === "Escape" && ghost && finish(false));
    el.addEventListener("keydown", (e) => {
      if (e.key !== "Enter" && e.key !== " ") return;
      e.preventDefault();
      add();
    });
  };
})();
