// irender-ui-kit: the `ui-theme-option` behaviour. theme_early.js has already applied
// the stored choices; this keeps every picker for the option in step and applies a
// click. Choosing the theme's default clears the attribute, so the base tokens show.
(window.__irenderBehaviors ||= {})["ui-theme-option"] = (el, api) => {
  const { option, fallback, bind } = api.config;
  const theme = window.__irenderTheme;
  const buttons = [...el.querySelectorAll("[data-choice]")];
  const show = () => {
    const current = theme.option(option) ?? fallback;
    buttons.forEach((b) => b.setAttribute("aria-pressed", String(b.dataset.choice === current)));
    if (bind && api.state[bind] !== current) queueMicrotask(() => api.state[bind] !== current && api.set(bind, current));
    told = current;
  };
  // What the state was last told, to tell a choice made through it.
  let told;
  addEventListener("irender:theme-option", (e) => e.detail.name === option && show());
  el.addEventListener("click", (e) => {
    const b = e.target.closest("[data-choice]");
    if (b && el.contains(b)) theme.setOption(option, b.dataset.choice === fallback ? null : b.dataset.choice);
  });
  show();
  return {
    update() {
      const v = bind ? api.state[bind] : null;
      if (v === null || v === undefined || v === told || !buttons.some((b) => b.dataset.choice === v)) return;
      told = v;
      theme.setOption(option, v === fallback ? null : v);
    },
  };
};
