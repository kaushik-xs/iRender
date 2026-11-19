// irender-ui-kit: the `ui-theme-colors` behaviour. A colour typed as hex or picked is
// stored (theme_early.js applies it) and picks the option's `custom` choice, so the
// page shows it at once; other pages follow. Chart colours follow too, unless someone
// picked a chart scheme of their own. Clearing a field goes back to the theme's colour
// (for a gradient end, to no gradient). Backgrounds are kept per mode: the fields show
// and set the mode showing. Reset clears every colour and goes back to the defaults.
(window.__irenderBehaviors ||= {})["ui-theme-colors"] = (el, api) => {
  const { option } = api.config;
  const theme = window.__irenderTheme;
  const HEX = /^#(?:[0-9a-f]{3}|[0-9a-f]{6})$/i;
  const long = (h) => (h.length === 4 ? h.replace(/[0-9a-f]/gi, (d) => d + d) : h).toLowerCase();
  const rows = [...el.querySelectorAll("[data-color]")].map((row) => {
    const [pick, text] = row.querySelectorAll("input");
    return { name: row.dataset.color, probe: row.dataset.probe, perMode: row.hasAttribute("data-per-mode"), pick, text };
  });
  // The stored name: a background's carries the mode.
  const key = (r) => (r.perMode && theme.scheme() === "dark" ? r.name + "-dark" : r.name);
  // What the theme shows for a row now, as #rrggbb: its colour resolved in a probe and
  // read back through a canvas, so `color-mix()` and the like come out as hex too.
  const probe = document.createElement("i");
  probe.hidden = true;
  el.append(probe);
  const ctx = document.createElement("canvas").getContext("2d", { willReadFrequently: true });
  const resolve = (css) => {
    probe.style.color = "";
    probe.style.color = css;
    ctx.clearRect(0, 0, 1, 1);
    ctx.fillStyle = getComputedStyle(probe).color;
    ctx.fillRect(0, 0, 1, 1);
    return "#" + [...ctx.getImageData(0, 0, 1, 1).data.slice(0, 3)].map((n) => n.toString(16).padStart(2, "0")).join("");
  };
  // The stored colours, or the theme's.
  const show = () => {
    const custom = theme.custom();
    for (const r of rows) {
      const stored = custom[key(r)];
      const v = HEX.test(stored || "") ? stored : r.probe ? resolve(r.probe) : "";
      if (document.activeElement !== r.text) r.text.value = v;
      if (v) r.pick.value = long(v);
      r.text.removeAttribute("aria-invalid");
    }
    const mode = el.querySelector("[data-mode]");
    if (mode) mode.textContent = window.__irender.label(theme.scheme() === "dark" ? "dark mode" : "light mode");
  };
  const choose = (r, value) => {
    if (!theme.setCustom(key(r), value)) return;
    if (value == null) return show();
    if (theme.option(option) !== "custom") theme.setOption(option, "custom");
    // Charts too, while they're on their default scheme (the chart kit's `custom`
    // scheme is built from the brand colours).
    if (!r.perMode && theme.option("chart") == null) theme.setOption("chart", "custom");
  };
  for (const r of rows) {
    r.pick.addEventListener("input", () => {
      r.text.value = r.pick.value;
      choose(r, r.pick.value);
    });
    r.text.addEventListener("input", () => {
      const v = r.text.value.trim();
      const ok = HEX.test(v);
      if (!ok && v !== "") r.text.setAttribute("aria-invalid", "true");
      else r.text.removeAttribute("aria-invalid");
      if (ok) {
        r.pick.value = long(v);
        choose(r, v);
      } else if (v === "") choose(r, null);
    });
    r.text.addEventListener("blur", show);
  }
  el.querySelector("[data-reset]")?.addEventListener("click", () => {
    for (const name of Object.keys(theme.custom())) theme.setCustom(name, null);
    if (theme.option(option) === "custom") theme.setOption(option, null);
    if (theme.option("chart") === "custom") theme.setOption("chart", null);
    show();
  });
  // Another page's colours, a palette picked (the theme's colours change) and the mode.
  addEventListener("irender:theme-option", () => show());
  addEventListener("irender:theme", () => show());
  show();
};
