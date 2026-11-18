//! Drives the gallery's interactive components in headless Chrome.
//!
//! Ignored by default because it needs Chrome:
//!   cargo test -p irender-ui-kit --test browser -- --ignored
//! Set CHROME=/path/to/chrome if it isn't found automatically.

use irender_core::Format;
use irender_html::Engine;
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

/// Opens, clicks and measures the overlays and controls, one line per check.
const DRIVER: &str = r##"<style>* { transition: none !important; }</style>
<script>
window.addEventListener("load", async () => {
  const out = [];
  const log = (...a) => out.push(a.join(" "));
  const wait = (ms) => new Promise((r) => setTimeout(r, ms));
  const $ = (s) => document.querySelector(s), $$ = (s) => [...document.querySelectorAll(s)];
  const root = $("[data-i-root]"), S = () => root.__irender.state;
  const calls = []; root.addEventListener("irender:call", (e) => calls.push(e.detail));
  const btn = (t) => $$("button").find((b) => b.textContent === t);
  const toast = () => $$("[role=status] > div").map((t) => t.firstElementChild.textContent).join(",") || "hidden";
  const rect = (el) => el.getBoundingClientRect();
  try {
    const edit = $("#edit-profile");
    btn("Edit profile").click();
    log("dialog", edit.open, edit.matches(":modal"));
    const input = edit.querySelector("input"); input.value = "Ada"; input.dispatchEvent(new Event("input"));
    btn("Save changes").click();
    log("saved", edit.open, S().profile, toast());
    await wait(4200);
    log("toast after wait", toast());

    const confirm = $("#confirm-delete");
    btn("Delete 3 projects").click();
    log("alert", confirm.open, confirm.getAttribute("role"), !!confirm.querySelector("[aria-label=Close]"));
    [...confirm.querySelectorAll("button")].find((b) => b.textContent === "Yes, delete").click();
    log("deleted", confirm.open, S().projects, toast());

    const menu = $("#account-menu"), trigger = btn("My account");
    trigger.scrollIntoView({ block: "center" });
    trigger.click();
    log("menu", menu.matches(":popover-open"), Math.round(rect(menu).top - rect(trigger).bottom), Math.round(rect(menu).left - rect(trigger).left));
    [...menu.querySelectorAll("button")].find((b) => b.textContent.startsWith("Profile")).click();
    log("menu item", menu.matches(":popover-open"), JSON.stringify(calls.at(-1).args));

    const pop = $("#dimensions"), dims = btn("Dimensions");
    dims.click();
    // Near the window's edge it shifts to stay on screen rather than centring exactly.
    log("popover", Math.round(rect(pop).top - rect(dims).bottom), rect(pop).left >= 0 && rect(pop).right <= innerWidth);
    dims.click();

    $$("[role=tab]")[1].click();
    log("tabs", $$("[role=tabpanel]").map((p) => (p.hidden ? "-" : "shown")).join(","), S().tab);

    const items = $$("#faq details");
    items[1].querySelector("summary").click();
    await wait(50);
    log("accordion", items.map((d) => (d.open ? "open" : "-")).join(","));

    btn("Center").click(); const pressed = S().align; btn("Center").click();
    log("toggle group", pressed, S().align);

    const slider = $("input[type=range]"); slider.value = "70"; slider.dispatchEvent(new Event("input"));
    log("slider", S().volume, typeof S().volume);
  } catch (e) {
    log("EXCEPTION", e.stack);
  }
  const pre = document.createElement("pre");
  pre.id = "result";
  pre.textContent = out.join("\n");
  document.body.append(pre);
});
</script>"##;

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn gallery_overlays_and_controls_in_chrome() {
    let Some(chrome) = chrome() else {
        eprintln!("skipping: Chrome not found (set CHROME=/path/to/chrome)");
        return;
    };
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/ui-kit/gallery.yaml")).unwrap();
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let mut doc = irender_core::parse(&src, Format::Yaml).unwrap();
    irender_ui_kit::apply_theme(&mut doc);
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{DRIVER}</body>"));

    let dir = std::env::temp_dir().join(format!("irender-ui-kit-browser-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("gallery.html");
    std::fs::write(&file, html).unwrap();
    // `--no-sandbox`: CI runners block the sandbox's user namespaces; the page is ours.
    let out = Command::new(&chrome)
        .args(["--headless=new", "--no-sandbox", "--disable-gpu", "--blink-settings=preferredColorScheme=1", "--window-size=1000,800", "--virtual-time-budget=8000", "--dump-dom"])
        .arg(format!("file://{}", file.display()))
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let dom = String::from_utf8_lossy(&out.stdout);
    let start = dom.find("<pre id=\"result\">").unwrap_or_else(|| panic!("driver did not run:\n{dom}")) + 17;
    let result = dom[start..start + dom[start..].find("</pre>").unwrap()].replace("&quot;", "\"");
    let lines: Vec<&str> = result.lines().collect();

    let expected = [
        "dialog true true",
        "saved false Ada Profile saved",
        // The toast hides after its duration.
        "toast after wait hidden",
        "alert true alertdialog false",
        "deleted false 0 Projects deleted",
        // 6px under the trigger, aligned to its left edge.
        "menu true 6 0",
        r#"menu item false {"item":"profile"}"#,
        "popover 6 true",
        "tabs -,shown password",
        "accordion -,open,-",
        // Pressing the pressed option again clears it.
        "toggle group center ",
        "slider 70 number",
    ];
    assert_eq!(lines, expected, "\n{result}");
}

/// Opens `html` in headless Chrome and returns what the driver wrote to `#result`.
fn drive(name: &str, html: &str) -> Option<String> {
    let Some(chrome) = chrome() else {
        eprintln!("skipping: Chrome not found (set CHROME=/path/to/chrome)");
        return None;
    };
    let dir = std::env::temp_dir().join(format!("irender-ui-kit-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("page.html");
    std::fs::write(&file, html).unwrap();
    // The budget is virtual time, which runs ahead while the page is idle, so a large one
    // costs nothing on a quiet machine and leaves room on a loaded one.
    let out = Command::new(&chrome)
        .args(["--headless=new", "--no-sandbox", "--disable-gpu", "--blink-settings=preferredColorScheme=1", "--window-size=1000,900", "--virtual-time-budget=30000", "--dump-dom"])
        .arg(format!("file://{}", file.display()))
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let dom = String::from_utf8_lossy(&out.stdout);
    let start = dom.find("<pre id=\"result\">").unwrap_or_else(|| panic!("driver did not run:\n{dom}")) + 17;
    Some(dom[start..start + dom[start..].find("</pre>").unwrap()].replace("&quot;", "\"").replace("&amp;", "&"))
}

/// A tiny search API: `GET /fruits?q=an` answers the fruits containing `an`, with
/// CORS headers so a file:// page can call it. Returns the port and a request count.
fn fruit_server() -> (u16, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
    use std::io::{Read, Write};
    use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let hits = Arc::new(AtomicUsize::new(0));
    let counter = hits.clone();
    std::thread::spawn(move || {
        const FRUITS: [&str; 8] = ["Apple", "Banana", "Cherry", "Mango", "Mangosteen", "Orange", "Pineapple", "Tangerine"];
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut buf = [0u8; 2048];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]);
            let q = request.split_whitespace().nth(1).and_then(|p| p.split("q=").nth(1)).unwrap_or("").to_lowercase();
            counter.fetch_add(1, Ordering::SeqCst);
            let found: Vec<_> = FRUITS
                .iter()
                .filter(|f| f.to_lowercase().contains(&q))
                .map(|f| serde_json::json!({ "value": f.to_lowercase(), "label": f }))
                .collect();
            let body = serde_json::to_string(&found).unwrap();
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    (port, hits)
}

const COMBO_DRIVER: &str = r##"<style>* { transition: none !important; }</style>
<script>
window.addEventListener("load", async () => {
  const out = [], log = (...a) => out.push(a.join(" "));
  const wait = (ms) => new Promise((r) => setTimeout(r, ms));
  const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
  const box = (label) => [...document.querySelectorAll("[data-ui-input]")].find((i) => i.getAttribute("aria-label") === label).closest("[data-i-x]");
  const inp = (b) => b.querySelector("[data-ui-input]");
  const key = (b, k) => inp(b).dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true }));
  const type = (b, t) => { inp(b).value = t; inp(b).dispatchEvent(new Event("input", { bubbles: true })); };
  const opts = (b) => [...b.querySelectorAll("[role=option]")].map((o) => o.firstChild.textContent + (o.hasAttribute("data-active") ? "*" : ""));
  const chips = (b) => [...b.querySelectorAll("[data-ui-chip]")].map((c) => c.firstChild.textContent).join(",");
  const open = (b) => b.querySelector("[data-ui-menu]").matches(":popover-open");
  try {
    const one = box("One"); inp(one).focus();
    key(one, "ArrowDown"); key(one, "ArrowDown");
    log("keyboard", open(one), opts(one).join(","), inp(one).getAttribute("aria-activedescendant") === one.querySelector("[data-active]").id);
    type(one, "ÉL"); log("search", opts(one).join(","));
    key(one, "Enter"); log("chose", S().one, open(one), one.querySelector("[data-ui-single]").textContent);
    inp(one).blur();

    const many = box("Many"); inp(many).focus();
    key(many, "Backspace"); key(many, "Backspace");
    log("backspace", JSON.stringify(S().many), chips(many));
    key(many, "ArrowDown"); key(many, "Enter");
    log("add", JSON.stringify(S().many), open(many));
    many.querySelector("[data-ui-clear]").dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true }));
    log("clear", JSON.stringify(S().many));
    type(many, "kiwi"); key(many, "Enter");
    log("create", JSON.stringify(S().many), chips(many));
    inp(many).blur();

    const fruit = box("Fruit"); inp(fruit).focus();
    type(fruit, "man"); log("loading", fruit.hasAttribute("data-loading"), fruit.querySelector("[data-ui-message]").textContent);
    await wait(800);
    log("loaded", opts(fruit).join(","), fruit.hasAttribute("data-loading"));
    key(fruit, "ArrowDown"); key(fruit, "Enter");
    log("fruit", S().fruit);
    type(fruit, "man"); await wait(800);
    log("cached", opts(fruit).join(","));
    inp(fruit).blur();
  } catch (e) {
    log("EXCEPTION", e.stack);
  }
  log("requests", window.__hits ?? "-");
  const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
});
</script>"##;

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn combobox_in_chrome() {
    let (port, hits) = fruit_server();
    let doc = serde_json::json!({ "version": 1,
        "state": { "one": null, "many": ["a", "b"], "fruit": null },
        "root": { "type": "stack", "children": [
            { "type": "ui-combobox", "props": { "bind": "one", "label": "One",
                "options": [{ "value": "x", "label": "Xylophone", "disabled": true }, "Apple", "Élan", "Yellow"] } },
            { "type": "ui-combobox", "props": { "bind": "many", "label": "Many", "multiple": true, "clearable": true,
                "creatable": true, "fixed": ["a"], "options": [{ "value": "a", "label": "A" }, { "value": "b", "label": "B" }, { "value": "c", "label": "C" }] } },
            { "type": "ui-combobox", "props": { "bind": "fruit", "label": "Fruit", "debounce": 100,
                "searchUrl": format!("http://127.0.0.1:{port}/fruits") } }
        ] } });
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let mut doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    irender_ui_kit::apply_theme(&mut doc);
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{COMBO_DRIVER}</body>"));
    let Some(result) = drive("combobox", &html) else { return };
    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(
        lines[..lines.len() - 1],
        [
            // ↓ opens on the first enabled option, ↓ again moves on; aria-activedescendant follows.
            "keyboard true Xylophone,Apple,Élan*,Yellow true",
            // Case and accents are ignored; "el" also matches inside "Yellow".
            "search Élan*,Yellow",
            "chose Élan false Élan",
            // Backspace removes the last value but not the fixed one.
            r#"backspace ["a"] A"#,
            r#"add ["a","b"] true"#,
            r#"clear ["a"]"#,
            r#"create ["a","kiwi"] A,kiwi"#,
            "loading true Loading…",
            // The spinner stops once the answer arrives.
            "loaded Mango*,Mangosteen false",
            "fruit mangosteen",
            // The same search is answered from the cache.
            "cached Mango*,Mangosteen",
        ],
        "\n{result}"
    );
    assert_eq!(hits.load(std::sync::atomic::Ordering::SeqCst), 1, "one request for two identical searches");
}

/// Forces dark, then light, then back to the system scheme (light: `drive` pins it, whatever the machine uses).
const THEME_DRIVER: &str = r##"<style>* { transition: none !important; }</style>
<script>
window.addEventListener("load", () => {
  const out = [];
  const log = (...a) => out.push(a.join(" "));
  const bg = () => getComputedStyle(document.body).backgroundColor;
  const marked = () => getComputedStyle(document.querySelector("#marked")).color;
  const [first, second] = document.querySelectorAll("[role=group]");
  const pressed = (g) => [...g.querySelectorAll("button")].filter((b) => b.getAttribute("aria-pressed") === "true").map((b) => b.textContent).join();
  const pick = (g, t) => [...g.querySelectorAll("button")].find((b) => b.textContent === t).click();
  try {
    log("start", pressed(first), bg(), marked());
    pick(first, "Dark");
    log("dark", pressed(first), pressed(second), bg(), marked(), document.documentElement.style.colorScheme, localStorage.getItem("irender-theme"));
    pick(second, "Light");
    log("light", pressed(first), bg(), marked());
    pick(first, "System");
    log("system", pressed(second), bg(), marked(), document.documentElement.style.colorScheme === "");
  } catch (e) {
    log("error", e.message);
  }
  localStorage.removeItem("irender-theme");
  const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
});
</script>
"##;

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn theme_toggle_in_chrome() {
    let doc = serde_json::json!({ "version": 1,
        "theme": { "tokens": { "color": { "surface": "rgb(255, 255, 255)" } }, "modes": { "dark": { "color": { "surface": "rgb(0, 0, 0)" } } } },
        "root": { "type": "stack", "children": [
            { "type": "ui-theme-toggle" },
            { "type": "ui-theme-toggle", "props": { "labels": { "light": "Light" } } },
            { "type": "text", "attrs": { "id": "marked" }, "props": { "value": "x" },
              "style": { "color": "rgb(1, 1, 1)", "@dark": { "color": "rgb(2, 2, 2)" } } }
        ] } });
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{THEME_DRIVER}</body>"));
    let Some(result) = drive("theme", &html) else { return };
    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(
        lines,
        [
            "start System rgb(255, 255, 255) rgb(1, 1, 1)",
            // The theme's dark mode and `@dark` styles follow, and every toggle shows it.
            "dark Dark Dark rgb(0, 0, 0) rgb(2, 2, 2) dark dark",
            "light Light rgb(255, 255, 255) rgb(1, 1, 1)",
            "system System rgb(255, 255, 255) rgb(1, 1, 1) true",
        ],
        "\n{result}"
    );
}

/// Dialogs and menus inside a list the browser rebuilds: each item's trigger opens
/// its own overlay, which opens next to it and closes from inside, also after the
/// keyed list reorders.
const LIST_OVERLAYS_DRIVER: &str = r##"<script>
window.addEventListener("load", () => {
  const out = [];
  const root = document.querySelector("[data-i-root]");
  const sections = () => [...document.querySelectorAll("#rows > section")];
  const open = () => document.querySelector("dialog[open] h2")?.textContent ?? "-";
  const menu = () => document.querySelector("[popover]:popover-open");
  const dialogIds = () => new Set([...document.querySelectorAll("#rows dialog")].map((d) => d.id)).size;
  const step = (i) => {
    const s = sections()[i];
    s.querySelector("button[aria-haspopup=dialog]").click();
    const shown = open();
    document.querySelector("dialog[open] button[command=close]").click();
    const trigger = s.querySelector("button[aria-haspopup=menu]");
    trigger.click();
    const m = menu();
    const near = m && s.contains(m) && Math.abs(m.getBoundingClientRect().top - trigger.getBoundingClientRect().bottom) < 20;
    const item = m?.querySelector("button")?.textContent;
    m?.querySelector("button").click();
    return [shown, open(), item, near, menu() ? "menu open" : "menu closed"].join(" ");
  };
  out.push("ids " + dialogIds());
  out.push("row 2: " + step(1));
  root.__irender.set("rows", [...root.__irender.state.rows].reverse());
  out.push("reversed, row 1: " + step(0));
  out.push("ids " + dialogIds());
  const pre = document.createElement("pre");
  pre.id = "result";
  pre.textContent = out.join("\n");
  document.body.append(pre);
});
</script>"##;

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn overlays_in_rebuilt_lists_in_chrome() {
    let doc = serde_json::json!({ "version": 1,
        "state": { "rows": [{ "id": 1, "name": "a" }, { "id": 2, "name": "b" }] },
        "root": { "type": "list", "attrs": { "id": "rows" }, "props": { "items": "{{ rows }}", "as": "r", "key": "{{ r.id }}" },
          "children": [{ "type": "element", "props": { "tag": "section" }, "style": { "padding": 40 }, "children": [
            { "type": "ui-dialog", "props": { "trigger": "Open {{ r.name }}", "title": "{{ r.name }}" },
              "children": [{ "type": "ui-dialog-close", "props": { "label": "Done" } }] },
            { "type": "ui-dropdown-menu", "props": { "trigger": "Menu" },
              "children": [{ "type": "ui-menu-item", "props": { "label": "Edit {{ r.name }}" } }] } ] }] } });
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{LIST_OVERLAYS_DRIVER}</body>"));
    let Some(result) = drive("list-overlays", &html) else { return };
    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(
        lines,
        ["ids 2", "row 2: b - Edit b true menu closed", "reversed, row 1: b - Edit b true menu closed", "ids 2"],
        "\n{result}"
    );
}

