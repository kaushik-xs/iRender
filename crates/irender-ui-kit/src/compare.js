// irender-ui-kit: the `ui-compare-slider` behaviour. Each divider is a range input over
// the picture; this sets `--pos` (`--pos2`, `--pos3`) as one moves (and from `bind`),
// says the split for screen readers ("Before 40%, after 60%"), fades a side's label as
// the divider covers it, and with `hover` lets a mouse move the divider without pressing.
// With `mode: fade` the second picture fades in over the first instead (CSS, from `--pos`).
// With three or four pictures, or `zoom`, dragging is this script's: the nearest divider
// moves, and once zoomed, a drag away from the dividers moves the view.
(window.__irenderBehaviors ||= {})["ui-compare-slider"] = (el, api) => {
  const cfg = api.config, K = window.__irenderKit, L = (t, v) => window.__irender.label(t, v);
  const frame = el.querySelector("[data-cmp-frame]"), inputs = [...frame.querySelectorAll(":scope > input")];
  const many = inputs.length > 1, own = frame.hasAttribute("data-own-drag");
  const clamp = (n, lo = 0, hi = 100) => Math.min(hi, Math.max(lo, n));
  const vars = inputs.map((_, k) => (k ? `--pos${k + 1}` : "--pos"));
  const places = () => inputs.map((i) => Number(i.value));

  // Divider `k` to `v`, between its neighbours; `write` puts it in state and says so.
  function show(k, v, write) {
    v = Math.round(Number(v));
    if (!Number.isFinite(v)) return;
    const p = places();
    v = clamp(v, k > 0 ? p[k - 1] : 0, k < p.length - 1 ? p[k + 1] : 100);
    const input = inputs[k];
    el.style.setProperty(vars[k], String(v));
    if (Number(input.value) !== v) input.value = String(v);
    const fade = el.hasAttribute("data-fade");
    // Labels: "After {b}% faded in", "Before {a}%, after {b}%", "{a}% to {b}%"
    if (many) input.setAttribute("aria-valuetext", `${v}%`);
    else input.setAttribute("aria-valuetext", fade ? L("After {b}% faded in", { b: v }) : L("Before {a}%, after {b}%", { a: v, b: 100 - v }));
    el.setAttribute("data-near", fade || many ? "" : v < 12 ? "before" : v > 88 ? "after" : "");
    if (!write) return;
    const now = many ? places() : v;
    if (cfg.bind && JSON.stringify(api.state[cfg.bind]) !== JSON.stringify(now)) api.set(cfg.bind, now);
    api.emit("change", { value: now });
  }
  inputs.forEach((input, k) => input.addEventListener("input", () => show(k, input.value, true)));

  // Where a pointer is, 0–100 along the dividers' way.
  const rtl = () => getComputedStyle(el).direction === "rtl";
  function along(e) {
    const r = frame.getBoundingClientRect();
    return cfg.vertical ? ((e.clientY - r.top) / r.height) * 100 : ((rtl() ? r.right - e.clientX : e.clientX - r.left) / r.width) * 100;
  }
  const nearest = (at) => places().reduce((best, p, k, all) => (Math.abs(p - at) < Math.abs(all[best] - at) || (p === all[best] && at > p) ? k : best), 0);

  if (cfg.hover) {
    frame.addEventListener("pointermove", (e) => {
      if (e.pointerType !== "mouse" || e.buttons || zoom.s > 1) return;
      show(0, along(e), true);
    });
  }

  // ---- zoom: one scale and offset for every picture, kept covering the frame
  const zoom = { s: 1, x: 0, y: 0 }, most = Number(cfg.zoom) || 1;
  let bar = null, out = null, plus = null, minus = null, reset = null;
  function place() {
    const r = frame.getBoundingClientRect();
    zoom.s = clamp(zoom.s, 1, most);
    zoom.x = clamp(zoom.x, r.width - r.width * zoom.s, 0);
    zoom.y = clamp(zoom.y, r.height - r.height * zoom.s, 0);
    frame.style.setProperty("--zs", String(zoom.s));
    frame.style.setProperty("--zx", String(zoom.x));
    frame.style.setProperty("--zy", String(zoom.y));
    frame.toggleAttribute("data-zoomed", zoom.s > 1);
    if (out) {
      out.textContent = `${Math.round(zoom.s * 100)}%`;
      plus.disabled = zoom.s >= most;
      minus.disabled = reset.disabled = zoom.s <= 1;
    }
  }
  // To `s`, keeping the point (`px`, `py` in the frame; its middle by default) where it is.
  function zoomTo(s, px, py, say = true) {
    if (!cfg.zoom) return;
    const r = frame.getBoundingClientRect();
    px ??= r.width / 2;
    py ??= r.height / 2;
    const to = clamp(s, 1, most), k = to / zoom.s;
    zoom.x = px - (px - zoom.x) * k;
    zoom.y = py - (py - zoom.y) * k;
    zoom.s = to;
    place();
    if (say) said();
  }
  // In state (`bindZoom`) and to listeners, once it settles.
  function said() {
    const z = Math.round(zoom.s * 100) / 100;
    if (cfg.bindZoom && api.state[cfg.bindZoom] !== z) api.set(cfg.bindZoom, z);
    api.emit("zoom", { zoom: z });
  }
  if (cfg.zoom) {
    // Labels: "Zoom in", "Zoom out", "Reset zoom", "Zoom"
    minus = K.button({ "data-cmp-zoom": "out", "aria-label": L("Zoom out"), title: L("Zoom out"), click: () => zoomTo(zoom.s / 1.5) }, K.icon("zoomout"));
    out = K.make("output", { "aria-live": "polite", "aria-label": L("Zoom") }, "100%");
    plus = K.button({ "data-cmp-zoom": "in", "aria-label": L("Zoom in"), title: L("Zoom in"), click: () => zoomTo(zoom.s * 1.5) }, K.icon("zoomin"));
    reset = K.button({ "data-cmp-zoom": "reset", "aria-label": L("Reset zoom"), title: L("Reset zoom"), click: () => zoomTo(1) }, K.icon("fit"));
    bar = K.make("div", { "data-cmp-zoombar": "", role: "group", "aria-label": L("Zoom") }, minus, out, plus, reset);
    frame.append(bar);
    place();
    frame.addEventListener("wheel", (e) => {
      // Ctrl or ⌘ and the wheel (a trackpad's pinch comes as one); the plain wheel scrolls the page.
      if (!(e.ctrlKey || e.metaKey)) return;
      e.preventDefault();
      const r = frame.getBoundingClientRect();
      zoomTo(zoom.s * Math.exp(-e.deltaY * 0.01), e.clientX - r.left, e.clientY - r.top);
    }, { passive: false });
    frame.addEventListener("dblclick", (e) => {
      if (e.target.closest("[data-cmp-zoombar]")) return;
      const r = frame.getBoundingClientRect();
      zoomTo(zoom.s > 1 ? 1 : Math.min(2, most), e.clientX - r.left, e.clientY - r.top);
    });
    for (const input of inputs) {
      input.addEventListener("keydown", (e) => {
        const k = e.key === "+" || e.key === "=" ? 1.5 : e.key === "-" || e.key === "_" ? 1 / 1.5 : e.key === "0" ? 0 : null;
        if (k === null || e.ctrlKey || e.metaKey || e.altKey) return;
        e.preventDefault();
        zoomTo(k ? zoom.s * k : 1);
      });
    }
    new ResizeObserver(() => place()).observe(frame);
  }

  // ---- dragging, when it's this script's: a divider, the view, or two fingers' pinch
  if (own) {
    const down = new Map();
    let drag = null;
    frame.addEventListener("pointerdown", (e) => {
      if (e.button !== 0 || e.target.closest("[data-cmp-zoombar]")) return;
      down.set(e.pointerId, { x: e.clientX, y: e.clientY });
      try { frame.setPointerCapture(e.pointerId); } catch { /* not a pointer it tracks */ }
      if (down.size === 2 && cfg.zoom) {
        const [a, b] = [...down.values()], r = frame.getBoundingClientRect();
        drag = { pinch: true, d: Math.hypot(a.x - b.x, a.y - b.y), s: zoom.s, mx: (a.x + b.x) / 2 - r.left, my: (a.y + b.y) / 2 - r.top };
        return;
      }
      if (down.size > 1) return;
      const at = along(e), k = nearest(at), r = frame.getBoundingClientRect();
      // Zoomed, a press near a divider (18 px) takes it; elsewhere it moves the view.
      const gap = (Math.abs(places()[k] - at) / 100) * (cfg.vertical ? r.height : r.width);
      if (zoom.s > 1 && gap > 18) {
        drag = { pan: true, x: e.clientX, y: e.clientY, zx: zoom.x, zy: zoom.y };
        frame.setAttribute("data-panning", "");
      } else {
        drag = { k };
        inputs[k].focus({ preventScroll: true });
        show(k, at, true);
      }
      e.preventDefault();
    });
    frame.addEventListener("pointermove", (e) => {
      if (!down.has(e.pointerId) || !drag) return;
      down.set(e.pointerId, { x: e.clientX, y: e.clientY });
      if (drag.pinch) {
        const [a, b] = [...down.values()];
        zoomTo(drag.s * (Math.hypot(a.x - b.x, a.y - b.y) / (drag.d || 1)), drag.mx, drag.my, false);
      } else if (drag.pan) {
        zoom.x = drag.zx + e.clientX - drag.x;
        zoom.y = drag.zy + e.clientY - drag.y;
        place();
      } else show(drag.k, along(e), true);
    });
    const up = (e) => {
      if (!down.delete(e.pointerId)) return;
      if (drag?.pinch) said();
      if (!down.size) {
        drag = null;
        frame.removeAttribute("data-panning");
      }
    };
    frame.addEventListener("pointerup", up);
    frame.addEventListener("pointercancel", up);
  }

  const read = () => {
    const v = cfg.bind ? api.state[cfg.bind] : null;
    const want = Array.isArray(v) ? v : v != null ? [v] : places();
    // Left to right, so each is held only by the ones already placed.
    inputs.forEach((_, k) => {
      if (want[k] != null) inputs[k].value = String(clamp(Math.round(Number(want[k]))));
    });
    inputs.forEach((_, k) => show(k, inputs[k].value, false));
    // A zoom set elsewhere, about the middle.
    const z = cfg.bindZoom ? Number(api.state[cfg.bindZoom]) : NaN;
    if (cfg.zoom && Number.isFinite(z) && Math.abs(z - zoom.s) > 0.005) zoomTo(z, undefined, undefined, false);
  };
  read();
  return { update: read };
};
