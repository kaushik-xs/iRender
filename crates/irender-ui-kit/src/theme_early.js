// irender-ui-kit: applies the stored `ui-theme-toggle` choice. It runs right after the
// page's `<style>`, before the content is shown, so a reload doesn't flash the system
// scheme first. Picking Light or Dark rewrites the page's `prefers-color-scheme` media
// rules, so the theme's dark mode and `@dark` styles follow the choice; System
// restores them. The toggle behaviour (theme_toggle.js) calls `apply` on a click.
// It also applies the theme options picked with `ui-theme-option` (a palette, a
// radius, …): each is a `data-theme-<option>` attribute on <html>, kept in
// localStorage (`irender-theme-options`) so every page shows the same choices.
(() => {
  const KEY = "irender-theme";
  const OPTIONS = "irender-theme-options";
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
    // Theme options: the choice for `name`, or null for the theme's default.
    option(name) {
      return document.documentElement.getAttribute("data-theme-" + name);
    },
    setOption(name, choice) {
      const all = storedOptions();
      if (choice == null) delete all[name];
      else all[name] = choice;
      try {
        localStorage.setItem(OPTIONS, JSON.stringify(all));
      } catch {}
      applyOption(name, choice);
    },
  });
  const storedOptions = () => {
    try {
      const all = JSON.parse(localStorage.getItem(OPTIONS) || "{}");
      return all && typeof all === "object" ? all : {};
    } catch {
      return {};
    }
  };
  const applyOption = (name, choice) => {
    if (!/^[a-z0-9-]+$/.test(name)) return;
    if (choice == null) document.documentElement.removeAttribute("data-theme-" + name);
    else document.documentElement.setAttribute("data-theme-" + name, String(choice));
    dispatchEvent(new CustomEvent("irender:theme-option", { detail: { name, choice: choice ?? null } }));
  };
  for (const [name, choice] of Object.entries(storedOptions())) applyOption(name, choice);
  theme.apply(stored());
  // A choice made on another page of the site (another tab, or a page in an iframe
  // here) applies at once: the browser tells every other page when storage changes.
  addEventListener("storage", (e) => {
    if (e.key === KEY) theme.apply(stored());
    if (e.key !== OPTIONS && e.key !== null) return;
    const all = storedOptions();
    for (const a of [...document.documentElement.attributes]) {
      const name = a.name.startsWith("data-theme-") ? a.name.slice(11) : null;
      if (name && !(name in all)) applyOption(name, null);
    }
    for (const [name, choice] of Object.entries(all)) if (theme.option(name) !== choice) applyOption(name, choice);
  });
})();
