// irender-ui-kit: the `ui-dropdown-menu` behaviour, the WAI-ARIA menu button pattern.
// Opening the menu focuses its first item; Up and Down move (and wrap), Home and End
// jump, and typing jumps to an item starting with the typed text. On the menu's
// button, Down or Up opens it on the first or last item. Tab closes the menu and moves
// on. Escape and choosing an item close it through the Popover API, which puts focus
// back on the button.
(window.__irenderBehaviors ||= {})["ui-dropdown-menu"] = (el) => {
  const items = () => [...el.querySelectorAll("[role=menuitem]")].filter((i) => !i.disabled);
  const focus = (list, i) => list.at(i)?.focus();
  let start = 0; // where opening puts focus: 0 = first item, -1 = last
  let typed = "";
  let typedAt = 0;

  el.addEventListener("toggle", (e) => {
    if (e.newState !== "open") return;
    focus(items(), start);
    start = 0;
  });

  el.addEventListener("keydown", (e) => {
    if (e.altKey || e.ctrlKey || e.metaKey) return;
    const list = items();
    const at = list.indexOf(document.activeElement);
    const move = { ArrowDown: at + 1, ArrowUp: at < 0 ? -1 : at - 1, Home: 0, End: -1 }[e.key];
    if (move !== undefined) {
      e.preventDefault();
      focus(list, move % list.length);
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

  // The button that opens it, found when a key is pressed: the menu's id can change
  // in a list the browser rebuilds.
  document.addEventListener("keydown", (e) => {
    if ((e.key !== "ArrowDown" && e.key !== "ArrowUp") || el.matches(":popover-open")) return;
    const trigger = e.target.closest?.("[popovertarget]");
    if (!trigger || trigger.getAttribute("popovertarget") !== el.id) return;
    e.preventDefault();
    start = e.key === "ArrowUp" ? -1 : 0;
    trigger.click();
  });
};
