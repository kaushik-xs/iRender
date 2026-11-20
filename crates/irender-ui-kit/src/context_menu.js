// irender-ui-kit: the `ui-context-menu` behaviour. A right-click on the area (or Shift+F10,
// or the menu key, on something focused in it, or a long press by touch) writes what was clicked to `bind` and
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
    // What it opens on hears first, so a data table can select the row (and `bind`'s
    // context and the items see that).
    target?.dispatchEvent?.(new CustomEvent("irender:context", { bubbles: true }));
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

  // A long press (500ms) by touch opens it too: iOS sends no `contextmenu` for one.
  // Moving first is a scroll; the browser's own event after it opens nothing more.
  let press = null, pressed = -Infinity;
  el.addEventListener("pointerdown", (e) => {
    if (e.pointerType !== "touch" || !e.isPrimary || menu.contains(e.target)) return;
    clearTimeout(press?.timer);
    press = { x: e.clientX, y: e.clientY, target: e.target, id: e.pointerId };
    press.timer = setTimeout(() => {
      if (press && open(press.x, press.y, press.target)) {
        pressed = performance.now();
        navigator.vibrate?.(8);
      }
      press = null;
    }, 500);
  });
  const unpress = (e) => {
    if (!press || e.pointerId !== press.id) return;
    if (e.type !== "pointermove" || Math.hypot(e.clientX - press.x, e.clientY - press.y) > 8) {
      clearTimeout(press.timer);
      press = null;
    }
  };
  for (const t of ["pointermove", "pointerup", "pointercancel"]) el.addEventListener(t, unpress);
  // The tap that ends a long press isn't a click on what was under it.
  el.addEventListener("click", (e) => performance.now() - pressed < 700 && !menu.contains(e.target) && (e.preventDefault(), e.stopPropagation()), true);
  el.addEventListener("contextmenu", (e) => {
    if (menu.contains(e.target)) return e.preventDefault();
    // Opened by a long press just now: this one (Android sends it) is the same press.
    if (performance.now() - pressed < 1000) return e.preventDefault();
    clearTimeout(press?.timer);
    press = null;
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
