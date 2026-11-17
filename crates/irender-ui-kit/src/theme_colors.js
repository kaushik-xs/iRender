// irender-ui-kit: the `ui-theme-colors` behaviour. A colour typed as hex or picked is
// stored (theme_early.js applies it) and picks the option's `custom` choice, so the
// page shows it at once; other pages follow. Chart colours follow too, unless someone
// picked a chart scheme of their own. Reset clears the colours and goes back to the
// defaults.
(window.__irenderBehaviors ||= {})["ui-theme-colors"] = (el, api) => {
  const { option } = api.config;
  const theme = window.__irenderTheme;
  const HEX = /^#(?:[0-9a-f]{3}|[0-9a-f]{6})$/i;
  const long = (h) => (h.length === 4 ? h.replace(/[0-9a-f]/gi, (d) => d + d) : h).toLowerCase();
  const rows = [...el.querySelectorAll("[data-color]")].map((row) => {
    const [pick, text] = row.querySelectorAll("input");
    return { name: row.dataset.color, pick, text, start: text.value };
  });
  // The stored colours, or what the inputs started with.
  const show = () => {
    const custom = theme.custom();
    for (const r of rows) {
      const v = HEX.test(custom[r.name] || "") ? custom[r.name] : r.start;
      if (document.activeElement !== r.text) r.text.value = v;
      r.pick.value = long(v);
      r.text.removeAttribute("aria-invalid");
    }
  };
  const choose = (r, value) => {
    if (!theme.setCustom(r.name, value)) return;
    if (theme.option(option) !== "custom") theme.setOption(option, "custom");
    // Charts too, while they're on their default scheme (the chart kit's `custom`
    // scheme is built from these colours).
    if (theme.option("chart") == null) theme.setOption("chart", "custom");
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
      }
    });
    r.text.addEventListener("blur", show);
  }
  el.querySelector("[data-reset]")?.addEventListener("click", () => {
    for (const r of rows) theme.setCustom(r.name, null);
    if (theme.option(option) === "custom") theme.setOption(option, null);
    if (theme.option("chart") === "custom") theme.setOption("chart", null);
    show();
  });
  addEventListener("irender:theme-option", (e) => e.detail.name === "custom" && show());
  show();
};
