// irender-ui-kit: what a typed cell takes and whether it's right, shared by the data
// table's editable cells and the import (`window.__irenderCells`). A cell's column is
// `{ type, options, required, min, max, pattern, unique, message }`.
(() => {
  if (window.__irenderCells) return;
  const L = (text, vars) => window.__irender.label(text, vars);
  const blank = (v) => v === null || v === undefined || v === "";
  const isNum = (v) => typeof v === "number" ? Number.isFinite(v) : typeof v === "string" && v.trim() !== "" && !isNaN(v);
  const isObj = (v) => v !== null && typeof v === "object" && !Array.isArray(v);
  // A choice is a value or `{ value, label }`.
  const choices = (c) => (Array.isArray(c.options) ? c.options : []).map((o) => (isObj(o) ? { value: o.value, label: String(o.label ?? o.value ?? "") } : { value: o, label: String(o ?? "") }));
  const fold = (s) => String(s ?? "").trim().normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();

  // "1,234.5", "₹ 1.234,50" (in a locale with decimal commas), "(120)" and "25%" as numbers;
  // null when it isn't one.
  const seps = {};
  function number(text, locale) {
    const parts = (seps[locale ?? ""] ||= new Intl.NumberFormat(locale || undefined).formatToParts(12345.6));
    const group = parts.find((p) => p.type === "group")?.value ?? ",", decimal = parts.find((p) => p.type === "decimal")?.value ?? ".";
    let s = String(text).trim().replace(/[\s  '’]/g, "");
    const percent = s.endsWith("%"), negative = /^\(.*\)$/.test(s) || /^[^\d]*[-−]/.test(s);
    s = s.replace(/^[^\d.,-]+|[^\d.,]+$/g, "").replace(/[-−]/g, "");
    // A lone "." is a decimal point in any locale unless it groups ("1.234" in de-DE).
    if (decimal === "," ? /,/.test(s) || /^\d{1,3}(\.\d{3})+$/.test(s) : true) s = s.split(group).join("").replace(decimal, ".");
    if (!/^(\d+\.?\d*|\.\d+)$/.test(s)) return null;
    const n = Number(s) * (negative ? -1 : 1);
    return percent ? Math.round(n * 1e10) / 1e12 : n;
  }
  // "2026-11-22", "22 Nov 2026" and the locale's own order, as `YYYY-MM-DD`; null otherwise.
  function date(text, locale) {
    const s = String(text).trim();
    const iso = (y, m, d) => {
      const t = new Date(Date.UTC(y, m - 1, d));
      return t.getUTCFullYear() === y && t.getUTCMonth() === m - 1 && t.getUTCDate() === d ? t.toISOString().slice(0, 10) : null;
    };
    let m = /^(\d{4})-(\d{1,2})-(\d{1,2})(?:[T ].*)?$/.exec(s);
    if (m) return iso(+m[1], +m[2], +m[3]);
    m = /^(\d{1,4})[/.\-\s](\d{1,2})[/.\-\s](\d{1,4})$/.exec(s);
    if (m) {
      const order = new Intl.DateTimeFormat(locale || undefined).formatToParts(new Date(2000, 0, 2)).map((p) => p.type).filter((t) => t === "day" || t === "month" || t === "year");
      const got = m[1].length === 4 ? { year: +m[1], month: +m[2], day: +m[3] } : Object.fromEntries(order.map((t, i) => [t, +m[i + 1]]));
      const year = got.year < 100 ? 2000 + got.year : got.year;
      return iso(year, got.month, got.day);
    }
    if (/[a-z]{3}/i.test(s) && /\d{4}/.test(s)) {
      const t = new Date(s);
      if (!isNaN(t)) return iso(t.getFullYear(), t.getMonth() + 1, t.getDate());
    }
    return null;
  }
  // Text typed or pasted into a cell as the column's value. What can't be read stays the
  // text it was, for `check` to flag, so nothing typed is lost.
  function parse(c, text, locale) {
    const s = String(text ?? "");
    if (s.trim() === "") return c.type === "checkbox" ? false : null;
    switch (c.type) {
      case "number": return number(s, locale) ?? s.trim();
      case "date": return date(s, locale) ?? s.trim();
      case "checkbox": return /^(true|yes|y|1|x|✓|✔|on)$/i.test(s.trim());
      case "select": {
        const all = choices(c), f = fold(s);
        return (all.find((o) => String(o.value) === s) ?? all.find((o) => fold(o.label) === f || fold(o.value) === f))?.value ?? s.trim();
      }
      case "multiline": return s;
      default: return s;
    }
  }
  // What's wrong with a value, or null. `uses` counts a unique column's values.
  function check(c, v, uses) {
    if (blank(v) || (c.required && c.type === "checkbox" && v === false)) return c.required ? L("Required") : null;
    if (c.type === "number") {
      if (!isNum(v)) return L("Not a number");
      if (c.min != null && Number(v) < Number(c.min)) return c.message || L("At least {n}", { n: Number(c.min) });
      if (c.max != null && Number(v) > Number(c.max)) return c.message || L("At most {n}", { n: Number(c.max) });
    } else if (c.type === "date") {
      if (!/^\d{4}-\d{2}-\d{2}/.test(String(v)) || isNaN(Date.parse(v))) return L("Not a date");
      if (c.min != null && String(v) < String(c.min)) return c.message || L("Not before {date}", { date: String(c.min) });
      if (c.max != null && String(v) > String(c.max)) return c.message || L("Not after {date}", { date: String(c.max) });
    } else if (c.type === "select" && choices(c).length && !choices(c).some((o) => o.value === v)) return L("Not one of the choices");
    if (c.pattern) {
      let re = null;
      try {
        re = new RegExp(`^(?:${c.pattern})$`, "u");
      } catch { /* a pattern the browser can't read checks nothing */ }
      if (re && !re.test(String(v))) return c.message || L("Not in the right format");
    }
    if (c.unique && uses && (uses.get(fold(v)) ?? 0) > 1) return L("Already used");
    return null;
  }
  // How often each value of a unique column is used, for `check`.
  const count = (values) => {
    const m = new Map();
    for (const v of values) if (!blank(v)) m.set(fold(v), (m.get(fold(v)) ?? 0) + 1);
    return m;
  };
  window.__irenderCells = { parse, check, count, choices, fold, number, date };
})();
