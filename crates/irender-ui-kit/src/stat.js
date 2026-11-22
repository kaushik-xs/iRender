// irender-ui-kit: `ui-stat` (the figure, its change, trend line and progress, as they
// change) and `ui-description-item` (its value formatted, copied and edited in place).
window.__irenderBehaviors ||= {};
window.__irenderBehaviors["ui-stat"] = (el, api) => {
  const cfg = api.config, K = window.__irenderKit, L = (t, v) => window.__irender.label(t, v);
  const $ = (s) => el.querySelector(s);
  const num = (v) => (K.blank(v) || isNaN(v) ? null : Number(v));
  const write = (v, o = {}) => K.format(v, { format: cfg.format, currency: cfg.currency, decimals: cfg.decimals, locale: cfg.locale, ...o });
  function draw() {
    const value = K.value(api, cfg.value), previous = num(K.value(api, cfg.previous)), n = num(value);
    const figure = $("[data-stat-figure]"), text = K.blank(value) ? "—" : write(value);
    if (figure.textContent !== text) figure.textContent = text;
    // The change: given, or from the figure before.
    let change = num(K.value(api, cfg.change)), by = change;
    const own = cfg.changeAs === "value";
    if (change === null && previous !== null && n !== null) {
      by = n - previous;
      change = own ? by : previous === 0 ? null : by / Math.abs(previous);
    }
    const c = $("[data-stat-change]");
    c.hidden = change === null;
    const way = by === null || change === null ? 0 : Math.sign(by);
    if (change !== null) {
      const said = own ? write(Math.abs(change)) : K.format(Math.abs(change), { format: "percent", decimals: Math.abs(change) < 0.1 && change % 0.01 ? 1 : 0, locale: cfg.locale });
      c.replaceChildren(...(way ? [K.icon(way > 0 ? "up" : "down", 13)] : []), K.make("span", { "aria-label": `${way > 0 ? L("up") : way < 0 ? L("down") : L("no change")} ${said}` }, said));
    }
    const good = cfg.goodWhen ?? "up";
    const tone = !way || good === "neutral" ? null : (way > 0) === (good === "up") ? "good" : "bad";
    if (tone) el.setAttribute("data-tone", tone);
    else el.removeAttribute("data-tone");
    const svg = $("svg[data-stat-trend]");
    if (svg) {
      const list = (K.value(api, cfg.trend) ?? []).map((v) => num(K.isObj(v) ? v.value ?? v.y : v)).filter((v) => v !== null);
      const lo = Math.min(...list), hi = Math.max(...list), x = (i) => (list.length < 2 ? 48 : (i / (list.length - 1)) * 96), y = (v) => (hi === lo ? 16 : 29 - ((v - lo) / (hi - lo)) * 26);
      const line = list.map((v, i) => `${i ? "L" : "M"}${x(i).toFixed(1)} ${y(v).toFixed(1)}`).join("");
      $("[data-stat-line]").setAttribute("d", line);
      $("[data-stat-area]").setAttribute("d", line && `${line}L96 32L0 32Z`);
      svg.style.display = list.length > 1 ? "" : "none";
    }
    const bar = $("[data-stat-bar]");
    if (bar) {
      const target = num(K.value(api, cfg.target));
      const part = target && n !== null ? n / target : 0, pct = K.format(part, { format: "percent", decimals: 0, locale: cfg.locale });
      bar.firstElementChild.style.width = K.clamp(part * 100, 0, 100) + "%";
      bar.setAttribute("aria-valuenow", String(Math.round(K.clamp(part * 100, 0, 100))));
      $("[data-stat-target]").textContent = target === null ? "" : K.value(api, cfg.targetLabel) ?? L("{percent} of {target}", { percent: pct, target: write(target) });
    }
  }
  draw();
  return { update: draw };
};

