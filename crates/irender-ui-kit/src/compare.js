// irender-ui-kit: the `ui-compare-slider` behaviour. The divider is the range input over
// the picture; this sets `--pos` as it moves (and from `bind`), says the split for screen
// readers ("Before 40%, after 60%"), fades a side's label as the divider covers it, and
// with `hover` lets a mouse move the divider without pressing. With `mode: fade` the
// second picture fades in over the first instead (CSS, from `--pos`).
(window.__irenderBehaviors ||= {})["ui-compare-slider"] = (el, api) => {
  const cfg = api.config, L = (t, v) => window.__irender.label(t, v);
  const input = el.querySelector("[data-cmp-frame] input"), frame = el.querySelector("[data-cmp-frame]");
  const clamp = (n) => Math.min(100, Math.max(0, n));
  // A vertical range runs from the top; so does the picture's divider.
  function show(v, write) {
    v = clamp(Math.round(Number(v)));
    if (!Number.isFinite(v)) return;
    el.style.setProperty("--pos", String(v));
    if (Number(input.value) !== v) input.value = String(v);
    // Labels: "After {b}% faded in"
    const fade = el.hasAttribute("data-fade");
    input.setAttribute("aria-valuetext", fade ? L("After {b}% faded in", { b: v }) : L("Before {a}%, after {b}%", { a: v, b: 100 - v }));
    el.setAttribute("data-near", fade ? "" : v < 12 ? "before" : v > 88 ? "after" : "");
    if (write && cfg.bind && api.state[cfg.bind] !== v) api.set(cfg.bind, v);
    if (write) api.emit("change", { value: v });
  }
  input.addEventListener("input", () => show(input.value, true));
  if (cfg.hover) {
    frame.addEventListener("pointermove", (e) => {
      if (e.pointerType !== "mouse" || e.buttons) return;
      const r = frame.getBoundingClientRect();
      const rtl = getComputedStyle(el).direction === "rtl";
      const v = cfg.vertical ? ((e.clientY - r.top) / r.height) * 100 : ((rtl ? r.right - e.clientX : e.clientX - r.left) / r.width) * 100;
      show(v, true);
    });
  }
  const read = () => show(cfg.bind && api.state[cfg.bind] != null ? api.state[cfg.bind] : input.value, false);
  read();
  return { update: read };
};