/// A stored choice is applied before the content is shown: a script placed right
/// after the content, which runs before any behaviour starts, already sees it.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn stored_theme_applies_before_the_content_shows_in_chrome() {
    let doc = serde_json::json!({ "version": 1,
        "theme": { "tokens": { "color": { "surface": "rgb(255, 255, 255)" } }, "modes": { "dark": { "color": { "surface": "rgb(0, 0, 0)" } } } },
        "root": { "type": "stack", "children": [
            { "type": "text", "attrs": { "id": "marked" }, "props": { "value": "x" },
              "style": { "color": "rgb(1, 1, 1)", "@dark": { "color": "rgb(2, 2, 2)" } } },
            { "type": "ui-theme-toggle" } ] } });
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test");
    let marked = html.find("id=\"marked\"").unwrap();
    let after = marked + html[marked..].find("</span>").unwrap() + 7;
    let probe = r#"<script>window.__seen = [getComputedStyle(document.getElementById("marked")).color, document.documentElement.dataset.theme, !!window.__irenderBehaviors?.["ui-theme-toggle"]].join(" ")</script>"#;
    let report = r#"<script>window.addEventListener("load", () => {
      const pre = document.createElement("pre"); pre.id = "result";
      pre.textContent = window.__seen + " | " + document.querySelector("[aria-pressed=true]").dataset.scheme;
      document.body.append(pre);
    });</script>"#;
    let html = format!("{}{probe}{}", &html[..after], &html[after..])
        .replace("<head>", r#"<head><script>localStorage.setItem("irender-theme", "dark")</script>"#)
        .replace("</body>", &format!("{report}</body>"));
    let Some(result) = drive("theme-early", &html) else { return };
    // Dark before the toggle's behaviour is even defined, and the toggle agrees later.
    assert_eq!(result, "rgb(2, 2, 2) dark false | dark");
}

/// A page without a toggle follows the choice made on one that has it.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn stored_theme_applies_on_pages_without_a_toggle_in_chrome() {
    let doc = serde_json::json!({ "version": 1,
        "root": { "type": "ui-badge", "attrs": { "id": "marked" }, "props": { "text": "x" },
                  "style": { "color": "rgb(1, 1, 1)", "@dark": { "color": "rgb(2, 2, 2)" } } } });
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test");
    let report = r#"<script>window.addEventListener("load", () => {
      const pre = document.createElement("pre"); pre.id = "result";
      pre.textContent = getComputedStyle(document.getElementById("marked")).color + " " + document.documentElement.dataset.theme;
      document.body.append(pre);
    });</script>"#;
    let html = html
        .replace("<head>", r#"<head><script>localStorage.setItem("irender-theme", "dark")</script>"#)
        .replace("</body>", &format!("{report}</body>"));
    let Some(result) = drive("theme-no-toggle", &html) else { return };
    assert_eq!(result, "rgb(2, 2, 2) dark");
}

/// Form controls' native parts (a select's popup, a checkbox, a slider's track)
/// follow the toggle, not only the system. `light-dark()` resolves against an
/// element's used colour scheme, so it shows which one each control gets.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn form_controls_follow_the_theme_toggle_in_chrome() {
    let doc = serde_json::json!({ "version": 1,
        "state": { "on": true, "n": 5, "s": "a", "t": "" },
        "root": { "type": "stack", "children": [
            { "type": "ui-theme-toggle" },
            { "type": "ui-input", "props": { "bind": "t" } },
            { "type": "ui-select", "props": { "bind": "s", "options": ["a", "b"] } },
            { "type": "ui-checkbox", "props": { "bind": "on", "label": "On" } },
            { "type": "ui-switch", "props": { "bind": "on", "label": "On" } },
            { "type": "ui-slider", "props": { "bind": "n" } } ] } });
    let driver = r#"<script>
    window.addEventListener("load", () => {
      const controls = ["input:not([type])", "input[type=text]", "select", "input[type=checkbox]:not([role])", "input[role=switch]", "input[type=range]"]
        .map((sel) => document.querySelector(sel)).filter(Boolean);
      const schemes = () => controls.map((el) => {
        el.style.outlineColor = "light-dark(rgb(1, 1, 1), rgb(2, 2, 2))";
        return getComputedStyle(el).outlineColor === "rgb(2, 2, 2)" ? "dark" : "light";
      }).join(",");
      const pick = (s) => document.querySelector(`[data-scheme=${s}]`).click();
      const out = ["system " + schemes()];
      pick("dark"); out.push("dark " + schemes());
      pick("light"); out.push("light " + schemes());
      pick("system"); out.push("system " + schemes());
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = controls.length + "\n" + out.join("\n");
      document.body.append(pre);
    });
    </script>"#;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("form-schemes", &html) else { return };
    // Headless Chrome's system scheme is light.
    let five = |s: &str| [s; 5].join(",");
    assert_eq!(
        result,
        format!("5\nsystem {}\ndark {}\nlight {}\nsystem {}", five("light"), five("dark"), five("light"), five("light")),
        "\n{result}"
    );
}

/// Toasts stack, each hiding on its own timer: a newer toast no longer hides early,
/// `max` drops the oldest, `duration: 0` stays until dismissed, and hovering pauses
/// the timers.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn toasts_queue_with_their_own_timers_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "toast": null },
        "root": { "type": "ui-toaster", "props": { "bind": "toast", "duration": 1000, "max": 2 } } });
    let driver = r#"<script>
    window.addEventListener("load", async () => {
      const out = [];
      const root = document.querySelector("[data-i-root]");
      const region = document.querySelector("[role=status]");
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const toasts = () => [...region.querySelectorAll(":scope > div")]
        .map((t) => [...t.children].filter((c) => c.tagName === "DIV").map((c) => c.textContent).join(":") + (t.dataset.variant ? "!" : "")).join(",") || "-";
      const add = async (toast) => { root.__irender.set("toast", toast); await wait(0); };
      await add({ title: "A", description: "first" });
      out.push("A " + toasts() + " key=" + root.__irender.state.toast);
      await wait(500); await add({ title: "B", variant: "destructive" });
      out.push("B " + toasts());
      await wait(600);
      out.push("A's time " + toasts());
      await add({ title: "C" }); await add({ title: "D" });
      out.push("max " + toasts());
      await add({ title: "E", duration: 0 });
      out.push("sticky " + toasts());
      region.dispatchEvent(new PointerEvent("pointerenter"));
      await wait(1500);
      out.push("hovered " + toasts());
      region.dispatchEvent(new PointerEvent("pointerleave"));
      await wait(1100);
      out.push("left " + toasts());
      region.querySelector("[data-dismiss]").click();
      out.push("dismissed " + toasts());
      await add({ title: "A", description: "first" });
      out.push("again " + toasts());
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n");
      document.body.append(pre);
    });
    </script>"#;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("toasts", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            // Shown, and the key is cleared for the next toast.
            "A A:first key=null",
            "B A:first,B!",
            // A hides on its own timer; B, added later, stays.
            "A's time B!",
            // At most two: B goes first.
            "max C,D",
            "sticky D,E",
            // Hovering pauses D's timer; leaving gives it a moment more. E stays.
            "hovered D,E",
            "left E",
            "dismissed -",
            // The same toast again shows again.
            "again A:first",
        ],
        "\n{result}"
    );
}

