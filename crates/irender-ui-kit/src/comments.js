// irender-ui-kit: the `ui-comments` behaviour. The comments are a list in state; this
// draws them, with a box to write in, and writes back what's added, edited, deleted,
// resolved or reacted to, each with an event (`comment`, `edit`, `delete`, `resolve`,
// `react`). The text is drawn as text: mentions marked, web addresses linked; with
// `markdown`, a little Markdown too (see `md`), built as elements, never as HTML.
(window.__irenderBehaviors ||= {})["ui-comments"] = (el, api) => {
  const cfg = api.config, K = window.__irenderKit, M = window.__irenderMentions, L = (t, v) => window.__irender.label(t, v);
  const { make, button, icon } = K;
  const REACTIONS = Array.isArray(cfg.reactions) ? cfg.reactions.map(String) : ["👍", "❤️", "🎉", "👀"];
  const list = () => (Array.isArray(api.state[cfg.bind]) ? api.state[cfg.bind] : []);
  const named = (w) => (K.isObj(w) ? String(w.name ?? "") : String(w ?? ""));
  const user = () => K.value(api, cfg.user) ?? null;
  const me = () => (user() ? named(user()) : null);
  const people = () => K.value(api, cfg.people) ?? [];
  const can = () => !!me() && !K.value(api, cfg.disabled);
  const newest = cfg.order === "newest";
  let replying = null, editing = null, picking = null, deleting = null;
  const open = new Set(); // resolved threads looked into
  const drafts = new Map();

  // ---- changes
  const set = (next, event, detail) => {
    api.set(cfg.bind, next);
    api.emit(event, detail);
  };
  // The comment `id` (or a reply) changed by `f`; null removes it.
  const change = (id, f) => list().flatMap((c) => {
    if (c.id === id) return f(c) ?? [];
    if (!Array.isArray(c.replies) || !c.replies.some((r) => r.id === id)) return [c];
    return [{ ...c, replies: c.replies.flatMap((r) => (r.id === id ? f(r) ?? [] : [r])) }];
  });
  function add(text, parent) {
    const c = { id: K.id(), who: user(), when: new Date().toISOString(), text };
    drafts.delete(parent ?? "");
    replying = null;
    if (parent) set(list().map((x) => (x.id === parent ? { ...x, replies: [...(x.replies ?? []), c] } : x)), "comment", { comment: c, parent });
    else set([...list(), c], "comment", { comment: c, parent: null });
    K.say(el, L("Comment added"));
  }
  function react(c, emoji) {
    const mine = me(), was = c.reactions?.[emoji] ?? [], on = !was.includes(mine);
    const next = on ? [...was, mine] : was.filter((n) => n !== mine);
    picking = null;
    set(change(c.id, (x) => {
      const r = { ...(x.reactions ?? {}), [emoji]: next };
      if (!next.length) delete r[emoji];
      return { ...x, reactions: r };
    }), "react", { id: c.id, emoji, on });
  }

  // ---- drawing
  const plain = (text) => M.split(text, people()).map((p, i) => (i % 2 ? make("span", { "data-mention": p.slice(1) === me() ? "me" : "" }, p) : K.linked(p)));
  // Markdown, the bits a comment needs: paragraphs, **bold**, *italic* (or _italic_),
  // ~~struck~~, `code`, fenced code, [links](https://…) (http(s), mail or a path on the
  // site), lists (- or 1.) and > quotes. Anything else is text; mentions and bare web
  // addresses work inside it as without it.
  const INLINE = [
    [/`([^`\n]+)`/, (m) => make("code", null, m[1])],
    [/\[([^\]\n]+)\]\(([^)\s]+)\)/, (m) => /^(https?:\/\/|mailto:|\/(?!\/)|#)/i.test(m[2]) ? make("a", { href: m[2], target: /^https?:/i.test(m[2]) ? "_blank" : null, rel: "noreferrer noopener" }, inline(m[1])) : m[0]],
    [/\*\*(?=\S)([^*\n]+?)\*\*/, (m) => make("strong", null, inline(m[1]))],
    [/~~(?=\S)([^~\n]+?)~~/, (m) => make("del", null, inline(m[1]))],
    [/(^|[^\w*])\*(?=\S)([^*\n]+?)\*(?!\w)/, (m) => [m[1], make("em", null, inline(m[2]))]],
    [/(^|[^\w])_(?=\S)([^_\n]+?)_(?!\w)/, (m) => [m[1], make("em", null, inline(m[2]))]],
  ];
  function inline(text) {
    const out = [];
    for (let rest = text; rest; ) {
      let best = null;
      for (const [re, f] of INLINE) {
        const m = re.exec(rest);
        if (m && (!best || m.index < best.m.index)) best = { m, f };
      }
      if (!best) {
        out.push(...plain(rest));
        break;
      }
      if (best.m.index) out.push(...plain(rest.slice(0, best.m.index)));
      out.push(...[].concat(best.f(best.m)).map((x) => (typeof x === "string" ? plain(x) : x)).flat());
      rest = rest.slice(best.m.index + best.m[0].length);
    }
    return out;
  }
  function md(text) {
    const lines = String(text).replace(/\r\n?/g, "\n").split("\n"), out = [];
    for (let i = 0; i < lines.length; ) {
      const l = lines[i];
      if (/^```/.test(l)) {
        const code = [];
        for (i++; i < lines.length && !/^```/.test(lines[i]); i++) code.push(lines[i]);
        i++;
        out.push(make("pre", null, make("code", null, code.join("\n"))));
      } else if (/^\s*[-*+]\s+/.test(l) || /^\s*\d+[.)]\s+/.test(l)) {
        const ordered = /^\s*\d/.test(l), items = [];
        while (i < lines.length && (ordered ? /^\s*\d+[.)]\s+/ : /^\s*[-*+]\s+/).test(lines[i])) items.push(lines[i++].replace(/^\s*(?:[-*+]|\d+[.)])\s+/, ""));
        out.push(make(ordered ? "ol" : "ul", null, items.map((x) => make("li", null, inline(x)))));
      } else if (/^>\s?/.test(l)) {
        const q = [];
        while (i < lines.length && /^>\s?/.test(lines[i])) q.push(lines[i++].replace(/^>\s?/, ""));
        out.push(make("blockquote", null, inline(q.join(" "))));
      } else if (!l.trim()) i++;
      else {
        // A paragraph: the lines up to a blank one or another block, each on its own line.
        const p = [];
        while (i < lines.length && lines[i].trim() && !/^(```|>|\s*[-*+]\s+|\s*\d+[.)]\s+)/.test(lines[i])) p.push(lines[i++]);
        out.push(make("p", null, p.flatMap((x, n) => (n ? [make("br"), ...inline(x)] : inline(x)))));
      }
    }
    return out;
  }
  const body = (text) => (cfg.markdown ? md(text) : plain(text));
  function box(key, { value = "", label, submit, cancel, done, focus }) {
    const area = make("textarea", { rows: 2, "aria-label": label, placeholder: K.value(api, cfg.placeholder) ?? L("Write a comment…"), maxlength: cfg.maxLength, "data-key": key });
    area.value = drafts.get(key) ?? value;
    const ok = make("button", { type: "button", "data-primary": "", "data-do": "send" }, submit);
    const left = cfg.maxLength ? make("small") : null;
    const check = () => {
      ok.disabled = !area.value.trim() || area.value.trim() === value;
      if (left) left.textContent = L("{n} characters left", { n: cfg.maxLength - area.value.length });
    };
    const send = () => area.value.trim() && done(area.value.trim());
    M(area, { people });
    area.addEventListener("input", () => (drafts.set(key, area.value), check()));
    area.addEventListener("keydown", (e) => {
      if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) e.preventDefault(), send();
      else if (e.key === "Escape" && cancel) e.preventDefault(), e.stopPropagation(), cancel();
    });
    ok.addEventListener("click", send);
    check();
    if (focus) setTimeout(() => area.focus());
    return make("div", { "data-c-box": "" }, area, make("div", { "data-c-actions": "" }, left ?? make("small"), cancel ? make("button", { type: "button", click: cancel }, L("Cancel")) : null, ok));
  }
  function comment(c, parent) {
    const own = cfg.editable && me() !== null && named(c.who) === me() && can();
    const item = make("li"), box0 = make("div", { "data-comment": c.id }), main = make("div");
    if (c.resolved) box0.setAttribute("data-resolved", "");
    const tools = make("span", { "data-c-tools": "" });
    const tool = (name, label, f, what) => tools.append(button({ "aria-label": label, title: label, click: f, "data-do": what }, icon(name, 15)));
    if (can()) {
      if (cfg.replies && !parent && !c.resolved) tool("reply", L("Reply"), () => ((replying = c.id), (editing = null), draw(true)), `reply-${c.id}`);
      if (cfg.resolvable && !parent) {
        tool(c.resolved ? "rotate" : "check", c.resolved ? L("Open again") : L("Resolve"), () => {
          open.delete(c.id);
          set(change(c.id, (x) => ({ ...x, resolved: !c.resolved })), "resolve", { id: c.id, resolved: !c.resolved });
        }, `resolve-${c.id}`);
      }
      if (own && !c.resolved) tool("edit", L("Edit"), () => ((editing = c.id), (replying = null), draw(true)), `edit-${c.id}`);
      if (own) tool("trash", L("Delete"), () => ((deleting = c.id), draw(true)), `delete-${c.id}`);
    }
    const t = make("time", { datetime: c.when, title: isNaN(new Date(c.when)) ? null : new Date(c.when).toLocaleString(cfg.locale || undefined) }, K.ago(c.when, cfg.locale));
    main.append(make("div", { "data-c-head": "" }, make("b", null, named(c.who) || L("Someone")), t, c.edited ? make("span", { "data-c-edited": "" }, L("edited")) : null, c.resolved ? make("span", { "data-c-badge": "" }, icon("check", 12), L("Resolved")) : null, tools));
    if (c.resolved) {
      const shown = open.has(c.id), n = (c.replies?.length ?? 0) + 1;
      main.append(make("button", { type: "button", "data-c-link": "", "data-c-open": shown, "aria-expanded": String(shown), "data-do": `open-${c.id}`, click: () => (shown ? open.delete(c.id) : open.add(c.id), draw(true)) }, shown ? L("Hide") : L(["Show {n} comment", "Show {n} comments"], { n })));
    }
    if (deleting === c.id) {
      main.append(make("div", { "data-c-actions": "", role: "alertdialog", "aria-label": L("Delete this comment?") }, make("small", null, (c.replies?.length ? L("Delete this comment and its replies?") : L("Delete this comment?"))),
        make("button", { type: "button", click: () => ((deleting = null), draw(true)) }, L("Keep")),
        make("button", { type: "button", "data-primary": "", "data-do": "confirm-delete", click: () => ((deleting = null), set(change(c.id, () => null), "delete", { id: c.id }), K.say(el, L("Comment deleted"))) }, L("Delete"))));
    }
    if (editing === c.id) {
      const stop = () => ((editing = null), drafts.delete("edit-" + c.id), draw(true));
      main.append(box("edit-" + c.id, { value: c.text ?? "", label: L("Edit comment"), submit: L("Save"), cancel: stop, focus: true, done: (text) => ((editing = null), drafts.delete("edit-" + c.id), set(change(c.id, (x) => ({ ...x, text, edited: true })), "edit", { id: c.id, text })) }));
    } else main.append(make(cfg.markdown ? "div" : "p", { "data-c-text": "", "data-md": cfg.markdown || null }, body(c.text ?? "")));
    const foot = make("div", { "data-c-foot": "" });
    for (const [emoji, who] of Object.entries(c.reactions ?? {})) {
      if (!Array.isArray(who) || !who.length) continue;
      foot.append(make("button", { type: "button", "data-c-chip": "", "data-do": `react-${c.id}-${emoji}`, "aria-pressed": String(who.includes(me())), disabled: !can(), title: who.join(", "), "aria-label": L("{emoji}, {n}: {names}", { emoji, n: who.length, names: who.join(", ") }), click: () => react(c, emoji) }, emoji, make("span", null, String(who.length))));
    }
    if (can() && REACTIONS.length && !c.resolved) {
      foot.append(button({ "data-c-react": "", "data-do": `pick-${c.id}`, "aria-label": L("Add a reaction"), title: L("Add a reaction"), "aria-expanded": String(picking === c.id), click: () => ((picking = picking === c.id ? null : c.id), draw(true)) }, icon("smile", 15)));
      if (picking === c.id) foot.append(make("div", { "data-c-picker": "", role: "group", "aria-label": L("Reactions") }, REACTIONS.map((r) => button({ "aria-label": r, click: () => react(c, r) }, r))));
    }
    main.append(foot);
    if (!parent) {
      const replies = Array.isArray(c.replies) ? c.replies : [];
      if (replies.length || replying === c.id) {
        const ol = make("ol", null, replies.map((r) => comment(r, c)));
        if (replying === c.id) {
          const stop = () => ((replying = null), draw(true));
          ol.append(make("li", null, box("reply-" + c.id, { label: L("Reply to {name}", { name: named(c.who) }), submit: L("Reply"), cancel: stop, focus: true, done: (text) => (drafts.delete("reply-" + c.id), add(text, c.id)) })));
        }
        main.append(ol);
      }
    }
    box0.append(K.avatar(c.who), main);
    item.append(box0);
    return item;
  }
  let was = null;
  function draw(force) {
    const sig = JSON.stringify([list(), user(), K.value(api, cfg.disabled), people(), K.value(api, cfg.placeholder)]);
    if (!force && sig === was) return;
    was = sig;
    // What has the focus keeps it: a box by its key (with its caret), a button by what it does.
    const f = el.contains(document.activeElement) ? document.activeElement : null;
    const key = f?.getAttribute("data-key"), act = f?.getAttribute("data-do"), caret = key != null ? [f.selectionStart, f.selectionEnd] : null;
    const all = newest ? [...list()].reverse() : list();
    const ol = make("ol", null, all.map((c) => comment(c, null)));
    const write = can() ? box("", { label: L("Write a comment"), submit: L("Comment"), done: (text) => add(text, null) }) : null;
    const none = all.length ? null : make("p", { "data-c-empty": "" }, K.value(api, cfg.emptyText) ?? L("No comments yet."));
    const live = el.querySelector(":scope > [data-kit-live]");
    el.replaceChildren(...[newest ? write : null, none ?? ol, newest ? null : write, live].filter(Boolean));
    if (key != null) {
      const a = el.querySelector(`textarea[data-key="${CSS.escape(key)}"]`);
      if (a) a.focus(), a.setSelectionRange(...caret);
    } else if (act) (el.querySelector(`[data-do="${CSS.escape(act)}"]`) ?? el.querySelector("textarea"))?.focus();
  }
  el.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && picking !== null) e.preventDefault(), (picking = null), draw(true);
  });
  document.addEventListener("pointerdown", (e) => {
    if (picking !== null && el.isConnected && !e.target.closest?.("[data-c-foot]")) (picking = null), draw(true);
  });
  draw();
  return { update: () => draw() };
};
