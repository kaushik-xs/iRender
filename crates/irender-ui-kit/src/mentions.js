// irender-ui-kit: naming people in a text area (`window.__irenderMentions`): typing `@`
// opens a list at the caret, narrowed as the name is typed; arrows and Enter (or a click)
// pick. Also the `ui-mention-input` behaviour.
(() => {
  if (window.__irenderMentions) return;
  const K = window.__irenderKit, L = (t, v) => window.__irender.label(t, v);
  const person = (p) => (K.isObj(p) ? { ...p, name: String(p.name ?? p.label ?? p.value ?? "") } : { name: String(p) });
  const fold = (s) => String(s).normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
  // Where a character of a text area is on the page, measured in a copy of it.
  function caretAt(area, index) {
    const cs = getComputedStyle(area), copy = K.make("div", { "aria-hidden": "true" }), mark = K.make("span", null, "​");
    for (const p of ["boxSizing", "width", "paddingTop", "paddingRight", "paddingBottom", "paddingLeft", "borderTopWidth", "borderRightWidth", "borderBottomWidth", "borderLeftWidth", "fontFamily", "fontSize", "fontWeight", "fontStyle", "letterSpacing", "lineHeight", "textTransform", "wordSpacing", "textIndent", "direction", "tabSize"]) copy.style[p] = cs[p];
    Object.assign(copy.style, { position: "absolute", visibility: "hidden", whiteSpace: "pre-wrap", overflowWrap: "break-word", top: "0", left: "0", borderStyle: "solid" });
    copy.append(area.value.slice(0, index), mark, area.value.slice(index) || " ");
    document.body.append(copy);
    const a = area.getBoundingClientRect(), c = copy.getBoundingClientRect(), m = mark.getBoundingClientRect();
    copy.remove();
    return { x: a.left + m.left - c.left - area.scrollLeft, top: a.top + m.top - c.top - area.scrollTop, bottom: a.top + m.bottom - c.top - area.scrollTop };
  }
  window.__irenderMentions = (area, o) => {
    const trigger = o.trigger || "@";
    let list = null, found = [], at = -1, pick = 0, id = null;
    const close = () => {
      list?.remove();
      list = null;
      area.removeAttribute("aria-expanded");
      area.removeAttribute("aria-activedescendant");
    };
    // The name being typed: from the trigger at a word's start up to the caret.
    function query() {
      const text = area.value.slice(0, area.selectionStart), i = text.lastIndexOf(trigger);
      if (i < 0 || (i > 0 && !/[\s([{"'“]/.test(text[i - 1]))) return null;
      const q = text.slice(i + trigger.length);
      return q.length > 40 || /\n/.test(q) || (q.match(/\s/g) ?? []).length > 2 || /^\s/.test(q) ? null : { i, q };
    }
    function update() {
      const m = area.selectionStart === area.selectionEnd ? query() : null;
      if (!m) return close();
      const q = fold(m.q);
      found = (o.people() ?? []).map(person).filter((p) => p.name && (fold(p.name).split(/\s+/).some((w) => w.startsWith(q)) || fold(p.name).startsWith(q))).slice(0, 8);
      if (!found.length) return close();
      if (at !== m.i) pick = 0;
      at = m.i;
      pick = Math.min(pick, found.length - 1);
      id ??= `mention-${K.id()}`;
      if (!list) {
        list = K.make("div", { "data-mention-list": "", role: "listbox", id, "aria-label": L("People"), popover: "manual" });
        // In the dialog the text area is in, if any, so it's above it.
        (area.closest("dialog[open], [popover]:popover-open") ?? document.body).append(list);
        list.showPopover?.();
        list.addEventListener("pointerdown", (e) => {
          e.preventDefault();
          const op = e.target.closest("[role=option]");
          if (op) choose([...list.children].indexOf(op));
        });
        area.setAttribute("aria-expanded", "true");
        area.setAttribute("aria-controls", id);
        area.setAttribute("aria-autocomplete", "list");
      }
      list.replaceChildren(...found.map((p, i) => K.make("div", { role: "option", id: `${id}-${i}`, "aria-selected": String(i === pick) }, K.avatar(p), K.make("span", null, p.name, p.detail ? K.make("small", null, p.detail) : null))));
      area.setAttribute("aria-activedescendant", `${id}-${pick}`);
      const c = caretAt(area, at), b = list.getBoundingClientRect();
      list.style.left = K.clamp(c.x, 8, innerWidth - b.width - 8) + "px";
      list.style.top = (c.bottom + b.height + 8 > innerHeight && c.top - b.height > 8 ? c.top - b.height - 4 : c.bottom + 4) + "px";
      list.children[pick]?.scrollIntoView({ block: "nearest" });
    }
    function choose(i) {
      const p = found[i], end = area.selectionStart;
      if (!p) return;
      const text = `${trigger}${p.name} `;
      area.focus();
      area.setRangeText(text, at, /^\s/.test(area.value.slice(end)) ? end + 1 : end, "end");
      close();
      area.dispatchEvent(new Event("input", { bubbles: true }));
      o.onPick?.(p);
      close();
    }
    area.addEventListener("input", update);
    area.addEventListener("click", update);
    area.addEventListener("blur", close);
    area.addEventListener("keydown", (e) => {
      if (!list || e.isComposing) return;
      if (e.key === "ArrowDown" || e.key === "ArrowUp") pick = (pick + (e.key === "ArrowDown" ? 1 : found.length - 1)) % found.length, update();
      else if ((e.key === "Enter" || e.key === "Tab") && !e.shiftKey) choose(pick);
      else if (e.key === "Escape") close();
      else return;
      e.preventDefault();
      e.stopImmediatePropagation();
    });
    area.addEventListener("keyup", (e) => ["ArrowLeft", "ArrowRight", "Home", "End"].includes(e.key) && update());
    return { close };
  };
  // The people named in a text, the longest names first so "Ann Lee" isn't read as "Ann".
  window.__irenderMentions.split = (text, people, trigger = "@") => {
    const names = (people ?? []).map(person).map((p) => p.name).filter(Boolean).sort((a, b) => b.length - a.length);
    if (!names.length) return [String(text)];
    const esc = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    return String(text).split(new RegExp(`(${esc(trigger)}(?:${names.map(esc).join("|")})(?![\\p{L}\\p{N}]))`, "u"));
  };
  (window.__irenderBehaviors ||= {})["ui-mention-input"] = (el, api) => {
    const cfg = api.config, area = el.querySelector("textarea"), trigger = cfg.trigger || "@";
    const people = () => K.value(api, cfg.people) ?? [];
    window.__irenderMentions(area, { people, trigger, onPick: (p) => api.emit("mention", { person: p }) });
    return {
      update() {
        if (!cfg.bindMentions) return;
        const names = [...new Set(window.__irenderMentions.split(api.state[cfg.bind] ?? "", people(), trigger).filter((_, i) => i % 2).map((s) => s.slice(trigger.length)))];
        if (JSON.stringify(api.state[cfg.bindMentions] ?? []) !== JSON.stringify(names)) api.set(cfg.bindMentions, names);
      },
    };
  };
})();
