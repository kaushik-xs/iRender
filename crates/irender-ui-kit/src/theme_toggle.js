// irender-ui-kit: the `ui-theme-toggle` behaviour. Picking Light or Dark rewrites the
// page's `prefers-color-scheme` media rules, so the theme's dark mode and `@dark`
// styles follow the choice; System restores them. The choice is kept in localStorage
// and shared by every toggle on the page.
(() => {
  const KEY = "irender-theme";
  const SCHEME = /\(\s*prefers-color-scheme\s*:\s*(light|dark)\s*\)/gi;
  // Conditions that always and never match, to stand in for the scheme in a query.
  const YES = "(min-width: 0px)", NO = "(max-width: -1px)";
  const original = new WeakMap();
  let current = null;

  const stored = () => {
    try {
      return localStorage.getItem(KEY) || "system";
    } catch {
      return "system";
    }
  };
  const mediaRules = (rules, out) => {
    for (const r of rules) {
      if (r instanceof CSSMediaRule) out.push(r);
      if (r.cssRules) mediaRules(r.cssRules, out);
    }
    return out;
  };
  const apply = (scheme) => {
    current = scheme;
    for (const sheet of document.styleSheets) {
      let rules;
      try {
        rules = mediaRules(sheet.cssRules, []);
      } catch {
        continue; // a cross-origin stylesheet
      }
      for (const r of rules) {
        if (!original.has(r)) original.set(r, r.media.mediaText);
        const text = original.get(r);
        if (!text.match(SCHEME)) continue;
        r.media.mediaText = scheme === "system" ? text : text.replace(SCHEME, (_, s) => (s.toLowerCase() === scheme ? YES : NO));
      }
    }
    // Native controls and scrollbars, and `[data-theme]` selectors (charts restyle on it).
    const html = document.documentElement;
    html.style.colorScheme = scheme === "system" ? "" : scheme;
    html.dataset.theme = scheme;
    dispatchEvent(new CustomEvent("irender:theme", { detail: scheme }));
  };

  (window.__irenderBehaviors ||= {})["ui-theme-toggle"] = (el) => {
    const buttons = [...el.querySelectorAll("[data-scheme]")];
    const show = (scheme) => buttons.forEach((b) => b.setAttribute("aria-pressed", String(b.dataset.scheme === scheme)));
    addEventListener("irender:theme", (e) => show(e.detail));
    if (current === null) apply(stored());
    show(current);
    el.addEventListener("click", (e) => {
      const b = e.target.closest("[data-scheme]");
      if (!b || !el.contains(b)) return;
      try {
        localStorage.setItem(KEY, b.dataset.scheme);
      } catch {}
      apply(b.dataset.scheme);
    });
  };
})();
