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

/// Table, Image and Rule: in rich text a table to fill in (Tab between its cells, Tab
/// past the last adding a row, the button in a table adding one under the current row),
/// an image from the bar under the toolbar, and a rule; in the Markdown tab, their
/// Markdown, with the part to type over selected.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn rich_text_tables_images_and_rules_in_chrome() {
    let Some(chrome) = chrome() else {
        eprintln!("skipping: Chrome not found (set CHROME=/path/to/chrome)");
        return;
    };
    let doc = serde_json::json!({ "version": 1, "state": { "doc": "Intro" },
        "root": { "type": "ui-markdown-editor", "props": { "bind": "doc" } } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = [];
      const frame = () => new Promise((r) => setTimeout(r, 20));
      try {
        const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
        const rich = document.querySelector("[data-md-rich]");
        const area = document.querySelector("textarea");
        const tool = (name) => document.querySelector(`[data-md=${name}]`).click();
        const key = (k, mods = {}, target = rich) => target.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true, ...mods }));
        const type = (t) => document.execCommand("insertText", false, t);
        const show = (label) => out.push(label + " " + JSON.stringify(S().doc));
        const end = () => {
          rich.focus();
          const r = document.createRange(); r.selectNodeContents(rich.lastElementChild || rich); r.collapse(false);
          getSelection().removeAllRanges(); getSelection().addRange(r);
        };
        end();
        tool("table"); await frame();
        out.push("table " + rich.querySelectorAll("table tr").length + " selected " + JSON.stringify(getSelection().toString()) + " pressed " + document.querySelector("[data-md=table]").getAttribute("aria-pressed"));
        type("Name"); key("Tab"); type("Role"); key("Tab"); type("Team");
        key("Tab"); type("Asha"); key("Tab", { shiftKey: true }); out.push("back " + JSON.stringify(getSelection().toString()));
        // Tab from the last cell adds a row.
        const cells = rich.querySelectorAll("td");
        const r = document.createRange(); r.selectNodeContents(cells[cells.length - 1]); r.collapse(false);
        getSelection().removeAllRanges(); getSelection().addRange(r);
        key("Tab"); await frame();
        out.push("rows " + rich.querySelectorAll("tbody tr").length);
        // The button inside a table: a row under the current one.
        tool("table"); await frame();
        out.push("rows " + rich.querySelectorAll("tbody tr").length);
        show("filled");

        end(); tool("rule"); type("After"); await frame(); show("rule");

        end(); tool("image");
        const bar = document.querySelector("[data-md-imagebar]");
        out.push("bar " + !bar.hidden + " focused " + (document.activeElement === bar.querySelector("[data-md-image-src]")));
        bar.querySelector("[data-md-image-src]").value = "example.com/a.png";
        bar.querySelector("[data-md-image-alt]").value = "A chart";
        bar.querySelector("[data-md-image=insert]").click(); await frame();
        out.push("bar " + !bar.hidden);
        // After "After" (Chrome, WebKit) or in a paragraph of its own (Firefox).
        out.push("image " + /After(\n\n)?!\[A chart\]\(https:\/\/example\.com\/a\.png\)$/.test(S().doc));
        // A script address isn't an image.
        end(); tool("image"); bar.querySelector("[data-md-image-src]").value = "javascript:alert(1)";
        key("Enter", {}, bar.querySelector("[data-md-image-src]")); await frame();
        out.push("script " + rich.querySelectorAll("img").length + " bar " + !bar.hidden);

        // The Markdown tab: the same tools write Markdown.
        root.__irender.set("doc", "Intro"); await frame();
        document.querySelector("[data-md-view=source]").click(); await frame();
        area.setSelectionRange(5, 5); tool("table");
        out.push("md table " + JSON.stringify(area.value) + " selected " + JSON.stringify(area.value.slice(area.selectionStart, area.selectionEnd)));
        area.setSelectionRange(area.value.length, area.value.length); tool("rule");
        tool("image");
        out.push("md image " + JSON.stringify(area.value.slice(area.value.lastIndexOf("\n") + 1)) + " selected " + JSON.stringify(area.value.slice(area.selectionStart, area.selectionEnd)));
      } catch (e) { out.push("EXCEPTION " + e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n");
      document.body.append(pre);
    });
    </script>"##;
    let result = run(&chrome, &page(doc, driver));
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            // A header row and two rows, the first header's text selected to type over.
            r##"table 3 selected "Column 1" pressed true"##,
            r##"back "Team""##,
            // Tab past the last cell adds a row; so does the button, inside the table.
            "rows 3",
            "rows 4",
            r##"filled "Intro\n\n| Name | Role | Team |\n| --- | --- | --- |\n| Asha |  |  |\n|  |  |  |\n|  |  |  |\n|  |  |  |""##,
            r##"rule "Intro\n\n| Name | Role | Team |\n| --- | --- | --- |\n| Asha |  |  |\n|  |  |  |\n|  |  |  |\n|  |  |  |\n\n---\n\nAfter""##,
            "bar true focused true",
            "bar false",
            // A bare domain is a web address.
            "image true",
            "script 1 bar false",
            r##"md table "Intro\n\n| Column 1 | Column 2 | Column 3 |\n| --- | --- | --- |\n|  |  |  |\n|  |  |  |\n\n" selected "Column 1""##,
            // After the rule, on a line of its own; the address selected to type over.
            r##"md image "![image description](https://)" selected "https://""##,
        ],
        "\n{result}"
    );
}

