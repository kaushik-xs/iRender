// iRender browser runtime: state, bindings and actions for server-rendered documents.
// Expression semantics mirror irender-core/src/expr.rs; keep the two in sync.
(() => {
  if (window.__irender) return;
  // `evaluate` is exposed so tests can check parity with the Rust evaluator.
  window.__irender = { evaluate: (template, state) => tv(template, state) };

  const URL_ATTRS = new Set(["href", "src", "action", "formaction", "poster", "cite", "data", "xlink:href"]);
  const safeUrl = (u) => !/^\s*(javascript|vbscript|data):/i.test(u);

  const fin = (n) => (Number.isFinite(n) ? n : null);
  const toNum = (v) =>
    typeof v === "number" ? v
    : typeof v === "boolean" ? +v
    : v === null || v === undefined ? 0
    : typeof v === "string" ? (v.trim() === "" ? NaN : Number(v))
    : NaN;
  const truthy = (v) =>
    !(v === null || v === undefined || v === false || v === 0 || v === "" || Number.isNaN(v));
  const str = (v) =>
    v === null || v === undefined ? ""
    : typeof v === "string" ? v
    : typeof v === "object" ? JSON.stringify(v)
    : Object.is(v, -0) ? "0"
    : String(v);
  const eq = (a, b) =>
    typeof a === "object" && a !== null && typeof b === "object" && b !== null
      ? JSON.stringify(a) === JSON.stringify(b)
      : a === b;
  const lt = (a, b, orEq) => {
    if (typeof a === "string" && typeof b === "string") return orEq ? a <= b : a < b;
    const x = toNum(a), y = toNum(b);
    return orEq ? x <= y : x < y;
  };
  const get = (v, path) => {
    for (const s of path) {
      if (typeof v === "string" && s === "length") v = v.length;
      else if (v !== null && typeof v === "object" && Object.prototype.hasOwnProperty.call(v, s)) v = v[s];
      else return null;
    }
    return v === undefined ? null : v;
  };

  // Evaluates the compact expression JSON produced by `Expr::to_js` against `state`.
  const ev = (e, s) => {
    if ("l" in e) return e.l;
    if ("p" in e) return get(s, e.p);
    if ("u" in e) {
      const a = ev(e.a, s);
      return e.u === "!" ? !truthy(a) : fin(-toNum(a));
    }
    if ("c" in e) return truthy(ev(e.c[0], s)) ? ev(e.c[1], s) : ev(e.c[2], s);
    if (e.b === "&&") { const l = ev(e.x, s); return truthy(l) ? ev(e.y, s) : l; }
    if (e.b === "||") { const l = ev(e.x, s); return truthy(l) ? l : ev(e.y, s); }
    const l = ev(e.x, s), r = ev(e.y, s);
    switch (e.b) {
      case "+": return typeof l === "string" || typeof r === "string" ? str(l) + str(r) : fin(toNum(l) + toNum(r));
      case "-": return fin(toNum(l) - toNum(r));
      case "*": return fin(toNum(l) * toNum(r));
      case "/": return fin(toNum(l) / toNum(r));
      case "%": return fin(toNum(l) % toNum(r));
      case "==": return eq(l, r);
      case "!=": return !eq(l, r);
      case "<": return lt(l, r, false);
      case "<=": return lt(l, r, true);
      case ">": return lt(r, l, false);
      case ">=": return lt(r, l, true);
    }
    return null;
  };
  // Evaluates a template: `{ r: expr }` (raw value) or `{ t: [string | expr, ...] }` (string).
  const tv = (t, s) => ("r" in t ? ev(t.r, s) : t.t.map((p) => (typeof p === "string" ? p : str(ev(p, s)))).join(""));

  function mount(root) {
    if (root.__irender) return;
    const stateEl = root.querySelector("script[data-i-state]");
    let state = stateEl ? JSON.parse(stateEl.textContent) : {};

    // An attribute's JSON, parsed once per element.
    const json = (el, attr, key) => el[key] ?? (el[key] = JSON.parse(el.getAttribute(attr)));
    // The names a rebuilt list put in scope (`item`, `index`), kept on the nodes it inserted.
    const scopeOf = (n) => {
      for (; n && n !== root; n = n.parentNode) if (n.__s) return n.__s;
      return null;
    };
    const env = (el) => {
      const s = scopeOf(el);
      return s ? { ...state, ...s } : state;
    };
    // `n` and its descendants that match `sel` (template contents excluded).
    const within = (n, sel) =>
      n.nodeType !== 1 ? [] : n.matches(sel) ? [n, ...n.querySelectorAll(sel)] : [...n.querySelectorAll(sel)];
    // `mount` actions of inserted elements, run after the next update.
    let pending = [];

    function wire(n) {
      for (const el of within(n, "[data-i-on]")) {
        for (const [name, actions] of Object.entries(json(el, "data-i-on", "__on"))) {
          if (name === "mount") pending.push([actions, el]);
          else el.addEventListener(name, (e) => run(actions, e, el));
        }
      }
      for (const el of within(n, "[data-i-bind]")) {
        const key = el.getAttribute("data-i-bind");
        const evt = el.type === "checkbox" || el.tagName === "SELECT" ? "change" : "input";
        el.addEventListener(evt, () => {
          let v = el.type === "checkbox" ? el.checked : el.value;
          if (el.type === "number" || el.type === "range") v = el.value === "" ? null : Number(el.value);
          set(key, v);
        });
      }
    }

    // Wires freshly inserted nodes and brings their `if` blocks and lists up to date.
    const inserted = (nodes, fresh) => {
      if (fresh) nodes.forEach(wire);
      nodes.forEach(structure);
    };

    // The `data-i-fi` marker that closes an `if` block, skipping nested blocks.
    const closing = (t) => {
      let depth = 0;
      for (let n = t.nextSibling; n; n = n.nextSibling) {
        if (n.nodeName !== "TEMPLATE") continue;
        if (n.hasAttribute("data-i-if")) depth++;
        else if (n.hasAttribute("data-i-fi") && depth-- === 0) return n;
      }
      return null;
    };

    // Content shown by the server sits between the markers; hidden content starts out
    // inside the first one and is cloned the first time it's shown. Hiding moves the
    // nodes aside, so they keep their listeners and input values.
    function toggle(t) {
      const show = truthy(tv(json(t, "data-i-if", "__c"), env(t)));
      t.__shown ??= !t.content.firstChild;
      if (show === t.__shown) return;
      t.__shown = show;
      const end = closing(t);
      if (!show) {
        const kept = document.createDocumentFragment();
        while (t.nextSibling !== end) kept.append(t.nextSibling);
        t.__kept = kept;
        return;
      }
      const fresh = !t.__kept;
      const frag = t.__kept ?? t.content.cloneNode(true);
      t.__kept = null;
      const nodes = [...frag.childNodes];
      const s = scopeOf(t);
      if (s) nodes.forEach((n) => (n.__s = s));
      end.before(frag);
      inserted(nodes, fresh);
    }

    // Lists whose `items` use state carry an item `<template>` as their first child.
    // When the array changes, every item is rebuilt from it.
    function rebuild(el) {
      const cfg = json(el, "data-i-list", "__l");
      const items = tv(cfg.v, env(el));
      if (el.__items === items) return;
      el.__items = items;
      const tpl = el.firstElementChild;
      while (tpl.nextSibling) tpl.nextSibling.remove();
      const base = scopeOf(el);
      (Array.isArray(items) ? items : []).forEach((item, i) => {
        const frag = tpl.content.cloneNode(true);
        const s = { ...base, [cfg.as]: item, [cfg.ix]: i };
        const nodes = [...frag.childNodes];
        nodes.forEach((n) => (n.__s = s));
        el.append(frag);
        inserted(nodes, true);
      });
    }

    function structure(n) {
      for (const el of within(n, "template[data-i-if],[data-i-list]")) {
        if (!root.contains(el)) continue; // removed by an earlier step
        if (el.tagName === "TEMPLATE") toggle(el);
        else rebuild(el);
      }
    }

    function update() {
      structure(root);
      for (const el of root.querySelectorAll("[data-i-b]")) {
        const scope = env(el);
        for (const b of json(el, "data-i-b", "__b")) {
          const v = tv(b.v, scope);
          if (b.k === "text") {
            el.textContent = str(v);
          } else if (b.k === "attr") {
            // ARIA states are "true" / "false"; other booleans follow HTML boolean attributes.
            const aria = /^aria-/i.test(b.n) && typeof v === "boolean";
            const s = aria ? String(v) : v === true ? "" : str(v);
            if (v === null || (v === false && !aria) || (URL_ATTRS.has(b.n) && !safeUrl(s))) el.removeAttribute(b.n);
            else el.setAttribute(b.n, s);
            if (b.n === "value" || b.n === "checked") el[b.n] = b.n === "checked" ? truthy(v) : s;
          } else if (b.k === "var") {
            if (v === null) el.style.removeProperty(b.n);
            else el.style.setProperty(b.n, typeof v === "number" && b.px ? v + "px" : str(v));
          } else if (b.k === "class") {
            const prev = (b.prev ?? b.i).split(/\s+/).filter(Boolean);
            const next = str(v).split(/\s+/).filter(Boolean);
            el.classList.remove(...prev);
            el.classList.add(...next);
            b.prev = next.join(" ");
          }
        }
      }
      for (const el of root.querySelectorAll("[data-i-bind]")) {
        if (el === document.activeElement) continue;
        const v = state[el.getAttribute("data-i-bind")];
        if (el.type === "checkbox") el.checked = truthy(v);
        else el.value = str(v);
      }
      const due = pending;
      pending = [];
      for (const [actions, el] of due) run(actions, null, el);
    }

    const set = (key, value) => {
      state = { ...state, [key]: value };
      update();
      root.dispatchEvent(new CustomEvent("irender:state", { bubbles: true, detail: { key, value, state } }));
    };
    const emit = (el, name, detail) => el.dispatchEvent(new CustomEvent(name, { bubbles: true, detail }));

    // `event` is null for `mount` actions.
    async function run(actions, event, el) {
      const val = (t) => tv(t, env(el));
      const values = (m) => Object.fromEntries(Object.entries(m).map(([k, v]) => [k, val(v)]));
      for (const a of actions) {
        if (a.a === "set") {
          set(a.k, val(a.v));
        } else if (a.a === "nav") {
          event?.preventDefault();
          const url = str(val(a.u));
          if (safeUrl(url)) location.assign(url);
        } else if (a.a === "submit") {
          event?.preventDefault();
          const body = values(a.b);
          let url = str(val(a.e));
          const init = { method: a.m, headers: { accept: "application/json" } };
          if (a.m === "GET") {
            const q = String(new URLSearchParams(Object.entries(body).map(([k, v]) => [k, str(v)])));
            if (q) url += (url.includes("?") ? "&" : "?") + q;
          } else {
            init.headers["content-type"] = "application/json";
            init.body = JSON.stringify(body);
          }
          try {
            const res = await fetch(url, init);
            const type = res.headers.get("content-type") || "";
            const data = type.includes("json") ? await res.json() : await res.text();
            if (a.into) set(a.into, data);
            emit(el, "irender:submitted", { ok: res.ok, status: res.status, data });
          } catch (error) {
            emit(el, "irender:error", { error: String(error) });
          }
        } else if (a.a === "call") {
          emit(el, "irender:call", { name: a.n, args: values(a.args), state });
        }
      }
    }

    // Server-rendered list items already match the initial state.
    for (const el of root.querySelectorAll("[data-i-list]")) el.__items = tv(json(el, "data-i-list", "__l").v, env(el));
    wire(root);

    // Host apps can read and write state: root.__irender.set("qty", 3)
    root.__irender = { get state() { return state; }, set };
    update();
  }

  const init = () => document.querySelectorAll("[data-i-root]").forEach(mount);
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", init);
  else init();
})();
