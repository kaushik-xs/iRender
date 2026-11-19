// irender-ui-kit: the `ui-alert` behaviour. Its ✕ hides the alert.
(window.__irenderBehaviors ||= {})["ui-alert"] = (el) => {
  el.querySelector("[data-dismiss]")?.addEventListener("click", () => {
    el.hidden = true;
  });
};
