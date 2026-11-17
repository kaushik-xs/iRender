//! Runs flows in headless Chrome and checks where nodes, ports and edges end up,
//! and what selecting, dragging and changing state do.
//!
//! Ignored by default because it needs Chrome:
//!   cargo test -p irender-flow-kit --test browser -- --ignored
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

/// Renders `doc` with `driver` appended, runs it in Chrome and returns what the
/// driver wrote into `<pre id="result">`, one line per check.
fn run(name: &str, doc: Json, driver: &str) -> Option<Vec<String>> {
    let Some(chrome) = chrome() else {
        eprintln!("skipping: Chrome not found (set CHROME=/path/to/chrome)");
        return None;
    };
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    irender_flow_kit::register(&mut engine.registry);
    let mut doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
    irender_ui_kit::apply_theme(&mut doc);
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    let script = format!(
        "<script>\nconst __errors = []; window.addEventListener('error', (e) => __errors.push(e.message));\n\
         window.addEventListener('load', async () => {{\n{PRELUDE}\n try {{\n{driver}\n }} catch (e) {{ log('threw', e.message, e.stack); }}\n{POSTLUDE}\n}});\n</script>"
    );
    let html = engine.render(&ir, &json!({})).unwrap().document("test").replace("</body>", &format!("{script}</body>"));
    let dir = std::env::temp_dir().join(format!("irender-flow-kit-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("page.html");
    std::fs::write(&file, html).unwrap();
    // `--no-sandbox`: CI runners block the sandbox's user namespaces; the page is ours.
    let out = Command::new(&chrome)
        .args(["--headless=new", "--no-sandbox", "--disable-gpu", "--window-size=1200,900", "--force-prefers-reduced-motion"])
        .args(["--virtual-time-budget=10000", "--dump-dom"])
        .arg(format!("file://{}", file.display()))
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let dom = String::from_utf8_lossy(&out.stdout);
    let start = dom.find("<pre id=\"result\">").unwrap_or_else(|| panic!("driver did not run:\n{dom}")) + 17;
    let result = dom[start..start + dom[start..].find("</pre>").unwrap()]
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&");
    Some(result.lines().map(String::from).collect())
}

const PRELUDE: &str = r##"
  const out = [], log = (...a) => out.push(a.join(" "));
  const wait = (ms) => new Promise((r) => setTimeout(r, ms));
  const root = document.querySelector("[data-i-root]"), S = () => root.__irender.state, set = (k, v) => root.__irender.set(k, v);
  const flow = (id) => document.getElementById(id);
  const node = (f, id) => flow(f).querySelector(`[data-flow-nodes] > [data-flow-node="${id}"]`);
  const edge = (f, id) => flow(f).querySelector(`[data-flow-edges] > svg[data-flow-edge="${id}"]`);
  const label = (f, id) => flow(f).querySelector(`[data-flow-edges] > [data-flow-label="${id}"]`);
  const at = (n) => (n.style.transform.match(/translate\(([-\d.e]+)px, ([-\d.e]+)px\)/) || []).slice(1).map(Number);
  const start = (p) => p.getAttribute("d").match(/^M([-\d.e]+),([-\d.e]+)/).slice(1).map(Number);
  const near = (a, b, d = 1) => Math.abs(a - b) <= d;
  const pointer = (target, type, x, y) =>
    target.dispatchEvent(new PointerEvent(type, { bubbles: true, cancelable: true, pointerId: 1, pointerType: "mouse", button: 0, clientX: x, clientY: y }));
  const click = (target) => { const r = target.getBoundingClientRect(); pointer(target, "pointerdown", r.left + 5, r.top + 5); pointer(target, "pointerup", r.left + 5, r.top + 5); };
  await wait(300);
"##;

const POSTLUDE: &str = r##"
  for (const e of __errors) log("error", e);
  const pre = document.createElement("pre"); pre.id = "result";
  pre.textContent = out.join("\n");
  document.body.append(pre);
"##;

fn check(lines: &[String], expected: &[&str]) {
    let errors: Vec<_> = lines.iter().filter(|l| l.starts_with("error") || l.starts_with("threw")).collect();
    assert!(errors.is_empty(), "page errors: {errors:#?}\nall: {lines:#?}");
    for e in expected {
        assert!(lines.iter().any(|l| l == e), "missing `{e}` in:\n{}", lines.join("\n"));
    }
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn positions_ports_edges_and_selection() {
    let doc = json!({ "version": 1, "state": { "sel": null, "picks": 0 }, "root": { "type": "stack", "children": [
        { "type": "ui-flow", "attrs": { "id": "a" },
          "props": { "fit": false, "viewport": { "x": 0, "y": 0, "zoom": 1 }, "bind": "sel", "controls": false, "background": "none",
            "nodes": [
                { "id": "1", "type": "input", "position": { "x": 0, "y": 0 }, "data": { "label": "One" } },
                { "id": "2", "position": { "x": 300, "y": 100 }, "data": { "label": "Two" }, "className": "extra", "style": { "width": 160 } },
                { "id": "3", "type": "output", "position": { "x": 600, "y": 0 }, "data": { "label": "Three" } },
                { "id": "4", "position": { "x": 0, "y": 300 }, "hidden": true, "data": { "label": "Gone" } } ],
            "edges": [
                { "source": "1", "target": "2", "label": "L" },
                { "id": "x", "source": "2", "target": "3", "type": "step", "markerEnd": { "type": "arrowclosed" } },
                { "id": "y", "source": "2", "target": "4" } ] },
          "on": { "nodeselect": { "action": "set_state", "key": "picks", "value": "{{ picks + 1 }}" } } } ] } });
    let driver = r##"
        const f = "a";
        log("pos2", at(node(f, "2")).join(","));
        log("ports", ["1", "2", "3"].map((id) => node(f, id).querySelectorAll("[data-flow-port]").length).join(","));
        log("class", node(f, "2").classList.contains("extra"), node(f, "2").offsetWidth);
        log("hidden4", node(f, "4").style.display);
        const n1 = node(f, "1"), [sx, sy] = start(edge(f, "1-2").querySelector("path"));
        log("start1-2", near(sx, n1.offsetWidth), near(sy, n1.offsetHeight / 2));
        log("label", label(f, "1-2").textContent, label(f, "1-2").hidden);
        log("marker", /arrowclosed/.test(edge(f, "x").querySelector("path").getAttribute("marker-end")));
        log("step", /L/.test(edge(f, "x").querySelector("path").getAttribute("d")) && !/C/.test(edge(f, "x").querySelector("path").getAttribute("d")));
        log("edgeToHidden", edge(f, "y").hidden);
        click(node(f, "2"));
        log("selected", S().sel, node(f, "2").hasAttribute("data-selected"), S().picks);
        click(flow(f).querySelector("[data-flow-pane]"));
        log("cleared", String(S().sel), node(f, "2").hasAttribute("data-selected"), S().picks);
        set("sel", "3");
        log("fromState", node(f, "3").hasAttribute("data-selected"));
    "##;
    let Some(lines) = run("basic", doc, driver) else { return };
    check(
        &lines,
        &[
            "pos2 300,100",
            "ports 1,2,1",
            "class true 160",
            "hidden4 none",
            "start1-2 true true",
            "label L false",
            "marker true",
            "step true",
            "edgeToHidden true",
            "selected 2 true 1",
            "cleared null false 2",
            "fromState true",
        ],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn templates_layout_state_and_dragging() {
    let task = json!({ "type": "ui-flow-node", "props": { "type": "task" }, "children": [
        { "type": "text", "attrs": { "class": "title" }, "props": { "value": "{{ node.data.label }}" } },
        { "type": "ui-flow-port", "props": { "type": "target" } },
        { "type": "ui-flow-port", "props": { "type": "source" } } ] });
    let branch = json!({ "type": "ui-flow-node", "props": { "type": "if" }, "children": [
        { "type": "text", "props": { "value": "{{ node.data.label }}" } },
        { "type": "ui-flow-port", "props": { "type": "target" } },
        { "type": "ui-flow-port", "props": { "id": "true", "side": "right" } },
        { "type": "ui-flow-port", "props": { "id": "false", "side": "bottom" } } ] });
    let doc = json!({ "version": 1,
        "state": {
            "dir": "LR", "sel": null, "moves": 0,
            "wf": [
                { "id": "a", "type": "task", "data": { "label": "A" } },
                { "id": "b", "type": "if", "data": { "label": "B?" } },
                { "id": "c", "type": "task", "data": { "label": "C" } },
                { "id": "d", "type": "task", "data": { "label": "D" } } ],
            "wfe": [
                { "id": "ab", "source": "a", "target": "b" },
                { "id": "bc", "source": "b", "sourceHandle": "true", "target": "c" },
                { "id": "bd", "source": "b", "sourceHandle": "false", "target": "d" } ] },
        "root": { "type": "ui-flow", "attrs": { "id": "w" },
            "props": { "nodes": "{{ wf }}", "edges": "{{ wfe }}", "layout": "layered", "direction": "{{ dir }}",
                       "bind": "sel", "bindNodes": "wf", "draggable": true, "highlight": "neighbors", "height": 600 },
            "on": { "nodemove": { "action": "set_state", "key": "moves", "value": "{{ moves + 1 }}" } },
            "children": [task, branch] } });
    let driver = r##"
        const f = "w";
        const ga = at(node(f, "a")), gb = at(node(f, "b")), gc = at(node(f, "c"));
        log("ranksLR", ga[0] < gb[0] && gb[0] < gc[0]);
        log("template", node(f, "a").querySelector(".title").textContent, node(f, "b").querySelectorAll("[data-flow-port]").length);
        const b = node(f, "b"), [bx, by] = start(edge(f, "bd").querySelector("path"));
        const [tx, ty] = start(edge(f, "bc").querySelector("path"));
        log("falseFromBottom", near(by, gb[1] + b.offsetHeight), near(bx, gb[0] + b.offsetWidth / 2));
        log("trueFromRight", near(tx, gb[0] + b.offsetWidth), near(ty, gb[1] + b.offsetHeight / 2));

        // A node added in state is rebuilt in; the others keep their elements.
        const keep = node(f, "a");
        set("wfe", [...S().wfe, { id: "ce", source: "c", target: "e" }]);
        set("wf", [...S().wf, { id: "e", type: "task", data: { label: "E" } }]);
        await wait(100);
        log("kept", node(f, "a") === keep, !!node(f, "e"), edge(f, "ce").hidden, at(node(f, "e"))[0] > at(node(f, "c"))[0]);

        // Highlighting the selection's neighbours.
        click(node(f, "a"));
        log("highlight", S().sel, node(f, "c").hasAttribute("data-dimmed"), node(f, "b").hasAttribute("data-dimmed"),
            edge(f, "ab").querySelector("path").hasAttribute("data-highlighted"), edge(f, "bc").hasAttribute("data-dimmed"));

        // Dragging writes the new position to `bindNodes`.
        const n = node(f, "d"), before = at(n), r = n.getBoundingClientRect();
        pointer(n, "pointerdown", r.left + 5, r.top + 5);
        pointer(n, "pointermove", r.left + 45, r.top + 35);
        pointer(n, "pointermove", r.left + 85, r.top + 65);
        pointer(n, "pointerup", r.left + 85, r.top + 65);
        const p = S().wf.find((x) => x.id === "d").position;
        log("dragged", !!p, at(n)[0] > before[0], S().moves, S().sel);

        // Top to bottom.
        set("dir", "TB");
        await wait(100);
        const ta = at(node(f, "a")), tb = at(node(f, "b"));
        log("ranksTB", ta[1] < tb[1]);
        const [cx, cy] = start(edge(f, "ab").querySelector("path"));
        log("portsFollowDirection", near(cy, ta[1] + node(f, "a").offsetHeight));
    "##;
    let Some(lines) = run("templates", doc, driver) else { return };
    check(
        &lines,
        &[
            "ranksLR true",
            "template A 3",
            "falseFromBottom true true",
            "trueFromRight true true",
            "kept true true false true",
            "highlight a true false true true",
            "dragged true true 1 a",
            "ranksTB true",
            "portsFollowDirection true",
        ],
    );
}