/// The dropdown menu's keyboard pattern: opening focuses the first item, the arrow
/// keys move and wrap, Home and End jump, typing jumps to a matching item, disabled
/// items are skipped, Tab closes the menu, and Up on the button opens it on the last
/// item. Choosing an item puts focus back on the button.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn dropdown_menu_keyboard_in_chrome() {
    let doc = serde_json::json!({ "version": 1,
        "root": { "type": "ui-dropdown-menu", "props": { "trigger": "Menu", "id": "m" }, "children": [
            { "type": "ui-menu-label", "props": { "text": "Account" } },
            { "type": "ui-menu-item", "props": { "label": "Profile" } },
            { "type": "ui-menu-item", "props": { "label": "Billing", "disabled": true } },
            { "type": "ui-menu-item", "props": { "label": "Settings" } },
            { "type": "ui-menu-separator" },
            { "type": "ui-menu-item", "props": { "label": "Support", "href": "#support" } },
            { "type": "ui-menu-item", "props": { "label": "Sign out" } } ] } });
    let driver = r#"<script>
    window.addEventListener("load", async () => {
      const out = [];
      const menu = document.getElementById("m");
      const trigger = document.querySelector("[popovertarget=m]");
      const press = (key) => document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }));
      const where = () => menu.contains(document.activeElement) ? document.activeElement.textContent
        : document.activeElement === trigger ? "button" : document.activeElement.tagName;
      const step = (label) => out.push(label + " " + where() + (menu.matches(":popover-open") ? "" : " (closed)"));
      out.push("roles " + menu.getAttribute("role") + " " + [...menu.querySelectorAll("[role=menuitem]")].length
        + " " + [...menu.querySelectorAll("[role=menuitem]")].every((i) => i.tabIndex === -1));
      // The menu focuses its first item on the popover's `toggle` event, a task later.
      const tick = () => new Promise((r) => setTimeout(r, 0));
      trigger.focus(); trigger.click(); await tick(); step("open");
      press("ArrowDown"); step("down");
      press("ArrowDown"); step("down");
      press("ArrowDown"); step("down");
      press("ArrowDown"); step("wrap");
      press("ArrowUp"); step("up");
      press("End"); step("end");
      press("Home"); step("home");
      press("s"); step("s");
      press("s"); step("s");
      await new Promise((r) => setTimeout(r, 600));
      press("s"); press("i"); step("si");
      press("Tab"); step("tab");
      trigger.focus(); press("ArrowUp"); await tick(); step("button up");
      document.activeElement.click(); step("chose");
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n");
      document.body.append(pre);
    });
    </script>"#;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("menu-keys", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "roles menu 5 true",
            "open Profile",
            // Billing is disabled, so it's skipped.
            "down Settings",
            "down Support",
            "down Sign out",
            "wrap Profile",
            "up Sign out",
            "end Sign out",
            "home Profile",
            // A letter steps through the items starting with it; after a pause, "si" spells it out.
            "s Settings",
            "s Support",
            "si Sign out",
            "tab button (closed)",
            "button up Sign out",
            "chose button (closed)",
        ],
        "\n{result}"
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn field_groups_spinner_and_scroll_area_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "name": "", "site": "" },
        "root": { "type": "stack", "children": [
            { "type": "ui-field", "props": { "label": "Name", "for": "name",
                "error": "{{ name != '' && name.length < 3 ? 'Too short' : '' }}" },
              "children": [ { "type": "ui-input", "props": { "bind": "name" }, "attrs": { "id": "name" } } ] },
            { "type": "ui-input-group", "props": { "prefix": "https://" }, "attrs": { "id": "group" },
              "children": [ { "type": "ui-input", "props": { "bind": "site" }, "attrs": { "id": "site" } } ] },
            { "type": "ui-button-group", "attrs": { "id": "buttons" }, "children": [
                { "type": "ui-button", "props": { "label": "A", "variant": "outline" } },
                { "type": "ui-button", "props": { "label": "B", "variant": "outline" } } ] },
            { "type": "ui-spinner", "attrs": { "id": "spin" } },
            { "type": "ui-scroll-area", "props": { "height": 40 }, "attrs": { "id": "scroll" }, "children": [
                { "type": "element", "style": { "height": 200 } } ] } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = [];
      const $ = (s) => document.querySelector(s), css = (el) => getComputedStyle(el);
      const name = $("#name"), error = $("[aria-live]");
      const type = (v) => { name.value = v; name.dispatchEvent(new Event("input")); };
      out.push("error " + css(error).display);
      type("ab"); out.push("error " + css(error).display + " " + error.textContent);
      const described = name.getAttribute("aria-describedby").split(" ").map((id) => document.getElementById(id)?.textContent);
      out.push("described " + described.join("|") + " labelled " + name.labels[0]?.textContent);
      type("abc"); out.push("error " + css(error).display);

      const group = $("#group"), site = $("#site");
      out.push("grouped field " + css(site).borderTopWidth + " " + css(site).boxShadow);
      site.focus({ focusVisible: true });
      out.push("group ring " + (css(group).boxShadow !== "none") + " field ring " + css(site).boxShadow);

      const [a, b] = document.querySelectorAll("#buttons button");
      out.push("joined " + (b.getBoundingClientRect().left - a.getBoundingClientRect().right) + " "
        + css(a).borderTopRightRadius + " " + css(b).borderTopLeftRadius);

      const spin = $("#spin");
      out.push("spinner " + spin.getAttribute("role") + " " + spin.getAttribute("aria-label") + " " + css(spin).animationName);

      const scroll = $("#scroll");
      out.push("scrolls " + (scroll.scrollHeight > scroll.clientHeight) + " " + scroll.tabIndex);
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n");
      document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let mut doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    irender_ui_kit::apply_theme(&mut doc);
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("field-groups", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            // The error is hidden while it is empty, and follows state.
            "error none",
            "error block Too short",
            // The input points at the error, and the label at the input.
            "described Too short labelled Name",
            "error none",
            // A field in a group gives its border and focus ring to the group.
            "grouped field 0px none",
            "group ring true field ring none",
            // Neighbouring buttons share a border and square their inner corners.
            "joined -1 0px 0px",
            "spinner status Loading ui-spin",
            "scrolls true 0",
        ],
        "\n{result}"
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn multiple_toggle_group_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "marks": ["i"] },
        "root": { "type": "ui-toggle-group", "props": { "bind": "marks", "multiple": true, "options": ["b", "i", "u"] } } });
    let driver = r##"<script>
    window.addEventListener("load", () => {
      const out = [];
      const root = document.querySelector("[data-i-root]");
      const buttons = [...document.querySelectorAll("[role=group] button")];
      const press = (t) => buttons.find((b) => b.textContent === t).click();
      const step = (label) => out.push(label + " " + JSON.stringify(root.__irender.state.marks) + " "
        + buttons.map((b) => b.getAttribute("aria-pressed") === "true" ? b.textContent : "-").join(""));
      step("start");
      press("b"); step("b");
      press("u"); step("u");
      press("i"); step("i");
      press("b"); press("u"); step("off");
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n");
      document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("toggle-multiple", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            r#"start ["i"] -i-"#,
            // Each press adds its option at the end, or takes it out.
            r#"b ["i","b"] bi-"#,
            r#"u ["i","b","u"] biu"#,
            r#"i ["b","u"] b-u"#,
            "off [] ---",
        ],
        "\n{result}"
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn combobox_inside_dialog_and_sheet_in_chrome() {
    let options = serde_json::json!(["Apple", "Banana", "Cherry", "Damson", "Elderberry", "Fig", "Grape"]);
    let doc = serde_json::json!({ "version": 1, "state": { "a": null, "b": null },
        "root": { "type": "stack", "children": [
            { "type": "ui-dialog", "props": { "id": "d", "trigger": "Open dialog", "title": "Fruit" }, "children": [
                { "type": "ui-combobox", "props": { "bind": "a", "options": options } } ] },
            { "type": "ui-sheet", "props": { "id": "s", "trigger": "Open sheet", "title": "Fruit", "side": "bottom" }, "children": [
                { "type": "ui-combobox", "props": { "bind": "b", "options": options } } ] } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = [];
      const tick = () => new Promise((r) => setTimeout(r, 50));
      for (const [id, key] of [["d", "a"], ["s", "b"]]) {
        const d = document.getElementById(id), input = d.querySelector("[data-ui-input]"), menu = d.querySelector("[data-ui-menu]");
        const control = d.querySelector("[data-ui-control]");
        const press = (k) => { const e = new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true });
          input.dispatchEvent(e); return e.defaultPrevented ? "kept" : "passed on"; };
        [...document.querySelectorAll("button")].find((b) => b.textContent === (id === "d" ? "Open dialog" : "Open sheet")).click();
        input.focus();
        press("ArrowDown"); await tick();
        const m = menu.getBoundingClientRect(), c = control.getBoundingClientRect();
        const hit = document.elementFromPoint(m.x + m.width / 2, m.y + 12);
        // Below the field, or above it when there's no room (the bottom sheet).
        const place = Math.round(m.top - c.bottom) === 4 ? "below" : Math.round(c.top - m.bottom) === 4 ? "above" : `${m.top},${c.bottom}`;
        out.push(`${id} menu ${menu.matches(":popover-open")} ${menu.contains(hit) ? "on top" : "covered"} ${place}`);
        menu.querySelectorAll("[role=option]")[1].dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true }));
        await tick();
        out.push(`${id} chose ${root().state[key]} ${d.open}`);
        press("ArrowDown"); await tick();
        input.value = "ch"; input.dispatchEvent(new Event("input", { bubbles: true })); await tick();
        // Each Escape the combobox uses is kept from the dialog; the last one closes it.
        out.push(`${id} escape ${press("Escape")} ${press("Escape")} ${press("Escape")} ${JSON.stringify(input.value)}`);
        d.close();
      }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n");
      document.body.append(pre);
    });
    const root = () => document.querySelector("[data-i-root]").__irender;
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let mut doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    irender_ui_kit::apply_theme(&mut doc);
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("combobox-dialog", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "d menu true on top below",
            "d chose Banana true",
            r#"d escape kept kept passed on """#,
            "s menu true on top above",
            "s chose Banana true",
            r#"s escape kept kept passed on """#,
        ],
        "\n{result}"
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn lists_follow_state_in_chrome() {
    let doc = serde_json::json!({ "version": 1,
        "state": { "opts": ["a", "b"], "pick": "b", "crumbs": [{ "label": "Home", "href": "/" }, { "label": "Docs" }],
                   "page": 1, "pages": 3 },
        "root": { "type": "stack", "children": [
            { "type": "ui-radio-group", "props": { "bind": "pick", "options": "{{ opts }}" }, "attrs": { "id": "radios" } },
            { "type": "ui-toggle-group", "props": { "bind": "pick", "options": "{{ opts }}" }, "attrs": { "id": "toggles" } },
            { "type": "ui-breadcrumb", "props": { "items": "{{ crumbs }}" } },
            { "type": "ui-pagination", "props": { "page": "{{ page }}", "pages": "{{ pages }}", "href": "#{{ p }}" } } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = [];
      const root = document.querySelector("[data-i-root]"), set = (k, v) => root.__irender.set(k, v);
      const tick = () => new Promise((r) => setTimeout(r, 0));
      const radios = () => [...document.querySelectorAll("#radios input")].map((i) => i.value + (i.checked ? "*" : "") + (i.disabled ? "!" : "")).join(" ");
      const toggles = () => [...document.querySelectorAll("#toggles button")].map((b) => b.textContent + (b.getAttribute("aria-pressed") === "true" ? "*" : "")).join(" ");
      const crumbs = () => [...document.querySelectorAll("nav[aria-label=breadcrumb] li:not([role])")]
        .map((li) => li.querySelector(":scope > a, :scope > span")).map((e) => e.tagName === "A" ? `{${e.textContent}}` : e.textContent + (e.getAttribute("aria-current") ? "@" : "")).join(" ");
      const pager = () => [...document.querySelectorAll("nav[aria-label=pagination] li")].map((li) => {
        const a = li.querySelector(":scope > a, :scope > span");
        const t = a.textContent.replace("‹ Previous", "prev").replace("Next ›", "next");
        return a.getAttribute("aria-current") ? `[${t}]` : a.getAttribute("aria-disabled") === "true" ? `(${t})` : t;
      }).join(" ");
      const step = (label) => out.push(`${label} | ${radios()} | ${toggles()} | ${crumbs()} | ${pager()}`);
      step("start");
      set("opts", ["a", "b", { value: "c", label: "C", disabled: true }]); await tick(); step("options");
      document.querySelector("#radios input[value=a]").click(); await tick(); step("chose a");
      set("crumbs", [{ label: "Home", href: "/" }, { label: "Docs", href: "/docs" }, { label: "Lists" }]); await tick(); step("crumbs");
      set("pages", 10); set("page", 5); await tick(); step("page 5 of 10");
      document.querySelector("nav[aria-label=pagination] a[aria-label='Go to next page']").getAttribute("href") === "#6" || out.push("next href wrong");
      set("page", 10); await tick(); step("last");
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n");
      document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("lists-state", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "start | a b* | a b* | {Home} Docs@ | (prev) [1] 2 3 next",
            "options | a b* c! | a b* C | {Home} Docs@ | (prev) [1] 2 3 next",
            "chose a | a* b c! | a* b C | {Home} Docs@ | (prev) [1] 2 3 next",
            "crumbs | a* b c! | a* b C | {Home} {Docs} Lists@ | (prev) [1] 2 3 next",
            "page 5 of 10 | a* b c! | a* b C | {Home} {Docs} Lists@ | prev 1 … 4 [5] 6 … 10 next",
            "last | a* b c! | a* b C | {Home} {Docs} Lists@ | prev 1 … 9 [10] (next)",
        ],
        "\n{result}"
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn split_buttons_in_chrome() {
    let set = |v: &str| serde_json::json!({ "click": { "action": "set_state", "key": "last", "value": v } });
    let doc = serde_json::json!({ "version": 1, "state": { "last": null },
        // Room on the left, so the menu can line up with the arrow instead of the window's edge.
        "root": { "type": "stack", "style": { "padding-left": 240 }, "children": [
            { "type": "ui-split-button", "props": { "label": "Download PDF", "variant": "outline", "id": "dl" }, "on": set("pdf"),
              "children": [
                { "type": "ui-menu-item", "props": { "label": "Download Excel" }, "on": set("xlsx") },
                { "type": "ui-menu-item", "props": { "label": "Download CSV" }, "on": set("csv") } ] },
            { "type": "ui-split-button", "props": { "label": "Publish", "id": "pub" }, "on": set("publish"),
              "children": [ { "type": "ui-menu-item", "props": { "label": "Schedule" }, "on": set("schedule") } ] } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = [];
      const root = document.querySelector("[data-i-root]");
      const tick = () => new Promise((r) => setTimeout(r, 0));
      const btn = (t) => [...document.querySelectorAll("button")].find((b) => b.textContent.trim() === t);
      const arrow = (id) => document.querySelector(`[popovertarget=${id}]:not([role])`);
      const open = (id) => document.getElementById(id).matches(":popover-open");
      const step = (label) => out.push(`${label} ${root.__irender.state.last} ${open("dl")} ${open("pub")}`);
      btn("Download PDF").click(); await tick(); step("primary");
      arrow("dl").click(); await tick(); step("arrow");
      const r = (el) => el.getBoundingClientRect();
      out.push("aligned " + Math.round(r(document.getElementById("dl")).right - r(arrow("dl")).right)
        + " joined " + Math.round(r(arrow("dl")).left - r(btn("Download PDF")).right));
      btn("Download CSV").click(); await tick(); step("item");
      arrow("pub").click(); await tick(); step("other arrow");
      btn("Schedule").click(); await tick(); step("other item");
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n");
      document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let mut doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    irender_ui_kit::apply_theme(&mut doc);
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("split-button", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            // The primary action runs without opening anything.
            "primary pdf false false",
            "arrow pdf true false",
            // The menu's right edge sits under the arrow's; the arrow overlaps one border.
            "aligned 0 joined -1",
            "item csv false false",
            "other arrow csv false true",
            "other item schedule false false",
        ],
        "\n{result}"
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn sidebar_collapses_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "open": true, "open2": true },
        "root": { "type": "stack", "children": [
            { "type": "ui-sidebar-layout", "props": { "bind": "open", "collapsible": "icon" }, "attrs": { "id": "icons" }, "style": { "min-height": 200 }, "children": [
                { "type": "ui-sidebar", "children": [ { "type": "ui-sidebar-content", "children": [
                    { "type": "ui-sidebar-group", "props": { "label": "Platform" }, "children": [
                        { "type": "ui-sidebar-item", "props": { "label": "Home", "icon": "H", "href": "#" } },
                        { "type": "ui-sidebar-item", "props": { "label": "Docs", "icon": "D", "open": true }, "children": [
                            { "type": "ui-sidebar-item", "props": { "label": "Intro", "href": "#" } } ] } ] } ] } ] },
                { "type": "ui-sidebar-trigger" } ] },
            { "type": "ui-sidebar-layout", "props": { "bind": "open2" }, "attrs": { "id": "slide" }, "style": { "min-height": 200 }, "children": [
                { "type": "ui-sidebar", "children": [ { "type": "ui-sidebar-content" } ] },
                { "type": "ui-sidebar-trigger" } ] } ] } });
    // Widths are measured straight after a click, so no transitions.
    let driver = r##"<style>* { transition: none !important; }</style><script>
    window.addEventListener("load", async () => {
      const out = [];
      const wait = () => new Promise((r) => setTimeout(r, 50));
      const $ = (s) => document.querySelector(s);
      const width = (id) => Math.round($(`#${id} > aside`).getBoundingClientRect().width);
      const label = $("#icons [data-label]");
      const shown = (el) => el.getBoundingClientRect().width > 1;
      const step = (name) => out.push(`${name} ${$("#icons").dataset.state} ${width("icons")} label ${shown(label) ? "shown" : "hidden"} ${label.textContent}`
        + ` sub ${$("#icons details ul").checkVisibility() ? "shown" : "hidden"} trigger ${$("#icons [data-sidebar-trigger]").getAttribute("aria-expanded")}`);
      step("open");
      $("#icons [data-sidebar-trigger]").click(); await wait(); step("icons");
      $("#icons [data-sidebar-trigger]").click(); await wait(); step("open again");
      $("#slide [data-sidebar-trigger]").click(); await wait();
      out.push(`offcanvas ${$("#slide").dataset.state} ${width("slide")}`);
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n");
      document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let mut doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    irender_ui_kit::apply_theme(&mut doc);
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("sidebar", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "open expanded 256 label shown Home sub shown trigger true",
            // Narrowed to icons: labels are hidden from sight but still read, sub-menus go.
            "icons collapsed 48 label hidden Home sub hidden trigger false",
            "open again expanded 256 label shown Home sub shown trigger true",
            // Off-canvas, it slides away entirely.
            "offcanvas collapsed 0",
        ],
        "\n{result}"
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn command_palette_in_chrome() {
    let item = |label: &str, extra: serde_json::Value| {
        let mut props = serde_json::json!({ "label": label });
        props.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        serde_json::json!({ "type": "ui-command-item", "props": props,
            "on": { "click": { "action": "set_state", "key": "ran", "value": format!("{{{{ ran + '{label};' }}}}") } } })
    };
    let doc = serde_json::json!({ "version": 1, "state": { "ran": "" },
        "root": { "type": "stack", "children": [
            { "type": "ui-command", "props": { "id": "cmd" }, "children": [
                { "type": "ui-command-group", "props": { "heading": "Suggestions" }, "children": [
                    item("Calendar", serde_json::json!({})),
                    item("Search emoji", serde_json::json!({ "keywords": ["smiley"] })),
                    item("Calculator", serde_json::json!({ "disabled": true })) ] },
                { "type": "ui-command-separator" },
                { "type": "ui-command-group", "props": { "heading": "Settings" }, "children": [
                    item("Profile", serde_json::json!({ "shortcut": "⌘P" })),
                    item("Settings", serde_json::json!({ "keywords": "preferences" })) ] } ] },
            { "type": "ui-command-dialog", "props": { "id": "dlg", "trigger": "Open", "shortcut": "mod+k" }, "children": [
                item("New project", serde_json::json!({})),
                item("Delete project", serde_json::json!({})) ] } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = [];
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const tick = () => new Promise((r) => setTimeout(r, 0));
      const input = (p) => p.querySelector("input");
      const type = async (p, text) => { input(p).value = text; input(p).dispatchEvent(new Event("input", { bubbles: true })); await tick(); };
      const key = async (target, k, mods = {}) => { target.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true, ...mods })); await tick(); };
      // Items on screen, the active one marked *; then the groups, separators and empty message on screen.
      const shows = (e) => e.getClientRects().length > 0;
      const view = (p) => {
        const items = [...p.querySelectorAll("[role=option]")].filter(shows)
          .map((i) => i.querySelector("[data-ui-label]").textContent + (i.getAttribute("aria-selected") === "true" ? "*" : "") + (i.disabled ? "!" : ""));
        const groups = [...p.querySelectorAll("[role=group]")].filter(shows).length;
        const seps = [...p.querySelectorAll("[role=separator]")].filter(shows).length;
        const active = input(p).getAttribute("aria-activedescendant");
        const ok = !active || document.getElementById(active)?.getAttribute("aria-selected") === "true";
        return `${items.join(",")} | groups ${groups} seps ${seps} empty ${shows(p.querySelector("[data-ui-empty]"))}${ok ? "" : " BAD-ACTIVEDESCENDANT"}`;
      };
      const p = document.getElementById("cmd");
      out.push("start " + view(p));
      await type(p, "set"); out.push("set " + view(p));
      await type(p, "SMIL"); out.push("keyword " + view(p));
      await type(p, "sea emo"); out.push("words " + view(p));
      await type(p, "zzz"); out.push("none " + view(p));
      await key(input(p), "Escape"); out.push("escape '" + input(p).value + "' " + view(p));
      await key(input(p), "ArrowDown"); await key(input(p), "ArrowDown"); out.push("down down " + view(p));
      await key(input(p), "ArrowUp"); await key(input(p), "ArrowUp"); await key(input(p), "ArrowUp"); out.push("up x3 " + view(p));
      await key(input(p), "Home"); out.push("home " + view(p));
      await key(input(p), "Enter"); out.push("enter " + S().ran);
      p.querySelectorAll("[role=option]")[2].click(); await tick(); out.push("disabled click " + S().ran);

      const dlg = document.getElementById("dlg"), dp = dlg.querySelector("[data-ui-command]");
      const apple = /Mac|iPhone|iPad/.test(navigator.platform);
      await key(document.body, "k", { metaKey: apple, ctrlKey: !apple });
      out.push(`shortcut open ${dlg.open} modal ${dlg.matches(":modal")} focused ${document.activeElement === input(dp)}`);
      await type(dp, "del"); await key(input(dp), "Enter");
      out.push(`ran ${S().ran} open ${dlg.open}`);
      document.querySelector("[commandfor=dlg]").click(); await tick();
      out.push(`button open ${dlg.open} cleared '${input(dp).value}' ${view(dp)}`);
      await key(document.body, "k", { metaKey: apple, ctrlKey: !apple });
      out.push(`shortcut closes ${!dlg.open}`);
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n");
      document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("command", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "start Calendar*,Search emoji,Calculator!,Profile,Settings | groups 2 seps 1 empty false",
            "set Settings* | groups 1 seps 0 empty false",
            "keyword Search emoji* | groups 1 seps 0 empty false",
            "words Search emoji* | groups 1 seps 0 empty false",
            "none  | groups 0 seps 0 empty true",
            "escape '' Calendar*,Search emoji,Calculator!,Profile,Settings | groups 2 seps 1 empty false",
            // The disabled item is skipped.
            "down down Calendar,Search emoji,Calculator!,Profile*,Settings | groups 2 seps 1 empty false",
            // Up wraps from the first to the last.
            "up x3 Calendar,Search emoji,Calculator!,Profile,Settings* | groups 2 seps 1 empty false",
            "home Calendar*,Search emoji,Calculator!,Profile,Settings | groups 2 seps 1 empty false",
            "enter Calendar;",
            "disabled click Calendar;",
            "shortcut open true modal true focused true",
            "ran Calendar;Delete project; open false",
            "button open true cleared '' New project*,Delete project | groups 0 seps 0 empty false",
            "shortcut closes true",
        ],
        "\n{result}"
    );
}

/// ⌘B / Ctrl+B and labels on a wide screen, and the sheet on a phone (a 390px iframe
/// of the same page, since media queries follow the frame's width).
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn sidebar_shortcut_labels_and_phone_sheet_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "open": true },
        "root": { "type": "ui-sidebar-layout", "props": { "bind": "open", "collapsible": "icon" }, "children": [
            { "type": "ui-sidebar", "children": [ { "type": "ui-sidebar-content", "children": [
                { "type": "ui-sidebar-group", "children": [
                    { "type": "ui-sidebar-item", "props": { "label": "Home", "icon": "H", "href": "#home" } },
                    { "type": "ui-sidebar-item", "props": { "label": "Docs", "icon": "D", "href": "#docs" } } ] } ] } ] },
            { "type": "ui-sidebar-trigger" } ] } });
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let mut doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    irender_ui_kit::apply_theme(&mut doc);
    let ir = engine.resolve(&doc).unwrap();
    let page = engine.render(&ir, &serde_json::json!({})).unwrap().document("test");
    let page = page.replace("</body>", "<style>* { transition: none !important; }</style></body>");
    let frame = serde_json::to_string(&page).unwrap().replace("</", "<\\/");
    let driver = format!(
        r##"<script>const FRAME = {frame};</script><script>
    window.addEventListener("load", async () => {{
      const out = [];
      try {{
      const wait = (ms = 50) => new Promise((r) => setTimeout(r, ms));
      const key = (doc, k, mods = {{}}) => doc.dispatchEvent(new (doc.defaultView.KeyboardEvent)("keydown", {{ key: k, bubbles: true, cancelable: true, ...mods }}));
      // Wide: the shortcut flips `bind`; collapsed, each item's label shows beside it.
      const layout = document.querySelector("[data-sidebar-layout]");
      key(document, "b", {{ ctrlKey: true }}); await wait();
      const item = document.querySelector("[data-sidebar-item]");
      const tip = getComputedStyle(item, "::after");
      out.push(`wide ${{layout.dataset.state}} tip ${{tip.content}} ${{tip.opacity}} panel ${{getComputedStyle(layout.querySelector("aside")).overflow}}`);
      key(document, "b", {{ metaKey: true }}); await wait();
      out.push(`wide ${{layout.dataset.state}}`);

      // A phone: the trigger opens a sheet and leaves `bind` alone.
      const iframe = document.createElement("iframe");
      iframe.style.cssText = "width: 390px; height: 600px; border: 0";
      iframe.srcdoc = FRAME;
      await new Promise((r) => {{ iframe.onload = r; document.body.append(iframe); }});
      await wait(200);
      const d = iframe.contentDocument, w = iframe.contentWindow;
      // Links stay on the page, as in an app that routes in the browser.
      d.addEventListener("click", (e) => e.target.closest("a") && e.preventDefault());
      const lay = d.querySelector("[data-sidebar-layout]"), aside = lay.querySelector("aside"), main = lay.querySelector("main");
      const trigger = d.querySelector("[data-sidebar-trigger]");
      const rect = () => Math.round(aside.getBoundingClientRect().left) + "," + Math.round(aside.getBoundingClientRect().height);
      const sheet = (label) => out.push(`${{label}} ${{lay.hasAttribute("data-mobile-open") ? "open" : "shut"}} ${{w.getComputedStyle(aside).visibility}} ${{rect()}}`
        + ` inert ${{main.inert}} expanded ${{trigger.getAttribute("aria-expanded")}} bind ${{lay.__irender?.state?.open ?? d.querySelector("[data-i-root]").__irender.state.open}}`
        + ` focus ${{d.activeElement === trigger ? "trigger" : aside.contains(d.activeElement) ? "panel" : d.activeElement.tagName}}`);
      sheet("phone");
      trigger.focus(); trigger.click(); await wait(); sheet("tap");
      key(d, "Escape"); await wait(); sheet("escape");
      trigger.click(); await wait();
      lay.dispatchEvent(new w.MouseEvent("click", {{ bubbles: true, cancelable: true }})); await wait(); sheet("backdrop");
      trigger.click(); await wait();
      aside.querySelector("a[href]").click(); await wait(); sheet("link");
      key(d, "b", {{ ctrlKey: true }}); await wait(); sheet("shortcut");
      }} catch (e) {{ out.push("EXCEPTION " + e.stack); }}
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n");
      document.body.append(pre);
    }});
    </script>"##
    );
    let html = page.replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("sidebar-phone", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            // Collapsed to icons: the label is ready beside the item (shown on hover or focus),
            // and the panel lets it out.
            r#"wide collapsed tip "Home" / "" 0 panel visible"#,
            "wide expanded",
            // Hidden off to the left until the trigger opens it over the page.
            "phone shut hidden -256,600 inert false expanded false bind true focus BODY",
            "tap open visible 0,600 inert true expanded true bind true focus panel",
            "escape shut hidden -256,600 inert false expanded false bind true focus trigger",
            "backdrop shut hidden -256,600 inert false expanded false bind true focus trigger",
            "link shut hidden -256,600 inert false expanded false bind true focus trigger",
            "shortcut open visible 0,600 inert true expanded true bind true focus panel",
        ],
        "\n{result}"
    );
}

/// A theme option picked on one page applies on the next before its content shows,
/// keeps every picker in step, and its dark colours win in dark mode. Picking the
/// default clears it.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn theme_options_apply_and_persist_in_chrome() {
    let doc = serde_json::json!({ "version": 1,
        "theme": { "options": { "palette": { "choices": { "rose": { "tokens": { "color": { "primary": "rgb(2, 2, 2)" } },
            "modes": { "dark": { "color": { "primary": "rgb(3, 3, 3)" } } } } } } } },
        "root": { "type": "stack", "children": [
            { "type": "ui-theme-option", "props": { "option": "palette" }, "attrs": { "id": "a" } },
            { "type": "ui-theme-option", "props": { "option": "palette" }, "attrs": { "id": "b" } },
            { "type": "ui-theme-option", "props": { "option": "density" } },
            { "type": "ui-button", "props": { "label": "Probe" }, "attrs": { "id": "probe" } } ] } });
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let mut doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    irender_ui_kit::apply_theme(&mut doc);
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test");
    let probe = html.find("id=\"probe\"").unwrap();
    let after = probe + html[probe..].find("</button>").unwrap() + 9;
    // Before the runtime: the background the stored choice gave the button.
    let early = r#"<script>window.__early = getComputedStyle(document.getElementById("probe")).backgroundColor</script>"#;
    // Colours are read straight after a click, so no transitions.
    let driver = r##"<style>* { transition: none !important; }</style><script>
    window.addEventListener("load", async () => {
      const out = [];
      const tick = () => new Promise((r) => setTimeout(r, 50));
      const bg = () => getComputedStyle(document.getElementById("probe")).backgroundColor;
      const pressed = (id) => [...document.querySelectorAll(`#${id} [data-choice=rose], #${id} [data-choice=zinc]`)].map((b) => b.dataset.choice + (b.getAttribute("aria-pressed") === "true" ? "*" : "")).join(" ");
      const height = () => Math.round(document.getElementById("probe").getBoundingClientRect().height);
      out.push(`stored ${window.__early} ${pressed("a")} | ${pressed("b")} ${document.documentElement.dataset.themePalette}`);
      document.querySelector("#b [data-choice=zinc]").click(); await tick();
      out.push(`zinc ${bg()} ${pressed("a")} attr ${document.documentElement.getAttribute("data-theme-palette")} stored ${localStorage.getItem("irender-theme-options")}`);
      document.querySelector("#a [data-choice=rose]").click(); await tick();
      window.__irenderTheme.choose("dark"); await tick();
      out.push(`rose dark ${bg()} ${pressed("b")}`);
      const before = height();
      document.querySelector("[data-choice=compact]").click(); await tick();
      out.push(`compact ${before} to ${height()}`);
      // Another page of the site picks blue and clears density: this one follows.
      localStorage.setItem("irender-theme-options", '{"palette":"blue"}');
      dispatchEvent(new StorageEvent("storage", { key: "irender-theme-options" })); await tick();
      out.push(`other page ${document.documentElement.dataset.themePalette} ${document.documentElement.dataset.themeDensity} ${pressed("a")}`);
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n");
      document.body.append(pre);
    });
    </script>"##;
    let html = format!("{}{early}{}", &html[..after], &html[after..])
        .replace("<head>", r#"<head><script>localStorage.setItem("irender-theme-options", '{"palette":"rose"}'); localStorage.removeItem("irender-theme")</script>"#)
        .replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("theme-options", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "stored rgb(2, 2, 2) zinc rose* | zinc rose* rose",
            // The default is no attribute and nothing stored: the base tokens.
            r#"zinc rgb(24, 24, 27) zinc* rose attr null stored {}"#,
            "rose dark rgb(3, 3, 3) zinc rose*",
            // Compact density: a 36px control is 0.8 × 36.
            "compact 36 to 29",
            "other page blue undefined zinc rose",
        ],
        "\n{result}"
    );
}

/// Custom backgrounds are kept per mode and set the page's, cards' and popovers'
/// colours; a gradient end turns the brand fills into a gradient, and clearing it
/// goes back to the plain colour.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn custom_backgrounds_and_gradients_in_chrome() {
    let doc = serde_json::json!({ "version": 1,
        "root": { "type": "stack", "children": [
            { "type": "ui-theme-colors" },
            { "type": "ui-card", "attrs": { "id": "card" }, "children": [{ "type": "text", "props": { "value": "Card" } }] },
            { "type": "ui-button", "props": { "label": "Main" }, "attrs": { "id": "main" } } ] } });
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let mut doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    irender_ui_kit::apply_theme(&mut doc);
    let ir = engine.resolve(&doc).unwrap();
    let driver = r##"<style>* { transition: none !important; }</style><script>
    window.addEventListener("load", async () => {
      const out = [];
      const tick = () => new Promise((r) => setTimeout(r, 50));
      const theme = window.__irenderTheme;
      const css = (el, prop) => getComputedStyle(el)[prop];
      const card = document.getElementById("card"), main = document.getElementById("main");
      const row = (n) => document.querySelector(`[data-color="${n}"]`).querySelectorAll("input");
      const type = async (n, val) => { const t = row(n)[1]; t.value = val; t.dispatchEvent(new Event("input", { bubbles: true })); await tick(); };
      const mode = () => document.querySelector("[data-mode]").textContent;
      theme.choose("light"); await tick();
      // The sidebar's field starts from its colour as the browser resolves it.
      out.push(`start ${row("card")[1].value} ${row("sidebar")[1].value} ${mode()}`);
      await type("card", "#fef3c7");
      await type("background", "#f4f4f5");
      out.push(`light ${css(card, "backgroundColor")} page ${css(document.body, "backgroundColor")} ${document.documentElement.dataset.themePalette}`);
      // What sits on the card (pinned table columns, tints) sees its colour.
      out.push(`ui-bg ${getComputedStyle(card).getPropertyValue("--ui-bg").trim()}`);
      theme.choose("dark"); await tick();
      out.push(`dark ${css(card, "backgroundColor")} field ${row("card")[1].value} ${mode()}`);
      await type("card", "#1e293b");
      out.push(`dark card ${css(card, "backgroundColor")}`);
      theme.choose("light"); await tick();
      out.push(`light again ${css(card, "backgroundColor")} field ${row("card")[1].value}`);
      const stored = JSON.parse(localStorage.getItem("irender-theme-custom"));
      out.push(`stored ${stored.card} ${stored["card-dark"]} ${stored.background}`);
      await type("primary-to", "#ec4899");
      out.push(`gradient ${css(main, "backgroundImage").startsWith("linear-gradient(135deg, rgb(24, 24, 27), rgb(236, 72, 153))")} on ${css(main, "color")}`);
      await type("primary-to", "");
      out.push(`cleared ${css(main, "backgroundImage")} ${css(main, "backgroundColor")} ${row("primary-to")[1].value === ""}`);
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n");
      document.body.append(pre);
    });
    </script>"##;
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test")
        .replace("<head>", r#"<head><script>localStorage.clear()</script>"#)
        .replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("custom-backgrounds", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "start #ffffff #f8f8f8 light mode",
            "light rgb(254, 243, 199) page rgb(244, 244, 245) custom",
            "ui-bg #fef3c7",
            // Dark mode has no card colour of its own yet: the theme's.
            "dark rgb(9, 9, 11) field #09090b dark mode",
            "dark card rgb(30, 41, 59)",
            "light again rgb(254, 243, 199) field #fef3c7",
            "stored #fef3c7 #1e293b #f4f4f5",
            // The text is light, for the gradient from zinc to pink as a whole.
            "gradient true on rgb(250, 250, 250)",
            "cleared none rgb(24, 24, 27) true",
        ],
        "\n{result}"
    );
}

/// Custom colours: a hex typed or picked picks the palette's `custom` choice, with
/// readable text on it; an incomplete one is flagged and ignored; the chart scheme
/// follows them; another palette drops them, and Reset clears them.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn custom_colours_in_chrome() {
    let doc = serde_json::json!({ "version": 1,
        "theme": { "options": { "chart": { "choices": { "default": {}, "custom": {} } } } },
        "root": { "type": "stack", "children": [
            { "type": "ui-theme-panel" },
            { "type": "ui-button", "props": { "label": "Main" }, "attrs": { "id": "main" } },
            { "type": "ui-button", "props": { "label": "Second", "variant": "brand" }, "attrs": { "id": "second" } } ] } });
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let mut doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    irender_ui_kit::apply_theme(&mut doc);
    let ir = engine.resolve(&doc).unwrap();
    let driver = r##"<style>* { transition: none !important; }</style><script>
    window.addEventListener("load", async () => {
      const out = [];
      const tick = () => new Promise((r) => setTimeout(r, 50));
      const html = document.documentElement, v = (n) => getComputedStyle(html).getPropertyValue(n).trim();
      const bg = (id) => getComputedStyle(document.getElementById(id)).backgroundColor;
      const fg = (id) => getComputedStyle(document.getElementById(id)).color;
      const row = (n) => document.querySelector(`[data-color="${n}"]`).querySelectorAll("input");
      const p = row("primary"), b = row("brand-2");
      const type = async (input, val) => { input.value = val; input.dispatchEvent(new Event("input", { bubbles: true })); await tick(); };
      await type(p[1], "#ff0");
      out.push(`yellow ${bg("main")} on ${fg("main")} ${html.dataset.themePalette} ${p[0].value} charts ${html.dataset.themeChart}`);
      await type(p[1], "#12");
      out.push(`partial ${p[1].getAttribute("aria-invalid")} ${bg("main")}`);
      await type(p[1], "#1e3a8a");
      p[0].value = "#f97316"; b[0].value = "#f97316"; b[0].dispatchEvent(new Event("input", { bubbles: true })); await tick();
      out.push(`navy ${bg("main")} on ${fg("main")} second ${bg("second")} ${b[1].value}`);
      out.push(`chart ${v("--color-chart-1")} ${v("--color-chart-2")}`);
      document.querySelector("[aria-label=Colour] [data-choice=rose]").click(); await tick();
      out.push(`rose ${bg("main")}`);
      document.querySelector("[data-reset]").click(); await tick();
      out.push(`reset ${localStorage.getItem("irender-theme-custom")} charts ${html.dataset.themeChart}`);
      // A chart scheme picked by hand stays when colours change.
      document.querySelector("[aria-label=chart] [data-choice=default]").click(); await tick();
      localStorage.setItem("irender-theme-options", JSON.stringify({ chart: "mono" }));
      window.__irenderTheme.setOption("chart", "mono");
      await type(p[1], "#123456");
      out.push(`kept ${html.dataset.themeChart}`);
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n");
      document.body.append(pre);
    });
    </script>"##;
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test")
        .replace("<head>", r#"<head><script>localStorage.clear()</script>"#)
        .replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("custom-colours", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            // Yellow gets dark text, navy light text.
            // Charts follow the custom colours while they're on their default.
            "yellow rgb(255, 255, 0) on rgb(9, 9, 11) custom #ffff00 charts custom",
            "partial true rgb(255, 255, 0)",
            "navy rgb(30, 58, 138) on rgb(250, 250, 250) second rgb(249, 115, 22) #f97316",
            "chart #1e3a8a #f97316",
            "rose rgb(225, 29, 72)",
            "reset {} charts undefined",
            "kept mono",
        ],
        "\n{result}"
    );
}

/// Reads a whole HTTP request: the headers, then the body up to its Content-Length.
/// WebKit sends the body in a separate packet, so one read can miss it, and closing
/// the connection before it's read fails the fetch.
fn read_request(stream: &mut std::net::TcpStream) -> String {
    use std::io::Read;
    let mut data = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = stream.read(&mut buf).unwrap_or(0);
        data.extend_from_slice(&buf[..n]);
        let text = String::from_utf8_lossy(&data);
        if let Some(end) = text.find("\r\n\r\n") {
            let length = text[..end].lines()
                .find_map(|line| line.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse().unwrap_or(0)))
                .unwrap_or(0);
            if n == 0 || data.len() >= end + 4 + length {
                return text.into_owned();
            }
        } else if n == 0 {
            return text.into_owned();
        }
    }
}

