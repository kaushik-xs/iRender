// irender-ui-kit: the `ui-gauge` behaviour, only on a gauge whose `min`, `max` or `bands`
// follow state. The value's arc and needle turn by CSS either way; this numbers the ends
// and ticks again for a new range, draws the bands again on it, and (bands from state)
// gives the value the tone and name of the band it's in now. A band that doesn't fit
// (its `to` not above the one before, or past `max`) is left out, with a warning.
(window.__irenderBehaviors ||= {})["ui-gauge"] = (el, api) => {
  const cfg = api.config, K = window.__irenderKit;
  const svg = el.querySelector("svg"), track = el.querySelector("[data-track]"), arc = el.querySelector("[data-arc]");
  const number = (v) => (v === null || v === undefined || v === "" ? NaN : Number(v));
  const fmt = (v) => K.format(cfg.decimals == null ? Math.round(v * 1e6) / 1e6 : v, { decimals: cfg.decimals, locale: cfg.lang });
  let said = "";
  function update() {
    let min = number(K.value(api, cfg.min)), max = number(K.value(api, cfg.max));
    if (!Number.isFinite(min)) min = 0;
    if (!Number.isFinite(max)) max = 100;
    if (!(max > min)) return;
    el.setAttribute("aria-valuemin", String(min));
    el.setAttribute("aria-valuemax", String(max));
    for (const e of el.querySelectorAll("[data-gauge-end]")) e.textContent = fmt(e.hasAttribute("data-end-max") ? max : min);
    for (const e of el.querySelectorAll("[data-gauge-tick]")) e.textContent = fmt(min + (max - min) * Number(e.getAttribute("data-at")));
    // The bands, checked as the server checks them, and placed on the range as it is now.
    const list = K.value(api, cfg.bands), bands = [];
    for (const [i, b] of (Array.isArray(list) ? list : []).entries()) {
      if (!b || typeof b !== "object") continue;
      const last = i === list.length - 1, from = bands.at(-1)?.to ?? min;
      const to = b.to === null || b.to === undefined ? (last ? max : NaN) : Number(b.to);
      if (!(to > from && to <= max)) {
        if (said !== JSON.stringify(list)) console.warn(`ui-gauge: band ${i + 1} left out: its \`to\` must be above ${from} and at most ${max}`);
        continue;
      }
      bands.push({ to, tone: cfg.colors[b.tone] ? b.tone : "neutral", label: b.label ?? "" });
    }
    said = JSON.stringify(list);
    for (const p of el.querySelectorAll("[data-band]")) p.remove();
    let from = 0;
    for (const b of bands) {
      const to = ((b.to - min) * 100) / (max - min);
      const p = document.createElementNS("http://www.w3.org/2000/svg", "path");
      for (const [k, v] of Object.entries({ "data-band": "", d: track.getAttribute("d"), "stroke-width": track.getAttribute("stroke-width"), pathLength: "100", "stroke-dasharray": `0 ${from} ${to - from} 200` })) p.setAttribute(k, v);
      p.style.setProperty("--band", cfg.colors[b.tone]);
      svg.insertBefore(p, svg.querySelector("[data-tick]") ?? arc);
      from = to;
    }
    // Fixed bands keep the server's tone and words, which follow the value already.
    if (!cfg.live) return;
    const v = number(K.value(api, cfg.value)), unit = K.value(api, cfg.unit);
    const band = bands.find((b) => v <= b.to) ?? bands.at(-1);
    if (!cfg.tone) el.setAttribute("data-tone", band?.tone ?? "primary");
    const text = [String(K.value(api, cfg.value) ?? ""), unit != null && unit !== "" ? ` ${unit}` : "", band?.label ? `, ${band.label}` : ""].join("");
    el.setAttribute("aria-valuetext", text);
  }
  update();
  return { update };
};
