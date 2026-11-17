// irender-markdown-kit: the `ui-markdown` behaviour, for Markdown from state. It renders
// with markdown.js, the same renderer the server used, which escapes every piece of text
// and keeps only safe links, so its output can go into the page as it is.
(window.__irenderBehaviors ||= {})["ui-markdown"] = (el, api) => {
  const { text } = api.config;
  let last = null;
  const draw = (state) => {
    const v = window.__irender.evaluate(text, state);
    const s = v === null || v === undefined ? "" : typeof v === "object" ? JSON.stringify(v) : String(v);
    if (s === last) return;
    last = s;
    el.innerHTML = window.__irenderMarkdown.render(s);
  };
  draw(api.state);
  return { update: draw };
};