/// Undo and redo in rich text take back every change, including those made to the page
/// directly: quoting, leaving a quote or a code block, ticking a task and adding a row.
/// Typing in a row is one step, and the browser's own undo uses the same history.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn rich_text_undo_and_redo_in_chrome() {
    let Some(chrome) = chrome() else {
        eprintln!("skipping: Chrome not found (set CHROME=/path/to/chrome)");
        return;
    };
    let doc = serde_json::json!({ "version": 1, "state": { "doc": "Intro" },
        "root": { "type": "ui-markdown-editor", "props": { "bind": "doc" } } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = [];
      const frame = () => new Promise((r) => setTimeout(r, 20));
      try {
        const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
        const rich = document.querySelector("[data-md-rich]");
        const tool = (name) => document.querySelector(`[data-md=${name}]`).click();
        const key = (k, mods = {}) => rich.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true, ...mods }));
        const mac = /Mac/.test(navigator.platform);
        const undo = () => key("z", mac ? { metaKey: true } : { ctrlKey: true });
        const redo = () => key("z", { shiftKey: true, ...(mac ? { metaKey: true } : { ctrlKey: true }) });
        const enter = () => { if (key("Enter")) document.execCommand("insertParagraph"); };
        const type = (t) => document.execCommand("insertText", false, t);
        const show = (label) => out.push(label + " " + JSON.stringify(S().doc));
        const end = (n = rich.lastElementChild) => {
          rich.focus();
          const r = document.createRange(); r.selectNodeContents(n); r.collapse(false);
          getSelection().removeAllRanges(); getSelection().addRange(r);
        };
        const load = async (text) => { root.__irender.set("doc", text); await frame(); };

        // Typing in a row is one step; quoting is another.
        end(); type(" one"); type(" two"); tool("quote"); await frame(); show("quoted");
        undo(); show("undo"); out.push("caret " + JSON.stringify(getSelection().anchorNode?.textContent) + " in " + rich.contains(getSelection().anchorNode));
        redo(); show("redo");
        undo(); undo(); show("typing undone");
        redo(); show("typing redone");

        await load("> a"); end(rich.querySelector("blockquote p"));
        enter(); enter(); type("b"); await frame(); show("left quote");
        undo(); undo(); show("undo");

        await load("```\ncode\n```"); end(rich.querySelector("code"));
        enter(); enter(); await frame(); show("left code");
        undo(); show("undo");

        await load("- [ ] task");
        rich.querySelector("input[type=checkbox]").click(); await frame(); show("ticked");
        undo(); out.push("undo " + JSON.stringify(S().doc) + " box " + rich.querySelector("input[type=checkbox]").checked);
        redo(); out.push("redo " + JSON.stringify(S().doc) + " box " + rich.querySelector("input[type=checkbox]").checked);

        await load("| a | b |\n| --- | --- |\n| 1 | 2 |");
        const cells = rich.querySelectorAll("td"); end(cells[cells.length - 1]);
        key("Tab"); await frame(); out.push("rows " + rich.querySelectorAll("tbody tr").length);
        undo(); out.push("undo rows " + rich.querySelectorAll("tbody tr").length);

        // The browser's own undo (the Edit menu's) goes to the same history.
        await load("Plain"); end(); tool("quote"); await frame();
        document.execCommand("undo"); await frame(); show("browser undo");
      } catch (e) { out.push("EXCEPTION " + e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n");
      document.body.append(pre);
    });
    </script>"##;
    let result = run(&chrome, &page(doc, driver));
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            r##"quoted "> Intro one two""##,
            // The caret goes back into the text.
            r##"undo "Intro one two""##,
            r##"caret "Intro one two" in true"##,
            r##"redo "> Intro one two""##,
            r##"typing undone "Intro""##,
            r##"typing redone "Intro one two""##,
            // Undoing the "b", then leaving the quote.
            r##"left quote "> a\n\nb""##,
            r##"undo "> a""##,
            // Back in the code block, on its new line.
            r##"left code "```\ncode\n```""##,
            r##"undo "```\ncode\n\n```""##,
            r##"ticked "- [x] task""##,
            r##"undo "- [ ] task" box false"##,
            r##"redo "- [x] task" box true"##,
            "rows 2",
            "undo rows 1",
            r##"browser undo "Plain""##,
        ],
        "\n{result}"
    );
}

