// irender-ui-kit: dragging by touch (and pen), for components that drag with the
// browser's drag and drop, which a mouse starts but most phones don't. A press held
// still for 300ms picks the item up (moving first is a scroll, as before); a copy then
// follows the finger, the page (and `o.scroller`) scrolls near their edges, and
// letting go drops it. Escape, or the browser taking the touch back, cancels.
//
//   window.__irenderTouchDrag(root, {
//     pick(target) → the element to drag, or null
//     start(el), over(x, y, under), drop(x, y, under), end(), scroller: element
//   })
//
// `under` is the element under the finger. `end` runs after a drop or a cancel.
window.__irenderTouchDrag ||= (root, o) => {
  const HOLD = 300, SLOP = 8, EDGE = 48;
  let press = null, drag = null, frame = 0;

  const under = (x, y) => document.elementFromPoint(x, y);
  function finish(dropped) {
    clearTimeout(press?.timer);
    if (drag) {
      cancelAnimationFrame(frame);
      drag.ghost.remove();
      drag.el.removeAttribute("data-touch-dragging");
      if (dropped) o.drop(drag.x, drag.y, under(drag.x, drag.y));
      o.end();
      // The tap that ends a drag isn't a click.
      const swallow = (e) => {
        e.stopPropagation();
        e.preventDefault();
      };
      root.addEventListener("click", swallow, { capture: true, once: true });
      setTimeout(() => root.removeEventListener("click", swallow, { capture: true }), 100);
    }
    press = drag = null;
  }
  function begin() {
    const { el, x, y } = press;
    const r = el.getBoundingClientRect();
    // A copy of the item under the finger; the item itself stays, marked, in its place.
    const ghost = el.cloneNode(true);
    ghost.removeAttribute("id");
    ghost.querySelectorAll("[id]").forEach((n) => n.removeAttribute("id"));
    ghost.setAttribute("aria-hidden", "true");
    ghost.setAttribute("data-touch-ghost", "");
    Object.assign(ghost.style, {
      position: "fixed", left: `${r.left}px`, top: `${r.top}px`, width: `${r.width}px`, height: `${r.height}px`, margin: "0",
      boxSizing: "border-box", zIndex: "2147483000", pointerEvents: "none", opacity: "0.92", rotate: "1.5deg",
      boxShadow: "0 12px 28px rgb(0 0 0 / 0.2)", transition: "none",
    });
    root.append(ghost);
    el.setAttribute("data-touch-dragging", "");
    drag = { el, ghost, dx: x - r.left, dy: y - r.top, x, y };
    navigator.vibrate?.(8);
    o.start(el);
    follow();
  }
  // The ghost goes where the finger is; near an edge the page or the scroller scrolls,
  // and what's under the finger is asked again.
  function follow() {
    if (!drag) return;
    const { x, y } = drag;
    drag.ghost.style.left = `${x - drag.dx}px`;
    drag.ghost.style.top = `${y - drag.dy}px`;
    let scrolled = false;
    const s = o.scroller;
    if (s) {
      const r = s.getBoundingClientRect();
      const step = x < r.left + EDGE ? -12 : x > r.right - EDGE ? 12 : 0;
      if (step && s.scrollWidth > s.clientWidth) {
        const before = s.scrollLeft;
        s.scrollLeft += step;
        scrolled ||= s.scrollLeft !== before;
      }
    }
    const v = y < EDGE ? -12 : y > innerHeight - EDGE ? 12 : 0;
    if (v) {
      const before = scrollY;
      scrollBy(0, v);
      scrolled ||= scrollY !== before;
    }
    o.over(x, y, under(x, y));
    if (scrolled) frame = requestAnimationFrame(follow);
  }

  root.addEventListener("pointerdown", (e) => {
    if (e.pointerType === "mouse" || !e.isPrimary || drag) return;
    const el = o.pick(e.target);
    if (!el) return;
    clearTimeout(press?.timer);
    press = { el, x: e.clientX, y: e.clientY, id: e.pointerId, timer: setTimeout(begin, HOLD) };
  });
  root.addEventListener("pointermove", (e) => {
    if (!press || e.pointerId !== press.id) return;
    if (drag) {
      cancelAnimationFrame(frame);
      drag.x = e.clientX;
      drag.y = e.clientY;
      follow();
    } else if (Math.hypot(e.clientX - press.x, e.clientY - press.y) > SLOP) finish(false);
  });
  root.addEventListener("pointerup", (e) => press && e.pointerId === press.id && finish(true));
  root.addEventListener("pointercancel", (e) => press && e.pointerId === press.id && finish(false));
  // While dragging, the finger doesn't scroll the page; the long-press menu and the
  // browser's own drag don't start.
  root.addEventListener("touchmove", (e) => drag && e.cancelable && e.preventDefault(), { passive: false });
  root.addEventListener("contextmenu", (e) => (press || drag) && e.preventDefault());
  root.addEventListener("dragstart", (e) => press && (e.preventDefault(), e.stopImmediatePropagation()), { capture: true });
  addEventListener("keydown", (e) => e.key === "Escape" && drag && (e.preventDefault(), finish(false)));
};
