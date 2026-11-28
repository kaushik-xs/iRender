// irender-ui-kit: the `ui-sortable-list` behaviour. The items are the runtime's keyed
// list of the state key, so this only works out the new order and writes it; the runtime
// moves the items. Each item's grip drags it (mouse, touch or pen alike, through pointer
// events: the grip doesn't scroll), and from the keyboard Space picks it up, the arrows
// move it and Space puts it down (Escape puts it back), or Alt and the arrows move it
// straight away. Lists with the same `group` take items from each other: dragged across,
// or, picked up, ←/→ to the list before or after. While an item's dragged the others make
// way for it, and moved items glide to their places (not with reduced motion).
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
  const still = () => matchMedia("(prefers-reduced-motion: reduce)").matches;
  // Where each item is, before a change, so they can glide from there after it.
  function flip() {
    if (still()) return () => {};
    const was = new Map(items().map((li) => [li.getAttribute("data-sl-item"), li.getBoundingClientRect().top]));
    return () => queueMicrotask(() => {
      for (const li of items()) {
        const before = was.get(li.getAttribute("data-sl-item"));
        if (before === undefined) continue;
        const dy = before - li.getBoundingClientRect().top;
        if (!dy) continue;
        // Drawn where it was, then (after a layout) let go to glide to where it is.
        li.style.transition = "none";
        li.style.transform = `translateY(${dy}px)`;
        void li.offsetHeight;
        li.style.transition = "transform .18s ease";
        li.style.transform = "";
        li.addEventListener("transitionend", () => (li.style.transition = ""), { once: true });
      }
    });
  }
  // This list's handle, for the others in its group; and theirs, in the page's order.
  el.__sl = { cfg, api, el, list, keyOf, nameOf, take: (it, to) => take(it, to) };
  const group = () => (cfg.group ? [...document.querySelectorAll("[data-sl-group]")].filter((x) => x.getAttribute("data-sl-group") === cfg.group && x.__sl) : [el]);
  // An item from another list of the group, put in at place `to`.
  function take(it, to) {
    const all = list(), next = all.slice();
    to = Math.max(0, Math.min(all.length, to));
    next.splice(to, 0, it);
    const glide = flip();
    api.set(cfg.bind, next);
    api.emit("change", { value: next, added: it, to });
    glide();
    return to;
  }
  // The item `k` across to `other` (another list's handle) at place `to`.
  function across(k, other, to) {
    const all = list(), from = all.findIndex((v) => keyOf(v) === k);
    if (from < 0 || other.el === el) return false;
    const it = all[from], next = all.filter((_, i) => i !== from);
    const glide = flip();
    api.set(cfg.bind, next);
    api.emit("change", { value: next, removed: it, to: other.cfg.bind });
    glide();
    const at = other.take(it, to);
    api.emit("transfer", { item: it, from: cfg.bind, to: other.cfg.bind, at });
    other.api.emit("transfer", { item: it, from: cfg.bind, to: other.cfg.bind, at });
    // Labels: "{name} moved to {list}, {n} of {total}"
    const name = other.el.querySelector("[data-sl-items]")?.getAttribute("aria-label") ?? other.cfg.bind;
    const say2 = other.el.querySelector("[data-sl-live]");
    if (say2) say2.textContent = L("{name} moved to {list}, {n} of {total}", { name: nameOf(it), list: name, n: at + 1, total: other.list().length });
    return true;
  }

  // The list with the item `k` at place `to`; written, said, and its grip focused again.
  function move(k, to, quiet) {
    const all = list(), from = all.findIndex((v) => keyOf(v) === k);
    to = Math.max(0, Math.min(all.length - 1, to));
    if (from < 0 || from === to) return false;
    const next = all.slice();
    const [it] = next.splice(from, 1);
    next.splice(to, 0, it);
    moving = true;
    const glide = flip();
    api.set(cfg.bind, next);
    api.emit("change", { value: next, moved: it, from, to });
    glide();
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
    } else if ((e.key === "ArrowLeft" || e.key === "ArrowRight") && picked === k && cfg.group) {
      // To the list before or after in the group, at the same place (or its end); it's
      // still picked up there.
      e.preventDefault();
      const lists = group(), i = lists.indexOf(el), other = lists[i + (e.key === "ArrowRight" ? 1 : -1)]?.__sl;
      if (!other) return;
      picked = null;
      itemOf(k)?.removeAttribute("data-sl-picked");
      if (across(k, other, at)) queueMicrotask(() => other.el.__slPick?.(k));
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
  // Picked up here after coming across from another list.
  el.__slPick = (k) => {
    const li = itemOf(k);
    if (!li) return;
    picked = k;
    pickedFrom = list().findIndex((v) => keyOf(v) === k);
    li.setAttribute("data-sl-picked", "");
    moving = true;
    li.querySelector("[data-sl-grip]")?.focus();
    moving = false;
  };
  // Leaving a picked item puts it down where it is.
  el.addEventListener("focusout", (e) => {
    if (picked !== null && !moving && !el.contains(e.relatedTarget)) drop(false);
  });

  // ---- dragging a grip
  let drag = null;
  function clearMarks() {
    for (const x of group()) for (const li of x.querySelectorAll(":scope [data-sl-items] > [data-sl-item]")) {
      li.removeAttribute("data-sl-drop");
      if (li.getAttribute("data-sl-item") !== drag?.key || x !== el) li.style.transform = "";
    }
  }
  // Where the pointer would put the dragged item: the place, and the item to mark, in
  // `into` (this list, or another of the group).
  function target(y, into = el) {
    const others = [...into.querySelectorAll(":scope [data-sl-items] > [data-sl-item]")].filter((li) => into !== el || li.getAttribute("data-sl-item") !== drag.key);
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
    drag = { key: li.getAttribute("data-sl-item"), id: e.pointerId, x: e.clientX, y: e.clientY, moved: false, to: null, grip, into: null };
    try {
      grip.setPointerCapture(e.pointerId);
    } catch {}
  });
  el.addEventListener("pointermove", (e) => {
    if (!drag || e.pointerId !== drag.id) return;
    if (!drag.moved && Math.abs(e.clientY - drag.y) < 4 && Math.abs(e.clientX - (drag.x ?? e.clientX)) < 4) return;
    const me = itemOf(drag.key);
    if (!drag.moved) {
      drag.moved = true;
      el.setAttribute("data-sl-active", "");
      me?.setAttribute("data-sl-dragging", "");
      drag.h = (me?.getBoundingClientRect().height ?? 0) + parseFloat(getComputedStyle(el.querySelector("[data-sl-items]")).rowGap || "0");
      drag.from = items().indexOf(me);
      // The others' middles as they'd be without it, from the list's top, by their layout
      // (not where they're drawn as they slide): where it lands is worked out from these.
      const ol = el.querySelector("[data-sl-items]");
      drag.mids = items().filter((li) => li !== me).map((li, j) => li.offsetTop - ol.offsetTop - (j >= drag.from ? drag.h : 0) + li.offsetHeight / 2);
    }
    clearMarks();
    // Over another list of the group: it goes there, shown by a line.
    const over = cfg.group ? group().find((x) => x !== el && (() => { const r = x.getBoundingClientRect(); return e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom; })()) : null;
    drag.into = over?.__sl ?? null;
    if (me) me.style.transform = `translate(${cfg.group ? e.clientX - drag.x : 0}px, ${e.clientY - drag.y}px)`;
    if (over) {
      const { to, mark } = target(e.clientY, over);
      drag.to = to;
      mark?.[0].setAttribute("data-sl-drop", mark[1]);
    } else {
      const y = e.clientY - el.querySelector("[data-sl-items]").getBoundingClientRect().top;
      let to = drag.mids.findIndex((m) => y < m);
      if (to < 0) to = drag.mids.length;
      drag.to = to;
      const others = items().filter((li) => li !== me);
      if (!still()) {
        // The others make way: those between where it was and where it'll go slide by its height.
        others.forEach((li, j) => {
          const shift = j >= drag.from && j < to ? -drag.h : j >= to && j < drag.from ? drag.h : 0;
          li.style.transform = shift ? `translateY(${shift}px)` : "";
        });
      } else if (others.length) (to < others.length ? others[to] : others.at(-1)).setAttribute("data-sl-drop", to < others.length ? "before" : "after");
    }
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
    const me = itemOf(d.key);
    me?.removeAttribute("data-sl-dragging");
    // Put back where it was drawn, all at once: the move glides from there.
    for (const li of items()) (li.style.transition = "none"), (li.style.transform = "");
    void el.offsetHeight;
    for (const li of items()) li.style.transition = "";
    if (done && d.moved && d.to !== null) {
      if (d.into) across(d.key, d.into, d.to);
      else move(d.key, d.to);
    }
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
