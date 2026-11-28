//! Drives the gallery's interactive components in headless Chrome.
//!
//! Ignored by default because it needs Chrome:
//!   cargo test -p irender-ui-kit --test browser -- --ignored
//! Set CHROME=/path/to/chrome if it isn't found automatically.

#[path = "browser/components.rs"]
mod components;

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
  const toast = () => $$("[role=status] > div:not([data-leaving])").map((t) => t.firstElementChild.textContent).join(",") || "hidden";
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

    $$("#tabs [role=tab]")[1].click();
    log("tabs", $$("#tabs [role=tabpanel][id^=settings-tabs]").map((p) => (p.hidden ? "-" : "shown")).join(","), S().tab);

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
/// As [`drive`], for a page served over HTTP (where IndexedDB keeps things), at `url`, in
/// real time: virtual time runs ahead of IndexedDB and key derivation, which take real
/// time, and can end the page early. The page holds its `load` (which `--dump-dom` waits
/// for) until its driver's done, with an image its server answers only then.
fn drive_url(url: &str) -> Option<String> {
    let Some(chrome) = chrome() else {
        eprintln!("skipping: Chrome not found (set CHROME=/path/to/chrome)");
        return None;
    };
    // A profile of its own: two Chromes on one profile hand over to each other.
    static RUNS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let run = RUNS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("irender-ui-kit-profile-{}-{run}", std::process::id()));
    let out = Command::new(&chrome)
        // A real-time cap (Chrome stops loading then; dump-dom.js waits that long).
        .args(["--headless=new", "--no-sandbox", "--disable-gpu", "--window-size=1000,900", "--timeout=120000", "--dump-dom"])
        .arg(format!("--user-data-dir={}", dir.display()))
        .arg(url)
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let dom = String::from_utf8_lossy(&out.stdout).into_owned();
    const RESULT: &str = "<pre id=\"result\">";
    let start = dom.find(RESULT).unwrap_or_else(|| panic!("driver did not run:\n{dom}")) + RESULT.len();
    Some(dom[start..start + dom[start..].find("</pre>").unwrap()].replace("&quot;", "\"").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&"))
}

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
    let load = || {
        let out = Command::new(&chrome)
            .args(["--headless=new", "--no-sandbox", "--disable-gpu", "--blink-settings=preferredColorScheme=1", "--window-size=1000,900", "--virtual-time-budget=30000", "--dump-dom"])
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
  // Waits for a server's answer rather than a fixed time. Mostly message-channel turns,
  // which hold Chrome's virtual time still (it runs ahead of a real server), with a
  // little of it now and then for the page's own timers.
  const tick = () => new Promise((r) => { const c = new MessageChannel(); c.port1.onmessage = r; c.port2.postMessage(0); });
  const until = async (f) => { for (let i = 0; i < 2e6 && !f(); i++) await (i % 50 ? tick() : wait(10)); };
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
    await until(() => !fruit.hasAttribute("data-loading") && opts(fruit).length);
    log("loaded", opts(fruit).join(","), fruit.hasAttribute("data-loading"));
    key(fruit, "ArrowDown"); key(fruit, "Enter");
    log("fruit", S().fruit);
    type(fruit, "man"); await wait(150); await until(() => !fruit.hasAttribute("data-loading") && opts(fruit).length);
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
    // Read once the colour has stopped changing: with transitions off, and after a few
    // reads that agree (a loaded machine can run the load event before styles settle).
    let report = r#"<style>*, *::before, *::after { transition: none !important; }</style><script>window.addEventListener("load", async () => {
      const color = () => getComputedStyle(document.getElementById("marked")).color;
      let last = color();
      for (let same = 0, i = 0; same < 3 && i < 100; i++) {
        await new Promise((r) => setTimeout(r, 20));
        const now = color();
        same = now === last ? same + 1 : 0;
        last = now;
      }
      const pre = document.createElement("pre"); pre.id = "result";
      pre.textContent = last + " " + document.documentElement.dataset.theme;
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
      const toasts = () => [...region.querySelectorAll(":scope > div:not([data-leaving])")]
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
      region.querySelector(":scope > div:not([data-leaving]) [data-dismiss]").click();
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

/// `ui-sidebar-action`: a group's button beside its label, an item's at the end of its
/// row (shown on hover or focus), one opening a menu, and none while collapsed to icons.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn sidebar_actions_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "open": true, "ran": "" },
        "root": { "type": "ui-sidebar-layout", "props": { "bind": "open", "collapsible": "icon" }, "style": { "min-height": 300 }, "children": [
            { "type": "ui-sidebar", "children": [ { "type": "ui-sidebar-content", "children": [
                { "type": "ui-sidebar-group", "props": { "label": "Projects" }, "children": [
                    { "type": "ui-sidebar-action", "props": { "label": "Add project", "icon": "+" },
                        "on": { "click": { "action": "set_state", "key": "ran", "value": "add" } } },
                    { "type": "ui-sidebar-item", "props": { "label": "Design engineering", "icon": "D", "href": "#design" }, "children": [
                        { "type": "ui-sidebar-action", "props": { "label": "More", "showOnHover": true }, "children": [
                            { "type": "ui-menu-item", "props": { "label": "Share" }, "on": { "click": { "action": "set_state", "key": "ran", "value": "share" } } },
                            { "type": "ui-menu-item", "props": { "label": "Delete", "variant": "destructive" } } ] } ] },
                    { "type": "ui-sidebar-item", "props": { "label": "Sales", "icon": "S" } } ] } ] } ] },
            { "type": "ui-sidebar-trigger" } ] } });
    let driver = r##"<style>* { transition: none !important; }</style><script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const $ = (s) => document.querySelector(s), S = () => $("[data-i-root]").__irender.state;
      const [add, more] = document.querySelectorAll("[data-sidebar-action]");
      const row = more.closest("li"), link = row.querySelector("a");
      const r = (el) => el.getBoundingClientRect();
      try {
        const label = $("[data-sidebar-group] > div");
        log("group", add.getAttribute("aria-label"), add.parentElement === label, Math.round(r(label).right - r(add).right) < 12, $("[data-sidebar-group] li a").textContent);
        add.click(); await wait(10);
        log("clicked", S().ran);
        log("item", Math.abs(r(more).top + r(more).height / 2 - (r(link).top + r(link).height / 2)) < 2, r(more).right <= r(row).right, r(link).right - r(more).left >= 20,
          getComputedStyle(more).opacity, more.getAttribute("aria-haspopup"), link.querySelector("[data-label]").getBoundingClientRect().right <= r(more).left + 1);
        link.focus(); await wait(10);
        log("focused", getComputedStyle(more).opacity);
        more.click(); await wait(20);
        const menu = document.getElementById(more.getAttribute("popovertarget"));
        log("menu", menu.matches(":popover-open"), menu.getAttribute("role"), [...menu.querySelectorAll("[role=menuitem]")].map((m) => m.textContent.trim()).join(","), location.hash);
        menu.querySelector("[role=menuitem]").click(); await wait(20);
        log("chose", S().ran, menu.matches(":popover-open"), location.hash);
        $("[data-sidebar-trigger]").click(); await wait(20);
        log("icons", getComputedStyle(add).display, getComputedStyle(more).display);
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("sidebar-actions", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "group Add project true true DDesign engineering",
            "clicked add",
            // Centred on the link's row, inside it, over its end; hidden until hovered.
            "item true true true 0 menu true",
            "focused 1",
            "menu true menu Share,Delete ",
            "chose share false ",
            "icons none none",
        ],
        "\n{result}"
    );
}

/// `remember`: open or closed comes back from localStorage as the page loads, without
/// sliding, and each change is saved; `remember: name` uses that name.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn sidebar_remembers_in_chrome() {
    let layout = |id: &str, bind: &str, remember: serde_json::Value| serde_json::json!({ "type": "ui-sidebar-layout", "props": { "bind": bind, "remember": remember }, "attrs": { "id": id }, "style": { "min-height": 100 }, "children": [
        { "type": "ui-sidebar", "children": [ { "type": "ui-sidebar-content" } ] },
        { "type": "ui-sidebar-trigger" } ] });
    let doc = serde_json::json!({ "version": 1, "state": { "nav": true, "docs": false, "plain": true },
        "root": { "type": "stack", "children": [ layout("a", "nav", serde_json::json!(true)), layout("b", "docs", serde_json::json!("docs-pages")), layout("c", "plain", serde_json::json!(null)) ] } });
    let early = r#"<script>localStorage.clear(); localStorage.setItem("irender-sidebar:nav", "false"); localStorage.setItem("irender-sidebar:docs-pages", "true");
    // Seen as it's set: a frame or two can pass before `load`.
    new MutationObserver((l) => l.forEach((m) => { if (m.target.id === "a" && m.target.hasAttribute("data-restoring")) window.restoring = true; })).observe(document, { subtree: true, attributes: true, attributeFilter: ["data-restoring"] });</script>"#;
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const $ = (s) => document.querySelector(s), S = () => $("[data-i-root]").__irender.state;
      await wait(150);
      try {
        log("restored", S().nav, S().docs, S().plain, $("#a").dataset.state, $("#b").dataset.state, !!window.restoring, $("#a").hasAttribute("data-restoring"));
        $("#a [data-sidebar-trigger]").click(); await wait(20);
        $("#b [data-sidebar-trigger]").click(); await wait(20);
        $("#c [data-sidebar-trigger]").click(); await wait(20);
        log("saved", localStorage.getItem("irender-sidebar:nav"), localStorage.getItem("irender-sidebar:docs-pages"), String(localStorage.getItem("irender-sidebar:plain")));
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replacen("<head>", &format!("<head>{early}"), 1).replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("sidebar-remember", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            // Restored with sliding off, which comes back on after a frame or two.
            "restored false true true collapsed expanded true false",
            "saved true false null",
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
      // Tab goes round the panel's links.
      const links = [...aside.querySelectorAll("a[href]")], name = () => d.activeElement.textContent;
      links.at(-1).focus(); key(d, "Tab"); await wait(5);
      out.push(`tab ${{name()}}`);
      key(d, "Tab", {{ shiftKey: true }}); await wait(5);
      out.push(`shift+tab ${{name()}}`);
      // A touch swipe toward the edge: a short one springs back, a long one shuts it.
      const touch = (type, x, y) => links[0].dispatchEvent(new w.PointerEvent(type, {{ bubbles: true, cancelable: true, pointerId: 7, pointerType: "touch", clientX: x, clientY: y, button: 0 }}));
      // Slowly, so it isn't a flick.
      touch("pointerdown", 200, 100); await wait(100); touch("pointermove", 190, 100); await wait(100); touch("pointermove", 170, 101); await wait(100); touch("pointermove", 150, 102); await wait(100);
      out.push(`dragging ${{aside.style.transform}} ${{aside.hasAttribute("data-dragging")}}`);
      touch("pointermove", 150, 102); touch("pointerup", 150, 102); await wait(400);
      sheet("short swipe");
      touch("pointerdown", 200, 100); touch("pointermove", 200, 140); touch("pointermove", 60, 140); touch("pointerup", 60, 140); await wait(400);
      sheet("scroll");
      touch("pointerdown", 200, 100); touch("pointermove", 190, 100); touch("pointermove", 60, 100); touch("pointerup", 60, 100); await wait(400);
      sheet("swipe");
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
            "tab HHome",
            "shift+tab DDocs",
            "dragging translateX(-50px) true",
            "short swipe open visible 0,600 inert true expanded true bind true focus panel",
            // Up and down first: a scroll, not a swipe.
            "scroll open visible 0,600 inert true expanded true bind true focus panel",
            "swipe shut hidden -256,600 inert false expanded false bind true focus trigger",
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

/// Padding, gaps, control size and icon size each scale one kind of size, on top of
/// density, here on part of the page.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn padding_gap_control_and_icon_sizes_scale_their_own_in_chrome() {
    let probes = |p: &str| serde_json::json!([
        { "type": "ui-button", "props": { "label": "Probe" }, "attrs": { "id": format!("{p}-button") } },
        { "type": "stack", "props": { "direction": "row", "gap": 10 }, "attrs": { "id": format!("{p}-stack") },
          "children": [ { "type": "text", "props": { "value": "a" } }, { "type": "text", "props": { "value": "b" } } ] },
        { "type": "ui-date-picker", "attrs": { "id": format!("{p}-date") } } ]);
    let doc = serde_json::json!({ "version": 1,
        "root": { "type": "stack", "children": [
            { "type": "stack", "children": probes("page") },
            { "type": "stack", "attrs": { "data-theme-padding": "roomy", "data-theme-gap": "roomy", "data-theme-control-size": "large", "data-theme-icon-size": "large" },
              "children": probes("part") } ] } });
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let mut doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    irender_ui_kit::apply_theme(&mut doc);
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test");
    let driver = r##"<script>
    window.addEventListener("load", () => {
      const $ = (id) => document.getElementById(id);
      const line = (p) => [
        Math.round($(`${p}-button`).getBoundingClientRect().height),
        getComputedStyle($(`${p}-button`)).paddingLeft,
        getComputedStyle($(`${p}-stack`)).columnGap,
        Math.round($(`${p}-date`).querySelector("svg").getBoundingClientRect().width * 10) / 10,
      ].join(" ");
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = `${line("page")}\n${line("part")}`;
      document.body.append(pre);
    });
    </script>"##;
    let html = html.replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("split-scales", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "36 16px 10px 16",
            // 40px controls, 1.25 × padding and gaps, 1.25 × icons.
            "40 20px 12.5px 20",
        ],
        "\n{result}"
    );
}

/// The typography and shape options on part of the page: heading font, weights, line
/// height and letter spacing for text, border width, shadows and the two radii.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn typography_and_shape_options_apply_in_chrome() {
    let probes = |p: &str| serde_json::json!([
        { "type": "ui-card", "props": { "title": "Title", "description": "Some words" }, "attrs": { "id": format!("{p}-card") } },
        { "type": "ui-button", "props": { "label": "Probe", "variant": "outline" }, "attrs": { "id": format!("{p}-button") } },
        { "type": "text", "props": { "value": "Plain" }, "attrs": { "id": format!("{p}-text") } } ]);
    let doc = serde_json::json!({ "version": 1,
        "root": { "type": "stack", "children": [
            { "type": "stack", "children": probes("page") },
            { "type": "stack", "attrs": {
                "data-theme-heading-font": "serif", "data-theme-font-weight": "bolder", "data-theme-line-height": "relaxed",
                "data-theme-letter-spacing": "wide", "data-theme-border": "thick", "data-theme-elevation": "flat",
                "data-theme-control-radius": "round", "data-theme-card-radius": "x-large" },
              "children": probes("part") } ] } });
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let mut doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    irender_ui_kit::apply_theme(&mut doc);
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test");
    let driver = r##"<script>
    window.addEventListener("load", () => {
      const cs = (sel) => getComputedStyle(document.querySelector(sel));
      const line = (p) => {
        const title = cs(`#${p}-card h3`), desc = cs(`#${p}-card p`), card = cs(`#${p}-card`), button = cs(`#${p}-button`), text = cs(`#${p}-text`);
        return [
          /serif/.test(title.fontFamily) && !/sans-serif/.test(title.fontFamily) ? "serif" : "same",
          // WebKit gives 23.800001px.
          title.fontWeight, `${+parseFloat(desc.lineHeight).toFixed(2)}px`, text.letterSpacing,
          card.borderTopWidth, card.borderTopLeftRadius, card.boxShadow === "none" ? "flat" : "shadow",
          button.borderTopWidth, button.borderTopLeftRadius,
        ].join(" ");
      };
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = `${line("page")}\n${line("part")}`;
      document.body.append(pre);
    });
    </script>"##;
    let html = html.replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("typography-shape", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "same 600 21px normal 1px 10px shadow 1px 8px",
            // Serif headings, bolder (semibold is 700), 1.7 × 14px lines, wide tracking
            // (0.025em of 16px), 2px borders, no card shadow, 24px cards, 18px controls.
            "serif 700 23.8px 0.4px 2px 24px flat 2px 18px",
        ],
        "\n{result}"
    );
}

/// The switch scales whole with density: its knob keeps a 1px gap all round, off and on.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn switch_knob_fits_its_track_at_every_density_in_chrome() {
    let switches: Vec<_> = ["compact", "default", "comfortable"].iter().flat_map(|d| [false, true].map(|on| serde_json::json!(
        { "type": "stack", "attrs": { "data-theme-density": d }, "children": [
            { "type": "ui-switch", "props": { "label": "S", "checked": on }, "attrs": { "data-probe": format!("{d} {}", if on { "on" } else { "off" }) } } ] }))).collect();
    let doc = serde_json::json!({ "version": 1, "root": { "type": "stack", "children": switches } });
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let mut doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    irender_ui_kit::apply_theme(&mut doc);
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test");
    // The knob is a pseudo-element: its box from its computed size and offset.
    let driver = r##"<style>* { transition: none !important; }</style><script>
    window.addEventListener("load", () => {
      const out = [...document.querySelectorAll("[data-probe]")].map((p) => {
        const input = p.matches("input") ? p : p.querySelector("input");
        const t = getComputedStyle(input), k = getComputedStyle(input, "::before");
        const inner = [parseFloat(t.width) - 2, parseFloat(t.height) - 2];
        const size = [parseFloat(k.width), parseFloat(k.height)];
        const shift = new DOMMatrix(k.transform).m41;
        const left = 1 + shift, top = 1;
        const gaps = [left, inner[0] - size[0] - left, top, inner[1] - size[1] - top].map((g) => Math.round(g * 10) / 10);
        return `${p.dataset.probe}: ${size[0] === size[1] ? "round" : "oval"} ${gaps.join(" ")}`;
      });
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n");
      document.body.append(pre);
    });
    </script>"##;
    let html = html.replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("switch-density", &html) else { return };
    assert_eq!(result.lines().count(), 6, "{result}");
    for line in result.lines() {
        let (_, gaps) = line.split_once(": ").unwrap();
        let mut g = gaps.split(' ');
        assert_eq!(g.next(), Some("round"), "{result}");
        let g: Vec<f64> = g.map(|v| v.parse().unwrap()).collect();
        // Top and bottom 1px; the knob at one end with 1px to spare.
        assert!((g[2] - 1.0).abs() < 0.2 && (g[3] - 1.0).abs() < 0.2, "{result}");
        let end = if line.contains(" on:") { g[1] } else { g[0] };
        assert!((end - 1.0).abs() < 0.2, "{result}");
    }
}

/// A choice set on a node applies inside it only: density and text size reach the
/// kit's components, the built-ins and `$space.*` tokens there, and the default
/// palette there undoes the one picked for the page.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn theme_choices_on_a_node_apply_inside_it_in_chrome() {
    let probes = |p: &str| serde_json::json!([
        { "type": "ui-button", "props": { "label": "Probe" }, "attrs": { "id": format!("{p}-button") } },
        { "type": "button", "props": { "label": "Built-in" }, "attrs": { "id": format!("{p}-builtin") } },
        { "type": "stack", "style": { "padding": "$space.md" }, "attrs": { "id": format!("{p}-box") } } ]);
    let doc = serde_json::json!({ "version": 1,
        "root": { "type": "stack", "children": [
            { "type": "stack", "children": probes("page") },
            { "type": "stack", "attrs": { "data-theme-density": "compact", "data-theme-palette": "zinc", "data-theme-text-size": "large" }, "children": probes("part") } ] } });
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let mut doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    irender_ui_kit::apply_theme(&mut doc);
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test");
    let driver = r##"<script>
    window.addEventListener("load", () => {
      const cs = (id) => getComputedStyle(document.getElementById(id));
      const line = (p) => [
        Math.round(document.getElementById(`${p}-button`).getBoundingClientRect().height),
        cs(`${p}-button`).fontSize, cs(`${p}-button`).backgroundColor,
        cs(`${p}-builtin`).paddingTop, cs(`${p}-box`).paddingTop,
      ].join(" ");
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = `${line("page")}\n${line("part")}`;
      document.body.append(pre);
    });
    </script>"##;
    let html = html
        .replace("<head>", r#"<head><script>localStorage.setItem("irender-theme-options", '{"palette":"rose"}'); localStorage.removeItem("irender-theme")</script>"#)
        .replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("theme-choices-on-a-node", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "36 14px rgb(225, 29, 72) 8px 16px",
            // 0.8 × the spacing, 1.125 × the text, and zinc's primary again.
            "29 15.75px rgb(24, 24, 27) 6.4px 12.8px",
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
      // The reply isn't still arriving.
      out.push(`streaming ${S().messages.some((m) => "streaming" in m)}`);
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
            "streaming false",
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
      // Waits for the reply in real time, as in the chat tests above: virtual time jumps
      // ahead of the test server under load, and a message channel keeps it still.
      const tick = () => new Promise((r) => { const c = new MessageChannel(); c.port1.onmessage = r; c.port2.postMessage(0); });
      const until = async (f) => { for (let i = 0; i < 2e6 && !f(); i++) await tick(); };
      try {
        const box = $("textarea");
        box.value = "Show me"; box.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
        await until(() => $("#inner"));
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
      } catch (e) { out.push(`EXCEPTION ${e.stack}`); }
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
      // Waits in real time for the server's answers, with a little virtual time now and
      // then for the page's own timers: virtual time alone runs ahead of the test server.
      const tick = () => new Promise((r) => { const c = new MessageChannel(); c.port1.onmessage = r; c.port2.postMessage(0); });
      for (let i = 0; i < 2e6 && shown().includes(": \n"); i++) await (i % 50 ? tick() : wait(10));
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

/// A virtual table keeps a row's focus while the row scrolls out of the drawn window:
/// the scroll area holds it, the row gets it back when it's drawn again, and a key
/// pressed meanwhile acts from that row.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn data_table_virtual_focus_in_chrome() {
    let rows = (0..500).map(|i| serde_json::json!({ "id": i, "name": format!("Row {i}") })).collect::<Vec<_>>();
    let doc = serde_json::json!({ "version": 1,
        "root": { "type": "ui-data-table", "props": { "rows": rows, "virtual": true, "pageSize": 0, "height": 240, "selectable": true,
            "columns": [{ "field": "name" }] } } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const T = document.querySelector("[data-i-x=ui-data-table]"), sc = T.querySelector("[data-dt-scroll]");
      const row = (name) => [...T.querySelectorAll("[data-dt-body] > [data-dt-row]")].find((r) => r.textContent.includes(name + "") && r.querySelector("[data-dt-col=name]").textContent === name);
      const scroll = async (top) => { sc.scrollTop = top; sc.dispatchEvent(new Event("scroll")); await wait(40); };
      const where = () => document.activeElement === sc ? "scroll area" : document.activeElement.querySelector?.("[data-dt-col=name]")?.textContent ?? document.activeElement.tagName;
      const key = (k) => document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true }));
      try {
        await wait(50);
        row("Row 3").focus();
        log("focused", where());
        await scroll(sc.scrollHeight / 2);
        log("away", where(), !!row("Row 3"));
        await scroll(0);
        log("back", where(), row("Row 3").tabIndex);
        await scroll(sc.scrollHeight / 2);
        key("ArrowDown"); await wait(20);
        log("down", where(), sc.scrollTop < 400);
        await scroll(sc.scrollHeight);
        key(" "); await wait(20);
        log("space", where(), T.querySelector("[data-dt-status]").textContent);
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("data-table-virtual-focus", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "focused Row 3",
            "away scroll area false",
            "back Row 3 0",
            // Down from Row 3, drawn again and scrolled to.
            "down Row 4 true",
            // Space picks the held row, not one in view.
            "space Row 4 1 of 500 row(s) selected.",
        ],
        "\n{result}"
    );
}

/// The server's starting order is the browser's: text by locale with numbers by value,
/// case and accents aside, so rows don't move when the script takes over.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn data_table_server_order_is_the_browsers_in_chrome() {
    let names = ["Item 10", "item 9", "Item 1", "zebra", "Émile", "eve", "Ada", "b", "B", "a", "v010", "v9", "v2a", "_x", "1", "10", "9", "Ölaf", "oscar", "Zoë", "zoe"];
    let rows: Vec<_> = names.iter().enumerate().map(|(i, n)| serde_json::json!({ "id": i, "name": n })).collect();
    let doc = serde_json::json!({ "version": 1,
        "root": { "type": "ui-data-table", "props": { "rows": rows, "rowKey": "id", "sort": "name", "pageSize": 0, "locale": "en-US", "columns": [{ "field": "name" }] } } });
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let page = engine.render(&ir, &serde_json::json!({})).unwrap();
    // The server's order: where each name's cell is in its markup.
    let body = &page.body;
    let mut server: Vec<(usize, &str)> = names.iter().map(|n| (body.find(&format!(">{n}<")).expect(n), *n)).collect();
    server.sort();
    let server: Vec<&str> = server.into_iter().map(|(_, n)| n).collect();
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      await new Promise((r) => setTimeout(r, 30));
      const names = [...document.querySelectorAll("[data-dt-body] > [data-dt-row]:not([hidden]) [data-dt-col=name]")].map((td) => td.textContent);
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = names.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let html = page.document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("data-table-server-order", &html) else { return };
    assert_eq!(server, result.lines().collect::<Vec<_>>(), "server first, then the browser's");
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
      // Waits in real time for the server, with a little virtual time now and then for the
      // table's own timers: virtual time alone runs ahead of the test server under load.
      const tick = () => new Promise((r) => { const c = new MessageChannel(); c.port1.onmessage = r; c.port2.postMessage(0); });
      const until = async (f) => { for (let i = 0; i < 2e6 && !f(); i++) await (i % 50 ? tick() : wait(10)); };
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

/// A remote table asks its API for facets and totals with a query's first batch, and
/// uses them for the select filter's choices and the footer.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn data_table_remote_facets_in_chrome() {
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
            let (path, q) = target.split_once('?').unwrap_or((&target, ""));
            let query: std::collections::HashMap<String, String> =
                q.split('&').filter_map(|kv| kv.split_once('=')).map(|(k, v)| (k.to_string(), v.replace("%2C", ",").replace("%3A", ":"))).collect();
            seen.lock().unwrap().push(format!("{path} {}", q.replace("%2C", ",").replace("%3A", ":")));
            let num = |k: &str, d: usize| query.get(k).and_then(|v| v.parse().ok()).unwrap_or(d);
            let (offset, limit) = (num("offset", 0), num("limit", 1000));
            // 30 rows: status a, b or c in turn; n is the row's number.
            let all: Vec<_> = (0..30).map(|i| serde_json::json!({ "id": i, "status": ("abc".as_bytes()[i % 3] as char).to_string(), "n": i })).collect();
            let rows: Vec<_> = all.iter().skip(offset).take(limit).cloned().collect();
            let mut body = serde_json::json!({ "rows": rows, "total": 30 });
            if path == "/full" && query.contains_key("facets") {
                body["facets"] = serde_json::json!({ "status": [{ "value": "a", "count": 10 }, { "value": "b", "count": 10 }, { "value": "c", "count": 10 }] });
            }
            if path == "/full" && query.contains_key("summary") {
                body["summary"] = serde_json::json!({ "n:sum": 435 });
            }
            let body = body.to_string();
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    let cols = serde_json::json!([
        { "field": "status", "label": "Status", "filter": "select" },
        { "field": "n", "label": "N", "summary": "sum" },
        { "field": "id", "label": "Rows", "summary": "count" } ]);
    let table = |id: &str, path: &str| serde_json::json!({ "type": "ui-data-table", "props": { "id": id, "url": format!("http://127.0.0.1:{port}/{path}"), "paging": "more", "pageSize": 4, "columns": cols } });
    let doc = serde_json::json!({ "version": 1, "root": { "type": "stack", "children": [table("full", "full"), table("plain", "plain")] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const tick = () => new Promise((r) => { const c = new MessageChannel(); c.port1.onmessage = r; c.port2.postMessage(0); });
      const until = async (f) => { for (let i = 0; i < 2e6 && !f(); i++) await (i % 50 ? tick() : wait(10)); };
      const T = (id) => document.getElementById(id);
      const idle = (t) => until(() => !t.hasAttribute("data-loading"));
      const sums = (t) => [...t.querySelectorAll("[data-dt-sum]")].map((td) => td.textContent).join(" ");
      const choices = async (t) => {
        const p = t.querySelector("[data-dt-filter]");
        p.showPopover();
        await wait(30);
        const s = [...p.querySelectorAll("[data-dt-options] label")].map((l) => [...l.querySelectorAll("span")].map((x) => x.textContent).join(":")).join(" ");
        p.hidePopover();
        return s;
      };
      try {
        const full = T("full"), plain = T("plain");
        await idle(full); await idle(plain);
        log("full", await choices(full), "|", sums(full));
        log("plain", await choices(plain), "|", sums(plain));
        full.querySelector("[data-dt-load]").click(); await wait(30); await idle(full);
        log("more", full.querySelectorAll("[data-dt-body] > [data-dt-row]").length, await choices(full), "|", sums(full));
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("data-table-remote-facets", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            // Every value with its count, and totals over all 30 rows.
            "full a:10 b:10 c:10 | 435 30",
            // Without them: the values and sums of the 4 rows loaded.
            "plain a:2 b:1 c:1 | 6 30",
            // The next batch keeps them, and doesn't ask again.
            "more 8 a:10 b:10 c:10 | 435 30",
        ],
        "\n{result}"
    );
    let mut log = log.lock().unwrap().clone();
    log.sort();
    assert_eq!(
        log,
        [
            "/full offset=0&limit=4&facets=status&summary=n:sum",
            "/full offset=4&limit=4",
            "/plain offset=0&limit=4&facets=status&summary=n:sum",
        ],
        "{log:?}"
    );
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
      // Waits until the page stops changing (40 ms without a DOM change, at most 2 s):
      // as quick as a fixed wait on an idle machine, and patient on a loaded one.
      const settle = (quiet = 40, most = 2000) => new Promise((done) => {
        let t;
        const end = () => { obs.disconnect(); clearTimeout(t); clearTimeout(cap); done(); };
        const obs = new MutationObserver(() => { clearTimeout(t); t = setTimeout(end, quiet); });
        obs.observe(document, { subtree: true, childList: true, attributes: true, characterData: true });
        t = setTimeout(end, quiet);
        const cap = setTimeout(end, most);
      });
      const T = (id) => document.getElementById(id);
      const rows = (t) => [...t.querySelectorAll("[data-dt-body] > [data-dt-row]")];
      const view = (t) => rows(t).map((r) => "  ".repeat(Number(r.getAttribute("aria-level")) - 1) + r.querySelector("[data-dt-col=name]").textContent.trim() + (r.getAttribute("aria-expanded") === "true" ? "-" : r.hasAttribute("aria-expanded") ? "+" : "")).join(" | ");
      const key = (el, k) => el.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true }));
      try {
        await wait(50);
        const t = T("tree");
        log("roles", t.querySelector("table").getAttribute("role"));
        log("start", view(t));
        rows(t)[0].querySelector("[data-dt-twisty]").click(); await settle();
        log("open alpha", view(t));
        const two = rows(t).find((r) => r.textContent.includes("Alpha two"));
        two.focus(); key(two, "ArrowRight"); await settle();
        log("right", view(t), document.activeElement.textContent.includes("Alpha two"));
        key(document.activeElement, "ArrowRight"); await settle();
        log("into", document.activeElement.querySelector("[data-dt-col=name]").textContent.trim());
        key(document.activeElement, "ArrowLeft"); await settle();
        log("to parent", document.activeElement.querySelector("[data-dt-col=name]").textContent.trim());
        t.querySelector("[data-dt-sort=name]").click(); t.querySelector("[data-dt-sort=name]").click(); await settle();
        log("desc", view(t));
        rows(t).find((r) => r.textContent.includes("Beta")).querySelector("[data-dt-twisty]").click();
        // Its children come from the server: wait for them (virtual time held still).
        const tick = () => new Promise((r) => { const c = new MessageChannel(); c.port1.onmessage = r; c.port2.postMessage(0); });
        for (let i = 0; i < 2e6 && !view(t).includes("Loaded 1"); i++) await (i % 50 ? tick() : wait(10));
        await settle();
        log("lazy", view(t));
        log("sum", t.querySelector("[data-dt-sum]").textContent);
        // Pinned columns: scroll sideways; Name stays at the left edge, Total at the right.
        const sc = t.querySelector("[data-dt-scroll]");
        const name = () => t.querySelector("[data-dt-body] [data-dt-col=name]").getBoundingClientRect();
        const total = () => t.querySelector("[data-dt-body] [data-dt-col=v]").getBoundingClientRect();
        const box = t.querySelector("[data-dt-body] [data-dt-box]").getBoundingClientRect();
        const before = [name().left, total().right];
        sc.scrollLeft = 400; await settle();
        const edge = sc.getBoundingClientRect();
        log("pinned", sc.scrollWidth > sc.clientWidth, name().left === before[0], Math.round(name().left - box.right) === 0, Math.abs(total().right - (edge.left + sc.clientWidth + 1)) <= 2,
          t.querySelector("[data-dt-body] [data-dt-col=name]").getAttribute("data-pin-edge"), t.querySelector("[data-dt-body] [data-dt-col=v]").getAttribute("data-pin-edge"));
        log("unpinned moved", t.querySelector("[data-dt-body] [data-dt-col=m0]").getBoundingClientRect().left < before[0]);

        const f = T("flat");
        log("flat", view(f));
        const s = f.querySelector("[data-dt-search]"); s.value = "account"; s.dispatchEvent(new Event("input", { bubbles: true })); await settle();
        log("search", view(f));
        s.value = ""; s.dispatchEvent(new Event("input", { bubbles: true })); await settle();
        rows(f)[0].querySelector("[data-dt-twisty]").click(); await settle();
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
            "calendar true 7",
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
    let doc = serde_json::json!({ "version": 1, "state": { "ctx": null, "ran": "", "row": null, "off": false, "picked": [] },
        "root": { "type": "stack", "children": [
            { "type": "ui-context-menu", "props": { "bind": "ctx", "disabled": "{{ off }}" }, "children": [
                { "type": "element", "attrs": { "id": "area", "tabindex": 0 }, "style": { "height": 120 }, "children": [
                    { "type": "element", "attrs": { "id": "file", "tabindex": 0, "data-context": "{\"name\":\"a.txt\"}" }, "children": [ { "type": "text", "props": { "value": "a.txt" } } ] } ] },
                item("Open", serde_json::json!({})),
                item("Locked", serde_json::json!({ "disabled": true })),
                { "type": "ui-menu-separator" },
                item("Delete", serde_json::json!({ "variant": "destructive" })) ] },
            { "type": "ui-context-menu", "props": { "bind": "row" }, "children": [
                { "type": "ui-data-table", "props": { "rows": [{ "id": 1, "name": "One" }, { "id": 2, "name": "Two" }, { "id": 3, "name": "Three" }], "rowKey": "id",
                    "selectable": true, "bindSelection": "picked", "pageSize": 0, "columns": [{ "field": "name" }] } },
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
        log("row", JSON.stringify(S().row), menu(two).querySelector("[role=menuitem]").textContent, menu(two).firstElementChild.textContent, JSON.stringify(S().picked), rowEl.tabIndex);
        menu(two).hidePopover(); await wait(20);
        // On a selected row the selection stays; on another, it's just that row.
        root.__irender.set("picked", [1, 3]); await wait(10);
        const rows = two.querySelectorAll("[data-dt-row]");
        right(rows[2].querySelector("td"), 50, 50); await wait(10);
        log("selected row", JSON.stringify(S().row), JSON.stringify(S().picked));
        menu(two).hidePopover(); await wait(20);
        rows[1].focus();
        key("F10", { shiftKey: true }); await wait(10);
        log("keyboard row", JSON.stringify(S().row), JSON.stringify(S().picked));
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
            // The row right-clicked is selected and focusable.
            "row {\"id\":2,\"name\":\"Two\"} Rename Two Two [2] 0",
            "selected row {\"id\":3,\"name\":\"Three\"} [1,3]",
            "keyboard row {\"id\":2,\"name\":\"Two\"} [2]",
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
      // Waits until the page stops changing (40 ms without a DOM change, at most 2 s):
      // as quick as a fixed wait on an idle machine, and patient on a loaded one.
      const settle = (quiet = 40, most = 2000) => new Promise((done) => {
        let t;
        const end = () => { obs.disconnect(); clearTimeout(t); clearTimeout(cap); done(); };
        const obs = new MutationObserver(() => { clearTimeout(t); t = setTimeout(end, quiet); });
        obs.observe(document, { subtree: true, childList: true, attributes: true, characterData: true });
        t = setTimeout(end, quiet);
        const cap = setTimeout(end, most);
      });
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const key = async (k) => { document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true })); await settle(); };
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
        document.activeElement.click(); await settle();
        log("checkbox", S().bar, isOpen(view), document.activeElement === view);
        await key("Enter");
        log("enter opens", isOpen(view), name());
        await key("ArrowDown"); await key("ArrowDown");
        log("radio", name(), document.activeElement.getAttribute("aria-checked"));
        document.activeElement.click(); await settle();
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
        share.dispatchEvent(new MouseEvent("mouseover", { bubbles: true })); await settle();
        log("hover", subMenu.matches(":popover-open"), name());
        [...subMenu.querySelectorAll("[role=menuitem]")][1].click(); await settle();
        log("sub chose", S().ran, isOpen(file), subMenu.matches(":popover-open"));
        file.click(); await settle();
        edit.dispatchEvent(new MouseEvent("mouseover", { bubbles: true })); await settle();
        log("hover bar", isOpen(file), isOpen(edit));
        menu(edit).hidePopover(); await settle();

        const nav = document.querySelector("nav[aria-label=Main]");
        const prod = nav.querySelector("[data-ui-nav-trigger]"), panel = document.getElementById(prod.getAttribute("aria-controls"));
        log("nav", prod.getAttribute("aria-expanded"), nav.querySelector("a[aria-current=page]")?.textContent);
        prod.click(); await settle();
        log("click", panel.matches(":popover-open"), prod.getAttribute("aria-expanded"), [...panel.querySelectorAll("a")].map((a) => a.textContent).join(" / "));
        prod.click(); await settle();
        prod.focus(); await key("ArrowDown");
        log("down", panel.matches(":popover-open"), name());
        await key("Escape"); await settle();
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
      // Scroll events come with animation frames, which headless Chrome may not run. Each
      // step moves the mark, so wait for that (updates are throttled, and a loaded
      // machine runs the timer late).
      const go = async (y) => {
        const was = cur();
        scrollTo(0, y); dispatchEvent(new Event("scroll"));
        for (let i = 0; i < 40 && cur() === was; i++) await wait(50);
        await wait(20);
      };
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

/// Filters: chips for what's applied, a panel that edits a draft until Apply, each
/// type's operators, and the `{ field: condition }` it writes filtering a table.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn filters_in_chrome() {
    let orders = serde_json::json!([
        { "id": "A", "status": "Paid", "amount": 3150, "date": "2026-10-28", "note": "Rush" },
        { "id": "B", "status": "Pending", "amount": 975, "date": "2026-10-25", "note": "" },
        { "id": "C", "status": "Paid", "amount": 6200, "date": "2026-10-22", "note": "Gift wrap" },
        { "id": "D", "status": "Refunded", "amount": 800, "date": "2026-10-20", "note": null },
        { "id": "E", "status": "Pending", "amount": 1500, "date": "2026-09-30", "note": "Call first" } ]);
    let filters = |bind: &str, live: bool| serde_json::json!({ "type": "ui-filters", "attrs": { "id": bind },
        "props": { "bind": bind, "rows": "{{ orders }}", "currency": "USD", "locale": "en-US", "live": live },
        "children": [
            { "type": "ui-filter", "props": { "field": "status", "label": "Status", "section": "Order" } },
            { "type": "ui-filter", "props": { "field": "amount", "label": "Amount", "type": "currency", "section": "Order" } },
            { "type": "ui-filter", "props": { "field": "date", "label": "Date", "type": "date", "operators": ["on", "before", "between"] } },
            { "type": "ui-filter", "props": { "field": "note", "label": "Note", "type": "text" } } ] });
    let doc = serde_json::json!({ "version": 1, "state": { "orders": orders, "f": { "status": ["Paid"], "amount": { "from": 1000 } }, "g": {} },
        "root": { "type": "stack", "children": [
            filters("f", false), filters("g", true),
            { "type": "ui-data-table", "props": { "rows": "{{ orders }}", "filter": "{{ f }}", "rowKey": "id" },
              "children": [{ "type": "ui-data-table-column", "props": { "field": "id", "label": "Id" } }] } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const F = document.getElementById("f"), G = document.getElementById("g");
      const $ = (s, from = F) => from.querySelector(s), $$ = (s, from = F) => [...from.querySelectorAll(s)];
      const chips = (el = F) => $$("[data-fl-chip]", el).map((c) => c.querySelector("[data-fl-edit]").textContent).join(" | ");
      const shown = () => [...document.querySelectorAll("[data-dt-body] > [data-dt-row]:not([hidden])")].map((r) => r.textContent.trim()).join("");
      const pane = (i, el = F) => $(`[data-fl-pane="${i}"]`, el);
      const options = (i) => $$("[data-fl-option]", pane(i)).map((o) => o.textContent + (o.querySelector("input").checked ? "*" : "")).join(",");
      const set = (input, v, type = "input") => { input.value = v; input.dispatchEvent(new Event(type, { bubbles: true })); };
      const json = (v) => JSON.stringify(v);
      try {
        log("start", chips(), "| count", $("[data-fl-count]").textContent, "| rows", shown());
        log("label", $("[data-fl-open]").getAttribute("aria-label"));
        // Open on the Amount chip: that field is shown.
        $$("[data-fl-edit]")[1].click(); await wait(0);
        const panel = $("[data-fl-panel]");
        log("open", panel.open, "pane", $$("[data-fl-pane]").findIndex((p) => !p.hidden), "op", $("[data-fl-op]", pane(1)).value, "a", $("[data-fl-a]", pane(1)).value, "b hidden", $("[data-fl-b]", pane(1)).hidden);
        log("tabs", $$("[role=tab]").map((t) => t.textContent).join(","), "sections", $$("[data-fl-section]").map((s) => s.textContent).join(","));
        // Arrow keys move between fields.
        $$("[role=tab]")[1].focus();
        $$("[role=tab]")[1].dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true }));
        log("arrow", document.activeElement.textContent, $$("[data-fl-pane]").findIndex((p) => !p.hidden));
        log("options", options(0));
        // Pick Pending, then cancel: nothing changes.
        const pending = $$("[data-fl-pick]", pane(0))[1];
        pending.click(); await wait(0);
        log("draft badge", $("[data-fl-n]", $$("[role=tab]")[0]).textContent, "state", json(S().f.status));
        panel.close(); await wait(0);
        log("cancel", json(S().f));
        // Again, and Apply.
        $("[data-fl-open]").click(); await wait(0);
        $$("[data-fl-pick]", pane(0))[1].click();
        // Amount: between, backwards, is an error and can't be applied.
        set($("[data-fl-op]", pane(1)), "between", "change");
        set($("[data-fl-b]", pane(1)), "500");
        log("bad", $("[data-fl-error]", pane(1)).textContent, "apply disabled", $("[data-fl-apply]").disabled);
        set($("[data-fl-b]", pane(1)), "4000");
        log("fixed", $("[data-fl-error]", pane(1)).hidden, "apply disabled", $("[data-fl-apply]").disabled);
        // Date: only the operators asked for.
        log("date ops", [...$("[data-fl-op]", pane(2)).options].map((o) => o.value).join(","));
        set($("[data-fl-op]", pane(2)), "before", "change");
        set($("[data-fl-a]", pane(2)), "2026-10-26");
        // Note: is not empty takes no value.
        set($("[data-fl-op]", pane(3)), "set", "change");
        log("no value", $("[data-fl-values]", pane(3)).hidden);
        $("[data-fl-apply]").click(); await wait(0);
        log("applied", json(S().f), panel.open);
        log("chips", chips());
        log("rows", shown());
        // None of: the list's other operator.
        $("[data-fl-open]").click(); await wait(0);
        set($("[data-fl-op]", pane(0)), "none", "change");
        log("kept picks", options(0));
        $("[data-fl-apply]").click(); await wait(0);
        log("none", json(S().f.status), "|", chips().split(" | ")[0], "| rows", shown());
        // A chip's ✕ removes that filter; Clear all, the rest.
        $$("[data-fl-remove]")[0].click(); await wait(0);
        log("removed", json(S().f), "focus", document.activeElement.hasAttribute("data-fl-edit"));
        $("[data-fl-clear]").click(); await wait(0);
        log("cleared", json(S().f), "| count hidden", $("[data-fl-count]").hidden, "| clear hidden", $("[data-fl-clear]").hidden, "| rows", shown());
        // Set from outside: the chips follow, and an is-empty condition reads back.
        root.__irender.set("f", { note: { empty: true }, date: { from: "2026-10-22", to: "2026-10-22" }, amount: 975 }); await wait(0);
        log("outside", chips(), "| rows", shown());
        root.__irender.set("f", { note: { empty: true } }); await wait(0);
        log("empty rows", shown());
        // A list's is-empty, set from outside, reads back with its values hidden.
        root.__irender.set("f", { status: { empty: false } }); await wait(0);
        $("[data-fl-open]").click(); await wait(0);
        log("list empty", chips(), "| op", $("[data-fl-op]", pane(0)).value, "| values hidden", $("[data-fl-options]", pane(0)).hidden, "| rows", shown());
        $("[data-fl-panel]").close();
        // Live: each change is written at once; Done closes.
        $("[data-fl-open]", G).click(); await wait(0);
        $$("[data-fl-pick]", pane(0, G))[0].click(); await wait(0);
        log("live", json(S().g), "| apply button", !!$("[data-fl-apply]", G), "| chips", chips(G));
        set($("[data-fl-op]", pane(3, G)), "contains", "change");
        set($("[data-fl-a]", pane(3, G)), "first");
        log("live text", json(S().g));
        $("[data-fl-reset]", G).click(); await wait(0);
        log("live reset", json(S().g));
        // Search shows only when a list is long.
        log("search hidden", $("[data-fl-search]", pane(0, G)).hidden);
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("filters", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "start Status:Paid | Amount:≥ $1,000 | count 2 | rows AC",
            "label Filters, 2 applied",
            // Opened on its chip, Amount shows its condition.
            "open true pane 1 op between a 1,000 b hidden false",
            "tabs Status1,Amount1,Date0,Note0 sections Order",
            "arrow Status1 0",
            // The field's values in the rows, with how many rows have each.
            "options Paid2*,Pending2,Refunded1",
            "draft badge 2 state [\"Paid\"]",
            "cancel {\"status\":[\"Paid\"],\"amount\":{\"from\":1000}}",
            "bad The first value can't be more than the second. apply disabled true",
            "fixed true apply disabled false",
            "date ops on,before,between",
            "no value true",
            "applied {\"status\":[\"Paid\",\"Pending\"],\"amount\":{\"from\":1000,\"to\":4000},\"date\":{\"lt\":\"2026-10-26\"},\"note\":{\"empty\":false}} false",
            "chips Status:Paid, Pending | Amount:$1,000 – $4,000 | Date:before Oct 26, 2026 | Note:is not empty",
            "rows E",
            "kept picks Paid2*,Pending2*,Refunded1",
            "none {\"not\":[\"Paid\",\"Pending\"]} | Status:not Paid, Pending | rows ",
            "removed {\"amount\":{\"from\":1000,\"to\":4000},\"date\":{\"lt\":\"2026-10-26\"},\"note\":{\"empty\":false}} focus true",
            "cleared {} | count hidden true | clear hidden true | rows ABCDE",
            "outside Amount:$975 | Date:Oct 22, 2026 | Note:is empty | rows ",
            "empty rows BD",
            "list empty Status:is not empty | op set | values hidden true | rows ABCDE",
            "live {\"status\":[\"Paid\"]} | apply button false | chips Status:Paid",
            "live text {\"status\":[\"Paid\"],\"note\":{\"contains\":\"first\"}}",
            "live reset {}",
            "search hidden true",
        ],
        "\n{result}"
    );
}

/// The tree: the ARIA tree keyboard, selecting, checking, searching, adding, deleting,
/// and moving with Alt and the arrows or by dragging, written back to `bindNodes`.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn tree_in_chrome() {
    let places = serde_json::json!([
        { "id": "in", "label": "India", "children": [
            { "id": "mh", "label": "Maharashtra", "children": [ { "id": "mumbai", "label": "Mumbai" }, { "id": "pune", "label": "Pune" } ] },
            { "id": "ka", "label": "Karnataka", "children": [ { "id": "blr", "label": "Bengaluru" } ] } ] },
        { "id": "de", "name": "Germany", "children": [ { "id": "berlin", "label": "Berlin" }, { "id": "munich", "label": "München" } ] },
        { "id": "jp", "label": "Japan" } ]);
    let doc = serde_json::json!({ "version": 1, "state": { "places": places, "picked": "pune", "place": null, "checked": [], "parent": "x" },
        "root": { "type": "ui-tree", "props": { "nodes": "{{ places }}", "bindNodes": "places", "bind": "picked", "bindNode": "place", "checkable": true,
            "bindChecked": "checked", "addable": true, "bindParent": "parent", "deletable": true, "movable": true, "search": true,
            "counts": true, "title": "Locations" } } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const T = document.querySelector("[data-i-x=ui-tree]");
      const $ = (s) => T.querySelector(s), $$ = (s) => [...T.querySelectorAll(s)];
      const li = (id) => $(`[data-tree-id="${id}"]`);
      // The visible tree as text: indented labels, [open/closed], *selected, checks.
      const shape = () => $$("[role=treeitem]").filter((x) => !x.closest("[hidden]")).map((x) =>
        "  ".repeat(x.getAttribute("aria-level") - 1) + x.querySelector("[data-tree-label]").textContent
        + ({ true: "-", false: "+" }[x.getAttribute("aria-expanded")] ?? "")
        + (x.getAttribute("aria-selected") === "true" ? "*" : "")
        + ({ true: " [x]", mixed: " [-]", false: "" }[x.getAttribute("aria-checked")])).join("|");
      const tree = (nodes = S().places) => nodes.map((n) => n.id + (n.children?.length ? `(${tree(n.children)})` : "")).join(",");
      const key = (k, o = {}) => document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true, ...o }));
      const focused = () => document.activeElement.getAttribute("data-tree-id");
      const events = [];
      for (const name of ["select", "add", "delete", "move"]) T.addEventListener(name, (e) => events.push(name + " " + JSON.stringify({ ...e.detail, node: e.detail.node?.label })));
      try {
        // Pune is selected, so its parents start open; one tab stop, on it.
        log("start", shape());
        log("node", S().place?.label);
        log("stop", $$("[tabindex='0']").map((x) => x.getAttribute("data-tree-id")).join(","), "counts", li("in").querySelector("[data-tree-count]").textContent);
        log("aria", $("[role=tree]").getAttribute("aria-label"), li("mh").getAttribute("aria-level"), li("mh").getAttribute("aria-posinset") + "/" + li("mh").getAttribute("aria-setsize"));
        // Keyboard: up to Mumbai, left to its parent, left closes it, down, right opens.
        li("pune").focus();
        key("ArrowUp"); log("up", focused());
        key("ArrowLeft"); log("left", focused());
        key("ArrowLeft"); await wait(0); log("closed", shape());
        key("ArrowDown"); key("ArrowRight"); await wait(0); log("opened", focused(), shape().split("|").slice(3).join("|"));
        key("ArrowRight"); log("into", focused());
        key("End"); log("end", focused()); key("Home"); log("home", focused());
        // Enter selects; the selection is in state.
        key("ArrowDown"); key("Enter"); await wait(0);
        log("select", S().picked, focused(), events.at(-1), S().place.label);
        // Space checks a node and everything under it; the parent shows some.
        li("mh").focus(); key(" "); await wait(0);
        log("check", JSON.stringify(S().checked), li("in").getAttribute("aria-checked"), $("[data-tree-checked]").textContent);
        li("blr").focus(); key(" "); await wait(0);
        log("all", JSON.stringify(S().checked), li("in").getAttribute("aria-checked"));
        key(" "); await wait(0);
        log("uncheck", JSON.stringify(S().checked), li("in").getAttribute("aria-checked"), li("ka").getAttribute("aria-checked"));
        // Search: matches and their parents, the match marked; Escape clears.
        const box = $("[data-tree-search]");
        box.value = "chen"; box.dispatchEvent(new Event("input", { bubbles: true })); await wait(0);
        log("search", shape(), li("munich").querySelector("mark").textContent);
        box.value = "zzz"; box.dispatchEvent(new Event("input", { bubbles: true })); await wait(0);
        log("none", $("[data-tree-empty]").hidden, $("[data-tree-empty]").textContent);
        box.focus(); box.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true })); await wait(0);
        log("cleared", box.value === "", $$("[role=treeitem]").length);
        // Add: a name box where the node will go; Enter adds it, Escape cancels.
        $("[data-tree-add-root]").click(); await wait(0);
        const name = () => $("[data-tree-name]");
        log("add top", JSON.stringify(S().parent), document.activeElement === name(), name().getAttribute("aria-label"));
        name().value = "Spain"; name().dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true })); await wait(0);
        const spain = S().places.at(-1);
        log("added", S().places.length, spain.label, typeof spain.id, focused() === spain.id, events.at(-1));
        li("de").querySelector("[data-tree-add]").click(); await wait(0);
        log("add under", S().parent, li("de").getAttribute("aria-expanded"), name().closest("ul").parentElement.getAttribute("data-tree-id"), name().getAttribute("aria-label"));
        name().value = "Hamburg"; name().dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true })); await wait(0);
        log("cancel", !name(), tree().replace(spain.id, "new"), focused());
        // Spain goes again, so the moves below start from the same tree.
        root.__irender.set("places", S().places.slice(0, -1)); await wait(0);
        // Move with Alt: Japan up above Germany, then into India's last place? Right: into Germany.
        li("jp").focus(); key("ArrowUp", { altKey: true }); await wait(0);
        log("alt up", tree(), focused(), events.at(-1));
        key("ArrowRight", { altKey: true }); await wait(0);
        log("alt right", tree(), events.at(-1));
        key("ArrowLeft", { altKey: true }); await wait(0);
        log("alt left", tree(), events.at(-1));
        key("ArrowDown", { altKey: true }); await wait(0);
        log("alt down", tree());
        // Dragging Berlin onto Karnataka's middle puts it inside, at the end.
        const drag = (from, to, where) => {
          const dt = new DataTransfer();
          const row = to.querySelector("[data-tree-row]").getBoundingClientRect();
          const y = row.top + row.height * where;
          from.querySelector("[data-tree-row]").dispatchEvent(new DragEvent("dragstart", { bubbles: true, dataTransfer: dt }));
          to.querySelector("[data-tree-label]").dispatchEvent(new DragEvent("dragover", { bubbles: true, cancelable: true, dataTransfer: dt, clientY: y }));
          const shown = to.getAttribute("data-drop");
          to.querySelector("[data-tree-label]").dispatchEvent(new DragEvent("drop", { bubbles: true, cancelable: true, dataTransfer: dt, clientY: y }));
          from.querySelector("[data-tree-row]")?.dispatchEvent(new DragEvent("dragend", { bubbles: true, dataTransfer: dt }));
          return shown;
        };
        log("drag inside", drag(li("berlin"), li("ka"), 0.5)); await wait(0);
        log("dragged", tree(), events.at(-1));
        log("drag before", drag(li("munich"), li("in"), 0.1)); await wait(0);
        log("dragged", tree());
        // Not into itself or below itself.
        const before = tree();
        drag(li("in"), li("mumbai"), 0.5); await wait(0);
        log("own child", tree() === before);
        // By touch: held still, a row is picked up, then dropped into Germany, and back
        // before India.
        const touch = (t, type, x, y) => t.dispatchEvent(new PointerEvent(type, { bubbles: true, cancelable: true, pointerId: 5, pointerType: "touch", isPrimary: true, clientX: x, clientY: y }));
        const tdrag = async (from, to, where) => {
          const a = from.querySelector("[data-tree-row]"), r = a.getBoundingClientRect(), t = to.querySelector("[data-tree-row]").getBoundingClientRect();
          touch(a, "pointerdown", r.left + 40, r.top + 5); await wait(350);
          const held = a.hasAttribute("data-touch-dragging");
          touch(a, "pointermove", t.left + 60, t.top + t.height * where);
          const shown = to.getAttribute("data-drop");
          touch(a, "pointerup", t.left + 60, t.top + t.height * where); await wait(0);
          return `${held} ${shown}`;
        };
        log("touch inside", await tdrag(li("munich"), li("de"), 0.5), tree());
        log("touch before", await tdrag(li("munich"), li("in"), 0.1), tree());
        // The tap that ended the drag isn't a click; a later one is.
        await wait(150);
        // Delete: a node's button, then the Delete key on the focused node.
        li("jp").querySelector("[data-tree-delete]").click(); await wait(0);
        log("deleted", tree(), events.at(-1), "focus", focused());
        li("blr").focus(); key("Delete"); await wait(0);
        log("delete key", tree(), focused());
        // The checked (Maharashtra and under it, from before), with the bulk bar: the
        // selected node goes too.
        $("[data-tree-delete-checked]").click(); await wait(0);
        log("selection", JSON.stringify(S().picked));
        log("bulk", tree(), JSON.stringify(S().checked), $("[data-tree-bulk]").hidden, events.at(-1));
        // From state: the tree follows.
        root.__irender.set("places", []); await wait(0);
        log("empty", $("[role=tree]").hidden, $("[data-tree-empty]").hidden, $("[data-tree-empty]").textContent);
        log("live", $("[data-tree-live]").getAttribute("role"));
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("tree", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            // Pune is selected, so its parents start open.
            "start India-|  Maharashtra-|    Mumbai|    Pune*|  Karnataka+|Germany+|Japan",
            "node Pune",
            "stop pune counts 5",
            "aria Locations 2 1/2",
            "up mumbai",
            "left mh",
            "closed India-|  Maharashtra+|  Karnataka+|Germany+|Japan",
            "opened ka     Bengaluru|Germany+|Japan",
            "into blr",
            "end jp",
            "home in",
            "select mh mh select {\"id\":\"mh\",\"node\":\"Maharashtra\"} Maharashtra",
            // Checking a node checks the nodes under it; its parent shows some.
            "check [\"mh\",\"mumbai\",\"pune\"] mixed 3 selected",
            "all [\"mh\",\"mumbai\",\"pune\",\"blr\",\"ka\",\"in\"] true",
            "uncheck [\"mh\",\"mumbai\",\"pune\"] mixed false",
            "search Germany-|  München chen",
            "none false No matches.",
            "cleared true 10",
            // Add: a name box where the node will go; Enter adds it, Escape cancels.
            "add top null true New item's name",
            "added 4 Spain string true add {\"parent\":null,\"node\":\"Spain\"}",
            "add under de true de New item under Germany",
            "cancel true in(mh(mumbai,pune),ka(blr)),de(berlin,munich),jp,new de",
            // Alt and the arrows: up, into the node above, out after the parent, down.
            "alt up in(mh(mumbai,pune),ka(blr)),jp,de(berlin,munich) jp move {\"id\":\"jp\",\"parent\":null,\"index\":1}",
            "alt right in(mh(mumbai,pune),ka(blr),jp),de(berlin,munich) move {\"id\":\"jp\",\"parent\":\"in\",\"index\":2}",
            "alt left in(mh(mumbai,pune),ka(blr)),jp,de(berlin,munich) move {\"id\":\"jp\",\"parent\":null,\"index\":1}",
            "alt down in(mh(mumbai,pune),ka(blr)),de(berlin,munich),jp",
            // Dragging onto the middle of a node puts it inside, near the top edge before it.
            "drag inside inside",
            "dragged in(mh(mumbai,pune),ka(blr,berlin)),de(munich),jp move {\"id\":\"berlin\",\"parent\":\"ka\",\"index\":1}",
            "drag before before",
            "dragged munich,in(mh(mumbai,pune),ka(blr,berlin)),de,jp",
            "own child true",
            "touch inside true inside in(mh(mumbai,pune),ka(blr,berlin)),de(munich),jp",
            "touch before true before munich,in(mh(mumbai,pune),ka(blr,berlin)),de,jp",
            // Focus goes to the next node shown, or the one before.
            "deleted munich,in(mh(mumbai,pune),ka(blr,berlin)),de delete {\"ids\":[\"jp\"]} focus de",
            "delete key munich,in(mh(mumbai,pune),ka(berlin)),de berlin",
            "selection null",
            "bulk munich,in(ka(berlin)),de [] true delete {\"ids\":[\"mh\"]}",
            "empty true false Nothing here yet.",
            "live status",
        ],
        "\n{result}"
    );
}

/// An upload API: `POST /upload` answers `{ id, url, folder }` (the `folder` field it was
/// sent), `POST /fail` answers 500 with `{ error }`, and `GET /files/…` serves the
/// examples' spreadsheet.
fn upload_server() -> u16 {
    use std::io::Write;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let xlsx = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/blocks/files/expenses.xlsx")).unwrap();
    std::thread::spawn(move || {
        let mut n = 0;
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let request = read_request(&mut stream);
            let cors = "Access-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: *\r\nAccess-Control-Allow-Methods: POST, GET";
            let json = |stream: &mut std::net::TcpStream, status: &str, body: String| {
                let _ = write!(stream, "HTTP/1.1 {status}\r\n{cors}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            };
            if request.starts_with("OPTIONS") {
                let _ = write!(stream, "HTTP/1.1 204 No Content\r\n{cors}\r\nConnection: close\r\n\r\n");
            } else if request.starts_with("GET /files/") {
                let _ = write!(stream, "HTTP/1.1 200 OK\r\n{cors}\r\nContent-Type: application/vnd.openxmlformats-officedocument.spreadsheetml.sheet\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", xlsx.len());
                let _ = stream.write_all(&xlsx);
            } else if request.starts_with("POST /upload") {
                n += 1;
                let name = request.split("filename=\"").nth(1).and_then(|s| s.split('"').next()).unwrap_or("file").to_string();
                let folder = request.split("name=\"folder\"\r\n\r\n").nth(1).and_then(|s| s.split("\r\n").next()).unwrap_or("").to_string();
                json(&mut stream, "200 OK", serde_json::json!({ "id": format!("s{n}"), "url": format!("http://127.0.0.1:{port}/files/s{n}/{name}"), "folder": folder }).to_string());
            } else if request.starts_with("POST /fail") {
                json(&mut stream, "500 Internal Server Error", r#"{"error":"Disk full"}"#.into());
            } else {
                let _ = write!(stream, "HTTP/1.1 404 Not Found\r\n{cors}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            }
        }
    });
    port
}

/// File upload: checks against accept, size and count; uploads with the server's answer
/// merged in; errors and retry; removing; the preview of CSV, text and .xlsx; files kept
/// in the browser without a `url`; and Download all as a zip.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn file_upload_in_chrome() {
    let port = upload_server();
    let at = |p: &str| format!("http://127.0.0.1:{port}{p}");
    let doc = serde_json::json!({ "version": 1,
        "state": { "files": [{ "id": "e1", "name": "expenses.xlsx", "size": 2078, "url": at("/files/e1/expenses.xlsx") }], "bad": [], "kept": [], "folder": "claims" },
        "root": { "type": "stack", "children": [
            { "type": "ui-file-upload", "attrs": { "id": "main" }, "props": { "bind": "files", "url": at("/upload"), "params": { "folder": "{{ folder }}" },
                "accept": ".csv,.txt,.xlsx,image/*", "maxSize": "1 KB", "maxFiles": 4, "hint": "CSV or text, up to 1 KB", "zipName": "claim" } },
            { "type": "ui-file-upload", "attrs": { "id": "bad" }, "props": { "bind": "bad", "url": at("/fail"), "view": "list" } },
            { "type": "ui-file-upload", "attrs": { "id": "kept" }, "props": { "bind": "kept", "multiple": false } } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const tick = () => new Promise((r) => { const c = new MessageChannel(); c.port1.onmessage = r; c.port2.postMessage(0); });
      const until = async (f) => { for (let i = 0; i < 2e6 && !f(); i++) await (i % 50 ? tick() : wait(10)); };
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const U = (id) => document.getElementById(id);
      const $ = (s, from) => from.querySelector(s), $$ = (s, from) => [...from.querySelectorAll(s)];
      const pick = (el, files) => {
        const dt = new DataTransfer();
        files.forEach(([name, text, type]) => dt.items.add(new File([text], name, { type: type ?? "" })));
        const input = $("[data-up-input]", el);
        input.files = dt.files;
        input.dispatchEvent(new Event("change", { bubbles: true }));
      };
      const cards = (el) => $$("[data-up-file]", el).map((c) => c.querySelector("[data-up-name]").textContent + ":" + c.getAttribute("data-kind") + ":" + c.getAttribute("data-status")).join(", ");
      const events = [];
      for (const name of ["add", "uploaded", "error", "remove", "download"]) document.addEventListener(name, (e) => e.target.closest?.("[data-i-x=ui-file-upload]") && events.push(name));
      try {
        const M = U("main");
        log("start", cards(M), "|", $("[data-up-total]", M).textContent, "| zone", $("[data-up-input]", M).getAttribute("accept"), $("[data-up-input]", M).getAttribute("aria-describedby") !== null);
        // Two fine, one of a type not accepted, one too big.
        pick(M, [["a.csv", "x,y\n1,\"two, three\"\n", "text/csv"], ["b.txt", "hello"], ["c.exe", "MZ"], ["d.csv", "z".repeat(2000)]]);
        log("errors", $$("[data-up-errors] li", M).map((l) => l.textContent).join(" | "));
        log("adding", S().files.map((f) => f.name + ":" + f.status).join(", "));
        await until(() => S().files.every((f) => f.status !== "uploading"));
        await wait(0);
        const a = S().files[1];
        // The server numbers them as they arrive, which can be either order: any `s` id is it.
        log("uploaded", S().files.map((f) => f.id.replace(/^s\d+$/, "s*") + ":" + f.status).join(", "), a.folder, a.url.endsWith("/files/s1/a.csv") || a.url.endsWith("/files/s2/a.csv"), cards(M));
        // At most four: one more fits.
        pick(M, [["e.txt", "e"], ["f.txt", "f"]]);
        log("full", S().files.length, $$("[data-up-errors] li", M).map((l) => l.textContent).join(" | "));
        await until(() => S().files.every((f) => f.status !== "uploading"));
        // Preview: the .xlsx from the server, then the CSV with its quoted comma.
        $("[data-up-file] [data-up-open]", M).click();
        const D = $("[data-up-preview]", M);
        await until(() => $("[data-up-body] table", D));
        log("xlsx", D.open, $("[data-up-title]", D).textContent, $("[data-up-count]", D).textContent, $$("[data-up-body] tr", D).length, $$("[data-up-body] tr", D)[2].textContent);
        $("[data-up-next]", D).click();
        await until(() => $("[data-up-title]", D).textContent === "a.csv" && $("[data-up-body] table", D));
        log("csv", $$("[data-up-body] th", D).map((t) => t.textContent).join("|"), $$("[data-up-body] td", D).map((t) => t.textContent).join("|"));
        D.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true }));
        await until(() => $("[data-up-body] pre", D));
        log("text", $("[data-up-title]", D).textContent, $("[data-up-body] pre", D).textContent, $("[data-up-save]", D).getAttribute("download"));
        D.close();
        await until(() => document.activeElement.closest?.("[data-up-file]"));
        log("closed", document.activeElement.closest("[data-up-file]")?.querySelector("[data-up-name]").textContent);
        // Remove: from state, with focus on the next file.
        $$("[data-up-remove]", M)[2].click(); await wait(0);
        log("removed", S().files.map((f) => f.name).join(","), events.at(-1), document.activeElement.closest("[data-up-file]")?.querySelector("[data-up-name]").textContent);
        // Download all: a zip of the files, named twice-over-safe.
        const clicks = [];
        HTMLAnchorElement.prototype.click = function () { clicks.push([this.download, this.href]); };
        $("[data-up-zip]", M).click();
        await until(() => clicks.length);
        const bytes = new Uint8Array(await (await fetch(clicks[0][1])).arrayBuffer());
        const text = new TextDecoder().decode(bytes);
        log("zip", clicks[0][0], String.fromCharCode(bytes[0], bytes[1]), ["expenses.xlsx", "a.csv", "e.txt"].map((n) => text.split(n).length - 1).join(","), events.at(-1));
        // A server error: shown on the file, with a retry.
        const B = U("bad");
        pick(B, [["g.txt", "g"]]);
        await until(() => S().bad[0]?.status === "error");
        await wait(0);
        log("error", S().bad[0].error, $("[data-up-error]", B).textContent, !!$("[data-up-retry]", B), $("[data-up-files]", B).getAttribute("data-view"));
        $("[data-up-retry]", B).click();
        await until(() => S().bad[0]?.status === "error" && events.filter((e) => e === "error").length === 2);
        log("retried", events.filter((e) => e === "error").length);
        // No url: kept in the browser, one at a time, still previewable.
        const K = U("kept");
        pick(K, [["h.txt", "first"], ["i.txt", "second"]]);
        await wait(0);
        log("kept", S().kept.map((f) => f.name + ":" + f.status).join(","), $("[data-up-input]", K).multiple);
        pick(K, [["j.txt", "third"]]);
        await wait(0);
        $("[data-up-open]", K).click();
        await until(() => $("[data-up-body] pre", K));
        log("replaced", S().kept.map((f) => f.name).join(","), $("[data-up-body] pre", K).textContent);
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("upload", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "start expenses.xlsx:sheet:done | 1 file, 2 KB | zone .csv,.txt,.xlsx,image/* true",
            // Not an accepted type, and over `maxSize`: refused, with the reason.
            "errors c.exe isn't a type that can be added. | d.csv is over the 1 KB limit.",
            "adding expenses.xlsx:undefined, a.csv:uploading, b.txt:uploading",
            // The server's answer is merged in: its ids, and the `folder` field it was sent.
            "uploaded e1:undefined, s*:done, s*:done claims true expenses.xlsx:sheet:done, a.csv:sheet:done, b.txt:text:done",
            "full 4 f.txt wasn't added: at most 4 files.",
            "xlsx true expenses.xlsx 1 of 4 6 2026-11-03Taj Lands EndHotel12600",
            "csv x|y 1|two, three",
            "text b.txt hello b.txt",
            "closed b.txt",
            "removed expenses.xlsx,a.csv,e.txt remove e.txt",
            // Each name twice: its local header and the central directory.
            "zip claim.zip PK 2,2,2 download",
            "error Disk full Disk full true list",
            "retried 2",
            // No `url`: kept in the browser; `multiple: false` keeps one.
            "kept h.txt:local false",
            "replaced j.txt third",
        ],
        "\n{result}"
    );
}

/// Status badge, progress ring, rating, password checklist and timeline, following
/// state: tones and labels, the ring's value, picking and clearing a rating, rules
/// ticked as a password is typed, and timeline entries added from state.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn status_rating_password_and_timeline_in_chrome() {
    let doc = serde_json::json!({ "version": 1,
        "state": { "status": "In review", "done": 40, "stars": 2, "avg": 3.5, "pw": "", "ok": false,
            "history": [ { "id": 1, "who": "Asha Rao", "action": "opened it", "when": "Mon" } ] },
        "root": { "type": "stack", "children": [
            { "type": "ui-status-badge", "attrs": { "id": "badge" }, "props": { "status": "{{ status }}", "tones": { "Shipped": "success" }, "labels": { "in_progress": "In progress" } } },
            { "type": "ui-progress-ring", "attrs": { "id": "ring" }, "props": { "value": "{{ done }}", "label": "Progress" } },
            { "type": "ui-rating", "attrs": { "id": "pick" }, "props": { "bind": "stars", "label": "Your rating" } },
            { "type": "ui-rating", "attrs": { "id": "shown" }, "props": { "value": "{{ avg }}", "label": "Average" } },
            { "type": "ui-password-checklist", "attrs": { "id": "pw" }, "props": { "value": "{{ pw }}", "bindValid": "ok", "rules": [ { "length": 10 }, "upper", "number", { "label": "No spaces", "pattern": "^\\S*$" } ] } },
            { "type": "ui-timeline", "attrs": { "id": "tl" }, "props": { "items": "{{ history }}" }, "children": [
                { "type": "ui-timeline-item", "props": { "action": "Created from an email", "when": "Sun", "tone": "info" } } ] } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state, set = (k, v) => root.__irender.set(k, v);
      const $ = (s) => document.querySelector(s), $$ = (s) => [...document.querySelectorAll(s)];
      try {
        const badge = $("#badge");
        log("badge", badge.getAttribute("data-tone"), badge.textContent);
        for (const s of ["Shipped", "BLOCKED", "in_progress", "Something else"]) {
          set("status", s); await wait(0);
          log("badge", s, badge.getAttribute("data-tone"), badge.textContent);
        }
        const ring = $("#ring");
        log("ring", ring.getAttribute("aria-valuenow"), ring.textContent, getComputedStyle(ring).getPropertyValue("--ui-value").trim(), ring.getAttribute("aria-label"));
        set("done", 75); await wait(0);
        // (The arc eases there; its length comes from this.)
        log("ring", ring.getAttribute("aria-valuenow"), ring.textContent, getComputedStyle(ring).getPropertyValue("--ui-value").trim());
        // Picking: a click on the fourth star, then the keyboard, then clearing.
        const pick = $("#pick"), stars = () => $$("#pick label").map((l) => (l.hasAttribute("data-on") ? "*" : ".")).join("");
        log("rating", stars(), pick.getAttribute("role"), $$("#pick input").map((i) => i.getAttribute("aria-label")).join(","));
        $$("#pick input")[3].click(); await wait(0);
        log("clicked", S().stars, stars());
        $$("#pick input")[3].dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
        $$("#pick input")[3].click(); await wait(0);
        log("cleared", S().stars, stars());
        set("stars", 5); await wait(0);
        log("from state", stars(), $$("#pick input")[4].checked);
        const shown = $("#shown");
        log("shown", shown.getAttribute("role"), shown.getAttribute("aria-label"), getComputedStyle(shown).getPropertyValue("--ui-value").trim(), shown.textContent.trim());
        // The checklist as a password is typed.
        const rules = () => $$("#pw li").map((l) => (l.hasAttribute("data-met") ? "✓" : "·")).join("");
        log("pw", rules(), $("#pw").getAttribute("data-strength"), $("#pw [data-strength-text]").textContent, S().ok, $$("#pw li")[0].textContent);
        set("pw", "Hello world"); await wait(0);
        log("pw", rules(), $("#pw").getAttribute("data-strength"), $("#pw [data-strength-text]").textContent, S().ok);
        set("pw", "Helloworld42"); await wait(0);
        log("pw", rules(), $("#pw").getAttribute("data-strength"), $("#pw [data-strength-text]").textContent, S().ok, $$("#pw li")[1].textContent);
        // Timeline: the child first, then the entries from state; a new one arrives.
        // The mark as shown: a dot, the initials, or (a name from state) its first letter.
        const mark = (m) => (m.hasAttribute("data-dot") ? "•" : m.querySelector("[data-first]") ? m.textContent[0] + "…" : m.textContent);
        const entries = () => $$("#tl > li, #tl [data-entries] > li").map((li) => mark(li.querySelector("[data-mark]")) + " " + li.querySelector("[data-line]").textContent).join(" | ");
        log("timeline", entries(), $("#tl").getAttribute("aria-label"));
        set("history", [...S().history, { id: 2, who: "Ravi", action: "changed the status", when: "Tue", changes: [{ field: "Status", from: "Open", to: "Done" }], comment: "Fixed in 2.4" }]); await wait(0);
        log("timeline", entries());
        const last = $$("#tl [data-entries] > li").at(-1);
        log("changes", [...last.querySelectorAll("[data-changes] li span")].map((s) => s.textContent).join(" "), last.querySelector("blockquote")?.textContent, $$("#tl blockquote").length);
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("five", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "badge warning In review",
            // From `tones`, the built-in list (any casing), `labels`, and neutral for the rest.
            "badge Shipped success Shipped",
            "badge BLOCKED danger BLOCKED",
            "badge in_progress primary In progress",
            "badge Something else neutral Something else",
            "ring 40 40% 40 Progress",
            "ring 75 75% 75",
            "rating **... radiogroup 1 star,2 stars,3 stars,4 stars,5 stars",
            // A click picks; a click on the chosen star clears it; state moves it.
            "clicked 4 ****.",
            "cleared 0 .....",
            "from state ***** true",
            "shown img Average: 3.5 out of 5 70 3.5",
            "pw ···· 0 Enter a password false At least 10 characters, not met",
            // Length 10 and no spaces fail, then all four pass, and bindValid follows.
            "pw ✓✓·· 2 Strength: Fair false",
            "pw ✓✓✓✓ 4 Strength: Strong true An uppercase letter, met",
            // The child entry, then those from state; a name from state shows its first letter.
            "timeline • Created from an emailSun | A… Asha Rao opened itMon Activity",
            "timeline • Created from an emailSun | A… Asha Rao opened itMon | R… Ravi changed the statusTue",
            "changes Status Open → Done Fixed in 2.4 1",
        ],
        "\n{result}"
    );
}

/// The kanban board: arrow keys between cards, Alt and the arrows to move one, dragging,
/// opening, adding a card by name, folding a column, the limit, and state.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn kanban_in_chrome() {
    let doc = serde_json::json!({ "version": 1,
        "state": { "open": null, "tasks": [
            { "id": "a", "column": "todo", "title": "Alpha", "priority": "High", "assignee": "Asha Rao", "tags": ["UI"] },
            { "id": "b", "column": "todo", "title": "Bravo" },
            { "id": "c", "column": "doing", "title": "Charlie", "due": "Fri" },
            { "id": "d", "column": "done", "title": "Delta" } ] },
        "root": { "type": "ui-kanban", "props": {
            "columns": [ { "id": "todo", "title": "To do" }, { "id": "doing", "title": "Doing", "limit": 1 }, { "id": "done", "title": "Done", "tone": "success" } ],
            "cards": "{{ tasks }}", "bindCards": "tasks", "bindCard": "open", "addable": true, "collapsible": true, "label": "Sprint" } } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const K = document.querySelector("[data-i-x=ui-kanban]");
      const $ = (s) => K.querySelector(s), $$ = (s) => [...K.querySelectorAll(s)];
      const card = (id) => $(`[data-kb-card="${id}"]`);
      const board = () => $$("[data-kb-column]").map((c) => c.getAttribute("data-kb-column") + ":" + $$(`[data-kb-column="${c.getAttribute("data-kb-column")}"] [data-kb-card]`).map((x) => x.getAttribute("data-kb-card")).join("")).join(" ");
      const tasks = () => S().tasks.map((t) => t.id + t.column[0]).join(",");
      const key = (k, o = {}) => document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true, ...o }));
      const focused = () => document.activeElement.getAttribute("data-kb-card");
      const events = [];
      for (const n of ["move", "add", "open"]) K.addEventListener(n, (e) => events.push(n + " " + JSON.stringify({ ...e.detail, card: e.detail.card?.title })));
      try {
        log("start", board(), "|", $$("[data-kb-count]").map((c) => c.textContent).join(","), "| over", $$("[data-over]").map((c) => c.getAttribute("data-kb-column")).join(","));
        log("aria", K.getAttribute("aria-label"), $("[data-kb-column]").getAttribute("aria-label"), "|", card("a").getAttribute("aria-label"), "| stops", $$("[data-kb-card][tabindex='0']").length);
        // Arrow keys move focus: down, right (to the same height or the last), left.
        card("a").focus();
        key("ArrowDown"); log("down", focused());
        key("ArrowRight"); log("right", focused());
        key("ArrowRight"); log("right", focused());
        key("ArrowLeft"); key("ArrowLeft"); log("left", focused());
        // Enter opens.
        key("Enter"); await wait(0);
        log("open", S().open?.title, events.at(-1));
        // Alt and the arrows move it: up in its column, right to the next column, down, left.
        key("ArrowUp", { altKey: true }); await wait(0);
        log("alt up", tasks(), board(), focused(), events.at(-1));
        // The open card follows its moves.
        key("ArrowRight", { altKey: true }); await wait(0);
        log("alt right", board(), "| over", $$("[data-over]").map((c) => c.getAttribute("data-kb-column")).join(","), $("[data-kb-column=doing] [data-kb-count]").textContent, "| open in", S().open.column);
        key("ArrowDown", { altKey: true }); await wait(0);
        log("alt down", board(), focused());
        key("ArrowRight", { altKey: true }); key("ArrowRight", { altKey: true }); await wait(0);
        log("alt right twice", board(), "| edge stays", focused());
        // Dragging Alpha above Delta, in Done.
        const dt = new DataTransfer();
        const target = card("d").getBoundingClientRect();
        card("a").dispatchEvent(new DragEvent("dragstart", { bubbles: true, dataTransfer: dt }));
        card("d").dispatchEvent(new DragEvent("dragover", { bubbles: true, cancelable: true, dataTransfer: dt, clientY: target.top + 2 }));
        log("drop line", !!$("[data-kb-column=done] [data-kb-drop]"), $("[data-kb-column=done]").hasAttribute("data-over-drop"));
        card("d").dispatchEvent(new DragEvent("drop", { bubbles: true, cancelable: true, dataTransfer: dt, clientY: target.top + 2 }));
        K.querySelector("[data-kb-board]").dispatchEvent(new DragEvent("dragend", { bubbles: true, dataTransfer: dt }));
        await wait(0);
        log("dragged", board(), tasks(), events.at(-1), !!$("[data-kb-drop]"));
        // By touch: held still, b is picked up; over Charlie's lower half it goes after it.
        const touch = (t, type, x, y) => t.dispatchEvent(new PointerEvent(type, { bubbles: true, cancelable: true, pointerId: 5, pointerType: "touch", isPrimary: true, clientX: x, clientY: y }));
        const b = card("b").getBoundingClientRect(), c = card("c").getBoundingClientRect();
        touch(card("b"), "pointerdown", b.left + 20, b.top + 10); await wait(350);
        log("held", card("b").hasAttribute("data-dragging"), $$("[data-touch-ghost]").length);
        touch(card("b"), "pointermove", c.left + 20, c.top + 5); touch(card("b"), "pointermove", c.left + 20, c.bottom - 4);
        log("touch line", !!$("[data-kb-column=doing] [data-kb-drop]"), $("[data-kb-column=doing] [data-kb-drop]")?.previousElementSibling?.getAttribute("data-kb-card"));
        touch(card("b"), "pointerup", c.left + 20, c.bottom - 4); await wait(0);
        log("touched", board(), events.at(-1), $$("[data-touch-ghost], [data-dragging], [data-kb-drop]").length);
        // A quick swipe scrolls: nothing moves.
        const c2 = card("c").getBoundingClientRect();
        touch(card("c"), "pointerdown", c2.left + 20, c2.top + 10); touch(card("c"), "pointermove", c2.left + 60, c2.top + 10); await wait(350);
        touch(card("c"), "pointerup", c2.left + 60, c2.top + 10); await wait(0);
        log("swiped", board(), $$("[data-touch-ghost]").length);
        // Adding: a name box, Enter adds at the column's end; Escape cancels.
        $("[data-kb-column=todo] [data-kb-add]").click(); await wait(0);
        const box = $("[data-kb-name]");
        log("box", document.activeElement === box, box.getAttribute("aria-label"));
        box.value = "Echo"; box.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true })); await wait(0);
        const echo = S().tasks.find((t) => t.title === "Echo");
        log("added", echo.column, S().tasks.indexOf(echo), focused() === echo.id, events.at(-1).startsWith('add {"card":"Echo"}'));
        $("[data-kb-column=done] [data-kb-add]").click(); await wait(0);
        $("[data-kb-name]").dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true })); await wait(0);
        log("cancel", !$("[data-kb-name]"), document.activeElement.matches("[data-kb-column=done] [data-kb-add]"));
        // Folding a column hides its cards, and the keys skip it.
        $("[data-kb-column=doing] [data-kb-fold]").click(); await wait(0);
        log("folded", $("[data-kb-column=doing]").hasAttribute("data-collapsed"), $("[data-kb-column=doing] [data-kb-fold]").getAttribute("aria-expanded"));
        // From state: the board follows.
        root.__irender.set("tasks", S().tasks.filter((t) => t.column !== "done")); await wait(0);
        log("state", board().replace(echo.id, "+"), $("[data-kb-column=done] [data-kb-empty]")?.textContent);
        log("live", $("[data-kb-live]").getAttribute("aria-live"));
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("kanban", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "start todo:ab doing:c done:d | 2,1 / 1,1 | over ",
            "aria Sprint To do, 2 cards | Alpha, High priority, assigned to Asha Rao | stops 1",
            // Arrow keys move focus: across columns to the same height, or the nearest card.
            "down b",
            "right c",
            "right d",
            "left a",
            "open Alpha open {\"card\":\"Alpha\"}",
            // Alt and the arrows move the card: not past the top, over the limit, not past the last column.
            "alt up at,bt,cd,dd todo:ab doing:c done:d a open {\"card\":\"Alpha\"}",
            "alt right todo:b doing:ac done:d | over doing 2 / 1 | open in doing",
            "alt down todo:b doing:ca done:d a",
            "alt right twice todo:b doing:c done:da | edge stays a",
            "drop line true true",
            // Dropped near Delta's top edge: before it.
            "dragged todo:b doing:c done:ad bt,cd,ad,dd move {\"id\":\"a\",\"from\":\"done\",\"to\":\"done\",\"index\":0} false",
            "held true 1",
            "touch line true c",
            "touched todo: doing:cb done:ad move {\"id\":\"b\",\"from\":\"todo\",\"to\":\"doing\",\"index\":1} 0",
            "swiped todo: doing:cb done:ad 0",
            "box true New card in To do",
            // Enter adds at the column's end; Escape puts focus back on the button.
            "added todo 4 true true",
            "cancel true true",
            "folded true false",
            "state todo:+ doing:cb done: No cards",
            "live polite",
        ],
        "\n{result}"
    );
}

/// The event calendar: a month grid with chips and `+N more`, events over several days,
/// clicks on events and days, the keyboard, the week of hours with overlaps side by
/// side, the list by day, the toolbar, and events from state.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn event_calendar_in_chrome() {
    let doc = serde_json::json!({ "version": 1,
        "state": { "ev": null, "day": null, "view": null, "events": [
            { "id": 1, "title": "Planning", "start": "2026-11-02T10:00", "end": "2026-11-02T11:30", "category": "Meetings" },
            { "id": 2, "title": "Maintenance", "start": "2026-11-04", "end": "2026-11-06", "category": "Works" },
            { "id": 3, "title": "Standup", "start": "2026-11-04T09:30", "end": "2026-11-04T09:45", "category": "Meetings" },
            { "id": 4, "title": "Audit", "start": "2026-11-04T11:00", "end": "2026-11-04T13:00" },
            { "id": 5, "title": "Drill", "start": "2026-11-04T11:30", "end": "2026-11-04T12:30", "tone": "danger" },
            { "id": 6, "title": "Night shift", "start": "2026-11-11T22:00", "end": "2026-11-12T06:00" } ] },
        "root": { "type": "ui-event-calendar", "props": { "events": "{{ events }}", "categories": { "Meetings": "info", "Works": "warning" },
            "bindEvent": "ev", "bindDay": "day", "bindView": "view", "today": "2026-11-04", "date": "2026-11-04", "locale": "en-GB", "label": "Plant calendar" } } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      // Engines write hours their own way in en-GB ("9:30", "09:30"): without the leading zero.
      const out = []; const log = (...a) => out.push(a.join(" ").replace(/\b0(\d:\d\d)/g, "$1"));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const C = document.querySelector("[data-i-x=ui-event-calendar]");
      const $ = (s) => C.querySelector(s), $$ = (s) => [...C.querySelectorAll(s)];
      const day = (d) => $(`[data-ev-day="${d}"]`);
      const chips = (d) => [...day(d).querySelectorAll("[data-ev-event], [data-ev-more]")].map((c) => c.textContent + (c.hasAttribute("data-from-before") ? "‹" : "") + (c.hasAttribute("data-on-after") ? "›" : "")).join(", ");
      const key = (k) => document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true }));
      const focused = () => document.activeElement.getAttribute("data-ev-day");
      const events = [];
      for (const n of ["open", "day", "navigate"]) C.addEventListener(n, (e) => events.push(n + " " + JSON.stringify({ ...e.detail, event: e.detail.event?.title })));
      try {
        log("month", $("[data-ev-title]").textContent, $$("[data-ev-week]").length, "weeks |", $$("[role=columnheader]").map((h) => h.textContent).join(","), "| today", $("[data-today]").getAttribute("data-ev-day"));
        log("legend", $$("[data-ev-legend] > span").map((s) => s.textContent + ":" + s.getAttribute("data-tone")).join(","), "| region", C.getAttribute("aria-label"));
        log("4 Nov", chips("2026-11-04"), "|", day("2026-11-04").getAttribute("aria-label"));
        log("5 Nov", chips("2026-11-05"), "| 6 Nov", chips("2026-11-06"), "| 12 Nov", chips("2026-11-12"));
        log("tones", [...day("2026-11-04").querySelectorAll("[data-ev-event]")].map((c) => c.getAttribute("data-tone")).join(","));
        log("outside", day("2026-10-26").hasAttribute("data-outside"), day("2026-11-01").hasAttribute("data-outside"));
        // A click on an event opens it; on a day, picks it.
        day("2026-11-02").querySelector("[data-ev-event]").click(); await wait(0);
        log("open", S().ev?.title, events.at(-1));
        day("2026-11-10").click(); await wait(0);
        log("pick", S().day, day("2026-11-10").hasAttribute("data-picked"), focused(), events.at(-1));
        // The keyboard: arrows move a day or a week, Page Down a month, Enter picks.
        key("ArrowRight"); key("ArrowDown"); log("arrows", focused());
        key("PageDown"); await wait(0); log("next month", focused(), $("[data-ev-title]").textContent, events.at(-1));
        key("PageUp"); key("Enter"); await wait(0); log("enter", S().day, $("[data-ev-title]").textContent);
        // Tab reaches the focused day's events, after it.
        day("2026-11-04").focus(); key("ArrowLeft"); key("ArrowRight");
        log("stops", $$("[data-ev-event][tabindex='0'], [data-ev-more][tabindex='0']").length);
        // +N more opens the day's week.
        day("2026-11-04").querySelector("[data-ev-more]").click(); await wait(0);
        log("more", $("[data-ev-body]").getAttribute("data-view"), $("[data-ev-title]").textContent, $("[data-ev-view=week]").getAttribute("aria-pressed"));
        // The week: all-day row, timed events by hour, overlaps side by side.
        const col = $('[data-ev-date-col="2026-11-04"]');
        const box = (t) => { const c = [...col.querySelectorAll("[data-ev-event]")].find((x) => x.textContent.includes(t)); return [c.style.top, c.style.left, c.style.width].join(" "); };
        log("allday", [...$$("[data-ev-allday-cell]")].map((c) => c.textContent || "-").join("|"));
        log("standup", box("Standup"), "| short", [...col.querySelectorAll("[data-ev-event]")][0].hasAttribute("data-short"));
        log("audit", box("Audit"), "| drill", box("Drill"));
        log("hours", $$("[data-ev-labels] [data-ev-hour]").length, $("[data-ev-labels] [data-ev-hour]").textContent);
        // The list: days with events, times or All day, the shift's second day Until.
        $("[data-ev-view=list]").click(); await wait(0);
        log("view", S().view);
        log("list", $$("[data-ev-group]").map((g) => g.getAttribute("data-ev-group").slice(8) + "=" + [...g.querySelectorAll("[data-ev-event]")].map((e) => e.querySelector("time").textContent + " " + e.querySelector("[data-ev-name]").textContent).join("/")).join(" | "));
        // The toolbar: next month (nothing there), Today, and from state.
        $("[data-ev-next]").click(); await wait(0);
        log("december", $("[data-ev-title]").textContent, $("[data-ev-empty]")?.textContent);
        $("[data-ev-today]").click(); await wait(0);
        log("today", $("[data-ev-title]").textContent);
        root.__irender.set("events", [...S().events, { id: 7, title: "Party", start: "2026-11-28T18:00" }]); await wait(0);
        log("added", $$("[data-ev-group]").at(-1).getAttribute("data-ev-group"), $$("[data-ev-group]").at(-1).textContent.includes("Party"));
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("events", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "month November 2026 6 weeks | Mon,Tue,Wed,Thu,Fri,Sat,Sun | today 2026-11-04",
            "legend Meetings:info,Works:warning | region Plant calendar",
            // Chips per day, `+N more` past three; ‹ › mark an event running on from or into another day.
            "4 Nov Maintenance›, 9:30Standup, +2 more | Wednesday 4 November, 4 events",
            "5 Nov Maintenance‹› | 6 Nov Maintenance‹ | 12 Nov Night shift‹",
            "tones warning,info",
            "outside true false",
            "open Planning open {\"event\":\"Planning\"}",
            "pick 2026-11-10 true 2026-11-10 day {\"date\":\"2026-11-10\"}",
            // The keyboard moves from the focused day: a day, a week, a month; Enter picks.
            "arrows 2026-11-18",
            "next month 2026-12-18 December 2026 navigate {\"date\":\"2026-12-18\",\"view\":\"month\"}",
            "enter 2026-11-18 November 2026",
            "stops 3",
            "more week 2 – 8 November 2026 true",
            "allday -|-|Maintenance|Maintenance|Maintenance|-|-",
            // The week: 7:00 to 20:00, so 9:30 is 2.5/13 down; overlapping events share the width.
            "standup 19.231% calc(0% + 2px) calc(100% - 4px) | short true",
            "audit 30.769% calc(0% + 2px) calc(50% - 4px) | drill 34.615% calc(50% + 2px) calc(50% - 4px)",
            "hours 13 7:00",
            "view list",
            // The list: days with events; a timed event's second day says until when.
            "list 02=10:00 – 11:30 Planning | 04=All day Maintenance/9:30 – 9:45 Standup/11:00 – 13:00 Audit/11:30 – 12:30 Drill | 05=All day Maintenance | 06=All day Maintenance | 11=22:00 – 6:00 Night shift | 12=Until 6:00 Night shift",
            "december December 2026 Nothing in December.",
            "today November 2026",
            "added 2026-11-28 true",
        ],
        "\n{result}"
    );
}

/// The number input: grouping as it's typed with the caret kept among its digits, in
/// two locales; decimals; clamping on leaving; Up, Down and the buttons; and state.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn number_input_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "amount": 125000, "qty": null, "eu": null },
        "root": { "type": "stack", "children": [
            { "type": "ui-field", "props": { "label": "Budget" }, "children": [
                { "type": "ui-number-input", "attrs": { "id": "amount" }, "props": { "bind": "amount", "currency": "INR", "locale": "en-IN", "min": 0, "max": 10000000 } } ] },
            { "type": "ui-number-input", "attrs": { "id": "qty" }, "props": { "bind": "qty", "label": "Quantity", "min": 1, "max": 20, "step": 2, "decimals": 0, "stepper": true, "suffix": "pcs" } },
            { "type": "ui-number-input", "attrs": { "id": "eu" }, "props": { "bind": "eu", "locale": "de-DE", "label": "Betrag", "suffix": "€", "decimals": 2 } } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const field = (id) => document.querySelector(`#${id} input`);
      // Types `text` as if the caret were at `caret` afterwards.
      const type = (input, text, caret = text.length) => { input.focus(); input.value = text; input.setSelectionRange(caret, caret); input.dispatchEvent(new InputEvent("input", { bubbles: true })); };
      const key = (input, k, shift = false) => input.dispatchEvent(new KeyboardEvent("keydown", { key: k, shiftKey: shift, bubbles: true, cancelable: true }));
      const blur = (input) => { input.dispatchEvent(new FocusEvent("blur")); };
      try {
        const a = field("amount");
        log("start", a.value, document.querySelector("#amount [data-num-prefix]").textContent, a.getAttribute("aria-description"), document.querySelector("label[for]").getAttribute("for") === a.id, a.getAttribute("inputmode"));
        type(a, "1250000"); log("typed", a.value, S().amount, a.selectionStart);
        // A digit typed in the middle: the caret stays after it.
        type(a, "12,503,000", 4); log("middle", a.value, S().amount, a.selectionStart, a.value.slice(0, a.selectionStart));
        type(a, "1,25,0.5"); log("fraction", a.value, S().amount);
        type(a, ""); log("empty", JSON.stringify(a.value), S().amount);
        type(a, "99999999"); blur(a); await wait(0); log("over max", a.value, S().amount);
        type(a, "-500"); log("negative", a.value, S().amount, a.getAttribute("aria-invalid"));
        blur(a); await wait(0); log("min", a.value, S().amount);
        // Steps: Up and Down, Shift for ten, the buttons, within min and max.
        const q = field("qty");
        log("qty", JSON.stringify(q.value), q.getAttribute("aria-label"), q.getAttribute("inputmode"), document.querySelector("#qty [data-num-step='1']").getAttribute("aria-label"));
        key(q, "ArrowUp"); log("up", q.value, S().qty);
        key(q, "ArrowUp"); key(q, "ArrowUp", true); log("shift up", q.value, S().qty);
        key(q, "ArrowDown"); log("down", S().qty);
        document.querySelector("#qty [data-num-step='-1']").click(); log("minus", S().qty);
        type(q, "7.6"); blur(q); await wait(0); log("rounded", q.value, S().qty);
        // German: dots group, a comma for decimals.
        const e = field("eu");
        type(e, "1234567,8"); log("de", e.value, S().eu);
        blur(e); await wait(0); log("de blur", e.value, S().eu);
        // From state.
        root.__irender.set("amount", 4500000); await wait(0); log("state", a.value);
        root.__irender.set("qty", null); await wait(0); log("cleared", JSON.stringify(q.value));
      } catch (err) { log("EXCEPTION", err.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("number", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "start 1,25,000 ₹ in INR true decimal",
            // Indian grouping as it's typed; a digit typed mid-number keeps the caret after it.
            "typed 12,50,000 1250000 9",
            "middle 1,25,03,000 12503000 4 1,25",
            "fraction 1,250.5 1250.5",
            "empty \"\" ",
            // Leaving the field clamps it; while typing it's only marked.
            "over max 1,00,00,000 10000000",
            "negative -500 -500 true",
            "min 0 0",
            "qty \"\" Quantity numeric Increase Quantity",
            // From empty, Up starts at `min`; Shift steps ten times; `decimals: 0` rounds.
            "up 1 1",
            "shift up 20 20",
            "down 18",
            "minus 16",
            "rounded 8 8",
            // German: dots group, a comma for the decimals.
            "de 1.234.567,8 1234567.8",
            // German: dots group, a comma for the decimals.
            "de blur 1.234.567,8 1234567.8",
            "state 45,00,000",
            "cleared \"\"",
        ],
        "\n{result}"
    );
}

/// The filters' date presets (ranges from `today`, marked while they're set) and saved
/// views (save the filters under a name, apply one, delete one).
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn filter_presets_and_views_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "f": { "status": ["Open"] }, "views": [ { "name": "Paid", "filters": { "status": ["Paid"] } } ] },
        "root": { "type": "ui-filters", "props": { "bind": "f", "bindViews": "views", "today": "2026-11-18", "live": true }, "children": [
            { "type": "ui-filter", "props": { "field": "status", "options": ["Open", "Paid"] } },
            { "type": "ui-filter", "props": { "field": "due", "type": "date" } },
            { "type": "ui-filter", "props": { "field": "sent", "type": "date", "presets": ["today", "thisQuarter"] } },
            { "type": "ui-filter", "props": { "field": "seen", "type": "date", "operators": ["on", "before"] } } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const F = document.querySelector("[data-i-x=ui-filters]");
      const $ = (s, from = F) => from.querySelector(s), $$ = (s, from = F) => [...from.querySelectorAll(s)];
      const pane = (i) => $(`[data-fl-pane="${i}"]`);
      try {
        $("[data-fl-open]").click(); await wait(0);
        log("presets", $$("[data-fl-preset]", pane(1)).map((b) => b.textContent).join(","));
        log("chosen", $$("[data-fl-preset]", pane(2)).map((b) => b.textContent).join(","), "| none without between", $("[data-fl-presets]", pane(3)) === null);
        const click = async (i, name) => { $(`[data-fl-preset="${name}"]`, pane(i)).click(); await wait(0); return JSON.stringify(S().f[i === 1 ? "due" : "sent"]); };
        for (const name of ["today", "yesterday", "last7", "last30", "thisMonth", "lastMonth", "thisYear"]) log(name, await click(1, name));
        log("pressed", $$("[data-fl-preset][aria-pressed=true]", pane(1)).map((b) => b.getAttribute("data-fl-preset")).join(","), $("[data-fl-op]", pane(1)).value);
        log("quarter", await click(2, "thisQuarter"));
        // Hidden while the operator takes no dates.
        const op = $("[data-fl-op]", pane(1)); op.value = "empty"; op.dispatchEvent(new Event("change", { bubbles: true })); await wait(0);
        log("empty hides", $("[data-fl-presets]", pane(1)).hidden);
        $("[data-fl-panel]").close();
        // Views: the menu, saving, the current one marked, applying, deleting.
        root.__irender.set("f", { status: ["Open"] }); await wait(0);
        $("[data-fl-views-open]").click(); await wait(0);
        log("menu", !$("[data-fl-views]").hidden, $("[data-fl-views-open]").getAttribute("aria-expanded"), $$("[data-fl-view-apply]").map((b) => b.textContent + (b.getAttribute("aria-current") === "true" ? "*" : "")).join(","), document.activeElement.textContent);
        const name = $("[data-fl-view-name]"); name.value = "Open ones";
        $("[data-fl-view-save]").click(); await wait(0); await wait(0);
        log("saved", JSON.stringify(S().views), $$("[data-fl-view-apply]").map((b) => b.textContent + (b.getAttribute("aria-current") === "true" ? "*" : "")).join(","));
        // Same name: replaced.
        root.__irender.set("f", { status: ["Open", "Paid"] }); await wait(0);
        name.value = "Open ones"; name.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true })); await wait(0);
        log("replaced", S().views.length, JSON.stringify(S().views.at(-1).filters));
        $$("[data-fl-view-apply]")[0].click(); await wait(0);
        log("applied", JSON.stringify(S().f), $("[data-fl-views]").hidden, document.activeElement.hasAttribute("data-fl-views-open"), $$("[data-fl-chip]").length);
        $("[data-fl-views-open]").click(); await wait(0);
        $$("[data-fl-view-delete]")[0].click(); await wait(0); await wait(0);
        log("deleted", S().views.map((v) => v.name).join(","), $$("[data-fl-view-delete]")[0].getAttribute("aria-label"));
        $("[data-fl-views]").dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
        log("escape", $("[data-fl-views]").hidden);
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("filter-views", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "presets Today,Yesterday,Last 7 days,Last 30 days,This month,Last month,This quarter,This year",
            "chosen Today,This quarter | none without between true",
            // Ranges from `today` (18 Nov 2026), as Between.
            "today {\"from\":\"2026-11-18\",\"to\":\"2026-11-18\"}",
            "yesterday {\"from\":\"2026-11-17\",\"to\":\"2026-11-17\"}",
            "last7 {\"from\":\"2026-11-12\",\"to\":\"2026-11-18\"}",
            "last30 {\"from\":\"2026-10-20\",\"to\":\"2026-11-18\"}",
            "thisMonth {\"from\":\"2026-11-01\",\"to\":\"2026-11-30\"}",
            "lastMonth {\"from\":\"2026-10-01\",\"to\":\"2026-10-31\"}",
            "thisYear {\"from\":\"2026-01-01\",\"to\":\"2026-12-31\"}",
            "pressed thisYear between",
            "quarter {\"from\":\"2026-10-01\",\"to\":\"2026-12-31\"}",
            "empty hides true",
            // Views: the current one marked, saving, the same name replaced, applying, deleting.
            "menu true true Paid Paid",
            "saved [{\"name\":\"Paid\",\"filters\":{\"status\":[\"Paid\"]}},{\"name\":\"Open ones\",\"filters\":{\"status\":[\"Open\"]}}] Paid,Open ones*",
            "replaced 2 {\"status\":[\"Open\",\"Paid\"]}",
            "applied {\"status\":[\"Paid\"]} true true 1",
            "deleted Open ones Delete the Open ones view",
            "escape true",
        ],
        "\n{result}"
    );
}

/// Notifications: the bell's count, the panel with Unread and All, the arrow keys,
/// marking read and unread, archiving, Mark all as read, opening one, and state.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn notifications_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "open": null, "notes": [
            { "id": 1, "title": "Comment", "body": "Looks good", "who": "Meera Iyer", "when": "5 min ago" },
            { "id": 2, "title": "Build failed", "tone": "danger", "when": "1 h ago" },
            { "id": 3, "title": "Paid", "read": true, "tone": "success" },
            { "id": 4, "title": "Old", "archived": true } ] },
        "root": { "type": "ui-notifications", "props": { "items": "{{ notes }}", "bindItems": "notes", "bindItem": "open", "allHref": "/all" } } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const N = document.querySelector("[data-i-x=ui-notifications]");
      const $ = (s) => N.querySelector(s), $$ = (s) => [...N.querySelectorAll(s)];
      const rows = () => $$("[data-nt-item]").map((li) => li.querySelector("[data-nt-title]").textContent + (li.hasAttribute("data-unread") ? "*" : "")).join(",");
      const row = (t) => $$("[data-nt-item]").find((li) => li.querySelector("[data-nt-title]").textContent === t);
      const focused = () => document.activeElement.closest?.("[data-nt-item]")?.querySelector("[data-nt-title]").textContent ?? document.activeElement.textContent;
      const events = [];
      for (const n of ["open", "read", "archive"]) N.addEventListener(n, (e) => events.push(n + " " + JSON.stringify(e.detail.items?.map((x) => x.title) ?? e.detail.item?.title) + (e.detail.read === undefined ? "" : " " + e.detail.read)));
      try {
        const bell = $("[data-nt-bell]");
        log("bell", bell.getAttribute("aria-label"), $("[data-nt-count]").textContent, bell.getAttribute("aria-haspopup"));
        bell.click(); await wait(0);
        const panel = $("[data-nt-panel]");
        log("open", panel.matches(":popover-open"), rows(), "| focus", focused(), "| tabs", $$("[data-nt-tab]").map((t) => t.textContent + (t.getAttribute("aria-pressed") === "true" ? "*" : "")).join(","));
        log("labels", row("Comment").querySelector("[data-nt-open]").getAttribute("aria-label"), "|", row("Comment").querySelector("[data-nt-mark]").textContent, row("Build failed").querySelector("[data-nt-mark]").hasAttribute("data-dot"));
        document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true, cancelable: true }));
        log("down", focused());
        // All shows the read one too, never the archived.
        $('[data-nt-tab="all"]').click(); await wait(0);
        log("all", rows());
        // Mark one read: the count and the tab follow.
        row("Comment").querySelector("[data-nt-toggle]").click(); await wait(0);
        log("read", rows(), $("[data-nt-count]").textContent, bell.getAttribute("aria-label"), events.at(-1));
        row("Comment").querySelector("[data-nt-toggle]").click(); await wait(0);
        log("unread", rows(), $("[data-nt-count]").textContent, row("Comment").querySelector("[data-nt-toggle]").getAttribute("aria-label"));
        // Archive: gone from both tabs, focus to the next.
        row("Build failed").querySelector("[data-nt-archive]").click(); await wait(0);
        log("archived", rows(), JSON.stringify(S().notes.find((n) => n.id === 2).archived), events.at(-1), "| focus", focused());
        // Mark all as read; Unread then says caught up.
        $("[data-nt-mark-all]").click(); await wait(0);
        log("all read", $("[data-nt-count]").hidden, $("[data-nt-mark-all]").disabled, events.at(-1));
        $('[data-nt-tab="unread"]').click(); await wait(0);
        log("caught up", rows() || "-", $("[data-nt-empty]").textContent, $("[data-nt-tab]").textContent);
        // A new one from state; opening it marks it read and closes the panel.
        root.__irender.set("notes", [{ id: 9, title: "New", body: "Hi", who: { name: "Asha Rao", initials: "AR" } }, ...S().notes]); await wait(0);
        log("new", rows(), $("[data-nt-count]").textContent, row("New").querySelector("[data-nt-mark]").textContent);
        row("New").querySelector("[data-nt-open]").click(); await wait(0);
        log("opened", S().open?.title, S().open?.read, S().notes[0].read, panel.matches(":popover-open"), events.at(-1));
        log("foot", $("[data-nt-foot] a").getAttribute("href"), $("[data-nt-live]").getAttribute("aria-live"));
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("notifications", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "bell Notifications, 2 unread 2 dialog",
            // Opening the panel: Unread, focus on the first notification.
            "open true Comment*,Build failed* | focus Comment | tabs Unread (2)*,All",
            "labels Unread. Comment. Looks good. 5 min ago | MI true",
            "down Build failed",
            // All adds the read ones, never the archived.
            "all Comment*,Build failed*,Paid",
            // The read toggle, both ways; the count and the bell's label follow.
            "read Comment,Build failed*,Paid 1 Notifications, 1 unread read [\"Comment\"] true",
            "unread Comment*,Build failed*,Paid 2 Mark Comment as read",
            // Archived: gone from both tabs; focus moves to the next.
            "archived Comment*,Paid true archive \"Build failed\" | focus Paid",
            "all read true true read [\"Comment\"] true",
            "caught up - You're all caught up. Unread",
            "new New* 1 AR",
            "opened New true true false open \"New\"",
            "foot /all polite",
        ],
        "\n{result}"
    );
}

/// The filter panel's date fields use the kit's calendar (a range for Between, one day
/// for the rest), and its amounts are grouped as they're typed.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn filter_calendar_and_amounts_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "f": {} },
        "root": { "type": "ui-filters", "props": { "bind": "f", "today": "2026-11-18", "currency": "INR", "locale": "en-IN" }, "children": [
            { "type": "ui-filter", "props": { "field": "due", "label": "Due", "type": "date" } },
            { "type": "ui-filter", "props": { "field": "amount", "label": "Amount", "type": "currency" } } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const F = document.querySelector("[data-i-x=ui-filters]");
      const $ = (s, from = F) => from.querySelector(s), $$ = (s, from = F) => [...from.querySelectorAll(s)];
      const pane = (i) => $(`[data-fl-pane="${i}"]`);
      const cal = (m) => $(`[data-fl-cal="${m}"]`, pane(0));
      const shown = (e) => e && !e.hidden && getComputedStyle(e).display !== "none";
      try {
        $("[data-fl-open]").click(); await wait(0);
        log("between", shown(cal("range")), shown(cal("single")), shown($("[data-fl-dates]", pane(0))), cal("range").getAttribute("aria-label"));
        // A preset fills the calendar too.
        $('[data-fl-preset="thisMonth"]', pane(0)).click(); await wait(0);
        log("preset", cal("range").querySelector("[data-range-start]")?.getAttribute("data-key"), cal("range").querySelector("[data-range-end]")?.getAttribute("data-key"));
        // Two days picked on the calendar make the range.
        cal("range").querySelector('[data-key="2026-11-05"]').click(); await wait(0);
        log("half", $("[data-fl-a]", pane(0)).value, "(unchanged until the end is picked)");
        cal("range").querySelector('[data-key="2026-11-12"]').click(); await wait(0);
        log("range", $("[data-fl-a]", pane(0)).value, $("[data-fl-b]", pane(0)).value, $('[data-fl-preset="thisMonth"]', pane(0)).getAttribute("aria-pressed"));
        // Before: the one-day calendar.
        const op = $("[data-fl-op]", pane(0)); op.value = "before"; op.dispatchEvent(new Event("change", { bubbles: true })); await wait(0);
        log("before", shown(cal("range")), shown(cal("single")));
        cal("single").querySelector('[data-key="2026-11-20"]').click(); await wait(0);
        log("day", $("[data-fl-a]", pane(0)).value);
        // Amounts: the rupee sign, Indian grouping as typed, a plain number in the filter.
        $$("[role=tab]")[1].click(); await wait(0);
        const a = $("[data-fl-a]", pane(1));
        log("amount", $("[data-fl-prefix]", pane(1)).textContent, a.getAttribute("inputmode"));
        const opA = $("[data-fl-op]", pane(1)); opA.value = "gt"; opA.dispatchEvent(new Event("change", { bubbles: true }));
        a.focus(); a.value = "125000"; a.setSelectionRange(6, 6); a.dispatchEvent(new InputEvent("input", { bubbles: true }));
        log("typed", a.value, a.selectionStart);
        $("[data-fl-apply]").click(); await wait(0);
        log("applied", JSON.stringify(S().f), $$("[data-fl-chip]").map((c) => c.textContent.replace("✕", "")).join(" | "));
        // Opened again: the amount shows grouped, the calendar shows the day.
        $("[data-fl-open]").click(); await wait(0);
        log("again", $("[data-fl-a]", pane(1)).value, cal("single").querySelector("[aria-selected=true] button")?.getAttribute("data-key") ?? cal("single").querySelector('[data-key="2026-11-20"]')?.closest("td").getAttribute("aria-selected"));
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("filter-calendar", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "between true false false Due range",
            "preset 2026-11-01 2026-11-30",
            "half 2026-11-01 (unchanged until the end is picked)",
            "range 2026-11-05 2026-11-12 false",
            "before false true",
            "day 2026-11-20",
            "amount ₹ decimal",
            "typed 1,25,000 8",
            "applied {\"due\":{\"lt\":\"2026-11-20\"},\"amount\":{\"gt\":125000}} Due:before 20 Nov 2026 | Amount:&gt; ₹1,25,000",
            "again 1,25,000 2026-11-20",
        ],
        "\n{result}"
    );
}

/// The event calendar's year (twelve small months, busy days marked; a day opens in the
/// day view, a month's name in the month view) and day views.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn event_calendar_year_and_day_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "day": null, "events": [
            { "id": 1, "title": "Audit", "start": "2026-03-10", "category": "Audits" },
            { "id": 2, "title": "Standup", "start": "2026-11-04T09:30", "end": "2026-11-04T09:45" },
            { "id": 3, "title": "Review", "start": "2026-11-04T14:00", "end": "2026-11-04T16:00", "tone": "warning" },
            { "id": 4, "title": "Shutdown", "start": "2026-12-24", "end": "2026-12-26", "tone": "danger" } ] },
        "root": { "type": "ui-event-calendar", "props": { "events": "{{ events }}", "categories": { "Audits": "info" }, "bindDay": "day",
            "view": "year", "date": "2026-11-04", "today": "2026-11-18", "locale": "en-GB", "hours": [8, 18] } } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" ").replace(/\b0(\d:\d\d)/g, "$1"));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const C = document.querySelector("[data-i-x=ui-event-calendar]");
      const $ = (s) => C.querySelector(s), $$ = (s) => [...C.querySelectorAll(s)];
      const key = (k) => document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true }));
      try {
        log("views", $$("[data-ev-view]").map((b) => b.textContent + (b.getAttribute("aria-pressed") === "true" ? "*" : "")).join(","));
        log("year", $("[data-ev-title]").textContent, $$("[data-ev-mini]").length, "months |", $$("[data-ev-month-open]").slice(0, 3).map((b) => b.textContent).join(","));
        log("busy", $$("[data-ev-mini] [data-busy]").map((d) => d.getAttribute("data-ev-day").slice(5) + ":" + d.getAttribute("data-tone")).join(","));
        log("labels", $('[data-ev-day="2026-11-04"]').getAttribute("aria-label"), "|", $('[data-ev-day="2026-11-18"]').hasAttribute("data-today"));
        // The arrow keys move across months.
        $('[data-ev-day="2026-11-04"]').focus(); key("ArrowDown"); key("ArrowDown"); key("ArrowDown"); key("ArrowDown");
        log("keys", document.activeElement.getAttribute("data-ev-day"));
        // Enter opens the day in the day view.
        $('[data-ev-day="2026-11-04"]').focus(); key("Enter"); await wait(0);
        log("enter", S().day, $("[data-ev-body]").getAttribute("data-view"), $("[data-ev-title]").textContent, $('[data-ev-view="day"]').getAttribute("aria-pressed"));
        // The day view: one column, timed events by hour.
        const col = $('[data-ev-date-col="2026-11-04"]');
        log("day", $$("[data-ev-col]").length, "column |", [...col.querySelectorAll("[data-ev-event]")].map((c) => c.textContent + " @" + c.style.top).join(" | "), "|", $$("[data-ev-labels] [data-ev-hour]").length, "hours");
        // Next and previous move a day.
        $("[data-ev-next]").click(); await wait(0);
        log("next", $("[data-ev-title]").textContent, $$("[data-ev-col] [data-ev-event]").length);
        $("[data-ev-prev]").click(); $("[data-ev-prev]").click(); await wait(0);
        log("prev", $("[data-ev-title]").textContent);
        // Back to the year, a month's name opens it.
        $('[data-ev-view="year"]').click(); await wait(0);
        $$("[data-ev-month-open]")[11].click(); await wait(0);
        log("month", $("[data-ev-body]").getAttribute("data-view"), $("[data-ev-title]").textContent, [...$('[data-ev-day="2026-12-25"]').querySelectorAll("[data-ev-event]")].map((c) => c.textContent).join(","));
        // Next in the year view moves a year.
        $('[data-ev-view="year"]').click(); $("[data-ev-next]").click(); await wait(0);
        log("next year", $("[data-ev-title]").textContent, $$("[data-ev-mini] [data-busy]").length);
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("events-year", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "views Day,Week,Month,Year*,List",
            "year 2026 12 months | January,February,March",
            // Busy days in their tone; the arrows move across months.
            "busy 03-10:info,11-04:primary,12-24:danger,12-25:danger,12-26:danger",
            "labels Wednesday 4 November, 2 events | true",
            "keys 2026-12-02",
            // Enter opens the day: one column, 8:00 to 18:00, so 9:30 is 15% down.
            "enter 2026-11-04 day Wednesday, 4 November 2026 true",
            "day 1 column | 9:30Standup @15% | 14:00Review @60% | 10 hours",
            "next Thursday, 5 November 2026 0",
            "prev Tuesday, 3 November 2026",
            // A month's name opens it; Next in the year moves a year.
            "month month December 2026 Shutdown",
            "next year 2027 0",
        ],
        "\n{result}"
    );
}

/// The event calendar's marked days (`days`): a shade in a tone, a label beside the date
/// and badges under it in the month, the week and the list; in the year, only the marked
/// days with `yearEvents: false`; and new marks from state.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn event_calendar_days_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": {
            "events": [ { "id": 1, "title": "Audit", "start": "2026-11-05T10:00", "end": "2026-11-05T11:00" } ],
            "days": [
                { "date": "2026-11-03", "tone": "danger", "label": "Closed" },
                { "date": "2026-11-04", "tone": "warning", "items": ["Morning", { "text": "Night", "tone": "info" }] },
                { "date": "2026-11-04", "label": "Short" },
                { "date": "2026-11-05", "items": ["Morning", "Evening"] },
                { "date": "2026-12-24", "end": "2026-12-26", "tone": "success", "label": "Holidays" },
                { "date": "nonsense", "tone": "danger" } ] },
        "root": { "type": "ui-event-calendar", "props": { "events": "{{ events }}", "days": "{{ days }}", "yearEvents": false,
            "date": "2026-11-04", "today": "2026-11-18", "locale": "en-GB" } } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const C = document.querySelector("[data-i-x=ui-event-calendar]");
      const $ = (s) => C.querySelector(s), $$ = (s) => [...C.querySelectorAll(s)];
      const day = (d) => $(`[data-ev-day="${d}"]`);
      const cell = (d) => { const c = day(d); return [c.hasAttribute("data-shade") ? "shade:" + c.getAttribute("data-tone") : "plain", c.querySelector("[data-ev-note]")?.textContent ?? "-", [...c.querySelectorAll("[data-ev-badge]")].map((b) => b.textContent + (b.getAttribute("data-tone") ? ":" + b.getAttribute("data-tone") : "")).join("/") || "-"].join(" "); };
      try {
        log("3 Nov", cell("2026-11-03"), "|", day("2026-11-03").getAttribute("aria-label"));
        log("4 Nov", cell("2026-11-04"), "|", day("2026-11-04").getAttribute("aria-label"));
        log("5 Nov", cell("2026-11-05"), "|", day("2026-11-05").querySelectorAll("[data-ev-event]").length, "event");
        log("6 Nov", cell("2026-11-06"));
        const bg = (d) => getComputedStyle(day(d)).backgroundColor;
        log("tinted", bg("2026-11-03") !== bg("2026-11-06"), bg("2026-11-03") !== bg("2026-11-04"));
        // The week: the column and the all-day cell shaded, the label and badges above.
        $('[data-ev-view="week"]').click(); await wait(0);
        const ad = (d) => $(`[data-ev-cell-date="${d}"]`);
        log("week", ad("2026-11-03").getAttribute("data-tone"), $('[data-ev-date-col="2026-11-03"]').hasAttribute("data-shade"), ad("2026-11-04").textContent, ad("2026-11-06").hasAttribute("data-shade"));
        // The list: days with marks as well as days with events.
        $('[data-ev-view="list"]').click(); await wait(0);
        log("list", $$("[data-ev-group]").map((g) => g.getAttribute("data-ev-group").slice(8) + "=" + (g.querySelector("h4 [data-ev-note]")?.textContent ?? "-") + "[" + g.querySelectorAll("[data-ev-badge]").length + "]" + g.querySelectorAll("[data-ev-event]").length).join(" | "));
        // The year: only the marked days with a tone, the label on hover.
        $('[data-ev-view="year"]').click(); await wait(0);
        log("year", $$("[data-ev-mini] [data-shade]").map((d) => d.getAttribute("data-ev-day").slice(5) + ":" + d.getAttribute("data-tone")).join(","), "| busy", $$("[data-ev-mini] [data-busy]").length, "|", day("2026-12-25").title);
        // From state: a new mark redraws.
        root.__irender.set("days", [{ date: "2026-11-20", tone: "info", label: "Stocktake" }]); await wait(0);
        log("state", $$("[data-ev-mini] [data-shade]").map((d) => d.getAttribute("data-ev-day").slice(5)).join(","));
        // As a map of date → mark.
        root.__irender.set("days", { "2026-11-09": { tone: "danger", label: "Closed" }, "2026-11-10": { items: ["A"] } }); await wait(0);
        $('[data-ev-view="month"]').click(); await wait(0);
        log("map", cell("2026-11-09"), "|", cell("2026-11-10"));
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("events-days", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "3 Nov shade:danger Closed - | Tuesday 3 November, Closed",
            // Two marks on a day share it: the tone of one, the label of the other.
            "4 Nov shade:warning Short Morning/Night:info | Wednesday 4 November, Short, Morning, Night",
            "5 Nov plain - Morning/Evening | 1 event",
            "6 Nov plain - -",
            "tinted true true",
            "week danger true ShortMorningNight false",
            "list 03=Closed[0]0 | 04=Short[2]0 | 05=-[2]1",
            // The year tints only the marked days with a tone (`yearEvents: false`).
            "year 11-03:danger,11-04:warning,12-24:success,12-25:success,12-26:success | busy 0 | Holidays",
            "state 11-20",
            "map shade:danger Closed - | plain - A",
        ],
        "\n{result}"
    );
}

/// The rail layout: sections as a vertical tab list, each with its panel; a click on
/// the section shown folds the panel; the fold button; the arrow keys; and state.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn rail_layout_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "section": "inbox", "open": true },
        "root": { "type": "ui-rail-layout", "props": { "bind": "section", "bindOpen": "open", "height": 400, "items": [
                { "id": "inbox", "label": "Inbox", "icon": "✉", "badge": 4 },
                { "id": "projects", "label": "Projects" },
                { "id": "reports", "label": "Reports" },
                { "id": "help", "label": "Help", "href": "#help" },
                { "id": "settings", "label": "Settings", "icon": "⚙", "bottom": true } ] },
            "children": [
                { "type": "ui-rail-panel", "props": { "for": "inbox", "title": "Mail" }, "children": [ { "type": "text", "props": { "value": "Inbox panel" } } ] },
                { "type": "ui-rail-panel", "props": { "for": "projects" }, "children": [ { "type": "text", "props": { "value": "Projects panel" } } ] },
                { "type": "element", "attrs": { "id": "main" }, "children": [ { "type": "text", "props": { "value": "Main {{ section }}" } } ] } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const L = document.querySelector("[data-i-x=ui-rail-layout]");
      const $ = (s) => L.querySelector(s), $$ = (s) => [...L.querySelectorAll(s)];
      const tab = (id) => $(`[data-rail-item="${id}"]`);
      const shown = () => $$("[data-rail-panel]").filter((p) => !p.hidden).map((p) => p.getAttribute("data-rail-panel")).join(",") || "-";
      const state = () => `${S().section}/${S().open} panel ${shown()} closed ${L.hasAttribute("data-closed")}`;
      const key = (k) => document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true }));
      try {
        log("rail", $("[data-rail]").getAttribute("aria-label"), $("[role=tablist]").getAttribute("aria-orientation"), $$("[role=tab]").map((t) => t.getAttribute("aria-label") + (t.getAttribute("aria-selected") === "true" ? "*" : "")).join(","));
        log("tab", tab("inbox").getAttribute("aria-controls") === $('[data-rail-panel="inbox"]').id, tab("inbox").textContent, tab("projects").textContent, tab("help").tagName, tab("help").getAttribute("href"), $('[data-rail-foot] [data-rail-item]').getAttribute("data-rail-item"));
        log("start", state(), $('[data-rail-panel="inbox"] h2').textContent, $('[data-rail-panel="projects"] h2').textContent, $("#main").textContent);
        tab("projects").click(); await wait(0);
        log("projects", state(), $("#main").textContent, tab("projects").getAttribute("aria-selected"), tab("projects").tabIndex);
        tab("projects").click(); await wait(0);
        log("fold", state());
        tab("projects").click(); await wait(0);
        log("unfold", state());
        $('[data-rail-panel="projects"] [data-rail-fold]').click(); await wait(0);
        log("button", state(), document.activeElement === tab("projects"));
        // A section without a panel: shown, no panel.
        tab("reports").click(); await wait(0);
        log("reports", state());
        // The keys move between sections; Enter shows one.
        tab("reports").focus(); key("ArrowUp"); log("up", document.activeElement.getAttribute("data-rail-item"), $$("[role=tab][tabindex='0']").length);
        key("End"); log("end", document.activeElement.getAttribute("data-rail-item"));
        key("ArrowDown"); log("wrap", document.activeElement.getAttribute("data-rail-item"));
        // From state.
        root.__irender.set("section", "inbox"); root.__irender.set("open", true); await wait(0);
        log("state", state(), tab("inbox").getAttribute("aria-selected"));
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("rail", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "rail Sections vertical Inbox*,Projects,Reports,Settings",
            "tab true ✉4 P A #help settings",
            "start inbox/true panel inbox closed false Mail Projects Main inbox",
            "projects projects/true panel projects closed false Main projects true 0",
            // A click on the section shown folds its panel, and opens it again; so does the fold button.
            "fold projects/false panel projects closed true",
            "unfold projects/true panel projects closed false",
            "button projects/false panel projects closed true true",
            // A section with no panel: the panel's place closes.
            "reports reports/true panel - closed true",
            // One tab stop; the arrows move it, End and wrapping.
            "up projects 1",
            "end settings",
            "wrap inbox",
            "state inbox/true panel inbox closed false true",
        ],
        "\n{result}"
    );
}

/// Resizable panels: the handles as separators, dragging, the keys within each panel's
/// limits, folding a collapsible panel and bringing it back, a double click to reset,
/// and sizes to and from state.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn resizable_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "sizes": [30, 70] },
        "root": { "type": "element", "style": { "width": "800px" }, "children": [
            { "type": "ui-resizable", "props": { "bind": "sizes", "height": 200, "label": "Files" }, "children": [
                { "type": "ui-resizable-panel", "props": { "min": 20, "max": 60, "collapsible": true, "label": "the sidebar" }, "children": [ { "type": "text", "props": { "value": "A" } } ] },
                { "type": "ui-resizable-panel", "props": { "min": 30 }, "children": [ { "type": "text", "props": { "value": "B" } } ] } ] },
            { "type": "ui-resizable", "attrs": { "id": "v" }, "props": { "direction": "vertical", "height": 300 }, "children": [
                { "type": "ui-resizable-panel", "props": { "size": 50 }, "children": [ { "type": "text", "props": { "value": "C" } } ] },
                { "type": "ui-resizable-panel", "children": [ { "type": "text", "props": { "value": "D" } } ] },
                { "type": "ui-resizable-panel", "children": [ { "type": "text", "props": { "value": "E" } } ] } ] } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const [G, V] = document.querySelectorAll("[data-i-x=ui-resizable]");
      const kids = (g, s) => [...g.children].filter((c) => c.matches(s));
      const widths = (g) => kids(g, "[data-rz-panel]").map((p) => Math.round(g === V ? p.offsetHeight : p.offsetWidth)).join("/");
      const H = kids(G, "[data-rz-handle]")[0];
      const key = (h, k, shift) => h.dispatchEvent(new KeyboardEvent("keydown", { key: k, shiftKey: !!shift, bubbles: true, cancelable: true }));
      const now = () => `${JSON.stringify(S().sizes)} ${widths(G)} ${H.getAttribute("aria-valuenow")}`;
      const ptr = (t, type, x, y) => t.dispatchEvent(new PointerEvent(type, { bubbles: true, cancelable: true, pointerId: 7, button: 0, clientX: x, clientY: y }));
      try {
        log("handle", H.getAttribute("role"), H.getAttribute("aria-orientation"), H.getAttribute("aria-label"), H.getAttribute("aria-valuemin"), H.getAttribute("aria-valuemax"), H.getAttribute("aria-controls") === kids(G, "[data-rz-panel]")[0].id, H.tabIndex);
        log("vertical", kids(V, "[data-rz-handle]").map((h) => h.getAttribute("aria-orientation") + ":" + h.getAttribute("aria-label") + ":" + h.getAttribute("aria-valuenow")).join(" "), widths(V));
        log("start", now());
        key(H, "ArrowRight"); log("right", now());
        key(H, "ArrowLeft", true); log("shift", now());
        key(H, "End"); log("end (max 60)", now());
        // Dragging, in pixels of the 799 the panels share.
        const x = H.getBoundingClientRect().left;
        ptr(H, "pointerdown", x, 50); ptr(H, "pointermove", x - 160, 50); log("drag", now(), H.hasAttribute("data-dragging")); ptr(H, "pointerup", x - 160, 50);
        log("dropped", now(), H.hasAttribute("data-dragging"));
        ptr(H, "pointerdown", x, 50); ptr(H, "pointermove", x - 700, 50); ptr(H, "pointerup", x - 700, 50);
        log("dragged to the end: folds", now(), kids(G, "[data-rz-panel]")[0].hasAttribute("data-collapsed"));
        key(H, "Enter"); log("enter: back", now());
        key(H, "Enter"); log("enter: folds", now());
        key(H, "Enter"); key(H, "Home"); log("home folds", now());
        H.dispatchEvent(new MouseEvent("dblclick", { bubbles: true })); log("reset", now());
        root.__irender.set("sizes", [55, 45]); await wait(0); log("state", now());
        // The vertical group's keys are Up and Down.
        const VH = kids(V, "[data-rz-handle]")[1];
        key(VH, "ArrowDown"); key(VH, "ArrowRight"); log("down", widths(V), VH.getAttribute("aria-valuenow"));
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("resizable", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "handle separator vertical Resize the sidebar 20 60 true 0",
            "vertical horizontal:Resize panels:50 horizontal:Resize panels:25 149/75/75",
            "start [30,70] 240/559 30",
            // 5% a press, 1% with Shift, up to the first panel's max.
            "right [35,65] 280/519 35",
            "shift [34,66] 272/527 34",
            "end (max 60) [60,40] 479/320 60",
            // Dragging, saved when the pointer is let go.
            "drag [60,40] 319/480 40 true",
            "dropped [39.97,60.03] 319/480 40 false",
            // Under half its min, a collapsible panel folds; Enter brings it back to its last size.
            "dragged to the end: folds [0,100] 0/799 0 true",
            "enter: back [39.97,60.03] 319/480 40",
            "enter: folds [0,100] 0/799 0",
            "home folds [0,100] 0/799 0",
            // A double click goes back to the starting sizes.
            "reset [30,70] 240/559 30",
            // From state.
            "state [55,45] 439/360 55",
            "down 149/89/60 30",
        ],
        "\n{result}"
    );
}

/// Code: the highlighter's tokens, line numbers and marked lines, copying, code from
/// state; the JSON editor's checks and messages, the value to state while valid, the
/// keys, Format, `type`, and changes to state from outside.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn code_block_and_json_editor_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "cfg": { "a": 1, "list": [true, null] }, "ok": null, "src": "let x = 1;" },
        "root": { "type": "element", "children": [
            { "type": "ui-code-block", "attrs": { "id": "rs" }, "props": { "language": "rust", "title": "main.rs", "lineNumbers": true, "highlight": [1, "3-4"],
                "code": "// hi\nfn main() {\n    let s = \"x\";\n    println!(\"{}\", 42);\n}\n" } },
            { "type": "ui-code-block", "attrs": { "id": "st" }, "props": { "language": "json", "code": "{{ cfg }}", "copy": false } },
            { "type": "ui-code-block", "attrs": { "id": "js" }, "props": { "language": "js", "code": "{{ src }}" } },
            { "type": "ui-json-editor", "attrs": { "id": "ed" }, "props": { "bind": "cfg", "bindValid": "ok", "type": "object", "label": "Config" } } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const C = window.__irenderCode;
      const toks = (sel) => [...document.querySelectorAll(`${sel} [data-t]`)].map((t) => t.dataset.t + ":" + t.textContent).join(" ");
      try {
        // ---- the highlighter
        log("rust", toks("#rs"));
        log("lines", [...document.querySelectorAll("#rs [data-ln]")].map((l, i) => (l.hasAttribute("data-hl") ? "*" : "") + (i + 1)).join(","), document.querySelector("#rs").hasAttribute("data-numbered"), document.querySelector("#rs pre").getAttribute("aria-label"));
        log("json", C.lines('{"k": [1.5e3, "v", true, null]} // c', "json").join("|"));
        log("multi", C.lines("a /* one\ntwo */ b", "js").join("|"));
        log("yaml", C.lines("name: x # note\n- id: 3\n  on: yes", "yaml").join("|"));
        log("sql", C.lines("SELECT id FROM t WHERE n > 2 -- x", "sql").join("|"));
        log("html", C.lines('<a href="/x">hi</a>', "html").join("|"));
        log("plain", C.lines("<b>&", "nope").join("|"));
        // ---- code from state
        log("state", document.querySelector("#st code").textContent.replace(/\s+/g, " "), document.querySelector("#st [data-code-copy]"));
        root.__irender.set("src", "const y = f(2);"); await wait(0);
        log("follows", toks("#js"));
        // ---- copying
        let copied = null;
        Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText: async (t) => { copied = t; } } });
        const btn = document.querySelector("#rs [data-code-copy]");
        btn.click(); await wait(0); await wait(0);
        log("copy", JSON.stringify(copied), btn.getAttribute("aria-label"), btn.hasAttribute("data-copied"), document.querySelector("#rs [data-code-live]").textContent);

        // ---- the JSON reader's messages
        for (const t of ['{"a": 1,}', "{a: 1}", "{'a': 1}", '{"a" 1}', '[1 2]', '{"a": "x', '{"a": 01}', '"\\q"', "", "  ", '{"a": tru}', '[1]]']) {
          const e = C.check(t); log("check", JSON.stringify(t), e ? `${e.line}:${e.column} ${e.message}` : "ok");
        }
        log("check", JSON.stringify('{\n  "a": [\n    1,\n  ]\n}'), (({ line, column, message }) => `${line}:${column} ${message}`)(C.check('{\n  "a": [\n    1,\n  ]\n}')));

        // ---- the editor
        const E = document.querySelector("[data-i-x=ui-json-editor]"), ta = E.querySelector("textarea");
        const err = () => E.querySelector("[data-je-error]").textContent || "-";
        const bad = () => [...E.querySelectorAll("[data-je-gutter] [data-err]")].map((s) => s.textContent).join(",") || "-";
        const now = () => `${JSON.stringify(S().cfg)} ok=${S().ok} invalid=${E.hasAttribute("data-invalid")} aria=${ta.getAttribute("aria-invalid")} err=${err()} line=${bad()}`;
        const type = (text) => { ta.value = text; ta.dispatchEvent(new Event("input", { bubbles: true })); };
        log("editor", ta.getAttribute("aria-label"), JSON.stringify(ta.value), E.querySelectorAll("[data-je-gutter] span").length, E.querySelector("[data-je-status]").textContent, S().ok);
        log("ids", ta.getAttribute("aria-describedby").split(" ").every((id) => document.getElementById(id)));
        type('{"a": 2}'); await wait(0); log("valid", now());
        type('{"a": 2,\n "b": }'); await wait(0); log("typing", now());
        await wait(700); log("paused", now());
        type('{"a": 3}'); await wait(0); log("fixed", now());
        type("[1, 2]"); ta.dispatchEvent(new Event("blur")); await wait(0); log("type", now());
        // Format, from a valid text and from an invalid one.
        type('{"b":[1,{"c":null}],"a":"x"}'); await wait(0);
        E.querySelector("[data-je-format]").click(); await wait(0);
        log("format", JSON.stringify(ta.value), S().ok, E.querySelectorAll("[data-je-gutter] span").length);
        type('{\n  "a": 1\n  "b": 2\n}'); E.querySelector("[data-je-format]").click(); await wait(0);
        log("format bad", now(), ta.selectionStart, document.activeElement === ta);
        // The keys: Tab indents, Shift+Tab takes it back, Enter keeps the indent and opens brackets.
        ta.focus(); type("{}"); ta.setSelectionRange(1, 1);
        const key = (k, shift) => ta.dispatchEvent(new KeyboardEvent("keydown", { key: k, shiftKey: !!shift, bubbles: true, cancelable: true }));
        key("Enter"); log("enter", JSON.stringify(ta.value), ta.selectionStart);
        key("Tab"); log("tab", JSON.stringify(ta.value));
        key("Tab", true); log("shift-tab", JSON.stringify(ta.value), ta.selectionStart);
        const before = ta.value; key("Escape"); const leave = new KeyboardEvent("keydown", { key: "Tab", bubbles: true, cancelable: true }); ta.dispatchEvent(leave);
        log("escape, tab", leave.defaultPrevented, ta.value === before);
        // State changed from outside: shown formatted.
        root.__irender.set("cfg", { z: [1] }); await wait(0);
        log("outside", JSON.stringify(ta.value), now(), toks("#st"));
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("code", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "rust c:// hi k:fn f:main k:let s:\"x\" f:println! s:\"{}\" n:42",
            "lines *1,2,*3,*4,5 true main.rs",
            // The highlighter, a language at a time; a token over lines is closed and reopened.
            "json {&lt;span data-t=\"p\"&gt;\"k\"&lt;/span&gt;: [&lt;span data-t=\"n\"&gt;1.5e3&lt;/span&gt;, &lt;span data-t=\"s\"&gt;\"v\"&lt;/span&gt;, &lt;span data-t=\"l\"&gt;true&lt;/span&gt;, &lt;span data-t=\"l\"&gt;null&lt;/span&gt;]} &lt;span data-t=\"c\"&gt;// c&lt;/span&gt;",
            "multi a &lt;span data-t=\"c\"&gt;/* one&lt;/span&gt;|&lt;span data-t=\"c\"&gt;two */&lt;/span&gt; b",
            "yaml &lt;span data-t=\"p\"&gt;name&lt;/span&gt;: x &lt;span data-t=\"c\"&gt;# note&lt;/span&gt;|- &lt;span data-t=\"p\"&gt;id&lt;/span&gt;: &lt;span data-t=\"n\"&gt;3&lt;/span&gt;|  &lt;span data-t=\"p\"&gt;on&lt;/span&gt;: &lt;span data-t=\"l\"&gt;yes&lt;/span&gt;",
            "sql &lt;span data-t=\"k\"&gt;SELECT&lt;/span&gt; id &lt;span data-t=\"k\"&gt;FROM&lt;/span&gt; t &lt;span data-t=\"k\"&gt;WHERE&lt;/span&gt; n &gt; &lt;span data-t=\"n\"&gt;2&lt;/span&gt; &lt;span data-t=\"c\"&gt;-- x&lt;/span&gt;",
            "html &lt;span data-t=\"t\"&gt;&lt;a&lt;/span&gt; &lt;span data-t=\"a\"&gt;href&lt;/span&gt;=&lt;span data-t=\"s\"&gt;\"/x\"&lt;/span&gt;&lt;span data-t=\"t\"&gt;&gt;&lt;/span&gt;hi&lt;span data-t=\"t\"&gt;&lt;/a&lt;/span&gt;&lt;span data-t=\"t\"&gt;&gt;&lt;/span&gt;",
            "plain &lt;b&gt;&amp;",
            // Code from state, shown as JSON, and following it.
            "state { \"a\": 1, \"list\": [ true, null ]} ",
            "follows k:const f:f n:2",
            "copy \"// hi\\nfn main() {\\n    let s = \\\"x\\\";\\n    println!(\\\"{}\\\", 42);\\n}\" Copied true Copied to the clipboard",
            // The JSON reader: where and why, the same in every browser.
            "check \"{\\\"a\\\": 1,}\" 1:8 Trailing comma before '}'",
            "check \"{a: 1}\" 1:2 Keys need double quotes",
            "check \"{'a': 1}\" 1:2 Keys take double quotes",
            "check \"{\\\"a\\\" 1}\" 1:6 Expected ':' after the key, found '1'",
            "check \"[1 2]\" 1:4 Expected ',' or ']', found '2'",
            "check \"{\\\"a\\\": \\\"x\" 1:7 Unterminated string",
            "check \"{\\\"a\\\": 01}\" 1:8 Invalid number",
            "check \"\\\"\\\\q\\\"\" 1:2 Bad escape \\q",
            "check \"\" 1:1 Enter a JSON value",
            "check \"  \" 1:3 Enter a JSON value",
            "check \"{\\\"a\\\": tru}\" 1:7 Unknown value 'tru': text takes double quotes",
            "check \"[1]]\" 1:4 Unexpected ']' after the value",
            "check \"{\\n  \\\"a\\\": [\\n    1,\\n  ]\\n}\" 3:6 Trailing comma before ']'",
            // The editor: errors wait for a pause in the typing (or a blur); fixes show at once.
            "editor Config \"{\\n  \\\"a\\\": 1,\\n  \\\"list\\\": [\\n    true,\\n    null\\n  ]\\n}\" 7 Valid JSON true",
            "ids true",
            "valid {\"a\":2} ok=true invalid=false aria=false err=- line=-",
            "typing {\"a\":2} ok=false invalid=false aria=false err=- line=-",
            "paused {\"a\":2} ok=false invalid=true aria=true err=Line 2, column 7: Unexpected '}' line=2",
            "fixed {\"a\":3} ok=true invalid=false aria=false err=- line=-",
            "type {\"a\":3} ok=false invalid=true aria=true err=Line 1, column 1: Expected an object, found an array line=1",
            // Format; on invalid text it shows the error and puts the caret there.
            "format \"{\\n  \\\"b\\\": [\\n    1,\\n    {\\n      \\\"c\\\": null\\n    }\\n  ],\\n  \\\"a\\\": \\\"x\\\"\\n}\" true 9",
            "format bad {\"b\":[1,{\"c\":null}],\"a\":\"x\"} ok=false invalid=true aria=true err=Line 3, column 3: Expected ',' or '}', found '\"' line=3 13 true",
            // Enter keeps the indent and opens a bracket; Tab and Shift+Tab; Escape lets Tab leave.
            "enter \"{\\n  \\n}\" 4",
            "tab \"{\\n    \\n}\"",
            "shift-tab \"{\\n  \\n}\" 4",
            "escape, tab false true",
            // State changed from outside: shown formatted.
            "outside \"{\\n  \\\"z\\\": [\\n    1\\n  ]\\n}\" {\"z\":[1]} ok=true invalid=false aria=false err=- line=- p:\"z\" n:1",
        ],
        "\n{result}"
    );
}

/// The rail layout's panel resized: dragging its edge, the keys within the limits,
/// Enter to fold it, a double click to reset, and the width to and from state.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn rail_panel_resize_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "section": "a", "open": true, "w": 300 },
        "root": { "type": "ui-rail-layout", "props": { "bind": "section", "bindOpen": "open", "height": 300, "panelWidth": 240, "resizable": true,
                "bindWidth": "w", "minPanelWidth": 200, "maxPanelWidth": 400, "items": [ { "id": "a", "label": "Alpha" } ] },
            "children": [ { "type": "ui-rail-panel", "props": { "for": "a" }, "children": [ { "type": "text", "props": { "value": "panel" } } ] },
                { "type": "text", "props": { "value": "main" } } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const L = document.querySelector("[data-i-x=ui-rail-layout]"), E = L.querySelector("[data-rail-resize]");
      const P = L.querySelector("[data-rail-panels]");
      // Without transitions: virtual time doesn't run them. The panel's own width, less its border.
      document.head.append(Object.assign(document.createElement("style"), { textContent: "* { transition: none !important }" }));
      const px = async () => { await wait(0); return Math.round(P.getBoundingClientRect().width) - 1; };
      const now = async () => `w=${S().w} panel=${await px()} aria=${E.getAttribute("aria-valuenow")} edge=${Math.round(E.getBoundingClientRect().left + 3 - L.getBoundingClientRect().left)}`;
      const key = (k, shift) => E.dispatchEvent(new KeyboardEvent("keydown", { key: k, shiftKey: !!shift, bubbles: true, cancelable: true }));
      const ptr = (type, x) => E.dispatchEvent(new PointerEvent(type, { bubbles: true, cancelable: true, pointerId: 3, button: 0, clientX: x, clientY: 50 }));
      try {
        log("handle", E.getAttribute("role"), E.getAttribute("aria-orientation"), E.getAttribute("aria-label"), E.getAttribute("aria-valuemin"), E.getAttribute("aria-valuemax"), E.getAttribute("aria-controls") === P.id);
        log("from state", await now());
        key("ArrowRight"); log("right", await now());
        key("ArrowLeft", true); log("shift-left", await now());
        key("End"); log("end", await now());
        key("Home"); log("home", await now());
        const x = E.getBoundingClientRect().left;
        ptr("pointerdown", x); ptr("pointermove", x + 90); log("dragging", L.hasAttribute("data-resizing"), S().w, await px()); ptr("pointerup", x + 90);
        log("dropped", await now(), L.hasAttribute("data-resizing"));
        ptr("pointerdown", x); ptr("pointermove", x + 900); ptr("pointerup", x + 900); log("past max", await now());
        E.dispatchEvent(new MouseEvent("dblclick", { bubbles: true })); log("reset", await now());
        root.__irender.set("w", 333); await wait(0); log("state", await now());
        E.focus(); key("Enter"); await wait(0); log("enter folds", S().open, L.hasAttribute("data-closed"), getComputedStyle(E).display, document.activeElement === L.querySelector("[data-rail-item=a]"));
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("rail-resize", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "handle separator vertical Resize the panel 200 400 true",
            "from state w=300 panel=300 aria=300 edge=356",
            "right w=316 panel=316 aria=316 edge=372",
            "shift-left w=312 panel=312 aria=312 edge=368",
            "end w=400 panel=400 aria=400 edge=456",
            "home w=200 panel=200 aria=200 edge=256",
            "dragging true 200 290",
            "dropped w=290 panel=290 aria=290 edge=346 false",
            "past max w=400 panel=400 aria=400 edge=456",
            "reset w=240 panel=240 aria=240 edge=296",
            "state w=333 panel=333 aria=333 edge=389",
            "enter folds false true none true",
        ],
        "\n{result}"
    );
}

/// The drawer: opened by its trigger; a short drag on its grip springs back, a long one
/// (or a flick) closes it; a mouse drag on its content does nothing, a touch drag does;
/// and a drag isn't a click on what it started on.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn drawer_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "n": 0 },
        "root": { "type": "ui-drawer", "props": { "trigger": "Open", "title": "Goal", "width": 400 }, "children": [
            { "type": "ui-button", "attrs": { "id": "inc" }, "props": { "label": "Add" }, "on": { "click": { "action": "set_state", "key": "n", "value": "{{ n + 1 }}" } } },
            { "type": "element", "attrs": { "id": "body" }, "style": { "height": "200px" }, "children": [ { "type": "text", "props": { "value": "content" } } ] } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      document.head.append(Object.assign(document.createElement("style"), { textContent: "* { transition: none !important }" }));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const D = document.querySelector("dialog"), G = D.querySelector("[data-drawer-grip]");
      let dismissed = 0; D.addEventListener("dismiss", () => dismissed++);
      const ptr = (t, type, y, kind = "mouse", ts) => t.dispatchEvent(new PointerEvent(type, { bubbles: true, cancelable: true, pointerId: 5, pointerType: kind, button: 0, clientX: 100, clientY: y }));
      const drag = async (t, from, to, kind, steps = 4) => {
        ptr(t, "pointerdown", from, kind);
        for (let i = 1; i <= steps; i++) { await wait(30); ptr(t, "pointermove", from + ((to - from) * i) / steps, kind); }
        const mid = D.style.transform, dragging = D.hasAttribute("data-dragging");
        ptr(t, "pointerup", to, kind);
        return `${mid || "-"} ${dragging}`;
      };
      try {
        const trigger = document.querySelector("button[commandfor]");
        log("trigger", trigger.textContent, trigger.getAttribute("aria-haspopup"), D.id === trigger.getAttribute("commandfor"));
        log("drawer", D.getAttribute("closedby"), D.getAttribute("aria-labelledby") === D.querySelector("h2").id, G.getAttribute("aria-hidden"), getComputedStyle(D).getPropertyValue("--drawer-width").trim());
        D.showModal(); await wait(0);
        log("open", D.open, getComputedStyle(D).transform);
        const top = D.getBoundingClientRect().top, h = D.offsetHeight;
        log("short drag", await drag(G, top + 5, top + 25, "mouse", 4), D.open, D.style.transform || "-");
        log("long drag", await drag(G, top + 5, top + 5 + h * 0.5, "mouse", 4), D.open, dismissed);
        D.showModal(); await wait(0);
        // A quick flick: a short distance, fast.
        ptr(G, "pointerdown", top + 5); await wait(10); ptr(G, "pointermove", top + 20); await wait(10); ptr(G, "pointermove", top + 45); ptr(G, "pointerup", top + 45);
        log("flick", D.open, dismissed);
        D.showModal(); await wait(0);
        const body = D.querySelector("#body");
        log("mouse on content", await drag(body, top + 100, top + 100 + h * 0.6, "mouse"), D.open);
        log("touch on content", await drag(body, top + 100, top + 100 + h * 0.6, "touch"), D.open);
        // A drag that starts on a button doesn't press it.
        D.showModal(); await wait(0);
        const inc = D.querySelector("#inc");
        await drag(inc, top + 40, top + 50, "touch");
        inc.click(); await wait(0);
        log("no click after a drag", S().n, D.open);
        inc.click(); await wait(0);
        log("a tap still clicks", S().n);
        // Escape, as any dialog.
        D.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })); D.close(); log("closed", D.open);
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("drawer", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "trigger Open dialog true",
            "drawer any true true 400px",
            "open true none",
            "short drag translateY(20px) true true -",
            "long drag translateY(171.5px) true false 1",
            "flick false 2",
            "mouse on content - false true",
            "touch on content translateY(205.8px) true false",
            "no click after a drag 0 true",
            "a tap still clicks 1",
            "closed false",
        ],
        "\n{result}"
    );
}

/// Moving events with `bindEvents`: dragged in the month and the week, resized by the
/// foot, moved with Alt and the arrows; the list goes back with dates written as they
/// were; a press that doesn't move still opens the event.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn event_calendar_moves_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "opened": null, "list": [
            { "id": "a", "title": "Standup", "start": "2026-11-04T09:00", "end": "2026-11-04T09:30" },
            { "id": "b", "title": "Call", "start": "2026-11-05T14:00" },
            { "id": "c", "title": "Offsite", "start": "2026-11-09", "end": "2026-11-10" } ] },
        "root": { "type": "element", "style": { "width": "900px" }, "children": [
            { "type": "ui-event-calendar", "props": { "events": "{{ list }}", "bindEvents": "list", "bindEvent": "opened", "date": "2026-11-04", "today": "2026-11-04", "hours": [8, 18], "snap": 15, "view": "month" } } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const C = document.querySelector("[data-i-x=ui-event-calendar]");
      const list = () => S().list.map((e) => `${e.id}:${e.start}${e.end ? "→" + e.end : ""}`).join(" ");
      const chip = (id) => C.querySelector(`[data-ev-event="${id}"]`);
      const mid = (el) => { const r = el.getBoundingClientRect(); return [r.left + r.width / 2, r.top + Math.min(8, r.height / 2)]; };
      const ptr = (t, type, x, y) => t.dispatchEvent(new PointerEvent(type, { bubbles: true, cancelable: true, pointerId: 9, button: 0, clientX: x, clientY: y }));
      const drag = async (el, dx, dy) => {
        const [x, y] = mid(el);
        ptr(el, "pointerdown", x, y);
        for (let i = 1; i <= 4; i++) ptr(window, "pointermove", x + (dx * i) / 4, y + (dy * i) / 4);
        const drop = [...C.querySelectorAll("[data-ev-drop]")].map((c) => c.getAttribute("data-ev-day") || c.getAttribute("data-ev-cell-date")).join(",") || "-";
        ptr(window, "pointerup", x + dx, y + dy);
        el.click(); // the click a real drag ends with: swallowed
        await wait(0);
        return drop;
      };
      const say = () => C.querySelector("[data-ev-live]").textContent;
      try {
        log("editable", C.hasAttribute("data-editable"));
        // Month: a press without a move opens the event.
        const a = chip("a"); const [ax, ay] = mid(a);
        ptr(a, "pointerdown", ax, ay); ptr(window, "pointerup", ax, ay); a.click(); await wait(0);
        log("click opens", S().opened?.id, list());
        // Month: two days on.
        const w = C.querySelector("[data-ev-month] [data-ev-day]").getBoundingClientRect().width;
        log("month drop", await drag(chip("a"), w * 2, 0), list(), S().opened?.id);
        await wait(40); log("said", say());
        // An all-day event a week on, its end with it.
        const h = C.querySelector("[data-ev-month] [data-ev-week]").getBoundingClientRect().height;
        log("all-day", await drag(chip("c"), 0, h), list());
        // Week: an hour later and a day on; then the event with no end made longer.
        C.querySelector("[data-ev-today]").click(); C.querySelector("[data-ev-view=week]").click(); await wait(0);
        const col = C.querySelector("[data-ev-date-col]").getBoundingClientRect();
        const hour = col.height / 10;
        log("week move", await drag(chip("b"), col.width, hour), list());
        const foot = chip("b").querySelector("[data-ev-resize]");
        log("resize", foot.getAttribute("aria-hidden"), await drag(foot, 0, hour), list());
        // The keys: Alt+Down a snap later, Alt+Shift+Up a snap shorter, Alt+Left a day back.
        const key = (k, shift) => document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: k, altKey: true, shiftKey: !!shift, bubbles: true, cancelable: true }));
        chip("b").focus(); key("ArrowDown"); log("alt-down", list(), document.activeElement.getAttribute("data-ev-event"));
        key("ArrowUp", true); log("alt-shift-up", list());
        key("ArrowLeft"); log("alt-left", list(), document.activeElement.getAttribute("data-ev-event"), document.activeElement.tabIndex);
        await wait(40); log("said", say());
        // Plain arrows on an event don't move it.
        document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true, cancelable: true })); log("no alt", list());
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("calendar-moves", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "editable true",
            "click opens a a:2026-11-04T09:00→2026-11-04T09:30 b:2026-11-05T14:00 c:2026-11-09→2026-11-10",
            "month drop 2026-11-06 a:2026-11-06T09:00→2026-11-06T09:30 b:2026-11-05T14:00 c:2026-11-09→2026-11-10 a",
            "said Standup moved to Friday, November 6, 9:00 AM to 9:30 AM",
            "all-day 2026-11-16 a:2026-11-06T09:00→2026-11-06T09:30 b:2026-11-05T14:00 c:2026-11-16→2026-11-17",
            "week move - a:2026-11-06T09:00→2026-11-06T09:30 b:2026-11-06T15:00 c:2026-11-16→2026-11-17",
            "resize true - a:2026-11-06T09:00→2026-11-06T09:30 b:2026-11-06T15:00→2026-11-06T16:30 c:2026-11-16→2026-11-17",
            "alt-down a:2026-11-06T09:00→2026-11-06T09:30 b:2026-11-06T15:15→2026-11-06T16:45 c:2026-11-16→2026-11-17 b",
            "alt-shift-up a:2026-11-06T09:00→2026-11-06T09:30 b:2026-11-06T15:15→2026-11-06T16:30 c:2026-11-16→2026-11-17",
            "alt-left a:2026-11-06T09:00→2026-11-06T09:30 b:2026-11-05T15:15→2026-11-05T16:30 c:2026-11-16→2026-11-17 b 0",
            "said Call moved to Thursday, November 5, 3:15 PM to 4:30 PM",
            "no alt a:2026-11-06T09:00→2026-11-06T09:30 b:2026-11-05T15:15→2026-11-05T16:30 c:2026-11-16→2026-11-17",
        ],
        "\n{result}"
    );
}

/// The data table's layout tools: rows grouped (headers with counts and subtotals,
/// folding, Group by in View, paging over groups), the band row, columns moved and
/// resized (and kept in `bindColumns`, both ways), and CSV export.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn data_table_groups_columns_and_export_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "g": "region", "cols": null, "rows": [
            { "id": 1, "name": "Ada", "region": "North", "net": 100, "note": "=SUM(A1)" },
            { "id": 2, "name": "Bo, Jr", "region": "South", "net": 50, "note": "say \"hi\"" },
            { "id": 3, "name": "Cy", "region": "North", "net": 25, "note": "" },
            { "id": 4, "name": "Di", "region": null, "net": 10, "note": "-" },
            { "id": 5, "name": "Ed", "region": "South", "net": 5, "note": "ok" } ] },
        "root": { "type": "ui-data-table", "props": { "rows": "{{ rows }}", "bindGroupBy": "g", "bindColumns": "cols", "resizable": true, "reorderable": true,
                "export": "people", "pageSize": 0, "title": "People" },
            "children": [
                { "type": "ui-data-table-column", "props": { "field": "name", "label": "Name" } },
                { "type": "ui-data-table-column", "props": { "field": "region", "label": "Region", "groupable": true } },
                { "type": "ui-data-table-column", "props": { "field": "net", "label": "Net", "group": "Money", "summary": "sum" } },
                { "type": "ui-data-table-column", "props": { "field": "note", "label": "Note", "group": "Money" } } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      document.head.append(Object.assign(document.createElement("style"), { textContent: "* { transition: none !important }" }));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const T = document.querySelector("[data-i-x=ui-data-table]");
      const lines = () => [...T.querySelectorAll("[data-dt-body] > tr:not([hidden])")].map((tr) => tr.hasAttribute("data-dt-group")
        ? `[${[...tr.cells].map((c) => c.textContent.trim()).filter(Boolean).join(" / ")}]`
        : tr.cells[0].textContent.trim()).join(" ");
      const head = () => [...T.querySelectorAll("thead tr:last-child th:not([hidden])")].map((t) => t.textContent.trim()).join(",");
      const bands = () => [...T.querySelectorAll("[data-dt-bands] th[data-dt-band]")].map((t) => `${t.textContent || "-"}:${t.colSpan}`).join(",");
      const cols = () => JSON.stringify(S().cols);
      try {
        log("grouped", lines());
        const fold = T.querySelector("[data-dt-fold]");
        log("fold button", fold.getAttribute("aria-expanded"), T.querySelector("[data-dt-group-count]").getAttribute("aria-label"));
        fold.click(); await wait(0);
        log("folded", lines(), document.activeElement === T.querySelector("[data-dt-fold]"));
        T.querySelector("[data-dt-fold]").click(); await wait(0);
        // Sorting by the group field the other way puts the groups the other way (blanks still last).
        T.querySelector('[data-dt-sort="region"]').click(); T.querySelector('[data-dt-sort="region"]').click(); await wait(0);
        log("desc", lines());
        T.querySelector('[data-dt-sort="region"]').click(); await wait(0);
        // View → Group by: None.
        const none = T.querySelector('[data-dt-group-by][value=""]');
        log("radios", [...T.querySelectorAll("[data-dt-group-by]")].map((r) => (r.value || "none") + (r.checked ? "*" : "")).join(","));
        none.checked = true; none.dispatchEvent(new Event("change", { bubbles: true })); await wait(0);
        log("ungrouped", S().g, lines());
        root.__irender.set("g", "region"); await wait(0);
        // The band row.
        log("bands", bands(), head());
        // Moving: Alt+Right on Name; the layout goes to state.
        const nameSort = T.querySelector('[data-dt-sort="name"]');
        nameSort.focus();
        nameSort.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", altKey: true, bubbles: true, cancelable: true })); await wait(0);
        log("moved", head(), bands(), cols(), document.activeElement === nameSort, T.querySelector("[data-dt-footer] [data-dt-live]").textContent);
        log("cells follow", [...T.querySelector("[data-dt-body] > [data-dt-row]:not([hidden])").cells].map((c) => c.textContent.trim()).join(","));
        // Resizing with the keys; a double click resets.
        const rz = T.querySelector('[data-dt-resizer="net"]');
        const w = () => Math.round(T.querySelector('th[data-dt-col="net"]').getBoundingClientRect().width);
        const before = w();
        rz.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true, cancelable: true })); await wait(0);
        log("resized", w() - before, S().cols.widths.net - before, rz.getAttribute("role"), rz.getAttribute("aria-label"), rz.getAttribute("aria-valuenow") === String(S().cols.widths.net));
        rz.dispatchEvent(new MouseEvent("dblclick", { bubbles: true })); await wait(0);
        log("reset", JSON.stringify(S().cols.widths), w() === before);
        // Hiding a column: the band shrinks, and state has it.
        const show = T.querySelector('[data-dt-show="note"]'); show.checked = false; show.dispatchEvent(new Event("change", { bubbles: true })); await wait(0);
        log("hidden", bands(), JSON.stringify(S().cols.hidden));
        // A layout from state.
        root.__irender.set("cols", { order: ["net", "name", "region", "note"], hidden: [], widths: { name: 200 } }); await wait(0);
        log("from state", head(), bands(), Math.round(T.querySelector('th[data-dt-col="name"]').getBoundingClientRect().width));
        // Export: the rows that pass, in the order and columns shown, as CSV.
        let got = null;
        T.addEventListener("export", (e) => (got = e.detail));
        const click = HTMLAnchorElement.prototype.click; let downloaded = null;
        HTMLAnchorElement.prototype.click = function () { downloaded = this.download; };
        T.querySelector("[data-dt-export]").click();
        HTMLAnchorElement.prototype.click = click;
        log("export", got.file, got.rows, downloaded, got.csv.charCodeAt(0) === 0xfeff);
        for (const l of got.csv.slice(1).split("\r\n")) log("csv", JSON.stringify(l));
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("table-layout", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "grouped [Region: North2 / 125] Ada Cy [Region: South2 / 55] Bo, Jr Ed [Region: (none)1 / 10] Di",
            "fold button true 2 rows",
            "folded [Region: North2 / 125] [Region: South2 / 55] Bo, Jr Ed [Region: (none)1 / 10] Di true",
            "desc [Region: South2 / 55] Bo, Jr Ed [Region: North2 / 125] Ada Cy [Region: (none)1 / 10] Di",
            "radios none,region*",
            "ungrouped  Ada Bo, Jr Cy Di Ed",
            "bands -:2,Money:2 Name,Region,Net,Note",
            "moved Region,Name,Net,Note -:2,Money:2 {\"order\":[\"region\",\"name\",\"net\",\"note\"],\"widths\":{},\"hidden\":[]} true Name moved to column 2 of 4",
            "cells follow North,Ada,100,=SUM(A1)",
            "resized 16 16 separator Resize Net true",
            "reset {} true",
            "hidden -:2,Money:1 [\"note\"]",
            "from state Net,Name,Region,Note Money:1,-:2,Money:1 200",
            "export people.csv 5 people.csv true",
            "csv \"Net,Name,Region,Note\"",
            "csv \"100,Ada,North,'=SUM(A1)\"",
            "csv \"25,Cy,North,\"",
            "csv \"50,\\\"Bo, Jr\\\",South,\\\"say \\\"\\\"hi\\\"\\\"\\\"\"",
            "csv \"5,Ed,South,ok\"",
            "csv \"10,Di,,'-\"",
            "csv \"\"",
        ],
        "\n{result}"
    );
}

/// A document's `lang` and `labels`: the server's text and the browser's in the
/// document's words, plural forms picked by the language's rules (Polish has `few` and
/// `many`), English for keys it doesn't give, and numbers in its way.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn labels_in_chrome() {
    let rows: Vec<_> = (1..=12).map(|i| serde_json::json!({ "id": i, "name": format!("R{i}") })).collect();
    let doc = serde_json::json!({ "version": 1, "lang": "pl",
        "labels": {
            "Rows per page": "Wierszy na stronę",
            "Page {page} of {pages}": "Strona {page} z {pages}",
            "{count} of {total} row(s) selected.": "Zaznaczono {count} z {total}",
            "{n} results": { "one": "{n} wynik", "few": "{n} wyniki", "many": "{n} wyników", "other": "{n} wyniku" } },
        "state": { "rows": rows, "picked": [] },
        "root": { "type": "ui-data-table", "props": { "rows": "{{ rows }}", "selectable": true, "bindSelection": "picked", "pageSize": 10, "search": true },
            "children": [ { "type": "ui-data-table-column", "props": { "field": "name", "label": "Name" } } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]");
      const T = document.querySelector("[data-i-x=ui-data-table]");
      const L = window.__irender.label;
      try {
        log("lang", document.documentElement.lang);
        log("page", T.querySelector("[data-dt-page]").textContent, "|", T.querySelector("[data-dt-footer]").textContent.includes("Wierszy na stronę"));
        for (const n of [1, 5]) {
          root.__irender.set("picked", Array.from({ length: n }, (_, i) => i + 1)); await wait(0);
          log("selected", T.querySelector("[data-dt-status]").textContent);
        }
        // Next page: the browser's text.
        T.querySelector('[data-dt-go="next"]').click(); await wait(0);
        log("next", T.querySelector("[data-dt-page]").textContent);
        log("plural", L(["{n} result", "{n} results"], { n: 1 }), "|", L(["{n} result", "{n} results"], { n: 3 }), "|", L(["{n} result", "{n} results"], { n: 7 }), "|", L(["{n} result", "{n} results"], { n: 1.5 }));
        log("english", L("Clear filter"), "|", L(["{n} row", "{n} rows"], { n: 1 }), "|", L("Select all {n}", { n: 1234.5 }));
        log("kept", L("{missing} stays", {}));
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let page = engine.render(&ir, &serde_json::json!({})).unwrap();
    assert!(page.document("t").contains(r#"<html lang="pl">"#));
    assert!(page.body.contains("Strona 1 z 2"), "the server's text is in the document's words");
    let html = page.document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("labels", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "lang pl",
            "page Strona 1 z 2 | true",
            "selected Zaznaczono 1 z 12",
            "selected Zaznaczono 5 z 12",
            "next Strona 2 z 2",
            "plural 1 wynik | 3 wyniki | 7 wyników | 1,5 wyniku",
            "english Clear filter | 1 row | Select all 1234,5",
            "kept {missing} stays",
        ],
        "\n{result}"
    );
}

/// Right to left (`lang: ar`): the page's `dir`, mirrored icons and the switch's thumb,
/// Left and Right swapped in the calendar and the tree, a resizable panel dragged the
/// other way, and a pinned column on the right.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn right_to_left_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "lang": "ar", "state": { "sizes": [30, 70], "on": true, "day": null, "node": null },
        "root": { "type": "stack", "style": { "width": "900px" }, "children": [
            { "type": "ui-switch", "props": { "bind": "on", "label": "On" } },
            { "type": "ui-event-calendar", "attrs": { "id": "cal" }, "props": { "events": [], "date": "2026-11-04", "today": "2026-11-04", "bindDay": "day", "view": "month" } },
            { "type": "ui-tree", "attrs": { "id": "tree" }, "props": { "nodes": [ { "id": "a", "label": "A", "children": [ { "id": "b", "label": "B" } ] } ], "bindNode": "node" } },
            { "type": "ui-resizable", "attrs": { "id": "rz" }, "props": { "bind": "sizes", "height": 100 }, "children": [
                { "type": "ui-resizable-panel", "children": [ { "type": "text", "props": { "value": "one" } } ] },
                { "type": "ui-resizable-panel", "children": [ { "type": "text", "props": { "value": "two" } } ] } ] },
            { "type": "ui-data-table", "attrs": { "id": "dt" }, "props": { "rows": [ { "a": 1, "b": 2 } ] }, "children": [
                { "type": "ui-data-table-column", "props": { "field": "a", "pin": "left" } },
                { "type": "ui-data-table-column", "props": { "field": "b" } } ] } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      document.head.append(Object.assign(document.createElement("style"), { textContent: "* { transition: none !important }" }));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const key = (t, k) => t.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true }));
      try {
        log("dir", document.documentElement.dir, root.getAttribute("dir"), getComputedStyle(document.querySelector("#cal")).direction);
        // The switch's thumb goes left when on; a previous arrow points right.
        const thumb = getComputedStyle(document.querySelector("input[role=switch], [data-ui-switch] input, input[type=checkbox]"), "::before").transform;
        log("thumb", thumb);
        const icon = document.querySelector("#cal svg[data-flip]");
        log("icon", getComputedStyle(icon).scale);
        // The calendar: Left is the next day.
        const day = document.querySelector('#cal [data-ev-day="2026-11-04"]');
        day.focus(); key(day, "ArrowLeft"); await wait(0);
        log("calendar left", document.activeElement.getAttribute("data-ev-day"));
        key(document.activeElement, "ArrowRight"); key(document.activeElement, "ArrowRight"); await wait(0);
        log("calendar right", document.activeElement.getAttribute("data-ev-day"));
        // The tree: Left opens a node.
        const item = () => document.querySelector('#tree [data-tree-id="a"]');
        const before = item().getAttribute("aria-expanded");
        item().focus(); key(item(), "ArrowRight"); await wait(0);
        const afterRight = item().getAttribute("aria-expanded");
        item().focus(); key(item(), "ArrowLeft"); await wait(0);
        log("tree", before, "right:", afterRight, "left:", item().getAttribute("aria-expanded"));
        // The resizable: the first panel is on the right; dragging left makes it wider.
        const rz = document.querySelector("#rz"), h = rz.querySelector("[data-rz-handle]"), first = rz.querySelector("[data-rz-panel]");
        log("first panel on the right", first.getBoundingClientRect().left > h.getBoundingClientRect().left);
        const x = h.getBoundingClientRect().left;
        const ptr = (type, cx) => h.dispatchEvent(new PointerEvent(type, { bubbles: true, cancelable: true, pointerId: 4, button: 0, clientX: cx, clientY: 10 }));
        ptr("pointerdown", x); ptr("pointermove", x - 90); ptr("pointerup", x - 90);
        log("drag left widens", JSON.stringify(S().sizes), S().sizes[0] > 30);
        key(h, "ArrowLeft"); log("left key widens", S().sizes[0] > JSON.parse(JSON.stringify(S().sizes))[0] - 1, S().sizes[0]);
        // The pinned column sticks to the right.
        const pinned = document.querySelector('#dt th[data-dt-col="a"]');
        log("pinned", pinned.style.insetInlineStart || "-", pinned.style.left || "-", getComputedStyle(pinned).right);
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("rtl", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "dir rtl rtl rtl",
            "thumb matrix(1, 0, 0, 1, -16, 0)",
            "icon -1 1",
            "calendar left 2026-11-05",
            "calendar right 2026-11-03",
            "tree false right: false left: true",
            "first panel on the right true",
            "drag left widens [40.01,59.99] true",
            "left key widens true 45.01",
            "pinned 0px - 0px",
        ],
        "\n{result}"
    );
}

/// Tiles placed by the server, moved and resized by dragging and with the keys, pushing
/// the others out of the way (but not a locked one), the layout in state both ways,
/// Escape putting a dragged tile back, and stacking on a narrow grid.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn dashboard_grid_in_chrome() {
    let tile = |id: &str, extra: serde_json::Value| {
        let mut props = serde_json::json!({ "id": id });
        props.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        serde_json::json!({ "type": "ui-tile", "props": props, "children": [ { "type": "text", "props": { "value": id } } ] })
    };
    let doc = serde_json::json!({ "version": 1, "state": { "layout": null, "moves": 0 },
        "root": { "type": "element", "style": { "width": "960px" }, "children": [
            { "type": "ui-dashboard-grid", "attrs": { "id": "g" }, "props": { "bind": "layout", "gap": 0, "rowHeight": 50 },
              "on": { "layout": { "action": "set_state", "key": "moves", "value": "{{ moves + 1 }}" } },
              "children": [
                tile("a", serde_json::json!({ "title": "A", "x": 0, "y": 0, "w": 6, "h": 2 })),
                tile("b", serde_json::json!({ "title": "B", "x": 6, "y": 0, "w": 6, "h": 2, "maxW": 8 })),
                tile("c", serde_json::json!({ "x": 0, "y": 2, "w": 12, "h": 1 })),
                tile("lock", serde_json::json!({ "title": "L", "x": 0, "y": 3, "w": 4, "h": 1, "locked": true })),
                tile("d", serde_json::json!({ "title": "D", "w": 4, "h": 1 })) ] },
            { "type": "ui-dashboard-grid", "attrs": { "id": "s" }, "props": { "stackBelow": 2000 }, "children": [
                tile("one", serde_json::json!({ "title": "One", "x": 6, "y": 0, "w": 6, "h": 2 })),
                tile("two", serde_json::json!({ "title": "Two", "x": 0, "y": 0, "w": 6, "h": 1 })) ] },
            { "type": "element", "attrs": { "dir": "rtl" }, "style": { "width": "960px" }, "children": [
                { "type": "ui-dashboard-grid", "attrs": { "id": "r" }, "props": { "gap": 0 }, "children": [ tile("r1", serde_json::json!({ "title": "R", "x": 0, "y": 0, "w": 4, "h": 1 })) ] } ] } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const G = document.getElementById("g"), T = (id) => G.querySelector(`[data-id=${id}]`);
      const at = () => G.__x.layout.map((t) => `${t.id}:${t.x},${t.y},${t.w}x${t.h}`).join(" ");
      // Where the grid puts the tiles, in cells of 80 by 50 px (offsets leave out the
      // glide, which headless Chrome doesn't run).
      const seen = () => [...G.querySelectorAll("[data-dg-tile]")].map((n) =>
        `${n.dataset.id}:${Math.round(n.offsetLeft / 80)},${Math.round(n.offsetTop / 50)},${Math.round(n.offsetWidth / 80)}x${Math.round(n.offsetHeight / 50)}`).join(" ");
      const key = (n, k, shift) => n.dispatchEvent(new KeyboardEvent("keydown", { key: k, shiftKey: !!shift, bubbles: true, cancelable: true }));
      const ptr = (t, type, x, y) => t.dispatchEvent(new PointerEvent(type, { bubbles: true, cancelable: true, pointerId: 7, button: 0, clientX: x, clientY: y }));
      const drag = (handle, dx, dy, end = "pointerup") => {
        const r = handle.getBoundingClientRect(), x = r.left + 5, y = r.top + 5;
        ptr(handle, "pointerdown", x, y); ptr(handle, "pointermove", x + dx / 2, y + dy / 2); ptr(handle, "pointermove", x + dx, y + dy);
        if (end) ptr(handle, end, x + dx, y + dy);
      };
      const head = (id) => T(id).querySelector("[data-dg-handle]");
      try {
        await wait(50);
        log("server", seen());
        const h = head("a");
        log("handle", h.tagName, h.getAttribute("role"), h.getAttribute("aria-roledescription"), h.getAttribute("aria-label"), !!document.getElementById(h.getAttribute("aria-describedby")), h.tabIndex);
        log("grip only", !!T("c").querySelector("[data-dg-grip-only][data-dg-handle]"), "locked", !!T("lock").querySelector("[data-dg-handle], [data-dg-resize]"));

        // c dragged up over a and b: they go under it.
        drag(T("c").querySelector("[data-dg-handle]"), 0, -100);
        await wait(250);
        log("c up", at(), "|", S().moves, JSON.stringify(S().layout[2]));
        // a dragged down past c: they swap.
        drag(head("a"), 0, 60);
        await wait(250);
        log("a down", at());
        // Resized from the corner: wider pushes b down; b can't pass its maxW.
        drag(T("a").querySelector("[data-dg-resize]"), 160, 50);
        await wait(250);
        log("a wider", at());
        // b can't pass its maxW, nor the grid's edge.
        drag(head("b"), -480, 0);
        drag(T("b").querySelector("[data-dg-resize]"), 640, 0);
        await wait(250);
        log("b max", at());
        // Escape puts it back.
        drag(head("b"), 0, -200, null);
        log("dragging", T("b").hasAttribute("data-dragging"), !G.querySelector("[data-dg-ghost]").hidden);
        key(head("b"), "Escape");
        await wait(250);
        log("escape", at(), T("b").hasAttribute("data-dragging"));

        // The keys: a step, past a tile, and Shift to resize.
        root.__irender.set("layout", null);
        await wait(250);
        log("reset", at(), "|", seen());
        key(head("a"), "ArrowDown"); await wait(250); log("down", at());
        key(head("a"), "ArrowUp"); await wait(250); log("up", at());
        key(head("d"), "ArrowLeft"); await wait(250); log("d left", at());
        key(head("a"), "ArrowLeft", true); key(head("a"), "ArrowDown", true); await wait(250); log("shift", at());
        log("said", G.querySelector("[data-dg-live]").textContent);

        // A layout from state.
        root.__irender.set("layout", [{ id: "b", x: 0, y: 0, w: 12, h: 1 }, { id: "a", x: 0, y: 5, w: 3, h: 1 }]);
        await wait(250);
        log("state", at(), "|", seen());

        // Narrow: one column in reading order, nothing to drag.
        const St = document.getElementById("s"), tiles = [...St.querySelectorAll("[data-dg-tile]")];
        log("stacked", getComputedStyle(St.querySelector("[data-dg-grid]")).gridTemplateColumns.split(" ").length,
          tiles.map((n) => n.dataset.id + ":" + getComputedStyle(n).order + ":" + Math.round(n.getBoundingClientRect().height)).join(" "),
          getComputedStyle(tiles[0].querySelector("[data-dg-resize]")).display);
        const before = St.__x.layout;
        drag(tiles[0].querySelector("[data-dg-handle]"), 0, 200);
        log("stacked drag", JSON.stringify(St.__x.layout) === JSON.stringify(before));

        // Right to left: column 0 is on the right, and Left moves a tile on from it.
        const R = document.getElementById("r"), r1 = R.querySelector("[data-dg-tile]"), edge = () => Math.round(R.getBoundingClientRect().right - r1.getBoundingClientRect().right);
        log("rtl", edge());
        key(r1.querySelector("[data-dg-handle]"), "ArrowLeft"); await wait(250);
        drag(r1.querySelector("[data-dg-handle]"), -160, 0); await wait(250);
        log("rtl moved", JSON.stringify(R.__x.layout[0]), Math.round(R.getBoundingClientRect().right - (r1.offsetLeft + r1.offsetWidth + R.querySelector("[data-dg-grid]").getBoundingClientRect().left)));
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("dashboard-grid", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "server a:0,0,6x2 b:6,0,6x2 c:0,2,12x1 lock:0,3,4x1 d:4,3,4x1",
            "handle HEADER button movable tile Move A true 0",
            "grip only true locked false",
            "c up a:0,1,6x2 b:6,1,6x2 c:0,0,12x1 lock:0,3,4x1 d:4,3,4x1 | 1 {\"id\":\"c\",\"x\":0,\"y\":0,\"w\":12,\"h\":1}",
            "a down a:0,4,6x2 b:6,1,6x2 c:0,0,12x1 lock:0,3,4x1 d:4,3,4x1",
            "a wider a:0,4,8x3 b:6,1,6x2 c:0,0,12x1 lock:0,3,4x1 d:4,3,4x1",
            "b max a:0,4,8x3 b:0,1,8x2 c:0,0,12x1 lock:0,3,4x1 d:4,3,4x1",
            "dragging true true",
            "escape a:0,4,8x3 b:0,1,8x2 c:0,0,12x1 lock:0,3,4x1 d:4,3,4x1 false",
        // Cleared state: every tile back where its props say.
            "reset a:0,0,6x2 b:6,0,6x2 c:0,2,12x1 lock:0,3,4x1 d:4,3,4x1 | a:0,0,6x2 b:6,0,6x2 c:0,2,12x1 lock:0,3,4x1 d:4,3,4x1",
        // A step down goes past the tiles below, and under the locked one.
            "down a:0,4,6x2 b:6,0,6x2 c:0,6,12x1 lock:0,3,4x1 d:4,2,4x1",
            "up a:0,0,6x2 b:6,0,6x2 c:0,4,12x1 lock:0,3,4x1 d:4,2,4x1",
            "d left a:0,0,6x2 b:6,0,6x2 c:0,4,12x1 lock:0,3,4x1 d:3,2,4x1",
            "shift a:0,0,5x3 b:6,0,6x2 c:0,5,12x1 lock:0,3,4x1 d:3,4,4x1",
            "said A: column 1, row 1, 5 by 3",
        // From state: b is cut to its maxW, and a floats up to the locked tile.
            "state a:0,4,3x1 b:0,0,8x1 c:0,1,12x1 lock:0,3,4x1 d:4,2,4x1 | a:0,4,3x1 b:0,0,8x1 c:0,1,12x1 lock:0,3,4x1 d:4,2,4x1",
        // One column in reading order, h rows tall (80 + 16 + 80), no resize corner.
            "stacked 1 one:1:176 two:0:80 none",
            "stacked drag true",
            "rtl 0",
            // One step with the key and two by dragging, both leftward.
            r#"rtl moved {"id":"r1","x":3,"y":0,"w":4,"h":1} 240"#,
        ],
        "\n{result}"
    );
}

/// Saved views: a ready-made one the table opens with, the "(edited)" mark and undo,
/// saving a new view (with a hidden column and a sort), saving changes, renaming,
/// switching between views and the starting table, and deleting.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn data_table_saved_views_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "views": null, "cur": null, "rows": [
            { "id": 1, "name": "Ada", "status": "Paid", "net": 100 },
            { "id": 2, "name": "Bo", "status": "Due", "net": 50 },
            { "id": 3, "name": "Cy", "status": "Paid", "net": 25 },
            { "id": 4, "name": "Di", "status": "Due", "net": 10 } ] },
        "root": { "type": "ui-data-table", "props": { "rows": "{{ rows }}", "bindViews": "views", "bindCurrentView": "cur", "defaultView": "paid", "pageSize": 0, "search": true,
                "views": [ { "id": "paid", "name": "Paid", "filters": { "status": ["Paid"] }, "sort": "-net" } ] },
            "children": [
                { "type": "ui-data-table-column", "props": { "field": "name", "label": "Name" } },
                { "type": "ui-data-table-column", "props": { "field": "status", "label": "Status", "filter": "select" } },
                { "type": "ui-data-table-column", "props": { "field": "net", "label": "Net" } } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const T = document.querySelector("[data-i-x=ui-data-table]");
      const $ = (s) => T.querySelector(s);
      const rows = () => [...T.querySelectorAll("[data-dt-body] > tr:not([hidden])")].map((tr) => tr.cells[0].textContent.trim()).join(",");
      const head = () => [...T.querySelectorAll("thead th[data-dt-col]:not([hidden])")].map((t) => t.textContent.trim()).join(",");
      const label = () => $("[data-dt-views-label]").textContent;
      const menu = () => [...T.querySelectorAll("[data-dt-views] [data-dt-view]")].map((r) => {
        const b = r.querySelector("[data-dt-view-pick]");
        return b.textContent + (b.getAttribute("aria-current") === "true" ? "*" : "") + (r.querySelector("[data-dt-view-delete]") ? "(own)" : "");
      }).join(" | ") + " :: " + [...T.querySelectorAll("[data-dt-view-actions] button")].map((b) => b.textContent).join(" | ");
      const open = async () => { if (!$("[data-dt-views]").matches(":popover-open")) $("[data-dt-views-button]").click(); await wait(0); };
      const type = async (text) => {
        const box = $("[data-dt-view-name]");
        box.value = text;
        box.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
        await wait(0);
      };
      try {
        await wait(0);
        log("opens with", label(), rows(), S().cur);
        await open();
        log("menu", menu());
        // A change marks the view; Undo puts it back.
        $('[data-dt-sort="name"]').click(); await wait(0);
        log("sorted", label(), rows());
        await open(); log("edited menu", menu());
        $("[data-dt-view-revert]").click(); await wait(0);
        log("undone", label(), rows());
        // A new view: hide Net, sort by name, save it.
        const net = $('[data-dt-show="net"]'); net.checked = false; net.dispatchEvent(new Event("change", { bubbles: true }));
        $('[data-dt-sort="name"]').click(); await wait(0);
        await open(); $("[data-dt-view-new]").click(); await wait(0);
        log("name box", document.activeElement === $("[data-dt-view-name]"), $("[data-dt-view-name]").getAttribute("aria-label"));
        await type("Paid by name");
        log("saved", label(), S().cur, JSON.stringify(S().views));
        log("own menu", menu());
        // Changes to an own view can be saved.
        $('[data-dt-sort="name"]').click(); await wait(0);
        log("own edited", label());
        await open(); $("[data-dt-view-save]").click(); await wait(0);
        log("saved changes", label(), S().views[0].sort);
        // Renamed.
        $('[data-dt-view-rename="view-1"]').click(); await wait(0);
        await type("Paid, Z to A");
        log("renamed", label(), S().views[0].name);
        // The starting table, then back to the view.
        $('[data-dt-view-pick=""]').click(); await wait(0);
        log("default", label(), rows(), head(), S().cur, $("[data-dt-views]").matches(":popover-open"));
        await open(); $('[data-dt-view-pick="view-1"]').click(); await wait(0);
        log("back", label(), rows(), head());
        // Deleted: the view goes, and the table is no view's.
        await open(); $('[data-dt-view-delete="view-1"]').click(); await wait(0);
        log("deleted", JSON.stringify(S().views), S().cur, label(), menu());
        // From state: picking a view by id.
        root.__irender.set("cur", "paid"); await wait(0);
        log("state", label(), rows());
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("saved-views", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            // `defaultView`: the paid rows, most first.
            "opens with Paid Ada,Cy paid",
            "menu Default view | Paid* :: Save as new view…",
            // A change marks the view; a ready-made one can't be saved over.
            "sorted Paid (edited) Ada,Cy",
            "edited menu Default view | Paid* :: Undo changes | Save as new view…",
            "undone Paid Ada,Cy",
            "name box true New view name",
            // The new view keeps the filter, the sort and the hidden column.
            "saved Paid by name view-1 [{\"id\":\"view-1\",\"name\":\"Paid by name\",\"sort\":\"name\",\"filters\":{\"status\":[\"Paid\"]},\"search\":\"\",\"groupBy\":null,\"pageSize\":0,\"columns\":{\"order\":[\"name\",\"status\",\"net\"],\"widths\":{},\"hidden\":[\"net\"]}}]",
            "own menu Default view | Paid | Paid by name*(own) :: Save as new view…",
            "own edited Paid by name (edited)",
            "saved changes Paid by name -name",
            "renamed Paid, Z to A Paid, Z to A",
            // The starting table: every row and column, and no view.
            "default Views Ada,Bo,Cy,Di Name,Status,Net  false",
            "back Paid, Z to A Cy,Ada Name,Status",
            "deleted []  Views Default view* | Paid :: Save as new view…",
            // An id written to `bindCurrentView` shows that view.
            "state Paid Ada,Cy",
        ],
        "\n{result}"
    );
}

/// Rows from an API, grouped: a row per group from the API with its count and
/// subtotal, a group's rows loaded when it's opened, open groups loading again when the
/// search changes, and Group by None going back to plain rows.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn data_table_remote_groups_in_chrome() {
    use std::io::{Read, Write};
    use std::sync::{Arc, Mutex};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let log = Arc::new(Mutex::new(Vec::<String>::new()));
    let seen = log.clone();
    std::thread::spawn(move || {
        let orders = [("Ada", "North", 100), ("Bo", "South", 50), ("Cy", "North", 25), ("Di", "East", 10), ("Ed", "South", 5)];
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut buf = [0u8; 4096];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]).to_string();
            let target = request.split_whitespace().nth(1).unwrap_or("").to_string();
            let query: std::collections::HashMap<String, String> = target
                .split_once('?')
                .map(|(_, q)| q.split('&').filter_map(|kv| kv.split_once('=')).map(|(k, v)| (k.to_string(), v.replace('+', " "))).collect())
                .unwrap_or_default();
            seen.lock().unwrap().push(target.split_once('?').map_or(String::new(), |(_, q)| q.to_string()));
            let q = query.get("q").map(|s| s.to_lowercase()).unwrap_or_default();
            let rows: Vec<_> = orders.iter().filter(|o| o.0.to_lowercase().contains(&q)).collect();
            let body = if let Some(g) = query.get("group") {
                serde_json::json!({ "rows": rows.iter().filter(|o| o.1 == g).map(|o| serde_json::json!({ "id": o.0, "name": o.0, "region": o.1, "net": o.2 })).collect::<Vec<_>>() })
            } else if query.get("groupBy").is_some_and(|g| g == "region") {
                let mut regions: Vec<&str> = rows.iter().map(|o| o.1).collect();
                regions.sort();
                regions.dedup();
                let groups: Vec<_> = regions.iter().map(|r| {
                    let mine: Vec<_> = rows.iter().filter(|o| o.1 == *r).collect();
                    serde_json::json!({ "region": r, "count": mine.len(), "net": mine.iter().map(|o| o.2).sum::<i32>() })
                }).collect();
                serde_json::json!({ "rows": groups, "total": groups.len() })
            } else {
                serde_json::json!({ "rows": rows.iter().map(|o| serde_json::json!({ "id": o.0, "name": o.0, "region": o.1, "net": o.2 })).collect::<Vec<_>>(), "total": rows.len() })
            };
            let body = body.to_string();
            let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        }
    });
    let url = format!("http://127.0.0.1:{port}/orders");
    let doc = serde_json::json!({ "version": 1, "state": { "g": "region", "seen": null },
        "root": { "type": "ui-data-table", "props": { "url": url, "paging": "none", "bindGroupBy": "g", "selectable": true, "search": true, "bindView": "seen",
            "columns": [ { "field": "name", "label": "Name" }, { "field": "region", "label": "Region", "groupable": true }, { "field": "net", "label": "Net", "summary": "sum" } ] } } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const until = async (f) => { for (let i = 0; i < 100 && !f(); i++) await wait(20); };
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const T = document.querySelector("[data-i-x=ui-data-table]");
      const lines = () => [...T.querySelectorAll("[data-dt-body] > [data-dt-row]:not([hidden])")].map((tr) =>
        (tr.hasAttribute("data-dt-group-row") ? "[" + [...tr.querySelectorAll("[data-dt-group-label], [data-dt-group-count]")].map((x) => x.textContent).join(" ") + " " + tr.cells[tr.cells.length - 1].textContent.trim() + (tr.getAttribute("aria-expanded") === "true" ? " open" : "") + "]"
          : tr.querySelector("[data-dt-tree]").textContent.trim() + ":" + tr.cells[tr.cells.length - 1].textContent.trim())).join(" ");
      const row = (text) => [...T.querySelectorAll("[data-dt-body] > [data-dt-row]")].find((tr) => tr.querySelector("[data-dt-group-label]")?.textContent === text);
      try {
        await until(() => T.querySelectorAll("[data-dt-group-row]").length === 3);
        log("groups", lines(), "| sum", T.querySelector("[data-dt-sum]").textContent);
        log("count label", T.querySelector("[data-dt-group-count]").getAttribute("aria-label"), getComputedStyle(row("North").querySelector("[data-dt-select]")).visibility);
        row("North").click();
        await until(() => lines().includes("Ada"));
        log("opened", lines());
        log("view", JSON.stringify(S().seen.map((r) => r.name ?? "group")));
        // Select all leaves the group rows out.
        T.querySelector("[data-dt-select-page]").click(); await wait(0);
        log("selection", T.querySelectorAll("[data-dt-body] [aria-selected=true]").length);
        T.querySelector("[data-dt-select-page]").click();
        // A new search: the groups come again, and the open one loads its rows again.
        const box = T.querySelector("[data-dt-search]");
        box.value = "d"; box.dispatchEvent(new Event("input", { bubbles: true }));
        await until(() => lines().includes("Ada") && !lines().includes("Cy"));
        log("searched", lines());
        // Enter on a group row closes it.
        row("North").dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true })); await wait(0);
        log("closed", lines());
        // Group by None: plain rows.
        root.__irender.set("g", null);
        await until(() => !T.querySelector("[data-dt-group-row]:not([hidden])") && lines().includes("Di"));
        log("plain", lines());
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("remote-groups", &html) else { return };
    let asked = log.lock().unwrap().join("\n");
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            // A row per group, from the API; the footer adds up the groups' subtotals.
            "groups [East 1 10] [North 2 125] [South 2 55] | sum 190",
            "count label 1 row hidden",
            "opened [East 1 10] [North 2 125 open] Ada:100 Cy:25 [South 2 55]",
            // Charts and select-all get the rows, not the groups.
            r#"view ["Ada","Cy"]"#,
            "selection 2",
            "searched [East 1 10] [North 1 100 open] Ada:100 [South 1 5]",
            "closed [East 1 10] [North 1 100] [South 1 5]",
            "plain Ada:100 Di:10 Ed:5",
        ],
        "\n{result}\n---\n{asked}"
    );
    assert_eq!(asked, "groupBy=region&summary=net%3Asum\ngroupBy=region&group=North\nq=d&groupBy=region&summary=net%3Asum\nq=d&groupBy=region&group=North\nq=d&summary=net%3Asum");
}

/// Quarter presets on a financial year starting in April: the filters panel's (named by
/// the script, ranges from `today`) and a data table date filter's calendar.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn quarter_presets_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "f": {} },
        "root": { "type": "stack", "children": [
            { "type": "ui-filters", "props": { "bind": "f", "today": "2026-11-18", "fiscalStart": 4, "live": true }, "children": [
                { "type": "ui-filter", "props": { "field": "due", "type": "date", "presets": ["thisQuarter", "lastQuarter", "thisYear", "lastYear", "yearToDate", "quarters"] } } ] },
            { "type": "ui-data-table", "props": { "rows": [ { "id": 1, "d": "2026-02-10" }, { "id": 2, "d": "2026-05-01" }, { "id": 3, "d": "2026-10-02" } ], "pageSize": 0 },
              "children": [ { "type": "ui-data-table-column", "props": { "field": "d", "label": "Day", "filter": "date", "fiscalStart": 4, "presets": ["quarters", "lastYear"] } } ] } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const F = document.querySelector("[data-i-x=ui-filters]"), T = document.querySelector("[data-i-x=ui-data-table]");
      try {
        const buttons = [...F.querySelectorAll("[data-fl-preset]")];
        log("panel", buttons.map((b) => b.textContent).join(","));
        const ranges = [];
        for (const b of buttons) {
          b.click(); await wait(0);
          log(b.textContent, S().f.due.from + ".." + S().f.due.to, b.getAttribute("aria-pressed"));
        }
        // The table's calendar: its presets, then a quarter as the filter.
        const cal = T.querySelector("[data-dt-calendar]");
        log("table", [...cal.querySelectorAll("[data-preset]")].map((b) => b.textContent).join(","));
        cal.querySelector("[data-preset='3']").click(); await wait(10);
        log("filtered", [...T.querySelectorAll("[data-dt-body] > tr:not([hidden])")].map((tr) => tr.cells[0].textContent.trim()).join(","));
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("quarters", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            // 18 Nov 2026 is in the third quarter of the year that started on 1 April 2026.
            "panel This quarter,Last quarter,This year,Last year,Year to date,Q3 FY2026–27,Q2 FY2026–27,Q1 FY2026–27,Q4 FY2025–26",
            "This quarter 2026-10-01..2026-12-31 true",
            "Last quarter 2026-07-01..2026-09-30 true",
            "This year 2026-04-01..2027-03-31 true",
            "Last year 2025-04-01..2026-03-31 true",
            "Year to date 2026-04-01..2026-11-18 true",
            "Q3 FY2026–27 2026-10-01..2026-12-31 true",
            "Q2 FY2026–27 2026-07-01..2026-09-30 true",
            "Q1 FY2026–27 2026-04-01..2026-06-30 true",
            "Q4 FY2025–26 2026-01-01..2026-03-31 true",
            // The table's Q4 FY2025–26 is January to March 2026.
            "table Q3 FY2026–27,Q2 FY2026–27,Q1 FY2026–27,Q4 FY2025–26,Last year",
            "filtered 2026-02-10",
        ],
        "\n{result}"
    );
}

/// A layout per width: the grid switches as its width crosses a breakpoint, a move
/// changes only the size it's made in, `bind` keeps each, and null puts them back.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn dashboard_grid_breakpoints_in_chrome() {
    let tile = |id: &str, p: serde_json::Value| {
        let mut props = serde_json::json!({ "id": id, "title": id.to_uppercase() });
        props.as_object_mut().unwrap().extend(p.as_object().unwrap().clone());
        serde_json::json!({ "type": "ui-tile", "props": props, "children": [ { "type": "text", "props": { "value": id } } ] })
    };
    let doc = serde_json::json!({ "version": 1, "state": { "layout": null },
        "root": { "type": "element", "attrs": { "id": "box" }, "style": { "width": "960px" }, "children": [
            { "type": "ui-dashboard-grid", "attrs": { "id": "g" }, "props": { "bind": "layout", "gap": 0, "rowHeight": 50, "stackBelow": 200,
                "breakpoints": { "lg": { "width": 700, "columns": 12 }, "md": { "width": 300, "columns": 6 } } },
              "children": [
                tile("a", serde_json::json!({ "x": 0, "y": 0, "w": 6, "h": 2 })),
                tile("b", serde_json::json!({ "x": 6, "y": 0, "w": 6, "h": 2 })),
                tile("c", serde_json::json!({ "x": 0, "y": 2, "w": 12, "h": 1, "layouts": { "md": { "x": 0, "y": 0, "w": 6, "h": 1 } } })) ] } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const G = document.getElementById("g"), box = document.getElementById("box");
      const cell = () => G.querySelector("[data-dg-grid]").clientWidth / (G.__x.size === "md" ? 6 : 12);
      const seen = () => [...G.querySelectorAll("[data-dg-tile]")].map((n) =>
        `${n.dataset.id}:${Math.round(n.offsetLeft / cell())},${Math.round(n.offsetTop / 50)},${Math.round(n.offsetWidth / cell())}x${Math.round(n.offsetHeight / 50)}`).join(" ");
      const at = () => G.__x.size + " " + G.__x.layout.map((t) => `${t.id}:${t.x},${t.y},${t.w}x${t.h}`).join(" ");
      const key = (id, k) => G.querySelector(`[data-id=${id}] [data-dg-handle]`).dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true }));
      try {
        await wait(50);
        log("wide", at(), "|", seen());
        box.style.width = "480px"; await wait(100);
        log("narrow", at(), "|", seen());
        // A move at this size: only its layout changes.
        key("a", "ArrowRight"); await wait(250);
        log("moved", at(), "|", JSON.stringify(S().layout));
        box.style.width = "960px"; await wait(100);
        log("wide again", at(), "|", seen());
        box.style.width = "480px"; await wait(100);
        log("narrow again", at());
        root.__irender.set("layout", null); await wait(100);
        log("reset", at());
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("dashboard-breakpoints", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "wide lg a:0,0,6x2 b:6,0,6x2 c:0,2,12x1 | a:0,0,6x2 b:6,0,6x2 c:0,2,12x1",
            // 480px: md's six columns, c first (its `layouts`), a and b scaled and packed.
            "narrow md a:0,1,3x2 b:3,1,3x2 c:0,0,6x1 | a:0,1,3x2 b:3,1,3x2 c:0,0,6x1",
            // Moved at md: lg's layout in `bind` is untouched.
            "moved md a:1,1,3x2 b:3,3,3x2 c:0,0,6x1 | {\"lg\":[{\"id\":\"a\",\"x\":0,\"y\":0,\"w\":6,\"h\":2},{\"id\":\"b\",\"x\":6,\"y\":0,\"w\":6,\"h\":2},{\"id\":\"c\",\"x\":0,\"y\":2,\"w\":12,\"h\":1}],\"md\":[{\"id\":\"a\",\"x\":1,\"y\":1,\"w\":3,\"h\":2},{\"id\":\"b\",\"x\":3,\"y\":3,\"w\":3,\"h\":2},{\"id\":\"c\",\"x\":0,\"y\":0,\"w\":6,\"h\":1}]}",
            "wide again lg a:0,0,6x2 b:6,0,6x2 c:0,2,12x1 | a:0,0,6x2 b:6,0,6x2 c:0,2,12x1",
            "narrow again md a:1,1,3x2 b:3,3,3x2 c:0,0,6x1",
            // Null: every size back to the tiles' own places.
            "reset md a:0,1,3x2 b:3,1,3x2 c:0,0,6x1",
        ],
        "\n{result}"
    );
}

/// Anchor positioning without the browser's: with native anchors turned off, the
/// fallback places each popover beside its trigger, on its side and alignment, flips it
/// when it doesn't fit, mirrors it right to left, and follows the page as it scrolls.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn anchor_fallback_in_chrome() {
    let item = |l: &str| serde_json::json!({ "type": "ui-menu-item", "props": { "label": l } });
    let doc = serde_json::json!({ "version": 1, "state": {},
        "root": { "type": "element", "style": { "padding": "40px 200px", "height": "2000px" }, "children": [
            { "type": "ui-dropdown-menu", "props": { "id": "menu", "trigger": "Menu", "align": "start" }, "children": [item("One"), item("Two")] },
            { "type": "ui-popover", "props": { "id": "right", "trigger": "Right", "side": "right" }, "children": [ { "type": "text", "props": { "value": "beside" } } ] },
            { "type": "element", "attrs": { "dir": "rtl" }, "children": [
                { "type": "ui-dropdown-menu", "props": { "id": "rtl", "trigger": "RTL", "align": "start" }, "children": [item("One")] } ] },
            { "type": "ui-data-table", "props": { "rows": [ { "id": 1, "s": "A" }, { "id": 2, "s": "B" } ], "views": [ { "id": "v", "name": "V" } ] },
              "children": [ { "type": "ui-data-table-column", "props": { "field": "s", "filter": "select" } } ] },
            { "type": "ui-date-picker", "props": { "id": "day" } },
            { "type": "ui-combobox", "props": { "id": "fruit", "options": ["Apple", "Banana"], "placeholder": "Fruit" } },
            { "type": "element", "style": { "position": "absolute", "top": "740px", "left": "200px" }, "children": [
                { "type": "ui-popover", "props": { "id": "low", "trigger": "Low" }, "children": [ { "type": "element", "style": { "height": "120px" }, "children": [ { "type": "text", "props": { "value": "tall" } } ] } ] } ] } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const frames = () => wait(80);
      const pop = (id) => [document.querySelector(`[popovertarget="${id}"]:not([popovertargetaction])`), document.getElementById(id)];
      // The gap to the trigger on the popover's side, and how it lines up on the other axis.
      const where = (id) => {
        const [t, p] = pop(id), a = t.getBoundingClientRect(), r = p.getBoundingClientRect();
        const mid = (lo, hi) => (lo + hi) / 2, centred = (x, y) => (Math.abs(x - y) <= 1 ? "centred" : `off ${Math.round(x - y)}`);
        return {
          menu: `under ${Math.round(r.top - a.bottom)}, left edges ${Math.round(r.left - a.left)}`,
          right: `after ${Math.round(r.left - a.right)}, ${centred(mid(r.top, r.bottom), mid(a.top, a.bottom))}`,
          rtl: `under ${Math.round(r.top - a.bottom)}, right edges ${Math.round(a.right - r.right)}`,
          low: `above ${Math.round(a.top - r.bottom)}, ${centred(mid(r.left, r.right), mid(a.left, a.right))}`,
        }[id] + "," ;
      };
      try {
        log("native off", !CSS.supports("position-area", "bottom") || getComputedStyle(document.querySelector("[popover]")).positionArea === "none", !!window.__irenderAnchors);
        for (const id of ["menu", "right", "rtl", "low"]) {
          const [t, p] = pop(id);
          t.click(); await frames(); await wait(20);
          log(where(id), p.hasAttribute("data-anchored"));
          p.hidePopover();
        }
        // The rest of the kit's popovers: each beside the button that opened it.
        for (const t of document.querySelectorAll("[data-i-x=ui-data-table] [popovertarget], [data-i-x=ui-date-picker] [popovertarget]")) {
          const p = document.getElementById(t.getAttribute("popovertarget"));
          t.click(); await frames();
          const a = t.getBoundingClientRect(), r = p.getBoundingClientRect();
          log(p.getAttribute("data-dt-filter") ?? (p.hasAttribute("data-dt-views") ? "views" : p.hasAttribute("data-dt-columns") ? "columns" : "picker"), Math.round(r.top - a.bottom) >= 0 && Math.round(r.top - a.bottom) <= 8, Math.abs(Math.round(r.left - a.left)) <= 1 || Math.abs(Math.round(r.right - a.right)) <= 1);
          p.hidePopover();
        }
        const box = document.querySelector("[data-i-x=ui-combobox] input");
        box.focus(); box.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true })); await frames();
        const list = document.querySelector("[data-i-x=ui-combobox] [popover]");
        const ra = box.closest("[data-i-x=ui-combobox]").querySelector("[data-cb-control], [data-cb-anchor], input").getBoundingClientRect(), rl = list.getBoundingClientRect();
        log("combobox", list.matches(":popover-open"), Math.round(rl.top - ra.bottom) >= 0 && Math.round(rl.top - ra.bottom) <= 12);
        list.hidePopover?.();
        // Scrolled: the open menu moves with its trigger.
        const [t, p] = pop("menu");
        t.click(); await frames();
        window.scrollTo(0, 30); await frames(); await wait(20);
        log("scrolled", Math.round(p.getBoundingClientRect().top - t.getBoundingClientRect().bottom));
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    // The browser's own anchor positioning off, and the fallback asked for.
    let off = "<style data-anchor-off>* { position-area: none !important; position-anchor: none !important; position-try-fallbacks: none !important; }</style>";
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test")
        .replacen("<html", "<html data-anchor-fallback", 1)
        .replace("</head>", &format!("{off}</head>"))
        .replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("anchor-fallback", &html) else { return };
    // The same places as the browser's own anchor positioning gives (checked against it).
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "native off true true",
            // 6px under the trigger, from its left edge.
            "under 6, left edges 0, true",
            // 6px after it, centred on it.
            "after 6, centred, true",
            // Right to left: from its right edge.
            "under 6, right edges 0, true",
            // No room under it: flipped above.
            "above 6, centred, true",
            // The data table's Views menu, its filter and View menu, the date picker and
            // the combobox's list: under their buttons, lined up with an edge.
            "views true true",
            "s true true",
            "columns true true",
            "picker true true",
            "combobox true true",
            "scrolled 6",
        ],
        "\n{result}"
    );
}

/// A group from an API loads its rows a batch at a time: the first when it's opened,
/// then the next from the "load more" row after them (a click or Enter), until all have.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn data_table_remote_group_batches_in_chrome() {
    use std::io::{Read, Write};
    use std::sync::{Arc, Mutex};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let log = Arc::new(Mutex::new(Vec::<String>::new()));
    let seen = log.clone();
    std::thread::spawn(move || {
        let rows = [("A", "X"), ("B", "X"), ("C", "X"), ("D", "X"), ("E", "X"), ("F", "Y")];
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut buf = [0u8; 4096];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]).to_string();
            let target = request.split_whitespace().nth(1).unwrap_or("").to_string();
            let query: std::collections::HashMap<String, String> = target
                .split_once('?')
                .map(|(_, q)| q.split('&').filter_map(|kv| kv.split_once('=')).map(|(k, v)| (k.to_string(), v.to_string())).collect())
                .unwrap_or_default();
            seen.lock().unwrap().push(target.split_once('?').map_or(String::new(), |(_, q)| q.to_string()));
            let num = |k: &str, d: usize| query.get(k).and_then(|v| v.parse().ok()).unwrap_or(d);
            let (offset, limit) = (num("offset", 0), num("limit", 100));
            let q = query.get("q").cloned().unwrap_or_default().to_lowercase();
            let mine: Vec<_> = rows.iter().filter(|r| r.0.to_lowercase() != q).collect();
            let body = if let Some(g) = query.get("group") {
                let list: Vec<_> = mine.iter().filter(|r| r.1 == g).collect();
                serde_json::json!({ "rows": list.iter().skip(offset).take(limit).map(|r| serde_json::json!({ "id": r.0, "name": r.0, "region": r.1 })).collect::<Vec<_>>(), "total": list.len() })
            } else {
                let x = mine.iter().filter(|r| r.1 == "X").count();
                serde_json::json!({ "rows": [{ "region": "X", "count": x }, { "region": "Y", "count": 1 }], "total": 2 })
            };
            let body = body.to_string();
            let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        }
    });
    let url = format!("http://127.0.0.1:{port}/rows");
    let doc = serde_json::json!({ "version": 1, "state": { "g": "region" },
        "root": { "type": "ui-data-table", "props": { "url": url, "paging": "pages", "pageSize": 2, "pageSizes": [], "bindGroupBy": "g", "search": true,
            "columns": [ { "field": "name", "label": "Name" }, { "field": "region", "label": "Region", "groupable": true } ] } } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const until = async (f) => { for (let i = 0; i < 100 && !f(); i++) await wait(20); };
      const T = document.querySelector("[data-i-x=ui-data-table]");
      const lines = () => [...T.querySelectorAll("[data-dt-body] > [data-dt-row]:not([hidden])")].map((tr) =>
        tr.hasAttribute("data-dt-group-row") ? "[" + tr.querySelector("[data-dt-group-label]").textContent + "]"
        : tr.hasAttribute("data-dt-more-row") ? "(" + tr.querySelector("[data-dt-load-group]").textContent + ")"
        : tr.querySelector("[data-dt-tree]").textContent.trim()).join(" ");
      const group = (v) => [...T.querySelectorAll("[data-dt-group-row]")].find((tr) => tr.querySelector("[data-dt-group-label]").textContent === v);
      const more = () => T.querySelector("[data-dt-more-row]");
      try {
        await until(() => group("X"));
        group("X").click();
        await until(() => lines().includes("B"));
        log("first", lines());
        more().querySelector("[data-dt-load-group]").click();
        await until(() => lines().includes("D") && !more()?.querySelector("button").disabled);
        log("second", lines());
        // Enter on the "load more" row loads the last batch; then it goes.
        more().focus();
        more().dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
        await until(() => lines().includes("E") && !more()); await wait(60);
        log("all", lines(), T.querySelector("[data-dt-footer] [data-dt-live]").textContent);
        // A new search: the group loads its first batch again.
        const box = T.querySelector("[data-dt-search]");
        box.value = "a"; box.dispatchEvent(new Event("input", { bubbles: true }));
        await until(() => lines().includes("C") && !lines().includes("A") && !lines().includes("D"));
        log("searched", lines());
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("remote-group-batches", &html) else { return };
    let asked = log.lock().unwrap().iter().filter(|q| q.contains("group=")).cloned().collect::<Vec<_>>().join("\n");
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            // Two at a time (`pageSize`), out of the 5 the API says the group has.
            "first [X] A B (Load 2 more of 5) [Y]",
            "second [X] A B C D (Load 1 more of 5) [Y]",
            "all [X] A B C D E [Y] 1 more row loaded",
            // A new search starts the group over.
            "searched [X] B C (Load 2 more of 4) [Y]",
        ],
        "\n{result}\n---\n{asked}"
    );
    assert_eq!(asked, "groupBy=region&group=X&offset=0&limit=2\ngroupBy=region&group=X&offset=2&limit=2\ngroupBy=region&group=X&offset=4&limit=2\nq=a&groupBy=region&group=X&offset=0&limit=2");
}

/// Nested rows: picking a row picks the rows under it, a row is picked when all under it
/// are and shows part-picked when some are, the footer counts every row, Expand all and
/// Collapse all in View, and `*` opens a row's siblings.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn data_table_nested_selection_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "picked": [], "rows": [
            { "id": "a", "name": "A", "children": [ { "id": "a1", "name": "A1" }, { "id": "a2", "name": "A2", "children": [ { "id": "a2a", "name": "A2a" } ] } ] },
            { "id": "b", "name": "B", "children": [ { "id": "b1", "name": "B1" } ] } ] },
        "root": { "type": "ui-data-table", "props": { "rows": "{{ rows }}", "selectable": true, "bindSelection": "picked", "pageSize": 0,
            "columns": [ { "field": "name", "label": "Name" } ] } } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const T = document.querySelector("[data-i-x=ui-data-table]");
      const rows = () => [...T.querySelectorAll("[data-dt-body] > [data-dt-row]:not([hidden])")].map((tr) => {
        const b = tr.querySelector("[data-dt-select]");
        return tr.querySelector("[data-dt-tree]").textContent.trim() + (b.indeterminate ? "~" : b.checked ? "*" : "");
      }).join(" ");
      const row = (name) => [...T.querySelectorAll("[data-dt-body] > [data-dt-row]")].find((tr) => tr.querySelector("[data-dt-tree]").textContent.trim() === name);
      const tick = async (name) => { const b = row(name).querySelector("[data-dt-select]"); b.checked = !b.checked; b.dispatchEvent(new Event("change", { bubbles: true })); await wait(30); };
      const picked = () => [...S().picked].sort().join(",");
      const footer = () => T.querySelector("[data-dt-status]").textContent;
      try {
        log("start", rows(), "|", footer());
        await tick("A");
        log("ticked A:", rows(), "|", picked(), "|", footer());
        T.querySelector('[data-dt-open-all="true"]').click(); await wait(0);
        log("expanded", rows(), "|", footer());
        // One row under A2 unpicked: A2 and A aren't, A shows part-picked.
        await tick("A2a");
        log("A2a off", rows(), "|", picked());
        await tick("A2a");
        log("A2a on", rows(), "|", picked());
        // B1 alone: B (its only row) is picked too.
        await tick("B1");
        log("B1", rows(), "|", picked());
        T.querySelector('[data-dt-open-all="false"]').click(); await wait(0);
        log("collapsed", rows());
        // `*` on A opens A and B.
        row("A").focus();
        row("A").dispatchEvent(new KeyboardEvent("keydown", { key: "*", bubbles: true, cancelable: true })); await wait(0);
        log("star", rows());
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("nested-selection", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            // Six rows, open or not.
            "start A B | 0 of 6 row(s) selected.",
            // A brings everything under it.
            "ticked A: A* B | a,a1,a2,a2a | 4 of 6 row(s) selected.",
            "expanded A* A1* A2* A2a* B B1 | 4 of 6 row(s) selected.",
            // A2a off: A2 (all its rows) and A (not all) go; A shows part-picked (~).
            "A2a off A~ A1* A2 A2a B B1 | a1",
            "A2a on A* A1* A2* A2a* B B1 | a,a1,a2,a2a",
            "B1 A* A1* A2* A2a* B* B1* | a,a1,a2,a2a,b,b1",
            "collapsed A* B*",
            // `*` opens A and B, the rows beside it; A2 stays as it was.
            "star A* A1* A2* B* B1*",
        ],
        "\n{result}"
    );
}

/// The command palette ranks what matches: a label that starts with what's typed, then
/// one with a word that does, then one with it inside, then one with its letters in
/// order; groups by their best item; keywords count less; the matched letters marked.
/// `rank: false` keeps the items' order.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn command_palette_ranking_in_chrome() {
    let item = |label: &str, keywords: Option<&str>| match keywords {
        Some(k) => serde_json::json!({ "type": "ui-command-item", "props": { "label": label, "keywords": k } }),
        None => serde_json::json!({ "type": "ui-command-item", "props": { "label": label } }),
    };
    let items = || vec![
        serde_json::json!({ "type": "ui-command-group", "props": { "heading": "Account" }, "children": [
            item("Reset password", None), item("System status", None), item("Set up billing", None), item("Settings", Some("preferences config")) ] }),
        serde_json::json!({ "type": "ui-command-group", "props": { "heading": "Help" }, "children": [ item("Keyboard shortcuts", None), item("Status page", None) ] }),
    ];
    let doc = serde_json::json!({ "version": 1, "root": { "type": "stack", "children": [
        { "type": "ui-command", "props": { "id": "ranked" }, "children": items() },
        { "type": "ui-command", "props": { "id": "plain", "rank": false }, "children": items() } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      // Items as they appear, top to bottom, the active one marked *.
      const view = (p) => [...p.querySelectorAll("[role=option]")].filter((i) => i.getClientRects().length)
        .sort((a, b) => a.getBoundingClientRect().top - b.getBoundingClientRect().top)
        .map((i) => i.querySelector("[data-ui-label]").textContent + (i.getAttribute("aria-selected") === "true" ? "*" : "")).join(", ");
      const type = async (p, text) => { const i = p.querySelector("input"); i.value = text; i.dispatchEvent(new Event("input", { bubbles: true })); await wait(0); };
      const R = document.getElementById("ranked"), P = document.getElementById("plain");
      try {
        for (const q of ["set", "stg", "config", "status", "sta"]) {
          await type(R, q);
          log(q, "→", view(R));
        }
        await type(R, "set");
        const h = CSS.highlights?.get("ui-command-match");
        log("marked", h ? [...h].map((r) => r.startContainer.textContent.slice(r.startOffset, r.endOffset)).join("") : "no highlight API");
        // Down moves in the order shown.
        R.querySelector("input").dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true, cancelable: true })); await wait(0);
        log("down", view(R));
        await type(R, "");
        log("cleared", view(R));
        await type(P, "set");
        log("rank false", view(P));
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("command-ranking", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            // Starts with it, shorter first; then a word inside; then the letters in order.
            "set → Settings*, Set up billing, Reset password, System status",
            "stg → Settings*, Set up billing, Status page",
            // Its keywords.
            "config → Settings*",
            // Help's best item beats Account's, so Help comes first.
            "status → Status page*, System status",
            "sta → Status page*, System status, Reset password",
            "marked setSetSetSet",
            "down Settings, Set up billing*, Reset password, System status",
            "cleared Reset password*, System status, Set up billing, Settings, Keyboard shortcuts, Status page",
            "rank false Reset password*, System status, Set up billing, Settings",
        ],
        "\n{result}"
    );
}

/// A submenu's grace triangle: heading from its item towards it across another item
/// keeps it open; moving away, or lingering past the moment, lets the other item act.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn submenu_grace_triangle_in_chrome() {
    let item = |l: &str| serde_json::json!({ "type": "ui-menu-item", "props": { "label": l } });
    let doc = serde_json::json!({ "version": 1, "root": { "type": "element", "style": { "padding": "40px" }, "children": [
        { "type": "ui-dropdown-menu", "props": { "id": "m", "trigger": "Actions", "align": "start" }, "children": [
            { "type": "ui-menu-sub", "props": { "label": "Share" }, "children": [ item("Email"), item("Link"), item("Embed") ] },
            item("Rename"), item("Archive"), item("Delete") ] } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const M = document.getElementById("m");
      const item = (l) => [...M.querySelectorAll("[role=menuitem]")].find((i) => i.textContent.trim() === l);
      const sub = () => document.getElementById(item("Share").getAttribute("aria-controls"));
      const open = () => sub().matches(":popover-open");
      const mouse = (type, t, x, y, related) => t.dispatchEvent(new MouseEvent(type, { bubbles: true, clientX: x, clientY: y, relatedTarget: related ?? null }));
      // From Share's right half down onto Rename, `dx` across and `dy` down.
      const leave = (dx, dy) => {
        const s = item("Share").getBoundingClientRect(), r = item("Rename").getBoundingClientRect();
        const x0 = s.right - 10, y0 = s.bottom - 2, x = x0 + dx, y = r.top + dy;
        mouse("mouseout", item("Share"), x0, y0, item("Rename"));
        mouse("mouseover", item("Rename"), x, y, item("Share"));
        return [x, y];
      };
      const reopen = async () => { mouse("mouseover", item("Share"), item("Share").getBoundingClientRect().left + 20, item("Share").getBoundingClientRect().top + 8); await wait(10); };
      try {
        document.querySelector('[popovertarget="m"]').click(); await wait(20);
        await reopen();
        log("hovered Share", open(), sub().getBoundingClientRect().left >= item("Share").getBoundingClientRect().right - 8);
        // Heading right towards it, across Rename: it stays open.
        leave(6, 3); await wait(50);
        log("towards it", open());
        // Into the submenu: still open, and Rename never acts.
        mouse("mouseover", sub().querySelector("[role=menuitem]"), sub().getBoundingClientRect().left + 10, sub().getBoundingClientRect().top + 10); await wait(450);
        log("reached it", open());
        // Straight down (or left), away from it: Rename acts at once and it closes.
        await reopen();
        const [x, y] = leave(-60, 3);
        log("away", open());
        await reopen();
        // Lingering on Rename: after the moment, it closes.
        leave(6, 3); await wait(50);
        log("lingering", open());
        await wait(450);
        log("after", open());
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("submenu-grace", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "hovered Share true true",
            // Crossing Rename on the way to the submenu: it stays open.
            "towards it true",
            "reached it true",
            // Away from it: Rename closes it at once.
            "away false",
            // Stopping on Rename: open for the moment, then closed.
            "lingering true",
            "after false",
        ],
        "\n{result}"
    );
}

/// Kanban swimlanes, card fields and moving columns: a row of cells per assignee, cards
/// moved within and across lanes (writing the field) by keys and by dragging, focus going
/// on into the next lane, lanes folding, and columns moved by their grip.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn kanban_lanes_fields_and_columns_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": {
            "cols": [ { "id": "todo", "title": "To do" }, { "id": "doing", "title": "Doing" }, { "id": "done", "title": "Done" } ],
            "tasks": [
                { "id": "a", "column": "todo", "title": "Alpha", "assignee": "Asha", "points": 3, "progress": 20, "risk": "High" },
                { "id": "b", "column": "todo", "title": "Bravo", "assignee": "Asha" },
                { "id": "c", "column": "doing", "title": "Charlie", "assignee": "Ravi" },
                { "id": "d", "column": "done", "title": "Delta" } ] },
        "root": { "type": "ui-kanban", "props": { "columns": "{{ cols }}", "cards": "{{ tasks }}", "bindCards": "tasks", "bindColumns": "cols",
            "swimlanes": "assignee", "reorderable": true, "collapsible": true,
            "fields": [ { "field": "points", "label": "Points" }, { "field": "progress", "label": "Done", "cell": "progress" }, { "field": "risk", "cell": "badge", "tones": { "High": "danger" } } ] } } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const K = document.querySelector("[data-i-x=ui-kanban]"), $ = (s) => K.querySelector(s), $$ = (s) => [...K.querySelectorAll(s)];
      const card = (id) => $(`[data-kb-card="${id}"]`);
      // Each lane: its name and count, then each cell's cards.
      const board = () => $$("[data-kb-lane-row]").map((r) => r.querySelector("[data-kb-lane-fold]").textContent + "(" + r.querySelector("[data-kb-lane-head] [data-kb-count]").textContent + "):" +
        [...r.querySelectorAll("[data-kb-column]")].map((c) => [...c.querySelectorAll("[data-kb-card]")].map((x) => x.getAttribute("data-kb-card")).join("")).join("/")).join(" ");
      const heads = () => $$("[data-kb-colhead] h3").map((h) => h.textContent + " " + h.parentElement.querySelector("[data-kb-count]").textContent).join(", ");
      const key = (k, o = {}) => document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true, ...o }));
      const focused = () => document.activeElement.getAttribute("data-kb-card");
      const who = () => S().tasks.map((t) => t.id + ":" + (t.assignee ?? "-") + "@" + t.column).join(" ");
      try {
        log("lanes", board(), "|", heads());
        log("fields", [...card("a").querySelectorAll("[data-kb-field]")].map((f) => f.textContent).join(" · "), card("a").querySelector("[data-kb-badge]").getAttribute("data-tone"), card("a").querySelector("[data-kb-progress]").getAttribute("aria-valuenow"));
        // Down at the end of a cell: on into the next lane's cell in that column, if it has cards.
        card("b").focus(); key("ArrowDown"); log("down at the end", focused());
        // Alt+Down at the end of the cell: into the next lane, writing its assignee.
        key("ArrowDown", { altKey: true }); await wait(0);
        log("alt down", board(), "|", who());
        // Dragging Alpha into Doing in Ravi's lane.
        const dt = new DataTransfer(), cell = $('[data-kb-lane="Ravi"][data-kb-column="doing"] [data-kb-cards]'), r = cell.getBoundingClientRect();
        card("a").dispatchEvent(new DragEvent("dragstart", { bubbles: true, dataTransfer: dt }));
        cell.dispatchEvent(new DragEvent("dragover", { bubbles: true, cancelable: true, dataTransfer: dt, clientY: r.bottom - 2 }));
        cell.dispatchEvent(new DragEvent("drop", { bubbles: true, cancelable: true, dataTransfer: dt, clientY: r.bottom - 2 }));
        $("[data-kb-board]").dispatchEvent(new DragEvent("dragend", { bubbles: true, dataTransfer: dt })); await wait(0);
        log("dragged", board(), "|", who());
        // Folding a lane hides its cells.
        $('[data-kb-lane-fold="Ravi"]').click(); await wait(0);
        log("folded", $$('[data-kb-lane-row="Ravi"] [data-kb-column]').length, $('[data-kb-lane-fold="Ravi"]').getAttribute("aria-expanded"));
        $('[data-kb-lane-fold="Ravi"]').click(); await wait(0);
        // Columns: Right on To do's grip moves it after Doing.
        $('[data-kb-grip="todo"]').focus(); key("ArrowRight"); await wait(0);
        log("column right", S().cols.map((c) => c.id).join(","), heads(), document.activeElement.getAttribute("data-kb-grip"));
        // Dragging Done's grip onto the first header (its left half): Done goes first.
        const g = new DataTransfer(), first = $$("[data-kb-colhead]")[0], fr = first.getBoundingClientRect();
        $('[data-kb-grip="done"]').dispatchEvent(new DragEvent("dragstart", { bubbles: true, dataTransfer: g }));
        first.dispatchEvent(new DragEvent("dragover", { bubbles: true, cancelable: true, dataTransfer: g, clientX: fr.left + 5, clientY: fr.top + 5 }));
        first.dispatchEvent(new DragEvent("drop", { bubbles: true, cancelable: true, dataTransfer: g, clientX: fr.left + 5, clientY: fr.top + 5 }));
        $("[data-kb-board]").dispatchEvent(new DragEvent("dragend", { bubbles: true, dataTransfer: g })); await wait(0);
        log("column dragged", S().cols.map((c) => c.id).join(","));
        await wait(60);
        log("said", $("[data-kb-live]").textContent);
        // By touch: hold To do's grip, then drag it onto the first header's left half.
        const touch = (t, type, x, y) => t.dispatchEvent(new PointerEvent(type, { bubbles: true, cancelable: true, pointerId: 5, pointerType: "touch", isPrimary: true, clientX: x, clientY: y }));
        const tg = $('[data-kb-grip="todo"]'), gr = tg.getBoundingClientRect(), hr = $$("[data-kb-colhead]")[0].getBoundingClientRect();
        touch(tg, "pointerdown", gr.left + 4, gr.top + 4); await wait(350);
        log("held", $$("[data-touch-ghost]").length, $("[data-touch-dragging]")?.hasAttribute("data-kb-head"));
        touch(tg, "pointermove", hr.left + 8, hr.top + 8); await wait(0);
        log("over", $$("[data-kb-head][data-over-drop]").length);
        touch(tg, "pointerup", hr.left + 8, hr.top + 8); await wait(0);
        log("column touched", S().cols.map((c) => c.id).join(","), $$("[data-touch-ghost]").length);
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("kanban-lanes", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            // A row per assignee, cards without one last; each lane's cells To do / Doing / Done.
            "lanes Asha(2):ab// Ravi(1):/c/ No assignee(1)://d | To do 2, Doing 1, Done 1",
            "fields Points3 · Done20% · High danger 20",
            // Ravi's To do cell is empty, so Down stays on Bravo.
            "down at the end b",
            // Alt+Down at the end of the cell: into Ravi's lane, and Bravo's assignee is Ravi.
            "alt down Asha(1):a// Ravi(2):b/c/ No assignee(1)://d | a:Asha@todo c:Ravi@doing d:-@done b:Ravi@todo",
            // Dragged into Ravi's Doing; Asha's lane, now empty, goes (`lanes` would keep it).
            "dragged Ravi(3):b/ca/ No assignee(1)://d | c:Ravi@doing a:Ravi@doing d:-@done b:Ravi@todo",
            "folded 0 false",
            "column right doing,todo,done Doing 2, To do 1, Done 1 todo",
            "column dragged done,doing,todo",
            "said Done moved to position 1 of 3",
            // The header follows the finger; dropped on the first, To do goes first.
            "held 1 true",
            "over 1",
            "column touched todo,done,doing 0",
        ],
        "\n{result}"
    );
}

/// A board whose children are the cards' content: drawn on the server and again in the
/// browser with the card in scope, in each cell (and lane), still moved by Alt and the
/// arrows, named and focusable as the board's own, with a control inside left to itself.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn kanban_card_template_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "open": null, "likes": 0,
            "tasks": [
                { "id": "a", "column": "todo", "title": "Alpha", "points": 3, "who": "Asha" },
                { "id": "b", "column": "todo", "title": "Bravo", "points": 5, "who": "Ravi" },
                { "id": "c", "column": "done", "title": "Charlie", "points": 1, "who": "Asha" } ] },
        "root": { "type": "ui-kanban", "props": { "columns": [ { "id": "todo", "title": "To do" }, { "id": "done", "title": "Done" } ],
            "cards": "{{ tasks }}", "bindCards": "tasks", "bindCard": "open", "swimlanes": "who" },
            "children": [
                { "type": "element", "props": { "tag": "strong" }, "attrs": { "data-t": true }, "children": [ { "type": "text", "props": { "value": "{{ card.title }}" } } ] },
                { "type": "element", "props": { "tag": "span" }, "attrs": { "data-p": true }, "children": [ { "type": "text", "props": { "value": "{{ card.points + ' pts · #' + (index + 1) }}" } } ] },
                { "type": "ui-button", "props": { "label": "Like" }, "on": { "click": { "action": "set_state", "key": "likes", "value": "{{ likes + 1 }}" } } } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const K = document.querySelector("[data-i-x=ui-kanban]"), $ = (s) => K.querySelector(s), $$ = (s) => [...K.querySelectorAll(s)];
      // Each cell (lane / column): its cards' own content.
      const board = () => $$("[data-kb-board] [data-kb-column]").map((c) => c.getAttribute("data-kb-lane") + "/" + c.getAttribute("data-kb-column") + ":" +
        [...c.querySelectorAll("[data-kb-card]")].map((x) => x.querySelector("[data-t]").textContent + "(" + x.querySelector("[data-p]").textContent + ")").join(",")).join(" ");
      const card = (id) => $(`[data-kb-card="${id}"]`);
      try {
        log("drawn", board());
        log("own", card("a").getAttribute("aria-label"), card("a").getAttribute("aria-roledescription"), card("a").draggable, $$("[data-kb-card]").map((x) => x.tabIndex).join(","), $$("[data-kb-proto] [data-kb-card]").length);
        // Alt+Right on Alpha: into Done, in Asha's lane, drawn from the template there.
        card("a").focus();
        document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", altKey: true, bubbles: true, cancelable: true })); await wait(0);
        log("moved", board(), document.activeElement.getAttribute("data-kb-card"), S().tasks.map((t) => t.id + "@" + t.column).join(" "));
        // The Like button in Bravo does its own thing; the card doesn't open.
        card("b").querySelector("button").click(); await wait(0);
        log("like", S().likes, S().open);
        // Enter on Bravo opens it.
        card("b").focus();
        document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true })); await wait(0);
        log("open", S().open?.id);
        // A card changed in state: its content follows.
        root.__irender.set("tasks", S().tasks.map((t) => (t.id === "b" ? { ...t, title: "Bravo 2", points: 8 } : t))); await wait(0);
        log("changed", board());
        // Dragged with the mouse into Ravi's Done.
        const dt = new DataTransfer(), cell = $('[data-kb-lane="Ravi"][data-kb-column="done"] [data-kb-cards]'), r = cell.getBoundingClientRect();
        card("c").dispatchEvent(new DragEvent("dragstart", { bubbles: true, dataTransfer: dt }));
        cell.dispatchEvent(new DragEvent("dragover", { bubbles: true, cancelable: true, dataTransfer: dt, clientY: r.bottom - 2 }));
        cell.dispatchEvent(new DragEvent("drop", { bubbles: true, cancelable: true, dataTransfer: dt, clientY: r.bottom - 2 }));
        $("[data-kb-board]").dispatchEvent(new DragEvent("dragend", { bubbles: true, dataTransfer: dt })); await wait(0);
        log("dragged", dt.getData("text/plain"), board());
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test");
    // The server draws the cards from the template too.
    assert!(html.contains("3 pts · #1") && html.contains("Charlie"), "server cards");
    let html = html.replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("kanban-card-template", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "drawn Asha/todo:Alpha(3 pts · #1) Asha/done:Charlie(1 pts · #1) Ravi/todo:Bravo(5 pts · #1) Ravi/done:",
            // Named and movable as the board's own cards, one tab stop; none in the template.
            "own Alpha card true 0,-1,-1 0",
            // Alpha keeps its place (first) in Done; the lanes follow the cards' new order.
            "moved Ravi/todo:Bravo(5 pts · #1) Ravi/done: Asha/todo: Asha/done:Alpha(3 pts · #1),Charlie(1 pts · #2) a b@todo a@done c@done",
            "like 1 ",
            "open b",
            "changed Ravi/todo:Bravo 2(8 pts · #1) Ravi/done: Asha/todo: Asha/done:Alpha(3 pts · #1),Charlie(1 pts · #2)",
            "dragged Charlie Ravi/todo:Bravo 2(8 pts · #1) Ravi/done:Charlie(1 pts · #1) Asha/todo: Asha/done:Alpha(3 pts · #1)",
        ],
        "\n{result}"
    );
}

/// Moving swimlanes with `reorderable`: a grip on each lane's header (not the lane of
/// cards without a value), Up and Down on it, dragging it, and by touch after a hold,
/// the order written to `bindLanes` as `lanes` had it.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn kanban_move_lanes_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": {
            "order": [ { "value": "Ravi", "label": "Ravi K" }, "Asha" ],
            "tasks": [
                { "id": "a", "column": "todo", "title": "Alpha", "assignee": "Asha" },
                { "id": "b", "column": "todo", "title": "Bravo", "assignee": "Ravi" },
                { "id": "c", "column": "todo", "title": "Charlie", "assignee": "Meera" },
                { "id": "d", "column": "todo", "title": "Delta" } ] },
        "root": { "type": "ui-kanban", "props": { "columns": [ { "id": "todo", "title": "To do" } ], "cards": "{{ tasks }}", "bindCards": "tasks",
            "swimlanes": "assignee", "lanes": "{{ order }}", "bindLanes": "order", "reorderable": true } } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const K = document.querySelector("[data-i-x=ui-kanban]"), $ = (s) => K.querySelector(s), $$ = (s) => [...K.querySelectorAll(s)];
      const lanes = () => $$("[data-kb-lane-fold]").map((b) => b.textContent).join(",");
      const order = () => JSON.stringify(S().order);
      const row = (k) => $(`[data-kb-lane-row="${k}"]`);
      try {
        log("grips", $$("[data-kb-lane-grip]").map((g) => g.getAttribute("aria-label")).join(", "));
        // Up on Asha's grip: Asha first, the order written with Ravi's label kept.
        $('[data-kb-lane-grip="Asha"]').focus();
        document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true, cancelable: true })); await wait(0);
        log("up", lanes(), order(), document.activeElement.getAttribute("data-kb-lane-grip"));
        // Dragging Meera's grip onto the top half of Asha's lane: Meera first.
        const dt = new DataTransfer(), ar = row("Asha").getBoundingClientRect();
        $('[data-kb-lane-grip="Meera"]').dispatchEvent(new DragEvent("dragstart", { bubbles: true, dataTransfer: dt }));
        row("Asha").dispatchEvent(new DragEvent("dragover", { bubbles: true, cancelable: true, dataTransfer: dt, clientX: ar.left + 40, clientY: ar.top + 4 }));
        log("over", row("Asha").getAttribute("data-over-drop"));
        row("Asha").dispatchEvent(new DragEvent("drop", { bubbles: true, cancelable: true, dataTransfer: dt, clientX: ar.left + 40, clientY: ar.top + 4 }));
        $("[data-kb-board]").dispatchEvent(new DragEvent("dragend", { bubbles: true, dataTransfer: dt })); await wait(0);
        log("dragged", lanes(), order());
        // By touch: hold Meera's grip, then let go on the lane of cards without a value: last.
        const touch = (t, type, x, y) => t.dispatchEvent(new PointerEvent(type, { bubbles: true, cancelable: true, pointerId: 5, pointerType: "touch", isPrimary: true, clientX: x, clientY: y }));
        const g = $('[data-kb-lane-grip="Meera"]'), gr = g.getBoundingClientRect(), nr = row("").getBoundingClientRect();
        touch(g, "pointerdown", gr.left + 4, gr.top + 4); await wait(350);
        log("held", $$("[data-touch-ghost]").length, $("[data-touch-dragging]")?.hasAttribute("data-kb-lane-head"));
        touch(g, "pointermove", nr.left + 40, nr.top + 6); await wait(0);
        touch(g, "pointerup", nr.left + 40, nr.top + 6); await wait(0);
        log("touched", lanes(), order());
        await wait(60);
        log("said", $("[data-kb-live]").textContent);
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("kanban-move-lanes", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            // No grip on the lane of cards without an assignee.
            "grips Move lane Ravi K, Move lane Asha, Move lane Meera",
            "up Asha,Ravi K,Meera,No assignee [\"Asha\",{\"value\":\"Ravi\",\"label\":\"Ravi K\"},\"Meera\"] Asha",
            "over before",
            "dragged Meera,Asha,Ravi K,No assignee [\"Meera\",\"Asha\",{\"value\":\"Ravi\",\"label\":\"Ravi K\"}]",
            "held 1 true",
            "touched Asha,Ravi K,Meera,No assignee [\"Asha\",{\"value\":\"Ravi\",\"label\":\"Ravi K\"},\"Meera\"]",
            "said Meera moved to position 3 of 3",
        ],
        "\n{result}"
    );
}

/// `swimlanes` and `fields` from state: a setting changes the lanes and what each card
/// shows, and a later move keeps them.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn kanban_lanes_and_fields_from_state_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": {
            "lanes": null, "rich": false,
            "short": [ { "field": "points", "label": "Points" } ],
            "long": [ { "field": "points", "label": "Points" }, "team" ],
            "tasks": [
                { "id": "a", "column": "todo", "title": "Alpha", "assignee": "Asha", "team": "Web", "points": 3 },
                { "id": "b", "column": "doing", "title": "Bravo", "assignee": "Ravi", "team": "Mobile", "points": 5 } ] },
        "root": { "type": "ui-kanban", "props": { "columns": [ "todo", "doing" ], "cards": "{{ tasks }}", "bindCards": "tasks",
            "swimlanes": "{{ lanes }}", "fields": "{{ rich ? long : short }}" } } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), api = root.__irender;
      const K = document.querySelector("[data-i-x=ui-kanban]"), $ = (s) => K.querySelector(s), $$ = (s) => [...K.querySelectorAll(s)];
      const lanes = () => $$("[data-kb-lane-row]").map((r) => r.getAttribute("data-kb-lane-row")).join(",") || "none";
      const fields = () => [...$('[data-kb-card="a"]').querySelectorAll("[data-kb-field]")].map((f) => f.textContent).join(" · ");
      try {
        log("start", lanes(), fields());
        api.set("lanes", "assignee"); await wait(0);
        log("lanes", lanes(), fields());
        api.set("rich", true); await wait(0);
        log("rich", lanes(), fields());
        $('[data-kb-card="a"]').focus();
        document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", altKey: true, bubbles: true, cancelable: true })); await wait(0);
        log("moved", api.state.tasks.map((t) => t.id + "@" + t.column).join(" "), lanes(), fields());
        api.set("lanes", null); await wait(0);
        log("no lanes", lanes(), fields());
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("kanban-lanes-state", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "start none Points3",
            "lanes Asha,Ravi Points3",
            "rich Asha,Ravi Points3 · Web",
            // Moving keeps both: the board redraws from the same settings (lanes in the cards' new order).
            "moved b@doing a@doing Ravi,Asha Points3 · Web",
            "no lanes none Points3 · Web",
        ],
        "\n{result}"
    );
}

/// A board's `filter`: only the cards that pass are shown, counts say "n of total", a
/// limit still counts every card, and moving among the shown cards keeps the hidden ones.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn kanban_filter_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": {
            "who": null, "f": {},
            "tasks": [
                { "id": "a", "column": "todo", "title": "Alpha", "assignee": "Asha", "points": 3 },
                { "id": "b", "column": "todo", "title": "Bravo", "assignee": "Ravi", "points": 5 },
                { "id": "c", "column": "todo", "title": "Charlie", "assignee": "Asha", "points": 8 },
                { "id": "d", "column": "doing", "title": "Delta", "assignee": "Ravi", "points": 2 } ] },
        "root": { "type": "ui-kanban", "props": { "columns": [ { "id": "todo", "title": "To do", "limit": 3 }, "doing" ], "cards": "{{ tasks }}", "bindCards": "tasks",
            "filter": { "assignee": "{{ who }}", "points": "{{ f.points }}" } } } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), api = root.__irender;
      const K = document.querySelector("[data-i-x=ui-kanban]"), $ = (s) => K.querySelector(s), $$ = (s) => [...K.querySelectorAll(s)];
      const board = () => $$("[data-kb-column]").map((c) => c.getAttribute("data-kb-column") + "(" + c.querySelector("[data-kb-count]").textContent + "):" + [...c.querySelectorAll("[data-kb-card]")].map((x) => x.getAttribute("data-kb-card")).join("")).join(" ");
      const order = () => api.state.tasks.map((t) => t.id + "@" + t.column).join(" ");
      try {
        log("all", board());
        api.set("who", "Asha"); await wait(0);
        log("asha", board());
        // Down past Charlie's hidden neighbour Bravo: Alpha goes after Charlie, Bravo stays.
        $('[data-kb-card="a"]').focus();
        document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", altKey: true, bubbles: true, cancelable: true })); await wait(0);
        log("down", board(), "|", order());
        document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", altKey: true, bubbles: true, cancelable: true })); await wait(0);
        log("right", board(), "|", order());
        api.set("who", null); api.set("f", { points: { gte: 5 } }); await wait(0);
        log("points", board());
        api.set("f", {}); await wait(0);
        log("none", board());
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("kanban-filter", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "all todo(3 / 3):abc doing(1):d",
            "asha todo(3 / 3):ac doing(0 of 1):",
            "down todo(3 / 3):ca doing(0 of 1): | b@todo c@todo a@todo d@doing",
            "right todo(2 / 3):c doing(1 of 2):a | b@todo c@todo d@doing a@doing",
            "points todo(2 / 3):bc doing(0 of 2):",
            "none todo(2 / 3):bc doing(2):da",
        ],
        "\n{result}"
    );
}

/// The carousel: slides of different widths stepped one at a time, dots marking and
/// going to slides, `loop` going round at the ends, and `autoplay` moving on by itself
/// while it isn't held by the pointer or paused.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn carousel_dots_loop_and_autoplay_in_chrome() {
    let slide = |w: u32, t: &str| serde_json::json!({ "type": "element", "style": { "width": format!("{w}px"), "height": "60px", "background": "#ddd" }, "children": [ { "type": "text", "props": { "value": t } } ] });
    let slides = || vec![slide(200, "A"), slide(380, "B"), slide(150, "C"), slide(300, "D"), slide(260, "E")];
    let doc = serde_json::json!({ "version": 1, "root": { "type": "stack", "style": { "width": "500px" }, "children": [
        { "type": "ui-carousel", "attrs": { "id": "c" }, "props": { "size": "max-content", "gap": 10, "dots": true, "loop": true }, "children": slides() },
        { "type": "ui-carousel", "attrs": { "id": "p" }, "props": { "size": "max-content", "gap": 10, "autoplay": 400 }, "children": slides() } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      // Headless Chrome doesn't animate smooth scrolling: scroll at once.
      const scrollBy = Element.prototype.scrollBy;
      Element.prototype.scrollBy = function (o) { return scrollBy.call(this, { ...o, behavior: "instant" }); };
      const C = document.getElementById("c"), P = document.getElementById("p");
      const track = (el) => el.querySelector("[data-track]");
      // The slide at the start of the view, by its text.
      const at = (el) => { const t = track(el), x = t.getBoundingClientRect().left; return [...t.children].find((s) => Math.abs(s.getBoundingClientRect().left - x) < 2)?.textContent ?? "?"; };
      const dot = () => [...C.querySelectorAll("[data-dot]")].findIndex((d) => d.hasAttribute("aria-current"));
      const settle = () => wait(60);
      try {
        log("start", at(C), C.querySelectorAll("[data-dot]").length, dot(), C.querySelector("[data-prev]").disabled, track(C).children[1].getAttribute("aria-label"));
        for (const n of [1, 2, 3]) { C.querySelector("[data-next]").click(); await settle(); log("next", at(C), dot()); }
        C.querySelectorAll("[data-dot]")[1].click(); await settle();
        log("dot 2", at(C), dot());
        C.querySelector("[data-prev]").click(); await settle(); C.querySelector("[data-prev]").click(); await settle();
        log("looped back", at(C) === "E" || Math.abs(track(C).scrollLeft) + track(C).clientWidth >= track(C).scrollWidth - 2, dot());
        // Autoplay: it moves on by itself (every 400ms).
        const s0 = track(P).scrollLeft; await wait(450);
        log("autoplay moved", track(P).scrollLeft !== s0);
        // Paused: it stays.
        P.querySelector("[data-play]").click();
        const s1 = track(P).scrollLeft; await wait(900);
        log("paused", track(P).scrollLeft === s1, P.querySelector("[data-play]").getAttribute("aria-pressed"), P.querySelector("[data-play]").getAttribute("aria-label"));
        // Held by the pointer: it stays too.
        P.querySelector("[data-play]").click();
        P.dispatchEvent(new PointerEvent("pointerenter"));
        const s2 = track(P).scrollLeft; await wait(900);
        log("hovered", track(P).scrollLeft === s2);
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("carousel", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            // Five dots, the first marked; with `loop`, Previous is never disabled.
            "start A 5 0 false 2 of 5",
            // A slide at a time, whatever their widths.
            "next B 1",
            "next C 2",
            "next D 3",
            "dot 2 B 1",
            // Previous from the first goes round to the end.
            "looped back true 4",
            "autoplay moved true",
            "paused true true Play slides",
            "hovered true",
        ],
        "\n{result}"
    );
}

/// Command palette pages and `bindSearch`: an item with `page` opens a page in place of
/// the rest (its title as a crumb), pages nest, Backspace in the empty box and the crumb
/// go back, and the search text is in state both ways.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn command_palette_pages_in_chrome() {
    let item = |label: &str, extra: serde_json::Value| {
        let mut props = serde_json::json!({ "label": label });
        props.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        serde_json::json!({ "type": "ui-command-item", "props": props, "on": { "click": { "action": "set_state", "key": "ran", "value": label } } })
    };
    let doc = serde_json::json!({ "version": 1, "state": { "ran": "", "q": "" },
        "root": { "type": "ui-command", "props": { "id": "cmd", "bindSearch": "q" }, "children": [
            item("New file", serde_json::json!({})),
            item("Change theme…", serde_json::json!({ "page": "theme" })),
            item("Teams…", serde_json::json!({ "page": "teams" })),
            { "type": "ui-command-page", "props": { "id": "theme", "title": "Theme" }, "children": [ item("Light", serde_json::json!({})), item("Dark", serde_json::json!({})) ] },
            { "type": "ui-command-page", "props": { "id": "teams", "title": "Teams" }, "children": [
                item("Design", serde_json::json!({})), item("Engineering…", serde_json::json!({ "page": "eng" })) ] },
            { "type": "ui-command-page", "props": { "id": "eng", "title": "Engineering" }, "children": [ item("Ravi", serde_json::json!({})), item("Asha", serde_json::json!({})) ] } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const P = document.getElementById("cmd"), input = P.querySelector("input"), crumb = P.querySelector("[data-ui-crumb]");
      const view = () => [...P.querySelectorAll("[role=option]")].filter((i) => i.getClientRects().length).map((i) => i.querySelector("[data-ui-label]").textContent + (i.getAttribute("aria-selected") === "true" ? "*" : "")).join(", ") + (crumb.hidden ? "" : ` [${crumb.textContent}]`);
      const key = async (k) => { input.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true })); await wait(0); };
      const type = async (t) => { input.value = t; input.dispatchEvent(new Event("input", { bubbles: true })); await wait(0); };
      const pick = async (label) => { [...P.querySelectorAll("[role=option]")].find((i) => i.textContent.includes(label)).click(); await wait(0); };
      try {
        log("top", view());
        await pick("Change theme"); log("theme", view(), "| ran", S().ran);
        await key("Backspace"); log("back", view());
        await type("tea"); log("typed", view(), "| q", S().q);
        await key("Enter"); log("enter on a page item", view(), "| q", JSON.stringify(S().q));
        await pick("Engineering"); log("nested", view());
        await key("Enter"); log("ran", S().ran, view());
        await key("Backspace"); await key("Backspace"); log("back twice", view());
        await pick("Teams"); crumb.click(); await wait(0); log("crumb", view());
        // From state: the search box follows.
        input.blur(); root.__irender.set("q", "dark"); await wait(0);
        log("from state", input.value, view());
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("command-pages", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "top New file*, Change theme…, Teams…",
            // Its page in place of the rest, its title as a crumb; its own `on.click` runs too.
            "theme Light*, Dark [Theme] | ran Change theme…",
            "back New file*, Change theme…, Teams…",
            "typed Teams…* | q tea",
            // Enter on a page's item opens it, and the search starts afresh.
            "enter on a page item Design*, Engineering… [Teams] | q \"\"",
            "nested Ravi*, Asha [Engineering]",
            "ran Ravi Ravi*, Asha [Engineering]",
            "back twice New file*, Change theme…, Teams…",
            "crumb New file*, Change theme…, Teams…",
            // From state; Dark is in the Theme page, not the top, so nothing matches.
            "from state dark ",
        ],
        "\n{result}"
    );
}

/// `animate` on an illustration: its parts move while it's true (it may follow state),
/// and stop when it turns false.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn illustration_animates_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "on": true },
        "root": { "type": "stack", "children": [
            { "type": "ui-illustration", "attrs": { "id": "a" }, "props": { "name": "maintenance", "animate": "{{ on }}" } },
            { "type": "ui-illustration", "attrs": { "id": "b" }, "props": { "name": "maintenance" } } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const running = (id) => document.getElementById(id).getAnimations({ subtree: true }).map((a) => a.animationName).sort().join(",") || "none";
      try {
        await wait(20);
        log("on", running("a"), "| plain", running("b"));
        document.querySelector("[data-i-root]").__irender.set("on", false); await wait(20);
        log("off", running("a"), document.getElementById("a").hasAttribute("data-animate"));
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("illustration-animate", &html) else { return };
    assert_eq!(result.lines().collect::<Vec<_>>(), ["on ill-spin | plain none", "off none false"], "\n{result}");
}

/// Past eight fields, a box over a filter panel's fields finds one by name or section:
/// the others hide (and their section headings), Enter opens the first match, the
/// arrow keys skip what's hidden, and Escape clears the box first.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn filters_find_a_field_in_chrome() {
    let f = |field: &str, label: &str, section: &str| serde_json::json!({ "type": "ui-filter", "props": { "field": field, "label": label, "section": section, "type": "text" } });
    let doc = serde_json::json!({ "version": 1, "state": { "f": {}, "g": {} },
        "root": { "type": "stack", "children": [
            { "type": "ui-filters", "attrs": { "id": "many" }, "props": { "bind": "f" }, "children": [
                f("status", "Status", "Order"), f("total", "Total", "Order"), f("placed", "Placed on", "Order"),
                f("name", "Customer name", "Customer"), f("email", "Email", "Customer"), f("city", "City", "Customer"),
                f("carrier", "Carrier", "Shipping"), f("tracking", "Tracking number", "Shipping"), f("eta", "Delivery date", "Shipping"),
                f("note", "Note", "Order") ] },
            { "type": "ui-filters", "attrs": { "id": "few" }, "props": { "bind": "g" }, "children": [ f("a", "A", "S"), f("b", "B", "S") ] } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const M = document.getElementById("many"), panel = M.querySelector("[data-fl-panel]"), find = M.querySelector("[data-fl-find]");
      const name = (t) => t.firstElementChild.textContent;
      // What's on screen, not only what's marked hidden.
      const seen = (t) => t.getClientRects().length > 0;
      const tabs = () => [...M.querySelectorAll("[data-fl-tab]")].filter(seen).map(name).join(",");
      const sections = () => [...M.querySelectorAll("[data-fl-section]")].filter(seen).map((t) => t.textContent).join(",");
      const current = () => name(M.querySelector("[data-fl-tab][aria-selected=true]"));
      const focused = () => document.activeElement === find ? "find" : document.activeElement.matches("[data-fl-tab]") ? "tab " + name(document.activeElement) : panel.contains(document.activeElement) ? "pane" : "outside";
      const type = async (t) => { find.value = t; find.dispatchEvent(new Event("input", { bubbles: true })); await wait(5); };
      const key = async (k) => { document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true })); await wait(5); };
      try {
        log("few has a box", !!document.querySelector("#few [data-fl-find]"));
        M.querySelector("[data-fl-open]").click(); await wait(20);
        log("open", find.placeholder, focused());
        await type("ship"); log("ship", tabs(), "|", sections());
        await type("date"); log("date", tabs(), "|", sections());
        find.focus(); await key("Enter"); log("enter", current(), focused());
        await type("o"); find.focus(); await key("ArrowDown"); log("down", current(), focused(), "|", tabs());
        await key("ArrowDown"); log("next", focused());
        await key("ArrowUp"); await key("ArrowUp"); log("up past the first", focused());
        await type("zzz"); log("none", tabs() || "-", seen(M.querySelector("[data-fl-unfound]")));
        find.focus(); await key("Escape"); log("escape", JSON.stringify(find.value), panel.open, tabs().split(",").length);
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("filters-find", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "few has a box false",
            "open Find a field… pane",
            // By section: every field under Shipping.
            "ship Carrier,Tracking number,Delivery date | Shipping",
            "date Delivery date | Shipping",
            "enter Delivery date pane",
            // "o" is in the Order and Customer fields; Down from the box goes to the first.
            "down Status tab Status | Status,Total,Placed on,Customer name,Email,City,Note",
            "next tab Total",
            "up past the first find",
            "none - true",
            "escape \"\" true 10",
        ],
        "\n{result}"
    );
}

/// A tile's `when`: hidden, it leaves the layout and the tiles below close up; shown, it
/// takes its own place (or the first free one, without one), the others making room.
/// The saved layout in `bind` isn't touched.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn dashboard_grid_tiles_shown_from_state_in_chrome() {
    let tile = |id: &str, extra: serde_json::Value| {
        let mut props = serde_json::json!({ "id": id, "title": id.to_uppercase() });
        props.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        serde_json::json!({ "type": "ui-tile", "props": props, "children": [ { "type": "text", "props": { "value": id } } ] })
    };
    let on = |id: &str| format!("{{{{ includes(on, '{id}') }}}}");
    let doc = serde_json::json!({ "version": 1, "state": { "layout": null, "on": ["c"] },
        "root": { "type": "element", "style": { "width": "960px" }, "children": [
            { "type": "ui-dashboard-grid", "attrs": { "id": "g" }, "props": { "bind": "layout", "gap": 0, "rowHeight": 50 }, "children": [
                tile("a", serde_json::json!({ "x": 0, "y": 0, "w": 6, "h": 2 })),
                tile("b", serde_json::json!({ "x": 6, "y": 0, "w": 6, "h": 2 })),
                tile("c", serde_json::json!({ "x": 0, "y": 2, "w": 12, "h": 1, "when": on("c") })),
                tile("d", serde_json::json!({ "w": 4, "h": 1, "when": on("d") })),
                tile("e", serde_json::json!({ "x": 0, "y": 4, "w": 4, "h": 1, "when": on("e") })),
                tile("f", serde_json::json!({ "x": 0, "y": 3, "w": 4, "h": 1 })) ] } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const G = document.getElementById("g");
      // The tiles on screen, in cells of 80 by 50 px.
      const seen = () => [...G.querySelectorAll("[data-dg-tile]")].filter((n) => n.offsetParent).map((n) =>
        `${n.dataset.id}:${Math.round(n.offsetLeft / 80)},${Math.round(n.offsetTop / 50)}`).join(" ");
      const set = async (v) => { root.__irender.set("on", v); await wait(50); };
      try {
        log("start", seen());
        await set([]); log("c hidden", seen());
        await set(["d"]); log("d shown", seen());
        await set(["d", "e"]); log("e shown", seen());
        await set(["c", "d", "e"]); log("c back", seen());
        log("bind", JSON.stringify(S().layout));
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("dashboard-grid-when", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            // D and E start hidden and take no room: F sits right under C.
            "start a:0,0 b:6,0 c:0,2 f:0,3",
            // Without C, F floats up into its row.
            "c hidden a:0,0 b:6,0 f:0,2",
            // D has no place: the first free one, beside F.
            "d shown a:0,0 b:6,0 d:4,2 f:0,2",
            // E has one (row 4), and floats up under F.
            "e shown a:0,0 b:6,0 d:4,2 e:0,3 f:0,2",
            // C comes back to row 2; the tiles there move down to make room.
            "c back a:0,0 b:6,0 c:0,2 d:4,3 e:0,4 f:0,3",
            "bind null",
        ],
        "\n{result}"
    );
}

/// A date range with times (`time` and `mode: range`): a start and an end time under the
/// calendar, kept as a new range is picked, in the value, `bindRange`, the form fields
/// and the button; typed as `date time – date time`; 00:00 and 23:59 when not picked.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn date_range_with_times_in_chrome() {
    let doc = serde_json::json!({ "version": 1,
        "state": { "rt": { "from": "2026-03-02T09:00", "to": "2026-03-06T17:30" }, "rtr": null, "typed": null, "plain": null },
        "root": { "type": "stack", "children": [
            { "type": "ui-date-picker", "props": { "id": "rt", "bind": "rt", "bindRange": "rtr", "mode": "range", "time": true, "name": "slot", "locale": "en-US" } },
            { "type": "ui-date-picker", "props": { "id": "typed", "bind": "typed", "mode": "range", "time": true, "input": true, "locale": "en-US" } },
            { "type": "ui-calendar", "props": { "id": "plain", "bind": "plain", "mode": "range", "time": true, "locale": "en-US", "value": null } } ] } });
    let driver = r##"<style>* { transition: none !important; }</style><script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const P = (id) => document.getElementById(id).closest("[data-i-x]");
      const J = (v) => JSON.stringify(v);
      try {
        const rt = P("rt");
        log("shown", rt.querySelector("[data-cal-text]").textContent);
        rt.querySelector("[data-cal-trigger]").click(); await wait(10);
        const times = () => [...rt.querySelectorAll("[data-cal-time-input]")];
        log("inputs", times().map((t) => `${t.getAttribute("aria-label")}=${t.value}`).join(" "));
        const end = times()[1]; end.value = "18:00"; end.dispatchEvent(new Event("change", { bubbles: true })); await wait(10);
        log("end time", J(S().rt));
        rt.querySelector('[data-key="2026-03-10"]').click(); await wait(10);
        rt.querySelector('[data-key="2026-03-12"]').click(); await wait(10);
        log("new range", J(S().rt), J(S().rtr));
        log("form", [...rt.querySelectorAll("input[type=hidden]")].map((i) => `${i.name}=${i.value}`).join(" "));
        log("button", rt.querySelector("[data-cal-text]").textContent);

        const f = P("typed").querySelector("[data-cal-input]");
        log("placeholder", f.placeholder);
        f.value = "03/02/2026 9:00 – 03/04/2026 5:30 pm"; f.dispatchEvent(new Event("input", { bubbles: true })); f.dispatchEvent(new Event("change", { bubbles: true })); await wait(10);
        log("typed", J(S().typed), f.value);
        f.value = "03/02/2026 25:00 – 03/04/2026"; f.dispatchEvent(new Event("input", { bubbles: true })); f.dispatchEvent(new Event("change", { bubbles: true })); await wait(10);
        log("bad time", f.getAttribute("aria-invalid"), f.title);

        // Two days of the month it shows (this one).
        // (Each pick draws the grid again, so the cells are looked up each time.)
        const cal = P("plain"), days = () => [...cal.querySelectorAll("[data-key]:not([data-outside])")];
        const [a, b] = [days()[2].dataset.key, days()[4].dataset.key];
        days()[2].click(); await wait(5); days()[4].click(); await wait(10);
        const v = S().plain;
        log("defaults", v.from === a + "T00:00", v.to === b + "T23:59");
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("date-range-times", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "shown Mar 2, 2026, 9:00 AM – Mar 6, 2026, 5:30 PM",
            "inputs Start time=09:00 End time=17:30",
            r#"end time {"from":"2026-03-02T09:00","to":"2026-03-06T18:00"}"#,
            // A new range keeps the times picked.
            r#"new range {"from":"2026-03-10T09:00","to":"2026-03-12T18:00"} {"from":"2026-03-10T09:00","to":"2026-03-12T18:00"}"#,
            "form slot[from]=2026-03-10T09:00 slot[to]=2026-03-12T18:00",
            "button Mar 10, 2026, 9:00 AM – Mar 12, 2026, 6:00 PM",
            "placeholder MM/DD/YYYY HH:MM – MM/DD/YYYY HH:MM",
            r#"typed {"from":"2026-03-02T09:00","to":"2026-03-04T17:30"} 03/02/2026 9:00 AM – 03/04/2026 5:30 PM"#,
            "bad time true Enter a time like 14:30",
            // Not picked: the whole of each day.
            "defaults true true",
        ],
        "\n{result}"
    );
}

/// `compare` on a range: the previous period and last year's follow the range, a custom
/// one takes two picks, and the comparison shows on the calendar, the button and the form.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn date_range_compare_in_chrome() {
    let doc = serde_json::json!({ "version": 1,
        "state": { "period": { "from": "2026-03-01", "to": "2026-03-31" }, "vs": null },
        "root": { "type": "stack", "children": [
            { "type": "ui-date-picker", "props": { "id": "cmp", "bind": "period", "bindCompare": "vs", "compare": true, "mode": "range", "months": 2, "name": "p", "locale": "en-US" } },
            { "type": "ui-calendar", "props": { "id": "inline", "mode": "range", "compare": ["previous"], "presets": [{ "label": "Week", "from": "2026-03-09", "to": "2026-03-15" }], "locale": "en-US" } } ] } });
    let driver = r##"<style>* { transition: none !important; }</style><script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const P = (id) => document.getElementById(id).closest("[data-i-x]");
      const J = (v) => JSON.stringify(v);
      try {
        const el = P("cmp"), open = async () => { el.querySelector("[data-cal-trigger]").click(); await wait(10); };
        const box = () => el.querySelector("[data-cal-compare]");
        const marked = () => [...el.querySelectorAll("[data-compare]")].map((b) => b.dataset.key);
        await open();
        log("off", box().querySelectorAll("button").length, el.querySelector("[data-cal-vs]").hidden);
        const check = box().querySelector("input"); check.focus(); check.click(); await wait(10);
        log("previous", J(S().vs), document.activeElement.getAttribute("data-cal-ctl"));
        log("text", box().querySelector("[data-cal-compare-text]").textContent);
        log("vs", el.querySelector("[data-cal-vs]").textContent);
        box().querySelector('[data-compare-kind="lastYear"]').click(); await wait(10);
        log("last year", J(S().vs));
        // A new range: the comparison follows it.
        el.querySelector('[data-key="2026-03-10"]').click(); await wait(5);
        el.querySelector('[data-key="2026-03-16"]').click(); await wait(10);
        log("follows", J(S().period), J(S().vs), el.querySelector("[data-cal-pop]").matches(":popover-open"));
        // Custom: two picks on the same calendar.
        await open();
        box().querySelector('[data-compare-kind="custom"]').click(); await wait(10);
        log("custom", box().querySelector("[data-cal-compare-text]").textContent);
        el.querySelector('[data-key="2026-03-04"]').click(); await wait(5);
        log("half", J(S().vs), J(S().period), box().querySelector("[data-cal-compare-text]").textContent);
        el.querySelector('[data-key="2026-03-02"]').click(); await wait(10);
        log("picked", J(S().vs), J(S().period));
        await open();
        log("marked", marked().join(","), el.querySelector('[data-key="2026-03-03"]').getAttribute("aria-label"));
        log("form", [...el.querySelectorAll("input[type=hidden]")].map((i) => `${i.name}=${i.value}`).join(" "));
        log("title", el.querySelector("[data-cal-trigger]").title);
        box().querySelector("input").click(); await wait(10);
        log("off again", J(S().vs), marked().length, el.querySelector("[data-cal-vs]").hidden);

        // Not bound: kept here, and told in `change`.
        const cal = P("inline"); let seen = null;
        cal.addEventListener("change", (e) => (seen = e.detail?.compare));
        cal.querySelector("[data-cal-compare] input").click(); await wait(10);
        cal.querySelector("[data-preset]").click(); await wait(10);
        log("inline", J(seen), cal.querySelectorAll("[data-compare]").length, cal.querySelectorAll("[data-compare-kinds]").length);
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("date-range-compare", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "off 0 true",
            // As many days just before: March's 31 days.
            r#"previous {"type":"previous","from":"2026-01-29","to":"2026-02-28"} compare"#,
            "text vs Jan 29, 2026 – Feb 28, 2026",
            "vs vs Jan 29, 2026 – Feb 28, 2026",
            r#"last year {"type":"lastYear","from":"2025-03-01","to":"2025-03-31"}"#,
            r#"follows {"from":"2026-03-10","to":"2026-03-16"} {"type":"lastYear","from":"2025-03-10","to":"2025-03-16"} false"#,
            "custom Pick the comparison's first day",
            // The range stays as it was while the comparison is picked.
            r#"half {"type":"custom","from":"2026-03-04","to":null} {"from":"2026-03-10","to":"2026-03-16"} Pick the comparison's last day"#,
            r#"picked {"type":"custom","from":"2026-03-02","to":"2026-03-04"} {"from":"2026-03-10","to":"2026-03-16"}"#,
            "marked 2026-03-02,2026-03-03,2026-03-04 Tuesday, March 3, 2026, compared",
            "form p[from]=2026-03-10 p[to]=2026-03-16 p[compareFrom]=2026-03-02 p[compareTo]=2026-03-04",
            "title Mar 10, 2026 – Mar 16, 2026 vs Mar 2, 2026 – Mar 4, 2026",
            "off again null 0 true",
            r#"inline {"type":"previous","from":"2026-03-02","to":"2026-03-08"} 7 0"#,
        ],
        "\n{result}"
    );
}

/// `ui-mindmap`: a tree for screen readers laid out both sides; the arrows; Tab, Enter
/// and typing to add and write topics; folding, deleting (not the middle), undo and redo;
/// dragging a topic onto another (not into itself); `bindSelected`; export; read only.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn mindmap_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "sel": null, "map": { "id": "c", "label": "Centre", "children": [
            { "id": "a", "label": "Alpha", "children": [ { "id": "a1", "label": "A one" }, { "id": "a2", "label": "A two" } ] },
            { "id": "b", "label": "Beta", "children": [ { "id": "b1", "label": "B one" } ] },
            { "id": "g", "label": "Gamma" } ] } },
        "root": { "type": "stack", "children": [
            { "type": "ui-mindmap", "attrs": { "id": "mm" }, "props": { "bind": "map", "bindSelected": "sel", "height": 420 } },
            { "type": "ui-mindmap", "attrs": { "id": "ro" }, "props": { "bind": "map", "readOnly": true, "height": 200 } } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const api = document.querySelector("[data-i-root]").__irender, S = () => api.state;
      const mm = document.getElementById("mm"), stage = mm.querySelector("[data-mm-stage]");
      const topic = (id) => mm.querySelector(`[data-mm-topic="${id}"]`);
      const side = (id) => { const c = topic("c").getBoundingClientRect(), t = topic(id).getBoundingClientRect(); return t.left > c.right ? "R" : t.right < c.left ? "L" : "M"; };
      const key = (k, o = {}) => stage.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, ...o }));
      const tree = (t = S().map) => t.label + (t.children?.length ? `(${t.children.map((c) => tree(c)).join(" ")})` : "");
      const live = () => mm.querySelector("[data-mm-live]").textContent;
      try {
        stage.focus(); await wait(10);
        log("tree", stage.getAttribute("role"), topic("a").getAttribute("role"), topic("a").getAttribute("aria-level"), topic("a").getAttribute("aria-expanded"), stage.getAttribute("aria-activedescendant") === topic("c").id, S().sel);
        log("sides", ["a", "b", "g"].map((id) => `${id}:${side(id)}`).join(" "), side("a1"), side("b1"));
        // The arrows: away from the middle to a child, towards it to the parent, up and down.
        key("ArrowRight"); await wait(5);
        const first = S().sel;
        key(side(first) === "R" ? "ArrowRight" : "ArrowLeft"); await wait(5);
        log("away", first, S().sel, live());
        key("ArrowDown"); await wait(5);
        log("down", S().sel);
        key(side(S().sel) === "R" ? "ArrowLeft" : "ArrowRight"); await wait(5);
        log("towards", S().sel);
        key("Home"); await wait(5);
        log("home", S().sel);

        // Tab adds a topic inside Alpha, written straight away.
        key("ArrowRight"); await wait(5);
        while (S().sel !== "a") { key("ArrowDown"); await wait(5); if (S().sel === "g" || S().sel === "b") { key("ArrowUp"); await wait(5); } if (S().sel !== "a" && S().sel !== "b" && S().sel !== "g") break; }
        key("Tab"); await wait(10);
        const ed = mm.querySelector("[data-mm-edit]");
        log("tab", !!ed, ed?.value, document.activeElement === ed);
        ed.value = "A three"; ed.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })); await wait(10);
        log("written", tree(), document.activeElement === stage);
        // Enter adds one beside it; Escape leaves its name as it is.
        key("Enter"); await wait(10);
        mm.querySelector("[data-mm-edit]").dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })); await wait(10);
        log("beside", tree());
        // Typing writes over the topic.
        key("Z"); await wait(10);
        const over = mm.querySelector("[data-mm-edit]");
        log("typed", over.value);
        over.value = "Zed"; over.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })); await wait(10);
        log("renamed", tree());

        // Fold Alpha, unfold it.
        stage.focus(); S(); api.set("sel", "a");
        topic("a").dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, pointerId: 1, button: 0 })); stage.dispatchEvent(new PointerEvent("pointerup", { bubbles: true, pointerId: 1 })); await wait(10);
        key(" "); await wait(10);
        log("folded", S().map.children[0].collapsed, topic("a").getAttribute("aria-expanded"), !!topic("a1"), mm.querySelector('[data-mm-fold="a"] text')?.textContent);
        key(" "); await wait(10);
        log("unfolded", !!topic("a1"));
        // Delete Beta (and what's inside); the middle can't go.
        topic("b").dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, pointerId: 1, button: 0 })); stage.dispatchEvent(new PointerEvent("pointerup", { bubbles: true, pointerId: 1 })); await wait(10);
        key("Delete"); await wait(10);
        log("deleted", tree(), live());
        key("Home"); key("Delete"); await wait(10);
        log("not the middle", !!S().map);
        key("z", { metaKey: true }); await wait(10);
        log("undone", tree());
        key("z", { metaKey: true, shiftKey: true }); await wait(10);
        log("redone", tree());
        key("z", { metaKey: true }); await wait(10);

        // Drag "A one" onto Gamma; then try Alpha into its own child (nothing happens).
        const drag = async (from, to) => {
          const f = topic(from).getBoundingClientRect(), t = topic(to).getBoundingClientRect();
          const p = (type, x, y, target) => (target ?? stage).dispatchEvent(new PointerEvent(type, { bubbles: true, pointerId: 2, button: 0, clientX: x, clientY: y }));
          p("pointerdown", f.left + 10, f.top + f.height / 2, topic(from).querySelector("rect:not([data-mm-ring])"));
          for (let k = 1; k <= 5; k++) { p("pointermove", f.left + 10 + (t.left + t.width / 2 - f.left - 10) * k / 5, f.top + f.height / 2 + (t.top + t.height / 2 - f.top - f.height / 2) * k / 5); await wait(2); }
          p("pointerup", t.left + t.width / 2, t.top + t.height / 2); await wait(10);
        };
        await drag("a1", "g");
        log("dragged", tree(), S().sel);
        await drag("a", "a2");
        log("not into itself", tree());

        // Export, as the file would be saved.
        let saved = null;
        const make = URL.createObjectURL;
        URL.createObjectURL = (b) => { saved = b; return make.call(URL, b); };
        HTMLAnchorElement.prototype.click = function () {};
        mm.querySelector('[aria-label="Export as SVG"]').click(); await wait(10);
        const text = await saved.text();
        log("export", saved.type, text.startsWith("<svg"), text.includes(">Centre</text>"), text.includes("role="));

        const ro = document.getElementById("ro");
        ro.querySelector("[data-mm-stage]").focus();
        ro.querySelector("[data-mm-stage]").dispatchEvent(new KeyboardEvent("keydown", { key: "Tab", bubbles: true }));
        ro.querySelector("[data-mm-stage]").dispatchEvent(new KeyboardEvent("keydown", { key: "x", bubbles: true })); await wait(10);
        log("read only", ro.querySelectorAll("[data-mm-bar] button").length, !!ro.querySelector("[data-mm-edit]"), tree() === tree());
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("mindmap", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            r#"tree tree treeitem 2 true true c"#,
            r#"sides a:R b:L g:L R L"#,
            r#"away a a1 A one, level 3"#,
            r#"down a2"#,
            r#"towards a"#,
            r#"home c"#,
            r#"tab true New topic true"#,
            r#"written Centre(Alpha(A one A two A three) Beta(B one) Gamma) true"#,
            r#"beside Centre(Alpha(A one A two A three New topic) Beta(B one) Gamma)"#,
            r#"typed Z"#,
            r#"renamed Centre(Alpha(A one A two A three Zed) Beta(B one) Gamma)"#,
            r#"folded true false false 4"#,
            r#"unfolded true"#,
            r#"deleted Centre(Alpha(A one A two A three Zed) Gamma) Beta and 1 inside it deleted"#,
            r#"not the middle true"#,
            r#"undone Centre(Alpha(A one A two A three Zed) Beta(B one) Gamma)"#,
            r#"redone Centre(Alpha(A one A two A three Zed) Gamma)"#,
            r#"dragged Centre(Alpha(A two A three Zed) Beta(B one) Gamma(A one)) a1"#,
            r#"not into itself Centre(Alpha(A two A three Zed) Beta(B one) Gamma(A one))"#,
            r#"export image/svg+xml true true false"#,
            r#"read only 5 false true"#,
        ],
        "\n{result}"
    );
}

/// `ui-whiteboard`: the items drawn; selecting (a click, Shift, a box), moving, undo and
/// redo; the pen, a rectangle dragged out, a note placed and written in; the keys (N, Tab,
/// ⌘D, Delete, arrows); zooming into `bindView`; export; and read only.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn whiteboard_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "view": null, "board": [
            { "id": "n1", "type": "note", "x": 0, "y": 0, "w": 100, "h": 80, "text": "Idea" },
            { "id": "r1", "type": "rect", "x": 200, "y": 0, "w": 100, "h": 60, "color": "#2563eb" },
            { "id": "a1", "type": "arrow", "x": 110, "y": 30, "points": [[0, 0], [80, 0]] } ] },
        "root": { "type": "stack", "children": [
            { "type": "ui-whiteboard", "attrs": { "id": "wb" }, "props": { "bind": "board", "bindView": "view", "height": 400 } },
            { "type": "ui-whiteboard", "attrs": { "id": "ro" }, "props": { "bind": "board", "readOnly": true, "height": 200 } } ] } });
    let driver = r##"<style>* { transition: none !important; }</style><script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const api = document.querySelector("[data-i-root]").__irender, S = () => api.state;
      const wb = document.getElementById("wb"), stage = wb.querySelector("[data-wb-stage]");
      const ids = () => S().board.map((i) => i.id).join(",");
      const at = (id) => { const i = S().board.find((x) => x.id === id); return `${Math.round(i.x)},${Math.round(i.y)}`; };
      const shown = () => [...wb.querySelectorAll("[data-wb-item]")].map((g) => g.dataset.type).join(",");
      const sel = () => wb.querySelectorAll("[data-wb-stage] svg > g:last-child > rect[stroke-dasharray]").length;
      const live = () => wb.querySelector("[data-wb-live]").textContent;
      // A pointer on the board at board point (x, y), the view being (24, 24) in at 100%.
      const r = () => stage.getBoundingClientRect();
      const pt = (x, y) => ({ clientX: r().left + x + 24, clientY: r().top + y + 24 });
      const ev = (type, x, y, o = {}, target) => (target ?? document.elementFromPoint(pt(x, y).clientX, pt(x, y).clientY) ?? stage).dispatchEvent(new PointerEvent(type, { bubbles: true, pointerId: 1, button: 0, buttons: 1, isPrimary: true, ...pt(x, y), ...o }));
      const drag = async (x0, y0, x1, y1, o) => { ev("pointerdown", x0, y0, o); await wait(5); for (let k = 1; k <= 4; k++) { ev("pointermove", x0 + (x1 - x0) * k / 4, y0 + (y1 - y0) * k / 4, o, stage); await wait(2); } ev("pointerup", x1, y1, o, stage); await wait(10); };
      const key = (k, o = {}) => stage.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, ...o }));
      try {
        log("drawn", shown(), wb.querySelector("[data-wb-note] div").textContent, wb.querySelector("[data-wb-empty]").hidden);
        // Select the note, Shift-add the rectangle, move both by dragging the note.
        ev("pointerdown", 50, 40); ev("pointerup", 50, 40, {}, stage); await wait(10);
        log("picked", sel(), live());
        ev("pointerdown", 250, 30, { shiftKey: true }); ev("pointerup", 250, 30, {}, stage); await wait(10);
        log("two", sel());
        await drag(50, 40, 80, 70);
        log("moved", at("n1"), at("r1"), at("a1"), live());
        key("z", { metaKey: true }); await wait(10);
        log("undone", at("n1"), at("r1"));
        key("z", { metaKey: true, shiftKey: true }); await wait(10);
        log("redone", at("n1"));
        key("Escape"); await wait(5);
        // A box round empty space and the arrow only.
        await drag(100, -20, 210, 60);
        log("box", sel());
        key("Escape");

        // The pen.
        key("p"); await wait(5);
        await drag(0, 200, 120, 260);
        const pen = S().board.at(-1);
        log("pen", pen.type, pen.points.length > 3, Math.round(pen.x), Math.round(pen.y), wb.querySelector('[data-wb-tool="pen"]').getAttribute("aria-pressed"));
        // A rectangle dragged out; the tool goes back to Select.
        key("r"); await wait(5);
        await drag(300, 200, 420, 280);
        const rect = S().board.at(-1);
        log("rect", rect.type, Math.round(rect.w), Math.round(rect.h), wb.querySelector('[data-wb-tool="select"]').getAttribute("aria-pressed"));
        // A note with a click, written in.
        wb.querySelector('[data-wb-tool="note"]').click(); await wait(5);
        ev("pointerdown", 500, 100); ev("pointerup", 500, 100, {}, stage); await wait(10);
        const ed = wb.querySelector("[data-wb-edit]");
        log("note", S().board.at(-1).type, !!ed, document.activeElement === ed);
        ed.value = "Ship it"; ed.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })); await wait(10);
        log("written", S().board.at(-1).text, !wb.querySelector("[data-wb-edit]"), document.activeElement === stage);

        // The keys: Tab goes along, ⌘D duplicates, arrows nudge, Delete deletes, N adds a note.
        key("Tab"); await wait(5);
        log("tab", live().split(",")[0]);
        const before = S().board.length;
        key("d", { metaKey: true }); await wait(10);
        log("dup", S().board.length - before);
        const copy = S().board.at(-1);
        key("ArrowRight", { shiftKey: true }); await wait(10);
        log("nudge", Math.round(S().board.at(-1).x - copy.x));
        key("Delete"); await wait(10);
        log("deleted", S().board.length === before, live());
        key("n"); await wait(10);
        log("key note", S().board.at(-1).type, !!wb.querySelector("[data-wb-edit]"));
        wb.querySelector("[data-wb-edit]").dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })); await wait(10);

        // Zoom at a point with ⌘ and the wheel, then fit.
        stage.dispatchEvent(new WheelEvent("wheel", { bubbles: true, cancelable: true, deltaY: -300, ctrlKey: true, ...pt(0, 0) })); await wait(10);
        log("zoom", S().view.zoom > 2.5, wb.querySelector("[data-wb-zoom]").textContent);
        key("0"); await wait(10);
        log("fit", S().view.zoom < 1.5);

        // Export: the file as it would be saved.
        let saved = null;
        const make = URL.createObjectURL;
        URL.createObjectURL = (b) => { saved = b; return make.call(URL, b); };
        HTMLAnchorElement.prototype.click = function () {};
        wb.querySelector('[aria-label="Export as SVG"]').click(); await wait(10);
        const text = await saved.text();
        log("export", saved.type, text.startsWith("<svg"), text.includes(">Ship it</text>"));

        const ro = document.getElementById("ro");
        log("read only", [...ro.querySelectorAll("[data-wb-tool]")].map((b) => b.dataset.wbTool).join(","), !!ro.querySelector('[aria-label="Undo"]'));
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("whiteboard", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            r#"drawn note,rect,arrow Idea true"#,
            r#"picked 1 Sticky note: Idea, 1 of 3"#,
            r#"two 2"#,
            r#"moved 30,30 230,30 110,30 2 items moved"#,
            r#"undone 0,0 200,0"#,
            r#"redone 30,30"#,
            r#"box 1"#,
            r#"pen pen true 0 200 true"#,
            r#"rect rect 120 80 true"#,
            r#"note note true true"#,
            r#"written Ship it true true"#,
            r#"tab Sticky note: Idea"#,
            r#"dup 1"#,
            r#"nudge 10"#,
            r#"deleted true 1 item deleted"#,
            r#"key note note true"#,
            r#"zoom true 272%"#,
            r#"fit true"#,
            r#"export image/svg+xml true true"#,
            r#"read only select,hand false"#,
        ],
        "\n{result}"
    );
}

/// `ui-approval`: where it stands by each stage's rule, deciding on your turn (not on
/// another's, nor a later stage's), a reason needed to reject or ask for changes, `done`,
/// sending again, changes pausing it, and decisions from elsewhere.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn approval_in_chrome() {
    let doc = serde_json::json!({ "version": 1,
        "state": { "me": "asha", "st": null, "dones": 0, "flow": [
            { "id": "lead", "title": "Lead", "approvers": [ { "id": "ravi", "name": "Ravi Kumar", "decision": "approved" } ] },
            { "id": "rev", "title": "Reviewers", "rule": 2, "approvers": [ { "id": "asha", "name": "Asha Rao" }, { "id": "mei", "name": "Mei Lin", "decision": "approved" }, { "id": "dev", "name": "Dev Lal" } ] },
            { "id": "legal", "title": "Legal", "approvers": [ { "id": "sam", "name": "Sam Okafor" } ] } ] },
        "root": { "type": "stack", "children": [
            { "type": "ui-approval", "attrs": { "id": "ap" }, "props": { "bind": "flow", "me": "{{ me }}", "owner": "kim", "bindStatus": "st" },
              "on": { "done": { "action": "set_state", "key": "dones", "value": "{{ dones + 1 }}" } } },
            { "type": "ui-approval", "attrs": { "id": "ro" }, "props": { "bind": "flow", "me": "asha", "readOnly": true } } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const api = document.querySelector("[data-i-root]").__irender, S = () => api.state;
      const ap = document.getElementById("ap"), $ = (s) => ap.querySelector(s);
      const states = () => [...ap.querySelectorAll("[data-ap-stage]")].map((s) => s.dataset.state).join(",");
      const top = () => $("[data-ap-summary]").textContent;
      const act = (d) => $(`[data-ap-act="${d}"]`);
      const as = async (who) => { api.set("me", who); await wait(10); };
      try {
        log("start", states(), top(), S().st, [...ap.querySelectorAll("[data-ap-rule]")].map((r) => r.textContent).join(" | "));
        log("mine", !!act("approved"), !!document.getElementById("ro").querySelector("[data-ap-act]"));
        act("approved").click(); await wait(10);
        log("approved", states(), top(), JSON.stringify(S().flow[1].approvers[0]).includes('"decision":"approved"'), !!S().flow[1].approvers[0].at, $("[data-ap-live]").textContent);
        await as("dev");
        log("not their turn", !!act("approved"));
        await as("sam");
        act("rejected").click(); await wait(10);
        log("needs a reason", $("[data-ap-error]")?.textContent, $("textarea").getAttribute("aria-invalid"), document.activeElement === $("textarea"));
        $("textarea").value = "Missing the data clause."; $("textarea").dispatchEvent(new Event("input", { bubbles: true }));
        act("rejected").click(); await wait(10);
        log("rejected", states(), top(), S().st, S().dones, !!act("resend"));
        await as("kim");
        log("owner", !!act("resend"));
        act("resend").click(); await wait(10);
        log("sent again", states(), S().st, S().flow.flatMap((s) => s.approvers).some((p) => p.decision));
        await as("ravi");
        $("textarea").value = "Add the cost centre."; $("textarea").dispatchEvent(new Event("input", { bubbles: true }));
        act("changes").click(); await wait(10);
        log("changes", states(), top(), S().st);
        // A decision from elsewhere: Ravi takes it back and approves.
        api.set("flow", S().flow.map((s, i) => (i ? s : { ...s, approvers: [{ id: "ravi", name: "Ravi Kumar", decision: "approved" }] }))); await wait(10);
        log("from elsewhere", states(), top());
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("approval", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            r#"start approved,current,waiting Waiting for Reviewers1 of 2 approvals · stage 2 of 3 pending Approved | Waiting · 2 of 3 must approve | Up next"#,
            r#"mine true false"#,
            r#"approved approved,approved,current Waiting for Legal0 of 1 approvals · stage 3 of 3 true true Approved: Reviewers. Waiting for Legal. 0 of 1 approvals · stage 3 of 3"#,
            r#"not their turn false"#,
            r#"needs a reason Say why you're rejecting it true true"#,
            r#"rejected approved,approved,rejected Rejected by Sam OkaforMissing the data clause. rejected 1 false"#,
            r#"owner true"#,
            r#"sent again current,waiting,waiting pending false"#,
            r#"changes changes,waiting,waiting Ravi Kumar asked for changesAdd the cost centre. changes"#,
            r#"from elsewhere approved,current,waiting Waiting for Reviewers0 of 2 approvals · stage 2 of 3"#,
        ],
        "\n{result}"
    );
}

/// `ui-file-browser`: a folder's items (folders first), opening and going up, selecting
/// with the keys, opening a file, renaming (and a clash), a new folder, deleting and
/// undoing, moving from the menu and by dragging onto the breadcrumb (not into itself),
/// searching, sorting and the list view.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn file_browser_in_chrome() {
    let doc = serde_json::json!({ "version": 1,
        "state": { "folder": null, "sel": [], "opened": null, "opens": 0, "files": [
            { "id": "docs", "name": "Docs", "folder": true },
            { "id": "pics", "name": "Pictures", "folder": true },
            { "id": "a", "name": "b.txt", "size": 10 },
            { "id": "b", "name": "a.pdf", "size": 5000 },
            { "id": "c", "name": "c.zip", "size": 900 },
            { "id": "d", "name": "Inside.docx", "parent": "docs", "size": 1 } ] },
        "root": { "type": "ui-file-browser", "attrs": { "id": "fm" }, "props": { "bind": "files", "bindFolder": "folder", "bindSelection": "sel", "bindOpen": "opened", "rootLabel": "Home" },
            "on": { "open": { "action": "set_state", "key": "opens", "value": "{{ opens + 1 }}" } } } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const api = document.querySelector("[data-i-root]").__irender, S = () => api.state;
      const fm = document.getElementById("fm"), $ = (s) => fm.querySelector(s);
      const names = () => [...fm.querySelectorAll("[data-fm-item]")].map((i) => i.querySelector("[data-fm-name]")?.textContent ?? "[input]").join(",");
      const crumbs = () => [...fm.querySelectorAll("[data-fm-crumbs] button")].map((b) => b.textContent).join(" > ");
      const item = (id) => fm.querySelector(`[data-fm-item="${id}"]`);
      const key = (k, o = {}) => { const t = document.activeElement; t.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, ...o })); };
      const files = () => S().files.map((f) => `${f.name}@${f.parent ?? "-"}`).sort().join(" ");
      const live = () => $("[data-fm-live]").textContent;
      const btn = (t) => [...fm.querySelectorAll("[data-fm-bar] > button")].find((b) => b.textContent === t);
      try {
        log("top", names(), $("[data-fm-foot]").textContent, crumbs());
        item("docs").dispatchEvent(new MouseEvent("dblclick", { bubbles: true })); await wait(10);
        log("opened folder", S().folder, crumbs(), names(), live());
        key("Backspace"); await wait(10);
        log("up", S().folder, crumbs(), $("[data-fm-files]").getAttribute("aria-activedescendant")?.endsWith("docs"), JSON.stringify(S().sel));
        // The keys: right to the next, Shift+right adds, ⌘A all, Escape none.
        key("ArrowRight"); await wait(5); key("ArrowRight", { shiftKey: true }); await wait(5);
        log("keys", JSON.stringify(S().sel), $("[data-fm-foot]").textContent);
        key("a", { metaKey: true }); await wait(5);
        log("all", S().sel.length); key("Escape"); await wait(5);
        key("End"); await wait(5); key("Enter"); await wait(10);
        log("open file", S().opened?.name, S().opens, live());

        // Rename b.txt; then try a name already there.
        item("a").click(); await wait(5); key("F2"); await wait(10);
        const input = $("[data-fm-rename]");
        log("rename", input.value, input.selectionStart, input.selectionEnd);
        input.value = "notes.txt"; input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })); await wait(10);
        log("renamed", S().files.find((f) => f.id === "a").name, live());
        item("a").click(); await wait(5); key("F2"); await wait(10);
        const again = $("[data-fm-rename]"); again.value = "A.PDF"; again.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })); await wait(10);
        log("clash", !!$("[data-fm-rename]"), $("[data-fm-rename]")?.getAttribute("aria-invalid"), live());
        $("[data-fm-rename]").dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })); await wait(10);

        btn("New folder").click(); await wait(10);
        const nf = $("[data-fm-rename]");
        log("new folder", nf?.value, S().files.filter((f) => f.folder).length);
        nf.value = "Archive"; nf.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })); await wait(10);
        log("named", names());

        item("c").click(); await wait(5); key("Delete"); await wait(10);
        log("deleted", names().includes("c.zip"), $("[data-fm-undo]").textContent.replace("Undo", " [Undo]"));
        $("[data-fm-undo] button").click(); await wait(10);
        log("undone", names().includes("c.zip"), $("[data-fm-undo]").hidden);

        // Move c.zip into Pictures from the menu.
        item("c").click(); await wait(5); btn("Move to…").click(); await wait(10);
        const menu = fm.querySelector("[data-fm-menu]");
        log("menu", [...menu.querySelectorAll("button")].map((b) => b.textContent).join(","));
        [...menu.querySelectorAll("button")].find((b) => b.textContent === "Pictures").click(); await wait(10);
        log("moved", S().files.find((f) => f.id === "c").parent, live());
        // Docs can't go into itself: its own name isn't offered.
        item("docs").click(); await wait(5); btn("Move to…").click(); await wait(10);
        log("not into itself", [...fm.querySelectorAll("[data-fm-menu] button")].map((b) => b.textContent).join(","));
        document.querySelector("[data-fm-menu]").hidePopover(); await wait(10);

        // Drag a.pdf into Docs, then back to Home on the breadcrumb.
        const drag = async (from, to) => {
          const dt = new DataTransfer();
          from.dispatchEvent(new DragEvent("dragstart", { bubbles: true, dataTransfer: dt }));
          to.dispatchEvent(new DragEvent("dragover", { bubbles: true, cancelable: true, dataTransfer: dt }));
          to.dispatchEvent(new DragEvent("drop", { bubbles: true, cancelable: true, dataTransfer: dt }));
          from.dispatchEvent(new DragEvent("dragend", { bubbles: true, dataTransfer: dt }));
          await wait(10);
        };
        item("b").click(); await wait(5);
        await drag(item("b"), item("docs"));
        log("dragged", S().files.find((f) => f.id === "b").parent, live());
        item("docs").dispatchEvent(new MouseEvent("dblclick", { bubbles: true })); await wait(10);
        item("b").click(); await wait(5);
        await drag(item("b"), fm.querySelector('[data-fm-crumb=""] button'));
        log("to the top", S().files.find((f) => f.id === "b").parent ?? "-", names());
        fm.querySelector('[data-fm-crumb=""] button').click(); await wait(10);

        const search = $("[data-fm-bar] input[type=search]"); search.value = "PDF"; search.dispatchEvent(new Event("input", { bubbles: true })); await wait(10);
        log("search", names(), document.activeElement === $("[data-fm-bar] input[type=search]"));
        const field = $("[data-fm-bar] input[type=search]"); field.value = ""; field.dispatchEvent(new Event("input", { bubbles: true })); await wait(10);
        const sort = $("[data-fm-bar] select"); sort.value = "size"; sort.dispatchEvent(new Event("change", { bubbles: true })); await wait(10);
        log("by size", names());
        btn("List").click(); await wait(10);
        log("list", $("[data-fm-files]").dataset.view, !!$("[data-fm-head]"), item("b").querySelector("[data-fm-size]").textContent);
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("file-browser", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            r#"top Docs,Pictures,a.pdf,b.txt,c.zip 5 items Home"#,
            r#"opened folder docs Home &gt; Docs Inside.docx Docs, 1 item"#,
            r#"up  Home true ["docs"]"#,
            r#"keys ["pics","b"] 2 of 5 selected"#,
            r#"all 5"#,
            r#"open file c.zip 1 Opened c.zip"#,
            r#"rename b.txt 0 1"#,
            r#"renamed notes.txt Renamed to notes.txt"#,
            r#"clash true true There's already a A.PDF here"#,
            r#"new folder New folder 3"#,
            r#"named Archive,Docs,Pictures,a.pdf,c.zip,notes.txt"#,
            r#"deleted false c.zip deleted [Undo]"#,
            r#"undone true true"#,
            r#"menu Home,Archive,Docs,Pictures"#,
            r#"moved pics c.zip moved to Pictures"#,
            r#"not into itself Home,Archive,Pictures"#,
            r#"dragged docs a.pdf moved to Docs"#,
            r#"to the top - Inside.docx"#,
            r#"search a.pdf true"#,
            r#"by size Archive,Docs,Pictures,a.pdf,notes.txt"#,
            r#"list list true 5 KB"#,
        ],
        "\n{result}"
    );
}

/// `ui-virtual-list`: a hundred thousand items with only those in view in the page, their
/// places for screen readers, the range in view, `end` near the end (once a length),
/// items of different heights, and an empty list.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn virtual_list_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "rows": [], "range": null, "ends": 0, "tall": [] },
        "root": { "type": "stack", "children": [
            { "type": "ui-virtual-list", "attrs": { "id": "v" }, "props": { "items": "{{ rows }}", "height": 300, "itemHeight": 30, "bindRange": "range", "label": "Rows" },
              "on": { "end": { "action": "set_state", "key": "ends", "value": "{{ ends + 1 }}" } },
              "children": [ { "type": "element", "style": { "height": "30px" }, "children": [ { "type": "text", "props": { "value": "{{ item.name }} #{{ index }}" } } ] } ] },
            { "type": "ui-virtual-list", "attrs": { "id": "t" }, "props": { "items": "{{ tall }}", "height": 200, "emptyText": "No rows" },
              "children": [ { "type": "element", "style": { "height": "{{ item.h }}px" }, "children": [ { "type": "text", "props": { "value": "{{ item.id }}" } } ] } ] } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      // Past the list's 16 ms scroll throttle and its first measuring.
      const frames = () => wait(60);
      const api = document.querySelector("[data-i-root]").__irender, S = () => api.state;
      const v = document.getElementById("v").closest("[data-vl]");
      const drawn = (el = v) => [...el.querySelectorAll("[data-vl-item]")];
      const first = (el = v) => drawn(el)[0];
      try {
        const t0 = performance.now();
        api.set("rows", Array.from({ length: 100000 }, (_, i) => ({ id: i, name: `Row ${i}` }))); await frames();
        log("loaded", drawn().length < 60, first().textContent, first().getAttribute("aria-posinset"), first().getAttribute("aria-setsize"), v.scrollHeight > 100000 * 25, performance.now() - t0 < 3000);
        log("range", JSON.stringify(S().range), S().ends);
        v.scrollTop = 50000 * 30; v.dispatchEvent(new Event("scroll")); await frames();
        const shown = drawn().map((d) => Number(d.getAttribute("aria-posinset")));
        log("middle", drawn().length < 60, shown.includes(50001), S().range.from > 49000 && S().range.to < 51000, S().range.total);
        v.scrollTop = v.scrollHeight; v.dispatchEvent(new Event("scroll")); await frames();
        log("end", drawn().at(-1).textContent, S().ends, S().range.to);
        v.dispatchEvent(new Event("scroll")); await frames();
        log("end again", S().ends);
        api.set("rows", [...S().rows, ...Array.from({ length: 50 }, (_, i) => ({ id: 100000 + i, name: `More ${i}` }))]); await frames();
        v.scrollTop = v.scrollHeight; v.dispatchEvent(new Event("scroll")); await frames();
        log("more", drawn().at(-1).textContent, S().ends, drawn().at(-1).getAttribute("aria-setsize"));

        const t = document.getElementById("t").closest("[data-vl]");
        log("empty", getComputedStyle(t.querySelector("[data-vl-empty]")).display, t.querySelector("[data-vl-empty]").textContent);
        api.set("tall", Array.from({ length: 1000 }, (_, i) => ({ id: `T${i}`, h: i % 2 ? 80 : 20 }))); await frames();
        t.scrollTop = 500 * 50; t.dispatchEvent(new Event("scroll")); await frames();
        const at = drawn(t).map((d) => Number(d.getAttribute("aria-posinset")));
        // Placed by their average height: what's drawn covers the view, with no gap.
        const pad = t.querySelector("[data-vl-pad=top]").offsetHeight, tall = drawn(t).reduce((h, d) => h + d.getBoundingClientRect().height, 0);
        log("heights", at.length < 40, pad <= t.scrollTop, pad + tall >= t.scrollTop + t.clientHeight, getComputedStyle(t.querySelector("[data-vl-empty]")).display);
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("virtual-list", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            // A few dozen of 100,000 in the page, each with its place in the whole list.
            "loaded true Row 0 #0 1 100000 true true",
            r#"range {"from":0,"to":21,"total":100000} 0"#,
            "middle true true true 100000",
            "end Row 99999 #99999 1 99999",
            // `end` once for each length of list.
            "end again 1",
            "more More 49 #100049 2 100050",
            "empty block No rows",
            "heights true true true none",
        ],
        "\n{result}"
    );
}

/// `ui-presence`: the stack (named with the count, past `max` behind `+N`), the list (each
/// status in words, `(you)`, what they're on), colours by place or as given, and people
/// joining and leaving from state, said with `announce`.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn presence_in_chrome() {
    let doc = serde_json::json!({ "version": 1,
        "state": { "here": [
            { "id": "a", "name": "Asha Rao", "status": "active", "where": "Budget" },
            { "id": "k", "name": "Kaushik Sundar", "status": "idle" },
            { "id": "r", "name": "Ravi Kumar", "status": "busy", "color": 7 },
            { "id": "m", "name": "Mei Lin", "initials": "ML", "status": "offline" } ] },
        "root": { "type": "ui-presence", "attrs": { "id": "p" }, "props": { "people": "{{ here }}", "me": "k", "max": 2, "announce": true } } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const api = document.querySelector("[data-i-root]").__irender;
      const root = document.getElementById("p").closest("[data-pr]");
      const stack = root.querySelector("[data-pr-stack]"), more = root.querySelector("[data-pr-more]");
      const shown = () => [...stack.querySelectorAll("[data-pr-av]")].filter((a) => a.getClientRects().length).map((a) => a.title).join(",");
      const list = () => [...root.querySelectorAll("[data-pr-list] li")].map((li) => `${li.querySelector("[data-pr-name]").textContent.trim()} / ${li.querySelector("[data-pr-where]").textContent}`).join(" | ");
      try {
        log("stack", stack.getAttribute("aria-label"), shown(), more.hidden, more.textContent);
        stack.click(); await wait(20);
        log("open", root.querySelector("[data-pr-pop]").matches(":popover-open"), root.querySelector("[data-pr-pop] h3").textContent);
        log("list", list());
        const rings = [...root.querySelectorAll("[data-pr-list] [data-pr-av]")].map((a) => a.dataset.color + ":" + a.dataset.status).join(" ");
        log("rings", rings, getComputedStyle(root.querySelector("[data-pr-list] [data-pr-av][data-status=active]"), "::after").backgroundColor !== "rgba(0, 0, 0, 0)");
        api.set("here", [...api.state.here, { id: "p", name: "Priya Nair", status: "active" }]); await wait(20);
        log("joined", stack.getAttribute("aria-label"), more.textContent, root.querySelector("[data-pr-live]").textContent);
        api.set("here", api.state.here.filter((x) => x.id === "a" || x.id === "p")); await wait(20);
        log("left", shown(), more.hidden, root.querySelector("[data-pr-live]").textContent);
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("presence", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "stack People here: 4 Asha Rao,Kaushik Sundar false +2",
            "open true People here · 4",
            "list Asha Rao / Active · Budget | Kaushik Sundar (you) / Idle | Ravi Kumar / Busy | Mei Lin / Offline",
            // Colours by place, or as given (Ravi's 7).
            "rings 1:active 2:idle 7:busy 4:offline true",
            "joined People here: 5 +3 Priya Nair joined",
            "left Asha Rao,Priya Nair true Kaushik Sundar left. Ravi Kumar left. Mei Lin left",
        ],
        "\n{result}"
    );
}

/// `ui-timeline` as an activity feed: a heading over each day, times as how long ago, New
/// after `since` (following state), and entries past `limit` behind a button; entries
/// from state drawn again as they change.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn timeline_feed_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "feed": [], "seen": null },
        "root": { "type": "stack", "children": [
            { "type": "ui-timeline", "attrs": { "id": "f" }, "props": { "items": "{{ feed }}", "group": "day", "relative": true, "since": "{{ seen }}", "limit": 4 } },
            { "type": "ui-timeline", "attrs": { "id": "g" }, "props": { "group": "day" }, "children": [
                { "type": "ui-timeline-item", "props": { "who": "Asha Rao", "action": "paid", "when": "2025-03-04T14:30" } } ] } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const api = document.querySelector("[data-i-root]").__irender;
      const f = document.getElementById("f"), n = Date.now(), ago = (min) => new Date(n - min * 60000).toISOString();
      const days = () => [...f.querySelectorAll("[data-tl-day]:not([hidden])")].map((d) => d.textContent).join(", ");
      const times = () => [...f.querySelectorAll("[data-tl-item]:not([data-tl-hidden]) time")].map((t) => t.textContent).join(", ");
      const fresh = () => [...f.querySelectorAll("[data-tl-new]:not([hidden])")].length;
      const shown = () => f.querySelectorAll("[data-tl-item]:not([data-tl-hidden])").length;
      const more = () => f.querySelector("[data-tl-show]");
      try {
        // Today (now and 5 minutes ago), yesterday, five days ago, and three older.
        const midnight = new Date(); midnight.setHours(0, 0, 0, 0);
        const sinceMidnight = (Date.now() - midnight) / 60000;
        api.set("seen", ago(3));
        api.set("feed", [
          { id: 1, who: "Asha Rao", action: "commented", when: ago(0) },
          { id: 2, who: "Ravi Kumar", action: "paid", when: ago(Math.min(5, sinceMidnight - 1)) },
          { id: 3, action: "Backup", when: ago(sinceMidnight + 60) },
          { id: 4, who: "Mei Lin", action: "joined", when: ago(sinceMidnight + 4 * 1440 + 60) },
          { id: 5, action: "Renewed", when: ago(sinceMidnight + 40 * 1440) },
          { id: 6, action: "Opened", when: ago(sinceMidnight + 41 * 1440) },
        ]); await wait(20);
        const weekday = new Intl.DateTimeFormat(undefined, { weekday: "long" }).format(new Date(n - (sinceMidnight + 4 * 1440 + 60) * 60000));
        log("days", days().replace(weekday, "WEEKDAY"));
        log("times", times().split(", ").slice(0, 2).join(", "), f.querySelector("time").title.length > 10);
        log("new", fresh(), shown(), more().hidden, more().textContent);
        api.set("seen", ago(-1)); await wait(20);
        log("all seen", fresh());
        more().querySelector("button").click(); await wait(20);
        log("more", shown(), more().hidden, document.activeElement?.closest("[data-tl-item]") === f.querySelectorAll("[data-tl-item]")[4]);
        api.set("feed", [{ id: 0, who: "Kaushik", action: "joined", when: ago(0) }, ...api.state.feed]); await wait(20);
        log("added", shown(), days().split(", ")[0]);
        const g = document.getElementById("g");
        log("fixed", g.querySelector("[data-tl-day]").textContent, g.querySelector("time").textContent.replace(/\s/g, " "), g.querySelector("time").getAttribute("datetime").slice(0, 10));
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("timeline-feed", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            // The older days are among the entries held back.
            "days Today, Yesterday, WEEKDAY",
            "times just now, 5 min. ago true",
            "new 1 4 false Show 2 more",
            "all seen 0",
            // Shown: focus on the first entry that was held back.
            "more 6 true true",
            "added 7 Today",
            // Grouped without `relative`: the time of day under the date's heading.
            "fixed March 4, 2025 2:30 PM 2025-03-04",
        ],
        "\n{result}"
    );
}

/// `ui-compare-slider`: the range moves the divider and `bind`, state moves them back, a
/// side's label fades as it's covered, and the clip follows: across, up and down, right
/// to left, and after the mouse with `hover`.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn compare_slider_in_chrome() {
    // A blank 400×200 picture.
    let img = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='400' height='200'/%3E";
    let doc = serde_json::json!({ "version": 1, "state": { "at": 50 },
        "root": { "type": "stack", "style": { "width": "400px" }, "children": [
            { "type": "ui-compare-slider", "attrs": { "id": "a" }, "props": { "bind": "at", "before": img, "after": img, "beforeAlt": "Before", "afterAlt": "After" } },
            { "type": "ui-compare-slider", "attrs": { "id": "v" }, "props": { "before": img, "after": img, "orientation": "vertical", "ratio": "2/1", "hover": true, "position": 20 } },
            { "type": "element", "attrs": { "dir": "rtl" }, "children": [
                { "type": "ui-compare-slider", "attrs": { "id": "r" }, "props": { "before": img, "after": img, "position": 30, "beforeLabel": "" } } ] } ] } });
    let driver = r##"<style>* { transition: none !important; }</style><script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const api = document.querySelector("[data-i-root]").__irender, S = () => api.state;
      const $ = (id, s) => document.getElementById(id).querySelector(s);
      const clip = (id) => getComputedStyle($(id, "[data-cmp-after]")).clipPath;
      const line = (id) => { const f = $(id, "[data-cmp-frame]").getBoundingClientRect(), l = $(id, "[data-cmp-line]").getBoundingClientRect(); return Math.round(((l.left + l.width / 2 - f.left) / f.width) * 100); };
      try {
        const a = $("a", "input");
        log("start", getComputedStyle(document.getElementById("a")).getPropertyValue("--pos"), a.getAttribute("aria-valuetext"), clip("a"), line("a"));
        a.value = "30"; a.dispatchEvent(new Event("input", { bubbles: true })); await wait(10);
        log("dragged", S().at, clip("a"), line("a"), a.getAttribute("aria-valuetext"));
        api.set("at", 5); await wait(10);
        log("from state", a.value, document.getElementById("a").dataset.near, getComputedStyle($("a", "[data-cmp-tag=before]")).opacity);
        api.set("at", 95); await wait(10);
        log("other side", document.getElementById("a").dataset.near, getComputedStyle($("a", "[data-cmp-tag=after]")).opacity);

        const v = document.getElementById("v"), f = $("v", "[data-cmp-frame]").getBoundingClientRect();
        log("vertical", clip("v"), $("v", "input").getAttribute("aria-orientation"), Math.round(f.width / f.height * 10) / 10);
        $("v", "[data-cmp-frame]").dispatchEvent(new PointerEvent("pointermove", { bubbles: true, pointerType: "mouse", clientX: f.left + 10, clientY: f.top + f.height * 0.75 }));
        await wait(10);
        log("hover", $("v", "input").value, clip("v"));

        log("rtl", clip("r"), line("r"), document.getElementById("r").querySelectorAll("[data-cmp-tag]").length);
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("compare-slider", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "start 50 Before 50%, after 50% inset(0px 0px 0px 50%) 50",
            "dragged 30 inset(0px 0px 0px 30%) 30 Before 30%, after 70%",
            // A side's label fades as the divider covers it.
            "from state 5 before 0",
            "other side after 0",
            "vertical inset(20% 0px 0px) vertical 2",
            "hover 75 inset(75% 0px 0px)",
            // Right to left: the first picture on the right; an empty label left out.
            "rtl inset(0px 30% 0px 0px) 70 1",
        ],
        "\n{result}"
    );
}

/// `ui-countdown`: the status as thresholds pass, said once each; `end` on reaching zero
/// (not for a deadline already past on arrival); counting up; `endText`; units, clock and
/// tiles; times from when the page opened. Under the test browser's virtual time the
/// clock hardly moves, so the deadline is moved instead.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn countdown_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "due": null, "left": null, "st": null, "ended": 0, "far": null },
        "root": { "type": "stack", "children": [
            { "type": "ui-countdown", "attrs": { "id": "c" }, "props": { "to": "{{ due }}", "warn": "8s", "danger": 3, "bindLeft": "left", "bindStatus": "st", "label": "Reply due" },
              "on": { "end": { "action": "set_state", "key": "ended", "value": "{{ ended + 1 }}" } } },
            { "type": "ui-countdown", "attrs": { "id": "rel" }, "props": { "to": "+1h 30m", "from": "-30m" } },
            { "type": "ui-countdown", "attrs": { "id": "past" }, "props": { "to": "-90s" }, "on": { "end": { "action": "set_state", "key": "ended", "value": 99 } } },
            { "type": "ui-countdown", "attrs": { "id": "stop" }, "props": { "to": "-1s", "overdue": false, "endText": "Sale over" } },
            { "type": "ui-countdown", "attrs": { "id": "u" }, "props": { "to": "{{ far }}" } },
            { "type": "ui-countdown", "attrs": { "id": "k" }, "props": { "to": "{{ far }}", "format": "clock" } },
            { "type": "ui-countdown", "attrs": { "id": "t" }, "props": { "to": "{{ far }}", "variant": "tiles" } } ] } });
    let driver = r##"<style>* { transition: none !important; }</style><script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const api = document.querySelector("[data-i-root]").__irender, S = () => api.state;
      const q = (id, s = "[data-cd-text]") => document.getElementById(id).querySelector(s);
      const c = document.getElementById("c");
      const at = (sec) => new Date(Date.now() + sec * 1000 + 400).toISOString();
      const show = () => [S().st, S().left, c.dataset.tone, q("c").textContent, q("c", "[data-cd-live]").textContent].join(" | ");
      try {
        api.set("due", at(20)); await wait(20);
        log("ok", show());
        api.set("due", at(7)); await wait(20);
        log("warning", show());
        api.set("due", at(7)); await wait(20);
        log("again", q("c", "[data-cd-live]").textContent === "Reply due: 7 seconds left");
        api.set("due", at(2)); await wait(20);
        log("danger", show());
        api.set("due", at(-1)); await wait(20);
        log("over", show(), "ended", S().ended);
        api.set("due", at(-3725)); await wait(20);
        log("later", q("c").textContent, "ended", S().ended);
        log("past on arrival", q("past").textContent, "|", q("stop").textContent);
        log("relative", q("rel").textContent, Math.round(parseFloat(document.getElementById("rel").style.getPropertyValue("--used"))));
        api.set("far", at(2 * 86400 + 4 * 3600 + 5 * 60)); await wait(20);
        log("days", q("u").textContent, "|", q("k").textContent, "|", [...document.querySelectorAll("#t [data-cd-tile] b")].map((b) => b.textContent).join(":"));
        api.set("far", at(65)); await wait(20);
        log("minutes", q("u").textContent, "|", q("k").textContent);
        log("timer", c.getAttribute("role"), c.getAttribute("aria-label"), q("c").getAttribute("datetime") === new Date(Date.parse(S().due)).toISOString());
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("countdown", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "ok ok | 20 | neutral | 20s | ",
            // Each threshold said once, as it passes.
            "warning warning | 7 | warning | 7s | Reply due: 7 seconds left",
            "again true",
            "danger danger | 2 | danger | 2s | Reply due: 2 seconds left",
            "over over | -1 | danger | 1s overdue | Reply due: Overdue ended 1",
            "later 1h 2m overdue ended 1",
            // Already past when the page opened: no `end`.
            "past on arrival 1m 30s overdue | Sale over",
            // +1h 30m from -30m: a quarter used.
            "relative 1h 30m 25",
            "days 2d 4h | 2d 04:05:00 | 02:04:05:00",
            "minutes 1m 05s | 0:01:05",
            "timer timer Reply due true",
        ],
        "\n{result}"
    );
}

/// `ui-form-builder`: adding from the palette, the inspector (the key following the label
/// until it's typed), moving and deleting, the preview's checks and answers, and the
/// document, which must resolve and render as a page of its own.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn form_builder_in_chrome() {
    let doc = serde_json::json!({ "version": 1,
        "state": { "fields": [{ "id": "name", "type": "text", "label": "Name", "required": true }, { "id": "team", "type": "select", "label": "Team", "options": ["A", "B"] }], "doc": null },
        "root": { "type": "ui-form-builder", "attrs": { "id": "fb" }, "props": { "bind": "fields", "bindDocument": "doc", "submitLabel": "Send" } } });
    let driver = r##"<style>* { transition: none !important; }</style><script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const api = document.querySelector("[data-i-root]").__irender, S = () => api.state;
      const fb = document.getElementById("fb"), $ = (s) => fb.querySelector(s);
      const ids = () => S().fields.map((f) => f.id).join(",");
      const type = (input, v, ev = "input") => { input.value = v; input.dispatchEvent(new Event(ev, { bubbles: true })); };
      const live = () => $("[data-fb-live]").textContent;
      try {
        log("start", ids(), $('[aria-current=true]')?.dataset.fbCard, $("[data-fb-inspector] input").value);
        const num = $('[data-fb-type="number"]'); num.focus(); num.click(); await wait(10);
        log("added", ids(), live(), document.activeElement === num, $('[aria-current=true]').dataset.fbCard);
        const ins = () => [...$("[data-fb-inspector]").querySelectorAll("input, textarea")];
        log("inspector", ins().map((i) => i.id.split("-in-")[1]).join(","));
        type(ins()[0], "Guests"); await wait(10);
        log("label", ids(), ins()[1].value);
        type(ins().find((i) => i.id.endsWith("-min")), "1"); type(ins().find((i) => i.id.endsWith("-max")), "10"); await wait(10);
        type(ins()[1], "Party size!", "change"); await wait(10);
        type(ins()[0], "Guests coming"); await wait(10);
        log("key typed", ids(), JSON.stringify(S().fields[1]));
        ins().find((i) => i.type === "checkbox").click(); await wait(10);
        const grip = $('[data-fb-card="party_size"] [data-fb-grip]'); grip.focus();
        grip.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true })); await wait(10);
        log("moved", ids(), live(), document.activeElement?.closest("[data-fb-card]")?.dataset.fbCard);
        $('[data-fb-card="team"] [data-fb-pick]').click(); await wait(10);
        type($("[data-fb-inspector] textarea"), "Red\n\nBlue \nGreen"); await wait(10);
        log("choices", JSON.stringify(S().fields.find((f) => f.id === "team").options));
        $('[data-fb-card="team"] [data-fb-duplicate]').click(); await wait(10);
        log("duplicated", ids());
        $('[data-fb-card="team_copy"] [data-fb-delete]').click(); await wait(10);
        log("deleted", ids(), live());

        $('[data-fb-tab="preview"]').click(); await wait(10);
        const pv = $('[data-fb-panel="preview"]');
        pv.querySelector("form").requestSubmit(); await wait(10);
        log("tried", [...pv.querySelectorAll("[data-fb-error]")].map((e) => e.textContent).join(" | "), document.activeElement?.id?.split("-pv-")[1]);
        const by = (id) => pv.querySelector(`[id$="-pv-${id}"]`);
        type(by("name"), "Asha"); type(by("party_size"), "12"); await wait(10);
        log("limit", [...pv.querySelectorAll("[data-fb-error]")].map((e) => e.textContent).join(" | "));
        type(by("party_size"), "3"); await wait(10);
        pv.querySelector("form").requestSubmit(); await wait(10);
        log("sent", pv.querySelector("pre")?.textContent.replace(/\s+/g, ""));

        $('[data-fb-tab="document"]').click(); await wait(10);
        const code = () => $('[data-fb-panel="document"] code').textContent;
        log("yaml", code().split("\n").slice(0, 3).join(" / "));
        [...$('[data-fb-panel="document"]').querySelectorAll("button")].find((b) => b.textContent === "JSON").click(); await wait(10);
        log("json", JSON.parse(code()).root.type, JSON.stringify(JSON.parse(code())) === JSON.stringify(S().doc));
        log("DOC", JSON.stringify(S().doc));
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("form-builder", &html) else { return };
    let (lines, made) = result.split_once("\nDOC ").unwrap_or_else(|| panic!("\n{result}"));
    assert_eq!(
        lines.lines().collect::<Vec<_>>(),
        [
            "start name,team name Name",
            // Added after the field picked; focus stays on the palette.
            "added name,number,team Number added: Number true number",
            "inspector label,key,desc,ph,min,max,req",
            // The key follows the label until it's typed.
            "label name,guests,team guests",
            r#"key typed name,party_size,team {"id":"party_size","type":"number","label":"Guests coming","min":1,"max":10}"#,
            "moved party_size,name,team Guests coming moved to 1 of 3 party_size",
            r#"choices ["Red","Blue","Green"]"#,
            "duplicated party_size,name,team,team_copy",
            "deleted party_size,name,team Team (copy) deleted",
            // The preview checks the form as the document will.
            "tried Guests coming is required. | Name is required. party_size",
            "limit Guests coming must be at most 10.",
            r#"sent {"party_size":3,"name":"Asha","team":null}"#,
            "yaml version: 1 / state: /   party_size: null",
            "json stack true",
        ],
        "\n{result}"
    );
    // The document the builder made is a page of its own.
    let made = irender_core::parse(made.trim(), Format::Json).unwrap_or_else(|e| panic!("{e}\n{made}"));
    let ir = engine.resolve(&made).unwrap_or_else(|d| panic!("{d}"));
    engine.render(&ir, &serde_json::json!({})).unwrap();
}

/// Every field type, with an `endpoint`: the document the builder makes resolves and
/// renders, and posts each answer.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn form_builder_every_type_in_chrome() {
    let fields: Vec<serde_json::Value> = ["text", "textarea", "email", "number", "date", "select", "radio", "checkboxes", "checkbox", "section"]
        .iter()
        .map(|t| serde_json::json!({ "id": format!("f_{t}"), "type": t, "label": format!("A {t}"), "required": true, "description": "Help", "placeholder": "Type", "options": ["x", "y"], "min": if *t == "date" { serde_json::json!("2026-01-01") } else { serde_json::json!(1) }, "max": if *t == "date" { serde_json::json!("2026-12-31") } else { serde_json::json!(9) } }))
        .collect();
    let doc = serde_json::json!({ "version": 1, "state": { "fields": fields, "doc": null },
        "root": { "type": "ui-form-builder", "props": { "bind": "fields", "bindDocument": "doc", "endpoint": "/api/answers" } } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      await new Promise((r) => setTimeout(r, 50));
      const pre = document.createElement("pre"); pre.id = "result";
      pre.textContent = JSON.stringify(document.querySelector("[data-i-root]").__irender.state.doc); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("form-builder-types", &html) else { return };
    let made: serde_json::Value = serde_json::from_str(result.trim()).unwrap_or_else(|e| panic!("{e}\n{result}"));
    // Posted as JSON: every answer, from state.
    assert_eq!(made["root"]["on"]["submit"]["endpoint"], "/api/answers", "{made}");
    assert_eq!(made["root"]["on"]["submit"]["body"].as_object().unwrap().len(), 9, "{made}");
    assert_eq!(made["root"]["props"]["rules"]["f_email"]["type"], "email", "{made}");
    assert_eq!(made["root"]["props"]["rules"]["f_date"]["min"], "2026-01-01", "{made}");
    let parsed = irender_core::parse(&made.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&parsed).unwrap_or_else(|d| panic!("{d}\n{made}"));
    engine.render(&ir, &serde_json::json!({})).unwrap();
    // Each type as the kit's own control; the section holds the fields after it (none here).
    let text = made.to_string();
    for kind in ["ui-input", "ui-textarea", "ui-number-input", "ui-date-picker", "ui-select", "ui-radio-group", "ui-toggle-group", "ui-checkbox", "ui-form-section", "ui-error-summary"] {
        assert!(text.contains(&format!("\"type\":\"{kind}\"")), "{kind} missing: {made}");
    }
}

/// `ui-sortable-list`: the keyboard (pick up, move, put down, put back; Alt and the arrows),
/// dragging a grip with a pointer, removing, plain values, and each item's own content.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn sortable_list_in_chrome() {
    let doc = serde_json::json!({ "version": 1,
        "state": { "tasks": [{ "id": "a", "label": "Alpha" }, { "id": "b", "label": "Bravo" }, { "id": "c", "label": "Charlie" }, { "id": "d", "label": "Delta" }], "tags": ["red", "green", "blue"] },
        "root": { "type": "stack", "children": [
            { "type": "ui-sortable-list", "attrs": { "id": "t" }, "props": { "bind": "tasks", "numbered": true, "removable": true }, "children": [
                { "type": "text", "props": { "value": "{{ item.label }} ({{ item.id }})" } } ] },
            { "type": "ui-sortable-list", "attrs": { "id": "g" }, "props": { "bind": "tags", "label": "Tags" } } ] } });
    let driver = r##"<style>* { transition: none !important; }</style><script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const api = document.querySelector("[data-i-root]").__irender, S = () => api.state;
      const t = document.getElementById("t"), live = () => t.querySelector("[data-sl-live]").textContent;
      const grip = (k, el = t) => el.querySelector(`[data-sl-item="${k}"] [data-sl-grip]`);
      const key = (target, k, o = {}) => target.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, ...o }));
      const shown = (el = t) => [...el.querySelectorAll("[data-sl-item]")].map((li) => li.textContent.trim()).join(" | ");
      const order = () => S().tasks.map((x) => x.id).join("");
      try {
        log("start", shown(), grip("a").getAttribute("aria-label"));
        grip("b").focus(); key(grip("b"), " "); await wait(10);
        log("picked", live(), t.querySelector("[data-sl-picked]")?.getAttribute("data-sl-item"));
        key(document.activeElement, "ArrowDown"); await wait(10);
        key(document.activeElement, "ArrowDown"); await wait(10);
        log("moved", order(), live(), document.activeElement === grip("b"), shown());
        key(document.activeElement, "Escape"); await wait(10);
        log("put back", order(), live());
        key(grip("b"), "Home", {}); await wait(10);
        grip("d").focus(); key(grip("d"), "ArrowUp", { altKey: true }); await wait(10);
        log("alt", order(), document.activeElement === grip("d"));
        grip("a").focus(); key(grip("a"), " "); await wait(5); key(document.activeElement, "End"); await wait(10); key(document.activeElement, " "); await wait(10);
        log("end", order(), live(), !!t.querySelector("[data-sl-picked]"));

        // A pointer drags Charlie's grip below Bravo.
        const r = (k) => t.querySelector(`[data-sl-item="${k}"]`).getBoundingClientRect();
        const g = grip("c"), from = g.getBoundingClientRect();
        const p = (type, y) => g.dispatchEvent(new PointerEvent(type, { bubbles: true, pointerId: 7, pointerType: "touch", isPrimary: true, clientX: from.left + 5, clientY: y }));
        p("pointerdown", from.top + 5);
        p("pointermove", from.top + 20);
        const below = r("b").bottom - 2;
        p("pointermove", below); await wait(5);
        log("dragging", !!t.querySelector("[data-sl-dragging]"), [...t.querySelectorAll("[data-sl-drop]")].map((li) => li.dataset.slItem + ":" + li.dataset.slDrop).join(","));
        p("pointerup", below); await wait(10);
        log("dropped", order(), !!t.querySelector("[data-sl-dragging], [data-sl-drop]"));

        t.querySelector('[data-sl-item="b"] [data-sl-remove]').click(); await wait(10);
        log("removed", order(), live(), document.activeElement?.closest("[data-sl-item]")?.dataset.slItem, shown());

        const gl = document.getElementById("g");
        log("plain", shown(gl), grip("green", gl).getAttribute("aria-label"));
        grip("blue", gl).focus(); key(grip("blue", gl), "ArrowUp", { altKey: true }); await wait(10);
        log("plain moved", S().tags.join(","));
        api.set("tags", []); await wait(10);
        log("empty", getComputedStyle(gl.querySelector("[data-sl-empty]")).display, getComputedStyle(t.querySelector("[data-sl-empty]")).display);
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("sortable-list", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "start 1Alpha (a)✕ | 2Bravo (b)✕ | 3Charlie (c)✕ | 4Delta (d)✕ Move Alpha",
            "picked Bravo picked up, at 2 of 4 b",
            // Moved twice: the places renumber and the grip keeps focus.
            "moved acdb Bravo moved to 4 of 4 true 1Alpha (a)✕ | 2Charlie (c)✕ | 3Delta (d)✕ | 4Bravo (b)✕",
            "put back abcd Bravo put back at 2",
            "alt abdc true",
            "end bdca Alpha put down at 4 of 4 false",
            // Charlie's grip dragged just under Bravo: it lands before Delta.
            "dragging true d:before",
            "dropped bcda false",
            // Focus goes to the item after the one removed.
            "removed cda Bravo removed c 1Charlie (c)✕ | 2Delta (d)✕ | 3Alpha (a)✕",
            "plain red | green | blue Move green",
            "plain moved red,blue,green",
            "empty block none",
        ],
        "\n{result}"
    );
}

/// `ui-gauge` follows state without a script of its own: the arc, the needle, the target,
/// the band's tone and what screen readers hear.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn gauge_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "v": 72, "goal": 85 },
        "root": { "type": "ui-gauge", "attrs": { "id": "g" }, "props": { "value": "{{ v }}", "unit": "%", "target": "{{ goal }}", "needle": true,
            "bands": [{ "to": 60, "tone": "danger", "label": "Poor" }, { "to": 85, "tone": "warning", "label": "Fair" }, { "tone": "success", "label": "Good" }] } } });
    let driver = r##"<style>* { transition: none !important; }</style><script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const api = document.querySelector("[data-i-root]").__irender, g = document.getElementById("g");
      const show = () => [
        // As a number: engines write it differently (`calc(28px)`, `28px`, `0%`).
        Math.round(parseFloat(getComputedStyle(g.querySelector("[data-arc]")).strokeDashoffset.replace(/^calc\(/, ""))),
        getComputedStyle(g.querySelector("[data-needle]")).transform !== "none",
        g.dataset.tone, g.getAttribute("aria-valuetext"), g.querySelector("[data-gauge-value]").textContent,
      ].join(" | ");
      const angle = (sel) => { const m = new DOMMatrix(getComputedStyle(g.querySelector(sel)).transform); return Math.round(Math.atan2(m.b, m.a) * 180 / Math.PI); };
      try {
        log("start", show(), angle("[data-needle]"), angle("[data-target]"));
        api.set("v", 95); await wait(20);
        log("high", show(), angle("[data-needle]"));
        api.set("v", 140); await wait(20);
        log("past max", show(), angle("[data-needle]"));
        api.set("v", 10); api.set("goal", 50); await wait(20);
        log("low", show(), angle("[data-target]"));
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("gauge", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            // 72 of 100: the arc's dash 28 short of the end, the needle at -120° + 72 × 2.4°,
            // the target (85) at 84°.
            "start 28 | true | warning | 72 %, Fair | 72% 53 84",
            "high 5 | true | success | 95 %, Good | 95% 108",
            // Past the end: the dial stops there, the figure doesn't.
            "past max 0 | true | success | 140 %, Good | 140% 120",
            "low 90 | true | danger | 10 %, Poor | 10% 0",
        ],
        "\n{result}"
    );
}

/// `ui-color-picker`: the area's keys, the hue and opacity sliders, typed colours (any
/// CSS colour, or a mistake), swatches, clearing, each format, and state set elsewhere.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn color_picker_in_chrome() {
    let doc = serde_json::json!({ "version": 1,
        "state": { "a": "#1e3a8a", "s": null, "r": null },
        "root": { "type": "stack", "children": [
            { "type": "ui-color-picker", "props": { "id": "a", "bind": "a", "alpha": true, "clearable": true, "name": "brand", "swatches": ["#16a34a", { "value": "#ffffff", "label": "White" }] } },
            { "type": "ui-color-picker", "props": { "id": "g", "inline": true, "format": "hsl", "value": "#808080" } },
            { "type": "ui-color-picker", "props": { "id": "s", "bind": "s", "custom": false, "swatches": [{ "value": "#ef4444", "label": "Red" }, { "value": "#3b82f6", "label": "Blue" }] } },
            { "type": "ui-color-picker", "props": { "id": "r", "bind": "r", "inline": true, "format": "rgb", "alpha": true } } ] } });
    let driver = r##"<style>* { transition: none !important; }</style><script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), api = root.__irender, S = () => api.state;
      const P = (id) => document.getElementById(id).closest("[data-i-x]");
      const key = (t, k, shift) => t.dispatchEvent(new KeyboardEvent("keydown", { key: k, shiftKey: !!shift, bubbles: true }));
      const slide = (t, v) => { t.value = String(v); t.dispatchEvent(new Event("input", { bubbles: true })); };
      const type = (t, v) => { t.value = v; t.dispatchEvent(new Event("input", { bubbles: true })); t.dispatchEvent(new Event("change", { bubbles: true })); };
      try {
        const a = P("a"), $ = (s) => a.querySelector(s);
        log("start", $("[data-color-value]").textContent, $("[data-color-chip]").style.getPropertyValue("--swatch"));
        $("[data-color-trigger]").click(); await wait(10);
        log("open", document.activeElement.hasAttribute("data-color-area"), $("[data-color-hue]").value, $("[data-color-text]").value);
        key($("[data-color-area]"), "ArrowUp", true); await wait(5);
        log("brighter", S().a, $("[data-color-area]").getAttribute("aria-valuetext"));
        slide($("[data-color-hue]"), 0); await wait(5);
        log("hue", S().a);
        slide($("[data-color-alpha]"), 50); await wait(5);
        log("alpha", S().a);
        type($("[data-color-text]"), "rebeccapurple"); await wait(5);
        log("typed", S().a);
        type($("[data-color-text]"), "nope"); await wait(5);
        log("mistake", S().a, $("[data-color-text]").getAttribute("aria-invalid"));
        $("[data-color-text]").dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })); await wait(5);
        log("escape", $("[data-color-text]").value, $("[data-color-text]").getAttribute("aria-invalid"));
        $('[data-swatch="1"]').click(); await wait(5);
        log("swatch", S().a, [...a.querySelectorAll("[data-swatch]")].map((b) => b.getAttribute("aria-pressed")).join(","), $("[data-color-live]").textContent);
        log("form", a.querySelector("input[type=hidden]").value);
        $("[data-color-x]").click(); await wait(5);
        log("cleared", S().a, $("[data-color-value]").textContent, a.hasAttribute("data-empty"), $("[data-color-x]").hidden);
        api.set("a", "hsl(0, 100%, 50%)"); await wait(5);
        log("from state", $("[data-color-value]").textContent, $("[data-color-hue]").value);

        // Grey keeps the hue picked: the format writes it.
        const g = P("g"); let seen = null;
        g.addEventListener("change", (e) => (seen = e.detail.value));
        slide(g.querySelector("[data-color-hue]"), 200); await wait(5);
        log("grey", seen, g.querySelector("[data-color-text]").value);
        key(g.querySelector("[data-color-area]"), "ArrowRight", true); await wait(5);
        log("hsl", seen);

        // Only swatches: a pick closes it.
        const sw = P("s");
        sw.querySelector("[data-color-trigger]").click(); await wait(10);
        log("swatches", document.activeElement.getAttribute("aria-label"), !!sw.querySelector("[data-color-area]"));
        sw.querySelector('[data-swatch="1"]').click(); await wait(10);
        log("picked", S().s, sw.querySelector("[data-color-pop]").matches(":popover-open"), document.activeElement === sw.querySelector("[data-color-trigger]"));

        const r = P("r"), rt = r.querySelector("[data-color-text]");
        type(rt, "hsl(120, 100%, 50%)"); await wait(5);
        log("rgb", S().r);
        type(rt, "#ff000080"); await wait(5);
        log("rgba", S().r, r.querySelector("[data-color-alpha]").value);
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("color-picker", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "start #1e3a8a linear-gradient(#1e3a8a, #1e3a8a)",
            "open true 224 #1e3a8a",
            // Shift+↑: ten points brighter.
            "brighter #2445a3 Saturation 78%, brightness 64%",
            "hue #a32424",
            "alpha #a3242480",
            // Any CSS colour; a whole colour has no opacity.
            "typed #663399",
            "mistake #663399 true",
            "escape #663399 ",
            "swatch #ffffff false,true White picked",
            "form #ffffff",
            "cleared  Pick a colour true true",
            "from state #ff0000 0",
            // Grey keeps the hue picked.
            "grey hsl(200, 0%, 50%) hsl(200, 0%, 50%)",
            "hsl hsl(200, 5%, 48%)",
            "swatches Red false",
            "picked #3b82f6 false true",
            "rgb rgb(0, 255, 0)",
            "rgba rgba(255, 0, 0, 0.5) 50",
        ],
        "\n{result}"
    );
}

/// `loading` on a command palette: while items are fetched, a Loading row shows (and
/// the list says it's busy) and the empty message waits; items that arrive show.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn command_palette_loading_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "q": "", "searching": false, "people": [] },
        "root": { "type": "ui-command", "props": { "id": "cmd", "bindSearch": "q", "loading": "{{ searching }}", "loadingText": "Finding people…" }, "children": [
            { "type": "list", "props": { "items": "{{ people }}", "as": "p", "key": "{{ p }}" }, "style": { "display": "contents" }, "children": [
                { "type": "ui-command-item", "props": { "label": "{{ p }}" } } ] } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const api = document.querySelector("[data-i-root]").__irender;
      const P = document.getElementById("cmd"), input = P.querySelector("input");
      const loading = P.querySelector("[data-ui-loading]"), empty = P.querySelector("[data-ui-empty]");
      const view = () => [`loading ${loading.hidden ? "hidden" : loading.textContent}`, `empty ${empty.hidden ? "hidden" : "shown"}`, `busy ${P.querySelector("[role=listbox]").getAttribute("aria-busy")}`,
        `items ${[...P.querySelectorAll("[role=option]")].filter((i) => i.getClientRects().length).map((i) => i.textContent.trim()).join(",") || "none"}`].join(" | ");
      try {
        log("start", view());
        input.value = "as"; input.dispatchEvent(new Event("input", { bubbles: true }));
        api.set("searching", true); await wait(10);
        log("fetching", view(), loading.getAttribute("role"));
        api.set("people", ["Asha", "Asif"]); api.set("searching", false); await wait(20);
        log("arrived", view());
        input.value = "zz"; input.dispatchEvent(new Event("input", { bubbles: true }));
        api.set("searching", true); api.set("people", []); await wait(10);
        log("again", view());
        api.set("searching", false); await wait(10);
        log("none", view());
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("command-loading", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "start loading hidden | empty shown | busy false | items none",
            "fetching loading Finding people… | empty hidden | busy true | items none status",
            "arrived loading hidden | empty hidden | busy false | items Asha,Asif",
            "again loading Finding people… | empty hidden | busy true | items none",
            "none loading hidden | empty shown | busy false | items none",
        ],
        "\n{result}"
    );
}

/// A long press by touch opens a context menu (iOS sends no `contextmenu`), moving first
/// doesn't, and the press's own `contextmenu` and click don't act again; menu checkbox and
/// radio items with `keepOpen` leave the menu open.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn long_press_and_keep_open_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "row": null, "grid": false, "size": "m", "clicks": 0 },
        "root": { "type": "stack", "children": [
            { "type": "ui-context-menu", "attrs": { "id": "ctx" }, "props": { "bind": "row" }, "children": [
                { "type": "element", "attrs": { "id": "area", "data-context": "\"area\"" }, "style": { "height": "120px" },
                  "on": { "click": { "action": "set_state", "key": "clicks", "value": "{{ clicks + 1 }}" } },
                  "children": [ { "type": "text", "props": { "value": "Press here" } } ] },
                { "type": "ui-menu-item", "props": { "label": "Copy" } } ] },
            { "type": "ui-dropdown-menu", "props": { "id": "view", "trigger": "View" }, "children": [
                { "type": "ui-menu-checkbox", "props": { "label": "Grid", "bind": "grid", "keepOpen": true } },
                { "type": "ui-menu-radio-group", "props": { "bind": "size", "options": ["s", "m", "l"], "keepOpen": true } },
                { "type": "ui-menu-checkbox", "props": { "label": "Closes", "bind": "grid" } } ] } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const area = document.getElementById("area"), menu = document.querySelector("#ctx [role=menu]");
      const touch = (type, x, y) => area.dispatchEvent(new PointerEvent(type, { bubbles: true, pointerId: 7, pointerType: "touch", isPrimary: true, clientX: x, clientY: y }));
      const r = area.getBoundingClientRect(), x = r.left + 30, y = r.top + 30;
      try {
        // Moving first: a scroll, no menu.
        touch("pointerdown", x, y); touch("pointermove", x, y + 30); await wait(600); touch("pointerup", x, y + 30);
        log("moved", menu.matches(":popover-open"));
        // Held still: open at the finger, with what was pressed.
        touch("pointerdown", x, y); await wait(600);
        log("held", menu.matches(":popover-open"), Math.round(menu.getBoundingClientRect().left - x), Math.round(menu.getBoundingClientRect().top - y), JSON.stringify(S().row));
        touch("pointerup", x, y);
        // Android's own contextmenu for the same press, and the tap's click: nothing more.
        area.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: x, clientY: y }));
        area.click(); await wait(0);
        log("after", menu.matches(":popover-open"), S().clicks);
        menu.hidePopover(); await wait(800);
        area.click(); await wait(0);
        log("later click", S().clicks);
        // keepOpen: the menu stays as items are chosen; an item without it closes it.
        const view = document.getElementById("view"), item = (l) => [...view.querySelectorAll("[role^=menuitem]")].find((i) => i.textContent.includes(l));
        document.querySelector('[popovertarget="view"]').click(); await wait(20);
        item("Grid").click(); await wait(20);
        log("checkbox", view.matches(":popover-open"), S().grid);
        item("l").click(); await wait(20);
        log("radio", view.matches(":popover-open"), S().size);
        item("Closes").click(); await wait(20);
        log("closes", view.matches(":popover-open"), S().grid);
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("long-press", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "moved false",
            // At the finger, with the pressed element's context.
            "held true 0 0 \"area\"",
            // The same press's `contextmenu` and click do nothing more.
            "after true 0",
            "later click 1",
            "checkbox true true",
            "radio true l",
            "closes false false",
        ],
        "\n{result}"
    );
}

/// A tree's renaming in place (F2, a double click on the name; Enter keeps, Escape
/// doesn't), icons per node, and children loaded from `childrenUrl` when a node with
/// `children: true` first opens (once; a failure closes it and opening tries again).
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn tree_rename_icons_and_lazy_children_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "files": [
            { "id": "docs", "label": "Docs", "icon": "📁", "children": [ { "id": "a", "label": "a.txt", "icon": "📄" } ] },
            { "id": "src", "label": "src", "icon": "📁", "children": true },
            { "id": "bad", "name": "broken", "children": true } ] },
        "root": { "type": "ui-tree", "props": { "nodes": "{{ files }}", "bindNodes": "files", "renamable": true, "childrenUrl": "/api/children", "label": "Files" } } });
    let driver = r##"<script>
    const asked = [];
    let fail = true;
    window.fetch = async (u) => {
      asked.push(String(u));
      const parent = new URL(u, location.href).searchParams.get("parent");
      await new Promise((r) => setTimeout(r, 20));
      if (parent === "bad" && fail) return new Response("no", { status: 500 });
      const body = parent === "src" ? [{ id: "main", label: "main.rs", icon: "🦀" }, { id: "lib", label: "lib", children: true }] : { nodes: [{ id: parent + "-1", label: "one" }] };
      return new Response(JSON.stringify(body), { headers: { "content-type": "application/json" } });
    };
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const T = document.querySelector("[data-i-x=ui-tree]");
      const $ = (s) => T.querySelector(s), $$ = (s) => [...T.querySelectorAll(s)];
      const li = (id) => $(`[data-tree-id="${id}"]`);
      const shape = () => $$("[role=treeitem]").filter((x) => !x.closest("[hidden]")).map((x) =>
        "  ".repeat(x.getAttribute("aria-level") - 1) + (x.querySelector(":scope > [data-tree-row] > [data-tree-icon]")?.textContent ?? "") + (x.querySelector(":scope > [data-tree-row] > [data-tree-label]")?.textContent ?? "[box]")
        + (x.hasAttribute("aria-expanded") ? (x.getAttribute("aria-expanded") === "true" ? "-" : "+") : "") + (x.getAttribute("aria-busy") ? "…" : "")).join(" ");
      const key = async (target, k) => { target.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true })); await wait(0); };
      const said = async () => { await wait(60); return $("[data-tree-live]").textContent; };
      let renamed = "";
      T.addEventListener("rename", (e) => (renamed = `${e.detail.id}=${e.detail.label}`));
      try {
        log("server", shape());
        await wait(0); // the script has drawn it again
        log("drawn", shape());
        // Opening a lazy node: busy, then its children, once.
        li("src").querySelector("[data-tree-twisty]").click(); await wait(0);
        log("opening", shape());
        await wait(60);
        log("opened", shape(), "|", await said());
        await key(li("src"), "ArrowLeft"); await key(li("src"), "ArrowRight"); await wait(60);
        log("asked", asked.length);
        // A failure closes it; opening again tries again.
        await key(li("bad"), "ArrowRight"); await wait(60);
        log("failed", shape().split(" ").slice(-1)[0], "|", await said());
        fail = false; await key(li("bad"), "ArrowRight"); await wait(60);
        log("retried", li("bad").getAttribute("aria-expanded"), li("bad-1")?.textContent, asked.length);
        // F2 renames; Enter keeps it, in state with the loaded children.
        li("main").focus(); await key(li("main"), "F2");
        const box = $("[data-tree-rename]");
        log("box", box.value, box.getAttribute("aria-label"), document.activeElement === box);
        box.value = "main2.rs"; await key(box, "Enter");
        log("renamed", renamed, S().files[1].children[0].label, document.activeElement === li("main"), "|", await said());
        // A double click on the name; Escape leaves it.
        li("a").querySelector("[data-tree-label]").dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
        $("[data-tree-rename]").value = "nope"; await key($("[data-tree-rename]"), "Escape");
        log("escaped", li("a").textContent, renamed);
        // `name` nodes keep `name`.
        li("bad").focus(); await key(li("bad"), "F2"); $("[data-tree-rename]").value = "fixed"; await key($("[data-tree-rename]"), "Enter");
        log("name", JSON.stringify(S().files[2].name), S().files[2].label);
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("tree-lazy", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "server 📁Docs+ 📁src+ broken+",
            "drawn 📁Docs+ 📁src+ broken+",
            // Busy while its children load, then they're there.
            "opening 📁Docs+ 📁src-… broken+",
            "opened 📁Docs+ 📁src-   🦀main.rs   lib+ broken+ | src: 2 items",
            // Closed and opened again: not asked again.
            "asked 1",
            "failed broken+ | Couldn't load what's in broken",
            "retried true one 3",
            "box main.rs Rename main.rs true",
            // Written into the loaded children, in state.
            "renamed main=main2.rs main2.rs true | Renamed to main2.rs",
            "escaped 📄a.txt main=main2.rs",
            "name \"fixed\" ",
        ],
        "\n{result}"
    );
}

/// Neither a palette's choices nor a hidden hover card makes a narrow page scroll
/// sideways: the choices wrap, and the card is kept on screen, above a trigger that's low.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn theme_options_and_hover_cards_fit_in_chrome() {
    let card = |id: &str| serde_json::json!({ "type": "ui-hover-card", "attrs": { "id": id }, "props": { "trigger": "Trigger" },
        "children": [{ "type": "text", "props": { "value": "Card" } }] });
    let doc = serde_json::json!({ "version": 1, "root": { "type": "element", "style": { "position": "relative", "height": "100vh" }, "children": [
        { "type": "element", "attrs": { "id": "narrow" }, "style": { "width": "350px" }, "children": [
            { "type": "ui-theme-option", "props": { "option": "palette", "swatch": "color.primary" } } ] },
        { "type": "element", "style": { "display": "flex", "justify-content": "space-between" }, "children": [card("start"), card("end")] },
        { "type": "element", "style": { "position": "absolute", "bottom": "0", "left": "0" }, "children": [card("low")] } ] } });
    let driver = r##"<script>
    window.addEventListener("load", () => {
      const out = [];
      const box = (id) => document.getElementById(id).querySelector("[data-ui-card]").getBoundingClientRect();
      const trigger = (id) => document.getElementById(id).firstElementChild.getBoundingClientRect();
      const group = document.querySelector("#narrow > [role=group]");
      const rows = new Set([...group.children].map((b) => b.offsetTop));
      out.push(`choices fit=${group.scrollWidth <= 350 && group.offsetWidth <= 350} wrap=${rows.size > 1}`);
      const inside = (id) => { const r = box(id); return r.left >= 0 && r.right <= innerWidth; };
      out.push(`start below=${box("start").top >= trigger("start").bottom} aligned=${Math.abs(box("start").left - trigger("start").left) < 1}`);
      out.push(`end inside=${inside("end")} below=${box("end").top >= trigger("end").bottom}`);
      out.push(`low inside=${inside("low")} above=${box("low").bottom <= trigger("low").top}`);
      out.push(`sideways ${document.documentElement.scrollWidth - document.documentElement.clientWidth}`);
      out.push(`hidden ${getComputedStyle(document.querySelector("#start [data-ui-card]")).visibility}`);
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
    let Some(result) = drive("hover-card-fit", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "choices fit=true wrap=true",
            "start below=true aligned=true",
            "end inside=true below=true",
            "low inside=true above=true",
            "sideways 0",
            "hidden hidden",
        ],
        "\n{result}"
    );
}

/// In a narrow place a tab list scrolls within itself, a toggle group wraps and a code's
/// boxes get narrower; where there's room, each is as wide as it wants to be, and a
/// toggle group in a row isn't squeezed by what's beside it.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn tabs_toggles_and_codes_fit_in_chrome() {
    let labels = ["Overview", "Past performance", "Key personnel", "Focus documents", "Settings"];
    let tabs = serde_json::json!({ "type": "ui-tabs", "props": { "bind": "tab" }, "children":
        labels.map(|l| serde_json::json!({ "type": "ui-tab-panel", "props": { "value": l, "label": l } })) });
    let group = serde_json::json!({ "type": "ui-toggle-group", "props": { "bind": "view", "variant": "outline", "options": labels } });
    let code = serde_json::json!({ "type": "ui-input-otp", "props": { "bind": "code", "length": 8, "groups": [4, 4], "label": "Code" } });
    let place = |id: &str, width: &str| serde_json::json!({ "type": "element", "attrs": { "id": id }, "style": { "width": width },
        "children": [tabs.clone(), group.clone(), code.clone()] });
    let doc = serde_json::json!({ "version": 1, "state": { "tab": "Overview", "view": null, "code": "" }, "root": { "type": "element", "children": [
        place("narrow", "290px"), place("wide", "900px"),
        { "type": "element", "attrs": { "id": "row" }, "style": { "display": "flex", "width": "500px" }, "children": [
            { "type": "ui-toggle-group", "props": { "bind": "view", "options": ["One", "Two", "Three"] } },
            { "type": "ui-input", "props": { "bind": "code" }, "style": { "width": "100%" } } ] } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = [];
      for (const id of ["narrow", "wide"]) {
        const at = document.getElementById(id), width = at.clientWidth;
        const list = at.querySelector("[role=tablist]"), group = at.querySelector("[role=group]"), otp = at.querySelector("[data-otp-input]").parentElement;
        const rows = new Set([...group.children].map((b) => b.offsetTop)).size;
        const slots = [...otp.querySelectorAll("[data-otp-slot]")].map((s) => s.getBoundingClientRect());
        const even = slots.every((s) => Math.abs(s.width - slots[0].width) < 0.6);
        out.push(`${id} tabs fit=${list.offsetWidth <= width} scrolls=${list.scrollWidth > list.clientWidth} tall=${list.scrollHeight > list.clientHeight}`);
        out.push(`${id} group fit=${group.offsetWidth <= width} wraps=${rows > 1}`);
        out.push(`${id} code fit=${slots.at(-1).right <= at.getBoundingClientRect().right + 0.6} even=${even} full=${Math.round(slots[0].width) === 42}`);
        // Going to the last tab brings it into view.
        const last = list.lastElementChild;
        last.focus();
        await new Promise((r) => setTimeout(r, 100));
        out.push(`${id} last ${last.getBoundingClientRect().right <= list.getBoundingClientRect().right + 0.6}`);
      }
      const row = document.querySelector("#row > [role=group]");
      out.push(`row wraps=${new Set([...row.children].map((b) => b.offsetTop)).size > 1}`);
      out.push(`sideways ${document.querySelector("#narrow").scrollWidth - document.querySelector("#narrow").clientWidth}`);
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
    let Some(result) = drive("fits", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "narrow tabs fit=true scrolls=true tall=false",
            "narrow group fit=true wraps=true",
            "narrow code fit=true even=true full=false",
            "narrow last true",
            "wide tabs fit=true scrolls=false tall=false",
            "wide group fit=true wraps=false",
            "wide code fit=true even=true full=true",
            "wide last true",
            "row wraps=false",
            "sideways 0",
        ],
        "\n{result}"
    );
}

/// A tab list is one stop for the Tab key, and the arrow keys, Home and End move between
/// its tabs and select them (mirrored right to left). A list with more tabs than fit
/// starts at its selected tab, follows it, and marks the sides with more.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn tabs_by_keyboard_in_chrome() {
    let panels = ["January", "February", "March", "April", "May", "June"].map(|l| {
        serde_json::json!({ "type": "ui-tab-panel", "props": { "value": l, "label": l },
            "children": [{ "type": "text", "props": { "value": format!("{l} panel") } }] })
    });
    let tabs = |bind: &str, id: &str| {
        let dir = if id == "rtl" { "rtl" } else { "ltr" };
        serde_json::json!({ "type": "element", "attrs": { "id": id, "dir": dir }, "style": { "width": "300px" },
            "children": [{ "type": "ui-tabs", "props": { "bind": bind }, "children": panels }] })
    };
    let doc = serde_json::json!({ "version": 1, "state": { "a": "April", "b": "January", "c": "none" },
        "root": { "type": "element", "children": [tabs("a", "ltr"), tabs("b", "rtl"), tabs("c", "none")] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = [];
      const frame = () => new Promise((r) => setTimeout(r, 100));
      try {
        const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
        const list = (id) => document.querySelector(`#${id} [role=tablist]`);
        const tab = (id, name) => [...list(id).children].find((t) => t.textContent === name);
        const key = (k, mods = {}) => document.activeElement.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true, ...mods }));
        const stops = (id) => [...list(id).children].filter((t) => t.tabIndex === 0).map((t) => t.textContent).join();
        const inView = (id, name) => { const t = tab(id, name).getBoundingClientRect(), b = list(id).getBoundingClientRect(); return t.left >= b.left - 1 && t.right <= b.right + 1; };
        const more = (id) => list(id).getAttribute("data-more");
        const shown = (id) => [...document.querySelectorAll(`#${id} [role=tabpanel]:not([hidden])`)].map((p) => p.textContent).join();
        await frame();
        out.push(`start stop=${stops("ltr")} view=${inView("ltr", "April")} more=${more("ltr")} page=${scrollY}`);
        tab("ltr", "April").focus();
        for (const k of ["ArrowRight", "End", "ArrowRight", "ArrowLeft", "Home"]) {
          key(k); await frame();
          const now = document.activeElement.textContent;
          out.push(`${k} ${now} ${S().a} stop=${stops("ltr")} view=${inView("ltr", now)} more=${more("ltr")} ${shown("ltr")}`);
        }
        key("ArrowRight", { metaKey: true }); await frame();
        out.push(`with ⌘ ${S().a}`);
        // State changed elsewhere.
        root.__irender.set("a", "June"); await frame();
        out.push(`state stop=${stops("ltr")} view=${inView("ltr", "June")} more=${more("ltr")}`);
        out.push(`rtl start more=${more("rtl")}`);
        tab("rtl", "January").focus();
        for (const k of ["ArrowLeft", "End", "ArrowRight"]) { key(k); await frame(); out.push(`rtl ${k} ${S().b} view=${inView("rtl", S().b)} more=${more("rtl")}`); }
        out.push(`none stop=${stops("none")}`);
      } catch (e) { out.push("EXCEPTION " + e.stack); }
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
    let Some(result) = drive("tabs-keys", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "start stop=April view=true more=both page=0",
            "ArrowRight May May stop=May view=true more=both May panel",
            "End June June stop=June view=true more=left June panel",
            // Past the last, the first again.
            "ArrowRight January January stop=January view=true more=right January panel",
            "ArrowLeft June June stop=June view=true more=left June panel",
            "Home January January stop=January view=true more=right January panel",
            "with ⌘ January",
            "state stop=June view=true more=left",
            "rtl start more=left",
            "rtl ArrowLeft February view=true more=left",
            "rtl End June view=true more=right",
            "rtl ArrowRight May view=true more=right",
            "none stop=January",
        ],
        "\n{result}"
    );
}

/// `ui-iframe`: its size (a height, or a ratio of its width), its `src` following state,
/// an unsafe address dropped, and no reload when other state changes.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn iframe_in_chrome() {
    let doc = serde_json::json!({ "version": 1, "state": { "url": "about:blank#one", "other": 0 },
        "root": { "type": "element", "style": { "width": 800 }, "children": [
            { "type": "ui-iframe", "attrs": { "id": "a" }, "props": { "src": "{{ url }}" } },
            { "type": "ui-iframe", "attrs": { "id": "b" }, "props": { "src": "about:blank", "ratio": "16 / 9", "bordered": false } } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const api = document.querySelector("[data-i-root]").__irender;
      const a = document.getElementById("a"), b = document.getElementById("b");
      const box = (el) => { const r = el.getBoundingClientRect(); return `${Math.round(r.width)}x${Math.round(r.height)}`; };
      try {
        log("sizes", box(a), box(b));
        log("sandbox", a.sandbox.contains("allow-scripts"), a.sandbox.contains("allow-top-navigation"), a.loading, a.allow);
        // A page without a fragment (changing only that never reloads), then other state
        // changing. Setting the same `src` again would load it again each time.
        api.set("url", "about:blank"); await Promise.race([new Promise((r) => a.addEventListener("load", r, { once: true })), wait(2000)]); await wait(50);
        let loads = 0; a.addEventListener("load", () => loads++);
        for (let i = 1; i <= 5; i++) { api.set("other", i); await wait(50); }
        log("other state", loads, "loads");
        api.set("url", "about:blank#two"); await wait(20);
        log("src", a.getAttribute("src"));
        api.set("url", "javascript:alert(1)"); await wait(20);
        log("unsafe", a.hasAttribute("src"));
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("iframe", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        ["sizes 800x400 800x450", "sandbox true false lazy fullscreen", "other state 0 loads", "src about:blank#two", "unsafe false"],
        "\n{result}"
    );
}

/// Records joined by id: a remote table asks for the people on its page in one call (each
/// id once), its cells pulse until they arrive, and the next page asks only for new ids;
/// accounts come one at a time; a query fills state with rows from an API, joined, and
/// another with records by id, by POST.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn data_sources_join_records_in_chrome() {
    use std::io::Write;
    use std::sync::{Arc, Mutex};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let log = Arc::new(Mutex::new(Vec::<String>::new()));
    let seen = log.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let seen = seen.clone();
            std::thread::spawn(move || {
                let request = read_request(&mut stream);
                let method = request.split_whitespace().next().unwrap_or("").to_string();
                let target = request.split_whitespace().nth(1).unwrap_or("").replace("%2C", ",");
                let (path, query) = target.split_once('?').unwrap_or((&target, ""));
                let arg = |k: &str| query.split('&').filter_map(|kv| kv.split_once('=')).find(|(a, _)| *a == k).map(|(_, v)| v.to_string());
                let person = |id: &str| serde_json::json!({ "id": id, "name": format!("Person {id}") });
                let body_json = request.split("\r\n\r\n").nth(1).and_then(|b| serde_json::from_str::<serde_json::Value>(b).ok());
                if method != "OPTIONS" {
                    seen.lock().unwrap().push(format!("{method} {}", target.trim_start_matches('/')));
                }
                let (status, body) = match (method.as_str(), path) {
                    ("OPTIONS", _) => ("204 No Content", String::new()),
                    (_, "/rows") => {
                        let offset: usize = arg("offset").and_then(|v| v.parse().ok()).unwrap_or(0);
                        let limit: usize = arg("limit").and_then(|v| v.parse().ok()).unwrap_or(1000);
                        // Owners repeat, and p9 isn't anyone.
                        let rows: Vec<_> = (0..12usize).skip(offset).take(limit).map(|i| serde_json::json!({ "id": i, "ownerId": if i == 5 { "p9".to_string() } else { format!("p{}", i % 4 + (i / 6) * 4) }, "accountId": format!("a{}", i % 2) })).collect();
                        ("200 OK", serde_json::json!({ "rows": rows, "total": 12 }).to_string())
                    }
                    (_, "/people") | (_, "/folks") => {
                        std::thread::sleep(std::time::Duration::from_millis(300));
                        let ids = arg("ids").unwrap_or_default();
                        let list: Vec<_> = ids.split(',').filter(|i| *i != "p9").map(person).collect();
                        ("200 OK", serde_json::Value::Array(list).to_string())
                    }
                    ("POST", "/people/batch") => {
                        let ids = body_json.as_ref().and_then(|b| b["ids"].as_array().cloned()).unwrap_or_default();
                        let map: serde_json::Map<_, _> = ids.iter().filter_map(|i| i.as_str()).map(|i| (i.to_string(), person(i))).collect();
                        ("200 OK", serde_json::json!({ "data": map }).to_string())
                    }
                    (_, p) if p.starts_with("/accounts/") => ("200 OK", serde_json::json!({ "id": &p[10..], "title": format!("Account {}", &p[10..]) }).to_string()),
                    _ => ("404 Not Found", "{}".to_string()),
                };
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: content-type\r\nAccess-Control-Allow-Methods: GET, POST\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
            });
        }
    });
    let base = format!("http://127.0.0.1:{port}");
    let doc = serde_json::json!({ "version": 1, "state": { "pick": ["p1", "p2"], "rows": null, "rowsStatus": null, "picked": null, "one": null,
        "local": [{ "id": 1, "who": "f1" }, { "id": 2, "who": "f2" }, { "id": 3, "who": "f3" }, { "id": 4, "who": "f4" }, { "id": 5, "who": "f5" }, { "id": 6, "who": "f6" }] },
        "root": { "type": "stack", "children": [
            { "type": "ui-data-source", "props": { "name": "people", "batch": { "url": format!("{base}/people"), "max": 3 } } },
            { "type": "ui-data-source", "props": { "name": "accounts", "one": format!("{base}/accounts/{{id}}") } },
            { "type": "ui-data-source", "props": { "name": "staff", "batch": { "url": format!("{base}/people/batch"), "method": "POST", "pick": "data" } } },
            { "type": "ui-data-table", "props": { "id": "t", "url": format!("{base}/rows"), "pageSize": 6, "pageSizes": [],
                "expand": { "owner": { "source": "people", "from": "ownerId" }, "account": "accounts" },
                "columns": [{ "field": "id" }, { "field": "owner.name", "label": "Owner" }, { "field": "account.title", "label": "Account" }] } },
            { "type": "ui-data-query", "props": { "url": format!("{base}/rows"), "params": { "limit": 3 }, "into": "rows", "status": "rowsStatus",
                "expand": { "owner": { "source": "people", "from": "ownerId" } } } },
            { "type": "ui-data-query", "props": { "source": "staff", "ids": "{{ pick }}", "into": "picked" } },
            { "type": "ui-data-query", "props": { "source": "people", "ids": "p0", "into": "one" } },
            { "type": "ui-data-source", "props": { "name": "folks", "batch": { "url": format!("{base}/folks") } } },
            { "type": "ui-data-table", "props": { "id": "local", "rows": "{{ local }}", "pageSize": 3, "pageSizes": [], "export": "local.csv",
                "expand": { "person": { "source": "folks", "from": "who" } },
                "columns": [{ "field": "id" }, { "field": "person.name", "label": "Person" }] } } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const pre = document.createElement("pre"); pre.id = "progress"; document.body.append(pre); const log = (...a) => { out.push(a.join(" ")); pre.textContent = out.join("\n"); };
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      // Timers, not message ticks: WebKit runs ticks ahead of the timers the sources wait on.
      const until = async (f) => { for (let i = 0; i < 3000 && !f(); i++) await wait(10); };
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const t = document.getElementById("t");
      const rows = () => [...t.querySelectorAll("[data-dt-body] > [data-dt-row]")];
      const cells = (n) => rows().map((r) => r.children[n].textContent.trim() || "_").join(",");
      const waiting = () => t.querySelectorAll("[data-dt-wait]").length;
      try {
        await until(() => rows().length === 6);
        log("rows first", rows().length, "pulsing", waiting() > 0);
        await until(() => !waiting());
        log("owners", cells(1));
        log("accounts", cells(2));
        t.querySelector("[data-dt-go=next]").click();
        await until(() => rows()[0]?.children[0].textContent.trim() === "6");
        await until(() => !waiting());
        log("page 2", cells(1));
        await until(() => S().rowsStatus?.state === "ready");
        log("query", S().rows.map((r) => r.owner?.name).join(","), S().rowsStatus.total, S().rowsStatus.pending);
        await until(() => S().picked?.length === 2);
        log("posted", S().picked.map((p) => p.name).join(","));
        root.__irender.set("pick", ["p2", "p3"]);
        await until(() => S().picked?.[1]?.name === "Person p3");
        log("posted again", S().picked.map((p) => p.name).join(","));
        log("one", S().one?.name);

        // A table of rows in state: only its page's ids, and an export with what's here.
        const lt = document.getElementById("local");
        const lrows = () => [...lt.querySelectorAll("[data-dt-body] > [data-dt-row]")].map((r) => r.children[1].textContent.trim() || "_").join(",");
        await until(() => !lt.querySelector("[data-dt-wait]") && lrows().startsWith("Person"));
        log("local", lrows());
        let csv = null; lt.addEventListener("export", (e) => (csv = e.detail.csv));
        lt.querySelector("[data-dt-export]").click();
        await wait(50);
        log("export", csv.trim().split(/\r?\n/).slice(1).map((l) => l.split(",")[1] || "_").join(","));
        lt.querySelector("[data-dt-go=next]").click();
        await until(() => !lt.querySelector("[data-dt-wait]") && lrows().startsWith("Person f4"));
        log("local 2", lrows());
      } catch (e) { log("EXCEPTION", e.stack); }
      pre.textContent = out.join("\n"); pre.id = "result";
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("data-sources", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "rows first 6 pulsing true",
            // p9 isn't anyone: blank, not pulsing.
            "owners Person p0,Person p1,Person p2,Person p3,Person p0,_",
            "accounts Account a0,Account a1,Account a0,Account a1,Account a0,Account a1",
            "page 2 Person p6,Person p7,Person p4,Person p5,Person p6,Person p7",
            // The API's total, not the rows it sent.
            "query Person p0,Person p1,Person p2 12 0",
            "posted Person p1,Person p2",
            "posted again Person p2,Person p3",
            "one Person p0",
            "local Person f1,Person f2,Person f3",
            // Only what was fetched: the rows on page 2 haven't been asked for.
            "export Person f1,Person f2,Person f3,_,_,_",
            "local 2 Person f4,Person f5,Person f6",
        ],
        "\n{result}"
    );
    let log = log.lock().unwrap();
    let people: Vec<_> = log.iter().filter(|q| q.starts_with("GET people")).cloned().collect();
    // The first page's five owners in calls of three (and p0 for the query that asks by id
    // with them); then only the second page's new ones. Never one id twice.
    let mut asked: Vec<String> = people.iter().flat_map(|q| q.split_once("ids=").unwrap().1.split(',').map(String::from).collect::<Vec<_>>()).collect();
    let n = asked.len();
    asked.sort();
    asked.dedup();
    assert_eq!(n, asked.len(), "an id was asked for twice: {people:?}");
    assert_eq!(asked, ["p0", "p1", "p2", "p3", "p4", "p5", "p6", "p7", "p9"], "{people:?}");
    assert!(people.iter().all(|q| q.split(',').count() <= 3), "{people:?}");
    assert_eq!(log.iter().filter(|q| q.starts_with("GET accounts/")).count(), 2, "{log:?}");
    assert_eq!(log.iter().filter(|q| q.starts_with("POST people/batch")).count(), 2, "{log:?}");
    let folks: Vec<_> = log.iter().filter(|q| q.starts_with("GET folks")).cloned().collect();
    assert_eq!(folks, ["GET folks?ids=f1,f2,f3", "GET folks?ids=f4,f5,f6"], "{log:?}");
}

/// A source named in `expand` (or a query's `source`) that nothing declares fails once the
/// page has loaded, rather than waiting, and is fetched when it's declared after all; a
/// `submit` with `invalidates` has the records it names fetched again once it succeeds
/// (its ids, or every record of a source), and one that fails doesn't.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn data_sources_undeclared_and_invalidated_in_chrome() {
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let log = Arc::new(Mutex::new(Vec::<String>::new()));
    let seen = log.clone();
    // Each promotion makes everyone's records newer.
    let version = Arc::new(AtomicUsize::new(1));
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let (seen, version) = (seen.clone(), version.clone());
            std::thread::spawn(move || {
                let request = read_request(&mut stream);
                let method = request.split_whitespace().next().unwrap_or("").to_string();
                let target = request.split_whitespace().nth(1).unwrap_or("").replace("%2C", ",");
                let (path, query) = target.split_once('?').unwrap_or((&target, ""));
                if method != "OPTIONS" {
                    seen.lock().unwrap().push(format!("{method} {}", target.trim_start_matches('/')));
                }
                let (status, body) = match (method.as_str(), path) {
                    ("OPTIONS", _) => ("204 No Content", String::new()),
                    ("GET", "/people") | ("GET", "/late") => {
                        let v = version.load(Ordering::SeqCst);
                        let ids = query.strip_prefix("ids=").unwrap_or_default();
                        let list: Vec<_> = ids.split(',').map(|id| serde_json::json!({ "id": id, "name": format!("{id} v{v}") })).collect();
                        ("200 OK", serde_json::Value::Array(list).to_string())
                    }
                    ("POST", "/promote") => {
                        version.fetch_add(1, Ordering::SeqCst);
                        ("200 OK", "{}".to_string())
                    }
                    _ => ("500 Internal Server Error", "{}".to_string()),
                };
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: content-type\r\nAccess-Control-Allow-Methods: GET, POST\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
            });
        }
    });
    let base = format!("http://127.0.0.1:{port}");
    let button = |id: &str, endpoint: String, invalidates: serde_json::Value| {
        serde_json::json!({ "type": "ui-button", "attrs": { "id": id }, "props": { "label": id },
            "on": { "click": { "action": "submit", "endpoint": endpoint, "method": "POST", "body": {}, "invalidates": invalidates } } })
    };
    let doc = serde_json::json!({ "version": 1, "state": { "who": "p1", "late": false, "ps": null, "ghost": null, "gs": null, "l": null, "ls": null,
        "rows": [{ "id": 1, "ownerId": "p1" }, { "id": 2, "ownerId": "p2" }, { "id": 3, "ownerId": "g1" }] },
        "root": { "type": "stack", "children": [
            { "type": "ui-data-source", "props": { "name": "people", "batch": { "url": format!("{base}/people") } } },
            { "type": "ui-data-query", "props": { "source": "people", "ids": ["p1", "p2"], "into": "ps" } },
            { "type": "ui-data-query", "props": { "source": "ghosts", "ids": "g1", "into": "ghost", "status": "gs" } },
            { "type": "ui-data-query", "props": { "source": "late", "ids": "l1", "into": "l", "status": "ls" } },
            { "type": "ui-data-table", "props": { "id": "t", "rows": "{{ rows }}", "expand": { "ghost": { "source": "ghosts", "from": "ownerId" } },
                "columns": [{ "field": "id" }, { "field": "ghost.name", "label": "Ghost" }] } },
            { "type": "if", "props": { "when": "{{ late }}" }, "children": [
                { "type": "ui-data-source", "props": { "name": "late", "batch": { "url": format!("{base}/late") } } } ] },
            button("one", format!("{base}/promote"), serde_json::json!({ "people": "{{ who }}" })),
            button("all", format!("{base}/promote"), serde_json::json!(["people"])),
            button("bad", format!("{base}/broken"), serde_json::json!(["people"])) ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const pre = document.createElement("pre"); pre.id = "progress"; document.body.append(pre); const log = (...a) => { out.push(a.join(" ")); pre.textContent = out.join("\n"); };
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const until = async (f) => { for (let i = 0; i < 3000 && !f(); i++) await wait(10); };
      const warned = []; const warn = console.warn; console.warn = (...a) => (warned.push(a.join(" ")), warn(...a));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const names = () => (S().ps ?? []).map((p) => p.name).join(",");
      try {
        await until(() => S().gs?.state === "error" && S().ls?.state === "error");
        log("undeclared", S().gs.state, JSON.stringify(S().ghost), S().ls.state);
        await until(() => document.querySelector("#t [data-dt-body] > [data-dt-row]:nth-child(3) [data-dt-wait]") === null);
        log("pulsing", document.querySelectorAll("#t [data-dt-wait]").length);
        await until(() => warned.length >= 2);
        log("warned", warned.filter((w) => w.includes('"ghosts"')).length, warned.filter((w) => w.includes('"late"')).length);
        root.__irender.set("late", true);
        await until(() => S().l?.name === "l1 v1");
        log("late", S().l.name, S().ls.state);

        await until(() => names() === "p1 v1,p2 v1");
        log("first", names());
        document.getElementById("one").click();
        await until(() => names().startsWith("p1 v2"));
        await wait(100);
        log("one", names());
        document.getElementById("all").click();
        await until(() => names() === "p1 v3,p2 v3");
        log("all", names());
        document.getElementById("bad").click();
        await wait(300);
        log("bad", names());
      } catch (e) { log("EXCEPTION", e.stack); }
      pre.textContent = out.join("\n"); pre.id = "result";
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("data-undeclared", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "undeclared error null error",
            "pulsing 0",
            "warned 1 1",
            "late l1 v1 ready",
            "first p1 v1,p2 v1",
            // Only p1: p2 keeps what it had.
            "one p1 v2,p2 v1",
            "all p1 v3,p2 v3",
            // Failed: nothing's fetched again.
            "bad p1 v3,p2 v3",
        ],
        "\n{result}"
    );
    let log = log.lock().unwrap();
    let people: Vec<_> = log.iter().filter(|q| q.starts_with("GET people")).cloned().collect();
    assert_eq!(people, ["GET people?ids=p1,p2", "GET people?ids=p1", "GET people?ids=p1,p2"], "{log:?}");
}

/// A joined cell whose record couldn't be fetched has a button that asks for it again;
/// an id with no record is asked for again after `missingTtl`, a record after `ttl`.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn data_sources_retry_failed_and_forget_missing_in_chrome() {
    use std::io::Write;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let log = Arc::new(Mutex::new(Vec::<String>::new()));
    let seen = log.clone();
    // The first call with p3 in it fails.
    let failed = Arc::new(AtomicBool::new(false));
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let (seen, failed) = (seen.clone(), failed.clone());
            std::thread::spawn(move || {
                let request = read_request(&mut stream);
                let target = request.split_whitespace().nth(1).unwrap_or("").replace("%2C", ",");
                let ids = target.split_once("ids=").map(|(_, i)| i.to_string()).unwrap_or_default();
                seen.lock().unwrap().push(ids.clone());
                let (status, body) = if ids.split(',').any(|i| i == "p3") && !failed.swap(true, Ordering::SeqCst) {
                    ("500 Internal Server Error", "{}".to_string())
                } else {
                    let list: Vec<_> = ids.split(',').filter(|i| *i != "p9").map(|id| serde_json::json!({ "id": id, "name": format!("Person {id}") })).collect();
                    ("200 OK", serde_json::Value::Array(list).to_string())
                };
                let _ = write!(stream, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            });
        }
    });
    let base = format!("http://127.0.0.1:{port}");
    let doc = serde_json::json!({ "version": 1, "state": { "ids": ["p1"], "got": null,
        "rows": [{ "id": 1, "ownerId": "p1" }, { "id": 2, "ownerId": "p3" }, { "id": 3, "ownerId": "p9" }] },
        "root": { "type": "stack", "children": [
            { "type": "ui-data-source", "props": { "name": "people", "batch": { "url": format!("{base}/people") }, "ttl": 600, "missingTtl": 5 } },
            { "type": "ui-data-table", "props": { "id": "t", "rows": "{{ rows }}", "expand": { "owner": { "source": "people", "from": "ownerId" } },
                "columns": [{ "field": "id" }, { "field": "owner.name", "label": "Owner" }] } },
            { "type": "ui-data-query", "props": { "source": "people", "ids": "{{ ids }}", "into": "got" } } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const pre = document.createElement("pre"); pre.id = "progress"; document.body.append(pre); const log = (...a) => { out.push(a.join(" ")); pre.textContent = out.join("\n"); };
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const until = async (f) => { for (let i = 0; i < 3000 && !f(); i++) await wait(10); };
      const root = document.querySelector("[data-i-root]");
      const t = document.getElementById("t");
      const rows = () => [...t.querySelectorAll("[data-dt-body] > [data-dt-row]")];
      const cells = () => rows().map((r) => r.children[1].textContent.trim() || "_").join(",");
      const retries = () => rows().map((r) => (r.querySelector("[data-dt-retry]")?.hidden === false ? "retry" : "-")).join(",");
      try {
        await until(() => !t.querySelector("[data-dt-wait=true]") && retries().includes("retry"));
        log("failed", cells(), retries());
        const b = rows()[1].querySelector("[data-dt-retry]");
        log("button", b.getAttribute("aria-label"), b.textContent.trim() === "");
        b.click();
        await until(() => cells().includes("Person p3"));
        log("retried", cells(), retries());
        // Past missingTtl, not ttl: asking again fetches p9 only.
        await wait(5500);
        root.__irender.set("ids", ["p1", "p3", "p9"]);
        await wait(800);
        log("again", JSON.stringify(root.__irender.state.got.map((p) => p.name)));
      } catch (e) { log("EXCEPTION", e.stack); }
      pre.textContent = out.join("\n"); pre.id = "result";
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("data-retry", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            // p1 and p9 came in the call that failed too: p9 is no one, so no button.
            "failed _,_,_ retry,retry,retry",
            "button Couldn't load. Try again true",
            "retried Person p1,Person p3,_ -,-,-",
            "again [\"Person p1\",\"Person p3\"]",
        ],
        "\n{result}"
    );
    let log = log.lock().unwrap();
    // The retry asks for every record that failed with it.
    assert_eq!(*log, ["p1,p3,p9", "p1,p3,p9", "p9"], "{log:?}");
}

/// A query asks for its address again with the last answer's ETag: a 304 keeps its rows,
/// and a new answer replaces them (and its ETag).
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn data_queries_ask_with_their_etag_in_chrome() {
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let log = Arc::new(Mutex::new(Vec::<String>::new()));
    let seen = log.clone();
    let version = Arc::new(AtomicUsize::new(1));
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let (seen, version) = (seen.clone(), version.clone());
            std::thread::spawn(move || {
                let request = read_request(&mut stream);
                let method = request.split_whitespace().next().unwrap_or("").to_string();
                let path = request.split_whitespace().nth(1).unwrap_or("").to_string();
                let asked = request.lines().find_map(|l| l.to_ascii_lowercase().strip_prefix("if-none-match: ").map(String::from));
                let v = version.load(Ordering::SeqCst);
                let etag = format!("\"v{v}\"");
                let cors = "Access-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: if-none-match\r\nAccess-Control-Expose-Headers: ETag\r\n";
                if method == "OPTIONS" {
                    let _ = write!(stream, "HTTP/1.1 204 No Content\r\n{cors}Content-Length: 0\r\nConnection: close\r\n\r\n");
                    return;
                }
                if path == "/bump" {
                    version.fetch_add(1, Ordering::SeqCst);
                    let _ = write!(stream, "HTTP/1.1 200 OK\r\n{cors}Content-Length: 2\r\nConnection: close\r\n\r\n{{}}");
                    return;
                }
                seen.lock().unwrap().push(asked.clone().unwrap_or_else(|| "-".into()));
                if asked.as_deref() == Some(etag.as_str()) {
                    let _ = write!(stream, "HTTP/1.1 304 Not Modified\r\n{cors}ETag: {etag}\r\nConnection: close\r\n\r\n");
                    return;
                }
                let body = serde_json::json!([{ "id": 1, "name": format!("row v{v}") }]).to_string();
                let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n{cors}ETag: {etag}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            });
        }
    });
    let base = format!("http://127.0.0.1:{port}");
    let doc = serde_json::json!({ "version": 1, "state": { "rows": null, "st": null, "again": 0 },
        "root": { "type": "stack", "children": [
            { "type": "ui-data-query", "props": { "url": format!("{base}/rows"), "into": "rows", "status": "st", "refresh": "{{ again }}" } } ] } });
    let driver = format!(r##"<script>
    let loads = "";
    document.addEventListener("loaded", (e) => (loads += e.detail.same ? "s" : "n"));
    window.addEventListener("load", async () => {{
      const out = []; const pre = document.createElement("pre"); pre.id = "progress"; document.body.append(pre); const log = (...a) => {{ out.push(a.join(" ")); pre.textContent = out.join("\n"); }};
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const until = async (f) => {{ for (let i = 0; i < 3000 && !f(); i++) await wait(10); }};
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state, set = (k, v) => root.__irender.set(k, v);
      try {{
        await until(() => loads.length === 1);
        const first = S().rows;
        log("first", first[0].name, S().st.state);
        set("again", 1);
        await until(() => loads.length === 2);
        log("same", S().rows[0].name, S().rows === first, S().st.state, loads);
        await fetch("{base}/bump", {{ method: "POST" }});
        set("again", 2);
        await until(() => loads.length === 3);
        log("changed", S().rows[0].name, loads);
        set("again", 3);
        await until(() => loads.length === 4);
        log("same again", S().rows[0].name, loads);
      }} catch (e) {{ log("EXCEPTION", e.stack); }}
      pre.textContent = out.join("\n"); pre.id = "result";
    }});
    </script>"##);
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("data-etag", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        ["first row v1 ready", "same row v1 true ready ns", "changed row v2 nsn", "same again row v2 nsns"],
        "\n{result}"
    );
    assert_eq!(*log.lock().unwrap(), ["-", "\"v1\"", "\"v1\"", "\"v2\""]);
}

/// What's kept on the device is let go when it was kept for another `version` or longer
/// ago than `maxAge`, a source keeps at most `maxRecords`, and a query at most `keep`
/// answers for its `url`, the oldest going first.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn data_kept_on_the_device_is_versioned_and_bounded_in_chrome() {
    use std::io::Write;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let log = Arc::new(Mutex::new(Vec::<String>::new()));
    let page = Arc::new(Mutex::new(String::new()));
    let done = Arc::new(AtomicBool::new(false));
    let (seen, html, fin) = (log.clone(), page.clone(), done.clone());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let (seen, html, fin) = (seen.clone(), html.clone(), fin.clone());
            std::thread::spawn(move || {
                let request = read_request(&mut stream);
                let target = request.split_whitespace().nth(1).unwrap_or("").replace("%2C", ",");
                let (path, query) = target.split_once('?').unwrap_or((&target, ""));
                let (ty, body) = match path {
                    "/" => ("text/html", html.lock().unwrap().clone()),
                    // Holds the page's load until the driver's done: Chrome dumps it then.
                    "/hold" => {
                        for _ in 0..1200 {
                            if fin.load(Ordering::SeqCst) {
                                break;
                            }
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                        ("image/gif", String::new())
                    }
                    "/finish" => {
                        fin.store(true, Ordering::SeqCst);
                        ("application/json", "{}".to_string())
                    }
                    "/people" => {
                        seen.lock().unwrap().push(target.clone());
                        let ids = query.strip_prefix("ids=").unwrap_or("");
                        ("application/json", serde_json::Value::Array(ids.split(',').map(|i| serde_json::json!({ "id": i, "name": format!("Fetched {i}") })).collect()).to_string())
                    }
                    _ => ("application/json", "[]".to_string()),
                };
                let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: {ty}\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            });
        }
    });
    let doc = serde_json::json!({ "version": 1, "state": { "ids": null, "people": null, "f": "a", "rows": null },
        "root": { "type": "stack", "children": [
            { "type": "ui-data-source", "props": { "name": "people", "batch": { "url": "/people" }, "persist": "device", "scope": "u1", "version": 2, "maxAge": 30, "maxRecords": 2 } },
            { "type": "ui-data-query", "props": { "source": "people", "ids": "{{ ids }}", "into": "people" } },
            { "type": "ui-data-query", "props": { "url": "/rows", "params": { "f": "{{ f }}" }, "into": "rows", "persist": "device", "scope": "u1", "keep": 2 } } ] } });
    let driver = r##"<script>
    document.addEventListener("DOMContentLoaded", async () => {
      const out = []; const pre = document.createElement("pre"); pre.id = "progress"; document.body.append(pre); const log = (...a) => { out.push(a.join(" ")); pre.textContent = out.join("\n"); };
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const until = async (f) => { for (let i = 0; i < 3000 && !f(); i++) await wait(10); };
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state, set = (k, v) => root.__irender.set(k, v);
      const db = window.__irenderOffline.db, SEP = "\u0001", now = Date.now();
      const slot = (id) => ["r", "u1", "people", id].join(SEP);
      try {
        // As an earlier visit would have left them: p1 for version 1, p2 for this one, p3
        // for this one but 40 days ago.
        await db.putMany("records", [
          [slot("p1"), { v: { id: "p1", name: "Kept p1" }, enc: null, at: now - 1000, ver: "1" }],
          [slot("p2"), { v: { id: "p2", name: "Kept p2" }, enc: null, at: now - 1000, ver: "2" }],
          [slot("p3"), { v: { id: "p3", name: "Kept p3" }, enc: null, at: now - 40 * 864e5, ver: "2" }],
        ]);
        set("ids", ["p1", "p2", "p3"]);
        await until(() => S().people?.length === 3 && S().people.every((p) => !p.name.startsWith("Kept") || p.id === "p2"));
        log("people", S().people.map((p) => p.name).join(","));
        await wait(300);
        const kept = (await db.entries("records", ["r", "u1", "people", ""].join(SEP))).map(([k]) => k.split(SEP)[3]).sort();
        log("kept", kept.join(","));
        for (const f of ["b", "c"]) {
          set("f", f);
          await wait(500);
        }
        await wait(300);
        const answers = (await db.entries("records", ["q", "u1", ""].join(SEP))).map(([k]) => JSON.parse(k.split(SEP)[2])[1].f).sort();
        log("answers", answers.join(","));
      } catch (e) { log("EXCEPTION", e.stack); }
      pre.textContent = out.join("\n"); pre.id = "result";
      fetch("/finish");
    });
    </script><img src="/hold" alt="" hidden>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    *page.lock().unwrap() = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive_url(&format!("http://127.0.0.1:{port}/")) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "people Fetched p1,Kept p2,Fetched p3",
            // Two at most: p2, kept longest ago, went.
            "kept p1,p3",
            "answers b,c",
        ],
        "\n{result}"
    );
    assert_eq!(*log.lock().unwrap(), ["/people?ids=p1,p3"]);
}

/// `expand` inside a joined record: each person's team is joined into the person, in a
/// table's rows and a query's, and each team is asked for once.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn data_sources_join_records_in_turn_in_chrome() {
    use std::io::Write;
    use std::sync::{Arc, Mutex};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let log = Arc::new(Mutex::new(Vec::<String>::new()));
    let seen = log.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let seen = seen.clone();
            std::thread::spawn(move || {
                let request = read_request(&mut stream);
                let target = request.split_whitespace().nth(1).unwrap_or("").replace("%2C", ",").trim_start_matches('/').to_string();
                seen.lock().unwrap().push(target.clone());
                let (path, query) = target.split_once('?').unwrap_or((&target, ""));
                let ids: Vec<&str> = query.strip_prefix("ids=").unwrap_or("").split(',').collect();
                let list: Vec<_> = match path {
                    "people" => ids.iter().map(|i| serde_json::json!({ "id": i, "name": format!("Person {i}"), "teamId": if *i == "p3" { "t2" } else { "t1" } })).collect(),
                    _ => {
                        std::thread::sleep(std::time::Duration::from_millis(400));
                        ids.iter().map(|i| serde_json::json!({ "id": i, "name": format!("Team {i}") })).collect()
                    }
                };
                let body = serde_json::Value::Array(list).to_string();
                let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            });
        }
    });
    let base = format!("http://127.0.0.1:{port}");
    let owner = serde_json::json!({ "owner": { "source": "people", "from": "ownerId", "expand": { "team": { "source": "teams", "from": "teamId" } } } });
    let doc = serde_json::json!({ "version": 1, "state": { "q": null,
        "rows": [{ "id": 1, "ownerId": "p1" }, { "id": 2, "ownerId": "p2" }, { "id": 3, "ownerId": "p3" }] },
        "root": { "type": "stack", "children": [
            { "type": "ui-data-source", "props": { "name": "people", "batch": { "url": format!("{base}/people") } } },
            { "type": "ui-data-source", "props": { "name": "teams", "batch": { "url": format!("{base}/teams") } } },
            { "type": "ui-data-table", "props": { "id": "t", "rows": "{{ rows }}", "expand": owner,
                "columns": [{ "field": "id" }, { "field": "owner.name", "label": "Owner" }, { "field": "owner.team.name", "label": "Team" }] } },
            { "type": "ui-data-query", "props": { "source": "people", "ids": ["p1", "p3"], "into": "q", "expand": { "team": { "source": "teams", "from": "teamId" } } } } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const pre = document.createElement("pre"); pre.id = "progress"; document.body.append(pre); const log = (...a) => { out.push(a.join(" ")); pre.textContent = out.join("\n"); };
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const until = async (f) => { for (let i = 0; i < 3000 && !f(); i++) await wait(10); };
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const t = document.getElementById("t");
      const rows = () => [...t.querySelectorAll("[data-dt-body] > [data-dt-row]")];
      const col = (n) => rows().map((r) => r.children[n].textContent.trim() || "_").join(",");
      try {
        await until(() => col(2).startsWith("Team"));
        log("teams", col(1), col(2));
        await until(() => S().q?.every((p) => p.team));
        log("query", S().q.map((p) => p.name + "/" + p.team.name).join(","));
      } catch (e) { log("EXCEPTION", e.stack); }
      pre.textContent = out.join("\n"); pre.id = "result";
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("data-in-turn", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        ["teams Person p1,Person p2,Person p3 Team t1,Team t1,Team t2", "query Person p1/Team t1,Person p3/Team t2"],
        "\n{result}"
    );
    let log = log.lock().unwrap();
    let teams: Vec<_> = log.iter().filter(|q| q.starts_with("teams")).collect();
    assert_eq!(teams, ["teams?ids=t1,t2"], "{log:?}");
}

/// A query whose rows are a source's `records` shows what's written to them at once: a
/// change, a deletion and a record made here, before and after they're saved; loaded
/// again, the rows are the server's.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn data_queries_show_writes_to_their_records_in_chrome() {
    use std::io::Write;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let log = Arc::new(Mutex::new(Vec::<String>::new()));
    let page = Arc::new(Mutex::new(String::new()));
    let done = Arc::new(AtomicBool::new(false));
    let (seen, html, fin) = (log.clone(), page.clone(), done.clone());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let (seen, html, fin) = (seen.clone(), html.clone(), fin.clone());
            std::thread::spawn(move || {
                let request = read_request(&mut stream);
                let method = request.split_whitespace().next().unwrap_or("").to_string();
                let path = request.split_whitespace().nth(1).unwrap_or("").to_string();
                let body = request.split("\r\n\r\n").nth(1).and_then(|b| serde_json::from_str::<serde_json::Value>(b).ok()).unwrap_or_default();
                if path == "/" {
                    let page = html.lock().unwrap().clone();
                    let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{page}", page.len());
                    return;
                }
                if path == "/hold" {
                    for _ in 0..1200 {
                        if fin.load(Ordering::SeqCst) {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }
                if path == "/finish" {
                    fin.store(true, Ordering::SeqCst);
                }
                if !matches!(path.as_str(), "/hold" | "/finish") {
                    seen.lock().unwrap().push(format!("{method} {path}"));
                }
                let (status, out) = match (method.as_str(), path.as_str()) {
                    ("GET", "/list") => ("200 OK", serde_json::json!([{ "id": "p1", "name": "Ana", "team": "x" }, { "id": "p2", "name": "Ben", "team": "y" }]).to_string()),
                    ("PATCH", "/people/p1") => ("200 OK", serde_json::json!({ "id": "p1", "name": body["name"], "team": "x" }).to_string()),
                    ("DELETE", "/people/p2") => ("204 No Content", String::new()),
                    ("POST", "/people") => ("201 Created", serde_json::json!({ "id": "p9", "name": body["name"], "team": "z" }).to_string()),
                    _ => ("404 Not Found", "{}".to_string()),
                };
                let _ = write!(stream, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, PATCH, DELETE\r\nAccess-Control-Allow-Headers: content-type, x-irender-data, x-queued-id, if-match\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{out}", out.len());
            });
        }
    });
    let base = "";
    let click = |id: &str, endpoint: String, method: &str, body: serde_json::Value| {
        serde_json::json!({ "type": "ui-button", "attrs": { "id": id }, "props": { "label": id }, "on": { "click": { "action": "submit", "endpoint": endpoint, "method": method, "body": body } } })
    };
    let doc = serde_json::json!({ "version": 1, "state": { "list": null, "again": 0 },
        "root": { "type": "stack", "children": [
            { "type": "ui-data-source", "props": { "name": "people", "one": format!("{base}/people/{{id}}"), "write": { "url": format!("{base}/people/{{id}}"), "create": format!("{base}/people") } } },
            { "type": "ui-data-query", "props": { "url": format!("{base}/list"), "into": "list", "records": "people", "refresh": "{{ again }}" } },
            click("rename", format!("{base}/people/p1"), "PATCH", serde_json::json!({ "name": "Ana B" })),
            click("drop", format!("{base}/people/p2"), "DELETE", serde_json::json!({})),
            click("add", format!("{base}/people"), "POST", serde_json::json!({ "name": "Cy" })) ] } });
    let driver = r##"<script>
    document.addEventListener("DOMContentLoaded", async () => {
      const out = []; const pre = document.createElement("pre"); pre.id = "progress"; document.body.append(pre); const log = (...a) => { out.push(a.join(" ")); pre.textContent = out.join("\n"); };
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const until = async (f) => { for (let i = 0; i < 3000 && !f(); i++) await wait(10); };
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const list = () => (S().list ?? []).map((r) => r.id + ":" + r.name + (r.team ? "/" + r.team : "")).join(",");
      try {
        await until(() => S().list?.length === 2);
        log("loaded", list());
        document.getElementById("rename").click();
        await until(() => list().includes("Ana B"));
        log("renamed", list());
        document.getElementById("drop").click();
        await until(() => !list().includes("p2"));
        log("dropped", list());
        document.getElementById("add").click();
        await until(() => list().includes("p9"));
        log("added", list());
        await wait(200);
        root.__irender.set("again", 1);
        await until(() => list().includes("Ben"));
        log("again", list());
      } catch (e) { log("EXCEPTION", e.stack); }
      pre.textContent = out.join("\n"); pre.id = "result";
      fetch("/finish");
    });
    </script><img src="/hold" alt="" hidden>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    *page.lock().unwrap() = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive_url(&format!("http://127.0.0.1:{port}/")) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "loaded p1:Ana/x,p2:Ben/y",
            // The row keeps its own fields; the change goes over them.
            "renamed p1:Ana B/x,p2:Ben/y",
            "dropped p1:Ana B/x",
            // Saved, it's known by the server's id.
            "added p1:Ana B/x,p9:Cy/z",
            // This test's server keeps nothing: loaded again, its rows are as they were.
            "again p1:Ana/x,p2:Ben/y",
        ],
        "\n{result}"
    );
    let log = log.lock().unwrap();
    assert_eq!(log.iter().filter(|l| !l.starts_with("GET")).collect::<Vec<_>>(), ["PATCH /people/p1", "DELETE /people/p2", "POST /people"], "{log:?}");
}

/// In a table of rows from state, joined fields are sorted, searched and filtered by,
/// though only a page of rows is drawn; `bindView` and a context menu's row have them.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn data_tables_sort_and_search_joined_fields_in_chrome() {
    use std::io::Write;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            std::thread::spawn(move || {
                let request = read_request(&mut stream);
                let target = request.split_whitespace().nth(1).unwrap_or("").replace("%2C", ",");
                let ids = target.split_once("ids=").map(|(_, i)| i.to_string()).unwrap_or_default();
                let name = |i: &str| match i { "p1" => "Zed", "p2" => "Amy", _ => "Mo" };
                let team = |i: &str| if i == "p1" { "Red" } else { "Blue" };
                let body = serde_json::Value::Array(ids.split(',').map(|i| serde_json::json!({ "id": i, "name": name(i), "team": team(i) })).collect()).to_string();
                let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            });
        }
    });
    let base = format!("http://127.0.0.1:{port}");
    let doc = serde_json::json!({ "version": 1, "state": { "sort": null, "q": "", "f": {}, "view": null, "ctx": null,
        "rows": [{ "id": 1, "ownerId": "p1" }, { "id": 2, "ownerId": "p2" }, { "id": 3, "ownerId": "p3" }, { "id": 4, "ownerId": "p2" }] },
        "root": { "type": "stack", "children": [
            { "type": "ui-data-source", "props": { "name": "people", "batch": { "url": format!("{base}/people") } } },
            { "type": "ui-data-table", "props": { "id": "t", "rows": "{{ rows }}", "pageSize": 2, "pageSizes": [], "bindSort": "sort", "bindSearch": "q", "bindFilters": "f", "bindView": "view",
                "expand": { "owner": { "source": "people", "from": "ownerId" } },
                "columns": [{ "field": "id" }, { "field": "owner.name", "label": "Owner" }, { "field": "owner.team", "label": "Team", "filter": "select" }] } } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const pre = document.createElement("pre"); pre.id = "progress"; document.body.append(pre); const log = (...a) => { out.push(a.join(" ")); pre.textContent = out.join("\n"); };
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const until = async (f) => { for (let i = 0; i < 3000 && !f(); i++) await wait(10); };
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state, set = (k, v) => root.__irender.set(k, v);
      const t = document.getElementById("t");
      const rows = () => [...t.querySelectorAll("[data-dt-body] > [data-dt-row]:not([hidden])")];
      const ids = () => rows().map((r) => r.children[0].textContent.trim()).join(",");
      const names = () => (S().view ?? []).map((r) => r.owner?.name ?? "_").join(",");
      try {
        await until(() => rows()[0]?.children[1].textContent.trim() === "Zed");
        log("page", ids());
        set("sort", "owner.name");
        await until(() => ids() === "2,4");
        log("sorted", ids());
        await until(() => !names().includes("_"));
        log("view", names());
        set("sort", "-owner.name");
        await until(() => ids() === "1,3");
        log("descending", ids());
        set("sort", null);
        set("q", "mo");
        await until(() => ids() === "3");
        log("searched", ids());
        set("q", "");
        set("f", { "owner.team": ["Blue"] });
        await until(() => ids() === "2,3");
        log("filtered", ids(), names());
        set("f", {});
        await until(() => ids() === "1,2");
        rows()[0].dispatchEvent(new CustomEvent("irender:context", { bubbles: true }));
        log("context", rows()[0].__irenderContext?.owner?.name);
      } catch (e) { log("EXCEPTION", e.stack); }
      pre.textContent = out.join("\n"); pre.id = "result";
    });
    </script>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("data-joined-sort", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        ["page 1,2", "sorted 2,4", "view Amy,Amy,Mo,Zed", "descending 1,3", "searched 3", "filtered 2,3 Amy,Mo,Amy", "context Zed"],
        "\n{result}"
    );
}

/// Records kept on the device (`persist: device`), from a page served over HTTP so that
/// IndexedDB keeps them: private fields aren't kept without a key, and are sealed once a
/// PIN's set; locking takes them off the page and unlocking brings them back from the
/// device with the server down; a query's last answer shows offline; `one` asks with its
/// ETag; another scope keeps its own; signing out wipes; turning the lock off lets the
/// sealed fields go.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn data_sources_keep_records_on_the_device_in_chrome() {
    use std::io::Write;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let log = Arc::new(Mutex::new(Vec::<String>::new()));
    let page = Arc::new(Mutex::new(String::new()));
    let down = Arc::new(AtomicBool::new(false));
    let done = Arc::new(AtomicBool::new(false));
    let (seen, html, off, fin) = (log.clone(), page.clone(), down.clone(), done.clone());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let (seen, html, off, fin) = (seen.clone(), html.clone(), off.clone(), fin.clone());
            std::thread::spawn(move || {
                let request = read_request(&mut stream);
                let target = request.split_whitespace().nth(1).unwrap_or("").replace("%2C", ",");
                let (path, query) = target.split_once('?').unwrap_or((&target, ""));
                let etag = request.lines().find_map(|l| l.to_ascii_lowercase().strip_prefix("if-none-match:").map(|v| v.trim().to_string()));
                let ty = if path == "/" { "text/html" } else { "application/json" };
                let (status, body, tag) = if path == "/" {
                    ("200 OK", html.lock().unwrap().clone(), "")
                } else if path == "/hold" {
                    // The page's load waits for this: answered once the driver's done (or
                    // after two minutes).
                    for _ in 0..1200 {
                        if fin.load(Ordering::SeqCst) {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                    ("404 Not Found", String::new(), "")
                } else if path == "/finish" {
                    fin.store(true, Ordering::SeqCst);
                    ("200 OK", "{}".into(), "")
                } else if path == "/down" {
                    off.store(true, Ordering::SeqCst);
                    ("200 OK", "{}".into(), "")
                } else if off.load(Ordering::SeqCst) {
                    seen.lock().unwrap().push(format!("down {target}"));
                    ("503 Service Unavailable", "{}".into(), "")
                } else {
                    seen.lock().unwrap().push(format!("{target}{}", etag.as_deref().map(|e| format!(" if {e}")).unwrap_or_default()));
                    match path {
                        "/people" => {
                            let ids = query.strip_prefix("ids=").unwrap_or("");
                            let list: Vec<_> = ids.split(',').map(|i| serde_json::json!({ "id": i, "name": format!("Person {i}"), "email": format!("{i}@example.com") })).collect();
                            ("200 OK", serde_json::Value::Array(list).to_string(), "")
                        }
                        "/accounts/a1" if etag.as_deref() == Some("\"v1\"") => ("304 Not Modified", String::new(), "\"v1\""),
                        "/accounts/a1" => ("200 OK", serde_json::json!({ "id": "a1", "title": "Account a1" }).to_string(), "\"v1\""),
                        "/visits" if query.contains("fail=1") => ("500 Internal Server Error", "{}".into(), ""),
                        "/visits" if query.contains("since=v1") => ("200 OK", serde_json::json!({ "rows": [], "cursor": "v1" }).to_string(), ""),
                        "/visits" => ("200 OK", serde_json::json!({ "rows": [{ "id": 1, "who": "Ana", "phone": "555-0101" }, { "id": 2, "who": "Ben", "phone": "555-0102" }], "total": 2, "cursor": "v1" }).to_string(), ""),
                        _ => ("404 Not Found", "{}".into(), ""),
                    }
                };
                let tag = if tag.is_empty() { String::new() } else { format!("ETag: {tag}\r\n") };
                let _ = write!(stream, "HTTP/1.1 {status}\r\nContent-Type: {ty}\r\n{tag}Cache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            });
        }
    });
    let doc = serde_json::json!({ "version": 1,
        "state": { "scope": "u1", "fail": null, "out": false, "rows": null, "rowsStatus": null, "picked": ["p1", "p2"], "people": null, "acct": null, "locked": false, "again": 0 },
        "root": { "type": "stack", "children": [
            { "type": "ui-app-lock", "props": { "methods": ["pin"], "name": "t", "after": 0, "bindLocked": "locked" } },
            { "type": "ui-data-source", "props": { "name": "people", "batch": { "url": "/people" }, "persist": "device", "scope": "{{ scope }}", "private": ["email"], "clear": "{{ out }}" } },
            { "type": "ui-data-source", "props": { "name": "accounts", "one": "/accounts/{id}", "persist": "device", "scope": "{{ scope }}" } },
            { "type": "ui-data-query", "props": { "source": "people", "ids": "{{ picked }}", "into": "people" } },
            { "type": "ui-data-query", "props": { "source": "accounts", "ids": "a1", "into": "acct" } },
            { "type": "ui-data-query", "props": { "url": "/visits", "params": { "fail": "{{ fail }}" }, "into": "rows", "status": "rowsStatus", "refresh": "{{ again }}", "sync": true, "persist": "device", "scope": "{{ scope }}", "private": ["phone"] } } ] } });
    let driver = r##"<script>
    document.addEventListener("DOMContentLoaded", async () => {
      const out = []; const pre = document.createElement("pre"); pre.id = "progress"; document.body.append(pre); const log = (...a) => { out.push(a.join(" ")); pre.textContent = out.join("\n"); };
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      // Timers, not message ticks: WebKit runs ticks ahead of the timers the sources wait on.
      const until = async (f) => { for (let i = 0; i < 6000 && !(await f()); i++) await wait(10); };
      const $ = (s) => document.querySelector(s);
      const root = $("[data-i-root]"), S = () => root.__irender.state, set = (k, v) => root.__irender.set(k, v);
      const tap = async (d) => { for (const c of d) $(`[data-k="${c}"]`).click(); await wait(30); };
      const kept = () => new Promise((ok) => { const r = indexedDB.open("irender-records"); r.onsuccess = () => { const tx = r.result.transaction("records"), st = tx.objectStore("records"), out = {}; const k = st.getAllKeys(), v = st.getAll(); tx.oncomplete = () => { k.result.forEach((key, i) => (out[key.split("\u0001").join("/")] = v.result[i])); r.result.close(); ok(out); }; }; });
      const names = () => (S().people ?? []).map((p) => `${p.name}<${p.email ?? "-"}>`).join(",");
      try {
        await until(() => S().people?.length === 2 && S().rowsStatus?.state === "ready" && S().acct);
        let k = await kept();
        log("no key", names(), k["r/u1/people/p1"].v.name, "email" in k["r/u1/people/p1"].v, String(k["r/u1/people/p1"].enc), "phone" in k["q/u1/" + Object.keys(k).find((x) => x.startsWith("q/")).slice(5)].v[0]);
        // A PIN: the key's made and the vault opens; what's fetched next is sealed.
        $("[data-set=pin]").click(); await wait(20); await tap("2580"); await tap("2580");
        await until(() => window.__irenderOffline.vault.key && !$("dialog").open && $("[data-lock]"));
        window.__irenderData.invalidate("people");
        // The query's rows again, now with a key: their phones are sealed.
        set("again", 1);
        await until(async () => (await kept())["r/u1/people/p1"]?.enc && Object.entries(await kept()).some(([key, v]) => key.startsWith("q/") && v.enc));
        k = await kept();
        const p1 = k["r/u1/people/p1"];
        log("sealed", "email" in p1.v, p1.enc.data.length > 0, new TextDecoder().decode(p1.enc.data).includes("example.com"));
        // `one` asks with its ETag.
        window.__irenderData.invalidate("accounts");
        await wait(200);
        // Locked: the email leaves the page.
        $("[data-lock]").click(); await until(() => !S().people?.[0]?.email);
        log("locked", S().locked, names());
        // The server goes down; unlocking brings the email back from the device.
        await fetch("/down");
        await tap("2580");
        await until(() => S().people?.[0]?.email);
        log("unlocked", S().locked, names());
        // The query: another address fails, the kept one shows its last answer.
        set("fail", 1); await until(() => S().rowsStatus?.state === "stale"); set("fail", null);
        await until(() => S().rowsStatus?.state === "stale" && S().rowsStatus.cached);
        log("offline query", S().rows.map((r) => r.who + ":" + r.phone).join(","), S().rowsStatus.state, S().rowsStatus.cached);
        // Someone else: nothing of theirs is kept; back again, what was kept shows.
        set("scope", "u2"); await until(() => !S().people?.length); log("u2", names() || "none");
        set("scope", "u1"); await until(() => S().people?.length === 2); log("u1 again", names());
        // Signing out wipes the people kept for u1.
        set("out", true); await until(async () => !Object.keys(await kept()).some((x) => x.startsWith("r/u1/people")));
        log("signed out", Object.keys(await kept()).filter((x) => x.startsWith("r/u1/")).join(","));
        // Turning the lock off lets what its key sealed go.
        $("[data-off]").click();
        await until(async () => !Object.values(await kept()).some((x) => x.enc));
        log("lock off", Object.values(await kept()).filter((x) => x.enc).length, String(window.__irenderOffline.vault.key));
      } catch (e) { log("EXCEPTION", e.stack); }
      pre.textContent = out.join("\n");
      pre.id = "result";
      fetch("/finish");
    });
    </script><img src="/hold" alt="" hidden>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    *page.lock().unwrap() = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive_url(&format!("http://127.0.0.1:{port}/")) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "no key Person p1<p1@example.com>,Person p2<p2@example.com> Person p1 false null false",
            "sealed false true false",
            "locked true Person p1<->,Person p2<->",
            "unlocked false Person p1<p1@example.com>,Person p2<p2@example.com>",
            "offline query Ana:555-0101,Ben:555-0102 stale true",
            "u2 none",
            "u1 again Person p1<p1@example.com>,Person p2<p2@example.com>",
            "signed out r/u1/accounts/a1",
            "lock off 0 null",
        ],
        "\n{result}"
    );
    let log = log.lock().unwrap();
    assert!(log.iter().any(|q| q == "/accounts/a1 if \"v1\""), "{log:?}");
    // The query's cursor: from memory after the PIN, and kept on the device for the load
    // with the server down.
    assert!(log.iter().any(|q| q == "/visits?since=v1"), "{log:?}");
    assert!(log.iter().any(|q| q == "down /visits?since=v1"), "{log:?}");
    // Unlocked with the server down, the people came from the device: the only ones asked
    // for while it was down were u2's, and u1's after signing out.
    assert!(log.iter().filter(|q| q.starts_with("down /people")).count() <= 2, "{log:?}");
}

/// `sync`: after a full answer, a query asks only for what changed since the server's
/// cursor and merges it (changed rows in place, new ones at the end, ids in `deleted` and
/// rows marked deleted gone); `full: 0` always loads everything; a 403 empties it, and a
/// 401 keeps the rows and fires `unauthorized`.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn data_queries_sync_what_changed_in_chrome() {
    use std::io::{Read, Write};
    use std::sync::{Arc, Mutex};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let log = Arc::new(Mutex::new(Vec::<String>::new()));
    let mode = Arc::new(Mutex::new(String::new()));
    let (seen, how) = (log.clone(), mode.clone());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut buf = [0u8; 4096];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]).to_string();
            let target = request.split_whitespace().nth(1).unwrap_or("").to_string();
            let (path, query) = target.split_once('?').unwrap_or((&target, ""));
            let since = query.split('&').find_map(|kv| kv.strip_prefix("since=")).unwrap_or("");
            let (status, body) = if path == "/forbid" || path == "/unauth" {
                *how.lock().unwrap() = path.to_string();
                ("200 OK", "{}".to_string())
            } else {
                seen.lock().unwrap().push(query.to_string());
                match how.lock().unwrap().as_str() {
                    "/forbid" => ("403 Forbidden", "{}".to_string()),
                    "/unauth" => ("401 Unauthorized", "{}".to_string()),
                    _ => ("200 OK", match since {
                        "c1" => serde_json::json!({ "rows": [{ "id": 2, "n": "b2" }, { "id": 4, "n": "d" }], "deleted": [3], "cursor": "c2" }),
                        "c2" => serde_json::json!({ "rows": [{ "id": 1, "deleted": true }], "cursor": "c3" }),
                        "c3" => serde_json::json!({ "rows": [], "cursor": "c3" }),
                        _ => serde_json::json!({ "rows": [{ "id": 1, "n": "a" }, { "id": 2, "n": "b" }, { "id": 3, "n": "c" }], "cursor": "c1", "total": 3 }),
                    }.to_string()),
                }
            };
            let _ = write!(stream, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        }
    });
    let base = format!("http://127.0.0.1:{port}");
    let doc = serde_json::json!({ "version": 1, "state": { "again": 0, "rows": null, "st": null, "all": null, "out": 0 },
        "root": { "type": "stack", "children": [
            { "type": "ui-data-query", "props": { "url": format!("{base}/items"), "params": { "who": "a" }, "into": "rows", "status": "st", "refresh": "{{ again }}", "sync": true },
              "on": { "unauthorized": { "action": "set_state", "key": "out", "value": "{{ out + 1 }}" } } },
            { "type": "ui-data-query", "props": { "url": format!("{base}/items"), "params": { "who": "b" }, "into": "all", "refresh": "{{ again }}", "sync": { "full": 0 } } } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const tick = () => new Promise((r) => { const c = new MessageChannel(); c.port1.onmessage = r; c.port2.postMessage(0); });
      const until = async (f) => { for (let i = 0; i < 2e6 && !f(); i++) await (i % 50 ? tick() : wait(10)); };
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const rows = () => (S().rows ?? []).map((r) => r.id + r.n).join(",");
      const again = async () => { const at = S().st?.at; root.__irender.set("again", S().again + 1); await wait(250); await until(() => S().st?.state !== "loading" && (S().st?.at !== at || S().st?.error)); };
      try {
        await until(() => S().st?.state === "ready" && S().all);
        log("full", rows(), S().st.total);
        await again(); log("delta", rows(), S().st.total);
        await again(); log("tombstone", rows());
        await again(); log("nothing new", rows());
        await fetch("__BASE__/unauth");
        await again(); log("401", rows(), S().st.state, S().st.error, S().out);
        await fetch("__BASE__/forbid");
        await again(); log("403", rows() || "none", S().st.state);
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##.replace("__BASE__", &base);
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive("data-sync", &html) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "full 1a,2b,3c 3",
            // 2 changed in place, 3 gone by `deleted`, 4 new at the end.
            "delta 1a,2b2,4d 3",
            "tombstone 2b2,4d",
            "nothing new 2b2,4d",
            "401 2b2,4d stale 401 1",
            "403 none ready",
        ],
        "\n{result}"
    );
    let log = log.lock().unwrap();
    let a: Vec<_> = log.iter().filter(|q| q.contains("who=a")).cloned().collect();
    assert_eq!(a[..4], ["who=a", "who=a&since=c1", "who=a&since=c2", "who=a&since=c3"], "{log:?}");
    // `full: 0`: never a delta.
    assert!(log.iter().filter(|q| q.contains("who=b")).all(|q| !q.contains("since")), "{log:?}");
}

/// Changes made here (`write`): shown at once and queued (kept on the device) while
/// there's no connection, two edits sent as one; a server that changed other fields
/// meanwhile is merged with; `ask` waits for `ui-data-conflicts`; a refused change goes
/// back; a record made here takes the server's id; a `submit` button goes the same way.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn data_sources_write_offline_and_settle_conflicts_in_chrome() {
    use std::collections::HashMap;
    use std::io::Write;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let log = Arc::new(Mutex::new(Vec::<String>::new()));
    let page = Arc::new(Mutex::new(String::new()));
    let down = Arc::new(AtomicBool::new(false));
    let done = Arc::new(AtomicBool::new(false));
    let mut start = HashMap::new();
    start.insert("p1".to_string(), serde_json::json!({ "id": "p1", "name": "Ana", "phone": "111", "email": "ana@x", "version": 1 }));
    start.insert("c1".to_string(), serde_json::json!({ "id": "c1", "title": "Draft", "version": 1 }));
    let data = Arc::new(Mutex::new(start));
    let (seen, html, off, fin, db) = (log.clone(), page.clone(), down.clone(), done.clone(), data.clone());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let (seen, html, off, fin, db) = (seen.clone(), html.clone(), off.clone(), fin.clone(), db.clone());
            std::thread::spawn(move || {
                let request = read_request(&mut stream);
                let method = request.split_whitespace().next().unwrap_or("").to_string();
                let target = request.split_whitespace().nth(1).unwrap_or("").replace("%2C", ",");
                let (path, query) = target.split_once('?').unwrap_or((&target, ""));
                let body: serde_json::Value = request.split("\r\n\r\n").nth(1).and_then(|b| serde_json::from_str(b).ok()).unwrap_or(serde_json::Value::Null);
                let ty = if path == "/" { "text/html" } else { "application/json" };
                let api = path.starts_with("/people") || path.starts_with("/cards");
                // Down: the connection drops, as with no network.
                if api && off.load(Ordering::SeqCst) {
                    seen.lock().unwrap().push(format!("dropped {method} {path}"));
                    return;
                }
                if api {
                    seen.lock().unwrap().push(format!("{method} {path} {}", if body.is_null() { String::new() } else { body.to_string() }));
                }
                let mut db = db.lock().unwrap();
                let (status, out) = match (method.as_str(), path) {
                    (_, "/") => ("200 OK", html.lock().unwrap().clone()),
                    (_, "/hold") => {
                        drop(db);
                        for _ in 0..1200 {
                            if fin.load(Ordering::SeqCst) {
                                break;
                            }
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                        ("404 Not Found", String::new())
                    }
                    (_, "/finish") => {
                        fin.store(true, Ordering::SeqCst);
                        ("200 OK", "{}".into())
                    }
                    (_, "/down") => {
                        off.store(true, Ordering::SeqCst);
                        ("200 OK", "{}".into())
                    }
                    (_, "/up") => {
                        off.store(false, Ordering::SeqCst);
                        ("200 OK", "{}".into())
                    }
                    // Someone else changes p1's email and c1's title.
                    (_, "/touch") => {
                        let p = db.get_mut("p1").unwrap();
                        p["email"] = "ana@new".into();
                        p["version"] = (p["version"].as_i64().unwrap() + 1).into();
                        let c = db.get_mut("c1").unwrap();
                        c["title"] = "Theirs".into();
                        c["version"] = (c["version"].as_i64().unwrap() + 1).into();
                        ("200 OK", "{}".into())
                    }
                    ("GET", "/people") | ("GET", "/cards") => {
                        let ids = query.strip_prefix("ids=").unwrap_or("");
                        let list: Vec<_> = ids.split(',').filter_map(|i| db.get(i).cloned()).collect();
                        ("200 OK", serde_json::Value::Array(list).to_string())
                    }
                    ("POST", "/people") => {
                        let mut rec = body.clone();
                        rec["id"] = "p9".into();
                        rec["version"] = 1.into();
                        db.insert("p9".into(), rec.clone());
                        ("201 Created", rec.to_string())
                    }
                    ("PATCH", p) => {
                        let id = p.rsplit('/').next().unwrap().to_string();
                        if body.get("bad").is_some() {
                            ("422 Unprocessable Entity", serde_json::json!({ "error": "That isn't allowed" }).to_string())
                        } else {
                            let cur = db.get_mut(&id).unwrap();
                            if body["version"] != cur["version"] {
                                ("409 Conflict", cur.to_string())
                            } else {
                                for (k, v) in body.as_object().unwrap() {
                                    if k != "version" {
                                        cur[k.as_str()] = v.clone();
                                    }
                                }
                                cur["version"] = (cur["version"].as_i64().unwrap() + 1).into();
                                ("200 OK", cur.to_string())
                            }
                        }
                    }
                    _ => ("404 Not Found", "{}".into()),
                };
                let _ = write!(stream, "HTTP/1.1 {status}\r\nContent-Type: {ty}\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{out}", out.len());
            });
        }
    });
    let doc = serde_json::json!({ "version": 1,
        "state": { "p": null, "c": null, "pw": [], "cw": [], "phone": "333", "events": "" },
        "root": { "type": "stack", "children": [
            { "type": "ui-data-source", "props": { "name": "people", "batch": { "url": "/people" }, "persist": "device", "scope": "u1", "bindWrites": "pw",
                "write": { "url": "/people/{id}", "create": "/people", "version": "version" } },
              "on": { "created": { "action": "set_state", "key": "events", "value": "{{ events + ' created' }}" }, "refused": { "action": "set_state", "key": "events", "value": "{{ events + ' refused' }}" } } },
            { "type": "ui-data-source", "props": { "name": "cards", "batch": { "url": "/cards" }, "bindWrites": "cw",
                "write": { "url": "/cards/{id}", "version": "version", "conflict": "ask" } } },
            { "type": "ui-data-query", "props": { "source": "people", "ids": "p1", "into": "p" } },
            { "type": "ui-data-query", "props": { "source": "cards", "ids": "c1", "into": "c" } },
            { "type": "ui-button", "attrs": { "id": "save" }, "props": { "label": "Save" },
              "on": { "click": { "action": "submit", "endpoint": "/people/p1", "method": "PATCH", "body": { "phone": "{{ phone }}" } } } },
            { "type": "ui-data-conflicts", "attrs": { "id": "dc" }, "props": { "nameField": "title", "labels": { "title": "Title" } } } ] } });
    let driver = r##"<script>
    document.addEventListener("DOMContentLoaded", async () => {
      const out = []; const pre = document.createElement("pre"); pre.id = "progress"; document.body.append(pre);
      const log = (...a) => { out.push(a.join(" ")); pre.textContent = out.join("\n"); };
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const until = async (f) => { for (let i = 0; i < 1500 && !(await f()); i++) await wait(20); };
      const $ = (s) => document.querySelector(s);
      const root = $("[data-i-root]"), S = () => root.__irender.state, D = window.__irenderData;
      const patch = (url, body) => fetch(url, { method: "PATCH", headers: { "content-type": "application/json" }, body: JSON.stringify(body) }).then((r) => r.json());
      const kept = () => new Promise((ok) => { const r = indexedDB.open("irender-records"); r.onsuccess = () => { const tx = r.result.transaction("records"), st = tx.objectStore("records"), out = {}; const k = st.getAllKeys(), v = st.getAll(); tx.oncomplete = () => { k.result.forEach((key, i) => (out[key.split("\u0001")[0]] = (out[key.split("\u0001")[0]] ?? 0) + 1)); r.result.close(); ok(out); }; }; });
      try {
        await until(() => S().p && S().c);
        // Offline: shown at once, queued, two edits as one, kept on the device.
        await fetch("/down");
        const a = await patch("/people/p1", { name: "Ana B" });
        // Until the dropped connection has failed (Firefox takes a while): a change being
        // sent can't take another.
        await until(() => S().pw[0]?.status === "waiting");
        await patch("/people/p1", { phone: "222" });
        await until(() => S().p.phone === "222" && S().pw[0]?.changes.phone && S().pw[0]?.status === "waiting");
        log("offline", a.queued, S().p.name, S().p.phone, S().pw.length, S().pw[0]?.status, JSON.stringify(S().pw[0]?.changes), (await kept()).w);
        // Back, with someone else's change to another field: merged, saved.
        await fetch("/touch"); await fetch("/up");
        D.flush();
        await until(async () => !S().pw.length && S().p.version === 3 && !(await kept()).w);
        log("merged", S().p.name, S().p.phone, S().p.email, S().p.version, (await kept()).w ?? 0);
        // `ask`: their title and ours, settled by choosing ours.
        await patch("/cards/c1", { title: "Mine" });
        await until(() => S().cw[0]?.status === "conflict");
        log("ask", S().c.title, S().cw[0].fields.map((f) => `${f.field}:${f.mine}/${f.theirs}`).join(), $("#dc").hidden, $("#dc [data-dc-field] span").textContent);
        $("#dc [data-keep]").click();
        await until(() => !S().cw.length);
        log("chose mine", S().c.title, S().c.version, $("#dc").hidden);
        // Refused: back to what it was, shown until dismissed.
        await patch("/people/p1", { name: "Nope", bad: true });
        await until(() => S().pw[0]?.status === "refused");
        log("refused", S().p.name, S().pw[0].error, S().events.trim(), !$("#dc").hidden);
        $("#dc [data-dismiss]").click(); await until(() => !S().pw.length); log("dismissed", $("#dc").hidden);
        // Made here: a temporary id, then the server's.
        const made = await fetch("/people", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ name: "New" }) }).then((r) => r.json());
        await until(() => S().events.includes("created"));
        log("created", made.id.startsWith("tmp-"), JSON.stringify(D.lookup("people", ["p9"])[0].value?.name));
        // A submit button, through the same queue.
        $("#save").click();
        await until(() => S().p.phone === "333" && !S().pw.length);
        log("submit", S().p.phone, S().p.version);
      } catch (e) { log("EXCEPTION", e.stack); }
      pre.id = "result";
      fetch("/finish");
    });
    </script><img src="/hold" alt="" hidden>"##;
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    let ir = engine.resolve(&doc).unwrap();
    *page.lock().unwrap() = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let Some(result) = drive_url(&format!("http://127.0.0.1:{port}/")) else { return };
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "offline true Ana B 222 1 waiting {\"name\":\"Ana B\",\"phone\":\"222\"} 1",
            "merged Ana B 222 ana@new 3 0",
            "ask Mine title:Mine/Theirs false Title",
            "chose mine Mine 3 true",
            "refused Ana B That isn't allowed refused true",
            "dismissed true",
            "created true \"New\"",
            "submit 333 4",
        ],
        "\n{result}"
    );
    let log = log.lock().unwrap();
    // Sent once with the old version (409), then on theirs.
    assert!(log.iter().any(|q| q.starts_with("PATCH /people/p1") && q.contains("\"version\":1")), "{log:?}");
    assert!(log.iter().any(|q| q.starts_with("PATCH /people/p1") && q.contains("\"version\":2") && q.contains("Ana B")), "{log:?}");
}
