// irender-ui-kit: the `ui-dashboard-grid` behaviour. Tiles sit on a grid of cells;
// dragging a tile by its header (or grip) or its corner moves or resizes it cell by
// cell, pushing the tiles it lands on down (or up into the space it left), and with
// `compact` every tile floats up. Pointer events, so touch works too. On a focused
// header the arrow keys move the tile and Shift with them resizes it; Escape puts a
// dragged tile back. The layout goes to `bind` when a move ends.
(window.__irenderBehaviors ||= {})["ui-dashboard-grid"] = (el, api) => {
  // Interface text in the document's words (its `labels`), by the English.
  const L = (text, vars) => window.__irender.label(text, vars);
  const cfg = api.config;
  const grid = el.querySelector("[data-dg-grid]");
  const ghost = grid.querySelector(":scope > [data-dg-ghost]");
  const live = el.querySelector(":scope > [data-dg-live]");
  const tileOf = (id) => grid.querySelector(`:scope > [data-dg-tile][data-id="${CSS.escape(id)}"]`);
  const reduced = matchMedia("(prefers-reduced-motion: reduce)");
  const rtl = () => getComputedStyle(el).direction === "rtl";
  const stacked = () => grid.clientWidth < cfg.stack;
  const copy = (l) => l.map((i) => ({ ...i }));
  const say = (text) => {
    live.textContent = "";
    setTimeout(() => (live.textContent = text), 30);
  };

  // ---- the layout (the same rules as `place` in dashboard_grid.rs)
  const collides = (a, b) => a !== b && a.id !== b.id && a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
  // Locked tiles first, then by row and column: each floats up (with `compact`) and
  // then down past anything it overlaps.
  function settle(l) {
    const done = [];
    for (const it of [...l].sort((a, b) => b.locked - a.locked || a.y - b.y || a.x - b.x)) {
      if (!it.locked) {
        if (cfg.compact) while (it.y > 0 && !done.some((d) => collides(d, { ...it, y: it.y - 1 }))) it.y--;
        for (let d; (d = done.find((d) => collides(d, it))); ) it.y = d.y + d.h;
      }
      done.push(it);
    }
    return l;
  }
  // Puts `it` at (x, y) and moves the tiles it lands on out of the way: one dragged
  // onto goes into the space above the dragged tile if it fits there, else below it.
  function moveTo(l, it, x, y, user, depth = 0) {
    it.x = x;
    it.y = y;
    if (depth > l.length * 4) return;
    const hits = l.filter((o) => collides(o, it)).sort((a, b) => a.y - b.y || a.x - b.x);
    for (const o of hits) {
      if (!collides(o, it)) continue;
      if (o.locked) {
        // A locked tile isn't moved: the moving one goes under it.
        if (!it.locked) moveTo(l, it, it.x, o.y + o.h, false, depth + 1);
        continue;
      }
      if (user) {
        const up = { ...o, y: Math.max(it.y - o.h, 0) };
        if (!l.some((p) => p !== o && collides(p, up))) {
          moveTo(l, o, o.x, up.y, false, depth + 1);
          continue;
        }
      }
      moveTo(l, o, o.x, it.y + it.h, false, depth + 1);
    }
  }
  const clampSize = (it, w, h) => {
    const hiW = Math.max(1, Math.min(it.maxW ?? cfg.columns, cfg.columns - it.x)), hiH = Math.max(1, it.maxH ?? Infinity);
    return [Math.min(hiW, Math.max(Math.min(it.minW || 1, hiW), w)), Math.min(hiH, Math.max(Math.min(it.minH || 1, hiH), h))];
  };
  // The layout after moving (or, with `size`, resizing) tile `id`, from layout `from`.
  function after(from, id, x, y, size) {
    const l = copy(from), it = l.find((i) => i.id === id);
    if (size) [it.w, it.h] = clampSize(it, size[0], size[1]);
    moveTo(l, it, Math.max(0, Math.min(cfg.columns - it.w, x)), Math.max(0, y), !size);
    return settle(l);
  }

  // ---- drawing
  let layout = copy(cfg.items);
  const place = (node, it) => {
    node.style.gridColumn = `${it.x + 1} / span ${it.w}`;
    node.style.gridRow = `${it.y + 1} / span ${it.h}`;
  };
  // Draws `layout`; tiles other than `still` glide from where they were.
  function paint(still) {
    const tiles = layout.map((it) => [it, tileOf(it.id)]).filter(([, n]) => n);
    const before = new Map(reduced.matches ? [] : tiles.map(([it, n]) => [it.id, n.getBoundingClientRect()]));
    const order = [...layout].sort((a, b) => a.y - b.y || a.x - b.x).map((i) => i.id);
    for (const [it, n] of tiles) {
      place(n, it);
      n.style.order = String(order.indexOf(it.id));
      n.style.setProperty("--h", String(it.h));
    }
    for (const [it, n] of tiles) {
      const was = before.get(it.id);
      if (!was || it.id === still) continue;
      const now = n.getBoundingClientRect(), dx = was.left - now.left, dy = was.top - now.top;
      if (!dx && !dy) continue;
      n.style.transition = "none";
      n.style.transform = `translate(${dx}px, ${dy}px)`;
      requestAnimationFrame(() => {
        n.style.transition = "transform .18s ease";
        n.style.transform = "";
      });
    }
  }
  function save(id, how) {
    const out = layout.map(({ id, x, y, w, h }) => ({ id, x, y, w, h }));
    last = JSON.stringify(out);
    if (cfg.bind) api.set(cfg.bind, out);
    api.emit("layout", { layout: out, id, how });
  }
  const nameOf = (id) => tileOf(id)?.getAttribute("aria-label") || id;
  const where = (it) => L("{name}: column {column}, row {row}, {width} by {height}", { name: nameOf(it.id), column: it.x + 1, row: it.y + 1, width: it.w, height: it.h });

  // ---- dragging and resizing
  // A cell's step across and down, in px.
  const steps = () => {
    const gap = cfg.gap, width = grid.clientWidth;
    return [(width - gap * (cfg.columns - 1)) / cfg.columns + gap, cfg.row + gap];
  };
  let drag = null;
  el.addEventListener("pointerdown", (e) => {
    if (!cfg.editable || e.button !== 0 || stacked()) return;
    const handle = e.target.closest("[data-dg-handle], [data-dg-resize]");
    const node = handle?.closest("[data-dg-tile]");
    if (!node || node.parentElement !== grid) return;
    // Buttons and links in a header are for clicking.
    if (e.target.closest("button, a, input, select, textarea, [role=button]:not([data-dg-handle])")) return;
    const it = layout.find((i) => i.id === node.dataset.id);
    if (!it || it.locked) return;
    e.preventDefault();
    drag = { id: it.id, node, resize: handle.hasAttribute("data-dg-resize"), at: [e.clientX, e.clientY], start: copy(layout), from: { ...it }, pointer: e.pointerId, rect: node.getBoundingClientRect(), moved: false };
    try {
      handle.setPointerCapture(e.pointerId);
    } catch {
      // A pointer the browser doesn't know (a synthetic event): moves still reach `el`.
    }
    node.toggleAttribute(drag.resize ? "data-resizing" : "data-dragging", true);
  });
  el.addEventListener("pointermove", (e) => {
    if (!drag || e.pointerId !== drag.pointer) return;
    const [sx, sy] = steps(), dir = rtl() ? -1 : 1;
    const dx = (e.clientX - drag.at[0]) * dir, dy = e.clientY - drag.at[1];
    const f = drag.from;
    if (!drag.moved && Math.hypot(dx, dy) < 4) return;
    drag.moved = true;
    const next = drag.resize
      ? after(drag.start, f.id, f.x, f.y, [Math.round(f.w + dx / sx), Math.round(f.h + dy / sy)])
      : after(drag.start, f.id, Math.round(f.x + dx / sx), Math.round(f.y + dy / sy));
    if (JSON.stringify(next) !== JSON.stringify(layout)) {
      layout = next;
      paint(drag.id);
    }
    const it = layout.find((i) => i.id === f.id);
    if (drag.resize) return;
    // The dragged tile follows the pointer; the ghost shows where it will land.
    place(ghost, it);
    ghost.hidden = false;
    drag.node.style.transition = "none";
    drag.node.style.transform = "";
    const now = drag.node.getBoundingClientRect();
    drag.node.style.transform = `translate(${drag.rect.left + (e.clientX - drag.at[0]) - now.left}px, ${drag.rect.top + dy - now.top}px)`;
  });
  function stop(cancel) {
    const d = drag;
    drag = null;
    ghost.hidden = true;
    d.node.removeAttribute("data-dragging");
    d.node.removeAttribute("data-resizing");
    if (cancel) layout = d.start;
    // The tile glides from under the pointer to its place.
    const was = d.node.getBoundingClientRect();
    d.node.style.transform = "";
    paint(d.id);
    if (!reduced.matches && !d.resize) {
      const now = d.node.getBoundingClientRect();
      d.node.style.transition = "none";
      d.node.style.transform = `translate(${was.left - now.left}px, ${was.top - now.top}px)`;
      requestAnimationFrame(() => {
        d.node.style.transition = "transform .18s ease";
        d.node.style.transform = "";
      });
    }
    if (cancel || !d.moved) return;
    const it = layout.find((i) => i.id === d.id);
    if (JSON.stringify(layout) !== JSON.stringify(d.start)) save(d.id, d.resize ? "resize" : "move");
    say(where(it));
  }
  const end = (e) => drag && e.pointerId === drag.pointer && stop(false);
  el.addEventListener("pointerup", end);
  el.addEventListener("pointercancel", (e) => drag && e.pointerId === drag.pointer && stop(true));
  el.addEventListener("lostpointercapture", (e) => drag && e.pointerId === drag.pointer && stop(false));

  // ---- the keys
  el.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && drag) {
      e.preventDefault();
      return stop(true);
    }
    const handle = e.target.closest?.("[data-dg-handle]");
    const node = handle?.closest("[data-dg-tile]");
    if (!handle || node?.parentElement !== grid || !cfg.editable || stacked() || e.ctrlKey || e.metaKey) return;
    const keys = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] };
    if (!keys[e.key]) return;
    e.preventDefault();
    let [dx, dy] = keys[e.key];
    if (rtl()) dx = -dx;
    const it = layout.find((i) => i.id === node.dataset.id);
    let next = layout;
    if (e.shiftKey) next = after(layout, it.id, it.x, it.y, [it.w + dx, it.h + dy]);
    else {
      // The nearest place in that direction where the tile ends up somewhere new: a
      // step down past a tile below goes all the way under it.
      for (let k = 1; k <= 64; k++) {
        const l = after(layout, it.id, it.x + dx * k, it.y + dy * k);
        const now = l.find((i) => i.id === it.id);
        if (now.x !== it.x || now.y !== it.y) {
          next = l;
          break;
        }
        if (dx && (it.x + dx * k < 0 || it.x + dx * k > cfg.columns - it.w)) break;
        if (dy < 0 && it.y + dy * k < 0) break;
      }
    }
    if (JSON.stringify(next) === JSON.stringify(layout)) return say(where(it));
    layout = next;
    paint();
    handle.focus();
    save(it.id, e.shiftKey ? "resize" : "move");
    say(where(layout.find((i) => i.id === it.id)));
  });

  // ---- state → grid
  // A layout from `bind`: tiles it names take its places, the rest keep theirs. Null
  // puts every tile back where its props say.
  let last = JSON.stringify(layout.map(({ id, x, y, w, h }) => ({ id, x, y, w, h })));
  function adopt(v) {
    // Cleared: back to the tiles' own places.
    if (!Array.isArray(v)) v = cfg.defaults;
    const l = copy(cfg.items);
    for (const it of l) {
      const s = v.find((s) => s && String(s.id) === it.id);
      if (!s) continue;
      for (const k of ["x", "y", "w", "h"]) if (Number.isFinite(Number(s[k]))) it[k] = Math.round(Number(s[k]));
      [it.w, it.h] = clampSize({ ...it, x: 0 }, it.w, it.h);
      it.x = Math.max(0, Math.min(cfg.columns - it.w, it.x));
      it.y = Math.max(0, it.y);
    }
    layout = settle(l);
    paint();
  }
  return {
    update() {
      if (!cfg.bind || drag) return;
      const v = api.state[cfg.bind];
      const now = JSON.stringify(v ?? null);
      if (now === last) return;
      last = now;
      adopt(v);
    },
    get layout() {
      return layout.map(({ id, x, y, w, h }) => ({ id, x, y, w, h }));
    },
  };
};
