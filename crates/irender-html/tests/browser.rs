//! End-to-end: render the product example, drive it in headless Chrome, check the DOM.
//!
//! Ignored by default because it needs Chrome:
//!   cargo test -p irender-html --test browser -- --ignored
//! Set CHROME=/path/to/chrome if it isn't found automatically.

mod support;

use irender_core::Format;
use irender_html::Engine;
use std::{path::PathBuf, process::Command};

fn chrome() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("CHROME") {
        return Some(path.into());
    }
    let candidates = [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
        "google-chrome",
        "google-chrome-stable",
        "chromium",
        "chromium-browser",
    ];
    candidates
        .iter()
        .map(PathBuf::from)
        .find(|p| Command::new(p).arg("--version").output().is_ok_and(|o| o.status.success()))
}

/// Clicks and types like a user, then records what the page shows after each step.
const DRIVER: &str = r##"<style>* { transition: none !important; }</style>
<script>
window.addEventListener("load", () => {
  const out = [];
  const [minus, plus] = document.querySelectorAll("button[aria-label]");
  const buy = document.querySelectorAll("button")[2];
  const price = document.querySelector("#product > span");
  const bar = document.querySelector("[role=progressbar] > div");
  const promo = document.querySelector("p");
  const input = document.querySelector("input");
  let call = null;
  document.addEventListener("irender:call", (e) => (call = e.detail));
  const snap = (step) => out.push([step, price.textContent, minus.disabled, input.value,
    Math.round((bar.offsetWidth / bar.parentElement.clientWidth) * 100) + "%", promo.hidden, buy.textContent,
    bar.parentElement.getAttribute("aria-valuenow")].join(" | "));
  snap("initial");
  plus.click(); plus.click();
  snap("+2");
  input.value = "6"; input.dispatchEvent(new Event("input"));
  snap("typed 6");
  minus.click();
  snap("-1");
  buy.click();
  out.push("call " + JSON.stringify({ name: call.name, args: call.args }));
  out.push("state " + JSON.stringify(document.querySelector("[data-i-root]").__irender.state));
  const pre = document.createElement("pre");
  pre.id = "result";
  pre.textContent = out.join("\n");
  document.body.append(pre);
});
</script>"##;

