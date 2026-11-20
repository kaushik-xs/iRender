// irender-ui-kit: the `ui-sidebar` behaviour, on a `ui-sidebar-layout` with `bind`.
// On a phone (below 768px) the panel is a sheet over the page: the trigger opens it
// whatever `bind` says, and a tap on the backdrop, Escape or following a link closes
// it. While it's open the page behind is inert, Tab cycles inside the panel, and focus
// goes into the panel and back to the trigger. A touch swipe toward its edge closes it. ⌘B / Ctrl+B toggles the sidebar: the sheet on a phone, `bind`
// otherwise. With several layouts on a page, the first one takes the shortcut. With
// `remember`, open or closed is kept in localStorage and restored as the page loads.
(window.__irenderBehaviors ||= {})["ui-sidebar"] = (el, api) => {
  const { key } = api.config;
  const phone = matchMedia("(max-width: 767px)");
  const panel = el.querySelector(":scope > aside");
  const main = el.querySelector(":scope > main");
  let state = api.state;
  let returnTo = null;

  const own = (node) => node && node.closest("[data-sidebar-layout]") === el;
  const triggers = () => [...el.querySelectorAll("[data-sidebar-trigger]")].filter(own);
  const sheetOpen = () => el.hasAttribute("data-mobile-open");
  const expanded = () => String(phone.matches ? sheetOpen() : !!state[key]);
  const sync = () => triggers().forEach((t) => t.setAttribute("aria-expanded", expanded()));

  const setSheet = (open, { restoreFocus = true } = {}) => {
    if (open === sheetOpen()) return;
    el.toggleAttribute("data-mobile-open", open);
    if (main) main.inert = open;
    sync();
    if (open) {
      returnTo = document.activeElement;
      panel?.querySelector("a[href], button, summary, input")?.focus();
    } else if (restoreFocus && returnTo && panel?.contains(document.activeElement)) {
      // Back to the trigger, or focus would be left in a hidden panel.
      returnTo.focus();
    }
    if (!open) returnTo = null;
  };
  const toggle = () => (phone.matches ? setSheet(!sheetOpen()) : api.set(key, !state[key]));

  // Capture: on a phone the trigger opens the sheet before its own click handler would
  // change `bind`.
  el.addEventListener(
    "click",
    (e) => {
      const trigger = e.target.closest?.("[data-sidebar-trigger]");
      if (trigger && own(trigger) && phone.matches) {
        e.stopImmediatePropagation();
        e.preventDefault();
        setSheet(!sheetOpen());
      } else if (sheetOpen() && e.target === el) {
        setSheet(false); // the backdrop is the layout's ::before
      } else if (sheetOpen() && e.target.closest?.("a[href]") && panel?.contains(e.target)) {
        setSheet(false);
      }
    },
    true,
  );

  // Tab and Shift+Tab go round the panel's controls, not out to the browser's own.
  const focusables = () =>
    [...(panel?.querySelectorAll("a[href], button:not([disabled]), summary, input:not([disabled]), select, textarea, [tabindex]:not([tabindex='-1'])") ?? [])].filter(
      (n) => n.getClientRects().length && !n.closest("[inert], details:not([open]) > :not(summary)"),
    );
  document.addEventListener("keydown", (e) => {
    if (e.key === "Tab" && sheetOpen() && !e.altKey && !e.ctrlKey && !e.metaKey) {
      const all = focusables();
      if (!all.length) return;
      const at = all.indexOf(document.activeElement);
      const next = e.shiftKey ? (at <= 0 ? all.at(-1) : null) : at === -1 || at === all.length - 1 ? all[0] : null;
      if (next) {
        e.preventDefault();
        next.focus();
      }
    } else if (e.key === "Escape" && sheetOpen()) {
      e.preventDefault();
      setSheet(false);
    } else if (e.key.toLowerCase() === "b" && (e.metaKey || e.ctrlKey) && !e.altKey && !e.shiftKey) {
      if (document.querySelector("[data-sidebar-layout][data-i-x]") !== el || e.target.isContentEditable) return;
      e.preventDefault();
      toggle();
    }
  });

  // ---- swiping the sheet shut: touch, toward the edge it hides behind
  let drag = null;
  // +1 when it closes rightward (a right-hand sidebar, or a left one right to left).
  const out = () => ((el.getAttribute("data-side") === "right") !== (getComputedStyle(el).direction === "rtl") ? 1 : -1);
  panel?.addEventListener("pointerdown", (e) => {
    if (!sheetOpen() || e.pointerType === "mouse" || e.target.closest("input, textarea, select, [contenteditable]")) return;
    drag = { x: e.clientX, y: e.clientY, id: e.pointerId, moving: false, d: 0, track: [[e.timeStamp, 0]] };
  });
  panel?.addEventListener("pointermove", (e) => {
    if (!drag || e.pointerId !== drag.id) return;
    const d = (e.clientX - drag.x) * out(), dy = e.clientY - drag.y;
    if (!drag.moving) {
      // A drag once it's clearly toward the edge; up and down is a scroll.
      if (Math.abs(dy) > 8 || d < -8) return void (drag = null);
      if (d < 8) return;
      drag.moving = true;
      panel.setAttribute("data-dragging", "");
      try {
        panel.setPointerCapture(e.pointerId);
      } catch {}
    }
    drag.d = Math.max(0, d);
    drag.track.push([e.timeStamp, drag.d]);
    if (drag.track.length > 6) drag.track.shift();
    panel.style.transform = `translateX(${drag.d * out()}px)`;
  });
  const endDrag = (e, cancel) => {
    if (!drag || e.pointerId !== drag.id) return;
    const { d, moving, track } = drag;
    drag = null;
    if (!moving) return;
    panel.removeAttribute("data-dragging");
    panel.style.transform = "";
    const [t0, d0] = track[0], [t1, d1] = track[track.length - 1];
    const speed = t1 > t0 ? (d1 - d0) / (t1 - t0) : 0; // px per ms, toward the edge
    // Past a third of its width, or flicked: shut. Otherwise it springs back.
    if (!cancel && (d > panel.offsetWidth / 3 || (speed > 0.5 && d > 20))) setSheet(false);
    // A drag isn't a click on what it started on.
    const stop = (c) => (c.stopPropagation(), c.preventDefault());
    window.addEventListener("click", stop, { capture: true, once: true });
    setTimeout(() => window.removeEventListener("click", stop, { capture: true }), 0);
  };
  panel?.addEventListener("pointerup", (e) => endDrag(e, false));
  panel?.addEventListener("pointercancel", (e) => endDrag(e, true));

  // Turning a phone into a wide screen (or resizing) leaves the sheet behind.
  phone.addEventListener("change", () => {
    setSheet(false, { restoreFocus: false });
    sync();
  });
  // `remember`: the last choice, restored without sliding, then saved as it changes.
  const store = api.config.remember ? `irender-sidebar:${api.config.remember}` : null;
  let saved = null;
  if (store) {
    try {
      saved = localStorage.getItem(store);
    } catch {}
    if ((saved === "true" || saved === "false") && (saved === "true") !== !!state[key]) {
      el.setAttribute("data-restoring", "");
      api.set(key, saved === "true");
      // After the next paint (or a moment, where frames don't come).
      const done = () => el.removeAttribute("data-restoring");
      requestAnimationFrame(() => requestAnimationFrame(done));
      setTimeout(done, 100);
    }
  }
  sync();
  return {
    update(next) {
      state = next;
      const now = String(!!state[key]);
      if (store && now !== saved) {
        saved = now;
        try {
          localStorage.setItem(store, now);
        } catch {}
      }
      if (phone.matches) sync(); // the runtime just wrote `bind`'s value
    },
  };
};
