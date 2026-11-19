// irender-ui-kit: the `ui-rail-layout` behaviour. A click on a section shows it (and
// opens the panel); a click on the one shown folds the panel, as does its fold button.
// The rail is a vertical tab list: Up and Down (and Home, End) move between sections,
// Enter or Space shows one. On a phone the panel lies over the page, and Escape or a
// click on the page folds it. Which section and whether it's open are state, so the
// runtime redraws the rest. With `resizable`, the panel's edge drags it wider or
// narrower, within its limits; its width can be state too.
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
  // ---- the panel's width: dragging its edge, or the keys on it
  const edge = el.querySelector(":scope > [data-rail-resize]");
  if (edge) {
    const clamp = (w) => Math.round(Math.min(cfg.max, Math.max(cfg.min, w)));
    const current = () => {
      const w = cfg.bindWidth ? Number(api.state[cfg.bindWidth]) : NaN;
      return w > 0 ? w : el.querySelector("[data-rail-panels]").getBoundingClientRect().width || cfg.width;
    };
    let width = clamp(current());
    const paint = (w) => {
      width = clamp(w);
      el.style.setProperty("--rail-panel", `${width}px`);
      edge.setAttribute("aria-valuenow", String(width));
      edge.setAttribute("aria-valuetext", `${width} pixels`);
    };
    const save = () => {
      if (cfg.bindWidth) api.set(cfg.bindWidth, width);
      api.emit("resize", { width });
    };
    let drag = null;
    edge.addEventListener("pointerdown", (e) => {
      if (e.button !== 0) return;
      e.preventDefault();
      drag = { at: e.clientX, from: width, id: e.pointerId };
      try {
        edge.setPointerCapture(e.pointerId);
      } catch {}
      el.setAttribute("data-resizing", "");
    });
    edge.addEventListener("pointermove", (e) => drag && e.pointerId === drag.id && paint(drag.from + e.clientX - drag.at));
    const end = (e) => {
      if (!drag || e.pointerId !== drag.id) return;
      drag = null;
      el.removeAttribute("data-resizing");
      save();
    };
    edge.addEventListener("pointerup", end);
    edge.addEventListener("pointercancel", end);
    edge.addEventListener("dblclick", () => {
      paint(cfg.width);
      save();
    });
    edge.addEventListener("keydown", (e) => {
      const step = e.shiftKey ? 4 : 16;
      const to = { ArrowRight: width + step, ArrowLeft: width - step, Home: cfg.min, End: cfg.max }[e.key];
      if (to !== undefined) {
        e.preventDefault();
        paint(to);
        save();
      } else if (e.key === "Enter" && cfg.bindOpen) {
        // Folds the panel; its section's icon brings it back.
        e.preventDefault();
        setOpen(false);
        el.querySelector(`[data-rail-item="${CSS.escape(api.state[cfg.bind])}"]`)?.focus();
      }
    });
    paint(width);
    let seen = cfg.bindWidth ? api.state[cfg.bindWidth] : undefined;
    return {
      update() {
        if (!cfg.bindWidth || drag) return;
        const now = api.state[cfg.bindWidth];
        if (now === seen) return;
        seen = now;
        if (Number(now) > 0) paint(Number(now));
      },
    };
  }
  return {};
};
