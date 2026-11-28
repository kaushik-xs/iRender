//! Draws maps in headless Chrome and drives them. The tiles are a data URL, so nothing is
//! asked of OpenStreetMap, and Google Maps is a small stand-in.
//!
//! Ignored by default because it needs Chrome, and Leaflet from the CDN:
//!   cargo test -p irender-map-kit --test browser -- --ignored
//! Set CHROME=/path/to/chrome if it isn't found automatically, and LEAFLET=/path/to/dist
//! (a folder with leaflet.js and leaflet.css) to test offline.

use irender_core::Format;
use irender_html::Engine;
use irender_map_kit::{MapKit, Tiles};
use serde_json::{Value as Json, json};
use std::{path::PathBuf, process::Command};

const TILE: &str = "data:image/gif;base64,R0lGODlhAQABAIAAAMLCwgAAACH5BAAAAAAALAAAAAABAAEAAAICRAEAOw==";

fn chrome() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("CHROME") {
        return Some(path.into());
    }
    ["/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", "google-chrome", "google-chrome-stable", "chromium"]
        .iter()
        .map(PathBuf::from)
        .find(|p| Command::new(p).arg("--version").output().is_ok_and(|o| o.status.success()))
}

fn kit() -> MapKit {
    let mut kit = MapKit { tiles: Tiles { url: TILE.into(), ..Tiles::default() }, google_key: Some("key".into()), ..MapKit::default() };
    if let Ok(path) = std::env::var("LEAFLET") {
        kit.leaflet = format!("file://{}/", std::fs::canonicalize(path).unwrap().display());
    }
    kit
}

/// Renders `doc` with `driver` appended, runs it in Chrome and returns what the driver
/// wrote into `<pre id="result">`, one line per check. `head` goes before the page's own
/// scripts.
fn run(name: &str, doc: Json, head: &str, driver: &str) -> Option<Vec<String>> {
    let Some(chrome) = chrome() else {
        eprintln!("skipping: Chrome not found (set CHROME=/path/to/chrome)");
        return None;
    };
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    irender_map_kit::register_with(&mut engine.registry, kit());
    let mut doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    irender_ui_kit::apply_theme(&mut doc);
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let script = format!(
        "<script>\nconst __errors = []; window.addEventListener('error', (e) => __errors.push(e.message));\n\
         const __ce = console.error; console.error = (...a) => {{ __errors.push(a.join(' ')); __ce(...a); }};\n\
         window.addEventListener('load', async () => {{\n{PRELUDE}\n try {{\n{driver}\n }} catch (e) {{ log('threw', e.message, e.stack); }}\n{POSTLUDE}\n}});\n</script>"
    );
    let html = engine.render(&ir, &json!({})).unwrap().document("test").replace("</head>", &format!("{head}</head>")).replace("</body>", &format!("{script}</body>"));
    let dir = std::env::temp_dir().join(format!("irender-map-kit-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("page.html");
    std::fs::write(&file, html).unwrap();
    // `--no-sandbox`: CI runners block the sandbox's user namespaces; the page is ours.
    let load = || {
        let out = Command::new(&chrome)
            .args(["--headless=new", "--no-sandbox", "--disable-gpu", "--blink-settings=preferredColorScheme=1", "--allow-file-access-from-files", "--window-size=1000,900", "--force-prefers-reduced-motion"])
            .args(["--virtual-time-budget=40000", "--dump-dom"])
            .arg(format!("file://{}", file.display()))
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    const RESULT: &str = "<pre id=\"result\">";
    // On a very busy machine Chrome now and then gives the page back before its driver is
    // done, so without a result it's loaded once more.
    let mut dom = load();
    if !dom.contains(RESULT) {
        dom = load();
    }
    let _ = std::fs::remove_dir_all(&dir);
    let start = dom.find(RESULT).unwrap_or_else(|| panic!("driver did not run:\n{dom}")) + RESULT.len();
    let result = dom[start..start + dom[start..].find("</pre>").unwrap()].replace("&quot;", "\"").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&");
    Some(result.lines().map(String::from).collect())
}

const PRELUDE: &str = r##"
  const out = [], log = (...a) => out.push(a.join(" "));
  const wait = (ms) => new Promise((r) => setTimeout(r, ms));
  const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state, set = (k, v) => root.__irender.set(k, v);
  const $ = (id, sel) => document.querySelector(`#${id} ${sel}`), $$ = (id, sel) => [...document.querySelectorAll(`#${id} ${sel}`)];
  const pin = (id, key) => $(id, `[data-map-pin="${key}"]`);
  const popup = (id) => $(id, "[data-map-popup]")?.textContent ?? "none";
  const ready = async (...ids) => { for (let i = 0; i < 500 && !ids.every((id) => document.getElementById(id).hasAttribute("data-ready")); i++) await wait(50); await wait(100); };
  const at = (el) => { const b = el.getBoundingClientRect(); return [b.left + b.width / 2, b.top + b.height / 2]; };
  const point = (el, type, x, y) => el.dispatchEvent(new PointerEvent(type, { bubbles: true, clientX: x, clientY: y, pointerId: 1, button: 0 }));
  const drag = (el, dx, dy) => { const [x, y] = at(el); point(el, "pointerdown", x, y); point(el, "pointermove", x + dx, y + dy); point(el, "pointerup", x + dx, y + dy); };
  const key = (el, k, more = {}) => el.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true, ...more }));
