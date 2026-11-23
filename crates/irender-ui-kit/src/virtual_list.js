// irender-ui-kit: the `ui-virtual-list` behaviour. It hands the runtime only the items in
// view (and a few either side), with the space of the rest above and below, so a list of
// any length scrolls smoothly. Items are measured as they're drawn, so they may differ in
// height. As it scrolls it writes the places in view to `bindRange`, and near the end it
// fires `end`, once for each length of list, for the page to load more.
(window.__irenderBehaviors ||= {})["ui-virtual-list"] = (el, api) => {
  const cfg = api.config, K = window.__irenderKit;
  const list = el.querySelector("[data-vl-items]");
  const top = el.querySelector("[data-vl-pad=top]"), bottom = el.querySelector("[data-vl-pad=bottom]");
  const empty = el.querySelector("[data-vl-empty]");
  const OVERSCAN = 6;
  let est = cfg.itemHeight + cfg.gap, sig = null, endAt = -1, range = null;

  const all = () => {
    const v = K.value(api, cfg.items);
    return Array.isArray(v) ? v : [];
  };
  // The places to draw: what's in view, and a few either side.
  function windowOf(n) {
    if (!n) return [0, 0];
    const s = Math.min(Math.max(0, Math.floor(el.scrollTop / est) - OVERSCAN), n - 1);
    return [s, Math.min(n, s + Math.ceil((el.clientHeight || 400) / est) + 2 * OVERSCAN)];
  }
  // Items' real height, averaged over the ones drawn.
  function measure() {
    const drawn = [...list.children].filter((c) => c.hasAttribute("data-vl-item"));
    if (!drawn.length) return;
    const h = drawn.reduce((sum, c) => sum + c.getBoundingClientRect().height, 0) / drawn.length;
    if (h > 0) est = Math.max(4, h + cfg.gap);
  }
  function draw() {
    const items = all(), n = items.length;
    const [s, e] = windowOf(n);
    top.style.height = `${Math.round(s * est)}px`;
    bottom.style.height = `${Math.round(Math.max(0, n - e) * est)}px`;
    empty.hidden = n > 0;
    const next = `${s}:${e}:${n}`;
    if (next !== sig || items !== drawnFrom) {
      sig = next;
      drawnFrom = items;
      // Focus on an item that leaves the window stays in the list.
      const held = list.contains(document.activeElement);
      api.list(list, items.slice(s, e), s);
      if (held && !list.contains(document.activeElement)) el.focus({ preventScroll: true });
    }
    for (const c of list.children) if (c.hasAttribute("data-vl-item")) c.setAttribute("aria-setsize", String(n));
    const r = { from: s, to: Math.max(s, e - 1), total: n };
    if (cfg.bindRange && (!range || range.from !== r.from || range.to !== r.to || range.total !== r.total)) {
      range = r;
      api.set(cfg.bindRange, r);
    }
    if (n && e >= n - cfg.endThreshold && endAt !== n) {
      endAt = n;
      api.emit("end", { total: n });
    }
  }
  let drawnFrom = null, drawing = false;
  const redraw = () => {
    if (drawing) return;
    drawing = true;
    try {
      draw();
    } finally {
      drawing = false;
    }
  };
  // At most one redraw per 16 ms while scrolling, and one after the last event, measuring
  // what was drawn first.
  let last = 0, trailing = null;
  const run = () => {
    last = Date.now();
    trailing = null;
    measure();
    redraw();
  };
  el.addEventListener("scroll", () => {
    clearTimeout(trailing);
    if (Date.now() - last >= 16) run();
    else trailing = setTimeout(run, 16);
  }, { passive: true });
  new ResizeObserver(() => redraw()).observe(el);
  redraw();
  // Once the first items are drawn, their real height.
  setTimeout(run, 0);
  return {
    update() {
      // The runtime calls this after `api.list` too: only a change of items draws again.
      if (all() !== drawnFrom) redraw();
    },
    // Scrolls to the item at `i` (for a page's own controls).
    scrollTo(i) {
      el.scrollTop = Math.max(0, i * est);
      redraw();
    },
  };
};
