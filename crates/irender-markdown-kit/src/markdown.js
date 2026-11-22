// irender-markdown-kit: the Markdown renderer, the same as markdown.rs line for line,
// so a live preview matches what the server renders (the parity test holds them to it).
// A CommonMark subset with GitHub's tables, task lists, strikethrough and bare links.
// Every piece of text is escaped, raw HTML shows as text, and a link or image whose
// URL isn't http(s):, mailto:, a #fragment or relative renders as its text alone.
(() => {
  const PUNCT = /^[!-/:-@[-`{-~]$/;
  const isPunct = (c) => c !== undefined && PUNCT.test(c);
  const word = (c) => c !== undefined && /^[A-Za-z0-9]$/.test(c);
  const space = (c) => c === undefined || c === " " || c === "\n";
  const repeat = (c, n) => c.repeat(n);

  const escape = (s) =>
    s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);

  // ---------------------------------------------------------------- blocks

  const blank = (line) => line.trim() === "";
  const indent = (line) => line.length - line.replace(/^ +/, "").length;

  const fence = (line) => {
    if (indent(line) > 3) return null;
    const t = line.trimStart();
    const c = t[0];
    if (c !== "`" && c !== "~") return null;
    let n = 0;
    while (t[n] === c) n++;
    if (n < 3) return null;
    const info = t.slice(n).trim();
    if (c === "`" && info.includes("`")) return null;
    return { c, n, lang: info.split(" ")[0] };
  };

  const heading = (line) => {
    if (indent(line) > 3) return null;
    const t = line.trimStart();
    let level = 0;
    while (t[level] === "#") level++;
    if (level === 0 || level > 6) return null;
    const rest = t.slice(level);
    if (rest !== "" && !rest.startsWith(" ")) return null;
    let text = rest.trim();
    const closing = text.replace(/#+$/, "");
    if (closing === "") text = "";
    else if (closing.endsWith(" ")) text = closing.trimEnd();
    return { level, text };
  };

  const rule = (line) => {
    if (indent(line) > 3) return false;
    const t = Array.from(line).filter((c) => c !== " ");
    return t.length >= 3 && ["-", "*", "_"].some((c) => t.every((x) => x === c));
  };

  const quote = (line) => indent(line) <= 3 && line.trimStart().startsWith(">");

  const marker = (line) => {
    const ind = indent(line);
    const t = line.slice(ind);
    let ordered, number, len;
    if (/^[-*+]/.test(t)) {
      ordered = false;
      number = 0;
      len = 1;
    } else {
      const digits = (t.match(/^[0-9]*/) || [""])[0].length;
      if (digits === 0 || digits > 9) return null;
      const d = t[digits];
      if (d !== "." && d !== ")") return null;
      ordered = true;
      number = parseInt(t.slice(0, digits), 10);
      len = digits + 1;
    }
    const after = t.slice(len);
    if (after === "") return { indent: ind, ordered, number, content: ind + len + 1 };
    if (!after.startsWith(" ")) return null;
    const spaces = after.length - after.replace(/^ +/, "").length;
    const gap = spaces > 4 || after.trim() === "" ? 1 : spaces;
    return { indent: ind, ordered, number, content: ind + len + gap };
  };

  const tableRow = (line) => line.includes("|");

  const splitRow = (line) => {
    let t = line.trim();
    if (t.startsWith("|")) t = t.slice(1);
    if (t.endsWith("|") && !t.endsWith("\\|")) t = t.slice(0, -1);
    const cells = [""];
    const chars = Array.from(t);
    for (let i = 0; i < chars.length; i++) {
      if (chars[i] === "\\" && chars[i + 1] === "|") {
        cells[cells.length - 1] += "|";
        i++;
      } else if (chars[i] === "|") cells.push("");
      else cells[cells.length - 1] += chars[i];
    }
    return cells.map((c) => c.trim());
  };

  const delimiterRow = (line) => {
    const cells = splitRow(line);
    if (cells.length === 0) return null;
    const aligns = [];
    for (const raw of cells) {
      const c = raw.trim();
      const left = c.startsWith(":");
      const right = c.endsWith(":");
      const dashes = c.replace(/^:+|:+$/g, "");
      if (dashes === "" || !/^-+$/.test(dashes)) return null;
      aligns.push(left && right ? "center" : left ? "left" : right ? "right" : "");
    }
    return aligns;
  };

  const interrupts = (line) => {
    const m = marker(line);
    return !!(fence(line) || heading(line) || rule(line) || quote(line) || (m && (!m.ordered || m.number === 1)));
  };

  const blocks = (lines, tight) => {
    const out = [];
    const n = lines.length;
    let i = 0;
    while (i < n) {
      const line = lines[i];
      if (blank(line)) {
        i++;
        continue;
      }
      const f = fence(line);
      if (f) {
        const ind = indent(line);
        i++;
        const code = [];
        while (i < n) {
          const l = lines[i];
          const g = fence(l);
          if (g && g.c === f.c && g.n >= f.n && g.lang === "") {
            i++;
            break;
          }
          code.push(l.slice(Math.min(indent(l), ind)));
          i++;
        }
        const cls = f.lang !== "" && /^[A-Za-z0-9_+\-.#]+$/.test(f.lang) ? ` class="language-${escape(f.lang)}"` : "";
        out.push(`<pre><code${cls}>${escape(code.join("\n"))}</code></pre>`);
        continue;
      }
      const h = heading(line);
      if (h) {
        out.push(`<h${h.level}>${inline(h.text)}</h${h.level}>`);
        i++;
        continue;
      }
      if (rule(line)) {
        out.push("<hr>");
        i++;
        continue;
      }
      if (quote(line)) {
        const inner = [];
        while (i < n && quote(lines[i])) {
          const t = lines[i].trimStart().slice(1);
          inner.push(t.startsWith(" ") ? t.slice(1) : t);
          i++;
        }
        out.push(`<blockquote>\n${blocks(inner, false)}\n</blockquote>`);
        continue;
      }
      const m = marker(line);
      if (m) {
        const [html, next] = list(lines, i, m);
        out.push(html);
        i = next;
        continue;
      }
      const aligns = tableRow(line) && i + 1 < n ? delimiterRow(lines[i + 1]) : null;
      if (aligns) {
        const head = splitRow(line);
        if (head.length === aligns.length) {
          const cell = (tag, k, text) => {
            const align = aligns[k] || "";
            const style = align === "" ? "" : ` style="text-align: ${align}"`;
            return `<${tag}${style}>${inline(text)}</${tag}>`;
          };
          let html = "<table>\n<thead>\n<tr>" + head.map((c, k) => cell("th", k, c)).join("") + "</tr>\n</thead>";
          i += 2;
          const rows = [];
          while (i < n && !blank(lines[i]) && tableRow(lines[i])) {
            const cells = splitRow(lines[i]);
            rows.push("<tr>" + aligns.map((_, k) => cell("td", k, cells[k] ?? "")).join("") + "</tr>");
            i++;
          }
          if (rows.length) html += "\n<tbody>\n" + rows.join("\n") + "\n</tbody>";
          html += "\n</table>";
          out.push(html);
          continue;
        }
      }
      const para = [line.trimStart()];
      i++;
      while (i < n && !blank(lines[i]) && !interrupts(lines[i])) {
        para.push(lines[i].trimStart());
        i++;
      }
      const text = inline(para.join("\n").trimEnd());
      out.push(tight ? text : `<p>${text}</p>`);
    }
    return out.join("\n");
  };

  const list = (lines, start, first) => {
    const n = lines.length;
    const { ordered, number } = first;
    const items = [];
    let loose = false;
    let i = start;
    const sibling = (l) => {
      const m = marker(l);
      return m && m.ordered === ordered && m.indent < first.content ? m : null;
    };
    while (i < n) {
      const m = sibling(lines[i]);
      if (!m) break;
      const item = [lines[i].slice(m.content)];
      i++;
      while (i < n) {
        const l = lines[i];
        if (blank(l)) {
          let j = i;
          while (j < n && blank(lines[j])) j++;
          if (j < n && indent(lines[j]) >= m.content) {
            for (let k = i; k < j; k++) item.push("");
            loose = true;
            i = j;
            continue;
          }
          break;
        }
        if (indent(l) >= m.content) {
          item.push(l.slice(m.content));
          i++;
          continue;
        }
        if (marker(l) || interrupts(l)) break;
        item.push(l.trimStart());
        i++;
      }
      items.push(item);
      let j = i;
      while (j < n && blank(lines[j])) j++;
      if (j > i && j < n && sibling(lines[j])) {
        loose = true;
        i = j;
      }
    }
    const tag = ordered ? "ol" : "ul";
    let html = `<${tag}${ordered && number !== 1 ? ` start="${number}"` : ""}>`;
    for (const item of items) {
      let prefix = "";
      let cls = "";
      const b = item[0].slice(0, 4);
      if (item[0].length >= 4 && (b === "[ ] " || b === "[x] " || b === "[X] ")) {
        prefix = `<input type="checkbox" disabled${b === "[ ] " ? "" : " checked"}> `;
        cls = ' class="task"';
        item[0] = item[0].slice(4);
      }
      let body = blocks(item, !loose);
      body = loose ? `\n${body}\n` : body.includes("\n") ? `${body}\n` : body;
      html += `\n<li${cls}>${prefix}${body}</li>`;
    }
    return [html + `\n</${tag}>`, i];
  };

  // ---------------------------------------------------------------- inline

  const safeUrl = (url) => {
    const u = url.trim().toLowerCase();
    if (u.startsWith("http://") || u.startsWith("https://") || u.startsWith("mailto:")) return true;
    const head = u.split(/[/?#]/)[0];
    return !head.includes(":");
  };

  const codeEnd = (s, i) => {
    let n = 0;
    while (s[i + n] === "`") n++;
    let k = i + n;
    while (k < s.length) {
      if (s[k] === "`") {
        let m = 0;
        while (s[k + m] === "`") m++;
        if (m === n) return [n, k];
        k += m;
      } else k++;
    }
    return null;
  };

  const bracketEnd = (s, i) => {
    let depth = 0;
    for (let k = i; k < s.length; k++) {
      const c = s[k];
      if (c === "\\") k++;
      else if (c === "`") {
        const e = codeEnd(s, k);
        if (e) k = e[1] + e[0] - 1;
      } else if (c === "[") depth++;
      else if (c === "]") {
        depth--;
        if (depth === 0) return k;
      }
    }
    return null;
  };

  const destination = (s, i) => {
    if (s[i] !== "(") return null;
    let k = i + 1;
    while (s[k] === " ") k++;
    let url = "";
    if (s[k] === "<") {
      k++;
      while (k < s.length && s[k] !== ">" && s[k] !== "\n") url += s[k++];
      if (s[k] !== ">") return null;
      k++;
    } else {
      let depth = 0;
      while (k < s.length) {
        const c = s[k];
        if (c === " " || c === "\n") break;
        if (c === "\\" && k + 1 < s.length && isPunct(s[k + 1])) {
          url += s[k + 1];
          k += 2;
          continue;
        }
        if (c === "(") depth++;
        else if (c === ")") {
          if (depth === 0) break;
          depth--;
        }
        url += c;
        k++;
      }
    }
    while (s[k] === " ") k++;
    let title = null;
    if (s[k] === '"' || s[k] === "'") {
      const q = s[k];
      let t = "";
      k++;
      while (k < s.length && s[k] !== q) t += s[k++];
      if (s[k] !== q) return null;
      k++;
      title = t;
      while (s[k] === " ") k++;
    }
    if (s[k] !== ")") return null;
    return [url, title, k + 1];
  };

  const closer = (s, i, c, n) => {
    let k = i + n;
    while (k < s.length) {
      if (s[k] === "\\") k += 2;
      else if (s[k] === "`") {
        const e = codeEnd(s, k);
        k = e ? e[1] + e[0] : k + 1;
      } else if (s[k] === c) {
        let run = 0;
        while (s[k + run] === c) run++;
        if (run === n && k > i + n && !space(s[k - 1]) && (c !== "_" || !word(s[k + run]))) return k;
        k += run;
      } else k++;
    }
    return null;
  };

  const inline = (text) => {
    const s = Array.from(text);
    return span(s, 0, s.length);
  };

  const span = (all, from, to) => {
    const s = all.slice(0, to);
    let out = "";
    let i = from;
    while (i < to) {
      const c = s[i];
      const next = s[i + 1];
      if (c === "\\") {
        if (next === "\n") {
          out += "<br>\n";
          i += 2;
          continue;
        }
        if (isPunct(next)) {
          out += escape(next);
          i += 2;
          continue;
        }
      }
      if (c === "`") {
        const e = codeEnd(s, i);
        if (e) {
          const [n, end] = e;
          let code = s.slice(i + n, end).join("").replace(/\n/g, " ");
          if (code.length >= 2 && code.startsWith(" ") && code.endsWith(" ") && code.trim() !== "") code = code.slice(1, -1);
          out += `<code>${escape(code)}</code>`;
          i = end + n;
        } else {
          let n = 0;
          while (s[i + n] === "`") n++;
          out += repeat("`", n);
          i += n;
        }
        continue;
      }
      const image = c === "!" && next === "[";
      if (image || c === "[") {
        const open = image ? i + 1 : i;
        const close = bracketEnd(s, open);
        const d = close === null ? null : destination(s, close + 1);
        if (d) {
          const [url, title, end] = d;
          const t = title === null ? "" : ` title="${escape(title)}"`;
          if (image) {
            const alt = s.slice(open + 1, close).join("");
            out += safeUrl(url) ? `<img src="${escape(url.trim())}" alt="${escape(alt)}"${t}>` : escape(alt);
          } else {
            const inner = span(s, open + 1, close);
            out += safeUrl(url) ? `<a href="${escape(url.trim())}"${t}>${inner}</a>` : inner;
          }
          i = end;
          continue;
        }
      }
      if (c === "<") {
        let rest = "";
        let k = i + 1;
        while (k < s.length && s[k] !== ">" && s[k] !== " " && s[k] !== "\n" && s[k] !== "<") rest += s[k++];
        const lower = rest.toLowerCase();
        if (s[k] === ">" && (lower.startsWith("http://") || lower.startsWith("https://") || lower.startsWith("mailto:"))) {
          const shown = rest.startsWith("mailto:") ? rest.slice(7) : rest;
          out += `<a href="${escape(rest)}">${escape(shown)}</a>`;
          i += Array.from(rest).length + 2;
          continue;
        }
      }
      if ((c === "h" || c === "H") && !word(i > 0 ? s[i - 1] : undefined)) {
        const rest = [];
        for (let k = i; k < s.length && s[k] !== " " && s[k] !== "\n" && s[k] !== "<"; k++) rest.push(s[k]);
        const lower = rest.join("").toLowerCase();
        if (lower.startsWith("http://") || lower.startsWith("https://")) {
          const url = rest.slice();
          while (url.length && ".,:;!?\"')]*_~".includes(url[url.length - 1])) url.pop();
          if (url.length > 8) {
            const u = escape(url.join(""));
            out += `<a href="${u}">${u}</a>`;
            i += url.length;
            continue;
          }
        }
      }
      if (c === "*" || c === "_" || c === "~") {
        let n = 0;
        while (s[i + n] === c) n++;
        const valid = c === "~" ? n === 2 : n <= 3;
        const prev = i > 0 ? s[i - 1] : undefined;
        const k = valid && !space(s[i + n]) && (c !== "_" || !word(prev)) ? closer(s, i, c, n) : null;
        if (k !== null) {
          const inner = span(s, i + n, k);
          out += c === "~" ? `<del>${inner}</del>` : n === 1 ? `<em>${inner}</em>` : n === 2 ? `<strong>${inner}</strong>` : `<em><strong>${inner}</strong></em>`;
          i = k + n;
          continue;
        }
        out += repeat(c, n);
        i += n;
        continue;
      }
      if (c === "\n") {
        const trimmed = out.replace(/ +$/, "");
        const spaces = out.length - trimmed.length;
        out = trimmed + (spaces >= 2 ? "<br>\n" : "\n");
        i++;
        continue;
      }
      out += escape(c);
      i++;
    }
    return out;
  };

  const render = (src) => {
    const text = String(src ?? "").replace(/\r\n/g, "\n").replace(/\r/g, "\n").replace(/\t/g, "    ");
    return blocks(text.split("\n"), false);
  };

  // ------------------------------------------------------------ streaming

  // Markdown still arriving, finished at its end so it shows as it will once it has all
  // come: open `**`, `_`, `~~` and code spans are closed (a mark with nothing after it
  // yet is left out), a link or image still being written shows as its text, and a
  // table's header row gets its `---` row. Only the last block can be unfinished; an
  // open code fence already runs to the end.
  const complete = (src) => {
    const text = String(src ?? "").replace(/\r\n?/g, "\n");
    const lines = text.split("\n");
    let fenced = null;
    let start = 0;
    lines.forEach((l, i) => {
      const f = fence(l);
      if (fenced) {
        if (f && f.c === fenced.c && f.n >= fenced.n && f.lang === "") fenced = null;
      } else if (f) fenced = f;
      else if (blank(l)) start = i + 1;
    });
    if (fenced) return text;
    const head = lines.slice(0, start).join("\n") + (start ? "\n" : "");
    let last = lines.slice(start).join("\n");
    if (last.trim() === "") return text;

    // A table: its last cell, and under a header row, a `---` row however much has come.
    const rows = last.split("\n");
    if (/^\s*\|/.test(rows[0])) {
      if (rows.length === 1 && rows[0].trim() === "|") return head.trimEnd();
      const bars = (row) => row.replace(/\\\|/g, "..");
      const cells = (row) => bars(row).trim().replace(/^\|/, "").replace(/\|$/, "").split("|").length;
      if (rows.length === 2 && /^[\s:|-]*$/.test(rows[1])) rows.pop();
      const row = rows.pop();
      const cut = bars(row).lastIndexOf("|") + 1;
      rows.push(row.slice(0, cut) + (/\S/.test(row.slice(cut)) ? " " + unfinished(row.slice(cut).trim()) : row.slice(cut)));
      if (rows.length === 1) rows.push("|" + " --- |".repeat(cells(rows[0])));
      return head + rows.join("\n");
    }
    return head + unfinished(last);
  };

  // Text with its end finished: the last link or image, code span and emphasis.
  const unfinished = (last) => {
    // A link or image being written: its text, or nothing for an image.
    last = last
      .replace(/!\[[^\]\n]*(\](\([^)\n]*)?)?$/, "")
      .replace(/\[([^\]\n]*)(\](\([^)\n]*)?)?$/, "$1");

    // Code spans first: marks inside one aren't marks, so they're dots here.
    const dots = (t, a, b) => t.slice(0, a) + ".".repeat(b - a) + t.slice(b);
    let plain = last.replace(/\\[\s\S]/g, "..");
    let open = null;
    for (const m of plain.matchAll(/`+/g)) {
      if (!open) open = m;
      else if (m[0].length === open[0].length) {
        plain = dots(plain, open.index, m.index + m[0].length);
        open = null;
      }
    }
    if (open && last.slice(open.index).replace(/`/g, "").trim() === "") {
      last = last.slice(0, open.index);
      plain = plain.slice(0, open.index);
    } else if (open) {
      last = last.trimEnd() + open[0];
      plain = dots(plain.padEnd(last.length), open.index, last.length).slice(0, last.length);
    }
    // Marks at the very end may be half of a longer one (`**bold*`), and either close
    // what's open, which the end does anyway, or open nothing yet: they're left out.
    const end = plain.replace(/[*_~]+$/, "").length;
    plain = plain.slice(0, end);

    // Emphasis: a stack of open marks, each closed by the same mark.
    const stack = [];
    for (const m of plain.matchAll(/\*+|_+|~~/g)) {
      const before = plain[m.index - 1], after = plain[m.index + m[0].length];
      const run = m[0];
      const kinds = run.length === 3 ? [run.slice(0, 2), run[0]] : run.length < 3 ? [run] : [];
      const canClose = before !== undefined && !/\s/.test(before) && !(run[0] === "_" && word(after));
      // Not after a letter or digit, so `2*3` isn't emphasis while it may be arithmetic.
      const canOpen = after !== undefined && !/\s/.test(after) && !word(before);
      const top = stack.findLastIndex((k) => k === kinds[0] || k === kinds.at(-1));
      if (canClose && top >= 0) stack.length = kinds.length === 2 && top > 0 && kinds.includes(stack[top - 1]) ? top - 1 : top;
      else if (canOpen) stack.push(...kinds);
    }
    // What's still open is closed, innermost first.
    return last.slice(0, end).trimEnd() + stack.reverse().join("");
  };

  if (typeof window !== "undefined") window.__irenderMarkdown = { render, complete };
  if (typeof module !== "undefined") module.exports = { render, complete };
})();