/// A chat API: answers `POST /chat` with the reply as server-sent events, and the
/// preflight a file:// page sends first. `/json` answers with one JSON message instead.
fn chat_server() -> u16 {
    use std::io::Write;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let request = read_request(&mut stream);
            let cors = "Access-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: content-type\r\nAccess-Control-Allow-Methods: POST";
            if request.starts_with("OPTIONS") {
                let _ = write!(stream, "HTTP/1.1 204 No Content\r\n{cors}\r\nConnection: close\r\n\r\n");
                continue;
            }
            // How many messages it was sent, so the reply shows the whole conversation arrived.
            let sent = request.matches("\"content\"").count();
            let module = [("GET /ok.wasm", 0, "<b>From the browser</b>"), ("GET /err.wasm", 1, "no such component")]
                .into_iter()
                .find(|(route, _, _)| request.starts_with(route));
            if let Some((_, status, out)) = module {
                let module = fake_renderer(status, out);
                let _ = write!(stream, "HTTP/1.1 200 OK\r\n{cors}\r\nContent-Type: application/wasm\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", module.len());
                let _ = stream.write_all(&module);
                continue;
            }
            if request.starts_with("GET ") {
                let _ = write!(stream, "HTTP/1.1 404 Not Found\r\n{cors}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                continue;
            }
            if request.starts_with("POST /render") {
                let ok = request.contains(r#"{"ui":{"type":"ui-badge"#);
                let body = if ok { "<i>From the server</i>" } else { "bad request" };
                let _ = write!(stream, "HTTP/1.1 {}\r\n{cors}\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", if ok { "200 OK" } else { "400 Bad Request" }, body.len());
                continue;
            }
            if request.starts_with("POST /ui") {
                let html = serde_json::to_string(&serde_json::json!({ "html": message_ui() })).unwrap();
                let _ = write!(stream, "HTTP/1.1 200 OK\r\n{cors}\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\ndata: {{\"content\":\"Here\"}}\n\ndata: {html}\n\ndata: [DONE]\n\n");
                continue;
            }
            if request.starts_with("POST /json") {
                let body = format!(r#"{{"content":"JSON reply to {sent}","name":"Bot"}}"#);
                let _ = write!(stream, "HTTP/1.1 200 OK\r\n{cors}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                continue;
            }
            let _ = write!(stream, "HTTP/1.1 200 OK\r\n{cors}\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n");
            for chunk in [r#"{"content":"Hi"}"#, r#"{"choices":[{"delta":{"content":" there"}}]}"#, &format!(r#"" ({sent})""#), "[DONE]"] {
                let _ = write!(stream, "data: {chunk}\n\n");
                let _ = stream.flush();
            }
        }
    });
    port
}

/// Sending appends the user's message, shows the typing indicator, and streams the
/// reply in; suggestions hide once the conversation starts and send when clicked; a
/// JSON endpoint's reply is one message; threads unfold.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn chat_sends_and_streams_replies_in_chrome() {
    let port = chat_server();
    let input = |endpoint: &str| serde_json::json!({ "type": "ui-chat-input",
        "props": { "messages": "messages", "bind": "draft", "waiting": "waiting", "endpoint": endpoint, "suggestions": ["Suggested"] },
        "on": { "send": { "action": "set_state", "key": "last", "value": "{{ draft }}" } } });
    let doc = serde_json::json!({ "version": 1,
        "state": { "messages": [{ "id": "t", "content": "Thread", "replies": [{ "content": "Reply" }] }], "draft": "", "waiting": false, "last": null, "json": [] },
        "root": { "type": "stack", "children": [
            { "type": "ui-conversation", "props": { "messages": "{{ messages }}", "typing": "{{ waiting }}", "height": 200 } },
            input(&format!("http://127.0.0.1:{port}/chat")),
            { "type": "ui-chat-input", "props": { "messages": "json", "endpoint": format!("http://127.0.0.1:{port}/json"), "suggestions": ["Go"] } } ] } });
    let driver = r#"<script>
    window.addEventListener("load", async () => {
      const out = [];
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      // Waits for a condition, in real time: Chrome's virtual time jumps ahead while the
      // page is idle, faster than the test server answers under load, so this yields
      // through a message channel instead of a timer, which keeps virtual time still.
      const tick = () => new Promise((r) => { const c = new MessageChannel(); c.port1.onmessage = r; c.port2.postMessage(0); });
      const until = async (f) => { for (let i = 0; i < 2e6 && !f(); i++) await tick(); };
      // Runs the updates queued as microtasks, but no timer or network task, so the
      // reply can't have arrived yet.
      const settle = async () => { for (let i = 0; i < 10; i++) await Promise.resolve(); };
      const [form, other] = document.querySelectorAll("form");
      const box = form.querySelector("textarea"), send = form.querySelector("[type=submit]");
      const log = document.querySelector("[role=log]");
      const bubbles = () => [...log.querySelectorAll(":scope > div > [data-role]")].map((m) => m.dataset.role + ":" + m.querySelector("[data-bubble]").textContent).join(" | ");
      out.push(`start ${send.disabled} suggestions=${!form.querySelector("[data-suggestions]").hidden}`);
      log.querySelector("summary").click();
      out.push(`thread ${log.querySelector("details").open} ${log.querySelector("details [data-bubble]").textContent}`);
      box.value = "Hello"; box.dispatchEvent(new Event("input", { bubbles: true }));
      out.push(`typed ${send.disabled} ${S().draft}`);
      box.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
      await settle();
      out.push(`sent ${S().messages.length} last=${S().last} draft=${JSON.stringify(S().draft)} box=${JSON.stringify(box.value)} waiting=${S().waiting} typing=${!!log.querySelector("[aria-label=Typing]")} stop=${send.getAttribute("aria-label")}`);
      // The reply is complete when the input stops waiting, which it does at the
      // stream's end; under load that can take a while.
      await until(() => S().messages.at(-1).role === "assistant" && !S().waiting);
      // The send button turns back from Stop just after.
      await until(() => send.getAttribute("aria-label") === "Send");
      await wait(20);
      out.push(`reply ${bubbles()}`);
      out.push(`after waiting=${S().waiting} typing=${!!log.querySelector("[aria-label=Typing]")} send=${send.getAttribute("aria-label")} suggestions=${!form.querySelector("[data-suggestions]").hidden} bottom=${log.scrollHeight - log.scrollTop - log.clientHeight < 2}`);
      other.querySelector("[data-suggestion]").click();
      await until(() => S().json.length >= 2);
      out.push(`json ${S().json.map((m) => m.role + ":" + m.content + (m.name ? "@" + m.name : "")).join(" | ")}`);
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n");
      document.body.append(pre);
    });
    </script>"#;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("chat", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "start true suggestions=false",
            "thread true Reply",
            "typed false Hello",
            "sent 2 last=Hello draft=\"\" box=\"\" waiting=true typing=true stop=Stop",
            // Thread, its reply and Hello.
            "reply assistant:Thread | user:Hello | assistant:Hi there (3)",
            "after waiting=false typing=false send=Send suggestions=false bottom=true",
            "json user:Go | assistant:JSON reply to 1@Bot",
        ],
        "\n{result}"
    );
}

