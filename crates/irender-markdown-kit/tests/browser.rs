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

/// Opens `html` in headless Chrome and returns what its driver wrote to `#result`.
fn run(chrome: &PathBuf, html: &str) -> String {
    // One folder per page: the tests run at the same time.
    static RUNS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = RUNS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("irender-markdown-kit-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("page.html");
    std::fs::write(&file, html).unwrap();
    // `--no-sandbox`: CI runners block the sandbox's user namespaces; the page is ours.
    let out = Command::new(chrome)
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
    dom[start..start + dom[start..].find("</pre>").unwrap()]
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&amp;", "&")
}

/// The page for `doc`, with the UI kit's theme and `driver` before `</body>`.
fn page(doc: serde_json::Value, driver: &str) -> String {
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    irender_markdown_kit::register(&mut engine.registry);
    let mut doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    irender_ui_kit::apply_theme(&mut doc);
    let ir = engine.resolve(&doc).unwrap();
    engine.render(&ir, &serde_json::json!({})).unwrap().document("test").replace("</body>", &format!("{driver}</body>"))
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
            { "type": "ui-markdown-editor", "props": { "bind": "doc", "mode": "tabs", "maxLength": 20 } },
            { "type": "ui-markdown", "props": { "text": "{{ note }}" }, "attrs": { "id": "view" } } ] } });
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
    let result = run(&chrome, &page(doc, driver));
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

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn rich_text_editor_in_chrome() {
    let Some(chrome) = chrome() else {
        eprintln!("skipping: Chrome not found (set CHROME=/path/to/chrome)");
        return;
    };
    let doc = serde_json::json!({ "version": 1, "state": { "doc": "hello world" },
        "root": { "type": "ui-markdown-editor", "props": { "bind": "doc", "placeholder": "Write…" } } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = [];
      const frame = () => new Promise((r) => setTimeout(r, 20));
      try {
        const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
        const rich = document.querySelector("[data-md-rich]");
        const area = document.querySelector("textarea");
        const tool = (name) => document.querySelector(`[data-md=${name}]`).click();
        const key = (k, mods = {}, target = rich) => {
          const e = new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true, ...mods });
          target.dispatchEvent(e);
          if (!e.defaultPrevented && k === "Enter") document.execCommand("insertParagraph");
          if (!e.defaultPrevented && k === " ") document.execCommand("insertText", false, " ");
        };
        const type = (t) => { for (const c of t) c === " " ? key(" ") : document.execCommand("insertText", false, c); };
        // Selects `text` where it first appears.
        const pick = (text) => {
          const walk = document.createTreeWalker(rich, NodeFilter.SHOW_TEXT);
          while (walk.nextNode()) {
            const i = walk.currentNode.data.indexOf(text);
            if (i >= 0) {
              const r = document.createRange();
              r.setStart(walk.currentNode, i); r.setEnd(walk.currentNode, i + text.length);
              getSelection().removeAllRanges(); getSelection().addRange(r);
              return;
            }
          }
          throw new Error("no " + text);
        };
        const reset = async (md) => {
          root.__irender.set("doc", md); await frame();
          rich.focus();
          const r = document.createRange(); r.selectNodeContents(rich.lastElementChild || rich); r.collapse(false);
          getSelection().removeAllRanges(); getSelection().addRange(r);
        };
        const show = (label) => out.push(label + " " + JSON.stringify(S().doc));

        rich.focus(); pick("world"); tool("bold"); show("bold");
        await frame();
        out.push("pressed " + document.querySelector("[data-md=bold]").getAttribute("aria-pressed"));
        key("i", { metaKey: true }); show("cmd-i");

        await reset(""); out.push("empty " + rich.hasAttribute("data-empty"));
        type("# Title"); key("Enter"); type("- a"); key("Enter"); type("b"); show("typed");
        tool("tasks"); show("tasks");
        rich.querySelector("li input").click(); show("ticked");

        await reset("go here"); pick("here"); tool("link");
        const bar = document.querySelector("[data-md-linkbar]"), url = bar.querySelector("input");
        out.push("bar " + !bar.hidden + " focused " + (document.activeElement === url));
        url.value = "example.com"; key("Enter", {}, url); show("link");
        out.push("bar " + !bar.hidden);

        await reset("");
        // Typed without the space shortcut, as a pasted line would be: kept as text.
        document.execCommand("insertText", false, "1. not a list *really*"); show("escaped");

        await reset("q"); tool("quote"); show("quote");
        key("Enter"); key("Enter"); type("after"); show("left quote");

        await reset(""); type("``` let x"); key("Enter"); type("y"); key("Enter"); key("Enter"); type("z"); show("code");

        await reset("");
        // A stand-in clipboard: Firefox keeps a script's DataTransfer empty.
        const data = { "text/html": '<h1 style="color:red">T</h1><p>Some <b>bold</b> <span style="font-style:italic">it</span><script>bad()<\/script></p><ul><li>x</li></ul>', "text/plain": "T Some bold" };
        const paste = new Event("paste", { bubbles: true, cancelable: true });
        Object.defineProperty(paste, "clipboardData", { value: { getData: (t) => data[t] ?? "" } });
        rich.dispatchEvent(paste); show("paste");

        root.__irender.set("doc", "**x**"); await frame();
        out.push("follows " + JSON.stringify(rich.innerHTML));

        document.querySelector("[data-md-view=source]").click(); await frame();
        out.push("source " + !!area.offsetParent + " rich " + !!rich.offsetParent + " " + JSON.stringify(area.value));
        area.setSelectionRange(2, 3); tool("bold"); show("source bold");
        document.querySelector("[data-md-view=rich]").click(); await frame();
        out.push("back " + JSON.stringify(rich.innerHTML) + " rich " + !!rich.offsetParent);
      } catch (e) { out.push("EXCEPTION " + e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n");
      document.body.append(pre);
    });
    </script>"##;
    let result = run(&chrome, &page(doc, driver));
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            r##"bold "hello **world**""##,
            // The button shows where the caret is bold.
            "pressed true",
            r##"cmd-i "hello **_world_**""##,
            "empty true",
            // `#`, `-` and a space format the line as it's typed.
            r##"typed "# Title\n\n- a\n- b""##,
            r##"tasks "# Title\n\n- [ ] a\n- [ ] b""##,
            r##"ticked "# Title\n\n- [x] a\n- [ ] b""##,
            "bar true focused true",
            // A bare domain is a web address.
            r##"link "go [here](https://example.com)""##,
            "bar false",
            // Text that would read as Markdown stays text.
            r##"escaped "1\\. not a list \\*really\\*""##,
            r##"quote "> q""##,
            // Enter on an empty line leaves the quote, and the code block.
            r##"left quote "> q\n\nafter""##,
            r##"code "```\nlet x\ny\n```\n\nz""##,
            // A pasted page keeps what Markdown holds; the script and colour go.
            r##"paste "# T\n\nSome **bold** _it_\n\n- x""##,
            r##"follows "<p><strong>x</strong></p>""##,
            r##"source true rich false "**x**""##,
            r##"source bold "x""##,
            r##"back "<p>x</p>" rich true"##,
        ],
        "\n{result}"
    );
}
