// irender-ui-kit: the behaviour of every menu (dropdown, context, menubar menus and
// submenus), the WAI-ARIA menu pattern. Opening a menu focuses its first item; Up and
// Down move (and wrap), Home and End jump, and typing jumps to an item starting with the
// typed text. On a dropdown menu's button, Down or Up opens it on the first or last item.
// Tab closes the menu and moves on. Escape and choosing an item close it through the
// Popover API, which puts focus back on the button.
//
// Submenus open beside their item on hover, Enter, Space or Right, and close on Left
// or Escape, focus going back to the item. Checkbox items flip their state key and radio
// items write their value.
(window.__irenderBehaviors ||= {})["ui-dropdown-menu"] = (el, api) => {
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
    } else if (e.key === "ArrowRight" && e.target.hasAttribute("data-ui-sub-trigger")) {
      e.preventDefault();
      e.stopPropagation();
      openSub(e.target, true);
    } else if (e.key === "ArrowLeft" && isSub) {
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
  el.addEventListener("mouseover", (e) => {
    const item = e.target.closest?.(ITEMS);
    if (!item || item.closest("[role=menu]") !== el) return;
    if (item.hasAttribute("data-ui-sub-trigger")) openSub(item, false);
    else for (const other of items()) if (other.hasAttribute("data-ui-sub-trigger") && sub(other)?.matches(":popover-open")) sub(other).hidePopover();
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
