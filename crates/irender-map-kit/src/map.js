// ui-map. Leaflet (OpenStreetMap) and Google Maps sit behind one small adapter, which draws
// the base map and shapes and holds elements at places; the pins, popups, clusters and
// buttons are the kit's own elements, the same on both.
(() => {
  const L = (text, vars) => (window.__irender?.label ? window.__irender.label(text, vars) : text);
  const isObj = (v) => v !== null && typeof v === "object" && !Array.isArray(v);
  const num = (v) => (v === null || v === undefined || v === "" || typeof v === "boolean" ? NaN : Number(v));
  const str = (v) => (v === null || v === undefined ? "" : String(v));
  const same = (a, b) => a === b || JSON.stringify(a) === JSON.stringify(b);
  const calm = () => matchMedia("(prefers-reduced-motion: reduce)").matches;
  const EFFECTS = ["pulse", "glow", "bounce", "blink"];
  const TONES = ["neutral", "info", "success", "warning", "danger", "primary"];
  const CLOSE = '<svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true"><path d="M4 4l8 8M12 4l-8 8"/></svg>';
  const PIN = '<svg viewBox="0 0 30 38" aria-hidden="true"><path d="M15 37C9 29 1.5 23 1.5 15a13.5 13.5 0 0 1 27 0c0 8-7.500 14-13.500 22z"/><circle cx="15" cy="15" r="5.500"/></svg>';

  // ------------------------------------------------------------------ places
  // `[lat, lng]`, `{ lat, lng }` and the other usual names, as `{ lat, lng }`, or null.
  const place = (v) => {
    const at = Array.isArray(v) ? { lat: num(v[0]), lng: num(v[1]) }
      : isObj(v) ? { lat: num(v.lat ?? v.latitude), lng: num(v.lng ?? v.lon ?? v.long ?? v.longitude) } : null;
    return at && Math.abs(at.lat) <= 90 && Math.abs(at.lng) <= 540 ? at : null;
  };
  const round = (p) => ({ lat: Math.round(p.lat * 1e6) / 1e6, lng: Math.round(p.lng * 1e6) / 1e6 });
  const boundsOf = (places) =>
    places.reduce((b, p) => (b
      ? { north: Math.max(b.north, p.lat), south: Math.min(b.south, p.lat), east: Math.max(b.east, p.lng), west: Math.min(b.west, p.lng) }
      : { north: p.lat, south: p.lat, east: p.lng, west: p.lng }), null);
  // Where a place is at a zoom, in px on the whole world's map.
  const world = (p, zoom) => {
    const size = 256 * 2 ** zoom, s = Math.min(0.9999, Math.max(-0.9999, Math.sin((p.lat * Math.PI) / 180)));
    return { x: (size * (p.lng + 180)) / 360, y: size * (0.5 - Math.log((1 + s) / (1 - s)) / (4 * Math.PI)) };
  };
  // The places a shape covers, enough to fit the view to.
  const extent = (s) => {
    if (s.type !== "circle") return s.rings.flat();
    const dLat = s.radius / 111320, dLng = dLat / Math.max(0.01, Math.cos((s.center.lat * Math.PI) / 180));
    return [{ lat: s.center.lat + dLat, lng: s.center.lng + dLng }, { lat: s.center.lat - dLat, lng: s.center.lng - dLng }];
  };

  // Shapes as the adapters draw them: `{ id, type, rings | center and radius, … }`.
  const shape = (s, i) => {
    if (!isObj(s)) return null;
    const look = { id: s.id ?? `shape-${i + 1}`, title: s.title, description: s.description, tone: s.tone, color: s.color, width: num(s.width) || 3, dashed: !!s.dashed, fill: s.fill, row: s };
    if (s.type === "circle") {
      const center = place(s.center ?? s), radius = num(s.radius);
      return center && radius > 0 ? { ...look, type: "circle", center, radius } : null;
    }
    const pts = Array.isArray(s.points) ? s.points : [];
    const nested = Array.isArray(pts[0]) && typeof pts[0][0] === "object";
    const rings = (nested ? pts : [pts]).map((r) => r.map(place).filter(Boolean)).filter((r) => r.length > 1);
    return rings.length ? { ...look, type: s.type === "area" ? "area" : "line", rings } : null;
  };
  // Overlays: `{ id, type, opacity, … }`, with `bounds` as `{ north, south, east, west }`.
  const corners = (b) => {
    if (isObj(b) && ["north", "south", "east", "west"].every((k) => Number.isFinite(num(b[k])))) return { north: num(b.north), south: num(b.south), east: num(b.east), west: num(b.west) };
    return Array.isArray(b) && b.length > 1 ? boundsOf(b.map(place).filter(Boolean)) : null;
  };
  const overlay = (o, i, allowed) => {
    if (!isObj(o) || o.show === false) return null;
    const opacity = Number.isFinite(num(o.opacity)) ? Math.min(1, Math.max(0, num(o.opacity))) : o.type === "heat" ? 0.7 : 1;
    const base = { id: str(o.id ?? `overlay-${i + 1}`), type: o.type, opacity, title: str(o.title) };
    if (o.type === "heat") {
      const points = (Array.isArray(o.points) ? o.points : []).flatMap((p) => {
        const at = place(isObj(p) ? p.position ?? p : p), w = num(Array.isArray(p) ? p[2] ?? 1 : p?.[o.weightField || "weight"] ?? 1);
        return at && w > 0 ? [{ at, w }] : [];
      });
      return { ...base, points, radius: num(o.radius) || 32, max: num(o.max) || 0 };
    }
    if (typeof o.url !== "string" || !allowed(o.url)) return null;
    if (o.type === "tiles") return { ...base, url: o.url, attribution: o.attribution, attributionUrl: allowed(o.attributionUrl) ? o.attributionUrl : null, maxZoom: num(o.maxZoom) || 19 };
    const bounds = corners(o.bounds);
    return bounds ? { ...base, type: "image", url: o.url, bounds } : null;
  };
  // GeoJSON as shapes and marker rows. Its positions are `[lng, lat]`.
  const fromGeo = (geo) => {
    const shapes = [], rows = [];
    const flip = (c) => [c[1], c[0]];
    const geometry = (g, p, id) => {
      if (!isObj(g)) return;
      const add = (type, points) => shapes.push({ ...p, id: id ?? `geo-${shapes.length + 1}`, type, points });
      const c = g.coordinates;
      if (g.type === "Point") rows.push({ ...p, id: id ?? p.id ?? `geo-p${rows.length + 1}`, lat: c?.[1], lng: c?.[0] });
      else if (g.type === "MultiPoint") c?.forEach((x, i) => geometry({ type: "Point", coordinates: x }, p, id === undefined ? undefined : `${id}-${i}`));
      else if (g.type === "LineString") add("line", c.map(flip));
      else if (g.type === "MultiLineString") add("line", c.map((r) => r.map(flip)));
      else if (g.type === "Polygon") add("area", c.map((r) => r.map(flip)));
      else if (g.type === "MultiPolygon") c.forEach((poly) => add("area", poly.map((r) => r.map(flip))));
      else if (g.type === "GeometryCollection") g.geometries?.forEach((x) => geometry(x, p, id));
    };
    const walk = (x) => {
      if (!isObj(x)) return;
      if (x.type === "FeatureCollection") x.features?.forEach(walk);
      else if (x.type === "Feature") geometry(x.geometry, x.properties || {}, x.id ?? x.properties?.id);
      else geometry(x, {}, undefined);
    };
    walk(geo);
    return { shapes, rows };
  };

  // ---- `filter`: the data table's conditions (`matches` in data_table.js), so one
  // `ui-filters` panel can narrow a map, a table and a chart alike
  const field = (r, f) => String(f).split(".").reduce((x, k) => (x !== null && typeof x === "object" ? x[k] : undefined), r);
  const blank = (v) => v === null || v === undefined || v === "";
  const isNum = (v) => typeof v === "number" || (typeof v === "string" && v.trim() !== "" && !isNaN(v));
  const looksLikeDate = (v) => typeof v === "string" && /^\d{4}-\d{2}/.test(v) && !isNaN(Date.parse(v));
  const fold = (s) => String(s ?? "").normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase();
  const equal = (a, b) => a === b || (!blank(a) && !blank(b) && typeof a !== "object" && typeof b !== "object" && String(a) === String(b));
  const cmp = (a, b) => {
    if (isNum(a) && isNum(b)) return Number(a) - Number(b);
    if (looksLikeDate(a) && looksLikeDate(b)) return Date.parse(a) - Date.parse(b);
    return String(a ?? "").localeCompare(String(b ?? ""), undefined, { numeric: true, sensitivity: "base" });
  };
  const endOfDay = (b) => (typeof b === "string" && /^\d{4}-\d{2}-\d{2}$/.test(b) ? b + "T23:59:59.999" : b);
  const OPS = {
    from: (v, b) => cmp(v, b) >= 0,
    to: (v, b) => cmp(v, looksLikeDate(v) ? endOfDay(b) : b) <= 0,
    gt: (v, b) => cmp(v, b) > 0,
    gte: (v, b) => cmp(v, b) >= 0,
    lt: (v, b) => cmp(v, b) < 0,
    lte: (v, b) => cmp(v, b) <= 0,
    contains: (v, b) => fold(v).includes(fold(b)),
  };
  function matches(v, c, field) {
    if (blank(c)) return true;
    if (Array.isArray(v)) return !v.length ? matches(null, c, field) || (Array.isArray(c) && !c.length) : v.some((x) => matches(x, c, field));
    if (Array.isArray(c)) return !c.length || c.some((x) => equal(v, isObj(x) ? x[field] : x));
    if (!isObj(c)) return equal(v, c);
    if (Object.hasOwn(c, "not")) return blank(c.not) || !matches(v, c.not, field);
    if (Object.hasOwn(c, "empty")) return blank(c.empty) || (c.empty ? blank(v) : !blank(v));
    const ops = Object.keys(c).filter((k) => Object.hasOwn(OPS, k));
    if (!ops.length) return equal(v, c[field]);
    return ops.every((k) => blank(c[k]) || (!blank(v) && OPS[k](v, c[k])));
  }
  const passes = (row, f) => !isObj(f) || Object.entries(f).every(([name, c]) => matches(field(row, name), c, name));

  // ------------------------------------------------------------------ loading
  const loads = {};
  const script = (src, done, setup) =>
    (loads[src] ||= new Promise((ok, fail) => {
      const failed = (why) => {
        delete loads[src];
        fail(new Error(why));
      };
      setup?.(ok, failed);
      const s = document.createElement("script");
      s.src = src;
      s.async = true;
      if (done) s.onload = () => (done() ? ok(done()) : failed(`${src} did not define what it should`));
      s.onerror = () => failed(`couldn't load ${src}`);
      document.head.append(s);
    }));
  const leafletLib = async (base) => {
    if (!document.querySelector("link[data-map-leaflet]") && !window.L?.map) {
      await new Promise((ok) => {
        const l = document.createElement("link");
        l.rel = "stylesheet";
        l.href = `${base}leaflet.css`;
        l.setAttribute("data-map-leaflet", "");
        l.onload = l.onerror = ok;
        document.head.append(l);
      });
    }
    return window.L?.map ? window.L : script(`${base}leaflet.js`, () => window.L);
  };
  // Google refuses a key after the map is made, through one page-wide function.
  const refused = new Set();
  const googleLib = (source) => {
    if (!window.__irenderMapAuth) {
      window.__irenderMapAuth = true;
      const before = window.gm_authFailure;
      window.gm_authFailure = () => {
        before?.();
        refused.forEach((f) => f());
      };
    }
    if (window.google?.maps?.Map) return Promise.resolve(window.google.maps);
    const q = new URLSearchParams({ key: source.key, loading: "async", callback: "__irenderMapReady", v: "weekly" });
    if (source.language) q.set("language", source.language);
    return script(`https://maps.googleapis.com/maps/api/js?${q}`, null, (ok) => (window.__irenderMapReady = () => ok(window.google.maps)));
  };

  // ------------------------------------------------------------------ Leaflet
  async function leaflet(box, o) {
    const Lf = await leafletLib(o.source.leaflet);
    const t = o.source.tiles, still = o.gestures === "none", shares = o.gestures === "cooperative";
    const touch = matchMedia("(pointer: coarse)").matches;
    const map = Lf.map(box, {
      zoomControl: false,
      center: [o.center.lat, o.center.lng],
      zoom: o.zoom,
      minZoom: o.minZoom ?? 1,
      maxZoom: o.maxZoom ?? t.maxZoom ?? 19,
      worldCopyJump: true,
      dragging: !still && !(shares && touch),
      scrollWheelZoom: o.gestures === "greedy",
      doubleClickZoom: !still,
      touchZoom: !still,
      boxZoom: !still,
      keyboard: !still,
      zoomAnimation: !calm(),
      fadeAnimation: !calm(),
      markerZoomAnimation: !calm(),
    });
    const ll = (p) => [p.lat, p.lng];
    const fit = (b, max, move) => map.fitBounds([[b.south, b.west], [b.north, b.east]], { paddingTopLeft: [32, 52], paddingBottomRight: [32, 32], maxZoom: max, ...move });
    // Fitted before there are tiles, so none are asked for that aren't needed.
    if (o.bounds) fit(o.bounds, o.closest, { animate: false });
    map.attributionControl.setPrefix('<a href="https://leafletjs.com" target="_blank" rel="noopener">Leaflet</a>');
    // Leaflet takes attribution as HTML; this one is text and a link.
    const credit = (x) => {
      if (!x.attribution) return "";
      const a = document.createElement("a");
      a.textContent = str(x.attribution);
      if (x.attributionUrl) Object.assign(a, { href: x.attributionUrl, target: "_blank", rel: "noopener" });
      return a.outerHTML;
    };
    const url = () => (o.dark() && t.darkUrl) || t.url;
    // OpenStreetMap's servers want to know which site asks.
    const tiles = Lf.tileLayer(url(), { attribution: credit(t), maxZoom: 22, maxNativeZoom: t.maxZoom ?? 19, referrerPolicy: "strict-origin-when-cross-origin" }).addTo(map);
    let loaded = false, errors = 0;
    tiles.on("tileload", () => (loaded = true)).on("tileerror", () => !loaded && ++errors === 3 && o.say(L("The map's tiles couldn't be loaded"), true));
    const themed = () => {
      if (tiles._url !== url()) tiles.setUrl(url());
      map.getPane("tilePane").toggleAttribute("data-filter", !t.darkUrl);
    };
    themed();
    // Apart from the base tiles, which dark mode darkens.
    Object.assign(map.createPane("overlays").style, { zIndex: 250, pointerEvents: "none" });
    map.on("zoomstart", o.zooming).on("moveend zoomend", o.moved).on("click", (e) => o.clicked({ lat: e.latlng.lat, lng: e.latlng.lng }));
    if (shares) {
      const key = /Mac|iP/.test(navigator.platform) ? "⌘" : "Ctrl";
      // Before Leaflet's own listener, which this turns on and off.
      o.el.addEventListener("wheel", (e) => {
        if (e.target.closest?.("[data-map-popup], [data-position]")) return;
        map.scrollWheelZoom[e.ctrlKey || e.metaKey ? "enable" : "disable"]();
        if (!e.ctrlKey && !e.metaKey) o.say(L("Use {key} + scroll to zoom the map", { key }));
      }, { capture: true, passive: true });
      if (touch) box.addEventListener("touchmove", (e) => e.touches.length === 1 && o.say(L("Use two fingers to zoom the map")), { passive: true });
    }
    const pt = (p) => ({ lat: p.lat, lng: p.lng });
    const motion = () => ({ animate: !calm() });
    return {
      map,
      themed,
      view: () => {
        const b = map.getBounds(), c = map.getCenter().wrap();
        return { center: pt(c), zoom: map.getZoom(), bounds: { north: b.getNorth(), south: b.getSouth(), east: b.getEast(), west: b.getWest() } };
      },
      go: (p, zoom) => map.setView(ll(p), zoom ?? map.getZoom(), motion()),
      zoomBy: (n) => map.setZoom(map.getZoom() + n, motion()),
      fit: (b, max) => fit(b, max, motion()),
      project: (p) => map.latLngToContainerPoint(ll(p)),
      unproject: (x, y) => pt(map.containerPointToLatLng([x, y])),
      resize: () => map.invalidateSize({ animate: false }),
      // While shapes are drawn, a double click finishes one rather than zooming.
      drawing: (on) => (on ? map.doubleClickZoom.disable() : !still && map.doubleClickZoom.enable()),
      // `put` runs once the element is on the page.
      hold(el, p, above, put) {
        const m = Lf.marker(ll(p), { icon: Lf.divIcon({ html: el, className: "", iconSize: [0, 0] }), keyboard: false, interactive: false, zIndexOffset: above === "under" ? -1e6 : above ? 100000 : 0 }).addTo(map);
        Lf.DomEvent.disableClickPropagation(el);
        if (above === true) Lf.DomEvent.disableScrollPropagation(el);
        put?.();
        return { move: (q) => m.setLatLng(ll(q)), raise: (on) => m.setZIndexOffset(on ? 50000 : above ? 100000 : 0), remove: () => m.remove() };
      },
      // A picture stretched over its bounds, or more tiles, over the map and under the shapes.
      lay(x) {
        const layer = (x.type === "image"
          ? Lf.imageOverlay(x.url, [[x.bounds.south, x.bounds.west], [x.bounds.north, x.bounds.east]], { opacity: x.opacity, alt: x.title, zIndex: 1 })
          : Lf.tileLayer(x.url, { opacity: x.opacity, attribution: credit(x), maxZoom: 22, maxNativeZoom: x.maxZoom, zIndex: 2, pane: "overlays", referrerPolicy: "strict-origin-when-cross-origin" })).addTo(map);
        return { fade: (n) => layer.setOpacity(n), remove: () => layer.remove() };
      },
      draw(s, color, click) {
        const style = { color, weight: s.width, opacity: 0.9, dashArray: s.dashed ? "6 6" : null, fillColor: color, fillOpacity: s.fill === false || s.type === "line" ? 0 : num(s.fill) || 0.15, fill: s.type !== "line", interactive: !!click };
        const rings = s.rings?.map((r) => r.map(ll));
        const layer = s.type === "circle" ? Lf.circle(ll(s.center), { ...style, radius: s.radius }) : s.type === "area" ? Lf.polygon(rings, style) : Lf.polyline(rings, style);
        if (click) layer.on("click", (e) => (Lf.DomEvent.stopPropagation(e), click(pt(e.latlng))));
        layer.addTo(map);
        return { remove: () => layer.remove(), paint: (c) => layer.setStyle({ color: c, fillColor: c }) };
      },
    };
  }

  // ------------------------------------------------------------------ Google Maps
  async function google(box, o) {
    const G = await googleLib(o.source);
    const options = () => ({
      center: o.center,
      zoom: o.zoom,
      minZoom: o.minZoom,
      maxZoom: o.maxZoom,
      disableDefaultUI: true,
      clickableIcons: false,
      mapTypeId: o.mapType || "roadmap",
      gestureHandling: o.gestures,
      keyboardShortcuts: o.gestures !== "none",
      ...(o.source.mapId ? { mapId: o.source.mapId } : {}),
      colorScheme: o.dark() ? "DARK" : "LIGHT",
    });
    const map = new G.Map(box, options());
    refused.add(() => o.failed(new Error(L("Google Maps refused the API key"))));
    map.addListener("zoom_changed", o.zooming);
    map.addListener("idle", o.moved);
    map.addListener("click", (e) => e.latLng && o.clicked({ lat: e.latLng.lat(), lng: e.latLng.lng() }));
    // An element at a place, in one of the map's panes.
    class Held extends G.OverlayView {
      constructor(el, p, above, put) {
        super();
        Object.assign(this, { el, p, above, put, up: false });
      }
      onAdd() {
        this.el.style.position = "absolute";
        this.getPanes()[this.above === "under" ? "overlayLayer" : this.above ? "floatPane" : "overlayMouseTarget"].append(this.el);
        G.OverlayView.preventMapHitsAndGesturesFrom(this.el);
      }
      draw() {
        const at = this.getProjection()?.fromLatLngToDivPixel(new G.LatLng(this.p.lat, this.p.lng));
        if (!at) return;
        this.el.style.left = `${at.x}px`;
        this.el.style.top = `${at.y}px`;
        this.el.style.zIndex = this.up ? 100000 : Math.round(at.y);
        const put = this.put;
        this.put = null;
        put?.();
      }
      onRemove() {
        this.el.remove();
      }
    }
    // One that holds nothing, for its projection.
    let ruler;
    await new Promise((ok, fail) => {
      ruler = new Held(document.createElement("div"), o.center, false, ok);
      ruler.setMap(map);
      setTimeout(() => fail(new Error("Google Maps didn't start")), 15000);
    });
    const pt = (p) => ({ lat: p.lat(), lng: p.lng() });
    const fit = (b, max) => {
      map.fitBounds(b, { top: 52, left: 32, right: 32, bottom: 32 });
      // It fits a moment later, and one place alone as close as it can.
      G.event.addListenerOnce(map, "idle", () => map.getZoom() > max && map.setZoom(max));
    };
    if (o.bounds) fit(o.bounds, o.closest);
    let scheme = o.dark();
    return {
      map,
      // Google takes its colour scheme only when a map is made.
      themed: () => o.dark() !== scheme && ((scheme = o.dark()), o.restart()),
      view: () => {
        const b = map.getBounds()?.toJSON();
        return { center: pt(map.getCenter()), zoom: map.getZoom(), bounds: b ?? { north: 90, south: -90, east: 180, west: -180 } };
      },
      go: (p, zoom) => {
        calm() ? map.setCenter(p) : map.panTo(p);
        if (zoom !== undefined) map.setZoom(zoom);
      },
      zoomBy: (n) => map.setZoom(map.getZoom() + n),
      fit,
      project: (p) => ruler.getProjection().fromLatLngToContainerPixel(new G.LatLng(p.lat, p.lng)),
      unproject: (x, y) => pt(ruler.getProjection().fromContainerPixelToLatLng(new G.Point(x, y))),
      resize() {},
      drawing: (on) => map.setOptions({ disableDoubleClickZoom: on || o.gestures === "none" }),
      hold(el, p, above, put) {
        const h = new Held(el, p, above, put);
        h.setMap(map);
        return { move: (q) => ((h.p = q), h.draw()), raise: (on) => ((h.up = on), h.draw()), remove: () => h.setMap(null) };
      },
      lay(x) {
        if (x.type === "image") {
          const g = new G.GroundOverlay(x.url, x.bounds, { opacity: x.opacity, clickable: false, map });
          return { fade: (n) => g.setOpacity(n), remove: () => g.setMap(null) };
        }
        const t = new G.ImageMapType({
          getTileUrl: ({ x: cx, y }, z) => (y < 0 || y >= 2 ** z || z > x.maxZoom ? null : x.url.replace("{s}", "a").replace("{r}", "").replace("{z}", z).replace("{x}", ((cx % 2 ** z) + 2 ** z) % 2 ** z).replace("{y}", y)),
          tileSize: new G.Size(256, 256),
          opacity: x.opacity,
        });
        map.overlayMapTypes.push(t);
        return {
          fade: (n) => t.setOpacity(n),
          remove: () => {
            const i = map.overlayMapTypes.getArray().indexOf(t);
            if (i >= 0) map.overlayMapTypes.removeAt(i);
          },
        };
      },
      draw(s, color, click) {
        const look = (c) => {
          const line = { strokeColor: c, strokeOpacity: 0.9, strokeWeight: s.width, clickable: !!click };
          // Google has no dashes: a line of repeated strokes stands in for them.
          if (s.dashed && s.type === "line") return { ...line, strokeOpacity: 0, icons: [{ icon: { path: "M 0,-1 0,1", strokeOpacity: 0.9, strokeColor: c, scale: s.width }, offset: "0", repeat: `${s.width * 5}px` }] };
          return s.type === "line" ? line : { ...line, fillColor: c, fillOpacity: s.fill === false ? 0 : num(s.fill) || 0.15 };
        };
        const made = s.type === "circle" ? [new G.Circle({ ...look(color), center: s.center, radius: s.radius, map })]
          : s.type === "area" ? [new G.Polygon({ ...look(color), paths: s.rings, map })]
          : s.rings.map((path) => new G.Polyline({ ...look(color), path, map }));
        if (click) made.forEach((m) => m.addListener("click", (e) => click(pt(e.latLng))));
        return { remove: () => made.forEach((m) => m.setMap(null)), paint: (c) => made.forEach((m) => m.setOptions(look(c))) };
      },
    };
  }

  // Maps on the page, which follow the theme.
  const maps = new Set();
  const retheme = () => maps.forEach((ref) => (ref.deref() ? ref.deref().themed() : maps.delete(ref)));
  let watching = false;
  const watch = () => {
    if (watching) return;
    watching = true;
    matchMedia("(prefers-color-scheme: dark)").addEventListener("change", retheme);
    addEventListener("irender:theme-option", retheme);
    const o = new MutationObserver(retheme);
    for (const n of [document.documentElement, document.body]) o.observe(n, { attributes: true, attributeFilter: ["class", "style", "data-theme", "data-mode"] });
  };
  // A colour as the libraries take it, and whether it's dark.
  const ink = (c) => {
    const x = (ink.c ||= Object.assign(document.createElement("canvas"), { width: 1, height: 1 }).getContext("2d", { willReadFrequently: true }));
    x.clearRect(0, 0, 1, 1);
    x.fillStyle = "#000";
    x.fillStyle = c;
    x.fillRect(0, 0, 1, 1);
    const [r, g, b] = x.getImageData(0, 0, 1, 1).data;
    return { css: `rgb(${r}, ${g}, ${b})`, dark: 0.299 * r + 0.587 * g + 0.114 * b < 128 };
  };

  (window.__irenderBehaviors ||= {})["ui-map"] = (el, api) => {
    const cfg = api.config;
    const canvas = el.querySelector("[data-map-canvas]"), note = el.querySelector("[data-map-note]"), live = el.querySelector("[data-map-live]"), proto = el.querySelector("[data-map-proto]");
    const env = () => ({ ...api.state, ...(api.scope?.() || {}) });
    const resolve = (v) => {
      if (Array.isArray(v)) return v.map(resolve);
      if (!isObj(v)) return v;
      if ("$irender" in v && Object.keys(v).length === 1) return window.__irender.evaluate(v.$irender, env());
      return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, resolve(x)]));
    };
    const props = {}, fixed = cfg.props;
    const read = () => {
      for (const k of ["markers", "shapes", "geojson", "center", "zoom", "fit", "cluster", "spread", "toneBy", "draggable", "filter", "alarmBy", "effectBy", "overlays"]) props[k] = k in fixed ? resolve(fixed[k]) : undefined;
    };
    read();
    const set = (key, value) => cfg[key] && !same(api.state[cfg[key]], value) && api.set(cfg[key], value);
    const make = (tag, attrs = {}, text) => {
      const e = document.createElement(tag);
      for (const [k, v] of Object.entries(attrs)) if (v !== null && v !== undefined && v !== false) e.setAttribute(k, v === true ? "" : v);
      if (text !== undefined) e.textContent = text;
      return e;
    };

    let map = null; // the adapter
    let rows = [], byId = new Map(); // the markers
    let shapes = [], drawn = [];
    const pins = new Map(); // id → { at, pin, held, row }, the pins and clusters on the map
    let selected = null, popup = null, point = null, me = null;

    // ---------------------------------------------------------------- saying things
    let noteTimer;
    const say = (text, stay) => {
      note.textContent = text;
      note.toggleAttribute("data-shown", !!text);
      clearTimeout(noteTimer);
      if (text && !stay) noteTimer = setTimeout(() => note.removeAttribute("data-shown"), 1500);
    };
    function failed(error) {
      if (el.hasAttribute("data-error")) return;
      console.error("ui-map:", error);
      map = null;
      el.removeAttribute("data-ready");
      el.setAttribute("data-error", "");
      say("");
      // The places are still worth something without the map.
      canvas.replaceChildren();
      el.querySelector("[data-map-failed]")?.remove();
      const box = make("div", { "data-map-failed": "", role: "alert" });
      box.append(make("strong", {}, L("Map unavailable")), make("div", {}, error.message));
      if (rows.length) {
        const ul = make("ul");
        ul.append(...rows.slice(0, 50).map((r) => make("li", {}, r.title || `${r.at.lat}, ${r.at.lng}`)));
        box.append(ul);
      }
      el.append(box);
    }

    // ---------------------------------------------------------------- what's on it
    const toned = (m) => {
      if (TONES.includes(m.tone)) return m.tone;
      const t = isObj(props.toneBy) ? props.toneBy.tones?.[str(m[props.toneBy.field])] : null;
      return TONES.includes(t) ? t : null;
    };
    const alarmed = (m) => {
      if (typeof m.alarm === "boolean") return m.alarm;
      const by = props.alarmBy;
      return isObj(by) && [].concat(by.values ?? []).some((v) => str(v) === str(m[by.field]));
    };
    const moving = (m, alarm) => {
      const by = props.effectBy;
      const e = m.effect ?? (isObj(by) ? by.effects?.[str(m[by.field])] : null) ?? (alarm ? "pulse" : null);
      return EFFECTS.includes(e) ? e : null;
    };
    // The alarms, and what's said of the ones that have just started.
    let ringing = null;
    function alarms() {
      const now = rows.filter((r) => r.alarm);
      const fresh = ringing ? now.filter((r) => !ringing.has(str(r.id))) : [];
      ringing = new Set(now.map((r) => str(r.id)));
      set("bindAlarms", now.map((r) => r.id));
      if (fresh.length) live.textContent = fresh.length > 3 ? L("{n} new alarms", { n: fresh.length }) : fresh.map((r) => L("Alarm: {title}", { title: r.title || L("Marker") })).join(". ");
      for (const r of fresh) api.emit("alarm", { id: r.id, marker: r.row });
    }
    function collect() {
      const geo = props.geojson ? fromGeo(props.geojson) : { shapes: [], rows: [] };
      const seen = new Set();
      rows = [...(Array.isArray(props.markers) ? props.markers : []), ...geo.rows].flatMap((m, i) => {
        if (!isObj(m)) return [];
        const at = place(fixed.latField || fixed.lngField ? { lat: m[fixed.latField || "lat"], lng: m[fixed.lngField || "lng"] } : m.position ?? m);
        const id = m.id ?? i;
        if (!at || seen.has(str(id)) || !passes(m, props.filter)) return [];
        seen.add(str(id));
        const alarm = alarmed(m);
        return [{ id, at, row: m, title: str(fixed.titleField ? m[fixed.titleField] : m.title ?? m.name), tone: toned(m) ?? (alarm && !m.color ? "danger" : null), color: m.color, alarm, effect: moving(m, alarm) }];
      });
      byId = new Map(rows.map((r) => [str(r.id), r]));
      shapes = [...(Array.isArray(props.shapes) ? props.shapes : []), ...geo.shapes].map(shape).filter(Boolean);
    }
    const pictures = () => (Array.isArray(props.overlays) ? props.overlays : []).flatMap((o) => {
      const b = isObj(o) && o.show !== false && o.type !== "heat" && o.type !== "tiles" ? corners(o.bounds) : null;
      return b ? [{ lat: b.north, lng: b.east }, { lat: b.south, lng: b.west }] : [];
    });
    const all = () => [...rows.map((r) => r.at), ...shapes.flatMap(extent), ...pictures()];
    const closest = () => num(fixed.maxZoom) || 16;
    function fitAll() {
      const b = boundsOf(all());
      if (b) map.fit(b, closest());
      return !!b;
    }
    const probe = make("i", { hidden: "" });
    el.append(probe);
    const colorOf = (s) => {
      probe.style.color = "";
      probe.removeAttribute("data-tone");
      if (TONES.includes(s.tone)) probe.setAttribute("data-tone", s.tone);
      probe.style.color = s.color && CSS.supports("color", s.color) ? s.color : "var(--tone)";
      return ink(getComputedStyle(probe).color).css;
    };
    const dark = () => {
      probe.style.color = "var(--color-surface, Canvas)";
      return ink(getComputedStyle(probe).color).dark;
    };
    function drawShapes() {
      drawn.forEach((d) => d.handle.remove());
      const mine = new Set(kinds().length ? ownRows().map((r) => str(r?.id)) : []);
      drawn = shapes.map((s) => {
        const acts = cfg.bindShape || (cfg.popup && (s.title || s.description)) || mine.has(str(s.id));
        const click = acts ? (at) => {
          if (sketch.kind) return sketchAt(at);
          // A shape drawn here is picked, to take away.
          if (mine.has(str(s.id))) {
            pickShape(s.id);
            set("bindShape", s.id);
            return;
          }
          set("bindShape", s.id);
          api.emit("shapeselect", { id: s.id, shape: s.row });
          if (cfg.popup && (s.title || s.description)) {
            select(null);
            open({ title: str(s.title), row: s.row, at }, null, true);
          }
        } : null;
        return { s, handle: map.draw(s, colorOf(s), click) };
      });
    }

    // ---------------------------------------------------------------- overlays
    // The ones on the map by id, kept while only how strong they are changes.
    const laid = new Map();
    const allowed = (u) => typeof u === "string" && !/^\s*(javascript|vbscript):/i.test(u) && (!cfg.safe || /^(https?:\/\/|[/.?#]|[^:]*$)/i.test(u.trim()));
    function drawOverlays() {
      if (!map) return;
      const want = new Map((Array.isArray(props.overlays) ? props.overlays : []).map((o, i) => overlay(o, i, allowed)).filter(Boolean).map((o) => [o.id, o]));
      const what = ({ opacity, points, ...x }) => JSON.stringify(x);
      for (const [id, l] of laid) if (!want.has(id) || what(want.get(id)) !== what(l.o)) (l.handle.remove(), laid.delete(id));
      for (const [id, o] of want) {
        const l = laid.get(id);
        if (!l) laid.set(id, { o, handle: o.type === "heat" ? heat(o) : map.lay(o) });
        else {
          if (l.o.opacity !== o.opacity) l.handle.fade(o.opacity);
          const moved = l.o.points && !same(l.o.points, o.points);
          l.o = o;
          if (moved) l.handle.redraw(o);
        }
      }
      // Leaflet credits overlays with its own; on Google they're said here.
      if (cfg.provider !== "google") return;
      let box = el.querySelector("[data-map-credit]");
      const credits = [...want.values()].filter((o) => o.attribution);
      if (!credits.length) return box?.remove();
      if (!box) el.append((box = make("div", { "data-map-credit": "" })));
      box.replaceChildren(...credits.flatMap((o, i) => [i ? " · " : "", o.attributionUrl ? make("a", { href: o.attributionUrl, target: "_blank", rel: "noopener" }, str(o.attribution)) : str(o.attribution)]));
    }
    // A heat map, on a canvas a little bigger than the view, drawn again when the view stops.
    function heat(first) {
      let o = first;
      const pad = 200, at = make("div", { "data-map-at": "", "data-map-heat": "", "aria-hidden": "true" }), c = make("canvas");
      at.append(c);
      const held = map.hold(at, map.view().center, "under");
      const paint = () => {
        const w = el.clientWidth + pad * 2, h = el.clientHeight + pad * 2;
        if (!el.clientWidth) return;
        held.move(map.unproject(-pad, -pad));
        c.width = w;
        c.height = h;
        c.style.opacity = o.opacity;
        const x = c.getContext("2d", { willReadFrequently: true });
        const most = o.max || o.points.reduce((m, p) => Math.max(m, p.w), 0);
        for (const p of o.points) {
          const q = map.project(p.at), px = q.x + pad, py = q.y + pad, r = o.radius;
          if (px < -r || py < -r || px > w + r || py > h + r) continue;
          const g = x.createRadialGradient(px, py, 0, px, py, r);
          const a = Math.min(1, p.w / most) * 0.6;
          g.addColorStop(0, `rgba(0,0,0,${a})`);
          g.addColorStop(1, "rgba(0,0,0,0)");
          x.fillStyle = g;
          x.fillRect(px - r, py - r, r * 2, r * 2);
        }
        // How dark it got says which colour, cool to hot.
        const ramp = document.createElement("canvas").getContext("2d");
        ramp.canvas.width = 256;
        ramp.canvas.height = 1;
        const g = ramp.createLinearGradient(0, 0, 256, 0);
        ["info", "success", "warning", "danger"].forEach((tone, i) => g.addColorStop(i / 3, colorOf({ tone })));
        ramp.fillStyle = g;
        ramp.fillRect(0, 0, 256, 1);
        const colors = ramp.getImageData(0, 0, 256, 1).data, img = x.getImageData(0, 0, w, h), d = img.data;
        for (let i = 0; i < d.length; i += 4) {
          const a = d[i + 3];
          if (!a) continue;
          d[i] = colors[a * 4];
          d[i + 1] = colors[a * 4 + 1];
          d[i + 2] = colors[a * 4 + 2];
          d[i + 3] = Math.min(255, a * 2);
        }
        x.putImageData(img, 0, 0);
      };
      paint();
      return { fade: (n) => (c.style.opacity = n), redraw: (next) => ((o = next ?? o), paint()), remove: () => held.remove() };
    }

    // ---------------------------------------------------------------- pins
    function paint(p) {
      const { row: r, pin } = p, m = r.row;
      const name = r.title || L("Marker");
      pin.setAttribute("aria-label", r.alarm ? L("{title}, alarm", { title: name }) : name);
      pin.title = r.title;
      pin.toggleAttribute("data-alarm", r.alarm);
      if (r.effect) pin.setAttribute("data-effect", r.effect);
      else pin.removeAttribute("data-effect");
      if (r.tone) pin.setAttribute("data-tone", r.tone);
      else pin.removeAttribute("data-tone");
      pin.style.removeProperty("--tone");
      if (r.color && CSS.supports("color", r.color)) pin.style.setProperty("--tone", r.color);
      pin.toggleAttribute("data-draggable", movable(r));
      pin.innerHTML = PIN;
      const label = str(m.label) || (r.alarm ? "!" : "");
      if (label) pin.append(make("span", { "aria-hidden": "true" }, [...label].slice(0, 3).join("")));
    }
    const movable = (r) => !!cfg.bindMarkers && (r.row.draggable ?? props.draggable) === true;
    const container = () => canvas.getBoundingClientRect();
    // Dragging an element that's held at a place: `moved(place, done)` as it goes.
    function drags(pin, where, can, moved) {
      let from = null, dragged = false;
      pin.addEventListener("pointerdown", (e) => {
        if (!can() || e.button) return;
        const c = container(), at = map.project(where());
        from = { x: e.clientX, y: e.clientY, dx: e.clientX - c.left - at.x, dy: e.clientY - c.top - at.y };
        dragged = false;
        try {
          pin.setPointerCapture(e.pointerId);
        } catch {}
        e.stopPropagation();
      });
      const to = (e) => {
        const c = container();
        return round(map.unproject(e.clientX - c.left - from.dx, e.clientY - c.top - from.dy));
      };
      pin.addEventListener("pointermove", (e) => {
        if (!from || (!dragged && Math.hypot(e.clientX - from.x, e.clientY - from.y) < 4)) return;
        dragged = true;
        pin.setAttribute("data-dragging", "");
        moved(to(e), false);
      });
      const end = (e) => {
        if (!from) return;
        pin.removeAttribute("data-dragging");
        if (dragged && e.type === "pointerup") moved(to(e), true);
        else if (dragged) moved(where(), true);
        from = null;
      };
      pin.addEventListener("pointerup", end);
      pin.addEventListener("pointercancel", end);
      // The click that ends a drag isn't one.
      pin.addEventListener("click", (e) => dragged && (e.stopImmediatePropagation(), (dragged = false)), true);
      pin.addEventListener("keydown", (e) => {
        const by = { ArrowUp: [0, -10], ArrowDown: [0, 10], ArrowLeft: [-10, 0], ArrowRight: [10, 0] }[e.key];
        if (!by || !e.altKey || !can()) return;
        e.preventDefault();
        e.stopPropagation();
        const at = map.project(where());
        moved(round(map.unproject(at.x + by[0], at.y + by[1])), true);
      });
    }
    function addPin(r) {
      const at = make("div", { "data-map-at": "" });
      const pin = make("button", { type: "button", "data-map-pin": str(r.id), style: "pointer-events:auto" });
      const p = { at, pin, row: r, held: null };
      at.append(pin);
      paint(p);
      drags(pin, () => p.row.at, () => movable(p.row), (to, done) => {
        p.row.at = to;
        p.held.move(to);
        if (popup?.id === str(p.row.id)) popup.held.move(to);
        if (!done) return;
        const key = { lat: fixed.latField || (["lat", "latitude"].find((k) => k in p.row.row) ?? "lat"), lng: fixed.lngField || (["lng", "lon", "long", "longitude"].find((k) => k in p.row.row) ?? "lng") };
        const list = (api.state[cfg.bindMarkers] || []).map((m, i) => (str(m.id ?? i) === str(p.row.id) ? (Array.isArray(m.position) ? { ...m, position: [to.lat, to.lng] } : isObj(m.position) ? { ...m, position: { ...m.position, ...to } } : { ...m, [key.lat]: to.lat, [key.lng]: to.lng }) : m));
        api.set(cfg.bindMarkers, list);
        api.emit("markermove", { id: p.row.id, ...to });
      });
      pin.addEventListener("click", (e) => {
        const keys = e.detail === 0;
        if (selected === str(p.row.id) && popup) return close(true);
        select(str(p.row.id), { keys });
      });
      p.held = map.hold(at, r.at, false);
      pins.set(str(r.id), p);
    }
    function addCluster(key, c) {
      const at = make("div", { "data-map-at": "" });
      const size = 32 + Math.min(20, Math.round(Math.log10(c.rows.length) * 10));
      const b = make("button", { type: "button", "data-map-cluster": "", style: `pointer-events:auto;--size:${size}px`, "aria-label": L("{n} places here: zoom in", { n: c.rows.length }) }, String(c.rows.length));
      // A group isn't where an alarm goes unseen.
      const loud = c.rows.filter((r) => r.alarm).length, first = c.rows.find((r) => r.alarm) ?? c.rows.find((r) => r.effect);
      if (loud) b.setAttribute("aria-label", L("{n} places here, {alarms} with an alarm: zoom in", { n: c.rows.length, alarms: loud }));
      b.toggleAttribute("data-alarm", !!loud);
      if (first?.effect) b.setAttribute("data-effect", first.effect === "bounce" ? "pulse" : first.effect);
      if (first) b.setAttribute("data-tone", first.tone ?? "primary");
      at.append(b);
      b.addEventListener("click", () => {
        const box = boundsOf(c.rows.map((r) => r.at));
        if (box.north - box.south < 1e-5 && box.east - box.west < 1e-5) map.go(c.at, Math.min(limit(), map.view().zoom + 3));
        else map.fit(box, limit());
      });
      pins.set(key, { at, cluster: c.rows.length, held: map.hold(at, c.at, false) });
    }
    const limit = () => num(fixed.maxZoom) || 19;
    // Which markers are close enough to share one spot at this zoom.
    function grouped(list, zoom) {
      const r = props.cluster === true ? 56 : num(props.cluster);
      if (!(r > 0) || zoom >= limit()) return list.map((x) => ({ rows: [x], at: x.at }));
      const cells = new Map(), groups = [];
      for (const x of list) {
        const p = world(x.at, zoom), cx = Math.floor(p.x / r), cy = Math.floor(p.y / r);
        let mine = null;
        for (let i = -1; i <= 1 && !mine; i++)
          for (let j = -1; j <= 1 && !mine; j++)
            mine = (cells.get(`${cx + i},${cy + j}`) || []).find((g) => str(x.id) !== selected && !g.alone && Math.hypot(g.x - p.x, g.y - p.y) < r) ?? null;
        if (mine) mine.rows.push(x);
        else {
          const g = { rows: [x], at: x.at, x: p.x, y: p.y, alone: str(x.id) === selected };
          groups.push(g);
          const k = `${cx},${cy}`;
          cells.set(k, [...(cells.get(k) || []), g]);
        }
      }
      for (const g of groups)
        if (g.rows.length > 1) g.at = { lat: g.rows.reduce((s, x) => s + x.at.lat, 0) / g.rows.length, lng: g.rows.reduce((s, x) => s + x.at.lng, 0) / g.rows.length };
      return groups;
    }
    const within = (p, b, pad = 0) => {
      const dy = (b.north - b.south) * pad, dx = (b.east - b.west) * pad;
      const lng = [p.lng, p.lng + 360, p.lng - 360];
      return p.lat <= b.north + dy && p.lat >= b.south - dy && lng.some((x) => x <= b.east + dx && x >= b.west - dx);
    };
    function drawPins() {
      if (!map) return;
      const v = map.view();
      // Thousands of elements are slow: past a few hundred, only those near the view.
      const near = rows.length > 300 ? rows.filter((r) => str(r.id) === selected || within(r.at, v.bounds, 0.5)) : rows;
      const want = new Map();
      for (const g of grouped(near, Math.round(v.zoom)))
        want.set(g.rows.length > 1 ? `\u0000${g.rows.map((r) => `${r.id}${r.alarm ? "!" : ""}${r.effect ?? ""}`).join("\u0001")}` : str(g.rows[0].id), g);
      for (const [k, p] of pins) if (!want.has(k)) (p.held.remove(), pins.delete(k));
      for (const [k, g] of want) {
        const p = pins.get(k);
        if (g.rows.length > 1) {
          if (p) p.held.move(g.at);
          else addCluster(k, g);
        } else if (!p) addPin(g.rows[0]);
        else if (p.row !== g.rows[0]) {
          p.row = g.rows[0];
          paint(p);
          p.held.move(p.row.at);
        }
      }
      spread();
      mark();
    }
    // Markers on the same spot (to a metre or so), shown alone, fan out round it, each on
    // a thin line to where it is, so every one can be seen and clicked (`spread: false`
    // leaves them stacked).
    function spread() {
      const spots = new Map();
      for (const p of pins.values()) {
        if (!p.pin) continue;
        const k = `${p.row.at.lat.toFixed(5)},${p.row.at.lng.toFixed(5)}`;
        spots.set(k, [...(spots.get(k) || []), p]);
      }
      for (const list of spots.values()) {
        const fan = props.spread !== false && list.length > 1;
        const r = Math.max(28, list.length * 7);
        list.sort((a, b) => str(a.row.id).localeCompare(str(b.row.id), undefined, { numeric: true }));
        list.forEach((p, i) => {
          const a = -Math.PI / 2 + (i * 2 * Math.PI) / list.length, dx = fan ? Math.round(Math.cos(a) * r) : 0, dy = fan ? Math.round(Math.sin(a) * r) : 0;
          p.pin.style.translate = fan ? `${dx}px ${dy}px` : "";
          p.pin.toggleAttribute("data-spread", fan);
          // Labels: "{n} places at the same spot"
          if (fan) p.pin.setAttribute("aria-description", L("{n} places at the same spot", { n: list.length }));
          else p.pin.removeAttribute("aria-description");
          let leg = p.at.querySelector("[data-map-leg]");
          if (!fan) return leg?.remove();
          if (!leg) {
            leg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
            leg.setAttribute("data-map-leg", "");
            leg.setAttribute("aria-hidden", "true");
            leg.innerHTML = "<line/>";
            p.at.prepend(leg);
          }
          for (const [k, v] of Object.entries({ x1: 0, y1: 0, x2: dx, y2: dy })) leg.firstChild.setAttribute(k, v);
        });
      }
    }
    function mark() {
      for (const [k, p] of pins) {
        if (!p.pin) continue;
        const on = k === selected;
        p.pin.toggleAttribute("data-selected", on);
        p.pin.setAttribute("aria-expanded", String(on && !!popup));
        if (!cfg.popup) p.pin.removeAttribute("aria-expanded");
        p.held.raise(on || p.row.alarm);
      }
    }

    // ---------------------------------------------------------------- selecting
    function select(id, { keys = false, quiet = false, show = true } = {}) {
      const r = id === null ? null : byId.get(id);
      selected = r ? id : null;
      if (!quiet) {
        set("bind", r ? r.id : null);
        if (r) api.emit("markerselect", { id: r.id, marker: r.row });
      }
      set("bindMarker", r ? r.row : null);
      if (!r) close(false);
      drawPins();
      if (!r) return;
      if (show && !within(r.at, map.view().bounds)) map.go(r.at);
      if (cfg.popup && (proto || r.title || r.row.description || r.row.href)) open(r, id, false, keys);
      else close(false);
      mark();
    }
    function close(clear) {
      const was = popup;
      if (popup) {
        popup.held.remove();
        popup = null;
        if (proto) {
          proto.hidden = true;
          el.append(proto);
        }
      }
      if (clear && selected !== null) {
        const pin = pins.get(selected)?.pin;
        const inside = was?.box.contains(document.activeElement) || was?.focused;
        select(null);
        if (inside) pin?.focus();
      }
      mark();
    }
    const href = (u) => typeof u === "string" && (!cfg.safe || /^(https?:|mailto:|tel:|[/?#.]|[^:]*$)/i.test(u.trim())) && !/^\s*javascript:/i.test(u);
    function open(r, id, plain, keys) {
      close(false);
      const at = make("div", { "data-map-at": "", style: "pointer-events:auto" });
      const box = make("div", { "data-map-popup": "", role: "dialog", "aria-label": r.title || L("Marker"), dir: "auto" });
      const x = make("button", { type: "button", "data-map-close": "", "aria-label": L("Close") });
      x.innerHTML = CLOSE;
      x.addEventListener("click", () => close(true));
      if (proto && !plain) {
        proto.hidden = false;
        box.append(proto);
      } else {
        if (r.title) box.append(make("strong", {}, r.title));
        if (r.row.description) box.append(make("p", {}, str(r.row.description)));
        if (r.row.href && href(r.row.href)) box.append(make("a", { href: r.row.href }, L("Open")));
      }
      box.append(x);
      at.append(box);
      const mine = (popup = { box, id, at: r.at, held: null, focused: keys });
      mine.held = map.hold(at, r.at, true, () => {
        if (popup !== mine) return;
        // The runtime only draws lists that are on the page.
        if (proto && !plain) api.list(proto, [byId.get(id)?.row ?? r.row]);
        fitPopup();
        requestAnimationFrame(fitPopup);
        if (keys) x.focus();
      });
    }
    // Keeps the popup inside the map: below its place when there's no room above.
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
    el.addEventListener("keydown", (e) => {
      if (e.key !== "Escape" || !popup) return;
      e.stopPropagation();
      close(true);
    });

    // ---------------------------------------------------------------- picking a place
    function drawPoint() {
      if (!cfg.bindPoint || !map) return;
      const at = place(api.state[cfg.bindPoint]);
      if (!at) {
        point?.held.remove();
        point = null;
      } else if (point) {
        point.at = at;
        point.held.move(at);
      } else {
        const holder = make("div", { "data-map-at": "" });
        const pin = make("button", { type: "button", "data-map-pin": "", "data-map-point": "", "data-draggable": "", "data-tone": "danger", "aria-label": L("Chosen place"), style: "pointer-events:auto" });
        pin.innerHTML = PIN;
        holder.append(pin);
        point = { at, held: map.hold(holder, at, false) };
        drags(pin, () => point.at, () => true, (to, done) => {
          point.at = to;
          point.held.move(to);
          if (done) api.set(cfg.bindPoint, to);
        });
        point.held.raise(true);
      }
    }

    // ---------------------------------------------------------------- drawing shapes
    // With `draw` and `bindShapes`: Draw a line, an area or a circle among the controls.
    // A line's or area's corners are clicked on the map, and Enter, a double click (a
    // second click on the last corner) or, for an area, a click on the first finishes it;
    // Backspace takes the last corner back, Escape stops. A circle is its middle and then
    // a point on its edge. The new row goes to `bindShapes` (which `shapes` reads); a
    // drawn shape clicked is picked, and Delete takes it away.
    const KINDS = ["line", "area", "circle"];
    const kinds = () => (!cfg.bindShapes ? [] : fixed.draw === true ? KINDS : Array.isArray(fixed.draw) ? KINDS.filter((k) => fixed.draw.includes(k)) : []);
    const sketch = { kind: null, pts: [], at: null, preview: null, dots: [] };
    let shapePicked = null;
    const ownRows = () => (Array.isArray(api.state[cfg.bindShapes]) ? api.state[cfg.bindShapes] : []);
    const metres = (a, b) => {
      const R = 6371008.8, f = (d) => (d * Math.PI) / 180, dLat = f(b.lat - a.lat), dLng = f(b.lng - a.lng);
      const h = Math.sin(dLat / 2) ** 2 + Math.cos(f(a.lat)) * Math.cos(f(b.lat)) * Math.sin(dLng / 2) ** 2;
      return 2 * R * Math.asin(Math.min(1, Math.sqrt(h)));
    };
    // Labels: "Click the line's corners on the map; Enter to finish", "Click the area's corners on the map; Enter to finish", "Click the circle's middle, then its edge"
    const HOW = { line: "Click the line's corners on the map; Enter to finish", area: "Click the area's corners on the map; Enter to finish", circle: "Click the circle's middle, then its edge" };
    function sketchMode(kind) {
      sketch.kind = kind && kinds().includes(kind) ? kind : null;
      sketch.pts = [];
      sketch.at = null;
      el.toggleAttribute("data-sketching", !!sketch.kind);
      for (const b of el.querySelectorAll("[data-map-action^=draw-]")) b.setAttribute("aria-pressed", String(b.dataset.mapAction === `draw-${sketch.kind}`));
      map?.drawing?.(!!sketch.kind);
      if (sketch.kind) {
        select(null);
        pickShape(null);
        say(L(HOW[sketch.kind]), true);
      } else say("");
      sketchDraw();
    }
    // The shape so far, with the pointer's place as its next corner.
    function sketchShape() {
      const pts = [...sketch.pts, ...(sketch.at && sketch.pts.length ? [sketch.at] : [])];
      if (sketch.kind === "circle") return pts.length === 2 ? { id: "sketch", type: "circle", center: pts[0], radius: Math.max(1, metres(pts[0], pts[1])), width: 2, dashed: true, fill: 0.1 } : null;
      return pts.length > 1 ? { id: "sketch", type: sketch.kind === "area" && pts.length > 2 ? "area" : "line", rings: [pts], width: 2, dashed: true, fill: 0.1 } : null;
    }
    function sketchDraw() {
      sketch.preview?.remove();
      sketch.preview = null;
      sketch.dots.forEach((d) => d.remove());
      sketch.dots = [];
      if (!map || !sketch.kind) return;
      const sh = sketchShape();
      if (sh) sketch.preview = map.draw(sh, colorOf({ tone: "primary" }), null);
      // Each corner so far, a dot.
      sketch.dots = sketch.pts.map((p) => {
        const h = make("div", { "data-map-at": "" });
        h.append(make("span", { "data-map-vertex": "" }));
        return map.hold(h, p, true);
      });
    }
    function sketchAt(p) {
      const q = map.project(p), near = (a, d) => { const r = map.project(a); return Math.hypot(r.x - q.x, r.y - q.y) < d; };
      const first = sketch.pts[0], last = sketch.pts.at(-1);
      if (sketch.kind === "circle") {
        sketch.pts.push(p);
        if (sketch.pts.length === 2) return sketchFinish();
      } else if (last && near(last, 6)) {
        // A second click on the last corner: a double click, which finishes it.
        return sketchFinish();
      } else if (sketch.kind === "area" && sketch.pts.length >= 3 && near(first, 10)) return sketchFinish();
      else sketch.pts.push(p);
      // Labels: "Corner {n}"
      live.textContent = L("Corner {n}", { n: sketch.pts.length });
      sketchDraw();
    }
    // Labels: "A line needs two corners or more", "An area needs three corners or more", "Line added", "Area added", "Circle added"
    function sketchFinish() {
      const kind = sketch.kind, pts = sketch.pts.map((p) => [Math.round(p.lat * 1e6) / 1e6, Math.round(p.lng * 1e6) / 1e6]);
      if (!kind) return;
      const enough = kind === "circle" ? pts.length === 2 : kind === "line" ? pts.length >= 2 : pts.length >= 3;
      if (!enough) return say(kind === "area" ? L("An area needs three corners or more") : L("A line needs two corners or more"), true);
      const rows = ownRows(), taken = new Set(rows.map((r) => str(r?.id)));
      let n = rows.length + 1, id = `${kind}-${n}`;
      while (taken.has(id)) id = `${kind}-${++n}`;
      const row = kind === "circle" ? { id, type: "circle", center: pts[0], radius: Math.round(metres(sketch.pts[0], sketch.pts[1])) } : { id, type: kind, points: pts };
      api.set(cfg.bindShapes, [...rows, row]);
      api.emit("draw", { kind, row });
      sketchMode(null);
      live.textContent = L({ line: "Line added", area: "Area added", circle: "Circle added" }[kind]);
    }
    // A drawn shape picked (by its id), shown in the primary colour; null lets go.
    // Labels: "Shape picked: Delete takes it away", "Shape removed"
    function pickShape(id) {
      shapePicked = id;
      const primary = colorOf({ tone: "primary" });
      drawn.forEach((d) => d.handle.paint(id !== null && str(d.s.id) === str(id) ? primary : colorOf(d.s)));
      if (id !== null) say(L("Shape picked: Delete takes it away"), true);
    }
    function removeShape() {
      if (shapePicked === null) return false;
      const rows = ownRows(), gone = rows.find((r) => str(r?.id) === str(shapePicked));
      if (!gone) return false;
      api.set(cfg.bindShapes, rows.filter((r) => r !== gone));
      api.emit("erase", { kind: gone.type, row: gone });
      shapePicked = null;
      say("");
      live.textContent = L("Shape removed");
      return true;
    }
    canvas.addEventListener("pointermove", (e) => {
      if (!sketch.kind || !map || !sketch.pts.length) return;
      const c = container();
      sketch.at = map.unproject(e.clientX - c.left, e.clientY - c.top);
      sketchDraw();
    });
    el.addEventListener("keydown", (e) => {
      if (e.target.closest?.("input, textarea, select, [contenteditable]")) return;
      if (sketch.kind) {
        if (e.key === "Enter") return e.preventDefault(), sketchFinish();
        if (e.key === "Escape") return e.preventDefault(), sketchMode(null);
        if (e.key === "Backspace") return e.preventDefault(), sketch.pts.pop(), sketchDraw();
      } else if ((e.key === "Delete" || e.key === "Backspace") && removeShape()) e.preventDefault();
      else if (e.key === "Escape" && shapePicked !== null) pickShape(null), say("");
    });

    // ---------------------------------------------------------------- where the viewer is
    function locate() {
      if (!navigator.geolocation) return say(L("Your location isn't available"));
      say(L("Finding you…"), true);
      navigator.geolocation.getCurrentPosition(
        ({ coords }) => {
          say("");
          const at = { lat: coords.latitude, lng: coords.longitude };
          me?.held.remove();
          me?.ring.remove();
          const holder = make("div", { "data-map-at": "" });
          holder.append(make("div", { "data-map-me": "", role: "img", "aria-label": L("You are here") }));
          const ring = { id: "me", type: "circle", center: at, radius: Math.max(5, coords.accuracy || 0), width: 1, tone: "info" };
          me = { held: map.hold(holder, at, false), ring: map.draw(ring, colorOf(ring), null) };
          const found = { ...round(at), accuracy: coords.accuracy };
          set("bindLocation", found);
          api.emit("locate", found);
          map.go(at, Math.max(map.view().zoom, 14));
        },
        () => say(L("Your location isn't available")),
        { enableHighAccuracy: true, timeout: 15000 },
      );
    }

    // ---------------------------------------------------------------- the view
    function moved() {
      if (!map) return;
      el.removeAttribute("data-zooming");
      drawPins();
      fitPopup();
      laid.forEach((l) => l.handle.redraw?.(l.o));
      const v = map.view();
      const r5 = (n) => Math.round(n * 1e5) / 1e5;
      const view = { center: { lat: r5(v.center.lat), lng: r5(v.center.lng) }, zoom: v.zoom, bounds: Object.fromEntries(Object.entries(v.bounds).map(([k, n]) => [k, r5(n)])) };
      set("bindView", view);
      if (cfg.bindVisible) set("bindVisible", rows.filter((r) => within(r.at, v.bounds)).map((r) => r.id));
      api.emit("viewchange", view);
    }
    function clicked(at) {
      if (sketch.kind) return sketchAt(at);
      if (shapePicked !== null) pickShape(null), say("");
      if (popup && !cfg.bindPoint) close(true);
      else if (selected !== null && !cfg.bindPoint) select(null);
      const p = round(at);
      if (cfg.bindPoint) api.set(cfg.bindPoint, p);
      api.emit("mapclick", p);
    }
    function command(name, a) {
      if (!map) return;
      if (name === "map.select") select(a.id === null || a.id === undefined ? null : str(a.id));
      else if (name === "map.center") {
        const to = a.id !== undefined && a.id !== null ? byId.get(str(a.id))?.at : place(a);
        if (to) map.go(to, Number.isFinite(num(a.zoom)) ? num(a.zoom) : undefined);
      } else if (name === "map.fit") {
        const ids = Array.isArray(a.ids) ? a.ids.map(str) : null;
        if (!ids) return void fitAll();
        const b = boundsOf(ids.map((i) => byId.get(i)?.at).filter(Boolean));
        if (b) map.fit(b, closest());
      } else if (name === "map.zoomin") map.zoomBy(1);
      else if (name === "map.zoomout") map.zoomBy(-1);
      else if (name === "map.locate") locate();
    }
    const calls = (e) => (typeof e.detail?.name === "string" && e.detail.name.startsWith("map.") ? e.detail : null);
    const onCall = (e) => {
      const d = calls(e);
      if (!d) return;
      const named = d.args?.map !== undefined && d.args?.map !== null;
      const mine = named ? el.id !== "" && str(d.args.map) === el.id : el.contains(e.target) || document.querySelectorAll('[data-i-x="ui-map"]').length === 1;
      if (mine) command(d.name, d.args || {});
    };
    document.addEventListener("irender:call", onCall);
    el.querySelector("[data-map-controls]")?.addEventListener("click", (e) => {
      const a = e.target.closest("[data-map-action]")?.getAttribute("data-map-action");
      if (!map || !a) return;
      if (a === "zoom-in") map.zoomBy(1);
      else if (a === "zoom-out") map.zoomBy(-1);
      else if (a === "fit") fitAll();
      else if (a.startsWith("draw-")) sketchMode(sketch.kind === a.slice(5) ? null : a.slice(5));
      else locate();
    });

    // ---------------------------------------------------------------- starting
    let starting = false;
    async function start() {
      if (starting || map) return;
      starting = true;
      collect();
      say(L("Loading map…"), true);
      const center = place(props.center), b = boundsOf(all());
      const middle = b ? { lat: (b.north + b.south) / 2, lng: (b.east + b.west) / 2 } : { lat: 20, lng: 0 };
      const zoom = num(props.zoom);
      const box = make("div", { style: "position:absolute;inset:0" });
      canvas.replaceChildren(box);
      let ready = false;
      try {
        const made = await (cfg.provider === "google" ? google : leaflet)(box, {
          el, source: cfg.source, gestures: cfg.gestures, mapType: fixed.mapType,
          center: center ?? middle,
          zoom: Number.isFinite(zoom) ? zoom : center ? 13 : b ? 11 : 1,
          // With a zoom and no center, it opens on the middle of what's there.
          bounds: b && (props.fit ?? !center) && (center || !Number.isFinite(zoom)) ? b : null,
          closest: closest(),
          minZoom: fixed.minZoom, maxZoom: fixed.maxZoom,
          dark, say, failed, restart,
          moved: () => ready && moved(),
          zooming: () => el.setAttribute("data-zooming", ""),
          clicked: (at) => ready && clicked(at),
        });
        // Google can refuse the key while the map is being made.
        if (el.hasAttribute("data-error")) return;
        map = made;
      } catch (e) {
        return failed(e);
      } finally {
        starting = false;
      }
      ready = true;
      say("");
      el.setAttribute("data-ready", "");
      self.themed();
      drawOverlays();
      drawShapes();
      drawPoint();
      alarms();
      const at = cfg.bind ? api.state[cfg.bind] : null;
      if (at !== null && at !== undefined) select(str(at), { quiet: true, show: false });
      moved();
    }
    function restart() {
      drawn.forEach((d) => d.handle.remove());
      drawn = [];
      laid.forEach((l) => l.handle.remove());
      laid.clear();
      close(false);
      pins.forEach((p) => p.held.remove());
      pins.clear();
      point?.held.remove();
      point = me = null;
      const view = map.view();
      map = null;
      Object.assign(props, { center: view.center, zoom: view.zoom, fit: false });
      start();
    }
    const self = {
      themed() {
        el.toggleAttribute("data-dark", dark());
        if (!map) return;
        map.themed();
        drawn.forEach((d) => d.handle.paint(colorOf(d.s)));
        laid.forEach((l) => l.handle.redraw?.(l.o));
      },
    };
    maps.add(new WeakRef(self));
    watch();
    el.toggleAttribute("data-dark", dark());
    // The library and the tiles are only fetched once the map is nearly in view. One that
    // already is doesn't wait for the observer, which needs a frame to be drawn.
    const near = 300;
    const io = new IntersectionObserver((es) => es.some((e) => e.isIntersecting) && (io.disconnect(), start()), { rootMargin: `${near}px` });
    setTimeout(() => {
      const r = el.getBoundingClientRect();
      if (r.width && r.top < innerHeight + near && r.bottom > -near) start();
      else io.observe(el);
    });
    new ResizeObserver(() => {
      el.style.setProperty("--map-width", `${Math.max(160, el.clientWidth - 24)}px`);
      el.style.setProperty("--map-height", `${Math.max(80, el.clientHeight - 80)}px`);
      if (!map || !el.clientWidth) return;
      map.resize();
      laid.forEach((l) => l.handle.redraw?.(l.o));
      fitPopup();
    }).observe(el);

    let last = { ...props }, lastBind = cfg.bind ? api.state[cfg.bind] : null;
    return {
      self,
      get map() {
        return map;
      },
      update() {
        if (cfg.reactive) read();
        if (!map) return;
        const changed = (k) => !same(last[k], props[k]);
        const places = ["markers", "geojson", "toneBy", "shapes", "filter", "alarmBy", "effectBy"].some(changed);
        const moves = changed("center") || changed("zoom"), redraw = places || changed("cluster") || changed("spread") || changed("draggable");
        const relaid = changed("overlays"), before = last;
        last = { ...props };
        if (relaid) drawOverlays();
        if (places) {
          collect();
          alarms();
          if (!same(before.shapes, props.shapes) || !same(before.geojson, props.geojson)) {
            drawShapes();
            if (shapePicked !== null) pickShape(shapes.some((x) => str(x.id) === str(shapePicked)) ? shapePicked : null);
          }
        }
        if (redraw) {
          drawPins();
          if (selected !== null && !byId.has(selected)) select(null);
          else if (selected !== null) {
            const r = byId.get(selected);
            set("bindMarker", r.row);
            if (popup?.id === selected) {
              popup.held.move(r.at);
              if (proto) api.list(proto, [r.row]);
            }
          }
          if (props.fit === "always") fitAll();
          if (cfg.bindVisible) {
            const b = map.view().bounds;
            set("bindVisible", rows.filter((r) => within(r.at, b)).map((r) => r.id));
          }
        }
        if (moves) {
          const c = place(props.center) ?? map.view().center, z = num(props.zoom);
          map.go(c, Number.isFinite(z) ? z : undefined);
        }
        if (cfg.bind) {
          const now = api.state[cfg.bind] ?? null;
          if (!same(now, lastBind ?? null)) {
            lastBind = now;
            if (str(now) !== str(selected)) select(now === null ? null : str(now), { quiet: true });
          }
        }
        drawPoint();
      },
    };
  };
})();
