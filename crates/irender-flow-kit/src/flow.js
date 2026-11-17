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
  let flows = 0;

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

  function route(kind, s, t) {
    if (kind === "straight") return { d: `M${s.x},${s.y} L${t.x},${t.y}`, x: (s.x + t.x) / 2, y: (s.y + t.y) / 2 };
    if (kind === "step") return polyline(orthogonal(s, t, 20), 0);
    if (kind === "smoothstep") return polyline(orthogonal(s, t, 20), 8);
    return bezier(s, t);
  }

  // Where the line between two boxes' centres leaves box `a`, and on which side.
  function border(a, b) {
    const ax = a.x + a.w / 2, ay = a.y + a.h / 2, bx = b.x + b.w / 2, by = b.y + b.h / 2;
    const dx = bx - ax, dy = by - ay;
    if (!dx && !dy) return { x: ax, y: ay, side: "right" };
    const sx = dx ? a.w / 2 / Math.abs(dx) : Infinity, sy = dy ? a.h / 2 / Math.abs(dy) : Infinity;
    const s = Math.min(sx, sy);
    const side = sx < sy ? (dx > 0 ? "right" : "left") : dy > 0 ? "bottom" : "top";
    return { x: ax + dx * s, y: ay + dy * s, side };
  }

  // -------------------------------------------------------------------- layout
  // A layered (Sugiyama-style) layout of the top-level nodes: cycles broken,
  // longest-path ranks, barycentre ordering, then positions that pull each node
  // towards its neighbours without letting nodes in a rank overlap. Components are
  // laid out separately and stacked. Returns top-left corners by id.
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
    let offset = 0;
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
      for (const id of list) {
        const a = rankAt.get(id), c = centre.get(id) - across(id) / 2 - lo + offset;
        pos.set(id, horizontal ? { x: a, y: c } : { x: c, y: a });
      }
      offset += hi - lo + nodeGap * 2;
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
    let fitted = false, centred;
    const dragged = new Map(); // id → position set by dragging
    let laidOut = new Map(), layoutKey = "";
    const geo = new Map(); // id → { x, y, w, h }, absolute, in flow coordinates
    const ports = new Map(); // id → [{ id, type, side, x, y }], relative to the node
    const dirty = new Set(); // node elements to measure again
    let nodes = [], byId = new Map(), edges = [];

    const opt = (name, d) => (props[name] === undefined || props[name] === null ? d : props[name]);
    const on = (name, d) => truthy(opt(name, d));
    const direction = () => (FLOW_SIDES[String(opt("direction", "LR")).toUpperCase()] ? String(opt("direction", "LR")).toUpperCase() : "LR");
    const zoomRange = () => [num(opt("minZoom"), 0.1), num(opt("maxZoom"), 2)];

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
      const classes = str(n.className).split(/\s+/).filter(Boolean);
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
      if (isObj(n.style)) for (const [k, v] of Object.entries(n.style)) set(k, v);
      if (typeof n.width === "number") set("min-width", n.width);
      if (typeof n.height === "number") set("min-height", n.height);
      styled.set(nodeEl, { classes, props });
    }

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
        else (groups[p.parentElement === nodeEl ? p.getAttribute("data-side") : "\u0000"] ||= []).push(p);
      }
      for (const [side, ps] of Object.entries(groups)) {
        if (side === "\u0000") continue;
        ps.forEach((p, i) => p.style.setProperty("--flow-port-offset", ((i + 1) / (ps.length + 1)) * 100 + "%"));
      }
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
        if (p.parentElement === nodeEl) {
          if (side === "left") x = 0;
          else if (side === "right") x = w;
          else if (side === "top") y = 0;
          else y = h;
        }
        return { id: p.getAttribute("data-flow-port") ?? "", type: p.getAttribute("data-port-type") === "target" ? "target" : "source", side, x, y };
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

    // Reads the nodes and edges, measures new or resized nodes, lays them out and
    // draws everything.
    function refresh(all) {
      nodes = (Array.isArray(props.nodes) ? props.nodes : []).filter((n) => isObj(n) && n.id !== null && n.id !== undefined && !n.hidden);
      byId = new Map(nodes.map((n) => [str(n.id), n]));
      edges = (Array.isArray(props.edges) ? props.edges : []).filter(isObj);
      for (const id of dragged.keys()) if (!byId.has(id)) dragged.delete(id);
      const els = nodeEls();
      for (const [id, nodeEl] of els) {
        const n = byId.get(id);
        if (!n) continue;
        show(nodeEl, true);
        if (nodeEl.__node !== n) { dress(nodeEl, n); nodeEl.__node = n; dirty.add(nodeEl); }
        if (!watched.has(nodeEl)) { watched.add(nodeEl); observer.observe(nodeEl); dirty.add(nodeEl); }
        if (all) dirty.add(nodeEl);
      }
      for (const nodeEl of dirty) {
        const id = nodeEl.getAttribute("data-flow-node");
        if (nodeEl.isConnected && byId.has(id)) measure(nodeEl, id);
      }
      dirty.clear();
      for (const id of geo.keys()) if (!byId.has(id)) { geo.delete(id); ports.delete(id); }
      layout();
      place(els);
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
      const selectable = on("selectable", true), draggable = on("draggable", false);
      for (const [id, nodeEl] of els) {
        const g = geo.get(id);
        show(nodeEl, !!g);
        if (!g) continue;
        nodeEl.style.transform = `translate(${g.x}px, ${g.y}px)`;
        flag(nodeEl, "data-selected", id === sel);
        flag(nodeEl, "data-dimmed", focusing && !near.has(id));
        flag(nodeEl, "data-selectable", selectable);
        flag(nodeEl, "data-draggable", draggable);
        if (selectable) nodeEl.tabIndex = 0;
        else nodeEl.removeAttribute("tabindex");
      }
      drawEdges(sel, focusing);
      drawMinimap();
    }
    // `display` rather than `hidden`, which a template's own `display` would override.
    const show = (n, v) => (n.style.display = v ? "" : "none");
    const flag = (n, attr, v) => (v ? n.setAttribute(attr, "") : n.removeAttribute(attr));

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

    function drawEdges(sel, focusing) {
      const els = edgeEls();
      const seen = new Set();
      for (const e of edges) {
        const id = edgeId(e);
        const r = els.get(id);
        if (!r || seen.has(id)) continue;
        seen.add(id);
        const s = str(e.source), t = str(e.target);
        const visible = !e.hidden && geo.has(s) && geo.has(t);
        if (r.svg) r.svg.hidden = !visible;
        if (r.label) r.label.hidden = !visible;
        if (!visible || !r.svg) continue;
        const type = str(e.type);
        const tc = cfg.edgeTypes[type] || cfg.edgeTypes[""] || {};
        const builtin = { default: "bezier", simplebezier: "bezier", straight: "straight", step: "step", smoothstep: "smoothstep" }[type];
        const kind = tc.path || builtin || opt("edgePath", "bezier");
        let a, b;
        if (tc.floating || type === "floating") {
          a = border(geo.get(s), geo.get(t));
          b = border(geo.get(t), geo.get(s));
        } else {
          a = end(s, e.sourceHandle, "source");
          b = end(t, e.targetHandle, "target");
        }
        const p = route(kind, a, b);
        const path = r.svg.querySelector("[data-flow-path]");
        path.setAttribute("d", p.d);
        const me = marker(e.markerEnd, tc.marker), ms = marker(e.markerStart, tc.markerStart);
        me ? path.setAttribute("marker-end", me) : path.removeAttribute("marker-end");
        ms ? path.setAttribute("marker-start", ms) : path.removeAttribute("marker-start");
        flag(path, "data-animated", e.animated ?? tc.animated ?? false);
        const touches = focusing && (s === sel || t === sel);
        flag(path, "data-highlighted", touches);
        flag(r.svg, "data-dimmed", focusing && !touches);
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
          r.label.style.transform = `translate(${p.x}px, ${p.y}px) translate(-50%, -50%)`;
          flag(r.label, "data-dimmed", focusing && !touches);
        }
      }
      for (const [id, r] of els) if (!seen.has(id)) { if (r.svg) r.svg.hidden = true; if (r.label) r.label.hidden = true; }
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
      const gap = 20 * v.zoom;
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
      for (const [id, g] of geo) html += `<rect x="${g.x}" y="${g.y}" width="${g.w}" height="${g.h}" rx="${4 * s}"${id === sel ? " data-selected" : ""}/>`;
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
    function select(id, open) {
      if (!on("selectable", true)) return;
      const next = id === null ? null : id;
      const changed = str(next) !== str(selected) || (next === null) !== (selected === null || selected === undefined);
      selected = next;
      if (changed) {
        place();
        // The id as the node has it, so a number stays a number.
        const value = next === null ? null : byId.get(next)?.id ?? next;
        if (cfg.bind) api.set(cfg.bind, value);
        api.emit("nodeselect", { id: value });
      }
      if (open && next !== null) api.emit("nodeopen", { id: byId.get(next)?.id ?? next });
    }

    // --------------------------------------------------- pointer: pan and drag
    const pointers = new Map();
    let gesture = null; // { kind: "pan" | "node" | "pinch", … }
    const interactive = (t) => t.closest("button, a[href], input, select, textarea, label, summary, [contenteditable], [data-flow-controls], [data-flow-minimap], [data-flow-nodrag]");
    el.addEventListener("pointerdown", (e) => {
      if (e.button !== 0 && e.pointerType === "mouse") return;
      const t = e.target;
      if (t.closest("[data-flow-controls], [data-flow-minimap]") || (t.closest("[data-position]") && !t.closest("[data-flow-pane]"))) return;
      pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
      if (pointers.size === 2 && on("zoom", true)) {
        const [p, q] = [...pointers.values()];
        gesture = { kind: "pinch", d: Math.hypot(p.x - q.x, p.y - q.y), zoom: view.zoom, view: { ...view }, mid: { x: (p.x + q.x) / 2, y: (p.y + q.y) / 2 } };
        return;
      }
      if (pointers.size > 1) return;
      const nodeEl = t.closest("[data-flow-node]");
      const inNode = nodeEl && nodeLayer.contains(nodeEl);
      // Controls inside a node work as usual.
      if (inNode && interactive(t)) return;
      if (inNode) {
        const id = nodeEl.getAttribute("data-flow-node");
        const n = byId.get(id);
        gesture = { kind: "node", id, el: nodeEl, x: e.clientX, y: e.clientY, from: n ? { ...relative(n) } : { x: 0, y: 0 }, moved: false };
      } else if (!t.closest("[data-flow-label]")) {
        gesture = { kind: "pan", x: e.clientX, y: e.clientY, view: { ...view }, moved: false };
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
    el.addEventListener("pointermove", (e) => {
      if (!pointers.has(e.pointerId)) return;
      pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
      if (!gesture) return;
      if (gesture.kind === "pinch") {
        const [p, q] = [...pointers.values()];
        const d = Math.hypot(p.x - q.x, p.y - q.y);
        const r = el.getBoundingClientRect();
        const [lo, hi] = zoomRange();
        const z = clamp((gesture.zoom * d) / (gesture.d || 1), lo, hi);
        const cx = gesture.mid.x - r.left, cy = gesture.mid.y - r.top;
        const v = gesture.view;
        setView({ x: cx - ((cx - v.x) * z) / v.zoom, y: cy - ((cy - v.y) * z) / v.zoom, zoom: z });
        return;
      }
      const dx = e.clientX - gesture.x, dy = e.clientY - gesture.y;
      if (!gesture.moved && Math.hypot(dx, dy) < 4) return;
      if (gesture.kind === "pan") {
        if (!on("pan", true)) return;
        start(e);
        el.setAttribute("data-panning", "");
        setView({ x: gesture.view.x + dx, y: gesture.view.y + dy, zoom: view.zoom });
      } else if (gesture.kind === "node") {
        if (!on("draggable", false)) return;
        start(e);
        gesture.el.setAttribute("data-dragging", "");
        dragged.set(gesture.id, { x: gesture.from.x + dx / view.zoom, y: gesture.from.y + dy / view.zoom });
        place();
      }
    });
    const release = (e) => {
      pointers.delete(e.pointerId);
      const g = gesture;
      if (!g || pointers.size) { if (g?.kind === "pinch" && pointers.size === 1) gesture = null; return; }
      gesture = null;
      el.removeAttribute("data-panning");
      if (e.type === "pointercancel") return;
      if (g.kind === "pan" && !g.moved) select(null);
      else if (g.kind === "node" && !g.moved) select(g.id);
      else if (g.kind === "node") {
        g.el.removeAttribute("data-dragging");
        moved(g.id);
      }
    };
    el.addEventListener("pointerup", release);
    el.addEventListener("pointercancel", release);
    el.addEventListener("dblclick", (e) => {
      const nodeEl = e.target.closest("[data-flow-node]");
      if (nodeEl && nodeLayer.contains(nodeEl) && !interactive(e.target)) select(nodeEl.getAttribute("data-flow-node"), true);
    });

    // After a drag: the nodes, with the dragged one's new position, go to `bindNodes`.
    function moved(id) {
      const p = dragged.get(id);
      if (!p) return;
      const all = Array.isArray(props.nodes) ? props.nodes : [];
      const node = byId.get(id);
      if (cfg.bindNodes) {
        const [ox, oy] = origin();
        const g = geo.get(id) || { w: 0, h: 0 };
        // Laid-out positions are corners; saved positions follow `nodeOrigin`.
        const laid = laidOut.has(id);
        const position = { x: Math.round(p.x + (laid ? ox * g.w : 0)), y: Math.round(p.y + (laid ? oy * g.h : 0)) };
        api.set(cfg.bindNodes, all.map((n) => (isObj(n) && str(n.id) === id ? { ...n, position } : n)));
      }
      api.emit("nodemove", { id: node?.id ?? id });
    }

    el.addEventListener("wheel", (e) => {
      if (!on("zoom", true) && !on("pan", true)) return;
      const r = el.getBoundingClientRect();
      const scale = e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? el.clientHeight : 1;
      const zooming = e.ctrlKey || e.metaKey || opt("wheel", "zoom") !== "pan";
      if (zooming && on("zoom", true)) {
        e.preventDefault();
        zoomBy(Math.exp((-e.deltaY * scale * (e.ctrlKey ? 0.01 : 0.002))), e.clientX - r.left, e.clientY - r.top);
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
        select(id, e.key === "Enter" && str(selected) === id);
      } else if (e.key === "Escape" && selected !== null && selected !== undefined) {
        select(null);
      } else if ((e.key === "+" || e.key === "=") && on("zoom", true)) {
        zoomBy(1.2, undefined, undefined, true);
      } else if (e.key === "-" && on("zoom", true)) {
        zoomBy(1 / 1.2, undefined, undefined, true);
      } else if (arrows[e.key]) {
        e.preventDefault();
        const [dx, dy] = arrows[e.key];
        const step = e.shiftKey ? 50 : 10;
        if (id !== null && on("draggable", false) && byId.has(id)) {
          const p = relative(byId.get(id));
          dragged.set(id, { x: p.x + dx * step, y: p.y + dy * step });
          place();
          clearTimeout(nodeEl.__moveTimer);
          nodeEl.__moveTimer = setTimeout(() => moved(id), 300);
        } else if (on("pan", true)) setView({ x: view.x - dx * step * 4, y: view.y - dy * step * 4, zoom: view.zoom });
      } else return;
    });

    refresh(true);
    return {
      update(state) {
        if (cfg.reactive) props = resolve(cfg.props);
        if (cfg.bind) {
          const v = state[cfg.bind];
          if (str(v) !== str(selected)) selected = v ?? null;
        }
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
})();
