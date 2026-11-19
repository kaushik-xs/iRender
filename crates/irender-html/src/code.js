// irender-html: code, for the kits that show it (`window.__irenderCode`): the UI kit's
// `ui-code-block` and `ui-json-editor`, and the Markdown kit's code fences. Included
// once a page, by whichever needs it first, under the key `irender-code`.
//
// The highlighter is a tokenizer, not a parser: comments, strings, numbers, keywords,
// keys, tags and calls, by one regular expression per language. It returns the code as
// lines of HTML, a token that spans lines closed and reopened at each line's end, so
// each line can be its own element (for line numbers and marked lines).
(() => {
  if (window.__irenderCode) return;
  // Interface text in the document's words (its `labels`), by the English; the plain
  // English where the runtime isn't on the page.
  const L = (text, vars = {}) =>
    window.__irender?.label ? window.__irender.label(text, vars)
    : String(Array.isArray(text) ? text[vars.n === 1 ? 0 : 1] : text).replace(/\{(\w+)\}/g, (m, k) => (k in vars ? String(vars[k]) : m));
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
    const show = (c) => (c === undefined ? L("the end") : c === "\n" ? L("a new line") : `'${c}'`);
    const value = () => {
      ws();
      const c = text[i];
      if (c === "{") return object();
      if (c === "[") return array();
      if (c === '"') return string();
      if (c === "-" || (c >= "0" && c <= "9")) return number();
      for (const w of ["true", "false", "null"]) if (text.startsWith(w, i)) return void (i += w.length);
      if (c === undefined) fail(L("Expected a value, found the end"));
      if (c === "'") fail(L("Strings take double quotes"));
      const word = /^[A-Za-z_$][\w$]*/.exec(text.slice(i));
      if (word) fail(L("Unknown value '{word}': text takes double quotes", { word: word[0] }));
      fail(L("Unexpected {what}", { what: show(c) }));
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
            if (!/^[\da-fA-F]{4}$/.test(text.slice(i + 2, i + 6))) fail(L("Bad \\u escape: it takes four hex digits"));
            i += 6;
            continue;
          }
          if (!'"\\/bfnrt'.includes(e) || e === undefined) fail(L("Bad escape \\{char}", { char: e ?? "" }));
          i += 2;
          continue;
        }
        if (c < " ") fail(L("Control character in a string"));
        i++;
      }
      fail(L("Unterminated string"), start);
    };
    const number = () => {
      const m = /^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?/.exec(text.slice(i));
      if (!m) fail(L("Invalid number"));
      i += m[0].length;
      if (/[\d.eE]/.test(text[i] ?? "")) fail(L("Invalid number"));
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
          if (text[i] === close) fail(L("Trailing comma before '{close}'", { close }), comma);
          continue;
        }
        if (text[i] === close) return void i++;
        fail(L("Expected ',' or '{close}', found {what}", { close, what: show(text[i]) }));
      }
    };
    const object = () =>
      list("}", () => {
        ws();
        if (text[i] !== '"') fail(text[i] === "'" ? L("Keys take double quotes") : /[\w$]/.test(text[i] ?? "") ? L("Keys need double quotes") : L("Expected a key, found {what}", { what: show(text[i]) }));
        string();
        ws();
        if (text[i] !== ":") fail(L("Expected ':' after the key, found {what}", { what: show(text[i]) }));
        i++;
        value();
      });
    const array = () => list("]", value);
    try {
      ws();
      if (i === text.length) fail(L("Enter a JSON value"));
      value();
      ws();
      if (i < text.length) fail(L("Unexpected {what} after the value", { what: show(text[i]) }));
      return null;
    } catch (e) {
      if (!("at" in e)) throw e;
      const before = text.slice(0, e.at).split("\n");
      return { message: e.msg, line: before.length, column: before[before.length - 1].length + 1, at: e.at };
    }
  }


  // ---- code fences in rendered Markdown (`<pre><code class="language-x">`): highlighted,
  // with a copy button over each. Each fence is done once.
  const COPY_ICON =
    '<svg data-idle width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round" aria-hidden="true"><rect x="5.5" y="5.5" width="8" height="8" rx="1.5"/><path d="M10.5 3.5v-.5a1 1 0 0 0-1-1h-6a1 1 0 0 0-1 1v6a1 1 0 0 0 1 1h.5"/></svg>' +
    '<svg data-done width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3.5 8.5 6.5 11.5 12.5 4.5"/></svg>';
  function enhance(root) {
    for (const code of root.querySelectorAll("pre > code")) {
      const pre = code.parentElement;
      if (pre.parentElement?.hasAttribute("data-fence")) continue;
      const text = code.textContent;
      const lang = (code.className.match(/language-([\w+\-.#]+)/) || [])[1]?.toLowerCase() || "";
      if (lang) code.innerHTML = lines(text, lang).join("\n");
      // A box around the pre, so the button stays put while the code scrolls.
      const box = document.createElement("div");
      box.setAttribute("data-fence", lang);
      pre.replaceWith(box);
      box.append(pre);
      const btn = document.createElement("button");
      btn.type = "button";
      btn.setAttribute("data-fence-copy", "");
      btn.setAttribute("aria-label", lang ? L("Copy {lang} code", { lang }) : L("Copy code"));
      btn.title = L("Copy");
      btn.innerHTML = COPY_ICON;
      const live = document.createElement("span");
      live.setAttribute("aria-live", "polite");
      live.setAttribute("data-fence-live", "");
      box.append(btn, live);
    }
    if (root.__fences) return;
    root.__fences = true;
    // One listener for every fence in `root`, now and after it's redrawn.
    let timer;
    root.addEventListener("click", async (e) => {
      const btn = e.target.closest?.("[data-fence-copy]");
      if (!btn || !root.contains(btn)) return;
      const ok = await copy(btn.parentElement.querySelector("code").textContent);
      const live = btn.parentElement.querySelector("[data-fence-live]");
      btn.toggleAttribute("data-copied", ok);
      live.textContent = ok ? L("Copied to the clipboard") : L("Couldn't copy: select the code and copy it");
      clearTimeout(timer);
      timer = setTimeout(() => {
        btn.removeAttribute("data-copied");
        live.textContent = "";
      }, 2000);
    });
  }

  window.__irenderCode = { lines, copy, check, enhance };
})();
