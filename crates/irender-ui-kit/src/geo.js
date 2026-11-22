// irender-ui-kit: `ui-location`, `ui-geo-checkin`, `ui-route-tracker`, `ui-compass` and
// `ui-level`, and the sums they share with the map (`window.__irenderGeo`).
(() => {
  const B = (window.__irenderBehaviors ||= {});
  const K = window.__irenderKit, D = window.__irenderDevice, L = (t, v) => window.__irender.label(t, v);
  const { make, icon, isObj, blank } = K, btn = D.btn;
  const R = 6371008.8, rad = Math.PI / 180;
  const list = (v) => (Array.isArray(v) ? v : []);
  // `{ lat, lng }` from an object or a pair, or null.
  function point(p) {
    const lat = Array.isArray(p) ? Number(p[0]) : blank(p?.lat) ? NaN : Number(p.lat), lng = Array.isArray(p) ? Number(p[1]) : blank(p?.lng ?? p?.lon) ? NaN : Number(p.lng ?? p.lon);
    return Number.isFinite(lat) && Number.isFinite(lng) && Math.abs(lat) <= 90 && Math.abs(lng) <= 180 ? { lat, lng } : null;
  }
  function distance(a, b) {
    const x = Math.sin(((b.lat - a.lat) * rad) / 2) ** 2 + Math.cos(a.lat * rad) * Math.cos(b.lat * rad) * Math.sin(((b.lng - a.lng) * rad) / 2) ** 2;
    return 2 * R * Math.asin(Math.min(1, Math.sqrt(x)));
  }
  function bearing(a, b) {
    const d = (b.lng - a.lng) * rad, y = Math.sin(d) * Math.cos(b.lat * rad), x = Math.cos(a.lat * rad) * Math.sin(b.lat * rad) - Math.sin(a.lat * rad) * Math.cos(b.lat * rad) * Math.cos(d);
    return (Math.atan2(y, x) / rad + 360) % 360;
  }
  function inside(p, points) {
    let is = false;
    const q = list(points).map(point).filter(Boolean);
    for (let i = 0, j = q.length - 1; i < q.length; j = i++) if (q[i].lat > p.lat !== q[j].lat > p.lat && p.lng < ((q[j].lng - q[i].lng) * (p.lat - q[i].lat)) / (q[j].lat - q[i].lat) + q[i].lng) is = !is;
    return is;
  }
  // Metres from a point to the nearest edge of an area.
  function edge(p, points) {
    const q = list(points).map(point).filter(Boolean), kx = Math.cos(p.lat * rad) * rad * R, ky = rad * R;
    let best = Infinity;
    for (let i = 0, j = q.length - 1; i < q.length; j = i++) {
      const ax = (q[j].lng - p.lng) * kx, ay = (q[j].lat - p.lat) * ky, dx = (q[i].lng - p.lng) * kx - ax, dy = (q[i].lat - p.lat) * ky - ay;
      const t = K.clamp(-(ax * dx + ay * dy) / (dx * dx + dy * dy || 1), 0, 1);
      best = Math.min(best, Math.hypot(ax + t * dx, ay + t * dy));
    }
    return best;
  }
  const lang = () => document.documentElement.lang || undefined;
  const num = (n, d) => new Intl.NumberFormat(lang(), { maximumFractionDigits: d }).format(n);
  const far = (m, imperial) => imperial ? (m < 160 ? `${num(m * 3.28084, 0)} ft` : `${num(m / 1609.344, m < 16093 ? 2 : 1)} mi`) : m < 1000 ? `${num(m, 0)} m` : `${num(m / 1000, m < 10000 ? 2 : 1)} km`;
  const clock = (s) => `${s >= 3600 ? `${Math.floor(s / 3600)}:` : ""}${String(Math.floor((s % 3600) / 60)).padStart(s >= 3600 ? 2 : 1, "0")}:${String(Math.floor(s % 60)).padStart(2, "0")}`;
  const read = (p) => {
    const c = p.coords, o = { lat: +c.latitude.toFixed(6), lng: +c.longitude.toFixed(6), accuracy: Math.round(c.accuracy), at: new Date(p.timestamp || Date.now()).toISOString() };
    for (const k of ["altitude", "heading", "speed"]) o[k] = Number.isFinite(c[k]) ? +c[k].toFixed(1) : null;
    return o;
  };
  // Where the device is, as good as `enough` metres if that's found within a while.
  function find({ precise = true, enough, wait = 20000 } = {}) {
    return new Promise((ok, no) => {
      if (!navigator.geolocation) return no(Object.assign(new Error(), { name: "NotSupportedError" }));
      const o = { enableHighAccuracy: precise, timeout: wait, maximumAge: 5000 };
      if (!enough) return navigator.geolocation.getCurrentPosition((p) => ok(read(p)), no, o);
      let best = null;
      const end = (e) => {
        navigator.geolocation.clearWatch(id);
        clearTimeout(t);
        best ? ok(best) : no(e);
      };
      const id = navigator.geolocation.watchPosition((p) => {
        const r = read(p);
        if (!best || r.accuracy <= best.accuracy) best = r;
        if (best.accuracy <= enough) end();
      }, (e) => (e.code === 1 || !best) && end(e), o);
      const t = setTimeout(() => end(Object.assign(new Error(), { code: 3 })), wait);
    });
  }
  window.__irenderGeo = { point, distance, bearing, inside, edge, far, clock, find, read };

  // ---------------------------------------------------------------- location
  B["ui-location"] = (el, api) => {
    const cfg = api.config, body = D.body(el);
    let busy = false, error = null, shut = false, watch = null, was = null;
    function set(p) {
      api.set(cfg.bind, p);
      api.emit("locate", p);
    }
    async function go() {
      if (watch !== null) return stop(), draw();
      error = null;
      if (cfg.watch) {
        if (!navigator.geolocation) return (error = D.unsupported("geolocation")), draw();
        watch = navigator.geolocation.watchPosition((p) => (el.isConnected ? ((error = null), set(read(p))) : stop()), (e) => {
          error = D.explain(e, "geolocation");
          shut = D.blocked(e);
          if (e.code === 1) stop();
          draw();
        }, { enableHighAccuracy: cfg.precise, maximumAge: 2000 });
        return draw();
      }
      busy = true;
      draw();
      try {
        set(await find(cfg));
        K.say(el, L("Location found"));
      } catch (e) {
        error = D.explain(e, "geolocation");
        shut = D.blocked(e);
        api.emit("failed", { error });
      }
      busy = false;
      draw();
    }
    const stop = () => {
      if (watch !== null) navigator.geolocation.clearWatch(watch);
      watch = null;
    };
    const link = (p) => {
      if (cfg.link === null) return null;
      const u = String(cfg.link ?? "https://www.openstreetmap.org/?mlat={lat}&mlon={lng}#map=17/{lat}/{lng}").replaceAll("{lat}", p.lat).replaceAll("{lng}", p.lng);
      return D.safeUrl(u) ? make("a", { href: u, target: "_blank", rel: "noreferrer noopener" }, L("Open in maps")) : null;
    };
    const fields = cfg.manual ? ["lat", "lng"].map((k) => make("input", { "data-w-field": "", type: "number", step: "any", min: k === "lat" ? -90 : -180, max: k === "lat" ? 90 : 180, placeholder: k === "lat" ? L("Latitude") : L("Longitude"), "aria-label": k === "lat" ? L("Latitude") : L("Longitude"), change: typed })) : [];
    if (fields.length) el.append(make("div", { "data-w-row": "" }, fields));
    function typed() {
      const p = point({ lat: fields[0].value, lng: fields[1].value });
      if (p) set({ ...p, accuracy: null, altitude: null, heading: null, speed: null, at: new Date().toISOString(), typed: true });
    }
    function draw() {
      const p = point(api.state[cfg.bind]) ? api.state[cfg.bind] : null, label = K.value(api, cfg.label);
      const sig = JSON.stringify([p, busy, error, watch !== null, label]);
      if (sig === was) return;
      was = sig;
      if (p && !fields.includes(document.activeElement)) fields.forEach((f, i) => (f.value = i ? p.lng : p.lat));
      const focus = body.contains(document.activeElement);
      const b = btn(p ? "" : "primary", { "data-locate": "", disabled: busy, click: go }, icon(watch !== null ? "stop" : "pin", 16), busy ? L("Finding it…") : watch !== null ? L("Stop following") : label ?? (p ? L("Update") : cfg.watch ? L("Follow my location") : L("Use my location")));
      const poor = p && cfg.enough && p.accuracy > cfg.enough;
      body.replaceChildren(
        p ? make("div", { "data-w-row": "" }, make("span", { "data-geo-at": "" }, `${num(p.lat, 5)}, ${num(p.lng, 5)}`), blank(p.accuracy) ? null : make("span", { "data-w-tag": "", "data-tone": p.accuracy <= 25 ? "good" : poor || p.accuracy > 100 ? "warn" : "info" }, `± ${far(p.accuracy)}`), make("span", { "data-w-note": "" }, K.ago(p.at)), link(p)) : make("span", { hidden: true }),
        make("div", { "data-w-row": "" }, b, error && shut ? btn("ghost", { click: () => D.help(el, "geolocation") }, L("How to fix")) : null),
        make("p", { "data-w-error": "", role: "alert" }, error ?? ""));
      if (focus) body.querySelector("button")?.focus();
    }
    D.gone(el, stop);
    draw();
    if (cfg.auto) D.permission("geolocation").then((s) => s === "granted" && go());
    return { update: draw };
  };

  // ---------------------------------------------------------------- check-in
  B["ui-geo-checkin"] = (el, api) => {
    const cfg = api.config, body = make("div", { "data-ci-body": "" }), enough = cfg.enough ?? 100;
    el.append(body);
    let here = null, busy = false, error = null, shut = false, was = null, reason = "", asking = false;
    const places = () => list(K.value(api, cfg.places)).filter(isObj);
    // How far from a place: 0 inside it.
    const away = (place, p) => {
      if (Array.isArray(place.points)) return inside(p, place.points) ? 0 : edge(p, place.points);
      const c = point(place);
      return c ? Math.max(0, distance(c, p) - (Number(place.radius) || 100)) : Infinity;
    };
    function nearest(p) {
      return places().map((place) => ({ place, d: away(place, p) })).sort((a, b) => a.d - b.d)[0] ?? null;
    }
    async function look() {
      busy = true;
      error = null;
      draw();
      try {
        here = await find({ enough: Math.min(enough, 30), wait: 12000 });
      } catch (e) {
        error = D.explain(e, "geolocation");
        shut = D.blocked(e);
      }
      busy = false;
      draw();
      return !error;
    }
    const open = () => (isObj(api.state[cfg.bind]) ? api.state[cfg.bind] : null);
    async function go() {
      if (!(await look())) return;
      const out = open(), near = out ? { place: places().find((x) => x.id === out.place?.id) ?? out.place, d: 0 } : nearest(here);
      if (!near) return (error = L("There's nowhere to check in")), draw();
      if (out) near.d = point(near.place) || near.place?.points ? away(near.place, here) : 0;
      if (here.accuracy > enough) return (error = L("The location is only good to {far}; move outside, or wait a little, and try again", { far: far(here.accuracy) })), api.emit("refused", { reason: "accuracy", accuracy: here.accuracy }), draw();
      const within = near.d === 0;
      if (!within && !out && (cfg.outside ?? "block") === "block") return (error = L("You're {far} from {place}. Come closer to check in.", { far: far(near.d), place: near.place.name ?? "" })), api.emit("refused", { reason: "away", distance: Math.round(near.d) }), draw();
      if (!within && cfg.outside === "reason" && !reason.trim()) return (asking = true), draw(), body.querySelector("input")?.focus();
      const mark = { at: new Date().toISOString(), lat: here.lat, lng: here.lng, accuracy: here.accuracy, distance: Math.round(near.d), inside: within };
      if (reason.trim()) mark.reason = reason.trim();
      const photo = K.value(api, cfg.photo);
      if (!blank(photo)) mark.photo = photo;
      reason = "";
      asking = false;
      if (out) {
        const done = { ...out, out: mark, seconds: Math.round((Date.parse(mark.at) - Date.parse(out.in.at)) / 1000) };
        api.set(cfg.bind, null);
        if (cfg.bindLog) api.set(cfg.bindLog, [...list(api.state[cfg.bindLog]), done]);
        api.emit("checkout", done);
        K.say(el, L("Checked out"));
      } else {
        const { id, name } = near.place, visit = { id: K.id(), place: { id: id ?? null, name: name ?? null }, in: mark };
        api.set(cfg.bind, visit);
        api.emit("checkin", visit);
        K.say(el, L("Checked in"));
      }
      draw();
    }
    function draw() {
      const v = open(), near = here && !v ? nearest(here) : null, can = !cfg.gated || !!K.value(api, cfg.ready);
      const inL = K.value(api, cfg.inLabel) ?? L("Check in"), outL = K.value(api, cfg.outLabel) ?? L("Check out");
      const sig = JSON.stringify([v?.id, here, busy, error, asking, can, near?.d, near?.place?.name, inL, outL]);
      if (sig === was) return tick();
      was = sig;
      const why = asking ? make("input", { "data-w-field": "", type: "text", placeholder: L("Why are you somewhere else?"), "aria-label": L("Why are you somewhere else?"), value: reason, input: (e) => (reason = e.target.value), keydown: (e) => e.key === "Enter" && go() }) : null;
      body.replaceChildren(
        v ? make("div", null, make("div", { "data-w-row": "" }, make("span", { "data-w-tag": "", "data-tone": v.in.inside === false ? "warn" : "good" }, icon("pin", 13), v.place?.name ?? L("Checked in")), make("span", { "data-w-note": "" }, L("since {time}", { time: new Date(v.in.at).toLocaleTimeString(lang(), { hour: "numeric", minute: "2-digit" }) }))), make("div", { "data-ci-since": "", role: "timer" }))
          : near ? make("div", { "data-w-row": "" }, make("span", { "data-w-tag": "", "data-tone": near.d === 0 ? "good" : "warn" }, icon("pin", 13), near.d === 0 ? L("At {place}", { place: near.place.name ?? "" }) : L("{far} from {place}", { far: far(near.d), place: near.place.name ?? "" })), make("span", { "data-w-note": "" }, `± ${far(here.accuracy)}`)) : make("p", { "data-w-note": "" }, L("Your location is looked up when you check in.")),
        ...(why ? [why] : []),
        make("div", { "data-w-row": "" }, btn(v ? "" : "primary", { "data-check": v ? "out" : "in", disabled: busy || !can, click: go }, icon(v ? "stop" : "check", 16), busy ? L("Finding where you are…") : v ? outL : inL), v ? null : btn("ghost", { disabled: busy, click: look }, icon("refresh", 15), L("Where am I?")), error && shut ? btn("ghost", { click: () => D.help(el, "geolocation") }, L("How to fix")) : null),
        make("p", { "data-w-error": "", role: "alert" }, error ?? ""));
      tick();
    }
    function tick() {
      const t = body.querySelector("[data-ci-since]"), v = open();
      if (t && v) t.textContent = clock(Math.max(0, (Date.now() - Date.parse(v.in.at)) / 1000));
    }
    const t = setInterval(() => (el.isConnected ? tick() : clearInterval(t)), 1000);
    draw();
    return { update: draw };
  };

  // ---------------------------------------------------------------- route
  B["ui-route-tracker"] = (el, api) => {
    const cfg = api.config, body = D.body(el), step = cfg.step ?? 10, enough = cfg.enough ?? 50, imperial = cfg.units === "imperial";
    let watch = null, error = null, was = null, lock = null, began = null, fresh = false;
    const IDLE = { state: "idle", points: [], distance: 0, seconds: 0, started: null, ended: null };
    const route = () => (isObj(api.state[cfg.bind]) ? { ...IDLE, ...api.state[cfg.bind] } : IDLE);
    const put = (v) => api.set(cfg.bind, { ...route(), ...v });
    const seconds = () => route().seconds + (began ? (Date.now() - began) / 1000 : 0);
    const bank = () => {
      const s = Math.round(seconds());
      began = null;
      return s;
    };
    async function awake(on) {
      try {
        if (on && cfg.awake) lock = await navigator.wakeLock?.request("screen");
        else lock?.release(), (lock = null);
      } catch { /* the screen goes off as it does */ }
    }
    function got(p) {
      const r = read(p), pts = route().points, prev = pts.at(-1);
      error = null;
      if (r.accuracy > enough) return draw();
      // What was covered while paused isn't part of it.
      const d = prev && !fresh ? distance({ lat: prev[0], lng: prev[1] }, r) : 0;
      // Standing still, a position wanders by about its accuracy.
      if (prev && !fresh && d < Math.max(step, Math.min(r.accuracy / 2, 25))) return draw();
      fresh = false;
      put({ points: [...pts, [r.lat, r.lng, r.at, r.accuracy]], distance: Math.round(route().distance + d) });
      api.emit("point", r);
    }
    function start() {
      error = null;
      if (!navigator.geolocation) return (error = D.unsupported("geolocation")), draw();
      const r = route();
      put(r.state === "paused" ? { state: "recording" } : { ...IDLE, state: "recording", started: new Date().toISOString() });
      if (watch === null) follow();
      api.emit("start", {});
    }
    function follow() {
      began = Date.now();
      fresh = true;
      watch = navigator.geolocation.watchPosition(got, (e) => {
        error = D.explain(e, "geolocation");
        if (e.code === 1) pause();
        draw();
      }, { enableHighAccuracy: true, maximumAge: 1000 });
      awake(true);
    }
    const halt = () => {
      if (watch !== null) navigator.geolocation.clearWatch(watch);
      watch = null;
      awake(false);
    };
    function pause() {
      halt();
      put({ state: "paused", seconds: bank() });
    }
    function finish() {
      halt();
      put({ state: "done", seconds: bank(), ended: new Date().toISOString() });
      api.emit("finish", route());
    }
    function save() {
      const esc = (s) => String(s).replace(/[&<>"]/g, (c) => `&#${c.charCodeAt(0)};`), r = route();
      const gpx = `<?xml version="1.0" encoding="UTF-8"?>\n<gpx version="1.1" creator="iRender" xmlns="http://www.topografix.com/GPX/1/1"><trk><name>${esc(r.started ?? "")}</name><trkseg>\n${r.points.map((p) => `<trkpt lat="${Number(p[0])}" lon="${Number(p[1])}"><time>${esc(p[2])}</time></trkpt>`).join("\n")}\n</trkseg></trk></gpx>\n`;
      window.__irenderSheets.save(cfg.download, new Blob([gpx], { type: "application/gpx+xml" }));
    }
    function draw() {
      const r = route(), s = seconds();
      const sig = JSON.stringify([r.state, error, r.points.length, r.distance]);
      const times = [clock(s), far(r.distance, imperial), s > 5 && r.distance ? `${num((r.distance / s) * (imperial ? 2.23694 : 3.6), 1)} ${imperial ? "mph" : "km/h"}` : "–"];
      if (sig !== was) {
        was = sig;
        const focus = body.contains(document.activeElement);
        const on = r.state === "recording";
        body.replaceChildren(
          make("dl", null, [[L("Time"), 0], [L("Distance"), 1], [L("Average"), 2]].map(([t, i]) => make("div", null, make("dt", null, t), make("dd", { "data-v": i })))),
          make("div", { "data-w-row": "" },
            on ? btn("", { "data-pause": "", click: pause }, icon("pause", 15), L("Pause")) : btn("primary", { "data-start": "", click: start }, icon("play", 15), r.state === "paused" ? L("Go on") : r.state === "done" ? L("Start a new one") : L("Start")),
            on || r.state === "paused" ? btn("", { "data-finish": "", click: finish }, icon("stop", 15), L("Finish")) : null,
            cfg.download && r.points.length ? btn("ghost", { click: save }, icon("download", 15), L("Save as GPX")) : null,
            on ? make("span", { "data-w-tag": "", "data-tone": "bad" }, L("Recording")) : null),
          make("p", { "data-w-note": "" }, on ? L("Keep the app open and in front: it stops when the phone sleeps.") : ""),
          make("p", { "data-w-error": "", role: "alert" }, error ?? ""));
        if (focus) body.querySelector("button")?.focus();
      }
      body.querySelectorAll("[data-v]").forEach((d, i) => (d.textContent = times[i]));
    }
    // Recording when the page was closed: it goes on.
    if (route().state === "recording" && navigator.geolocation) follow();
    document.addEventListener("visibilitychange", () => document.visibilityState === "visible" && watch !== null && awake(true));
    const t = setInterval(() => (el.isConnected ? began && draw() : clearInterval(t)), 1000);
    // What's been timed isn't lost with the page.
    addEventListener("pagehide", () => began && put({ seconds: bank() }));
    D.gone(el, halt);
    draw();
    return {
      update() {
        // Put back by a store after it had started.
        if (route().state === "recording" && watch === null && navigator.geolocation) follow();
        draw();
      },
    };
  };

  // ---------------------------------------------------------------- compass and level
  const NS = "http://www.w3.org/2000/svg";
  const svg = (tag, attrs, ...kids) => {
    const e = document.createElementNS(NS, tag);
    for (const [k, v] of Object.entries(attrs ?? {})) if (v !== null && v !== undefined && v !== false) e.setAttribute(k, v === true ? "" : v);
    e.append(...kids.flat().filter((k) => k !== null && k !== undefined));
    return e;
  };
  // The sensors, once they're allowed (an iPhone asks, after a press).
  function sensors(el, event, on, silent) {
    let heard = false, listening = false;
    const start = btn("primary", { "data-start": "", click: go }, icon("compass", 16), L("Turn on the sensors")), note = make("p", { "data-w-note": "", role: "status" });
    const hear = (e) => {
      if (!el.isConnected) return stop();
      if (on(e) === false) return;
      heard = true;
      note.textContent = "";
    };
    function listen() {
      if (listening) return;
      listening = true;
      event.forEach((n) => addEventListener(n, hear));
      setTimeout(() => !heard && el.isConnected && (note.textContent = L("This device has no such sensor, or it isn't saying.")), 2500);
    }
    const stop = () => event.forEach((n) => removeEventListener(n, hear));
    async function go() {
      try {
        const ask = window.DeviceOrientationEvent?.requestPermission;
        if (ask && (await DeviceOrientationEvent.requestPermission()) !== "granted") throw Object.assign(new Error(), { name: "NotAllowedError" });
        start.hidden = true;
        listen();
      } catch (e) {
        note.textContent = D.explain(e, "motion") ?? "";
      }
    }
    const needs = typeof window.DeviceOrientationEvent?.requestPermission === "function";
    start.hidden = silent || !needs;
    if (!silent) needs ? null : "DeviceOrientationEvent" in window ? listen() : (note.textContent = D.unsupported("motion"));
    return [start, note];
  }
  const DIRS = () => [L("N"), L("NE"), L("E"), L("SE"), L("S"), L("SW"), L("W"), L("NW")];
  B["ui-compass"] = (el, api) => {
    const cfg = api.config, S = cfg.size ?? 220, given = cfg.heading !== undefined;
    let heading = null, shown = 0, was = null;
    const rose = svg("g"), arrow = svg("g", { hidden: true });
    for (let a = 0; a < 360; a += 10) rose.append(svg("line", { x1: 0, y1: -96, x2: 0, y2: a % 30 ? -90 : -84, stroke: "currentColor", "stroke-width": a % 90 ? 1 : 2, opacity: a % 30 ? 0.35 : 0.8, transform: `rotate(${a})` }));
    DIRS().forEach((t, i) => i % 2 === 0 && rose.append(svg("g", { transform: `rotate(${i * 45})` }, svg("text", { y: -68, "font-weight": 600, style: i === 0 ? "fill:var(--color-destructive,#dc2626)" : null }, t))));
    rose.append(svg("path", { d: "M0 -54 9 0H-9z", fill: "var(--color-destructive, #dc2626)" }), svg("path", { d: "M0 54 9 0H-9z", fill: "currentColor", opacity: 0.3 }));
    arrow.append(svg("path", { d: "M0 -110 8 -97H-8z", fill: "var(--color-primary)" }));
    const dial = svg("svg", { "data-dial": "", viewBox: "-112 -112 224 224", width: S, height: S, role: "img" }, svg("circle", { "data-dial-face": "", r: 100 }), rose, arrow, svg("path", { d: "M0 -108 6 -99H-6z", fill: "currentColor" }));
    const text = make("div", { "data-dial-value": "" }), way = make("p", { "data-w-note": "" });
    const tools = given ? [] : sensors(el, ["deviceorientationabsolute", "deviceorientation"], (e) => {
      // An iPhone's is from north already; elsewhere it's counted the other way round.
      let h = typeof e.webkitCompassHeading === "number" ? e.webkitCompassHeading : e.absolute && e.alpha !== null ? 360 - e.alpha : null;
      if (h === null) return false;
      h = (h + (screen.orientation?.angle ?? 0) + 360) % 360;
      heading = h;
      const r = Math.round(h) % 360;
      if (cfg.bind && api.state[cfg.bind] !== r) api.set(cfg.bind, r);
      draw();
    });
    el.append(dial, text, way, ...tools);
    function draw() {
      if (given) {
        const v = K.value(api, cfg.heading);
        heading = blank(v) || !Number.isFinite(Number(v)) ? null : ((Number(v) % 360) + 360) % 360;
      }
      const from = point(K.value(api, cfg.from)), to = point(K.value(api, cfg.target)), b = from && to ? bearing(from, to) : null;
      const now = JSON.stringify([heading === null ? null : Math.round(heading), b, from, to]);
      if (now === was) return;
      was = now;
      // The short way round.
      const h = heading ?? 0;
      shown += ((h - shown + 540) % 360) - 180;
      rose.setAttribute("transform", `rotate(${-shown})`);
      arrow.toggleAttribute("hidden", b === null);
      arrow.style.display = b === null ? "none" : "";
      if (b !== null) arrow.setAttribute("transform", `rotate(${b - h})`);
      text.textContent = heading === null ? "–" : `${Math.round(heading) % 360}° ${DIRS()[Math.round(heading / 45) % 8]}`;
      dial.setAttribute("aria-label", heading === null ? L("Compass") : L("Facing {way}", { way: text.textContent }));
      way.textContent = b === null ? "" : L("{far} away, at {deg}°", { far: far(distance(from, to)), deg: Math.round(b) % 360 });
    }
    draw();
    return { update: draw };
  };
  B["ui-level"] = (el, api) => {
    const cfg = api.config, S = cfg.size ?? 220, tol = cfg.tolerance ?? 0.5, given = cfg.value !== undefined;
    let raw = null, zero = { x: 0, y: 0 }, was = null, smooth = null, on = false;
    const bubble = svg("circle", { "data-bubble": "", r: 22, fill: "var(--color-warning, #d97706)", opacity: 0.9 });
    const dial = svg("svg", { "data-dial": "", viewBox: "-112 -112 224 224", width: S, height: S, role: "img" }, svg("circle", { "data-dial-face": "", r: 100 }), svg("path", { d: "M-100 0H100M0 -100V100", stroke: "currentColor", opacity: 0.2 }), [26, 60].map((r) => svg("circle", { r, fill: "none", stroke: "currentColor", opacity: 0.3 })), bubble);
    const text = make("div", { "data-dial-value": "" });
    const tools = given ? [] : sensors(el, ["deviceorientation"], (e) => {
      if (e.beta === null || e.gamma === null) return false;
      // Flat, how far it leans each way; on an edge, how far it's turned.
      const flat = cfg.mode === "surface" || (cfg.mode !== "edge" && Math.abs(e.beta) < 45 && Math.abs(e.gamma) < 45);
      const g = { x: e.gamma, y: e.beta }, b = e.beta * rad, c = e.gamma * rad;
      const gx = Math.sin(c) * Math.cos(b), gy = -Math.sin(b);
      const now = flat ? g : { x: ((Math.atan2(gx, -gy) / rad + 45 + 360) % 90) - 45, y: 0 };
      smooth = smooth ? { x: smooth.x * 0.8 + now.x * 0.2, y: smooth.y * 0.8 + now.y * 0.2 } : now;
      raw = smooth;
      draw();
    });
    const set = btn("", { click: () => { zero = { ...(raw ?? zero) }; draw(true); } }, L("Set this as level")), reset = btn("ghost", { click: () => { zero = { x: 0, y: 0 }; draw(true); } }, L("Reset"));
    el.append(dial, text, ...tools, make("div", { "data-w-row": "" }, set, reset));
    function draw(force) {
      if (given) {
        const v = K.value(api, cfg.value);
        raw = isObj(v) ? { x: Number(v.x) || 0, y: Number(v.y) || 0 } : null;
      }
      const x = raw ? +(raw.x - zero.x).toFixed(1) : null, y = raw ? +(raw.y - zero.y).toFixed(1) : null, now = JSON.stringify([x, y, zero]);
      if (now === was && !force) return;
      was = now;
      set.hidden = !raw;
      reset.hidden = !raw || (!zero.x && !zero.y);
      const is = raw && Math.hypot(x, y) <= tol;
      el.toggleAttribute("data-level", !!is);
      if (is && !on && !given) navigator.vibrate?.(20);
      on = !!is;
      // 10° off is at the rim.
      const d = raw ? Math.hypot(x, y) : 0, k = d ? Math.min(d, 10) / d : 0;
      bubble.setAttribute("cx", raw ? -x * k * 7.6 : 0);
      bubble.setAttribute("cy", raw ? -y * k * 7.6 : 0);
      text.textContent = raw ? (is ? L("Level") : `${num(x, 1)}°${y ? ` · ${num(y, 1)}°` : ""}`) : "–";
      dial.setAttribute("aria-label", raw ? L("Off level by {x}° and {y}°", { x, y }) : L("Spirit level"));
      if (cfg.bind && raw) {
        const v = { x, y, level: !!is };
        if (JSON.stringify(api.state[cfg.bind]) !== JSON.stringify(v)) api.set(cfg.bind, v);
      }
    }
    draw();
    return { update: () => draw() };
  };
})();
