// irender-markdown-kit: the `ui-markdown` behaviour. For Markdown from state it renders
// with markdown.js, the same renderer the server used, which escapes every piece of text
// and keeps only safe links, so its output can go into the page as it is. Either way it
// highlights the code fences and gives each a copy button (irender-html's code.js).
// While `streaming` is true, the text is finished at its end first (markdown.js's
// `complete`), so a reply's `**` or link shows formatted while it arrives; only the block
// still arriving is drawn again for each chunk, and the whole once it's done.
(window.__irenderBehaviors ||= {})["ui-markdown"] = (el, api) => {
  const { text, streaming } = api.config;
  let last = null;
  // While streaming: the finished blocks, drawn once, and how many nodes they made. Each
  // chunk then draws only the block still arriving at the end.
  let done = null, doneNodes = 0;
  // Where the last finished block ends: after the last blank line outside a code fence.
  const boundary = (s) => {
    let at = 0, fence = null, pos = 0;
    for (const line of s.split("\n")) {
      const f = /^\s{0,3}(`{3,}|~{3,})/.exec(line);
      if (f && (!fence || (f[1][0] === fence[0] && f[1].length >= fence.length))) fence = fence ? null : f[1];
      pos += line.length + 1;
      if (!fence && line.trim() === "" && pos <= s.length) at = pos;
    }
    return at;
  };
  // The state, and the list item it's in (a chat message, say) when it's in one.
  const env = (state) => {
    const scope = api.scope?.();
    return scope ? { ...state, ...scope } : state;
  };
  const draw = (state) => {
    const v = window.__irender.evaluate(text, env(state));
    const raw = v === null || v === undefined ? "" : typeof v === "object" ? JSON.stringify(v) : String(v);
    const live = !!(streaming && window.__irender.evaluate(streaming, env(state)));
    const key = (live ? "1" : "0") + raw;
    if (key === last) return;
    last = key;
    const M = window.__irenderMarkdown;
    if (!live) {
      // Finished (or never streamed): the whole text, once, as the server would draw it.
      done = null;
      el.innerHTML = M.render(raw);
    } else {
      const cut = boundary(raw), head = raw.slice(0, cut);
      const nodes = (md) => {
        const t = document.createElement("template");
        t.innerHTML = M.render(md);
        return t.content;
      };
      if (done !== null && head.startsWith(done)) {
        // The block still arriving goes; blocks finished since are added after the others.
        while (el.childNodes.length > doneNodes) el.lastChild.remove();
        if (head !== done) el.append(nodes(head.slice(done.length)));
      } else el.innerHTML = M.render(head);
      done = head;
      doneNodes = el.childNodes.length;
      el.append(nodes(M.complete(raw.slice(cut))));
    }
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