/// With `threads`, `messages` shows the current thread's: switching shows another's,
/// and sending with no thread starts one, titled after the message, at the top.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn chat_threads_in_chrome() {
    let port = chat_server();
    let doc = serde_json::json!({ "version": 1,
        "state": { "chat": [], "thread": "a", "waiting": false, "threads": [
            { "id": "a", "title": "First", "messages": [{ "id": 1, "role": "user", "content": "A1" }] },
            { "id": "b", "title": "Second", "messages": [{ "id": 1, "role": "user", "content": "B1" }] }] },
        "root": { "type": "stack", "children": [
            { "type": "ui-conversation", "props": { "messages": "{{ chat }}", "typing": "{{ waiting }}" } },
            { "type": "ui-chat-input", "props": { "messages": "chat", "threads": "threads", "thread": "thread", "waiting": "waiting",
                "endpoint": format!("http://127.0.0.1:{port}/chat") } } ] } });
    let driver = r#"<script>
    window.addEventListener("load", async () => {
      const out = [];
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      // Waits for a condition, in real time: Chrome's virtual time jumps ahead while the
      // page is idle, faster than the test server answers under load, so this yields
      // through a message channel instead of a timer, which keeps virtual time still.
      const tick = () => new Promise((r) => { const c = new MessageChannel(); c.port1.onmessage = r; c.port2.postMessage(0); });
      const until = async (f) => { for (let i = 0; i < 2e6 && !f(); i++) await tick(); };
      const box = document.querySelector("textarea");
      const shown = () => [...document.querySelectorAll("[role=log] [data-bubble]")].map((b) => b.textContent).join(",");
      const titles = () => S().threads.map((t) => t.title + ":" + t.messages.length).join(" ");
      // Runs the updates queued as microtasks, but no timer or network task, so the
      // reply can't have arrived yet.
      const settle = async () => { for (let i = 0; i < 10; i++) await Promise.resolve(); };
      const go = async (id) => { root.__irender.set("thread", id); await wait(0); };
      await wait(0);
      out.push(`start ${shown()}`);
      await go("b");
      out.push(`b ${shown()}`);
      await go(null);
      out.push(`new ${JSON.stringify(shown())}`);
      box.value = "A very long first message that makes the title"; box.dispatchEvent(new Event("input", { bubbles: true }));
      box.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
      await settle();
      const created = S().thread;
      out.push(`sent current=${created === S().threads[0].id} ${titles()} waiting=${S().waiting}`);
      await until(() => S().chat.length >= 2 && !S().waiting);
      await wait(20);
      out.push(`reply ${shown()}`);
      await go("a");
      out.push(`a ${shown()}`);
      await go(created);
      out.push(`back ${shown()}`);
      await go("b");
      box.value = "B2"; box.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
      // The thread list takes the finished reply after the conversation does.
      await until(() => S().chat.length >= 3 && !S().waiting && S().threads[0].messages.length >= 3);
      await wait(20);
      out.push(`top ${titles()}`);
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n");
      document.body.append(pre);
    });
    </script>"#;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("chat-threads", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "start A1",
            "b B1",
            "new \"\"",
            "sent current=true A very long first message that makes the…:1 First:1 Second:1 waiting=true",
            "reply A very long first message that makes the title,Hi there (1)",
            "a A1",
            "back A very long first message that makes the title,Hi there (1)",
            // Sending moves the thread to the top.
            "top Second:3 A very long first message that makes the…:2 First:1",
        ],
        "\n{result}"
    );
}

/// A reply's UI, as a host renders it: a counter with its own state, in a carousel.
fn message_ui() -> String {
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let mut doc = irender_core::parse(
        r#"{ "version": 1, "state": { "n": 5 }, "root": { "type": "ui-carousel", "props": { "size": 300 }, "children": [
            { "type": "ui-button", "props": { "label": "N {{ n }}" }, "attrs": { "id": "inner" },
              "on": { "click": { "action": "set_state", "key": "n", "value": "{{ n + 1 }}" } } },
            { "type": "list", "props": { "items": [1, 2, 3] }, "style": { "display": "contents" },
              "children": [{ "type": "text", "props": { "value": "Slide {{ item }}" } }] } ] } }"#,
        Format::Json,
    )
    .unwrap();
    irender_ui_kit::apply_theme(&mut doc);
    engine.render_embedded(&engine.resolve(&doc).unwrap(), &serde_json::json!({})).unwrap().embedded()
}

/// A reply's `html` is inserted into its message and started as a root of its own: its
/// state is its own, it survives the conversation redrawing, and its behaviours run.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn message_ui_in_chrome() {
    let port = chat_server();
    let doc = serde_json::json!({ "version": 1, "state": { "chat": [], "n": 0 },
        "root": { "type": "stack", "children": [
            { "type": "text", "props": { "value": "Outer {{ n }}" }, "attrs": { "id": "outer" } },
            { "type": "ui-conversation", "props": { "messages": "{{ chat }}" } },
            { "type": "ui-chat-input", "props": { "messages": "chat", "endpoint": format!("http://127.0.0.1:{port}/ui") } } ] } });
    let driver = r##"<style>* { scroll-behavior: auto !important; }</style><script>
    window.addEventListener("load", async () => {
      const out = [];
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const $ = (s) => document.querySelector(s);
      const box = $("textarea");
      box.value = "Show me"; box.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
      for (let i = 0; i < 50 && !$("#inner"); i++) await wait(20);
      out.push(`reply ${S().chat.at(-1).content} html=${S().chat.at(-1).html.startsWith("<style>")} roots=${document.querySelectorAll("[data-i-root]").length}`);
      $("#inner").click(); await wait(0);
      out.push(`clicked ${$("#inner").textContent} | ${$("#outer").textContent} chat-n=${"n" in S() ? S().n : "-"}`);
      root.__irender.set("chat", [...S().chat, { id: "x", role: "user", content: "again" }]); await wait(0);
      out.push(`redrawn ${$("#inner").textContent}`);
      const track = $("[data-track]"), next = $("[data-next]"), prev = $("[data-prev]");
      out.push(`slides ${track.querySelectorAll("[data-slide]").length} prev=${prev.disabled} next=${next.disabled}`);
      // Smooth scrolling doesn't advance under virtual time.
      const scrollBy = Element.prototype.scrollBy;
      Element.prototype.scrollBy = function (o) { scrollBy.call(this, { ...o, behavior: "instant" }); };
      next.click(); await wait(50);
      out.push(`next ${track.scrollLeft}`);
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n");
      document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("message-ui", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "reply Here html=true roots=2",
            // The counter's state is its own: the page's `n` is untouched.
            "clicked N 6 | Outer 0 chat-n=0",
            "redrawn N 6",
            "slides 4 prev=true next=false",
            // One slide and the gap.
            "next 312",
        ],
        "\n{result}"
    );
}

/// A WebAssembly module with irender-wasm's exports that answers every render with
/// `status` and `out`, to drive the page's loader without the real module.
fn fake_renderer(status: u8, out: &str) -> Vec<u8> {
    fn uleb(mut n: usize, to: &mut Vec<u8>) {
        loop {
            let b = (n & 0x7f) as u8;
            n >>= 7;
            to.push(if n == 0 { b } else { b | 0x80 });
            if n == 0 { break; }
        }
    }
    // `i32.const n` for small non-negative n, then `end`.
    fn constant(n: usize) -> Vec<u8> {
        let mut c = vec![0x41];
        let mut n = n as i64;
        loop {
            let b = (n & 0x7f) as u8;
            n >>= 7;
            let done = n == 0 && b & 0x40 == 0;
            c.push(if done { b } else { b | 0x80 });
            if done { break; }
        }
        c.push(0x0b);
        c
    }
    fn section(id: u8, body: Vec<u8>, to: &mut Vec<u8>) {
        to.push(id);
        uleb(body.len(), to);
        to.extend(body);
    }
    let name = |n: &str, kind: u8, index: u8| {
        let mut e = vec![n.len() as u8];
        e.extend(n.as_bytes());
        e.extend([kind, index]);
        e
    };
    let (input, output) = (1024, 4096);
    let mut m = b"\0asm\x01\0\0\0".to_vec();
    // (i32) -> i32, (i32, i32) -> i32, () -> i32
    section(1, vec![3, 0x60, 1, 0x7f, 1, 0x7f, 0x60, 2, 0x7f, 0x7f, 1, 0x7f, 0x60, 0, 1, 0x7f], &mut m);
    section(3, vec![4, 0, 1, 2, 2], &mut m);
    section(5, vec![1, 0, 1], &mut m);
    let mut exports = vec![5];
    for e in [name("memory", 2, 0), name("irender_alloc", 0, 0), name("irender_render", 0, 1), name("irender_out_ptr", 0, 2), name("irender_out_len", 0, 3)] {
        exports.extend(e);
    }
    section(7, exports, &mut m);
    let mut code = vec![4];
    for body in [constant(input), constant(status as usize), constant(output), constant(out.len())] {
        uleb(body.len() + 1, &mut code);
        code.push(0); // no locals
        code.extend(body);
    }
    section(10, code, &mut m);
    let mut data = vec![1, 0];
    data.extend(constant(output));
    uleb(out.len(), &mut data);
    data.extend(out.as_bytes());
    section(11, data, &mut m);
    m
}

/// A message's `ui` without `html` is rendered by the first way that works: the browser
/// module, then the server's endpoint; with neither, the message says so. `html` wins.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn message_ui_render_ways_in_chrome() {
    let port = chat_server();
    let at = |p: &str| format!("http://127.0.0.1:{port}{p}");
    let ui = serde_json::json!({ "type": "ui-badge", "props": { "text": "B" } });
    let message = |id: &str, render: serde_json::Value, html: Option<&str>| {
        let mut props = serde_json::json!({ "content": id, "ui": ui, "render": render });
        if let Some(h) = html {
            props["html"] = h.into();
        }
        serde_json::json!({ "type": "ui-message", "attrs": { "id": id }, "props": props })
    };
    let doc = serde_json::json!({ "version": 1, "root": { "type": "stack", "children": [
        message("browser", serde_json::json!({ "browser": at("/ok.wasm"), "url": at("/render") }), None),
        message("fallback", serde_json::json!({ "browser": at("/err.wasm"), "url": at("/render") }), None),
        message("missing", serde_json::json!({ "browser": at("/missing.wasm") }), None),
        message("server", serde_json::json!({ "url": at("/render") }), None),
        message("html", serde_json::json!({ "browser": at("/ok.wasm") }), Some("<u>Given</u>")) ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const ids = ["browser", "fallback", "missing", "server", "html"];
      const shown = () => ids.map((id) => id + ": " + document.querySelector(`#${id} [data-ui]`).innerHTML).join("\n");
      for (let i = 0; i < 100 && shown().includes(": \n"); i++) await wait(20);
      await wait(100);
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = shown();
      document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("message-ui-ways", &html) else { return };
    assert_eq!(
        result.replace("&lt;", "<").replace("&gt;", ">").lines().collect::<Vec<_>>(),
        [
            "browser: <b>From the browser</b>",
            "fallback: <i>From the server</i>",
            r#"missing: <p data-ui-note="">This reply has UI that can't be shown here.</p>"#,
            "server: <i>From the server</i>",
            "html: <u>Given</u>",
        ],
        "\n{result}"
    );
}

/// The tasks block (examples/blocks/data-table-01.yaml): sorting, search, a faceted
/// filter, paging, selection and the bulk bar, page size, hiding a column, keyboard.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn data_table_in_chrome() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/blocks/");
    let src = std::fs::read_to_string(format!("{dir}data-table-01.yaml")).unwrap();
    let data: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(format!("{dir}data-table-01.data.json")).unwrap()).unwrap();
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    // The block's icons are decoration; without the icon kit they render as text.
    let src = src.replace("type: ui-icon, props: { name: ", "type: text, props: { value: ").replace(", size: 16 }", " }");
    let mut doc = irender_core::parse(&src, Format::Yaml).unwrap_or_else(|d| panic!("{d}"));
    irender_ui_kit::apply_theme(&mut doc);
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const tick = () => new Promise((r) => setTimeout(r, 0));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const t = document.querySelector("[data-i-x=ui-data-table]");
      const q = (s) => t.querySelector(s), qa = (s) => [...t.querySelectorAll(s)];
      const vis = () => qa("[data-dt-body] > [data-dt-row]:not([hidden])");
      const col = (k) => vis().map((r) => r.querySelector(`[data-dt-col=${k}]`).textContent.trim());
      const status = () => q("[data-dt-status]").textContent + " | " + q("[data-dt-page]").textContent;
      const fire = (el, type) => el.dispatchEvent(new Event(type, { bubbles: true }));
      try {
        log("start", vis().length, status(), "reset", q("[data-dt-reset]").hidden ? "hidden" : "shown");
        log("priority", [...new Set(col("priority"))].join(","));
        q("[data-dt-sort=title]").click(); await tick();
        const titles = vis().map((r) => r.querySelector("[data-dt-col=title] span:last-child").textContent);
        log("title", q("th[data-dt-col=title]").getAttribute("aria-sort"), titles.every((x, i) => i === 0 || titles[i - 1].localeCompare(x) <= 0));
        q("[data-dt-sort=title]").click(); q("[data-dt-sort=title]").click(); await tick();
        log("title off", q("th[data-dt-col=title]").hasAttribute("aria-sort"));
        const s = q("[data-dt-search]"); s.value = "okta"; fire(s, "input"); await tick();
        log("search", vis().length, status(), "reset", q("[data-dt-reset]").hidden ? "hidden" : "shown");
        s.value = ""; fire(s, "input"); await tick();
        const trig = q("[data-dt-trigger=status]"); trig.click(); await tick();
        const pop = q("[data-dt-filter=status]");
        log("status filter", pop.matches(":popover-open"), [...pop.querySelectorAll("[data-dt-option]")].map((l) => l.textContent).join(" "));
        [...pop.querySelectorAll("[data-dt-pick]")].find((b) => b.__v === "Done").click(); await tick();
        log("done", status(), trig.querySelector("[data-dt-chips]").textContent, [...new Set(col("status"))].join(","));
        trig.click();
        q("[data-dt-go=next]").click(); await tick();
        log("next", status());
        vis()[0].querySelector("[data-dt-select]").click(); await tick();
        log("picked", S().picked.length, "bulk", q("[data-dt-bulk]").hidden ? "hidden" : "shown", q("[data-dt-count]").textContent);
        q("[data-dt-select-page]").click(); await tick();
        log("page", S().picked.length, q("[data-dt-select-all]").hidden ? "" : q("[data-dt-select-all]").textContent);
        q("[data-dt-select-all]").click(); await tick();
        log("all", S().picked.length, status());
        q("[data-dt-clear-selection]").click(); await tick();
        q("[data-dt-reset]").click(); await tick();
        log("reset", status(), S().picked.length);
        const size = q("[data-dt-size]"); size.value = "20"; fire(size, "change"); await tick();
        log("size", vis().length, status());
        q("[data-dt-show=assignee]").click(); await tick();
        log("hidden column", q("th[data-dt-col=assignee]").hidden, vis()[0].querySelector("[data-dt-col=assignee]").hidden);
        vis()[0].focus(); vis()[0].dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true })); await tick();
        log("down", document.activeElement === vis()[1], vis()[1].tabIndex);
        document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: " ", bubbles: true })); await tick();
        log("space", S().picked.length, vis()[1].getAttribute("aria-selected"));
        log("formats", /^[A-Z][a-z]{2} \d{1,2}, 2026$/.test(col("due")[0]), q("[data-dt-sum]").textContent);
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let html = engine.render(&ir, &data).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("data-table", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "start 10 0 of 64 row(s) selected. | Page 1 of 7 reset hidden",
            // Sorted by `priorityRank`, highest first (the icon's name is its text here).
            "priority arrow-upHigh",
            "title ascending true",
            "title off false",
            "search 4 0 of 4 (of 64) row(s) selected. | Page 1 of 1 reset shown",
            // Choices in the given order, each with its count.
            "status filter true Backlog8 Todo23 In Progress15 Done15 Canceled3",
            "done 0 of 15 (of 64) row(s) selected. | Page 1 of 2 Done Done",
            "next 0 of 15 (of 64) row(s) selected. | Page 2 of 2",
            "picked 1 bulk shown 1 selected",
            "page 5 Select all 15",
            "all 15 15 of 15 (of 64) row(s) selected. | Page 2 of 2",
            "reset 0 of 64 row(s) selected. | Page 1 of 7 0",
            "size 20 0 of 64 row(s) selected. | Page 1 of 4",
            "hidden column true true",
            "down true 0",
            "space 1 true",
            "formats true 350",
        ],
        "\n{result}"
    );
}