/// Chat messages show their content as Markdown when the kit is registered: rendered on
/// the server, and redrawn as a reply streams into state.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn chat_messages_as_markdown_in_chrome() {
    let Some(chrome) = chrome() else { return };
    let doc = serde_json::json!({ "version": 1,
        "state": { "chat": [{ "id": 1, "role": "user", "content": "See [the docs](https://example.com)" }] },
        "root": { "type": "stack", "children": [
            { "type": "ui-message", "props": { "role": "assistant", "content": "**Bold** and `code`\n\n- one\n- two" } },
            { "type": "ui-message", "props": { "role": "assistant", "content": "*plain* stays", "markdown": false } },
            { "type": "ui-conversation", "props": { "messages": "{{ chat }}" } } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
      const bubbles = () => [...document.querySelectorAll("[data-bubble]")];
      const shown = (b) => getComputedStyle(b).display !== "none";
      try {
        const [first, plain] = bubbles();
        log("server", first.querySelector("[data-markdown]").innerHTML.replace(/\n/g, ""));
        log("plain", plain.querySelector("[data-markdown]") === null, plain.textContent);
        const log1 = document.querySelector("[role=log]");
        const user = log1.querySelector("[data-bubble]");
        log("user link", user.querySelector("a")?.getAttribute("href"), getComputedStyle(user.querySelector("a")).color === getComputedStyle(user).color);
        // A reply streams in: empty, then a chunk at a time.
        root.__irender.set("chat", [...S().chat, { id: 2, role: "assistant", content: "" }]); await wait(10);
        const reply = () => log1.querySelectorAll("[data-bubble]")[1];
        log("empty reply hidden", !shown(reply()));
        for (const chunk of ["Here is ", "**the plan**:\n\n", "1. Draft\n", "2. Ship"]) {
          const list = S().chat;
          root.__irender.set("chat", [...list.slice(0, -1), { ...list.at(-1), content: list.at(-1).content + chunk }]);
          await wait(10);
        }
        log("streamed", shown(reply()), reply().querySelector("[data-markdown]").innerHTML.replace(/\n/g, ""));
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let result = run(&chrome, &page(doc, driver));
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "server <p><strong>Bold</strong> and <code>code</code></p><ul><li>one</li><li>two</li></ul>",
            "plain true *plain* stays",
            "user link https://example.com true",
            "empty reply hidden true",
            "streamed true <p>Here is <strong>the plan</strong>:</p><ol><li>Draft</li><li>Ship</li></ol>",
        ],
        "\n{result}"
    );
}

/// Code fences: highlighted, with a copy button, in fixed Markdown, Markdown from state,
/// the editor's preview and a chat message.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn code_fences_in_chrome() {
    let Some(chrome) = chrome() else { return };
    let doc = serde_json::json!({ "version": 1, "state": { "note": "none yet", "doc": "" },
        "root": { "type": "stack", "children": [
            { "type": "ui-markdown", "attrs": { "id": "fixed" }, "props": { "text": "Run:\n\n```bash\nirender --version  # prints it\n```\n\n```\nplain <b>\n```" } },
            { "type": "ui-markdown", "attrs": { "id": "live" }, "props": { "text": "{{ note }}" } },
            { "type": "ui-markdown-editor", "props": { "bind": "doc", "mode": "split" } },
            { "type": "ui-message", "attrs": { "id": "msg" }, "props": { "role": "assistant", "content": "Try:\n\n```sql\nSELECT 1;\n```" } } ] } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = []; const log = (...a) => out.push(a.join(" "));
      const wait = (ms) => new Promise((r) => setTimeout(r, ms));
      const root = document.querySelector("[data-i-root]");
      const toks = (el) => [...el.querySelectorAll("[data-t]")].map((t) => t.dataset.t + ":" + t.textContent).join(" ");
      const fences = (el) => [...el.querySelectorAll("[data-fence]")].map((f) => `[${f.dataset.fence || "-"}] ${f.querySelector("code").textContent} | ${f.querySelector("[data-fence-copy]").getAttribute("aria-label")}`).join(" ; ");
      try {
        const fixed = document.querySelector("#fixed");
        log("fixed", toks(fixed));
        log("fences", fences(fixed));
        log("escaped", fixed.querySelectorAll("b").length);
        // Copying, with the clipboard stubbed.
        let copied = null;
        Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText: async (t) => { copied = t; } } });
        const btn = fixed.querySelector("[data-fence-copy]");
        btn.click(); await wait(0); await wait(0);
        log("copy", JSON.stringify(copied), btn.hasAttribute("data-copied"), fixed.querySelector("[data-fence-live]").textContent);
        // From state: highlighted after each change.
        const live = document.querySelector("#live");
        log("live before", live.querySelectorAll("[data-fence]").length);
        root.__irender.set("note", "```js\nconst a = 1;\n```"); await wait(0);
        log("live", toks(live), fences(live));
        root.__irender.set("note", "```python\nprint('x')\n```"); await wait(0);
        log("live again", toks(live), live.querySelectorAll("[data-fence]").length);
        // The editor's preview.
        const area = document.querySelector("[data-md-editor] textarea");
        area.value = "```json\n{\"a\": true}\n```"; area.dispatchEvent(new Event("input", { bubbles: true })); await wait(0);
        log("preview", toks(document.querySelector("[data-md-preview]")));
        // A chat message.
        log("message", toks(document.querySelector("#msg")), fences(document.querySelector("#msg")));
      } catch (e) { log("EXCEPTION", e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n"); document.body.append(pre);
    });
    </script>"##;
    let result = run(&chrome, &page(doc, driver));
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            "fixed f:irender n:--version c:# prints it",
            "fences [bash] irender --version  # prints it | Copy bash code ; [-] plain <b> | Copy code",
            "escaped 0",
            "copy \"irender --version  # prints it\" true Copied to the clipboard",
            "live before 0",
            "live k:const n:1 [js] const a = 1; | Copy js code",
            "live again f:print s:'x' 1",
            "preview p:\"a\" l:true",
            "message k:SELECT n:1 [sql] SELECT 1; | Copy sql code",
        ],
        "\n{result}"
    );
}

