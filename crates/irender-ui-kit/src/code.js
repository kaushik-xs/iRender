// irender-ui-kit: the behaviours of `ui-code-block` and `ui-json-editor`. The
// highlighter, copying and the JSON check are irender-html's `window.__irenderCode`
// (code.js there), on the page before these.

// ---- `ui-code-block`: highlights its code, keeps it in step with state, and copies it.
(window.__irenderBehaviors ||= {})["ui-code-block"] = (el, api) => {
  const cfg = api.config;
  const code = el.querySelector("[data-code-body]");
  const btn = el.querySelector("[data-code-copy]");
  const live = el.querySelector("[data-code-live]");
  // Interface text in the document's words (its `labels`), by the English.
  const L = (text, vars) => window.__irender.label(text, vars);
  const env = () => {
    const s = api.scope();
    return s ? { ...api.state, ...s } : api.state;
  };
  const isLive = (v) => v && typeof v === "object" && !Array.isArray(v) && "$irender" in v && Object.keys(v).length === 1;
  const source = () => {
    const v = isLive(cfg.code) ? window.__irender.evaluate(cfg.code.$irender, env()) : cfg.code;
    return typeof v === "string" ? v : v === undefined || v === null ? "" : JSON.stringify(v, null, 2);
  };
  // Marked lines: numbers and ranges ("3-5").
  const marked = new Set();
  for (const m of cfg.highlight || []) {
    const [a, b = a] = String(m).split("-").map(Number);
    for (let n = a; n <= b && n - a < 10000; n++) marked.add(n);
  }
  let shown = null;
  function draw() {
    const text = source().replace(/\n$/, "");
    if (text === shown) return;
    shown = text;
    code.innerHTML = window.__irenderCode
      .lines(text, cfg.language)
      .map((l, i) => `<span data-ln${marked.has(i + 1) ? " data-hl" : ""}>${l}</span>`)
      .join("");
  }
  let timer;
  btn?.addEventListener("click", async () => {
    const ok = await window.__irenderCode.copy(shown ?? source());
    btn.toggleAttribute("data-copied", ok);
    btn.setAttribute("aria-label", ok ? L("Copied") : L("Copy code"));
    live.textContent = ok ? L("Copied to the clipboard") : L("Couldn't copy: select the code and copy it");
    api.emit("copy", { ok });
    clearTimeout(timer);
    timer = setTimeout(() => {
      btn.removeAttribute("data-copied");
      btn.setAttribute("aria-label", L("Copy code"));
      live.textContent = "";
    }, 2000);
  });
  draw();
  return { update: draw };
};

