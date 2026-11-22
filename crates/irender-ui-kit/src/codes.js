// irender-ui-kit: `ui-qr-code`, `ui-barcode` and `ui-code-scanner`. The QR encoder takes
// bytes (UTF-8) in versions 1 to 40 and picks the mask that reads best, as the standard
// (ISO/IEC 18004) sets out; the barcodes are bars and gaps from each kind's own table.
(() => {
  const B = (window.__irenderBehaviors ||= {});
  const K = window.__irenderKit, L = (t, v) => window.__irender.label(t, v);
  const NS = "http://www.w3.org/2000/svg";
  const svg = (tag, attrs, ...kids) => {
    const e = document.createElementNS(NS, tag);
    for (const [k, v] of Object.entries(attrs)) if (v !== null && v !== undefined) e.setAttribute(k, v);
    e.append(...kids);
    return e;
  };
  const color = (c, d) => (typeof c === "string" && /^[#\w(),.%\s/-]+$/.test(c) ? c : d);

  // ---------------------------------------------------------------- QR
  // Error-correction codewords per block, and blocks, by level then version.
  const ECC = { L: [7, 10, 15, 20, 26, 18, 20, 24, 30, 18, 20, 24, 26, 30, 22, 24, 28, 30, 28, 28, 28, 28, 30, 30, 26, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30], M: [10, 16, 26, 18, 24, 16, 18, 22, 22, 26, 30, 22, 22, 24, 24, 28, 28, 26, 26, 26, 26, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28], Q: [13, 22, 18, 26, 18, 24, 18, 22, 20, 24, 28, 26, 24, 20, 30, 24, 28, 28, 26, 30, 28, 30, 30, 30, 30, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30], H: [17, 28, 22, 16, 22, 28, 26, 26, 24, 28, 24, 28, 22, 24, 24, 30, 28, 28, 26, 28, 30, 24, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30] };
  const BLOCKS = { L: [1, 1, 1, 1, 1, 2, 2, 2, 2, 4, 4, 4, 4, 4, 6, 6, 6, 6, 7, 8, 8, 9, 9, 10, 12, 12, 12, 13, 14, 15, 16, 17, 18, 19, 19, 20, 21, 22, 24, 25], M: [1, 1, 1, 2, 2, 4, 4, 4, 5, 5, 5, 8, 9, 9, 10, 10, 11, 13, 14, 16, 17, 17, 18, 20, 21, 23, 25, 26, 28, 29, 31, 33, 35, 37, 38, 40, 43, 45, 47, 49], Q: [1, 1, 2, 2, 4, 4, 6, 6, 8, 8, 8, 10, 12, 16, 12, 17, 16, 18, 21, 20, 23, 23, 25, 27, 29, 34, 34, 35, 38, 40, 43, 45, 48, 51, 53, 56, 59, 62, 65, 68], H: [1, 1, 2, 4, 4, 4, 5, 6, 8, 8, 11, 11, 16, 16, 18, 16, 19, 21, 25, 25, 25, 34, 30, 32, 35, 37, 40, 42, 45, 48, 51, 54, 57, 60, 63, 66, 70, 74, 77, 81] };
  // Modules left for data after the fixed patterns.
  function raw(v) {
    let n = (16 * v + 128) * v + 64;
    if (v >= 2) {
      const a = Math.floor(v / 7) + 2;
      n -= (25 * a - 10) * a - 55;
      if (v >= 7) n -= 36;
    }
    return n;
  }
  const room = (v, l) => Math.floor(raw(v) / 8) - ECC[l][v - 1] * BLOCKS[l][v - 1];
  function mul(x, y) {
    let z = 0;
    for (let i = 7; i >= 0; i--) z = ((z << 1) ^ ((z >>> 7) * 0x11d)) ^ (((y >>> i) & 1) * x);
    return z & 255;
  }
  function divisor(n) {
    const out = new Uint8Array(n);
    out[n - 1] = 1;
    let root = 1;
    for (let i = 0; i < n; i++) {
      for (let j = 0; j < n; j++) out[j] = mul(out[j], root) ^ (out[j + 1] ?? 0);
      root = mul(root, 2);
    }
    return out;
  }
  function remainder(data, div) {
    const out = new Array(div.length).fill(0);
    for (const b of data) {
      const f = b ^ out.shift();
      out.push(0);
      for (let i = 0; i < div.length; i++) out[i] ^= mul(div[i], f);
    }
    return out;
  }
  // `smallest`: the version it starts from, for a code with a logo to have room around it.
  function qr(text, level, smallest = 1) {
    const bytes = [...new TextEncoder().encode(text)];
    let v = smallest;
    const bitsFor = (x) => 4 + (x < 10 ? 8 : 16) + bytes.length * 8;
    while (v <= 40 && bitsFor(v) > room(v, level) * 8) v++;
    if (v > 40) return null;
    const bits = [], put = (n, len) => { for (let i = len - 1; i >= 0; i--) bits.push((n >>> i) & 1); };
    put(4, 4);
    put(bytes.length, v < 10 ? 8 : 16);
    bytes.forEach((b) => put(b, 8));
    const cap = room(v, level) * 8;
    put(0, Math.min(4, cap - bits.length));
    put(0, (8 - (bits.length % 8)) % 8);
    for (let p = 0xec; bits.length < cap; p ^= 0xec ^ 0x11) put(p, 8);
    const data = [];
    for (let i = 0; i < bits.length; i += 8) data.push(bits.slice(i, i + 8).reduce((a, b) => (a << 1) | b, 0));
    // Blocks, their correction, and the two interleaved.
    const nb = BLOCKS[level][v - 1], ne = ECC[level][v - 1], total = Math.floor(raw(v) / 8), short = nb - (total % nb), len = Math.floor(total / nb), div = divisor(ne), blocks = [];
    for (let i = 0, k = 0; i < nb; i++) {
      const d = data.slice(k, (k += len - ne + (i < short ? 0 : 1))), e = remainder(d, div);
      if (i < short) d.push(null);
      blocks.push([...d, ...e]);
    }
    const all = [];
    for (let i = 0; i < blocks[0].length; i++) for (const b of blocks) if (b[i] !== null) all.push(b[i]);

    const size = v * 4 + 17, m = Array.from({ length: size }, () => new Array(size).fill(false)), fixed = Array.from({ length: size }, () => new Array(size).fill(false));
    const set = (x, y, on) => { if (x >= 0 && y >= 0 && x < size && y < size) (m[y][x] = !!on), (fixed[y][x] = true); };
    for (let i = 0; i < size; i++) set(6, i, i % 2 === 0), set(i, 6, i % 2 === 0);
    for (const [cx, cy] of [[3, 3], [size - 4, 3], [3, size - 4]]) for (let y = -4; y <= 4; y++) for (let x = -4; x <= 4; x++) { const d = Math.max(Math.abs(x), Math.abs(y)); set(cx + x, cy + y, d !== 2 && d !== 4); }
    const align = [];
    if (v > 1) {
      const n = Math.floor(v / 7) + 2, step = v === 32 ? 26 : Math.ceil((v * 4 + 4) / (n * 2 - 2)) * 2;
      for (let p = size - 7; align.length < n - 1; p -= step) align.unshift(p);
      align.unshift(6);
    }
    align.forEach((y, i) => align.forEach((x, j) => {
      if ((i === 0 && j === 0) || (i === 0 && j === align.length - 1) || (i === align.length - 1 && j === 0)) return;
      for (let a = -2; a <= 2; a++) for (let b = -2; b <= 2; b++) set(x + b, y + a, Math.max(Math.abs(a), Math.abs(b)) !== 1);
    }));
    const format = (mask) => {
      const d = ({ L: 1, M: 0, Q: 3, H: 2 }[level] << 3) | mask;
      let r = d;
      for (let i = 0; i < 10; i++) r = (r << 1) ^ ((r >>> 9) * 0x537);
      const f = ((d << 10) | r) ^ 0x5412, bit = (i) => (f >>> i) & 1;
      for (let i = 0; i <= 5; i++) set(8, i, bit(i));
      set(8, 7, bit(6)), set(8, 8, bit(7)), set(7, 8, bit(8));
      for (let i = 9; i < 15; i++) set(14 - i, 8, bit(i));
      for (let i = 0; i < 8; i++) set(size - 1 - i, 8, bit(i));
      for (let i = 8; i < 15; i++) set(8, size - 15 + i, bit(i));
      set(8, size - 8, 1);
    };
    format(0);
    if (v >= 7) {
      let r = v;
      for (let i = 0; i < 12; i++) r = (r << 1) ^ ((r >>> 11) * 0x1f25);
      const b = (v << 12) | r;
      for (let i = 0; i < 18; i++) {
        const on = (b >>> i) & 1, a = size - 11 + (i % 3), c = Math.floor(i / 3);
        set(a, c, on), set(c, a, on);
      }
    }
    let k = 0;
    for (let right = size - 1; right >= 1; right -= 2) {
      if (right === 6) right = 5;
      for (let vert = 0; vert < size; vert++) for (let j = 0; j < 2; j++) {
        const x = right - j, y = ((right + 1) & 2) === 0 ? size - 1 - vert : vert;
        if (!fixed[y][x] && k < all.length * 8) m[y][x] = ((all[k >>> 3] >>> (7 - (k & 7))) & 1) === 1, k++;
      }
    }
    const MASK = [(x, y) => (x + y) % 2, (x, y) => y % 2, (x) => x % 3, (x, y) => (x + y) % 3, (x, y) => (Math.floor(x / 3) + Math.floor(y / 2)) % 2, (x, y) => ((x * y) % 2) + ((x * y) % 3), (x, y) => (((x * y) % 2) + ((x * y) % 3)) % 2, (x, y) => (((x + y) % 2) + ((x * y) % 3)) % 2];
    const apply = (i) => { for (let y = 0; y < size; y++) for (let x = 0; x < size; x++) if (!fixed[y][x] && MASK[i](x, y) === 0) m[y][x] = !m[y][x]; };
    function penalty() {
      let p = 0, dark = 0;
      const line = (get) => {
        for (let a = 0; a < size; a++) {
          let run = 0, last = null, bits = "";
          for (let b = 0; b < size; b++) {
            const on = get(a, b);
            run = on === last ? run + 1 : 1;
            last = on;
            if (run === 5) p += 3;
            else if (run > 5) p++;
            bits += on ? "1" : "0";
          }
          const padded = `0000${bits}0000`;
          for (const pat of ["10111010000", "00001011101"]) for (let i = padded.indexOf(pat); i >= 0; i = padded.indexOf(pat, i + 1)) p += 40;
        }
      };
      line((a, b) => m[a][b]);
      line((a, b) => m[b][a]);
      for (let y = 0; y < size; y++) for (let x = 0; x < size; x++) {
        if (m[y][x]) dark++;
        if (x + 1 < size && y + 1 < size && m[y][x] === m[y][x + 1] && m[y][x] === m[y + 1][x] && m[y][x] === m[y + 1][x + 1]) p += 3;
      }
      return p + Math.floor(Math.abs((dark * 20) / (size * size) - 10)) * 10;
    }
    let best = 0, least = Infinity;
    for (let i = 0; i < 8; i++) {
      apply(i), format(i);
      const p = penalty();
      if (p < least) (least = p), (best = i);
      apply(i);
    }
    apply(best), format(best);
    return m;
  }
  window.__irenderQr = qr;

  // Saves the drawn code as it is, or as a picture. A logo from an address goes in as
  // data first: a file can't fetch it.
  async function inline(copy) {
    for (const im of copy.querySelectorAll("image")) {
      const src = im.getAttribute("href");
      if (src.startsWith("data:")) continue;
      try {
        const blob = await (await fetch(src)).blob();
        im.setAttribute("href", await new Promise((ok, no) => {
          const r = new FileReader();
          r.onload = () => ok(r.result);
          r.onerror = no;
          r.readAsDataURL(blob);
        }));
      } catch {
        // A site that doesn't share it: the code is saved without its logo.
        im.remove();
      }
    }
  }
  async function save(el, name) {
    const s = el.querySelector("svg[data-code]"), copy = s.cloneNode(true), box = s.viewBox.baseVal;
    copy.setAttribute("xmlns", NS);
    await inline(copy);
    const blob = new Blob([new XMLSerializer().serializeToString(copy)], { type: "image/svg+xml" });
    if (name.endsWith(".svg")) return window.__irenderSheets.save(blob, name);
    const url = URL.createObjectURL(blob), img = new Image(), k = Math.max(1, Math.round(1024 / box.width));
    img.onload = () => {
      const c = K.make("canvas", { width: box.width * k, height: box.height * k }), g = c.getContext("2d");
      g.fillStyle = "#fff";
      g.fillRect(0, 0, c.width, c.height);
      g.drawImage(img, 0, 0, c.width, c.height);
      URL.revokeObjectURL(url);
      c.toBlob((b) => b && window.__irenderSheets.save(b, name));
    };
    img.src = url;
  }
  const finish = (el, cfg, node, text) => {
    const b = cfg.download && node.tagName === "svg" ? K.button({ "data-code-save": "", click: () => save(el, cfg.download) }, L("Download")) : null;
    el.replaceChildren(...[node, text, b].filter(Boolean));
  };
  const failed = (why) => K.make("span", { "data-code-error": "", role: "img", "aria-label": why }, why);

  B["ui-qr-code"] = (el, api) => {
    const cfg = api.config;
    const iconTpl = el.querySelector("template[data-code-icon]"), iconSvg = iconTpl?.content.querySelector("svg");
    let was = null;
    const safe = (u) => typeof u === "string" && u.trim() !== "" && !/^\s*(javascript|vbscript):/i.test(u);
    function draw() {
      const text = String(K.value(api, cfg.value) ?? ""), logo = K.value(api, cfg.logo), letters = String(K.value(api, cfg.logoText) ?? "").trim();
      const fg = color(K.value(api, cfg.color), "#000"), bg = color(K.value(api, cfg.background), "#fff"), mark = safe(logo) ? "image" : iconSvg ? "icon" : letters ? "text" : null;
      const back = K.value(api, cfg.logoBackground), ink = color(K.value(api, cfg.logoColor), fg);
      const sig = JSON.stringify([text, logo, letters, fg, bg, back, ink, K.value(api, cfg.label)]);
      if (sig === was) return;
      was = sig;
      const m = text ? qr(text, mark ? "H" : cfg.level ?? "M", mark ? 3 : 1) : null;
      if (!m) return finish(el, cfg, failed(text ? L("Too long for a QR code") : L("Nothing to encode")));
      const n = m.length, q = 4, all = n + q * 2, mid = q + n / 2;
      let d = "";
      const eye = (x, y) => (x < 7 && y < 7) || (x >= n - 7 && y < 7) || (x < 7 && y >= n - 7);
      // The logo's space: an odd number of modules, so it's centred on them.
      let hole = mark ? Math.max(5, Math.round(n * (cfg.logoSize ?? 0.22))) : 0;
      if (hole && hole % 2 === 0) hole++;
      const shape = cfg.logoShape ?? "rounded", c = (n - 1) / 2, half = hole / 2;
      const under = (x, y) => hole && (shape === "circle" ? Math.hypot(x - c, y - c) < half + 0.35 : Math.abs(x - c) < half && Math.abs(y - c) < half);
      for (let y = 0; y < n; y++) for (let x = 0; x < n; x++) {
        if (!m[y][x] || under(x, y)) continue;
        if (cfg.dots === "round" && !eye(x, y)) d += `M${x + q + 0.5} ${y + q + 0.05}a.45 .45 0 1 0 .001 0z`;
        else d += `M${x + q} ${y + q}h1v1h-1z`;
      }
      const s = svg("svg", { "data-code": "", viewBox: `0 0 ${all} ${all}`, role: "img", "aria-label": K.value(api, cfg.label) ?? L("QR code: {value}", { value: text }), "shape-rendering": cfg.dots === "round" ? null : "crispEdges" }, svg("rect", { width: all, height: all, fill: bg }), svg("path", { d, fill: fg }));
      if (mark) {
        // A little inside its space, so modules don't touch it.
        const pad = 0.5, w = hole - pad * 2, at = mid - w / 2, r = shape === "circle" ? w / 2 : shape === "square" ? 0 : w * 0.18, id = `qr-${K.id()}`;
        const g = svg("g", { "shape-rendering": "geometricPrecision", "aria-hidden": "true" });
        if (back !== "none") g.append(svg("rect", { x: at, y: at, width: w, height: w, rx: r, fill: color(back, bg) }));
        // Cut to its shape; on a backing of its own colour, with room around it.
        const inset = back && back !== "none" ? w * 0.14 : shape === "circle" && mark !== "image" ? w * 0.1 : 0, iw = w - inset * 2, ia = at + inset;
        if (mark === "image") {
          g.append(svg("clipPath", { id }, svg("rect", { x: ia, y: ia, width: iw, height: iw, rx: Math.max(0, r - inset) })));
          g.append(svg("image", { href: logo, x: ia, y: ia, width: iw, height: iw, preserveAspectRatio: shape === "circle" ? "xMidYMid slice" : "xMidYMid meet", "clip-path": `url(#${id})` }));
        } else if (mark === "icon") {
          const i = iconSvg.cloneNode(true);
          i.removeAttribute("class");
          i.removeAttribute("style");
          for (const [k, v] of Object.entries({ x: ia + iw * 0.06, y: ia + iw * 0.06, width: iw * 0.88, height: iw * 0.88, color: ink })) i.setAttribute(k, v);
          g.append(i);
        } else {
          const t = svg("text", { x: mid, y: mid, "text-anchor": "middle", "dominant-baseline": "central", "font-family": "system-ui, sans-serif", "font-weight": 700, "font-size": Math.min(iw * 0.62, (iw / Math.max(1, [...letters].length)) * 1.25), fill: ink });
          t.textContent = [...letters].slice(0, 4).join("");
          g.append(t);
        }
        s.append(g);
      }
      finish(el, cfg, s);
      // Kept for the next time: drawing replaces what's inside.
      if (iconTpl) el.append(iconTpl);
    }
    draw();
    return { update: draw };
  };

  // ---------------------------------------------------------------- barcodes
  const C128 = "212222 222122 222221 121223 121322 131222 122213 122312 132212 221213 221312 231212 112232 122132 122231 113222 123122 123221 223211 221132 221231 213212 223112 312131 311222 321122 321221 312212 322112 322211 212123 212321 232121 111323 131123 131321 112313 132113 132311 211313 231113 231311 112133 112331 132131 113123 113321 133121 313121 211331 231131 213113 213311 213131 311123 311321 331121 312113 312311 332111 314111 221411 431111 111224 111422 121124 121421 141122 141221 112214 112412 122114 122411 142112 142211 241211 221114 413111 241112 134111 111242 121142 121241 114212 124112 124211 411212 421112 421211 212141 214121 412121 111143 111341 131141 114113 114311 411113 411311 113141 114131 311141 411131 211412 211214 211232 2331112".split(" ");
  const C39 = Object.fromEntries("0 nnnwwnwnn,1 wnnwnnnnw,2 nnwwnnnnw,3 wnwwnnnnn,4 nnnwwnnnw,5 wnnwwnnnn,6 nnwwwnnnn,7 nnnwnnwnw,8 wnnwnnwnn,9 nnwwnnwnn,A wnnnnwnnw,B nnwnnwnnw,C wnwnnwnnn,D nnnnwwnnw,E wnnnwwnnn,F nnwnwwnnn,G nnnnnwwnw,H wnnnnwwnn,I nnwnnwwnn,J nnnnwwwnn,K wnnnnnnww,L nnwnnnnww,M wnwnnnnwn,N nnnnwnnww,O wnnnwnnwn,P nnwnwnnwn,Q nnnnnnwww,R wnnnnnwwn,S nnwnnnwwn,T nnnnwnwwn,U wwnnnnnnw,V nwwnnnnnw,W wwwnnnnnn,X nwnnwnnnw,Y wwnnwnnnn,Z nwwnwnnnn,- nwnnnnwnw,. wwnnnnwnn,$ nwnwnwnnn,/ nwnwnnnwn,+ nwnnnwnwn,% nnnwnwnwn,* nwnnwnwnn".split(",").map((s) => [s[0], s.slice(2)]).concat([[" ", "nwwnnnwnn"]]));
  const EAN = ["3211", "2221", "2122", "1411", "1132", "1231", "1114", "1312", "1213", "3112"], PARITY = ["LLLLLL", "LLGLGG", "LLGGLG", "LLGGGL", "LGLLGG", "LGGLLG", "LGGGLL", "LGLGLG", "LGLGGL", "LGGLGL"];
  const ITF = ["nnwwn", "wnnnw", "nwnnw", "wwnnn", "nnwnw", "wnwnn", "nwwnn", "nnnww", "wnnwn", "nwnwn"];
  // Each: the widths of bars and gaps in turn (a bar first), and the text under it.
  const checkDigit = (s) => String((10 - ([...s].reverse().reduce((a, c, i) => a + Number(c) * (i % 2 ? 1 : 3), 0) % 10)) % 10);
  function ean(s, n) {
    s = s.replace(/[\s-]/g, "");
    if (!/^\d+$/.test(s) || (s.length !== n && s.length !== n - 1)) return null;
    if (s.length === n - 1) s += checkDigit(s);
    return s.endsWith(checkDigit(s.slice(0, -1))) ? s : null;
  }
  const KINDS = {
    code128(s) {
      if (!s || /[^\x00-\x7f]/.test(s)) return null;
      // Pairs of digits in code C, the rest in B (or A, for control characters).
      const codes = [];
      let set = null, i = 0;
      const start = { A: 103, B: 104, C: 105 }, to = { A: 101, B: 100, C: 99 };
      const use = (x) => { if (set !== x) codes.push(set === null ? start[x] : to[x]), (set = x); };
      while (i < s.length) {
        const run = /^\d+/.exec(s.slice(i))?.[0].length ?? 0;
        if (run >= (set === "C" ? 2 : set === null || i + run === s.length ? 4 : 6) || (run === 2 && s.length === 2)) {
          use("C");
          for (let n = run - (run % 2); n > 0; n -= 2, i += 2) codes.push(Number(s.slice(i, i + 2)));
          continue;
        }
        const c = s.charCodeAt(i);
        use(c < 32 ? "A" : c >= 96 || set === "C" || set === null ? "B" : set);
        codes.push(set === "A" ? (c < 32 ? c + 64 : c - 32) : c - 32);
        i++;
      }
      codes.push(codes.reduce((a, c, k) => a + c * Math.max(1, k), 0) % 103, 106);
      return { w: codes.map((c) => C128[c]).join("").split("").map(Number), text: s };
    },
    code39(s) {
      s = s.toUpperCase();
      if (!s || [...s].some((c) => !C39[c] || c === "*")) return null;
      return { w: [...`*${s}*`].flatMap((c, i) => [...(i ? [1] : []), ...[...C39[c]].map((x) => (x === "w" ? 3 : 1))]), text: s };
    },
    ean13(s) {
      const d = ean(s, 13);
      if (!d) return null;
      const par = PARITY[d[0]], w = [1, 1, 1];
      for (let i = 1; i <= 6; i++) w.push(...(par[i - 1] === "L" ? EAN[d[i]] : [...EAN[d[i]]].reverse().join("")).split("").map(Number));
      w.push(1, 1, 1, 1, 1);
      for (let i = 7; i <= 12; i++) w.push(...EAN[d[i]].split("").map(Number));
      return { w: [...w, 1, 1, 1], text: `${d[0]} ${d.slice(1, 7)} ${d.slice(7)}` };
    },
    ean8(s) {
      const d = ean(s, 8);
      if (!d) return null;
      const half = (a) => [...a].flatMap((c) => EAN[c].split("").map(Number));
      return { w: [1, 1, 1, ...half(d.slice(0, 4)), 1, 1, 1, 1, 1, ...half(d.slice(4)), 1, 1, 1], text: `${d.slice(0, 4)} ${d.slice(4)}` };
    },
    upca(s) {
      const d = ean(s, 12);
      return d ? { w: KINDS.ean13("0" + d).w, text: `${d[0]} ${d.slice(1, 6)} ${d.slice(6, 11)} ${d[11]}` } : null;
    },
    itf(s) {
      s = s.replace(/\s/g, "");
      if (!/^\d+$/.test(s)) return null;
      if (s.length % 2) s = "0" + s;
      const w = [1, 1, 1, 1];
      for (let i = 0; i < s.length; i += 2) for (let k = 0; k < 5; k++) w.push(ITF[s[i]][k] === "w" ? 3 : 1, ITF[s[i + 1]][k] === "w" ? 3 : 1);
      return { w: [...w, 3, 1, 1], text: s };
    },
  };
  window.__irenderBarcode = (type, value) => KINDS[type]?.(String(value)) ?? null;
  B["ui-barcode"] = (el, api) => {
    const cfg = api.config, type = cfg.type ?? "code128";
    let was = null;
    function draw() {
      const value = String(K.value(api, cfg.value) ?? ""), sig = JSON.stringify([value, K.value(api, cfg.label)]);
      if (sig === was) return;
      was = sig;
      const code = KINDS[type](value);
      if (!code) return finish(el, cfg, failed(value ? L("“{value}” can't be made into this kind of barcode", { value }) : L("Nothing to encode")));
      const u = cfg.barWidth ?? 2, h = cfg.height ?? 64, quiet = 10 * u, foot = cfg.showText ? 18 : 0;
      let x = quiet, d = "";
      code.w.forEach((w, i) => {
        if (i % 2 === 0) d += `M${x} 4h${w * u}v${h}h${-w * u}z`;
        x += w * u;
      });
      const width = x + quiet, height = h + 8 + foot;
      const s = svg("svg", { "data-code": "", width, viewBox: `0 0 ${width} ${height}`, role: "img", "aria-label": K.value(api, cfg.label) ?? L("Barcode: {value}", { value: code.text }), "shape-rendering": "crispEdges" }, svg("rect", { width, height, fill: "#fff" }), svg("path", { d, fill: "#000" }));
      if (cfg.showText) {
        const t = svg("text", { x: width / 2, y: h + 19, "text-anchor": "middle", "font-family": "ui-monospace, SFMono-Regular, Menlo, monospace", "font-size": 13, fill: "#000" });
        t.textContent = code.text;
        s.append(t);
      }
      finish(el, cfg, s);
    }
    draw();
    return { update: draw };
  };

  // ---------------------------------------------------------------- scanner
  B["ui-code-scanner"] = (el, api) => {
    const cfg = api.config, $ = (s) => el.querySelector(s);
    const view = $("[data-scan-view]"), video = $("video"), start = $("[data-scan-start]"), note = $("[data-scan-note]"), input = $("[data-scan-input]");
    let stream = null, detector = null, facing = "environment", last = null, lastAt = 0, frame = 0;
    const say = (t) => (note.textContent = t);
    function got(value, format) {
      const now = performance.now();
      if (value === last && now - lastAt < 2500) return;
      last = value;
      lastAt = now;
      api.set(cfg.bind, value);
      if (cfg.bindCode) api.set(cfg.bindCode, { value, format });
      if (cfg.bindList) {
        const list = Array.isArray(api.state[cfg.bindList]) ? api.state[cfg.bindList] : [];
        if (!list.includes(value)) api.set(cfg.bindList, [...list, value]);
      }
      say(L("Read {value}", { value }));
      view.setAttribute("data-hit", "");
      setTimeout(() => view.removeAttribute("data-hit"), 400);
      if (cfg.sound) beep();
      navigator.vibrate?.(60);
      api.emit("scan", { value, format });
      if (!cfg.continuous) stop();
    }
    function beep() {
      try {
        const a = new AudioContext(), o = a.createOscillator(), g = a.createGain();
        o.frequency.value = 1200;
        g.gain.value = 0.08;
        o.connect(g).connect(a.destination);
        o.start();
        o.stop(a.currentTime + 0.09);
        o.onended = () => a.close();
      } catch { /* no sound */ }
    }
    async function go() {
      if (!("BarcodeDetector" in window)) return say(L("This browser can't read codes with the camera. Type the code, or use a handheld scanner.")), input?.focus();
      try {
        const can = await BarcodeDetector.getSupportedFormats();
        const want = Array.isArray(cfg.formats) ? cfg.formats.filter((f) => can.includes(f)) : can;
        detector = new BarcodeDetector({ formats: want.length ? want : can });
        stream = await navigator.mediaDevices.getUserMedia({ video: { facingMode: facing }, audio: false });
      } catch (e) {
        return say(e.name === "NotAllowedError" ? L("The camera isn't allowed on this page.") : L("The camera couldn't be started."));
      }
      video.srcObject = stream;
      await video.play();
      view.hidden = false;
      start.hidden = true;
      say(L("Hold the code in the frame."));
      let busy = false;
      const look = async () => {
        if (!stream || !el.isConnected) return stop();
        if (!busy && video.readyState >= 2) {
          busy = true;
          try {
            const [c] = await detector.detect(video);
            if (c) got(c.rawValue, c.format);
          } catch { /* the next frame may do */ }
          busy = false;
        }
        frame = setTimeout(look, 120);
      };
      look();
    }
    function stop() {
      clearTimeout(frame);
      stream?.getTracks().forEach((t) => t.stop());
      stream = null;
      view.hidden = true;
      start.hidden = false;
    }
    start.addEventListener("click", go);
    $("[data-scan-stop]").addEventListener("click", () => (stop(), say(""), start.focus()));
    $("[data-scan-flip]").addEventListener("click", () => {
      facing = facing === "environment" ? "user" : "environment";
      stop();
      go();
    });
    input?.addEventListener("keydown", (e) => {
      if (e.key !== "Enter" || !input.value.trim()) return;
      e.preventDefault();
      last = null;
      got(input.value.trim(), "typed");
      input.value = "";
    });
    return {};
  };
})();
