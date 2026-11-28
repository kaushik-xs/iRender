// iRender browser runtime: state, bindings and actions for server-rendered documents.
// Expression semantics mirror irender-core/src/expr.rs; keep the two in sync.
(() => {
  if (window.__irender) return;
  // `evaluate` is exposed so tests can check parity with the Rust evaluator.
  window.__irender = { evaluate: (template, state) => tv(template, state) };

  // Interface text in the document's words (the Rust side is irender-core's labels.rs).
  // `text` is the English, or `[one, other]` for a count in `vars.n`; the label is found
  // by the English (the `other` form), and may be plural forms picked by the document's
  // `lang`. `{name}`s are filled from `vars`, numbers written in the `lang`'s way (English
  // without one, as the server writes them).
  const labelSets = [];
  const rules = {}, numbers = {};
  window.__irender.label = (text, vars = {}) => {
    const [one, other] = Array.isArray(text) ? text : [text, text];
    let set = labelSets.find((x) => x.labels && Object.hasOwn(x.labels, other));
    const lang = (set ?? labelSets.find((x) => x.lang))?.lang || "en";
    const l = set?.labels[other], n = vars.n;
    let s;
    if (typeof l === "string") s = l;
    else if (l && typeof l === "object") s = l[typeof n === "number" ? (rules[lang] ||= new Intl.PluralRules(lang)).select(n) : "other"] ?? l.other;
    else s = n === 1 ? one : other;
    return String(s).replace(/\{(\w+)\}/g, (m, k) => {
      if (!(k in vars)) return m;
      const v = vars[k];
      return typeof v === "number" ? (numbers[lang] ||= new Intl.NumberFormat(lang)).format(v) : String(v);
    });
  };

  const CSS_WIDE = /^\s*(inherit|initial|unset|revert|revert-layer)\s*$/i;
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

  // The built-in functions (`Func` in expr.rs), each bounded like there.
  const MAX_RANGE = 1000;
  const fns = {
    range: (a) => {
      const int = (v) => { const n = toNum(v); return Number.isFinite(n) ? Math.trunc(n) : 0; };
      const [start, end] = a.length > 1 ? [int(a[0]), int(a[1])] : [0, int(a[0])];
      return Array.from({ length: Math.max(0, Math.min(MAX_RANGE, end - start)) }, (_, i) => fin(start + i));
    },
    includes: ([list, v]) => Array.isArray(list) && list.some((x) => eq(x, v)),
    toggle: ([list, v]) => {
      const items = Array.isArray(list) ? list : [];
      const rest = items.filter((x) => !eq(x, v));
      return rest.length === items.length ? [...items, v] : rest;
    },
    // Halves away from zero, as Rust's `f64::round`.
    round: ([x, d]) => {
      const n = Number.isFinite(toNum(x)) ? toNum(x) : 0, k = Math.min(10, Math.max(0, Math.trunc(Number.isFinite(toNum(d)) ? toNum(d) : 0))), p = 10 ** k;
      return fin((Math.round(Math.abs(n) * p) / p) * (Math.sign(n) || 1));
    },
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
    if ("f" in e) return fns[e.f] ? fns[e.f](e.a.map((x) => ev(x, s))) : null;
    // `value[key]` with a computed key: a string names a field, a whole number an index.
    if ("i" in e) {
      const k = ev(e.i[1], s);
      return typeof k === "string" || (Number.isInteger(k) && k >= 0) ? get(ev(e.i[0], s), [String(k)]) : null;
    }
    // An object or list built from parts (a literal value with `{{ }}` inside it).
    if ("o" in e) return Object.fromEntries(e.o.map(([k, x]) => [k, ev(x, s)]));
    if ("s" in e) return e.s.map((x) => ev(x, s));
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
    const stateEl = [...root.children].find((c) => c.matches("script[data-i-state]"));
    let state = stateEl ? JSON.parse(stateEl.textContent) : {};
    const labelsEl = [...root.children].find((c) => c.matches("script[data-i-labels]"));
    if (labelsEl) labelSets.push(JSON.parse(labelsEl.textContent));

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
    // Controls that choose a value rather than type one.
    const picks = (el) => el.type === "checkbox" || el.type === "radio" || el.tagName === "SELECT";
    // A root can hold other roots, e.g. a rendered fragment in a chat message: their
    // elements belong to them, not to this one.
    const own = (els) => (root.querySelector("[data-i-root]") ? els.filter((el) => el.closest("[data-i-root]") === root) : els);
    // `n` and its descendants that match `sel` (template contents excluded).
    const within = (n, sel) =>
      own(n.nodeType !== 1 ? [] : n.matches(sel) ? [n, ...n.querySelectorAll(sel)] : [...n.querySelectorAll(sel)]);
    const all = (sel) => own([...root.querySelectorAll(sel)]);
    // `mount` actions of inserted elements, run after the next update.
    let pending = [];

    function wire(n) {
      // Component behaviours (see `RenderCx::global_js`), started once per element.
      for (const el of within(n, "[data-i-x]")) {
        const start = (window.__irenderBehaviors || {})[el.getAttribute("data-i-x")];
        if (!start || el.__x) continue;
        el.__x = start(el, {
          config: json(el, "data-i-xc", "__xc"),
          get state() { return state; },
          set,
          scope: () => scopeOf(el),
          emit: (name, detail) => emit(el, name, detail),
          // Supplies the items of a list the browser rebuilds (an element inside this
          // one), in place of its own `items`, and redraws. Before the first update it
          // only records them.
          list: (listEl, items, offset = 0) => {
            listEl.__supply = { items, offset };
            if (root.__irender) update();
          },
          // Several at once, `[listEl, items, offset?]`s, with one redraw.
          lists: (entries) => {
            for (const [listEl, items, offset = 0] of entries) listEl.__supply = { items, offset };
            if (root.__irender && entries.length) update();
          },
        }) || {};
      }
      for (const el of within(n, "[data-i-on]")) {
        for (const [name, actions] of Object.entries(json(el, "data-i-on", "__on"))) {
          if (name === "mount") pending.push([actions, el]);
          else el.addEventListener(name, (e) => run(actions, e, el));
        }
      }
      for (const el of within(n, "[data-i-bind]")) {
        const key = el.getAttribute("data-i-bind");
        const evt = picks(el) ? "change" : "input";
        el.addEventListener(evt, () => {
          let v = el.type === "checkbox" ? el.checked : el.value;
          if (el.multiple) {
            // A list, of numbers when the key held numbers.
            const nums = Array.isArray(state[key]) && state[key].length > 0 && state[key].every((x) => typeof x === "number");
            v = [...el.selectedOptions].map((o) => (nums && o.value.trim() !== "" && !Number.isNaN(Number(o.value)) ? Number(o.value) : o.value));
          } else if (el.type === "number" || el.type === "range") v = el.value === "" ? null : Number(el.value);
          // A radio or select over numbers keeps a number-typed key a number.
          else if (picks(el) && el.type !== "checkbox" && typeof state[key] === "number" && v.trim() !== "" && !Number.isNaN(Number(v))) v = Number(v);
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
    // When the array changes, items are rebuilt from it. With a `key`, an item whose
    // key survives keeps its nodes (and their focus and input values) and only gets
    // its new `item` and `index`; without one, every item is rebuilt.
    //
    // A behaviour can supply the items itself (`api.list`), e.g. only the rows in
    // view; their indexes then count from its `offset`.
    function rebuild(el) {
      const cfg = json(el, "data-i-list", "__l");
      const base = scopeOf(el);
      const sup = el.__supply;
      const items = sup ? sup.items : tv(cfg.v, env(el));
      const old = el.__recs ?? [];
      if (el.__items === items) {
        // An outer item may have changed under a kept list: refresh what it copied.
        if (base) for (const r of old) Object.assign(r.s, base, { [cfg.as]: r.item, [cfg.ix]: r.i });
        return;
      }
      el.__items = items;
      const tpl = el.firstElementChild;
      // Each item's nodes run from its first node to the next item's: an `if` inside
      // may have added some since.
      old.forEach((r, k) => {
        r.nodes = [];
        for (let n = r.first; n && n !== old[k + 1]?.first; n = n.nextSibling) r.nodes.push(n);
      });
      if (!el.__recs) while (tpl.nextSibling) tpl.nextSibling.remove(); // server-rendered items
      // Old items by key; a duplicated key is reused once per occurrence.
      const free = new Map();
      if (cfg.k) for (const r of old) free.has(r.key) ? free.get(r.key).push(r) : free.set(r.key, [r]);
      const all = env(el);
      const recs = (Array.isArray(items) ? items : []).map((item, n) => {
        const i = n + (sup?.offset ?? 0);
        const vars = { [cfg.as]: item, [cfg.ix]: i };
        const key = cfg.k ? str(tv(cfg.k, { ...all, ...vars })) : null;
        const r = cfg.k ? free.get(key)?.shift() : undefined;
        if (r) {
          Object.assign(r.s, base, vars);
          r.item = item;
          r.i = i;
          r.kept = true;
          return r;
        }
        const frag = tpl.content.cloneNode(true);
        if (!frag.firstChild) frag.append(document.createTextNode(""));
        const s = { ...base, ...vars };
        const nodes = [...frag.childNodes];
        nodes.forEach((n) => (n.__s = s));
        return { key, item, i, s, nodes, first: nodes[0], frag };
      });
      const focused = el.contains(document.activeElement) ? document.activeElement : null;
      for (const r of old) if (!r.kept) r.nodes.forEach((n) => n.remove());
      // Put items in order, moving only those out of place.
      let at = tpl;
      for (const r of recs) {
        if (r.frag) at.after(r.frag);
        else if (at.nextSibling !== r.first) at.after(...r.nodes);
        at = r.nodes[r.nodes.length - 1];
      }
      // Moving a node blurs it.
      if (focused?.isConnected && document.activeElement !== focused) focused.focus({ preventScroll: true });
      for (const r of recs) {
        if (r.frag) inserted(r.nodes, true);
        delete r.frag;
        delete r.kept;
      }
      el.__recs = recs;
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
      for (const el of all("[data-i-b]")) {
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
            // Only when it changes: setting an iframe's `src` (or a media element's) again,
            // even to the same address, loads it again.
            else if (el.getAttribute(b.n) !== s) el.setAttribute(b.n, s);
            if (b.n === "value" || b.n === "checked") el[b.n] = b.n === "checked" ? truthy(v) : s;
          } else if (b.k === "var") {
            // As the server writes it: `initial` when there's no value, so an ancestor's
            // var of the same name never shows through, and a CSS-wide keyword on the
            // property itself, since it can't pass through a var.
            const css = v === null ? null : typeof v === "number" && b.px ? v + "px" : str(v);
            const keyword = css !== null && CSS_WIDE.test(css);
            if (b.p) el.style.removeProperty(b.p);
            el.style.setProperty(b.n, css === null || keyword ? "initial" : css);
            if (keyword && b.p) el.style.setProperty(b.p, css.trim());
          } else if (b.k === "opts") {
            // A select's options, from state: rebuilt when the array changes.
            if (v === b.prev) continue;
            b.prev = v;
            const keep = el.value;
            for (const o of [...el.options]) if (!o.hasAttribute("data-i-ph")) o.remove();
            for (const o of Array.isArray(v) ? v : []) {
              const obj = o !== null && typeof o === "object";
              const opt = document.createElement("option");
              opt.value = str(obj ? o.value : o);
              opt.textContent = str(obj ? (o.label ?? o.value) : o);
              opt.disabled = obj && truthy(o.disabled);
              el.append(opt);
            }
            el.value = keep;
          } else if (b.k === "class") {
            const prev = (b.prev ?? b.i).split(/\s+/).filter(Boolean);
            const next = str(v).split(/\s+/).filter(Boolean);
            el.classList.remove(...prev);
            el.classList.add(...next);
            b.prev = next.join(" ");
          }
        }
      }
      for (const el of all("[data-i-bind]")) {
        // Don't fight someone typing; controls that pick a value are safe to set.
        if (el === document.activeElement && !picks(el)) continue;
        const v = state[el.getAttribute("data-i-bind")];
        if (el.type === "checkbox") el.checked = truthy(v);
        else if (el.type === "radio") el.checked = el.value === str(v);
        else if (el.multiple) {
          const on = new Set((Array.isArray(v) ? v : v == null ? [] : [v]).map(str));
          for (const o of el.options) o.selected = on.has(o.value);
        } else el.value = str(v);
      }
      for (const el of all("[data-i-x]")) el.__x?.update?.(state);
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
            // Saved: the records it changed are fetched again (`ui-data-source`s, in the UI kit).
            if (a.inv && res.ok) {
              for (const [name, ids] of Object.entries(values(a.inv))) {
                window.__irenderData?.invalidate(name, ids === true || ids == null ? null : Array.isArray(ids) ? ids : [ids]);
              }
            }
            emit(el, "irender:submitted", { ok: res.ok, status: res.status, data });
          } catch (error) {
            emit(el, "irender:error", { error: String(error) });
          }
        } else if (a.a === "call") {
          emit(el, "irender:call", { name: a.n, args: values(a.args), state });
        } else if (a.a === "wait") {
          await new Promise((resolve) => setTimeout(resolve, a.ms));
        }
      }
    }

    // Server-rendered list items already match the initial state. A keyed list's are
    // replaced by template items on the first update instead, so that they can be kept
    // later: the server folded their `item` into plain text. Removed before wiring, so
    // their `mount` actions don't run twice.
    for (const el of all("[data-i-list]")) {
      const cfg = json(el, "data-i-list", "__l");
      if (!cfg.k) el.__items = tv(cfg.v, env(el));
      else while (el.firstElementChild.nextSibling) el.firstElementChild.nextSibling.remove();
    }
    wire(root);

    // Host apps can read and write state: root.__irender.set("qty", 3)
    root.__irender = { get state() { return state; }, set };
    update();
  }

  const init = () => document.querySelectorAll("[data-i-root]").forEach(mount);
  // Starts the roots in `el`, e.g. a fragment rendered on the server and inserted later.
  window.__irender.mount = (el) => [...(el.matches?.("[data-i-root]") ? [el] : []), ...el.querySelectorAll("[data-i-root]")].forEach(mount);
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", init);
  else init();
})();
