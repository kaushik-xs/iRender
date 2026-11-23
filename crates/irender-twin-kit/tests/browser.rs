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
            .args(["--window-size=1000,900", "--force-prefers-reduced-motion", "--virtual-time-budget=60000", "--dump-dom"])
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
  const twin = (id) => document.getElementById(id).__twin;
  const tag = (id, key) => $(id, `[data-twin-tag="${key}"]`);
  const tags = (id) => $$(id, "[data-twin-tag]").map((t) => t.dataset.twinTag + ":" + t.textContent);
  const popup = (id) => $(id, "[data-twin-popup]")?.textContent ?? "none";
  const ready = async (...ids) => {
    for (let i = 0; i < 600 && !ids.every((id) => document.getElementById(id).matches("[data-ready], [data-error]")); i++) await wait(50);
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
        "root": { "type": "ui-twin", "attrs": { "id": "w" }, "props": {
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
      log("objects", twin("w").items.size, "loaded", [...twin("w").items.values()].every((it) => it.body && it.size.x > 0));
      log("tags", tags("w").join(" | "));
      log("tones", tag("w", "m1").dataset.tone, tag("w", "m2").dataset.tone, tag("w", "m3").dataset.tone, "alarm", tag("w", "m3").hasAttribute("data-alarm"));
      log("alarms", S().alarms, $("w", "[data-twin-live]").textContent);
      const s = twin("w").items.get("s1");
      log("heat", "#" + s.mats[0].emissive.getHexString(), "raised", near(s.group.position.y, 2));
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
    "##;
    let Some(lines) = run("objects", doc, "", &guarded(driver)) else { return };
    check(
        lines,
        &[
            "objects 4 loaded true",
            "tags m1:64°C | m2:72°C | m3:!Machine 3 | s1:30.0°C",
            "tones success warning danger alarm true",
            r#"alarms ["m3"] "#,
            "heat #bf0040 raised true",
            r#"hotter !90°C danger ["m2","m3"] Alarm: Machine 2"#,
            "picked m1 Machine 1 Machine 1Motor64°C true",
            "closed null none m1",
            "from state 90 Machine 2Motor90°C",
            r#"follows 66 Machine 2Motor66°C ["m3"]"#,
            r#"filtered ["m1","m2"] 2"#,
            "on screen true",
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
            { "type": "ui-twin", "attrs": { "id": "p" }, "props": {
                "paths": [{ "id": "line", "points": [[0, 0], [10, 0]] }],
                "views": [{ "id": "top", "label": "Top", "position": [0, 30, 0.1], "target": [0, 0, 0] }],
                "bindView": "view",
                "live": { "url": "feed.json", "every": 1000 }, "bindLive": "values",
                "readouts": [{ "field": "load", "label": "Load", "unit": "%" }],
                "objects": [
                    { "id": "fixed", "shape": "box", "path": "line", "progress": 0.5, "title": "Fixed" },
                    { "id": "moving", "shape": "box", "path": "line", "speed": 2, "title": "Moving" },
                    { "id": "fed", "shape": "box", "position": [0, 5], "title": "Fed" } ] } },
            { "type": "ui-twin", "attrs": { "id": "e" }, "props": {
                "objects": "{{ layout }}", "bindObjects": "layout", "editable": true, "snap": 0.5,
                "palette": [{ "label": "Crate", "shape": "box", "title": "Crate" }] } } ] } });
    let head = r#"<script>window.fetch = (u) => Promise.resolve({ ok: true, json: () => Promise.resolve(String(u).includes("feed") ? [{ id: "fed", load: 87 }] : {}) });</script>"#;
    let driver = r##"
      await ready("p", "e");
      const pos = (id) => twin("p").items.get(id).group.position;
      log("progress", near(pos("fixed").x, 5), near(pos("fixed").z, 0));
      const x0 = pos("moving").x; await wait(500);
      log("moving", pos("moving").x > x0, "facing", near(twin("p").items.get("moving").group.rotation.y, Math.PI / 2));
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
        "root": { "type": "ui-twin", "attrs": { "id": "w" }, "props": {
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