window.__irenderBehaviors["ui-description-item"] = (el, api) => {
  const cfg = api.config, K = window.__irenderKit, C = window.__irenderCells, L = (t, v) => window.__irender.label(t, v);
  const shown = el.querySelector("[data-dl-value]"), copy = el.querySelector("[data-dl-copy]");
  const raw = () => (cfg.bind ? api.state[cfg.bind] : K.value(api, cfg.value));
  const opts = () => C.choices({ options: K.value(api, cfg.options) });
  const label = () => el.querySelector("dt").textContent;
  function text(v) {
    if (K.blank(v)) return cfg.bind ? K.value(api, cfg.placeholder) ?? L("Add") : "—";
    if (cfg.edit === "select") return opts().find((o) => o.value === v)?.label ?? String(v);
    if (cfg.format === "date" || cfg.format === "datetime" || cfg.edit === "date") {
      const t = new Date(v);
      if (isNaN(t)) return String(v);
      const plain = /^\d{4}-\d{2}-\d{2}$/.test(v);
      return new Intl.DateTimeFormat(cfg.locale || undefined, { dateStyle: "medium", ...(cfg.format === "datetime" ? { timeStyle: "short" } : {}), ...(plain ? { timeZone: "UTC" } : {}) }).format(t);
    }
    return cfg.format ? K.format(v, { format: cfg.format, currency: cfg.currency, locale: cfg.locale }) : typeof v === "object" ? JSON.stringify(v) : String(v);
  }
  function draw() {
    if (!shown || !shown.isConnected) return;
    const v = raw(), t = text(v);
    if (shown.textContent !== t) shown.textContent = t;
    shown.toggleAttribute("data-empty", K.blank(v));
    if (cfg.bind) shown.setAttribute("aria-label", L("{name}: {value}, edit", { name: label(), value: K.blank(v) ? L("empty") : t }));
    if (copy) copy.hidden = K.blank(v) && !!shown;
  }
  function edit() {
    const v = raw();
    const input = K.make(cfg.edit === "select" ? "select" : cfg.edit === "multiline" ? "textarea" : "input", { "data-dl-input": "", "aria-label": label() });
    if (cfg.edit === "select") {
      input.append(new Option("", ""));
      opts().forEach((o, i) => input.append(new Option(o.label, String(i), false, o.value === v)));
    } else {
      if (cfg.edit === "date") input.type = "date";
      if (cfg.edit === "number") input.inputMode = "decimal";
      if (cfg.edit === "multiline") input.rows = 3;
      input.value = K.blank(v) ? "" : String(v);
    }
    let closed = false;
    const close = (save, focus) => {
      if (closed) return;
      closed = true;
      const next = cfg.edit === "select" ? (input.value === "" ? null : opts()[Number(input.value)].value) : C.parse({ type: cfg.edit ?? "text" }, input.value, cfg.locale);
      input.replaceWith(shown);
      if (save && next !== v && !(K.blank(next) && K.blank(v))) {
        api.set(cfg.bind, next);
        api.emit("change", { value: next, previous: v ?? null });
      }
      draw();
      if (focus) shown.focus();
    };
    input.addEventListener("keydown", (e) => {
      if (e.isComposing) return;
      if (e.key === "Escape") e.preventDefault(), e.stopPropagation(), close(false, true);
      else if (e.key === "Enter" && (cfg.edit !== "multiline" || e.metaKey || e.ctrlKey)) e.preventDefault(), close(true, true);
    });
    input.addEventListener("blur", () => close(true, false));
    if (cfg.edit === "select") input.addEventListener("change", () => close(true, true));
    shown.replaceWith(input);
    input.focus();
    input.select?.();
  }
  if (cfg.bind) shown?.addEventListener("click", edit);
  if (copy) {
    const rest = () => copy.replaceChildren(K.icon("copy", 14));
    rest();
    copy.addEventListener("click", async () => {
      const v = raw();
      try {
        await navigator.clipboard.writeText(shown ? String(K.blank(v) ? "" : v) : el.querySelector("dd").innerText.trim());
        copy.replaceChildren(K.icon("check", 14));
        K.say(el, L("Copied"));
        api.emit("copy", { value: v });
      } catch {
        K.say(el, L("Couldn't copy"));
      }
      setTimeout(rest, 1500);
    });
  }
  draw();
  return { update: draw };
};