/// Rows from state: the runtime rebuilds them and the table arranges them again,
/// keeping its sort; `filter` from state, `bindView` and `bindRow`.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn data_table_rows_from_state_in_chrome() {
    let rows = |n: i64| (1..=n).map(|i| serde_json::json!({ "id": i, "name": format!("Item {i}"), "qty": (i * 37) % 11, "kind": if i % 2 == 0 { "even" } else { "odd" } })).collect::<Vec<_>>();
    let doc = serde_json::json!({ "version": 1, "state": { "rows": rows(6), "kind": null, "view": [], "row": null },
        "root": { "type": "ui-data-table", "props": { "rows": "{{ rows }}", "sort": "-qty", "pageSize": 4, "filter": { "kind": "{{ kind }}" },
            "bindView": "view", "bindRow": "row", "columns": [{ "field": "name" }, { "field": "qty", "summary": "sum" }, { "field": "kind" }] } } });
    let driver = format!(r##"<script>
    window.addEventListener("load", async () => {{
      const out = []; const log = (...a) => out.push(a.join(" "));
      const tick = () => new Promise((r) => setTimeout(r, 0));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const t = document.querySelector("[data-i-x=ui-data-table]");
      const vis = () => [...t.querySelectorAll("[data-dt-body] > [data-dt-row]:not([hidden])")];
      const qty = () => vis().map((r) => r.querySelector("[data-dt-col=qty]").textContent).join(",");
      try {{
        await tick();
        log("start", qty(), S().view.length, t.querySelector("[data-dt-sum]").textContent);
        root.__irender.set("kind", "even"); await tick();
        log("even", qty(), S().view.map((r) => r.id).join(","));
        root.__irender.set("rows", {more}); await tick();
        log("more rows", qty(), S().view.length, t.querySelector("[data-dt-page]").textContent);
        vis()[0].querySelector("[data-dt-col=name]").click(); await tick();
        log("row", S().row.name);
      }} catch (e) {{ log("EXCEPTION", e.stack); }}
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    }});
    </script>"##, more = serde_json::Value::from(rows(12)));
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("data-table-state", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            // qty is (id × 37) mod 11: 4, 8, 1, 5, 9, 2 for ids 1–6, sorted descending.
            "start 9,8,5,4 6 29",
            "even 8,5,2 2,4,6",
            // Ids 8, 10 and 12 add 10, 7 and 4; the sort and the filter still hold.
            "more rows 10,8,7,5 6 Page 1 of 2",
            "row Item 8",
        ],
        "\n{result}"
    );
}

/// Paging options over local rows: Load more, infinite scroll with a virtual window,
/// and a virtual table of rows from state.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn data_table_paging_options_in_chrome() {
    let rows = |n: usize| (0..n).map(|i| serde_json::json!({ "id": i, "name": format!("Row {i}"), "n": i })).collect::<Vec<_>>();
    let cols = serde_json::json!([{ "field": "name" }, { "field": "n", "format": "number" }]);
    let doc = serde_json::json!({ "version": 1, "state": { "live": rows(500) },
        "root": { "type": "stack", "children": [
            { "type": "ui-data-table", "props": { "id": "more", "rows": rows(45), "paging": "more", "pageSize": 10, "search": true, "columns": cols } },
            { "type": "ui-data-table", "props": { "id": "scroll", "rows": rows(1000), "paging": "scroll", "virtual": true, "height": 300,
                "pageSize": 100, "selectable": true, "columns": cols } },
            { "type": "ui-data-table", "props": { "id": "state", "rows": "{{ live }}", "virtual": true, "pageSize": 0, "height": 200, "sort": "-n", "columns": cols } } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]");
      const T = (id) => document.getElementById(id);
      const rows = (t) => [...t.querySelectorAll("[data-dt-body] > [data-dt-row]:not([hidden])")];
      const st = (t) => t.querySelector("[data-dt-status]").textContent;
      const names = (t) => rows(t).map((r) => r.querySelector("[data-dt-col=name]").textContent);
      // Scroll events come with animation frames, which headless Chrome may not run.
      const scroll = async (t, top) => { const s = t.querySelector("[data-dt-scroll]"); s.scrollTop = top; s.dispatchEvent(new Event("scroll")); await wait(40); };
      try {
        await wait(50);
        const m = T("more");
        log("more", rows(m).length, st(m), m.querySelector("[data-dt-load]").hidden);
        m.querySelector("[data-dt-load]").click(); await wait(10);
        log("load", rows(m).length, st(m));
        for (let i = 0; i < 3; i++) m.querySelector("[data-dt-load]").click();
        await wait(10);
        log("all", rows(m).length, st(m), m.querySelector("[data-dt-more]").hidden);
        const s = m.querySelector("[data-dt-search]"); s.value = "Row 1"; s.dispatchEvent(new Event("input", { bubbles: true })); await wait(10);
        log("search", rows(m).length, st(m));

        const sc = T("scroll");
        log("scroll", rows(sc).length < 30, st(sc), names(sc)[0]);
        await scroll(sc, 1e6); await wait(50);
        log("bottom", st(sc), rows(sc).length < 30);
        await scroll(sc, 1e6); await wait(50);
        log("again", st(sc));
        const mid = sc.querySelector("[data-dt-scroll]");
        await scroll(sc, mid.scrollHeight / 2);
        const at = Number(names(sc)[0].slice(4));
        log("middle", at > 60 && at < 200, rows(sc).length < 30);
        rows(sc)[0].focus(); rows(sc)[0].dispatchEvent(new KeyboardEvent("keydown", { key: "Home", bubbles: true })); await wait(10);
        log("home", document.activeElement.querySelector("[data-dt-col=name]").textContent);
        document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: "End", bubbles: true })); await wait(10);
        log("end", document.activeElement.querySelector("[data-dt-col=name]").textContent);
        sc.querySelector("[data-dt-select-page]").click(); await wait(10);
        // End reached the last loaded row, which loads the next batch: all are selected.
        const [shown, picked] = st(sc).match(/Showing (\d+) .* (\d+) selected/).slice(1);
        log("select loaded", shown === picked, Number(shown) >= 300);

        const lv = T("state");
        log("state", rows(lv).length < 30, names(lv)[0], lv.querySelector("[data-dt-pad=bottom] td").style.height !== "0px");
        root.__irender.set("live", root.__irender.state.live.slice(0, 3)); await wait(20);
        log("fewer", names(lv).join(","), lv.querySelector("[data-dt-pad=bottom] td").style.height);
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("data-table-paging", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "more 10 Showing 10 of 45 false",
            "load 20 Showing 20 of 45",
            "all 45 Showing 45 of 45 true",
            // A new search starts again from the first batch: "row" and "1" match
            // rows 1, 10–19, 21, 31 and 41.
            "search 10 Showing 10 of 14 (of 45)",
            "scroll true Showing 100 of 1,000 · 0 selected Row 0",
            "bottom Showing 200 of 1,000 · 0 selected true",
            "again Showing 300 of 1,000 · 0 selected",
            "middle true true",
            "home Row 0",
            "end Row 299",
            "select loaded true true",
            "state true Row 499 true",
            "fewer Row 2,Row 1,Row 0 0px",
        ],
        "\n{result}"
    );
}

