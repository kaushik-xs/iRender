// irender-ui-kit: `ui-signature`, `ui-image-cropper`, `ui-plan` and `ui-pdf`.
(() => {
  const B = (window.__irenderBehaviors ||= {});
  const K = window.__irenderKit, L = (t, v) => window.__irender.label(t, v);
  const { make, button, icon, isObj, blank } = K;
  const NS = "http://www.w3.org/2000/svg";
  const svg = (tag, attrs, ...kids) => {
    const e = document.createElementNS(NS, tag);
    for (const [k, v] of Object.entries(attrs ?? {})) if (v !== null && v !== undefined && v !== false) e.setAttribute(k, v === true ? "" : v);
    e.append(...kids.flat().filter((k) => k !== null && k !== undefined));
    return e;
  };
  const safe = (u) => typeof u === "string" && u !== "" && !/^\s*(javascript|vbscript):/i.test(u);
  const css = (c) => (typeof c === "string" && /^[#\w(),.%\s/-]+$/.test(c) ? c : null);

  // ---------------------------------------------------------------- signature
  B["ui-signature"] = (el, api) => {
    const cfg = api.config, methods = Array.isArray(cfg.methods) && cfg.methods.length ? cfg.methods : ["draw", "type"];
    let method = methods[0], strokes = [], last, w = 0, h = 0;
    const canvas = make("canvas", { role: "img", "aria-label": L("Signature pad: draw with a finger, a pen or the mouse") });
    const typed = make("input", { "data-sg-type": "", "aria-label": L("Type your name"), placeholder: L("Your name"), autocomplete: "name", hidden: true });
    const saved = make("img", { "data-sg-saved": "", alt: L("Signature"), hidden: true });
    const hint = make("span", { "data-sg-hint": "" });
    const pad = make("div", { "data-sg-pad": "" }, canvas, typed, saved, make("span", { "data-sg-line": "" }), hint);
    const undo = button({ "data-do": "undo", click: () => (strokes.pop(), paint(), save()) }, L("Undo"));
    const clear = button({ click: () => wipe(true) }, L("Clear"));
    const names = { draw: L("Draw"), type: L("Type") };
    const modes = methods.length > 1 ? methods.map((m) => button({ "data-mode": m, click: () => { method = m; wipe(true); (m === "type" ? typed : canvas).focus(); } }, names[m] ?? m)) : [];
    el.append(pad, make("div", { "data-m-bar": "" }, modes, make("span", { "data-push": "" }), undo, clear));
    const ink = () => css(K.value(api, cfg.color)) ?? "#18181b";
    const off = () => !!K.value(api, cfg.disabled);
    function size() {
      const b = canvas.getBoundingClientRect(), r = Math.max(1, devicePixelRatio || 1);
      if (!b.width || (canvas.width === Math.round(b.width * r) && canvas.height === Math.round(b.height * r))) return;
      // What's drawn keeps its place as the pad is made wider.
      w = b.width;
      h = b.height;
      canvas.width = Math.round(w * r);
      canvas.height = Math.round(h * r);
      canvas.getContext("2d").setTransform(r, 0, 0, r, 0, 0);
      paint();
    }
    // A stroke's outline as short curves, thinner where it was faster.
    function trace(g, s) {
      g.strokeStyle = g.fillStyle = ink();
      g.lineCap = g.lineJoin = "round";
      if (s.length === 1) return g.beginPath(), g.arc(s[0].x, s[0].y, 1.6, 0, 7), g.fill();
      for (let i = 1; i < s.length; i++) {
        const a = s[i - 1], b = s[i], z = s[i - 2] ?? a, m1 = { x: (z.x + a.x) / 2, y: (z.y + a.y) / 2 }, m2 = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
        g.beginPath();
        g.lineWidth = b.w;
        g.moveTo(i === 1 ? a.x : m1.x, i === 1 ? a.y : m1.y);
        g.quadraticCurveTo(a.x, a.y, i === s.length - 1 ? b.x : m2.x, i === s.length - 1 ? b.y : m2.y);
        g.stroke();
      }
    }
    function paint() {
      const g = canvas.getContext("2d");
      g.clearRect(0, 0, w, h);
      for (const s of strokes) trace(g, s);
    }
    // The ink alone, cut to its edges.
    function bounds() {
      const p = strokes.flat();
      const x0 = Math.min(...p.map((q) => q.x)) - 6, y0 = Math.min(...p.map((q) => q.y)) - 6, x1 = Math.max(...p.map((q) => q.x)) + 6, y1 = Math.max(...p.map((q) => q.y)) + 6;
      return { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
    }
    function picture() {
      if (method === "type") {
        const name = typed.value.trim();
        if (!name) return null;
        const c = make("canvas"), g = c.getContext("2d"), font = `84px ${getComputedStyle(typed).fontFamily}`;
        g.font = font;
        c.width = Math.ceil(g.measureText(name).width) + 48;
        c.height = 150;
        const g2 = c.getContext("2d");
        g2.font = font;
        g2.fillStyle = ink();
        g2.textBaseline = "middle";
        g2.fillText(name, 24, 75);
        return c.toDataURL("image/png");
      }
      if (!strokes.length) return null;
      const b = bounds();
      if (cfg.format === "svg") {
        const n = (v) => Math.round(v * 10) / 10;
        const paths = strokes.map((s) => (s.length === 1 ? `<circle cx="${n(s[0].x - b.x)}" cy="${n(s[0].y - b.y)}" r="1.6"/>` : `<path fill="none" d="M${s.map((p) => `${n(p.x - b.x)} ${n(p.y - b.y)}`).join("L")}"/>`)).join("");
        return `data:image/svg+xml;base64,${btoa(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${n(b.w)} ${n(b.h)}" width="${n(b.w)}" height="${n(b.h)}" stroke="${ink()}" fill="${ink()}" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round">${paths}</svg>`)}`;
      }
      const c = make("canvas", { width: Math.ceil(b.w * 2), height: Math.ceil(b.h * 2) }), g = c.getContext("2d");
      g.scale(2, 2);
      g.translate(-b.x, -b.y);
      for (const s of strokes) trace(g, s);
      return c.toDataURL("image/png");
    }
    function save() {
      last = picture();
      api.set(cfg.bind, last);
      if (cfg.bindMeta) api.set(cfg.bindMeta, last ? { method: method === "type" ? "typed" : "drawn", name: method === "type" ? typed.value.trim() : null, at: new Date().toISOString() } : null);
      api.emit("change", { value: last });
      draw();
    }
    function wipe(tell) {
      strokes = [];
      typed.value = "";
      paint();
      if (tell) save();
    }
    let s = null;
    canvas.tabIndex = 0;
    canvas.addEventListener("pointerdown", (e) => {
      if (e.button !== 0 || off()) return;
      e.preventDefault();
      size();
      K.capture(canvas, e);
      canvas.focus({ preventScroll: true });
      s = { id: e.pointerId, pts: [], t: e.timeStamp };
      strokes.push(s.pts);
      point(e);
    });
    function point(e) {
      const b = canvas.getBoundingClientRect(), x = e.clientX - b.left, y = e.clientY - b.top, p = s.pts.at(-1);
      const d = p ? Math.hypot(x - p.x, y - p.y) : 0;
      if (p && d < 1) return;
      const speed = p ? d / Math.max(1, e.timeStamp - s.t) : 0;
      const want = e.pointerType === "pen" && e.pressure ? 1 + e.pressure * 3 : K.clamp(3.4 - speed * 0.9, 1.3, 3.4);
      s.pts.push({ x, y, w: p ? p.w * 0.6 + want * 0.4 : 2.4 });
      s.t = e.timeStamp;
    }
    canvas.addEventListener("pointermove", (e) => {
      if (!s || e.pointerId !== s.id) return;
      for (const c of e.getCoalescedEvents?.().length ? e.getCoalescedEvents() : [e]) point(c);
      paint();
    });
    const end = (e) => {
      if (!s || e.pointerId !== s.id) return;
      s = null;
      paint();
      save();
    };
    canvas.addEventListener("pointerup", end);
    canvas.addEventListener("pointercancel", end);
    canvas.addEventListener("keydown", (e) => {
      if ((e.key === "z" && (e.metaKey || e.ctrlKey)) || e.key === "Backspace") e.preventDefault(), strokes.pop(), paint(), save();
    });
    typed.addEventListener("input", save);
    new ResizeObserver(size).observe(canvas);
    function draw() {
      const v = api.state[cfg.bind] ?? null;
      // Set from outside: shown as it is, until it's cleared.
      if (v !== (last ?? null)) {
        last = v;
        strokes = [];
        typed.value = "";
        paint();
      }
      const other = !!v && !strokes.length && !typed.value;
      saved.hidden = !other;
      if (other && safe(v) && saved.src !== v) saved.src = v;
      canvas.hidden = other || method !== "draw";
      typed.hidden = other || method !== "type";
      typed.disabled = off();
      el.toggleAttribute("data-off", off());
      hint.textContent = K.value(api, cfg.hint) ?? L("Sign here");
      hint.hidden = other;
      undo.hidden = method !== "draw" || other;
      undo.disabled = off() || !strokes.length;
      clear.disabled = off() || !v;
      for (const b of modes) b.setAttribute("aria-pressed", String(b.getAttribute("data-mode") === method)), (b.disabled = off());
    }
    size();
    draw();
    return { update: draw };
  };

  // ---------------------------------------------------------------- cropper
  B["ui-image-cropper"] = (el, api) => {
    const cfg = api.config, list = Array.isArray(cfg.ratio) ? cfg.ratio : [cfg.ratio ?? "free"];
    const num = (r) => {
      if (blank(r) || r === "free") return null;
      const m = /^\s*([\d.]+)\s*[/:]\s*([\d.]+)\s*$/.exec(String(r));
      const n = m ? Number(m[1]) / Number(m[2]) : Number(r);
      return n > 0 ? n : null;
    };
    let ratio = num(list[0]), img = null, rot = 0, flip = 1, zoom = 1, box = null, view = null, srcWas = null;
    const input = make("input", { type: "file", accept: "image/*", "aria-label": L("Choose a picture") });
    const drop = make("label", { "data-cr-drop": "" }, make("strong", null, L("Drop a picture here, or click to choose one")), input);
    const error = make("p", { "data-cr-error": "", role: "alert", hidden: true });
    const canvas = make("canvas", { "aria-hidden": "true" });
    const handles = ["nw", "ne", "sw", "se", "n", "s", "e", "w"].map((h) => make("span", { "data-cr-h": h }));
    const frame = make("div", { "data-cr-box": "", "data-round": cfg.round, tabindex: 0, role: "application", "aria-label": L("Crop: the arrow keys move it, with Shift they size it") }, handles);
    const stage = make("div", { "data-cr-stage": "" }, canvas, frame);
    const slider = make("input", { type: "range", min: 1, max: 4, step: 0.01, value: 1, "aria-label": L("Zoom") });
    const ratios = list.length > 1 ? list.map((r) => button({ "data-r": String(r), click: () => ((ratio = num(r)), reset(false), draw()) }, r === "free" ? L("Free") : String(r).replace(/\s*\/\s*/, ":"))) : [];
    const go = button({ click: crop }, icon("check", 14), L("Crop"));
    const bar = make("div", { "data-m-bar": "" }, ratios, button({ "aria-label": L("Turn"), title: L("Turn"), click: () => ((rot = (rot + 90) % 360), reset(false), draw()) }, icon("rotate", 15)), button({ click: () => ((flip = -flip), draw()) }, L("Flip")), slider, make("span", { "data-push": "" }), cfg.src === undefined ? button({ click: () => ((img = null), show()) }, L("Another")) : null, go);
    el.append(drop, error, stage, bar);
    const side = () => (rot % 180 ? [img.naturalHeight, img.naturalWidth] : [img.naturalWidth, img.naturalHeight]);
    // Where the turned picture is on the stage: its scale and corner.
    function place() {
      const b = stage.getBoundingClientRect(), [iw, ih] = side(), fit = Math.min((b.width - 24) / iw, (b.height - 24) / ih);
      view = { W: b.width, H: b.height, s: fit * zoom, iw, ih };
      view.x = (b.width - iw * view.s) / 2 + (view.px ?? pan.x);
      view.y = (b.height - ih * view.s) / 2 + pan.y;
    }
    const pan = { x: 0, y: 0 };
    // The crop, in parts of the turned picture.
    function reset(all = true) {
      if (!img) return;
      if (all) (rot = 0), (flip = 1), (zoom = 1), (slider.value = 1);
      pan.x = pan.y = 0;
      const [iw, ih] = side();
      let w = 1, h = 1;
      if (ratio) (iw / ih > ratio ? (w = (ih * ratio) / iw) : (h = iw / ratio / ih)), (w *= 0.92), (h *= 0.92);
      else (w = h = 0.86);
      box = { x: (1 - w) / 2, y: (1 - h) / 2, w, h };
    }
    function draw() {
      if (!img || !stage.clientWidth) return;
      place();
      const r = Math.max(1, devicePixelRatio || 1), g = canvas.getContext("2d");
      canvas.width = view.W * r;
      canvas.height = view.H * r;
      g.setTransform(r, 0, 0, r, 0, 0);
      paint(g, view.x, view.y, view.s);
      Object.assign(frame.style, { left: view.x + box.x * view.iw * view.s + "px", top: view.y + box.y * view.ih * view.s + "px", width: box.w * view.iw * view.s + "px", height: box.h * view.ih * view.s + "px" });
      for (const h of handles) {
        const k = h.getAttribute("data-cr-h");
        Object.assign(h.style, { left: k.includes("w") ? "0" : k.includes("e") ? "100%" : "50%", top: k.includes("n") ? "0" : k.includes("s") ? "100%" : "50%", cursor: `${k}-resize`, display: ratio && k.length === 1 ? "none" : "" });
      }
      for (const b of ratios) b.setAttribute("aria-pressed", String(num(b.getAttribute("data-r")) === ratio));
      if (cfg.bindCrop) {
        const c = { x: +box.x.toFixed(4), y: +box.y.toFixed(4), width: +box.w.toFixed(4), height: +box.h.toFixed(4), rotation: rot };
        if (JSON.stringify(api.state[cfg.bindCrop]) !== JSON.stringify(c)) api.set(cfg.bindCrop, c);
      }
    }
    function paint(g, x, y, s) {
      const [iw, ih] = side();
      g.save();
      g.translate(x + (iw * s) / 2, y + (ih * s) / 2);
      g.rotate((rot * Math.PI) / 180);
      g.scale(flip, 1);
      g.drawImage(img, (-img.naturalWidth * s) / 2, (-img.naturalHeight * s) / 2, img.naturalWidth * s, img.naturalHeight * s);
      g.restore();
    }
    // A crop moved or sized, kept in the picture and to its shape.
    function set(n, anchor) {
      const [iw, ih] = side(), least = 24 / Math.min(iw, ih);
      let { x, y, w, h } = n;
      if (anchor) {
        w = Math.max(least, w);
        h = Math.max(least, h);
        if (ratio) {
          h = (w * iw) / ratio / ih;
          const room = anchor.includes("n") ? box.y + box.h : 1 - box.y, roomX = anchor.includes("w") ? box.x + box.w : 1 - box.x;
          const k = Math.min(1, room / h, roomX / w);
          w *= k;
          h *= k;
        } else {
          w = Math.min(w, anchor.includes("w") ? box.x + box.w : 1 - box.x);
          h = Math.min(h, anchor.includes("n") ? box.y + box.h : 1 - box.y);
        }
        x = anchor.includes("w") ? box.x + box.w - w : box.x;
        y = anchor.includes("n") ? box.y + box.h - h : box.y;
      }
      box = { x: K.clamp(x, 0, 1 - w), y: K.clamp(y, 0, 1 - h), w, h };
      draw();
    }
    let drag = null;
    frame.addEventListener("pointerdown", (e) => {
      if (e.button !== 0) return;
      e.preventDefault();
      frame.focus({ preventScroll: true });
      K.capture(frame, e);
      drag = { id: e.pointerId, h: e.target.closest("[data-cr-h]")?.getAttribute("data-cr-h") ?? null, x: e.clientX, y: e.clientY, box: { ...box } };
    });
    frame.addEventListener("pointermove", (e) => {
      if (!drag || e.pointerId !== drag.id) return;
      const dx = (e.clientX - drag.x) / (view.iw * view.s), dy = (e.clientY - drag.y) / (view.ih * view.s), b = drag.box, k = drag.h;
      if (!k) return set({ ...b, x: b.x + dx, y: b.y + dy });
      box = { ...b };
      set({ w: k.includes("e") ? b.w + dx : k.includes("w") ? b.w - dx : b.w, h: k.includes("s") ? b.h + dy : k.includes("n") ? b.h - dy : b.h }, k);
    });
    for (const ev of ["pointerup", "pointercancel"]) frame.addEventListener(ev, () => (drag = null));
    // Dragging outside the crop moves the picture under it.
    let move = null;
    stage.addEventListener("pointerdown", (e) => {
      if (e.target.closest("[data-cr-box]") || e.button !== 0) return;
      K.capture(stage, e);
      move = { x: e.clientX - pan.x, y: e.clientY - pan.y };
    });
    stage.addEventListener("pointermove", (e) => move && ((pan.x = e.clientX - move.x), (pan.y = e.clientY - move.y), draw()));
    stage.addEventListener("pointerup", () => (move = null));
    stage.addEventListener("wheel", (e) => {
      if (!e.ctrlKey && !e.metaKey) return;
      e.preventDefault();
      zoom = K.clamp(zoom * Math.exp(-e.deltaY * 0.01), 1, 4);
      slider.value = zoom;
      draw();
    }, { passive: false });
    slider.addEventListener("input", () => ((zoom = Number(slider.value)), draw()));
    frame.addEventListener("keydown", (e) => {
      const d = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] }[e.key];
      if (e.key === "Enter") return e.preventDefault(), crop();
      if (!d) return;
      e.preventDefault();
      const by = e.altKey ? 0.002 : 0.01;
      if (e.shiftKey) set({ w: box.w + d[0] * by, h: box.h + d[1] * by + (ratio ? d[1] * by : 0) }, "se");
      else set({ ...box, x: box.x + d[0] * by, y: box.y + d[1] * by });
    });
    function crop() {
      if (!img) return;
      const [iw, ih] = side();
      let w = Math.round(box.w * iw);
      w = cfg.width ?? Math.min(w, cfg.maxWidth ?? 2048);
      const s = w / (box.w * iw), h = Math.max(1, Math.round(box.h * ih * s));
      const c = make("canvas", { width: w, height: h }), g = c.getContext("2d"), type = `image/${cfg.type ?? "jpeg"}`;
      if (type === "image/jpeg") (g.fillStyle = "#fff"), g.fillRect(0, 0, w, h);
      paint(g, -box.x * iw * s, -box.y * ih * s, s);
      let out;
      try {
        out = c.toDataURL(type, cfg.quality ?? 0.9);
      } catch {
        return fail(L("This picture's site doesn't let it be cropped here."));
      }
      api.set(cfg.bind, out);
      api.emit("crop", { value: out, width: w, height: h });
      K.say(el, L("Picture cropped"));
    }
    const fail = (t) => ((error.textContent = t ?? ""), (error.hidden = !t));
    function show() {
      drop.hidden = !!img || cfg.src !== undefined;
      stage.hidden = bar.hidden = !img;
      if (img) draw();
    }
    function load(src) {
      fail(null);
      const i = new Image();
      if (/^https?:/.test(src) && !src.startsWith(location.origin)) i.crossOrigin = "anonymous";
      i.onload = () => {
        img = i;
        reset();
        show();
      };
      i.onerror = () => fail(L("That isn't a picture that can be shown."));
      i.src = src;
    }
    const take = (f) => f && (f.type.startsWith("image/") ? load(URL.createObjectURL(f)) : fail(L("That isn't a picture that can be shown.")));
    input.addEventListener("change", () => (take(input.files[0]), (input.value = "")));
    drop.addEventListener("dragover", (e) => (e.preventDefault(), drop.setAttribute("data-over", "")));
    drop.addEventListener("dragleave", () => drop.removeAttribute("data-over"));
    drop.addEventListener("drop", (e) => (e.preventDefault(), drop.removeAttribute("data-over"), take(e.dataTransfer.files[0])));
    new ResizeObserver(() => img && draw()).observe(stage);
    const update = () => {
      const src = K.value(api, cfg.src);
      if (src === srcWas) return;
      srcWas = src;
      if (safe(src)) load(src);
      else if (cfg.src !== undefined) (img = null), show();
    };
    show();
    update();
    return { update };
  };

  // ---------------------------------------------------------------- plan
  B["ui-plan"] = (el, api) => {
    const cfg = api.config, W = cfg.width ?? 1000, H = cfg.height ?? 600, grid = cfg.grid;
    const TONES = { neutral: "var(--color-muted)", info: "var(--color-info, #2563eb)", success: "var(--color-success, #16a34a)", warning: "var(--color-warning, #d97706)", danger: "var(--color-destructive, #dc2626)", primary: "var(--color-primary)" };
    const root = svg("svg", { "data-pl": "", width: W, height: H, viewBox: `0 0 ${W} ${H}`, role: "group" });
    const back = svg("g"), layer = svg("g");
    root.append(back, layer);
    const frame = make("div", { "data-pl-frame": "" });
    // What's zoomed is a box around the drawing: an SVG element has no offset size.
    const sheet = make("div", { "data-pl-sheet": "", style: `width:${W}px;height:${H}px` });
    sheet.append(root);
    frame.append(sheet);
    const zoom = make("div", { "data-pl-zoom": "" }, button({ "aria-label": L("Zoom in"), click: () => view.zoom(view.scale * 1.4) }, icon("plus", 15)), button({ "aria-label": L("Zoom out"), click: () => view.zoom(view.scale / 1.4) }, icon("minus", 15)), button({ "aria-label": L("Show all"), title: L("Show all"), click: () => view.reset() }, icon("fit", 15)));
    frame.append(zoom);
    const legend = make("ul", { "data-pl-legend": "", hidden: true });
    el.append(frame, legend);
    const fit = () => Math.min((frame.clientWidth || W) / W, (frame.clientHeight || H) / H);
    const view = window.__irenderPanZoom(frame, sheet, { wheel: true, min: 0.1, max: 12, start: fit, free: false, ignore: "[data-pl-zoom], [data-pl-item][data-movable]" });
    const items = () => (K.value(api, cfg.items) ?? []).filter(isObj);
    let local = null, was = null;
    const picked = () => (cfg.bind ? api.state[cfg.bind] ?? null : local);
    const toneOf = (x) => x.tone ?? (isObj(cfg.tones) ? cfg.tones[x.status] : null) ?? null;
    const at = (r, f) => String(f).split(".").reduce((v, k) => (v !== null && typeof v === "object" ? v[k] : undefined), r);
    const passes = (x, filter) => !isObj(filter) || Object.entries(filter).every(([f, c]) => {
      const v = at(x, f);
      if (blank(c) || (Array.isArray(c) && !c.length)) return true;
      if (Array.isArray(c)) return c.includes(v);
      if (isObj(c)) return (blank(c.from) || Number(v) >= Number(c.from)) && (blank(c.to) || Number(v) <= Number(c.to)) && (blank(c.contains) || String(v ?? "").toLowerCase().includes(String(c.contains).toLowerCase()));
      return c === v;
    });
    const num = (v, d) => (blank(v) || isNaN(v) ? d : Number(v));
    // An item's box, in the plan's units.
    const boxOf = (x) => {
      if (x.shape === "polygon" && Array.isArray(x.points)) {
        const ps = x.points.map((p) => (Array.isArray(p) ? p : [p.x, p.y]).map(Number)), xs = ps.map((p) => p[0]), ys = ps.map((p) => p[1]);
        const ox = num(x.x, 0), oy = num(x.y, 0);
        return { x: ox + Math.min(...xs), y: oy + Math.min(...ys), w: Math.max(...xs) - Math.min(...xs), h: Math.max(...ys) - Math.min(...ys), ps: ps.map((p) => [p[0] + ox, p[1] + oy]) };
      }
      const w = num(x.width, x.shape === "pin" ? 0 : 80), h = num(x.height, x.shape === "circle" ? w : x.shape === "pin" ? 0 : 60);
      return { x: num(x.x, 0), y: num(x.y, 0), w, h };
    };
    function draw(force) {
      const all = items(), filter = K.value(api, cfg.filter), image = K.value(api, cfg.image), sig = JSON.stringify([all, filter, picked(), image]);
      if ((!force && sig === was) || drag) return;
      const first = was === null;
      was = sig;
      back.replaceChildren(...[
        safe(image) ? svg("image", { href: image, width: W, height: H, preserveAspectRatio: "xMidYMid meet" }) : svg("rect", { width: W, height: H, rx: 4, style: "fill:var(--color-surface);stroke:var(--color-border)" }),
        grid ? svg("path", { "data-pl-grid": "", d: Array.from({ length: Math.floor(W / grid) }, (_, i) => `M${(i + 1) * grid} 0V${H}`).join("") + Array.from({ length: Math.floor(H / grid) }, (_, i) => `M0 ${(i + 1) * grid}H${W}`).join(""), opacity: 0.5 }) : null,
      ].filter(Boolean));
      const focus = el.contains(document.activeElement) ? document.activeElement.getAttribute("data-pl-item") : null;
      layer.replaceChildren(...all.map((x) => {
        const b = boxOf(x), tone = toneOf(x), id = String(x.id), cx = b.x + b.w / 2, cy = b.y + b.h / 2, text = x.label ?? "", color = css(x.color);
        const g = svg("g", { "data-pl-item": id, "data-tone": tone, role: "button", tabindex: 0, "aria-pressed": String(String(picked() ?? "") === id), "aria-label": [x.label ?? id, x.status, x.sublabel].filter(Boolean).join(", "), "data-faded": !passes(x, filter), "data-movable": !!cfg.bindItems && !x.locked, transform: x.rotation ? `rotate(${num(x.rotation, 0)} ${cx} ${cy})` : null, style: color ? `--tone:${color}` : null });
        if (x.shape === "circle") g.append(svg("circle", { cx, cy, r: Math.min(b.w, b.h) / 2 }));
        else if (x.shape === "polygon" && b.ps) g.append(svg("polygon", { points: b.ps.map((p) => p.join(",")).join(" ") }));
        else if (x.shape === "pin") g.append(svg("path", { d: `M${b.x} ${b.y}c-4-9-12-14-12-22a12 12 0 1 1 24 0c0 8-8 13-12 22z` }));
        else g.append(svg("rect", { x: b.x, y: b.y, width: b.w, height: b.h, rx: num(x.radius, 4) }));
        if (x.shape === "pin") g.append(Object.assign(svg("text", { x: b.x, y: b.y - 22, "font-size": 11 }), { textContent: String(text).slice(0, 3) }));
        else if (text) {
          // As large as fits, within reason.
          const size = K.clamp(Math.min((b.w / Math.max(3, String(text).length)) * 1.6, b.h * (x.sublabel ? 0.3 : 0.42)), 5, 26);
          g.append(Object.assign(svg("text", { x: cx, y: x.sublabel ? cy - size * 0.55 : cy, "font-size": size }), { textContent: text }));
          if (x.sublabel) g.append(Object.assign(svg("text", { "data-sub": "", x: cx, y: cy + size * 0.65, "font-size": Math.max(5, Math.min(size * 0.75, (b.w / String(x.sublabel).length) * 1.7)) }), { textContent: x.sublabel }));
        }
        return g;
      }));
      if (focus) layer.querySelector(`[data-pl-item="${CSS.escape(focus)}"]`)?.focus({ preventScroll: true });
      if (cfg.legend) {
        const by = new Map();
        for (const x of all) if (!blank(x.status)) by.set(x.status, { n: (by.get(x.status)?.n ?? 0) + 1, tone: toneOf(x) });
        legend.hidden = !by.size;
        legend.replaceChildren(...[...by].map(([s, v]) => make("li", null, make("i", { style: `--tone:${TONES[v.tone] ?? "var(--color-muted)"}` }), `${s} (${v.n})`)));
      }
      if (first) view.reset();
    }
    function pick(id, go) {
      const x = items().find((i) => String(i.id) === id);
      if (!x) return;
      local = x.id;
      if (cfg.bind) api.set(cfg.bind, x.id);
      if (cfg.bindItem) api.set(cfg.bindItem, x);
      api.emit("pick", { item: x });
      if (go) {
        const b = boxOf(x);
        view.to(Math.max(view.scale, Math.min((frame.clientWidth / Math.max(b.w, 60)) * 0.3, fit() * 3)), b.x + b.w / 2, b.y + b.h / 2);
        layer.querySelector(`[data-pl-item="${CSS.escape(id)}"]`)?.focus({ preventScroll: true });
      }
      draw(true);
    }
    const snap = (v) => (grid ? Math.round(v / grid) * grid : Math.round(v));
    function moveTo(id, x, y) {
      const now = items().find((i) => String(i.id) === id);
      if (!now) return;
      const next = { ...now, x, y };
      api.set(cfg.bindItems, (api.state[cfg.bindItems] ?? []).map((i) => (String(i.id) === id ? next : i)));
      if (cfg.bindItem && String(api.state[cfg.bindItem]?.id) === id) api.set(cfg.bindItem, next);
      api.emit("move", { item: next, previous: now });
    }
    let drag = null;
    layer.addEventListener("pointerdown", (e) => {
      const g = e.target.closest("[data-pl-item][data-movable]");
      if (!g || e.button !== 0) return;
      e.stopPropagation();
      const x = items().find((i) => String(i.id) === g.getAttribute("data-pl-item"));
      drag = { id: e.pointerId, g, x, px: e.clientX, py: e.clientY, on: false, dx: 0, dy: 0 };
      K.capture(g, e);
    });
    layer.addEventListener("pointermove", (e) => {
      if (!drag || e.pointerId !== drag.id) return;
      const dx = (e.clientX - drag.px) / view.scale, dy = (e.clientY - drag.py) / view.scale;
      if (!drag.on && Math.hypot(e.clientX - drag.px, e.clientY - drag.py) < 4) return;
      drag.on = true;
      drag.dx = snap(num(drag.x.x, 0) + dx) - num(drag.x.x, 0);
      drag.dy = snap(num(drag.x.y, 0) + dy) - num(drag.x.y, 0);
      drag.g.setAttribute("data-dragging", "");
      drag.g.style.translate = `${drag.dx}px ${drag.dy}px`;
    });
    const drop = (e, keep) => {
      if (!drag || e.pointerId !== drag.id) return;
      const d = drag;
      drag = null;
      if (d.on && keep) {
        moveTo(String(d.x.id), num(d.x.x, 0) + d.dx, num(d.x.y, 0) + d.dy);
        el.__dragged = true;
        setTimeout(() => (el.__dragged = false));
      }
      draw(true);
    };
    layer.addEventListener("pointerup", (e) => drop(e, true));
    layer.addEventListener("pointercancel", (e) => drop(e, false));
    layer.addEventListener("click", (e) => {
      const g = e.target.closest("[data-pl-item]");
      if (g && !el.__dragged) pick(g.getAttribute("data-pl-item"));
    });
    frame.addEventListener("keydown", (e) => {
      const g = e.target.closest?.("[data-pl-item]");
      const d = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] }[e.key];
      if (e.key === "+" || e.key === "=") view.zoom(view.scale * 1.3);
      else if (e.key === "-") view.zoom(view.scale / 1.3);
      else if (e.key === "0") view.reset();
      else if (g && (e.key === "Enter" || e.key === " ")) pick(g.getAttribute("data-pl-item"));
      else if (g && d && e.altKey && g.hasAttribute("data-movable")) {
        const x = items().find((i) => String(i.id) === g.getAttribute("data-pl-item")), by = grid ?? 10;
        moveTo(String(x.id), num(x.x, 0) + d[0] * by, num(x.y, 0) + d[1] * by);
      } else if (g && d) {
        // The nearest item that way.
        const all = items(), me = boxOf(all.find((i) => String(i.id) === g.getAttribute("data-pl-item"))), cx = me.x + me.w / 2, cy = me.y + me.h / 2;
        const best = all.filter((i) => String(i.id) !== g.getAttribute("data-pl-item")).map((i) => { const b = boxOf(i), dx = b.x + b.w / 2 - cx, dy = b.y + b.h / 2 - cy, along = dx * d[0] + dy * d[1]; return { i, along, off: Math.abs(dx * d[1]) + Math.abs(dy * d[0]) }; }).filter((o) => o.along > 1).sort((a, b) => a.along + a.off * 2 - (b.along + b.off * 2))[0];
        if (best) layer.querySelector(`[data-pl-item="${CSS.escape(String(best.i.id))}"]`)?.focus();
      } else return;
      e.preventDefault();
    });
    const search = el.querySelector("[data-pl-search] input");
    if (search) {
      const ul = make("ul", { hidden: true });
      search.parentElement.append(ul);
      const find = () => {
        const q = search.value.trim().toLowerCase();
        const hits = q ? items().filter((x) => [x.label, x.sublabel, x.id, x.status].some((v) => String(v ?? "").toLowerCase().includes(q))).slice(0, 12) : [];
        ul.hidden = !q;
        ul.replaceChildren(...(hits.length ? hits.map((x) => make("li", null, button({ click: () => ((ul.hidden = true), (search.value = x.label ?? x.id), pick(String(x.id), true)) }, x.label ?? x.id, x.status ? make("small", null, x.status) : null))) : [make("li", null, make("small", null, L("No results")))]));
      };
      search.addEventListener("input", find);
      search.addEventListener("keydown", (e) => {
        if (e.key === "Enter") e.preventDefault(), ul.querySelector("button")?.click();
        else if (e.key === "ArrowDown") e.preventDefault(), ul.querySelector("button")?.focus();
        else if (e.key === "Escape") ul.hidden = true;
      });
      search.parentElement.addEventListener("focusout", (e) => !search.parentElement.contains(e.relatedTarget) && (ul.hidden = true));
    }
    let widthWas = 0;
    new ResizeObserver(() => {
      if (frame.clientWidth === widthWas) return;
      widthWas = frame.clientWidth;
      view.reset();
    }).observe(frame);
    draw();
    return { update: () => draw() };
  };

  // ---------------------------------------------------------------- pdf
  const libs = new Map();
  const library = (src) => {
    if (!libs.has(src)) {
      libs.set(src, import(src).then((m) => {
        // A worker from another site goes through a blob, which its module may import.
        const worker = src.replace(/pdf(\.min)?\.mjs$/, "pdf.worker$1.mjs");
        m.GlobalWorkerOptions.workerSrc = new URL(worker, location.href).origin === location.origin ? worker : URL.createObjectURL(new Blob([`import "${worker}";`], { type: "text/javascript" }));
        return m;
      }));
    }
    return libs.get(src);
  };
  B["ui-pdf"] = (el, api) => {
    const cfg = api.config, tools = Array.isArray(cfg.tools) ? cfg.tools : ["highlight", "box", "note"];
    let doc = null, srcWas = null, scale = 0, page = 1, tool = null, pages = [], picked = null, pop = null, notesWas = null, seq = 0;
    const title = make("span", { "data-pdf-title": "" });
    const num = make("input", { type: "text", inputmode: "numeric", "aria-label": L("Page"), value: "1" }), count = make("span", null, "/ –");
    const link = cfg.download ? make("a", { "aria-label": L("Download"), title: L("Download"), download: "" }, icon("download", 15)) : null;
    // Labels: "Highlight", "Box", "Note", "Signature"
    const NAMES = { highlight: "Highlight", box: "Box", note: "Note", signature: "Signature" };
    const toolBs = cfg.bindNotes ? tools.filter((t) => NAMES[t]).map((t) => button({ "data-tool": t, "aria-pressed": "false", click: () => setTool(tool === t ? null : t) }, L(NAMES[t]))) : [];
    const bar = make("div", { "data-m-bar": "" }, title, button({ "aria-label": L("Previous page"), click: () => goTo(page - 1) }, icon("up", 14)), num, count, button({ "aria-label": L("Next page"), click: () => goTo(page + 1) }, icon("down", 14)), make("span", { "data-push": "" }), toolBs, button({ "aria-label": L("Zoom out"), click: () => setScale(scale / 1.2) }, icon("zoomout", 15)), button({ "aria-label": L("Zoom in"), click: () => setScale(scale * 1.2) }, icon("zoomin", 15)), button({ click: () => setScale(0) }, L("Fit")), link);
    const box = make("div", { "data-pdf-pages": "", tabindex: 0, role: "region", "aria-label": L("Pages") });
    const none = make("p", { "data-pdf-none": "", role: "status" });
    el.replaceChildren(bar, box);
    const notes = () => ((cfg.bindNotes ? api.state[cfg.bindNotes] : K.value(api, cfg.notes)) ?? []).filter(isObj);
    const writeNotes = (list, event, note) => {
      api.set(cfg.bindNotes, list);
      api.emit(event, { note });
    };
    function setTool(t) {
      tool = t;
      for (const b of toolBs) b.setAttribute("aria-pressed", String(b.getAttribute("data-tool") === t));
      for (const p of pages) p.el.toggleAttribute("data-tool", !!t);
    }
    async function load(src) {
      const n = ++seq;
      none.textContent = L("Loading…");
      box.replaceChildren(none);
      pages = [];
      doc?.destroy?.();
      doc = null;
      try {
        const lib = await library(cfg.library);
        const d = await lib.getDocument({ url: new URL(src, document.baseURI).href, isEvalSupported: false }).promise;
        if (n !== seq) return d.destroy();
        doc = d;
        await layout();
      } catch (e) {
        if (n !== seq) return;
        // Without pdf.js, the browser's own viewer.
        if (libs.get(cfg.library) && !doc) {
          none.textContent = L("Couldn't show this PDF ({error}).", { error: e?.message ?? String(e) });
          box.replaceChildren(none, make("p", { "data-pdf-none": "" }, make("a", { href: src, target: "_blank", rel: "noreferrer" }, L("Open it in a new tab"))));
        }
      }
    }
    async function layout() {
      const keep = page, first = await doc.getPage(1), base = first.getViewport({ scale: 1 });
      if (!scale) scale = K.clamp((box.clientWidth - 34) / base.width, 0.3, 3);
      count.textContent = `/ ${doc.numPages}`;
      if (cfg.bindPages && api.state[cfg.bindPages] !== doc.numPages) api.set(cfg.bindPages, doc.numPages);
      pages = Array.from({ length: doc.numPages }, (_, i) => ({ n: i + 1, el: make("div", { "data-pdf-page": i + 1, role: "img", "aria-label": L("Page {n}", { n: i + 1 }), style: `width:${base.width * scale}px;height:${base.height * scale}px` }), drawn: false }));
      box.replaceChildren(...pages.map((p) => p.el));
      for (const p of pages) p.el.toggleAttribute("data-tool", !!tool);
      const seen = new IntersectionObserver((es) => es.forEach((e) => e.isIntersecting && render(pages[Number(e.target.getAttribute("data-pdf-page")) - 1])), { root: box, rootMargin: "600px" });
      pages.forEach((p) => seen.observe(p.el));
      goTo(keep, false);
      notesWas = null;
      drawNotes();
    }
    async function render(p) {
      if (p.drawn || !doc) return;
      p.drawn = true;
      const pg = await doc.getPage(p.n), view = pg.getViewport({ scale }), r = Math.min(2, devicePixelRatio || 1);
      p.el.style.width = view.width + "px";
      p.el.style.height = view.height + "px";
      const c = make("canvas", { width: Math.floor(view.width * r), height: Math.floor(view.height * r) });
      p.el.prepend(c);
      try {
        await pg.render({ canvasContext: c.getContext("2d"), viewport: pg.getViewport({ scale: scale * r }) }).promise;
        // Its text over it, to select and find.
        const t = await pg.getTextContent(), layer = make("div", { "data-pdf-text": "" }), lib = await library(cfg.library);
        for (const it of t.items) {
          if (!it.str) continue;
          const m = lib.Util.transform(view.transform, it.transform), size = Math.hypot(m[2], m[3]);
          const s = make("span", { style: `left:${m[4]}px;top:${m[5] - size * 0.82}px;font-size:${size}px;font-family:sans-serif` }, it.str);
          s.__w = it.width * scale;
          layer.append(s);
        }
        c.after(layer);
        for (const s of layer.children) {
          const w = s.getBoundingClientRect().width;
          if (w && s.__w) s.style.transform = `scaleX(${s.__w / w})`;
        }
        p.el.removeAttribute("role");
      } catch { /* drawn again at another size */ }
    }
    function setScale(s) {
      scale = s ? K.clamp(s, 0.3, 4) : 0;
      if (doc) layout();
    }
    function goTo(n, smooth = true) {
      if (!doc) return;
      n = K.clamp(Math.trunc(n) || 1, 1, doc.numPages);
      page = n;
      const p = pages[n - 1]?.el;
      if (p) box.scrollTo({ top: p.offsetTop - box.offsetTop - 12, behavior: smooth && !matchMedia("(prefers-reduced-motion: reduce)").matches ? "smooth" : "auto" });
      mark();
    }
    function mark() {
      if (document.activeElement !== num) num.value = String(page);
      if (cfg.bindPage && Number(api.state[cfg.bindPage] ?? 1) !== page) api.set(cfg.bindPage, page);
    }
    box.addEventListener("scroll", () => {
      const mid = box.scrollTop + box.clientHeight / 3;
      const p = pages.findLast?.((x) => x.el.offsetTop - box.offsetTop <= mid);
      if (p && p.n !== page) (page = p.n), mark();
    }, { passive: true });
    num.addEventListener("change", () => goTo(Number(num.value)));
    num.addEventListener("keydown", (e) => e.key === "Enter" && goTo(Number(num.value)));

    // ---- notes
    function drawNotes() {
      const list = notes(), sig = JSON.stringify([list, picked, pages.length]);
      if (sig === notesWas) return;
      notesWas = sig;
      for (const p of pages) p.el.querySelectorAll("[data-note]").forEach((n) => n.remove());
      for (const x of list) {
        const p = pages[Number(x.page ?? 1) - 1];
        if (!p) continue;
        const type = NAMES[x.type] ? x.type : "highlight", color = css(x.color);
        const b = button({ "data-note": type, "data-id": x.id, "aria-label": [L(NAMES[type]), x.text, x.who].filter(Boolean).join(": "), title: x.text ?? "", "aria-pressed": String(picked === x.id), style: `left:${x.x * 100}%;top:${x.y * 100}%;width:${(x.width ?? 0.02) * 100}%;height:${(x.height ?? 0.02) * 100}%;${color ? `--n:${color};` : ""}` }, type === "note" ? "!" : "");
        if (type === "signature" && safe(x.image)) b.style.backgroundImage = `url("${x.image.replace(/["\\\n]/g, "")}")`;
        b.addEventListener("click", () => open(x.id));
        p.el.append(b);
      }
    }
    function open(id, edit) {
      const x = notes().find((n) => n.id === id);
      if (!x) return;
      picked = id;
      if (cfg.bindNote) api.set(cfg.bindNote, x);
      api.emit("note", { note: x });
      drawNotes();
      close(false);
      const p = pages[Number(x.page ?? 1) - 1].el, can = !!cfg.bindNotes;
      const area = can ? make("textarea", { "aria-label": L("Note"), placeholder: L("Write a note…") }) : null;
      if (area) area.value = x.text ?? "";
      pop = make("div", { "data-note-pop": "", role: "dialog", "aria-label": L("Note") }, x.who ? make("small", null, x.who) : null, area ?? make("span", null, x.text ?? ""),
        make("div", null, can ? button({ click: () => { close(false); picked = null; writeNotes(notes().filter((n) => n.id !== id), "noteremove", x); } }, L("Remove")) : make("span"), button({ click: () => close(true) }, can ? L("Done") : L("Close"))));
      pop.__id = id;
      pop.style.top = `min(calc(${(x.y + (x.type === "note" ? 0 : x.height ?? 0.02)) * 100}% + 6px), calc(100% - 150px))`;
      pop.style.left = `min(${x.x * 100}%, calc(100% - 238px))`;
      p.append(pop);
      pop.addEventListener("keydown", (e) => e.key === "Escape" && (e.stopPropagation(), close(true)));
      if (edit) area?.focus();
    }
    function close(save) {
      if (!pop) return;
      const area = pop.querySelector("textarea"), id = pop.__id, p = pop;
      pop = null;
      if (area && save !== null) {
        const x = notes().find((n) => n.id === id);
        if (x && (x.text ?? "") !== area.value) writeNotes(notes().map((n) => (n.id === id ? { ...n, text: area.value } : n)), "noteedit", { ...x, text: area.value });
      }
      p.remove();
      if (save) {
        picked = null;
        drawNotes();
        box.querySelector(`[data-id="${CSS.escape(String(id))}"]`)?.focus();
      }
    }
    let drag = null;
    box.addEventListener("pointerdown", (e) => {
      const p = e.target.closest("[data-pdf-page]");
      if (!tool || !p || e.button !== 0 || e.target.closest("[data-note-pop], [data-note]")) return;
      e.preventDefault();
      const b = p.getBoundingClientRect();
      drag = { p, b, id: e.pointerId, x: (e.clientX - b.left) / b.width, y: (e.clientY - b.top) / b.height, ghost: make("div", { "data-note": tool === "highlight" ? "highlight" : "box", style: "pointer-events:none" }) };
      if (tool === "highlight" || tool === "box") p.append(drag.ghost);
      K.capture(p, e);
    });
    const span = (e) => {
      const x = K.clamp((e.clientX - drag.b.left) / drag.b.width, 0, 1), y = K.clamp((e.clientY - drag.b.top) / drag.b.height, 0, 1);
      return { x: Math.min(x, drag.x), y: Math.min(y, drag.y), width: Math.abs(x - drag.x), height: Math.abs(y - drag.y) };
    };
    box.addEventListener("pointermove", (e) => {
      if (!drag || e.pointerId !== drag.id) return;
      const s = span(e);
      Object.assign(drag.ghost.style, { left: s.x * 100 + "%", top: s.y * 100 + "%", width: s.width * 100 + "%", height: s.height * 100 + "%" });
    });
    box.addEventListener("pointerup", (e) => {
      if (!drag || e.pointerId !== drag.id) return;
      let s = span(e);
      const d = drag, t = tool, n = Number(d.p.getAttribute("data-pdf-page"));
      drag.ghost.remove();
      drag = null;
      if (t === "note") s = { x: d.x, y: d.y, width: 0.02, height: 0.02 };
      else if (t === "signature") s = { x: K.clamp(d.x - 0.12, 0, 0.76), y: K.clamp(d.y - 0.03, 0, 0.94), width: 0.24, height: 0.06 };
      else if (s.width < 0.01 || s.height < 0.005) return;
      const image = t === "signature" ? K.value(api, cfg.signature) : undefined;
      if (t === "signature" && !safe(image)) return K.say(el, L("There's no signature to put here yet"));
      const who = K.value(api, cfg.user);
      const note = { id: K.id(), page: n, type: t, ...Object.fromEntries(Object.entries(s).map(([k, v]) => [k, Math.round(v * 1e4) / 1e4])), text: "", ...(who ? { who: isObj(who) ? who.name : who } : {}), at: new Date().toISOString(), ...(image ? { image } : {}) };
      writeNotes([...notes(), note], "noteadd", note);
      setTool(null);
      if (t !== "signature") open(note.id, true);
    });
    box.addEventListener("pointercancel", () => (drag?.ghost.remove(), (drag = null)));

    function draw() {
      const src = K.value(api, cfg.src);
      title.textContent = K.value(api, cfg.title) ?? "";
      title.hidden = !title.textContent;
      if (link && safe(src)) link.href = src;
      if (src !== srcWas) {
        srcWas = src;
        page = 1;
        if (!safe(src)) (none.textContent = L("No PDF to show.")), box.replaceChildren(none);
        else if (near) load(src);
      } else if (cfg.bindPage && doc && Number(api.state[cfg.bindPage]) && Number(api.state[cfg.bindPage]) !== page) goTo(Number(api.state[cfg.bindPage]));
      if (pages.length) drawNotes();
    }
    // pdf.js and the file are fetched once it's near the screen.
    let near = false;
    const seen = new IntersectionObserver((es) => {
      if (!es.some((e) => e.isIntersecting)) return;
      near = true;
      seen.disconnect();
      if (safe(srcWas)) load(srcWas);
    }, { rootMargin: "400px" });
    seen.observe(el);
    draw();
    return { update: draw };
  };
})();
