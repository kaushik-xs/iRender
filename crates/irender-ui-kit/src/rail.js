// irender-ui-kit: the `ui-rail-layout` behaviour. A click on a section shows it (and
// opens the panel); a click on the one shown folds the panel, as does its fold button.
// The rail is a vertical tab list: Up and Down (and Home, End) move between sections,
// Enter or Space shows one. On a phone the panel lies over the page, and Escape or a
// click on the page folds it. Which section and whether it's open are state, so the
// runtime redraws the rest.
(window.__irenderBehaviors ||= {})["ui-rail-layout"] = (el, api) => {
  const cfg = api.config;
  const tabs = () => [...el.querySelectorAll("[data-rail-tabs] [role=tab]")];
  const hasPanel = (id) => cfg.panels.includes(id);
  const isOpen = () => (cfg.bindOpen ? api.state[cfg.bindOpen] !== false : true);
  const setOpen = (v) => cfg.bindOpen && api.set(cfg.bindOpen, v);
  const narrow = () => matchMedia("(max-width: 720px)").matches;

  function show(id) {
    const now = api.state[cfg.bind];
    if (now === id && hasPanel(id)) {
      // The section shown: fold its panel, or open it again.
      setOpen(!isOpen());
    } else {
      api.set(cfg.bind, id);
      setOpen(true);
    }
    api.emit("section", { id, open: isOpen() });
  }

  el.addEventListener("click", (e) => {
    const t = e.target;
    const tab = t.closest("[data-rail-tabs] [role=tab]");
    if (tab) return show(tab.getAttribute("data-rail-item"));
    if (t.closest("[data-rail-fold]")) {
      setOpen(false);
      el.querySelector(`[data-rail-item="${CSS.escape(api.state[cfg.bind])}"]`)?.focus();
      return;
    }
    // On a phone, a click on the page folds the panel over it.
    if (narrow() && t.closest("[data-rail-main]") && isOpen()) setOpen(false);
  });
  el.querySelector("[data-rail-tabs]").addEventListener("keydown", (e) => {
    const list = tabs(), i = list.indexOf(document.activeElement);
    if (i < 0) return;
    const to = { ArrowDown: i + 1, ArrowUp: i - 1, Home: 0, End: list.length - 1 }[e.key];
    if (to !== undefined) {
      e.preventDefault();
      const next = list[(to + list.length) % list.length];
      list.forEach((t) => (t.tabIndex = t === next ? 0 : -1));
      next.focus();
    }
  });
  el.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && narrow() && isOpen() && el.querySelector("[data-rail-panels]").contains(document.activeElement)) {
      setOpen(false);
      el.querySelector(`[data-rail-item="${CSS.escape(api.state[cfg.bind])}"]`)?.focus();
    }
  });
  return {};
};
