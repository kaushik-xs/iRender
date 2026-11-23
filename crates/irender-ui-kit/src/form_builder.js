// irender-ui-kit: the `ui-form-builder` behaviour. It draws the builder from the fields
// in state (`bind`) and writes them back as they change, with the form as an iRender
// document (`bindDocument`). Build: a palette of field types (clicked, or dragged onto
// the fields), the fields in order (dragged by their grips, or moved with the arrow keys
// on a grip), and an inspector for the one picked. Preview: the form to try, its
// required, email and limit checks included. Document: the form as JSON or YAML, ready
// to copy into a page. Everything it writes is text, never HTML.
(window.__irenderBehaviors ||= {})["ui-form-builder"] = (el, api) => {
  const cfg = api.config;
  const L = (text, vars) => window.__irender.label(text, vars);
  const isObj = (v) => v !== null && typeof v === "object" && !Array.isArray(v);
  const $ = (s) => el.querySelector(s);
  const live = $("[data-fb-live]");
  const say = (msg) => live && (live.textContent = msg);
  const h = (tag, attrs = {}, ...kids) => {
    const e = document.createElement(tag);
    for (const [k, v] of Object.entries(attrs)) {
      if (v === null || v === undefined || v === false) continue;
      if (k.startsWith("on")) e.addEventListener(k.slice(2), v);
      else e.setAttribute(k, v === true ? "" : v);
    }
    e.append(...kids.filter((k) => k !== null && k !== undefined && k !== false));
    return e;
  };

  // Labels: "Short text", "Long text", "Email", "Number", "Date", "Dropdown", "One choice", "Several choices", "Yes or no", "Section"
  const KINDS = {
    text: ["Short text", "Aa"], textarea: ["Long text", "¶"], email: ["Email", "@"], number: ["Number", "#"], date: ["Date", "31"],
    select: ["Dropdown", "▾"], radio: ["One choice", "◉"], checkboxes: ["Several choices", "☑"], checkbox: ["Yes or no", "✓"], section: ["Section", "§"],
  };
  const kindName = (t) => L(KINDS[t]?.[0] ?? t);
  const CHOICES = new Set(["select", "radio", "checkboxes"]);
  const PLACEHOLDER = new Set(["text", "textarea", "email", "number", "select"]);
  const LIMITS = new Set(["number", "date"]);
  // State keys the generated document keeps for itself.
  const RESERVED = new Set(["errors", "submitted"]);

  // ---- the fields
  const fields = () => (Array.isArray(api.state[cfg.bind]) ? api.state[cfg.bind].filter(isObj) : []);
  const slug = (s) => {
    let k = String(s ?? "").normalize("NFKD").replace(/[̀-ͯ]/g, "").toLowerCase().replace(/[^a-z0-9]+/g, "_").replace(/^_+|_+$/g, "");
    if (!k) k = "field";
    if (/^\d/.test(k)) k = `f_${k}`;
    return k.slice(0, 40);
  };
  // `base`, or `base_2`, `base_3`, … : one no other field (but `self`) has.
  function unique(base, self, list = fields()) {
    const taken = new Set(list.filter((f) => f !== self).map((f) => f.id));
    let k = base, n = 1;
    while (taken.has(k) || RESERVED.has(k)) k = `${base}_${++n}`;
    return k;
  }
  function make(type, list) {
    const label = type === "section" ? L("New section") : kindName(type);
    const f = { id: "", type, label };
    f.id = unique(slug(label), f, list);
    if (CHOICES.has(type)) f.options = [L("Option 1"), L("Option 2"), L("Option 3")];
    return f;
  }

  let selected = null, tab = "build", written;
  // The fields and the inspector, after a change that leaves the palette as it is.
  const redraw = () => {
    drawCanvas();
    drawInspector();
  };
  function write(next, also) {
    written = JSON.stringify(next);
    api.set(cfg.bind, next);
    publish(next);
    also?.();
    api.emit("change", { value: next });
  }
  const at = (id) => fields().findIndex((f) => f.id === id);
  // The field `id` changed by `patch`; the inspector isn't drawn again (it has focus).
  function patch(id, p) {
    const list = fields().map((f) => (f.id === id ? { ...f, ...p } : f));
    for (const f of list) for (const k of Object.keys(f)) if (f[k] === undefined || f[k] === "") delete f[k];
    write(list, () => {
      drawCanvas();
      if (tab === "preview") drawPreview();
    });
  }
  function add(type, index) {
    const list = fields().slice(), f = make(type, list);
    list.splice(index ?? (selected !== null && at(selected) >= 0 ? at(selected) + 1 : list.length), 0, f);
    selected = f.id;
    write(list, redraw);
    say(L("{type} added: {label}", { type: kindName(type), label: f.label }));
    return f;
  }
  function move(id, to) {
    const list = fields().slice(), from = list.findIndex((f) => f.id === id);
    to = Math.max(0, Math.min(list.length - 1, to));
    if (from < 0 || from === to) return;
    const [f] = list.splice(from, 1);
    list.splice(to, 0, f);
    write(list, redraw);
    say(L("{name} moved to {n} of {total}", { name: f.label, n: to + 1, total: list.length }));
    el.querySelector(`[data-fb-card="${CSS.escape(id)}"] [data-fb-grip]`)?.focus();
  }
  function remove(id) {
    const list = fields(), i = list.findIndex((f) => f.id === id);
    if (i < 0) return;
    const next = list.filter((f) => f.id !== id), gone = list[i];
    selected = next[Math.min(i, next.length - 1)]?.id ?? null;
    write(next, redraw);
    say(L("{name} deleted", { name: gone.label }));
    (selected !== null ? el.querySelector(`[data-fb-card="${CSS.escape(selected)}"] [data-fb-pick]`) : el.querySelector("[data-fb-palette] button"))?.focus();
  }
  function duplicate(id) {
    const list = fields().slice(), i = list.findIndex((f) => f.id === id);
    if (i < 0) return;
    const copy = { ...structuredClone(list[i]), label: L("{label} (copy)", { label: list[i].label }) };
    copy.id = unique(slug(copy.label), copy, list);
    list.splice(i + 1, 0, copy);
    selected = copy.id;
    write(list, redraw);
    say(L("{name} added", { name: copy.label }));
    el.querySelector(`[data-fb-card="${CSS.escape(copy.id)}"] [data-fb-pick]`)?.focus();
  }

  // ---- build: the palette
  const panel = (k) => el.querySelector(`[data-fb-panel="${k}"]`);
  function drawBuild() {
    const wrap = h("div", { "data-fb-build": true });
    const palette = h("div", { "data-fb-palette": true, role: "group", "aria-label": L("Add a field") });
    for (const t of cfg.types) {
      palette.append(h("button", { type: "button", draggable: "true", "data-fb-type": t, title: L("Add {type}, or drag it onto the form", { type: kindName(t) }) }, h("span", { "data-fb-glyph": true, "aria-hidden": "true" }, KINDS[t][1]), kindName(t)));
    }
    wrap.append(h("div", {}, h("p", { "data-fb-heading": true }, L("Add a field")), palette));
    wrap.append(h("div", {}, h("p", { "data-fb-heading": true, id: `${cfg.id}-fields-title` }, L("The form")), h("div", { "data-fb-canvas": true, role: "list", "aria-labelledby": `${cfg.id}-fields-title` })));
    wrap.append(h("div", {}, h("p", { "data-fb-heading": true }, L("Field settings")), h("div", { "data-fb-inspector": true })));
    wrap.append(h("span", { id: `${cfg.id}-move-help`, hidden: true }, L("The arrow keys move it; drag it with a mouse or a finger.")));
    panel("build").replaceChildren(wrap);
    drawCanvas();
    drawInspector();
  }

  // ---- build: the fields, in order
  function drawCanvas() {
    const canvas = $("[data-fb-canvas]");
    if (!canvas) return;
    const list = fields();
    if (selected !== null && !list.some((f) => f.id === selected)) selected = null;
    const focused = document.activeElement?.closest?.("[data-fb-card]") && canvas.contains(document.activeElement) ? [document.activeElement.closest("[data-fb-card]").dataset.fbCard, document.activeElement.hasAttribute("data-fb-grip") ? "grip" : document.activeElement.hasAttribute("data-fb-pick") ? "pick" : null] : null;
    const cards = list.map((f) => {
      const on = f.id === selected;
      const meta = [kindName(f.type), f.type === "section" ? null : f.id].filter(Boolean).join(" · ");
      return h("div", { "data-fb-card": f.id, role: "listitem", "aria-current": on ? "true" : null, "data-section": f.type === "section" || null },
        h("button", { type: "button", "data-fb-grip": true, "aria-label": L("Move {name}", { name: f.label }), "aria-describedby": `${cfg.id}-move-help` }, "⠿"),
        h("button", { type: "button", "data-fb-pick": true, "aria-pressed": on ? "true" : "false" },
          h("span", { "data-fb-glyph": true, "aria-hidden": "true" }, KINDS[f.type]?.[1] ?? "?"),
          h("span", {}, f.label || L("Untitled")),
          f.required ? h("span", { "data-fb-required": true, "aria-label": L("required") }, "*") : null,
          h("span", { "data-fb-meta": true }, meta)),
        on ? h("button", { type: "button", "data-fb-duplicate": true, "aria-label": L("Duplicate {name}", { name: f.label }), title: L("Duplicate") }, "⧉") : null,
        on ? h("button", { type: "button", "data-fb-delete": true, "aria-label": L("Delete {name}", { name: f.label }), title: L("Delete") }, "✕") : null);
    });
    canvas.replaceChildren(...(cards.length ? cards : [h("p", { "data-fb-empty": true }, L("Add a field from the list, or drag one here."))]));
    if (focused) el.querySelector(`[data-fb-card="${CSS.escape(focused[0])}"] [data-fb-${focused[1] ?? "pick"}]`)?.focus();
  }

  // ---- build: the inspector, for the field picked
  function drawInspector() {
    const box = $("[data-fb-inspector]");
    if (!box) return;
    const f = fields().find((x) => x.id === selected);
    if (!f) return box.replaceChildren(h("p", { "data-fb-help": true }, L("Pick a field to set it up.")));
    const n = (k) => `${cfg.id}-in-${k}`;
    const field = (k, label, control, help) => h("div", { "data-fb-field": true }, h("label", { for: n(k) }, label), control, help ? h("span", { "data-fb-help": true, id: `${n(k)}-help` }, help) : null);
    const text = (k, value, attrs = {}) => h("input", { id: n(k), type: "text", value: value ?? "", autocomplete: "off", ...attrs });
    const section = f.type === "section";
    const items = [h("p", { "data-fb-meta": true }, kindName(f.type))];
    const label = text("label", f.label);
    label.addEventListener("input", () => {
      const cur = fields().find((x) => x.id === selected);
      if (!cur) return;
      // A key that came from the label follows it; one typed by hand stays.
      const follows = cur.type !== "section" ? cur.id === unique(slug(cur.label), cur) : true;
      const id = follows ? unique(slug(label.value), cur) : cur.id;
      selected = id;
      patch(cur.id, { label: label.value, id });
      const keyIn = $(`#${CSS.escape(n("key"))}`);
      if (keyIn && document.activeElement !== keyIn) keyIn.value = id;
    });
    items.push(field("label", section ? L("Title") : L("Label"), label));
    if (!section) {
      const key = text("key", f.id, { "aria-describedby": `${n("key")}-help`, spellcheck: "false" });
      key.addEventListener("change", () => {
        const cur = fields().find((x) => x.id === selected);
        if (!cur) return;
        const id = unique(slug(key.value || cur.label), cur);
        key.value = id;
        selected = id;
        patch(cur.id, { id });
      });
      items.push(field("key", L("Key"), key, L("The answer is kept under this name.")));
    }
    const desc = section ? h("textarea", { id: n("desc"), rows: "2" }) : text("desc", f.description);
    if (section) desc.value = f.description ?? "";
    desc.addEventListener("input", () => patch(selected, { description: desc.value }));
    items.push(field("desc", section ? L("Description") : L("Help text"), desc));
    if (PLACEHOLDER.has(f.type)) {
      const ph = text("ph", f.placeholder);
      ph.addEventListener("input", () => patch(selected, { placeholder: ph.value }));
      items.push(field("ph", L("Placeholder"), ph));
    }
    if (CHOICES.has(f.type)) {
      const opts = h("textarea", { id: n("opts"), rows: "4", "aria-describedby": `${n("opts")}-help` });
      opts.value = (f.options ?? []).join("\n");
      opts.addEventListener("input", () => patch(selected, { options: opts.value.split("\n").map((s) => s.trim()).filter(Boolean) }));
      items.push(field("opts", L("Choices"), opts, L("One on each line.")));
    }
    if (LIMITS.has(f.type)) {
      const type = f.type === "date" ? "date" : "number";
      const lo = h("input", { id: n("min"), type, value: f.min ?? "" }), hi = h("input", { id: n("max"), type, value: f.max ?? "" });
      const read = (i) => (i.value === "" ? undefined : type === "number" ? Number(i.value) : i.value);
      lo.addEventListener("input", () => patch(selected, { min: read(lo) }));
      hi.addEventListener("input", () => patch(selected, { max: read(hi) }));
      items.push(h("div", { "data-fb-row": true }, field("min", f.type === "date" ? L("Earliest") : L("Lowest"), lo), field("max", f.type === "date" ? L("Latest") : L("Highest"), hi)));
    }
    if (!section) {
      const req = h("input", { type: "checkbox", id: n("req"), checked: !!f.required });
      req.addEventListener("change", () => patch(selected, { required: req.checked || undefined }));
      items.push(h("label", { "data-fb-check": true }, req, f.type === "checkbox" ? L("Must be ticked") : L("Required")));
    }
    box.replaceChildren(...items);
  }

  // ---- build: picking, moving, adding and deleting
  el.addEventListener("click", (e) => {
    const b = e.target.closest?.("button");
    if (!b || !el.contains(b)) return;
    if (b.hasAttribute("data-fb-type")) return add(b.getAttribute("data-fb-type"));
    if (b.hasAttribute("data-fb-tab")) return show(b.getAttribute("data-fb-tab"), true);
    const card = b.closest("[data-fb-card]");
    if (!card) return;
    const id = card.getAttribute("data-fb-card");
    if (b.hasAttribute("data-fb-pick")) {
      selected = id;
      drawCanvas();
      drawInspector();
      el.querySelector(`[data-fb-card="${CSS.escape(id)}"] [data-fb-pick]`)?.focus();
    } else if (b.hasAttribute("data-fb-delete")) remove(id);
    else if (b.hasAttribute("data-fb-duplicate")) duplicate(id);
  });
  el.addEventListener("keydown", (e) => {
    const t = e.target;
    // The tabs: arrows go along them, as tabs do.
    if (t.matches?.("[data-fb-tab]") && (e.key === "ArrowRight" || e.key === "ArrowLeft" || e.key === "Home" || e.key === "End")) {
      e.preventDefault();
      const all = [...el.querySelectorAll("[data-fb-tab]")], i = all.indexOf(t);
      const next = e.key === "Home" ? 0 : e.key === "End" ? all.length - 1 : (i + (e.key === "ArrowRight" ? 1 : -1) + all.length) % all.length;
      return show(all[next].getAttribute("data-fb-tab"), true);
    }
    const card = t.closest?.("[data-fb-card]");
    if (!card) return;
    const id = card.getAttribute("data-fb-card");
    const step = { ArrowUp: -1, ArrowDown: 1 }[e.key];
    if (t.hasAttribute("data-fb-grip") && step) {
      e.preventDefault();
      move(id, at(id) + step);
    } else if (t.hasAttribute("data-fb-pick") && (e.key === "Delete" || e.key === "Backspace")) {
      e.preventDefault();
      remove(id);
    } else if (t.hasAttribute("data-fb-pick") && step) {
      e.preventDefault();
      const cards = [...el.querySelectorAll("[data-fb-card]")], i = cards.indexOf(card);
      cards[i + step]?.querySelector("[data-fb-pick]")?.focus();
    }
  });

  // Where something dropped at `y` lands among the cards (but `skip`), and the card to mark.
  function target(y, skip) {
    const cards = [...el.querySelectorAll("[data-fb-card]")].filter((c) => c.getAttribute("data-fb-card") !== skip);
    let to = cards.length;
    for (const [i, c] of cards.entries()) {
      const r = c.getBoundingClientRect();
      if (y < r.top + r.height / 2) {
        to = i;
        break;
      }
    }
    return { to, mark: to < cards.length ? [cards[to], "before"] : cards.length ? [cards.at(-1), "after"] : null };
  }
  const unmark = () => el.querySelectorAll("[data-drop]").forEach((c) => c.removeAttribute("data-drop"));

  // A type dragged from the palette (with a mouse) onto the fields.
  el.addEventListener("dragstart", (e) => {
    const b = e.target.closest?.("[data-fb-type]");
    if (!b) return;
    e.dataTransfer.setData("application/x-irender-field", b.getAttribute("data-fb-type"));
    e.dataTransfer.effectAllowed = "copy";
  });
  el.addEventListener("dragover", (e) => {
    const canvas = e.target.closest?.("[data-fb-canvas]");
    if (!canvas || !e.dataTransfer.types.includes("application/x-irender-field")) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
    canvas.setAttribute("data-fb-over", "");
    unmark();
    target(e.clientY).mark?.[0].setAttribute("data-drop", target(e.clientY).mark[1]);
  });
  el.addEventListener("dragleave", (e) => {
    const canvas = e.target.closest?.("[data-fb-canvas]");
    if (canvas && !canvas.contains(e.relatedTarget)) {
      canvas.removeAttribute("data-fb-over");
      unmark();
    }
  });
  el.addEventListener("drop", (e) => {
    const canvas = e.target.closest?.("[data-fb-canvas]");
    const type = e.dataTransfer?.getData("application/x-irender-field");
    if (!canvas || !type || !KINDS[type]) return;
    e.preventDefault();
    canvas.removeAttribute("data-fb-over");
    unmark();
    add(type, target(e.clientY).to);
  });

  // A field's grip dragged with a mouse, a finger or a pen.
  let drag = null;
  el.addEventListener("pointerdown", (e) => {
    const grip = e.target.closest?.("[data-fb-grip]");
    if (!grip || (e.pointerType === "mouse" && e.button !== 0)) return;
    e.preventDefault();
    grip.focus({ preventScroll: true });
    drag = { id: grip.closest("[data-fb-card]").getAttribute("data-fb-card"), pointer: e.pointerId, y: e.clientY, moved: false, to: null };
    try {
      grip.setPointerCapture(e.pointerId);
    } catch {}
  });
  el.addEventListener("pointermove", (e) => {
    if (!drag || e.pointerId !== drag.pointer) return;
    if (!drag.moved && Math.abs(e.clientY - drag.y) < 4) return;
    if (!drag.moved) {
      drag.moved = true;
      el.querySelector(`[data-fb-card="${CSS.escape(drag.id)}"]`)?.setAttribute("data-dragging", "");
    }
    unmark();
    const { to, mark } = target(e.clientY, drag.id);
    drag.to = to;
    mark?.[0].setAttribute("data-drop", mark[1]);
  });
  const endDrag = (e, done) => {
    if (!drag || e.pointerId !== drag.pointer) return;
    const d = drag;
    drag = null;
    unmark();
    el.querySelector("[data-dragging]")?.removeAttribute("data-dragging");
    if (done && d.moved && d.to !== null) move(d.id, d.to);
  };
  el.addEventListener("pointerup", (e) => endDrag(e, true));
  el.addEventListener("pointercancel", (e) => endDrag(e, false));

  // ---- preview: the form to try
  let answers = {}, tried = false, sent = null;
  function problems(list) {
    const out = {};
    for (const f of list) {
      if (f.type === "section") continue;
      const v = answers[f.id];
      const empty = v === undefined || v === null || v === "" || (Array.isArray(v) && !v.length) || (f.type === "checkbox" && !v);
      if (empty) {
        if (f.required) out[f.id] = f.type === "checkbox" ? L("Tick this to go on.") : L("{label} is required.", { label: f.label });
        continue;
      }
      if (f.type === "email" && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(v)) out[f.id] = L("Enter an email address, like name@example.com.");
      if (f.type === "number" && (f.min !== undefined && Number(v) < f.min)) out[f.id] = L("{label} must be at least {n}.", { label: f.label, n: f.min });
      if (f.type === "number" && (f.max !== undefined && Number(v) > f.max)) out[f.id] = L("{label} must be at most {n}.", { label: f.label, n: f.max });
      if (f.type === "date" && f.min && v < f.min) out[f.id] = L("Pick a date on or after {date}.", { date: f.min });
      if (f.type === "date" && f.max && v > f.max) out[f.id] = L("Pick a date on or before {date}.", { date: f.max });
    }
    return out;
  }
  function drawPreview() {
    const box = panel("preview"), list = fields();
    if (sent) {
      return box.replaceChildren(h("div", { "data-fb-preview": true },
        h("div", { "data-fb-done": true, role: "status" }, L("Thank you, your answers are in. This is what the form sent:")),
        h("pre", {}, JSON.stringify(sent, null, 2)),
        h("div", { "data-fb-actions": true }, h("button", { type: "button", "data-fb-button": true, onclick: () => { sent = null; answers = {}; tried = false; drawPreview(); } }, L("Try it again")))));
    }
    const errors = tried ? problems(list) : {};
    const form = h("form", { "data-fb-preview": true, novalidate: true, "aria-label": L("Preview") });
    if (!list.length) form.append(h("p", { "data-fb-help": true }, L("The form has no fields yet: add some under Build.")));
    for (const f of list) {
      if (f.type === "section") {
        form.append(h("div", {}, h("h3", { "data-fb-section-title": true }, f.label), f.description ? h("p", { "data-fb-help": true }, f.description) : null));
        continue;
      }
      const id = `${cfg.id}-pv-${f.id}`, help = f.description ? h("span", { "data-fb-help": true, id: `${id}-help` }, f.description) : null;
      const err = errors[f.id] ? h("span", { "data-fb-error": true, id: `${id}-err` }, errors[f.id]) : null;
      const described = [help && `${id}-help`, err && `${id}-err`].filter(Boolean).join(" ") || null;
      const common = { id, "aria-describedby": described, "aria-invalid": err ? "true" : null, "aria-required": f.required ? "true" : null };
      const set = (v) => {
        answers[f.id] = v;
        if (tried) drawPreviewKeepingFocus();
      };
      const title = [f.label, f.required ? h("span", { "data-fb-required": true, "aria-hidden": "true" }, " *") : null];
      let control;
      if (f.type === "textarea") {
        control = h("textarea", { ...common, rows: "4", placeholder: f.placeholder ?? null });
        control.value = answers[f.id] ?? "";
        control.addEventListener("input", () => set(control.value));
      } else if (f.type === "select") {
        control = h("select", common, h("option", { value: "" }, f.placeholder ?? L("Choose…")), ...(f.options ?? []).map((o) => h("option", { value: o, selected: answers[f.id] === o }, o)));
        control.addEventListener("change", () => set(control.value || null));
      } else if (f.type === "radio" || f.type === "checkboxes") {
        const many = f.type === "checkboxes", cur = answers[f.id] ?? (many ? [] : null);
        const group = h("fieldset", { "data-fb-choices": true, "aria-describedby": described, "aria-invalid": err ? "true" : null },
          h("legend", { "data-fb-legend": true }, ...title),
          ...(f.options ?? []).map((o, i) => {
            const input = h("input", { type: many ? "checkbox" : "radio", name: id, id: `${id}-${i}`, value: o, checked: many ? cur.includes(o) : cur === o });
            input.addEventListener("change", () => set(many ? (input.checked ? [...(answers[f.id] ?? []), o] : (answers[f.id] ?? []).filter((x) => x !== o)) : o));
            return h("label", { "data-fb-check": true, for: `${id}-${i}` }, input, o);
          }));
        form.append(h("div", { "data-fb-field": true, "data-fb-for": f.id }, group, help, err));
        continue;
      } else if (f.type === "checkbox") {
        const input = h("input", { ...common, type: "checkbox", checked: !!answers[f.id] });
        input.addEventListener("change", () => set(input.checked));
        form.append(h("div", { "data-fb-field": true, "data-fb-for": f.id }, h("label", { "data-fb-check": true, for: id }, input, ...title), help, err));
        continue;
      } else {
        const type = { email: "email", number: "number", date: "date" }[f.type] ?? "text";
        control = h("input", { ...common, type, placeholder: f.placeholder ?? null, min: f.min ?? null, max: f.max ?? null, value: answers[f.id] ?? "" });
        control.addEventListener("input", () => set(control.value === "" ? null : type === "number" ? Number(control.value) : control.value));
      }
      form.append(h("div", { "data-fb-field": true, "data-fb-for": f.id }, h("label", { for: id }, ...title), control, help, err));
    }
    form.append(h("div", { "data-fb-actions": true }, h("button", { type: "submit", "data-fb-button": true, "data-primary": true }, cfg.submitLabel)));
    form.addEventListener("submit", (e) => {
      e.preventDefault();
      tried = true;
      const wrong = problems(fields());
      if (Object.keys(wrong).length) {
        drawPreview();
        say(L(["{n} answer needs a look", "{n} answers need a look"], { n: Object.keys(wrong).length }));
        panel("preview").querySelector("[aria-invalid=true]")?.focus();
        return;
      }
      sent = Object.fromEntries(fields().filter((f) => f.type !== "section").map((f) => [f.id, answers[f.id] ?? (f.type === "checkboxes" ? [] : f.type === "checkbox" ? false : null)]));
      drawPreview();
      say(L("Thank you, your answers are in."));
    });
    box.replaceChildren(form);
  }
  // Drawn again after a change (errors come and go), with focus and caret kept.
  function drawPreviewKeepingFocus() {
    const a = document.activeElement, id = a?.id, start = a?.selectionStart, end = a?.selectionEnd;
    drawPreview();
    const again = id && document.getElementById(id);
    if (again) {
      again.focus();
      try {
        if (start != null) again.setSelectionRange(start, end);
      } catch {}
    }
  }

  // ---- the document: the kit's own form for the fields
  function documentOf(list) {
    const state = {}, rules = {}, top = [{ type: "ui-error-summary" }];
    let into = top;
    for (const f of list) {
      if (f.type === "section") {
        const s = { type: "ui-form-section", props: { title: f.label, ...(f.description ? { description: f.description } : {}) }, children: [] };
        top.push(s);
        into = s.children;
        continue;
      }
      const id = f.id, bind = { bind: id };
      state[id] = f.type === "checkboxes" ? [] : f.type === "checkbox" ? false : f.type === "text" || f.type === "textarea" || f.type === "email" ? "" : null;
      const rule = { label: f.label };
      if (f.required) rule.required = true;
      if (f.type === "email") rule.type = "email";
      if (f.type === "date") rule.type = "date";
      if (LIMITS.has(f.type) && f.min !== undefined) rule.min = f.min;
      if (LIMITS.has(f.type) && f.max !== undefined) rule.max = f.max;
      if (Object.keys(rule).length > 1) rules[id] = rule;
      const ph = f.placeholder ? { placeholder: f.placeholder } : {};
      const limits = { ...(f.min !== undefined ? { min: f.min } : {}), ...(f.max !== undefined ? { max: f.max } : {}) };
      const control = {
        text: { type: "ui-input", props: { ...bind, ...ph } },
        email: { type: "ui-input", props: { ...bind, type: "email", ...ph }, attrs: { autocomplete: "email" } },
        textarea: { type: "ui-textarea", props: { ...bind, rows: 4, ...ph } },
        number: { type: "ui-number-input", props: { ...bind, ...limits, ...ph } },
        date: { type: "ui-date-picker", props: { ...bind, ...limits } },
        select: { type: "ui-select", props: { ...bind, options: f.options ?? [], ...ph } },
        radio: { type: "ui-radio-group", props: { ...bind, options: f.options ?? [] } },
        checkboxes: { type: "ui-toggle-group", props: { ...bind, multiple: true, variant: "outline", options: f.options ?? [] } },
        checkbox: { type: "ui-checkbox", props: { ...bind, label: f.label } },
      }[f.type];
      if (!control) continue;
      const props = { ...(f.type === "checkbox" ? {} : { label: f.label }), ...(f.description ? { description: f.description } : {}), error: `{{ errors.${id} }}` };
      into.push({ type: "ui-field", props, children: [control] });
    }
    const ids = list.filter((f) => f.type !== "section").map((f) => f.id);
    state.errors = {};
    const button = { type: "ui-button", props: { label: cfg.submitLabel, type: "submit" } };
    const form = { type: "ui-form", props: { bindErrors: "errors", rules }, on: {}, children: [...top, button] };
    if (cfg.endpoint) {
      form.on.submit = { action: "submit", endpoint: cfg.endpoint, method: "POST", body: Object.fromEntries(ids.map((k) => [k, `{{ ${k} }}`])) };
      return { version: 1, state, root: form };
    }
    state.submitted = false;
    form.on.submit = { action: "set_state", key: "submitted", value: true };
    return {
      version: 1,
      state,
      root: { type: "stack", props: { gap: 16 }, children: [
        { type: "if", props: { when: "{{ submitted }}" }, children: [{ type: "ui-alert", props: { variant: "success", title: L("Thank you"), description: L("Your answers are in.") } }] },
        { type: "if", props: { when: "{{ !submitted }}" }, children: [form] },
      ] },
    };
  }
  // YAML for a document: block style, strings quoted where YAML would read them otherwise.
  function yaml(v) {
    const plain = /^[A-Za-z_][\w .,/()'?!–-]*$/;
    const special = /^(true|false|null|yes|no|on|off|y|n|~)$/i;
    const scalar = (x) => (typeof x === "string" ? (plain.test(x) && !special.test(x) && !/[:#]\s|\s$/.test(x) ? x : JSON.stringify(x)) : x === null ? "null" : String(x));
    const key = (k) => (/^[A-Za-z_][\w-]*$/.test(k) ? k : JSON.stringify(k));
    const lines = [];
    const emit = (x, ind, lead) => {
      // `lead`: what goes before the first line ("- " for a list item).
      if (Array.isArray(x)) {
        if (!x.length) return lines.push(`${lead}[]`);
        x.forEach((item, i) => {
          const pre = i === 0 && lead ? lead : ind;
          if (isObj(item) || Array.isArray(item)) emit(item, `${ind}  `, `${pre}- `);
          else lines.push(`${pre}- ${scalar(item)}`);
        });
      } else if (isObj(x)) {
        const entries = Object.entries(x);
        if (!entries.length) return lines.push(`${lead}{}`);
        entries.forEach(([k, val], i) => {
          const pre = i === 0 && lead ? lead : ind;
          if ((isObj(val) && Object.keys(val).length) || (Array.isArray(val) && val.length)) {
            lines.push(`${pre}${key(k)}:`);
            emit(val, `${ind}  `, "");
          } else lines.push(`${pre}${key(k)}: ${isObj(val) ? "{}" : Array.isArray(val) ? "[]" : scalar(val)}`);
        });
      } else lines.push(`${lead}${scalar(x)}`);
    };
    emit(v, "", "");
    return `${lines.join("\n")}\n`;
  }
  let format = "yaml";
  function drawDocument() {
    const doc = documentOf(fields());
    const text = format === "yaml" ? yaml(doc) : `${JSON.stringify(doc, null, 2)}\n`;
    const pick = (f, label) => h("button", { type: "button", "data-fb-button": true, "aria-pressed": format === f ? "true" : "false", onclick: () => { format = f; drawDocument(); panel("document").querySelector(`[aria-pressed=true]`)?.focus(); } }, label);
    const copy = h("button", { type: "button", "data-fb-button": true, onclick: async () => {
      try {
        await navigator.clipboard.writeText(text);
        say(L("Copied"));
        copy.textContent = L("Copied");
      } catch {
        say(L("Couldn't copy: select the text and copy it"));
      }
    } }, L("Copy"));
    panel("document").replaceChildren(
      h("div", { "data-fb-actions": true, style: null }, h("div", { "data-fb-actions": true, role: "group", "aria-label": L("Format") }, pick("yaml", "YAML"), pick("json", "JSON")), copy),
      h("p", { "data-fb-help": true }, L("A page of its own: render it with irender and the UI kit, or put its root in a page and its state beside the page's.")),
      h("pre", { tabindex: "0", "aria-label": L("The form as an iRender document") }, h("code", {}, text)),
    );
  }
  function publish(list) {
    if (cfg.bindDocument) api.set(cfg.bindDocument, documentOf(list));
    if (tab === "document") drawDocument();
  }

  // ---- the tabs
  function show(k, focus) {
    tab = k;
    for (const b of el.querySelectorAll("[data-fb-tab]")) {
      const on = b.getAttribute("data-fb-tab") === k;
      b.setAttribute("aria-selected", on ? "true" : "false");
      b.tabIndex = on ? 0 : -1;
      if (on && focus) b.focus();
    }
    for (const p of el.querySelectorAll("[data-fb-panel]")) p.hidden = p.getAttribute("data-fb-panel") !== k;
    if (k === "build") drawBuild();
    else if (k === "preview") drawPreview();
    else drawDocument();
  }

  // The fields as state has them now; another component may have changed them.
  function sync() {
    const list = fields(), now = JSON.stringify(list);
    if (now === written) return;
    written = now;
    publish(list);
    if (tab === "build") {
      drawCanvas();
      if (!$("[data-fb-inspector]")?.contains(document.activeElement)) drawInspector();
    } else show(tab);
  }
  written = JSON.stringify(fields());
  selected = fields()[0]?.id ?? null;
  drawBuild();
  publish(fields());
  return { update: sync };
};
