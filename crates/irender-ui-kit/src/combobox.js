// irender-ui-kit: the `ui-combobox` behaviour. The server renders the control; this
// builds the menu, filters it, loads options, and handles the mouse and keyboard.
// Everything it creates uses textContent, never HTML, so option labels are inert.
(window.__irenderBehaviors ||= {})["ui-combobox"] = (el, api) => {
  const cfg = api.config;
  const T = cfg.text;
  const $ = (s) => el.querySelector(s);
  const control = $("[data-ui-control]"), values = $("[data-ui-values]"), input = $("[data-ui-input]");
  const menu = $("[data-ui-menu]"), live = $("[data-ui-live]"), hidden = $("[data-ui-hidden]"), clear = $("[data-ui-clear]");

  const keyOf = (v) => JSON.stringify(v ?? null);
  const same = (a, b) => keyOf(a) === keyOf(b);
  // Case- and accent-insensitive text for matching.
  const fold = (s) => String(s ?? "").normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
  const F = cfg.fields;
  const norm = (o) => {
    if (o === null || typeof o !== "object" || Array.isArray(o)) return { value: o, label: String(o ?? "") };
    // Groups arrive as `{ label, options }` from APIs and state, `{ group, options }` from the server.
    if (Array.isArray(o.options)) return { group: String(o.group ?? o.label ?? ""), options: o.options.map(norm) };
    // Server-normalized options carry `value`/`label`; others use the configured fields.
    const pick = (field, plain) => (o[field] !== undefined ? o[field] : o[plain]);
    const value = pick(F.value, "value") ?? null;
    return { value, label: String(pick(F.label, "label") ?? value ?? ""), description: pick(F.description, "description") ?? null, disabled: !!o.disabled };
  };
  const flat = (list) => list.flatMap((o) => (o.group != null ? o.options : [o]));
  const at = (v, path) => path.reduce((x, k) => (x != null && typeof x === "object" ? x[k] : undefined), v);

  // Every option seen so far, so chosen values keep their labels.
  const known = new Map();
  const remember = (list) => flat(list).forEach((o) => known.has(keyOf(o.value)) || known.set(keyOf(o.value), o));
  const labelOf = (v) => known.get(keyOf(v))?.label ?? String(v ?? "");
  const base = () => (cfg.optionsPath ? at(api.state, cfg.optionsPath) || [] : cfg.options).map(norm);
  remember(cfg.options.map(norm));

  // The value: in state when bound, here otherwise.
  let local = cfg.value;
  const current = () => (cfg.key ? api.state[cfg.key] : local);
  const selected = () => {
    const v = current();
    if (cfg.multiple) return Array.isArray(v) ? v : v == null ? [] : [v];
    return v == null || v === "" ? [] : [v];
  };
  const isSelected = (v) => selected().some((s) => same(s, v));
  const fixed = (v) => cfg.fixed.some((f) => same(f, v));
  const disabled = () => input.disabled;

  function commit(list) {
    const v = cfg.multiple ? list : list.length ? list[list.length - 1] : null;
    if (cfg.key) api.set(cfg.key, v);
    else {
      local = v;
      render();
    }
    // A plain `change`, so documents can react with `on: { change: … }`.
    api.emit("change", { value: v });
  }
  const announce = (msg) => (live.textContent = msg);

  // ---- remote options
  let remote = null, loading = false, failed = false, seq = 0, timer;
  const cache = new Map();
  function search(q) {
    clearTimeout(timer);
    if (!cfg.searchUrl || q.length < cfg.minChars || !q) {
      remote = null;
      loading = false;
      return;
    }
    if (cache.has(q)) {
      remote = cache.get(q);
      loading = false;
      return;
    }
    loading = true;
    failed = false;
    const id = ++seq;
    timer = setTimeout(async () => {
      try {
        const url = cfg.searchUrl + (cfg.searchUrl.includes("?") ? "&" : "?") + new URLSearchParams({ q });
        const res = await fetch(url, { headers: { accept: "application/json" } });
        if (!res.ok) throw new Error(String(res.status));
        const data = await res.json();
        const list = (Array.isArray(data) ? data : data.options || []).map(norm);
        cache.set(q, list);
        remember(list);
        if (id === seq) remote = list;
      } catch {
        if (id === seq) {
          failed = true;
          remote = [];
        }
      }
      if (id === seq) {
        loading = false;
        el.toggleAttribute("data-loading", false);
        renderMenu();
      }
    }, cfg.debounce);
  }

  // ---- menu
  let open = false, active = -1, activeKey = null, shown = [];
  const query = () => input.value.trim();

  function entries() {
    const q = fold(query());
    const fromRemote = remote && query();
    const list = fromRemote ? remote : base();
    remember(list);
    // Loaded results are filtered too, as react-select does.
    const keep = (o) => (!cfg.hideSelected || !isSelected(o.value)) && (!q || fold(o.label).includes(q) || fold(o.value).includes(q));
    const out = [];
    for (const o of list) {
      if (o.group != null) {
        const opts = o.options.filter(keep);
        if (opts.length) out.push({ group: o.group }, ...opts.map((option) => ({ option, grouped: true })));
      } else if (keep(o)) out.push({ option: o });
    }
    const raw = query();
    if (cfg.creatable && raw && !flat(list).some((o) => fold(o.label) === fold(raw)) && !selected().some((v) => fold(labelOf(v)) === fold(raw)))
      out.push({ option: { value: raw, label: raw, create: true } });
    return out;
  }

  function renderMenu() {
    menu.replaceChildren();
    shown = [];
    if (!open) return;
    let group = null, groups = 0, ids = 0;
    for (const e of entries()) {
      if (e.group != null) {
        group = document.createElement("div");
        group.setAttribute("role", "group");
        const heading = document.createElement("div");
        heading.dataset.uiGroupLabel = "";
        heading.id = `${menu.id}-g${groups++}`;
        heading.textContent = e.group;
        group.setAttribute("aria-labelledby", heading.id);
        group.append(heading);
        menu.append(group);
        continue;
      }
      if (!e.grouped) group = null;
      const o = e.option;
      const row = document.createElement("div");
      row.setAttribute("role", "option");
      row.id = `${menu.id}-o${ids++}`;
      row.setAttribute("aria-selected", String(!o.create && isSelected(o.value)));
      if (o.create) row.setAttribute("data-create", "");
      const label = document.createElement("span");
      label.textContent = o.create ? T.create.replace("{input}", o.label) : o.label;
      row.append(label);
      if (o.description) {
        const d = document.createElement("span");
        d.dataset.uiDescription = "";
        d.textContent = o.description;
        row.append(d);
      }
      if (o.disabled) row.setAttribute("aria-disabled", "true");
      else {
        const index = shown.length;
        shown.push({ o, el: row });
        row.addEventListener("mousedown", (ev) => {
          ev.preventDefault();
          choose(o);
        });
        row.addEventListener("mousemove", () => active !== index && highlight(index));
      }
      (group || menu).append(row);
    }
    if (!menu.querySelector("[role=option]")) {
      const msg = document.createElement("div");
      msg.dataset.uiMessage = "";
      const busy = loading || el.getAttribute("aria-busy") === "true";
      msg.textContent = busy ? T.loading : failed ? T.error : T.noOptions;
      menu.append(msg);
    }
    // Keep the highlighted option across re-renders, else start at the first.
    const keep = shown.findIndex((s) => keyOf(s.o.value) === activeKey);
    highlight(keep >= 0 ? keep : shown.length ? 0 : -1);
  }

  function highlight(i) {
    active = i;
    shown.forEach((s, j) => s.el.toggleAttribute("data-active", j === i));
    const s = shown[i];
    activeKey = s ? keyOf(s.o.value) : null;
    if (s) {
      input.setAttribute("aria-activedescendant", s.el.id);
      s.el.scrollIntoView({ block: "nearest" });
    } else input.removeAttribute("aria-activedescendant");
  }

  function move(delta, clamp) {
    if (!shown.length) return;
    let i = active < 0 ? (delta > 0 ? -1 : shown.length) : active;
    i += delta;
    i = clamp ? Math.max(0, Math.min(shown.length - 1, i)) : (i + shown.length) % shown.length;
    highlight(i);
  }

  function setOpen(v) {
    if (v && disabled()) return;
    if (v === open) return;
    open = v;
    input.setAttribute("aria-expanded", String(v));
    el.toggleAttribute("data-open", v);
    if (v) {
      activeKey = null;
      renderMenu();
      try { menu.showPopover(); } catch {}
      announce(`${shown.length} result${shown.length === 1 ? "" : "s"} available.`);
    } else {
      try { menu.hidePopover(); } catch {}
      menu.replaceChildren();
      shown = [];
      active = -1;
      input.removeAttribute("aria-activedescendant");
    }
  }

  function resetInput() {
    input.value = "";
    el.removeAttribute("data-typing");
    remote = null;
  }

  function choose(o) {
    const value = o.value;
    if (o.create) known.set(keyOf(value), { value, label: o.label });
    let list = selected();
    if (cfg.multiple) list = isSelected(value) ? (fixed(value) ? list : list.filter((v) => !same(v, value))) : [...list, value];
    else list = [value];
    resetInput();
    commit(list);
    announce(`${labelOf(value)}, ${isSelected(value) || !cfg.multiple ? "selected" : "removed"}.`);
    if (cfg.closeOnSelect) setOpen(false);
    else renderMenu();
  }

  function remove(v) {
    if (fixed(v)) return;
    commit(selected().filter((x) => !same(x, v)));
    announce(`${labelOf(v)}, removed.`);
  }

  // ---- the control
  function render() {
    const sel = selected();
    for (const n of [...values.children]) if (n !== input) n.remove();
    if (cfg.multiple) {
      for (const v of sel) {
        const chip = document.createElement("span");
        chip.dataset.uiChip = "";
        const text = document.createElement("span");
        text.textContent = labelOf(v);
        chip.append(text);
        if (fixed(v)) chip.dataset.fixed = "";
        else {
          const x = document.createElement("button");
          x.type = "button";
          x.tabIndex = -1;
          x.dataset.uiRemove = "";
          x.setAttribute("aria-label", T.remove.replace("{label}", labelOf(v)));
          x.textContent = "✕";
          x.addEventListener("mousedown", (ev) => {
            ev.preventDefault();
            ev.stopPropagation();
            if (!disabled()) remove(v);
          });
          chip.append(x);
        }
        values.insertBefore(chip, input);
      }
    } else if (sel.length) {
      const single = document.createElement("span");
      single.dataset.uiSingle = "";
      single.textContent = labelOf(sel[0]);
      values.insertBefore(single, input);
    }
    input.placeholder = sel.length ? "" : T.placeholder;
    el.toggleAttribute("data-has-value", sel.length > 0);
    el.toggleAttribute("data-disabled", disabled());
    clear.hidden = !cfg.clearable || disabled() || !sel.some((v) => !fixed(v));
    const asText = (v) => (v == null ? "" : typeof v === "object" ? JSON.stringify(v) : String(v));
    hidden.replaceChildren(
      ...(cfg.name ? (cfg.multiple ? sel : [sel[0] ?? null]) : []).map((v) => {
        const h = document.createElement("input");
        h.type = "hidden";
        h.name = cfg.name;
        h.value = asText(v);
        return h;
      }),
    );
    if (disabled()) setOpen(false);
    if (open) renderMenu();
  }

  input.addEventListener("input", () => {
    if (!cfg.searchable) return;
    el.toggleAttribute("data-typing", input.value !== "");
    search(query());
    el.toggleAttribute("data-loading", loading);
    activeKey = null;
    if (!open) setOpen(true);
    else renderMenu();
    if (!loading) announce(`${shown.length} result${shown.length === 1 ? "" : "s"} available.`);
  });

  input.addEventListener("keydown", (e) => {
    const k = e.key;
    if (k === "ArrowDown" || k === "ArrowUp") {
      e.preventDefault();
      if (!open) setOpen(true);
      else move(k === "ArrowDown" ? 1 : -1);
    } else if ((k === "Home" || k === "End") && open) {
      e.preventDefault();
      highlight(k === "Home" ? 0 : shown.length - 1);
    } else if ((k === "PageDown" || k === "PageUp") && open) {
      e.preventDefault();
      move(k === "PageDown" ? 5 : -5, true);
    } else if (k === "Enter") {
      if (!open) return;
      e.preventDefault();
      if (shown[active]) choose(shown[active].o);
    } else if (k === "Tab") {
      if (open && shown[active] && !e.shiftKey && query()) choose(shown[active].o);
      setOpen(false);
    } else if (k === "Escape") {
      // Each Escape undoes one thing: the menu, then the typed text. Only one that
      // finds nothing to undo reaches a dialog the combobox is in, and closes it.
      if (open) {
        e.preventDefault();
        setOpen(false);
      } else if (input.value) {
        e.preventDefault();
        resetInput();
      }
    } else if (k === "Backspace" && input.value === "") {
      const removable = selected().filter((v) => !fixed(v));
      if (removable.length && (cfg.multiple || cfg.clearable)) {
        e.preventDefault();
        remove(removable[removable.length - 1]);
      }
    } else if (k === " " && !cfg.searchable) {
      e.preventDefault();
      setOpen(!open);
    }
  });

  input.addEventListener("focus", () => el.toggleAttribute("data-focused", true));
  input.addEventListener("blur", () => {
    el.toggleAttribute("data-focused", false);
    setOpen(false);
    resetInput();
  });

  control.addEventListener("mousedown", (e) => {
    if (disabled() || e.button !== 0) return;
    if (e.target !== input) e.preventDefault();
    input.focus();
    setOpen(e.target === input ? true : !open);
  });

  clear.addEventListener("mousedown", (e) => {
    e.preventDefault();
    e.stopPropagation();
    if (disabled()) return;
    resetInput();
    commit(selected().filter(fixed));
    announce("All selected options cleared.");
    input.focus();
  });

  render();
  return { update: render };
};