// ---- `ui-json-editor`: a text area over its highlighted copy, with line numbers, a
// check as you type, the error's line marked, and Format. The value goes to `bind` only
// while the text is valid JSON (of the `type` asked for).
(window.__irenderBehaviors ||= {})["ui-json-editor"] = (el, api) => {
  const cfg = api.config;
  const ta = el.querySelector("textarea");
  const hl = el.querySelector("[data-je-code]");
  const gutter = el.querySelector("[data-je-gutter]");
  const err = el.querySelector("[data-je-error]");
  const status = el.querySelector("[data-je-status]");
  const fmt = el.querySelector("[data-je-format]");
  const L = (text, vars) => window.__irender.label(text, vars);
  const pad = " ".repeat(cfg.indent);
  const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);
  const kind = (v) => (v === null ? "null" : Array.isArray(v) ? "array" : typeof v);
  let written = api.state[cfg.bind];
  let problem = null;

  function judge(text) {
    const e = window.__irenderCode.check(text);
    if (e) return { error: e };
    const value = JSON.parse(text);
    if (cfg.type && cfg.type !== "any" && kind(value) !== cfg.type) {
      // The type names stay English JSON words; the sentence around them is a label.
      return { error: { message: L("Expected {expected}, found {found}", { expected: cfg.type === "array" ? "an array" : `an ${cfg.type}`, found: kind(value) === "array" || kind(value) === "object" ? "an " + kind(value) : kind(value) }), line: 1, column: 1, at: 0 } };
    }
    return { value };
  }
  function paint() {
    const text = ta.value;
    const lines = window.__irenderCode.lines(text, "json");
    const bad = problem?.line;
    hl.innerHTML = lines.map((l, i) => `<span data-ln${i + 1 === bad ? " data-err" : ""}>${l || " "}</span>`).join("");
    gutter.innerHTML = lines.map((_, i) => `<span${i + 1 === bad ? " data-err" : ""}>${i + 1}</span>`).join("");
    scroll();
  }
  function scroll() {
    hl.style.transform = `translate(${-ta.scrollLeft}px, ${-ta.scrollTop}px)`;
    gutter.style.transform = `translateY(${-ta.scrollTop}px)`;
  }
  // Shows the problem (or none): the message, the line marked, `aria-invalid`.
  function report(p) {
    problem = p;
    el.toggleAttribute("data-invalid", !!p);
    ta.setAttribute("aria-invalid", p ? "true" : "false");
    err.textContent = p ? L("Line {line}, column {column}: {message}", { line: String(p.line), column: String(p.column), message: p.message }) : "";
    status.textContent = p ? L("Invalid JSON") : L("Valid JSON");
    paint();
  }
  let quiet;
  function changed({ now = false } = {}) {
    const r = judge(ta.value);
    if (cfg.bindValid) api.set(cfg.bindValid, !r.error);
    if (!r.error) {
      written = r.value;
      api.set(cfg.bind, r.value);
      api.emit("change", { value: r.value });
    }
    clearTimeout(quiet);
    // An error waits for a pause in the typing; a fix shows at once.
    if (!r.error || now) report(r.error || null);
    else {
      paint();
      quiet = setTimeout(() => report(r.error), 600);
    }
  }
  ta.addEventListener("input", () => changed());
  ta.addEventListener("blur", () => problem || judge(ta.value).error ? changed({ now: true }) : 0);
  ta.addEventListener("scroll", scroll);

  // Writes text over the selection as typing would, so undo still works.
  const type = (text) => {
    if (!document.execCommand("insertText", false, text)) ta.setRangeText(text, ta.selectionStart, ta.selectionEnd, "end");
  };
  let leaving = false;
  ta.addEventListener("keydown", (e) => {
    if (e.key === "Escape") return void (leaving = true);
    if (e.key === "Tab" && !leaving && !e.ctrlKey && !e.metaKey && !e.altKey) {
      e.preventDefault();
      const v = ta.value, s = ta.selectionStart, start = v.lastIndexOf("\n", s - 1) + 1;
      if (e.shiftKey) {
        // Takes up to one indent off the line.
        const n = v.slice(start, start + pad.length).match(/^ */)[0].length;
        if (n) {
          ta.setSelectionRange(start, start + n);
          type("");
          ta.setSelectionRange(Math.max(start, s - n), Math.max(start, s - n));
        }
      } else type(pad);
      return;
    }
    leaving = false;
    if (e.key === "Enter" && !e.shiftKey && !e.ctrlKey && !e.metaKey) {
      // Keeps the line's indent, one more after an opening bracket, and puts a closing
      // bracket straight after on its own line.
      e.preventDefault();
      const v = ta.value, s = ta.selectionStart, start = v.lastIndexOf("\n", s - 1) + 1;
      const indent = v.slice(start).match(/^ */)[0];
      const prev = v.slice(start, s).trimEnd().slice(-1), next = v[ta.selectionEnd];
      if ("{[".includes(prev) && prev && (next === "}" || next === "]")) {
        type(`\n${indent}${pad}\n${indent}`);
        const at = ta.selectionStart - indent.length - 1;
        ta.setSelectionRange(at, at);
      } else type(`\n${indent}${"{[".includes(prev) && prev ? pad : ""}`);
    }
  });

  fmt?.addEventListener("click", () => {
    const r = judge(ta.value);
    if (r.error) {
      report(r.error);
      ta.focus();
      ta.setSelectionRange(r.error.at, r.error.at);
      return;
    }
    ta.focus();
    ta.select();
    type(JSON.stringify(r.value, null, cfg.indent));
    ta.setSelectionRange(0, 0);
    ta.scrollTop = 0;
    changed({ now: true });
  });

  // Starts checked: state may hold something the text can't be, or nothing at all.
  const first = judge(ta.value);
  if (cfg.bindValid) api.set(cfg.bindValid, !first.error);
  report(ta.value.trim() || cfg.type ? first.error || null : null);
  return {
    update() {
      const now = api.state[cfg.bind];
      if (same(now, written)) return;
      // Changed from outside: show it, unless it's what the text already says.
      written = now;
      const r = judge(ta.value);
      if (!r.error && same(r.value, now)) return;
      ta.value = now === undefined ? "" : JSON.stringify(now, null, cfg.indent);
      const after = judge(ta.value);
      if (cfg.bindValid) api.set(cfg.bindValid, !after.error);
      report(after.error || null);
    },
  };
};