/// Rows from `url`: batches as the table scrolls, the query sent to the server, a
/// new sort starting over, server pages with the total, and a failed load retried.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn data_table_remote_rows_in_chrome() {
    use std::io::{Read, Write};
    use std::sync::{Arc, Mutex};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let log = Arc::new(Mutex::new(Vec::<String>::new()));
    let seen = log.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut buf = [0u8; 4096];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]).to_string();
            let target = request.split_whitespace().nth(1).unwrap_or("").to_string();
            let query: std::collections::HashMap<String, String> = target
                .split_once('?')
                .map(|(_, q)| q.split('&').filter_map(|kv| kv.split_once('=')).map(|(k, v)| (k.to_string(), v.replace("%2C", ",").replace('+', " "))).collect())
                .unwrap_or_default();
            seen.lock().unwrap().push(target.split_once('?').map_or(String::new(), |(_, q)| q.to_string()));
            let num = |k: &str, d: usize| query.get(k).and_then(|v| v.parse().ok()).unwrap_or(d);
            let (offset, limit) = (num("offset", 0), num("limit", 1000));
            let desc = query.get("sort").is_some_and(|s| s == "-n");
            let (status, body) = if query.get("fail").is_some_and(|f| f == "1") {
                ("500 Internal Server Error", "{}".to_string())
            } else {
                let mut all: Vec<usize> = (0..65).collect();
                if desc {
                    all.reverse();
                }
                let rows: Vec<_> = all.iter().skip(offset).take(limit).map(|i| serde_json::json!({ "id": i, "n": i })).collect();
                ("200 OK", serde_json::json!({ "rows": rows, "total": 65 }).to_string())
            };
            let _ = write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    let url = format!("http://127.0.0.1:{port}/rows");
    let cols = serde_json::json!([{ "field": "n", "label": "N" }]);
    let doc = serde_json::json!({ "version": 1, "state": { "fail": null },
        "root": { "type": "stack", "children": [
            { "type": "ui-data-table", "props": { "id": "scroll", "url": url, "paging": "scroll", "pageSize": 20, "height": 240,
                "params": { "fail": "{{ fail }}" }, "columns": cols } },
            { "type": "ui-data-table", "props": { "id": "pages", "url": url, "paging": "pages", "pageSize": 30, "pageSizes": [], "columns": cols } } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const until = async (f) => { for (let i = 0; i < 100 && !f(); i++) await wait(20); };
      const root = document.querySelector("[data-i-root]");
      const T = (id) => document.getElementById(id);
      const rows = (t) => [...t.querySelectorAll("[data-dt-body] > [data-dt-row]")];
      const st = (t) => t.querySelector("[data-dt-status]").textContent;
      const idle = (t) => until(() => !t.hasAttribute("data-loading"));
      const scroll = async (t) => { const s = t.querySelector("[data-dt-scroll]"); s.scrollTop = 1e6; s.dispatchEvent(new Event("scroll")); await wait(30); await idle(t); };
      try {
        const sc = T("scroll"), pg = T("pages");
        await idle(sc); await idle(pg);
        log("first", rows(sc).length, st(sc));
        await scroll(sc); log("second", rows(sc).length, st(sc));
        await scroll(sc); await scroll(sc);
        log("all", rows(sc).length, st(sc), sc.querySelector("[data-dt-more]").hidden);
        sc.querySelector("[data-dt-sort=n]").click(); await wait(30); await idle(sc);
        sc.querySelector("[data-dt-sort=n]").click(); await wait(30); await idle(sc);
        log("desc", rows(sc).length, rows(sc)[0].textContent, st(sc), sc.querySelector("[data-dt-scroll]").scrollTop);
        root.__irender.set("fail", 1); await wait(300); await idle(sc);
        log("failed", rows(sc).length, sc.querySelector("[data-dt-retry]").hidden, sc.querySelector("[data-dt-empty]").textContent);
        root.__irender.set("fail", null); await wait(300); await idle(sc);
        log("recovered", rows(sc).length, st(sc));

        log("pages", rows(pg).length, st(pg), pg.querySelector("[data-dt-page]").textContent);
        pg.querySelector("[data-dt-go=last]").click(); await wait(30); await idle(pg);
        log("last", rows(pg).length, rows(pg)[0].textContent, st(pg), pg.querySelector("[data-dt-go=next]").disabled);
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("data-table-remote", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "first 20 Showing 20 of 65",
            "second 40 Showing 40 of 65",
            "all 65 Showing 65 of 65 true",
            // Ascending, then descending: each starts over from the first batch, at the top.
            "desc 20 64 Showing 20 of 65 0",
            "failed 0 false Couldn't load rows.",
            "recovered 20 Showing 20 of 65",
            "pages 30 Rows 1–30 of 65 Page 1 of 3",
            "last 5 60 Rows 61–65 of 65 true",
        ],
        "\n{result}"
    );
    let log = log.lock().unwrap();
    assert!(log.iter().any(|q| q == "offset=20&limit=20"), "{log:?}");
    assert!(log.iter().any(|q| q.contains("sort=-n")), "{log:?}");
}

/// Nested rows (children arrays, children loaded on demand, and flat rows with a
/// parent field) and pinned columns.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn data_table_tree_and_pinned_columns_in_chrome() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut buf = [0u8; 4096];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]).to_string();
            let parent = request.split("parent=").nth(1).and_then(|p| p.split(['&', ' ']).next()).unwrap_or("").to_string();
            let rows: Vec<_> = (1..=3).map(|i| serde_json::json!({ "id": format!("{parent}.{i}"), "name": format!("Loaded {i}"), "v": i })).collect();
            let body = serde_json::to_string(&rows).unwrap();
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    let wide: Vec<_> = (0..10).map(|i| serde_json::json!({ "field": format!("m{i}"), "label": format!("Month {i}"), "width": 120 })).collect();
    let mut columns = vec![serde_json::json!({ "field": "name", "label": "Name", "pin": "left", "width": 200 })];
    columns.extend(wide);
    columns.push(serde_json::json!({ "field": "v", "label": "Total", "pin": "right", "summary": "sum" }));
    let nested = serde_json::json!([
        { "id": "a", "name": "Alpha", "v": 10, "children": [
            { "id": "a1", "name": "Alpha one", "v": 4 },
            { "id": "a2", "name": "Alpha two", "v": 6, "children": [{ "id": "a2x", "name": "Deep", "v": 6 }] } ] },
        { "id": "b", "name": "Beta", "v": 5, "children": true },
        { "id": "c", "name": "Gamma", "v": 1 }
    ]);
    let flat = serde_json::json!([
        { "id": 1, "name": "CEO" }, { "id": 2, "name": "CTO", "boss": 1 }, { "id": 3, "name": "Engineer", "boss": 2 },
        { "id": 4, "name": "CFO", "boss": 1 }, { "id": 5, "name": "Accountant", "boss": 4 }
    ]);
    let doc = serde_json::json!({ "version": 1,
        "root": { "type": "stack", "style": { "width": 700 }, "children": [
            { "type": "ui-data-table", "props": { "id": "tree", "rows": nested, "selectable": true, "pageSize": 0,
                "childrenUrl": format!("http://127.0.0.1:{port}/kids"), "columns": columns } },
            { "type": "ui-data-table", "props": { "id": "flat", "rows": flat, "parentField": "boss", "expanded": true, "search": true, "pageSize": 0,
                "columns": [{ "field": "name" }] } } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const T = (id) => document.getElementById(id);
      const rows = (t) => [...t.querySelectorAll("[data-dt-body] > [data-dt-row]")];
      const view = (t) => rows(t).map((r) => "  ".repeat(Number(r.getAttribute("aria-level")) - 1) + r.querySelector("[data-dt-col=name]").textContent.trim() + (r.getAttribute("aria-expanded") === "true" ? "-" : r.hasAttribute("aria-expanded") ? "+" : "")).join(" | ");
      const key = (el, k) => el.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true }));
      try {
        await wait(50);
        const t = T("tree");
        log("roles", t.querySelector("table").getAttribute("role"));
        log("start", view(t));
        rows(t)[0].querySelector("[data-dt-twisty]").click(); await wait(10);
        log("open alpha", view(t));
        const two = rows(t).find((r) => r.textContent.includes("Alpha two"));
        two.focus(); key(two, "ArrowRight"); await wait(10);
        log("right", view(t), document.activeElement.textContent.includes("Alpha two"));
        key(document.activeElement, "ArrowRight"); await wait(10);
        log("into", document.activeElement.querySelector("[data-dt-col=name]").textContent.trim());
        key(document.activeElement, "ArrowLeft"); await wait(10);
        log("to parent", document.activeElement.querySelector("[data-dt-col=name]").textContent.trim());
        t.querySelector("[data-dt-sort=name]").click(); t.querySelector("[data-dt-sort=name]").click(); await wait(10);
        log("desc", view(t));
        rows(t).find((r) => r.textContent.includes("Beta")).querySelector("[data-dt-twisty]").click();
        await wait(300);
        log("lazy", view(t));
        log("sum", t.querySelector("[data-dt-sum]").textContent);
        // Pinned columns: scroll sideways; Name stays at the left edge, Total at the right.
        const sc = t.querySelector("[data-dt-scroll]");
        const name = () => t.querySelector("[data-dt-body] [data-dt-col=name]").getBoundingClientRect();
        const total = () => t.querySelector("[data-dt-body] [data-dt-col=v]").getBoundingClientRect();
        const box = t.querySelector("[data-dt-body] [data-dt-box]").getBoundingClientRect();
        const before = [name().left, total().right];
        sc.scrollLeft = 400; await wait(30);
        const edge = sc.getBoundingClientRect();
        log("pinned", sc.scrollWidth > sc.clientWidth, name().left === before[0], Math.round(name().left - box.right) === 0, Math.abs(total().right - (edge.left + sc.clientWidth + 1)) <= 2,
          t.querySelector("[data-dt-body] [data-dt-col=name]").getAttribute("data-pin-edge"), t.querySelector("[data-dt-body] [data-dt-col=v]").getAttribute("data-pin-edge"));
        log("unpinned moved", t.querySelector("[data-dt-body] [data-dt-col=m0]").getBoundingClientRect().left < before[0]);

        const f = T("flat");
        log("flat", view(f));
        const s = f.querySelector("[data-dt-search]"); s.value = "account"; s.dispatchEvent(new Event("input", { bubbles: true })); await wait(10);
        log("search", view(f));
        s.value = ""; s.dispatchEvent(new Event("input", { bubbles: true })); await wait(10);
        rows(f)[0].querySelector("[data-dt-twisty]").click(); await wait(10);
        log("closed", view(f));
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("data-table-tree", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "roles treegrid",
            "start Alpha+ | Beta+ | Gamma",
            "open alpha Alpha- |   Alpha one |   Alpha two+ | Beta+ | Gamma",
            "right Alpha- |   Alpha one |   Alpha two- |     Deep | Beta+ | Gamma true",
            "into Deep",
            "to parent Alpha two",
            // Siblings sort among themselves, under their parents.
            "desc Gamma | Beta+ | Alpha- |   Alpha two- |     Deep |   Alpha one",
            "lazy Gamma | Beta- |   Loaded 3 |   Loaded 2 |   Loaded 1 | Alpha- |   Alpha two- |     Deep |   Alpha one",
            // Only the top rows are added up.
            "sum 16",
            "pinned true true true true left right",
            "unpinned moved true",
            "flat CEO- |   CTO- |     Engineer |   CFO- |     Accountant",
            // A match shows with the rows above it.
            "search CEO- |   CFO- |     Accountant",
            "closed CEO+",
        ],
        "\n{result}"
    );
}

/// Date pickers and the calendar: the keyboard, limits, ranges and presets, a time,
/// months, financial quarters and years, several days, month and year menus, and the
/// field label and form inputs.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn date_picker_in_chrome() {
    let doc = serde_json::json!({ "version": 1,
        "state": { "d": "2026-03-10", "r": { "from": "2026-03-01", "to": "2026-03-02" }, "rr": null, "q": "2026-Q1", "qr": null,
                   "t": "2026-03-10T09:00", "m": "2026-03", "y": "2026", "days": [], "dob": null },
        "root": { "type": "stack", "children": [
            { "type": "ui-field", "props": { "label": "Due" }, "children": [
                { "type": "ui-date-picker", "props": { "bind": "d", "min": "2026-03-05", "max": "2026-04-20", "disabledDays": [0], "weekStart": 1, "locale": "en-US", "name": "due" } } ] },
            { "type": "ui-date-picker", "props": { "id": "range", "bind": "r", "bindRange": "rr", "mode": "range", "months": 2, "locale": "en-US",
                "presets": [{ "label": "Q1", "from": "2026-01-01", "to": "2026-03-31" }, "last7"] } },
            { "type": "ui-date-picker", "props": { "id": "quarter", "bind": "q", "bindRange": "qr", "picker": "quarter", "fiscalStart": 4, "locale": "en-US" } },
            { "type": "ui-date-picker", "props": { "id": "time", "bind": "t", "time": true, "step": 30, "locale": "en-US" } },
            { "type": "ui-date-picker", "props": { "id": "month", "bind": "m", "picker": "month", "locale": "en-US" } },
            { "type": "ui-date-picker", "props": { "id": "year", "bind": "y", "picker": "year", "fiscalStart": 4 } },
            { "type": "ui-calendar", "props": { "id": "cal", "bind": "days", "mode": "multiple", "value": null, "locale": "en-US" } },
            { "type": "ui-date-picker", "props": { "id": "dob", "bind": "dob", "dropdowns": true, "locale": "en-US" } } ] } });
    let driver = r##"<style>* { transition: none !important; }</style><script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const P = (id) => document.getElementById(id).closest("[data-i-x]");
      const text = (p) => p.querySelector("[data-cal-text]").textContent;
      const open = (p) => p.querySelector("[data-cal-pop]").matches(":popover-open");
      const act = () => document.activeElement.getAttribute("data-key");
      const key = async (k, mods = {}) => { document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true, ...mods })); await wait(5); };
      const cell = (p, k) => p.querySelector(`[data-key="${k}"]`);
      try {
        const due = document.querySelector("[data-cal-trigger]").closest("[data-i-x]");
        const label = document.querySelector(`label[for="${due.querySelector("[data-cal-trigger]").id}"]`);
        log("label", label?.textContent, text(due), due.querySelector("input[type=hidden]").value);
        due.querySelector("[data-cal-trigger]").click(); await wait(10);
        log("open", open(due), act(), due.querySelector("[data-cal-caption]").textContent, due.querySelector("th").textContent);
        await key("ArrowRight"); await key("PageDown");
        log("moved", act(), due.querySelector("[data-cal-caption]").textContent);
        await key("Enter");
        log("picked", S().d, open(due), document.activeElement === due.querySelector("[data-cal-trigger]"), text(due), due.querySelector("input[type=hidden]").value);
        due.querySelector("[data-cal-trigger]").click(); await wait(10);
        await key("Home"); log("home", act());
        await key("End"); log("end", act(), cell(due, act()).getAttribute("aria-disabled"));
        await key("Enter"); log("disabled day", S().d, open(due));
        await key("PageDown");
        log("past max", act(), cell(due, act()).getAttribute("aria-disabled"), due.querySelector("[data-cal-step='1']").disabled);
        due.querySelector("[data-cal-step='-1']").click(); due.querySelector("[data-cal-step='-1']").click(); await wait(5);
        log("min", due.querySelector("[data-cal-caption]").textContent, due.querySelector("[data-cal-step='-1']").disabled, cell(due, "2026-03-04").getAttribute("aria-disabled"));
        due.querySelector("[data-cal-pop]").hidePopover();

        const range = P("range");
        range.querySelector("[data-cal-trigger]").click(); await wait(10);
        log("range open", [...range.querySelectorAll("[data-cal-caption]")].map((c) => c.textContent).join(" / "), text(range));
        cell(range, "2026-03-03").click(); await wait(5);
        log("start", JSON.stringify(S().r), text(range), open(range));
        cell(range, "2026-04-02").dispatchEvent(new MouseEvent("mouseover", { bubbles: true })); await wait(5);
        log("preview", range.querySelectorAll("[data-in-range]").length);
        cell(range, "2026-04-02").click(); await wait(5);
        log("range", JSON.stringify(S().r), JSON.stringify(S().rr), text(range), open(range));
        range.querySelector("[data-cal-trigger]").click(); await wait(10);
        range.querySelector("[data-preset='0']").click(); await wait(5);
        log("preset", JSON.stringify(S().rr), open(range));
        range.querySelector("[data-cal-trigger]").click(); await wait(10);
        log("preset pressed", range.querySelector("[data-preset='0']").getAttribute("aria-pressed"));
        range.querySelector("[data-preset='1']").click(); await wait(5);
        log("last7", (Date.parse(S().rr.to) - Date.parse(S().rr.from)) / 864e5 + 1);

        const q = P("quarter");
        log("quarter", text(q));
        q.querySelector("[data-cal-trigger]").click(); await wait(10);
        log("quarters", [...q.querySelectorAll("[data-key]")].map((b) => b.textContent).join(","), q.querySelector("[data-cal-caption]").textContent, act());
        await key("ArrowRight"); await key("Enter");
        log("q2", S().q, JSON.stringify(S().qr), text(q));

        const t = P("time");
        log("time", text(t));
        t.querySelector("[data-cal-trigger]").click(); await wait(10);
        const input = t.querySelector("[data-cal-time-input]");
        input.value = "14:30"; input.dispatchEvent(new Event("change", { bubbles: true })); await wait(5);
        log("new time", S().t, input.step);
        cell(t, "2026-03-12").click(); await wait(5);
        log("new day", S().t, open(t), text(t));
        t.querySelector("[data-cal-pop]").hidePopover();

        const m = P("month");
        m.querySelector("[data-cal-trigger]").click(); await wait(10);
        log("months", m.querySelectorAll("[data-key]").length, act(), document.activeElement.getAttribute("aria-label"));
        await key("ArrowDown"); await key("Enter");
        log("june", S().m, text(m));

        const y = P("year");
        log("year", text(y));
        y.querySelector("[data-cal-trigger]").click(); await wait(10);
        await key("ArrowRight"); await key("Enter");
        log("next year", S().y, text(y));

        const cal = P("cal");
        cal.querySelector("[data-cal-step='1']").click(); await wait(5);
        // Each pick redraws the grid, so find each day again by its key.
        const ks = [...cal.querySelectorAll("[data-key]")].filter((b) => !b.hasAttribute("data-outside")).slice(0, 3).map((b) => b.getAttribute("data-key"));
        for (const k of ks) { cell(cal, k).click(); await wait(5); }
        const n = S().days.length;
        cal.querySelector(`[data-key="${S().days[0]}"]`).click(); await wait(5);
        log("several", n, S().days.length);

        const dob = P("dob");
        dob.querySelector("[data-cal-trigger]").click(); await wait(10);
        const yr = dob.querySelector("[data-cal-year]"), mo = dob.querySelector("[data-cal-month]");
        yr.value = "1990"; yr.dispatchEvent(new Event("change", { bubbles: true })); await wait(5);
        dob.querySelector("[data-cal-month]").value = "5"; dob.querySelector("[data-cal-month]").dispatchEvent(new Event("change", { bubbles: true })); await wait(5);
        cell(dob, "1990-05-15").click(); await wait(5);
        log("dob", S().dob, text(dob));
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("date-picker", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "label Due Mar 10, 2026 2026-03-10",
            "open true 2026-03-10 March 2026 Mo",
            "moved 2026-04-11 April 2026",
            "picked 2026-04-11 false true Apr 11, 2026 2026-04-11",
            "home 2026-04-06",
            "end 2026-04-12 true",
            "disabled day 2026-04-11 true",
            "past max 2026-05-12 true true",
            "min March 2026 true true",
            "range open March 2026 / April 2026 Mar 1, 2026 – Mar 2, 2026",
            "start {\"from\":\"2026-03-03\",\"to\":null} Mar 3, 2026 – … true",
            "preview 29",
            "range {\"from\":\"2026-03-03\",\"to\":\"2026-04-02\"} {\"from\":\"2026-03-03\",\"to\":\"2026-04-02\"} Mar 3, 2026 – Apr 2, 2026 false",
            "preset {\"from\":\"2026-01-01\",\"to\":\"2026-03-31\"} false",
            "preset pressed true",
            "last7 7",
            "quarter Q1 FY2026–27",
            "quarters Q1,Q2,Q3,Q4 FY2026–27 2026-Q1",
            "q2 2026-Q2 {\"from\":\"2026-07-01\",\"to\":\"2026-09-30\"} Q2 FY2026–27",
            "time Mar 10, 2026, 9:00 AM",
            "new time 2026-03-10T14:30 1800",
            "new day 2026-03-12T14:30 true Mar 12, 2026, 2:30 PM",
            "months 12 2026-03 March 2026",
            "june 2026-06 Jun 2026",
            "year FY2026–27",
            "next year 2027 FY2027–28",
            "several 3 2",
            "dob 1990-05-15 May 15, 1990",
        ],
        "\n{result}"
    );
}

/// A data table's date filter is a range calendar: a finished range filters the rows.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn data_table_date_filter_in_chrome() {
    let rows: Vec<_> = (1..=31).map(|d| serde_json::json!({ "id": d, "day": format!("2026-03-{d:02}") })).collect();
    let doc = serde_json::json!({ "version": 1, "state": { "f": null },
        "root": { "type": "ui-data-table", "props": { "rows": rows, "pageSize": 0, "locale": "en-US", "bindFilters": "f",
            "columns": [{ "field": "day", "format": "date", "filter": "date" }] } } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const t = document.querySelector("[data-i-x=ui-data-table]");
      const shown = () => t.querySelectorAll("[data-dt-body] > [data-dt-row]:not([hidden])").length;
      try {
        t.querySelector("[data-dt-trigger=day]").click(); await wait(10);
        const cal = t.querySelector("[data-dt-calendar]");
        log("calendar", !!cal, cal.querySelectorAll("[data-preset]").length);
        // Show March 2026 the way the filter would, then pick a range in it.
        root.__irender.set("f", { day: { from: "2026-03-01", to: "2026-03-31" } }); await wait(5);
        log("from state", shown(), cal.querySelector("[data-cal-caption]").textContent);
        cal.querySelector('[data-key="2026-03-05"]').click(); await wait(5);
        log("half", shown(), JSON.stringify(S().f));
        cal.querySelector('[data-key="2026-03-20"]').click(); await wait(5);
        log("range", shown(), JSON.stringify(S().f), t.querySelector("[data-dt-trigger=day] [data-dt-chips]").textContent);
        t.querySelector("[data-dt-filter=day] [data-dt-clear]").click(); await wait(5);
        log("cleared", shown(), JSON.stringify(S().f), cal.querySelectorAll("[aria-pressed=true][data-key]").length);
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("data-table-date", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "calendar true 6",
            "from state 31 March 2026",
            // Half a range doesn't filter yet.
            "half 31 {\"day\":{\"from\":\"2026-03-01\",\"to\":\"2026-03-31\"}}",
            "range 16 {\"day\":{\"from\":\"2026-03-05\",\"to\":\"2026-03-20\"}} Mar 5, 2026 – Mar 20, 2026",
            "cleared 31 {} 0",
        ],
        "\n{result}"
    );
}

/// Context menus: opening at the pointer (kept on screen) and from the keyboard, what was
/// right-clicked (a `data-context`, a data table's row), the menu's keys, focus going
/// back, and turning it off.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn context_menu_in_chrome() {
    let item = |label: &str, extra: serde_json::Value| {
        let mut props = serde_json::json!({ "label": label });
        props.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        serde_json::json!({ "type": "ui-menu-item", "props": props,
            "on": { "click": { "action": "set_state", "key": "ran", "value": format!("{label} {{{{ ctx }}}}") } } })
    };
    let doc = serde_json::json!({ "version": 1, "state": { "ctx": null, "ran": "", "row": null, "off": false },
        "root": { "type": "stack", "children": [
            { "type": "ui-context-menu", "props": { "bind": "ctx", "disabled": "{{ off }}" }, "children": [
                { "type": "element", "attrs": { "id": "area", "tabindex": 0 }, "style": { "height": 120 }, "children": [
                    { "type": "element", "attrs": { "id": "file", "tabindex": 0, "data-context": "{\"name\":\"a.txt\"}" }, "children": [ { "type": "text", "props": { "value": "a.txt" } } ] } ] },
                item("Open", serde_json::json!({})),
                item("Locked", serde_json::json!({ "disabled": true })),
                { "type": "ui-menu-separator" },
                item("Delete", serde_json::json!({ "variant": "destructive" })) ] },
            { "type": "ui-context-menu", "props": { "bind": "row" }, "children": [
                { "type": "ui-data-table", "props": { "rows": [{ "id": 1, "name": "One" }, { "id": 2, "name": "Two" }], "pageSize": 0, "columns": [{ "field": "name" }] } },
                { "type": "ui-menu-label", "props": { "text": "{{ row.name }}" } },
                { "type": "ui-menu-item", "props": { "label": "Rename {{ row.name }}" } } ] } ] } });
    let driver = r##"<style>* { transition: none !important; }</style><script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const [one, two] = document.querySelectorAll("[data-i-x=ui-context-menu]");
      const menu = (m) => m.querySelector(":scope > [role=menu]");
      const isOpen = (m) => menu(m).matches(":popover-open");
      const right = (el, x, y) => el.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: x, clientY: y }));
      const key = (k, mods = {}) => document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true, ...mods }));
      const name = () => document.activeElement.textContent;
      try {
        const area = document.getElementById("area");
        area.focus();
        const prevented = !right(area, 40, 30); await wait(10);
        const r = menu(one).getBoundingClientRect();
        log("open", prevented, isOpen(one), Math.round(r.left), Math.round(r.top), JSON.stringify(S().ctx), menu(one).getAttribute("role"), name());
        key("ArrowDown"); await wait(5); log("down", name());
        key("ArrowDown"); await wait(5); log("skips disabled", name());
        document.activeElement.click(); await wait(20);
        log("chose", S().ran, isOpen(one), document.activeElement === area);

        right(document.getElementById("file"), 40, 30); await wait(10);
        log("context", JSON.stringify(S().ctx));
        menu(one).querySelector("[role=menuitem]").click(); await wait(20);
        log("ran", S().ran);

        // Near the window's corner it opens up and to the left, staying on screen.
        right(area, innerWidth - 5, innerHeight - 5); await wait(10);
        const c = menu(one).getBoundingClientRect();
        log("on screen", c.right <= innerWidth, c.bottom <= innerHeight);
        menu(one).hidePopover(); await wait(20);

        document.getElementById("file").focus();
        key("F10", { shiftKey: true }); await wait(10);
        log("shift+f10", isOpen(one), JSON.stringify(S().ctx), name());
        menu(one).hidePopover(); await wait(20);
        log("focus back", document.activeElement.id);

        root.__irender.set("off", true); await wait(5);
        log("disabled", right(area, 40, 30), isOpen(one));
        root.__irender.set("off", false);

        const rowEl = two.querySelectorAll("[data-dt-row]")[1];
        right(rowEl.querySelector("td"), 50, 50); await wait(10);
        log("row", JSON.stringify(S().row), menu(two).querySelector("[role=menuitem]").textContent, menu(two).firstElementChild.textContent);
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("context-menu", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "open true true 40 30 null menu Open",
            "down Delete",
            "skips disabled Open",
            "chose Open  false true",
            "context {\"name\":\"a.txt\"}",
            "ran Open {\"name\":\"a.txt\"}",
            "on screen true true",
            "shift+f10 true {\"name\":\"a.txt\"} Open",
            "focus back file",
            // The browser's own menu: the event isn't cancelled.
            "disabled true false",
            "row {\"id\":2,\"name\":\"Two\"} Rename Two Two",
        ],
        "\n{result}"
    );
}

