//! The editor and `ui-markdown` in headless Chrome.
//!
//! Ignored by default because it needs Chrome:
//!   cargo test -p irender-markdown-kit --test browser -- --ignored
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

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn editor_and_live_markdown_in_chrome() {
    let Some(chrome) = chrome() else {
        eprintln!("skipping: Chrome not found (set CHROME=/path/to/chrome)");
        return;
    };
    let doc = serde_json::json!({ "version": 1, "state": { "doc": "", "note": "*a*" },
        "root": { "type": "stack", "children": [
            { "type": "ui-markdown-editor", "props": { "bind": "doc", "maxLength": 20 } },
            { "type": "ui-markdown", "props": { "text": "{{ note }}" }, "attrs": { "id": "view" } } ] } });
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    irender_markdown_kit::register(&mut engine.registry);
    let mut doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    irender_ui_kit::apply_theme(&mut doc);
    let ir = engine.resolve(&doc).unwrap();
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = [];
      const frame = () => new Promise((r) => setTimeout(r, 20));
      try {
        const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
        const area = document.querySelector("textarea");
        const tool = (name) => document.querySelector(`[data-md=${name}]`).click();
        const key = (k, mods = {}) => {
          const e = new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true, ...mods });
          area.dispatchEvent(e);
          return e.defaultPrevented;
        };
        const set = (text, at = text.length) => {
          area.focus();
          area.value = text;
          area.dispatchEvent(new Event("input", { bubbles: true }));
          area.setSelectionRange(at, at);
        };
        const show = (label) => out.push(label + " " + JSON.stringify(area.value) + " bind " + JSON.stringify(S().doc));

        set("hello"); area.setSelectionRange(0, 5); tool("bold"); show("bold");
        tool("bold"); show("unbold");
        set("one two"); area.setSelectionRange(4, 7); key("i", { metaKey: true }); show("cmd-i");
        set("- one"); key("Enter"); show("enter");
        set("- one\n- "); key("Enter"); show("empty item");
        set("1. a"); key("Enter"); show("numbers");
        set("- [x] a"); key("Enter"); show("task");
        set("- a\n- b", 6); out.push("tab kept " + key("Tab")); show("tab");
        set("plain"); out.push("tab on prose kept " + key("Tab"));
        set("a\nb"); area.setSelectionRange(0, 3); tool("bullets"); show("bullets");
        tool("bullets"); show("unbullets");
        set("x"); area.setSelectionRange(0, 1); tool("link"); show("link");
        out.push("selected " + JSON.stringify(area.value.slice(area.selectionStart, area.selectionEnd)));

        set("# Title\n\n- a"); await frame();
        document.querySelector("[data-md-view=preview]").click(); await frame();
        const preview = document.querySelector("[data-md-preview]");
        out.push("preview " + document.querySelector("[data-md-body]").dataset.view + " " + JSON.stringify(preview.innerHTML) + " visible " + !!preview.offsetParent + " area " + !!area.offsetParent);
        document.querySelector("[data-md-view=write]").click();

        set("x".repeat(25)); await frame();
        const count = document.querySelector("[data-md-count]");
        out.push("count " + count.textContent + " over " + count.hasAttribute("data-over"));

        const view = document.getElementById("view");
        out.push("view " + JSON.stringify(view.innerHTML));
        root.__irender.set("note", "**b** <i>raw</i>"); await frame();
        out.push("view " + JSON.stringify(view.innerHTML));
      } catch (e) { out.push("EXCEPTION " + e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n");
      document.body.append(pre);
    });
    </script>"##;
    let html = engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"));
    let dir = std::env::temp_dir().join(format!("irender-markdown-kit-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("page.html");
    std::fs::write(&file, html).unwrap();
    // `--no-sandbox`: CI runners block the sandbox's user namespaces; the page is ours.
    let out = Command::new(&chrome)
        .args(["--headless=new", "--no-sandbox", "--disable-gpu", "--blink-settings=preferredColorScheme=1", "--window-size=1000,900", "--virtual-time-budget=5000", "--dump-dom"])
        .arg(format!("file://{}", file.display()))
        .output()
        .unwrap();
    // KEEP=1 keeps the page, to open it by hand.
    if std::env::var_os("KEEP").is_none() {
        let _ = std::fs::remove_dir_all(&dir);
    }
    let dom = String::from_utf8_lossy(&out.stdout);
    let start = dom.find("<pre id=\"result\">").unwrap_or_else(|| panic!("driver did not run:\n{dom}")) + 17;
    let result = dom[start..start + dom[start..].find("</pre>").unwrap()]
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&amp;", "&");
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            r#"bold "**hello**" bind "**hello**""#,
            // The same button takes it off again.
            r#"unbold "hello" bind "hello""#,
            r#"cmd-i "one _two_" bind "one _two_""#,
            // Enter continues a list; an empty item ends it.
            r#"enter "- one\n- " bind "- one\n- ""#,
            r#"empty item "- one\n" bind "- one\n""#,
            r#"numbers "1. a\n2. " bind "1. a\n2. ""#,
            r#"task "- [x] a\n- [ ] " bind "- [x] a\n- [ ] ""#,
            "tab kept true",
            // Tab on the second item nests it under the first.
            r#"tab "- a\n  - b" bind "- a\n  - b""#,
            // Tab outside a list moves focus on, as usual.
            "tab on prose kept false",
            r#"bullets "- a\n- b" bind "- a\n- b""#,
            r#"unbullets "a\nb" bind "a\nb""#,
            r#"link "[x](https://)" bind "[x](https://)""#,
            r#"selected "https://""#,
            r#"preview preview "<h1>Title</h1>\n<ul>\n<li>a</li>\n</ul>" visible true area false"#,
            "count 25/20 over true",
            r#"view "<p><em>a</em></p>""#,
            r#"view "<p><strong>b</strong> &lt;i&gt;raw&lt;/i&gt;</p>""#,
        ],
        "\n{result}"
    );
}
