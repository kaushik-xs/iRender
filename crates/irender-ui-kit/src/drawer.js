// irender-ui-kit: the `ui-drawer` behaviour: drag it down to close. The grab strip at its
// top takes a drag from any pointer; on a touch screen the rest of it does too, while
// its content is scrolled to the top. Let go past a quarter of its height, or with a
// flick, and it closes; otherwise it springs back. Escape and the ✕ close it as any
// dialog (the drag is the pointer's way, not the only way).
(window.__irenderBehaviors ||= {})["ui-drawer"] = (el, api) => {
  let drag = null;
  const scrolled = (t) => {
    for (let n = t; n && n !== el.parentElement; n = n.parentElement) if (n.scrollTop > 0) return true;
    return false;
  };
  el.addEventListener("pointerdown", (e) => {
    if (e.button !== 0 || !el.open) return;
    const grip = e.target.closest("[data-drawer-grip]");
    // Away from the grip: touch only, not in a field, and not while the content is scrolled.
    if (!grip && (e.pointerType === "mouse" || e.target.closest("input, textarea, select, [contenteditable]") || scrolled(e.target))) return;
    drag = { y: e.clientY, x: e.clientX, id: e.pointerId, moving: !!grip, dy: 0, track: [[e.timeStamp, e.clientY]] };
    if (grip) {
      e.preventDefault();
      start(e);
    }
  });
  function start(e) {
    drag.moving = true;
    el.setAttribute("data-dragging", "");
    try {
      el.setPointerCapture(e.pointerId);
    } catch {}
  }
  el.addEventListener("pointermove", (e) => {
    if (!drag || e.pointerId !== drag.id) return;
    const dy = e.clientY - drag.y, dx = e.clientX - drag.x;
    if (!drag.moving) {
      // Becomes a drag only once it's clearly downward; otherwise it's a scroll or a tap.
      if (Math.abs(dx) > 8 || dy < -4) return void (drag = null);
      if (dy < 8) return;
      start(e);
    }
    drag.dy = Math.max(0, dy);
    drag.track.push([e.timeStamp, e.clientY]);
    if (drag.track.length > 6) drag.track.shift();
    el.style.transform = `translateY(${drag.dy}px)`;
  });
  const end = (e) => {
    if (!drag || e.pointerId !== drag.id) return;
    const { dy, moving, track } = drag;
    drag = null;
    if (!moving) return;
    el.removeAttribute("data-dragging");
    const [t0, y0] = track[0], [t1, y1] = track[track.length - 1];
    const speed = t1 > t0 ? (y1 - y0) / (t1 - t0) : 0; // px per ms, downward
    if (dy > el.offsetHeight / 4 || (speed > 0.5 && dy > 20)) {
      el.style.transform = "";
      el.close();
      api.emit("dismiss", {});
    } else {
      // Springs back with the drawer's own transition.
      el.style.transform = "";
    }
    // A drag isn't a click on what it started on.
    const stop = (c) => (c.stopPropagation(), c.preventDefault());
    window.addEventListener("click", stop, { capture: true, once: true });
    setTimeout(() => window.removeEventListener("click", stop, { capture: true }), 0);
  };
  el.addEventListener("pointerup", end);
  el.addEventListener("pointercancel", (e) => {
    if (!drag || e.pointerId !== drag.id) return;
    drag = null;
    el.removeAttribute("data-dragging");
    el.style.transform = "";
  });
  return {};
};
