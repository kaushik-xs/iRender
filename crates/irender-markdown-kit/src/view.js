// irender-markdown-kit: the `ui-markdown` behaviour. For Markdown from state it renders
// with markdown.js, the same renderer the server used, which escapes every piece of text
// and keeps only safe links, so its output can go into the page as it is. Either way it
// highlights the code fences and gives each a copy button (irender-html's code.js).
(window.__irenderBehaviors ||= {})["ui-markdown"] = (el, api) => {
  const { text } = api.config;
  let last = null;
  // The state, and the list item it's in (a chat message, say) when it's in one.
  const env = (state) => {
    const scope = api.scope?.();
    return scope ? { ...state, ...scope } : state;
  };
  const draw = (state) => {
    const v = window.__irender.evaluate(text, env(state));
    const s = v === null || v === undefined ? "" : typeof v === "object" ? JSON.stringify(v) : String(v);
    if (s === last) return;
    last = s;
    el.innerHTML = window.__irenderMarkdown.render(s);
    window.__irenderCode?.enhance(el);
  };
  // Drawn by the server and fixed: only the fences to do.
  if (text === null) {
    window.__irenderCode?.enhance(el);
    return {};
  }
  draw(api.state);
  return { update: draw };
};
