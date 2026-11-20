// irender-ui-kit: the `ui-sidebar` behaviour, on a `ui-sidebar-layout` with `bind`.
// On a phone (below 768px) the panel is a sheet over the page: the trigger opens it
// whatever `bind` says, and a tap on the backdrop, Escape or following a link closes
// it. While it's open the page behind is inert, and focus goes into the panel and back
// to the trigger. ⌘B / Ctrl+B toggles the sidebar: the sheet on a phone, `bind`
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

  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && sheetOpen()) {
      e.preventDefault();
      setSheet(false);
    } else if (e.key.toLowerCase() === "b" && (e.metaKey || e.ctrlKey) && !e.altKey && !e.shiftKey) {
      if (document.querySelector("[data-sidebar-layout][data-i-x]") !== el || e.target.isContentEditable) return;
      e.preventDefault();
      toggle();
    }
  });

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
