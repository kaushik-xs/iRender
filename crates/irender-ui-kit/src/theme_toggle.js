// irender-ui-kit: the `ui-theme-toggle` behaviour. theme_early.js has already applied
// the stored choice; this keeps every toggle's buttons in step and applies a click.
(window.__irenderBehaviors ||= {})["ui-theme-toggle"] = (el) => {
  const theme = window.__irenderTheme;
  const buttons = [...el.querySelectorAll("[data-scheme]")];
  const show = (scheme) => buttons.forEach((b) => b.setAttribute("aria-pressed", String(b.dataset.scheme === scheme)));
  addEventListener("irender:theme", (e) => show(e.detail));
  show(theme.current);
  el.addEventListener("click", (e) => {
    const b = e.target.closest("[data-scheme]");
    if (b && el.contains(b)) theme.choose(b.dataset.scheme);
  });
};
