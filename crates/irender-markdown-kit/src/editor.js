// irender-markdown-kit: the `ui-markdown-editor` behaviour.
//
// `mode: rich` (the default) edits the formatted text itself, the way readers will see
// it, and writes Markdown to `bind` as it changes; its Markdown tab shows that text. The
// other modes edit the Markdown in a text area, with a preview.
//
// In the text area, the toolbar and shortcuts (⌘B bold, ⌘I italic, ⌘⇧X strikethrough,
// ⌘E code, ⌘K link) wrap the selection, or a placeholder word to type over; heading,
// quote and the lists prefix every selected line, and take the prefix off when all of
// them have it. Enter continues a list or quote (an empty item ends it), and Tab /
// Shift+Tab indent list lines. Edits go through the browser's own editing, so undo
// works, and fire `input`, so `bind` follows. The preview is rendered with markdown.js,
// as the server would.
//
// In rich text the same buttons and shortcuts format the selection, a link is typed in
// a bar under the toolbar, and starting a line with `#`, `-`, `1.`, `>` or `[ ]` and a
// space turns it into a heading, list, quote or task. Pasted pages and documents keep
// only what Markdown can hold.
(() => {
  // Markdown from HTML: what the rich editor holds, or a pasted page. Anything Markdown
  // can't say (colours, fonts, layout) is dropped and its text kept.
  const toMarkdown = (() => {
    const BLOCK = /^(P|DIV|H[1-6]|UL|OL|LI|BLOCKQUOTE|PRE|HR|TABLE|SECTION|ARTICLE|HEADER|FOOTER|MAIN|ASIDE|NAV|FIGURE)$/;
    const SKIP = /^(SCRIPT|STYLE|TEMPLATE|HEAD|TITLE|META|LINK|NOSCRIPT|IFRAME|OBJECT|SVG|BUTTON|SELECT|TEXTAREA)$/;
    const isBlock = (n) => n.nodeType === 1 && BLOCK.test(n.nodeName);

    // Text that would otherwise read as Markdown.
    const escapeText = (t) => t.replace(/[\\`*_[\]~<]/g, "\\$&");
    // A line that would start a heading, list, quote or rule.
    const escapeStart = (t) =>
      t.replace(/^(\s*)(#{1,6}(?=\s|$)|[-+>](?=[\s-]|$)|\d+(?=[.)](\s|$)))/, (m, sp, mark) => (/^\d/.test(mark) ? m + "\\" : sp + "\\" + mark));

    // `**` and the like around `inner`, with its edge spaces outside them.
    const around = (mark, inner, end = mark) => {
      const m = inner.match(/^(\s*)([\s\S]*?)(\s*)$/);
      return m[2] === "" ? inner : m[1] + mark + m[2] + end + m[3];
    };

    const inline = (node) => {
      let out = "";
      for (const n of node.childNodes) {
        if (n.nodeType === 3) {
          out += escapeText(n.data.replace(/[ \t\n\r]+/g, " ").replace(/ /g, " "));
          continue;
        }
        if (n.nodeType !== 1 || SKIP.test(n.nodeName)) continue;
        const tag = n.nodeName;
        const style = n.getAttribute("style") || "";
        if (tag === "BR") out += "\\\n";
        else if (tag === "INPUT") continue;
        else if (isBlock(n)) out += (out && !out.endsWith(" ") ? " " : "") + inline(n);
        else if (tag === "IMG") {
          const src = n.getAttribute("src") || "";
          if (src) out += `![${escapeText(n.getAttribute("alt") || "")}](${src.replace(/[()\s]/g, encodeURIComponent)})`;
        } else if (tag === "CODE" || tag === "KBD" || tag === "SAMP") {
          const t = n.textContent.replace(/ /g, " ").replace(/\n/g, " ");
          if (t === "") continue;
          let ticks = "`";
          while (t.includes(ticks)) ticks += "`";
          const pad = t.startsWith("`") || t.endsWith("`") ? " " : "";
          out += ticks + pad + t + pad + ticks;
        } else if (tag === "A") {
          const text = inline(n);
          const href = n.getAttribute("href") || "";
          out += href && text.trim() ? around("[", text, `](${href.replace(/[()\s]/g, encodeURIComponent)})`) : text;
        } else {
          let inner = inline(n);
          const bold = tag === "STRONG" || tag === "B" || /font-weight:\s*(bold|[6-9]00)/.test(style);
          const italic = tag === "EM" || tag === "I" || /font-style:\s*italic/.test(style);
          const strike = tag === "DEL" || tag === "S" || tag === "STRIKE" || /line-through/.test(style);
          if (strike) inner = around("~~", inner);
          if (italic) inner = around("_", inner);
          if (bold) inner = around("**", inner);
          out += inner;
        }
      }
      return out;
    };
    // A paragraph's text: no hard break at its end, no spaces at its edges, and no line
    // that would start a block.
    const para = (node) =>
      inline(node)
        .replace(/(\\\n)+$/, "")
        .replace(/^ +| +$/g, "")
        .split(/ *\\\n */)
        .map(escapeStart)
        .join("\\\n");

    // The text in a code block, `<br>`s as line breaks.
    const codeText = (node) => {
      let out = "";
      for (const n of node.childNodes) {
        if (n.nodeType === 3) out += n.data.replace(/ /g, " ");
        else if (n.nodeName === "BR") out += "\n";
        else if (n.nodeType === 1) out += (isBlock(n) && out && !out.endsWith("\n") ? "\n" : "") + codeText(n);
      }
      return out;
    };

    const list = (node, depth) => {
      let n = Number(node.getAttribute("start")) || 1;
      const items = [];
      for (const li of node.children) {
        if (li.nodeName !== "LI") {
          if (li.nodeName === "UL" || li.nodeName === "OL") items.push(list(li, depth));
          continue;
        }
        const mark = node.nodeName === "OL" ? `${n++}. ` : "- ";
        const box = li.querySelector(":scope > input[type=checkbox]");
        const task = box || li.classList.contains("task") ? (box?.checked ? "[x] " : "[ ] ") : "";
        const body = blocks(li, depth + 1);
        const pad = " ".repeat(mark.length);
        const text = body.split("\n").map((l, i) => (i === 0 ? l : l === "" ? "" : pad + l)).join("\n");
        items.push(mark + task + text);
      }
      return items.join("\n");
    };

    const table = (node) => {
      const rows = [...node.querySelectorAll("tr")].filter((r) => r.closest("table") === node);
      if (!rows.length) return "";
      const cells = (r) => [...r.children].filter((c) => c.nodeName === "TD" || c.nodeName === "TH");
      const width = Math.max(...rows.map((r) => cells(r).length));
      const cell = (c) => (c ? para(c).replace(/\\\n/g, " ").replace(/\|/g, "\\|") : "");
      const line = (r) => "| " + Array.from({ length: width }, (_, k) => cell(cells(r)[k])).join(" | ") + " |";
      const align = (c) => {
        const a = (c?.style.textAlign || c?.getAttribute("align") || "").toLowerCase();
        return a === "center" ? ":---:" : a === "right" ? "---:" : a === "left" ? ":---" : "---";
      };
      return [line(rows[0]), "| " + Array.from({ length: width }, (_, k) => align(cells(rows[0])[k])).join(" | ") + " |", ...rows.slice(1).map(line)].join("\n");
    };

    // The blocks in `node`, a blank line between them. Inline content between blocks
    // (a `div` holding text then a list) is a paragraph of its own.
    const blocks = (node, depth = 0) => {
      const out = [];
      let run = document.createElement("div");
      const flush = () => {
        const t = para(run);
        if (t.trim() !== "") out.push(t);
        run = document.createElement("div");
      };
      for (const n of [...node.childNodes]) {
        if (n.nodeType === 1 && SKIP.test(n.nodeName)) continue;
        if (!isBlock(n)) {
          run.append(n.cloneNode(true));
          continue;
        }
        flush();
        const tag = n.nodeName;
        let t = "";
        if (/^H[1-6]$/.test(tag)) {
          const text = para(n).replace(/\\\n/g, " ");
          if (text.trim()) t = "#".repeat(Number(tag[1])) + " " + text;
        } else if (tag === "UL" || tag === "OL") t = list(n, depth);
        else if (tag === "BLOCKQUOTE") {
          const inner = blocks(n, depth);
          if (inner.trim()) t = inner.split("\n").map((l) => (l === "" ? ">" : "> " + l)).join("\n");
        } else if (tag === "PRE") {
          const code = codeText(n).replace(/\n$/, "");
          const lang = (n.querySelector("code")?.className.match(/language-([\w+\-.#]+)/) || [])[1] || "";
          let fence = "```";
          while (code.includes(fence)) fence += "`";
          t = `${fence}${lang}\n${code}\n${fence}`;
        } else if (tag === "HR") t = "---";
        else if (tag === "TABLE") t = table(n);
        else if (tag === "LI") {
          // An item outside a list, as a pasted fragment can have.
          const ul = document.createElement("ul");
          ul.append(n.cloneNode(true));
          t = list(ul, depth);
        }
        else t = blocks(n, depth);
        if (t.trim() !== "") out.push(t);
      }
      flush();
      // A list item's own text and its sub-list sit on neighbouring lines.
      if (node.nodeName === "LI") return out.join("\n");
      return out.join("\n\n");
    };

    return (node) => blocks(node).replace(/\n{3,}/g, "\n\n").trim();
  })();

  (window.__irenderBehaviors ||= {})["ui-markdown-editor"] = (el, api) => {
    const { max } = api.config;
    const area = el.querySelector("textarea");
    const body = el.querySelector("[data-md-body]");
    const preview = el.querySelector("[data-md-preview]");
    const rich = el.querySelector("[data-md-rich]");
    const linkbar = el.querySelector("[data-md-linkbar]");
    const count = el.querySelector("[data-md-count]");
    const tabs = [...el.querySelectorAll("[data-md-view]")];
    const md = window.__irenderMarkdown;
    // Interface text in the document's words (its `labels`), by the English.
    const L = (text, vars) => window.__irender.label(text, vars);
    // Rich text is showing (not its Markdown tab).
    const isRich = () => rich && body.dataset.view === "rich";

    // ------------------------------------------------------------ rich text

    // The Markdown the rich editor last wrote, so its own change isn't drawn back over it.
    let written = null;
    const fill = () => {
      written = area.value;
      rich.innerHTML = area.value.trim() === "" ? "<p><br></p>" : md.render(area.value);
      for (const box of rich.querySelectorAll("li.task > input[type=checkbox]")) fixBox(box);
      empty();
    };
    const fixBox = (box) => {
      box.disabled = false;
      box.contentEditable = "false";
      box.setAttribute("aria-label", L("Done"));
    };
    const empty = () => rich.toggleAttribute("data-empty", rich.textContent.trim() === "" && !rich.querySelector("img, hr, li, input"));
    const sync = () => {
      // A task item made by Enter has the class but no box yet.
      for (const li of rich.querySelectorAll("li.task")) {
        if (li.firstElementChild?.nodeName !== "INPUT") {
          const box = document.createElement("input");
          box.type = "checkbox";
          fixBox(box);
          const r = caretRange();
          const here = r?.collapsed && r.startContainer === li && r.startOffset === 0;
          li.prepend(box);
          if (here) place(li, 1);
        }
      }
      if (!rich.firstChild) rich.innerHTML = "<p><br></p>";
      const text = toMarkdown(rich);
      empty();
      if (text === area.value) return;
      written = text;
      area.value = text;
      area.dispatchEvent(new Event("input", { bubbles: true }));
    };

    // The block element the caret is in, inside the editor.
    const caretRange = () => {
      const sel = getSelection();
      if (!sel.rangeCount) return null;
      const r = sel.getRangeAt(0);
      return rich.contains(r.commonAncestorContainer) ? r : null;
    };
    const closest = (sel) => {
      const r = caretRange();
      if (!r) return null;
      const n = r.startContainer.nodeType === 1 ? r.startContainer : r.startContainer.parentElement;
      const found = n?.closest(sel);
      return found && rich.contains(found) ? found : null;
    };
    const blockAt = () => closest("p, div, li, h1, h2, h3, h4, h5, h6, pre, td, th");
    const place = (node, at = 0) => {
      const r = document.createRange();
      r.setStart(node, at);
      r.collapse(true);
      getSelection().removeAllRanges();
      getSelection().addRange(r);
    };
    const exec = (cmd, value = null) => {
      rich.focus();
      document.execCommand(cmd, false, value);
    };
    const esc = (s) => s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);
    // Takes an element's children out of it, in its place.
    const unwrap = (n) => {
      n.replaceWith(...n.childNodes);
    };

    const toggleBlock = (tag) => {
      const cur = closest("h1, h2, h3, h4, h5, h6, pre");
      exec("formatBlock", cur && cur.nodeName === tag.toUpperCase() ? "p" : tag);
    };
    const richQuote = () => {
      const q = closest("blockquote");
      if (q) {
        unwrap(q);
        sync();
        return;
      }
      // Every top-level block the selection touches goes into the quote.
      const r = caretRange();
      if (!r) return;
      const top = (n) => {
        while (n && n.parentNode !== rich) n = n.parentNode;
        return n;
      };
      let a = top(r.startContainer === rich ? rich.childNodes[r.startOffset] || rich.lastChild : r.startContainer);
      let b = top(r.endContainer === rich ? rich.childNodes[Math.max(0, r.endOffset - 1)] || rich.lastChild : r.endContainer);
      if (!a) return;
      if (a.nodeType === 3 || a.nodeName === "BR") {
        const p = document.createElement("p");
        a.replaceWith(p);
        p.append(a);
        if (b === a) b = p;
        a = p;
      }
      const q2 = document.createElement("blockquote");
      // Moving the blocks loses the selection; it's put back on the same nodes after.
      const { startContainer, startOffset, endContainer, endOffset } = r;
      a.before(q2);
      let n = a;
      while (n) {
        const next = n === b ? null : n.nextSibling;
        q2.append(n);
        n = next;
      }
      const back = document.createRange();
      back.setStart(startContainer, startOffset);
      back.setEnd(endContainer, endOffset);
      getSelection().removeAllRanges();
      getSelection().addRange(back);
      sync();
    };
    const listOf = (ordered) => {
      const cur = closest("ul, ol");
      const task = cur?.querySelector(":scope > li.task");
      if (cur && !ordered && task) {
        // A task list becomes a plain one.
        for (const li of cur.querySelectorAll(":scope > li.task")) {
          li.classList.remove("task");
          li.querySelector(":scope > input")?.remove();
        }
        sync();
        return;
      }
      exec(ordered ? "insertOrderedList" : "insertUnorderedList");
    };
    const tasks = () => {
      let cur = closest("ul, ol");
      if (cur?.querySelector(":scope > li.task")) {
        // Off again: back to a paragraph.
        for (const li of cur.querySelectorAll(":scope > li.task")) li.querySelector(":scope > input")?.remove();
        exec(cur.nodeName === "OL" ? "insertOrderedList" : "insertUnorderedList");
        return;
      }
      if (!cur || cur.nodeName === "OL") exec("insertUnorderedList");
      cur = closest("ul");
      if (!cur) return;
      for (const li of cur.querySelectorAll(":scope > li")) li.classList.add("task");
      sync();
    };
    const richCode = () => {
      const r = caretRange();
      if (!r) return;
      const c = closest("code");
      if (c && c.closest("pre")) return toggleBlock("pre");
      if (c) {
        unwrap(c);
        sync();
        return;
      }
      const text = r.toString();
      if (text.includes("\n") || (text === "" && (blockAt()?.textContent.trim() ?? "") === "")) return toggleBlock("pre");
      exec("insertHTML", `<code>${esc(text || L("code"))}</code>`);
    };

    // The link bar: opened on the selection, which is put back when it closes.
    let saved = null;
    const urlField = linkbar?.querySelector("input");
    const openLink = () => {
      const r = caretRange();
      if (!r || !linkbar) return;
      saved = r.cloneRange();
      const a = closest("a");
      linkbar.hidden = false;
      urlField.value = a?.getAttribute("href") || "";
      linkbar.querySelector("[data-md-link=remove]").hidden = !a;
      urlField.focus();
      urlField.select();
    };
    const closeLink = (act) => {
      linkbar.hidden = true;
      const url = urlField.value.trim();
      rich.focus();
      if (saved) {
        getSelection().removeAllRanges();
        getSelection().addRange(saved);
      }
      const a = closest("a");
      if (act === "remove" || (act === "apply" && url === "" && a)) {
        if (a) {
          const r = document.createRange();
          r.selectNodeContents(a);
          getSelection().removeAllRanges();
          getSelection().addRange(r);
        }
        exec("unlink");
      } else if (act === "apply" && url !== "") {
        // A bare domain is a web address.
        const href = /^([a-z][a-z0-9+.-]*:|#|\/|\.)/i.test(url) ? url : /^[^\s@]+@[^\s@]+$/.test(url) ? "mailto:" + url : "https://" + url;
        if (a) {
          a.setAttribute("href", href);
          sync();
        } else if (getSelection().isCollapsed) exec("insertHTML", `<a href="${esc(href)}">${esc(url)}</a>`);
        else exec("createLink", href);
      }
      saved = null;
    };
    urlField?.addEventListener("keydown", (e) => {
      if (e.key === "Enter") {
        e.preventDefault();
        closeLink("apply");
      } else if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        closeLink("cancel");
      }
    });

    const richRun = {
      bold: () => exec("bold"),
      italic: () => exec("italic"),
      strike: () => exec("strikeThrough"),
      code: richCode,
      link: openLink,
      heading: () => toggleBlock("h2"),
      quote: richQuote,
      bullets: () => listOf(false),
      numbers: () => listOf(true),
      tasks,
    };

    // Which buttons are on where the caret is.
    const STATE = { bold: "bold", italic: "italic", strike: "strikeThrough" };
    const pressed = () => {
      if (!rich || !el.isConnected || !caretRange()) return;
      const on = {
        heading: !!closest("h1, h2, h3, h4, h5, h6"),
        quote: !!closest("blockquote"),
        code: !!closest("code, pre"),
        link: !!closest("a"),
        bullets: closest("ul, ol")?.nodeName === "UL" && !closest("li.task"),
        numbers: closest("ul, ol")?.nodeName === "OL",
        tasks: !!closest("li.task"),
      };
      for (const [k, cmd] of Object.entries(STATE)) on[k] = document.queryCommandState(cmd);
      for (const b of el.querySelectorAll("[data-md]")) b.setAttribute("aria-pressed", String(!!on[b.dataset.md]));
    };

    // `#`, `-`, `1.`, `>` or `[ ]` then a space, at the start of a paragraph.
    const autoformat = () => {
      // Firefox types into the editor itself when it has no paragraph yet.
      if (blockAt() === rich) exec("formatBlock", "p");
      const block = blockAt();
      const r = caretRange();
      if (!block || !r || !r.collapsed || !/^(P|DIV)$/.test(block.nodeName) || block === rich || block.closest("li, pre")) return false;
      const before = document.createRange();
      before.setStart(block, 0);
      before.setEnd(r.startContainer, r.startOffset);
      const typed = before.toString();
      const act = /^#{1,6}$/.test(typed)
        ? () => exec("formatBlock", "h" + typed.length)
        : /^[-*+]$/.test(typed)
          ? () => exec("insertUnorderedList")
          : /^\d+[.)]$/.test(typed)
            ? () => exec("insertOrderedList")
            : typed === ">"
              ? richQuote
              : /^\[ ?\]$/.test(typed)
                ? tasks
                : typed === "```"
                  ? () => exec("formatBlock", "pre")
                  : null;
      if (!act) return false;
      getSelection().removeAllRanges();
      getSelection().addRange(before);
      exec("delete");
      act();
      return true;
    };

    // Enter on an empty line leaves a quote or code block.
    const leave = () => {
      const pre = closest("pre");
      const r = caretRange();
      if (pre && r?.collapsed) {
        const after = document.createRange();
        after.setStart(r.startContainer, r.startOffset);
        after.setEnd(pre, pre.childNodes.length);
        const before = document.createRange();
        before.setStart(pre, 0);
        before.setEnd(r.startContainer, r.startOffset);
        // Chrome breaks lines in a code block with `<br>`s, which a range's text leaves out.
        const lines = (range) => {
          let t = "";
          const walk = document.createTreeWalker(range.cloneContents(), NodeFilter.SHOW_ALL);
          while (walk.nextNode()) t += walk.currentNode.nodeType === 3 ? walk.currentNode.data : walk.currentNode.nodeName === "BR" ? "\n" : "";
          return t;
        };
        const text = lines(before);
        // At the end, past the line break browsers keep there so the last line shows.
        if (/^\n?$/.test(lines(after)) && text.endsWith("\n")) {
          const keep = text.replace(/\n+$/, "");
          const target = pre.querySelector("code") || pre;
          target.textContent = keep;
          const p = document.createElement("p");
          p.innerHTML = "<br>";
          pre.after(p);
          place(p);
          sync();
          return true;
        }
        // A line break, not a new block (WebKit splits the block on a newline).
        exec("insertLineBreak");
        return true;
      }
      const block = blockAt();
      const q = closest("blockquote");
      if (q && block && block.parentNode === q && block.textContent.trim() === "" && block !== q.firstElementChild) {
        q.after(block);
        place(block);
        sync();
        return true;
      }
      return false;
    };

    if (rich) {
      document.execCommand("defaultParagraphSeparator", false, "p");
      fill();
      rich.addEventListener("input", sync);
      rich.addEventListener("change", sync);
      // A box is ticked by a click, without the caret going into it.
      rich.addEventListener("mousedown", (e) => {
        if (e.target.matches?.("input[type=checkbox]")) e.preventDefault();
      });
      rich.addEventListener("click", (e) => {
        if (!e.target.matches?.("input[type=checkbox]")) return;
        // The click already ticked it.
        sync();
      });
      rich.addEventListener("paste", (e) => {
        const html = e.clipboardData?.getData("text/html");
        const text = e.clipboardData?.getData("text/plain") ?? "";
        if (!html && !text) return;
        e.preventDefault();
        if (html) {
          // Parsed inert (no scripts run, no images load), then kept to what Markdown holds.
          const doc = new DOMParser().parseFromString(html, "text/html");
          exec("insertHTML", md.render(toMarkdown(doc.body)));
        } else if (closest("pre")) exec("insertText", text);
        else exec("insertHTML", md.render(text.replace(/\r\n?/g, "\n")).replace(/^<p>([\s\S]*)<\/p>$/, "$1"));
      });
      rich.addEventListener("keydown", (e) => {
        const mod = e.metaKey || e.ctrlKey;
        const key = e.key.toLowerCase();
        // ⌘U is kept from the browser: Markdown has no underline.
        const cmd = mod && !e.altKey && (e.shiftKey ? { x: "strike" } : { b: "bold", i: "italic", e: "code", k: "link", u: "none" })[key];
        if (cmd) {
          e.preventDefault();
          e.stopPropagation();
          richRun[cmd]?.();
          return;
        }
        if (e.key === " " && !mod && autoformat()) {
          e.preventDefault();
          return;
        }
        if (e.key === "Enter" && !mod && !e.shiftKey && leave()) {
          e.preventDefault();
          return;
        }
        if (e.key === "Tab" && !mod && closest("li")) {
          e.preventDefault();
          exec(e.shiftKey ? "outdent" : "indent");
        }
      });
      document.addEventListener("selectionchange", pressed);
      rich.addEventListener("keyup", pressed);
    }

    // ------------------------------------------------------------ Markdown text

    const draw = () => {
      const text = area.value;
      if (preview) {
        preview.innerHTML = text.trim() === "" ? `<p style="color: var(--color-muted, #71717a)">${esc(L("Nothing to preview"))}</p>` : md.render(text);
        window.__irenderCode?.enhance(preview);
      }
      if (rich) {
        if (text !== written) fill();
        rich.contentEditable = area.disabled ? "false" : "true";
      }
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
      const label = value.slice(a, b) || L("link text");
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
      } else wrap("`", "`", L("code"));
    };

    const run = (cmd) => {
      if (area.disabled) return;
      if (isRich()) return richRun[cmd]?.();
      ({
        bold: () => wrap("**", "**", L("bold text")),
        italic: () => wrap("_", "_", L("italic text")),
        strike: () => wrap("~~", "~~", L("struck text")),
        code,
        link,
        heading: () => prefixLines("heading"),
        quote: () => prefixLines("quote"),
        bullets: () => prefixLines("bullets"),
        numbers: () => prefixLines("numbers"),
        tasks: () => prefixLines("tasks"),
      })[cmd]?.();
    };

    // The toolbar doesn't take focus, so the selection it works on stays.
    el.addEventListener("mousedown", (e) => {
      if (e.target.closest("[data-md], [data-md-view]")) e.preventDefault();
    });
    el.addEventListener("click", (e) => {
      const act = e.target.closest("[data-md-link]");
      if (act && el.contains(act)) closeLink(act.dataset.mdLink);
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
      if (linkbar) linkbar.hidden = true;
      if (view === "preview") draw();
      else if (view === "rich") {
        if (area.value !== written) fill();
        rich.focus();
      } else {
        for (const b of el.querySelectorAll("[data-md]")) b.removeAttribute("aria-pressed");
        area.focus();
      }
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
})();
