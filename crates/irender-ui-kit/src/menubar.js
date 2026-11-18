// irender-ui-kit: the `ui-menubar` behaviour, the WAI-ARIA menubar pattern. The bar's
// buttons are one stop in the tab order. Left and Right move along them, wrapping, and
// when a menu is open the next one opens in its place; Down, Enter and Space open a
// menu on its first item (Up on its last); Home and End jump. Left and Right inside an
// open menu move to the next menu too, unless a submenu takes them. While a menu is
// open, hovering another button opens that one. Closing a menu puts focus back on its
// button.
(window.__irenderBehaviors ||= {})["ui-menubar"] = (el) => {
  const triggers = () => [...el.querySelectorAll(":scope > [data-ui-menubar-trigger]")].filter((t) => !t.disabled);
  const menuOf = (t) => document.getElementById(t.getAttribute("popovertarget"));
  const open = () => triggers().find((t) => menuOf(t)?.matches(":popover-open"));
  const rove = (t) => {
    for (const b of triggers()) b.tabIndex = b === t ? 0 : -1;
  };

  function show(t, start, keyboard) {
    const m = menuOf(t);
    if (!m) return;
    const was = open();
    if (was && was !== t) menuOf(was).hidePopover();
    rove(t);
    if (m.matches(":popover-open")) return;
    m.__start = start;
    m.__noFocus = !keyboard;
    m.showPopover();
    if (!keyboard) t.focus();
  }
  // Along the bar from `t`, wrapping.
  const step = (t, n) => {
    const list = triggers();
    return list.at((list.indexOf(t) + n) % list.length);
  };

  el.addEventListener("keydown", (e) => {
    const t = e.target;
    if (t.matches("[data-ui-menubar-trigger]")) {
      const go = { ArrowRight: 1, ArrowLeft: -1 }[e.key];
      if (go) {
        e.preventDefault();
        const next = step(t, go);
        if (open()) show(next, 0, true);
        else {
          rove(next);
          next.focus();
        }
      } else if (e.key === "Home" || e.key === "End") {
        e.preventDefault();
        const next = triggers().at(e.key === "Home" ? 0 : -1);
        rove(next);
        next.focus();
      } else if (e.key === "ArrowDown" || e.key === "ArrowUp" || e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        show(t, e.key === "ArrowUp" ? -1 : 0, true);
      }
      return;
    }
    // Inside a menu: Left and Right go to the neighbouring menus.
    const menu = t.closest("[data-ui-menubar-menu]");
    if (!menu || (e.key !== "ArrowLeft" && e.key !== "ArrowRight")) return;
    const own = triggers().find((b) => menuOf(b) === menu);
    if (!own) return;
    e.preventDefault();
    show(step(own, e.key === "ArrowRight" ? 1 : -1), 0, true);
  });

  // With a menu open, the pointer moves the open menu along the bar.
  el.addEventListener("mouseover", (e) => {
    const t = e.target.closest?.("[data-ui-menubar-trigger]");
    const was = open();
    if (t && was && was !== t && !t.disabled) show(t, 0, false);
  });
  el.addEventListener("focusin", (e) => {
    if (e.target.matches("[data-ui-menubar-trigger]")) rove(e.target);
  });

  for (const t of triggers()) {
    const m = menuOf(t);
    m?.addEventListener("toggle", (e) => {
      t.setAttribute("aria-expanded", String(e.newState === "open"));
      if (e.newState === "closed") {
        setTimeout(() => {
          const now = document.activeElement;
          // Unless the click that closed it went elsewhere, focus goes back to the button.
          // The browser may have put it on another of the bar's buttons, where the menu
          // was opened from.
          if (!open() && (!now || now === document.body || el.contains(now))) t.focus();
        });
      }
    });
  }
  return {};
};
