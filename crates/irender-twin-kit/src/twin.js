// ui-twin: a 3D world drawn with three.js, every part of it from the document: models (glTF,
// OBJ, FBX, STL, PLY, USD and others), objects placed and moved, paths things follow, zones,
// walls, the ground, the light and the camera's views. Machine and sensor values come from
// state or a feed of their own (`live`), are shown on the objects, and turn them warning,
// alarm or a colour by value. Tags, popups and buttons are the kit's own elements.
(() => {
  const L = (text, vars) => (window.__irender?.label ? window.__irender.label(text, vars) : text);
  const isObj = (v) => v !== null && typeof v === "object" && !Array.isArray(v);
  const num = (v) => (v === null || v === undefined || v === "" || typeof v === "boolean" ? NaN : Number(v));
  const fin = (v) => Number.isFinite(num(v));
  const str = (v) => (v === null || v === undefined ? "" : String(v));
  const same = (a, b) => a === b || JSON.stringify(a) === JSON.stringify(b);
  // One query, read each time: asked for each object each frame, a new one would cost.
  const reduced = matchMedia("(prefers-reduced-motion: reduce)");
  const calm = () => reduced.matches;
  const deg = (d) => (num(d) * Math.PI) / 180;
  const round = (n, p = 3) => Math.round(n * 10 ** p) / 10 ** p;
  const TONES = ["neutral", "info", "success", "warning", "danger", "primary"];
  const EFFECTS = ["pulse", "glow", "blink", "spin", "bob"];
  const UNITS = { m: 1, cm: 0.01, mm: 0.001, in: 0.0254, ft: 0.3048 };
  const CLOSE = '<svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true"><path d="M4 4l8 8M12 4l-8 8"/></svg>';
  // Each model format and the three.js loader that opens it.
  const LOADERS = {
    glb: ["loaders/GLTFLoader.js", "GLTFLoader"],
    gltf: ["loaders/GLTFLoader.js", "GLTFLoader"],
    obj: ["loaders/OBJLoader.js", "OBJLoader"],
    fbx: ["loaders/FBXLoader.js", "FBXLoader"],
    stl: ["loaders/STLLoader.js", "STLLoader"],
    ply: ["loaders/PLYLoader.js", "PLYLoader"],
    dae: ["loaders/ColladaLoader.js", "ColladaLoader"],
    "3mf": ["loaders/3MFLoader.js", "ThreeMFLoader"],
    "3ds": ["loaders/TDSLoader.js", "TDSLoader"],
    wrl: ["loaders/VRMLLoader.js", "VRMLLoader"],
    usd: ["loaders/USDLoader.js", "USDLoader"],
    usda: ["loaders/USDLoader.js", "USDLoader"],
    usdc: ["loaders/USDLoader.js", "USDLoader"],
    usdz: ["loaders/USDLoader.js", "USDLoader"],
  };
  // A model's format from its address: its extension, or a `data:` address's type
  // (`model/gltf-binary`, `model/obj`, `model/vnd.usdz+zip`…).
  const MIMES = { "gltf-binary": "glb", "gltf+json": "gltf", "usdz+zip": "usdz", "vnd.usdz+zip": "usdz", "vnd.collada+xml": "dae", "vrml": "wrl" };
  const extOf = (u) => {
    const d = /^\s*data:[a-z]+\/([^;,]+)/i.exec(str(u));
    if (d) return MIMES[d[1].toLowerCase()] ?? d[1].toLowerCase().replace(/^(x-|vnd\.)/, "");
    return (str(u).split(/[?#]/)[0].match(/\.([a-z0-9]+)$/i)?.[1] ?? "").toLowerCase();
  };

  // ------------------------------------------------------------------ values
  // `[x, z]` (on the ground), `[x, y, z]` or `{ x, y, z }`, in metres; y is up.
  const vec = (v) => {
    if (Array.isArray(v) && v.length === 2 && fin(v[0]) && fin(v[1])) return { x: num(v[0]), y: 0, z: num(v[1]) };
    if (Array.isArray(v) && v.length >= 3 && [0, 1, 2].every((i) => fin(v[i]))) return { x: num(v[0]), y: num(v[1]), z: num(v[2]) };
    if (isObj(v) && (fin(v.x) || fin(v.z))) return { x: num(v.x) || 0, y: num(v.y) || 0, z: num(v.z) || 0 };
    return null;
  };
  // Degrees: a number turns about the vertical, `[x, y, z]` about each axis.
  const turn = (v) => {
    if (fin(v)) return { x: 0, y: deg(v), z: 0 };
    const t = Array.isArray(v) ? { x: v[0], y: v[1], z: v[2] } : isObj(v) ? v : {};
    return { x: deg(t.x || 0), y: deg(t.y || 0), z: deg(t.z || 0) };
  };
  const scaleOf = (v) => (fin(v) ? { x: num(v), y: num(v), z: num(v) } : Array.isArray(v) && v.length === 3 && v.every(fin) ? { x: num(v[0]), y: num(v[1]), z: num(v[2]) } : { x: 1, y: 1, z: 1 });
  const field = (r, f) => String(f).split(".").reduce((x, k) => (x !== null && typeof x === "object" ? x[k] : undefined), r);

  // ---- `filter`: the data table's conditions, so a `ui-filters` panel narrows the world too
  const blank = (v) => v === null || v === undefined || v === "";
  const isNum = (v) => typeof v === "number" || (typeof v === "string" && v.trim() !== "" && !isNaN(v));
  const fold = (s) => String(s ?? "").normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
  const equal = (a, b) => a === b || (!blank(a) && !blank(b) && typeof a !== "object" && typeof b !== "object" && String(a) === String(b));
  const cmp = (a, b) => (isNum(a) && isNum(b) ? Number(a) - Number(b) : String(a ?? "").localeCompare(String(b ?? ""), undefined, { numeric: true, sensitivity: "base" }));
  const OPS = { from: (v, b) => cmp(v, b) >= 0, to: (v, b) => cmp(v, b) <= 0, gt: (v, b) => cmp(v, b) > 0, gte: (v, b) => cmp(v, b) >= 0, lt: (v, b) => cmp(v, b) < 0, lte: (v, b) => cmp(v, b) <= 0, contains: (v, b) => fold(v).includes(fold(b)) };
  function matches(v, c, name) {
    if (blank(c)) return true;
    if (Array.isArray(v)) return !v.length ? matches(null, c, name) || (Array.isArray(c) && !c.length) : v.some((x) => matches(x, c, name));
    if (Array.isArray(c)) return !c.length || c.some((x) => equal(v, isObj(x) ? x[name] : x));
    if (!isObj(c)) return equal(v, c);
    if (Object.hasOwn(c, "not")) return blank(c.not) || !matches(v, c.not, name);
    if (Object.hasOwn(c, "empty")) return blank(c.empty) || (c.empty ? blank(v) : !blank(v));
    const ops = Object.keys(c).filter((k) => Object.hasOwn(OPS, k));
    if (!ops.length) return equal(v, c[name]);
    return ops.every((k) => blank(c[k]) || (!blank(v) && OPS[k](v, c[k])));
  }
  const passes = (row, f) => !isObj(f) || Object.entries(f).every(([name, c]) => matches(field(row, name), c, name));

  // ------------------------------------------------------------------ colours
  const ink = (c) => {
    const x = (ink.c ||= Object.assign(document.createElement("canvas"), { width: 1, height: 1 }).getContext("2d", { willReadFrequently: true }));
    x.clearRect(0, 0, 1, 1);
    x.fillStyle = "#000";
    x.fillStyle = c;
    x.fillRect(0, 0, 1, 1);
    const [r, g, b, a] = x.getImageData(0, 0, 1, 1).data;
    return { r, g, b, a, css: `rgb(${r}, ${g}, ${b})`, dark: 0.299 * r + 0.587 * g + 0.114 * b < 128 };
  };
  // Between colours by where a value is from `min` to `max`.
  const ramp = (colors, t) => {
    const cs = colors.map(ink), at = Math.min(1, Math.max(0, t)) * (cs.length - 1), i = Math.min(cs.length - 2, Math.floor(at)), f = at - i;
    if (cs.length === 1) return cs[0].css;
    const mix = (k) => Math.round(cs[i][k] + (cs[i + 1][k] - cs[i][k]) * f);
    return `rgb(${mix("r")}, ${mix("g")}, ${mix("b")})`;
  };
  const twins = new Set();
  const retheme = () => twins.forEach((ref) => (ref.deref() ? ref.deref().themed() : twins.delete(ref)));
  let watching = false;
  const watch = () => {
    if (watching) return;
    watching = true;
    matchMedia("(prefers-color-scheme: dark)").addEventListener("change", retheme);
    addEventListener("irender:theme-option", retheme);
    const o = new MutationObserver(retheme);
    for (const n of [document.documentElement, document.body]) o.observe(n, { attributes: true, attributeFilter: ["class", "style", "data-theme", "data-mode"] });
  };

  // The next frame; a timer stands in where frames don't come (a headless browser's virtual
  // time). A view that's hidden or out of sight asks for none. Returns a way to cancel it.
  const nextFrame = (f) => {
    let done = false, timer = 0;
    const go = (t) => {
      if (done) return;
      done = true;
      clearTimeout(timer);
      f(t ?? performance.now());
    };
    const id = requestAnimationFrame(go);
    timer = setTimeout(() => {
      cancelAnimationFrame(id);
      go(performance.now());
    }, 100);
    return () => {
      done = true;
      clearTimeout(timer);
      cancelAnimationFrame(id);
    };
  };

  // ------------------------------------------------------------------ loading
  const mods = {};
  const lib = (url) => (mods[url] ||= import(url).catch((e) => {
    delete mods[url];
    throw new Error(`couldn't load ${url}: ${e.message}`);
  }));

  (window.__irenderBehaviors ||= {})["ui-twin"] = (el, api) => {
    const cfg = api.config, src = cfg.source;
    const host = el.querySelector("[data-twin-canvas]"), note = el.querySelector("[data-twin-note]"), live = el.querySelector("[data-twin-live]");
    const tagLayer = el.querySelector("[data-twin-tags]"), list = el.querySelector("[data-twin-list]"), viewsBox = el.querySelector("[data-twin-views]"), paletteBox = el.querySelector("[data-twin-palette]");
    const proto = el.querySelector("[data-twin-proto]");
    const addon = (path) => lib(src.addons.replace("{name}", path));
    const env = () => ({ ...api.state, ...(api.scope?.() || {}) });
    const resolve = (v) => {
      if (Array.isArray(v)) return v.map(resolve);
      if (!isObj(v)) return v;
      if ("$irender" in v && Object.keys(v).length === 1) return window.__irender.evaluate(v.$irender, env());
      return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, resolve(x)]));
    };
    const WORLD = ["models", "objects", "paths", "zones", "walls", "ground", "environment", "camera", "views", "readouts", "colorBy"];
    const LIVE = [...WORLD, "world", "src", "live", "follow", "filter", "toneBy", "alarmBy", "effectBy", "labels", "editable", "snap", "palette", "lod", "multiple", "measure"];
    const props = {}, fixed = cfg.props;
    let fetched = null; // the world from `src`
    const read = () => {
      for (const k of LIVE) props[k] = k in fixed ? resolve(fixed[k]) : undefined;
    };
    // A part of the world: its own prop, else `world`'s, else `src`'s.
    const W = (k) => props[k] ?? (isObj(props.world) ? props.world[k] : undefined) ?? (isObj(fetched) ? fetched[k] : undefined);
    read();
    const set = (key, value) => cfg[key] && !same(api.state[cfg[key]], value) && api.set(cfg[key], value);
    const make = (tag, attrs = {}, text) => {
      const e = document.createElement(tag);
      for (const [k, v] of Object.entries(attrs)) if (v !== null && v !== undefined && v !== false) e.setAttribute(k, v === true ? "" : v);
      if (text !== undefined) e.textContent = text;
      return e;
    };
    const allowed = (u) => typeof u === "string" && !/^\s*(javascript|vbscript):/i.test(u) && (!cfg.safe || !/^\s*data:(?!model\/|application\/octet-stream|image\/)/i.test(u));
    const lang = document.documentElement.lang || undefined;

    // ---------------------------------------------------------------- saying things
    let noteTimer;
    const say = (text, stay) => {
      note.textContent = text;
      note.toggleAttribute("data-shown", !!text);
      clearTimeout(noteTimer);
      if (text && !stay) noteTimer = setTimeout(() => note.removeAttribute("data-shown"), 1600);
    };

    let T = null, renderer = null, scene = null, camera = null, controls = null, world = null;
    let items = new Map(), recs = [], zones = [], walls = [], paths = new Map(), pathMeshes = [];
    let selected = null, popup = null, ground = null, grid = null, sun = null, hemi = null, ringOf = null;
    // With `multiple`, the objects picked together (the selected one among them); else just it.
    const picked = new Set(), sels = [];
    const multi = () => !!props.multiple;
    const live$ = new Map(); // id → the latest values from `live`
    const colors = {};
    let stopped = false;

    function failed(error) {
      if (el.hasAttribute("data-error")) return;
      console.error("ui-twin:", error);
      stopped = true;
      el.removeAttribute("data-ready");
      el.setAttribute("data-error", "");
      say("");
      host.replaceChildren();
      tagLayer.replaceChildren();
      const box = make("div", { "data-twin-failed": "", role: "alert" });
      box.append(make("strong", {}, L("3D view unavailable")), make("div", {}, error.message));
      // What's in the world is still worth something without it.
      collect();
      if (recs.length) {
        const ul = make("ul");
        for (const r of recs) {
          const li = make("li", { "data-level": r.alarm ? "alarm" : r.level }, r.title || r.id);
          for (const x of r.reads) li.append(make("span", {}, ` · ${x.label ? `${x.label} ` : ""}${x.text}`));
          ul.append(li);
        }
        box.append(ul);
      }
      el.append(box);
      api.emit("failed", { error: error.message });
    }

    // ---------------------------------------------------------------- what's in it
    const toned = (m) => {
      if (TONES.includes(m.tone)) return m.tone;
      const t = isObj(props.toneBy) ? props.toneBy.tones?.[str(field(m, props.toneBy.field))] : null;
      return TONES.includes(t) ? t : null;
    };
    const alarmed = (m) => {
      if (typeof m.alarm === "boolean") return m.alarm;
      const by = props.alarmBy;
      return isObj(by) && [].concat(by.values ?? []).some((v) => str(v) === str(field(m, by.field)));
    };
    // Readouts: `{ field, label, unit, decimals, warn, alarm, below, tag, models }`.
    // Formatters by decimals, made once: a live world formats thousands of values a second.
    const formats = new Map();
    const numbers = (d, fixed) => {
      const k = `${d} ${fixed}`;
      if (!formats.has(k)) formats.set(k, new Intl.NumberFormat(lang, { maximumFractionDigits: d, minimumFractionDigits: fixed ? d : 0 }));
      return formats.get(k);
    };
    const fmt = (v, o) => {
      if (typeof v === "boolean") return v ? L("On") : L("Off");
      if (!isNum(v)) return str(v);
      const d = fin(o.decimals) ? num(o.decimals) : Math.abs(Number(v)) >= 100 ? 0 : 1;
      const n = numbers(d, fin(o.decimals)).format(Number(v));
      return o.unit ? (/^[%°]/.test(o.unit) ? `${n}${o.unit}` : `${n} ${o.unit}`) : n;
    };
    const levelOf = (o, v) => {
      if (!isNum(v)) return null;
      const n = Number(v), over = (t) => fin(t) && (o.below ? n <= num(t) : n >= num(t));
      return over(o.alarm) ? "alarm" : over(o.warn) ? "warn" : null;
    };
    // A readout's last values per object (`history: n`): a point each time its value changes.
    const hist = new Map();
    function readings(m, id) {
      const rs = Array.isArray(W("readouts")) ? W("readouts") : [];
      const out = [];
      for (const o of rs) {
        if (!isObj(o) || !o.field) continue;
        if (o.models && ![].concat(o.models).map(str).includes(str(m.model))) continue;
        if (isObj(o.when) && !passes(m, o.when)) continue;
        const v = field(m, o.field);
        if (v === undefined || v === null || v === "") continue;
        let past = null;
        const n = fin(o.history) ? Math.min(500, Math.floor(num(o.history))) : 0;
        if (n > 1 && isNum(v)) {
          const k = `${id}\u0000${o.field}`;
          past = hist.get(k) ?? [];
          if (past.at(-1) !== Number(v)) past = [...past, Number(v)].slice(-n);
          hist.set(k, past);
        }
        // The object's first two are on its tag, unless `tag` says otherwise.
        out.push({ field: o.field, label: str(o.label ?? o.field), text: fmt(v, o), level: levelOf(o, v), tag: o.tag ?? out.filter((x) => x.tag).length < 2, past, o });
      }
      return out;
    }
    const heat = (m) => {
      const c = W("colorBy");
      if (!isObj(c) || !c.field) return null;
      const v = num(field(m, c.field));
      if (!Number.isFinite(v)) return null;
      const lo = fin(c.min) ? num(c.min) : 0, hi = fin(c.max) ? num(c.max) : 100;
      return ramp(Array.isArray(c.colors) && c.colors.length ? c.colors : ["#2563eb", "#16a34a", "#eab308", "#dc2626"], hi === lo ? 0 : (v - lo) / (hi - lo));
    };
    function collect() {
      const seen = new Set();
      const objects = Array.isArray(W("objects")) ? W("objects") : [];
      recs = objects.flatMap((m0, i) => {
        if (!isObj(m0)) return [];
        const id = str(m0.id ?? i);
        if (seen.has(id)) return [];
        const m = live$.has(id) ? { ...m0, ...live$.get(id) } : m0;
        if (m.show === false || !passes(m, props.filter)) return [];
        seen.add(id);
        const reads = readings(m, id);
        const worst = reads.some((x) => x.level === "alarm") ? "alarm" : reads.some((x) => x.level === "warn") ? "warn" : null;
        const alarm = typeof m.alarm === "boolean" ? m.alarm : alarmed(m) || worst === "alarm";
        const by = props.effectBy;
        const effect = m.effect ?? (isObj(by) ? by.effects?.[str(field(m, by.field))] : null);
        // Its own tone, else what its values say, else its status's.
        const tone = TONES.includes(m.tone) ? m.tone : alarm ? "danger" : worst === "warn" ? "warning" : toned(m);
        const pos = vec(m.position) ?? vec({ x: m.x, y: m.y, z: m.z }) ?? { x: 0, y: 0, z: 0 };
        return [{
          id, row: m, title: str(m.title ?? m.name ?? ""), description: m.description, label: m.label,
          model: m.model, shape: m.shape ?? (m.model ? null : "box"), size: m.size, color: m.color, scale: scaleOf(m.scale),
          pos, rot: turn(m.rotation ?? m.heading ?? 0), tone, alarm, level: worst, effect: EFFECTS.includes(effect) ? effect : null,
          heat: heat(m), reads, path: m.path === undefined ? null : str(m.path), speed: fin(m.speed) ? num(m.speed) : 1, loop: m.loop !== false,
          progress: fin(m.progress) ? num(m.progress) : null, attach: m.attach === undefined || m.attach === null ? null : str(m.attach),
          opacity: fin(m.opacity) ? num(m.opacity) : 1, clickable: m.clickable !== false, animation: m.animation, href: m.href,
        }];
      });
      // What's gone from the world forgets its values; what's only filtered out keeps them.
      if (hist.size) {
        const ids = new Set(objects.map((m, i) => (isObj(m) ? str(m.id ?? i) : null)));
        for (const k of hist.keys()) if (!ids.has(k.slice(0, k.indexOf("\u0000")))) hist.delete(k);
      }
    }
    // A readout's last values as a line, its latest a dot; the label says their range.
    function spark(x) {
      const p = x.past, w = 64, h = 18, lo = Math.min(...p), hi = Math.max(...p);
      const at = (v, i) => [(i / (p.length - 1)) * (w - 4) + 2, hi === lo ? h / 2 : h - 2 - ((v - lo) / (hi - lo)) * (h - 4)].map((n) => +n.toFixed(1));
      const pts = p.map(at), [cx, cy] = pts.at(-1);
      const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
      svg.setAttribute("viewBox", `0 0 ${w} ${h}`);
      svg.setAttribute("width", w);
      svg.setAttribute("height", h);
      svg.setAttribute("role", "img");
      svg.setAttribute("aria-label", L("Last {n}: {low} to {high}", { n: p.length, low: fmt(lo, x.o), high: fmt(hi, x.o) }));
      svg.setAttribute("data-twin-spark", "");
      svg.innerHTML = `<polyline points="${pts.join(" ")}" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round" stroke-linecap="round"/><circle cx="${cx}" cy="${cy}" r="2" fill="currentColor"/>`;
      return svg;
    }

    // ---------------------------------------------------------------- models
    const templates = new Map();
    let gltfLoader = null;
    async function gltf() {
      if (gltfLoader) return gltfLoader;
      const [{ GLTFLoader }, { MeshoptDecoder }, { DRACOLoader }, { KTX2Loader }] = await Promise.all([
        addon("loaders/GLTFLoader.js"), addon("libs/meshopt_decoder.module.js"), addon("loaders/DRACOLoader.js"), addon("loaders/KTX2Loader.js"),
      ]);
      const l = new GLTFLoader().setMeshoptDecoder(MeshoptDecoder);
      l.setDRACOLoader(new DRACOLoader().setDecoderPath(src.draco));
      l.setKTX2Loader(new KTX2Loader().setTranscoderPath(src.basis).detectSupport(renderer));
      return (gltfLoader = l);
    }
    // A model as the document names it: `models: { name: url | { url, … } }`, or a url.
    const specOf = (name) => {
      const lib = W("models");
      const m = isObj(lib) ? lib[str(name)] : undefined;
      if (typeof m === "string") return { url: m };
      if (isObj(m) && typeof m.url === "string") return m;
      return typeof name === "string" && /[./]/.test(name) ? { url: name } : null;
    };
    async function open(spec) {
      const url = spec.url, kind = str(spec.format || extOf(url)).toLowerCase(), how = LOADERS[kind];
      if (!allowed(url)) throw new Error(`model address not allowed: ${url}`);
      if (!how) throw new Error(`can't open .${kind || "?"} models (${url})`);
      if (kind === "glb" || kind === "gltf") {
        const g = await (await gltf()).loadAsync(url);
        return { obj: g.scene, clips: g.animations ?? [] };
      }
      const mod = await addon(how[0]);
      const loader = new mod[how[1]]();
      if (kind === "obj" && spec.mtl) {
        const { MTLLoader } = await addon("loaders/MTLLoader.js");
        const mats = await new MTLLoader().loadAsync(spec.mtl);
        mats.preload();
        loader.setMaterials(mats);
      }
      const got = await loader.loadAsync(url);
      if (got?.isBufferGeometry) {
        if (!got.hasAttribute("normal")) got.computeVertexNormals();
        const mat = new T.MeshStandardMaterial({ color: spec.color ?? "#a1a1aa", roughness: 0.55, metalness: 0.1, vertexColors: got.hasAttribute("color") });
        return { obj: new T.Mesh(got, mat), clips: [] };
      }
      const obj = got?.scene ?? got;
      return { obj, clips: obj.animations ?? got?.animations ?? [] };
    }
    // Loaded once, made to stand on the ground at its middle, in metres, y up.
    const opened = new Set(); // the models loaded so far, by key
    function template(spec) {
      const key = JSON.stringify(spec);
      if (!templates.has(key)) {
        templates.set(key, open(spec).then(({ obj, clips }) => {
          const inner = new T.Group();
          inner.add(obj);
          if (str(spec.up).toLowerCase() === "z") obj.rotation.x = -Math.PI / 2;
          const s = scaleOf(spec.scale ?? 1), u = UNITS[spec.units] ?? 1;
          inner.scale.set(s.x * u, s.y * u, s.z * u);
          inner.rotation.y = deg(spec.rotate || 0);
          inner.updateMatrixWorld(true);
          let box = new T.Box3().setFromObject(inner);
          const size = box.getSize(new T.Vector3());
          if (spec.fit !== undefined && size.x + size.y + size.z > 0) {
            const f = fin(spec.fit) ? num(spec.fit) / Math.max(size.x, size.y, size.z) : Array.isArray(spec.fit) ? Math.min(...[size.x, size.y, size.z].map((d, i) => (fin(spec.fit[i]) && d > 0 ? num(spec.fit[i]) / d : Infinity))) : 1;
            if (Number.isFinite(f) && f > 0) inner.scale.multiplyScalar(f);
            inner.updateMatrixWorld(true);
            box = new T.Box3().setFromObject(inner);
          }
          const c = box.getCenter(new T.Vector3());
          const origin = spec.origin ?? "base";
          if (origin !== "none") inner.position.set(-c.x, origin === "center" ? -c.y : -box.min.y, -c.z);
          const outer = new T.Group();
          outer.add(inner);
          let skinned = false;
          outer.traverse((o) => {
            if (o.isMesh) {
              o.castShadow = true;
              o.receiveShadow = true;
            }
            if (o.isSkinnedMesh) skinned = true;
          });
          outer.updateMatrixWorld(true);
          return { scene: outer, clips, skinned, size: new T.Box3().setFromObject(outer).getSize(new T.Vector3()) };
        }).then(async (t) => {
          await warm(t);
          opened.add(key);
          return t;
        }));
      }
      return templates.get(key);
    }
    // A model's GPU work before any object shows it, without holding the page: its shaders
    // compile in parallel (as a copy and as instances), and its textures go up a few at a
    // time. Done all at once in the first frame that drew it, it could stop the page.
    async function warm(t) {
      if (!renderer) return;
      const probe = new T.Group();
      probe.add(t.scene.clone(true));
      t.scene.traverse((o) => {
        if (o.isMesh && !o.isSkinnedMesh) {
          const im = new T.InstancedMesh(o.geometry, o.material, 1);
          im.castShadow = im.receiveShadow = true;
          probe.add(im);
        }
      });
      try {
        if (renderer.compileAsync) await renderer.compileAsync(probe, camera, scene);
        const textures = new Set();
        t.scene.traverse((o) => {
          for (const m of [].concat(o.material ?? [])) for (const v of Object.values(m)) if (v?.isTexture) textures.add(v);
        });
        let start = performance.now();
        for (const tex of textures) {
          renderer.initTexture(tex);
          if (performance.now() - start > 8) {
            await new Promise((r) => nextFrame(r));
            start = performance.now();
          }
        }
      } catch (e) {
        console.warn("ui-twin:", e);
      }
      probe.traverse((o) => o.isInstancedMesh && o.dispose());
    }
    let SkeletonUtils = null;
    const copy = async (t) => {
      if (!t.skinned) return t.scene.clone(true);
      SkeletonUtils ||= await addon("utils/SkeletonUtils.js");
      return SkeletonUtils.clone(t.scene);
    };
    // A shape, standing on the ground: box, cylinder, sphere, cone, sensor or none.
    // A shape's geometry and height, standing on the ground; none for `none` or `point`.
    function shapeGeometry(r) {
      const sz = Array.isArray(r.size) ? r.size.map(num) : fin(r.size) ? [num(r.size)] : [];
      if (r.shape === "cylinder" || r.shape === "cone") {
        const d = sz[0] || 1, h = sz[1] || 1;
        return { geo: r.shape === "cone" ? new T.ConeGeometry(d / 2, h, 32) : new T.CylinderGeometry(d / 2, d / 2, h, 32), h };
      }
      if (r.shape === "sphere" || r.shape === "sensor") {
        const d = sz[0] || (r.shape === "sensor" ? 0.35 : 1);
        return { geo: new T.SphereGeometry(d / 2, 24, 16), h: d };
      }
      if (r.shape === "none" || r.shape === "point") return null;
      const [w = 1, h = 1, d = w] = sz;
      return { geo: new T.BoxGeometry(w, h, d), h };
    }
    function shapeMesh(r) {
      const color = r.color && CSS.supports("color", r.color) ? ink(r.color).css : r.tone ? colors[r.tone] : colors.shape;
      const mat = new T.MeshStandardMaterial({ color, roughness: 0.6, metalness: 0.05 });
      const s = shapeGeometry(r);
      if (!s) return { obj: new T.Group(), size: new T.Vector3(0.2, 0.2, 0.2) };
      const { geo, h } = s;
      if (r.shape === "sensor") mat.emissiveIntensity = 0.6;
      const mesh = new T.Mesh(geo, mat);
      mesh.position.y = h / 2;
      mesh.castShadow = mesh.receiveShadow = true;
      mesh.userData.own = true;
      const g = new T.Group();
      g.add(mesh);
      return { obj: g, size: new T.Box3().setFromObject(g).getSize(new T.Vector3()) };
    }

    // ---------------------------------------------------------------- drawing things
    // A model whose objects look alike is drawn once per part for all of them (instancing):
    // a few draw calls however many there are. One that needs a look of its own (a glow, an
    // see-through, its own animation) is a copy of its own while it does, so nothing looks
    // different. Those in alarm are instances of their own, glowing together; those coloured
    // by value too, each glowing in its own colour. Shapes are instances as well, each in
    // its own colour, but one in alarm is a copy.
    const ownLook = (r) => !!(r.opacity < 1 || r.animation || (r.effect && r.effect !== "spin" && r.effect !== "bob" && r.effect !== "pulse"));
    const lookOf = (r) => (ownLook(r) || (r.alarm && !r.model) ? "own" : r.alarm ? "alarm" : r.heat && r.model ? "heat" : null);
    const bodyKey = (r) => JSON.stringify([r.model ? specOf(r.model) : null, r.shape, r.size, r.model ? null : r.color, r.model ? null : r.tone, lookOf(r)]);
    const batches = new Map(); // a model's key → its instanced parts and the objects drawn with them
    // A model's meshes as a batch's parts, each where it is in the model.
    const partsOf = (scene, lit) => {
      const parts = [];
      scene.updateMatrixWorld(true);
      scene.traverse((o) => {
        if (o.isMesh && !o.isSkinnedMesh) parts.push({ geometry: o.geometry, material: Array.isArray(o.material) ? o.material.map(lit) : lit(o.material), local: o.matrixWorld.clone(), im: null });
      });
      return parts;
    };
    function batchFor(key, t, look, spec) {
      let b = batches.get(key);
      if (b) return b;
      const glowing = [];
      // In alarm: the same parts in materials of their own, which glow together. Coloured
      // by value: in materials that glow in each instance's colour.
      const lit = (m) => {
        if (look === "heat") return glowMaterial(m);
        if (look !== "alarm") return m;
        const c = m.clone();
        if (c.emissive) glowing.push(c.emissive.set(colors.danger) && c);
        return c;
      };
      b = { parts: partsOf(t.scene, lit), slots: [], cap: 0, glowing, t, spec, model: JSON.stringify(spec), lit, far: null, farParts: null, farAt: Infinity };
      if (look === "heat") b.tint = (it) => [null, it.rec.heat, 0.55];
      batches.set(key, b);
      return b;
    }
    // A material that glows in each instance's own colour (`glow`, one per instance), not
    // its emissive's. It has no emissive map of its own then, as one glowing by value hasn't.
    function glowMaterial(m) {
      const c = m.clone();
      c.onBeforeCompile = (sh) => {
        if (!sh.fragmentShader.includes("#include <emissivemap_fragment>") || !sh.vertexShader.includes("#include <begin_vertex>")) {
          console.warn("ui-twin: this three.js's shaders can't glow by instance; colours by value won't show");
          return;
        }
        sh.vertexShader = sh.vertexShader
          .replace("#include <common>", "#include <common>\nattribute vec3 glow;\nvarying vec3 vGlow;")
          .replace("#include <begin_vertex>", "#include <begin_vertex>\nvGlow = glow;");
        sh.fragmentShader = sh.fragmentShader
          .replace("#include <common>", "#include <common>\nvarying vec3 vGlow;")
          .replace("#include <emissivemap_fragment>", "#include <emissivemap_fragment>\ntotalEmissiveRadiance = vGlow;");
      };
      c.customProgramCacheKey = () => "ui-twin-glow";
      return c;
    }
    // A glowing batch's shaders, compiled in the background (as a model's are, see warm)
    // before it's drawn: compiled in the frame that first drew it, they could stop the page.
    async function warmBatch(b) {
      b.warming = true;
      loading++;
      progress();
      const probe = new T.Group();
      for (const p of b.parts) {
        const g = new T.BufferGeometry();
        g.setIndex(p.geometry.index);
        for (const [k, a] of Object.entries(p.geometry.attributes)) if (k !== "glow") g.setAttribute(k, a);
        g.setAttribute("glow", new T.InstancedBufferAttribute(new Float32Array(3), 3));
        const im = new T.InstancedMesh(g, p.material, 1);
        im.castShadow = im.receiveShadow = true;
        // Shapes have a colour each too, which is another program.
        if (!b.spec) im.setColorAt(0, new T.Color(1, 1, 1));
        probe.add(im);
      }
      try {
        if (renderer.compileAsync) await renderer.compileAsync(probe, camera, scene);
      } catch (e) {
        console.warn("ui-twin:", e);
      }
      probe.traverse((o) => o.isInstancedMesh && o.dispose());
      b.ready = true;
      loaded++;
      progress();
      wake();
    }
    // Shapes of one kind and size, drawn together: each in its own colour (`color`, its
    // tone's, or the kit's), a sensor glowing in its value's colour or its tone's.
    function shapeBatch(r) {
      const key = `shape ${JSON.stringify([r.shape, r.size])}`;
      let b = batches.get(key);
      if (!b) {
        const s = shapeGeometry(r);
        if (!s) return null;
        const mat = glowMaterial(new T.MeshStandardMaterial({ color: "#ffffff", roughness: 0.6, metalness: 0.05 }));
        b = { parts: [{ geometry: s.geo, material: mat, local: new T.Matrix4().makeTranslation(0, s.h / 2, 0), im: null }], slots: [], cap: 0, glowing: [], spec: null, far: null, farParts: null, farAt: Infinity };
        b.size = (s.geo.boundingBox || (s.geo.computeBoundingBox(), s.geo.boundingBox)).getSize(new T.Vector3());
        b.tint = (it) => {
          const x = it.rec, base = x.color && CSS.supports("color", x.color) ? ink(x.color).css : x.tone ? colors[x.tone] : colors.shape;
          if (x.shape === "sensor") return [x.heat ?? base, x.heat ?? (x.tone ? colors[x.tone] : colors.info), x.heat ? 0.55 : 0.6];
          return [base, x.heat, 0.55];
        };
        batches.set(key, b);
      }
      return { obj: new T.Group(), size: b.size.clone(), batch: b };
    }
    // Room for every object of a batch: each part is an InstancedMesh, made bigger by doubling.
    // The far parts, once they're ready, get theirs then.
    function roomFor(b) {
      const grow = b.slots.length > b.cap;
      if (grow) b.cap = Math.max(8, b.cap * 2, b.slots.length);
      for (const [list, far] of [[b.parts, false], [b.farParts ?? [], true]]) for (const part of list) {
        if (part.im && !grow) continue;
        if (part.im) {
          world.remove(part.im);
          part.im.dispose();
        }
        // A colour for each instance to glow in, on a geometry of the part's own (its
        // vertices shared): the same model's other batches have room for other numbers.
        if (b.tint) {
          if (!part.ownGeometry) {
            const g = new T.BufferGeometry();
            g.setIndex(part.geometry.index);
            for (const [k, a] of Object.entries(part.geometry.attributes)) g.setAttribute(k, a);
            for (const x of part.geometry.groups) g.addGroup(x.start, x.count, x.materialIndex);
            g.boundingSphere = part.geometry.boundingSphere;
            g.boundingBox = part.geometry.boundingBox;
            part.geometry = part.ownGeometry = g;
          }
          part.glow = new T.InstancedBufferAttribute(new Float32Array(b.cap * 3), 3);
          part.glow.setUsage(T.DynamicDrawUsage);
          part.geometry.setAttribute("glow", part.glow);
        }
        const im = new T.InstancedMesh(part.geometry, part.material, b.cap);
        im.instanceMatrix.setUsage(T.DynamicDrawUsage);
        im.castShadow = im.receiveShadow = true;
        // Its instances move, so its bounds would be out of date for culling; they're
        // worked out again for picking.
        im.frustumCulled = false;
        im.userData.batch = b;
        im.userData.far = far;
        part.im = im;
        world.add(im);
      }
    }

    // ---------------------------------------------------------------- far away
    // A model far from the camera is drawn simpler: a model of its own (`far: url`), or
    // made in the browser with meshoptimizer's simplifier (`far: { ratio }`, or `lod` for
    // every model used by many objects). Only objects drawn together have it.
    const LOD_MANY = 100;
    function farOf(b) {
      if (!b.spec) return null;
      const f = b.spec.far, lod = props.lod ?? "auto";
      if (f === false || lod === false || lod === "none") return null;
      if (typeof f === "string") return { url: f };
      if (isObj(f)) return f;
      // Its objects in every batch of the model (in alarm or not).
      let many = 0;
      for (const o of batches.values()) if (o.model === b.model) many += o.slots.length;
      return lod === true || isObj(lod) || many >= LOD_MANY ? {} : null;
    }
    let simplifier = null, geoUtils = null;
    const simplified = new WeakMap();
    async function farFor(b, f) {
      b.far = "making";
      try {
        const lod = isObj(props.lod) ? props.lod : {}, size = b.t.size.length();
        let parts;
        if (f.url) parts = partsOf((await template({ ...b.spec, url: f.url, format: f.format, far: false })).scene, b.lit);
        else {
          const ratio = Math.min(0.9, Math.max(0.01, num(f.ratio ?? lod.ratio) || 0.15));
          simplifier ||= lib(src.simplifier).then(async (m) => (await m.MeshoptSimplifier.ready, m.MeshoptSimplifier));
          const S = await simplifier;
          parts = [];
          let start = performance.now();
          for (const p of b.parts) {
            const g = await simpler(p, ratio, size * 0.03, S);
            if (g) parts.push({ ...p, geometry: g, im: null });
            // A part at a time, so a big model doesn't hold the page.
            if (performance.now() - start > 8) {
              await new Promise((r) => nextFrame(r));
              start = performance.now();
            }
          }
        }
        const d = num(f.distance ?? lod.distance);
        b.farAt = d > 0 ? d : Math.max(15, size * 12);
        b.farParts = parts;
        b.far = "ready";
        api.emit("lod", { model: b.spec.url, distance: b.farAt });
        wake();
      } catch (e) {
        console.warn("ui-twin: no simpler model:", e.message);
        b.far = "failed";
      }
    }
    // A part with fewer triangles, null when it's too small to see from afar, or the part
    // itself when it can't be made simpler (several materials, or few triangles already).
    async function simpler(p, ratio, tiny, S) {
      const g = p.geometry, key = g.index ?? g.attributes.position;
      if (simplified.has(key)) return simplified.get(key);
      g.boundingSphere || g.computeBoundingSphere();
      let out = g;
      const tris = (g.index ? g.index.count : g.attributes.position.count) / 3;
      if (g.boundingSphere.radius * p.local.getMaxScaleOnAxis() < tiny) out = null;
      else if (g.groups.length <= 1 && tris > 200 && g.attributes.position) {
        let base = g;
        if (!g.index) {
          geoUtils ||= addon("utils/BufferGeometryUtils.js");
          base = (await geoUtils).mergeVertices(g);
        }
        const pos = base.attributes.position, xyz = new Float32Array(pos.count * 3);
        for (let i = 0; i < pos.count; i++) (xyz[i * 3] = pos.getX(i)), (xyz[i * 3 + 1] = pos.getY(i)), (xyz[i * 3 + 2] = pos.getZ(i));
        const idx = Uint32Array.from(base.index.array), want = Math.max(3, Math.floor((idx.length * ratio) / 3) * 3);
        const [kept] = S.simplify(idx, xyz, 3, want, 0.02, ["Prune"]);
        if (!kept.length) out = null;
        else if (kept.length < idx.length * 0.9) {
          out = new T.BufferGeometry();
          for (const [k, a] of Object.entries(base.attributes)) if (k !== "glow") out.setAttribute(k, a);
          out.setIndex(new T.BufferAttribute(kept, 1));
          out.computeBoundingSphere();
        }
      }
      simplified.set(key, out);
      return out;
    }
    const m4 = { a: null };
    // Only the instances in view are drawn (`drawn`, packed first): a big world seen up close
    // draws the few objects near the camera. Each is tested with room to spare, so the
    // shadow of one just out of view still falls into it.
    const frustum = { f: null, m: null, s: null };
    function seer() {
      const f = (frustum.f ||= new T.Frustum()), s = (frustum.s ||= new T.Sphere());
      f.setFromProjectionMatrix((frustum.m ||= new T.Matrix4()).multiplyMatrices(camera.projectionMatrix, camera.matrixWorldInverse));
      return (it) => {
        if (!it.size) return true;
        const k = it.rec.scale, h = it.size.y * k.y;
        it.group.getWorldPosition(s.center).y += h / 2;
        s.radius = Math.hypot(it.size.x * k.x, h, it.size.z * k.z) * 0.75 + 1;
        // How far it is, for which of its batch's parts draw it.
        it.away = s.center.distanceTo(camera.position);
        return f.intersectsSphere(s);
      };
    }
    // Each instance's colour (shapes) and glow: `[colour, glow colour, glow power]`.
    const tint = { c: null };
    function tintAt(part, tints) {
      const c = (tint.c ||= new T.Color());
      tints.forEach(([base, glow, power], i) => {
        if (base) part.im.setColorAt(i, c.set(base));
        if (glow) c.set(glow).multiplyScalar(power);
        else c.setRGB(0, 0, 0);
        part.glow.setXYZ(i, c.r, c.g, c.b);
      });
      part.glow.needsUpdate = true;
      if (part.im.instanceColor) part.im.instanceColor.needsUpdate = true;
    }
    const sameList = (a, b) => a && a.length === b.length && a.every((x, i) => x === b[i]);
    // Each instance where its object is: the object's place, its body's own turn (a spinning
    // fan), then the part's place in the model. `moved`: something but the camera has moved,
    // so they're all placed again; otherwise only a batch whose objects in view changed is.
    function placeInstances(moved) {
      if (!batches.size && !ringBatches.size) return;
      if (moved) world.updateMatrixWorld();
      const seen = seer();
      placeRings(seen, moved);
      const m = (m4.a ||= new T.Matrix4());
      for (const b of batches.values()) {
        // One glowing in each instance's colour shows once its shaders are ready.
        if (b.tint && !b.ready) {
          if (!b.warming) warmBatch(b);
          continue;
        }
        if (b.far === null) {
          const f = farOf(b);
          if (f) farFor(b, f);
        }
        roomFor(b);
        // `lod` may have been turned off since its far parts were made.
        const all = b.slots.filter(seen), farAt = b.farParts && farOf(b) ? b.farAt : Infinity;
        const drawn = all.filter((it) => it.away < farAt), far = all.filter((it) => it.away >= farAt);
        if (!moved && sameList(b.drawn, drawn) && sameList(b.drawnFar, far)) continue;
        b.drawn = drawn;
        b.drawnFar = far;
        for (const [list, parts] of [[drawn, b.parts], [far, b.farParts ?? []]]) {
          const tints = b.tint && list.map(b.tint);
          for (const part of parts) {
            list.forEach((it, i) => part.im.setMatrixAt(i, m.multiplyMatrices(it.body.matrixWorld, part.local)));
            if (tints) tintAt(part, tints);
            part.im.count = list.length;
            part.im.instanceMatrix.needsUpdate = true;
            part.im.boundingSphere = null;
          }
        }
      }
    }
    function unbatch(it) {
      if (!it.batch) return;
      const i = it.batch.slots.indexOf(it);
      if (i >= 0) it.batch.slots.splice(i, 1);
      it.batch = null;
    }
    let loading = 0, loaded = 0;
    let busy = false;
    function progress() {
      if (loading > loaded) {
        busy = true;
        say(L("Loading models… {n} of {total}", { n: loaded, total: loading }), true);
      } else {
        if (busy) say("");
        busy = false;
        if (!el.hasAttribute("data-ready")) {
          el.setAttribute("data-ready", "");
          api.emit("ready", { objects: recs.length });
        }
      }
    }
    async function dress(it) {
      const r = it.rec, key = bodyKey(r);
      if (it.key === key) return;
      it.key = key;
      let made;
      if (r.model) {
        const spec = specOf(r.model);
        // Counted while it loads; one already loaded (an object turning to its own copy for an
        // alarm and back) isn't loading.
        const counted = !(spec && opened.has(JSON.stringify(spec)));
        if (counted) {
          loading++;
          progress();
        }
        try {
          if (!spec) throw new Error(`no model named ${r.model}`);
          const t = await template(spec);
          if (it.key !== key || !items.has(it.id)) return;
          const look = lookOf(r);
          made = !t.skinned && look !== "own"
            ? { obj: new T.Group(), size: t.size.clone(), clips: [], batch: batchFor(JSON.stringify(spec) + (look ? ` ${look}` : ""), t, look, spec) }
            : { obj: await copy(t), size: t.size.clone(), clips: t.clips };
        } catch (e) {
          console.warn("ui-twin:", e.message);
          api.emit("failed", { id: r.id, model: r.model, error: e.message });
          if (it.key !== key) return;
          // Where it would be, so the rest still makes sense.
          const g = new T.Group(), m = new T.Mesh(new T.BoxGeometry(1, 1, 1), new T.MeshBasicMaterial({ color: colors.muted, wireframe: true }));
          m.position.y = 0.5;
          g.add(m);
          made = { obj: g, size: new T.Vector3(1, 1, 1) };
        } finally {
          if (counted) {
            loaded++;
            progress();
          }
        }
      } else made = (lookOf(r) !== "own" && shapeBatch(r)) || shapeMesh(r);
      if (it.body) {
        it.group.remove(it.body);
        dispose(it.body, true);
      }
      unbatch(it);
      it.body = made.obj;
      if (made.batch) {
        it.batch = made.batch;
        made.batch.slots.push(it);
      }
      it.size = made.size;
      it.mats = null;
      it.group.add(it.body);
      reshadow();
      it.mixer?.stopAllAction();
      it.mixer = null;
      it.clips = made.clips ?? [];
      play(it);
      paint(it);
      ringFor(it);
      wake();
    }
    // Plays a model's own animation: `animation: true` (its first) or a clip's name.
    function play(it) {
      const a = it.rec.animation;
      it.mixer?.stopAllAction();
      it.mixer = null;
      if (!a || !it.clips?.length) return;
      const clip = typeof a === "string" ? T.AnimationClip.findByName(it.clips, a) : it.clips[0];
      if (!clip) return;
      it.mixer = new T.AnimationMixer(it.body);
      it.mixer.clipAction(clip).play();
    }
    // Their own materials, once something about them changes (a colour, a glow, see-through).
    function ownMaterials(it) {
      if (it.mats) return it.mats;
      it.mats = [];
      it.body.traverse((o) => {
        if (!o.isMesh) return;
        const one = (m) => {
          const c = o.userData.own ? m : m.clone();
          c.userData.base = { emissive: c.emissive?.clone(), intensity: c.emissiveIntensity, opacity: c.opacity, transparent: c.transparent };
          it.mats.push(c);
          return c;
        };
        o.material = Array.isArray(o.material) ? o.material.map(one) : one(o.material);
      });
      return it.mats;
    }
    // Colour by value, alarm and glow on the materials; see-through.
    function paint(it, t = 0) {
      if (!it.body) return;
      const r = it.rec;
      const still = calm();
      let glow = null, power = 0;
      if (r.alarm) [glow, power] = [colors.danger, still || r.effect === "glow" ? 0.6 : 0.35 + 0.45 * (0.5 + 0.5 * Math.sin(t * 6))];
      else if (r.effect === "blink" && r.tone) [glow, power] = [colors[r.tone], still ? 0.5 : Math.sin(t * Math.PI * 2) > 0 ? 0.7 : 0];
      else if (r.effect === "glow") [glow, power] = [r.tone ? colors[r.tone] : colors.primary, 0.55];
      else if (r.heat) [glow, power] = [r.heat, 0.55];
      if (r.shape === "sensor" && !glow) [glow, power] = [r.heat ?? (r.tone ? colors[r.tone] : colors.info), 0.6];
      const want = glow !== null || r.opacity < 1;
      if (!want && !it.mats) return;
      for (const m of ownMaterials(it)) {
        const b = m.userData.base;
        if (m.emissive) {
          if (glow) {
            m.emissive.set(glow);
            m.emissiveIntensity = power;
          } else if (b.emissive) {
            m.emissive.copy(b.emissive);
            m.emissiveIntensity = b.intensity;
          }
        }
        m.transparent = r.opacity < 1 || b.transparent;
        m.opacity = r.opacity < 1 ? r.opacity : b.opacity;
        if (r.heat && r.shape === "sensor" && m.color) m.color.set(r.heat);
      }
    }
    // A ring on the ground in the object's tone. Those that stay still are instances, one
    // draw call for each size of ring however many there are; one that pulses (an alarm,
    // `effect: pulse`) is a ring of its own.
    const ringBatches = new Map(); // a ring's size → { geometry, im, slots, cap }
    function ringShape(w, d) {
      const rounded = (hw, hd, rad) => {
        const p = new T.Shape();
        p.moveTo(-hw + rad, -hd);
        p.lineTo(hw - rad, -hd);
        p.quadraticCurveTo(hw, -hd, hw, -hd + rad);
        p.lineTo(hw, hd - rad);
        p.quadraticCurveTo(hw, hd, hw - rad, hd);
        p.lineTo(-hw + rad, hd);
        p.quadraticCurveTo(-hw, hd, -hw, hd - rad);
        p.lineTo(-hw, -hd + rad);
        p.quadraticCurveTo(-hw, -hd, -hw + rad, -hd);
        return p;
      };
      const line = Math.min(0.08, Math.min(w, d) * 0.12), rad = Math.min(w, d) * 0.25;
      const outer = rounded(w / 2, d / 2, rad);
      outer.holes.push(rounded(w / 2 - line, d / 2 - line, Math.max(0.01, rad - line)));
      return new T.ShapeGeometry(outer, 6);
    }
    const ringMaterial = () => new T.MeshBasicMaterial({ transparent: true, opacity: 0.9, depthWrite: false, side: T.DoubleSide });
    function unring(it) {
      if (it.ring) {
        it.group.remove(it.ring);
        dispose(it.ring);
        it.ring = null;
      }
      if (it.ringIn) {
        const i = it.ringIn.slots.indexOf(it);
        if (i >= 0) it.ringIn.slots.splice(i, 1);
        it.ringIn = null;
        placed = false;
      }
    }
    function ringFor(it) {
      const r = it.rec;
      if (!r.tone || !it.size) return unring(it);
      // Its footprint with a margin, as a rounded outline.
      const w = Math.max(0.5, it.size.x + 0.24), d = Math.max(0.5, it.size.z + 0.24), sig = `${w.toFixed(2)}x${d.toFixed(2)}`;
      // On the ground under it, however high it's raised.
      const y = 0.012 - (it.group.parent === world ? r.pos.y : 0) / (r.scale.y || 1);
      it.ringColor = colors[r.tone];
      it.ringY = y;
      if (r.effect !== "pulse" && !r.alarm) {
        if (it.ring || it.ringIn?.sig !== sig) unring(it);
        if (!it.ringIn) {
          let b = ringBatches.get(sig);
          if (!b) ringBatches.set(sig, (b = { sig, geometry: ringShape(w, d), im: null, slots: [], cap: 0 }));
          b.slots.push((it.ringIn = b, it));
        }
        placed = false;
        return;
      }
      if (it.ringIn || it.ring?.userData.sig !== sig) unring(it);
      if (!it.ring) {
        it.ring = new T.Mesh(ringShape(w, d), ringMaterial());
        it.ring.rotation.x = -Math.PI / 2;
        it.ring.renderOrder = 2;
        it.ring.userData.sig = sig;
        it.ring.raycast = () => {};
        it.group.add(it.ring);
      }
      it.ring.material.color.set(it.ringColor);
      it.ring.position.y = y;
    }
    function placeRings(seen, moved) {
      const m = new T.Matrix4(), local = new T.Matrix4(), turn = new T.Matrix4().makeRotationX(-Math.PI / 2), c = new T.Color();
      for (const b of ringBatches.values()) {
        const drawn = b.slots.filter(seen);
        if (!moved && b.im && sameList(b.drawn, drawn)) continue;
        b.drawn = drawn;
        if (b.slots.length > b.cap || !b.im) {
          b.cap = Math.max(8, b.cap * 2, b.slots.length);
          if (b.im) {
            world.remove(b.im);
            b.im.material.dispose();
            b.im.dispose();
          }
          b.im = new T.InstancedMesh(b.geometry, ringMaterial(), b.cap);
          b.im.renderOrder = 2;
          b.im.frustumCulled = false;
          b.im.raycast = () => {};
          world.add(b.im);
        }
        drawn.forEach((it, i) => {
          local.makeTranslation(0, it.ringY, 0).multiply(turn);
          b.im.setMatrixAt(i, m.multiplyMatrices(it.group.matrixWorld, local));
          b.im.setColorAt(i, c.set(it.ringColor));
        });
        b.im.count = drawn.length;
        b.im.instanceMatrix.needsUpdate = true;
        if (b.im.instanceColor) b.im.instanceColor.needsUpdate = true;
      }
    }
    function dispose(o, keepShared) {
      o.traverse?.((x) => {
        if (!x.isMesh && !x.isLine) return;
        if (!keepShared || x.userData.own || x.parent?.userData?.own) x.geometry?.dispose();
        for (const m of [].concat(x.material ?? [])) if (!keepShared || x.userData.own) m.dispose?.();
      });
    }

    // ---------------------------------------------------------------- paths
    function drawPaths() {
      pathMeshes.forEach((m) => {
        world.remove(m);
        dispose(m);
      });
      pathMeshes = [];
      paths = new Map();
      for (const [i, p] of (Array.isArray(W("paths")) ? W("paths") : []).entries()) {
        if (!isObj(p)) continue;
        const pts = (Array.isArray(p.points) ? p.points : []).map(vec).filter(Boolean).map((v) => new T.Vector3(v.x, v.y + (num(p.height) || 0), v.z));
        if (pts.length < 2) continue;
        if (p.closed) pts.push(pts[0].clone());
        const lens = [0];
        for (let k = 1; k < pts.length; k++) lens.push(lens[k - 1] + pts[k].distanceTo(pts[k - 1]));
        const id = str(p.id ?? `path-${i + 1}`);
        paths.set(id, { pts, lens, total: lens.at(-1), closed: !!p.closed });
        if (p.show === false) continue;
        const width = num(p.width) || 0.12, color = p.color && CSS.supports("color", p.color) ? ink(p.color).css : colors[TONES.includes(p.tone) ? p.tone : "info"];
        const curve = new T.CurvePath();
        for (let k = 1; k < pts.length; k++) curve.add(new T.LineCurve3(pts[k - 1].clone().setY(pts[k - 1].y + width / 2), pts[k].clone().setY(pts[k].y + width / 2)));
        const flow = p.flow !== undefined && p.flow !== false;
        const mat = new T.MeshBasicMaterial({ color, transparent: true, opacity: 0.9 });
        if (flow) {
          const c = Object.assign(document.createElement("canvas"), { width: 64, height: 8 }), x = c.getContext("2d");
          x.fillStyle = "#fff";
          x.fillRect(0, 0, 64, 8);
          x.fillStyle = "rgba(255,255,255,0.25)";
          x.fillRect(32, 0, 32, 8);
          const tex = new T.CanvasTexture(c);
          tex.wrapS = T.RepeatWrapping;
          tex.repeat.set(Math.max(1, Math.round(lens.at(-1) / (width * 6))), 1);
          mat.map = tex;
          mat.userData.flow = fin(p.flow) ? num(p.flow) : 1;
          mat.userData.repeat = width * 6;
        }
        const tube = new T.Mesh(new T.TubeGeometry(curve, Math.max(8, pts.length * 12), width / 2, 8, false), mat);
        tube.raycast = () => {};
        tube.userData.path = id;
        world.add(tube);
        pathMeshes.push(tube);
      }
    }
    // Where a distance along a path is, and which way it's going.
    function along(path, d) {
      const { pts, lens, total } = path;
      const at = Math.min(total, Math.max(0, d));
      let k = 1;
      while (k < lens.length - 1 && lens[k] < at) k++;
      const a = pts[k - 1], b = pts[k], f = (at - lens[k - 1]) / Math.max(1e-6, lens[k] - lens[k - 1]);
      return { x: a.x + (b.x - a.x) * f, y: a.y + (b.y - a.y) * f, z: a.z + (b.z - a.z) * f, yaw: Math.atan2(b.x - a.x, b.z - a.z) };
    }

    // ---------------------------------------------------------------- zones and walls
    const toneColor = (o, fallback) => (o.color && CSS.supports("color", o.color) ? ink(o.color).css : colors[TONES.includes(o.tone) ? o.tone : fallback]);
    function drawZones() {
      zones.forEach((z) => {
        world.remove(z.mesh);
        dispose(z.mesh);
        z.tag?.remove();
      });
      zones = [];
      for (const [i, z] of (Array.isArray(W("zones")) ? W("zones") : []).entries()) {
        if (!isObj(z) || z.show === false) continue;
        let pts = (Array.isArray(z.points) ? z.points : []).map(vec).filter(Boolean);
        const c = vec(z.center), s = Array.isArray(z.size) ? z.size.map(num) : null;
        if (pts.length < 3 && c && s && s[0] > 0 && s[1] > 0) pts = [[-1, -1], [1, -1], [1, 1], [-1, 1]].map(([a, b]) => ({ x: c.x + (a * s[0]) / 2, y: 0, z: c.z + (b * s[1]) / 2 }));
        if (pts.length < 3) continue;
        const shape = new T.Shape(pts.map((p) => new T.Vector2(p.x, -p.z)));
        const height = num(z.height) || 0, color = toneColor(z, "info"), opacity = fin(z.opacity) ? num(z.opacity) : height ? 0.12 : 0.18;
        const geo = height ? new T.ExtrudeGeometry(shape, { depth: height, bevelEnabled: false }) : new T.ShapeGeometry(shape);
        const mesh = new T.Mesh(geo, new T.MeshBasicMaterial({ color, transparent: true, opacity, depthWrite: false, side: T.DoubleSide }));
        mesh.rotation.x = -Math.PI / 2;
        mesh.position.y = 0.006;
        mesh.renderOrder = 1;
        // In the shape's own plane, which is turned to lie on the ground.
        const edge = new T.LineLoop(new T.BufferGeometry().setFromPoints(pts.map((p) => new T.Vector3(p.x, -p.z, height))), new T.LineBasicMaterial({ color, transparent: true, opacity: 0.8 }));
        mesh.add(edge);
        const id = str(z.id ?? `zone-${i + 1}`);
        mesh.userData.zone = id;
        world.add(mesh);
        const mid = pts.reduce((a, p) => ({ x: a.x + p.x / pts.length, z: a.z + p.z / pts.length }), { x: 0, z: 0 });
        const title = str(z.title ?? z.label ?? "");
        let tag = null;
        if (z.label ?? z.title) {
          tag = make("span", { "data-twin-zone": id, "data-tone": TONES.includes(z.tone) ? z.tone : null }, str(z.label ?? z.title));
          tagLayer.append(tag);
        }
        zones.push({ id, row: z, mesh, tag, title, at: new T.Vector3(mid.x, height + 0.05, mid.z), clickable: !!(cfg.bindZone || (cfg.popup && (z.title || z.description))) });
      }
    }
    function drawWalls() {
      walls.forEach((w) => {
        world.remove(w);
        dispose(w);
      });
      walls = [];
      for (const w of Array.isArray(W("walls")) ? W("walls") : []) {
        if (!isObj(w) || w.show === false) continue;
        const pts = (Array.isArray(w.points) ? w.points : []).map(vec).filter(Boolean);
        if (w.closed && pts.length > 2) pts.push(pts[0]);
        const h = num(w.height) || 3, t = num(w.thickness) || 0.2, opacity = fin(w.opacity) ? num(w.opacity) : 1;
        const mat = new T.MeshStandardMaterial({ color: w.color && CSS.supports("color", w.color) ? ink(w.color).css : colors.wall, roughness: 0.9, transparent: opacity < 1, opacity });
        const g = new T.Group();
        for (let k = 1; k < pts.length; k++) {
          const a = pts[k - 1], b = pts[k], len = Math.hypot(b.x - a.x, b.z - a.z);
          if (!len) continue;
          const m = new T.Mesh(new T.BoxGeometry(len + t, h, t), mat);
          m.position.set((a.x + b.x) / 2, h / 2 + (a.y || 0), (a.z + b.z) / 2);
          m.rotation.y = -Math.atan2(b.z - a.z, b.x - a.x);
          m.castShadow = opacity === 1;
          m.receiveShadow = true;
          g.add(m);
        }
        world.add(g);
        walls.push(g);
      }
    }

    // ---------------------------------------------------------------- the ground and light
    function extent() {
      const b = new T.Box3();
      for (const it of items.values()) if (it.group.parent) b.expandByPoint(new T.Vector3(it.rec.pos.x, 0, it.rec.pos.z));
      for (const z of zones) b.expandByObject(z.mesh);
      for (const w of walls) b.expandByObject(w);
      for (const p of paths.values()) p.pts.forEach((v) => b.expandByPoint(v));
      if (b.isEmpty()) b.set(new T.Vector3(-5, 0, -5), new T.Vector3(5, 0, 5));
      return b;
    }
    function drawGround() {
      for (const o of [ground, grid]) if (o) {
        world.remove(o);
        dispose(o);
      }
      ground = grid = null;
      const g = isObj(W("ground")) ? W("ground") : {};
      if (W("ground") === false || g.show === false) return;
      const b = extent(), c = b.getCenter(new T.Vector3()), s = b.getSize(new T.Vector3());
      const auto = Math.max(10, Math.ceil(Math.max(s.x, s.z) + 8));
      const size = Array.isArray(g.size) ? g.size.map(num) : fin(g.size) ? [num(g.size), num(g.size)] : [auto, auto];
      const at = vec(g.center) ?? { x: Array.isArray(g.size) || fin(g.size) ? 0 : Math.round(c.x), z: Array.isArray(g.size) || fin(g.size) ? 0 : Math.round(c.z) };
      ground = new T.Mesh(new T.PlaneGeometry(size[0], size[1]), new T.MeshStandardMaterial({ color: g.color && CSS.supports("color", g.color) ? ink(g.color).css : colors.ground, roughness: 0.95 }));
      ground.rotation.x = -Math.PI / 2;
      ground.position.set(at.x, 0, at.z);
      ground.receiveShadow = true;
      ground.userData.ground = true;
      world.add(ground);
      const cell = g.grid === false ? 0 : fin(g.grid) ? num(g.grid) : 1;
      if (cell > 0) {
        const n = Math.max(size[0], size[1]);
        grid = new T.GridHelper(n, Math.round(n / cell), colors.gridMajor, colors.grid);
        grid.position.set(at.x, 0.003, at.z);
        grid.material.transparent = true;
        grid.material.opacity = 0.7;
        grid.raycast = () => {};
        world.add(grid);
      }
      lightUp(b);
    }
    function lightUp(b = extent()) {
      const e = isObj(W("environment")) ? W("environment") : {};
      const preset = ["studio", "day", "night"].includes(e.preset) ? e.preset : "studio";
      const power = fin(e.light) ? num(e.light) : 1;
      hemi.intensity = (preset === "night" ? 0.2 : preset === "day" ? 0.9 : 0.55) * power;
      hemi.color.set(preset === "day" ? "#f0f6ff" : "#ffffff");
      hemi.groundColor.set(preset === "night" ? "#202030" : "#8a8a8a");
      sun.intensity = (preset === "night" ? 0.5 : preset === "day" ? 2.2 : 1.5) * power;
      scene.environmentIntensity = (preset === "night" ? 0.15 : 0.45) * power;
      const c = b.getCenter(new T.Vector3()), s = b.getSize(new T.Vector3()), r = Math.max(8, Math.max(s.x, s.z) * 0.75);
      sun.position.set(c.x + r * 0.6, r * 1.6, c.z + r * 0.4);
      sun.target.position.copy(c);
      sun.castShadow = e.shadows !== false && cfg.quality !== "low";
      Object.assign(sun.shadow.camera, { left: -r, right: r, top: r, bottom: -r, near: 0.5, far: r * 4 });
      sun.shadow.camera.updateProjectionMatrix();
      const bg = e.background;
      scene.background = bg && bg !== "none" && CSS.supports("color", bg) ? new T.Color(ink(bg).css) : null;
      scene.fog = e.fog ? new T.Fog(scene.background ?? new T.Color(colors.backdrop), r * 2, r * 6) : null;
      reshadow();
    }
    function themed() {
      const probe = make("i", { hidden: "" });
      el.append(probe);
      const css = (v) => {
        probe.style.color = v;
        return ink(getComputedStyle(probe).color).css;
      };
      for (const t of TONES) {
        probe.setAttribute("data-tone", t);
        colors[t] = css("var(--tone)");
      }
      probe.removeAttribute("data-tone");
      colors.ground = css("color-mix(in srgb, var(--color-subtle, #f4f4f5) 75%, var(--color-border, #e4e4e7))");
      colors.grid = css("color-mix(in srgb, var(--color-border, #e4e4e7) 80%, transparent)");
      colors.gridMajor = css("var(--color-border, #e4e4e7)");
      colors.wall = css("color-mix(in srgb, var(--color-muted, #71717a) 45%, var(--color-surface, #fff))");
      colors.shape = css("color-mix(in srgb, var(--color-muted, #71717a) 55%, var(--color-surface, #fff))");
      colors.muted = css("var(--color-muted, #71717a)");
      colors.backdrop = css("var(--color-subtle, #f4f4f5)");
      probe.remove();
      for (const o of [ruler.line, ruler.ends]) o?.material.color.set(colors.primary);
      if (!scene) return;
      drawGround();
      drawZones();
      drawPaths();
      for (const it of items.values()) {
        it.key = null;
        dress(it);
        ringFor(it);
      }
      for (const m of sels) m.material.color.set(colors.primary);
      wake();
    }

    // ---------------------------------------------------------------- objects
    const t0 = performance.now();
    function sync() {
      let gone = false;
      collect();
      const want = new Map(recs.map((r) => [r.id, r]));
      for (const [id, it] of items) if (!want.has(id)) {
        it.group.parent?.remove(it.group);
        dispose(it.group, true);
        unbatch(it);
        unring(it);
        it.tag?.remove();
        it.option?.remove();
        items.delete(id);
        if (picked.delete(id) || selected === id) gone = true;
      }
      // What's left of the picked ones stays picked.
      if (gone) pickMany([...picked], { popup: false });
      const now = performance.now(), dur = fin(cfg.transition) ? num(cfg.transition) : 600;
      for (const r of recs) {
        let it = items.get(r.id);
        const fresh = !it;
        if (fresh) {
          it = { id: r.id, rec: r, group: new T.Group(), key: null };
          it.group.userData.item = r.id;
          items.set(r.id, it);
        }
        const before = it.rec;
        it.rec = r;
        const parent = r.attach && items.get(r.attach) && r.attach !== r.id ? items.get(r.attach).group : world;
        if (it.group.parent !== parent) parent.add(it.group);
        // Moved by the data: there smoothly, unless it follows a path.
        if (fresh || r.path !== null || dur <= 0 || calm() || dragging?.it === it) it.move = null;
        else if (!same(before.pos, r.pos) || !same(before.rot, r.rot)) it.move = { from: { ...it.group.position }, yaw: it.group.rotation.y, t: now, dur };
        if (!it.move && r.path === null) place(it);
        it.group.scale.set(r.scale.x, r.scale.y, r.scale.z);
        it.group.visible = true;
        if (fresh || bodyKey(before) !== bodyKey(r) || !it.body) dress(it);
        else {
          if (before.animation !== r.animation) play(it);
          paint(it);
          ringFor(it);
          it.tag && (it.tag.__size = null);
        }
        tagFor(it);
        optionFor(it);
      }
      // In the order they're in the document; moved only when they aren't, which would lose the focus.
      const order = recs.map((r) => items.get(r.id).option).filter(Boolean);
      if (order.some((o, i) => list.children[i] !== o)) list.replaceChildren(...order);
      alarms();
      if (selected !== null && items.has(selected)) {
        set("bindObject", items.get(selected).rec.row);
        if (popup?.id === selected && proto && !popup.plain) api.list(proto, [items.get(selected).rec.row]);
        else if (popup?.id === selected) fillPopup(popup.box, items.get(selected).rec);
      }
      if (adding !== null && items.has(adding)) {
        const id = adding;
        adding = null;
        select(id, { popup: true });
      }
      if (!loading) progress();
      reshadow();
    }
    function place(it) {
      const r = it.rec;
      it.group.position.set(r.pos.x, r.pos.y, r.pos.z);
      it.group.rotation.set(r.rot.x, r.rot.y, r.rot.z);
      shadowsNow = true;
      placed = false;
    }
    let ringing = null;
    function alarms() {
      const now = recs.filter((r) => r.alarm);
      const fresh = ringing ? now.filter((r) => !ringing.has(r.id)) : [];
      ringing = new Set(now.map((r) => r.id));
      set("bindAlarms", now.map((r) => r.row.id ?? r.id));
      if (fresh.length) live.textContent = fresh.length > 3 ? L("{n} new alarms", { n: fresh.length }) : fresh.map((r) => L("Alarm: {title}", { title: r.title || r.id })).join(". ");
      for (const r of fresh) api.emit("alarm", { id: r.row.id ?? r.id, object: r.row });
    }
    // Tags over objects: a name, its readouts, alarms. `labels: auto | all | none`.
    function tagFor(it) {
      const r = it.rec, mode = props.labels ?? "auto";
      const reads = r.reads.filter((x) => x.tag);
      const shown = r.alarm || (mode !== "none" && (mode === "all" || r.label !== undefined || reads.length || r.shape === "sensor"));
      if (!shown) {
        it.tag?.remove();
        it.tag = null;
        return;
      }
      if (!it.tag) {
        it.tag = make("button", { type: "button", "data-twin-tag": r.id, tabindex: "-1" });
        it.tag.addEventListener("click", (e) => (measuring() ? measureAt(floorOf(r.id)) : adds(e) ? toggle(r.id) : select(r.id, { popup: true })));
        it.tag.toggleAttribute("data-selected", picked.has(r.id));
        tagLayer.append(it.tag);
      }
      const t = it.tag;
      t.setAttribute("data-tone", r.tone ?? "neutral");
      // Coloured by value, the dot says so too.
      if (r.heat && !r.tone) t.style.setProperty("--tone", r.heat);
      else t.style.removeProperty("--tone");
      t.toggleAttribute("data-alarm", r.alarm);
      t.toggleAttribute("data-effect", r.alarm && !calm());
      const name = str(r.label ?? r.title ?? r.id);
      const sig = JSON.stringify([name, reads.map((x) => [x.text, x.level]), r.alarm]);
      if (t.__sig === sig) return;
      t.__sig = sig;
      t.__size = null;
      t.replaceChildren(make("i", { "data-dot": "" }));
      if (r.alarm) t.append(make("b", { "data-bang": "" }, "!"));
      if (name && (mode === "all" || r.label !== undefined || !reads.length)) t.append(make("span", {}, name));
      for (const x of reads) t.append(make("span", { "data-read": "", "data-level": x.level }, x.text));
      t.setAttribute("aria-label", [name, ...reads.map((x) => `${x.label} ${x.text}`)].filter(Boolean).join(", "));
    }
    // The objects as a list for the keyboard and screen readers.
    function optionFor(it) {
      const r = it.rec;
      if (!r.clickable) {
        it.option?.remove();
        it.option = null;
        return;
      }
      if (!it.option) {
        it.option = make("li");
        const b = make("button", { type: "button", "data-twin-option": r.id });
        b.addEventListener("click", (e) => (measuring() ? measureAt(floorOf(r.id)) : adds(e) ? toggle(r.id) : select(r.id, { popup: true, fly: true, keys: true })));
        b.addEventListener("focus", () => (focused = r.id, wake()));
        b.addEventListener("blur", () => (focused = null, wake()));
        it.option.append(b);
      }
      const b = it.option.firstChild;
      const text = [r.title || r.id, r.alarm ? L("Alarm") : null, ...r.reads.map((x) => `${x.label} ${x.text}`)].filter(Boolean).join(", ");
      if (b.textContent !== text) b.textContent = text;
      b.setAttribute("aria-pressed", String(picked.has(r.id)));
    }
    let focused = null, adding = null;

    // ---------------------------------------------------------------- selecting
    function select(id, o = {}) {
      id = id === null || id === undefined ? null : str(id);
      if (id !== null && !items.has(id)) id = null;
      const was = selected;
      selected = id;
      if (!o.keep) {
        picked.clear();
        if (id !== null) picked.add(id);
      }
      marked(o.quiet);
      set("bindObject", id === null ? null : items.get(id).rec.row);
      // From `bindSelected`, `bind` follows (quietly: no events).
      if (o.bind) set("bind", id === null ? null : (items.get(id).rec.row.id ?? id));
      if (!o.quiet) {
        set("bind", id === null ? null : (items.get(id).rec.row.id ?? id));
        if (id !== was) api.emit("select", { id: id === null ? null : items.get(id).rec.row.id ?? id, object: id === null ? null : items.get(id).rec.row });
      }
      if (id === null || picked.size > 1) close(false);
      if (id !== null) {
        if (o.fly) flyTo(items.get(id));
        if (o.popup !== false && cfg.popup && picked.size <= 1) openPopup(items.get(id), o.keys);
      }
      wake();
    }
    // Several at once (`multiple`): the last one is the selected one, with its popup when
    // it's alone.
    function pickMany(ids, o = {}) {
      ids = [...new Set([].concat(ids ?? []).map(str))].filter((id) => items.has(id));
      if (!multi()) ids = ids.slice(-1);
      picked.clear();
      ids.forEach((id) => picked.add(id));
      const one = o.primary !== undefined && ids.includes(str(o.primary)) ? str(o.primary) : ids.includes(selected) ? selected : ids.at(-1) ?? null;
      select(one, { ...o, keep: true, popup: ids.length === 1 && o.popup !== false });
      if (ids.length > 1) live.textContent = L("{n} selected", { n: ids.length });
    }
    // In or out of the objects picked (⌘, Ctrl or Shift and a click).
    function toggle(id) {
      const ids = [...picked];
      pickMany(picked.has(id) ? ids.filter((x) => x !== id) : [...ids, id], { popup: false, primary: picked.has(id) ? undefined : id });
    }
    const adds = (e) => multi() && (e.metaKey || e.ctrlKey || e.shiftKey);
    // Marks the picked ones in the list and on their tags, and says which they are.
    let lastPicked = "[]";
    function marked(quiet) {
      for (const it of items.values()) {
        it.option?.firstChild.setAttribute("aria-pressed", String(picked.has(it.id)));
        it.tag?.toggleAttribute("data-selected", picked.has(it.id));
      }
      const rows = [...picked].map((id) => items.get(id).rec.row), ids = rows.map((r, i) => r.id ?? [...picked][i]);
      const now = JSON.stringify(ids);
      if (now === lastPicked) return;
      lastPicked = now;
      set("bindSelected", ids);
      if (!quiet) api.emit("selectionchange", { ids, objects: rows });
    }
    function fillPopup(box, r) {
      box.querySelectorAll(":scope > :not([data-twin-close])").forEach((n) => n.remove());
      const parts = [];
      if (r.title) parts.push(make("strong", {}, r.title));
      if (r.description) parts.push(make("p", {}, str(r.description)));
      if (r.reads.length) {
        const dl = make("dl");
        for (const x of r.reads) {
          const dd = make("dd", { "data-level": x.level }, x.text);
          if (x.past?.length > 1) dd.prepend(spark(x));
          dl.append(make("dt", {}, x.label), dd);
        }
        parts.push(dl);
      }
      if (r.href && allowed(r.href) && (!cfg.safe || /^(https?:|[/?#.]|[^:]*$)/i.test(r.href))) parts.push(make("a", { href: r.href }, L("Open")));
      if (editable() && r.path === null && r.attach === null) {
        const row = make("div", { "data-twin-edit": "" });
        const b = (label, f) => {
          const x = make("button", { type: "button" }, label);
          x.addEventListener("click", f);
          return x;
        };
        row.append(b(L("Rotate"), () => turnBy(items.get(r.id), 1)), b(L("Remove"), () => removeObject(r.id)));
        parts.push(row);
      }
      box.prepend(...parts);
    }
    function openPopup(target, keys, zone) {
      close(false);
      const r = zone ? null : target.rec;
      const box = make("div", { "data-twin-popup": "", role: "dialog", "aria-label": (zone ? target.title : r.title) || L("Object"), dir: "auto" });
      const x = make("button", { type: "button", "data-twin-close": "", "aria-label": L("Close") });
      x.innerHTML = CLOSE;
      x.addEventListener("click", () => close(true));
      const plain = zone || !proto;
      if (!plain) {
        proto.hidden = false;
        box.append(proto);
      } else if (zone) {
        if (target.title) box.append(make("strong", {}, target.title));
        if (target.row.description) box.append(make("p", {}, str(target.row.description)));
      } else fillPopup(box, r);
      box.append(x);
      tagLayer.append(box);
      popup = { box, id: zone ? null : target.id, zone: zone ? target : null, plain };
      if (!plain) api.list(proto, [r.row]);
      place2d();
      if (keys) x.focus();
    }
    function close(back) {
      if (!popup) return;
      const was = popup;
      popup = null;
      if (proto && !was.plain) {
        proto.hidden = true;
        el.append(proto);
      }
      was.box.remove();
      if (back) {
        const b = was.id !== null && items.get(was.id)?.option?.firstChild;
        (b && el.contains(b) ? b : host).focus({ preventScroll: true });
      }
      if (was.id !== null && back) select(null);
    }

    // ---------------------------------------------------------------- the camera
    let flight = null, followAt = null;
    const view = () => ({ position: camera.position.toArray().map((n) => round(n, 2)), target: controls.target.toArray().map((n) => round(n, 2)) });
    function fly(position, target, quick) {
      const p = vec(position), t = vec(target) ?? (p ? { x: controls.target.x, y: controls.target.y, z: controls.target.z } : null);
      if (!p || !t) return;
      if (quick || calm()) {
        camera.position.set(p.x, p.y, p.z);
        controls.target.set(t.x, t.y, t.z);
        controls.update();
        flight = null;
        return wake();
      }
      flight = { from: camera.position.clone(), fromT: controls.target.clone(), to: new T.Vector3(p.x, p.y, p.z), toT: new T.Vector3(t.x, t.y, t.z), t: performance.now(), dur: 700 };
      wake();
    }
    function fit(quick) {
      const b = extent();
      for (const it of items.values()) if (it.group.parent) {
        b.expandByObject(it.group);
        if (it.batch && it.size) b.expandByPoint(it.group.getWorldPosition(new T.Vector3()).add(new T.Vector3(0, it.size.y, 0)));
      }
      const c = b.getCenter(new T.Vector3()), s = b.getSize(new T.Vector3());
      const r = Math.max(2, s.length() / 2);
      const d = (r / Math.sin(deg(camera.fov / 2))) * 0.72;
      const dir = new T.Vector3(0.75, 0.85, 1).normalize();
      fly(c.clone().addScaledVector(dir, d).toArray(), [c.x, 0, c.z], quick);
    }
    function flyTo(it) {
      const p = it.group.getWorldPosition(new T.Vector3());
      const off = camera.position.clone().sub(controls.target);
      const want = Math.max(6, Math.max(it.size?.x ?? 1, it.size?.y ?? 1, it.size?.z ?? 1) * 4);
      off.setLength(Math.min(off.length(), want));
      fly(p.clone().add(off).toArray(), [p.x, p.y + (it.size?.y ?? 0) / 3, p.z]);
    }
    function goToView(id) {
      const v = (Array.isArray(W("views")) ? W("views") : []).find((x) => isObj(x) && str(x.id ?? x.label) === str(id));
      if (v) fly(v.position, v.target);
      for (const b of viewsBox?.querySelectorAll("button") ?? []) b.setAttribute("aria-pressed", String(b.dataset.twinView === str(id)));
    }
    function drawViews() {
      if (!viewsBox) return;
      const vs = (Array.isArray(W("views")) ? W("views") : []).filter(isObj);
      viewsBox.hidden = !vs.length;
      const sig = JSON.stringify(vs.map((v) => [v.id, v.label]));
      if (viewsBox.__sig === sig) return;
      viewsBox.__sig = sig;
      viewsBox.replaceChildren(...vs.map((v) => {
        const id = str(v.id ?? v.label), b = make("button", { type: "button", "data-twin-view": id, "aria-pressed": String(cfg.bindView && str(api.state[cfg.bindView]) === id) }, str(v.label ?? v.id));
        b.addEventListener("click", () => {
          if (cfg.bindView) api.set(cfg.bindView, v.id ?? v.label);
          goToView(id);
        });
        return b;
      }));
    }
    function cameraStart() {
      const c = isObj(W("camera")) ? W("camera") : {};
      camera.fov = fin(c.fov) ? num(c.fov) : 45;
      camera.updateProjectionMatrix();
      const v = cfg.bindView ? (Array.isArray(W("views")) ? W("views") : []).find((x) => isObj(x) && str(x.id ?? x.label) === str(api.state[cfg.bindView])) : null;
      if (v && vec(v.position)) fly(v.position, v.target, true);
      else if (vec(c.position)) fly(c.position, c.target ?? [0, 0, 0], true);
      else fit(true);
    }

    // ---------------------------------------------------------------- each frame
    let raf = null, inView = false, dirty = true, last = 0;
    // Whether the instances are where their objects are: anything but the camera moving
    // may have moved one, and they're all placed again then. While the view only turns,
    // only which of them are in view is looked at again (`looked`).
    let placed = false, looked = false;
    const v3 = () => new T.Vector3();
    const look = () => {
      dirty = looked = true;
      if (!raf && inView && renderer && !stopped) raf = nextFrame(frame);
    };
    const wake = () => {
      placed = false;
      look();
    };
    // Moves what moves; true while anything does.
    function step(now, dt) {
      const t = (now - t0) / 1000, still = calm();
      let moving = false;
      // Instances in alarm glow together, as one in alarm of its own does (see paint).
      for (const b of batches.values()) if (b.glowing.length && b.slots.length) {
        const power = still ? 0.6 : 0.35 + 0.45 * (0.5 + 0.5 * Math.sin(t * 6));
        for (const m of b.glowing) {
          m.emissive.set(colors.danger);
          m.emissiveIntensity = power;
        }
        moving ||= !still;
      }
      if (flight) {
        const f = Math.min(1, (now - flight.t) / flight.dur), e = 1 - (1 - f) ** 3;
        camera.position.lerpVectors(flight.from, flight.to, e);
        controls.target.lerpVectors(flight.fromT, flight.toT, e);
        if (f >= 1) flight = null;
        moving = true;
      }
      for (const it of items.values()) {
        const r = it.rec;
        if (r.path !== null && paths.has(r.path)) {
          const p = paths.get(r.path);
          // `progress` (0 to 1) places it; with `speed` too, that's where it starts from. The
          // distance is its own, so a change of speed (from the data) doesn't make it jump.
          const speed = r.row.speed !== undefined || r.progress === null ? r.speed : 0;
          if (it.dist === undefined || (speed === 0 && r.row.speed === undefined)) it.dist = (r.progress ?? 0) * p.total;
          else it.dist += speed * dt;
          if (r.loop || p.closed) it.dist = ((it.dist % p.total) + p.total) % p.total;
          const at = along(p, it.dist);
          it.group.position.set(at.x, at.y + r.pos.y, at.z);
          if (r.row.face !== false) it.group.rotation.set(r.rot.x, at.yaw + r.rot.y, r.rot.z);
          else it.group.rotation.set(r.rot.x, r.rot.y, r.rot.z);
          placed &&= speed === 0;
          moving ||= speed !== 0;
          casters ||= speed !== 0;
        } else if (it.move) {
          const f = Math.min(1, (now - it.move.t) / it.move.dur), e = 1 - (1 - f) ** 3;
          const { from } = it.move;
          it.group.position.set(from.x + (r.pos.x - from.x) * e, from.y + (r.pos.y - from.y) * e, from.z + (r.pos.z - from.z) * e);
          let dy = r.rot.y - it.move.yaw;
          dy = Math.atan2(Math.sin(dy), Math.cos(dy));
          it.group.rotation.set(r.rot.x, it.move.yaw + dy * e, r.rot.z);
          if (f >= 1) it.move = null;
          moving = casters = true;
          placed = false;
        }
        if (!it.body) continue;
        if (!still && r.effect === "spin") {
          it.body.rotation.y += dt * deg(fin(r.row.spin) ? num(r.row.spin) : 90);
          moving = casters = true;
          placed = false;
        }
        if (!still && r.effect === "bob") {
          it.body.position.y = 0.06 * (1 + Math.sin(t * 3));
          moving = casters = true;
          placed = false;
        }
        if (it.mixer) {
          it.mixer.update(still ? 0 : dt);
          moving ||= !still;
          casters ||= !still;
        }
        if (!still && (r.alarm || r.effect === "blink")) {
          paint(it, t);
          moving = true;
        }
        if (it.ring) {
          const pulse = !still && (r.effect === "pulse" || (r.alarm && r.effect !== "glow"));
          const f = pulse ? (t * 0.8) % 1 : 0;
          it.ring.scale.set(1 + f * 0.35, 1 + f * 0.35, 1);
          it.ring.material.opacity = pulse ? 0.9 * (1 - f) + 0.1 : 0.9;
          moving ||= pulse;
        }
      }
      for (const m of pathMeshes) if (m.material.map && m.material.userData.flow && !still) {
        m.material.map.offset.x = -((t * m.material.userData.flow) / m.material.userData.repeat) % 1;
        moving = true;
      }
      // Follows an object: the camera keeps its distance as it moves.
      const fid = props.follow === null || props.follow === undefined || props.follow === "" ? null : str(props.follow);
      const fit2 = fid !== null ? items.get(fid) : null;
      if (fit2) {
        const p = fit2.group.getWorldPosition(v3());
        if (followAt) {
          const d = p.clone().sub(followAt);
          if (d.lengthSq() > 0) {
            camera.position.add(d);
            controls.target.add(d);
            moving = true;
          }
        } else if (!flight) flyTo(fit2);
        followAt = p;
      } else followAt = null;
      // A ring under each picked object, from a pool.
      const s = selected !== null ? items.get(selected) : null;
      let n = 0;
      for (const id of picked) {
        const x = items.get(id);
        if (!x?.size) continue;
        if (!sels[n]) {
          sels[n] = ringOf(1);
          sels[n].material.color.set(colors.primary);
        }
        const m = sels[n++], p = x.group.getWorldPosition(v3());
        m.visible = true;
        m.position.set(p.x, 0.02, p.z);
        m.scale.setScalar(Math.max(0.45, Math.hypot(x.size.x * x.rec.scale.x, x.size.z * x.rec.scale.z) / 2 + 0.25));
      }
      for (; n < sels.length; n++) sels[n].visible = false;
      const f = focused !== null ? items.get(focused) : null;
      if (f && !picked.has(f.id) && f.size) {
        const p = f.group.getWorldPosition(v3());
        hover.visible = true;
        hover.position.set(p.x, 0.02, p.z);
        hover.scale.setScalar(Math.max(0.45, Math.hypot(f.size.x, f.size.z) / 2 + 0.25));
      } else hover.visible = false;
      return moving;
    }
    // Tags, zone names and the popup over where their things are now.
    function place2d(again = 0) {
      if (!camera) return;
      const w = host.clientWidth, h = host.clientHeight, p = v3();
      // Styles are written only when they change: with many tags, writing them all each
      // frame costs more than drawing the view.
      const show = (node, on) => {
        if (node.__on !== on) node.style.visibility = (node.__on = on) ? "" : "hidden";
      };
      // Where it goes, written by `move` once every tag has been measured: a size read
      // after a write would lay out the page again, once for each tag.
      const put = (node, at) => {
        p.copy(at).project(camera);
        if (p.z > 1 || p.z < -1 || Math.abs(p.x) > 1.2 || Math.abs(p.y) > 1.2) return null;
        return [round(((p.x + 1) / 2) * w, 1), round(((1 - p.y) / 2) * h, 1)];
      };
      const move = (node, [x, y]) => {
        if (node.__x !== x || node.__y !== y) node.style.transform = `translate(${(node.__x = x)}px, ${(node.__y = y)}px)`;
      };
      // Over its top: from its place as last drawn (the frame has just updated it).
      const q = v3();
      const top = (it) => {
        q.setFromMatrixPosition(it.group.matrixWorld);
        q.y += (it.size?.y ?? 1) * it.rec.scale.y + 0.25;
        return q;
      };
      // Tags that would cover one that matters more (selected, in alarm, warning) wait.
      const shown = [], off = [];
      const at = (node, xy, rank, middle) => {
        node.__size ||= [node.offsetWidth, node.offsetHeight];
        const [w, h] = node.__size, [x, y] = xy;
        shown.push({ node, xy, rank, l: x - w / 2, r: x + w / 2, t: middle ? y - h / 2 : y - h, b: middle ? y + h / 2 : y });
      };
      for (const it of items.values()) {
        if (!it.tag) continue;
        const xy = put(it.tag, top(it)), r = it.rec;
        if (xy) at(it.tag, xy, it.id === selected ? 5 : r.alarm ? 4 : r.level === "warn" || r.tone === "warning" ? 3 : 2);
        else off.push(it.tag);
      }
      for (const z of zones) if (z.tag) {
        const xy = put(z.tag, z.at);
        if (xy) at(z.tag, xy, 1, true);
        else off.push(z.tag);
      }
      for (const node of off) show(node, false);
      // A hidden tag stays where it was: only those shown are moved.
      const keep = (s, on) => {
        if (on) move(s.node, s.xy);
        show(s.node, on);
      };
      if (props.labels !== "all") {
        shown.sort((a, b) => b.rank - a.rank);
        // The tags kept so far, by the 64px cells they cover, so each is checked against its
        // neighbours only.
        const cells = new Map(), C = 64;
        const each = (s, f) => {
          for (let cx = Math.floor((s.l - 2) / C); cx <= Math.floor((s.r + 2) / C); cx++)
            for (let cy = Math.floor((s.t - 2) / C); cy <= Math.floor((s.b + 2) / C); cy++) if (f(cx * 65536 + cy)) return true;
          return false;
        };
        // An alarm's tag covers any other, but not another alarm's: one hidden behind it is
        // counted on it instead ("+2"), so a crowded hall isn't a pile of red. The selected
        // one always shows.
        for (const s of shown) {
          s.more = 0;
          let by = null;
          each(s, (c) => cells.get(c)?.some((k) => s.l < k.r + 2 && s.r > k.l - 2 && s.t < k.b + 2 && s.b > k.t - 2 && (by = k)));
          const on = !by || s.rank >= 5;
          keep(s, on);
          if (on) each(s, (c) => void (cells.get(c) ?? cells.set(c, []).get(c)).push(s));
          else if (s.rank === 4 && by.rank >= 4) by.more++;
        }
      } else for (const s of shown) keep(s, true);
      let resized = false;
      for (const s of shown) {
        const more = s.more ? `+${s.more}` : null;
        if (s.node.__more === more) continue;
        if (more) s.node.setAttribute("data-more", more);
        else s.node.removeAttribute("data-more");
        // Wider or narrower with its count: measured again, and the tags placed again
        // with it (a wider one may cover a neighbour now), a couple of times at most.
        s.node.__more = more;
        s.node.__size = null;
        resized = true;
      }
      if (resized && props.labels !== "all" && again < 2) return place2d(again + 1);
      const end = ruler.b ?? ruler.at;
      if (ruler.tag && ruler.a && end) {
        const xy = put(ruler.tag, q.lerpVectors(ruler.a, end, 0.5));
        if (xy) move(ruler.tag, xy);
        show(ruler.tag, !!xy);
      }
      if (popup) {
        const at = popup.zone ? popup.zone.at : items.has(popup.id) ? top(items.get(popup.id)) : null;
        const xy = at && put(popup.box, at);
        if (xy) move(popup.box, xy);
        if (at) show(popup.box, !!xy);
        fitPopup();
      }
    }
    // Keeps the popup inside the view.
    function fitPopup() {
      if (!popup) return;
      const { box } = popup, c = el.getBoundingClientRect();
      box.style.setProperty("--shift", "0px");
      box.removeAttribute("data-below");
      let b = box.getBoundingClientRect();
      if (b.top < c.top + 8) box.setAttribute("data-below", "");
      b = box.getBoundingClientRect();
      const shift = b.left < c.left + 8 ? c.left + 8 - b.left : b.right > c.right - 8 ? c.right - 8 - b.right : 0;
      if (Math.abs(shift) < b.width) box.style.setProperty("--shift", `${shift}px`);
    }
    // The shadow map is drawn again only when something that casts a shadow has changed: the
    // sun's shadows don't depend on the camera, so turning the view costs no shadow pass.
    // What moves on its own (on a path, spinning) redraws it at most every 50 ms; anything
    // else (the data, a drag, a model loaded) at once.
    let casters = false, shadowsNow = true, shadowAt = 0, frameMs = 0;
    const reshadow = () => {
      shadowsNow = true;
      wake();
    };
    function shadows(now) {
      if (!renderer.shadowMap.enabled) return;
      if (shadowsNow || (casters && now - shadowAt >= 50)) {
        renderer.shadowMap.needsUpdate = true;
        shadowAt = now;
        shadowsNow = casters = false;
      }
    }
    // While the camera moves, the view is drawn at a lower resolution (`motionScale` of the
    // pixels each way); it's drawn sharp again once the camera has been still for 400 ms.
    // Objects moving on their own don't lower it: with the camera still, the view stays sharp.
    const sharp = () => Math.min(devicePixelRatio || 1, cfg.quality === "low" ? 1 : 2);
    const scale = fin(cfg.motionScale) ? Math.min(1, Math.max(0.25, num(cfg.motionScale))) : 0.65;
    let lastCam = null, movedAt = 0, soft = false, restoreTimer = 0;
    function motion(now) {
      if (scale >= 1) return;
      const m = camera.matrixWorld;
      const moved = lastCam && !lastCam.equals(m);
      lastCam = (lastCam ?? new T.Matrix4()).copy(m);
      if (moved) {
        movedAt = now;
        if (!soft) {
          soft = true;
          renderer.setPixelRatio(sharp() * scale);
          el.setAttribute("data-moving", "");
        }
        clearTimeout(restoreTimer);
        restoreTimer = setTimeout(wake, 420);
      } else if (soft && now - movedAt >= 400) {
        soft = false;
        renderer.setPixelRatio(sharp());
        el.removeAttribute("data-moving");
        dirty = true;
      }
    }
    function frame(now) {
      raf = null;
      if (stopped) return;
      if (!el.isConnected) return stop();
      const dt = Math.min(0.1, last ? (now - last) / 1000 : 0);
      last = now;
      const moving = step(now, dt);
      const turned = controls.update();
      // Still soft from moving: drawn once more when it's sharp again.
      if (soft && !moving && !turned && !flight && performance.now() - movedAt >= 400) dirty = true;
      if (dirty || moving || turned) {
        const t1 = performance.now();
        camera.updateMatrixWorld();
        motion(now);
        // Tags move only when the view or something in it has: not while alarms just glow.
        const shift = !placed || looked || !!flight;
        if (shift) placeInstances(!placed);
        placed = true;
        looked = false;
        shadows(now);
        renderer.render(scene, camera);
        if (shift) place2d();
        // How long a frame's work takes on the main thread, smoothed, for `info`.
        frameMs = frameMs ? frameMs * 0.9 + (performance.now() - t1) * 0.1 : performance.now() - t1;
        dirty = false;
        el.__frames = (el.__frames || 0) + 1;
      }
      if (inView && !document.hidden && (moving || turned || flight)) raf = nextFrame(frame);
    }
    function stop() {
      stopped = true;
      raf?.();
      clearInterval(poll);
      source?.close();
      renderer?.setAnimationLoop(null);
      renderer?.dispose();
    }

    // ---------------------------------------------------------------- pointer and keys
    let caster = null, down = null, dragging = null;
    const pointerAt = (e) => {
      const r = host.getBoundingClientRect();
      return new T.Vector2(((e.clientX - r.left) / r.width) * 2 - 1, -((e.clientY - r.top) / r.height) * 2 + 1);
    };
    function pick(e) {
      caster ||= new T.Raycaster();
      caster.setFromCamera(pointerAt(e), camera);
      const instanced = [...batches.values()].flatMap((b) => [...b.parts, ...(b.farParts ?? [])].map((p) => p.im).filter(Boolean));
      const targets = [...[...items.values()].filter((it) => it.rec.clickable && it.group.visible).map((it) => it.group), ...instanced, ...zones.filter((z) => z.clickable).map((z) => z.mesh)];
      for (const h of caster.intersectObjects(targets, true)) {
        // An instance stands for the object in that place of its batch.
        const b = h.object.userData.batch;
        if (b) {
          const it = (h.object.userData.far ? b.drawnFar : b.drawn)?.[h.instanceId];
          if (it?.rec.clickable && items.get(it.id) === it) return { item: it };
          continue;
        }
        for (let o = h.object; o; o = o.parent) {
          if (o.userData.item !== undefined) return { item: items.get(o.userData.item) };
          if (o.userData.zone !== undefined) return { zone: zones.find((z) => z.id === o.userData.zone), point: h.point };
        }
      }
      return null;
    }
    const groundPoint = (e) => {
      caster ||= new T.Raycaster();
      caster.setFromCamera(pointerAt(e), camera);
      const p = new T.Vector3();
      return caster.ray.intersectPlane(new T.Plane(new T.Vector3(0, 1, 0), 0), p) ? p : null;
    };
    const editable = () => !!props.editable && !!cfg.bindObjects;
    const snapTo = (n) => {
      const s = fin(props.snap) ? num(props.snap) : 0;
      return s > 0 ? Math.round(n / s) * s : round(n, 3);
    };
    // Where an object's middle is in the view, in pixels from its top left; null behind it.
    function onScreen(it) {
      const p = (it.batch ? it.group.getWorldPosition(new T.Vector3()).add(new T.Vector3(0, (it.size?.y ?? 0) / 2, 0)) : new T.Box3().setFromObject(it.body ?? it.group).getCenter(new T.Vector3())).project(camera);
      return p.z > 1 ? null : { x: round(((p.x + 1) / 2) * host.clientWidth, 1), y: round(((1 - p.y) / 2) * host.clientHeight, 1) };
    }
    // With `multiple`, Shift and a drag from where there's no object draws a box: what's
    // in it is picked too. It's the view's before the camera's (Shift-drag pans otherwise).
    let box = null;
    function bindBox() {
      const at = (e) => {
        const r = host.getBoundingClientRect();
        return { x: e.clientX - r.left, y: e.clientY - r.top };
      };
      const rect = () => ({ l: Math.min(box.a.x, box.b.x), t: Math.min(box.a.y, box.b.y), r: Math.max(box.a.x, box.b.x), b: Math.max(box.a.y, box.b.y) });
      host.addEventListener("pointerdown", (e) => {
        if (!multi() || !e.shiftKey || e.button !== 0 || !renderer || measuring() || e.target !== renderer.domElement || pick(e)?.item) return;
        e.stopPropagation();
        box = { a: at(e), b: at(e), id: e.pointerId, el: make("div", { "data-twin-box": "" }), base: [...picked] };
        tagLayer.append(box.el);
        try {
          host.setPointerCapture(e.pointerId);
        } catch {
          // A pointer the browser doesn't track (a synthetic one): moves still come here.
        }
      }, true);
      host.addEventListener("pointermove", (e) => {
        if (!box || e.pointerId !== box.id) return;
        e.stopPropagation();
        box.b = at(e);
        const q = rect();
        Object.assign(box.el.style, { transform: `translate(${q.l}px, ${q.t}px)`, width: `${q.r - q.l}px`, height: `${q.b - q.t}px` });
      }, true);
      const end = (e) => {
        if (!box || e.pointerId !== box.id) return;
        e.stopPropagation();
        const q = rect(), was = box;
        box = null;
        was.el.remove();
        if (e.type === "pointercancel" || (q.r - q.l < 4 && q.b - q.t < 4)) return;
        const inside = [...items.values()].filter((it) => {
          if (!it.rec.clickable || !it.group.visible) return false;
          const p = onScreen(it);
          return p && p.x >= q.l && p.x <= q.r && p.y >= q.t && p.y <= q.b;
        });
        pickMany([...was.base, ...inside.map((it) => it.id)], { popup: false });
      };
      host.addEventListener("pointerup", end, true);
      host.addEventListener("pointercancel", end, true);
    }
    function bindPointer() {
      bindBox();
      const c = renderer.domElement;
      c.addEventListener("pointerdown", (e) => {
        if (e.button !== 0) return;
        down = { x: e.clientX, y: e.clientY };
        if (!editable() || measuring()) return;
        const hit = pick(e);
        const it = hit?.item;
        if (!it || it.rec.path !== null || it.rec.attach !== null) return;
        const g = groundPoint(e);
        if (!g) return;
        // A picked one drags the others picked with it, each keeping its place to it.
        const all = (picked.has(it.id) ? [...picked].map((id) => items.get(id)) : [it]).filter((x) => x && x.rec.path === null && x.rec.attach === null);
        dragging = { it, all: all.map((x) => ({ it: x, off: { x: x.group.position.x - g.x, z: x.group.position.z - g.z } })), moved: false, id: e.pointerId };
        controls.enabled = false;
        c.setPointerCapture(e.pointerId);
      });
      c.addEventListener("pointermove", (e) => {
        if (dragging && e.pointerId === dragging.id) {
          const g = groundPoint(e);
          if (!g) return;
          dragging.moved = true;
          for (const { it, off } of dragging.all) {
            it.rec.pos = { ...it.rec.pos, x: snapTo(g.x + off.x), z: snapTo(g.z + off.z) };
            place(it);
          }
          host.setAttribute("data-dragging", "");
          return wake();
        }
        if (e.buttons) return;
        if (!hoverQueued) {
          hoverQueued = true;
          nextFrame(() => {
            hoverQueued = false;
            if (!renderer) return;
            // Half measured, the line follows the pointer.
            if (measuring()) return ruler.a && !ruler.b && ((ruler.at = surfacePoint(e)), rulerDraw());
            const h = pick(e);
            c.style.cursor = h ? (editable() && h.item ? "grab" : "pointer") : "";
          });
        }
      });
      const up = (e) => {
        if (dragging && e.pointerId === dragging.id) {
          const d = dragging;
          dragging = null;
          controls.enabled = true;
          host.removeAttribute("data-dragging");
          if (d.moved) {
            writeBack(...d.all.map((x) => x.it));
            down = null;
            return;
          }
        }
        if (!down || e.type === "pointercancel") return;
        const far = Math.hypot(e.clientX - down.x, e.clientY - down.y) > 5;
        down = null;
        if (far) return;
        if (measuring()) return measureAt(surfacePoint(e));
        const hit = pick(e);
        if (hit?.item && adds(e)) toggle(hit.item.id);
        else if (hit?.item) select(hit.item.id, { popup: true });
        else if (hit?.zone) {
          set("bindZone", hit.zone.row.id ?? hit.zone.id);
          api.emit("zoneselect", { id: hit.zone.row.id ?? hit.zone.id, zone: hit.zone.row });
          if (cfg.popup && (hit.zone.title || hit.zone.row.description)) {
            select(null);
            openPopup(hit.zone, false, true);
          }
        } else select(null);
      };
      c.addEventListener("pointerup", up);
      c.addEventListener("pointercancel", up);
    }
    let hoverQueued = false;

    // ---------------------------------------------------------------- measuring
    // With `measure`, the ruler (its button, or M): two clicks, on objects, walls or the
    // floor, and the distance between them is drawn and said. Picked from a tag or the
    // list, an object counts from its middle on the floor. Drawn over everything, so a
    // line through a wall still shows.
    const ruler = { on: false, a: null, b: null, at: null, line: null, ends: null, tag: null };
    const measuring = () => ruler.on && !!props.measure;
    function surfacePoint(e) {
      caster ||= new T.Raycaster();
      caster.setFromCamera(pointerAt(e), camera);
      const instanced = [...batches.values()].flatMap((b) => [...b.parts, ...(b.farParts ?? [])].map((p) => p.im).filter(Boolean));
      const targets = [...[...items.values()].filter((it) => it.group.visible).map((it) => it.group), ...instanced, ...walls, ...zones.map((z) => z.mesh)];
      const h = caster.intersectObjects(targets, true)[0];
      return h ? h.point.clone() : groundPoint(e);
    }
    const floorOf = (id) => {
      const it = items.get(str(id));
      return it ? it.group.getWorldPosition(v3()).setY(0) : null;
    };
    function measureMode(on) {
      ruler.on = !!on && !!props.measure;
      host.toggleAttribute("data-measuring", ruler.on);
      for (const b of el.querySelectorAll("[data-twin-action=measure]")) b.setAttribute("aria-pressed", String(ruler.on));
      if (ruler.on) {
        select(null);
        say(L("Click two points to measure between them"), true);
        host.focus({ preventScroll: true });
      } else {
        say("");
        measure(null);
      }
    }
    // The distance from one point to another, drawn; null clears it.
    function measure(a, b) {
      ruler.a = a ?? null;
      ruler.b = (a && b) ?? null;
      ruler.at = null;
      rulerDraw();
      if (!ruler.b) return (set("bindMeasure", null), null);
      // To the centimetre.
      const d = ruler.b.clone().sub(ruler.a), m = { from: ruler.a.toArray().map((x) => round(x, 2)), to: ruler.b.toArray().map((x) => round(x, 2)), distance: round(d.length(), 2), across: round(Math.hypot(d.x, d.z), 2), up: round(d.y, 2) };
      set("bindMeasure", m);
      api.emit("measure", m);
      live.textContent = L("{d} m between them", { d: metres(m.distance) });
      return m;
    }
    function measureAt(p) {
      if (!p) return;
      if (ruler.a && !ruler.b) {
        say("");
        measure(ruler.a, p);
      } else {
        measure(p);
        say(L("Click a second point"), true);
      }
    }
    const metres = (n) => numbers(2).format(n);
    function rulerDraw() {
      const to = ruler.b ?? ruler.at, pts = [ruler.a, to].filter(Boolean);
      if (!ruler.line && pts.length) {
        const over = { color: colors.primary, depthTest: false, depthWrite: false, transparent: true };
        const dot = make("canvas", { width: 32, height: 32 }), g = dot.getContext("2d");
        g.fillStyle = "#fff";
        g.beginPath();
        g.arc(16, 16, 15, 0, Math.PI * 2);
        g.fill();
        ruler.line = new T.Line(new T.BufferGeometry(), new T.LineBasicMaterial(over));
        ruler.ends = new T.Points(new T.BufferGeometry(), new T.PointsMaterial({ ...over, size: 11, sizeAttenuation: false, map: new T.CanvasTexture(dot), alphaTest: 0.5 }));
        for (const o of [ruler.line, ruler.ends]) {
          o.renderOrder = 10;
          o.frustumCulled = false;
          o.raycast = () => {};
          scene.add(o);
        }
        ruler.tag = make("div", { "data-twin-measure": "", "aria-hidden": "true" });
        tagLayer.append(ruler.tag);
      }
      if (!ruler.line) return;
      ruler.line.geometry.setFromPoints(pts);
      ruler.ends.geometry.setFromPoints(pts);
      ruler.line.visible = ruler.ends.visible = pts.length > 0;
      if (to && ruler.a) {
        const d = to.clone().sub(ruler.a), across = Math.hypot(d.x, d.z);
        ruler.tag.replaceChildren(make("span", {}, `${metres(d.length())} m`));
        // Across and up as well when the line slopes.
        if (Math.abs(d.y) >= 0.01 && across >= 0.01) ruler.tag.append(make("small", {}, `↔ ${metres(across)} · ↕ ${metres(Math.abs(d.y))}`));
      }
      ruler.tag.__on = undefined;
      ruler.tag.style.visibility = to && ruler.a ? "" : "hidden";
      look();
    }
    // Moved objects' places and turns, written back to their rows in the same form, at once.
    function writeBack(...its) {
      const key = cfg.bindObjects, rows = api.state[key];
      if (!Array.isArray(rows) || !its.length) return;
      const by = new Map(its.map((it) => [it.id, it]));
      const moved = [];
      const next = rows.map((row, i) => {
        const it = isObj(row) ? by.get(str(row.id ?? i)) : null;
        if (!it) return row;
        const r = it.rec, p = { x: round(r.pos.x), y: round(r.pos.y), z: round(r.pos.z) }, yaw = round((r.rot.y * 180) / Math.PI, 1);
        moved.push({ id: r.row.id ?? it.id, position: [p.x, p.y, p.z], rotation: yaw });
        const o = { ...row };
        if (Array.isArray(row.position)) o.position = row.position.length === 2 ? [p.x, p.z] : [p.x, p.y, p.z];
        else if (isObj(row.position)) o.position = { ...row.position, x: p.x, z: p.z };
        else if ("x" in row || "z" in row) Object.assign(o, { x: p.x, z: p.z });
        else o.position = p.y ? [p.x, p.y, p.z] : [p.x, p.z];
        if (yaw || "rotation" in row || "heading" in row) {
          if (Array.isArray(row.rotation)) o.rotation = [row.rotation[0] ?? 0, yaw, row.rotation[2] ?? 0];
          else if ("heading" in row) o.heading = yaw;
          else o.rotation = yaw;
        }
        return o;
      });
      api.set(key, next);
      for (const m of moved) api.emit("move", m);
    }
    // Each turned a quarter about itself.
    function turnBy(its, dir) {
      its = [].concat(its).filter(Boolean);
      if (!its.length) return;
      for (const it of its) {
        it.rec.rot = { ...it.rec.rot, y: it.rec.rot.y + dir * (Math.PI / 2) };
        place(it);
      }
      writeBack(...its);
      const r = its[0].rec;
      live.textContent = its.length > 1 ? L("Turned {n}", { n: its.length }) : L("Turned to {deg}°", { deg: Math.round((((r.rot.y * 180) / Math.PI) % 360 + 360) % 360) });
      wake();
    }
    function removeObject(...ids) {
      const key = cfg.bindObjects, rows = api.state[key], its = ids.map((id) => items.get(id)).filter(Boolean);
      if (!Array.isArray(rows) || !its.length) return;
      const gone = new Set(its.map((it) => it.id));
      select(null);
      api.set(key, rows.filter((x, i) => !(isObj(x) && gone.has(str(x.id ?? i)))));
      live.textContent = its.length > 1 ? L("Removed {n}", { n: its.length }) : L("Removed {title}", { title: its[0].rec.title || its[0].id });
      for (const it of its) api.emit("remove", { id: it.rec.row.id ?? it.id, object: it.rec.row });
      host.focus({ preventScroll: true });
    }
    // Adds what the palette offers, where it's dropped or in the middle of the view.
    function addObject(entry, at) {
      const key = cfg.bindObjects, rows = Array.isArray(api.state[key]) ? api.state[key] : [];
      const { id: base0, title, ...rest } = entry;
      const base = str(base0 ?? title ?? entry.model ?? entry.shape ?? "object").replace(/[^\w-]+/g, "-").toLowerCase();
      const taken = new Set(rows.map((x, i) => str(isObj(x) ? x.id ?? i : i)));
      let n = 1;
      while (taken.has(`${base}-${n}`)) n++;
      const id = `${base}-${n}`, p = at ?? controls.target;
      const row = { ...rest, id, title: title ? `${title} ${n}` : id, position: [snapTo(p.x), snapTo(p.z)] };
      delete row.label;
      api.set(key, [...rows, row]);
      api.emit("add", { id, object: row });
      live.textContent = L("Added {title}", { title: row.title });
      // Selected as soon as it's drawn, and the view has the focus, so the keys move,
      // turn or remove what's just been added.
      host.focus({ preventScroll: true });
      if (items.has(id)) select(id, { popup: true });
      else adding = id;
    }
    // The ruler's button is there while `measure` is.
    function drawRuler() {
      for (const b of el.querySelectorAll("[data-twin-action=measure]")) b.hidden = !props.measure;
      if (!props.measure && ruler.on) measureMode(false);
    }
    function drawPalette() {
      if (!paletteBox) return;
      const entries = editable() && Array.isArray(props.palette) ? props.palette.filter(isObj) : [];
      paletteBox.hidden = !entries.length;
      const sig = JSON.stringify(entries);
      if (paletteBox.__sig === sig) return;
      paletteBox.__sig = sig;
      paletteBox.replaceChildren(...entries.map((entry) => {
        const b = make("button", { type: "button", "data-twin-add": "", title: L("Click to add, or drag into the view") }, str(entry.label ?? entry.title ?? entry.model ?? entry.shape));
        let from = null;
        b.addEventListener("pointerdown", (e) => {
          from = { x: e.clientX, y: e.clientY, id: e.pointerId };
          b.setPointerCapture(e.pointerId);
        });
        b.addEventListener("pointerup", (e) => {
          if (!from || e.pointerId !== from.id) return;
          const moved = Math.hypot(e.clientX - from.x, e.clientY - from.y) > 8;
          from = null;
          if (!moved) return;
          const r = host.getBoundingClientRect();
          if (e.clientX < r.left || e.clientX > r.right || e.clientY < r.top || e.clientY > r.bottom) return;
          b.__dropped = true;
          addObject(entry, groundPoint(e));
        });
        b.addEventListener("click", () => {
          if (b.__dropped) return (b.__dropped = false);
          addObject(entry, null);
        });
        return b;
      }));
    }
    function keys(e) {
      if (!renderer || e.target.closest?.("[data-twin-views], [data-twin-controls], [data-twin-panel], [data-twin-palette]")) return;
      if (e.target.closest?.("[data-twin-popup]") && e.key !== "Escape") return;
      if (e.key === "Escape" && measuring()) {
        e.preventDefault();
        e.stopPropagation();
        return measureMode(false);
      }
      if (e.key === "Escape" && (popup || selected !== null)) {
        e.preventDefault();
        e.stopPropagation();
        if (popup) close(true);
        else select(null);
        return;
      }
      const it = selected !== null ? items.get(selected) : null;
      // Moving and turning the picked objects (the selected one, or all of them).
      const all = [...picked].map((id) => items.get(id)).filter((x) => x && x.rec.path === null && x.rec.attach === null);
      if (it && editable() && e.altKey && all.length) {
        const s = fin(props.snap) && num(props.snap) > 0 ? num(props.snap) : 0.5;
        const d = { ArrowLeft: [-s, 0], ArrowRight: [s, 0], ArrowUp: [0, -s], ArrowDown: [0, s] }[e.key];
        if (d) {
          e.preventDefault();
          for (const x of all) {
            x.rec.pos = { ...x.rec.pos, x: snapTo(x.rec.pos.x + d[0]), z: snapTo(x.rec.pos.z + d[1]) };
            place(x);
          }
          writeBack(...all);
          const r = (all.includes(it) ? it : all[0]).rec;
          live.textContent = all.length > 1 ? L("Moved {n}", { n: all.length }) : L("Moved to {x}, {z}", { x: round(r.pos.x, 2), z: round(r.pos.z, 2) });
          return wake();
        }
        if (e.code === "KeyR") {
          e.preventDefault();
          return turnBy(all, e.shiftKey ? -1 : 1);
        }
      }
      if (it && editable() && (e.key === "Delete" || e.key === "Backspace") && !e.target.closest?.("input, textarea, [contenteditable]")) {
        e.preventDefault();
        return removeObject(...picked);
      }
      if (e.target !== host && !e.target.closest?.("[data-twin-list]")) return;
      if (props.measure && (e.key === "m" || e.key === "M") && !e.metaKey && !e.ctrlKey && !e.altKey) {
        e.preventDefault();
        return measureMode(!ruler.on);
      }
      if (multi() && (e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "a") {
        e.preventDefault();
        return pickMany([...items.values()].filter((x) => x.rec.clickable).map((x) => x.id), { popup: false });
      }
      const off = camera.position.clone().sub(controls.target);
      const sph = new T.Spherical().setFromVector3(off);
      const pan = (dx, dz) => {
        const right = new T.Vector3().subVectors(camera.position, controls.target).cross(camera.up).setY(0).normalize().negate();
        const fwd = new T.Vector3().subVectors(controls.target, camera.position).setY(0).normalize();
        const m = right.multiplyScalar(dx).add(fwd.multiplyScalar(dz)).multiplyScalar(sph.radius * 0.08);
        camera.position.add(m);
        controls.target.add(m);
      };
      const k = e.key;
      if (e.shiftKey && k.startsWith("Arrow")) pan(k === "ArrowLeft" ? -1 : k === "ArrowRight" ? 1 : 0, k === "ArrowUp" ? 1 : k === "ArrowDown" ? -1 : 0);
      else if (k === "ArrowLeft" || k === "ArrowRight") sph.theta += (k === "ArrowLeft" ? -1 : 1) * deg(10);
      else if (k === "ArrowUp" || k === "ArrowDown") sph.phi = Math.min(controls.maxPolarAngle, Math.max(0.05, sph.phi + (k === "ArrowUp" ? -1 : 1) * deg(6)));
      else if (k === "+" || k === "=") sph.radius = Math.max(controls.minDistance, sph.radius * 0.85);
      else if (k === "-" || k === "_") sph.radius = Math.min(controls.maxDistance, sph.radius / 0.85);
      else if (k === "Home" || k === "0") return (e.preventDefault(), fit());
      else return;
      e.preventDefault();
      if (!(e.shiftKey && k.startsWith("Arrow"))) camera.position.copy(controls.target).add(new T.Vector3().setFromSpherical(sph));
      controls.update();
      wake();
    }
    el.addEventListener("keydown", keys);
    el.addEventListener("click", (e) => {
      const b = e.target.closest?.("[data-twin-action]");
      if (!b || !renderer) return;
      const a = b.dataset.twinAction, off = camera.position.clone().sub(controls.target);
      if (a === "measure") measureMode(!ruler.on);
      else if (a === "fit") fit();
      else if (a === "zoom-in" || a === "zoom-out") {
        off.multiplyScalar(a === "zoom-in" ? 0.75 : 1 / 0.75);
        off.clampLength(controls.minDistance, controls.maxDistance);
        fly(controls.target.clone().add(off).toArray(), controls.target.toArray());
      }
    });

    // ---------------------------------------------------------------- live data
    let poll = 0, source = null, liveSig = null, liveFailed = false;
    function take(data) {
      const rows = Array.isArray(data) ? data : Array.isArray(data?.objects) ? data.objects : null;
      const entries = rows ? rows.filter(isObj).map((r) => [str(r.id), r]) : isObj(data) ? Object.entries(data).filter(([, v]) => isObj(v)) : [];
      for (const [id, v] of entries) live$.set(id, { ...(live$.get(id) ?? {}), ...v });
      set("bindLive", Object.fromEntries(live$));
      el.removeAttribute("data-stale");
      liveFailed = false;
      api.emit("data", { count: entries.length });
      if (renderer) sync();
    }
    function feed() {
      const l = isObj(props.live) ? props.live : typeof props.live === "string" ? { url: props.live } : null;
      const sig = JSON.stringify(l);
      if (sig === liveSig) return;
      liveSig = sig;
      clearInterval(poll);
      source?.close();
      source = null;
      if (!l) return;
      const lost = (e) => {
        el.setAttribute("data-stale", "");
        if (!liveFailed) api.emit("failed", { live: true, error: String(e?.message ?? e) });
        liveFailed = true;
      };
      if (l.stream && allowed(l.stream) && window.EventSource) {
        source = new EventSource(l.stream);
        source.onmessage = (m) => {
          try {
            take(JSON.parse(m.data));
          } catch (e) {
            lost(e);
          }
        };
        source.onerror = () => lost(new Error("stream lost"));
      }
      if (l.url && allowed(l.url)) {
        const get = () => fetch(l.url, { headers: { accept: "application/json" } }).then((r) => (r.ok ? r.json() : Promise.reject(new Error(`HTTP ${r.status}`)))).then(take, lost);
        get();
        poll = setInterval(get, Math.max(500, fin(l.every) ? num(l.every) : 5000));
      }
    }
    let srcSig = null;
    function load() {
      const u = typeof props.src === "string" ? props.src : null;
      if (u === srcSig) return;
      srcSig = u;
      if (!u) return (fetched = null);
      if (!allowed(u)) return failed(new Error(`world address not allowed: ${u}`));
      fetch(u, { headers: { accept: "application/json" } })
        .then((r) => (r.ok ? r.json() : Promise.reject(new Error(`HTTP ${r.status} for ${u}`))))
        .then((w) => {
          if (srcSig !== u) return;
          fetched = w;
          if (renderer) rebuild();
        }, (e) => failed(e));
    }

    // ---------------------------------------------------------------- starting
    let hover = null;
    // Once: it's called both when the page opens in view and when it comes into view, and
    // the renderer only exists once three.js has loaded.
    let started = false;
    async function start() {
      if (started || renderer || stopped) return;
      started = true;
      say(L("Loading 3D view…"), true);
      el.setAttribute("data-loading", "");
      try {
        T = await lib(src.three);
        const [{ OrbitControls }, { RoomEnvironment }] = await Promise.all([addon("controls/OrbitControls.js"), addon("environments/RoomEnvironment.js")]);
        try {
          renderer = new T.WebGLRenderer({ antialias: cfg.quality !== "low", alpha: true, powerPreference: "high-performance" });
        } catch {
          throw new Error(L("This browser can't draw 3D (WebGL is off or unavailable)."));
        }
        renderer.setPixelRatio(sharp());
        renderer.outputColorSpace = T.SRGBColorSpace;
        renderer.toneMapping = T.NeutralToneMapping;
        renderer.shadowMap.enabled = cfg.quality !== "low";
        renderer.shadowMap.autoUpdate = false;
        renderer.shadowMap.type = T.PCFShadowMap;
        renderer.setClearColor(0x000000, 0);
        renderer.domElement.setAttribute("aria-hidden", "true");
        host.replaceChildren(renderer.domElement);
        scene = new T.Scene();
        const pm = new T.PMREMGenerator(renderer);
        scene.environment = pm.fromScene(new RoomEnvironment(), 0.04).texture;
        pm.dispose();
        camera = new T.PerspectiveCamera(45, 1, 0.05, 5000);
        hemi = new T.HemisphereLight(0xffffff, 0x8a8a8a, 1);
        sun = new T.DirectionalLight(0xffffff, 1.8);
        sun.shadow.mapSize.set(2048, 2048);
        sun.shadow.bias = -0.0004;
        sun.shadow.normalBias = 0.02;
        scene.add(hemi, sun, sun.target);
        world = new T.Group();
        scene.add(world);
        ringOf = (o) => {
          const m = new T.Mesh(new T.RingGeometry(0.88, 1, 64), new T.MeshBasicMaterial({ transparent: true, opacity: o, depthWrite: false, side: T.DoubleSide }));
          m.rotation.x = -Math.PI / 2;
          m.renderOrder = 3;
          m.visible = false;
          m.raycast = () => {};
          scene.add(m);
          return m;
        };
        controls = new OrbitControls(camera, renderer.domElement);
        controls.enableDamping = !calm();
        controls.dampingFactor = 0.12;
        controls.maxPolarAngle = deg(88);
        controls.minDistance = 0.5;
        controls.maxDistance = 2000;
        controls.screenSpacePanning = false;
        controls.enableZoom = cfg.gestures !== "none";
        controls.enabled = cfg.gestures !== "none";
        controls.addEventListener("change", look);
        controls.addEventListener("end", () => set("bindCamera", view()));
        themed();
        hover = ringOf(0.6);
        hover.material.color.set(colors.primary);
        resize();
        rebuild(true);
        bindPointer();
        twins.add(new WeakRef(self));
        watch();
        el.removeAttribute("data-loading");
        if (!loading) say("");
        const at = cfg.bind ? api.state[cfg.bind] : null;
        if (at !== null && at !== undefined) select(at, { quiet: true, popup: false });
        const many = cfg.bindSelected ? api.state[cfg.bindSelected] : null;
        if (Array.isArray(many) && many.length) pickMany(many, { quiet: true, bind: true, popup: false });
      } catch (e) {
        failed(e instanceof Error ? e : new Error(String(e)));
      }
    }
    function rebuild(first) {
      drawWalls();
      drawZones();
      drawPaths();
      sync();
      drawGround();
      drawViews();
      drawPalette();
      drawRuler();
      if (first) cameraStart();
      wake();
    }
    function resize() {
      if (!renderer) return;
      const w = Math.max(1, host.clientWidth), h = Math.max(1, host.clientHeight);
      renderer.setSize(w, h, false);
      renderer.domElement.style.width = "100%";
      renderer.domElement.style.height = "100%";
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
      wake();
    }
    new ResizeObserver(() => nextFrame(resize)).observe(host);
    const near = 300;
    const io = new IntersectionObserver((es) => {
      inView = es.some((e) => e.isIntersecting);
      if (inView) {
        if (!renderer) start();
        wake();
      }
    }, { rootMargin: `${near}px` });
    setTimeout(() => {
      const r = el.getBoundingClientRect();
      if (r.width && r.top < innerHeight + near && r.bottom > -near) {
        inView = true;
        start();
      }
      io.observe(el);
    });
    document.addEventListener("visibilitychange", () => !document.hidden && wake());
    load();
    feed();

    let lastProps = JSON.stringify(props), lastBind = cfg.bind ? api.state[cfg.bind] ?? null : null, lastView = cfg.bindView ? api.state[cfg.bindView] ?? null : null;
    const self = {
      themed: () => T && themed(),
      get three() {
        return T;
      },
      get scene() {
        return scene;
      },
      get camera() {
        return camera;
      },
      get controls() {
        return controls;
      },
      // What the last frame drew: draw calls, triangles, programs, textures.
      get info() {
        return renderer ? { ms: Math.round(frameMs * 100) / 100, pixelRatio: renderer.getPixelRatio(), calls: renderer.info.render.calls, triangles: renderer.info.render.triangles, programs: renderer.info.programs?.length ?? 0, textures: renderer.info.memory.textures, geometries: renderer.info.memory.geometries } : null;
      },
      items,
      // Where the middle of an object is drawn in the view, in px from its top left.
      at(id) {
        const it = items.get(str(id));
        return it && camera ? onScreen(it) : null;
      },
      select: (id) => select(id, { popup: true, fly: true }),
      // Several (`multiple`): their ids, the last the selected one; and which are picked.
      selectMany: (ids) => pickMany(ids, { popup: false }),
      get selection() {
        return [...picked];
      },
      fit: () => fit(true),
      // The distance between two objects (ids) or points (`[x, z]`, `[x, y, z]`), drawn;
      // `measure(null)` clears it.
      measure(a, b) {
        if (!camera) return null;
        const p = (v) => (isObj(v) || Array.isArray(v) ? ((v = vec(v)) && new T.Vector3(v.x, v.y, v.z)) : floorOf(v));
        return a === null || a === undefined ? measure(null) : measure(p(a), p(b));
      },
      get measuring() {
        return measuring();
      },
      update() {
        if (cfg.reactive) read();
        load();
        feed();
        if (!renderer) return;
        const sig = JSON.stringify(props);
        if (sig !== lastProps) {
          const before = JSON.parse(lastProps);
          lastProps = sig;
          const changed = (...ks) => ks.some((k) => !same(before[k], props[k]));
          if (changed("world", "walls")) drawWalls();
          if (changed("world", "zones")) drawZones();
          if (changed("world", "paths")) drawPaths();
          sync();
          if (changed("world", "ground", "walls", "zones", "environment")) drawGround();
          if (changed("world", "views")) drawViews();
          if (changed("palette", "editable")) drawPalette();
          if (changed("measure")) drawRuler();
          if (changed("camera") && isObj(props.camera) && vec(props.camera.position)) fly(props.camera.position, props.camera.target);
        }
        if (cfg.bind) {
          const now = api.state[cfg.bind] ?? null;
          if (!same(now, lastBind)) {
            lastBind = now;
            if (str(now) !== str(selected)) select(now, { quiet: true, fly: true, popup: now !== null });
          }
        }
        if (cfg.bindSelected) {
          const now = api.state[cfg.bindSelected];
          if (Array.isArray(now) && JSON.stringify(now) !== lastPicked) pickMany(now, { quiet: true, bind: true, popup: false });
        }
        if (cfg.bindView) {
          const now = api.state[cfg.bindView] ?? null;
          if (!same(now, lastView)) {
            lastView = now;
            if (now !== null) goToView(now);
          }
        }
        wake();
      },
    };
    api.self = self;
    el.__twin = self;
    return self;
  };
})();
