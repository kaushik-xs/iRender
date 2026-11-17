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
        .args(["--headless=new", "--no-sandbox", "--disable-gpu", "--window-size=1000,800", "--virtual-time-budget=8000", "--dump-dom"])
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
    let out = Command::new(&chrome)
        .args(["--headless=new", "--no-sandbox", "--disable-gpu", "--window-size=1000,900", "--virtual-time-budget=10000", "--dump-dom"])
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

/// Forces dark, then light, then back to the system scheme (light in headless Chrome).
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
