// irender-ui-kit: `ui-diff`, `ui-log` and `ui-code-editor`.
(() => {
  const B = (window.__irenderBehaviors ||= {});
  const K = window.__irenderKit, L = (t, v) => window.__irender.label(t, v);
  const { make, button, icon, isObj, blank } = K;

  // ---------------------------------------------------------------- diff
  // The common parts of two lists, by Myers' shortest edit: `[[kind, i, j]]`, kind " ", "-" or "+".
  function edits(a, b) {
    let s = 0, e = 0;
    while (s < a.length && s < b.length && a[s] === b[s]) s++;
    while (e < a.length - s && e < b.length - s && a[a.length - 1 - e] === b[b.length - 1 - e]) e++;
    const A = a.slice(s, a.length - e), Bx = b.slice(s, b.length - e), n = A.length, m = Bx.length, out = [];
    for (let i = 0; i < s; i++) out.push([" ", i, i]);
    const max = n + m;
    // Two texts with next to nothing in common: all of one, then all of the other.
    if (max > 20000) {
      for (let i = 0; i < n; i++) out.push(["-", s + i, null]);
      for (let j = 0; j < m; j++) out.push(["+", null, s + j]);
    } else if (max) {
      const v = new Map([[1, 0]]), trace = [];
      let d = 0;
      done: for (; d <= max; d++) {
        trace.push(new Map(v));
        for (let k = -d; k <= d; k += 2) {
          let x = k === -d || (k !== d && (v.get(k - 1) ?? -1) < (v.get(k + 1) ?? -1)) ? v.get(k + 1) ?? 0 : (v.get(k - 1) ?? 0) + 1;
          let y = x - k;
          while (x < n && y < m && A[x] === Bx[y]) x++, y++;
          v.set(k, x);
          if (x >= n && y >= m) break done;
        }
        if (d > 3000) break;
      }
      const mid = [];
      let x = n, y = m;
      if (d > 3000) {
        for (let i = 0; i < n; i++) out.push(["-", s + i, null]);
        for (let j = 0; j < m; j++) out.push(["+", null, s + j]);
      } else {
        for (; d >= 0 && (x > 0 || y > 0); d--) {
          const t = trace[d], k = x - y, pk = k === -d || (k !== d && (t.get(k - 1) ?? -1) < (t.get(k + 1) ?? -1)) ? k + 1 : k - 1, px = t.get(pk) ?? 0, py = px - pk;
          while (x > px && y > py && x > 0 && y > 0) mid.push([" ", s + --x, s + --y]);
          if (d > 0) x === px ? mid.push(["+", null, s + --y]) : mid.push(["-", s + --x, null]);
        }
        out.push(...mid.reverse());
      }
    }
    for (let i = e; i > 0; i--) out.push([" ", a.length - i, b.length - i]);
    return out;
  }
  window.__irenderDiff = edits;
  const asText = (v) => (typeof v === "string" ? v : v === null || v === undefined ? "" : JSON.stringify(v, null, 2));
  const words = (s) => s.match(/\p{L}[\p{L}\p{N}_]*|\p{N}+|\s+|[^\s\p{L}\p{N}]/gu) ?? [];
  // A line's words, those not in the other line marked.
  function marked(a, b, tag) {
    const x = words(a), y = words(b), out = document.createDocumentFragment(), es = edits(x, y).filter((e) => e[0] !== "+");
    // Mostly different: the whole line is the change.
    const same = es.filter((e) => e[0] === " ").reduce((n, e) => n + x[e[1]].trim().length, 0);
    if (same < a.replace(/\s/g, "").length * 0.3) return a;
    let run = "";
    const flush = () => (run && out.append(make(tag, null, run)), (run = ""));
    for (const e of es) {
      if (e[0] === "-") run += x[e[1]];
      else flush(), out.append(x[e[1]]);
    }
    flush();
    return out;
  }
  B["ui-diff"] = (el, api) => {
    const cfg = api.config, ctx = cfg.context ?? 3;
    let was = null, open = new Set(), local = cfg.view ?? "split";
    const view = () => (cfg.bindView ? api.state[cfg.bindView] ?? local : local);
    const narrow = matchMedia("(max-width: 640px)");
    function draw(force) {
      const before = K.value(api, cfg.before), after = K.value(api, cfg.after), v = narrow.matches ? "unified" : view();
      const sig = JSON.stringify([before, after, v, K.value(api, cfg.title), K.value(api, cfg.labels)]);
      if (!force && sig === was) return;
      if (was && JSON.parse(was).slice(0, 2).join() !== [before, after].join()) open = new Set();
      was = sig;
      const names = K.value(api, cfg.labels) ?? [];
      const title = K.value(api, cfg.title);
      el.toggleAttribute("data-wrap", !!cfg.wrap);
      el.setAttribute("data-view", v);
      if (cfg.mode === "fields") return fields(before ?? {}, after ?? {}, names, title);
      const a = asText(before).replace(/\n$/, "").split("\n"), b = asText(after).replace(/\n$/, "").split("\n");
      if (asText(before) === "") a.length = 0;
      if (asText(after) === "") b.length = 0;
      const es = edits(a, b), adds = es.filter((e) => e[0] === "+").length, dels = es.filter((e) => e[0] === "-").length;
      const code = (lines) => (cfg.language && window.__irenderCode ? window.__irenderCode.lines(lines.join("\n"), cfg.language) : null);
      const ha = code(a), hb = code(b);
      const cell = (lines, html, i, other, tag) => {
        const td = make("td", { "data-line": "" });
        // Our own highlighter's HTML, from escaped text.
        if (html && (other === undefined || other === null)) td.innerHTML = html[i] ?? "";
        else td.append(other === undefined || other === null ? lines[i] : marked(lines[i], other, tag));
        return td;
      };
      // Rows: unchanged lines, and runs of removed then added ones beside one another.
      const rows = [];
      for (let i = 0; i < es.length; ) {
        if (es[i][0] === " ") rows.push({ k: " ", a: es[i][1], b: es[i++][2] });
        else {
          const d = [], p = [];
          while (i < es.length && es[i][0] !== " ") es[i][0] === "-" ? d.push(es[i++][1]) : p.push(es[i++][2]);
          for (let n = 0; n < Math.max(d.length, p.length); n++) rows.push({ k: "±", a: d[n] ?? null, b: p[n] ?? null });
        }
      }
      const near = rows.map((r) => r.k !== " ");
      const keep = rows.map((_, i) => { for (let j = Math.max(0, i - ctx); j <= Math.min(rows.length - 1, i + ctx); j++) if (near[j]) return true; return false; });
      const table = make("table", { "aria-label": L("Changes") }), body = make("tbody");
      const no = (n) => make("td", { "data-no": "" }, n === null ? "" : n + 1);
      const sign = (s) => make("span", { "data-sign": "", "aria-label": s === "+" ? L("added") : s === "-" ? L("removed") : null }, s);
      for (let i = 0; i < rows.length; ) {
        if (!keep[i] && !open.has(i)) {
          let j = i;
          while (j < rows.length && !keep[j]) j++;
          if (j - i > 1 && (adds || dels)) {
            const at = i;
            body.append(make("tr", { "data-fold": "" }, make("td", { colspan: v === "split" ? 4 : 3 }, button({ click: () => { for (let k = at; k < j; k++) open.add(k); draw(true); } }, L("Show {n} unchanged lines", { n: j - i })))));
            i = j;
            continue;
          }
        }
        const r = rows[i++];
        const pair = r.a !== null && r.b !== null && r.k !== " ";
        if (v === "split") {
          const side = (lines, html, n, other, tag, k) => (n === null ? [make("td", { "data-no": "", "data-k": "none" }), make("td", { "data-line": "", "data-k": "none" })] : [Object.assign(no(n)), cell(lines, html, n, other, tag)].map((c) => (k && c.setAttribute("data-k", k), c)));
          body.append(make("tr", null, side(a, ha, r.a, pair ? b[r.b] : null, "del", r.k === " " ? null : "del"), side(b, hb, r.b, pair ? a[r.a] : null, "ins", r.k === " " ? null : "add")));
        } else if (r.k === " ") {
          const c = cell(b, hb, r.b);
          c.prepend(sign(" "));
          body.append(make("tr", null, no(r.a), no(r.b), c));
        } else {
          // Removed lines together, then added ones.
          const run = [r];
          while (i < rows.length && rows[i].k !== " ") run.push(rows[i++]);
          for (const x of run) if (x.a !== null) { const c = cell(a, ha, x.a, x.b !== null ? b[x.b] : null, "del"); c.prepend(sign("-")); body.append(make("tr", { "data-k": "del" }, no(x.a), no(null), c)); }
          for (const x of run) if (x.b !== null) { const c = cell(b, hb, x.b, x.a !== null ? a[x.a] : null, "ins"); c.prepend(sign("+")); body.append(make("tr", { "data-k": "add" }, no(null), no(x.b), c)); }
        }
      }
      table.append(body);
      const pick = (x, t) => button({ "aria-pressed": String(view() === x), click: () => ((local = x), cfg.bindView ? api.set(cfg.bindView, x) : draw(true)) }, t);
      const bar = make("div", { "data-d-bar": "" }, title ? make("span", { "data-d-title": "" }, title) : null, names.length ? make("span", null, names.join(" → ")) : null, make("span", { "data-added": "" }, `+${adds}`), make("span", { "data-removed": "" }, `−${dels}`), make("span", { "data-push": "" }), narrow.matches ? null : [pick("split", L("Split")), pick("unified", L("Unified"))]);
      el.replaceChildren(bar, adds || dels ? make("div", { "data-d-scroll": "", tabindex: 0, role: "region", "aria-label": L("Changes") }, table) : make("p", { "data-none": "" }, L("No changes")));
    }
    function fields(x, y, names, title) {
      const labels = K.value(api, cfg.fieldLabels) ?? {}, keys = [...new Set([...Object.keys(labels), ...Object.keys(x), ...Object.keys(y)])];
      const same = (k) => JSON.stringify(x[k] ?? null) === JSON.stringify(y[k] ?? null);
      const list = keys.filter((k) => (k in x || k in y) && (cfg.unchanged || !same(k)));
      const text = (v) => (blank(v) ? "" : Array.isArray(v) && v.every((i) => !isObj(i)) ? v.join(", ") : typeof v === "boolean" ? (v ? L("Yes") : L("No")) : asText(v));
      const rows = list.map((k) => {
        const a = text(x[k]), b = text(y[k]), eq = same(k);
        const del = a && !eq ? (b ? marked(a, b, "del") : make("del", null, a)) : a || "—", ins = b && !eq ? (a ? marked(b, a, "ins") : make("ins", null, b)) : b || "—";
        return make("tr", null, make("th", { scope: "row" }, labels[k] ?? k), make("td", { "data-k": eq || !a ? null : "del" }, del), make("td", { "data-k": eq || !b ? null : "add" }, ins));
      });
      const n = keys.filter((k) => !same(k)).length;
      el.replaceChildren(
        ...[title ? make("div", { "data-d-bar": "" }, make("span", { "data-d-title": "" }, title), make("span", { "data-push": "" }, L(["{n} change", "{n} changes"], { n }))) : null,
          rows.length ? make("div", { "data-d-scroll": "", tabindex: 0, role: "region", "aria-label": L("Changes") }, make("table", { "data-fields": "" }, make("thead", null, make("tr", null, make("th", { scope: "col" }, L("Field")), make("th", { scope: "col" }, names[0] ?? L("Before")), make("th", { scope: "col" }, names[1] ?? L("After")))), make("tbody", null, rows))) : make("p", { "data-none": "" }, L("No changes"))].filter(Boolean));
    }
    narrow.addEventListener("change", () => el.isConnected && draw(true));
    draw();
    return { update: () => draw() };
  };

  // ---------------------------------------------------------------- log
  const LEVELS = { trace: "debug", debug: "debug", verbose: "debug", info: "info", notice: "info", success: "success", ok: "success", warn: "warn", warning: "warn", error: "error", err: "error", fatal: "error", critical: "error", severe: "error" };
  const ANSI = { 30: "#555", 31: "#dc2626", 32: "#16a34a", 33: "#b45309", 34: "#2563eb", 35: "#9333ea", 36: "#0891b2", 37: "#71717a", 90: "#71717a", 91: "#ef4444", 92: "#22c55e", 93: "#d97706", 94: "#3b82f6", 95: "#a855f7", 96: "#06b6d4", 97: "#a1a1aa" };
  // A line's text, its terminal colours as spans.
  function ansi(text) {
    if (!text.includes("\u001b[")) return [text];
    const out = [];
    let style = {};
    text.split(/(\u001b\[[\d;]*[A-Za-z])/).forEach((p, i) => {
      if (i % 2) {
        if (!p.endsWith("m")) return;
        for (const c of p.slice(2, -1).split(";").map(Number)) {
          if (c === 0) style = {};
          else if (c === 1) style.fontWeight = "600";
          else if (c === 3) style.fontStyle = "italic";
          else if (c === 4) style.textDecoration = "underline";
          else if (c === 39) delete style.color;
          else if (ANSI[c]) style.color = `color-mix(in srgb, ${ANSI[c]} 80%, currentColor)`;
        }
      } else if (p) {
        const s = make("span", null, p);
        Object.assign(s.style, style);
        out.push(s);
      }
    });
    return out;
  }
  B["ui-log"] = (el, api) => {
    const cfg = api.config, loc = cfg.locale || undefined, max = cfg.maxLines ?? 2000;
    let hide = new Set(), q = "", wrap = cfg.wrap, follow = cfg.follow, was = null, shown = [], current = null;
    const box = make("div", { "data-log-lines": "", "data-d-scroll": "", tabindex: 0, role: "log", "aria-live": "off" });
    const search = cfg.search ? make("input", { type: "search", placeholder: L("Search…"), "aria-label": L("Search") }) : null;
    const count = make("span"), chips = make("span", { style: "display:inline-flex;gap:2px" });
    const wrapB = button({ "aria-pressed": String(wrap), click: () => ((wrap = !wrap), wrapB.setAttribute("aria-pressed", String(wrap)), el.toggleAttribute("data-wrap", wrap)) }, L("Wrap"));
    const latest = button({ "data-log-latest": "", hidden: true, click: () => ((follow = true), toEnd()) }, icon("down", 13), L("Latest"));
    const title = make("span", { "data-d-title": "" });
    const bar = make("div", { "data-d-bar": "" }, title, search, chips, make("span", { "data-push": "" }), count, wrapB, cfg.download ? button({ "aria-label": L("Download"), title: L("Download"), click: save }, icon("download", 14)) : null);
    el.append(bar, make("div", { "data-log-wrap": "" }, box, latest));
    el.toggleAttribute("data-wrap", wrap);
    const line = (x) => {
      const l = isObj(x) ? x : { text: String(x) };
      const lv = LEVELS[String(l.level ?? "").toLowerCase()] ?? (isObj(x) ? "" : LEVELS[/\b(trace|debug|info|warn(?:ing)?|error|fatal)\b/i.exec(l.text)?.[1].toLowerCase()] ?? "");
      return { raw: x, time: l.time ?? l.timestamp ?? null, level: l.level ?? "", lv, source: l.source ?? "", text: typeof l.text === "string" ? l.text : l.message !== undefined ? asText(l.message) : asText(l.text ?? "") };
    };
    function time(t) {
      if (blank(t) || cfg.time === "none") return null;
      const d = new Date(t);
      if (isNaN(d)) return String(t);
      if (cfg.time === "relative") return K.ago(t, cfg.locale);
      return cfg.time === "datetime" ? d.toLocaleString(loc, { dateStyle: "short", timeStyle: "medium" }) : d.toLocaleTimeString(loc, { hour: "2-digit", minute: "2-digit", second: "2-digit", hour12: false });
    }
    function mark(nodes) {
      if (!q) return nodes;
      const f = q.toLowerCase();
      const walk = (n) => {
        if (typeof n === "string") {
          const out = [];
          let i = 0, at;
          while ((at = n.toLowerCase().indexOf(f, i)) >= 0) out.push(n.slice(i, at), make("mark", null, n.slice(at, at + f.length))), (i = at + f.length);
          out.push(n.slice(i));
          return out;
        }
        const s = n.cloneNode(false);
        s.append(...walk(n.textContent));
        return [s];
      };
      return nodes.flatMap(walk);
    }
    const toEnd = () => (box.scrollTop = box.scrollHeight);
    function draw(force) {
      const all = K.value(api, cfg.lines) ?? [], list = Array.isArray(all) ? all : [];
      title.textContent = K.value(api, cfg.title) ?? "";
      title.hidden = !title.textContent;
      const sig = JSON.stringify([[...hide], q]);
      if (!force && was && was.list === list && was.sig === sig) return;
      const grew = was && was.sig === sig && !force && list.length >= was.list.length && was.list.length > 0 && list[was.list.length - 1] === was.list[was.list.length - 1] && list[0] === was.list[0] && list.length <= max;
      const from = grew ? was.list.length : Math.max(0, list.length - max);
      const fresh = list.slice(from).map(line);
      const pass = (l) => !hide.has(l.lv) && (!q || `${l.text} ${l.source}`.toLowerCase().includes(q.toLowerCase()));
      const rows = fresh.filter(pass);
      const nodes = rows.map((l) => {
        const t = time(l.time);
        const n = make("div", { "data-log-line": "", "data-lv": l.lv || null, "data-click": !!cfg.bindLine }, t ? make("time", { datetime: String(l.time) }, t) : null, l.level ? make("span", { "data-log-level": "" }, String(l.level)) : null, l.source ? make("span", { "data-log-source": "", title: l.source }, l.source) : null, make("span", { "data-log-text": "" }, mark(ansi(l.text))));
        n.__line = l.raw;
        return n;
      });
      was = { list, sig };
      if (grew) {
        box.querySelector("[data-none]")?.remove();
        box.append(...nodes);
        shown.push(...rows);
      } else {
        box.replaceChildren(...nodes);
        shown = rows;
      }
      if (!shown.length) box.replaceChildren(make("div", { "data-none": "" }, list.length ? L("No lines match.") : K.value(api, cfg.emptyText) ?? L("Nothing yet.")));
      const t = shown.length === list.length ? L(["{n} line", "{n} lines"], { n: list.length }) : L("{shown} of {total} lines", { shown: shown.length, total: list.length });
      if (count.textContent !== t) count.textContent = t;
      // The levels, as buttons that show and hide them.
      const found = K.value(api, cfg.levels) ?? ["debug", "info", "success", "warn", "error"].filter((x) => list.slice(-max).some((y) => line(y).lv === x));
      const csig = JSON.stringify([found, [...hide]]);
      if (chips.__sig !== csig) {
        chips.__sig = csig;
        // Labels: "Debug", "Info", "Success", "Warn", "Error"
        const NAMES = { debug: "Debug", info: "Info", success: "Success", warn: "Warn", error: "Error" };
        chips.replaceChildren(...(found.length > 1 ? found.map((x) => { const k = LEVELS[String(x).toLowerCase()] ?? x; return button({ "aria-pressed": String(!hide.has(k)), "data-lv": k, click: () => (hide.has(k) ? hide.delete(k) : hide.add(k), draw(true)) }, NAMES[k] ? L(NAMES[k]) : String(x)); }) : []));
      }
      if (follow) toEnd();
      else latest.hidden = false;
    }
    function save() {
      const text = (K.value(api, cfg.lines) ?? []).map(line).map((l) => [l.time, l.level && String(l.level).toUpperCase(), l.source, l.text.replace(/\u001b\[[\d;]*[A-Za-z]/g, "")].filter(Boolean).join(" ")).join("\n");
      window.__irenderSheets.save(new Blob([text + "\n"], { type: "text/plain" }), cfg.download);
    }
    box.addEventListener("scroll", () => {
      const end = box.scrollHeight - box.scrollTop - box.clientHeight < 24;
      if (cfg.follow) follow = end;
      latest.hidden = end;
    }, { passive: true });
    search?.addEventListener("input", () => ((q = search.value.trim()), draw(true)));
    box.addEventListener("click", (e) => {
      const n = e.target.closest("[data-log-line]");
      if (!n || !cfg.bindLine || String(getSelection())) return;
      current?.removeAttribute("aria-current");
      (current = n).setAttribute("aria-current", "true");
      api.set(cfg.bindLine, n.__line);
      api.emit("line", { line: n.__line });
    });
    draw();
    return { update: () => draw() };
  };

  // ---------------------------------------------------------------- code editor
  const COMMENT = { js: "//", javascript: "//", ts: "//", typescript: "//", jsx: "//", tsx: "//", rust: "//", rs: "//", json: "//", jsonc: "//", css: null, python: "#", py: "#", yaml: "#", yml: "#", bash: "#", sh: "#", shell: "#", sql: "--", postgres: "--" };
  const PAIRS = { "(": ")", "[": "]", "{": "}", '"': '"', "'": "'", "`": "`" };
  B["ui-code-editor"] = (el, api) => {
    const cfg = api.config, area = el.querySelector("textarea"), pre = el.querySelector("[data-ce-code]"), gutter = el.querySelector("[data-ce-gutter]"), problems = el.querySelector("[data-ce-problems]"), foot = el.querySelector("[data-ce-foot]"), main = el.querySelector("[data-ce-main]");
    const tab = " ".repeat(cfg.indent);
    let shown = null, last, leaving = false, list = null, found = [], pick = 0, lineWas = -1;
    const lang = () => String(K.value(api, cfg.language) ?? "text");
    const locked = () => !!K.value(api, cfg.readOnly);
    const issues = () => (K.value(api, cfg.diagnostics) ?? []).filter(isObj);
    function paint() {
      const text = area.value, ds = issues(), sig = JSON.stringify([text, lang(), ds, gutter?.clientWidth]);
      if (sig !== shown) {
        shown = sig;
        const html = window.__irenderCode.lines(text, lang());
        // Our own highlighter's HTML, from escaped text.
        pre.innerHTML = html.map((l) => `<span data-ln>${l || "​"}</span>`).join("");
        const sev = new Map();
        for (const d of ds) {
          const n = Number(d.line);
          if (sev.get(n) !== "error") sev.set(n, d.severity === "warning" || d.severity === "info" ? d.severity : "error");
        }
        [...pre.children].forEach((n, i) => sev.has(i + 1) && n.setAttribute("data-sev", sev.get(i + 1)));
        if (gutter) gutter.replaceChildren(...html.map((_, i) => make("span", { "data-sev": sev.get(i + 1) ?? null }, i + 1)));
        problems.replaceChildren(...ds.map((d) => make("li", { "data-sev": d.severity ?? "error" }, button({ click: () => goLine(Number(d.line), Number(d.column) || 1) }, `${L("Line {n}", { n: String(d.line) })}: ${d.message ?? ""}`))));
        if (ds.some((d) => (d.severity ?? "error") === "error")) area.setAttribute("aria-invalid", "true");
        else area.removeAttribute("aria-invalid");
        // Wrapped lines are taller: their numbers keep to them.
        if (gutter && el.hasAttribute("data-wrap")) [...pre.children].forEach((n, i) => (gutter.children[i].style.height = n.getBoundingClientRect().height + "px"));
        lineWas = -1;
      }
      cursor();
    }
    function cursor() {
      const before = area.value.slice(0, area.selectionStart), line = before.split("\n").length, column = before.length - before.lastIndexOf("\n");
      if (line !== lineWas) {
        pre.children[lineWas - 1]?.removeAttribute("data-cur");
        if (document.activeElement === area) pre.children[line - 1]?.setAttribute("data-cur", "");
        lineWas = line;
      }
      foot.textContent = `${L("Line {line}, column {column}", { line: String(line), column: String(column) })} · ${lang()}`;
      if (cfg.bindCursor) {
        const c = api.state[cfg.bindCursor];
        if (c?.line !== line || c?.column !== column) api.set(cfg.bindCursor, { line, column });
      }
    }
    function goLine(n, c = 1) {
      const lines = area.value.split("\n"), at = lines.slice(0, n - 1).reduce((s, l) => s + l.length + 1, 0) + Math.min(c - 1, lines[n - 1]?.length ?? 0);
      area.focus();
      area.setSelectionRange(at, at);
      pre.children[n - 1]?.scrollIntoView({ block: "nearest" });
      cursor();
    }
    // Typed through the browser, so its own undo has it.
    function insert(text, from = area.selectionStart, to = area.selectionEnd) {
      area.focus();
      area.setSelectionRange(from, to);
      const ok = text === "" ? document.execCommand("delete") : document.execCommand("insertText", false, text);
      if (!ok) {
        area.setRangeText(text, from, to, "end");
        area.dispatchEvent(new Event("input", { bubbles: true }));
      }
    }
    const lineSpan = () => {
      const v = area.value, a = v.lastIndexOf("\n", area.selectionStart - 1) + 1;
      let b = v.indexOf("\n", Math.max(area.selectionEnd - (area.selectionEnd > area.selectionStart && v[area.selectionEnd - 1] === "\n" ? 1 : 0), a));
      if (b < 0) b = v.length;
      return [a, b];
    };
    function lines(f) {
      const [a, b] = lineSpan(), out = area.value.slice(a, b).split("\n").map(f).join("\n");
      const s = area.selectionStart, one = s === area.selectionEnd, off = area.value.length - s;
      insert(out, a, b);
      if (one) area.setSelectionRange(Math.max(a, area.value.length - off), Math.max(a, area.value.length - off));
      else area.setSelectionRange(a, a + out.length);
    }
    area.addEventListener("input", () => {
      last = area.value;
      api.set(cfg.bind, area.value);
      paint();
      offer(false);
      api.emit("change", { value: area.value });
    });
    area.addEventListener("keydown", (e) => {
      if (e.isComposing) return;
      if (list) {
        if (e.key === "ArrowDown" || e.key === "ArrowUp") return e.preventDefault(), (pick = (pick + (e.key === "ArrowDown" ? 1 : found.length - 1)) % found.length), drawList();
        if (e.key === "Enter" || e.key === "Tab") return e.preventDefault(), choose(pick);
        if (e.key === "Escape") return e.preventDefault(), e.stopPropagation(), close();
      }
      if (e.key === "Escape") return void (leaving = true);
      if (locked()) return;
      const mod = e.metaKey || e.ctrlKey, s = area.selectionStart, t = area.selectionEnd, v = area.value;
      if (e.key === " " && e.ctrlKey) return e.preventDefault(), offer(true);
      if (e.key === "Tab" && !mod && !e.altKey) {
        // Escape, then Tab, leaves.
        if (leaving) return;
        e.preventDefault();
        if (e.shiftKey) lines((l) => l.replace(new RegExp(`^(\\t| {1,${cfg.indent}})`), ""));
        else if (v.slice(s, t).includes("\n")) lines((l) => (l ? tab + l : l));
        else insert(" ".repeat(cfg.indent - ((s - v.lastIndexOf("\n", s - 1) - 1) % cfg.indent)));
        return;
      }
      leaving = false;
      if (mod && e.key === "/") {
        const c = COMMENT[lang()];
        if (!c) return;
        e.preventDefault();
        const [a, b] = lineSpan(), on = v.slice(a, b).split("\n").filter((l) => l.trim()).every((l) => l.trimStart().startsWith(c));
        const esc = c.replace(/[/]/g, "\\/");
        return lines((l) => (on ? l.replace(new RegExp(`^(\\s*)${esc} ?`), "$1") : l.trim() ? l.replace(/^(\s*)/, `$1${c} `) : l));
      }
      if (e.key === "Enter" && !mod) {
        e.preventDefault();
        const line = v.slice(v.lastIndexOf("\n", s - 1) + 1, s), ind = /^\s*/.exec(line)[0], opens = /[([{:]\s*$/.test(line) && (!/:\s*$/.test(line) || ["python", "py", "yaml", "yml"].includes(lang()));
        const shuts = /^[)\]}]/.test(v.slice(t));
        insert(`\n${ind}${opens ? tab : ""}${opens && shuts ? `\n${ind}` : ""}`);
        if (opens && shuts) area.setSelectionRange(area.selectionStart - ind.length - 1, area.selectionStart - ind.length - 1);
        return cursor();
      }
      if (mod || e.altKey) return;
      if (PAIRS[e.key] && (s !== t || !/[\p{L}\p{N}]/u.test(v[t] ?? ""))) {
        // Around what's selected, or closed at once; a quote after a letter is an apostrophe.
        if (s === t && /["'`]/.test(e.key) && (/[\p{L}\p{N}]/u.test(v[s - 1] ?? "") || v[t] === e.key)) {
          if (v[t] === e.key) e.preventDefault(), area.setSelectionRange(s + 1, s + 1);
          return;
        }
        e.preventDefault();
        const inner = v.slice(s, t);
        insert(e.key + inner + PAIRS[e.key]);
        area.setSelectionRange(s + 1, s + 1 + inner.length);
      } else if (/^[)\]}]$/.test(e.key) && s === t && v[t] === e.key) e.preventDefault(), area.setSelectionRange(s + 1, s + 1);
      else if (e.key === "Backspace" && s === t && PAIRS[v[s - 1]] && v[s] === PAIRS[v[s - 1]]) e.preventDefault(), insert("", s - 1, s + 1);
    });
    for (const ev of ["keyup", "click", "focus"]) area.addEventListener(ev, cursor);
    area.addEventListener("blur", () => (close(), pre.children[lineWas - 1]?.removeAttribute("data-cur"), (lineWas = -1)));

    // ---- words offered
    function close() {
      list?.remove();
      list = null;
      area.removeAttribute("aria-activedescendant");
    }
    function offer(asked) {
      const all = (K.value(api, cfg.completions) ?? []).map((c) => (isObj(c) ? { ...c, label: String(c.label ?? c.insert) } : { label: String(c) }));
      const at = area.selectionStart, word = /[\p{L}\p{N}_.$]*$/u.exec(area.value.slice(0, at))[0];
      if (!all.length || area.selectionEnd !== at || (!asked && !word)) return close();
      const q = word.toLowerCase();
      found = all.filter((c) => c.label.toLowerCase().startsWith(q) && c.label !== word).slice(0, 30);
      if (!found.length) return close();
      pick = 0;
      if (!list) {
        list = make("div", { "data-ce-list": "", role: "listbox", id: `ce-${K.id()}`, "aria-label": L("Suggestions") });
        main.append(list);
        list.addEventListener("pointerdown", (e) => (e.preventDefault(), choose([...list.children].indexOf(e.target.closest("[role=option]")))));
      }
      list.__word = word;
      // Under the caret, measured in the highlighted copy.
      const before = area.value.slice(0, at - word.length), ln = before.split("\n").length - 1, col = before.length - before.lastIndexOf("\n") - 1, n = pre.children[ln];
      const r = document.createRange();
      let left = 12;
      if (n) {
        const walk = document.createTreeWalker(n, NodeFilter.SHOW_TEXT);
        let k = col, tx = walk.nextNode();
        while (tx && k > tx.length) (k -= tx.length), (tx = walk.nextNode());
        if (tx) r.setStart(tx, Math.min(k, tx.length)), r.collapse(true), (left = r.getBoundingClientRect().left - main.getBoundingClientRect().left);
        list.style.top = n.offsetTop + n.offsetHeight + 2 + "px";
      }
      list.style.left = left + "px";
      drawList();
    }
    function drawList() {
      list.replaceChildren(...found.map((c, i) => make("div", { role: "option", id: `${list.id}-${i}`, "aria-selected": String(i === pick) }, make("span", null, c.label), c.detail ? make("small", null, c.detail) : null)));
      area.setAttribute("aria-activedescendant", `${list.id}-${pick}`);
      list.children[pick].scrollIntoView({ block: "nearest" });
    }
    function choose(i) {
      const c = found[i], w = list.__word;
      close();
      if (c) insert(String(c.insert ?? c.label), area.selectionStart - w.length, area.selectionStart);
      close();
    }
    function draw() {
      const v = String(api.state[cfg.bind] ?? "");
      if (v !== last && v !== area.value) {
        area.value = v;
        area.scrollTop = 0;
      }
      last = v;
      paint();
    }
    draw();
    return { update: draw };
  };
})();
