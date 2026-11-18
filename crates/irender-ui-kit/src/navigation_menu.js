// irender-ui-kit: the `ui-navigation-menu` behaviour, the disclosure navigation pattern.
// An item's button shows or hides its panel; the pointer opens it after a moment over the
// button and closes it a moment after leaving both. Down on the button opens the panel
// and moves into it, Escape closes it and returns to the button, and Left and Right move
// along the bar. Only one panel is open at a time.
(window.__irenderBehaviors ||= {})["ui-navigation-menu"] = (el) => {
  const tops = () => [...el.querySelectorAll(":scope > ul > li > :is([data-ui-nav-trigger], a)")];
  const panelOf = (t) => document.getElementById(t.getAttribute("aria-controls"));
  const OPEN = 120, CLOSE = 250;
  let timer = null;
  const later = (ms, f) => {
    clearTimeout(timer);
    timer = setTimeout(f, ms);
  };
  const show = (t, focusIn) => {
    const p = panelOf(t);
    if (!p) return;
    if (!p.matches(":popover-open")) p.showPopover();
    if (focusIn) p.querySelector("a, button, input, [tabindex]")?.focus();
  };
  const hide = (t) => {
    const p = panelOf(t);
    if (p?.matches(":popover-open")) p.hidePopover();
  };

  el.addEventListener("click", (e) => {
    const t = e.target.closest("[data-ui-nav-trigger]");
    if (!t) return;
    clearTimeout(timer);
    panelOf(t).matches(":popover-open") ? hide(t) : show(t, false);
  });
  el.addEventListener("keydown", (e) => {
    const t = e.target;
    const list = tops(), at = list.indexOf(t);
    if (at >= 0 && (e.key === "ArrowRight" || e.key === "ArrowLeft")) {
      e.preventDefault();
      list.at((at + (e.key === "ArrowRight" ? 1 : -1)) % list.length).focus();
    } else if (t.matches("[data-ui-nav-trigger]") && e.key === "ArrowDown") {
      e.preventDefault();
      show(t, true);
    }
  });

  for (const t of el.querySelectorAll("[data-ui-nav-trigger]")) {
    const p = panelOf(t);
    if (!p) continue;
    const enter = () => later(p.matches(":popover-open") ? 0 : OPEN, () => show(t, false));
    const leave = () => later(CLOSE, () => hide(t));
    t.addEventListener("pointerenter", (e) => e.pointerType === "mouse" && enter());
    t.addEventListener("pointerleave", (e) => e.pointerType === "mouse" && leave());
    p.addEventListener("pointerenter", () => clearTimeout(timer));
    p.addEventListener("pointerleave", (e) => e.pointerType === "mouse" && leave());
    p.addEventListener("toggle", (e) => {
      t.setAttribute("aria-expanded", String(e.newState === "open"));
      if (e.newState === "closed") {
        setTimeout(() => {
          const now = document.activeElement;
          if (!now || now === document.body || p.contains(now)) t.focus({ preventScroll: true });
        });
      }
    });
    // Escape in the panel closes it (the Popover API does) and comes back to the button.
    p.addEventListener("keydown", (e) => {
      if (e.key === "Escape") {
        e.preventDefault();
        hide(t);
        t.focus();
      }
    });
  }
  return {};
};
