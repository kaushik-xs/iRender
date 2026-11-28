// irender-ui-kit: the `ui-virtual-list` behaviour. It hands the runtime only the items in
// view (and a few either side), with the space of the rest above and below, so a list of
// any length scrolls smoothly. Items are measured as they're drawn, so they may differ in
// height. As it scrolls it writes the places in view to `bindRange`, and near the end it
// fires `end`, once for each length of list, for the page to load more. With `columns`
// (or `minItemWidth`) items are cards in a grid, a row at a time; `scrollTo` (a place or
// an item's key, from state) brings an item to the top; with `groupBy` the group of the
// items at the top stays over them.
(window.__irenderBehaviors ||= {})["ui-virtual-list"] = (el, api) => {
  const cfg = api.config, K = window.__irenderKit;
  const list = el.querySelector("[data-vl-items]");
  const top = el.querySelector("[data-vl-pad=top]"), bottom = el.querySelector("[data-vl-pad=bottom]");
  const empty = el.querySelector("[data-vl-empty]"), sticky = el.querySelector("[data-vl-sticky]");
  const OVERSCAN = 6;
  // `est` is a row's height: an item's, or a row of cards'.
  let est = cfg.itemHeight + cfg.gap, sig = null, endAt = -1, range = null, cols = 1, aimed;

  const all = () => {
    const v = K.value(api, cfg.items);
    return Array.isArray(v) ? v : [];
  };
  const at = (v, f) => String(f).split(".").reduce((x, k) => (x !== null && typeof x === "object" ? x[k] : undefined), v);
  // Cards a row: `columns`, or as many `minItemWidth` wide as fit.
  function columns() {
    if (Number(cfg.columns) > 0) return Math.floor(Number(cfg.columns));
    if (Number(cfg.minItemWidth) > 0) return Math.max(1, Math.floor(((list.clientWidth || el.clientWidth) + cfg.gap) / (Number(cfg.minItemWidth) + cfg.gap)));
    return 1;
  }
  // The rows to draw: what's in view, and a few either side. While the view is still
  // within the rows drawn, they stay: placed by the average height, a new window would
  // move them under it.
  let win = null;
  function windowOf(rows) {
    if (!rows) return (win = null), [0, 0];
    if (win && win.rows === rows && win.cols === cols) {
      const from = top.offsetHeight, to = from + list.getBoundingClientRect().height, v = el.scrollTop;
      if (v >= from && v + el.clientHeight <= to + 1 && (win.s === 0 || v > from) && (win.e === rows || v + el.clientHeight < to)) return [win.s, win.e];
    }
    const s = Math.min(Math.max(0, Math.floor(el.scrollTop / est) - OVERSCAN), rows - 1);
    const e = Math.min(rows, s + Math.ceil((el.clientHeight || 400) / est) + 2 * OVERSCAN);
    win = { s, e, rows, cols };
    return [s, e];
  }
  // Rows' real height, averaged over the ones drawn.
  function measure() {
    const drawn = [...list.children].filter((c) => c.hasAttribute("data-vl-item"));
    if (!drawn.length) return;
    const h = drawn.reduce((sum, c) => sum + c.getBoundingClientRect().height, 0) / drawn.length;
    if (h > 0) est = Math.max(4, h + cfg.gap);
  }
  function draw() {
    const items = all(), n = items.length;
    const was = cols;
    cols = columns();
    if (cols !== was) list.style.setProperty("--vl-cols", String(cols));
    const rows = Math.ceil(n / cols);
    const [s, e] = windowOf(rows);
    const from = s * cols, to = Math.min(n, e * cols);
    top.style.height = `${Math.round(s * est)}px`;
    bottom.style.height = `${Math.round(Math.max(0, rows - e) * est)}px`;
    empty.hidden = n > 0;
    const next = `${from}:${to}:${n}`;
    if (next !== sig || items !== drawnFrom) {
      sig = next;
      drawnFrom = items;
      // Focus on an item that leaves the window stays in the list.
      const held = list.contains(document.activeElement);
      api.list(list, items.slice(from, to), from);
      if (held && !list.contains(document.activeElement)) el.focus({ preventScroll: true });
    }
    for (const c of list.children) if (c.hasAttribute("data-vl-item")) c.setAttribute("aria-setsize", String(n));
    if (sticky) {
      // The group of the first item at the top.
      const first = items[Math.min(n - 1, Math.floor(el.scrollTop / est) * cols)];
      const g = n ? at(first, cfg.groupBy) : null;
      sticky.hidden = g === null || g === undefined || g === "";
      if (!sticky.hidden && sticky.textContent !== String(g)) sticky.textContent = String(g);
    }
    const r = { from, to: Math.max(from, to - 1), total: n };
    if (cfg.bindRange && (!range || range.from !== r.from || range.to !== r.to || range.total !== r.total)) {
      range = r;
      api.set(cfg.bindRange, r);
    }
    if (n && to >= n - cfg.endThreshold && endAt !== n) {
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
  // To the item at `i`: near it by the average height, then onto it once it's drawn (a
  // few times, as what's drawn around it is measured).
  // Each step waits a turn: the items drawn for it may come in the next one.
  const turn = () => new Promise((r) => setTimeout(r));
  let aiming = 0;
  async function scrollTo(i) {
    const n = all().length, me = ++aiming;
    if (!(i >= 0 && i < n)) return;
    const row = Math.floor(i / cols);
    el.scrollTop = Math.max(0, row * est);
    run();
    for (let k = 0; k < 8; k++) {
      await turn();
      if (me !== aiming) return;
      const drawn = [...list.children].filter((c) => c.hasAttribute("data-vl-item"));
      const item = drawn.find((c) => Number(c.getAttribute("aria-posinset")) === i + 1);
      const over = sticky && !sticky.hidden ? sticky.offsetHeight : 0;
      let d;
      if (item) d = item.getBoundingClientRect().top - el.getBoundingClientRect().top - over;
      else {
        // Not drawn yet: from the nearest one drawn, by the rows between at their average.
        const near = drawn.reduce((b, c) => (Math.abs(Number(c.getAttribute("aria-posinset")) - 1 - i) < Math.abs(Number(b?.getAttribute("aria-posinset")) - 1 - i) || !b ? c : b), null);
        if (!near) break;
        const p = Number(near.getAttribute("aria-posinset")) - 1;
        d = near.getBoundingClientRect().top - el.getBoundingClientRect().top - over + (Math.floor(i / cols) - Math.floor(p / cols)) * est;
      }
      if (Math.abs(d) < 1) break;
      const was = el.scrollTop;
      el.scrollTop += d;
      run();
      if (el.scrollTop === was) break;
    }
  }
  // `scrollTo` from state: a place, or an item's key.
  function aim() {
    if (!("scrollTo" in cfg)) return;
    const v = K.value(api, cfg.scrollTo);
    if (v === aimed) return;
    aimed = v;
    if (v === null || v === undefined || v === "") return;
    const items = all();
    let i = items.findIndex((x) => x && typeof x === "object" && String(at(x, cfg.itemKey)) === String(v));
    if (i < 0 && typeof v === "number") i = v;
    if (i >= 0) setTimeout(() => scrollTo(i));
  }
  aim();
  return {
    update() {
      // The runtime calls this after `api.list` too: only a change of items draws again.
      if (all() !== drawnFrom) redraw();
      aim();
    },
    // Scrolls to the item at `i` (for a page's own controls).
    scrollTo,
  };
};
