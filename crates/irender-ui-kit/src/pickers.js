// irender-ui-kit: `ui-query-builder`, `ui-transfer-list` and `ui-tree-select`.
(() => {
  const B = (window.__irenderBehaviors ||= {});
  const K = window.__irenderKit, L = (t, v) => window.__irender.label(t, v);
  const { make, button, icon, isObj, blank } = K;
  const fold = (s) => String(s ?? "").normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
  const choice = (o) => (isObj(o) ? { ...o, label: String(o.label ?? o.value ?? "") } : { value: o, label: String(o ?? "") });

  // ---------------------------------------------------------------- query builder
  // Operators by type: name, words, how many values, and the test.
  const num = (v) => (blank(v) || isNaN(v) ? NaN : Number(v));
  const list = (v) => (Array.isArray(v) ? v : blank(v) ? [] : [v]);
  const OPS = {
    text: [["contains", "contains", 1], ["notContains", "doesn't contain", 1], ["is", "is", 1], ["isNot", "is not", 1], ["starts", "starts with", 1], ["ends", "ends with", 1], ["empty", "is empty", 0], ["notEmpty", "is not empty", 0]],
    number: [["eq", "is", 1], ["ne", "is not", 1], ["gt", "is more than", 1], ["gte", "is at least", 1], ["lt", "is less than", 1], ["lte", "is at most", 1], ["between", "is between", 2], ["empty", "is empty", 0], ["notEmpty", "is not empty", 0]],
    date: [["on", "is", 1], ["before", "is before", 1], ["after", "is after", 1], ["between", "is between", 2], ["last", "is in the last (days)", 1, "number"], ["next", "is in the next (days)", 1, "number"], ["empty", "is empty", 0], ["notEmpty", "is not empty", 0]],
    select: [["is", "is", 1], ["isNot", "is not", 1], ["in", "is any of", "many"], ["notIn", "is none of", "many"], ["empty", "is empty", 0], ["notEmpty", "is not empty", 0]],
    multiselect: [["has", "includes", 1], ["hasNot", "doesn't include", 1], ["hasAny", "includes any of", "many"], ["hasAll", "includes all of", "many"], ["empty", "is empty", 0], ["notEmpty", "is not empty", 0]],
    boolean: [["true", "is true", 0], ["false", "is false", 0]],
  };
  OPS.currency = OPS.number;
  // Labels: "contains", "doesn't contain", "is", "is not", "starts with", "ends with", "is empty", "is not empty"
  // Labels: "is more than", "is at least", "is less than", "is at most", "is between", "is before", "is after"
  // Labels: "is in the last (days)", "is in the next (days)", "is any of", "is none of", "includes", "doesn't include"
  // Labels: "includes any of", "includes all of", "is true", "is false"
  const day = (v) => String(v ?? "").slice(0, 10);
  const DAY = 86400000;
  const today = () => {
    const d = new Date();
    return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
  };
  const TEST = {
    contains: (a, v) => fold(a).includes(fold(v)), notContains: (a, v) => !fold(a).includes(fold(v)), starts: (a, v) => fold(a).startsWith(fold(v)), ends: (a, v) => fold(a).endsWith(fold(v)),
    is: (a, v) => (typeof a === "string" && typeof v === "string" ? fold(a) === fold(v) : a === v || String(a) === String(v)), isNot: (a, v) => !TEST.is(a, v),
    eq: (a, v) => num(a) === num(v), ne: (a, v) => num(a) !== num(v), gt: (a, v) => num(a) > num(v), gte: (a, v) => num(a) >= num(v), lt: (a, v) => num(a) < num(v), lte: (a, v) => num(a) <= num(v),
    between: (a, v, t) => (t === "date" ? day(a) >= day(v[0]) && day(a) <= day(v[1]) && !blank(a) : num(a) >= num(v[0]) && num(a) <= num(v[1])),
    on: (a, v) => day(a) === day(v), before: (a, v) => !blank(a) && day(a) < day(v), after: (a, v) => !blank(a) && day(a) > day(v),
    last: (a, v) => !blank(a) && Date.parse(day(a) + "T00:00") > today() - num(v) * DAY && Date.parse(day(a) + "T00:00") <= today(),
    next: (a, v) => !blank(a) && Date.parse(day(a) + "T00:00") >= today() && Date.parse(day(a) + "T00:00") < today() + num(v) * DAY,
    in: (a, v) => list(v).some((x) => TEST.is(a, x)), notIn: (a, v) => !TEST.in(a, v),
    has: (a, v) => list(a).some((x) => TEST.is(x, v)), hasNot: (a, v) => !TEST.has(a, v), hasAny: (a, v) => list(v).some((x) => TEST.has(a, x)), hasAll: (a, v) => list(v).every((x) => TEST.has(a, x)),
    empty: (a) => blank(a) || (Array.isArray(a) && !a.length), notEmpty: (a) => !TEST.empty(a), true: (a) => a === true || a === "true" || a === 1, false: (a) => !TEST.true(a),
  };
  B["ui-query-builder"] = (el, api) => {
    const cfg = api.config, fields = new Map(cfg.fields.map((f) => [f.field, { ...f, type: f.type ?? "text", label: String(f.label ?? f.field) }]));
    const first = [...fields.keys()][0];
    const ops = (f) => {
      const all = OPS[f?.type ?? "text"];
      return Array.isArray(f?.operators) ? all.filter((o) => f.operators.includes(o[0])) : all;
    };
    const opOf = (r) => ops(fields.get(r.field)).find((o) => o[0] === r.operator) ?? ops(fields.get(r.field))[0];
    const isGroup = (x) => isObj(x) && Array.isArray(x.rules);
    const clean = (q) => ({ op: q?.op === "or" ? "or" : "and", ...(q?.not ? { not: true } : {}), rules: (isGroup(q) ? q.rules : []).filter(isObj).map((r) => (isGroup(r) ? clean(r) : r)) });
    const query = () => clean(api.state[cfg.bind]);
    const rule = () => ({ field: first, operator: ops(fields.get(first))[0][0], value: null });
    const off = () => !!K.value(api, cfg.disabled);
    // The query changed at a path of places in `rules`.
    function set(path, f, focus) {
      const go = (g, p) => (p.length ? { ...g, rules: g.rules.flatMap((r, i) => (i === p[0] ? (p.length === 1 ? f(r) ?? [] : go(r, p.slice(1))) : [r])) } : g);
      const q = path.length ? go(query(), path) : f(query());
      api.set(cfg.bind, q);
      api.emit("change", { query: q });
      if (focus) el.querySelector(`[data-path="${focus}"]`)?.focus();
    }
    const filled = (r) => {
      const o = opOf(r);
      return o[2] === 0 || (o[2] === "many" ? list(r.value).length > 0 : o[2] === 2 ? Array.isArray(r.value) && r.value.length === 2 && r.value.every((v) => !blank(v)) : !blank(r.value));
    };
    const complete = (g) => g.rules.every((r) => (isGroup(r) ? complete(r) && r.rules.length > 0 : fields.has(r.field) && filled(r)));
    const at = (row, f) => String(f).split(".").reduce((x, k) => (x !== null && typeof x === "object" ? x[k] : undefined), row);
    // Rules that aren't filled in yet don't narrow anything.
    function match(row, g) {
      const rs = g.rules.filter((r) => (isGroup(r) ? r.rules.length : fields.has(r.field) && filled(r)));
      const one = (r) => (isGroup(r) ? match(row, r) : TEST[opOf(r)[0]](at(row, r.field), r.value, fields.get(r.field).type));
      const ok = !rs.length || (g.op === "or" ? rs.some(one) : rs.every(one));
      return g.not && rs.length ? !ok : ok;
    }
    const shown = (f, v) => {
      if (blank(v)) return "…";
      if (f.options) return (f.options.map(choice).find((o) => o.value === v)?.label ?? String(v));
      if (f.type === "date" && /^\d{4}-\d{2}-\d{2}$/.test(v)) return new Intl.DateTimeFormat(cfg.locale || undefined, { dateStyle: "medium", timeZone: "UTC" }).format(new Date(v));
      if (f.type === "number" || f.type === "currency") return K.format(v, { currency: f.type === "currency" ? f.currency : undefined, locale: cfg.locale }) + (f.unit ? ` ${f.unit}` : "");
      return `“${v}”`;
    };
    function words(g, top = true) {
      const parts = g.rules.map((r) => {
        if (isGroup(r)) return words(r, false);
        const f = fields.get(r.field);
        if (!f) return "";
        const o = opOf(r), and = ` ${L("and")} `;
        const v = o[2] === 0 ? "" : o[2] === "many" ? list(r.value).map((x) => shown(f, x)).join(", ") || "…" : o[2] === 2 ? [0, 1].map((i) => shown(f, r.value?.[i])).join(and) : o[3] === "number" ? (blank(r.value) ? "…" : String(r.value)) : shown(f, r.value);
        return `${f.label} ${L(o[1])}${v ? ` ${v}` : ""}`;
      }).filter(Boolean);
      const text = parts.join(` ${g.op === "or" ? L("or") : L("and")} `);
      return g.not ? `${L("not")} (${text})` : !top && parts.length > 1 ? `(${text})` : text;
    }

    // ---- drawing
    function value(r, path) {
      const f = fields.get(r.field), o = opOf(r), key = path.join("."), many = o[2] === "many";
      const put = (v) => set(path, (x) => [{ ...x, value: v }]);
      const type = o[3] ?? f?.type;
      const input = (v, change, i) => {
        const e = make("input", { "data-path": `${key}:v${i ?? ""}`, "aria-label": L("Value"), type: type === "date" ? "date" : "text", inputmode: type === "number" || type === "currency" ? "decimal" : null, disabled: off(), "data-q-empty": blank(v) });
        e.value = blank(v) ? "" : String(v);
        e.addEventListener("input", () => change(e.value === "" ? null : type === "number" || type === "currency" ? window.__irenderCells.parse({ type: "number" }, e.value, cfg.locale) : e.value));
        return e;
      };
      const options = (f?.options ?? []).map(choice);
      if (o[2] === 0) return [];
      if (many) {
        const has = list(r.value);
        return options.map((c) => button({ "data-q-chip": "", "data-path": `${key}:c${c.value}`, "aria-pressed": String(has.includes(c.value)), disabled: off(), click: () => set(path, (x) => [{ ...x, value: list(x.value).includes(c.value) ? list(x.value).filter((v) => v !== c.value) : [...list(x.value), c.value] }], `${key}:c${c.value}`) }, c.label));
      }
      if (o[2] === 2) {
        const pair = Array.isArray(r.value) ? r.value : [null, null];
        return [input(pair[0], (v) => set(path, (x) => [{ ...x, value: [v, (Array.isArray(x.value) ? x.value : [])[1] ?? null] }]), 0), make("small", null, L("and")), input(pair[1], (v) => set(path, (x) => [{ ...x, value: [(Array.isArray(x.value) ? x.value : [])[0] ?? null, v] }]), 1)];
      }
      if (options.length && type !== "number") {
        const s = make("select", { "data-path": `${key}:v`, "aria-label": L("Value"), disabled: off(), "data-q-empty": blank(r.value) }, new Option(L("Select…"), ""), options.map((c, i) => new Option(c.label, String(i), false, c.value === r.value)));
        s.addEventListener("change", () => put(s.value === "" ? null : options[Number(s.value)].value));
        return [s];
      }
      return [input(r.value, put), f?.unit ? make("small", null, f.unit) : null];
    }
    function drawRule(r, path) {
      const key = path.join(".");
      const fs = make("select", { "data-path": `${key}:f`, "aria-label": L("Field"), disabled: off() }, [...fields.values()].map((f) => new Option(f.label, f.field, false, f.field === r.field)));
      fs.addEventListener("change", () => set(path, () => [{ field: fs.value, operator: ops(fields.get(fs.value))[0][0], value: null }], `${key}:f`));
      const os = make("select", { "data-path": `${key}:o`, "aria-label": L("Condition"), disabled: off() }, ops(fields.get(r.field)).map((o) => new Option(L(o[1]), o[0], false, o[0] === opOf(r)[0])));
      os.addEventListener("change", () => set(path, (x) => {
        const was = opOf(x), o = ops(fields.get(x.field)).find((p) => p[0] === os.value);
        // A value of the same kind stays.
        return [{ ...x, operator: os.value, value: was[2] === o[2] && was[3] === o[3] ? x.value : o[2] === "many" && was[2] === 1 && !blank(x.value) ? [x.value] : null }];
      }, `${key}:o`));
      return make("div", { "data-q-rule": "", role: "group", "aria-label": L("Condition {n}", { n: path.at(-1) + 1 }) }, fs, os, make("div", { "data-q-value": "" }, value(r, path)), off() ? null : button({ "data-q-tool": "", "data-path": `${key}:x`, "aria-label": L("Remove condition"), title: L("Remove"), click: () => set(path, () => null, `${path.slice(0, -1).join(".")}:add`) }, icon("x", 14)));
    }
    function drawGroup(g, path) {
      const key = path.join("."), dis = off();
      const seg = make("span", { "data-q-seg": "", role: "group", "aria-label": L("Match") }, [["and", L("All")], ["or", L("Any")]].map(([op, t]) => button({ "aria-pressed": String(g.op === op), "data-path": `${key}:${op}`, disabled: dis, click: () => set(path, (x) => (path.length ? [{ ...x, op }] : { ...x, op }), `${key}:${op}`) }, t)));
      const add = (x) => set(path, (q) => (path.length ? [{ ...q, rules: [...q.rules, x] }] : { ...q, rules: [...q.rules, x] }), `${[...path, g.rules.length].join(".")}:${isGroup(x) ? "and" : "f"}`);
      const bar = make("div", { "data-q-bar": "" }, seg,
        cfg.negate ? button({ "data-q-chip": "", "data-path": `${key}:not`, "aria-pressed": String(!!g.not), disabled: dis, click: () => set(path, (x) => { const n = { ...x, not: !x.not }; if (!n.not) delete n.not; return path.length ? [n] : n; }, `${key}:not`) }, L("Not")) : null,
        make("span", { "data-push": "" }),
        dis ? null : button({ "data-q-add": "", "data-path": `${key}:add`, click: () => add(rule()) }, icon("plus", 14), L("Condition")),
        dis || path.length + 1 >= cfg.maxDepth ? null : button({ "data-q-add": "", "data-path": `${key}:addg`, click: () => add({ op: g.op === "and" ? "or" : "and", rules: [rule()] }) }, icon("plus", 14), L("Group")),
        path.length && !dis ? button({ "data-q-tool": "", "aria-label": L("Remove group"), title: L("Remove group"), click: () => set(path, () => null, `${path.slice(0, -1).join(".")}:add`) }, icon("x", 14)) : null);
      const rules = g.rules.map((r, i) => (isGroup(r) ? drawGroup(r, [...path, i]) : drawRule(r, [...path, i])));
      return make("div", { "data-q-group": "", role: "group", "aria-label": path.length ? L("Group") : null }, bar, rules.length ? make("div", { "data-q-list": "" }, rules) : null);
    }
    let was = null;
    function draw() {
      const q = query(), sig = JSON.stringify([q, off()]);
      if (sig !== was) {
        was = sig;
        // What's being typed in stays as it is: only its neighbours are drawn again.
        const f = el.contains(document.activeElement) ? document.activeElement : null, key = f?.getAttribute("data-path");
        const caret = f?.matches("input[type=text]") ? [f.selectionStart, f.selectionEnd, f.value] : null;
        el.replaceChildren(drawGroup(q, []), ...(cfg.showText ? [make("p", { "data-q-text": "" })] : []));
        const back = key && el.querySelector(`[data-path="${CSS.escape(key)}"]`);
        if (back) {
          if (caret && back.matches("input")) back.value = caret[2];
          back.focus();
          if (caret && back.type === "text") back.setSelectionRange(caret[0], caret[1]);
        }
      }
      const text = words(q);
      const p = el.querySelector(":scope > [data-q-text]");
      if (p) p.textContent = text || L("No conditions: everything matches.");
      const put = (k, v) => k && JSON.stringify(api.state[k] ?? null) !== JSON.stringify(v) && api.set(k, v);
      put(cfg.bindText, text);
      put(cfg.bindValid, complete(q));
      if (cfg.bindMatches) put(cfg.bindMatches, (K.value(api, cfg.rows) ?? []).filter((r) => match(r, q)));
    }
    draw();
    return { update: draw };
  };

  // ---------------------------------------------------------------- transfer list
  B["ui-transfer-list"] = (el, api) => {
    const cfg = api.config, sides = {}, picked = { from: new Set(), to: new Set() }, on = { from: 0, to: 0 };
    const options = () => (K.value(api, cfg.options) ?? []).map(choice);
    const chosen = () => list(api.state[cfg.bind]);
    const off = () => !!K.value(api, cfg.disabled);
    const uid = `t-${K.id()}`;
    let last = null;
    for (const side of ["from", "to"]) {
      const search = make("input", { type: "search", "aria-label": L("Search"), placeholder: L("Search…"), input: () => draw() });
      const box = make("div", { role: "listbox", tabindex: 0, "aria-multiselectable": "true", id: `${uid}-${side}` });
      const title = make("span", { id: `${uid}-${side}-t` }), count = make("small");
      box.setAttribute("aria-labelledby", title.id);
      sides[side] = { search, box, title, count, el: make("div", { "data-t-list": side }, make("div", { "data-t-head": "" }, title, count), cfg.search ? make("div", { "data-t-search": "" }, search) : null, box) };
    }
    const moves = make("div", { "data-t-moves": "" },
      ...[["add", "right", L("Add the selected"), "from"], ["remove", "left", L("Remove the selected"), "to"]].map(([m, i, label, side]) => button({ "data-m": m, "aria-label": label, title: label, click: () => move(side, [...picked[side]]) }, icon(i, 16))));
    el.append(sides.from.el, moves, sides.to.el);
    const shown = (side) => {
      const has = chosen(), q = fold(sides[side].search.value.trim()), all = options();
      const own = side === "from" ? all.filter((o) => !has.includes(o.value)) : has.map((v) => all.find((o) => o.value === v) ?? { value: v, label: String(v) });
      return q ? own.filter((o) => fold(o.label).includes(q) || fold(o.description).includes(q)) : own;
    };
    function move(side, values) {
      if (off()) return;
      const has = chosen(), blocked = new Set(options().filter((o) => o.disabled).map((o) => o.value));
      values = values.filter((v) => !blocked.has(v));
      if (side === "from" && cfg.max != null) values = values.slice(0, Math.max(0, cfg.max - has.length));
      if (!values.length) return;
      const next = side === "from" ? [...has, ...values.filter((v) => !has.includes(v))] : has.filter((v) => !values.includes(v));
      values.forEach((v) => picked[side].delete(v));
      api.set(cfg.bind, next);
      api.emit("change", { value: next });
      K.say(el, side === "from" ? L(["{n} added", "{n} added"], { n: values.length }) : L(["{n} removed", "{n} removed"], { n: values.length }));
    }
    function reorder(v, to) {
      const has = chosen().filter((x) => x !== v);
      has.splice(K.clamp(to, 0, has.length), 0, v);
      api.set(cfg.bind, has);
      api.emit("change", { value: has });
    }
    function draw() {
      const titles = K.value(api, cfg.titles) ?? [];
      for (const side of ["from", "to"]) {
        const s = sides[side], all = shown(side);
        on[side] = K.clamp(on[side], 0, Math.max(0, all.length - 1));
        s.title.textContent = titles[side === "from" ? 0 : 1] ?? (side === "from" ? L("Available") : L("Chosen"));
        s.count.textContent = side === "to" && cfg.max != null ? `${all.length} / ${cfg.max}` : String(all.length);
        for (const v of [...picked[side]]) if (!all.some((o) => o.value === v)) picked[side].delete(v);
        const kids = [];
        let group = null;
        all.forEach((o, i) => {
          if (side === "from" && o.group && o.group !== group) kids.push(make("div", { "data-t-group": "", role: "presentation" }, (group = o.group)));
          kids.push(make("div", { role: "option", id: `${uid}-${side}-${i}`, "data-i": i, "aria-selected": String(picked[side].has(o.value)), "aria-disabled": o.disabled ? "true" : null, "data-on": i === on[side], draggable: off() || o.disabled ? null : "true" }, make("span", null, o.label, o.description ? make("small", null, o.description) : null)));
        });
        s.box.replaceChildren(...(kids.length ? kids : [make("div", { "data-t-none": "" }, s.search.value ? L("No results") : side === "from" ? L("Nothing left") : L("None chosen"))]));
        if (all.length) s.box.setAttribute("aria-activedescendant", `${uid}-${side}-${on[side]}`);
        else s.box.removeAttribute("aria-activedescendant");
      }
      const full = cfg.max != null && chosen().length >= cfg.max;
      moves.children[0].disabled = off() || !picked.from.size || full;
      moves.children[1].disabled = off() || !picked.to.size;
    }
    for (const side of ["from", "to"]) {
      const { box } = sides[side];
      const at = (n) => {
        const o = n.closest?.("[role=option]");
        return o ? Number(o.getAttribute("data-i")) : -1;
      };
      const toggle = (i, only) => {
        const o = shown(side)[i];
        if (!o || o.disabled) return;
        if (only) picked[side].clear();
        picked[side].has(o.value) ? picked[side].delete(o.value) : picked[side].add(o.value);
      };
      box.addEventListener("click", (e) => {
        const i = at(e.target);
        if (i < 0) return;
        if (e.shiftKey && last?.side === side) for (let k = Math.min(last.i, i); k <= Math.max(last.i, i); k++) { const o = shown(side)[k]; if (o && !o.disabled) picked[side].add(o.value); }
        else toggle(i);
        last = { side, i };
        on[side] = i;
        draw();
      });
      box.addEventListener("dblclick", (e) => {
        const o = shown(side)[at(e.target)];
        if (o) move(side, [o.value]);
      });
      box.addEventListener("keydown", (e) => {
        const all = shown(side), v = all[on[side]]?.value;
        if (e.key === "ArrowDown" || e.key === "ArrowUp") {
          const d = e.key === "ArrowDown" ? 1 : -1;
          if (e.altKey) {
            if (side !== "to" || !cfg.reorderable || off() || v === undefined) return;
            const i = chosen().indexOf(v);
            reorder(v, i + d);
            on.to = K.clamp(on.to + d, 0, all.length - 1);
          } else {
            on[side] = K.clamp(on[side] + d, 0, Math.max(0, all.length - 1));
            if (e.shiftKey && all[on[side]] && !all[on[side]].disabled) picked[side].add(all[on[side]].value);
          }
        } else if (e.key === "Home") on[side] = 0;
        else if (e.key === "End") on[side] = all.length - 1;
        else if (e.key === " ") toggle(on[side]);
        else if (e.key === "Enter") move(side, picked[side].size ? [...picked[side]] : v === undefined ? [] : [v]);
        else if (e.key.toLowerCase() === "a" && (e.metaKey || e.ctrlKey)) all.forEach((o) => !o.disabled && picked[side].add(o.value));
        else return;
        e.preventDefault();
        draw();
        box.querySelector("[data-on]")?.scrollIntoView({ block: "nearest" });
      });
      box.addEventListener("dragstart", (e) => {
        const o = shown(side)[at(e.target)];
        if (!o) return;
        const vs = picked[side].has(o.value) ? [...picked[side]] : [o.value];
        e.dataTransfer.setData("text/plain", vs.join(", "));
        e.dataTransfer.effectAllowed = "move";
        drag = { side, vs };
      });
      box.addEventListener("dragover", (e) => {
        if (!drag || (drag.side === side && !(side === "to" && cfg.reorderable))) return;
        e.preventDefault();
        box.querySelectorAll("[data-drop]").forEach((n) => n.removeAttribute("data-drop"));
        e.target.closest?.("[role=option]")?.setAttribute("data-drop", "");
      });
      box.addEventListener("drop", (e) => {
        if (!drag) return;
        e.preventDefault();
        const i = at(e.target), over = shown(side)[i]?.value, d = drag;
        drag = null;
        if (d.side !== side) {
          move(d.side, d.vs);
          if (side === "to" && cfg.reorderable && over !== undefined) d.vs.forEach((v) => reorder(v, chosen().filter((x) => x !== v).indexOf(over)));
        } else if (side === "to" && cfg.reorderable) reorder(d.vs[0], over === undefined ? chosen().length : chosen().filter((x) => x !== d.vs[0]).indexOf(over));
        draw();
      });
    }
    let drag = null;
    el.addEventListener("dragend", () => ((drag = null), el.querySelectorAll("[data-drop]").forEach((n) => n.removeAttribute("data-drop"))));
    draw();
    return { update: draw };
  };

  // ---------------------------------------------------------------- tree select
  B["ui-tree-select"] = (el, api) => {
    const cfg = api.config, trigger = el.querySelector("[data-ts-button]"), pop = el.querySelector(":scope > [popover]"), clear = el.querySelector("[data-ts-clear]"), tree = pop.querySelector("[data-i-x=ui-tree]");
    const name = (n) => String(n.label ?? n.name ?? n.id ?? "");
    const kids = (n) => (Array.isArray(n.children) ? n.children : []);
    // Each node with those above it.
    function* walk(nodes, above = []) {
      for (const n of nodes ?? []) {
        if (!isObj(n)) continue;
        yield { n, above };
        yield* walk(kids(n), [...above, n]);
      }
    }
    const all = () => [...walk(K.value(api, cfg.nodes))];
    const label = ({ n, above }) => (cfg.path && above.length ? [make("span", { "data-ts-above": "" }, above.map(name).join(" / ") + " / "), make("span", null, name(n))] : [make("span", null, name(n))]);
    if (clear) clear.append(icon("x", 13));
    let was = null;
    function draw() {
      const v = api.state[cfg.bind], nodes = all(), sig = JSON.stringify([v, nodes.length, K.value(api, cfg.placeholder)]);
      if (sig === was) return;
      was = sig;
      const none = () => make("span", { "data-ts-none": "" }, K.value(api, cfg.placeholder) ?? L("Select…"));
      let out;
      if (cfg.multiple) {
        const ids = list(v).map(String), on = new Set(ids);
        // A node whose whole branch is chosen stands for it.
        const top = nodes.filter(({ n, above }) => on.has(String(n.id)) && !above.some((a) => on.has(String(a.id))));
        out = top.length ? [...top.slice(0, 4).map((x) => make("span", { "data-ts-tag": "" }, name(x.n))), top.length > 4 ? make("span", { "data-ts-tag": "" }, `+${top.length - 4}`) : null] : [none()];
      } else {
        const x = nodes.find(({ n }) => String(n.id) === String(v ?? ""));
        out = x ? label(x) : [none()];
      }
      trigger.replaceChildren(...out.filter(Boolean));
      if (clear) clear.hidden = trigger.disabled || (cfg.multiple ? !list(v).length : blank(v));
    }
    tree?.addEventListener("select", (e) => {
      if (cfg.multiple || !e.detail) return;
      if (cfg.leavesOnly) {
        if (kids(e.detail.node ?? {}).length) return;
        api.set(cfg.bind, e.detail.node?.id ?? e.detail.id);
      }
      api.emit("change", { value: api.state[cfg.bind], node: e.detail.node });
      pop.hidePopover();
      trigger.focus();
    });
    clear?.addEventListener("click", () => {
      api.set(cfg.bind, cfg.multiple ? [] : null);
      api.emit("change", { value: api.state[cfg.bind] });
      trigger.focus();
    });
    pop.addEventListener("toggle", (e) => {
      trigger.setAttribute("aria-expanded", String(e.newState === "open"));
      if (e.newState === "open") (pop.querySelector("input[type=search]") ?? pop.querySelector("[tabindex='0']"))?.focus();
    });
    trigger.addEventListener("keydown", (e) => e.key === "ArrowDown" && !pop.matches(":popover-open") && (e.preventDefault(), pop.showPopover()));
    draw();
    return { update: draw };
  };
})();
