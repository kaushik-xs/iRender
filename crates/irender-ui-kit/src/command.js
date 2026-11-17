// irender-ui-kit: the `ui-command` behaviour. The server renders the search box and
// every item; this filters them as you type and moves the active one, while focus
// stays in the search box (`aria-activedescendant`). Enter clicks the active item, so
// its own `on.click` runs; in a `ui-command-dialog`, running one closes the dialog.
(window.__irenderBehaviors ||= {})["ui-command"] = (el, api) => {
  const input = el.querySelector("[data-ui-command-input]");
  const list = el.querySelector("[role=listbox]");
  const empty = el.querySelector("[data-ui-empty]");
  const live = el.querySelector("[data-ui-live]");
  const dialog = el.closest("dialog");

  // Case- and accent-insensitive text for matching.
  const fold = (s) => String(s ?? "").normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
  const items = () => [...list.querySelectorAll("[role=option]")];
  const usable = () => items().filter((i) => !i.hidden && !i.disabled);
  let active = null;
  let ids = 0;

  function setActive(item) {
    active = item;
    for (const i of items()) i.setAttribute("aria-selected", String(i === item));
    if (item) {
      input.setAttribute("aria-activedescendant", item.id);
      item.scrollIntoView({ block: "nearest" });
    } else input.removeAttribute("aria-activedescendant");
  }

  // Shows the items whose label or keywords hold every word typed, the groups with
  // any of them, and separators only while nothing is typed.
  function filter() {
    const words = fold(input.value).split(/\s+/).filter(Boolean);
    let shown = 0;
    for (const i of items()) {
      if (!i.id) i.id = `${list.id}-${ids++}`;
      const text = fold(`${i.querySelector("[data-ui-label]")?.textContent ?? ""} ${i.dataset.uiKeywords ?? ""}`);
      i.hidden = !words.every((w) => text.includes(w));
      if (!i.hidden) shown++;
    }
    for (const g of list.querySelectorAll("[role=group]")) g.hidden = !g.querySelector("[role=option]:not([hidden])");
    for (const s of list.querySelectorAll("[role=separator]")) s.hidden = words.length > 0;
    empty.hidden = shown > 0;
    const can = usable();
    setActive(can.includes(active) ? active : (can[0] ?? null));
    return shown;
  }

  input.addEventListener("input", () => {
    active = null;
    const n = filter();
    live.textContent = `${n} result${n === 1 ? "" : "s"} available.`;
  });

  input.addEventListener("keydown", (e) => {
    if (e.altKey || e.ctrlKey || e.metaKey) return;
    const can = usable();
    const at = can.indexOf(active);
    const to = { ArrowDown: at + 1, ArrowUp: at < 0 ? -1 : at - 1, Home: 0, End: -1 }[e.key];
    if (to !== undefined) {
      // With text typed, Home and End move in it, as in any text box.
      if ((e.key === "Home" || e.key === "End") && input.value) return;
      e.preventDefault();
      if (can.length) setActive(can.at(to % can.length));
    } else if (e.key === "Enter") {
      e.preventDefault();
      active?.click();
    } else if (e.key === "Escape" && input.value) {
      // The first Escape clears the search; only the next one reaches the dialog.
      e.preventDefault();
      input.value = "";
      filter();
    }
  });

  // Items aren't focusable, so a press on one keeps focus in the search box.
  list.addEventListener("mousedown", (e) => e.target.closest("[role=option]") && e.preventDefault());
  list.addEventListener("mousemove", (e) => {
    const item = e.target.closest("[role=option]");
    if (item && item !== active && !item.disabled) setActive(item);
  });
  // Bubbles here after the item's own `on.click` has run.
  list.addEventListener("click", (e) => {
    if (dialog && e.target.closest("[role=option]")) shut();
  });

  // Each opening starts afresh, even one right after closing.
  function reset() {
    input.value = "";
    active = null;
    filter();
  }
  function shut() {
    dialog.close();
    reset();
  }

  if (dialog) {
    // Escape and a click outside close it without us; `close` arrives a moment later.
    dialog.addEventListener("close", reset);
    const shortcut = api.config.shortcut;
    if (shortcut) {
      const parts = shortcut.toLowerCase().split("+");
      const key = parts.pop();
      const apple = /Mac|iPhone|iPad/.test(navigator.platform);
      const want = {
        meta: parts.includes("meta") || (apple && parts.includes("mod")),
        ctrl: parts.includes("ctrl") || (!apple && parts.includes("mod")),
        alt: parts.includes("alt"),
        shift: parts.includes("shift"),
      };
      document.addEventListener("keydown", (e) => {
        if (e.key.toLowerCase() !== key || e.metaKey !== want.meta || e.ctrlKey !== want.ctrl) return;
        if (e.altKey !== want.alt || e.shiftKey !== want.shift) return;
        e.preventDefault();
        if (dialog.open) shut();
        else dialog.showModal();
      });
    }
  }

  // Items from state are rebuilt by the runtime: filter the new ones too.
  new MutationObserver(() => filter()).observe(list, { childList: true, subtree: true });
  filter();
};
