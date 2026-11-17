// irender-ui-kit: the `ui-message-html` behaviour. Shows a message's `html`, a piece of
// an iRender page rendered on the server (see `render_message_ui`), and starts it as
// its own root. Its scripts are the kits' behaviours, re-added so that they run.
(window.__irenderBehaviors ||= {})["ui-message-html"] = (el) => {
  let shown = null;
  const show = () => {
    const html = el.getAttribute("data-html") || "";
    if (html === shown) return;
    shown = html;
    el.innerHTML = html;
    for (const old of el.querySelectorAll("script:not([type])")) {
      const script = document.createElement("script");
      script.textContent = old.textContent;
      old.replaceWith(script);
    }
    window.__irender.mount(el);
  };
  show();
  return { update: show };
};