/// Opens `html` in headless Chrome and returns the text of `<pre id="result">`, which
/// the page's driver script writes. `None` if Chrome isn't installed.
fn drive(name: &str, html: &str) -> Option<String> {
    let Some(chrome) = chrome() else {
        eprintln!("skipping: Chrome not found (set CHROME=/path/to/chrome)");
        return None;
    };
    let dir = std::env::temp_dir().join(format!("irender-browser-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("page.html");
    std::fs::write(&file, html).unwrap();

    // `--no-sandbox`: CI runners (e.g. Ubuntu 24.04) block the sandbox's user
    // namespaces, and the page is one this test just generated.
    let out = Command::new(&chrome)
        .args(["--headless=new", "--no-sandbox", "--disable-gpu", "--window-size=600,800", "--virtual-time-budget=5000", "--dump-dom"])
        .arg(format!("file://{}", file.display()))
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let dom = String::from_utf8_lossy(&out.stdout);
    let start = dom.find("<pre id=\"result\">").unwrap_or_else(|| panic!("driver did not run:\n{dom}")) + 17;
    Some(
        dom[start..start + dom[start..].find("</pre>").unwrap()]
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&")
            .replace("&quot;", "\""),
    )
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn product_example_is_interactive_in_chrome() {
    let (src, data) = support::product();
    let engine = Engine::new();
    let ir = engine.load(&src, Format::Json).unwrap();
    let html = engine.render(&ir, &data).unwrap().document("test").replace("</body>", &format!("{DRIVER}</body>"));
    let Some(result) = drive("product", &html) else { return };
    let lines: Vec<&str> = result.lines().collect();

    assert_eq!(lines[0], "initial | ₹1499 | true | 1 | 20% | true | Buy now | 1", "{result}");
    assert_eq!(lines[1], "+2 | ₹4497 | false | 3 | 60% | true | Buy 3 | 3", "{result}");
    assert_eq!(lines[2], "typed 6 | ₹8994 | false | 6 | 100% | false | Buy 6 | 6", "{result}");
    assert_eq!(lines[3], "-1 | ₹7495 | false | 5 | 100% | false | Buy 5 | 5", "{result}");
    assert_eq!(lines[4], r#"call {"name":"buy","args":{"sku":"POS-01","qty":5}}"#);
    assert_eq!(lines[5], r#"state {"qty":5}"#);
}

/// `if` blocks, lists rebuilt from state, item scopes in actions and `mount` actions.
const DYNAMIC: &str = r##"{ "version": 1,
  "state": { "open": false, "todos": [{ "title": "a", "done": true }], "last": "", "mounted": 0, "remote": null },
  "root": { "type": "element",
    "on": { "mount": [
      { "action": "set_state", "key": "mounted", "value": "{{ mounted + 1 }}" },
      { "action": "submit", "method": "GET", "endpoint": "data:application/json,[\"x\",\"y\"]", "into": "remote" } ] },
    "children": [
      { "type": "button", "attrs": { "id": "toggle" }, "props": { "label": "t" },
        "on": { "click": { "action": "set_state", "key": "open", "value": "{{ !open }}" } } },
      { "type": "if", "props": { "when": "{{ open }}" }, "children": [
        { "type": "text", "attrs": { "id": "panel" }, "props": { "value": "panel {{ todos.length }}" } },
        { "type": "input", "attrs": { "id": "field" } } ] },
      { "type": "list", "props": { "items": "{{ todos }}", "as": "todo", "tag": "ul" }, "children": [
        { "type": "element", "props": { "tag": "li" }, "class": ["{{ todo.done ? 'done' : '' }}"],
          "on": { "click": { "action": "set_state", "key": "last", "value": "{{ todo.title }}{{ index }}" },
                  "mount": { "action": "set_state", "key": "mounted", "value": "{{ mounted + 1 }}" } },
          "children": [
            { "type": "text", "props": { "value": "{{ todo.title }}" } },
            { "type": "if", "props": { "when": "{{ todo.done }}" }, "children": [{ "type": "text", "props": { "value": "✓" } }] } ] } ] },
      { "type": "list", "props": { "items": "{{ remote }}", "tag": "ol" },
        "children": [{ "type": "text", "props": { "tag": "li", "value": "{{ item }}" } }] } ] } }"##;

const DYNAMIC_DRIVER: &str = r##"<script>
window.addEventListener("load", () => {
  const out = [];
  const root = document.querySelector("[data-i-root]");
  document.addEventListener("irender:error", (e) => out.push("error " + e.detail.error));
  const texts = (sel) => [...document.querySelectorAll(sel)]
    .map((el) => el.textContent + (el.classList.contains("done") ? "*" : "")).join(",");
  const snap = (step) => out.push([step, document.getElementById("panel")?.textContent ?? "-", texts("ul > li"),
    root.__irender.state.last, root.__irender.state.mounted].join(" | "));
  const toggle = document.getElementById("toggle");
  snap("initial");
  toggle.click();
  snap("open");
  document.getElementById("field").value = "kept";
  toggle.click();
  snap("closed");
  toggle.click();
  out.push("field " + document.getElementById("field").value);
  root.__irender.set("todos", [{ title: "a", done: false }, { title: "b", done: true }]);
  snap("set todos");
  document.querySelectorAll("ul > li")[1].click();
  snap("clicked b");
  setTimeout(() => {
    out.push("remote " + texts("ol > li"));
    const pre = document.createElement("pre");
    pre.id = "result";
    pre.textContent = out.join("\n");
    document.body.append(pre);
  }, 200);
});
</script>"##;

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn conditionals_lists_and_mount_actions_in_chrome() {
    let engine = Engine::new();
    let ir = engine.load(DYNAMIC, Format::Json).unwrap();
    let page = engine.render(&ir, &serde_json::json!({})).unwrap();
    let html = page.document("test").replace("</body>", &format!("{DYNAMIC_DRIVER}</body>"));
    let Some(result) = drive("dynamic", &html) else { return };
    let lines: Vec<&str> = result.lines().collect();

    // The page's mount action ran once and the server-rendered item's once.
    assert_eq!(lines[0], "initial | - | a✓* |  | 2", "{result}");
    assert_eq!(lines[1], "open | panel 1 | a✓* |  | 2", "{result}");
    assert_eq!(lines[2], "closed | - | a✓* |  | 2", "{result}");
    assert_eq!(lines[3], "field kept", "hidden content keeps its nodes\n{result}");
    // Rebuilt items mount again (+2) and evaluate `todo` per item.
    assert_eq!(lines[4], "set todos | panel 2 | a,b✓* |  | 4", "{result}");
    assert_eq!(lines[5], "clicked b | panel 2 | a,b✓* | b1 | 4", "{result}");
    assert_eq!(lines[6], "remote x,y", "{result}");
}

/// Radios (also from a rebuilt list), a numeric select and radios on one key, a
/// checkbox, selects with fixed and state-driven options, and a textarea.
const FORM: &str = r##"{ "version": 1,
  "state": { "sizes": ["s", "m", "l"], "size": "m", "qty": 2, "gift": false, "country": "",
             "cities": ["Pune", "Delhi"], "city": "Delhi", "notes": "hi" },
  "root": { "type": "element", "children": [
    { "type": "list", "props": { "items": "{{ sizes }}", "as": "s" }, "attrs": { "id": "sizes" },
      "children": [{ "type": "input", "props": { "type": "radio", "bind": "size", "value": "{{ s }}" } }] },
    { "type": "list", "props": { "items": [1, 2, 3], "as": "n" }, "attrs": { "id": "qty" },
      "children": [{ "type": "input", "props": { "type": "radio", "bind": "qty", "value": "{{ n }}" } }] },
    { "type": "select", "props": { "bind": "qty", "options": [1, 2, 3] }, "attrs": { "id": "qty-select" } },
    { "type": "input", "props": { "type": "checkbox", "bind": "gift" }, "attrs": { "id": "gift" } },
    { "type": "select", "props": { "bind": "country", "placeholder": "Country",
      "options": [{ "value": "in", "label": "India" }, { "value": "de", "label": "Germany" }] }, "attrs": { "id": "country" } },
    { "type": "select", "props": { "bind": "city", "options": "{{ cities }}" }, "attrs": { "id": "city" } },
    { "type": "textarea", "props": { "bind": "notes" }, "attrs": { "id": "notes" } } ] } }"##;

const FORM_DRIVER: &str = r##"<script>
window.addEventListener("load", () => {
  const out = [];
  const root = document.querySelector("[data-i-root]");
  const $ = (s) => document.querySelector(s);
  const checked = (sel) => [...document.querySelectorAll(sel + " input")].map((r) => r.value + (r.checked ? "*" : "")).join(",");
  const state = () => { const s = root.__irender.state; return [s.size, s.qty, typeof s.qty, s.gift, s.country, s.city, s.notes].join("|"); };
  const snap = (step) => out.push([step, checked("#sizes"), checked("#qty"), $("#qty-select").value, $("#gift").checked,
    $("#country").value, $("#city").value, [...$("#city").options].map((o) => o.value).join(","), $("#notes").value, state()].join(" ; ").replace(/\n/g, "⏎"));
  const pick = (el, value) => { el.value = value; el.dispatchEvent(new Event("change")); };
  snap("initial");
  document.querySelectorAll("#sizes input")[2].click();
  document.querySelectorAll("#qty input")[2].click();
  snap("clicked l, 3");
  pick($("#qty-select"), "1");
  $("#gift").click();
  pick($("#country"), "de");
  snap("select 1, gift, de");
  root.__irender.set("cities", ["Pune", "Mumbai", "Delhi"]);
  snap("more cities");
  root.__irender.set("sizes", ["xs", "s"]);
  root.__irender.set("size", "s");
  snap("new sizes");
  $("#notes").value = "two\nlines"; $("#notes").dispatchEvent(new Event("input"));
  root.__irender.set("country", "");
  snap("typed, cleared");
  const pre = document.createElement("pre");
  pre.id = "result";
  pre.textContent = out.join("\n");
  document.body.append(pre);
});
</script>"##;

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn form_controls_in_chrome() {
    let engine = Engine::new();
    let ir = engine.load(FORM, Format::Json).unwrap();
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{FORM_DRIVER}</body>"));
    let Some(result) = drive("form", &html) else { return };
    let lines: Vec<&str> = result.lines().collect();

    assert_eq!(lines[0], "initial ; s,m*,l ; 1,2*,3 ; 2 ; false ;  ; Delhi ; Pune,Delhi ; hi ; m|2|number|false||Delhi|hi", "{result}");
    // Radios write their own value; a number-typed key stays a number.
    assert_eq!(lines[1], "clicked l, 3 ; s,m,l* ; 1,2,3* ; 3 ; false ;  ; Delhi ; Pune,Delhi ; hi ; l|3|number|false||Delhi|hi", "{result}");
    // The select and the radios share `qty`.
    assert_eq!(lines[2], "select 1, gift, de ; s,m,l* ; 1*,2,3 ; 1 ; true ; de ; Delhi ; Pune,Delhi ; hi ; l|1|number|true|de|Delhi|hi", "{result}");
    // Options rebuilt from state keep the bound choice.
    assert_eq!(lines[3], "more cities ; s,m,l* ; 1*,2,3 ; 1 ; true ; de ; Delhi ; Pune,Mumbai,Delhi ; hi ; l|1|number|true|de|Delhi|hi", "{result}");
    // Radios rebuilt by a list pick up the current state.
    assert_eq!(lines[4], "new sizes ; xs,s* ; 1*,2,3 ; 1 ; true ; de ; Delhi ; Pune,Mumbai,Delhi ; hi ; s|1|number|true|de|Delhi|hi", "{result}");
    // Clearing the key brings back the placeholder.
    assert_eq!(lines[5], "typed, cleared ; xs,s* ; 1*,2,3 ; 1 ; true ;  ; Delhi ; Pune,Mumbai,Delhi ; two⏎lines ; s|1|number|true||Delhi|two⏎lines", "{result}");
}
