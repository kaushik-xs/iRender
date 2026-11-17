// irender-markdown-kit: the `ui-markdown-editor` behaviour.
//
// The toolbar and shortcuts (⌘B bold, ⌘I italic, ⌘⇧X strikethrough, ⌘E code, ⌘K link)
// wrap the selection, or a placeholder word to type over; heading, quote and the lists
// prefix every selected line, and take the prefix off when all of them have it. Enter
// continues a list or quote (an empty item ends it), and Tab / Shift+Tab indent list
// lines. Edits go through the browser's own editing, so undo works, and fire `input`,
// so `bind` follows. The preview is rendered with markdown.js, as the server would.
(window.__irenderBehaviors ||= {})["ui-markdown-editor"] = (el, api) => {
  const { max } = api.config;
  const area = el.querySelector("textarea");
  const body = el.querySelector("[data-md-body]");
  const preview = el.querySelector("[data-md-preview]");
  const count = el.querySelector("[data-md-count]");
  const tabs = [...el.querySelectorAll("[data-md-view]")];

  const draw = () => {
    const text = area.value;
    preview.innerHTML = text.trim() === "" ? '<p style="color: var(--color-muted, #71717a)">Nothing to preview</p>' : window.__irenderMarkdown.render(text);
    if (count) {
      const n = Array.from(text).length;
      count.textContent = `${n}/${max}`;
      count.toggleAttribute("data-over", n > max);
    }
  };
  // Once per burst of edits, after the current one.
  let pending = false;
  const later = () => {
    if (pending) return;
    pending = true;
    setTimeout(() => {
      pending = false;
      draw();
    }, 0);
  };
  area.addEventListener("input", later);

  // Replaces [from, to) with `text` through the browser's editing (so undo works), then
  // selects [selFrom, selTo).
  // WebKit's insertText of "" also deletes a newline before the selection, so the result
  // is checked, and set directly when it isn't what was asked for.
  const replace = (from, to, text, selFrom, selTo) => {
    const want = area.value.slice(0, from) + text + area.value.slice(to);
    area.focus();
    area.setSelectionRange(from, to);
    const ok = document.queryCommandSupported?.("insertText") && document.execCommand("insertText", false, text);
    if (!ok || area.value !== want) {
      area.value = want;
      area.dispatchEvent(new Event("input", { bubbles: true }));
    }
    area.setSelectionRange(selFrom, selTo);
  };

  // `**…**` and the like around the selection, or a word to type over.
  const wrap = (before, after, word) => {
    const { selectionStart: a, selectionEnd: b, value } = area;
    const sel = value.slice(a, b);
    if (sel && value.slice(a - before.length, a) === before && value.slice(b, b + after.length) === after) {
      replace(a - before.length, b + after.length, sel, a - before.length, b - before.length);
      return;
    }
    const inner = sel || word;
    replace(a, b, before + inner + after, a + before.length, a + before.length + inner.length);
  };

  // The selected lines, whole: [start of the first, end of the last].
  const lineRange = () => {
    const { selectionStart: a, selectionEnd: b, value } = area;
    const start = value.lastIndexOf("\n", a - 1) + 1;
    let end = value.indexOf("\n", b > a && value[b - 1] === "\n" ? b - 1 : b);
    if (end < 0) end = value.length;
    return [start, end];
  };

  const LIST = /^(\s*)([-*+]|\d+[.)])\s+(\[[ xX]\]\s+)?/;
  const prefixLines = (kind) => {
    const [start, end] = lineRange();
    const lines = area.value.slice(start, end).split("\n");
    const pattern = { heading: /^#{1,6}\s/, quote: /^>\s?/, bullets: /^\s*[-*+]\s(?!\[)/, numbers: /^\s*\d+[.)]\s/, tasks: /^\s*[-*+]\s\[[ xX]\]\s/ }[kind];
    const all = lines.filter((l) => l.trim() !== "").every((l) => pattern.test(l));
    const out = lines.map((l, i) => {
      if (all) return l.replace(pattern, "");
      if (l.trim() === "" && lines.length > 1) return l;
      const bare = kind === "heading" || kind === "quote" ? l : l.replace(LIST, "$1");
      const lead = (bare.match(/^\s*/) || [""])[0];
      const rest = bare.slice(lead.length);
      const mark = { heading: "## ", quote: "> ", bullets: "- ", numbers: `${i + 1}. `, tasks: "- [ ] " }[kind];
      return kind === "heading" || kind === "quote" ? mark + bare : lead + mark + rest;
    });
    const text = out.join("\n");
    replace(start, end, text, start, start + text.length);
  };

  const link = () => {
    const { selectionStart: a, selectionEnd: b, value } = area;
    const label = value.slice(a, b) || "link text";
    const text = `[${label}](https://)`;
    // Select the URL to type over.
    const at = a + label.length + 3;
    replace(a, b, text, at, at + 8);
  };

  const code = () => {
    const { selectionStart: a, selectionEnd: b, value } = area;
    if (value.slice(a, b).includes("\n")) {
      const [start, end] = lineRange();
      const block = "```\n" + value.slice(start, end) + "\n```";
      replace(start, end, block, start + 4, start + block.length - 4);
    } else wrap("`", "`", "code");
  };

  const run = (cmd) => {
    if (area.disabled) return;
    ({
      bold: () => wrap("**", "**", "bold text"),
      italic: () => wrap("_", "_", "italic text"),
      strike: () => wrap("~~", "~~", "struck text"),
      code,
      link,
      heading: () => prefixLines("heading"),
      quote: () => prefixLines("quote"),
      bullets: () => prefixLines("bullets"),
      numbers: () => prefixLines("numbers"),
      tasks: () => prefixLines("tasks"),
    })[cmd]?.();
  };

  el.addEventListener("click", (e) => {
    const tool = e.target.closest("[data-md]");
    if (tool && el.contains(tool)) {
      e.preventDefault();
      run(tool.dataset.md);
    }
    const tab = e.target.closest("[data-md-view]");
    if (tab && el.contains(tab)) show(tab.dataset.mdView);
  });

  const show = (view) => {
    body.dataset.view = view;
    tabs.forEach((t) => t.setAttribute("aria-selected", String(t.dataset.mdView === view)));
    if (view === "preview") draw();
    else area.focus();
  };

  area.addEventListener("keydown", (e) => {
    const mod = e.metaKey || e.ctrlKey;
    const key = e.key.toLowerCase();
    const shortcut = mod && !e.altKey && (e.shiftKey ? { x: "strike" } : { b: "bold", i: "italic", e: "code", k: "link" })[key];
    if (shortcut) {
      // Kept from the page too, so the sidebar's ⌘B doesn't also fire.
      e.preventDefault();
      e.stopPropagation();
      run(shortcut);
      return;
    }
    if (e.key === "Enter" && !mod && !e.shiftKey && area.selectionStart === area.selectionEnd) {
      const { selectionStart: a, value } = area;
      const start = value.lastIndexOf("\n", a - 1) + 1;
      const line = value.slice(start, a);
      const m = line.match(/^(\s*)([-*+]|\d+[.)])(\s+)(\[[ xX]\]\s+)?/) || line.match(/^(\s*)(>)(\s?)()/);
      if (!m) return;
      e.preventDefault();
      // An empty item ends the list.
      if (line.slice(m[0].length).trim() === "") {
        replace(start, a, "", start, start);
        return;
      }
      const num = m[2].match(/^(\d+)([.)])$/);
      const marker = num ? `${Number(num[1]) + 1}${num[2]}` : m[2];
      const next = `\n${m[1]}${marker}${m[3] || " "}${m[4] ? "[ ] " : ""}`;
      replace(a, a, next, a + next.length, a + next.length);
      return;
    }
    if (e.key === "Tab" && !mod) {
      const [start, end] = lineRange();
      const lines = area.value.slice(start, end).split("\n");
      // Only list lines: elsewhere Tab moves on, as it should.
      if (!lines.every((l) => l.trim() === "" || LIST.test(l))) return;
      e.preventDefault();
      const text = lines.map((l) => (e.shiftKey ? l.replace(/^ {1,2}/, "") : "  " + l)).join("\n");
      replace(start, end, text, start, start + text.length);
    }
  });

  return {
    // The bound key changed elsewhere (or this input set it): the preview follows.
    update: () => later(),
  };
};
