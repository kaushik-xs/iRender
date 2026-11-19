// irender-ui-kit: the behaviour of every menu (dropdown, context, menubar menus and
// submenus), the WAI-ARIA menu pattern. Opening a menu focuses its first item; Up and
// Down move (and wrap), Home and End jump, and typing jumps to an item starting with the
// typed text. On a dropdown menu's button, Down or Up opens it on the first or last item.
// Tab closes the menu and moves on. Escape and choosing an item close it through the
// Popover API, which puts focus back on the button.
//
// Submenus open beside their item on hover, Enter, Space or Right, and close on Left
// or Escape, focus going back to the item. Heading for an open submenu across other
// items doesn't close it. Checkbox items flip their state key and radio
// items write their value.
(window.__irenderBehaviors ||= {})["ui-dropdown-menu"] = (el, api) => {
  // Right to left, Left and Right swap: the next one is on the left.
  const side = (k) => (getComputedStyle(el).direction === "rtl" ? { ArrowLeft: "ArrowRight", ArrowRight: "ArrowLeft" }[k] ?? k : k);

  const ITEMS = "[role=menuitem],[role=menuitemcheckbox],[role=menuitemradio]";
  // This menu's own items, not those of a submenu inside it.
  const items = () => [...el.querySelectorAll(ITEMS)].filter((i) => !i.disabled && i.closest("[role=menu]") === el);
  const focus = (list, i) => list.at(i)?.focus();
  const sub = (trigger) => trigger && document.getElementById(trigger.getAttribute("aria-controls"));
  const isSub = el.hasAttribute("data-ui-submenu");
  let typed = "";
  let typedAt = 0;

  el.addEventListener("toggle", (e) => {
    const trigger = isSub ? el.parentElement?.querySelector(`[aria-controls="${CSS.escape(el.id)}"]`) : null;
    trigger?.setAttribute("aria-expanded", String(e.newState === "open"));
    if (e.newState !== "open") {
      // A submenu closed from the keyboard returns focus to its item.
      if (isSub && (el.contains(document.activeElement) || document.activeElement === document.body)) trigger?.focus();
      return;
    }
    // A menu opened by hovering leaves focus where it is.
    if (el.__noFocus) {
      el.__noFocus = false;
      return;
    }
    focus(items(), el.__start ?? 0);
    el.__start = null;
  });

  function openSub(trigger, viaKeyboard) {
    const m = sub(trigger);
    if (!m || trigger.disabled) return;
    for (const other of items()) if (other !== trigger && other.hasAttribute("data-ui-sub-trigger")) sub(other)?.matches(":popover-open") && sub(other).hidePopover();
    if (m.matches(":popover-open")) {
      if (viaKeyboard) focus([...m.querySelectorAll(ITEMS)].filter((i) => i.closest("[role=menu]") === m && !i.disabled), 0);
      return;
    }
    m.__noFocus = !viaKeyboard;
    m.showPopover();
  }

  el.addEventListener("keydown", (e) => {
    // Keys in a submenu are its own.
    if (e.target.closest("[role=menu]") !== el) return;
    if (e.altKey || e.ctrlKey || e.metaKey) return;
    const list = items();
    const at = list.indexOf(document.activeElement);
    const move = { ArrowDown: at + 1, ArrowUp: at < 0 ? -1 : at - 1, Home: 0, End: -1 }[e.key];
    if (move !== undefined) {
      e.preventDefault();
      focus(list, move % list.length);
    } else if (side(e.key) === "ArrowRight" && e.target.hasAttribute("data-ui-sub-trigger")) {
      e.preventDefault();
      e.stopPropagation();
      openSub(e.target, true);
    } else if (side(e.key) === "ArrowLeft" && isSub) {
      e.preventDefault();
      e.stopPropagation();
      el.hidePopover();
    } else if (e.key === "Tab") {
      el.hidePopover();
    } else if (e.key.length === 1 && e.key !== " ") {
      // Typing again soon extends the search; one letter, even pressed repeatedly,
      // steps through the items starting with it.
      typed = e.timeStamp - typedAt < 500 ? typed + e.key.toLowerCase() : e.key.toLowerCase();
      typedAt = e.timeStamp;
      const step = [...typed].every((c) => c === typed[0]);
      const search = step ? typed[0] : typed;
      const from = step ? at + 1 : Math.max(at, 0);
      const order = [...list.slice(from), ...list.slice(0, from)];
      order.find((i) => i.textContent.trim().toLowerCase().startsWith(search))?.focus();
    }
  });

  el.addEventListener("click", (e) => {
    const item = e.target.closest(ITEMS);
    if (!item || item.closest("[role=menu]") !== el) return;
    if (item.hasAttribute("data-ui-sub-trigger")) return openSub(item, e.detail === 0);
    // Checkbox and radio items keep their state in state.
    const key = item.getAttribute("data-ui-toggle");
    if (key && api) api.set(key, !api.state[key]);
    const pick = item.getAttribute("data-ui-pick");
    if (pick && api) api.set(pick, JSON.parse(item.getAttribute("data-ui-value")));
  });

  // Hovering an item opens its submenu, or closes the one open beside another item.
  function hover(item) {
    if (item.hasAttribute("data-ui-sub-trigger")) openSub(item, false);
    else for (const other of items()) if (other.hasAttribute("data-ui-sub-trigger") && sub(other)?.matches(":popover-open")) sub(other).hidePopover();
  }
  // The way to an open submenu: leaving its item, the pointer gets a triangle from where
  // it left to the submenu's near edge. While it moves inside, the items it crosses wait
  // (up to 400ms) before acting, so heading for the submenu diagonally doesn't close it.
  let grace = null, waiting = null, timer = 0, over = null;
  const inside = ([a, b, c], x, y) => {
    const side = (p, q) => (q[0] - p[0]) * (y - p[1]) - (q[1] - p[1]) * (x - p[0]);
    const d1 = side(a, b), d2 = side(b, c), d3 = side(c, a);
    return !((d1 < 0 || d2 < 0 || d3 < 0) && (d1 > 0 || d2 > 0 || d3 > 0));
  };
  function settle() {
    clearTimeout(timer);
    const item = waiting;
    grace = waiting = null;
    // The item the pointer is on now acts.
    if (item && item === over) hover(item);
  }
  el.addEventListener("mouseout", (e) => {
    if (e.relatedTarget?.closest?.(ITEMS)?.closest("[role=menu]") !== el) over = null;
    const item = e.target.closest?.("[data-ui-sub-trigger]");
    const m = item && item.closest("[role=menu]") === el ? sub(item) : null;
    if (!m?.matches(":popover-open") || item.contains(e.relatedTarget) || m.contains(e.relatedTarget)) return;
    const r = m.getBoundingClientRect(), x = e.clientX, y = e.clientY;
    const edge = r.left >= x ? r.left : r.right, back = r.left >= x ? -4 : 4;
    grace = { m, triangle: [[x + back, y], [edge, r.top - 8], [edge, r.bottom + 8]], until: e.timeStamp + 400 };
  });
  el.addEventListener("mouseover", (e) => {
    // Reached the submenu: nothing else acts.
    if (grace?.m.contains(e.target)) {
      grace = waiting = over = null;
      return clearTimeout(timer);
    }
    const item = e.target.closest?.(ITEMS);
    if (!item || item.closest("[role=menu]") !== el) return;
    over = item;
    if (grace) {
      if (e.timeStamp < grace.until && inside(grace.triangle, e.clientX, e.clientY)) {
        waiting = item;
        clearTimeout(timer);
        timer = setTimeout(settle, Math.max(0, grace.until - e.timeStamp));
        return;
      }
      grace = waiting = null;
      clearTimeout(timer);
    }
    hover(item);
  });
  // Out of the triangle (or into the submenu): what's under the pointer acts at once.
  el.addEventListener("mousemove", (e) => {
    if (!grace) return;
    if (grace.m.contains(e.target)) {
      grace = waiting = null;
      return clearTimeout(timer);
    }
    if (!inside(grace.triangle, e.clientX, e.clientY)) settle();
  });

  // The button that opens it, found when a key is pressed: the menu's id can change
  // in a list the browser rebuilds.
  if (!isSub)
    document.addEventListener("keydown", (e) => {
      if ((e.key !== "ArrowDown" && e.key !== "ArrowUp") || el.matches(":popover-open")) return;
      const trigger = e.target.closest?.("[popovertarget]");
      if (!trigger || trigger.getAttribute("popovertarget") !== el.id || trigger.hasAttribute("data-ui-menubar-trigger")) return;
      e.preventDefault();
      el.__start = e.key === "ArrowUp" ? -1 : 0;
      trigger.click();
    });
  return {};
};
