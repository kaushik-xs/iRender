// irender-ui-kit: the `ui-tabs` behaviour, on the tab list. The list is one stop for the
// Tab key, at the selected tab; the arrow keys (and Home, End) move to another tab and
// select it, as a click does. When there are more tabs than fit, the selected one and the
// focused one are kept in view, and the sides with more get `data-more` for the fade.
// Which tab is selected is state, so the runtime redraws the rest.
(window.__irenderBehaviors ||= {})["ui-tabs"] = (el, api) => {
  const tabs = () => [...el.children].filter((t) => t.getAttribute("role") === "tab");
  const selected = () => tabs().find((t) => t.getAttribute("aria-selected") === "true");
  // Room for the fade beside a tab brought into view.
  const ROOM = 40;

  function more() {
    const most = el.scrollWidth - el.clientWidth, at = Math.abs(el.scrollLeft);
    const rtl = getComputedStyle(el).direction === "rtl";
    const before = most > 1 && at > 1, after = most > 1 && at < most - 1;
    const left = rtl ? after : before, right = rtl ? before : after;
    const sides = left && right ? "both" : left ? "left" : right ? "right" : null;
    if (sides) el.setAttribute("data-more", sides);
    else el.removeAttribute("data-more");
  }
  // Within the list only: the page stays where it is.
  function reveal(tab) {
    if (tab && el.scrollWidth > el.clientWidth + 1) {
      const t = tab.getBoundingClientRect(), box = el.getBoundingClientRect();
      const room = Math.max(0, Math.min(ROOM, (box.width - t.width) / 2));
      if (t.left < box.left + room) el.scrollLeft -= box.left + room - t.left;
      else if (t.right > box.right - room) el.scrollLeft += t.right - (box.right - room);
    }
    more();
  }

  let last;
  function update() {
    const list = tabs(), now = selected();
    // None selected (the state holds no panel's value): the first takes the Tab key.
    for (const t of list) t.tabIndex = t === (now ?? list[0]) ? 0 : -1;
    if (now !== last) {
      last = now;
      reveal(now);
    }
  }

  el.addEventListener("keydown", (e) => {
    if (e.altKey || e.ctrlKey || e.metaKey || e.shiftKey) return;
    const list = tabs(), i = list.indexOf(e.target);
    if (i < 0) return;
    const step = getComputedStyle(el).direction === "rtl" ? -1 : 1;
    // In a column, up and down (on a narrow screen the column is a row again).
    const column = api.config.vertical && getComputedStyle(el).flexDirection === "column";
    const to = { ...(column ? { ArrowDown: i + 1, ArrowUp: i - 1 } : { ArrowRight: i + step, ArrowLeft: i - step }), Home: 0, End: list.length - 1 }[e.key];
    if (to === undefined) return;
    e.preventDefault();
    const next = list[(to + list.length) % list.length];
    next.focus({ preventScroll: true });
    // Only moved to: Enter or Space, as on any button, opens it.
    if (!api.config.manual) next.click();
  });
  // By keyboard: scrolling under a pressed pointer would lose its click, which selects
  // the tab and so reveals it.
  el.addEventListener("focusin", (e) => e.target.matches("[role=tab]:focus-visible") && reveal(e.target));
  el.addEventListener("scroll", more, { passive: true });
  // Shown after being hidden (in a dialog, say), or a different width.
  new ResizeObserver(() => reveal(tabs().find((t) => t === document.activeElement) ?? selected())).observe(el);
  update();
  return { update };
};
