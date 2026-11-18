// irender-ui-kit: the `ui-context-menu` behaviour. A right-click on the area (or Shift+F10,
// or the menu key, on something focused in it) writes what was clicked to `bind` and
// opens the menu at the pointer, kept on screen. The menu is a `ui-dropdown-menu`'s, so
// it focuses its first item and handles the keys; closing puts focus back where it was.
(window.__irenderBehaviors ||= {})["ui-context-menu"] = (el, api) => {
  const menu = el.querySelector(":scope > [role=menu]");
  let back = null, keyed = null;

  // What was clicked: the nearest `data-context` (JSON when it parses), or a value a
  // component left on an element (a data table's row).
  function contextOf(t) {
    for (let n = t; n && n !== el; n = n.parentElement) {
      if (n.__irenderContext !== undefined) return n.__irenderContext;
      if (n.hasAttribute?.("data-context")) {
        const v = n.getAttribute("data-context");
        try {
          return JSON.parse(v);
        } catch {
          return v;
        }
      }
    }
    return null;
  }

  function open(x, y, target) {
    if (el.hasAttribute("data-disabled")) return false;
    const context = contextOf(target);
    if (api.config.bind) api.set(api.config.bind, context);
    api.emit("contextopen", { context });
    back = document.activeElement;
    if (menu.matches(":popover-open")) menu.hidePopover();
    menu.style.left = "0px";
    menu.style.top = "0px";
    menu.showPopover();
    const r = menu.getBoundingClientRect();
    const left = x + r.width > innerWidth - 4 ? Math.max(4, x - r.width) : x;
    const top = y + r.height > innerHeight - 4 ? Math.max(4, y - r.height) : y;
    menu.style.left = left + "px";
    menu.style.top = top + "px";
    return true;
  }

  el.addEventListener("contextmenu", (e) => {
    if (menu.contains(e.target)) return e.preventDefault();
    // Shift+F10 was handled as a key; some browsers follow it with this event, once.
    if (keyed && keyed.target === e.target && e.timeStamp - keyed.at < 500) {
      keyed = null;
      return e.preventDefault();
    }
    // The menu key sends this event at the focused element, without a position.
    let { clientX: x, clientY: y } = e;
    if (!x && !y) {
      const r = e.target.getBoundingClientRect();
      [x, y] = [r.left + 8, r.bottom];
    }
    if (open(x, y, e.target)) e.preventDefault();
  });
  el.addEventListener("keydown", (e) => {
    if (e.key !== "F10" || !e.shiftKey || menu.contains(e.target)) return;
    const r = e.target.getBoundingClientRect();
    if (open(r.left + 8, r.bottom, e.target)) {
      e.preventDefault();
      keyed = { target: e.target, at: e.timeStamp };
    }
  });
  // Closing returns focus to where it was, unless the click that closed it went elsewhere.
  menu.addEventListener("toggle", (e) => {
    if (e.newState !== "closed") return;
    setTimeout(() => {
      const now = document.activeElement;
      if (back?.isConnected && (!now || now === document.body || menu.contains(now))) back.focus({ preventScroll: true });
    });
  });
  return {};
};
