// irender-ui-kit: the `ui-resizable` behaviour. Panels share the group's length in
// percentages; each handle moves the line between the panels on either side of it,
// within their `min` and `max`. Dragging (pointer events, so touch too), the arrow keys
// (5%, Shift for 1%), Home and End, Enter to fold a collapsible panel and bring it
// back, and a double click to go back to the starting sizes. The sizes go to `bind`
// when a move ends.
(window.__irenderBehaviors ||= {})["ui-resizable"] = (el, api) => {
  const cfg = api.config;
  const vertical = cfg.direction === "vertical";
  // Side by side on a right-to-left page, the first panel is on the right: moves go the other way.
  const along = () => (!vertical && getComputedStyle(el).direction === "rtl" ? -1 : 1);
  const panels = [...el.children].filter((c) => c.hasAttribute("data-rz-panel"));
  const handles = [...el.children].filter((c) => c.hasAttribute("data-rz-handle"));
  const limits = cfg.panels; // [{ min, max, collapsible }]
  const round = (n) => Math.round(n * 100) / 100;
  let sizes = start();
  const initial = sizes.slice(); // where a double click goes back to
  const unfolded = sizes.slice(); // each panel's last size before it folded

  function start() {
    const s = cfg.bind && Array.isArray(api.state[cfg.bind]) && api.state[cfg.bind].length === panels.length ? api.state[cfg.bind].map(Number) : cfg.sizes.slice();
    return s.every(Number.isFinite) ? s : cfg.sizes.slice();
  }
  function paint() {
    panels.forEach((p, i) => {
      p.style.flexGrow = String(sizes[i]);
      p.toggleAttribute("data-collapsed", sizes[i] === 0);
    });
    handles.forEach((h, i) => {
      h.setAttribute("aria-valuenow", String(Math.round(sizes[i])));
      h.setAttribute("aria-valuetext", `${Math.round(sizes[i])}%`);
    });
  }
  function save() {
    if (cfg.bind) api.set(cfg.bind, sizes.map(round));
    api.emit("resize", { sizes: sizes.map(round) });
  }
  // Moves the line after panel `i` to make it `want`% (the panel after takes the rest of
  // their two), as far as both panels' limits allow.
  function moveTo(i, want) {
    const a = limits[i], b = limits[i + 1], pair = sizes[i] + sizes[i + 1];
    let lo = Math.max(a.min, pair - b.max), hi = Math.min(a.max, pair - b.min);
    // A collapsible panel can go to 0 from below its min, and back.
    if (a.collapsible && want < a.min / 2) return set(i, 0, pair);
    if (b.collapsible && pair - want < b.min / 2) return set(i, pair, pair);
    set(i, Math.min(hi, Math.max(lo, want)), pair);
  }
  function set(i, size, pair) {
    [i, i + 1].forEach((j) => sizes[j] > 0 && (unfolded[j] = sizes[j]));
    sizes[i] = round(size);
    sizes[i + 1] = round(pair - size);
    paint();
  }

  // ---- dragging
  let drag = null;
  el.addEventListener("pointerdown", (e) => {
    const h = e.target.closest("[data-rz-handle]");
    if (!h || e.button !== 0 || h.parentElement !== el) return;
    e.preventDefault();
    const i = handles.indexOf(h);
    // The panels' length, without the handles'.
    const total = panels.reduce((t, p) => t + (vertical ? p.offsetHeight : p.offsetWidth), 0) || 1;
    drag = { i, h, total, startAt: vertical ? e.clientY : e.clientX, startSize: sizes[i], id: e.pointerId };
    try {
      h.setPointerCapture(e.pointerId);
    } catch {
      // A pointer the browser doesn't know (a synthetic event): moves still reach `el`.
    }
    h.setAttribute("data-dragging", "");
  });
  el.addEventListener("pointermove", (e) => {
    if (!drag || e.pointerId !== drag.id) return;
    const moved = ((vertical ? e.clientY : e.clientX) - drag.startAt) / drag.total * 100 * along();
    moveTo(drag.i, drag.startSize + moved);
  });
  const end = (e) => {
    if (!drag || e.pointerId !== drag.id) return;
    drag.h.removeAttribute("data-dragging");
    drag = null;
    save();
  };
  el.addEventListener("pointerup", end);
  el.addEventListener("pointercancel", end);
  el.addEventListener("dblclick", (e) => {
    const h = e.target.closest("[data-rz-handle]");
    if (!h || h.parentElement !== el) return;
    sizes = initial.slice();
    paint();
    save();
  });

  // ---- the keys
  el.addEventListener("keydown", (e) => {
    const h = e.target.closest?.("[data-rz-handle]");
    if (!h || h.parentElement !== el) return;
    const i = handles.indexOf(h), step = e.shiftKey ? 1 : 5;
    const grow = vertical ? "ArrowDown" : along() === 1 ? "ArrowRight" : "ArrowLeft", shrink = vertical ? "ArrowUp" : along() === 1 ? "ArrowLeft" : "ArrowRight";
    let done = true;
    if (e.key === grow) moveTo(i, sizes[i] + step);
    else if (e.key === shrink) moveTo(i, sizes[i] - step);
    else if (e.key === "Home") moveTo(i, limits[i].collapsible ? 0 : -1);
    else if (e.key === "End") moveTo(i, sizes[i] + sizes[i + 1]);
    else if (e.key === "Enter" && (limits[i].collapsible || limits[i + 1].collapsible)) {
      // Fold the collapsible side, or bring it back to where it was.
      const side = limits[i].collapsible ? i : i + 1, pair = sizes[i] + sizes[i + 1];
      if (sizes[side] === 0) {
        const back = Math.min(unfolded[side] || limits[side].min, pair);
        set(i, side === i ? back : pair - back, pair);
      } else {
        set(i, side === i ? 0 : pair, pair);
      }
    } else done = false;
    if (done) {
      e.preventDefault();
      save();
    }
  });

  paint();
  let last = cfg.bind ? api.state[cfg.bind] : null;
  return {
    update() {
      if (!cfg.bind || drag) return;
      const now = api.state[cfg.bind];
      if (now === last) return;
      last = now;
      if (Array.isArray(now) && now.length === panels.length && now.every((n) => Number.isFinite(Number(n)))) {
        sizes = now.map(Number);
        paint();
      }
    },
  };
};
