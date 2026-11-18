// irender-ui-kit: the `ui-number-input` behaviour. The field shows the number with the
// locale's grouping ("1,25,000" in en-IN, "125.000" in de-DE) as it's typed, keeping
// the caret among the same digits; state gets a plain number, or null when empty. Up
// and Down step it (Shift for ten steps), the − and + buttons too; leaving the field
// clamps it to `min` and `max` and rounds it to `decimals`.
(window.__irenderBehaviors ||= {})["ui-number-input"] = (el, api) => {
  const cfg = api.config;
  const input = el.querySelector("[data-num-input]");
  const locale = cfg.locale || undefined;
  // The locale's separators, from how it writes 12345.6.
  const parts = new Intl.NumberFormat(locale).formatToParts(12345.6);
  const group = parts.find((p) => p.type === "group")?.value ?? ",";
  const decimal = parts.find((p) => p.type === "decimal")?.value ?? ".";
  const decimals = cfg.decimals ?? (cfg.currency ? new Intl.NumberFormat(locale, { style: "currency", currency: cfg.currency }).resolvedOptions().maximumFractionDigits : null);
  const step = Number(cfg.step) || 1;
  const min = cfg.min ?? null, max = cfg.max ?? null;
  const grouping = new Intl.NumberFormat(locale, { useGrouping: true, maximumFractionDigits: 20 });

  const value = () => {
    const v = cfg.bind ? api.state[cfg.bind] : null;
    return v === null || v === undefined || v === "" || !Number.isFinite(Number(v)) ? null : Number(v);
  };
  // Text as typed → a number: digits, one decimal separator, a leading minus.
  function parse(text) {
    let s = String(text).replace(new RegExp(`[${escape(group)}\\s\\u00a0\\u202f]`, "g"), "");
    s = s.replace(decimal, ".");
    s = s.replace(/[^\d.-]/g, "");
    const neg = s.startsWith("-");
    s = s.replace(/-/g, "");
    const [whole, ...frac] = s.split(".");
    const clean = (neg ? "-" : "") + whole + (frac.length ? "." + frac.join("") : "");
    if (!/\d/.test(clean)) return null;
    const n = Number(clean);
    return Number.isFinite(n) ? n : null;
  }
  function escape(c) {
    return c.replace(/[\\\]^-]/g, "\\$&");
  }
  // A number → text with grouping, keeping what's typed after the decimal separator.
  function show(n, typedFraction) {
    if (n === null) return "";
    const whole = Math.trunc(n);
    let text = grouping.format(Math.abs(whole));
    if (n < 0 || Object.is(n, -0)) text = "-" + text;
    if (typedFraction !== undefined) text += decimal + typedFraction;
    else if (decimals !== null && decimals > 0 && !Number.isInteger(n)) text = new Intl.NumberFormat(locale, { minimumFractionDigits: 0, maximumFractionDigits: decimals }).format(n);
    else if (!Number.isInteger(n)) text = new Intl.NumberFormat(locale, { maximumFractionDigits: 20 }).format(n);
    return text;
  }
  const digitsBefore = (text, at) => (text.slice(0, at).match(/\d/g) ?? []).length;
  const caretAfter = (text, digits) => {
    if (digits <= 0) return text.startsWith("-") ? 1 : 0;
    let seen = 0;
    for (let i = 0; i < text.length; i++) if (/\d/.test(text[i]) && ++seen === digits) return i + 1;
    return text.length;
  };
  const clamp = (n) => {
    if (n === null) return null;
    if (min !== null && n < min) n = min;
    if (max !== null && n > max) n = max;
    if (decimals !== null) n = Math.round(n * 10 ** decimals) / 10 ** decimals;
    return n;
  };
  let last = value();
  const write = (n) => {
    last = n;
    if (cfg.bind && api.state[cfg.bind] !== n) api.set(cfg.bind, n);
    input.setAttribute("aria-invalid", (min !== null && n !== null && n < min) || (max !== null && n !== null && n > max) ? "true" : "false");
    api.emit("change", { value: n });
  };

  input.addEventListener("input", () => {
    const text = input.value, at = input.selectionStart ?? text.length;
    const before = digitsBefore(text, at);
    const n = parse(text);
    // Keep a decimal separator and the fraction as typed ("12." or "12.50").
    const sepAt = text.lastIndexOf(decimal);
    const allowFraction = decimals === null || decimals > 0;
    const typedFraction = allowFraction && sepAt >= 0 ? text.slice(sepAt + decimal.length).replace(/\D/g, "").slice(0, decimals ?? 20) : undefined;
    const negOnly = /^\s*-\s*$/.test(text);
    const next = negOnly ? "-" : show(n === null ? null : n, n === null ? undefined : typedFraction);
    input.value = next;
    const caret = caretAfter(next, before);
    input.setSelectionRange(caret, caret);
    write(negOnly ? null : parse(next));
  });
  input.addEventListener("blur", () => {
    const n = clamp(parse(input.value));
    input.value = show(n);
    write(n);
  });
  const nudge = (k) => {
    const n = clamp((value() ?? (min !== null && min > 0 ? min - step : 0)) + k * step);
    input.value = show(n);
    write(n);
  };
  input.addEventListener("keydown", (e) => {
    if (e.key === "ArrowUp" || e.key === "ArrowDown") {
      e.preventDefault();
      nudge((e.key === "ArrowUp" ? 1 : -1) * (e.shiftKey ? 10 : 1));
    }
  });
  el.addEventListener("click", (e) => {
    const b = e.target.closest("[data-num-step]");
    if (!b || b.disabled) return;
    nudge(Number(b.getAttribute("data-num-step")));
  });

  input.value = show(value());
  return {
    update() {
      const v = value();
      if (v === last) return;
      last = v;
      // From outside: shown as it is, unless it's what's being typed.
      if (document.activeElement !== input || parse(input.value) !== v) input.value = show(v);
    },
  };
};
