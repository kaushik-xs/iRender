// irender-markdown-kit: the `ui-markdown` behaviour. For Markdown from state it renders
// with markdown.js, the same renderer the server used, which escapes every piece of text
// and keeps only safe links, so its output can go into the page as it is. Either way it
// highlights the code fences and gives each a copy button (irender-html's code.js).
// While `streaming` is true, the text is finished at its end first (markdown.js's
// `complete`), so a reply's `**` or link shows formatted while it arrives.
(window.__irenderBehaviors ||= {})["ui-markdown"] = (el, api) => {
  const { text, streaming } = api.config;
  let last = null;
  // The state, and the list item it's in (a chat message, say) when it's in one.
  const env = (state) => {
    const scope = api.scope?.();
    return scope ? { ...state, ...scope } : state;
  };
  const draw = (state) => {
    const v = window.__irender.evaluate(text, env(state));
    let s = v === null || v === undefined ? "" : typeof v === "object" ? JSON.stringify(v) : String(v);
    if (streaming && window.__irender.evaluate(streaming, env(state))) s = window.__irenderMarkdown.complete(s);
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
