// irender-ui-kit: code. A small highlighter shared by `ui-code-block` and
// `ui-json-editor` (`window.__irenderCode`), and their behaviours.
//
// The highlighter is a tokenizer, not a parser: comments, strings, numbers, keywords,
// keys, tags and calls, by one regular expression per language. It returns the code as
// lines of HTML, a token that spans lines closed and reopened at each line's end, so
// each line can be its own element (for line numbers and marked lines).
(() => {
  if (window.__irenderCode) return;
  const words = (s) => `\\b(?:${s.trim().split(/\s+/).join("|")})\\b`;
  const STR = `"(?:[^"\\\\\\n]|\\\\.)*"|'(?:[^'\\\\\\n]|\\\\.)*'`;
  const NUM = `\\b(?:0x[\\da-fA-F_]+|\\d[\\d_]*(?:\\.\\d[\\d_]*)?(?:[eE][+-]?\\d+)?)\\b`;
  const LIT = words("true false null undefined None True False nil NaN Infinity");
  const CALL = `\\b[A-Za-z_$][\\w$]*(?=\\s*\\()`;
  const SLASH = `\\/\\/[^\\n]*|\\/\\*[\\s\\S]*?\\*\\/`;
  const HASH = `#[^\\n]*`;
  // Each language: [token type, pattern], tried in order at each place.
  // Types: c comment, s string, n number, k keyword, l literal, p key, t tag, a attribute,
  // f call, v variable.
  const LANGS = {
    json: [["c", SLASH], ["p", `"(?:[^"\\\\\\n]|\\\\.)*"(?=\\s*:)`], ["s", STR], ["n", `-?${NUM}`], ["l", LIT]],
    yaml: [["c", `(?:^|(?<=\\s))#[^\\n]*`], ["p", `(?<=^[ \\t]*(?:- )?)(?:"[^"\\n]*"|'[^'\\n]*'|[\\w.$/-]+)(?=:(?:\\s|$))`], ["s", STR], ["k", `(?<=^|\\s)[|>][-+]?(?=\\s*$)|^---$|[&*][\\w-]+`], ["n", `-?${NUM}`], ["l", words("true false null yes no on off ~")]],
    js: [
      ["c", SLASH], ["s", `${STR}|\`(?:[^\`\\\\]|\\\\.)*\``], ["k", words(`async await break case catch class const continue debugger default delete do else
        export extends finally for from function get if import in instanceof let new of return set static super switch this throw try typeof
        var void while with yield type interface enum implements private public protected readonly as`)],
      ["l", LIT], ["n", NUM], ["f", CALL],
    ],
    rust: [
      ["c", SLASH], ["s", `"(?:[^"\\\\]|\\\\.)*"|'(?:[^'\\\\\\n]|\\\\.)'`], ["k", words(`as async await break const continue crate dyn else enum extern fn for if impl in
        let loop match mod move mut pub ref return self Self static struct super trait type unsafe use where while`)],
      ["l", words("true false None Some Ok Err")], ["n", NUM], ["f", `\\b[A-Za-z_]\\w*!?(?=\\s*\\()|\\b\\w+!`],
    ],
    python: [
      ["c", HASH], ["s", `"""[\\s\\S]*?"""|'''[\\s\\S]*?'''|[rbf]?(?:${STR})`], ["k", words(`and as assert async await break class continue def del elif else except finally
        for from global if import in is lambda nonlocal not or pass raise return try while with yield self`)],
      ["l", LIT], ["n", NUM], ["f", CALL],
    ],
    bash: [
      ["c", `(?:^|(?<=\\s))#[^\\n]*`], ["s", STR], ["v", `\\$\\{[^}\\n]*\\}|\\$[\\w@#?*!$-]`],
      ["k", words("if then else elif fi for while until do done case esac in function return export local readonly set unset exit source")],
      ["f", `(?<=^\\s*|[|;&]\\s*|\\$\\(\\s*)(?:sudo\\s+)?[\\w./-]+`], ["n", `(?<=\\s)-{1,2}[\\w-]+`],
    ],
    html: [["c", `<!--[\\s\\S]*?-->`], ["t", `<\\/?[\\w:-]+|\\/?>`], ["a", `[\\w:@.-]+(?==)`], ["s", STR]],
    css: [
      ["c", `\\/\\*[\\s\\S]*?\\*\\/`], ["s", STR], ["k", `@[\\w-]+|!important`], ["p", `[\\w-]+(?=\\s*:[^;{}]*[;}])`],
      ["n", `-?\\d*\\.?\\d+(?:%|[a-z]+)?\\b|#[\\da-fA-F]{3,8}\\b`], ["f", `[\\w-]+(?=\\()`], ["t", `[.#]?[\\w-]+(?=[^{};]*\\{)`],
    ],
    sql: [
      ["c", `--[^\\n]*|\\/\\*[\\s\\S]*?\\*\\/`], ["s", `'(?:[^']|'')*'`], ["p", `"[^"\\n]*"`],
      ["k", words(`select from where and or not insert into values update set delete create table alter drop join left right inner outer full
        cross on group by order having limit offset as distinct union all is in like ilike between case when then else end primary key
        foreign references index view with returning asc desc default unique check exists`)],
      ["l", words("null true false")], ["n", NUM], ["f", CALL],
    ],
  };
  const ALIAS = { javascript: "js", ts: "js", typescript: "js", jsx: "js", tsx: "js", rs: "rust", py: "python", sh: "bash", shell: "bash",
    zsh: "bash", console: "bash", xml: "html", svg: "html", yml: "yaml", jsonc: "json", postgres: "sql", postgresql: "sql" };
  const compiled = {};
  const rules = (lang) => {
    const name = ALIAS[lang] || lang;
    if (!LANGS[name]) return null;
    // SQL's words in any case; the rest as written.
    return (compiled[name] ||= new RegExp(LANGS[name].map(([, p]) => `(${p})`).join("|"), name === "sql" ? "gmi" : "gm"));
  };
  const esc = (s) => s.replace(/[&<>]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[c]);

  /** The code as lines of HTML, highlighted for `lang` (plain for one it doesn't know). */
  function lines(code, lang) {
    const re = rules(lang), types = re && LANGS[ALIAS[lang] || lang].map(([t]) => t);
    const out = [""];
    const put = (text, type) => {
      text.split("\n").forEach((part, i) => {
        if (i) out.push("");
        if (part) out[out.length - 1] += type ? `<span data-t="${type}">${esc(part)}</span>` : esc(part);
      });
    };
    if (!re) {
      put(code);
      return out;
    }
    re.lastIndex = 0;
    let at = 0, m;
    while ((m = re.exec(code))) {
      if (!m[0]) {
        re.lastIndex++;
        continue;
      }
      put(code.slice(at, m.index));
      put(m[0], types[m.slice(1).findIndex((g) => g !== undefined)]);
      at = m.index + m[0].length;
    }
    put(code.slice(at));
    return out;
  }

  /** Copies text, by the clipboard API or, where that's missing or refused, a selection. */
  async function copy(text) {
    try {
      await navigator.clipboard.writeText(text);
      return true;
    } catch {
      const t = Object.assign(document.createElement("textarea"), { value: text });
      t.setAttribute("readonly", "");
      t.style.cssText = "position:fixed;opacity:0;pointer-events:none";
      document.body.append(t);
      t.select();
      let ok = false;
      try {
        ok = document.execCommand("copy");
      } catch {}
      t.remove();
      return ok;
    }
  }

  // ---- a JSON reader that says where it stopped and why, the same in every browser
  // (`JSON.parse`'s messages differ between them, and Safari's give no place).
  function check(text) {
    let i = 0;
    const fail = (msg, at = i) => {
      throw { at, msg };
    };
    const ws = () => {
      while (i < text.length && " \t\n\r".includes(text[i])) i++;
    };
    const show = (c) => (c === undefined ? "the end" : c === "\n" ? "a new line" : `'${c}'`);
    const value = () => {
      ws();
      const c = text[i];
      if (c === "{") return object();
      if (c === "[") return array();
      if (c === '"') return string();
      if (c === "-" || (c >= "0" && c <= "9")) return number();
      for (const w of ["true", "false", "null"]) if (text.startsWith(w, i)) return void (i += w.length);
      if (c === undefined) fail("Expected a value, found the end");
      if (c === "'") fail("Strings take double quotes");
      const word = /^[A-Za-z_$][\w$]*/.exec(text.slice(i));
      if (word) fail(`Unknown value '${word[0]}': text takes double quotes`);
      fail(`Unexpected ${show(c)}`);
    };
    const string = () => {
      const start = i++;
      while (i < text.length) {
        const c = text[i];
        if (c === '"') return void i++;
        if (c === "\n") break;
        if (c === "\\") {
          const e = text[i + 1];
          if (e === "u") {
            if (!/^[\da-fA-F]{4}$/.test(text.slice(i + 2, i + 6))) fail("Bad \\u escape: it takes four hex digits");
            i += 6;
            continue;
          }
          if (!'"\\/bfnrt'.includes(e) || e === undefined) fail(`Bad escape \\${e ?? ""}`);
          i += 2;
          continue;
        }
        if (c < " ") fail("Control character in a string");
        i++;
      }
      fail("Unterminated string", start);
    };
    const number = () => {
      const m = /^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?/.exec(text.slice(i));
      if (!m) fail("Invalid number");
      i += m[0].length;
      if (/[\d.eE]/.test(text[i] ?? "")) fail("Invalid number");
    };
    const list = (close, item) => {
      i++;
      ws();
      if (text[i] === close) return void i++;
      for (;;) {
        item();
        ws();
        if (text[i] === ",") {
          const comma = i++;
          ws();
          if (text[i] === close) fail(`Trailing comma before '${close}'`, comma);
          continue;
        }
        if (text[i] === close) return void i++;
        fail(`Expected ',' or '${close}', found ${show(text[i])}`);
      }
    };
    const object = () =>
      list("}", () => {
        ws();
        if (text[i] !== '"') fail(text[i] === "'" ? "Keys take double quotes" : /[\w$]/.test(text[i] ?? "") ? "Keys need double quotes" : `Expected a key, found ${show(text[i])}`);
        string();
        ws();
        if (text[i] !== ":") fail(`Expected ':' after the key, found ${show(text[i])}`);
        i++;
        value();
      });
    const array = () => list("]", value);
    try {
      ws();
      if (i === text.length) fail("Enter a JSON value");
      value();
      ws();
      if (i < text.length) fail(`Unexpected ${show(text[i])} after the value`);
      return null;
    } catch (e) {
      if (!("at" in e)) throw e;
      const before = text.slice(0, e.at).split("\n");
      return { message: e.msg, line: before.length, column: before[before.length - 1].length + 1, at: e.at };
    }
  }

  window.__irenderCode = { lines, copy, check };
})();

// ---- `ui-code-block`: highlights its code, keeps it in step with state, and copies it.
(window.__irenderBehaviors ||= {})["ui-code-block"] = (el, api) => {
  const cfg = api.config;
  const code = el.querySelector("[data-code-body]");
  const btn = el.querySelector("[data-code-copy]");
  const live = el.querySelector("[data-code-live]");
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
    btn.setAttribute("aria-label", ok ? "Copied" : "Copy code");
    live.textContent = ok ? "Copied to the clipboard" : "Couldn't copy: select the code and copy it";
    api.emit("copy", { ok });
    clearTimeout(timer);
    timer = setTimeout(() => {
      btn.removeAttribute("data-copied");
      btn.setAttribute("aria-label", "Copy code");
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
      return { error: { message: `Expected ${cfg.type === "array" ? "an array" : `an ${cfg.type}`}, found ${kind(value) === "array" || kind(value) === "object" ? "an " + kind(value) : kind(value)}`, line: 1, column: 1, at: 0 } };
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
    err.textContent = p ? `Line ${p.line}, column ${p.column}: ${p.message}` : "";
    status.textContent = p ? "Invalid JSON" : "Valid JSON";
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
