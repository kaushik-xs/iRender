// irender-ui-kit: `ui-attachment`, `ui-sources`, `ui-tool-call`, `ui-trace` and
// `ui-questionnaire`.
(() => {
  const B = (window.__irenderBehaviors ||= {});
  const K = window.__irenderKit, L = (t, v) => window.__irender.label(t, v);
  const { make, button, icon, isObj, blank } = K;
  const safe = (u) => typeof u === "string" && u !== "" && !/^\s*(javascript|vbscript):/i.test(u);
  const web = (u) => safe(u) && !/^\s*data:/i.test(u);
  const asText = (v) => (typeof v === "string" ? v : JSON.stringify(v, null, 2));
  const bytes = (n) => {
    if (blank(n) || !(n >= 0)) return "";
    const u = ["B", "KB", "MB", "GB"];
    let i = 0;
    while (n >= 1024 && i < 3) (n /= 1024), i++;
    return `${n.toFixed(i && n < 10 ? 1 : 0)} ${u[i]}`;
  };
  const took = (ms) => {
    if (blank(ms) || isNaN(ms)) return "";
    const s = ms / 1000;
    return ms < 1000 ? `${Math.round(ms)} ms` : s < 60 ? `${s.toFixed(s < 10 ? 1 : 0)} s` : `${Math.floor(s / 60)} min ${Math.round(s % 60)} s`;
  };
  // Redrawn only when what it shows changes.
  const onChange = (f) => {
    let was = null;
    return (force) => {
      const v = f.read(), sig = JSON.stringify(v);
      if (sig === was && force !== true) return;
      was = sig;
      f.draw(v);
    };
  };
  const block = (title, v) => {
    const pre = make("pre", { tabindex: 0 });
    // Our own highlighter's HTML, from escaped text.
    if (typeof v !== "string" && window.__irenderCode) pre.innerHTML = window.__irenderCode.lines(asText(v), "json").join("\n");
    else pre.textContent = asText(v);
    return make("div", { "data-part": "" }, make("small", null, title), pre);
  };

  // ---------------------------------------------------------------- attachment
  B["ui-attachment"] = (el, api) => {
    const cfg = api.config;
    el.setAttribute("data-layout", cfg.layout ?? "chips");
    const draw = onChange({
      read: () => (K.value(api, cfg.files) ?? []).filter(isObj),
      draw(files) {
        el.hidden = !files.length;
        el.replaceChildren(make("ul", null, files.map((f, i) => {
          const st = f.status ?? "done", image = safe(f.thumb) ? f.thumb : /^image\//.test(f.type ?? "") && safe(f.url) ? f.url : null;
          const note = st === "error" ? f.error ?? L("Failed") : st === "uploading" ? L("Uploading… {progress}%", { progress: String(Math.round(f.progress ?? 0)) }) : st === "processing" ? L("Processing…") : f.text ?? bytes(f.size);
          const name = web(f.url) ? make("a", { "data-name": "", href: f.url, target: "_blank", rel: "noreferrer", title: f.name }, f.name ?? "") : make("span", { "data-name": "", title: f.name }, f.name ?? "");
          return make("li", { "data-status": st },
            make("span", { "data-thumb": "", "aria-hidden": "true" }, image ? make("img", { src: image, alt: "" }) : (/\.([a-z0-9]{1,4})$/i.exec(f.name ?? "")?.[1] ?? "file").toUpperCase()),
            make("span", { "data-text": "" }, name, note ? make("small", null, note) : null),
            cfg.bindFiles ? button({ "data-tool": "", "aria-label": L("Remove {name}", { name: f.name ?? "" }), click: () => {
              const list = (api.state[cfg.bindFiles] ?? []).filter((x) => x !== f && (f.id === undefined || x.id !== f.id));
              api.set(cfg.bindFiles, list);
              api.emit("remove", { file: f });
              (el.querySelectorAll("[data-tool]")[Math.min(i, list.length - 1)] ?? el).focus?.();
            } }, icon("x", 13)) : null,
            st === "uploading" ? make("span", { "data-bar": "", style: `width:${K.clamp(f.progress ?? 0, 0, 100)}%` }) : st === "processing" ? make("span", { "data-bar": "", "data-busy": "" }) : null);
        })));
      },
    });
    draw();
    return { update: draw };
  };

  // ---------------------------------------------------------------- sources
  B["ui-sources"] = (el, api) => {
    const cfg = api.config, layout = cfg.layout ?? "fold";
    let open = layout !== "fold", pop = null, timer;
    el.setAttribute("data-layout", layout);
    const host = (u) => {
      try {
        return new URL(u, location.href).hostname.replace(/^www\./, "");
      } catch {
        return "";
      }
    };
    const body = (s) => make("div", { "data-src": "" }, web(s.url) ? make("a", { href: s.url, target: "_blank", rel: "noreferrer" }, s.title ?? s.url) : make("b", null, s.title ?? ""), make("small", null, [s.site ?? (web(s.url) ? host(s.url) : null), s.page !== undefined ? L("page {n}", { n: String(s.page) }) : null, s.date].filter(Boolean).join(" · ")), s.text ? make("p", null, s.text) : null);
    function shut() {
      clearTimeout(timer);
      pop?.remove();
      pop = null;
    }
    function show(b, s) {
      shut();
      pop = make("div", { "data-pop": "", popover: "manual", role: "tooltip" }, body(s));
      el.append(pop);
      pop.showPopover?.();
      const r = b.getBoundingClientRect(), p = pop.getBoundingClientRect();
      pop.style.left = K.clamp(r.left + r.width / 2 - p.width / 2, 8, innerWidth - p.width - 8) + "px";
      pop.style.top = (r.top - p.height - 6 > 8 ? r.top - p.height - 6 : r.bottom + 6) + "px";
      pop.addEventListener("pointerenter", () => clearTimeout(timer));
      pop.addEventListener("pointerleave", shut);
    }
    const draw = onChange({
      read: () => [(K.value(api, cfg.items) ?? []).filter(isObj), K.value(api, cfg.text) ?? null, K.value(api, cfg.label) ?? null, open],
      draw([items, text, label]) {
        shut();
        const kids = [];
        if (text !== null) {
          kids.push(make("p", { "data-cited": "" }, String(text).split(/(\[\d{1,3}\])/).map((p, i) => {
            const n = i % 2 ? Number(p.slice(1, -1)) : 0, s = items[n - 1];
            if (!s) return p;
            const b = button({ "data-cite": n, "aria-label": L("Source {n}: {name}", { n: String(n), name: s.title ?? s.url ?? "" }) }, String(n));
            b.addEventListener("pointerenter", (e) => e.pointerType === "mouse" && show(b, s));
            b.addEventListener("pointerleave", () => (timer = setTimeout(shut, 200)));
            b.addEventListener("focus", () => b.matches(":focus-visible") && show(b, s));
            b.addEventListener("blur", shut);
            b.addEventListener("click", () => {
              api.emit("cite", { index: n - 1, source: s });
              if (!open) (open = true), draw();
              el.querySelectorAll("[data-hit]").forEach((x) => x.removeAttribute("data-hit"));
              const li = el.querySelectorAll("ol > li")[n - 1];
              li?.setAttribute("data-hit", "");
              li?.scrollIntoView({ block: "nearest" });
            });
            return b;
          })));
        }
        if (items.length) {
          const name = label ?? L("Sources");
          if (layout === "fold") kids.push(button({ "data-fold": "", "aria-expanded": String(open), click: () => ((open = !open), draw(), el.querySelector("[data-fold]").focus()) }, icon("right", 12), `${name} (${items.length})`));
          if (open) kids.push(make("ol", { "aria-label": name }, items.map((s, i) => make("li", null, make("i", { "aria-hidden": "true" }, i + 1), body(s)))));
        }
        el.replaceChildren(...kids);
      },
    });
    el.addEventListener("keydown", (e) => e.key === "Escape" && pop && (e.stopPropagation(), shut()));
    draw();
    return { update: draw };
  };

  // ---------------------------------------------------------------- tool call
  B["ui-tool-call"] = (el, api) => {
    const cfg = api.config, id = `tc-${K.id()}`;
    let open = cfg.open, asked = null, answered = null;
    // Labels: "Waiting", "Running", "Done", "Failed", "Needs approval"
    const NAMES = { pending: "Waiting", running: "Running", done: "Done", error: "Failed", approval: "Needs approval" };
    const draw = onChange({
      read: () => ["title", "status", "name", "input", "output", "error", "duration"].map((k) => K.value(api, cfg[k]) ?? null).concat(open, answered),
      draw([title, status, name, input, output, error, duration]) {
        const st = NAMES[status] ? status : error ? "error" : "done";
        if (st === "approval" && asked !== st) open = true;
        asked = st;
        el.setAttribute("data-status", st);
        const parts = [input !== null ? block(L("Input"), input) : null, output !== null ? block(L("Output"), output) : null, error !== null ? make("p", { "data-error": "" }, asText(error)) : null].filter(Boolean);
        const ask = st === "approval" && answered === null;
        if (ask) {
          const answer = (yes) => () => {
            answered = yes;
            if (cfg.bindApproval) api.set(cfg.bindApproval, yes);
            api.emit(yes ? "approve" : "deny", { title, name, input });
            draw(true);
          };
          parts.push(make("div", { "data-ask": "" }, make("span", null, L("Allow this?")), button({ click: answer(false) }, L("Deny")), button({ "data-yes": "", click: answer(true) }, L("Allow"))));
        }
        const word = st === "approval" && answered !== null ? (answered ? L("Allowed") : L("Denied")) : L(NAMES[st]);
        const head = button({ "data-head": "", "aria-expanded": parts.length ? String(open) : null, "aria-controls": parts.length ? id : null, disabled: !parts.length, click: () => ((open = !open), draw(), el.querySelector("[data-head]").focus()) },
          make("span", { "data-mark": "", role: "img", "aria-label": word }, st === "done" ? icon("check", 12) : st === "error" ? icon("x", 11) : st === "approval" ? "!" : null), make("b", null, title ?? ""), name ? make("code", null, name) : null, make("small", null, [st === "done" || st === "error" ? null : word, took(duration)].filter(Boolean).join(" · ")), parts.length ? make("span", { "data-chevron": "" }, icon("right", 13)) : null);
        el.replaceChildren(...[head, open && parts.length ? make("div", { "data-body": "", id }, parts) : null].filter(Boolean));
      },
    });
    draw();
    return { update: draw };
  };

  // ---------------------------------------------------------------- trace
  B["ui-trace"] = (el, api) => {
    const cfg = api.config;
    const HUES = ["var(--color-primary)", "#7c3aed", "#0891b2", "#d97706", "#16a34a", "#db2777", "#64748b"];
    let shut = new Set(), seeded = false, picked = null, focusId = null;
    const color = (c) => (typeof c === "string" && /^[#\w(),.%\s/-]+$/.test(c) ? c : null);
    const draw = onChange({
      read: () => [(K.value(api, cfg.spans) ?? []).filter(isObj), K.value(api, cfg.title) ?? null, [...shut], picked],
      draw([spans, title]) {
        const ids = new Set(spans.map((s) => String(s.id))), kids = new Map();
        for (const s of spans) {
          const p = s.parent != null && ids.has(String(s.parent)) && String(s.parent) !== String(s.id) ? String(s.parent) : "";
          kids.set(p, [...(kids.get(p) ?? []), s]);
        }
        for (const l of kids.values()) l.sort((a, b) => (a.start ?? 0) - (b.start ?? 0));
        const from = Math.min(...spans.map((s) => Number(s.start) || 0)), to = Math.max(...spans.map((s) => (Number(s.start) || 0) + (Number(s.duration) || 0))), all = Math.max(1, to - from);
        const names = [...new Set(spans.map((s) => s.kind).filter(Boolean))];
        const lines = [], seen = new Set();
        const walk = (p, d, up) => (kids.get(p) ?? []).forEach((s, i, l) => {
          const id = String(s.id), own = kids.get(id) ?? [];
          if (seen.has(id)) return; // a loop
          seen.add(id);
          if (!seeded && cfg.expanded != null && cfg.expanded !== true && d >= Number(cfg.expanded) && own.length) shut.add(id);
          lines.push({ s, id, d, n: own.length, up, i, of: l.length });
          if (!shut.has(id)) walk(id, d + 1, id);
        });
        walk("", 0, null);
        seeded = true;
        const total = (k) => spans.reduce((a, s) => a + (Number(s[k]) || 0), 0);
        const head = make("div", { "data-head": "" }, make("b", null, title ?? L("Trace")), make("span", null, took(all)), make("span", null, L(["{n} step", "{n} steps"], { n: spans.length })), total("tokens") ? make("span", null, L("{n} tokens", { n: total("tokens") })) : null, total("cost") ? make("span", null, K.format(total("cost"), { currency: "USD", decimals: 4, locale: cfg.locale })) : null);
        const hadFocus = el.contains(document.activeElement) && document.activeElement.matches("[role=treeitem]");
        if (!lines.some((x) => x.id === focusId)) focusId = lines[0]?.id ?? null;
        const tree = make("div", { role: "tree", "aria-label": title ?? L("Trace") }, lines.map((x) => {
          const s = x.s, open = !shut.has(x.id), c = color(cfg.kinds?.[s.kind]) ?? HUES[Math.max(0, names.indexOf(s.kind)) % HUES.length];
          return make("div", { role: "treeitem", "data-id": x.id, "data-up": x.up, "aria-level": x.d + 1, "aria-setsize": x.of, "aria-posinset": x.i + 1, "aria-expanded": x.n ? String(open) : null, "aria-selected": String(picked === x.id), tabindex: x.id === focusId ? 0 : -1, "data-status": s.status ?? null, style: `--d:${x.d}` },
            make("span", { "data-name": "" }, make("span", { "data-twist": "", style: x.n ? null : "visibility:hidden" }, icon("right", 12)), make("span", { title: s.name }, s.name ?? x.id)),
            make("span", { "data-lane": "", "aria-hidden": "true" }, make("span", { "data-span": "", "data-status": s.status ?? null, style: `${s.status === "error" ? "" : `--c:${c};`}inset-inline-start:${(((Number(s.start) || 0) - from) / all) * 100}%;width:${((Number(s.duration) || 0) / all) * 100}%` })),
            make("span", { "data-took": "" }, took(s.duration)));
        }));
        const s = spans.find((x) => String(x.id) === picked);
        const row = (k, v) => (v === null || v === undefined || v === "" ? [] : [make("dt", null, k), make("dd", null, typeof v === "object" ? make("pre", null, asText(v)) : String(v).includes("\n") ? make("pre", null, v) : String(v))]);
        const detail = s ? make("div", { "data-detail": "" }, make("dl", null, row(L("Step"), s.name), row(L("Kind"), s.kind), row(L("Took"), took(s.duration)), row(L("Tokens"), s.tokens), row(L("Cost"), s.cost), row(L("Input"), s.input), row(L("Output"), s.output), row(L("Error"), s.error))) : null;
        el.replaceChildren(...[head, tree, detail].filter(Boolean));
        if (hadFocus) tree.querySelector("[tabindex='0']")?.focus({ preventScroll: true });
      },
    });
    const pick = (id) => {
      picked = picked === id ? null : id;
      const s = (K.value(api, cfg.spans) ?? []).find((x) => String(x.id) === id);
      if (cfg.bindSpan) api.set(cfg.bindSpan, picked === null ? null : s);
      api.emit("span", { span: s });
      draw();
    };
    el.addEventListener("click", (e) => {
      const t = e.target.closest("[role=treeitem]");
      if (!t) return;
      focusId = t.getAttribute("data-id");
      if (e.target.closest("[data-twist]")) shut.has(focusId) ? shut.delete(focusId) : shut.add(focusId), draw();
      else pick(focusId);
    });
    el.addEventListener("keydown", (e) => {
      const t = e.target.closest?.("[role=treeitem]");
      if (!t) return;
      const all = [...el.querySelectorAll("[role=treeitem]")], i = all.indexOf(t), id = t.getAttribute("data-id"), k = K.rtl(el) ? { ArrowLeft: "ArrowRight", ArrowRight: "ArrowLeft" }[e.key] ?? e.key : e.key;
      const go = (n) => n && ((focusId = n.getAttribute("data-id")), all.forEach((x) => (x.tabIndex = x === n ? 0 : -1)), n.focus());
      if (k === "ArrowDown") go(all[i + 1]);
      else if (k === "ArrowUp") go(all[i - 1]);
      else if (k === "Home") go(all[0]);
      else if (k === "End") go(all.at(-1));
      else if (k === "ArrowRight") t.getAttribute("aria-expanded") === "false" ? (shut.delete(id), draw()) : t.getAttribute("aria-expanded") === "true" && go(all[i + 1]);
      else if (k === "ArrowLeft") t.getAttribute("aria-expanded") === "true" ? (shut.add(id), draw()) : go(all.find((x) => x.getAttribute("data-id") === t.getAttribute("data-up")));
      else if (k === "Enter" || k === " ") pick(id);
      else return;
      e.preventDefault();
    });
    draw();
    return { update: draw };
  };

  // ---------------------------------------------------------------- questionnaire
  B["ui-questionnaire"] = (el, api) => {
    const cfg = api.config, steps = cfg.mode !== "all", uid = `qn-${K.id()}`;
    let at = 0, tried = false, done = false;
    const questions = () => (K.value(api, cfg.questions) ?? []).filter(isObj).map((q, i) => ({ type: q.options ? "choice" : "text", ...q, id: String(q.id ?? i) }));
    const answers = () => (isObj(api.state[cfg.bind]) ? api.state[cfg.bind] : {});
    const choice = (o) => (isObj(o) ? o : { value: o, label: String(o) });
    const has = (v) => !(blank(v) || (Array.isArray(v) && !v.length));
    const set = (id, v) => {
      const next = { ...answers(), [id]: v };
      if (!has(v)) delete next[id];
      api.set(cfg.bind, next);
      api.emit("answer", { id, value: v, answers: next });
    };
    function wrong(q) {
      const v = answers()[q.id];
      if (!has(v)) return q.required ? L("Please answer this one") : null;
      if (q.type === "number") {
        if (typeof v !== "number") return L("Not a number");
        if (q.min != null && v < q.min) return L("At least {n}", { n: q.min });
        if (q.max != null && v > q.max) return L("At most {n}", { n: q.max });
      }
      if (q.type === "choices" && q.max != null && v.length > q.max) return L("Pick at most {n}", { n: q.max });
      return null;
    }
    const said = (q) => {
      const v = answers()[q.id], label = (x) => (q.options ?? []).map(choice).find((o) => o.value === x)?.label ?? (typeof x === "boolean" ? (x ? L("Yes") : L("No")) : String(x));
      return !has(v) ? "—" : Array.isArray(v) ? v.map(label).join(", ") : label(v);
    };
    function field(q) {
      const v = answers()[q.id], name = `${uid}-${q.id}`, out = [];
      const type = q.type === "yesno" ? "choice" : q.type;
      const opts = q.type === "yesno" ? [{ value: true, label: L("Yes") }, { value: false, label: L("No") }] : (q.options ?? []).map(choice);
      if (type === "choice" || type === "choices") {
        const many = type === "choices", list = many ? (Array.isArray(v) ? v : []) : [v];
        const isOther = (x) => has(x) && !opts.some((o) => o.value === x), other = list.find(isOther);
        const change = (val, on) => {
          if (!many) return set(q.id, val), steps && !q.other && at < questions().length - 1 && setTimeout(() => move(1), 180);
          set(q.id, on ? [...list.filter((x) => x !== val), val] : list.filter((x) => x !== val));
        };
        out.push(...opts.map((o) => make("label", { "data-opt": "" }, Object.assign(make("input", { type: many ? "checkbox" : "radio", name, "data-key": `${q.id}:${o.value}`, change: (e) => change(o.value, e.target.checked) }), { checked: list.includes(o.value) }), make("span", null, o.label, o.description ? make("small", null, o.description) : null))));
        if (q.other) {
          const t = make("input", { type: "text", "data-key": `${q.id}:other`, placeholder: typeof q.other === "string" ? q.other : L("Something else…"), "aria-label": L("Something else") });
          t.value = other ?? "";
          t.addEventListener("input", () => {
            const x = t.value.trim() === "" ? null : t.value;
            set(q.id, many ? [...list.filter((y) => !isOther(y)), ...(x === null ? [] : [x])] : x);
          });
          out.push(t);
        }
      } else if (type === "scale") {
        const lo = Number(q.min ?? 1), hi = Number(q.max ?? 5);
        out.push(make("div", { "data-scale": "" }, Array.from({ length: K.clamp(hi - lo + 1, 1, 21) }, (_, i) => make("label", null, Object.assign(make("input", { type: "radio", name, "data-key": `${q.id}:${lo + i}`, "aria-label": String(lo + i), change: () => { set(q.id, lo + i); if (steps && at < questions().length - 1) setTimeout(() => move(1), 180); } }), { checked: v === lo + i }), String(lo + i)))));
        if (Array.isArray(q.labels)) out.push(make("div", { "data-ends": "" }, make("span", null, q.labels[0] ?? ""), make("span", null, q.labels[1] ?? "")));
      } else {
        const t = make(type === "text" ? "textarea" : "input", { type: type === "text" ? null : "text", rows: type === "text" ? 3 : null, inputmode: type === "number" ? "decimal" : null, "data-key": q.id, placeholder: q.placeholder, "aria-label": q.question, "aria-required": q.required ? "true" : null });
        t.value = blank(v) ? "" : String(v);
        t.addEventListener("input", () => set(q.id, t.value === "" ? null : type === "number" ? (isNaN(t.value) || t.value.trim() === "" ? t.value : Number(t.value)) : t.value));
        out.push(t);
      }
      const m = tried ? wrong(q) : null;
      return make("fieldset", { "data-q": q.id, "aria-describedby": m ? `${uid}-${q.id}-e` : null }, make("legend", null, q.question ?? "", q.required ? make("span", { "data-req": "", "aria-label": L("Required") }, " *") : null), q.description ? make("p", { "data-q-more": "" }, q.description) : null, out, m ? make("p", { "data-error": "", id: `${uid}-${q.id}-e`, role: "alert" }, m) : null);
    }
    function move(d) {
      const list = questions();
      if (d > 0 && wrong(list[at])) {
        tried = true;
        draw(true);
        return el.querySelector("input, textarea")?.focus();
      }
      tried = false;
      at = K.clamp(at + d, 0, list.length - 1);
      draw(true);
      el.querySelector("fieldset :is(input:checked, input, textarea)")?.focus();
    }
    function submit() {
      const list = questions(), bad = list.findIndex(wrong);
      if (bad >= 0) {
        tried = true;
        if (steps) at = bad;
        draw(true);
        el.querySelector(`[data-q="${CSS.escape(list[bad].id)}"] :is(input, textarea)`)?.focus();
        return;
      }
      done = true;
      if (cfg.bindDone) api.set(cfg.bindDone, true);
      api.emit("submit", { answers: answers() });
      draw(true);
      K.say(el, L("Sent"));
    }
    const draw = onChange({
      read: () => [questions(), answers(), K.value(api, cfg.title) ?? null, K.value(api, cfg.submitLabel) ?? null, K.value(api, cfg.doneText) ?? null, cfg.bindDone ? !!api.state[cfg.bindDone] : done, at],
      draw([list, , title, submitLabel, doneText, isDone]) {
        done = isDone;
        at = K.clamp(at, 0, Math.max(0, list.length - 1));
        const live = el.querySelector(":scope > [data-kit-live]");
        // What's being typed in keeps its place.
        const f = el.contains(document.activeElement) ? document.activeElement : null, key = f?.getAttribute("data-key"), caret = f?.matches("textarea, input[type=text]") ? [f.selectionStart, f.selectionEnd, f.value] : null;
        if (isDone) {
          el.replaceChildren(...[title ? make("div", { "data-q-title": "" }, title) : null, doneText ? make("p", { style: "margin:0" }, doneText) : null, make("dl", null, list.map((q) => [make("dt", null, q.question ?? ""), make("dd", null, said(q))])), live].filter(Boolean));
          return;
        }
        const last = !steps || at === list.length - 1, q = list[at];
        const foot = make("div", { "data-foot": "" }, steps && at > 0 ? button({ click: () => move(-1) }, L("Back")) : null, make("span", { "data-push": "" }), steps && !last && q && !q.required && cfg.skippable && !has(answers()[q.id]) ? button({ click: () => move(1) }, L("Skip")) : null, last ? button({ "data-primary": "", click: submit }, submitLabel ?? L("Send")) : button({ "data-primary": "", click: () => move(1) }, L("Next")));
        el.replaceChildren(...[
          title || (steps && list.length > 1) ? make("div", { "data-q-title": "" }, title ?? "", steps && list.length > 1 ? make("small", null, L("{n} of {count}", { n: at + 1, count: list.length })) : null) : null,
          steps && list.length > 1 ? make("progress", { max: list.length, value: at + (q && has(answers()[q.id]) ? 1 : 0), "aria-label": L("Progress") }) : null,
          ...(steps ? (q ? [field(q)] : []) : list.map(field)), list.length ? foot : null, live].filter(Boolean));
        const back = key && el.querySelector(`[data-key="${CSS.escape(key)}"]`);
        if (back) {
          if (caret) back.value = caret[2];
          back.focus();
          if (caret) back.setSelectionRange(caret[0], caret[1]);
        }
      },
    });
    el.addEventListener("keydown", (e) => {
      if (e.key === "Enter" && !e.isComposing && e.target.matches("input") && !e.shiftKey) e.preventDefault(), steps && at < questions().length - 1 ? move(1) : submit();
    });
    draw();
    return { update: draw };
  };
})();
