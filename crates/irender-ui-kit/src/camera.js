// irender-ui-kit: `ui-camera`, `ui-permission-manager` and `ui-photo-annotate`.
(() => {
  const B = (window.__irenderBehaviors ||= {});
  const K = window.__irenderKit, D = window.__irenderDevice, L = (t, v) => window.__irender.label(t, v);
  const { make, icon, isObj, blank } = K, btn = D.btn;
  const list = (v) => (Array.isArray(v) ? v : []);
  const shape = (r) => {
    const m = /^\s*([\d.]+)\s*[/:]\s*([\d.]+)\s*$/.exec(String(r ?? ""));
    const n = m ? Number(m[1]) / Number(m[2]) : Number(r);
    return n > 0 ? n : null;
  };
  const picture = (src) => new Promise((ok, no) => {
    const i = new Image();
    if (/^https?:/.test(src)) i.crossOrigin = "anonymous";
    i.onload = () => ok(i);
    i.onerror = () => no(new Error(L("The picture couldn't be read")));
    i.src = src;
  });

  // ---------------------------------------------------------------- camera
  B["ui-camera"] = (el, api) => {
    const cfg = api.config, type = `image/${cfg.type ?? "jpeg"}`, ratio = shape(cfg.ratio), can = !!navigator.mediaDevices?.getUserMedia;
    let stream = null, facing = cfg.facing === "front" ? "user" : "environment", shot = null, starting = false, counting = 0;
    const video = make("video", { playsinline: true, "aria-label": L("Camera") });
    video.muted = true;
    const shown = make("img", { alt: L("The photo taken"), hidden: true });
    const grid = make("div", { "data-cam-grid": "", hidden: !cfg.grid }), flash = make("div", { "data-cam-flash": "" }), count = make("div", { "data-cam-count": "", "aria-hidden": "true", hidden: true });
    const tool = (name, label, go, pressed) => K.button({ "aria-label": label, title: label, "aria-pressed": pressed === undefined ? null : String(pressed), click: go }, icon(name, 18));
    const torch = tool("bolt", L("Light"), () => light(torch.getAttribute("aria-pressed") !== "true"), false);
    const lines = tool("grid", L("Composing lines"), () => ((grid.hidden = !grid.hidden), lines.setAttribute("aria-pressed", String(!grid.hidden))), !!cfg.grid);
    const zoom = make("input", { type: "range", "data-cam-zoom": "", "aria-label": L("Zoom"), step: 0.1, input: () => track()?.applyConstraints({ advanced: [{ zoom: Number(zoom.value) }] }).catch(() => {}) });
    const close = tool("x", L("Close the camera"), () => (stop(), draw(), start.focus()));
    const flip = tool("flip", L("Switch camera"), () => ((facing = facing === "user" ? "environment" : "user"), open()));
    const shutter = K.button({ "data-shutter": "", "aria-label": L("Take the photo"), click: take });
    const again = K.button({ "data-wide": "", click: () => ((shot = null), draw()) }, L("Retake")), use = K.button({ "data-wide": "use", click: () => (keep(shot), (shot = null), draw()) }, L("Use photo"));
    const live = make("div", { "data-cam-foot": "" }, flip, shutter, make("span", { style: "width:40px" })), check = make("div", { "data-cam-foot": "" }, again, use);
    const top = make("div", { "data-cam-top": "" }, torch, lines, zoom, make("span", { style: "margin-inline-start:auto" }), close);
    const view = make("div", { "data-cam-view": "", hidden: true }, video, shown, cfg.guide && cfg.guide !== "none" ? make("div", { "data-cam-guide": cfg.guide }) : null, grid, count, flash, top, live, check);
    const error = make("p", { "data-w-error": "", role: "alert" }), name = make("span");
    const start = btn("primary", { "data-start": "", click: open }, icon("camera", 16), name);
    // `capture` opens the phone's own camera where this one can't be used.
    const input = make("input", { type: "file", accept: "image/*", multiple: cfg.multiple, capture: can ? null : facing, change: () => chosen([...input.files]) });
    const files = make("label", { "data-w-btn": can ? "" : "primary" }, icon(can ? "file" : "camera", 15), can ? L("Choose a photo") : L("Take a photo"), input);
    const row = make("div", { "data-w-row": "" }, can ? start : null, cfg.files || !can ? files : null);
    const shots = make("ul", { "data-cam-shots": "" });
    if (ratio) view.style.setProperty("--cam-ratio", String(ratio));
    el.append(view, shots, row, error);
    const track = () => stream?.getVideoTracks()[0];
    const off = () => !!K.value(api, cfg.disabled);
    const all = () => (cfg.multiple ? list(api.state[cfg.bind]) : isObj(api.state[cfg.bind]) ? [api.state[cfg.bind]] : []);
    const full = () => cfg.multiple && cfg.max && all().length >= cfg.max;
    async function open() {
      if (starting || off()) return;
      starting = true;
      stop();
      error.textContent = "";
      try {
        const s = await navigator.mediaDevices.getUserMedia({ video: { facingMode: { ideal: facing }, width: { ideal: 1920 }, height: { ideal: 1440 } }, audio: false });
        if (!el.isConnected) return s.getTracks().forEach((t) => t.stop());
        stream = s;
        track().addEventListener("ended", () => stream === s && (stop(), draw()));
        video.srcObject = s;
        // What it's facing, where it says: a laptop's only camera faces the front.
        const is = track().getSettings?.().facingMode ?? (D.ios || D.android ? facing : "user");
        video.toggleAttribute("data-front", is === "user");
        const caps = track().getCapabilities?.() ?? {};
        torch.hidden = !caps.torch;
        torch.setAttribute("aria-pressed", "false");
        zoom.hidden = !(caps.zoom && caps.zoom.max > caps.zoom.min);
        if (!zoom.hidden) (zoom.min = caps.zoom.min), (zoom.max = caps.zoom.max), (zoom.value = track().getSettings().zoom ?? caps.zoom.min);
        navigator.mediaDevices.enumerateDevices().then((d) => (flip.style.visibility = d.filter((x) => x.kind === "videoinput").length > 1 ? "" : "hidden")).catch(() => {});
        await video.play().catch(() => {});
        if (!ratio && video.videoWidth) view.style.setProperty("--cam-ratio", String(video.videoWidth / video.videoHeight));
        api.emit("open", { facing });
      } catch (e) {
        error.textContent = D.explain(e, "camera") ?? "";
        api.emit("failed", { name: e.name, error: error.textContent });
      } finally {
        starting = false;
        draw();
      }
      if (stream) shutter.focus({ preventScroll: true });
    }
    function stop() {
      stream?.getTracks().forEach((t) => t.stop());
      stream = null;
      video.srcObject = null;
      clearInterval(counting);
      counting = 0;
      count.hidden = true;
      shot = null;
    }
    async function light(on) {
      try {
        await track().applyConstraints({ advanced: [{ torch: on }] });
        torch.setAttribute("aria-pressed", String(on));
      } catch { /* it has none after all */ }
    }
    function take() {
      if (counting || !stream) return;
      if (!(cfg.timer > 0)) return snap();
      let n = Math.ceil(cfg.timer);
      const tick = () => {
        count.textContent = n;
        count.hidden = n <= 0;
        if (n > 0) return n--, K.say(el, count.textContent);
        clearInterval(counting);
        counting = 0;
        snap();
      };
      counting = setInterval(tick, 1000);
      tick();
    }
    // Cut to its shape, made smaller, turned right way round, and stamped.
    function shrink(source, w, h) {
      let [sw, sh] = [w, h];
      if (ratio) w / h > ratio ? (sw = h * ratio) : (sh = w / ratio);
      const k = Math.min(1, (cfg.maxSize ?? 1600) / Math.max(sw, sh)), c = make("canvas", { width: Math.round(sw * k), height: Math.round(sh * k) }), g = c.getContext("2d");
      if (type === "image/jpeg") (g.fillStyle = "#fff"), g.fillRect(0, 0, c.width, c.height);
      g.drawImage(source, (w - sw) / 2, (h - sh) / 2, sw, sh, 0, 0, c.width, c.height);
      let mark = K.value(api, cfg.stamp);
      if (!blank(mark)) {
        mark = String(mark).split(/\btime\b/).join(new Date().toLocaleString());
        const size = Math.max(12, Math.round(c.width / 42)), pad = size * 0.7;
        g.font = `600 ${size}px system-ui, sans-serif`;
        g.fillStyle = "rgb(0 0 0 / 0.55)";
        g.fillRect(0, c.height - size - pad * 1.6, c.width, size + pad * 1.6);
        g.fillStyle = "#fff";
        g.textBaseline = "middle";
        g.fillText(mark, pad, c.height - (size + pad * 1.6) / 2, c.width - pad * 2);
      }
      const url = c.toDataURL(type, cfg.quality ?? 0.8), is = /^data:([^;]+)/.exec(url)[1], at = new Date();
      return { id: K.id(), url, name: `photo-${D.stamp(at)}.${is.split("/")[1].replace("jpeg", "jpg")}`, type: is, size: Math.round(((url.length - url.indexOf(",") - 1) * 3) / 4), width: c.width, height: c.height, at: at.toISOString() };
    }
    function snap() {
      if (!stream || !video.videoWidth) return;
      flash.style.transition = "none";
      flash.style.opacity = "0.9";
      requestAnimationFrame(() => ((flash.style.transition = ""), (flash.style.opacity = "0")));
      navigator.vibrate?.(30);
      const photo = { ...shrink(video, video.videoWidth, video.videoHeight), facing: facing === "user" ? "front" : "back" };
      K.say(el, L("Photo taken"));
      if (!cfg.review) return keep(photo);
      shot = photo;
      draw();
      use.focus({ preventScroll: true });
    }
    function keep(photo) {
      if (!photo) return;
      api.set(cfg.bind, cfg.multiple ? [...all(), photo] : photo);
      api.emit("photo", photo);
      if (!cfg.multiple || full()) stop();
      draw();
    }
    async function chosen(picked) {
      error.textContent = "";
      for (const f of picked) {
        if (full()) break;
        const src = URL.createObjectURL(f);
        try {
          const i = await createImageBitmap(f).catch(() => picture(src));
          keep(shrink(i, i.naturalWidth ?? i.width, i.naturalHeight ?? i.height));
        } catch (e) {
          error.textContent = `${f.name}: ${e.message}`;
        }
        URL.revokeObjectURL(src);
        if (!cfg.multiple) break;
      }
      input.value = "";
    }
    let drawn = null;
    function draw() {
      const now = all(), on = !!stream;
      view.hidden = !on;
      shown.hidden = check.hidden = !shot;
      live.hidden = top.hidden = !!shot;
      if (shot && shown.src !== shot.url) shown.src = shot.url;
      name.textContent = K.value(api, cfg.label) ?? (now.length && !cfg.multiple ? L("Take another") : L("Open the camera"));
      start.hidden = on;
      start.disabled = input.disabled = off() || !!full();
      row.hidden = off() || on;
      const sig = now.map((p) => p.id ?? p.url).join();
      if (sig === drawn) return;
      drawn = sig;
      shots.replaceChildren(...now.map((p, i) => make("li", null, make("img", { src: /^(data:image\/|https?:|blob:|\/|\.)/i.test(p.url ?? "") ? p.url : "", alt: p.name ?? L("Photo {n}", { n: i + 1 }) }), off() ? null : K.button({ "aria-label": L("Delete photo {n}", { n: i + 1 }), click: () => (api.set(cfg.bind, cfg.multiple ? all().filter((x) => x !== p && x.id !== p.id) : null), api.emit("remove", p)) }, icon("x", 12)))));
      shots.hidden = !now.length;
    }
    document.addEventListener("visibilitychange", () => document.visibilityState === "hidden" && stream && (stop(), draw()));
    D.gone(el, stop);
    draw();
    if (cfg.autoStart && can) D.permission("camera").then((p) => p === "granted" && open());
    return { update: draw };
  };

  // ---------------------------------------------------------------- permissions
  const ICONS = { camera: "camera", microphone: "mic", geolocation: "pin", notifications: "bell", motion: "compass", storage: "download", clipboard: "copy", nfc: "nfc" };
  const ASK = {
    camera: async () => (await navigator.mediaDevices.getUserMedia({ video: true })).getTracks().forEach((t) => t.stop()),
    microphone: async () => (await navigator.mediaDevices.getUserMedia({ audio: true })).getTracks().forEach((t) => t.stop()),
    geolocation: () => new Promise((ok, no) => navigator.geolocation.getCurrentPosition(ok, (e) => (e.code === 1 ? no(e) : ok()), { timeout: 15000 })),
    notifications: () => Notification.requestPermission(),
    motion: async () => {
      if ((await (DeviceOrientationEvent.requestPermission?.() ?? "granted")) !== "granted") throw Object.assign(new Error(), { name: "NotAllowedError" });
    },
    storage: () => navigator.storage.persist(),
    clipboard: () => navigator.clipboard.readText(),
    nfc: async () => {
      const stop = new AbortController();
      await new NDEFReader().scan({ signal: stop.signal });
      stop.abort();
    },
  };
  const HAS = {
    camera: () => navigator.mediaDevices?.getUserMedia, microphone: () => navigator.mediaDevices?.getUserMedia, geolocation: () => navigator.geolocation, notifications: () => "Notification" in window,
    motion: () => "DeviceOrientationEvent" in window, storage: () => navigator.storage?.persist, clipboard: () => navigator.clipboard?.readText, nfc: () => "NDEFReader" in window,
  };
  B["ui-permission-manager"] = (el, api) => {
    const cfg = api.config, all = cfg.permissions.map((p) => (isObj(p) ? p : { name: p }));
    const now = {}, asked = {}, body = D.body(el);
    let was = null;
    const words = { granted: () => [L("Allowed"), "good"], denied: () => [L("Blocked"), "bad"], prompt: () => [L("Not asked yet"), "info"], unsupported: () => [L("Not on this device"), null] };
    // The last look counts, when an earlier one takes longer.
    let turn = 0;
    async function look() {
      const n = ++turn, seen = {};
      for (const p of all) {
        if (!HAS[p.name]() || !window.isSecureContext) seen[p.name] = "unsupported";
        else if (p.name === "storage") seen.storage = (await navigator.storage.persisted()) ? "granted" : asked.storage ? "denied" : "prompt";
        // Where nothing says but asking: an iPhone's sensors, and some browsers' cameras.
        else seen[p.name] = (await D.permission(p.name)) ?? asked[p.name] ?? (p.name === "motion" && !DeviceOrientationEvent.requestPermission ? "granted" : "prompt");
      }
      if (n === turn) Object.assign(now, seen), tell();
    }
    function tell() {
      const v = Object.fromEntries(all.map((p) => [p.name, now[p.name] ?? "prompt"])), ready = all.every((p) => !p.required || v[p.name] === "granted");
      if (cfg.bind && JSON.stringify(api.state[cfg.bind] ?? null) !== JSON.stringify(v)) api.set(cfg.bind, v);
      if (cfg.bindReady && api.state[cfg.bindReady] !== ready) api.set(cfg.bindReady, ready);
      draw();
    }
    async function ask(p) {
      try {
        await ASK[p.name]();
        asked[p.name] = "granted";
      } catch (e) {
        asked[p.name] = "denied";
      }
      await look();
      K.say(el, `${p.label ?? D.name(p.name)}: ${words[now[p.name]]()[0]}`);
      api.emit("answer", { name: p.name, state: now[p.name] });
    }
    function draw() {
      const title = K.value(api, cfg.title), sig = JSON.stringify([title, now]);
      if (sig === was) return;
      was = sig;
      const focus = el.contains(document.activeElement) ? document.activeElement.closest("[data-perm]")?.getAttribute("data-perm") : null;
      body.replaceChildren(...[title ? make("p", { "data-w-title": "" }, title) : null, make("ul", { "data-w-list": "" }, all.map((p) => {
        const s = now[p.name] ?? "prompt", [word, tone] = words[s]();
        return make("li", { "data-perm": p.name, "data-state": s },
          make("span", { "data-perm-icon": "" }, icon(ICONS[p.name], 18)),
          make("div", { "data-w-main": "" }, make("span", { "data-perm-name": "" }, p.label ?? D.name(p.name), p.required ? make("small", null, L("needed")) : null, make("span", { "data-w-tag": "", "data-tone": tone }, word)), p.reason ? make("small", null, p.reason) : null),
          s === "prompt" ? btn("", { "aria-label": L("Allow {what}", { what: p.label ?? D.name(p.name) }), click: () => ask(p) }, L("Allow")) : s === "denied" ? btn("ghost", { "aria-label": L("How to allow {what}", { what: p.label ?? D.name(p.name) }), click: () => D.help(el, p.name) }, L("How to fix")) : null);
      }))].filter(Boolean));
      if (focus) el.querySelector(`[data-perm="${focus}"] button`)?.focus();
    }
    for (const p of all) D.watch(p.name, () => el.isConnected && look());
    document.addEventListener("visibilitychange", () => document.visibilityState === "visible" && el.isConnected && look());
    draw();
    look();
    return { update: draw };
  };

  // ---------------------------------------------------------------- annotate
  const COLORS = ["#ef4444", "#f59e0b", "#22c55e", "#3b82f6", "#ffffff", "#111111"];
  B["ui-photo-annotate"] = (el, api) => {
    const cfg = api.config, W = (cfg.width ?? 6) / 1000, colors = list(cfg.colors).length ? cfg.colors.filter((c) => /^[#\w(),.%\s/-]+$/.test(String(c))) : COLORS;
    let img = null, srcWas = null, tool = cfg.tools.find((t) => t !== "select") ?? "select", color = colors[0], marks = [], undone = [], past = [], sel = null, drag = null, last = null, box = null;
    const names = { select: L("Select"), arrow: L("Arrow"), box: L("Box"), circle: L("Circle"), pen: L("Draw"), text: L("Text"), measure: L("Measure"), blur: L("Blur") };
    const icons = { select: "cursor", arrow: "arrow", box: "square", circle: "circle", pen: "pencil", text: "text", measure: "ruler", blur: "blur" };
    const canvas = make("canvas", { tabindex: 0, role: "application", "data-readonly": cfg.readonly, "aria-label": cfg.readonly ? L("The photo with its marks") : L("The photo: pick a tool and drag on it; Delete removes the mark selected") });
    const stage = make("div", { "data-an-stage": "" }, canvas), error = make("p", { "data-w-error": "", role: "alert" });
    const tools = cfg.tools.map((t) => btn("", { "data-tool": t, "data-icon-only": "", "aria-label": names[t], title: names[t], click: () => pick(t) }, icon(icons[t], 17)));
    const swatches = make("div", { "data-w-row": "", role: "radiogroup", "aria-label": L("Colour"), style: "gap:6px" }, colors.map((c, i) => K.button({ "data-an-swatch": "", role: "radio", style: `background:${c}`, "aria-label": L("Colour {n}", { n: i + 1 }), click: () => { color = c; if (sel) change(() => (mark(sel).color = c)); draw(); } })));
    const undo = btn("ghost", { "data-icon-only": "", "data-undo": "", "aria-label": L("Undo"), title: L("Undo"), click: () => step(past, undone) }, icon("undo", 16)), redo = btn("ghost", { "data-icon-only": "", "aria-label": L("Redo"), title: L("Redo"), click: () => step(undone, past) }, icon("redo", 16));
    const gone = btn("ghost", { "data-icon-only": "", "data-remove": "", "aria-label": L("Delete the mark"), title: L("Delete the mark"), click: remove }, icon("trash", 16));
    if (!cfg.readonly) el.append(make("div", { "data-w-row": "" }, tools, make("span", { "data-an-sep": "" }), swatches, make("span", { "data-w-push": "" }), gone, undo, redo));
    el.append(stage, error);
    const mark = (id) => marks.find((m) => m.id === id);
    const unit = () => K.value(api, cfg.unit) ?? "cm";
    const copy = () => JSON.stringify(marks);
    function pick(t) {
      tool = t;
      if (t !== "select") sel = null;
      draw();
    }
    function change(f) {
      const before = copy();
      f();
      if (copy() === before) return draw();
      past.push(before);
      undone = [];
      save();
    }
    function step(from, to) {
      if (!from.length) return;
      to.push(copy());
      marks = JSON.parse(from.pop());
      if (!mark(sel)) sel = null;
      save();
    }
    function remove() {
      if (sel) change(() => ((marks = marks.filter((m) => m.id !== sel)), (sel = null)));
    }
    function save() {
      last = copy();
      if (cfg.bindMarks) api.set(cfg.bindMarks, JSON.parse(last));
      if (cfg.bind) api.set(cfg.bind, result());
      api.emit("change", { marks: JSON.parse(last) });
      draw();
    }
    function result() {
      if (!img) return null;
      const k = Math.min(1, (cfg.maxSize ?? 2000) / Math.max(img.naturalWidth, img.naturalHeight)), c = make("canvas", { width: Math.round(img.naturalWidth * k), height: Math.round(img.naturalHeight * k) });
      paint(c.getContext("2d"), c.width, c.height, false);
      try {
        return c.toDataURL(`image/${cfg.type ?? "jpeg"}`, cfg.quality ?? 0.85);
      } catch {
        error.textContent = L("This picture's site doesn't let it be marked here.");
        return null;
      }
    }
    // A length in the picture's own pixels: the first measurement with a length typed sets
    // how long one is.
    const px = (m) => Math.hypot((m.points[1][0] - m.points[0][0]) * img.naturalWidth, (m.points[1][1] - m.points[0][1]) * img.naturalHeight);
    const known = () => marks.find((m) => m.type === "measure" && Number(m.length) > 0);
    const length = (m) => {
      const k = known();
      if (m === k || (k && m.id === k.id)) return `${m.length} ${unit()}`;
      return k ? `${+((px(m) / px(k)) * Number(k.length)).toPrecision(3)} ${unit()}` : "?";
    };
    function paint(g, w, h, live) {
      g.imageSmoothingQuality = "high";
      g.drawImage(img, 0, 0, w, h);
      const lw = Math.max(1.5, W * w);
      g.lineCap = g.lineJoin = "round";
      // Blurs first, so the other marks stay sharp over them.
      for (const m of [...marks.filter((x) => x.type === "blur"), ...marks.filter((x) => x.type !== "blur")]) {
        const p = m.points.map(([x, y]) => [x * w, y * h]), [a, b = a] = [p[0], p.at(-1)];
        const x = Math.min(a[0], b[0]), y = Math.min(a[1], b[1]), bw = Math.abs(a[0] - b[0]), bh = Math.abs(a[1] - b[1]);
        g.strokeStyle = g.fillStyle = m.color ?? color;
        g.lineWidth = lw;
        g.beginPath();
        if (m.type === "box") g.strokeRect(x, y, bw, bh);
        else if (m.type === "circle") g.ellipse(x + bw / 2, y + bh / 2, bw / 2, bh / 2, 0, 0, 7), g.stroke();
        else if (m.type === "pen") {
          g.moveTo(...a);
          p.slice(1).forEach((q) => g.lineTo(...q));
          if (p.length === 1) g.lineTo(a[0] + 0.1, a[1]);
          g.stroke();
        } else if (m.type === "arrow") {
          const t = Math.atan2(b[1] - a[1], b[0] - a[0]), n = lw * 4.2;
          g.moveTo(...a);
          g.lineTo(b[0] - Math.cos(t) * n * 0.7, b[1] - Math.sin(t) * n * 0.7);
          g.stroke();
          g.beginPath();
          g.moveTo(...b);
          for (const d of [-0.45, 0.45]) g.lineTo(b[0] - Math.cos(t + d) * n, b[1] - Math.sin(t + d) * n);
          g.closePath();
          g.fill();
        } else if (m.type === "measure") {
          const t = Math.atan2(b[1] - a[1], b[0] - a[0]) + Math.PI / 2, n = lw * 2.4;
          g.lineWidth = lw * 0.6;
          g.moveTo(...a);
          g.lineTo(...b);
          for (const q of [a, b]) g.moveTo(q[0] - Math.cos(t) * n, q[1] - Math.sin(t) * n), g.lineTo(q[0] + Math.cos(t) * n, q[1] + Math.sin(t) * n);
          g.stroke();
          label(g, length(m), (a[0] + b[0]) / 2, (a[1] + b[1]) / 2, w, m.color, 0.028);
        } else if (m.type === "text") label(g, m.text ?? "", a[0], a[1], w, m.color, 0.036);
        else if (m.type === "blur" && bw > 2 && bh > 2) {
          // Drawn tiny and back up without smoothing: what was there isn't in the result.
          const n = Math.max(2, Math.min(10, Math.round(bw / (w * 0.02)))), mh = Math.max(1, Math.round((n * bh) / bw)), c = make("canvas", { width: n, height: mh });
          const k = img.naturalWidth / w;
          c.getContext("2d").drawImage(img, x * k, y * (img.naturalHeight / h), bw * k, bh * (img.naturalHeight / h), 0, 0, n, mh);
          g.imageSmoothingEnabled = false;
          g.drawImage(c, 0, 0, n, mh, x, y, bw, bh);
          g.imageSmoothingEnabled = true;
        }
        if (live && m.id === sel) {
          g.save();
          g.setLineDash([5, 4]);
          g.lineWidth = 1.5;
          g.strokeStyle = "#2563eb";
          const bb = bounds(m, w, h);
          g.strokeRect(bb.x - 5, bb.y - 5, bb.w + 10, bb.h + 10);
          g.setLineDash([]);
          g.fillStyle = "#fff";
          if (m.type !== "pen" && m.type !== "text") for (const q of [a, b]) g.beginPath(), g.arc(q[0], q[1], 6, 0, 7), g.fill(), g.stroke();
          g.restore();
        }
      }
    }
    function label(g, text, x, y, w, c, size) {
      const s = Math.max(11, w * size);
      g.font = `600 ${s}px system-ui, sans-serif`;
      g.textAlign = "center";
      g.textBaseline = "middle";
      const n = g.measureText(text).width;
      // On its own colour, in whichever of black and white reads on it.
      g.fillStyle = c ?? "#18181b";
      const hex = /^#([\da-f]{2})([\da-f]{2})([\da-f]{2})$/i.exec(g.fillStyle), light = hex && 0.3 * parseInt(hex[1], 16) + 0.59 * parseInt(hex[2], 16) + 0.11 * parseInt(hex[3], 16) > 160;
      g.beginPath();
      g.roundRect ? g.roundRect(x - n / 2 - s * 0.4, y - s * 0.75, n + s * 0.8, s * 1.5, s * 0.3) : g.rect(x - n / 2 - s * 0.4, y - s * 0.75, n + s * 0.8, s * 1.5);
      g.fill();
      g.fillStyle = light ? "#111" : "#fff";
      g.fillText(text, x, y);
    }
    function bounds(m, w, h) {
      const xs = m.points.map((p) => p[0] * w), ys = m.points.map((p) => p[1] * h);
      let x = Math.min(...xs), y = Math.min(...ys), bw = Math.max(...xs) - x, bh = Math.max(...ys) - y;
      if (m.type === "text") {
        const s = Math.max(11, w * 0.036), n = (m.text ?? "").length * s * 0.58;
        (x -= n / 2), (y -= s * 0.75), (bw = n), (bh = s * 1.5);
      }
      return { x, y, w: bw, h: bh };
    }
    const at = (e) => {
      const b = canvas.getBoundingClientRect();
      return [K.clamp((e.clientX - b.left) / b.width, 0, 1), K.clamp((e.clientY - b.top) / b.height, 0, 1)];
    };
    // The mark under a point, the last drawn first: near its line, or inside it.
    function hit([x, y]) {
      const w = canvas.clientWidth, h = canvas.clientHeight, X = x * w, Y = y * h;
      const near = (a, b) => {
        const dx = b[0] - a[0], dy = b[1] - a[1], t = K.clamp(((X - a[0]) * dx + (Y - a[1]) * dy) / (dx * dx + dy * dy || 1), 0, 1);
        return Math.hypot(X - a[0] - t * dx, Y - a[1] - t * dy) < 14;
      };
      return [...marks].reverse().find((m) => {
        const p = m.points.map((q) => [q[0] * w, q[1] * h]);
        if (["arrow", "measure", "pen"].includes(m.type)) return p.length === 1 ? near(p[0], p[0]) : p.slice(1).some((q, i) => near(p[i], q));
        const b = bounds(m, w, h);
        return X > b.x - 8 && X < b.x + b.w + 8 && Y > b.y - 8 && Y < b.y + b.h + 8;
      });
    }
    canvas.addEventListener("pointerdown", (e) => {
      if (cfg.readonly || !img || e.button !== 0) return;
      const p = at(e);
      finish();
      if (tool === "select" || tool === "text") {
        const cur = mark(sel), w = canvas.clientWidth, h = canvas.clientHeight;
        const end = cur && cur.type !== "pen" && cur.type !== "text" ? [0, cur.points.length - 1].find((i) => Math.hypot((cur.points[i][0] - p[0]) * w, (cur.points[i][1] - p[1]) * h) < 14) : undefined;
        const m = end !== undefined ? cur : hit(p);
        if (tool === "text" && !m) return e.preventDefault(), words({ id: K.id(), type: "text", points: [p], text: "", color }, true);
        sel = m?.id ?? null;
        if (m) drag = { before: copy(), p, end, pts: m.points.map((q) => [...q]) };
      } else {
        const m = { id: K.id(), type: tool, points: tool === "pen" ? [p] : [p, p], color };
        drag = { before: copy(), m };
        marks.push(m);
        sel = null;
      }
      K.capture(canvas, e);
      draw();
    });
    canvas.addEventListener("pointermove", (e) => {
      if (!drag) return;
      const p = at(e);
      if (drag.m) drag.m.type === "pen" ? drag.m.points.push(p.map((v) => +v.toFixed(4))) : (drag.m.points[1] = p);
      else {
        const m = mark(sel), dx = p[0] - drag.p[0], dy = p[1] - drag.p[1];
        if (drag.end !== undefined) m.points[drag.end] = p;
        else m.points = drag.pts.map(([x, y]) => [x + dx, y + dy]);
      }
      draw();
    });
    const up = () => {
      if (!drag) return;
      const d = drag, m = d.m;
      drag = null;
      if (m && m.type !== "pen" && Math.hypot((m.points[1][0] - m.points[0][0]) * canvas.clientWidth, (m.points[1][1] - m.points[0][1]) * canvas.clientHeight) < 6) return (marks = marks.filter((x) => x !== m)), draw();
      for (const x of marks) x.points = x.points.map((q) => q.map((v) => +v.toFixed(4)));
      if (copy() === d.before) return draw();
      past.push(d.before);
      undone = [];
      if (m) sel = m.id;
      if (m?.type === "measure" && !known()) return draw(), words(m, false);
      save();
    };
    canvas.addEventListener("pointerup", up);
    canvas.addEventListener("pointercancel", up);
    canvas.addEventListener("dblclick", (e) => {
      const m = cfg.readonly ? null : hit(at(e));
      if (m && (m.type === "text" || m.type === "measure")) words(m, false);
    });
    canvas.addEventListener("keydown", (e) => {
      if (cfg.readonly) return;
      const d = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] }[e.key];
      if ((e.key === "Delete" || e.key === "Backspace") && sel) remove();
      else if (e.key === "z" && (e.ctrlKey || e.metaKey)) e.shiftKey ? step(undone, past) : step(past, undone);
      else if (e.key === "Escape") (sel = null), draw();
      else if (d && sel) change(() => (mark(sel).points = mark(sel).points.map(([x, y]) => [+(x + d[0] * 0.01).toFixed(4), +(y + d[1] * 0.01).toFixed(4)])));
      else return;
      e.preventDefault();
    });
    // The words of a text, or a measurement's length, typed where it is.
    let field = null;
    function words(m, fresh) {
      const isText = m.type === "text", c = canvas.getBoundingClientRect(), s = stage.getBoundingClientRect(), p = isText ? m.points[0] : [(m.points[0][0] + m.points[1][0]) / 2, (m.points[0][1] + m.points[1][1]) / 2];
      const input = make("input", { "data-an-text": "", "data-w-field": "", type: isText ? "text" : "number", inputmode: isText ? null : "decimal", min: 0, step: "any", "aria-label": isText ? L("Text") : L("Its length in {unit}", { unit: unit() }), placeholder: isText ? L("Text") : L("Length in {unit}", { unit: unit() }) });
      input.value = isText ? m.text ?? "" : m.length ?? "";
      input.style.left = `${K.clamp(c.left - s.left + p[0] * c.width, 82, Math.max(82, s.width - 82))}px`;
      input.style.top = `${c.top - s.top + p[1] * c.height}px`;
      let over = false;
      const done = (keepIt) => {
        if (over) return;
        over = true;
        field = null;
        const v = input.value.trim();
        input.remove();
        if (fresh && (!keepIt || !v)) return draw();
        if (fresh) return change(() => (marks.push({ ...m, text: v }), (sel = m.id)));
        if (keepIt) change(() => (isText ? (v ? (mark(m.id).text = v) : (marks = marks.filter((x) => x.id !== m.id))) : (mark(m.id).length = Number(v) > 0 ? Number(v) : null)));
        // The mark itself is new to state, whatever was typed.
        if (last !== copy()) save();
      };
      input.addEventListener("keydown", (e) => {
        e.stopPropagation();
        if (e.key === "Enter") done(true), canvas.focus({ preventScroll: true });
        else if (e.key === "Escape") done(false), canvas.focus({ preventScroll: true });
      });
      input.addEventListener("blur", () => done(true));
      field = () => done(true);
      stage.append(input);
      input.focus({ preventScroll: true });
    }
    const finish = () => field?.();
    function size() {
      if (!img) return;
      const w = Math.min(stage.clientWidth, img.naturalWidth), most = Math.max(240, innerHeight * 0.7), k = Math.min(w / img.naturalWidth, most / img.naturalHeight), r = Math.max(1, devicePixelRatio || 1);
      const now = [Math.round(img.naturalWidth * k), Math.round(img.naturalHeight * k)];
      if (!now[0] || (box && box[0] === now[0] && box[1] === now[1])) return;
      box = now;
      canvas.style.width = `${now[0]}px`;
      canvas.style.height = `${now[1]}px`;
      canvas.width = now[0] * r;
      canvas.height = now[1] * r;
    }
    function draw() {
      for (const t of tools) t.setAttribute("aria-pressed", String(t.getAttribute("data-tool") === tool));
      [...swatches.children].forEach((s, i) => s.setAttribute("aria-checked", String(colors[i] === color)));
      canvas.setAttribute("data-tool", tool);
      undo.disabled = !past.length;
      redo.disabled = !undone.length;
      gone.disabled = !sel;
      if (!img) return;
      size();
      const r = canvas.width / (parseFloat(canvas.style.width) || 1), g = canvas.getContext("2d");
      g.setTransform(r, 0, 0, r, 0, 0);
      paint(g, canvas.width / r, canvas.height / r, true);
    }
    async function load(src) {
      error.textContent = "";
      img = null;
      box = null;
      past = [];
      undone = [];
      sel = null;
      canvas.hidden = true;
      if (!src) return;
      try {
        const i = await picture(src);
        if (src !== srcWas) return;
        img = i;
        canvas.hidden = false;
        draw();
        if (cfg.bind && marks.length) api.set(cfg.bind, result());
      } catch (e) {
        error.textContent = e.message;
      }
    }
    function update() {
      const v = K.value(api, cfg.src), url = isObj(v) ? v.url : v, src = typeof url === "string" && /^(data:image\/|https?:|blob:|[\w./-])/i.test(url) && !/^\s*(javascript|vbscript):/i.test(url) ? url : null;
      if (cfg.bindMarks) {
        const kept = JSON.stringify(list(api.state[cfg.bindMarks]));
        if (kept !== copy() && !drag && !field) (marks = JSON.parse(kept)), (last = kept), srcWas === src && draw();
      }
      if (src !== srcWas) {
        // Another picture's marks aren't this one's.
        if (srcWas !== null && !cfg.bindMarks) marks = [];
        srcWas = src;
        load(src);
      }
    }
    new ResizeObserver(() => img && draw()).observe(stage);
    update();
    draw();
    return { update };
  };
})();
