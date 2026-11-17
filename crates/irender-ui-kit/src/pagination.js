// irender-ui-kit: the `ui-pagination` behaviour, for a page, page count or `siblings`
// from state. The server renders the links for the state the page starts with; this
// redraws them when it changes, with the same window (`page_window` in navigation.rs):
// the first and last pages, `siblings` around the current one, and `…` for a gap,
// except that a gap of one page shows the page.
(window.__irenderBehaviors ||= {})["ui-pagination"] = (el, api) => {
  const { page, pages, siblings, href } = api.config;
  const [link, gap] = el.querySelectorAll(":scope > template");
  const evaluate = window.__irender.evaluate;

  // As the server reads the props: a whole number, at least 0, or the default.
  const count = (js, state, fallback) => {
    const v = js && typeof js === "object" && ("r" in js || "t" in js) ? evaluate(js, state) : js;
    const n = typeof v === "string" ? (v.trim() === "" ? NaN : Number(v)) : typeof v === "number" ? v : typeof v === "boolean" ? +v : v == null ? 0 : NaN;
    return Number.isFinite(n) ? Math.max(0, Math.trunc(n)) : fallback;
  };

  const pageWindow = (at, total, around) => {
    const out = [];
    const lo = Math.max(at - around, 1), hi = Math.min(at + around, total);
    const push = (p) => {
      const prev = out.at(-1);
      if (prev != null && p > prev + 1) out.push(p === prev + 2 ? prev + 1 : null);
      out.push(p);
    };
    for (const p of [1, ...Array.from({ length: Math.max(0, hi - lo + 1) }, (_, i) => lo + i), total]) {
      if (!out.includes(p)) push(p);
    }
    return out;
  };

  // Each link as [label, href or null for disabled, aria-label, current]; null for `…`.
  const links = (state) => {
    const total = Math.max(count(pages, state, 1), 1);
    const at = Math.min(Math.max(count(page, state, 1), 1), total);
    const around = count(siblings, state, 1);
    const to = (p) => {
      const v = href ? evaluate(href, { ...state, p }) : null;
      return v == null ? null : String(v);
    };
    return [
      ["‹ Previous", at > 1 ? to(at - 1) : null, "Go to previous page", false, at <= 1],
      ...pageWindow(at, total, around).map((p) => (p === null ? null : [String(p), to(p), null, p === at, false])),
      ["Next ›", at < total ? to(at + 1) : null, "Go to next page", false, at >= total],
    ];
  };

  const draw = (list) =>
    list.map((item) => {
      if (item === null) return gap.content.firstElementChild.cloneNode(true);
      const [label, target, aria, current, disabled] = item;
      const li = link.content.firstElementChild.cloneNode(true);
      const a = li.firstElementChild;
      a.textContent = label;
      if (disabled) a.setAttribute("aria-disabled", "true");
      else if (target !== null) a.setAttribute("href", target);
      if (aria) a.setAttribute("aria-label", aria);
      if (current) a.setAttribute("aria-current", "page");
      return li;
    });

  // The server's links match the starting state, so only a change redraws them.
  let last = JSON.stringify(links(api.state));
  return {
    update(state) {
      const list = links(state);
      const key = JSON.stringify(list);
      if (key === last) return;
      last = key;
      el.replaceChildren(link, gap, ...draw(list));
    },
  };
};
