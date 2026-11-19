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

    $$("#tabs [role=tab]")[1].click();
    log("tabs", $$("#tabs [role=tabpanel]").map((p) => (p.hidden ? "-" : "shown")).join(","), S().tab);

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
    assert_eq!(asked, "groupBy=region\ngroupBy=region&group=North\nq=d&groupBy=region\nq=d&groupBy=region&group=North\nq=d");
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
