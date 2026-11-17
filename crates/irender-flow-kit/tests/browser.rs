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
    // Ports from a `list` (whose wrapper isn't positioned) still share the side.
    let merge = json!({ "type": "ui-flow-node", "props": { "type": "merge" }, "children": [
        { "type": "text", "props": { "value": "Merge" } },
        { "type": "list", "props": { "items": "{{ range(node.data.inputCount) }}", "as": "i" }, "children": [
            { "type": "ui-flow-port", "props": { "id": "input-{{ i }}", "type": "target" } } ] } ] });
    let doc = json!({ "version": 1,
        "state": {
            "dir": "LR", "sel": null, "moves": 0,
            "wf": [
                { "id": "a", "type": "task", "data": { "label": "A" } },
                { "id": "b", "type": "if", "data": { "label": "B?" } },
                { "id": "c", "type": "task", "data": { "label": "C" } },
                { "id": "d", "type": "task", "data": { "label": "D" } },
                { "id": "m", "type": "merge", "data": { "inputCount": 3 } } ],
            "wfe": [
                { "id": "ab", "source": "a", "target": "b" },
                { "id": "bc", "source": "b", "sourceHandle": "true", "target": "c" },
                { "id": "bd", "source": "b", "sourceHandle": "false", "target": "d" } ] },
        "root": { "type": "ui-flow", "attrs": { "id": "w" },
            "props": { "nodes": "{{ wf }}", "edges": "{{ wfe }}", "layout": "layered", "direction": "{{ dir }}",
                       "bind": "sel", "bindNodes": "wf", "draggable": true, "highlight": "neighbors", "height": 600 },
            "on": { "nodemove": { "action": "set_state", "key": "moves", "value": "{{ moves + 1 }}" } },
            "children": [task, branch, merge] } });
    let driver = r##"
        const f = "w";
        const ga = at(node(f, "a")), gb = at(node(f, "b")), gc = at(node(f, "c"));
        log("ranksLR", ga[0] < gb[0] && gb[0] < gc[0]);
        log("template", node(f, "a").querySelector(".title").textContent, node(f, "b").querySelectorAll("[data-flow-port]").length);
        const b = node(f, "b"), [bx, by] = start(edge(f, "bd").querySelector("path"));
        const [tx, ty] = start(edge(f, "bc").querySelector("path"));
        log("falseFromBottom", near(by, gb[1] + b.offsetHeight), near(bx, gb[0] + b.offsetWidth / 2));
        const tops = [...node(f, "m").querySelectorAll("[data-flow-port]")].map((p) => Math.round(p.getBoundingClientRect().top));
        log("rangePorts", tops.length, new Set(tops).size, tops[0] < tops[1] && tops[1] < tops[2]);
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
            "rangePorts 3 3 true",
            "trueFromRight true true",
            "kept true true false true",
            "highlight a true false true true",
            "dragged true true 1 d",
            "ranksTB true",
            "portsFollowDirection true",
        ],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn editing() {
    let step = json!({ "type": "ui-flow-node", "props": { "type": "step" }, "children": [
        { "type": "text", "props": { "value": "{{ node.data.label }}" } },
        { "type": "ui-flow-port", "props": { "type": "target", "accepts": ["step", "start"] } },
        { "type": "ui-flow-port", "props": { "type": "source", "max": 2 } } ] });
    let start = json!({ "type": "ui-flow-node", "props": { "type": "start" }, "children": [
        { "type": "text", "props": { "value": "Start" } },
        { "type": "ui-flow-port", "props": { "type": "source" } } ] });
    let end = json!({ "type": "ui-flow-node", "props": { "type": "end" }, "children": [
        { "type": "text", "props": { "value": "End" } },
        { "type": "ui-flow-port", "props": { "type": "target", "connectable": "{{ open }}" } } ] });
    let doc = json!({ "version": 1,
        "state": {
            "open": false, "sel": null, "selection": null, "doomed": null, "connects": 0, "adds": 0,
            "nodes": [
                { "id": "s", "type": "start", "position": { "x": 0, "y": 0 } },
                { "id": "a", "type": "step", "position": { "x": 250, "y": 0 }, "data": { "label": "A" } },
                { "id": "b", "type": "step", "position": { "x": 250, "y": 150 }, "data": { "label": "B" } },
                { "id": "z", "type": "end", "position": { "x": 500, "y": 0 } } ],
            "edges": [] },
        "root": { "type": "stack", "children": [
            { "type": "ui-flow", "attrs": { "id": "f" },
              "props": { "nodes": "{{ nodes }}", "edges": "{{ edges }}", "editable": true, "fit": false,
                         "viewport": { "x": 20, "y": 20, "zoom": 1 }, "height": 500,
                         "bindNodes": "nodes", "bindEdges": "edges", "bind": "sel", "bindSelection": "selection",
                         "newEdge": { "type": "smoothstep" }, "newNode": { "type": "step", "data": { "label": "New" } },
                         "confirmDelete": "{{ open ? 'ask' : null }}", "bindDelete": "doomed" },
              "on": { "connect": { "action": "set_state", "key": "connects", "value": "{{ connects + 1 }}" },
                      "nodeadd": { "action": "set_state", "key": "adds", "value": "{{ adds + 1 }}" } },
              "children": [step, start, end,
                { "type": "ui-flow-panel", "children": [
                    { "type": "button", "attrs": { "id": "fit" }, "props": { "label": "Fit" }, "on": { "click": { "action": "call", "name": "flow.fit" } } } ] }] },
            { "type": "ui-flow-item", "attrs": { "id": "palette" }, "props": { "node": { "type": "step", "data": { "label": "From palette" } } },
              "children": [{ "type": "text", "props": { "value": "Step" } }] },
            { "type": "button", "attrs": { "id": "del" }, "props": { "label": "Delete" }, "on": { "click": { "action": "call", "name": "flow.delete" } } },
            { "type": "element", "props": { "tag": "dialog" }, "attrs": { "id": "ask" } } ] } });
    let driver = r##"
        const f = "f";
        const port = (id, type) => node(f, id).querySelector(`[data-flow-port][data-port-type="${type}"]`);
        const centre = (el) => { const r = el.getBoundingClientRect(); return [r.left + r.width / 2, r.top + r.height / 2]; };
        const drag = (from, [x, y], opts = {}) => {
          const [x0, y0] = centre(from);
          from.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, pointerId: 1, pointerType: "mouse", button: 0, clientX: x0, clientY: y0, ...opts }));
          const over = document.elementFromPoint(x, y) || flow(f);
          for (const k of [0.3, 0.7, 1]) over.dispatchEvent(new PointerEvent("pointermove", { bubbles: true, pointerId: 1, pointerType: "mouse", clientX: x0 + (x - x0) * k, clientY: y0 + (y - y0) * k, ...opts }));
          over.dispatchEvent(new PointerEvent("pointerup", { bubbles: true, pointerId: 1, pointerType: "mouse", button: 0, clientX: x, clientY: y, ...opts }));
        };
        log("connectable", flow(f).hasAttribute("data-connectable"));

        // start → A: allowed. The edge gets `newEdge`'s fields.
        drag(port("s", "source"), centre(port("a", "target")));
        log("edge1", JSON.stringify(S().edges.map((e) => [e.source, e.target, e.type])), S().connects);
        // The same again: `unique` refuses it.
        drag(port("s", "source"), centre(port("a", "target")));
        log("unique", S().edges.length);
        // Dropped on a node's body: joins its first port that fits.
        drag(port("a", "source"), centre(node(f, "b")));
        log("onBody", S().edges.some((e) => e.source === "a" && e.target === "b"));
        // A → A: no self-connections.
        drag(port("a", "source"), centre(port("a", "target")));
        log("self", S().edges.length);
        // From a target port backwards: B's target ← A's source would be a duplicate,
        // but start's source → B is fine.
        drag(port("b", "target"), centre(port("s", "source")));
        log("backwards", S().edges.some((e) => e.source === "s" && e.target === "b"));
        // `accepts`: end isn't a step or start, so B can't take it... and the end port
        // isn't connectable while `open` is false.
        drag(port("a", "source"), centre(port("z", "target")));
        log("closed", S().edges.some((e) => e.target === "z"));
        set("open", true);
        await wait(50);
        drag(port("a", "source"), centre(port("z", "target")));
        log("opened", S().edges.some((e) => e.target === "z"));
        // `max: 2` on A's source: it has b and z now.
        log("max", S().edges.filter((e) => e.source === "a").length);
        set("open", false);

        // Dropped on empty canvas: a `newNode` there, joined to the port.
        const r = flow(f).getBoundingClientRect();
        drag(port("b", "source"), [r.left + 400, r.top + 380]);
        const made = S().nodes.find((n) => n.data?.label === "New");
        log("newNode", !!made, S().edges.some((e) => e.source === "b" && e.target === made?.id), S().sel === made?.id, S().adds);

        // Selecting an edge, then deleting it with the keyboard. (Near its end: s-a and
        // s-b share their first stretch.)
        const hit = edge(f, "s-a").querySelector("[data-flow-hit]");
        const [hx, hy] = (() => { const p = hit.getPointAtLength(hit.getTotalLength() * 0.9); const m = hit.getScreenCTM(); return [p.x * m.a + m.e, p.y * m.d + m.f]; })();
        const target = document.elementFromPoint(hx, hy);
        pointer(target, "pointerdown", hx, hy); pointer(target, "pointerup", hx, hy);
        log("edgeSelected", JSON.stringify(S().selection.edges), edge(f, "s-a").querySelector("[data-flow-path]").hasAttribute("data-selected"));
        flow(f).dispatchEvent(new KeyboardEvent("keydown", { key: "Delete", bubbles: true }));
        log("edgeDeleted", S().edges.some((e) => e.id === "s-a"));

        // Box select with Shift, then drag the group.
        const pane = flow(f).querySelector("[data-flow-pane]");
        const [ax, ay] = centre(node(f, "a")), [bx, by] = centre(node(f, "b"));
        pointer(pane, "pointerdown", r.left + 230, r.top + 5);
        const box = { bubbles: true, pointerId: 1, pointerType: "mouse", shiftKey: true };
        pane.dispatchEvent(new PointerEvent("pointerdown", { ...box, button: 0, clientX: r.left + 240, clientY: r.top + 8 }));
        pane.dispatchEvent(new PointerEvent("pointermove", { ...box, clientX: r.left + 440, clientY: r.top + 260 }));
        pane.dispatchEvent(new PointerEvent("pointerup", { ...box, button: 0, clientX: r.left + 440, clientY: r.top + 260 }));
        log("boxed", JSON.stringify(S().selection.nodes.slice().sort()));
        const pa = S().nodes.find((n) => n.id === "a").position, pb = S().nodes.find((n) => n.id === "b").position;
        drag(node(f, "a"), [ax + 30, ay + 40]);
        const qa = S().nodes.find((n) => n.id === "a").position, qb = S().nodes.find((n) => n.id === "b").position;
        log("groupDrag", qa.x - pa.x, qa.y - pa.y, qb.x - pb.x, qb.y - pb.y);

        // Deleting a node takes its edges; with `confirmDelete`, only after the dialog.
        set("open", true);
        await wait(50);
        click(node(f, "b"));
        flow(f).dispatchEvent(new KeyboardEvent("keydown", { key: "Backspace", bubbles: true }));
        log("asked", JSON.stringify(S().doomed?.nodes), document.getElementById("ask").open, S().nodes.some((n) => n.id === "b"));
        document.getElementById("del").click(); // a `flow.delete` call from outside the flow
        await wait(50);
        log("confirmed", S().nodes.some((n) => n.id === "b"), S().edges.some((e) => e.source === "b" || e.target === "b"), String(S().doomed));

        // The palette: a click adds in the middle of the view, a drop where it lands.
        const before = S().nodes.length;
        document.getElementById("palette").click();
        log("paletteClick", S().nodes.length - before, S().nodes.at(-1).data.label, S().nodes.at(-1).id);
        const dt = new DataTransfer();
        document.getElementById("palette").dispatchEvent(new DragEvent("dragstart", { bubbles: true, dataTransfer: dt }));
        const over = new DragEvent("dragover", { bubbles: true, cancelable: true, dataTransfer: dt, clientX: r.left + 120, clientY: r.top + 300 });
        pane.dispatchEvent(over);
        pane.dispatchEvent(new DragEvent("drop", { bubbles: true, cancelable: true, dataTransfer: dt, clientX: r.left + 120, clientY: r.top + 300 }));
        const dropped = S().nodes.at(-1);
        log("paletteDrop", over.defaultPrevented, dropped.position.x, dropped.position.y);

        // A command from a panel button.
        const z0 = flow(f).querySelector("[data-flow-pane]").style.transform;
        document.getElementById("fit").click();
        await wait(400);
        log("command", flow(f).querySelector("[data-flow-pane]").style.transform !== z0);
    "##;
    let Some(lines) = run("editing", doc, driver) else { return };
    check(
        &lines,
        &[
            "connectable true",
            r#"edge1 [["s","a","smoothstep"]] 1"#,
            "unique 1",
            "onBody true",
            "self 2",
            "backwards true",
            "closed false",
            "opened true",
            "max 2",
            "newNode true true true 1",
            r#"edgeSelected ["s-a"] true"#,
            "edgeDeleted false",
            r#"boxed ["a","b"]"#,
            "groupDrag 30 40 30 40",
            r#"asked ["b"] true true"#,
            "confirmed false false null",
            "paletteClick 1 From palette step_2",
            "paletteDrop true 100 280",
            "command true",
        ],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn containers_clipboard_history_and_search() {
    let step = json!({ "type": "ui-flow-node", "props": { "type": "step" }, "children": [
        { "type": "text", "props": { "value": "{{ node.data.label }}" } },
        { "type": "ui-flow-port", "props": { "type": "target" } },
        { "type": "ui-flow-port", "props": { "type": "source" } } ] });
    let lp = json!({ "type": "ui-flow-node", "props": { "type": "loop", "container": true, "resizable": true, "minWidth": 200 },
        "style": { "text-align": "left" },
        "children": [
            { "type": "text", "props": { "value": "Loop" } },
            { "type": "ui-flow-port", "props": { "type": "target" } },
            { "type": "ui-flow-port", "props": { "id": "start", "scope": "children", "side": "left", "offset": 40 }, "attrs": { "id": "start" } },
            { "type": "ui-flow-port", "props": { "id": "done" } } ] });
    let doc = json!({ "version": 1,
        "state": {
            "q": "", "matches": [], "hist": null, "sel": null,
            "nodes": [
                { "id": "a", "type": "step", "position": { "x": 0, "y": 40 }, "data": { "label": "Alpha" } },
                { "id": "b", "type": "step", "position": { "x": 0, "y": 300 }, "data": { "label": "Beta" } },
                { "id": "loop", "type": "loop", "position": { "x": 300, "y": 0 }, "width": 300, "height": 200 },
                { "id": "c1", "type": "step", "parentId": "loop", "position": { "x": 120, "y": 80 }, "data": { "label": "Inner" } } ],
            "edges": [{ "id": "a-b", "source": "a", "target": "b" }] },
        "root": { "type": "ui-flow", "attrs": { "id": "f" },
            "props": { "nodes": "{{ nodes }}", "edges": "{{ edges }}", "editable": true, "fit": false, "viewport": { "x": 20, "y": 20, "zoom": 1 },
                       "height": 600, "bindNodes": "nodes", "bindEdges": "edges", "bind": "sel", "bindHistory": "hist",
                       "connect": { "sameParent": true }, "search": "{{ q }}", "bindMatches": "matches" },
            "children": [step, lp] } });
    let driver = r##"
        const f = "f";
        const centre = (el) => { const r = el.getBoundingClientRect(); return [r.left + r.width / 2, r.top + r.height / 2]; };
        const drag = (from, [x, y], opts = {}) => {
          const [x0, y0] = opts.at || centre(from);
          from.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, pointerId: 1, pointerType: "mouse", button: 0, clientX: x0, clientY: y0 }));
          for (const k of [0.3, 0.7, 1]) from.dispatchEvent(new PointerEvent("pointermove", { bubbles: true, pointerId: 1, pointerType: "mouse", clientX: x0 + (x - x0) * k, clientY: y0 + (y - y0) * k }));
          from.dispatchEvent(new PointerEvent("pointerup", { bubbles: true, pointerId: 1, pointerType: "mouse", button: 0, clientX: x, clientY: y }));
        };
        const port = (id, type) => node(f, id).querySelector(`[data-flow-port][data-port-type="${type}"]`);
        const key = (k, mods = {}) => flow(f).dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true, metaKey: true, ...mods }));
        const N = (id) => S().nodes.find((n) => n.id === id);

        log("z", +node(f, "loop").style.zIndex < +node(f, "c1").style.zIndex, +edge(f, "a-b").style.zIndex < +node(f, "a").style.zIndex);
        log("child", at(node(f, "c1")).join(","));

        // `scope: children`: the loop's start port joins only nodes inside it.
        drag(document.getElementById("start"), centre(port("c1", "target")));
        log("scopeIn", S().edges.some((e) => e.source === "loop" && e.sourceHandle === "start" && e.target === "c1"));
        drag(document.getElementById("start"), centre(port("b", "target")));
        log("scopeOut", S().edges.some((e) => e.sourceHandle === "start" && e.target === "b"));
        // `sameParent`: a top-level node can't reach inside.
        drag(port("b", "source"), centre(port("c1", "target")));
        log("sameParent", S().edges.some((e) => e.source === "b" && e.target === "c1"));
        log("hist1", S().hist.undo, S().hist.redo);

        // Into the loop: the node keeps its place on screen, and the edge to b,
        // which now crosses out of the loop, goes.
        const before = at(node(f, "a")), [lx, ly] = centre(node(f, "loop"));
        drag(node(f, "a"), [lx - 40, ly + 40]);
        const a = N("a");
        log("into", a.parentId, S().edges.some((e) => e.id === "a-b"), at(node(f, "a"))[0] - at(node(f, "loop"))[0] === a.position.x);
        // And out again.
        const r = flow(f).getBoundingClientRect();
        drag(node(f, "c1"), [r.left + 900, r.top + 450]);
        log("out", N("c1").parentId === undefined, S().edges.some((e) => e.target === "c1"));
        log("order", S().nodes.findIndex((n) => n.id === "loop") < S().nodes.findIndex((n) => n.id === "a"));

        // Resizing writes width and height.
        const h = node(f, "loop").querySelector("[data-flow-resize]"), [hx, hy] = centre(h);
        drag(h, [hx + 50, hy + 30]);
        log("resized", N("loop").width, N("loop").height);
        drag(h, [hx - 400, hy]);
        log("minWidth", N("loop").width);

        // Undo and redo, from the keyboard and as commands.
        const steps = S().hist.undo;
        key("z");
        log("undo", N("loop").width, S().hist.undo === steps - 1, S().hist.redo);
        key("z", { shiftKey: true });
        log("redo", N("loop").width, S().hist.redo);
        flow(f).dispatchEvent(new CustomEvent("irender:call", { detail: { name: "flow.undo", args: {} } }));
        log("undoCommand", N("loop").width);

        // Copy the loop (with what's inside), paste: new ids, parents and edges follow.
        drag(document.getElementById("start"), centre(port("a", "target")));
        click(node(f, "loop"));
        key("c");
        const count = S().nodes.length;
        key("v");
        const added = S().nodes.slice(count);
        const newLoop = added.find((n) => n.type === "loop");
        log("pasted", added.length, newLoop?.id, added.filter((n) => n.parentId === newLoop?.id).length, S().sel);
        const inner = S().edges.filter((e) => e.source === newLoop?.id);
        log("pastedEdges", inner.length, inner.every((e) => added.some((n) => n.id === e.target)));
        // Cut takes them out again.
        key("x");
        log("cut", S().nodes.length === count);

        // Search: matches go to state, the rest is dimmed.
        set("q", "alph");
        await wait(50);
        log("search", JSON.stringify(S().matches), node(f, "a").hasAttribute("data-match"), node(f, "b").hasAttribute("data-dimmed"), node(f, "a").hasAttribute("data-active"));
        set("q", "");
        await wait(50);
        log("cleared", JSON.stringify(S().matches), node(f, "b").hasAttribute("data-dimmed"));
    "##;
    let Some(lines) = run("containers", doc, driver) else { return };
    check(
        &lines,
        &[
            "z true true",
            "child 420,80",
            "scopeIn true",
            "scopeOut false",
            "sameParent false",
            "hist1 1 0",
            "into loop false true",
            "out true false",
            "order true",
            "resized 350 230",
            "minWidth 200",
            "undo 350 true 1",
            "redo 200 0",
            "undoCommand 350",
            "pasted 2 loop_1 1 a_1",
            "pastedEdges 1 true",
            "cut true",
            r#"search ["a"] true true true"#,
            "cleared [] false",
        ],
    );
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn filter_self_loops_and_packing() {
    // a → b → c, a self-reference on c, and five unconnected nodes.
    let mut nodes: Vec<Json> = ["a", "b", "c"].iter().map(|id| json!({ "id": id, "data": { "label": id } })).collect();
    nodes.extend((1..=5).map(|i| json!({ "id": format!("lone{i}"), "data": { "label": format!("lone {i}") } })));
    let doc = json!({ "version": 1, "state": { "picked": [] },
        "root": { "type": "ui-flow", "attrs": { "id": "f" },
            "props": { "nodes": nodes, "edges": [
                    { "id": "ab", "source": "a", "target": "b" }, { "id": "bc", "source": "b", "target": "c" },
                    { "id": "cc", "source": "c", "target": "c", "type": "floating", "label": "parent" } ],
                "layout": "layered", "filter": "{{ picked }}", "height": 600 } } });
    let driver = r##"
        const f = "f";
        const shown = () => [...flow(f).querySelectorAll("[data-flow-nodes] > [data-flow-node]")].filter((n) => n.style.display !== "none").map((n) => n.getAttribute("data-flow-node")).sort().join(",");
        // Unconnected nodes are packed into rows, not one long column.
        const lone = [1, 2, 3, 4, 5].map((i) => at(node(f, "lone" + i)));
        log("packed", new Set(lone.map(([x]) => x)).size > 1, new Set(lone.map(([, y]) => y)).size < 5);
        // A floating self-edge is a loop, with its label.
        const d = edge(f, "cc").querySelector("[data-flow-path]").getAttribute("d");
        log("selfLoop", /^M[-\d.]+,[-\d.]+ C/.test(d), !edge(f, "cc").hidden, label(f, "cc").textContent);
        // `filter`: the picked node, its neighbours as context, nothing else.
        set("picked", ["a"]);
        await wait(100);
        log("filter", shown(), node(f, "b").hasAttribute("data-neighbor"), node(f, "a").hasAttribute("data-neighbor"), edge(f, "bc").hidden);
        set("picked", []);
        await wait(100);
        log("all", shown().split(",").length);
    "##;
    let Some(lines) = run("filter", doc, driver) else { return };
    check(&lines, &["packed true true", "selfLoop true true parent", "filter a,b true false true", "all 8"]);
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn side_form_updates_and_saved_styles() {
    let node = json!({ "type": "ui-flow-node", "props": { "type": "step", "variant": "plain" },
        "attrs": { "data-colour": "{{ node.data.colour }}" },
        "style": { "background": "white", "&[data-colour=rose]": { "background": "rgb(250, 230, 239)" } },
        "children": [{ "type": "text", "props": { "value": "{{ node.data.label }}" } }] });
    let doc = json!({ "version": 1,
        "state": { "picked": null, "label": "", "updates": 0,
            "nodes": [
                { "id": "a", "type": "step", "position": { "x": 0, "y": 0 }, "data": { "label": "A", "keep": 1 },
                  "style": { "background": "rgb(0, 0, 255)" }, "className": "saved" },
                { "id": "b", "type": "step", "position": { "x": 300, "y": 0 }, "data": { "label": "B" } } ] },
        "root": { "type": "ui-flow", "attrs": { "id": "f" },
            "props": { "nodes": "{{ nodes }}", "bindNodes": "nodes", "bindNode": "picked", "nodeStyles": false, "fit": false },
            "on": { "nodeupdate": { "action": "set_state", "key": "updates", "value": "{{ updates + 1 }}" } },
            "children": [node] } });
    let driver = r##"
        const f = "f";
        // `nodeStyles: false`: the saved style and class are left out; the template's apply.
        log("saved", getComputedStyle(node(f, "a")).backgroundColor, node(f, "a").classList.contains("saved"));
        // `bindNode` holds the selected node itself, and follows it.
        click(node(f, "a"));
        log("bound", S().picked?.id, S().picked?.data.label);
        flow(f).dispatchEvent(new CustomEvent("irender:call", { detail: { name: "flow.update", args: { label: "Renamed", colour: "rose" } } }));
        await wait(50);
        const a = S().nodes.find((n) => n.id === "a");
        log("updated", a.data.label, a.data.colour, a.data.keep, S().picked.data.label, S().updates,
            getComputedStyle(node(f, "a")).backgroundColor, node(f, "a").textContent);
        // By id, without a selection.
        flow(f).dispatchEvent(new CustomEvent("irender:call", { detail: { name: "flow.update", args: { id: "b", label: "Bee" } } }));
        log("byId", S().nodes.find((n) => n.id === "b").data.label, S().picked.id);
        click(flow(f).querySelector("[data-flow-pane]"));
        log("cleared", String(S().picked));
    "##;
    let Some(lines) = run("form", doc, driver) else { return };
    check(&lines, &[
        "saved rgb(255, 255, 255) false",
        "bound a A",
        "updated Renamed rose 1 Renamed 1 rgb(250, 230, 239) Renamed",
        "byId Bee a",
        "cleared null",
    ]);
}

#[test]
#[ignore = "needs Chrome; run with --ignored"]
fn shapes_outline_ports_and_floating_edges() {
    let tpl = json!({ "type": "ui-flow-node", "props": { "type": "s", "shape": "{{ node.data.shape }}" },
        "style": { "width": 200, "height": 100 },
        "parts": { "shape": { "style": { "fill": "rgb(1, 2, 3)" } } },
        "children": [
            { "type": "text", "props": { "value": "{{ node.id }}" } },
            { "type": "ui-flow-port", "props": { "id": "in", "type": "target" } },
            { "type": "ui-flow-port", "props": { "id": "up", "type": "target", "side": "left", "offset": "25%" } },
            { "type": "ui-flow-port", "props": { "id": "out", "type": "source" } } ] });
    let doc = json!({ "version": 1,
        "state": { "nodes": [
                { "id": "d", "type": "s", "position": { "x": 0, "y": 0 }, "data": { "shape": "diamond" } },
                { "id": "c", "type": "s", "position": { "x": 400, "y": 300 }, "data": { "shape": "circle" } } ],
            "edges": [{ "id": "dc", "source": "d", "target": "c", "type": "floating" }] },
        "root": { "type": "ui-flow", "attrs": { "id": "f" },
            "props": { "nodes": "{{ nodes }}", "edges": "{{ edges }}", "bindNodes": "nodes", "fit": false, "minimap": true,
                       "viewport": { "x": 0, "y": 0, "zoom": 1 } },
            "children": [tpl, { "type": "ui-flow-edge", "props": { "floating": true, "path": "straight" } }] } });
    let driver = r##"
        const f = "f";
        const box = (id) => node(f, id).getBoundingClientRect();
        const portAt = (id, pid) => { const p = node(f, id).querySelector(`[data-flow-port="${pid}"]`).getBoundingClientRect(), b = box(id);
          return [Math.round(p.left + p.width / 2 - b.left), Math.round(p.top + p.height / 2 - b.top)]; };
        const svg = node(f, "d").querySelector("[data-flow-shape]");
        log("outline", svg.getAttribute("viewBox"), /^M100,0 L200,50 L100,100 L0,50 Z$/.test(svg.firstElementChild.getAttribute("d")),
            getComputedStyle(svg.firstElementChild).fill);
        // Ports on a diamond: the tips, and a quarter of the way down the left edges.
        log("ports", portAt("d", "in").join(","), portAt("d", "out").join(","), portAt("d", "up").join(","));
        // A floating edge meets the circle's outline, not its box.
        const d = edge(f, "dc").querySelector("[data-flow-path]").getAttribute("d").match(/[-\d.]+/g).map(Number);
        const [ex, ey] = d.slice(-2), cx = 400 + 100, cy = 300 + 50;
        log("circle", Math.abs(Math.hypot(ex - cx, ey - cy) - 50) < 1.5);
        // From the diamond's outline too: |x−100|/100 + |y−50|/50 = 1.
        const [sx, sy] = d.slice(0, 2);
        log("diamond", Math.abs(Math.abs(sx - 100) / 100 + Math.abs(sy - 50) / 50 - 1) < 0.02);
        // The shape can change with the data.
        set("nodes", S().nodes.map((n) => (n.id === "d" ? { ...n, data: { shape: "ellipse" } } : n)));
        await wait(100);
        log("changed", node(f, "d").getAttribute("data-shape"), /a100,50/.test(node(f, "d").querySelector("[data-flow-shape] path").getAttribute("d")));
        // Selected: the outline shows it, not a box around the node.
        click(node(f, "c"));
        log("selected", getComputedStyle(node(f, "c")).outlineStyle, getComputedStyle(node(f, "c").querySelector("[data-flow-shape] path")).strokeWidth);
        log("minimap", flow(f).querySelectorAll("[data-flow-minimap] path[transform]").length);
    "##;
    let Some(lines) = run("shapes", doc, driver) else { return };
    check(&lines, &[
        "outline 0 0 200 100 true rgb(1, 2, 3)",
        "ports 0,50 200,50 50,25",
        "circle true",
        "diamond true",
        "changed ellipse true",
        "selected none 2px",
        "minimap 2",
    ]);
}
