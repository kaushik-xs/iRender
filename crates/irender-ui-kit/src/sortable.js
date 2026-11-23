// irender-ui-kit: the `ui-sortable-list` behaviour. The items are the runtime's keyed
// list of the state key, so this only works out the new order and writes it; the runtime
// moves the items. Each item's grip drags it (mouse, touch or pen alike, through pointer
// events: the grip doesn't scroll), and from the keyboard Space picks it up, the arrows
// move it and Space puts it down (Escape puts it back), or Alt and the arrows move it
// straight away.
(window.__irenderBehaviors ||= {})["ui-sortable-list"] = (el, api) => {
  const cfg = api.config;
  const L = (text, vars) => window.__irender.label(text, vars);
  const live = el.querySelector("[data-sl-live]");
  const say = (msg) => live && (live.textContent = msg);
  const isObj = (v) => v !== null && typeof v === "object" && !Array.isArray(v);

  const list = () => (Array.isArray(api.state[cfg.bind]) ? api.state[cfg.bind] : []);
  const keyOf = (v) => String(cfg.itemKey && isObj(v) ? v[cfg.itemKey] : v);
  const nameOf = (v) => String(cfg.labelField && isObj(v) ? (v[cfg.labelField] ?? v.name ?? v.title ?? keyOf(v)) : v);
  const items = () => [...el.querySelectorAll(":scope [data-sl-items] > [data-sl-item]")];
  const itemOf = (k) => items().find((li) => li.getAttribute("data-sl-item") === k);
  const off = () => el.hasAttribute("data-disabled") && el.getAttribute("data-disabled") !== "false";

  // The list with the item `k` at place `to`; written, said, and its grip focused again.
  function move(k, to, quiet) {
    const all = list(), from = all.findIndex((v) => keyOf(v) === k);
    to = Math.max(0, Math.min(all.length - 1, to));
    if (from < 0 || from === to) return false;
    const next = all.slice();
    const [it] = next.splice(from, 1);
    next.splice(to, 0, it);
    moving = true;
    api.set(cfg.bind, next);
    api.emit("change", { value: next, moved: it, from, to });
    if (!quiet) say(L("{name} moved to {n} of {total}", { name: nameOf(it), n: to + 1, total: next.length }));
    refocus(k);
    return true;
  }
  // After the runtime has moved the items, the grip that had focus gets it back.
  function refocus(k) {
    queueMicrotask(() => {
      const li = itemOf(k);
      li?.querySelector("[data-sl-grip]")?.focus({ preventScroll: false });
      if (picked === k) li?.setAttribute("data-sl-picked", "");
      moving = false;
    });
  }

  // ---- the keyboard, on a grip
  // `moving`: while the runtime moves the items, focus may leave for a moment.
  let picked = null, pickedFrom = -1, moving = false;
  function drop(cancel) {
    if (picked === null) return;
    const k = picked, it = list().find((v) => keyOf(v) === k);
    picked = null;
    itemOf(k)?.removeAttribute("data-sl-picked");
    if (cancel) {
      move(k, pickedFrom, true);
      say(L("{name} put back at {n}", { name: nameOf(it), n: pickedFrom + 1 }));
    } else say(L("{name} put down at {n} of {total}", { name: nameOf(it), n: list().findIndex((v) => keyOf(v) === k) + 1, total: list().length }));
    refocus(k);
  }
  el.addEventListener("keydown", (e) => {
    const grip = e.target.closest?.("[data-sl-grip]");
    if (!grip || off()) return;
    const k = grip.closest("[data-sl-item]").getAttribute("data-sl-item");
    const at = list().findIndex((v) => keyOf(v) === k);
    const step = { ArrowUp: -1, ArrowDown: 1 }[e.key];
    if (e.key === " " || e.key === "Enter") {
      e.preventDefault();
      if (picked === k) return drop(false);
      drop(false);
      picked = k;
      pickedFrom = at;
      grip.closest("[data-sl-item]").setAttribute("data-sl-picked", "");
      const it = list()[at];
      say(L("{name} picked up, at {n} of {total}", { name: nameOf(it), n: at + 1, total: list().length }));
    } else if (e.key === "Escape" && picked === k) {
      e.preventDefault();
      drop(true);
    } else if (step && (picked === k || e.altKey)) {
      e.preventDefault();
      move(k, at + step);
    } else if ((e.key === "Home" || e.key === "End") && picked === k) {
      e.preventDefault();
      move(k, e.key === "Home" ? 0 : list().length - 1);
    } else if (step) {
      // Up and down go between the grips.
      e.preventDefault();
      const all = items(), i = all.findIndex((li) => li.getAttribute("data-sl-item") === k);
      all[i + step]?.querySelector("[data-sl-grip]")?.focus();
    }
  });
  // Leaving a picked item puts it down where it is.
  el.addEventListener("focusout", (e) => {
    if (picked !== null && !moving && !el.contains(e.relatedTarget)) drop(false);
  });

  // ---- dragging a grip
  let drag = null;
  function clearMarks() {
    for (const li of items()) li.removeAttribute("data-sl-drop");
  }
  // Where the pointer would put the dragged item: the place, and the item to mark.
  function target(y) {
    const others = items().filter((li) => li.getAttribute("data-sl-item") !== drag.key);
    let to = others.length;
    for (const [i, li] of others.entries()) {
      const r = li.getBoundingClientRect();
      if (y < r.top + r.height / 2) {
        to = i;
        break;
      }
    }
    const mark = to < others.length ? [others[to], "before"] : others.length ? [others.at(-1), "after"] : null;
    return { to, mark };
  }
  el.addEventListener("pointerdown", (e) => {
    const grip = e.target.closest?.("[data-sl-grip]");
    if (!grip || off() || grip.disabled || (e.pointerType === "mouse" && e.button !== 0)) return;
    e.preventDefault();
    grip.focus({ preventScroll: true });
    const li = grip.closest("[data-sl-item]");
    drag = { key: li.getAttribute("data-sl-item"), id: e.pointerId, y: e.clientY, moved: false, to: null, grip };
    try {
      grip.setPointerCapture(e.pointerId);
    } catch {}
  });
  el.addEventListener("pointermove", (e) => {
    if (!drag || e.pointerId !== drag.id) return;
    if (!drag.moved && Math.abs(e.clientY - drag.y) < 4) return;
    if (!drag.moved) {
      drag.moved = true;
      el.setAttribute("data-sl-active", "");
      itemOf(drag.key)?.setAttribute("data-sl-dragging", "");
    }
    clearMarks();
    const { to, mark } = target(e.clientY);
    drag.to = to;
    mark?.[0].setAttribute("data-sl-drop", mark[1]);
    // Near the window's edges, the page scrolls.
    const edge = 40;
    if (e.clientY < edge) scrollBy(0, -10);
    else if (e.clientY > innerHeight - edge) scrollBy(0, 10);
  });
  function end(e, done) {
    if (!drag || e.pointerId !== drag.id) return;
    const d = drag;
    drag = null;
    clearMarks();
    el.removeAttribute("data-sl-active");
    itemOf(d.key)?.removeAttribute("data-sl-dragging");
    if (done && d.moved && d.to !== null) move(d.key, d.to);
  }
  el.addEventListener("pointerup", (e) => end(e, true));
  el.addEventListener("pointercancel", (e) => end(e, false));
  addEventListener("keydown", (e) => {
    if (e.key === "Escape" && drag) {
      e.preventDefault();
      end({ pointerId: drag.id }, false);
    }
  });

  // ---- removing
  el.addEventListener("click", (e) => {
    const x = e.target.closest?.("[data-sl-remove]");
    if (!x || off()) return;
    const li = x.closest("[data-sl-item]"), k = li.getAttribute("data-sl-item");
    const all = list(), at = all.findIndex((v) => keyOf(v) === k);
    if (at < 0) return;
    const next = all.filter((_, i) => i !== at);
    // Focus goes to the next item's grip, or the one before, or the list's note.
    const neighbour = keyOf(next[Math.min(at, next.length - 1)] ?? "");
    api.set(cfg.bind, next);
    api.emit("change", { value: next, removed: all[at] });
    say(L("{name} removed", { name: nameOf(all[at]) }));
    if (next.length) queueMicrotask(() => itemOf(neighbour)?.querySelector("[data-sl-grip]")?.focus());
  });
};
