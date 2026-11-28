//! Draws twins in headless Chrome (WebGL from SwiftShader) and drives them. Models are
//! `data:` URLs, so the pages need nothing but three.js from the CDN.
//!
//! Ignored by default because it needs Chrome, and three.js from the CDN:
//!   cargo test -p irender-twin-kit --test browser -- --ignored
//! Set CHROME=/path/to/chrome if it isn't found automatically.

use irender_core::Format;
use irender_html::Engine;
use serde_json::{Value as Json, json};
use std::{path::PathBuf, process::Command};

fn chrome() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("CHROME") {
        return Some(path.into());
    }
    ["/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", "google-chrome", "google-chrome-stable", "chromium"]
        .iter()
        .map(PathBuf::from)
        .find(|p| Command::new(p).arg("--version").output().is_ok_and(|o| o.status.success()))
}

/// A file as a `data:` URL.
fn data_url(kind: &str, bytes: &[u8]) -> String {
    const ABC: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = format!("data:{kind};base64,");
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            out.push(if i <= c.len() { ABC[(n >> (18 - 6 * i) & 63) as usize] as char } else { '=' });
        }
    }
    out
}

fn pallet() -> String {
    data_url("model/gltf-binary", include_bytes!("../../../examples/twin/models/pallet.glb"))
}

fn beacon() -> String {
    data_url("model/obj", include_bytes!("../../../examples/twin/models/beacon.obj"))
}

/// The views are small and `quality: low` (no shadows or smoothing): CI's Chrome draws WebGL
/// in software, where a full-size view with shadows takes a long time a frame.
///
/// Renders `doc` with `driver` appended, runs it in Chrome and returns what the driver
/// wrote into `<pre id="result">`, one line per check. `head` goes before the page's own
/// scripts. None when there's no Chrome, or the browser can't draw WebGL.
fn run(name: &str, doc: Json, head: &str, driver: &str) -> Option<Vec<String>> {
    let Some(chrome) = chrome() else {
        eprintln!("skipping: Chrome not found (set CHROME=/path/to/chrome)");
        return None;
    };
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    irender_twin_kit::register(&mut engine.registry);
    let mut doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    irender_ui_kit::apply_theme(&mut doc);
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let script = format!(
        "<script>\nconst __errors = []; window.addEventListener('error', (e) => __errors.push(e.message));\n\
         const __ce = console.error; console.error = (...a) => {{ __errors.push(a.join(' ')); __ce(...a); }};\n\
         window.addEventListener('load', async () => {{\n{PRELUDE}\n try {{\n{driver}\n }} catch (e) {{ log('threw', e.message, e.stack); }}\n{POSTLUDE}\n}});\n</script>"
    );
    let html = engine.render(&ir, &json!({})).unwrap().document("test").replace("</head>", &format!("{head}</head>")).replace("</body>", &format!("{script}</body>"));
    let dir = std::env::temp_dir().join(format!("irender-twin-kit-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("page.html");
    std::fs::write(&file, html).unwrap();
    // `--no-sandbox`: CI runners block the sandbox's user namespaces; the page is ours.
    // SwiftShader draws WebGL without a GPU.
    let load = || {
        let out = Command::new(&chrome)
            .args(["--headless=new", "--no-sandbox", "--disable-gpu", "--enable-unsafe-swiftshader", "--blink-settings=preferredColorScheme=1", "--allow-file-access-from-files"])
            .args(["--window-size=1000,900", "--force-prefers-reduced-motion", "--virtual-time-budget=600000", "--dump-dom"])
            .arg(format!("file://{}", file.display()))
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    const RESULT: &str = "<pre id=\"result\">";
    let mut dom = load();
    if !dom.contains(RESULT) {
        dom = load();
    }
    let _ = std::fs::remove_dir_all(&dir);
    let start = dom.find(RESULT).unwrap_or_else(|| panic!("driver did not run:\n{dom}")) + RESULT.len();
    let result = dom[start..start + dom[start..].find("</pre>").unwrap()].replace("&quot;", "\"").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&");
    let lines: Vec<String> = result.lines().map(String::from).collect();
    if lines.first().is_some_and(|l| l.starts_with("no webgl")) {
        eprintln!("skipping: this browser can't draw WebGL here ({})", lines[0]);
        return None;
    }
    Some(lines)
}

const PRELUDE: &str = r##"
  const out = [], log = (...a) => out.push(a.map((x) => (typeof x === "string" ? x : JSON.stringify(x))).join(" "));
  const wait = (ms) => new Promise((r) => setTimeout(r, ms));
  const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state, set = (k, v) => root.__irender.set(k, v);
  const $ = (id, sel) => document.querySelector(`#${id} ${sel}`), $$ = (id, sel) => [...document.querySelectorAll(`#${id} ${sel}`)];
  // Firefox won't capture (or let go of) a pointer it isn't tracking, as the drivers'
  // synthetic ones aren't, and three's OrbitControls captures each one pressed: a real
  // one always is.
  for (const f of ["setPointerCapture", "releasePointerCapture"]) {
    const was = Element.prototype[f];
    Element.prototype[f] = function (id) { try { was.call(this, id); } catch (e) { if (e.name !== "NotFoundError") throw e; } };
  }
  const twin = (id) => document.getElementById(id).__twin;
  const tag = (id, key) => $(id, `[data-twin-tag="${key}"]`);
  const tags = (id) => $$(id, "[data-twin-tag]").map((t) => t.dataset.twinTag + ":" + t.textContent);
  const popup = (id) => $(id, "[data-twin-popup]")?.textContent ?? "none";
  // Chrome runs the page in virtual time, which races ahead while the GPU compiles shaders
  // (three.js waits for them with timers): allow plenty of it.
  const ready = async (...ids) => {
    for (let i = 0; i < 6000 && !ids.every((id) => document.getElementById(id).matches("[data-ready], [data-error]")); i++) await wait(50);
    await wait(150);
    const off = ids.map((id) => document.getElementById(id)).find((e) => e.hasAttribute("data-error"));
    if (off && /WebGL/.test(off.textContent)) throw Object.assign(new Error("no webgl"), { webgl: true });
  };
  const key = (el, k, more = {}) => el.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true, ...more }));
  const near = (a, b, d = 0.05) => Math.abs(a - b) <= d;
