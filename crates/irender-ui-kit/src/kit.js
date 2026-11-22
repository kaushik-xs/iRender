// irender-ui-kit: what several behaviours share (`window.__irenderKit`). Everything it
// makes is an element or text, never HTML, apart from its own icons.
(() => {
  if (window.__irenderKit) return;
  const isObj = (v) => v !== null && typeof v === "object" && !Array.isArray(v);
  const blank = (v) => v === null || v === undefined || v === "";
  // An element with attributes (`null` and `false` ones left out, functions as listeners)
  // and children (text, elements or lists of them).
  function make(tag, attrs, ...kids) {
    const e = document.createElement(tag);
    for (const [k, v] of Object.entries(attrs ?? {})) {
      if (typeof v === "function") e.addEventListener(k, v);
      else if (v !== null && v !== undefined && v !== false) e.setAttribute(k, v === true ? "" : String(v));
    }
    for (const c of kids.flat(Infinity)) if (c !== null && c !== undefined && c !== false) e.append(c instanceof Node ? c : String(c));
    return e;
  }
  const PATHS = {
    x: "M4 4l8 8M12 4l-8 8", check: "m3.5 8.5 3 3 6-7", plus: "M8 3v10M3 8h10", minus: "M3 8h10", up: "M8 13V3M4 7l4-4 4 4", down: "M8 3v10M4 9l4 4 4-4",
    chevron: "M4 6l4 4 4-4", left: "M10 3.5 5.5 8l4.5 4.5", right: "M6 3.5 10.5 8 6 12.5", copy: "M5.5 5.5h8v8h-8zM10.5 2.5h-8v8", trash: "M3 4.5h10M6.5 4.5V3h3v1.5M4.5 4.5l.6 8.5h5.8l.6-8.5M7 7v4M9 7v4",
    edit: "M11 2.8 13.2 5 5.5 12.7 2.5 13.5l.8-3zM9.5 4.3l2.2 2.2", reply: "M6.5 4 2.5 8l4 4M2.5 8h7a4 4 0 0 1 4 4v.5", smile: "M8 14A6 6 0 1 0 8 2a6 6 0 0 0 0 12zM5.5 9.5a3 3 0 0 0 5 0M6 6.2h.01M10 6.2h.01",
    search: "M7 11.5a4.5 4.5 0 1 0 0-9 4.5 4.5 0 0 0 0 9zM10.5 10.5l3 3", grip: "M6 3.5h.01M10 3.5h.01M6 8h.01M10 8h.01M6 12.5h.01M10 12.5h.01", more: "M3.5 8h.01M8 8h.01M12.5 8h.01",
    zoomin: "M7 11.5a4.5 4.5 0 1 0 0-9 4.5 4.5 0 0 0 0 9zM10.5 10.5l3 3M7 5v4M5 7h4", zoomout: "M7 11.5a4.5 4.5 0 1 0 0-9 4.5 4.5 0 0 0 0 9zM10.5 10.5l3 3M5 7h4", fit: "M2.5 6v-3.5H6M10 2.5h3.5V6M13.5 10v3.5H10M6 13.5H2.5V10",
    rotate: "M13 8a5 5 0 1 1-1.6-3.7M13 2.5v3h-3", download: "M8 2.5v8M4.5 7 8 10.5 11.5 7M3 13.5h10", clock: "M8 14A6 6 0 1 0 8 2a6 6 0 0 0 0 12zM8 4.8V8l2.3 1.6", play: "M5 3l8 5-8 5z", pause: "M5 3v10M11 3v10",
    cursor: "M3.5 2.5l9.5 4.2-4.2 1.3-1.6 4.3z", camera: "M2 5h2.6l1.2-1.8h4.4L11.4 5H14v8H2zM8 11.2a2.4 2.4 0 1 0 0-4.8 2.4 2.4 0 0 0 0 4.8z", mic: "M8 9.8a2 2 0 0 0 2-2V4a2 2 0 1 0-4 0v3.8a2 2 0 0 0 2 2zM3.8 7.6a4.2 4.2 0 0 0 8.4 0M8 11.8V14", video: "M1.8 4.5h8.4v7H1.8zM10.2 7l4-2v6l-4-2z", pin: "M8 14s4.5-4.3 4.5-7.6A4.5 4.5 0 0 0 8 2a4.5 4.5 0 0 0-4.5 4.4C3.5 9.7 8 14 8 14zM8 8a1.6 1.6 0 1 0 0-3.2A1.6 1.6 0 0 0 8 8z",
    bell: "M8 2a3.8 3.8 0 0 0-3.8 3.8c0 3.6-1.7 4.7-1.7 4.7h11s-1.7-1.1-1.7-4.7A3.8 3.8 0 0 0 8 2zM6.8 13.2a1.3 1.3 0 0 0 2.4 0", lock: "M3.5 7.2h9v6.3h-9zM5.2 7.2V5a2.8 2.8 0 0 1 5.6 0v2.2", key: "M6 10a3.2 3.2 0 1 0-3-3.2A3.2 3.2 0 0 0 6 10zM8.5 8.3 14 13.5M11.5 11.2l1.5-1.5", share: "M8 10V2M5 4.7 8 2l3 2.7M3.5 8v5.5h9V8", bolt: "M9 1.8 3.5 9H8l-1 5.2L12.5 7H8z",
    grid: "M2.5 2.5h11v11h-11zM2.5 6.2h11M2.5 9.8h11M6.2 2.5v11M9.8 2.5v11", flip: "M13 6A5.2 5.2 0 0 0 3.5 5M3 2.5V5h2.5M3 10a5.2 5.2 0 0 0 9.5 1M13 13.5V11h-2.5", stop: "M4 4h8v8H4z", text: "M3 3.5h10M8 3.5v9.5", arrow: "M3 13 13 3M7 3h6v6", square: "M3 3h10v10H3z", circle: "M8 13.5a5.5 5.5 0 1 0 0-11 5.5 5.5 0 0 0 0 11z", pencil: "M2.5 13.5c2-3 3-1 4.5-3S9 5 13.5 2.5", ruler: "M2 10.5 10.5 2 14 5.5 5.5 14zM4.8 7.7l1.5 1.5M7 5.5l1.5 1.5M9.2 3.3l1.5 1.5",
    blur: "M8 2v.01M4 4v.01M12 4v.01M2.5 8v.01M8 8v.01M13.5 8v.01M4 12v.01M12 12v.01M8 14v.01", undo: "M5.5 3 2.5 6l3 3M2.5 6h7a4 4 0 0 1 0 8H6", redo: "M10.5 3l3 3-3 3M13.5 6h-7a4 4 0 0 0 0 8H10", cloud: "M4.5 12.5a3 3 0 0 1-.3-6 4 4 0 0 1 7.7 1 2.6 2.6 0 0 1-.4 5z", off: "M2 2l12 12M4.5 12.5a3 3 0 0 1-.3-6M6.8 4.3a4 4 0 0 1 5.1 3.2 2.6 2.6 0 0 1 1.2 4.2", phone: "M4.5 1.8h7v12.4h-7zM7.3 12.2h1.4",
    user: "M8 8a2.8 2.8 0 1 0 0-5.6A2.8 2.8 0 0 0 8 8zM2.8 13.8a5.2 5.2 0 0 1 10.4 0", file: "M4 1.8h5l3 3v9.4H4zM9 1.8v3h3", link: "M6.5 9.5l3-3M7 4.5l1.2-1.2a3 3 0 0 1 4.3 4.3L11.3 8.8M9 11.5l-1.2 1.2a3 3 0 0 1-4.3-4.3L4.7 7.2", sun: "M8 11a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM8 1.5v1M8 13.5v1M1.5 8h1M13.5 8h1M3.4 3.4l.7.7M11.9 11.9l.7.7M3.4 12.6l.7-.7M11.9 4.1l.7-.7", screen: "M2 3h12v8H2zM6 13.5h4M8 11v2.5", speaker: "M2.5 6v4h2.7L9 13V3L5.2 6zM11 5.8a3 3 0 0 1 0 4.4M12.8 4a5.5 5.5 0 0 1 0 8", sparkle: "M8 2l1.5 4.5L14 8l-4.5 1.5L8 14l-1.5-4.5L2 8l4.5-1.5z", compass: "M8 14A6 6 0 1 0 8 2a6 6 0 0 0 0 12zM10.5 5.5 9 9l-3.5 1.5L7 7z", nfc: "M4 5.5a4 4 0 0 1 0 5M6.8 3.8a6.5 6.5 0 0 1 0 8.4M9.6 2.2a9.5 9.5 0 0 1 0 11.6", bluetooth: "M4.5 5l7 6L8 14V2l3.5 3-7 6", usb: "M8 14V2M6.3 3.7 8 2l1.7 1.7M4.5 6.5V8.5L8 10.5M11.5 5.5v2L8 9.5", scan: "M2.5 5.5v-3h3M10.5 2.5h3v3M13.5 10.5v3h-3M5.5 13.5h-3v-3M4.5 8h7", refresh: "M13 8a5 5 0 1 1-1.6-3.7M13 2.5v3h-3", warn: "M8 2.5 14.5 13.5h-13zM8 6.5v3.5M8 12h.01", info: "M8 14A6 6 0 1 0 8 2a6 6 0 0 0 0 12zM8 7.5V11M8 5.2h.01", back: "M13 8H3M7 4 3 8l4 4", polygon: "M8 2l6 4.5L11.7 13.5H4.3L2 6.5z",
  };
  function icon(name, size = 16) {
    const s = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    for (const [k, v] of Object.entries({ viewBox: "0 0 16 16", fill: "none", stroke: "currentColor", "stroke-width": name === "grip" || name === "more" ? 2.2 : 1.5, "stroke-linecap": "round", "stroke-linejoin": "round", "aria-hidden": "true" })) s.setAttribute(k, v);
    s.style.width = s.style.height = `calc(${size}px * var(--size-icon, 1))`;
    s.style.flex = "none";
    if (name === "left" || name === "right" || name === "reply") s.setAttribute("data-flip", "");
    const p = document.createElementNS("http://www.w3.org/2000/svg", "path");
    p.setAttribute("d", PATHS[name] ?? "");
    s.append(p);
    return s;
  }
  const button = (attrs, ...kids) => make("button", { type: "button", ...attrs }, ...kids);
  // Config values with state in them (`{ "$irender": expression }`, at any depth), now.
  const env = (api) => ({ ...api.state, ...(api.scope() ?? {}) });
  const has = (v) => (Array.isArray(v) ? v.some(has) : isObj(v) && ("$irender" in v || Object.values(v).some(has)));
  const value = (api, v) => !has(v) ? v : Array.isArray(v) ? v.map((x) => value(api, x)) : "$irender" in v ? window.__irender.evaluate(v.$irender, env(api)) : Object.fromEntries(Object.entries(v).map(([k, x]) => [k, value(api, x)]));
  // A number in a locale: `number`, `currency`, `percent`, `compact` (1.2M) or `bytes`.
  function format(v, o = {}) {
    if (blank(v)) return "";
    const n = Number(v);
    if (typeof v === "boolean" || String(v).trim() === "" || !Number.isFinite(n)) return String(v);
    if (o.format === "none") return String(v);
    const opt = o.format === "currency" || o.currency ? { style: "currency", currency: o.currency || "USD" } : o.format === "percent" ? { style: "percent" } : {};
    if (o.format === "compact" || o.compact) Object.assign(opt, { notation: "compact", maximumFractionDigits: 1 });
    if (o.decimals != null) opt.minimumFractionDigits = opt.maximumFractionDigits = Number(o.decimals);
    return new Intl.NumberFormat(o.locale || undefined, opt).format(n);
  }
  // "5 minutes ago", "yesterday", then the date.
  function ago(when, locale, now = Date.now()) {
    const t = new Date(when);
    if (blank(when) || isNaN(t)) return String(when ?? "");
    const s = (t - now) / 1000, a = Math.abs(s);
    if (a >= 7 * 86400) return new Intl.DateTimeFormat(locale || undefined, { dateStyle: "medium" }).format(t);
    const [n, unit] = a < 45 ? [0, "second"] : a < 3600 ? [Math.round(s / 60), "minute"] : a < 86400 ? [Math.round(s / 3600), "hour"] : [Math.round(s / 86400), "day"];
    return new Intl.RelativeTimeFormat(locale || undefined, { numeric: "auto" }).format(n, unit);
  }
  // The days an event that repeats falls on, as days from its first, within `from` to `to`
  // (both in days from it). `repeat`: { every: day|week|month|year, interval, on: [weekdays
  // 0 to 6, for weeks], until, count, except: [dates] }; `first` is its first day.
  const MAX = 1000;
  function repeats(repeat, first, from, to) {
    const r = isObj(repeat) ? repeat : { every: repeat }, step = Math.max(1, Math.trunc(Number(r.interval) || 1));
    const iso = (d) => `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
    const day = (n) => new Date(first.getFullYear(), first.getMonth(), first.getDate() + n);
    const until = r.until ? String(r.until).slice(0, 10) : null, except = new Set((Array.isArray(r.except) ? r.except : []).map((d) => String(d).slice(0, 10)));
    const limit = Number(r.count) > 0 ? Number(r.count) : Infinity, out = [];
    let seen = 0;
    // Counted from the first, so `count` holds; false once there are no more to come.
    const take = (n) => {
      const d = iso(day(n));
      if (n > to || (until && d > until) || seen >= limit) return false;
      seen++;
      if (n >= from && !except.has(d)) out.push(n);
      return true;
    };
    const diff = (d) => Math.round((Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()) - Date.UTC(first.getFullYear(), first.getMonth(), first.getDate())) / 864e5);
    if (r.every === "week") {
      const on = Array.isArray(r.on) && r.on.length ? [...new Set(r.on.map(Number))].filter((x) => x >= 0 && x < 7) : [first.getDay()];
      // Weeks from the first's own, Monday first.
      const monday = -((first.getDay() + 6) % 7), days = on.map((w) => (w + 6) % 7).sort((a, b) => a - b);
      for (let w = 0; w < MAX * 8 && out.length < MAX; w += step) {
        for (const x of days) {
          const n = monday + w * 7 + x;
          if (n >= 0 && !take(n)) return out;
        }
      }
    } else if (r.every === "month" || r.every === "year") {
      for (let k = 0; k < MAX * 4 && out.length < MAX; k += step) {
        const d = r.every === "month" ? new Date(first.getFullYear(), first.getMonth() + k, first.getDate()) : new Date(first.getFullYear() + k, first.getMonth(), first.getDate());
        // Months without that day are left out.
        if (d.getDate() !== first.getDate()) continue;
        if (!take(diff(d))) return out;
      }
    } else {
      for (let n = 0; n < 366 * 60 && out.length < MAX; n += step) if (!take(n)) return out;
    }
    return out;
  }
  const initials = (name) => {
    const w = String(name ?? "").trim().split(/\s+/).filter(Boolean);
    return (w.length > 1 ? [...w[0]][0] + [...w.at(-1)][0] : [...(w[0] ?? "")].slice(0, 2).join("")).toUpperCase();
  };
  const avatar = (who) => {
    const p = isObj(who) ? who : { name: who };
    const s = make("span", { "data-avatar": "", "aria-hidden": "true" });
    if (p.avatar && /^(https?:|\/|\.{0,2}\/|[\w-]+\/)/i.test(p.avatar)) s.append(make("img", { src: p.avatar, alt: "" }));
    else s.append(p.initials || initials(p.name));
    return s;
  };
  // Announced by a live region inside `el`.
  function say(el, text) {
    let live = el.querySelector(":scope > [data-kit-live]");
    if (!live) {
      live = make("div", { "data-kit-live": "", role: "status", style: "position:absolute;width:1px;height:1px;overflow:hidden;clip-path:inset(50%);white-space:nowrap" });
      el.append(live);
    }
    live.textContent = "";
    setTimeout(() => (live.textContent = text), 30);
  }
  // Text with its web addresses as links, for a comment or a log line.
  const linked = (text) => String(text).split(/(https?:\/\/[^\s<>"')\]]+[^\s<>"')\].,;:!?])/g).map((p, i) => (i % 2 ? make("a", { href: p, target: "_blank", rel: "noreferrer noopener" }, p) : p));
  const rtl = (el) => getComputedStyle(el).direction === "rtl";
  const id = () => globalThis.crypto?.randomUUID?.() ?? `k${Date.now().toString(36)}${Math.random().toString(36).slice(2, 8)}`;
  // Keeps a drag's pointer, where it's still there to keep.
  const capture = (el, e) => {
    try {
      el.setPointerCapture(e.pointerId);
    } catch { /* a pointer that's already gone */ }
  };
  const clamp = (v, lo, hi) => Math.min(hi, Math.max(lo, v));
  window.__irenderKit = { make, icon, button, value, env, format, ago, initials, avatar, say, linked, rtl, id, clamp, capture, repeats, isObj, blank };
})();
