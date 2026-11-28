// irender-ui-kit: the `ui-color-picker` behaviour. The server renders the frame; this
// keeps the colour in state as text in the chosen format, and the area, sliders, text
// field, swatches and button in step with it. The colour is held as hue, saturation,
// brightness and opacity, so the hue stays put while the colour is grey or black.
// Colours reach CSS only through `style.setProperty`, which drops anything that isn't one.
(() => {
  const B = (window.__irenderBehaviors ||= {});
  if (B["ui-color-picker"]) return;

  const clamp = (n, lo, hi) => Math.min(hi, Math.max(lo, n));
  const round = (n, d = 0) => Math.round(n * 10 ** d) / 10 ** d;
  const L = (text, vars) => window.__irender.label(text, vars);

  // ---- conversions, all on 0–255 channels, 0–360 hue and 0–1 for the rest
  function hsvToRgb({ h, s, v }) {
    const f = (n) => {
      const k = (n + h / 60) % 6;
      return v - v * s * Math.max(0, Math.min(k, 4 - k, 1));
    };
    return { r: f(5) * 255, g: f(3) * 255, b: f(1) * 255 };
  }
  function rgbToHsv({ r, g, b }) {
    [r, g, b] = [r / 255, g / 255, b / 255];
    const max = Math.max(r, g, b), d = max - Math.min(r, g, b);
    const h = d === 0 ? null : max === r ? ((g - b) / d) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
    return { h: h === null ? null : (h * 60 + 360) % 360, s: max === 0 ? 0 : d / max, v: max };
  }
  function rgbToHsl({ r, g, b }) {
    [r, g, b] = [r / 255, g / 255, b / 255];
    const max = Math.max(r, g, b), min = Math.min(r, g, b), l = (max + min) / 2, d = max - min;
    const s = d === 0 ? 0 : d / (1 - Math.abs(2 * l - 1));
    const { h } = rgbToHsv({ r: r * 255, g: g * 255, b: b * 255 });
    return { h: h ?? 0, s, l };
  }
  const hex2 = (n) => Math.round(clamp(n, 0, 255)).toString(16).padStart(2, "0");

  // Any CSS colour the browser knows (names, hex, rgb, hsl, …) as channels, through a
  // canvas, which writes back `#rrggbb` or `rgba(r, g, b, a)`; null if it isn't one.
  let ctx = null;
  function parse(text) {
    if (typeof text !== "string" || !text.trim()) return null;
    ctx ||= document.createElement("canvas").getContext("2d");
    const out = [];
    // Two different starting colours: a colour it didn't take leaves each as it was.
    for (const start of ["#010203", "#040506"]) {
      ctx.fillStyle = start;
      ctx.fillStyle = text.trim();
      out.push(ctx.fillStyle);
    }
    if (out[0] !== out[1]) return null;
    const c = out[0];
    let m = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(c);
    if (m) return { r: parseInt(m[1], 16), g: parseInt(m[2], 16), b: parseInt(m[3], 16), a: 1 };
    m = /^rgba?\(([\d.]+),\s*([\d.]+),\s*([\d.]+)(?:,\s*([\d.]+))?\)$/.exec(c);
    if (m) return { r: +m[1], g: +m[2], b: +m[3], a: m[4] === undefined ? 1 : +m[4] };
    return null;
  }

  function start(el, api) {
    const cfg = api.config;
    const $ = (s) => el.querySelector(s);
    const area = $("[data-color-area]"), hueIn = $("[data-color-hue]"), alphaIn = $("[data-color-alpha]");
    const field = $("[data-color-text]"), dropper = $("[data-color-eyedropper]");
    const trigger = $("[data-color-trigger]"), pop = $("[data-color-pop]"), clearX = $("[data-color-x]");
    const live = $("[data-color-live]"), hidden = el.querySelector(":scope > input[type=hidden]");
    const swatches = [...el.querySelectorAll("[data-swatch]")];
    const recentBox = $("[data-color-recent]"), contrastBox = $("[data-color-contrast]");
    const K = window.__irenderKit;

    // ---- the colours picked last (`recent`), kept in the browser per `recentKey`
    const SLOT = `irender:colors:${cfg.recentKey ?? ""}`;
    const loadRecent = () => {
      try {
        const v = JSON.parse(localStorage.getItem(SLOT) ?? "[]");
        return Array.isArray(v) ? v.filter((x) => typeof x === "string" && parse(x)) : [];
      } catch {
        return [];
      }
    };
    let recentTimer = 0;
    function remember() {
      clearTimeout(recentTimer);
      if (!cfg.recent || !has) return;
      const text = write(color), p = parse(text);
      const sameAs = (x) => {
        const q = parse(x);
        return q && Math.round(q.r) === Math.round(p.r) && Math.round(q.g) === Math.round(p.g) && Math.round(q.b) === Math.round(p.b) && round(q.a, 2) === round(p.a, 2);
      };
      const list = [text, ...loadRecent().filter((x) => !sameAs(x))].slice(0, cfg.recent);
      try {
        localStorage.setItem(SLOT, JSON.stringify(list));
      } catch { /* not kept: shown for now */ }
      drawRecent(list);
    }
    // Labels: "Recent: {color}"
    function drawRecent(list = loadRecent()) {
      if (!recentBox) return;
      recentBox.hidden = !list.length;
      recentBox.replaceChildren(...list.map((v) => {
        const b = document.createElement("button");
        b.type = "button";
        b.setAttribute("data-recent", v);
        b.setAttribute("aria-label", L("Recent: {color}", { color: v }));
        b.title = v;
        b.style.setProperty("--swatch", `linear-gradient(${v}, ${v})`);
        return b;
      }));
    }
    drawRecent();

    // ---- contrast against `contrastWith`, as WCAG reads it (the colour over it when see-through)
    const lum = ({ r, g, b }) => {
      const ch = (v) => ((v /= 255) <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4);
      return 0.2126 * ch(r) + 0.7152 * ch(g) + 0.0722 * ch(b);
    };
    // Labels: "Contrast {ratio}:1", "AAA", "AA", "AA large", "Too low"
    function drawContrast() {
      if (!contrastBox) return;
      const bg = parse(K?.value(api, cfg.contrastWith));
      if (!has || !bg) return contrastBox.replaceChildren();
      const fg = hsvToRgb(color), a = cfg.alpha ? color.a : 1;
      const over = { r: fg.r * a + bg.r * (1 - a), g: fg.g * a + bg.g * (1 - a), b: fg.b * a + bg.b * (1 - a) };
      const [l1, l2] = [lum(over), lum(bg)].sort((x, y) => y - x);
      const ratio = (l1 + 0.05) / (l2 + 0.05), shown = Math.floor(ratio * 100) / 100;
      const grade = ratio >= 7 ? "AAA" : ratio >= 4.5 ? "AA" : ratio >= 3 ? "AA large" : null;
      const sample = document.createElement("span");
      sample.setAttribute("data-sample", "");
      sample.setAttribute("aria-hidden", "true");
      sample.textContent = "Aa";
      sample.style.background = `rgb(${bg.r} ${bg.g} ${bg.b})`;
      sample.style.color = `rgb(${Math.round(over.r)} ${Math.round(over.g)} ${Math.round(over.b)})`;
      const badge = document.createElement("span");
      badge.setAttribute("data-grade", grade ?? "fail");
      badge.textContent = grade ? L(grade) : L("Too low");
      contrastBox.replaceChildren(sample, L("Contrast {ratio}:1", { ratio: shown.toFixed(2).replace(/\.?0+$/, "") }), badge);
      contrastBox.setAttribute("data-ratio", String(shown));
    }

    // ---- the colour as text, in the format asked for
    function write(c) {
      const rgb = hsvToRgb(c), a = cfg.alpha ? round(c.a, 2) : 1;
      if (cfg.format === "rgb") {
        const [r, g, b] = [rgb.r, rgb.g, rgb.b].map(Math.round);
        return a < 1 ? `rgba(${r}, ${g}, ${b}, ${a})` : `rgb(${r}, ${g}, ${b})`;
      }
      if (cfg.format === "hsl") {
        const { h, s, l } = rgbToHsl(rgb);
        // The hue picked, not the one worked back from the channels.
        const parts = `${Math.round(s === 0 ? c.h : h) % 360}, ${round(s * 100)}%, ${round(l * 100)}%`;
        return a < 1 ? `hsla(${parts}, ${a})` : `hsl(${parts})`;
      }
      return `#${hex2(rgb.r)}${hex2(rgb.g)}${hex2(rgb.b)}${a < 1 ? hex2(a * 255) : ""}`;
    }

    // ---- the value: in state when bound, here otherwise
    let local = cfg.value ?? null;
    const raw = () => (cfg.bind ? api.state[cfg.bind] : local);
    let color = { h: 0, s: 0, v: 0, a: 1 }, has = false, written = undefined;
    // State into the colour, unless it's what this wrote (which keeps the hue).
    function read() {
      const v = raw();
      if (v === written && has) return;
      const p = parse(v);
      has = !!p;
      if (p) {
        const hsv = rgbToHsv(p);
        color = { h: hsv.h ?? color.h, s: hsv.s, v: hsv.v, a: cfg.alpha ? p.a : 1 };
      }
      written = v;
    }
    function set(c, say) {
      color = c;
      has = true;
      const out = write(c);
      written = out;
      if (cfg.bind) api.set(cfg.bind, out);
      else local = out;
      draw();
      api.emit("change", { value: out });
      if (say && live) live.textContent = say;
      // Remembered once it settles: not every step of a drag.
      clearTimeout(recentTimer);
      if (cfg.recent) recentTimer = setTimeout(remember, 800);
    }
    function clear() {
      has = false;
      written = null;
      if (cfg.bind) api.set(cfg.bind, null);
      else local = null;
      draw();
      api.emit("change", { value: null });
      if (live) live.textContent = L("Cleared");
    }

    // ---- drawing
    const css = (target, name, value) => target?.style.setProperty(name, value);
    function draw() {
      const text = has ? write(color) : "";
      const rgb = hsvToRgb(color);
      const solid = `rgb(${rgb.r} ${rgb.g} ${rgb.b})`;
      css(el, "--hue", String(round(color.h, 1)));
      css(el, "--sat", String(color.s * 100));
      css(el, "--bri", String(color.v * 100));
      css(el, "--opaque", solid);
      if (area) {
        area.setAttribute("aria-valuetext", L("Saturation {s}%, brightness {v}%", { s: Math.round(color.s * 100), v: Math.round(color.v * 100) }));
        area.setAttribute("aria-valuenow", String(Math.round(color.s * 100)));
      }
      if (hueIn && document.activeElement !== hueIn) hueIn.value = String(Math.round(color.h));
      if (alphaIn && document.activeElement !== alphaIn) alphaIn.value = String(Math.round(color.a * 100));
      if (field && !field.__dirty) field.value = text;
      const chip = $("[data-color-chip]");
      if (chip) {
        chip.style.removeProperty("--swatch");
        if (has) css(chip, "--swatch", `linear-gradient(${text}, ${text})`);
      }
      const shown = $("[data-color-value]");
      if (shown) shown.textContent = text || shown.getAttribute("data-placeholder");
      el.toggleAttribute("data-empty", !has);
      if (clearX) clearX.hidden = !has;
      if (hidden) hidden.value = text;
      for (const b of swatches) {
        const p = parse(cfg.swatches[+b.dataset.swatch].value);
        const same = has && p && Math.round(p.r) === Math.round(rgb.r) && Math.round(p.g) === Math.round(rgb.g) && Math.round(p.b) === Math.round(rgb.b) && (!cfg.alpha || round(p.a, 2) === round(color.a, 2));
        b.setAttribute("aria-pressed", same ? "true" : "false");
      }
      drawContrast();
    }
    // The swatches' own colours, once.
    for (const b of swatches) {
      const v = cfg.swatches[+b.dataset.swatch].value;
      if (parse(v)) css(b, "--swatch", `linear-gradient(${v}, ${v})`);
    }

    // ---- the area: the pointer, and the arrow keys (Shift: ten at a time)
    function fromPointer(e) {
      const r = area.getBoundingClientRect();
      set({ ...color, s: clamp((e.clientX - r.left) / r.width, 0, 1), v: clamp(1 - (e.clientY - r.top) / r.height, 0, 1) });
    }
    let dragging = null;
    area?.addEventListener("pointerdown", (e) => {
      if (e.button !== 0) return;
      e.preventDefault();
      area.focus();
      dragging = e.pointerId;
      try {
        area.setPointerCapture(e.pointerId);
      } catch {}
      fromPointer(e);
    });
    area?.addEventListener("pointermove", (e) => {
      if (dragging === e.pointerId) fromPointer(e);
    });
    for (const type of ["pointerup", "pointercancel", "lostpointercapture"]) area?.addEventListener(type, () => (dragging = null));
    area?.addEventListener("keydown", (e) => {
      const n = (e.shiftKey ? 10 : 1) / 100;
      const step = { ArrowLeft: [-n, 0], ArrowRight: [n, 0], ArrowUp: [0, n], ArrowDown: [0, -n] }[e.key];
      if (!step) return;
      e.preventDefault();
      // Right to left, left is more saturated.
      const dir = getComputedStyle(area).direction === "rtl" ? -1 : 1;
      set({ ...color, s: clamp(color.s + step[0] * dir, 0, 1), v: clamp(color.v + step[1], 0, 1) });
    });
    hueIn?.addEventListener("input", () => set({ ...color, h: +hueIn.value }));
    alphaIn?.addEventListener("input", () => set({ ...color, a: +alphaIn.value / 100 }));

    // ---- typing any CSS colour: taken on Enter, on leaving the field, or as soon as it's one
    function commit(final) {
      if (!field) return;
      const text = field.value.trim();
      if (!text) {
        field.__dirty = false;
        field.removeAttribute("aria-invalid");
        if (final && has) clear();
        return;
      }
      const p = parse(text);
      if (!p) {
        if (final) {
          field.setAttribute("aria-invalid", "true");
          if (live) live.textContent = L("Enter a colour like #1e3a8a or rgb(30, 58, 138)");
        }
        return;
      }
      field.removeAttribute("aria-invalid");
      const hsv = rgbToHsv(p);
      if (final) field.__dirty = false;
      set({ h: hsv.h ?? color.h, s: hsv.s, v: hsv.v, a: cfg.alpha ? p.a : 1 });
    }
    field?.addEventListener("input", () => {
      field.__dirty = true;
      commit(false);
    });
    field?.addEventListener("change", () => commit(true));
    field?.addEventListener("keydown", (e) => {
      if (e.key === "Enter") {
        e.preventDefault();
        commit(true);
        draw();
      } else if (e.key === "Escape" && field.__dirty) {
        e.preventDefault();
        e.stopPropagation();
        field.__dirty = false;
        field.removeAttribute("aria-invalid");
        draw();
      }
    });
    field?.addEventListener("blur", () => {
      commit(true);
      draw();
    });

    // ---- the eyedropper, where the browser has one
    if (dropper && "EyeDropper" in window) {
      dropper.hidden = false;
      dropper.addEventListener("click", async () => {
        try {
          const { sRGBHex } = await new window.EyeDropper().open();
          const p = parse(sRGBHex);
          if (!p) return;
          const hsv = rgbToHsv(p);
          set({ h: hsv.h ?? color.h, s: hsv.s, v: hsv.v, a: color.a }, L("{color} picked", { color: sRGBHex }));
        } catch {}
      });
    }

    // ---- swatches, the clear buttons and the popover
    el.addEventListener("click", (e) => {
      const b = e.target.closest?.("[data-swatch], [data-recent], [data-color-clear], [data-color-x]");
      if (!b || !el.contains(b)) return;
      if (b.hasAttribute("data-recent")) {
        const v = b.getAttribute("data-recent"), p = parse(v);
        if (!p) return;
        const hsv = rgbToHsv(p);
        set({ h: hsv.h ?? color.h, s: hsv.s, v: hsv.v, a: cfg.alpha ? p.a : 1 }, L("{color} picked", { color: v }));
        return;
      }
      if (b.hasAttribute("data-swatch")) {
        const sw = cfg.swatches[+b.dataset.swatch], p = parse(sw.value);
        if (!p) return;
        const hsv = rgbToHsv(p);
        set({ h: hsv.h ?? color.h, s: hsv.s, v: hsv.v, a: cfg.alpha ? p.a : 1 }, L("{color} picked", { color: sw.label }));
        // Only swatches: a pick is the whole job.
        if (pop && !cfg.custom) close();
        return;
      }
      clear();
      if (b.hasAttribute("data-color-x")) trigger?.focus();
    });
    function close() {
      try {
        pop.hidePopover();
      } catch {}
      trigger?.focus();
    }
    pop?.addEventListener("toggle", (e) => {
      if (e.newState !== "open") return remember();
      read();
      draw();
      // Into the panel: the area, or the chosen swatch (else the first).
      const target = area || swatches.find((b) => b.getAttribute("aria-pressed") === "true") || swatches[0];
      target?.focus({ preventScroll: true });
    });

    read();
    draw();
    return {
      update() {
        read();
        draw();
      },
    };
  }

  B["ui-color-picker"] = start;
})();
