// irender-ui-kit: `ui-document-scanner`. The page's edges are found in a small grey copy of
// the picture (the largest bright patch's corners), and it's straightened by working out
// where each pixel of the result comes from.
(() => {
  const B = (window.__irenderBehaviors ||= {});
  const K = window.__irenderKit, D = window.__irenderDevice, L = (t, v) => window.__irender.label(t, v);
  const { make, icon, isObj, blank } = K, btn = D.btn;
  const list = (v) => (Array.isArray(v) ? v : []);
  const NS = "http://www.w3.org/2000/svg";

  // ---- finding the page: [[x, y] × 4] from the top left, as parts of the picture
  const WHOLE = [[0.04, 0.04], [0.96, 0.04], [0.96, 0.96], [0.04, 0.96]];
  function edges(source, sw, sh) {
    const k = 220 / Math.max(sw, sh), w = Math.max(8, Math.round(sw * k)), h = Math.max(8, Math.round(sh * k));
    const c = make("canvas", { width: w, height: h }), g = c.getContext("2d", { willReadFrequently: true });
    g.filter = "blur(1px)";
    g.drawImage(source, 0, 0, w, h);
    const px = g.getImageData(0, 0, w, h).data, grey = new Uint8Array(w * h), hist = new Array(256).fill(0);
    for (let i = 0; i < grey.length; i++) hist[(grey[i] = (px[i * 4] * 77 + px[i * 4 + 1] * 151 + px[i * 4 + 2] * 28) >> 8)]++;
    // Otsu's threshold: the split with the least mixed sides.
    let sum = 0, total = grey.length, sumB = 0, wB = 0, best = 0, cut = 128;
    hist.forEach((n, v) => (sum += n * v));
    for (let t = 0; t < 256; t++) {
      wB += hist[t];
      if (!wB) continue;
      const wF = total - wB;
      if (!wF) break;
      sumB += t * hist[t];
      const d = (sumB / wB - (sum - sumB) / wF) ** 2 * wB * wF;
      if (d > best) (best = d), (cut = t);
    }
    // The largest bright patch.
    const seen = new Uint8Array(w * h);
    let patch = null;
    for (let s = 0; s < grey.length; s++) {
      if (seen[s] || grey[s] <= cut) continue;
      const stack = [s], got = [];
      seen[s] = 1;
      while (stack.length) {
        const p = stack.pop(), x = p % w;
        got.push(p);
        for (const q of [x > 0 ? p - 1 : -1, x < w - 1 ? p + 1 : -1, p - w, p + w]) if (q >= 0 && q < grey.length && !seen[q] && grey[q] > cut) (seen[q] = 1), stack.push(q);
      }
      if (!patch || got.length > patch.length) patch = got;
    }
    if (!patch || patch.length < total * 0.12 || patch.length > total * 0.97) return WHOLE;
    const far = [null, null, null, null], score = [Infinity, Infinity, Infinity, Infinity];
    for (const p of patch) {
      const x = p % w, y = (p / w) | 0;
      [x + y, y - x, -x - y, x - y].forEach((v, i) => v < score[i] && ((score[i] = v), (far[i] = [x / (w - 1), y / (h - 1)])));
    }
    const [a, b, cc, d] = far, area = Math.abs((a[0] * b[1] - b[0] * a[1] + b[0] * cc[1] - cc[0] * b[1] + cc[0] * d[1] - d[0] * cc[1] + d[0] * a[1] - a[0] * d[1]) / 2);
    // A patch that doesn't fill its corners isn't a page.
    return area > 0.1 && patch.length / total > area * 0.75 ? far : WHOLE;
  }

  // ---- straightening
  // The eight numbers that take the corners of a w × h page to `q`.
  function homography(q, w, h) {
    const src = [[0, 0], [w, 0], [w, h], [0, h]], m = [];
    src.forEach(([x, y], i) => {
      const [u, v] = q[i];
      m.push([x, y, 1, 0, 0, 0, -u * x, -u * y, u], [0, 0, 0, x, y, 1, -v * x, -v * y, v]);
    });
    for (let i = 0; i < 8; i++) {
      let p = i;
      for (let r = i + 1; r < 8; r++) if (Math.abs(m[r][i]) > Math.abs(m[p][i])) p = r;
      [m[i], m[p]] = [m[p], m[i]];
      if (Math.abs(m[i][i]) < 1e-9) return null;
      for (let r = 0; r < 8; r++) if (r !== i) {
        const f = m[r][i] / m[i][i];
        for (let c = i; c < 9; c++) m[r][c] -= f * m[i][c];
      }
    }
    return m.map((r, i) => r[8] / r[i]);
  }
  const span = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1]);
  function straighten(canvas, corners, most) {
    const W = canvas.width, H = canvas.height, q = corners.map(([x, y]) => [x * (W - 1), y * (H - 1)]);
    let w = Math.max(span(q[0], q[1]), span(q[3], q[2])), h = Math.max(span(q[0], q[3]), span(q[1], q[2]));
    const k = Math.min(1, most / Math.max(w, h));
    w = Math.max(16, Math.round(w * k));
    h = Math.max(16, Math.round(h * k));
    const m = homography(q, w, h), src = canvas.getContext("2d").getImageData(0, 0, W, H).data, out = new ImageData(w, h), o = out.data;
    if (!m) return null;
    for (let y = 0, i = 0; y < h; y++) for (let x = 0; x < w; x++, i += 4) {
      const z = m[6] * x + m[7] * y + 1, sx = (m[0] * x + m[1] * y + m[2]) / z, sy = (m[3] * x + m[4] * y + m[5]) / z;
      const x0 = K.clamp(Math.floor(sx), 0, W - 2), y0 = K.clamp(Math.floor(sy), 0, H - 2), fx = K.clamp(sx - x0, 0, 1), fy = K.clamp(sy - y0, 0, 1), p = (y0 * W + x0) * 4;
      for (let c = 0; c < 3; c++) o[i + c] = (src[p + c] * (1 - fx) + src[p + 4 + c] * fx) * (1 - fy) + (src[p + W * 4 + c] * (1 - fx) + src[p + W * 4 + 4 + c] * fx) * fy;
      o[i + 3] = 255;
    }
    return out;
  }
  // Cleaned up against the paper around each pixel: its brightness nearby, from a summed
  // table, so shadows go too.
  function clean(img, how) {
    if (how === "none") return img;
    const { width: w, height: h, data: d } = img, n = w + 1, sum = new Float64Array(n * (h + 1)), grey = new Float32Array(w * h), r = Math.max(8, Math.round(Math.max(w, h) / 28));
    for (let y = 0; y < h; y++) {
      let row = 0;
      for (let x = 0; x < w; x++) {
        const i = y * w + x, v = (grey[i] = d[i * 4] * 0.3 + d[i * 4 + 1] * 0.59 + d[i * 4 + 2] * 0.11);
        row += v;
        sum[(y + 1) * n + x + 1] = sum[y * n + x + 1] + row;
      }
    }
    // The paper's own brightness: what's far darker is ink, however much of it there is.
    const hist = new Uint32Array(256);
    for (const v of grey) hist[Math.min(255, v | 0)]++;
    let paper = 255, left = grey.length * 0.1;
    while (paper > 80 && (left -= hist[paper]) > 0) paper--;
    for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
      const x0 = Math.max(0, x - r), x1 = Math.min(w, x + r + 1), y0 = Math.max(0, y - r), y1 = Math.min(h, y + r + 1);
      const mean = (sum[y1 * n + x1] - sum[y0 * n + x1] - sum[y1 * n + x0] + sum[y0 * n + x0]) / ((x1 - x0) * (y1 - y0)), i = y * w + x, j = i * 4;
      if (how === "bw") d[j] = d[j + 1] = d[j + 2] = grey[i] < mean * 0.86 || grey[i] < paper * 0.36 ? 0 : 255;
      else {
        const f = 245 / Math.max(paper * 0.55, mean);
        if (how === "grey") d[j] = d[j + 1] = d[j + 2] = Math.min(255, Math.max(0, (grey[i] * f - 20) * 1.1));
        else for (let c = 0; c < 3; c++) d[j + c] = Math.min(255, Math.max(0, (d[j + c] * f - 12) * 1.06));
      }
    }
    return img;
  }

  // ---- a PDF of JPEGs, a page each
  function pdf(pages, paper) {
    const parts = [], at = [], enc = new TextEncoder();
    let size = 0;
    const put = (x) => {
      const b = typeof x === "string" ? enc.encode(x) : x;
      parts.push(b);
      size += b.length;
    };
    const obj = (n, f) => ((at[n] = size), put(`${n} 0 obj\n`), f(), put("\nendobj\n"));
    put("%PDF-1.4\n%âãÏÓ\n");
    obj(1, () => put("<< /Type /Catalog /Pages 2 0 R >>"));
    obj(2, () => put(`<< /Type /Pages /Count ${pages.length} /Kids [${pages.map((_, i) => `${3 + i * 3} 0 R`).join(" ")}] >>`));
    pages.forEach((p, i) => {
      const n = 3 + i * 3, sheet = { a4: [595.28, 841.89], letter: [612, 792] }[paper];
      let [W, H] = sheet ?? (p.width > p.height ? [842, (842 * p.height) / p.width] : [(842 * p.width) / p.height, 842]);
      if (sheet && p.width > p.height) [W, H] = [H, W];
      const k = Math.min(W / p.width, H / p.height), w = p.width * k, h = p.height * k, draw = `q ${w.toFixed(2)} 0 0 ${h.toFixed(2)} ${((W - w) / 2).toFixed(2)} ${((H - h) / 2).toFixed(2)} cm /I Do Q`;
      obj(n, () => put(`<< /Type /Page /Parent 2 0 R /MediaBox [0 0 ${W.toFixed(2)} ${H.toFixed(2)}] /Resources << /XObject << /I ${n + 1} 0 R >> >> /Contents ${n + 2} 0 R >>`));
      obj(n + 1, () => (put(`<< /Type /XObject /Subtype /Image /Width ${p.width} /Height ${p.height} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length ${p.bytes.length} >>\nstream\n`), put(p.bytes), put("\nendstream")));
      obj(n + 2, () => put(`<< /Length ${draw.length} >>\nstream\n${draw}\nendstream`));
    });
    const start = size;
    put(`xref\n0 ${at.length}\n0000000000 65535 f \n${at.slice(1).map((a) => `${String(a).padStart(10, "0")} 00000 n \n`).join("")}trailer\n<< /Size ${at.length} /Root 1 0 R >>\nstartxref\n${start}\n%%EOF`);
    return new Blob(parts, { type: "application/pdf" });
  }
  window.__irenderScan = { edges, straighten, clean, pdf };

  B["ui-document-scanner"] = (el, api) => {
    const cfg = api.config, most = cfg.maxSize ?? 1800, quality = cfg.quality ?? 0.75, can = !!navigator.mediaDevices?.getUserMedia;
    let stream = null, pic = null, corners = null, filter = cfg.filter ?? "colour", busy = false, seen = 0, found = null, drag = null, was = null, pdfFor = null;
    const video = make("video", { playsinline: true, "aria-label": L("Camera") });
    video.muted = true;
    const canvas = make("canvas", { role: "img", "aria-label": L("The picture, with the page's corners on it") });
    const over = document.createElementNS(NS, "svg"), poly = document.createElementNS(NS, "polygon");
    over.setAttribute("data-ds-over", "");
    over.append(poly);
    const names = [L("Top left corner"), L("Top right corner"), L("Bottom right corner"), L("Bottom left corner")];
    const dots = names.map((n, i) => K.button({ "data-ds-corner": i, "aria-label": L("{corner}: the arrow keys move it", { corner: n }) }));
    const view = make("div", { "data-ds-view": "", hidden: true }, video, canvas, over, dots), body = D.body(el), error = make("p", { "data-w-error": "", role: "alert" });
    el.prepend(view);
    el.append(error);
    const all = () => list(api.state[cfg.bind]).filter(isObj);
    const full = () => cfg.max && all().length >= cfg.max;
    function stop() {
      stream?.getTracks().forEach((t) => t.stop());
      stream = null;
      video.srcObject = null;
      clearTimeout(seen);
      found = null;
    }
    async function open() {
      error.textContent = "";
      try {
        stream = await navigator.mediaDevices.getUserMedia({ video: { facingMode: { ideal: "environment" }, width: { ideal: 2560 }, height: { ideal: 1440 } }, audio: false });
        if (!el.isConnected) return stop();
        video.srcObject = stream;
        stream.getVideoTracks()[0].addEventListener("ended", () => (stop(), draw()));
        draw();
        await video.play().catch(() => {});
        const look = () => {
          if (!stream || !el.isConnected) return;
          if (video.videoWidth) (found = edges(video, video.videoWidth, video.videoHeight)), place();
          seen = setTimeout(look, 260);
        };
        look();
        body.querySelector("[data-snap]")?.focus();
      } catch (e) {
        stop();
        error.textContent = D.explain(e, "camera") ?? "";
        api.emit("failed", { name: e.name, error: error.textContent });
        draw();
      }
    }
    // A picture to straighten, from the camera or a file.
    function take(source, w, h) {
      const k = Math.min(1, (most * 1.4) / Math.max(w, h));
      canvas.width = Math.round(w * k);
      canvas.height = Math.round(h * k);
      canvas.getContext("2d", { willReadFrequently: true }).drawImage(source, 0, 0, canvas.width, canvas.height);
      corners = edges(canvas, canvas.width, canvas.height).map((c) => [...c]);
      stop();
      pic = true;
      draw();
      requestAnimationFrame(place);
      dots[0].focus({ preventScroll: true });
    }
    async function file(f) {
      error.textContent = "";
      try {
        const i = await createImageBitmap(f);
        take(i, i.width, i.height);
      } catch {
        error.textContent = L("The picture couldn't be read");
      }
    }
    // The corners over what's shown.
    function place() {
      const shown = pic ? canvas : video, q = pic ? corners : found, b = shown.getBoundingClientRect(), v = view.getBoundingClientRect();
      over.style.display = q && b.width ? "" : "none";
      dots.forEach((d) => (d.hidden = !pic));
      if (!q || !b.width) return;
      let w = b.width, h = b.height, x = b.left - v.left, y = b.top - v.top;
      if (!pic && video.videoWidth) {
        // The video's own box, within its element.
        const k = Math.min(w / video.videoWidth, h / video.videoHeight);
        x += (w - video.videoWidth * k) / 2;
        y += (h - video.videoHeight * k) / 2;
        w = video.videoWidth * k;
        h = video.videoHeight * k;
      }
      Object.assign(over.style, { left: `${x}px`, top: `${y}px`, width: `${w}px`, height: `${h}px` });
      over.setAttribute("viewBox", `0 0 ${w} ${h}`);
      poly.setAttribute("points", q.map(([px, py]) => `${px * w},${py * h}`).join(" "));
      if (pic) dots.forEach((d, i) => ((d.style.left = `${x + q[i][0] * w}px`), (d.style.top = `${y + q[i][1] * h}px`)));
    }
    dots.forEach((d, i) => {
      d.addEventListener("pointerdown", (e) => {
        if (e.button !== 0) return;
        e.preventDefault();
        K.capture(d, e);
        d.focus({ preventScroll: true });
        drag = e.pointerId;
      });
      d.addEventListener("pointermove", (e) => {
        if (drag !== e.pointerId) return;
        const b = canvas.getBoundingClientRect();
        corners[i] = [K.clamp((e.clientX - b.left) / b.width, 0, 1), K.clamp((e.clientY - b.top) / b.height, 0, 1)];
        place();
      });
      for (const n of ["pointerup", "pointercancel"]) d.addEventListener(n, () => (drag = null));
      d.addEventListener("keydown", (e) => {
        const m = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] }[e.key], by = e.shiftKey ? 0.002 : 0.01;
        if (!m) return;
        e.preventDefault();
        corners[i] = [K.clamp(corners[i][0] + m[0] * by, 0, 1), K.clamp(corners[i][1] + m[1] * by, 0, 1)];
        place();
      });
    });
    function accept() {
      const img = straighten(canvas, corners, most);
      if (!img) return (error.textContent = L("Those corners don't make a page"));
      clean(img, filter);
      const c = make("canvas", { width: img.width, height: img.height });
      c.getContext("2d").putImageData(img, 0, 0);
      const page = { id: K.id(), url: c.toDataURL("image/jpeg", quality), width: img.width, height: img.height };
      pic = null;
      put([...all(), page]);
      K.say(el, L("Page {n} added", { n: all().length }));
      api.emit("page", page);
    }
    function put(pages) {
      api.set(cfg.bind, pages);
      draw();
      bindPdf();
    }
    const bytes = (url) => new Promise((ok, no) => {
      // Pages that came some other way are made JPEGs.
      if (/^data:image\/jpeg;base64,/.test(url)) return ok(Uint8Array.from(atob(url.slice(url.indexOf(",") + 1)), (c) => c.charCodeAt(0)));
      const i = new Image();
      i.onload = () => {
        const c = make("canvas", { width: i.naturalWidth, height: i.naturalHeight });
        c.getContext("2d").drawImage(i, 0, 0);
        ok(Uint8Array.from(atob(c.toDataURL("image/jpeg", quality).split(",")[1]), (x) => x.charCodeAt(0)));
      };
      i.onerror = () => no(new Error(L("The picture couldn't be read")));
      i.src = url;
    });
    async function build() {
      const pages = [];
      for (const p of all()) pages.push({ ...p, bytes: await bytes(p.url) });
      return pdf(pages, cfg.paper);
    }
    const fileName = () => String(K.value(api, cfg.name) ?? "scan.pdf");
    async function bindPdf() {
      const key = all().map((p) => p.id).join();
      if (!cfg.bindPdf || key === pdfFor) return;
      pdfFor = key;
      if (!key) return api.set(cfg.bindPdf, null);
      const blob = await build();
      if (pdfFor === key) api.set(cfg.bindPdf, { url: await D.dataUrl(blob), name: fileName(), type: "application/pdf", size: blob.size, pages: all().length });
    }
    function draw() {
      const now = all(), label = K.value(api, cfg.label) ?? (now.length ? L("Scan another page") : L("Scan a page")), sig = JSON.stringify([now.map((p) => p.id), label, !!stream, !!pic, filter, full()]);
      if (sig === was) return;
      was = sig;
      view.hidden = !stream && !pic;
      video.hidden = !stream;
      canvas.hidden = !pic;
      const focus = body.contains(document.activeElement) ? document.activeElement.getAttribute("data-f") : null;
      const filters = { colour: L("Colour"), grey: L("Grey"), bw: L("Black and white"), none: L("As it is") };
      const move = (i, to) => {
        const next = [...now];
        next.splice(to, 0, ...next.splice(i, 1));
        put(next);
      };
      const pick = make("input", { type: "file", accept: "image/*", capture: can ? null : "environment", change: (e) => { const f = e.target.files[0]; e.target.value = ""; f && file(f); } });
      body.replaceChildren(...[
        pic ? make("div", { "data-w-row": "" }, make("select", { "data-w-field": "", "data-f": "filter", "aria-label": L("Clean up"), change: (e) => ((filter = e.target.value), draw()) }, Object.entries(filters).map(([v, t]) => make("option", { value: v, selected: v === filter }, t))), btn("", { "data-f": "find", click: () => ((corners = edges(canvas, canvas.width, canvas.height).map((c) => [...c])), place()) }, L("Find the edges")), btn("primary", { "data-f": "ok", "data-accept": "", click: accept }, icon("check", 15), L("Add the page")), btn("ghost", { "data-f": "no", click: () => ((pic = null), draw()) }, L("Cancel")))
          : stream ? make("div", { "data-w-row": "" }, btn("primary", { "data-snap": "", click: () => video.videoWidth && take(video, video.videoWidth, video.videoHeight) }, icon("camera", 16), L("Take it")), btn("ghost", { click: () => (stop(), draw()) }, L("Close")))
          : full() ? null : make("div", { "data-w-row": "" }, can ? btn("primary", { "data-open": "", "data-f": "open", click: open }, icon("scan", 16), label) : null, make("label", { "data-w-btn": can ? "" : "primary" }, icon("file", 15), can ? L("From a picture") : label, pick)),
        now.length ? make("ol", { "data-ds-pages": "" }, now.map((p, i) => make("li", null, make("img", { src: /^(data:image\/|https?:|\/|\.)/i.test(p.url ?? "") ? p.url : "", alt: L("Page {n}", { n: i + 1 }) }), make("div", null, i > 0 ? K.button({ "data-f": `l${p.id}`, "aria-label": L("Move page {n} earlier", { n: i + 1 }), click: () => move(i, i - 1) }, icon("left", 13)) : null, String(i + 1), i < now.length - 1 ? K.button({ "data-f": `r${p.id}`, "aria-label": L("Move page {n} later", { n: i + 1 }), click: () => move(i, i + 1) }, icon("right", 13)) : null, K.button({ "aria-label": L("Delete page {n}", { n: i + 1 }), click: () => put(now.filter((x) => x !== p && x.id !== p.id)) }, icon("trash", 13)))))) : null,
        now.length && !pic && !stream ? make("div", { "data-w-row": "" }, btn("", { "data-f": "save", "data-pdf": "", click: async () => window.__irenderSheets.save(fileName(), await build()) }, icon("download", 15), L("Save as PDF"))) : null].filter(Boolean));
      if (focus) body.querySelector(`[data-f="${CSS.escape(focus)}"]`)?.focus();
      place();
    }
    new ResizeObserver(place).observe(view);
    document.addEventListener("visibilitychange", () => document.visibilityState === "hidden" && stream && (stop(), draw()));
    D.gone(el, stop);
    draw();
    bindPdf();
    return { update: () => (draw(), bindPdf()) };
  };
})();