"##;

const POSTLUDE: &str = r##"
  const pre = document.createElement("pre"); pre.id = "result";
  pre.textContent = out.concat(__errors.filter((e) => !/WebGL|GPU stall/.test(e)).map((e) => "error " + e)).join("\n");
  document.body.append(pre);
  // Chrome writes the page out when its virtual time is spent, and a twin that's still
  // drawing frames spends it at the real pace: without them, it's spent at once.
  document.querySelectorAll("[data-i-x=ui-twin]").forEach((e) => e.remove());
"##;

/// Turns the driver's "no webgl" into a first line `run` skips on.
fn guarded(driver: &str) -> String {
    format!("try {{\n{driver}\n}} catch (e) {{ if (e.webgl) {{ out.length = 0; log('no webgl'); }} else throw e; }}")
}

fn check(lines: Vec<String>, expected: &[&str]) {
    assert_eq!(lines, expected, "\n{}", lines.join("\n"));
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn objects_readouts_alarms_and_selection() {
    let doc = json!({ "version": 1,
        "state": { "picked": null, "obj": null, "alarms": [], "area": "", "rows": [
            { "id": "m1", "model": "pallet", "position": [0, 0], "title": "Machine 1", "temp": 64, "status": "running", "area": "a" },
            { "id": "m2", "model": "beacon", "position": [3, 0], "title": "Machine 2", "temp": 72, "status": "running", "area": "a" },
            { "id": "m3", "shape": "box", "size": [1, 2, 1], "position": [-3, 1, 0], "title": "Machine 3", "status": "fault", "area": "b" },
            { "id": "s1", "shape": "sensor", "position": [0, 2, 3], "title": "Sensor", "air": 30 } ] },
        "root": { "type": "ui-twin", "attrs": { "id": "w" }, "props": { "quality": "low", "height": 260,
            "objects": "{{ rows }}", "bind": "picked", "bindObject": "obj", "bindAlarms": "alarms",
            "filter": { "area": "{{ area }}" },
            "models": { "pallet": pallet(), "beacon": { "url": beacon(), "color": "#f59e0b" } },
            "toneBy": { "field": "status", "tones": { "running": "success" } },
            "alarmBy": { "field": "status", "values": ["fault"] },
            "colorBy": { "field": "air", "min": 0, "max": 40, "colors": ["#0000ff", "#ff0000"] },
            "readouts": [
                { "field": "temp", "label": "Motor", "unit": "°C", "warn": 70, "alarm": 85 },
                { "field": "air", "label": "Air", "unit": "°C", "decimals": 1 } ] } } });
    let driver = r##"
      await ready("w");
      // Loaded, not the wireframe box that stands in for a model that failed.
      const stand = (it) => { let w = false; it.body.traverse((o) => (w ||= !!o.material?.wireframe)); return w; };
      log("objects", twin("w").items.size, "loaded", [...twin("w").items.values()].every((it) => it.body && it.size.x > 0 && !stand(it)));
      log("tags", tags("w").join(" | "));
      log("tones", tag("w", "m1").dataset.tone, tag("w", "m2").dataset.tone, tag("w", "m3").dataset.tone, "alarm", tag("w", "m3").hasAttribute("data-alarm"));
      log("alarms", S().alarms, $("w", "[data-twin-live]").textContent);
      const s = twin("w").items.get("s1");
      // A sensor is an instance: its colour and glow are its batch's, at its place in it.
      const part = s.batch.parts[0], at = s.batch.drawn.indexOf(s), c = new (twin("w").three.Color)();
      part.im.getColorAt(at, c);
      const glow = new (twin("w").three.Color)(part.glow.getX(at), part.glow.getY(at), part.glow.getZ(at)).multiplyScalar(1 / 0.55);
      log("heat", "#" + c.getHexString(), "#" + glow.getHexString(), "raised", near(s.group.position.y, 2));
      // A value past its limit is an alarm, and the change is announced.
      set("rows", S().rows.map((r) => (r.id === "m2" ? { ...r, temp: 90 } : r))); await wait(80);
      log("hotter", tag("w", "m2").textContent, tag("w", "m2").dataset.tone, S().alarms, $("w", "[data-twin-live]").textContent);
      // Picked from the list, with its popup.
      $("w", '[data-twin-option="m1"]').click(); await wait(80);
      log("picked", S().picked, S().obj.title, popup("w"), $("w", '[data-twin-option="m1"]').getAttribute("aria-pressed"));
      key($("w", "[data-twin-close]"), "Escape"); await wait(50);
      log("closed", S().picked, popup("w"), document.activeElement.dataset.twinOption);
      // From state, and it follows its row.
      set("picked", "m2"); await wait(80);
      log("from state", S().obj.temp, popup("w"));
      set("rows", S().rows.map((r) => (r.id === "m2" ? { ...r, temp: 66 } : r))); await wait(80);
      log("follows", S().obj.temp, popup("w"), S().alarms);
      // Filtered.
      set("area", "a"); await wait(80);
      log("filtered", [...twin("w").items.keys()], $$("w", "[data-twin-option]").length);
      // Where it's drawn in the view.
      const p = twin("w").at("m1");
      log("on screen", p.x > 0 && p.x < 1000 && p.y > 0 && p.y < 900);
      // Softer while the camera moves, sharp again once it's still (`quality: low` is 1 sharp).
      const ratio = () => twin("w").info.pixelRatio;
      for (let i = 0; i < 40 && ratio() !== 1; i++) await wait(50);
      twin("w").camera.position.x += 1; twin("w").controls.update();
      for (let i = 0; i < 40 && ratio() === 1; i++) await wait(20);
      log("moving", ratio(), document.getElementById("w").hasAttribute("data-moving"));
      for (let i = 0; i < 40 && ratio() !== 1; i++) await wait(50);
      log("still", ratio(), document.getElementById("w").hasAttribute("data-moving"));
    "##;
    let Some(lines) = run("objects", doc, "", &guarded(driver)) else { return };
    check(
        lines,
        &[
            "objects 4 loaded true",
            "tags m1:64°C | m2:72°C | m3:!Machine 3 | s1:30.0°C",
            "tones success warning danger alarm true",
            r#"alarms ["m3"] "#,
            "heat #bf0040 #bf0040 raised true",
            r#"hotter !90°C danger ["m2","m3"] Alarm: Machine 2"#,
            "picked m1 Machine 1 Machine 1Motor64°C true",
            "closed null none m1",
            "from state 90 Machine 2Motor90°C",
            r#"follows 66 Machine 2Motor66°C ["m3"]"#,
            r#"filtered ["m1","m2"] 2"#,
            "on screen true",
            "moving 0.65 true",
            "still 1 false",
        ],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn paths_views_live_data_and_editing() {
    let doc = json!({ "version": 1,
        "state": { "view": null, "values": {}, "layout": [
            { "id": "a", "shape": "box", "position": [1, 1], "title": "Box" },
            { "id": "b", "shape": "cylinder", "size": [1, 2], "position": { "x": -2, "z": 0 }, "rotation": 0, "title": "Can" } ] },
        "root": { "type": "element", "children": [
            { "type": "ui-twin", "attrs": { "id": "p" }, "props": { "quality": "low", "height": 260,
                "paths": [{ "id": "line", "points": [[0, 0], [10, 0]] }],
                "views": [{ "id": "top", "label": "Top", "position": [0, 30, 0.1], "target": [0, 0, 0] }],
                "bindView": "view",
                "live": { "url": "feed.json", "every": 1000 }, "bindLive": "values",
                "readouts": [{ "field": "load", "label": "Load", "unit": "%" }],
                "objects": [
                    { "id": "fixed", "shape": "box", "path": "line", "progress": 0.5, "title": "Fixed" },
                    { "id": "moving", "shape": "box", "path": "line", "speed": 2, "title": "Moving" },
                    { "id": "fed", "shape": "box", "position": [0, 5], "title": "Fed" } ] } },
            { "type": "ui-twin", "attrs": { "id": "e" }, "props": { "quality": "low", "height": 260,
                "objects": "{{ layout }}", "bindObjects": "layout", "editable": true, "snap": 0.5,
                "palette": [{ "label": "Crate", "shape": "box", "title": "Crate" }] } } ] } });
    let head = r#"<script>window.fetch = (u) => Promise.resolve({ ok: true, json: () => Promise.resolve(String(u).includes("feed") ? [{ id: "fed", load: 87 }] : {}) });</script>"#;
    let driver = r##"
      await ready("p", "e");
      const pos = (id) => twin("p").items.get(id).group.position;
      log("progress", near(pos("fixed").x, 5), near(pos("fixed").z, 0));
      const x0 = pos("moving").x;
      for (let i = 0; i < 30 && near(pos("moving").x, x0, 0.01); i++) await wait(100);
      // Somewhere else: it may have come round the end of the path again.
      log("moving", !near(pos("moving").x, x0, 0.01), "facing", near(twin("p").items.get("moving").group.rotation.y, Math.PI / 2));
      log("live", S().values.fed.load, tag("p", "fed").textContent);
      $("p", '[data-twin-view="top"]').click(); await wait(80);
      const c = twin("p").camera.position;
      log("view", S().view, near(c.y, 30, 0.5), $("p", '[data-twin-view="top"]').getAttribute("aria-pressed"));
      // Moved and turned with the keys, written back in each row's own form.
      $("e", '[data-twin-option="a"]').click(); await wait(50);
      const view = $("e", "[data-twin-canvas]"); view.focus();
      key(view, "ArrowRight", { altKey: true }); key(view, "ArrowDown", { altKey: true }); await wait(50);
      log("moved", S().layout[0].position);
      view.dispatchEvent(new KeyboardEvent("keydown", { key: "r", code: "KeyR", altKey: true, bubbles: true, cancelable: true })); await wait(50);
      log("turned", S().layout[0].rotation, $("e", "[data-twin-live]").textContent);
      $("e", '[data-twin-option="b"]').click(); await wait(50); view.focus();
      key(view, "ArrowLeft", { altKey: true }); await wait(50);
      log("object form", S().layout[1].position);
      // Added from the palette, and removed.
      $("e", "[data-twin-add]").click(); await wait(80);
      log("added", S().layout.length, S().layout[2].id, S().layout[2].title, S().layout[2].position.length);
      key(document.activeElement, "Delete"); await wait(80);
      log("removed", S().layout.map((r) => r.id), $("e", "[data-twin-live]").textContent);
    "##;
    let Some(lines) = run("paths", doc, head, &guarded(driver)) else { return };
    check(
        lines,
        &[
            "progress true true",
            "moving true facing true",
            "live 87 87%",
            "view top true true",
            "moved [1.5,1.5]",
            "turned 90 Turned to 90°",
            r#"object form {"x":-2.5,"z":0}"#,
            "added 3 crate-1 Crate 1 2",
            r#"removed ["a","b"] Removed Crate 1"#,
        ],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn without_webgl_the_objects_are_listed() {
    let doc = json!({ "version": 1,
        "root": { "type": "ui-twin", "attrs": { "id": "w" }, "props": { "quality": "low", "height": 260,
            "readouts": [{ "field": "temp", "label": "Temp", "unit": "°C", "alarm": 80 }],
            "objects": [{ "id": "a", "title": "Oven", "temp": 91 }, { "id": "b", "title": "Mixer" }] } } });
    let head = "<script>HTMLCanvasElement.prototype.getContext = () => null;</script>";
    let driver = r##"
      for (let i = 0; i < 400 && !document.getElementById("w").hasAttribute("data-error"); i++) await wait(50);
      log($("w", "[data-twin-failed] strong").textContent);
      log($$("w", "[data-twin-failed] li").map((l) => l.textContent + ":" + l.dataset.level).join(" | "));
    "##;
    let Some(lines) = run("nogl", doc, head, driver) else { return };
    check(lines, &["3D view unavailable", "Oven · Temp 91°C:alarm | Mixer:undefined"]);
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn readout_history_is_a_sparkline_in_the_popup() {
    let doc = json!({ "version": 1,
        "state": { "rows": [{ "id": "a", "shape": "box", "position": [0, 0], "title": "Oven", "temp": 60, "load": 5 }] },
        "root": { "type": "ui-twin", "attrs": { "id": "w" }, "props": { "quality": "low", "height": 260, "objects": "{{ rows }}",
            "readouts": [{ "field": "temp", "label": "Temp", "unit": "°C", "history": 3 }, { "field": "load", "label": "Load" }] } } });
    let driver = r##"
      await ready("w");
      const temp = (t) => set("rows", [{ ...S().rows[0], temp: t }]);
      const spark = () => $("w", "[data-twin-spark]");
      const points = () => spark()?.querySelector("polyline").getAttribute("points").split(" ").length ?? 0;
      twin("w").select("a"); await wait(80);
      log("one value", points(), $$("w", "[data-twin-popup] dd").map((d) => d.textContent).join(","));
      temp(64); await wait(80);
      log("two", points(), spark().getAttribute("aria-label"));
      // The same value again isn't a point; past `history` the oldest go.
      temp(64); await wait(80); temp(70); await wait(80); temp(66); await wait(80);
      log("kept", points(), spark().getAttribute("aria-label"));
      // Gone from the world, it forgets them.
      set("rows", []); await wait(80); set("rows", [{ id: "a", shape: "box", position: [0, 0], title: "Oven", temp: 50 }]); await wait(80);
      twin("w").select("a"); await wait(80);
      log("back", points());
    "##;
    let Some(lines) = run("history", doc, "", &guarded(driver)) else { return };
    check(lines, &["one value 0 60°C,5", "two 2 Last 2: 60°C to 64°C", "kept 3 Last 3: 64°C to 70°C", "back 0"]);
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn big_worlds_stay_instanced_and_culled() {
    // Pallets in a row: one fine, one warning, one in alarm, and one far off to the side.
    let row = |id: &str, x: i32, temp: i32| json!({ "id": id, "model": "pallet", "position": [x, 0], "title": id, "temp": temp });
    let doc = json!({ "version": 1,
        "state": { "picked": null, "rows": [row("ok", -3, 50), row("warm", 0, 75), row("hot", 3, 90), row("far", 60, 50)] },
        "root": { "type": "ui-twin", "attrs": { "id": "w" }, "props": { "quality": "low", "height": 260, "objects": "{{ rows }}", "bind": "picked",
            "models": { "pallet": pallet() }, "camera": { "position": [0, 6, 9], "target": [0, 0, 0] },
            "readouts": [{ "field": "temp", "label": "Temp", "warn": 70, "alarm": 85 }] } } });
    let driver = r##"
      await ready("w");
      const tw = twin("w"), it = (id) => tw.items.get(id);
      const meshes = (id) => { let n = 0; it(id).body.traverse((o) => (n += o.isMesh ? 1 : 0)); return n; };
      // In alarm it's an instance of its own batch, glowing, not a copy of the model.
      log("batched", !!it("ok").batch, it("ok").batch === it("warm").batch, !!it("hot").batch && it("hot").batch !== it("ok").batch, meshes("hot"));
      log("glows", it("hot").batch.glowing.length > 0, it("hot").batch.glowing.every((m) => m.emissiveIntensity > 0), it("ok").batch.glowing.length);
      // A warning's ring is an instance too; one in alarm pulses, a ring of its own.
      log("rings", !!it("warm").ringIn, !it("warm").ring, !!it("hot").ring, !it("hot").ringIn);
      // "Ready" comes as the last model opens, a frame before its instances are placed.
      for (let i = 0; i < 40 && !it("ok").batch.drawn; i++) await wait(50);
      // Out of view, not drawn.
      log("drawn", it("ok").batch.drawn.map((x) => x.id).join());
      // Back to normal: back among the others, as a copy never was.
      set("rows", S().rows.map((r) => (r.id === "hot" ? { ...r, temp: 60 } : r))); await wait(150);
      log("calm", it("hot").batch === it("ok").batch, !it("hot").ring, it("ok").batch.drawn.length);
      // Picked where it's drawn: an instance is found among those in view.
      const c = $("w", "canvas"), b = c.getBoundingClientRect(), p = tw.at("warm");
      const at = { clientX: b.left + p.x, clientY: b.top + p.y, button: 0, pointerId: 1, bubbles: true };
      c.dispatchEvent(new PointerEvent("pointerdown", at)); c.dispatchEvent(new PointerEvent("pointerup", at)); await wait(80);
      log("picked", S().picked);
      // Turned to the far one, it's drawn, and the others aren't.
      tw.camera.position.set(60, 6, 9); tw.controls.target.set(60, 0, 0); tw.controls.update(); await wait(150);
      log("turned", it("ok").batch.drawn.map((x) => x.id).join());
    "##;
    let Some(lines) = run("big", doc, "", &guarded(driver)) else { return };
    check(
        lines,
        &["batched true true true 0", "glows true true 0", "rings true true true true", "drawn ok,warm", "calm true true 3", "picked warm", "turned far"],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn models_are_simpler_far_away() {
    // Pallets simplified in the browser, crates with a far model of their own (the beacon),
    // and cans never simpler.
    let row = |id: &str, model: &str, x: i32| json!({ "id": id, "model": model, "position": [x, 0], "title": id });
    let doc = json!({ "version": 1,
        "state": { "picked": null, "rows": [row("p1", "pallet", -2), row("p2", "pallet", 0), row("c1", "crate", 2), row("n1", "can", 4)] },
        "root": { "type": "ui-twin", "attrs": { "id": "w" }, "props": { "quality": "low", "height": 260, "objects": "{{ rows }}", "bind": "picked",
            "lod": { "distance": 6, "ratio": 0.3 }, "camera": { "position": [0, 8, 10], "target": [0, 0, 0] },
            "models": { "pallet": pallet(), "crate": { "url": pallet(), "far": beacon() }, "can": { "url": pallet(), "scale": 0.5, "far": false } } } } });
    let driver = r##"
      await ready("w");
      const tw = twin("w"), it = (id) => tw.items.get(id), tri = (g) => (g.index ? g.index.count : g.attributes.position.count) / 3;
      const tris = (parts) => parts.reduce((s, p) => s + tri(p.geometry), 0);
      for (let i = 0; i < 60 && !["p1", "c1"].every((id) => it(id).batch.drawnFar?.length); i++) await wait(100);
      const p = it("p1").batch, c = it("c1").batch, n = it("n1").batch;
      log("simplified", p.far, p.farAt, tris(p.farParts) < tris(p.parts), p.farParts.every((x, i) => x.material === p.parts[i]?.material));
      log("own far model", c.far, c.farParts.length > 0 && c.farParts.every((x) => !c.parts.some((y) => y.geometry === x.geometry)));
      log("never", n.far, n.farParts);
      log("far", p.drawnFar.map((x) => x.id).join(), p.drawn.length, c.drawnFar.map((x) => x.id).join(), n.drawn.map((x) => x.id).join());
      // A far instance is picked like a near one.
      const cv = $("w", "canvas"), b = cv.getBoundingClientRect(), at = tw.at("p2");
      const e = { clientX: b.left + at.x, clientY: b.top + at.y, button: 0, pointerId: 1, bubbles: true };
      cv.dispatchEvent(new PointerEvent("pointerdown", e)); cv.dispatchEvent(new PointerEvent("pointerup", e)); await wait(80);
      log("picked", S().picked);
      // Up close, in full.
      // Up close to p1, about 4 m off, with p2 behind it past 6 m.
      tw.camera.position.set(-6, 1, 0); tw.controls.target.set(-2, 0, 0); tw.controls.update(); await wait(150);
      log("close", p.drawn.map((x) => x.id).join(), p.drawnFar.map((x) => x.id).join());
    "##;
    let Some(lines) = run("lod", doc, "", &guarded(driver)) else { return };
    check(
        lines,
        &["simplified ready 6 true true", "own far model ready true", "never null null", "far p1,p2 0 c1 n1", "picked p2", "close p1 p2"],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn coloured_by_value_and_shapes_are_instances() {
    let doc = json!({ "version": 1,
        "state": { "rows": [
            { "id": "cold", "model": "pallet", "position": [-2, 0], "air": 0 },
            { "id": "hot", "model": "pallet", "position": [0, 0], "air": 40 },
            { "id": "red", "shape": "box", "position": [2, 0], "color": "#ff0000" },
            { "id": "ok", "shape": "box", "position": [3.5, 0], "tone": "success" },
            { "id": "bad", "shape": "box", "position": [5, 0], "alarm": true } ] },
        "root": { "type": "ui-twin", "attrs": { "id": "w" }, "props": { "quality": "low", "height": 260, "objects": "{{ rows }}", "labels": "none",
            "models": { "pallet": pallet() }, "camera": { "position": [1, 7, 9], "target": [1, 0, 0] },
            "colorBy": { "field": "air", "min": 0, "max": 40, "colors": ["#0000ff", "#ff0000"] } } } });
    let driver = r##"
      await ready("w");
      const tw = twin("w"), it = (id) => tw.items.get(id), C = tw.three.Color;
      for (let i = 0; i < 40 && !["cold", "red"].every((id) => it(id).batch?.drawn); i++) await wait(50);
      // The glow an instance has, as the colour it's in (at the power it glows at).
      const glow = (id, power = 0.55) => { const b = it(id).batch, i = b.drawn.indexOf(it(id)), g = b.parts[0].glow; return "#" + new C(g.getX(i), g.getY(i), g.getZ(i)).multiplyScalar(1 / power).getHexString(); };
      const colour = (id) => { const b = it(id).batch, c = new C(); b.parts[0].im.getColorAt(b.drawn.indexOf(it(id)), c); return "#" + c.getHexString(); };
      log("models", it("cold").batch === it("hot").batch, !!it("cold").batch.tint, glow("cold"), glow("hot"));
      // A new value is a new glow, in the same batch.
      const was = it("cold").batch;
      set("rows", S().rows.map((r) => (r.id === "cold" ? { ...r, air: 40 } : r))); await wait(150);
      log("changed", it("cold").batch === was, glow("cold"));
      log("shapes", it("red").batch === it("ok").batch, colour("red"), colour("ok") !== colour("red"), glow("red"));
      // In alarm, a shape is a copy of its own, glowing.
      log("alarm", !it("bad").batch, it("bad").body.children.length);
      log("no meshes of their own", ["cold", "hot", "red", "ok"].every((id) => it(id).body.children.length === 0));
    "##;
    let Some(lines) = run("heat", doc, "", &guarded(driver)) else { return };
    check(
        lines,
        &["models true true #0000ff #ff0000", "changed true #ff0000", "shapes true #ff0000 true #000000", "alarm true 1", "no meshes of their own true"],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn alarm_tags_that_overlap_are_counted() {
    // Three in alarm side by side, seen from far off: their tags would pile up. The fourth
    // is well clear of them, of a's tag with its count too.
    let row = |id: &str, x: f64| json!({ "id": id, "shape": "box", "position": [x, 0], "title": id, "temp": 99 });
    let doc = json!({ "version": 1,
        "state": { "picked": null, "rows": [row("a", 0.0), row("b", 1.5), row("c", 3.0), row("far", 60.0)] },
        "root": { "type": "ui-twin", "attrs": { "id": "w" }, "props": { "quality": "low", "height": 260, "objects": "{{ rows }}", "bind": "picked",
            "camera": { "position": [20, 120, 160], "target": [20, 0, 0] },
            "readouts": [{ "field": "temp", "label": "Temp", "alarm": 90 }] } } });
    let driver = r##"
      await ready("w"); await wait(200);
      const seen = () => $$("w", "[data-twin-tag]").filter((t) => t.style.visibility !== "hidden").map((t) => t.dataset.twinTag + (t.dataset.more ?? "")).join();
      log("far off", seen());
      // Up close, each has its own.
      const tw = twin("w"); tw.camera.position.set(1.5, 2, 5); tw.controls.target.set(1.5, 0, 0); tw.controls.update(); await wait(200);
      log("close", seen());
    "##;
    let Some(lines) = run("pile", doc, "", &guarded(driver)) else { return };
    check(lines, &["far off a+2,far", "close a,b,c"]);
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn several_objects_picked_moved_and_removed_together() {
    let row = |id: &str, x: i32, z: i32| json!({ "id": id, "shape": "box", "position": [x, z], "title": id });
    let doc = json!({ "version": 1,
        "state": { "picked": null, "sel": [], "layout": [row("a", -3, 0), row("b", 0, 0), row("c", 3, 2), row("d", 4, 3)] },
        "root": { "type": "ui-twin", "attrs": { "id": "w" }, "props": { "quality": "low", "height": 300, "objects": "{{ layout }}",
            "bind": "picked", "bindSelected": "sel", "bindObjects": "layout", "editable": true, "multiple": true, "labels": "none",
            "camera": { "position": [0, 12, 10], "target": [0, 0, 0] } } } });
    let driver = r##"
      await ready("w");
      const tw = twin("w"), opt = (id) => $("w", `[data-twin-option="${id}"]`), rings = () => tw.scene.children.filter((o) => o.isMesh && o.visible && o.geometry.type === "RingGeometry" && o.material.opacity === 1).length;
      const xs = () => S().layout.map((r) => r.id + ":" + r.position[0]).join(" ");
      opt("a").click(); await wait(50);
      log("one", JSON.stringify(S().sel), S().picked, popup("w") !== "none");
      opt("b").dispatchEvent(new MouseEvent("click", { bubbles: true, ctrlKey: true })); await wait(200);
      log("two", JSON.stringify(S().sel), S().picked, popup("w"), rings(), $("w", "[data-twin-live]").textContent);
      opt("a").dispatchEvent(new MouseEvent("click", { bubbles: true, metaKey: true })); await wait(50);
      log("out again", JSON.stringify(S().sel));
      // A box from the floor around c and d, with Shift: added to b.
      const cv = $("w", "canvas"), r = cv.getBoundingClientRect(), pc = tw.at("c"), pd = tw.at("d");
      const l = Math.min(pc.x, pd.x) - 40, t = Math.min(pc.y, pd.y) - 40, rr = Math.max(pc.x, pd.x) + 90, bb = Math.max(pc.y, pd.y) + 90;
      const ev = (type, x, y) => cv.dispatchEvent(new PointerEvent(type, { clientX: r.left + x, clientY: r.top + y, button: 0, pointerId: 7, shiftKey: true, bubbles: true }));
      // From the floor beyond them (bottom right) to above and left of them.
      ev("pointerdown", rr, bb); ev("pointermove", (l + rr) / 2, (t + bb) / 2); ev("pointermove", l, t);
      log("box drawn", $$("w", "[data-twin-box]").length, tw.controls.enabled);
      ev("pointerup", l, t); await wait(200);
      log("boxed", JSON.stringify(S().sel), $$("w", "[data-twin-box]").length, rings());
      // Moved together with the keys; a, not picked, stays.
      const view = $("w", "[data-twin-canvas]"); view.focus();
      key(view, "ArrowRight", { altKey: true }); await wait(80);
      log("moved", xs(), $("w", "[data-twin-live]").textContent);
      // Picked from state, then all with ⌘A, then removed together.
      set("sel", ["a", "d"]); await wait(80);
      log("from state", [...tw.selection].join(), S().picked);
      view.focus(); key(view, "a", { metaKey: true }); await wait(50);
      log("all", JSON.stringify(S().sel));
      set("sel", ["b", "c"]); await wait(80); view.focus();
      key(view, "Delete"); await wait(80);
      log("removed", S().layout.map((r) => r.id).join(), JSON.stringify(S().sel), $("w", "[data-twin-live]").textContent);
    "##;
    let Some(lines) = run("multi", doc, "", &guarded(driver)) else { return };
    check(
        lines,
        &[
            r#"one ["a"] a true"#,
            r#"two ["a","b"] b none 2 2 selected"#,
            r#"out again ["b"]"#,
            "box drawn 1 true",
            r#"boxed ["b","c","d"] 0 3"#,
            "moved a:-3 b:0.5 c:3.5 d:4.5 Moved 3",
            "from state a,d d",
            r#"all ["a","b","c","d"]"#,
            r#"removed a,d [] Removed 2"#,
        ],
    );
}

/// Walls and zones drawn by hand while editing: corners clicked on the floor (snapped),
/// Enter or the first corner to finish, Backspace and Escape; one picked and deleted.
#[test]
#[ignore = "needs Chrome and three.js from the CDN"]
fn walls_and_zones_drawn_by_hand() {
    let doc = json!({ "version": 1,
        "state": { "walls": [], "zones": [{ "id": "dock", "points": [[-6, -6], [-2, -6], [-2, -2], [-6, -2]], "label": "Dock" }], "edit": true, "rows": [] },
        "root": { "type": "ui-twin", "attrs": { "id": "w" }, "props": { "quality": "low", "height": 300, "objects": "{{ rows }}", "bindObjects": "rows",
            "editable": "{{ edit }}", "snap": 1, "walls": "{{ walls }}", "zones": "{{ zones }}", "bindWalls": "walls", "bindZones": "zones", "labels": "none",
            "camera": { "position": [0, 16, 12], "target": [0, 0, 0] } } } });
    let driver = r##"
      await ready("w");
      const tw = twin("w"), T = tw.three, wallB = $("w", "[data-twin-action=draw-wall]"), zoneB = $("w", "[data-twin-action=draw-zone]");
      const cv = $("w", "canvas"), r = cv.getBoundingClientRect(), view = $("w", "[data-twin-canvas]");
      const click = (x, z) => {
        const p = new T.Vector3(x, 0, z).project(tw.camera), at = { clientX: r.left + ((p.x + 1) / 2) * r.width, clientY: r.top + ((1 - p.y) / 2) * r.height, button: 0, pointerId: 3, bubbles: true };
        cv.dispatchEvent(new PointerEvent("pointerdown", at)); cv.dispatchEvent(new PointerEvent("pointerup", at));
      };
      log("buttons", wallB.hidden, zoneB.hidden);
      // A wall: three corners (the clicks a little off the metre; they snap), one taken back, one more, Enter.
      wallB.click(); await wait(30);
      log("drawing", wallB.getAttribute("aria-pressed"), $("w", "[data-twin-note]").textContent);
      // The camera's controls would capture these made-up pointers, which Chrome refuses.
      tw.controls.enabled = false;
      click(0.1, 0.1); await wait(20); click(4.2, -0.1); await wait(20); click(4, 9); await wait(20);
      view.focus(); key(view, "Backspace"); await wait(20);
      click(4, 3); await wait(20);
      key(view, "Enter"); await wait(80);
      log("wall", JSON.stringify(S().walls), wallB.getAttribute("aria-pressed"));
      // A zone, with Z: three corners and a click on the first closes it.
      key(view, "z"); await wait(20);
      tw.controls.enabled = false;
      click(6, 0); await wait(20); click(9, 0); await wait(20); click(9, 3); await wait(20); click(6, 0); await wait(80);
      log("zone", JSON.stringify(S().zones.at(-1)));
      // Too few corners: said, and it goes on.
      key(view, "w"); await wait(20); tw.controls.enabled = false; click(-8, 4); await wait(20); key(view, "Enter"); await wait(20);
      log("short", $("w", "[data-twin-note]").textContent, S().walls.length);
      key(view, "Escape"); await wait(20);
      // The dock zone picked, and Delete takes it away.
      tw.controls.enabled = false;
      click(-4, -4); await wait(50);
      log("picked", $("w", "[data-twin-note]").textContent);
      key(view, "Delete"); await wait(80);
      log("deleted", S().zones.map((z) => z.id).join(","));
      // Not editable: the buttons go.
      set("edit", false); await wait(80);
      log("off", wallB.hidden, zoneB.hidden);
    "##;
    let Some(lines) = run("draw-walls", doc, "", &guarded(driver)) else { return };
    check(
        lines,
        &[
            "buttons false false",
            "drawing true Click the wall's corners on the floor; Enter to finish",
            r#"wall [{"id":"wall-1","points":[[0,0],[4,0],[4,3]]}] false"#,
            r#"zone {"id":"zone-2","points":[[6,0],[9,0],[9,3]],"label":"Zone 2"}"#,
            "short A wall needs two corners or more 1",
            "picked Zone picked: Delete takes it away",
            "deleted zone-2",
            "off true true",
        ],
    );
}

#[test]
#[ignore = "needs Chrome and three.js from the CDN"]
fn measuring_between_points_and_objects() {
    let row = |id: &str, x: i32, z: i32| json!({ "id": id, "shape": "box", "position": [x, z], "title": id });
    let doc = json!({ "version": 1,
        "state": { "picked": null, "m": null, "on": true },
        "root": { "type": "ui-twin", "attrs": { "id": "w" }, "props": { "quality": "low", "height": 300, "objects": [row("a", -3, 0), row("b", 3, 0)],
            "bind": "picked", "bindMeasure": "m", "measure": "{{ on }}", "labels": "none",
            "camera": { "position": [0, 12, 10], "target": [0, 0, 0] } } } });
    let driver = r##"
      await ready("w");
      const tw = twin("w"), T = tw.three, btn = $("w", "[data-twin-action=measure]"), opt = (id) => $("w", `[data-twin-option="${id}"]`);
      const tag = () => { const t = $("w", "[data-twin-measure]"); return t && t.style.visibility !== "hidden" ? t.textContent : "none"; };
      log("button", btn.hidden, btn.getAttribute("aria-pressed"), tw.measuring);
      btn.click(); await wait(50);
      log("on", btn.getAttribute("aria-pressed"), tw.measuring, $("w", "[data-twin-note]").textContent);
      // From the list: one object to another, from their middles on the floor; nothing's selected.
      opt("a").click(); await wait(50);
      log("half", JSON.stringify(S().m), $("w", "[data-twin-note]").textContent);
      opt("b").click(); await wait(80);
      log("objects", JSON.stringify(S().m), tag(), S().picked, $("w", "[data-twin-live]").textContent);
      // Two clicks on the floor, 4 m apart.
      const cv = $("w", "canvas"), r = cv.getBoundingClientRect();
      const click = (x, y, z) => {
        const p = new T.Vector3(x, y, z).project(tw.camera), at = { clientX: r.left + ((p.x + 1) / 2) * r.width, clientY: r.top + ((1 - p.y) / 2) * r.height, button: 0, pointerId: 3, bubbles: true };
        cv.dispatchEvent(new PointerEvent("pointerdown", at)); cv.dispatchEvent(new PointerEvent("pointerup", at));
      };
      // The camera's controls would capture this made-up pointer, which Chrome refuses.
      tw.controls.enabled = false;
      click(0, 0, 4); await wait(30); click(0, 0, 0); await wait(80);
      tw.controls.enabled = true;
      log("floor", Math.round(S().m.distance * 10) / 10, Math.round(S().m.up * 10) / 10, S().picked);
      // A sloping one says across and up too.
      tw.measure([0, 0, 0], [3, 4, 0]); await wait(80);
      log("slope", S().m.distance, S().m.across, S().m.up, tag());
      // M and Escape: off, and the line goes.
      const view = $("w", "[data-twin-canvas]"); view.focus();
      key(view, "Escape"); await wait(50);
      log("escape", btn.getAttribute("aria-pressed"), tw.measuring, S().m, tag(), tw.scene.children.some((o) => o.isLine && o.visible));
      key(view, "m"); await wait(50);
      log("m", tw.measuring);
      // Turned off from state, the button goes with it.
      set("on", false); await wait(80);
      log("off", btn.hidden, tw.measuring);
    "##;
    let Some(lines) = run("measure", doc, "", &guarded(driver)) else { return };
    check(
        lines,
        &[
            "button false false false",
            "on true true Click two points to measure between them",
            "half null Click a second point",
            r#"objects {"from":[-3,0,0],"to":[3,0,0],"distance":6,"across":6,"up":0} 6 m null 6 m between them"#,
            "floor 4 0 null",
            "slope 5 3 4 5 m↔ 3 · ↕ 4",
            "escape false false null none false",
            "m true",
            "off true false",
        ],
    );
}
