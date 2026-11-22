// irender-ui-kit: `ui-field-map`. Places are Web Mercator: at zoom z the world is 256 × 2^z px
// wide. Tiles are placed and scaled for the zoom, pins are moved over them, and areas and
// lines are SVG.
(() => {
  const B = (window.__irenderBehaviors ||= {});
  const K = window.__irenderKit, D = window.__irenderDevice, G = window.__irenderGeo, L = (t, v) => window.__irender.label(t, v);
  const { make, icon, isObj, blank } = K;
  const NS = "http://www.w3.org/2000/svg";
  const svg = (tag, attrs, ...kids) => {
    const e = document.createElementNS(NS, tag);
    for (const [k, v] of Object.entries(attrs ?? {})) if (v !== null && v !== undefined && v !== false) typeof v === "function" ? e.addEventListener(k, v) : e.setAttribute(k, v === true ? "" : v);
    e.append(...kids);
    return e;
  };
  const list = (v) => (Array.isArray(v) ? v : []);
  const css = (c, d) => (typeof c === "string" && /^[#\w(),.%\s/-]+$/.test(c) ? c : d);
  const SIZE = 256, MAXLAT = 85.0511;
  const toX = (lng) => (lng + 180) / 360, toY = (lat) => {
    const s = Math.sin((K.clamp(lat, -MAXLAT, MAXLAT) * Math.PI) / 180);
    return 0.5 - Math.log((1 + s) / (1 - s)) / (4 * Math.PI);
  };
  const toLng = (x) => x * 360 - 180, toLat = (y) => (Math.atan(Math.sinh(Math.PI * (1 - 2 * y))) * 180) / Math.PI;
  const round = (p) => ({ lat: +p.lat.toFixed(6), lng: +p.lng.toFixed(6) });
  B["ui-field-map"] = (el, api) => {
    const cfg = api.config, lo = cfg.minZoom ?? 1, hi = cfg.maxZoom ?? 19;
    const tiles = make("div", { "data-map-tiles": "", "aria-hidden": "true" }), over = svg("svg", { "data-map-over": "" }), pins = make("div", { "data-map-pins": "" });
    const pane = make("div", { "data-map-pane": "", tabindex: 0, role: "application", "aria-label": L("Map: the arrow keys move it, + and - zoom") }, tiles, over, pins);
    const bt = (name, label, go) => K.button({ "aria-label": label, title: label, click: go }, icon(name, 17));
    const me = cfg.locate ? bt("pin", L("Where I am"), locate) : null;
    const tools = make("div", { "data-map-tools": "" }, bt("plus", L("Zoom in"), () => zoomTo(zoom + 1)), bt("minus", L("Zoom out"), () => zoomTo(zoom - 1)), bt("fit", L("Show everything"), () => fit(true)), me);
    const hint = make("div", { "data-map-hint": "", role: "status", hidden: true }), scale = make("div", { "data-map-scale": "", "aria-hidden": "true" });
    const kinds = { marker: [L("Place a marker"), "pin"], line: [L("Draw a line"), "pencil"], polygon: [L("Draw an area"), "polygon"], circle: [L("Draw a circle"), "circle"] };
    const drawTools = list(cfg.draw).map((k) => K.button({ "data-draw": k, "aria-label": kinds[k][0], title: kinds[k][0], "aria-pressed": "false", click: () => pick(tool === k ? null : k) }, icon(kinds[k][1], 17)));
    const done = K.button({ hidden: true, click: () => finish() }, L("Done")), drop = K.button({ hidden: true, "aria-label": L("Delete what's selected"), title: L("Delete what's selected"), click: remove }, icon("trash", 16));
    el.append(pane, tools, scale, hint);
    if (drawTools.length) el.append(make("div", { "data-map-draw": "" }, drawTools, done, drop));
    if (cfg.attribution) el.append(make("div", { "data-map-by": "" }, cfg.attribution));
    let cx = 0.5, cy = 0.4, zoom = 2, W = 0, H = 0, ready = false, seen = false, own = null, tool = null, sketch = null, sel = null, popFor = null, pop = null, zoomed = false;
    const world = () => SIZE * 2 ** zoom;
    const at = (p) => ({ x: (toX(p.lng) - cx) * world() + W / 2, y: (toY(p.lat) - cy) * world() + H / 2 });
    const from = (x, y) => ({ lat: toLat(cy + (y - H / 2) / world()), lng: ((((toLng(cx + (x - W / 2) / world()) + 180) % 360) + 360) % 360) - 180 });
    const metres = (lat) => (40075016.686 * Math.cos((lat * Math.PI) / 180)) / world();
    const drawn = () => (cfg.bindDrawn ? list(api.state[cfg.bindDrawn]).filter(isObj) : []);
    const picked = () => (cfg.bindSelected ? api.state[cfg.bindSelected] ?? null : sel);
    const ring = (c, r) => Array.from({ length: 49 }, (_, i) => {
      const t = (i / 48) * 2 * Math.PI, m = 40075016.686 * Math.cos((c.lat * Math.PI) / 180), k = r / m;
      return [toLat(toY(c.lat) + Math.sin(t) * k), toLng(toX(c.lng) + Math.cos(t) * k)];
    });
    // ---- what's on it, as it is now
    let data = { markers: [], areas: [], lines: [], me: null };
    let sig = null;
    function gather() {
      const mk = (m, i, extra) => ({ ...m, ...G.point(m), id: m.id ?? `m${i}`, ...extra });
      const markers = list(K.value(api, cfg.markers)).filter((m) => G.point(m)).map((m, i) => mk(m, i));
      const areas = list(K.value(api, cfg.areas)).filter(isObj).map((a, i) => ({ ...a, id: a.id ?? `a${i}` }));
      const lines = list(K.value(api, cfg.lines)).filter((l) => isObj(l) || Array.isArray(l)).map((l, i) => (Array.isArray(l) ? { id: `l${i}`, points: l } : { ...l, id: l.id ?? `l${i}` }));
      for (const d of drawn()) {
        const p = list(d.points);
        if (d.type === "marker" && G.point(p[0])) markers.push({ ...d, ...G.point(p[0]), drawn: true });
        else if (d.type === "circle" && G.point(p[0])) areas.push({ ...d, points: undefined, ...G.point(p[0]), drawn: true });
        else if (d.type === "line") lines.push({ ...d, drawn: true });
        else if (d.type === "polygon") areas.push({ ...d, drawn: true });
      }
      data = { markers, areas, lines, me: K.value(api, cfg.position) ?? own };
      return JSON.stringify([data, picked(), tool, sketch]);
    }
    const areaPoints = (a) => (Array.isArray(a.points) ? a.points.map(G.point).filter(Boolean) : G.point(a) ? ring(G.point(a), Number(a.radius) || 100).map(G.point) : []);
    function everything() {
      return [...data.markers, ...data.areas.flatMap(areaPoints), ...data.lines.flatMap((l) => list(l.points).map(G.point).filter(Boolean)), ...(G.point(data.me) ? [G.point(data.me)] : [])];
    }
    function bounds(ps, most = 17) {
      if (!ps.length || !W) return;
      const xs = ps.map((p) => toX(p.lng)), ys = ps.map((p) => toY(p.lat)), x0 = Math.min(...xs), x1 = Math.max(...xs), y0 = Math.min(...ys), y1 = Math.max(...ys);
      cx = (x0 + x1) / 2;
      cy = (y0 + y1) / 2;
      const pad = Math.min(W, H) > 220 ? 100 : 30;
      zoom = K.clamp(Math.min(Math.log2(Math.max(40, W - pad) / SIZE / Math.max(x1 - x0, 1e-9)), Math.log2(Math.max(40, H - pad) / SIZE / Math.max(y1 - y0, 1e-9)), most), lo, Math.min(hi, most));
    }
    function fit(tell) {
      const ps = everything();
      if (ps.length) bounds(ps, ps.length === 1 ? 15 : 17);
      place();
      if (tell) moved();
    }
    // ---- drawing
    function place() {
      if (!W) return;
      cy = K.clamp(cy, 0, 1);
      cx = ((cx % 1) + 1) % 1;
      drawTiles();
      over.replaceChildren(...shapes());
      placePins();
      drawScale();
    }
    const imgs = new Map();
    function drawTiles() {
      if (cfg.tiles === "none" || !seen) return;
      const tz = K.clamp(Math.round(zoom), 0, 19), n = 2 ** tz, k = 2 ** (zoom - tz), px = SIZE * k;
      const ox = cx * n * px - W / 2, oy = cy * n * px - H / 2, want = new Set();
      for (let y = Math.max(0, Math.floor(oy / px)); y <= Math.min(n - 1, Math.floor((oy + H) / px)); y++) {
        for (let x = Math.floor(ox / px); x <= Math.floor((ox + W) / px); x++) {
          const id = `${tz}/${x}/${y}`;
          want.add(id);
          let i = imgs.get(id);
          if (!i) {
            const src = cfg.tiles.replace("{z}", tz).replace("{x}", ((x % n) + n) % n).replace("{y}", y);
            i = make("img", { src, alt: "", draggable: "false", referrerpolicy: "strict-origin-when-cross-origin", error: () => (i.style.visibility = "hidden") });
            imgs.set(id, i);
            tiles.append(i);
          }
          i.style.transform = `translate(${x * px - ox}px, ${y * px - oy}px) scale(${k * 1.002})`;
        }
      }
      for (const [id, i] of imgs) if (!want.has(id)) {
        // The last zoom's are kept under the new ones until those are there.
        i.remove();
        imgs.delete(id);
      }
    }
    const d = (ps, close) => ps.map((p, i) => {
      const q = at(p);
      return `${i ? "L" : "M"}${q.x.toFixed(1)} ${q.y.toFixed(1)}`;
    }).join("") + (close ? "Z" : "");
    function shapes() {
      const out = [], now = picked();
      const shape = (s, ps, close) => {
        if (!ps.length) return;
        const on = String(now) === String(s.id), c = css(s.color, "var(--color-primary)");
        const p = svg("path", { "data-map-shape": s.id, d: d(ps, close), fill: close ? c : "none", "fill-opacity": close ? (on ? 0.32 : 0.18) : null, stroke: c, "stroke-width": (Number(s.width) || 3) + (on ? 1.5 : 0), "stroke-linejoin": "round", "stroke-linecap": "round", "stroke-dasharray": s.dashed ? "8 7" : null, "vector-effect": "non-scaling-stroke" });
        if (!close) out.push(svg("path", { "data-map-shape": s.id, d: d(ps), fill: "none", stroke: "transparent", "stroke-width": 16 }));
        if (s.label) p.append(svg("title", null, String(s.label)));
        out.push(p);
      };
      for (const a of data.areas) shape(a, areaPoints(a), true);
      for (const l of data.lines) shape(l, list(l.points).map(G.point).filter(Boolean), false);
      const m = G.point(data.me);
      if (m && Number(data.me.accuracy) > 0) out.push(svg("path", { d: d(ring(m, Number(data.me.accuracy)).map(G.point), true), fill: "#2563eb", "fill-opacity": 0.14, stroke: "#2563eb", "stroke-opacity": 0.4 }));
      if (sketch) {
        const ps = sketch.type === "circle" ? ring(sketch.points[0], sketch.radius || 1).map(G.point) : [...sketch.points, ...(sketch.next ? [sketch.next] : [])];
        out.push(svg("path", { d: d(ps, sketch.type !== "line"), fill: sketch.type === "line" ? "none" : "var(--color-primary)", "fill-opacity": 0.15, stroke: "var(--color-primary)", "stroke-width": 2.5, "stroke-dasharray": "6 5" }));
        if (sketch.type !== "circle") for (const p of sketch.points) out.push(svg("circle", { cx: at(p).x, cy: at(p).y, r: 5, fill: "#fff", stroke: "var(--color-primary)", "stroke-width": 2 }));
      }
      // The corners of what's drawn and selected can be dragged.
      const s = drawn().find((x) => String(x.id) === String(now));
      if (s && !tool && s.type !== "marker") list(s.points).map(G.point).forEach((p, i) => p && out.push(svg("circle", { "data-map-handle": i, "data-map-shape": s.id, cx: at(p).x, cy: at(p).y, r: 7, fill: "#fff", stroke: "var(--color-primary)", "stroke-width": 2.5, style: "cursor:move" })));
      return out;
    }
    const made = new Map();
    function placePins() {
      const keep = new Set(), now = picked();
      const put = (id, build, p) => {
        keep.add(id);
        let e = made.get(id);
        if (!e) {
          e = build();
          made.set(id, e);
          pins.append(e);
        }
        e.style.transform = `translate(${p.x.toFixed(1)}px, ${p.y.toFixed(1)}px)`;
        return e;
      };
      const pin = (m) => {
        const label = m.label ?? L("Marker");
        const e = put(`pin:${JSON.stringify([m.id, m.label, m.color, m.glyph])}`, () => make("button", { type: "button", "data-map-pin": m.id, "aria-label": label, title: m.label ?? null, style: css(m.color, null) ? `--pin:${m.color}` : null, click: () => select(m.id) }, svg("svg", { viewBox: "0 0 30 38", "aria-hidden": "true" }, svg("path", { d: "M15 37S2 22.5 2 14.5a13 13 0 0 1 26 0C28 22.5 15 37 15 37z", fill: "currentColor", stroke: "#fff", "stroke-width": 1.5 }), svg("circle", { cx: 15, cy: 14.5, r: blank(m.glyph) ? 5 : 0, fill: "#fff" })), blank(m.glyph) ? null : make("b", null, [...String(m.glyph)].slice(0, 2).join(""))), at(m));
        e.setAttribute("aria-pressed", String(String(now) === String(m.id)));
      };
      if (cfg.cluster && zoom < Math.min(hi, 17)) {
        const left = data.markers.map((m) => ({ m, p: at(m) })).filter(({ p }) => p.x > -80 && p.y > -80 && p.x < W + 80 && p.y < H + 80);
        while (left.length) {
          const a = left.shift(), near = [a];
          if (String(now) !== String(a.m.id)) for (let i = left.length - 1; i >= 0; i--) if (Math.hypot(left[i].p.x - a.p.x, left[i].p.y - a.p.y) < 46 && String(now) !== String(left[i].m.id)) near.push(...left.splice(i, 1));
          if (near.length === 1) pin(a.m);
          else {
            const ms = near.map((n) => n.m), p = { x: near.reduce((s, n) => s + n.p.x, 0) / near.length, y: near.reduce((s, n) => s + n.p.y, 0) / near.length };
            put(`cluster:${ms.map((m) => m.id).sort().join()}`, () => make("button", { type: "button", "data-map-cluster": ms.length, "aria-label": L("{n} markers here: zoom in", { n: ms.length }), click: () => { const z = zoom; bounds(ms, hi); if (zoom <= z + 0.2) zoom = Math.min(hi, z + 2); place(); moved(); } }, String(ms.length)), p);
          }
        }
      } else data.markers.forEach(pin);
      const m = G.point(data.me);
      if (m) put("me", () => make("div", { "data-map-me": "", role: "img", "aria-label": L("You are here") }), at(m));
      for (const [id, e] of made) if (!keep.has(id)) {
        const had = e.contains(document.activeElement);
        e.remove();
        made.delete(id);
        if (had) pane.focus({ preventScroll: true });
      }
      const s = data.markers.find((x) => String(x.id) === String(now) && (x.label || x.text));
      const key = s ? JSON.stringify([s.id, s.label, s.text]) : null;
      if (key !== popFor) {
        pop?.remove();
        pop = s ? make("div", { "data-map-pop": "", role: "status" }, s.label ? make("strong", null, s.label) : null, s.text ? make("span", null, s.text) : null) : null;
        if (pop) pins.append(pop);
        popFor = key;
      }
      if (pop) {
        const p = at(s), w = pop.offsetWidth, h = pop.offsetHeight, up = p.y - 48 - h > 4;
        pop.style.transform = `translate(${K.clamp(p.x - w / 2, 6, Math.max(6, W - w - 6))}px, ${up ? p.y - 48 - h : p.y + 10}px)`;
      }
    }
    function drawScale() {
      const per = metres(toLat(cy)), most = per * 90, p = 10 ** Math.floor(Math.log10(most)), n = [5, 2, 1].map((k) => k * p).find((k) => k <= most) ?? p;
      scale.style.width = `${n / per}px`;
      scale.textContent = G.far(n);
    }
    // ---- moving
    let told = 0, was = null;
    function moved() {
      clearTimeout(told);
      told = setTimeout(() => {
        const nw = from(0, 0), se = from(W, H), c = round(from(W / 2, H / 2));
        const v = { ...c, zoom: +zoom.toFixed(2), bounds: [[+se.lat.toFixed(6), +nw.lng.toFixed(6)], [+nw.lat.toFixed(6), +se.lng.toFixed(6)]] };
        if (cfg.bindView && JSON.stringify(api.state[cfg.bindView]) !== JSON.stringify(v)) api.set(cfg.bindView, v);
        api.emit("move", v);
      }, 120);
    }
    function zoomTo(z, x = W / 2, y = H / 2) {
      z = K.clamp(z, lo, hi);
      const wx = cx + (x - W / 2) / world(), wy = cy + (y - H / 2) / world();
      zoom = z;
      cx = wx - (x - W / 2) / world();
      cy = wy - (y - H / 2) / world();
      place();
      moved();
    }
    const pts = new Map();
    let pan = null, pinch = null, far = false, drag = null;
    const rel = (e) => {
      const b = pane.getBoundingClientRect();
      return { x: e.clientX - b.left, y: e.clientY - b.top };
    };
    pane.addEventListener("pointerdown", (e) => {
      if (e.button !== 0 || e.target.closest("[data-map-cluster]")) return;
      const p = rel(e);
      far = false;
      const mk = e.target.closest("[data-map-pin]");
      const dm = mk && drawn().find((x) => String(x.id) === mk.getAttribute("data-map-pin"));
      if (mk && !dm) return;
      const handle = e.target.closest?.("[data-map-handle]");
      if (tool === "circle") {
        K.capture(pane, e);
        sketch = { type: "circle", points: [round(from(p.x, p.y))], radius: 0 };
        drag = { circle: true };
        return;
      }
      if (handle || (dm && String(picked()) === String(dm.id))) {
        drag = { id: handle ? handle.getAttribute("data-map-shape") : dm.id, i: handle ? Number(handle.getAttribute("data-map-handle")) : 0 };
        K.capture(pane, e);
        return;
      }
      pts.set(e.pointerId, p);
      if (pts.size === 2) {
        const [a, b] = [...pts.values()];
        pinch = { d: Math.hypot(a.x - b.x, a.y - b.y), zoom };
        pan = null;
      } else pan = { x: p.x, y: p.y, cx, cy };
    });
    pane.addEventListener("pointermove", (e) => {
      const p = rel(e);
      if (drag?.circle) {
        sketch.radius = Math.round(G.distance(sketch.points[0], from(p.x, p.y)));
        far = true;
        return place();
      }
      if (drag) {
        far = true;
        const q = round(from(p.x, p.y));
        api.set(cfg.bindDrawn, drawn().map((s) => (String(s.id) === String(drag.id) ? { ...s, points: s.points.map((o, i) => (i === drag.i ? [q.lat, q.lng] : o)) } : s)));
        return;
      }
      if (sketch && sketch.type !== "circle" && e.pointerType === "mouse") (sketch.next = from(p.x, p.y)), !pan && place();
      if (!pts.has(e.pointerId)) return;
      pts.set(e.pointerId, p);
      if (pinch && pts.size === 2) {
        const [a, b] = [...pts.values()];
        far = true;
        zoomTo(pinch.zoom + Math.log2(Math.max(10, Math.hypot(a.x - b.x, a.y - b.y)) / Math.max(10, pinch.d)), (a.x + b.x) / 2, (a.y + b.y) / 2);
      } else if (pan) {
        if (!far && Math.hypot(p.x - pan.x, p.y - pan.y) < 5) return;
        if (!far) K.capture(pane, e), pane.setAttribute("data-moving", "");
        far = true;
        cx = pan.cx - (p.x - pan.x) / world();
        cy = pan.cy - (p.y - pan.y) / world();
        place();
      }
    });
    const up = (e) => {
      pts.delete(e.pointerId);
      if (pts.size < 2) pinch = null;
      pane.removeAttribute("data-moving");
      if (drag?.circle) {
        drag = null;
        if (sketch.radius > 1) return finish();
        sketch = null;
        return place();
      }
      if (drag) {
        drag = null;
        api.emit("draw", { shapes: drawn() });
        return;
      }
      if (pan && far) moved();
      pan = null;
    };
    pane.addEventListener("pointerup", up);
    pane.addEventListener("pointercancel", up);
    pane.addEventListener("click", (e) => {
      if (far) return e.stopPropagation(), e.preventDefault(), (far = false);
      if (e.target.closest("button")) return;
      const p = rel(e), q = round(from(p.x, p.y));
      if (tool === "marker") return commit({ type: "marker", points: [[q.lat, q.lng]] });
      if (tool === "line" || tool === "polygon") {
        sketch ??= { type: tool, points: [] };
        sketch.points.push(q);
        sketch.next = null;
        return draw();
      }
      const s = e.target.closest?.("[data-map-shape]");
      if (s) return select(s.getAttribute("data-map-shape"));
      select(null);
      if (cfg.bindPoint) api.set(cfg.bindPoint, q);
      api.emit("point", q);
    }, true);
    pane.addEventListener("dblclick", (e) => {
      if (e.target.closest("button")) return;
      e.preventDefault();
      if (sketch) {
        // Its second click put the same corner again.
        if (sketch.points.length > 2) sketch.points.pop();
        return finish();
      }
      if (!tool) zoomTo(zoom + 1, rel(e).x, rel(e).y);
    });
    pane.addEventListener("wheel", (e) => {
      if (!cfg.wheel && !e.ctrlKey && !e.metaKey) {
        if (!zoomed) flash(L("Hold Ctrl or ⌘ to zoom with the wheel"));
        return;
      }
      zoomed = true;
      e.preventDefault();
      zoomTo(zoom - e.deltaY * (e.ctrlKey ? 0.01 : 0.003), rel(e).x, rel(e).y);
    }, { passive: false });
    pane.addEventListener("keydown", (e) => {
      const dir = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] }[e.key];
      if (dir && e.target === pane) (cx += (dir[0] * 80) / world()), (cy += (dir[1] * 80) / world()), place(), moved();
      else if (e.key === "+" || e.key === "=") zoomTo(zoom + 1);
      else if (e.key === "-" || e.key === "_") zoomTo(zoom - 1);
      else if (e.key === "Enter" && sketch) finish();
      else if (e.key === "Escape" && (tool || sketch)) pick(null);
      else if ((e.key === "Delete" || e.key === "Backspace") && !drop.hidden) remove();
      else return;
      e.preventDefault();
    });
    let flashing = 0;
    function flash(text) {
      hint.textContent = text;
      hint.hidden = false;
      clearTimeout(flashing);
      flashing = setTimeout(tip, 1600);
    }
    function tip() {
      const t = tool === "marker" ? L("Click where it goes") : tool === "circle" ? L("Drag from its middle") : tool ? (sketch ? L("Click for each point, then Done") : L("Click where it starts")) : "";
      hint.textContent = t;
      hint.hidden = !t;
    }
    // ---- picking and drawing
    function select(id) {
      const v = id === null ? null : [...data.markers, ...data.areas, ...data.lines].find((x) => String(x.id) === String(id))?.id ?? null;
      if (String(v) === String(picked())) return;
      sel = v;
      if (cfg.bindSelected) api.set(cfg.bindSelected, v);
      if (v !== null) api.emit("select", { id: v });
      draw();
    }
    function pick(t) {
      tool = t;
      sketch = null;
      el.toggleAttribute("data-tool", !!t);
      for (const b of drawTools) b.setAttribute("aria-pressed", String(b.getAttribute("data-draw") === t));
      tip();
      draw();
    }
    function commit(s) {
      const v = { id: K.id(), ...s };
      api.set(cfg.bindDrawn, [...drawn(), v]);
      api.emit("draw", { shape: v, shapes: drawn() });
      pick(null);
      select(v.id);
    }
    function finish() {
      const s = sketch, least = s?.type === "polygon" ? 3 : 2;
      if (!s) return;
      if (s.type === "circle") return commit({ type: "circle", points: [[s.points[0].lat, s.points[0].lng]], radius: s.radius });
      if (s.points.length < least) return flash(L("It needs at least {n} points", { n: least }));
      commit({ type: s.type, points: s.points.map((p) => [p.lat, p.lng]) });
    }
    function remove() {
      const id = picked();
      api.set(cfg.bindDrawn, drawn().filter((s) => String(s.id) !== String(id)));
      select(null);
      api.emit("draw", { shapes: drawn() });
      draw();
    }
    async function locate() {
      me.disabled = true;
      try {
        own = await G.find({});
        cx = toX(own.lng);
        cy = toY(own.lat);
        zoom = Math.max(zoom, 15);
        draw();
        moved();
        api.emit("locate", own);
      } catch (e) {
        flash(D.explain(e, "geolocation") ?? "");
      }
      me.disabled = false;
    }
    let centre = null;
    function draw() {
      const c = JSON.stringify([K.value(api, cfg.center) ?? null, K.value(api, cfg.zoom) ?? null]);
      const fresh = c !== centre;
      if (fresh && W) {
        const [p, z] = JSON.parse(c), q = G.point(p);
        if (q) (cx = toX(q.lng)), (cy = toY(q.lat));
        if (!blank(z) && Number.isFinite(Number(z))) zoom = K.clamp(Number(z), lo, hi);
        else if (q && centre === null) zoom = 13;
        centre = c;
      }
      const now = gather();
      if (now === sig && !fresh) return;
      sig = now;
      if (W && !ready) {
        ready = true;
        if (cfg.fit) return fit(false), mode(), moved();
        moved();
      }
      place();
      mode();
    }
    function mode() {
      const id = picked();
      done.hidden = !(sketch && sketch.type !== "circle");
      drop.hidden = !drawn().some((s) => String(s.id) === String(id));
      tip();
    }
    const size = () => {
      const w = pane.clientWidth, h = pane.clientHeight;
      if (w === W && h === H) return;
      W = w;
      H = h;
      sig = null;
      draw();
    };
    new ResizeObserver(size).observe(pane);
    new IntersectionObserver((es, o) => es.some((e) => e.isIntersecting) && ((seen = true), o.disconnect(), place()), { rootMargin: "200px" }).observe(el);
    size();
    draw();
    return { update: draw };
  };
})();