"##;

const POSTLUDE: &str = r##"
  const pre = document.createElement("pre"); pre.id = "result";
  pre.textContent = out.concat(__errors.map((e) => "error " + e)).join("\n");
  document.body.append(pre);
"##;

fn check(lines: Vec<String>, expected: &[&str]) {
    assert_eq!(lines, expected, "\n{}", lines.join("\n"));
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn markers_popups_and_selection_in_chrome() {
    let doc = json!({ "version": 1,
        "state": { "picked": null, "row": null, "seen": [], "status": null, "rows": [
            { "id": "a", "lat": 51.5, "lng": -0.14, "title": "A", "description": "First", "status": "open" },
            { "id": "b", "lat": 51.51, "lng": -0.1, "title": "B", "status": "late", "label": "2" },
            { "id": "c", "latitude": 51.49, "longitude": -0.12, "name": "C" },
            { "id": "bad", "lat": "x", "lng": 2 } ] },
        "root": { "type": "element", "children": [
            { "type": "ui-map", "attrs": { "id": "m" }, "props": { "markers": "{{ rows }}", "bind": "picked", "bindMarker": "row", "bindVisible": "seen",
                "filter": { "status": "{{ status }}" }, "toneBy": { "field": "status", "tones": { "late": "danger" } } } },
            { "type": "ui-map", "attrs": { "id": "t" }, "props": { "markers": "{{ rows }}" }, "children": [
                { "type": "ui-map-popup", "children": [{ "type": "text", "props": { "value": "It is {{ marker.title }}" } }] } ] } ] } });
    let driver = r##"
      await ready("m", "t");
      log("pins", $$("m", "[data-map-pin]").length, "seen", S().seen.join());
      log("names", $$("m", "[data-map-pin]").map((p) => p.getAttribute("aria-label")).join());
      log("tone", pin("m", "b").dataset.tone, pin("m", "a").dataset.tone, "label", pin("m", "b").textContent);
      pin("m", "a").click(); await wait(50);
      log("picked", S().picked, S().row.title, popup("m"), pin("m", "a").hasAttribute("data-selected"));
      set("picked", "b"); await wait(50);
      log("from state", popup("m"), S().row.id);
      key($("m", "[data-map-close]"), "Escape"); await wait(50);
      log("closed", S().picked, S().row, popup("m"));
      pin("m", "a").click(); await wait(30);
      set("rows", S().rows.map((r) => (r.id === "a" ? { ...r, title: "A2" } : r))); await wait(50);
      log("row follows", S().row.title, pin("m", "a").getAttribute("aria-label"));
      set("rows", S().rows.filter((r) => r.id !== "a")); await wait(50);
      log("removed", S().picked, popup("m"), $$("m", "[data-map-pin]").length);
      set("status", "late"); await wait(50);
      log("filtered", $$("m", "[data-map-pin]").map((p) => p.dataset.mapPin).join(), S().seen.join());
      pin("t", "b").click(); await wait(50);
      log("template", popup("t"));
      pin("t", "c").click(); await wait(50);
      log("template again", popup("t"), $$("t", "[data-map-popup]").length);
      document.dispatchEvent(new CustomEvent("irender:call", { detail: { name: "map.select", args: { map: "t", id: null } } })); await wait(50);
      log("by call", popup("t"));
    "##;
    let Some(lines) = run("markers", doc, "", driver) else { return };
    check(
        lines,
        &[
            "pins 3 seen a,b,c",
            "names A,B,C",
            "tone danger  label 2",
            "picked a A AFirst true",
            "from state B b",
            "closed   none",
            "row follows A2 A2",
            "removed  none 2",
            "filtered b b",
            "template It is B",
            "template again It is  1",
            "by call none",
        ],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn clusters_shapes_and_moving_in_chrome() {
    let rows: Vec<Json> = (0..40).map(|i| json!({ "id": format!("r{i}"), "lat": 48.0 + f64::from(i % 2) * 4.0 + f64::from(i) * 0.0005, "lng": 2.0 + f64::from(i % 2) * 8.0, "title": format!("R{i}") })).collect();
    let doc = json!({ "version": 1,
        "state": { "spot": null, "shape": null, "r": 300, "view": null, "cluster": true,
            "stops": [{ "id": "a", "lat": 48.86, "lng": 2.33, "title": "A" }, { "id": "b", "lat": 48.85, "lng": 2.35, "title": "B" }] },
        "root": { "type": "element", "children": [
            { "type": "ui-map", "attrs": { "id": "c" }, "props": { "markers": rows, "cluster": "{{ cluster }}", "bindView": "view", "height": 200 } },
            { "type": "ui-map", "attrs": { "id": "p" }, "props": { "center": [12.97, 77.59], "zoom": 12, "bindPoint": "spot", "height": 200 } },
            { "type": "ui-map", "attrs": { "id": "s" }, "props": { "markers": "{{ stops }}", "bindMarkers": "stops", "draggable": true, "popup": false, "height": 200 } },
            { "type": "ui-map", "attrs": { "id": "g" }, "props": { "bindShape": "shape", "height": 200,
                "shapes": [{ "id": "z", "type": "circle", "center": [48.85, 2.35], "radius": "{{ r }}", "title": "Zone" }],
                "geojson": { "type": "FeatureCollection", "features": [
                    { "type": "Feature", "id": "area", "properties": { "title": "Area" }, "geometry": { "type": "Polygon", "coordinates": [[[2.3, 48.8], [2.4, 48.8], [2.4, 48.9], [2.3, 48.8]]] } },
                    { "type": "Feature", "properties": {}, "geometry": { "type": "LineString", "coordinates": [[2.3, 48.8], [2.4, 48.9]] } },
                    { "type": "Feature", "id": "pt", "properties": { "title": "Point" }, "geometry": { "type": "Point", "coordinates": [2.35, 48.85] } } ] } } } ] } });
    let driver = r##"
      await ready("c", "p", "s", "g");
      log("clusters", $$("c", "[data-map-cluster]").map((b) => b.textContent).join(), $$("c", "[data-map-pin]").length);
      const z = S().view.zoom;
      $("c", "[data-map-cluster]").click(); await wait(300);
      log("zoomed in", S().view.zoom > z);
      $("c", '[data-map-action="fit"]').click(); await wait(200);
      set("cluster", false); await wait(100);
      log("apart", $$("c", "[data-map-cluster]").length, $$("c", "[data-map-pin]").length);
      const box = document.querySelector("#p .leaflet-container"), [x, y] = at(box);
      for (const t of ["mousedown", "mouseup", "click"]) box.dispatchEvent(new MouseEvent(t, { bubbles: true, clientX: x + 20, clientY: y, view: window }));
      await wait(400);
      const first = S().spot;
      log("spot", Math.abs(first.lat - 12.97) < 0.01, first.lng > 77.59);
      drag($("p", "[data-map-point]"), 30, 20); await wait(50);
      log("spot moved", S().spot.lng > first.lng, S().spot.lat < first.lat);
      const before = S().stops[0];
      drag(pin("s", "a"), 25, 0); await wait(50);
      log("dragged", S().stops[0].lng > before.lng, S().stops[0].title, S().stops[1].lng);
      const lat = S().stops[1].lat;
      key(pin("s", "b"), "ArrowUp", { altKey: true }); await wait(50);
      log("by key", S().stops[1].lat > lat);
      log("shapes", $$("g", ".leaflet-overlay-pane path").length, $$("g", "[data-map-pin]").length);
      const d = () => $$("g", ".leaflet-overlay-pane path")[0].getAttribute("d"), small = d();
      set("r", 3000); await wait(100);
      log("circle follows", d() !== small);
      $$("g", ".leaflet-overlay-pane path")[1].dispatchEvent(new MouseEvent("click", { bubbles: true, view: window, clientX: 10, clientY: 10 })); await wait(100);
      log("clicked", S().shape, popup("g"));
    "##;
    let Some(lines) = run("shapes", doc, "", driver) else { return };
    check(
        lines,
        &[
            "clusters 20,20 0",
            "zoomed in true",
            "apart 0 40",
            "spot true true",
            "spot moved true true",
            "dragged true A 2.35",
            "by key true",
            "shapes 3 1",
            "circle follows true",
            "clicked area Area",
        ],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn alarms_in_chrome() {
    let doc = json!({ "version": 1,
        "state": { "on": ["b"], "alarms": [], "cluster": false, "rows": [
            { "id": "a", "lat": 10, "lng": 20, "title": "A" },
            { "id": "b", "lat": 11, "lng": 21, "title": "B" },
            { "id": "c", "lat": 12, "lng": 22, "title": "C", "effect": "glow", "tone": "warning" },
            { "id": "d", "lat": 13, "lng": 23, "title": "D", "alarm": true, "effect": "blink", "label": "7", "tone": "info" },
            { "id": "e", "lat": 14, "lng": 24, "title": "E", "kind": "hot" } ] },
        "root": { "type": "ui-map", "attrs": { "id": "m" },
            "props": { "markers": "{{ rows }}", "alarmBy": { "field": "id", "values": "{{ on }}" }, "effectBy": { "field": "kind", "effects": { "hot": "bounce" } }, "bindAlarms": "alarms", "cluster": "{{ cluster }}" } } });
    let driver = r##"
      let heard = [];
      document.getElementById("m").addEventListener("alarm", (e) => heard.push(e.detail.id));
      await ready("m");
      const look = (k) => { const p = pin("m", k); return [k, p.hasAttribute("data-alarm"), p.dataset.effect, p.dataset.tone, p.textContent, p.getAttribute("aria-label")].join("|"); };
      log(["a", "b", "c", "d", "e"].map(look).join(" "));
      log("ids", S().alarms.join(), "said", $("m", "[data-map-live]").textContent);
      log("ring", getComputedStyle(pin("m", "b"), "::before").content, "none", getComputedStyle(pin("m", "a"), "::before").content);
      set("on", ["a", "b"]); await wait(50);
      log("started", look("a"), S().alarms.join(), $("m", "[data-map-live]").textContent, heard.join());
      set("on", []); await wait(50);
      log("cleared", look("a"), look("b"), S().alarms.join());
      set("rows", S().rows.map((r, i) => ({ ...r, lat: 10 + i * 1e-5, lng: 20 }))); set("cluster", true);
      $("m", '[data-map-action="zoom-out"]').click(); await wait(200);
      for (let i = 0; i < 8; i++) { $("m", '[data-map-action="zoom-out"]').click(); await wait(60); }
      const g = $("m", "[data-map-cluster]");
      log("group", g.textContent, g.hasAttribute("data-alarm"), g.dataset.effect, g.getAttribute("aria-label"));
    "##;
    let Some(lines) = run("alarms", doc, "", driver) else { return };
    check(
        lines,
        &[
            "a|false||||A b|true|pulse|danger|!|B, alarm c|false|glow|warning||C d|true|blink|info|7|D, alarm e|false|bounce|||E",
            "ids b,d said ",
            "ring \"\" none none",
            "started a|true|pulse|danger|!|A, alarm a,b,d Alarm: A a",
            "cleared a|false||||A b|false||||B d",
            "group 5 true blink 5 places here, 1 with an alarm: zoom in",
        ],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn overlays_in_chrome() {
    let doc = json!({ "version": 1,
        "state": { "on": true, "strength": 0.5, "points": [[10, 20], [10.2, 20.2, 3]] },
        "root": { "type": "ui-map", "attrs": { "id": "m" }, "props": { "overlays": [
            { "id": "pic", "type": "image", "url": TILE, "bounds": [[9, 19], [11, 21]], "opacity": "{{ strength }}", "title": "Plan" },
            { "id": "more", "type": "tiles", "url": TILE, "attribution": "Someone <b>", "attributionUrl": "https://example.com/", "show": "{{ on }}" },
            { "id": "heat", "type": "heat", "points": "{{ points }}", "radius": 40, "show": "{{ on }}" },
            { "id": "bad", "type": "image", "url": TILE } ] } } });
    let driver = r##"
      await ready("m");
      const pic = () => $("m", "img.leaflet-image-layer"), credit = () => $("m", ".leaflet-control-attribution").textContent;
      const ink = () => { const c = $("m", "[data-map-heat] canvas"); return c.getContext("2d").getImageData(0, 0, c.width, c.height).data.reduce((n, v, i) => n + (i % 4 === 3 && v ? 1 : 0), 0); };
      log("on", $$("m", "img.leaflet-image-layer").length, pic().alt, pic().style.opacity, $$("m", ".leaflet-overlays-pane .leaflet-tile").length > 0, credit().includes("Someone <b>"));
      const few = ink();
      log("heat", few > 0, getComputedStyle($("m", "[data-map-heat] canvas")).opacity);
      const node = pic();
      set("strength", 0.9); await wait(50);
      log("stronger", pic().style.opacity, pic() === node);
      set("points", [...S().points, [10.9, 19.1, 3]]); await wait(50);
      log("more heat", ink() > few);
      set("on", false); await wait(50);
      log("off", $$("m", "[data-map-heat]").length, $$("m", ".leaflet-overlays-pane .leaflet-tile").length, credit().includes("Someone"));
      set("on", true); await wait(50);
      log("on again", $$("m", "[data-map-heat]").length, credit().includes("Someone"));
    "##;
    let Some(lines) = run("overlays", doc, "", driver) else { return };
    check(
        lines,
        &["on 1 Plan 0.5 true true", "heat true 0.7", "stronger 0.9 true", "more heat true", "off 0 0 false", "on again 1 true"],
    );
}

/// The part of Google's API the kit uses, on a flat world of 100px a degree.
const GOOGLE: &str = r##"<script>
(() => {
  class LatLng { constructor(a, b) { this.a = a; this.b = b; } lat() { return this.a; } lng() { return this.b; } }
  class Map {
    constructor(el, o) { Object.assign(this, { el, o, on: {}, c: o.center, z: o.zoom, overlayMapTypes: { list: [], push(t) { this.list.push(t); }, getArray() { return this.list; }, removeAt(i) { this.list.splice(i, 1); } } }); window.__gmap = this; setTimeout(() => this.fire("idle"), 10); }
    addListener(n, f) { (this.on[n] ||= []).push(f); }
    fire(n, e) { (this.on[n] || []).splice(0, 99, ...(this.on[n] || [])); (this.on[n] || []).forEach((f) => f(e)); }
    getCenter() { return new LatLng(this.c.lat, this.c.lng); }
    getZoom() { return this.z; }
    getBounds() { return { toJSON: () => ({ north: this.c.lat + 2, south: this.c.lat - 2, east: this.c.lng + 2, west: this.c.lng - 2 }) }; }
    setCenter(p) { this.c = { lat: p.lat, lng: p.lng }; this.fire("idle"); }
    panTo(p) { this.setCenter(p); }
    setZoom(z) { this.z = z; this.fire("idle"); }
    fitBounds(b) { this.fitted = b; this.c = { lat: (b.north + b.south) / 2, lng: (b.east + b.west) / 2 }; this.z = 20; setTimeout(() => this.fire("idle"), 5); }
  }
  class OverlayView {
    static preventMapHitsAndGesturesFrom() {}
    setMap(m) {
      if (!m) return this.onRemove();
      this.map = m;
      // Later, as Google does.
      setTimeout(() => { this.onAdd(); this.draw(); m.addListener("idle", () => this.el.isConnected && this.draw()); }, 5);
    }
    getPanes() { return { overlayLayer: this.map.el, floatPane: this.map.el, overlayMouseTarget: this.map.el }; }
    getProjection() {
      const c = this.map.c, to = (p) => ({ x: 500 + (p.lng() - c.lng) * 100, y: 200 - (p.lat() - c.lat) * 100 });
      return { fromLatLngToDivPixel: to, fromLatLngToContainerPixel: to, fromContainerPixelToLatLng: (p) => new LatLng(c.lat - (p.y - 200) / 100, c.lng + (p.x - 500) / 100) };
    }
  }
  const drawn = (window.__drawn = []);
  class Shape { constructor(o) { this.o = o; drawn.push(this); } setMap(m) { this.o.map = m; } setOptions(o) { Object.assign(this.o, o); } addListener(n, f) { this[n] = f; } }
  const ground = (window.__ground = []);
  class GroundOverlay { constructor(url, b, o) { Object.assign(this, { url, b, o: { ...o } }); ground.push(this); } setOpacity(n) { this.o.opacity = n; } setMap() {} }
  class ImageMapType { constructor(o) { this.o = o; } setOpacity(n) { this.o.opacity = n; } }
  window.google = { maps: { GroundOverlay, ImageMapType, Size: class {}, Map, OverlayView, LatLng, Point: class { constructor(x, y) { this.x = x; this.y = y; } }, Circle: class extends Shape {}, Polygon: class extends Shape {}, Polyline: class extends Shape {},
    event: { addListenerOnce: (m, n, f) => { let done = false; m.addListener(n, () => { if (!done) { done = true; f(); } }); } } } };
})();
</script>"##;

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn google_maps_in_chrome() {
    let doc = json!({ "version": 1,
        "state": { "strength": 0.5, "picked": null, "spot": null, "shape": null, "rows": [{ "id": "a", "lat": 10, "lng": 20, "title": "A" }, { "id": "b", "lat": 11, "lng": 21, "title": "B" }] },
        "root": { "type": "ui-map", "attrs": { "id": "g" }, "props": { "provider": "google", "markers": "{{ rows }}", "bind": "picked", "bindMarkers": "rows", "bindPoint": "spot", "draggable": true, "maxZoom": 12,
            "shapes": [{ "id": "z", "type": "circle", "center": [10, 20], "radius": 500, "tone": "danger" }, { "id": "l", "type": "line", "dashed": true, "points": [[10, 20], [11, 21]] }], "overlays": [
                { "id": "pic", "type": "image", "url": TILE, "bounds": { "north": 11, "south": 10, "east": 21, "west": 20 }, "opacity": "{{ strength }}" },
                { "id": "more", "type": "tiles", "url": "https://tiles.example/{z}/{x}/{y}.png", "attribution": "Someone", "show": "{{ strength < 1 }}" },
                { "id": "heat", "type": "heat", "points": "{{ rows }}" } ], "bindShape": "shape" },
            "children": [{ "type": "ui-map-popup", "children": [{ "type": "text", "props": { "value": "It is {{ marker.title }}" } }] }] } });
    let driver = r##"
      await ready("g"); await wait(200);
      const G = window.__gmap;
      log("map", G.o.gestureHandling, G.o.disableDefaultUI, G.o.colorScheme, "zoom", G.z);
      log("fitted", Object.values(G.fitted).map(Math.round).join());
      log("pins", $$("g", "[data-map-pin]").length);
      pin("g", "a").click(); await wait(100);
      log("picked", S().picked, popup("g"));
      drag(pin("g", "b"), 50, -20); await wait(50);
      log("dragged", S().rows[1].lat, S().rows[1].lng);
      log("shapes", window.__drawn.map((s) => s.constructor === google.maps.Circle ? "circle " + s.o.strokeColor : "line " + (s.o.icons ? "dashed" : "")).join());
      window.__drawn[0].click({ latLng: new google.maps.LatLng(10, 20) }); await wait(50);
      log("shape", S().shape);
      G.fire("click", { latLng: new google.maps.LatLng(10.5, 20.5) }); await wait(100);
      log("spot", JSON.stringify(S().spot), $$("g", "[data-map-point]").length);
      log("overlays", window.__ground.length, window.__ground[0].o.opacity, G.overlayMapTypes.list.length, G.overlayMapTypes.list[0].o.getTileUrl({ x: -1, y: 1 }, 2), $$("g", "[data-map-heat]").length, $("g", "[data-map-credit]").textContent);
      set("strength", 1); await wait(50);
      log("faded", window.__ground[0].o.opacity, G.overlayMapTypes.list.length, $("g", "[data-map-credit]"));
      $("g", '[data-map-action="zoom-out"]').click(); await wait(50);
      log("zoom", G.z);
      const was = G;
      document.documentElement.setAttribute("data-theme", "dark"); document.documentElement.style.setProperty("--color-surface", "#000"); document.querySelector("#g").style.setProperty("--color-surface", "#000"); await wait(300);
      log("dark", window.__gmap !== was, window.__gmap.o.colorScheme, window.__gmap.z, $$("g", "[data-map-pin]").length);
      const ce = console.error; console.error = () => {};
      window.gm_authFailure(); await wait(20);
      console.error = ce;
      log("refused", $("g", "[data-map-failed]").textContent);
    "##;
    let Some(lines) = run("google", doc, GOOGLE, driver) else { return };
    check(
        lines,
        &[
            "map cooperative true LIGHT zoom 12",
            "fitted 11,10,21,20",
            "pins 2",
            "picked a It is A",
            "dragged 11.2 21.5",
            "shapes circle rgb(220, 38, 38),line dashed",
            "shape z",
            "spot {\"lat\":10.5,\"lng\":20.5} 1",
            "overlays 1 0.5 1 https://tiles.example/2/3/1.png 1 Someone",
            "faded 1 0 ",
            "zoom 11",
            "dark true DARK 11 3",
            "refused Map unavailableGoogle Maps refused the API keyAB",
        ],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn a_map_that_cannot_load_lists_its_places() {
    let doc = json!({ "version": 1, "root": { "type": "ui-map", "attrs": { "id": "m" }, "props": { "markers": [{ "id": 1, "lat": 1, "lng": 2, "title": "Somewhere" }] } } });
    let driver = r##"
      for (let i = 0; i < 100 && !$("m", "[data-map-failed]"); i++) await wait(50);
      log($("m", "[data-map-failed]").textContent.includes("Map unavailable"), $("m", "[data-map-failed] li").textContent);
      __errors.length = 0;
    "##;
    // Leaflet's address goes nowhere.
    let head = r##"<script>
      const add = Element.prototype.append;
      Element.prototype.append = function (...els) {
        els.forEach((e) => { if (e.tagName === "SCRIPT" && /leaflet/.test(e.src)) e.src = "file:///nowhere/leaflet.js"; });
        return add.apply(this, els);
      };
    </script>"##;
    let Some(lines) = run("failed", doc, head, driver) else { return };
    check(lines, &["true Somewhere"]);
}

/// Markers on the same spot fan out round it on lines to it (not with `spread: false`),
/// and shapes are drawn by hand: a line by its corners and Enter, an area closed on its
/// first corner, a circle by its middle and edge; picked, and taken away with Delete.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn same_spot_markers_spread_and_shapes_drawn_by_hand() {
    let same: Vec<Json> = (0..3).map(|i| json!({ "id": format!("m{i}"), "lat": 48.85, "lng": 2.35, "title": format!("M{i}") })).collect();
    let doc = json!({ "version": 1,
        "state": { "drawn": [], "fan": true },
        "root": { "type": "element", "children": [
            { "type": "ui-map", "attrs": { "id": "a" }, "props": { "markers": same.clone().into_iter().chain([json!({ "id": "x", "lat": 48.9, "lng": 2.4, "title": "X" })]).collect::<Vec<_>>(), "spread": "{{ fan }}", "center": [48.87, 2.37], "zoom": 12, "height": 300 } },
            { "type": "ui-map", "attrs": { "id": "d" }, "props": { "center": [48.85, 2.35], "zoom": 13, "height": 400, "draw": true, "bindShapes": "drawn", "shapes": "{{ drawn }}" } } ] } });
    let driver = r##"
      await ready("a", "d");
      const offs = () => ["m0", "m1", "m2", "x"].map((k) => pin("a", k).style.translate || "-").join(" ");
      log("fanned", offs(), $$("a", "[data-map-leg]").length, pin("a", "m1").getAttribute("aria-description"));
      // Each can be clicked on its own.
      pin("a", "m2").click(); await wait(100);
      log("picked", popup("a"));
      set("fan", false); await wait(100);
      log("stacked", offs(), $$("a", "[data-map-leg]").length);
      // Drawing: the buttons, and a line of three corners finished with Enter.
      const box = document.querySelector("#d .leaflet-container"), r = box.getBoundingClientRect();
      const clickAt = async (dx, dy) => { const x = r.left + r.width / 2 + dx, y = r.top + r.height / 2 + dy; for (const t of ["mousedown", "mouseup", "click"]) box.dispatchEvent(new MouseEvent(t, { bubbles: true, clientX: x, clientY: y, view: window })); await wait(60); };
      log("buttons", $$("d", "[data-map-action^=draw-]").map((b) => b.dataset.mapAction + ":" + b.getAttribute("aria-pressed")).join(" "));
      $("d", '[data-map-action="draw-line"]').click(); await wait(30);
      log("drawing", $("d", '[data-map-action="draw-line"]').getAttribute("aria-pressed"), $("d", "[data-map-note]").textContent);
      await clickAt(-100, 0); await clickAt(0, -50); await clickAt(100, 0);
      log("corners", $$("d", "[data-map-vertex]").length);
      key(document.getElementById("d"), "Enter"); await wait(100);
      const l = S().drawn[0];
      log("line", l.id, l.type, l.points.length, l.points[0][1] < l.points[2][1], $("d", '[data-map-action="draw-line"]').getAttribute("aria-pressed"), $$("d", ".leaflet-overlay-pane path").length);
      // An area: three corners, then its first again closes it.
      $("d", '[data-map-action="draw-area"]').click(); await wait(30);
      await clickAt(-60, 60); await clickAt(60, 60); await clickAt(0, 120); await clickAt(-60, 60);
      log("area", S().drawn[1]?.id, S().drawn[1]?.type, S().drawn[1]?.points.length);
      // Backspace takes a corner back; too few, and Enter says so.
      $("d", '[data-map-action="draw-line"]').click(); await wait(30);
      await clickAt(-150, -100); await clickAt(-120, -100);
      key(document.getElementById("d"), "Backspace"); await wait(30);
      log("back", $$("d", "[data-map-vertex]").length);
      key(document.getElementById("d"), "Enter"); await wait(30);
      log("short", $("d", "[data-map-note]").textContent, S().drawn.length);
      key(document.getElementById("d"), "Escape"); await wait(30);
      // A circle: its middle, then a point on its edge.
      $("d", '[data-map-action="draw-circle"]').click(); await wait(30);
      await clickAt(150, -100); await clickAt(190, -100);
      const c = S().drawn[2];
      log("circle", c.id, c.type, c.center.length, c.radius > 100 && c.radius < 2000);
      // The line clicked is picked; Delete takes it away.
      const path = $$("d", ".leaflet-overlay-pane path")[0], pr = path.getBoundingClientRect();
      path.dispatchEvent(new MouseEvent("click", { bubbles: true, view: window, clientX: pr.left + 2, clientY: pr.top + 2 })); await wait(50);
      log("shape picked", $("d", "[data-map-note]").textContent);
      key(document.getElementById("d"), "Delete"); await wait(100);
      log("removed", S().drawn.map((s) => s.id).join(","), $$("d", ".leaflet-overlay-pane path").length);
    "##;
    let Some(lines) = run("draw", doc, "", driver) else { return };
    check(
        lines,
        &[
            "fanned 0px -28px 24px 14px -24px 14px - 3 3 places at the same spot",
            "picked M2",
            "stacked - - - - 0",
            "buttons draw-line:false draw-area:false draw-circle:false",
            "drawing true Click the line's corners on the map; Enter to finish",
            "corners 3",
            "line line-1 line 3 true false 1",
            "area area-2 area 3",
            "back 1",
            "short A line needs two corners or more 2",
            "circle circle-3 circle 2 true",
            "shape picked Shape picked: Delete takes it away",
            "removed area-2,circle-3 2",
        ],
    );
}