/// `maxLength` in rich text: an edit that takes the Markdown past it is taken back, the
/// caret staying where it was; text already over it can still be cut; a paste is cut to
/// what fits.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn rich_text_max_length_in_chrome() {
    let Some(chrome) = chrome() else {
        eprintln!("skipping: Chrome not found (set CHROME=/path/to/chrome)");
        return;
    };
    let doc = serde_json::json!({ "version": 1, "state": { "doc": "Hello" },
        "root": { "type": "ui-markdown-editor", "props": { "bind": "doc", "maxLength": 12 } } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = [];
      const frame = () => new Promise((r) => setTimeout(r, 20));
      try {
        const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
        const rich = document.querySelector("[data-md-rich]");
        const count = document.querySelector("[data-md-count]");
        const type = (t) => document.execCommand("insertText", false, t);
        const show = (label) => out.push(label + " " + JSON.stringify(S().doc) + " " + count.textContent + (count.hasAttribute("data-over") ? " over" : ""));
        const end = () => {
          rich.focus();
          const r = document.createRange(); r.selectNodeContents(rich.lastElementChild); r.collapse(false);
          getSelection().removeAllRanges(); getSelection().addRange(r);
        };
        // Firefox leaves a made-up paste event's clipboard empty.
        const paste = (text, html = "") => {
          const e = new Event("paste", { bubbles: true, cancelable: true });
          Object.defineProperty(e, "clipboardData", { value: { getData: (t) => ({ "text/plain": text, "text/html": html })[t] ?? "" } });
          rich.dispatchEvent(e);
        };
        end(); await frame(); type(" there, world"); await frame(); show("too long");
        type(" you"); await frame(); show("fits");
        type("!!!"); await frame(); show("full");
        type("?"); await frame(); show("past");
        // Bold adds four characters of Markdown.
        const r = document.createRange(); r.setStart(rich.querySelector("p").firstChild, 0); r.setEnd(rich.querySelector("p").firstChild, 5);
        getSelection().removeAllRanges(); getSelection().addRange(r);
        document.querySelector("[data-md=bold]").click(); await frame(); show("bold");
        out.push("selected " + JSON.stringify(getSelection().toString()));

        root.__irender.set("doc", ""); await frame();
        end(); paste("wonderful world today"); await frame(); show("paste");
        // Formatted text that fits keeps its formatting; past the limit, its text is cut.
        root.__irender.set("doc", ""); await frame();
        end(); paste("big news", "<strong>big</strong> news"); await frame(); show("html paste");
        root.__irender.set("doc", ""); await frame();
        end(); paste("big news today", "<p><strong>big</strong> news today</p>"); await frame(); show("long html paste");

        // Over the limit already: cutting is fine, adding isn't.
        root.__irender.set("doc", "Far too long already"); await frame(); show("loaded");
        end(); document.execCommand("delete"); await frame(); show("cut");
        type("y"); await frame(); show("added");
      } catch (e) { out.push("EXCEPTION " + e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n");
      document.body.append(pre);
    });
    </script>"##;
    let result = run(&chrome, &page(doc, driver));
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            r##"too long "Hello" 5/12"##,
            r##"fits "Hello you" 9/12"##,
            r##"full "Hello you!!!" 12/12"##,
            r##"past "Hello you!!!" 12/12"##,
            // Bold would make it 16; the selection stays.
            r##"bold "Hello you!!!" 12/12"##,
            r##"selected "Hello""##,
            r##"paste "wonderful wo" 12/12"##,
            r##"html paste "**big** news" 12/12"##,
            r##"long html paste "big news tod" 12/12"##,
            r##"loaded "Far too long already" 20/12 over"##,
            r##"cut "Far too long alread" 19/12 over"##,
            r##"added "Far too long alread" 19/12 over"##,
        ],
        "\n{result}"
    );
}

