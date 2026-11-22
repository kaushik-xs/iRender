// irender-ui-kit: the `ui-rating` behaviour. The stars are radio buttons, so the arrow
// keys and Space pick; this writes the pick to state, marks the stars up to it (or up
// to the hovered one, as a preview), and clears the rating when the chosen star is
// clicked again.
(window.__irenderBehaviors ||= {})["ui-rating"] = (el, api) => {
  const cfg = api.config;
  const inputs = [...el.querySelectorAll("input")];
  const value = () => Number(api.state[cfg.bind]) || 0;
  const mark = (n) => inputs.forEach((input) => input.parentElement.toggleAttribute("data-on", Number(input.value) <= n));
  const draw = () => {
    const v = value();
    inputs.forEach((input) => (input.checked = Number(input.value) === v));
    mark(v);
  };
  // Pressed on the chosen star: a click then clears it rather than picking it again.
  let again = false;
  el.addEventListener("pointerdown", (e) => {
    const input = e.target.closest("label")?.querySelector("input");
    again = !!input && cfg.clearable && input.checked;
  });
  el.addEventListener("click", (e) => {
    const input = e.target.closest("label")?.querySelector("input");
    if (!input || input.disabled || e.target !== input) return;
    if (again) {
      again = false;
      input.checked = false;
      api.set(cfg.bind, 0);
      api.emit("change", { value: 0 });
    }
  });
  el.addEventListener("change", (e) => {
    // Only the radios' own change: the one this sends bubbles through here too.
    if (!e.target.matches?.("input")) return;
    const n = Number(e.target.value);
    api.set(cfg.bind, n);
    api.emit("change", { value: n });
  });
  el.addEventListener("pointerover", (e) => {
    const input = e.target.closest("label")?.querySelector("input");
    if (input && !input.disabled) mark(Number(input.value));
  });
  el.addEventListener("pointerleave", draw);
  draw();
  return { update: draw };
};