/// Typing into a date picker: the locale's order, month names, two-digit years, ranges,
/// times and quarters; errors for text that isn't a date or is outside the limits;
/// Escape, clearing, and the calendar from the keyboard.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn date_picker_typing_in_chrome() {
    let doc = serde_json::json!({ "version": 1,
        "state": { "a": "2026-03-10", "e": null, "b": null, "r": null, "t": null, "q": null },
        "root": { "type": "stack", "children": [
            { "type": "ui-field", "props": { "label": "Due", "error": "{{ e }}" }, "children": [
                { "type": "ui-date-picker", "props": { "bind": "a", "bindError": "e", "input": true, "locale": "en-GB", "min": "2026-03-05", "max": "2026-12-31", "disabledDays": [0] } } ] },
            { "type": "ui-date-picker", "props": { "id": "b", "bind": "b", "input": true, "locale": "en-US" } },
            { "type": "ui-date-picker", "props": { "id": "r", "bind": "r", "mode": "range", "input": true, "locale": "en-GB" } },
            { "type": "ui-date-picker", "props": { "id": "t", "bind": "t", "time": true, "input": true, "locale": "en-GB" } },
            { "type": "ui-date-picker", "props": { "id": "q", "bind": "q", "picker": "quarter", "fiscalStart": 4, "input": true, "locale": "en-GB" } } ] } });
    let driver = r##"<style>* { transition: none !important; }</style><script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const fields = [...document.querySelectorAll("[data-cal-input]")];
      const [fa, fb, fr, ft, fq] = fields;
      const type = async (f, text, how = "Enter") => {
        f.focus(); f.value = text; f.dispatchEvent(new Event("input", { bubbles: true }));
        if (how === "Enter") f.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
        else f.blur();
        await wait(5);
      };
      const err = () => `${S().e} ${fa.getAttribute("aria-invalid")}`;
      try {
        const label = document.querySelector(`label[for="${fa.id}"]`);
        log("start", label?.textContent, fa.value, fa.placeholder, fb.placeholder, fr.placeholder);
        await type(fa, "16/3/2026"); log("numbers", S().a, fa.value);
        await type(fa, "1 Apr 2026", "blur"); log("month name", S().a, fa.value);
        await type(fa, "20 march 26"); log("short year", S().a);
        await type(fa, "31/02/2026"); log("not a date", S().a, err(), fa.value);
        // Left alone while other state changes, the text waits to be fixed.
        fa.blur(); root.__irender.set("q", null); await wait(5);
        log("kept", fa.value);
        fa.focus(); fa.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true })); await wait(5);
        log("escape", fa.value, err());
        await type(fa, "1/3/2026"); log("too early", S().a, err());
        await type(fa, "2026-03-08"); log("sunday", err());
        await type(fa, "2026-03-09"); log("iso", S().a, err());
        await type(fa, ""); log("cleared", JSON.stringify(S().a), fa.value);
        await type(fb, "3/15/26"); log("us", S().b, fb.value);
        await type(fr, "5/12/2026 to 1/12/2026"); log("range", JSON.stringify(S().r), fr.value);
        await type(fr, "1 Dec 2026 – 3 Dec 2026"); log("range names", JSON.stringify(S().r));
        await type(fr, "1/12/2026"); log("half range", fr.getAttribute("aria-invalid"), fr.title);
        await type(ft, "15/03/2026 2:30 pm"); log("time", S().t, ft.value);
        await type(ft, "15/03/2026 25:00"); log("bad time", ft.title);
        await type(fq, "Q2 FY2026-27"); log("quarter", S().q, fq.value);
        await type(fq, "q3 2026"); log("quarter 2", S().q);
        fa.focus(); fa.value = "2026-06-10"; fa.dispatchEvent(new Event("input", { bubbles: true }));
        fa.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true, cancelable: true })); await wait(10);
        const pop = fa.closest("[data-i-x]").querySelector("[data-cal-pop]");
        log("calendar", S().a, pop.matches(":popover-open"), document.activeElement.getAttribute("data-key"));
        document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true, cancelable: true })); await wait(5);
        document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true })); await wait(5);
        log("picked", S().a, fa.value, document.activeElement === fa);
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("date-typing", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "start Due 10/03/2026 DD/MM/YYYY MM/DD/YYYY DD/MM/YYYY – DD/MM/YYYY",
            "numbers 2026-03-16 16/03/2026",
            "month name 2026-04-01 01/04/2026",
            "short year 2026-03-20",
            "not a date 2026-03-20 Enter a date like DD/MM/YYYY true 31/02/2026",
            "kept 31/02/2026",
            "escape 20/03/2026 null null",
            "too early 2026-03-20 Pick a date on or after 5 Mar 2026 true",
            "sunday 8 Mar 2026 can't be picked true",
            "iso 2026-03-09 null null",
            "cleared null ",
            "us 2026-03-15 03/15/2026",
            "range {\"from\":\"2026-12-01\",\"to\":\"2026-12-05\"} 01/12/2026 – 05/12/2026",
            "range names {\"from\":\"2026-12-01\",\"to\":\"2026-12-03\"}",
            "half range true Enter two dates, like DD/MM/YYYY – DD/MM/YYYY",
            "time 2026-03-15T14:30 15/03/2026 14:30",
            "bad time Enter a time like 14:30",
            "quarter 2026-Q2 Q2 FY2026–27",
            "quarter 2 2026-Q3",
            "calendar 2026-06-10 true 2026-06-10",
            "picked 2026-06-11 11/06/2026 true",
        ],
        "\n{result}"
    );
}

/// The menubar, submenus, checkbox and radio items, and the navigation menu.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn menubar_and_navigation_menu_in_chrome() {
    let item = |label: &str| serde_json::json!({ "type": "ui-menu-item", "props": { "label": label },
        "on": { "click": { "action": "set_state", "key": "ran", "value": label } } });
    let doc = serde_json::json!({ "version": 1, "state": { "ran": "", "bar": true, "zoom": "100" },
        "root": { "type": "stack", "children": [
            { "type": "ui-menubar", "children": [
                { "type": "ui-menubar-menu", "props": { "label": "File" }, "children": [
                    item("New"), item("Open"),
                    { "type": "ui-menu-sub", "props": { "label": "Share" }, "children": [ item("Email"), item("Link") ] },
                    { "type": "ui-menu-separator" }, item("Quit") ] },
                { "type": "ui-menubar-menu", "props": { "label": "Edit" }, "children": [ item("Undo"), item("Redo") ] },
                { "type": "ui-menubar-menu", "props": { "label": "View" }, "children": [
                    { "type": "ui-menu-checkbox", "props": { "label": "Status bar", "bind": "bar" } },
                    { "type": "ui-menu-separator" },
                    { "type": "ui-menu-radio-group", "props": { "bind": "zoom", "label": "Zoom", "options": [{ "value": "100", "label": "100%" }, { "value": "150", "label": "150%" }] } } ] } ] },
            { "type": "ui-navigation-menu", "children": [
                { "type": "ui-navigation-item", "props": { "label": "Products" }, "children": [
                    { "type": "ui-navigation-link", "props": { "label": "Analytics", "href": "#analytics", "description": "See what's happening" } },
                    { "type": "ui-navigation-link", "props": { "label": "Automation", "href": "#automation" } } ] },
                { "type": "ui-navigation-link", "props": { "label": "Docs", "href": "#docs", "active": true } } ] } ] } });
    let driver = r##"<style>* { transition: none !important; }</style><script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const key = async (k) => { document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true })); await wait(10); };
      const name = () => (document.activeElement.textContent || "").trim();
      const bar = document.querySelector("[role=menubar]");
      const [file, edit, view] = bar.querySelectorAll(":scope > [role=menuitem]");
      const menu = (t) => document.getElementById(t.getAttribute("popovertarget"));
      const isOpen = (t) => menu(t).matches(":popover-open");
      try {
        log("bar", bar.getAttribute("aria-label"), [file, edit, view].map((t) => t.tabIndex).join(","));
        file.focus(); await key("ArrowRight");
        log("right", name(), isOpen(edit), [file, edit, view].map((t) => t.tabIndex).join(","));
        await key("ArrowDown");
        log("down", isOpen(edit), name(), edit.getAttribute("aria-expanded"));
        await key("ArrowRight");
        log("menu right", isOpen(edit), isOpen(view), name(), document.activeElement.getAttribute("role"), document.activeElement.getAttribute("aria-checked"));
        document.activeElement.click(); await wait(20);
        log("checkbox", S().bar, isOpen(view), document.activeElement === view);
        await key("Enter");
        log("enter opens", isOpen(view), name());
        await key("ArrowDown"); await key("ArrowDown");
        log("radio", name(), document.activeElement.getAttribute("aria-checked"));
        document.activeElement.click(); await wait(20);
        log("picked", S().zoom);
        view.focus(); await key("ArrowRight");
        log("wraps", name());
        await key("ArrowDown"); await key("ArrowDown"); await key("ArrowDown");
        log("sub item", name(), document.activeElement.getAttribute("aria-haspopup"));
        await key("ArrowRight");
        const share = document.querySelector("[data-ui-sub-trigger]");
        const subMenu = document.getElementById(share.getAttribute("aria-controls"));
        log("sub open", subMenu.matches(":popover-open"), name(), share.getAttribute("aria-expanded"), isOpen(file));
        await key("ArrowLeft");
        log("sub closed", subMenu.matches(":popover-open"), name(), isOpen(file));
        share.dispatchEvent(new MouseEvent("mouseover", { bubbles: true })); await wait(10);
        log("hover", subMenu.matches(":popover-open"), name());
        [...subMenu.querySelectorAll("[role=menuitem]")][1].click(); await wait(20);
        log("sub chose", S().ran, isOpen(file), subMenu.matches(":popover-open"));
        file.click(); await wait(10);
        edit.dispatchEvent(new MouseEvent("mouseover", { bubbles: true })); await wait(10);
        log("hover bar", isOpen(file), isOpen(edit));
        menu(edit).hidePopover(); await wait(20);

        const nav = document.querySelector("nav[aria-label=Main]");
        const prod = nav.querySelector("[data-ui-nav-trigger]"), panel = document.getElementById(prod.getAttribute("aria-controls"));
        log("nav", prod.getAttribute("aria-expanded"), nav.querySelector("a[aria-current=page]")?.textContent);
        prod.click(); await wait(10);
        log("click", panel.matches(":popover-open"), prod.getAttribute("aria-expanded"), [...panel.querySelectorAll("a")].map((a) => a.textContent).join(" / "));
        prod.click(); await wait(10);
        prod.focus(); await key("ArrowDown");
        log("down", panel.matches(":popover-open"), name());
        await key("Escape"); await wait(20);
        log("escape", panel.matches(":popover-open"), document.activeElement === prod);
        await key("ArrowRight");
        log("along", name());
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("menubar", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "bar Menu bar 0,-1,-1",
            "right Edit false -1,0,-1",
            "down true Undo true",
            "menu right false true Status bar menuitemcheckbox true",
            "checkbox false false true",
            "enter opens true Status bar",
            "radio 150% false",
            "picked 150",
            "wraps File",
            "sub item Share menu",
            "sub open true Email true true",
            "sub closed false Share true",
            "hover true Share",
            "sub chose Link false false",
            "hover bar false true",
            "nav false Docs",
            "click true true AnalyticsSee what's happening / Automation",
            "down true AnalyticsSee what's happening",
            "escape false true",
            "along Docs",
        ],
        "\n{result}"
    );
}

/// One-time codes: what the pattern keeps out, pasting, the boxes, the `complete` event,
/// clearing from state, and the field label.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn input_otp_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "code": "", "pin": "", "ref": "", "done": "" },
        "root": { "type": "stack", "children": [
            { "type": "ui-field", "props": { "label": "Code" }, "children": [
                { "type": "ui-input-otp", "props": { "bind": "code", "groups": [3, 3] },
                  "on": { "complete": { "action": "set_state", "key": "done", "value": "{{ code }}" } } } ] },
            { "type": "ui-input-otp", "props": { "id": "pin", "bind": "pin", "length": 4, "mask": true } },
            { "type": "ui-input-otp", "props": { "id": "ref", "bind": "ref", "length": 5, "pattern": "alphanumeric" } } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const [code, pin, ref] = document.querySelectorAll("[data-otp-input]");
      const boxes = (i) => [...i.parentElement.querySelectorAll("[data-otp-slot]")].map((s) => s.querySelector("[data-otp-dot]") ? "•" : s.textContent || "_").join("");
      const active = (i) => [...i.parentElement.querySelectorAll("[data-otp-slot]")].findIndex((s) => s.hasAttribute("data-active"));
      const type = async (i, v) => { i.focus(); i.value = v; i.setSelectionRange(v.length, v.length); i.dispatchEvent(new Event("input", { bubbles: true })); await wait(20); };
      try {
        log("label", document.querySelector(`label[for="${code.id}"]`)?.textContent, code.getAttribute("autocomplete"), code.inputMode, code.maxLength);
        log("groups", code.parentElement.querySelectorAll("[data-otp-group]").length, code.parentElement.querySelectorAll("[data-otp-sep]").length);
        await type(code, "12a3");
        log("typed", S().code, boxes(code), active(code), JSON.stringify(S().done));
        await type(code, "98-76-54");
        log("pasted", S().code, boxes(code), S().done);
        await type(code, "98765");
        log("backspace", S().code, boxes(code), active(code));
        root.__irender.set("code", ""); await wait(20);
        log("cleared", JSON.stringify(code.value), boxes(code));
        await type(pin, "4821");
        log("pin", S().pin, boxes(pin), pin.type);
        await type(ref, "ab-12c9");
        log("ref", S().ref, boxes(ref));
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("otp", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "label Code one-time-code numeric 6",
            "groups 2 1",
            "typed 123 123___ 3 \"\"",
            // Pasted with dashes: the digits are kept, and the full code completes.
            "pasted 987654 987654 987654",
            "backspace 98765 98765_ 5",
            "cleared \"\" ______",
            "pin 4821 •••• password",
            "ref AB12C AB12C",
        ],
        "\n{result}"
    );
}

/// The scroll spy marks the link for the section being read, and keeps it in view in a
/// list that scrolls on its own.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn scroll_spy_in_chrome() {
    let ids = ["one", "two", "three", "four", "five"];
    let links: Vec<_> = ids.iter().map(|id| serde_json::json!({ "type": "element", "props": { "tag": "a" }, "attrs": { "href": format!("#{id}") },
        "style": { "display": "block", "height": 60 }, "children": [{ "type": "text", "props": { "value": id } }] })).collect();
    let sections: Vec<_> = ids.iter().enumerate().map(|(i, id)| serde_json::json!({ "type": "element", "props": { "tag": "section" }, "attrs": { "id": id },
        // The last is short: it's current only once the page reaches the bottom.
        "style": { "height": if i == 4 { 200 } else { 900 } }, "children": [{ "type": "text", "props": { "value": id } }] })).collect();
    let doc = serde_json::json!({ "version": 1, "root": { "type": "element", "children": [
        { "type": "ui-scroll-spy", "props": { "offset": 20 }, "style": { "position": "fixed", "top": 0, "left": 0, "width": 120, "height": 150, "overflow-y": "auto" }, "children": links },
        { "type": "element", "style": { "margin-left": 140 }, "children": sections } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const nav = document.querySelector("[data-i-x=ui-scroll-spy]");
      const cur = () => [...nav.querySelectorAll("[aria-current=location]")].map((a) => a.textContent).join(",");
      // Scroll events come with animation frames, which headless Chrome may not run.
      const go = async (y) => { scrollTo(0, y); dispatchEvent(new Event("scroll")); await wait(80); };
      try {
        log("start", cur(), nav.getAttribute("aria-label"), nav.tagName);
        await go(950); log("two", cur());
        await go(2000); log("three", cur(), "list scrolled", nav.scrollTop > 0);
        await go(1e6); log("bottom", cur(), "last link shown", nav.querySelector("[aria-current]").getBoundingClientRect().bottom <= nav.getBoundingClientRect().bottom);
        await go(0); log("back", cur(), "list back", nav.scrollTop === 0);
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("scroll-spy", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "start one On this page NAV",
            "two two",
            "three three list scrolled true",
            "bottom five last link shown true",
            "back one list back true",
        ],
        "\n{result}"
    );
}