/// Headings of every level, from the button (## → ### → text) and ⌘⌥1–6 / ⌘⌥0, in rich
/// text and the Markdown tab; and pasted addresses: over text they link it, alone they
/// are a link written `<…>`, and a pasted line keeps the spaces at its ends.
#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn rich_text_headings_and_pasted_links_in_chrome() {
    let Some(chrome) = chrome() else {
        eprintln!("skipping: Chrome not found (set CHROME=/path/to/chrome)");
        return;
    };
    let doc = serde_json::json!({ "version": 1, "state": { "doc": "Title" },
        "root": { "type": "ui-markdown-editor", "props": { "bind": "doc" } } });
    let driver = r##"<script>
    window.addEventListener("load", async () => {
      const out = [];
      const frame = () => new Promise((r) => setTimeout(r, 20));
      try {
        const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state;
        const rich = document.querySelector("[data-md-rich]");
        const area = document.querySelector("textarea");
        const tool = (name) => document.querySelector(`[data-md=${name}]`).click();
        const mac = /Mac/.test(navigator.platform);
        const level = (n, target = rich) => target.dispatchEvent(new KeyboardEvent("keydown", { key: "x", code: "Digit" + n, altKey: true, bubbles: true, cancelable: true, ...(mac ? { metaKey: true } : { ctrlKey: true }) }));
        const show = (label) => out.push(label + " " + JSON.stringify(S().doc));
        const paste = (text) => {
          const e = new Event("paste", { bubbles: true, cancelable: true });
          Object.defineProperty(e, "clipboardData", { value: { getData: (t) => (t === "text/plain" ? text : "") } });
          rich.dispatchEvent(e);
        };
        const end = () => {
          rich.focus();
          const r = document.createRange(); r.selectNodeContents(rich.lastElementChild); r.collapse(false);
          getSelection().removeAllRanges(); getSelection().addRange(r);
        };
        end();
        tool("heading"); await frame(); show("button");
        tool("heading"); await frame(); show("again");
        tool("heading"); await frame(); show("off");
        level(4); await frame(); show("level 4");
        out.push("pressed " + document.querySelector("[data-md=heading]").getAttribute("aria-pressed"));
        tool("heading"); await frame(); show("button on h4");
        level(1); await frame(); level(0); await frame(); show("level 0");
        out.push("title " + document.querySelector("[data-md=heading]").title);

        // Pasted addresses.
        root.__irender.set("doc", "Read the docs"); await frame();
        const t = rich.querySelector("p").firstChild;
        const r = document.createRange(); r.setStart(t, 9); r.setEnd(t, 13);
        getSelection().removeAllRanges(); getSelection().addRange(r);
        paste("https://example.com/docs"); await frame(); show("over text");
        end(); paste(" or https://example.com/faq "); await frame(); paste("now"); await frame(); show("alone");
        out.push("links " + rich.querySelectorAll("a").length);

        // The Markdown tab.
        root.__irender.set("doc", "One\nTwo"); await frame();
        document.querySelector("[data-md-view=source]").click(); await frame();
        area.setSelectionRange(0, 7);
        tool("heading"); out.push("md button " + JSON.stringify(area.value));
        tool("heading"); out.push("md again " + JSON.stringify(area.value));
        tool("heading"); out.push("md off " + JSON.stringify(area.value));
        area.setSelectionRange(0, 0); level(5, area); out.push("md level 5 " + JSON.stringify(area.value));
        level(0, area); out.push("md level 0 " + JSON.stringify(area.value));
      } catch (e) { out.push("EXCEPTION " + e.stack); }
      const pre = document.createElement("pre"); pre.id = "result"; pre.textContent = out.join("\n");
      document.body.append(pre);
    });
    </script>"##;
    let result = run(&chrome, &page(doc, driver));
    assert_eq!(
        result.lines().collect::<Vec<_>>(),
        [
            r######"button "## Title""######,
            r######"again "### Title""######,
            r######"off "Title""######,
            r######"level 4 "#### Title""######,
            "pressed true",
            // On another level, the button takes the heading off.
            r######"button on h4 "Title""######,
            r######"level 0 "Title""######,
            "title Heading (⌘⌥1–6)",
            r######"over text "Read the [docs](https://example.com/docs)""######,
            // The spaces around the pasted line stay.
            r######"alone "Read the [docs](https://example.com/docs) or <https://example.com/faq> now""######,
            "links 2",
            r######"md button "## One\n## Two""######,
            r######"md again "### One\n### Two""######,
            r######"md off "One\nTwo""######,
            r######"md level 5 "##### One\nTwo""######,
            r######"md level 0 "One\nTwo""######,
        ],
        "\n{result}"
    );
}
