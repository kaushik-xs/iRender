// irender-ui-kit: the `ui-command` behaviour. The server renders the search box and
// every item; this filters them as you type and moves the active one, while focus
// stays in the search box (`aria-activedescendant`). Enter clicks the active item, so
// its own `on.click` runs; in a `ui-command-dialog`, running one closes the dialog.
(window.__irenderBehaviors ||= {})["ui-command"] = (el, api) => {
  const input = el.querySelector("[data-ui-command-input]");
  const list = el.querySelector("[role=listbox]");
  const empty = el.querySelector("[data-ui-empty]");
  const live = el.querySelector("[data-ui-live]");
  const crumb = el.querySelector("[data-ui-crumb]");
  // Pages: an item's `page` opens a `ui-command-page`, shown in place of the rest.
  const pages = [];
  const pageOf = (n) => n.parentElement?.closest("[data-ui-page]")?.getAttribute("data-ui-page") ?? null;
  const here = () => pages.at(-1) ?? null;
  function openPage(id) {
    const p = list.querySelector(`[data-ui-page="${CSS.escape(id)}"]`);
    if (!p) return;
    pages.push(id);
    showPage();
  }
  function back() {
    pages.pop();
    showPage();
  }
  function showPage(focus = true) {
    const id = here();
    for (const p of list.querySelectorAll("[data-ui-page]")) p.hidden = p.getAttribute("data-ui-page") !== id;
    crumb.hidden = id === null;
    if (id !== null) {
      const title = list.querySelector(`[data-ui-page="${CSS.escape(id)}"]`).getAttribute("data-ui-page-title");
      crumb.textContent = title;
      crumb.setAttribute("aria-label", L("Back from {page}", { page: title }));
    }
    input.value = "";
    if (api.config.bindSearch) api.set(api.config.bindSearch, "");
    active = null;
    filter();
    live.textContent = id === null ? "" : L("{page}: {n} items", { page: crumb.textContent, n: usable().length });
    if (focus) input.focus();
  }
  // Interface text in the document's words (its `labels`), by the English.
  const L = (text, vars) => window.__irender.label(text, vars);
  const dialog = el.closest("dialog");

  // Case- and accent-insensitive text for matching.
  const fold = (s) => String(s ?? "").normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
  const items = () => [...list.querySelectorAll("[role=option]")];
  // In the order shown: ranked items are moved with CSS `order`, not in the DOM.
  const usable = () => {
    const can = items().filter((i) => !i.hidden && !i.disabled);
    return ranked ? can.sort((a, b) => a.getBoundingClientRect().top - b.getBoundingClientRect().top) : can;
  };
  let ranked = false;
  let active = null;
  let ids = 0;

  function setActive(item) {
    active = item;
    for (const i of items()) i.setAttribute("aria-selected", String(i === item));
    if (item) {
      input.setAttribute("aria-activedescendant", item.id);
      // Keeps it in view inside the list, without scrolling the page to the palette.
      const top = item.offsetTop, bottom = top + item.offsetHeight; // the list is positioned
      if (top < list.scrollTop) list.scrollTop = top;
      else if (bottom > list.scrollTop + list.clientHeight) list.scrollTop = bottom - list.clientHeight;
    } else input.removeAttribute("aria-activedescendant");
  }

  // How well `word` matches `text` (both folded): at the start, at a word's start, inside
  // it, or its letters in order; with the positions that matched. Null: no match.
  function score(word, text) {
    const span = (from) => Array.from(word, (_, n) => from + n);
    if (text.startsWith(word)) return { s: 900, at: span(0) };
    const starts = [...text.matchAll(/(?:^|[^\p{L}\p{N}])(?=[\p{L}\p{N}])/gu)].map((m) => m.index + m[0].length);
    const w = starts.find((n) => text.startsWith(word, n));
    if (w !== undefined) return { s: 700, at: span(w) };
    const inside = text.indexOf(word);
    if (inside >= 0) return { s: 500, at: span(inside) };
    // Letters in order, each at the next word's start where it can be.
    const at = [];
    let from = 0;
    for (const ch of word) {
      const k = starts.find((n) => n >= from && text[n] === ch) ?? text.indexOf(ch, from);
      if (k < 0) return null;
      at.push(k);
      from = k + 1;
    }
    let s = 300;
    for (let n = 1; n < at.length; n++) s -= Math.min(40, (at[n] - at[n - 1] - 1) * 4);
    return { s: Math.max(10, s), at };
  }
  // An item's match for the words typed: every word in its label (marked), or else its
  // keywords (scoring less); a label that is what was typed scores most.
  function match(item, words, typed) {
    const label = fold(item.querySelector("[data-ui-label]")?.textContent ?? ""), keys = fold(item.dataset.uiKeywords ?? "");
    let s = label === typed ? 2000 : 0;
    const at = [];
    for (const w of words) {
      const l = score(w, label), k = l ? null : score(w, keys);
      if (!l && !k) return null;
      s += l ? l.s : k.s - 200;
      if (l) at.push(...l.at);
    }
    // Shorter labels first, among equals.
    return { s: s - label.length / 100, at };
  }
  // The matched letters, marked without changing the label (where the browser can).
  const marks = (window.__irenderCommandMarks ||= new Map());
  function mark(found) {
    if (!window.CSS?.highlights || typeof Highlight !== "function") return;
    const ranges = [];
    for (const [item, at] of found) {
      const label = item.querySelector("[data-ui-label]");
      const node = label && document.createTreeWalker(label, NodeFilter.SHOW_TEXT).nextNode();
      if (!node) continue;
      for (const n of at) {
        if (n >= node.length) continue;
        const r = new Range();
        r.setStart(node, n);
        r.setEnd(node, n + 1);
        ranges.push(r);
      }
    }
    marks.set(el, ranges);
    CSS.highlights.set("ui-command-match", new Highlight(...[...marks.values()].flat()));
  }

  // Shows the items whose label or keywords match every word typed, best first (with
  // `rank`), the groups with any of them, and separators only while nothing is typed.
  function filter() {
    const typed = fold(input.value).trim(), words = typed.split(/\s+/).filter(Boolean);
    ranked = api.config.rank !== false && words.length > 0;
    let shown = 0;
    const found = [], scores = new Map();
    for (const i of items()) {
      if (!i.id) i.id = `${list.id}-${ids++}`;
      // Only the open page's items (or, at the top, those in no page).
      const m = pageOf(i) !== here() ? null : words.length ? match(i, words, typed) : { s: 0, at: [] };
      i.hidden = !m;
      if (!m) continue;
      shown++;
      scores.set(i, m.s);
      if (m.at.length) found.push([i, m.at]);
    }
    // Best first among their siblings, and groups by their best.
    const value = (c) => (c.matches("[role=option]") ? scores.get(c) ?? -1
      : c.matches("[role=group]") ? Math.max(-1, ...[...c.querySelectorAll("[role=option]:not([hidden])")].map((i) => scores.get(i) ?? 0))
      : null);
    for (const p of new Set([list, ...items().map((i) => i.parentElement)])) {
      const sorted = [...p.children].filter((c) => value(c) !== null).map((c, n) => [c, n]).sort(([a, n], [b, m]) => value(b) - value(a) || n - m);
      sorted.forEach(([c], n) => (c.style.order = ranked ? String(n + 1) : ""));
    }
    mark(found);
    for (const g of list.querySelectorAll("[role=group]")) g.hidden = !g.querySelector("[role=option]:not([hidden])");
    for (const s of list.querySelectorAll("[role=separator]")) s.hidden = words.length > 0 || pageOf(s) !== here();
    empty.hidden = shown > 0;
    // Typing again starts from the best match.
    const can = usable();
    setActive(can.includes(active) && !ranked ? active : (can[0] ?? null));
    if (ranked) list.scrollTop = 0;
    return shown;
  }

  input.addEventListener("input", () => {
    active = null;
    const n = filter();
    live.textContent = L(["{n} result available.", "{n} results available."], { n });
    if (api.config.bindSearch) api.set(api.config.bindSearch, input.value);
  });
  crumb.addEventListener("click", back);

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
      if (api.config.bindSearch) api.set(api.config.bindSearch, "");
      filter();
    } else if ((e.key === "Backspace" || e.key === "Escape") && !input.value && pages.length) {
      // In a page, with nothing typed: back a page.
      e.preventDefault();
      back();
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
    const item = e.target.closest("[role=option]");
    // An item with a `page` opens it, and the palette stays.
    const page = item?.getAttribute("data-ui-open-page");
    if (page) return openPage(page);
    if (dialog && item) shut();
  });

  // Each opening starts afresh, even one right after closing.
  function reset() {
    input.value = "";
    active = null;
    pages.length = 0;
    showPage(false);
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
  return {
    // The search text from `bindSearch`, when it's changed from outside.
    update() {
      const key = api.config.bindSearch;
      if (!key || document.activeElement === input) return;
      const v = String(api.state[key] ?? "");
      if (v !== input.value) {
        input.value = v;
        filter();
      }
    },
  };
};
