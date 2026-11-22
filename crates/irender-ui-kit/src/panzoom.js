// irender-ui-kit: zooming and moving what's inside a frame (`window.__irenderPanZoom`),
// with the wheel, a pinch, a drag, a double click and the keys. Used by the lightbox, the
// plan and the PDF viewer.
window.__irenderPanZoom ||= (frame, target, o = {}) => {
  const K = window.__irenderKit;
  const min = o.min ?? 1, max = o.max ?? 8;
  let s = 1, x = 0, y = 0;
  const pointers = new Map();
  let drag = null, pinch = null, moved = false;
  // Kept over the frame: no further than its edges, or centred when it's smaller.
  function keep() {
    const f = frame.getBoundingClientRect(), w = target.offsetWidth * s, h = target.offsetHeight * s;
    if (o.free) {
      x = K.clamp(x, 40 - w, f.width - 40);
      y = K.clamp(y, 40 - h, f.height - 40);
      return;
    }
    const ox = target.offsetLeft, oy = target.offsetTop;
    x = w <= f.width ? (f.width - w) / 2 - ox : K.clamp(x, f.width - w - ox, -ox);
    y = h <= f.height ? (f.height - h) / 2 - oy : K.clamp(y, f.height - h - oy, -oy);
  }
  function draw() {
    keep();
    target.style.transformOrigin = "0 0";
    target.style.transform = `translate(${x}px, ${y}px) scale(${s})`;
    o.onChange?.({ scale: s, x, y });
  }
  // To `next`, with the point of the frame at (cx, cy) staying where it is.
  function zoom(next, cx, cy) {
    const f = frame.getBoundingClientRect();
    cx = (cx ?? f.left + f.width / 2) - f.left - target.offsetLeft;
    cy = (cy ?? f.top + f.height / 2) - f.top - target.offsetTop;
    next = K.clamp(next, min, max);
    x = cx - ((cx - x) / s) * next;
    y = cy - ((cy - y) / s) * next;
    s = next;
    draw();
  }
  const reset = () => ((s = o.start?.() ?? 1), (x = 0), (y = 0), draw());
  frame.addEventListener("wheel", (e) => {
    // Without a modifier the page scrolls, unless the frame takes every wheel.
    if (!o.wheel && !e.ctrlKey && !e.metaKey) return;
    e.preventDefault();
    zoom(s * Math.exp(-e.deltaY * (e.ctrlKey ? 0.01 : 0.0015)), e.clientX, e.clientY);
  }, { passive: false });
  frame.addEventListener("pointerdown", (e) => {
    if (e.button !== 0 || (o.ignore && e.target.closest(o.ignore))) return;
    pointers.set(e.pointerId, e);
    moved = false;
    if (pointers.size === 2) {
      const [a, b] = [...pointers.values()];
      pinch = { d: Math.hypot(a.clientX - b.clientX, a.clientY - b.clientY), s };
      drag = null;
    } else drag = { x: e.clientX, y: e.clientY, x0: x, y0: y };
  });
  frame.addEventListener("pointermove", (e) => {
    if (!pointers.has(e.pointerId)) return;
    pointers.set(e.pointerId, e);
    if (pinch && pointers.size === 2) {
      const [a, b] = [...pointers.values()];
      zoom((pinch.s * Math.hypot(a.clientX - b.clientX, a.clientY - b.clientY)) / Math.max(1, pinch.d), (a.clientX + b.clientX) / 2, (a.clientY + b.clientY) / 2);
      moved = true;
    } else if (drag) {
      const dx = e.clientX - drag.x, dy = e.clientY - drag.y;
      if (!moved && Math.hypot(dx, dy) < 4) return;
      if (!moved) K.capture(frame, e);
      moved = true;
      x = drag.x0 + dx;
      y = drag.y0 + dy;
      draw();
    }
  });
  const up = (e) => {
    if (!pointers.delete(e.pointerId)) return;
    if (pointers.size < 2) pinch = null;
    drag = null;
    if (moved) o.onEnd?.(e);
  };
  frame.addEventListener("pointerup", up);
  frame.addEventListener("pointercancel", up);
  // The click that ends a drag isn't a click.
  frame.addEventListener("click", (e) => moved && (e.stopPropagation(), e.preventDefault(), (moved = false)), true);
  frame.addEventListener("dblclick", (e) => {
    if (o.ignore && e.target.closest(o.ignore)) return;
    s > (o.start?.() ?? 1) * 1.05 ? reset() : zoom(s * 2.5, e.clientX, e.clientY);
  });
  const pan = (dx, dy) => ((x += dx), (y += dy), draw());
  const to = (next, px, py) => {
    // The point (px, py) of the target in the middle of the frame.
    const f = frame.getBoundingClientRect();
    s = K.clamp(next, min, max);
    x = f.width / 2 - px * s - target.offsetLeft;
    y = f.height / 2 - py * s - target.offsetTop;
    draw();
  };
  return { zoom, reset, pan, draw, to, get scale() { return s; }, get moved() { return moved; }, at: () => ({ x, y, s }), set: (v) => ((s = v.s), (x = v.x), (y = v.y), draw()) };
};
