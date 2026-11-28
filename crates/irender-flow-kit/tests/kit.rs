use irender_core::Format;
use irender_html::{Engine, Page, RenderError};
use serde_json::{Value as Json, json};

fn engine(untrusted: bool) -> Engine {
    let mut engine = if untrusted { Engine::untrusted() } else { Engine::new() };
    irender_ui_kit::register(&mut engine.registry);
    irender_flow_kit::register(&mut engine.registry);
    engine
}

fn render_on(engine: &Engine, root: Json, state: Json) -> Result<Page, RenderError> {
    let doc = irender_core::parse(&json!({ "version": 1, "state": state, "root": root }).to_string(), Format::Json)
        .unwrap_or_else(|d| panic!("{d}"));
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    engine.render(&ir, &json!({}))
}

fn render(root: Json, state: Json) -> Result<Page, RenderError> {
    render_on(&engine(false), root, state)
}

fn unescape(s: &str) -> String {
    s.replace("&quot;", "\"").replace("&#39;", "'").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

/// The flow's behaviour config.
fn config(page: &Page) -> Json {
    let rest = page.body.split(" data-i-xc=\"").nth(1).unwrap();
    serde_json::from_str(&unescape(&rest[..rest.find('"').unwrap()])).unwrap()
}

fn nodes() -> Json {
    json!([
        { "id": "a", "type": "input", "position": { "x": 10, "y": 20 }, "data": { "label": "Start" } },
        { "id": "b", "type": "task", "position": { "x": 200, "y": 20 }, "data": { "label": "Work" } },
        { "id": "c", "type": "output", "position": { "x": 400, "y": 20 }, "data": { "label": "End" } }
    ])
}

fn task() -> Json {
    json!({ "type": "ui-flow-node", "props": { "type": "task" }, "children": [
        { "type": "text", "props": { "value": "Task {{ node.data.label }}" } },
        { "type": "ui-flow-port", "props": { "type": "target", "side": "left" } },
        { "type": "ui-flow-port", "props": { "id": "out", "offset": "30%" } } ] })
}

#[test]
fn server_data_renders_each_node_with_its_template() {
    let page = render(
        json!({ "type": "ui-flow", "props": { "nodes": nodes(), "edges": [{ "source": "a", "target": "b", "label": "go" }] },
                "children": [task()] }),
        json!({}),
    )
    .unwrap();
    let body = &page.body;
    // The task template for `b`, the default node for the others.
    assert!(body.contains(">Task Work<"), "{body}");
    assert!(body.contains(">Start<") && body.contains(">End<"));
    assert!(!body.contains(">Task Start<"));
    // Saved positions, so the page reads without the script.
    assert!(body.contains("transform: translate(200px, 20px)"), "{body}");
    // React Flow's input node has no target port, and its output node no source.
    let ports = |id: &str| {
        let rest = &body[body.find(&format!("data-flow-node=\"{id}\"")).unwrap() + 1..];
        rest[..rest.find("data-flow-node=").unwrap_or(rest.len())].matches("data-port-type=").count()
    };
    assert_eq!(ports("a"), 1);
    assert_eq!(ports("c"), 1);
    assert!(body.contains("data-port-type=\"target\" data-side=\"left\""), "{body}");
    assert!(body.contains("data-flow-port=\"out\""));
    assert!(body.contains("data-offset=\"30%\""));
    // An edge without an id gets one from its ends, and its label.
    assert!(body.contains("data-flow-edge=\"a-b\""), "{body}");
    assert!(body.contains(">go</div>"));
    // Nothing for the browser to rebuild.
    assert!(!body.contains("data-i-list"));
    assert!(!body.contains("<template"));
    assert_eq!(config(&page)["props"]["nodes"][1]["id"], "b");
    assert_eq!(page.scripts.len(), 1);
}

#[test]
fn nodes_in_state_rebuild_in_the_browser_keyed_by_id() {
    let page = render(
        json!({ "type": "ui-flow", "props": { "nodes": "{{ wf }}", "edges": "{{ links }}", "bind": "sel" },
                "children": [task(), { "type": "ui-flow-edge", "props": { "type": "retry", "path": "step", "marker": "arrowclosed" } }] }),
        json!({ "wf": nodes(), "links": [], "sel": null }),
    )
    .unwrap();
    let body = unescape(&page.body);
    assert_eq!(body.matches("data-i-list=").count(), 2, "{body}");
    assert!(body.contains(r#""as":"node""#) && body.contains(r#""as":"edge""#));
    assert!(body.contains(r#""k":"#), "keyed: {body}");
    // Each template is a branch on the item's type; the default one takes the rest.
    assert!(body.contains("data-i-if"), "{body}");
    let cfg = config(&page);
    assert_eq!(cfg["props"]["nodes"]["$irender"]["r"]["p"], json!(["wf"]));
    assert_eq!(cfg["reactive"], true);
    assert_eq!(cfg["bind"], "sel");
    assert_eq!(cfg["edgeTypes"]["retry"], json!({ "path": "step", "marker": "arrowclosed" }));
    assert_eq!(cfg["safe"], false);
}

#[test]
fn templates_are_styled_and_take_parts() {
    let page = render(
        json!({ "type": "ui-flow", "props": { "nodes": nodes(), "edges": [{ "id": "e", "source": "a", "target": "b", "type": "retry" }] },
                "children": [
                    { "type": "ui-flow-node", "props": { "type": "task", "variant": "plain" }, "style": { "color": "red" } },
                    { "type": "ui-flow-edge", "props": { "type": "retry" }, "style": { "stroke": "red" },
                      "parts": { "label": { "style": { "color": "blue" } } },
                      "children": [{ "type": "ui-button", "props": { "label": "Retry" } }] } ] }),
        json!({}),
    )
    .unwrap();
    assert!(page.css.contains("color:red") || page.css.contains("color: red"), "{}", page.css);
    assert!(page.css.contains("stroke:red") || page.css.contains("stroke: red"), "{}", page.css);
    assert!(page.body.contains(">Retry<"), "the edge's children are its label: {}", page.body);
}

#[test]
fn panels_controls_and_minimap() {
    let page = render(
        json!({ "type": "ui-flow", "props": { "nodes": [], "minimap": true, "title": "Plan" },
                "children": [{ "type": "ui-flow-panel", "props": { "position": "top-right" },
                               "children": [{ "type": "text", "props": { "value": "Legend" } }] }] }),
        json!({}),
    )
    .unwrap();
    let body = &page.body;
    assert!(body.contains("data-position=\"top-right\"") && body.contains(">Legend<"));
    assert!(body.contains("aria-label=\"Zoom in\"") && body.contains("data-flow-action=\"fit\""));
    assert!(body.contains("data-flow-minimap"));
    assert!(body.contains("aria-label=\"Plan\""));
    let bare = render(json!({ "type": "ui-flow", "props": { "nodes": [], "controls": false } }), json!({})).unwrap();
    assert!(!bare.body.contains("data-flow-controls") && !bare.body.contains("data-flow-minimap"));
}

#[test]
fn untrusted_documents_get_the_safe_config() {
    let page = render_on(&engine(true), json!({ "type": "ui-flow", "props": { "nodes": nodes() } }), json!({})).unwrap();
    assert_eq!(config(&page)["safe"], true);
}

#[test]
fn misplaced_children_are_errors() {
    let err = |root: Json| render(root, json!({})).unwrap_err().message;
    assert!(err(json!({ "type": "ui-flow", "props": { "nodes": [] }, "children": [{ "type": "text", "props": { "value": "x" } }] }))
        .contains("takes `ui-flow-node`"));
    assert!(err(json!({ "type": "ui-flow-node" })).contains("direct child of `ui-flow`"));
    assert!(err(json!({ "type": "ui-flow-panel" })).contains("direct child of `ui-flow`"));
    assert!(err(json!({ "type": "ui-flow-port" })).contains("inside a `ui-flow-node`"));
    assert!(err(json!({ "type": "ui-flow", "props": { "nodes": [] }, "children": [
        { "type": "ui-flow-node", "props": { "type": "a" } }, { "type": "ui-flow-node", "props": { "type": "a" } }] }))
    .contains("already a `ui-flow-node` template of type `a`"));
    assert!(err(json!({ "type": "ui-flow", "props": { "nodes": [] }, "children": [
        { "type": "ui-flow-node" }, { "type": "ui-flow-node", "props": { "type": "default" } }] }))
    .contains("type `default`"));
    assert!(err(json!({ "type": "ui-flow", "props": { "nodes": [] }, "children": [
        { "type": "ui-flow-node", "props": { "type": "a b" } }] }))
    .contains("fixed name"));
    assert!(err(json!({ "type": "ui-flow", "props": { "nodes": 3 } })).contains("`nodes` must evaluate to an array"));
}

#[test]
fn editing_props_ports_and_palette_items() {
    let page = render(
        json!({ "type": "stack", "children": [
            { "type": "ui-flow", "props": { "nodes": "{{ wf }}", "edges": "{{ links }}", "editable": true,
                "bindNodes": "wf", "bindEdges": "links", "bindSelection": "picked", "bindDelete": "doomed",
                "confirmDelete": "ask", "connect": { "sameParent": true }, "newEdge": { "type": "step" } },
              "children": [{ "type": "ui-flow-node", "props": { "type": "task" }, "children": [
                  { "type": "ui-flow-port", "props": { "type": "target", "accepts": ["task"], "max": 1, "connectable": false } },
                  { "type": "ui-flow-port", "props": { "connectable": "{{ open }}" } } ] }] },
            { "type": "ui-flow-item", "props": { "node": { "type": "task", "data": { "label": "{{ label }}" } } },
              "children": [{ "type": "text", "props": { "value": "Task" } }] } ] }),
        json!({ "wf": nodes(), "links": [{ "id": "e", "source": "a", "target": "b" }], "picked": null, "doomed": null, "open": true, "label": "New" }),
    )
    .unwrap();
    let body = unescape(&page.body);
    let cfg = config(&page);
    for key in ["bindNodes", "bindEdges", "bindSelection", "bindDelete"] {
        assert!(cfg[key].is_string(), "{key}: {cfg}");
    }
    assert_eq!(cfg["props"]["connect"], json!({ "sameParent": true }));
    assert_eq!(cfg["props"]["confirmDelete"], "ask");
    // Port rules: a fixed `connectable: false` and one that follows state.
    assert!(body.contains(r#"data-accepts="[&quot;task&quot;]""#) || body.contains(r#"data-accepts="["task"]""#), "{body}");
    assert!(body.contains(r#"data-max="1""#));
    assert!(body.contains("data-noconnect"));
    assert!(body.contains(r#""n":"data-noconnect""#), "a live binding: {body}");
    // Every edge has a hit path for clicks.
    assert!(body.contains("data-flow-hit"));
    // The palette item: draggable, focusable, with its node (state read in the browser).
    assert!(body.contains(r#"draggable="true""#) && body.contains(r#"data-i-x="ui-flow-item""#), "{body}");
    assert!(body.contains(r#""node":{"type":"task","data":{"label":{"$irender""#), "{body}");
}

#[test]
fn containers_and_search_props() {
    let page = render(
        json!({ "type": "ui-flow", "props": { "nodes": nodes(), "search": "{{ q }}", "bindMatches": "hits", "bindHistory": "hist" },
                "children": [
                    { "type": "ui-flow-node", "props": { "type": "task", "container": true, "resizable": true, "minWidth": 200 },
                      "children": [{ "type": "ui-flow-port", "props": { "id": "start", "scope": "children" } }] },
                    { "type": "ui-flow-node", "children": [{ "type": "text", "props": { "value": "x" } }] } ] }),
        json!({ "q": "", "hits": [], "hist": null }),
    )
    .unwrap();
    let cfg = config(&page);
    assert_eq!(cfg["nodeTypes"]["task"], json!({ "container": true, "resizable": true, "minWidth": 200 }));
    assert_eq!(cfg["nodeTypes"][""], json!({}));
    assert_eq!(cfg["bindMatches"], "hits");
    assert_eq!(cfg["bindHistory"], "hist");
    assert_eq!(cfg["props"]["search"]["$irender"]["r"]["p"], json!(["q"]));
    assert_eq!(page.body.matches("data-flow-resize").count(), 1, "only the resizable template has a handle");
    assert!(page.body.contains(r#"data-scope="children""#));
}

#[test]
fn shapes_render_an_outline_and_reject_unknown_ones() {
    let page = render(
        json!({ "type": "ui-flow", "props": { "nodes": nodes() }, "children": [
            { "type": "ui-flow-node", "props": { "type": "task", "shape": "diamond" }, "parts": { "shape": { "style": { "fill": "red" } } } } ] }),
        json!({}),
    )
    .unwrap();
    assert!(page.body.contains(r#"data-shape="diamond""#), "{}", page.body);
    assert!(page.body.contains("data-flow-shape") && page.body.contains("<path"));
    assert!(page.css.contains("fill:red") || page.css.contains("fill: red"));
    let err = render(
        json!({ "type": "ui-flow", "props": { "nodes": nodes() }, "children": [
            { "type": "ui-flow-node", "props": { "type": "task", "shape": "star" } } ] }),
        json!({}),
    )
    .unwrap_err();
    assert!(err.message.contains("unknown shape `star`"), "{}", err.message);
}

/// Without the script, edges are straight lines between the nodes' middles where
/// they're saved (sizes, parents and `nodeOrigin` counted); with `layout: layered`,
/// which only the browser works out, they're left for it.
#[test]
fn edges_are_drawn_straight_on_the_server() {
    let nodes = json!([
        { "id": "a", "position": { "x": 10, "y": 20 }, "data": { "label": "Start" } },
        { "id": "g", "position": { "x": 300, "y": 0 }, "width": 200, "height": 100 },
        { "id": "b", "parentId": "g", "position": { "x": 10, "y": 10 }, "width": 100, "height": 50, "data": { "label": "In" } }
    ]);
    let edges = json!([{ "id": "ab", "source": "a", "target": "b" }, { "id": "ax", "source": "a", "target": "gone" }]);
    let body = render(json!({ "type": "ui-flow", "props": { "nodes": nodes, "edges": edges } }), json!({})).unwrap().body;
    // a: 10 + 75, 20 + 20; b: 300 + 10 + 50, 0 + 10 + 25.
    assert!(body.contains(r#"d="M85 40L360 35""#), "{body}");
    assert_eq!(body.matches("data-flow-path=\"\" d=").count(), 1, "{body}");
    // The nodes sit on the same corners: the child inside its parent.
    assert!(body.contains(r#"data-flow-node="b" role="group" aria-roledescription="node" aria-label="In" style="transform: translate(310px, 10px)""#), "{body}");
    let centred = render(json!({ "type": "ui-flow", "props": { "nodes": nodes, "edges": edges, "nodeOrigin": [0.5, 0.5] } }), json!({})).unwrap().body;
    assert!(centred.contains(r#"d="M10 20L210 -40""#), "{centred}");
    let laid = render(json!({ "type": "ui-flow", "props": { "nodes": nodes, "edges": edges, "layout": "layered" } }), json!({})).unwrap().body;
    assert!(!laid.contains("data-flow-path=\"\" d="), "{laid}");
    // From state, the server's lines are there too (the script redraws them).
    let live = render(json!({ "type": "ui-flow", "props": { "nodes": "{{ n }}", "edges": "{{ e }}" } }), json!({ "n": nodes, "e": edges })).unwrap().body;
    assert!(live.contains(r#"d="M85 40L360 35""#), "{live}");
}
