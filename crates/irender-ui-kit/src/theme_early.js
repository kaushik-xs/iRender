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
  // Custom colours (`ui-theme-colors`): a primary and a second brand colour, as hex.
  // They apply while the palette's or the chart colours' `custom` choice is picked.
  const CUSTOM = "irender-theme-custom";
  const HEX = /^#(?:[0-9a-f]{3}|[0-9a-f]{6})$/i;
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
    // Custom colours: `{ primary, "brand-2" }`, each a hex or missing.
    custom() {
      return storedCustom();
    },
    setCustom(name, value) {
      if (value != null && !HEX.test(value)) return false;
      const all = storedCustom();
      if (value == null) delete all[name];
      else all[name] = value.toLowerCase();
      try {
        localStorage.setItem(CUSTOM, JSON.stringify(all));
      } catch {}
      applyCustom();
      dispatchEvent(new CustomEvent("irender:theme-option", { detail: { name: "custom", choice: null } }));
      return true;
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
  const storedCustom = () => {
    try {
      const all = JSON.parse(localStorage.getItem(CUSTOM) || "{}");
      return all && typeof all === "object" ? all : {};
    } catch {
      return {};
    }
  };
  // Dark or light text, whichever reads better on `hex`.
  const onColor = (hex) => {
    const h = hex.length === 4 ? hex.replace(/[0-9a-f]/gi, (d) => d + d) : hex;
    const [r, g, b] = [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16) / 255).map((c) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4));
    return 0.2126 * r + 0.7152 * g + 0.0722 * b > 0.4 ? "#09090b" : "#fafafa";
  };
  // Sets (or clears) the custom colours on <html>, where they win over the theme's.
  const applyCustom = () => {
    const style = document.documentElement.style;
    const set = (k, v) => (v ? style.setProperty(k, v) : style.removeProperty(k));
    const c = storedCustom();
    const primary = HEX.test(c.primary) ? c.primary : null;
    const second = HEX.test(c["brand-2"]) ? c["brand-2"] : null;
    const palette = theme.option("palette") === "custom";
    set("--color-primary", palette && primary);
    set("--color-primary-hover", palette && primary && `color-mix(in srgb, ${primary} 85%, black)`);
    set("--color-on-primary", palette && primary && onColor(primary));
    set("--color-ring", palette && primary);
    set("--color-brand-2", palette && second);
    set("--color-on-brand-2", palette && second && onColor(second));
    // A chart scheme from the two: each, lighter, darker and mixed.
    const chart = theme.option("chart") === "custom";
    const a = primary || "var(--color-primary)";
    const b = second || "var(--color-brand-2, var(--color-primary))";
    const mix = (x, y, p) => `color-mix(in srgb, ${x} ${p}%, ${y})`;
    const scheme = [a, b, mix(a, "white", 60), mix(b, "white", 60), mix(a, "black", 70), mix(b, "black", 70), mix(a, b, 50), mix(a, "gray", 40)];
    scheme.forEach((v, i) => set(`--color-chart-${i + 1}`, chart && v));
  };
  const applyOption = (name, choice) => {
    if (!/^[a-z0-9-]+$/.test(name)) return;
    if (choice == null) document.documentElement.removeAttribute("data-theme-" + name);
    else document.documentElement.setAttribute("data-theme-" + name, String(choice));
    applyCustom();
    dispatchEvent(new CustomEvent("irender:theme-option", { detail: { name, choice: choice ?? null } }));
  };
  for (const [name, choice] of Object.entries(storedOptions())) applyOption(name, choice);
  theme.apply(stored());
  // A choice made on another page of the site (another tab, or a page in an iframe
  // here) applies at once: the browser tells every other page when storage changes.
  addEventListener("storage", (e) => {
    if (e.key === KEY) theme.apply(stored());
    if (e.key === CUSTOM) {
      applyCustom();
      dispatchEvent(new CustomEvent("irender:theme-option", { detail: { name: "custom", choice: null } }));
    }
    if (e.key !== OPTIONS && e.key !== null) return;
    const all = storedOptions();
    for (const a of [...document.documentElement.attributes]) {
      const name = a.name.startsWith("data-theme-") ? a.name.slice(11) : null;
      if (name && !(name in all)) applyOption(name, null);
    }
    for (const [name, choice] of Object.entries(all)) if (theme.option(name) !== choice) applyOption(name, choice);
  });
})();
