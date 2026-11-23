// irender-ui-kit: `ui-form`. Checks the state keys with rules, says what's wrong for the
// fields left so far, stops a submit while anything is (and goes to the first such field),
// and compares the form's keys with how they were to know its unsaved changes. A `remote`
// rule asks the server once the others pass, a while after typing stops; a submit waits for it.
// `ui-error-summary` lists the messages; `ui-save-bar` shows while there are changes.
(() => {
  const L = (t, v) => window.__irender.label(t, v);
  const blank = (v) => v === null || v === undefined || v === "" || (Array.isArray(v) && !v.length);
  const isObj = (v) => v !== null && typeof v === "object" && !Array.isArray(v);
  // The state key a control writes: its own binding, or its component's.
  const keyOf = (n, form) => {
    for (; n && n !== form; n = n.parentElement) {
      const k = n.getAttribute("data-i-bind") ?? (n.hasAttribute("data-i-x") ? n.__xc?.bind : null);
      if (typeof k === "string") return k;
    }
    return null;
  };
  const focusable = "input:not([type=hidden]), select, textarea, button, [tabindex]:not([tabindex='-1']), [contenteditable=true]";
  // Goes to the field that holds `key`.
  function goTo(form, key) {
    for (const n of form.querySelectorAll("[data-i-bind], [data-i-x]")) {
      if ((n.getAttribute("data-i-bind") ?? n.__xc?.bind) !== key) continue;
      const t = n.matches(focusable) ? n : n.querySelector(focusable);
      if (!t) continue;
      t.focus({ preventScroll: true });
      t.scrollIntoView({ block: "center", behavior: matchMedia("(prefers-reduced-motion: reduce)").matches ? "auto" : "smooth" });
      return true;
    }
    return false;
  }

  (window.__irenderBehaviors ||= {})["ui-form"] = (el, api) => {
    const cfg = api.config;
    const env = () => ({ ...api.state, ...(api.scope() ?? {}) });
    const live = (v) => Array.isArray(v) ? v.map(live) : isObj(v) ? ("$irender" in v ? window.__irender.evaluate(v.$irender, env()) : Object.fromEntries(Object.entries(v).map(([k, x]) => [k, live(x)]))) : v;
    const snap = () => Object.fromEntries(cfg.keys.map((k) => [k, api.state[k] ?? null]));
    let base = snap(), tried = false, wasDirty = null, waiting = false;
    const touched = new Set();
    // ---- remote rules: the check under way per key, and answers already had.
    const asking = new Map(), answers = new Map(), checking = new Set();
    const spec = (r) => (typeof r.remote === "string" ? { url: r.remote } : r.remote);
    function ask(key, r, v, at) {
      const s = spec(r), text = String(v), param = encodeURIComponent(s.param ?? "value");
      const url = s.url.includes("{value}") ? s.url.replaceAll("{value}", encodeURIComponent(text)) : `${s.url}${s.url.includes("?") ? "&" : "?"}${param}=${encodeURIComponent(text)}`;
      at.ctl = new AbortController();
      at.timer = null;
      const done = (message) => {
        if (asking.get(key) !== at) return;
        at.done = true;
        at.message = message;
        if (message !== undefined) answers.set(`${key}\u0000${at.sig}`, message);
        draw();
      };
      fetch(url, { headers: { accept: "application/json", ...(s.headers ?? {}) }, signal: at.ctl.signal })
        .then((res) => {
          if (!res.ok) throw new Error(`HTTP ${res.status}`);
          return res.json();
        })
        .then((a) => {
          // true, { valid: true } or { ok: true } is accepted; text is what's wrong.
          const ok = a === true || (isObj(a) && (a.valid ?? a.ok) === true);
          done(ok ? null : (typeof a === "string" && a) || (isObj(a) && a.message) || s.message || L("Not available"));
        })
        .catch((e) => {
          if (e?.name === "AbortError") return;
          // Not known: the server checks it again when it's sent.
          api.emit("failed", { key, error: String(e?.message ?? e) });
          done(undefined);
        });
    }
    // What the server said of the key's value: a message, null when accepted, or undefined
    // while it's being asked (it's asked a while after the value last changed).
    function remote(key, r, v) {
      const sig = JSON.stringify(v);
      let at = asking.get(key);
      if (at?.sig === sig) return at.done ? at.message : undefined;
      if (at) {
        clearTimeout(at.timer);
        at.ctl?.abort();
      }
      at = { sig, done: false };
      asking.set(key, at);
      const had = `${key}\u0000${sig}`;
      if (answers.has(had)) return Object.assign(at, { done: true, message: answers.get(had) }).message;
      at.timer = setTimeout(() => ask(key, r, v, at), waiting ? 0 : Number(spec(r).wait ?? 400));
      return undefined;
    }
    // Asks now what's still waiting for typing to stop.
    function hurry() {
      for (const [key, at] of asking) if (at.timer) {
        clearTimeout(at.timer);
        ask(key, live(cfg.rules[key]), api.state[key], at);
      }
    }

    // What's wrong with each key, whether shown yet or not.
    function check() {
      const out = {};
      checking.clear();
      for (const [key, rule] of Object.entries(cfg.rules)) {
        const r = live(rule), v = api.state[key], s = typeof v === "string" ? v : v === null || v === undefined ? "" : String(v);
        const list = Array.isArray(v);
        const say = (m) => (out[key] ??= r.message && m !== L("Required") ? String(r.message) : m);
        if (blank(v) || (typeof v === "string" && !v.trim()) || (r.required && v === false)) {
          if (r.required) say(L("Required"));
          continue;
        }
        if (r.type === "email" && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(s)) say(L("Enter an email address"));
        if (r.type === "url" && !/^https?:\/\/\S+$/i.test(s)) say(L("Enter a web address"));
        if (r.type === "number" && (s.trim() === "" || isNaN(s))) say(L("Not a number"));
        if (r.type === "date" && isNaN(Date.parse(s))) say(L("Not a date"));
        // Numbers by size, dates (and times) by their text; a list by how many it has.
        const num = list ? v.length : typeof v === "number" || r.type === "number" ? Number(v) : null;
        const below = (m) => num !== null ? num < Number(m) : s < String(m);
        const above = (m) => num !== null ? num > Number(m) : s > String(m);
        if (!blank(r.min) && below(r.min)) say(list ? L("Pick at least {n}", { n: Number(r.min) }) : num === null ? L("Not before {date}", { date: String(r.min) }) : L("At least {n}", { n: Number(r.min) }));
        if (!blank(r.max) && above(r.max)) say(list ? L("Pick at most {n}", { n: Number(r.max) }) : num === null ? L("Not after {date}", { date: String(r.max) }) : L("At most {n}", { n: Number(r.max) }));
        if (!blank(r.minLength) && s.length < r.minLength) say(L("At least {n} characters", { n: Number(r.minLength) }));
        if (!blank(r.maxLength) && s.length > r.maxLength) say(L("At most {n} characters", { n: Number(r.maxLength) }));
        if (r.match && v !== api.state[r.match]) say(L("Doesn't match"));
        if (r.pattern) {
          try {
            if (!new RegExp(`^(?:${r.pattern})$`, "u").test(s)) say(L("Not in the right format"));
          } catch { /* a pattern the browser can't read checks nothing */ }
        }
        if (!out[key] && r.remote && spec(r)?.url) {
          const m = remote(key, r, v);
          if (m === undefined && !asking.get(key).done) checking.add(key);
          else if (m) out[key] = m;
        }
      }
      // A key that's blank or wrong by the other rules isn't asked about.
      for (const [key, at] of asking) if (!checking.has(key) && !at.done) {
        clearTimeout(at.timer);
        at.ctl?.abort();
        asking.delete(key);
      }
      return out;
    }
    const put = (key, v) => key && JSON.stringify(api.state[key] ?? null) !== JSON.stringify(v) && api.set(key, v);
    const isDirty = () => JSON.stringify(snap()) !== JSON.stringify(base);
    const mine = (n) => n.closest("form") === el;
    let drawing = false;
    function draw() {
      if (drawing) return;
      drawing = true;
      try {
        // Set to false by the page: saved.
        if (cfg.dirty && wasDirty === true && api.state[cfg.dirty] === false) saved();
        const all = check(), keys = Object.keys(all);
        el.__errors = all;
        el.__tried = tried;
        put(cfg.errors, Object.fromEntries(Object.entries(all).filter(([k]) => tried || touched.has(k))));
        put(cfg.valid, !keys.length && !checking.size);
        put(cfg.checking, [...checking]);
        el.toggleAttribute("data-form-checking", checking.size > 0);
        el.setAttribute("aria-busy", String(checking.size > 0));
        // A submit that waited for the server, now answered.
        if (waiting && !checking.size) {
          waiting = false;
          // After this drawing, which another can't start inside.
          queueMicrotask(() => (keys.length ? submitted(keys) : el.requestSubmit()));
        }
        put(cfg.tried, tried && keys.length > 0);
        const d = isDirty();
        wasDirty = d;
        put(cfg.dirty, d);
        for (const b of el.querySelectorAll("[data-form-bar]")) if (mine(b)) b.hidden = !d;
        const off = !!live(cfg.disabled);
        if (off !== !!el.inert) el.inert = off;
        for (const s of el.querySelectorAll("[data-i-x=ui-error-summary]")) s.__x?.draw?.();
      } finally {
        drawing = false;
      }
    }
    function saved() {
      base = snap();
      tried = false;
      touched.clear();
      wasDirty = false;
    }

    el.addEventListener("focusout", (e) => {
      const k = keyOf(e.target, el);
      // Within the same component (a date picker's calendar) it isn't left yet.
      if (!k || !(k in cfg.rules) || touched.has(k) || (e.relatedTarget && keyOf(e.relatedTarget, el) === k)) return;
      touched.add(k);
      // After the click that took the focus: messages move what's under the pointer.
      setTimeout(draw, 120);
    });
    el.addEventListener("submit", (e) => {
      e.preventDefault();
      if (e.target !== el) return;
      const wrong = Object.keys(check());
      if (!wrong.length && !checking.size && !live(cfg.disabled)) return;
      // Before the form's own `submit` actions, which then don't run.
      e.stopImmediatePropagation();
      if (!wrong.length && checking.size && !live(cfg.disabled)) {
        waiting = true;
        hurry();
        return;
      }
      submitted(wrong);
    });
    function submitted(wrong) {
      tried = true;
      draw();
      const summary = [...el.querySelectorAll("[data-i-x=ui-error-summary]")].find(mine);
      if (summary) {
        summary.focus({ preventScroll: true });
        summary.scrollIntoView({ block: "nearest" });
      } else {
        // In the order they're in on the page.
        for (const n of el.querySelectorAll("[data-i-bind], [data-i-x]")) {
          const k = n.getAttribute("data-i-bind") ?? n.__xc?.bind;
          if (wrong.includes(k) && goTo(el, k)) break;
        }
      }
      api.emit("invalid", { errors: check() });
    }
    el.addEventListener("irender:submitted", (e) => {
      if (!e.detail?.ok) return;
      saved();
      draw();
    });
    el.addEventListener("click", (e) => {
      if (!e.target.closest("[data-form-discard]") || !mine(e.target)) return;
      // A field being typed in is put back too, which it isn't while it has the focus.
      if (document.activeElement?.matches("[data-i-bind]") && el.contains(document.activeElement)) document.activeElement.blur();
      const back = base;
      tried = false;
      touched.clear();
      for (const [k, v] of Object.entries(back)) if (api.state[k] !== v) api.set(k, v);
      api.emit("discard", {});
      draw();
    });

    // ---- leaving
    const guard = el.querySelector(":scope > [data-form-guard]");
    if (cfg.guard && guard) {
      let to = null;
      addEventListener("beforeunload", (e) => {
        if (el.isConnected && isDirty() && !to) e.preventDefault();
      });
      document.addEventListener("click", (e) => {
        const a = e.target.closest?.("a[href]");
        if (!a || e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey || a.target === "_blank" || a.hasAttribute("download")) return;
        // Within the page isn't leaving it.
        if (a.getAttribute("href").startsWith("#") || (a.pathname === location.pathname && a.host === location.host && a.search === location.search && a.hash) || !el.isConnected || !isDirty()) return;
        e.preventDefault();
        guard.__to = a.href;
        guard.showModal();
      });
      guard.querySelector("[data-form-stay]").addEventListener("click", () => guard.close());
      guard.querySelector("[data-form-leave]").addEventListener("click", () => {
        to = guard.__to;
        guard.close();
        location.assign(to);
      });
    }
    return { update: draw };
  };

  window.__irenderBehaviors["ui-error-summary"] = (el, api) => {
    const cfg = api.config, form = el.closest("form");
    const env = () => ({ ...api.state, ...(api.scope() ?? {}) });
    const value = (v) => (isObj(v) && "$irender" in v ? window.__irender.evaluate(v.$irender, env()) : v);
    let was = "";
    function draw() {
      const rules = form.__xc?.rules ?? {}, list = [];
      if (form.__tried) for (const [k, m] of Object.entries(form.__errors ?? {})) list.push([k, rules[k]?.label ? `${value(rules[k].label)}: ${m}` : m]);
      const more = value(cfg.errors);
      if (Array.isArray(more)) for (const m of more) list.push([null, isObj(m) ? String(m.message ?? "") : String(m)]);
      else if (isObj(more)) for (const [k, m] of Object.entries(more)) if (!blank(m)) list.push([k, rules[k]?.label ? `${value(rules[k].label)}: ${m}` : String(m)]);
      el.hidden = !list.length;
      const sig = JSON.stringify(list);
      if (sig === was) return;
      was = sig;
      el.querySelector("strong").textContent = value(cfg.title) ?? L(["There's a problem", "There are {n} problems"], { n: list.length });
      el.querySelector("ul").replaceChildren(...list.map(([k, text]) => {
        const li = document.createElement("li");
        if (k === null) return (li.textContent = text), li;
        const b = Object.assign(document.createElement("button"), { type: "button", textContent: text });
        b.addEventListener("click", () => goTo(form, k));
        li.append(b);
        return li;
      }));
    }
    return { update: draw, draw };
  };
})();
