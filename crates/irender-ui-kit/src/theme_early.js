// irender-ui-kit: applies the stored `ui-theme-toggle` choice. It runs right after the
// page's `<style>`, before the content is shown, so a reload doesn't flash the system
// scheme first. Picking Light or Dark rewrites the page's `prefers-color-scheme` media
// rules, so the theme's dark mode and `@dark` styles follow the choice; System
// restores them. The toggle behaviour (theme_toggle.js) calls `apply` on a click.
(() => {
  const KEY = "irender-theme";
  const SCHEME = /\(\s*prefers-color-scheme\s*:\s*(light|dark)\s*\)/gi;
  // Conditions that always and never match, to stand in for the scheme in a query.
  const YES = "(min-width: 0px)", NO = "(max-width: -1px)";

  // A page with several iRender fragments runs this once per fragment: later runs
  // only bring the newly added stylesheet in line.
  if (window.__irenderTheme) return window.__irenderTheme.apply(window.__irenderTheme.current);

  const original = new WeakMap();
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
  const theme = (window.__irenderTheme = {
    current: "system",
    apply(scheme) {
      theme.current = scheme;
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
    },
    choose(scheme) {
      try {
        localStorage.setItem(KEY, scheme);
      } catch {}
      theme.apply(scheme);
    },
  });
  theme.apply(stored());
})();
