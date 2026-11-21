//! Node-and-edge diagrams for iRender: flowcharts, workflows, ER diagrams.
//!
//! ```no_run
//! use irender_core::Format;
//! use irender_html::Engine;
//!
//! let mut engine = Engine::new();
//! irender_flow_kit::register(&mut engine.registry);
//!
//! let doc = irender_core::parse(r#"{ "version": 1, "root": { "type": "ui-flow", "props": {
//!     "nodes": [{ "id": "a", "position": { "x": 0, "y": 0 }, "data": { "label": "Start" } },
//!               { "id": "b", "position": { "x": 240, "y": 0 }, "data": { "label": "End" } }],
//!     "edges": [{ "id": "a-b", "source": "a", "target": "b" }] } } }"#, Format::Json)?;
//! let page = engine.render(&engine.resolve(&doc)?, &serde_json::json!({}))?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! `ui-flow` takes nodes and edges in React Flow's JSON shape, so documents saved by
//! React Flow render as they are. Every node type is a `ui-flow-node` template: an
//! ordinary iRender tree that reads `node` (its `id`, `type`, `data`, …) and places
//! `ui-flow-port`s where edges attach. Edge types are `ui-flow-edge` templates in
//! the same way, and their children are the edge's label. Nodes and edges that
//! use state are rebuilt in the browser, keyed by id, so a node keeps its DOM (and
//! the state of the components inside it) while the graph changes around it.
//!
//! The kit's script (pan and zoom, layout, edge routing, selection, dragging, the
//! minimap) is added once per page that has a flow. It needs no other library.

use irender_core::{
    ComponentInfo, Node, PropDef, PropKind, StyleBlock, Template, Val,
    expr::{self, Expr, Part, UnOp},
};
use irender_html::{ClientList, Component, Element, RenderCx, RenderError, Registry};
use serde_json::{Map, Value as Json, json};
use std::{borrow::Cow, sync::LazyLock};

const SCRIPT: &str = include_str!("flow.js");

/// Adds `ui-flow`, `ui-flow-node`, `ui-flow-edge`, `ui-flow-port` and
/// `ui-flow-panel` to `registry`.
pub fn register(registry: &mut Registry) {
    registry
        .register(Flow)
        .register(Template_(Kind::Node))
        .register(Template_(Kind::Edge))
        .register(Port)
        .register(Panel)
        .register(Item);
}

/// The outlines a node can have.
pub const SHAPES: &[&str] =
    &["rect", "rounded", "pill", "circle", "ellipse", "diamond", "hexagon", "parallelogram", "triangle", "cylinder", "polygon"];

/// The paths an edge can take.
pub const PATHS: &[&str] = &["bezier", "straight", "step", "smoothstep"];

/// The names nodes and edges are in scope under inside their templates.
const NODE: &str = "node";
const EDGE: &str = "edge";
const NODE_INDEX: &str = "nodeIndex";
const EDGE_INDEX: &str = "edgeIndex";

/// An edge's id, or one made from its ends when it has none. `edgeId` in flow.js
/// must agree.
const EDGE_ID: &str = "{{ edge.id || edge.source + '-' + edge.target }}";

// ------------------------------------------------------------------ ui-flow

const FLOW_PROPS: &[PropDef] = &[
    PropDef::new("nodes", PropKind::Any, "Nodes as React Flow saves them: `{ id, type, position: { x, y }, data, parentId, width, height, className, style, hidden }`").required(),
    PropDef::new("edges", PropKind::Any, "Edges: `{ id, source, target, sourceHandle, targetHandle, type, label, animated, markerEnd, markerStart, className, style, hidden }`"),
    PropDef::new("layout", PropKind::Any, "`layered` places the top-level nodes automatically, ignoring their positions; default: use `position`"),
    PropDef::new("direction", PropKind::Any, "`LR` (default), `TB`, `RL` or `BT`: the layout's direction, and where ports without a `side` go"),
    PropDef::new("nodeGap", PropKind::Any, "Layout: space between nodes in a rank (default 40)"),
    PropDef::new("rankGap", PropKind::Any, "Layout: space between ranks (default 80)"),
    PropDef::new("nodeOrigin", PropKind::Any, "Which point of a node `position` names, as fractions: `[0, 0]` top left (default), `[0.5, 0]` top centre"),
    PropDef::new("edgePath", PropKind::Any, "Path for edges whose type and template don't name one: `bezier` (default), `straight`, `step` or `smoothstep`"),
    PropDef::new("fit", PropKind::Any, "Fit the view to the nodes when they first appear (default true)"),
    PropDef::new("fitPadding", PropKind::Any, "Space left around the nodes when fitting, as a fraction of the view (default 0.1)"),
    PropDef::new("viewport", PropKind::Any, "The view to start at when `fit` is false: `{ x, y, zoom }`"),
    PropDef::new("center", PropKind::Any, "A node id: the view pans to that node whenever this changes"),
    PropDef::new("highlight", PropKind::Any, "`neighbors`: while a node is selected, dim everything not connected to it"),
    PropDef::new("selectable", PropKind::Any, "Click or Enter selects a node or edge; Shift adds to the selection (default true)"),
    PropDef::new("editable", PropKind::Any, "Turns on `draggable`, `connectable` and `deletable` together (default false)"),
    PropDef::new("draggable", PropKind::Any, "Nodes can be dragged (default: `editable`)"),
    PropDef::new("connectable", PropKind::Any, "Drag from a port to another to connect them; needs `bindEdges` (default: `editable`)"),
    PropDef::new("deletable", PropKind::Any, "Delete or Backspace removes the selection; needs `bindNodes` / `bindEdges` (default: `editable`)"),
    PropDef::new("connect", PropKind::Any, "Connection rules: `{ sameParent, self, unique, pairs: [{ source, target }] }` (node types)"),
    PropDef::new("newEdge", PropKind::Any, "Fields every new edge starts with, e.g. `{ type: smoothstep, markerEnd: arrowclosed }`"),
    PropDef::new("newNode", PropKind::Any, "A node (`{ type, data }`) made where a connection is dropped on empty canvas, and joined to it"),
    PropDef::new("confirmDelete", PropKind::Any, "The id of a dialog to open before deleting; its button sends `flow.delete`"),
    PropDef::new("pan", PropKind::Any, "Drag the background to pan (default true)"),
    PropDef::new("zoom", PropKind::Any, "Wheel, pinch and keys zoom (default true)"),
    PropDef::new("wheel", PropKind::Any, "What the wheel does: `zoom` (default) or `pan` (Ctrl/⌘ + wheel zooms)"),
    PropDef::new("minZoom", PropKind::Any, "Default 0.1"),
    PropDef::new("maxZoom", PropKind::Any, "Default 2"),
    PropDef::new("background", PropKind::Enum(&["dots", "lines", "none"]), "Background pattern (default dots)"),
    PropDef::new("controls", PropKind::Bool, "Zoom and fit buttons (default true)"),
    PropDef::new("minimap", PropKind::Bool, "An overview in the corner that pans the view (default false)"),
    PropDef::new("height", PropKind::Any, "Number (px) or CSS length; default 480. The width is the container's"),
    PropDef::new("title", PropKind::Any, "Accessible name"),
    PropDef::new("bind", PropKind::StateKey, "State key holding the selected node's id (null when none)"),
    PropDef::new("bindNodes", PropKind::StateKey, "State key that gets the nodes, with new positions, after a drag"),
    PropDef::new("bindViewport", PropKind::StateKey, "State key kept in sync with the view: `{ x, y, zoom }`"),
    PropDef::new("nodeStyles", PropKind::Any, "Apply each node's saved `style` and `className` (default true). Turn off when the template works the look out from `data`"),
    PropDef::new("filter", PropKind::Any, "Node ids to show; the rest are hidden, except the nodes they connect to, shown as context (`data-neighbor`). Empty or null shows all"),
    PropDef::new("filterNeighbors", PropKind::Any, "With `filter`, also show the nodes the shown ones connect to (default true)"),
    PropDef::new("search", PropKind::Any, "Text to find in the nodes: matches are marked `data-match`, everything else is dimmed"),
    PropDef::new("bindMatches", PropKind::StateKey, "State key holding the ids of the nodes `search` matches"),
    PropDef::new("bindHistory", PropKind::StateKey, "State key holding how many edits can be undone and redone: `{ undo, redo }`"),
    PropDef::new("bindEdges", PropKind::StateKey, "State key that gets the edges after a connection or a deletion"),
    PropDef::new("bindNode", PropKind::StateKey, "State key holding the selected node itself (its `id`, `type`, `data`, …), or null: for a side panel that shows or edits it"),
    PropDef::new("bindSelection", PropKind::StateKey, "State key holding every selected node and edge: `{ nodes: [ids], edges: [ids] }`"),
    PropDef::new("bindDelete", PropKind::StateKey, "With `confirmDelete`: what would be deleted, `{ nodes, edges }`, while the dialog is open"),
];

/// Props the browser resolves against state; the rest are read once on the server.
const LIVE: &[&str] = &[
    "nodes", "edges", "layout", "direction", "nodeGap", "rankGap", "nodeOrigin", "edgePath", "fit", "fitPadding",
    "viewport", "center", "highlight", "selectable", "draggable", "pan", "zoom", "wheel", "minZoom", "maxZoom",
    "editable", "connectable", "deletable", "connect", "newEdge", "newNode", "confirmDelete", "search", "filter",
    "filterNeighbors", "nodeStyles",
];

struct Flow;

impl Component for Flow {
    fn name(&self) -> &'static str {
        "ui-flow"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A diagram of nodes and edges in React Flow's JSON shape. Node and edge types are `ui-flow-node` / `ui-flow-edge` children; `ui-flow-panel` children float above it.",
            props: FLOW_PROPS,
            parts: &["controls", "minimap"],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let templates = Templates::collect(node)?;
        let mut props = Map::new();
        let mut reactive = false;
        for name in LIVE {
            if let Some(v) = node.prop(name) {
                let (value, live) = dynamic(cx, node, name, v)?;
                reactive |= live;
                props.insert(name.to_string(), value);
            }
        }
        let mut edge_types = Map::new();
        for (ty, t) in &templates.edges {
            let mut c = Map::new();
            for name in ["path", "marker", "markerStart", "animated", "floating"] {
                if let Some(v) = cx.prop(t, name).filter(|v| !v.is_null()) {
                    c.insert(name.into(), v);
                }
            }
            edge_types.insert(ty.clone().unwrap_or_default(), c.into());
        }
        let mut node_types = Map::new();
        for (ty, t) in &templates.nodes {
            let mut c = Map::new();
            for name in ["container", "resizable", "minWidth", "minHeight"] {
                if let Some(v) = cx.prop(t, name).filter(|v| !v.is_null()) {
                    c.insert(name.into(), v);
                }
            }
            node_types.insert(ty.clone().unwrap_or_default(), c.into());
        }
        let policy = cx.policy();
        let config = json!({
            "props": props,
            "reactive": reactive,
            "edgeTypes": edge_types,
            "nodeTypes": node_types,
            "bind": cx.prop_string(node, "bind"),
            "bindNodes": cx.prop_string(node, "bindNodes"),
            "bindViewport": cx.prop_string(node, "bindViewport"),
            "bindEdges": cx.prop_string(node, "bindEdges"),
            "bindSelection": cx.prop_string(node, "bindSelection"),
            "bindDelete": cx.prop_string(node, "bindDelete"),
            "bindMatches": cx.prop_string(node, "bindMatches"),
            "bindNode": cx.prop_string(node, "bindNode"),
            "bindHistory": cx.prop_string(node, "bindHistory"),
            // Documents the policy doesn't trust can't load URLs through node styles.
            "safe": !(policy.allow_event_attributes && policy.allow_unsafe_urls),
        });
        cx.global_js("ui-flow", SCRIPT);
        cx.global_css("ui-flow", GLOBAL_CSS);

        let mut sizing = StyleBlock::new();
        let height = node.prop("height").cloned().unwrap_or(Val::Lit(json!(480)));
        if let Some(v) = cx.style_value(node, "height", &height)? {
            sizing.push("height", v);
        }
        let background = cx.prop_string(node, "background").unwrap_or_else(|| "dots".into());
        let mut root = cx.element("div", node);
        root.base_style_ref(&ROOT)
            .base_style(sizing)
            .behavior("ui-flow", config)
            .attr("role", "group")
            .attr("aria-roledescription", cx.label("flow"))
            .attr("tabindex", "0")
            .attr("data-background", background);
        if let Some(title) = node.prop("title") {
            root.attr("aria-label", title.clone());
        }
        cx.open(&root, node)?;

        let mut pane = Element::new("div");
        pane.base_style_ref(&PANE).attr("data-flow-pane", "");
        cx.open(&pane, node)?;
        layer(cx, node, "data-flow-edges", node.prop("edges"), EDGE, EDGE_INDEX, EDGE_ID, |cx| {
            item(cx, EDGE, &templates.edges, |cx, t| edge_item(cx, node, t))
        })?;
        layer(cx, node, "data-flow-nodes", node.prop("nodes"), NODE, NODE_INDEX, "{{ node.id }}", |cx| {
            item(cx, NODE, &templates.nodes, |cx, t| node_item(cx, node, t))
        })?;
        cx.close("div");

        for panel in &templates.panels {
            let mut el = cx.element("div", panel);
            let position = cx.prop_string(panel, "position").unwrap_or_else(|| "top-left".into());
            el.base_style_ref(&PANEL).attr("data-position", position);
            cx.open(&el, panel)?;
            cx.children(panel)?;
            cx.close("div");
        }
        if cx.prop(node, "controls") != Some(Json::Bool(false)) {
            let mut controls = cx.part("div", node, "controls");
            controls.base_style_ref(&CONTROLS).attr("data-flow-controls", "");
            cx.open(&controls, node)?;
            for (action, label, icon) in CONTROL_BUTTONS {
                let label = cx.label(label);
                let mut b = Element::new("button");
                b.base_style_ref(&CONTROL).attr("type", "button").attr("aria-label", label.as_str()).attr("title", label.as_str()).attr("data-flow-action", *action);
                cx.open(&b, node)?;
                cx.markup(icon);
                cx.close("button");
            }
            cx.close("div");
        }
        if cx.prop(node, "minimap") == Some(Json::Bool(true)) {
            let mut minimap = cx.part("div", node, "minimap");
            minimap.base_style_ref(&MINIMAP).attr("data-flow-minimap", "").attr("aria-hidden", "true");
            cx.open(&minimap, node)?;
            cx.close("div");
        }
        cx.close("div");
        Ok(())
    }
}

// Labels (looked up where they're used): "Zoom in", "Zoom out", "Fit view"
const CONTROL_BUTTONS: &[(&str, &str, &str)] = &[
    ("zoom-in", "Zoom in", r#"<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" aria-hidden="true"><path d="M8 3v10M3 8h10"/></svg>"#),
    ("zoom-out", "Zoom out", r#"<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" aria-hidden="true"><path d="M3 8h10"/></svg>"#),
    ("fit", "Fit view", r#"<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M2.5 6V2.5H6M10 2.5h3.5V6M13.5 10v3.5H10M6 13.5H2.5V10"/></svg>"#),
];

/// The node and edge layers: a list keyed by id, rebuilt in the browser when its
/// items use state.
#[allow(clippy::too_many_arguments)]
fn layer(
    cx: &mut RenderCx<'_>,
    flow: &Node,
    marker: &str,
    items: Option<&Val>,
    item: &str,
    index: &str,
    key: &str,
    render: impl Fn(&mut RenderCx<'_>) -> Result<(), RenderError>,
) -> Result<(), RenderError> {
    let name = item.to_string() + "s";
    let values = match items.map(|v| cx.eval(v)) {
        Some(Json::Array(values)) => values,
        None | Some(Json::Null) => Vec::new(),
        Some(_) => return Err(RenderError::new(flow, format!("`{name}` must evaluate to an array"))),
    };
    let mut el = Element::new("div");
    el.base_style_ref(&LAYER).attr(marker, "");
    let live = items.filter(|v| cx.is_reactive(v));
    if let Some(items) = live {
        let key = cx.item_val_js(&tpl(key), item, index);
        el.list(ClientList { items, item: item.into(), index: index.into(), key: Some(key) });
    }
    cx.open(&el, flow)?;
    if live.is_some() {
        cx.item_template_with(item, index, &render)?;
    }
    for (i, value) in values.into_iter().enumerate() {
        cx.with_scope(vec![(item.into(), value), (index.into(), json!(i))], &render)?;
    }
    cx.close("div");
    Ok(())
}

/// One node or edge: the template whose type matches, or the default one. Each
/// template is a branch the browser switches on `type`, so an item whose type
/// changes swaps templates.
fn item(
    cx: &mut RenderCx<'_>,
    var: &str,
    templates: &[(Option<String>, &Node)],
    render: impl Fn(&mut RenderCx<'_>, Option<&Node>) -> Result<(), RenderError>,
) -> Result<(), RenderError> {
    let declared: Vec<&str> = templates.iter().filter_map(|(ty, _)| ty.as_deref()).collect();
    for (ty, t) in templates {
        branch(cx, &type_test(var, ty.as_deref(), &declared), |cx| render(cx, Some(t)))?;
    }
    if !templates.iter().any(|(ty, _)| ty.is_none()) {
        branch(cx, &type_test(var, None, &declared), |cx| render(cx, None))?;
    }
    Ok(())
}

fn branch(
    cx: &mut RenderCx<'_>,
    when: &Val,
    f: impl FnOnce(&mut RenderCx<'_>) -> Result<(), RenderError>,
) -> Result<(), RenderError> {
    if cx.is_reactive(when) {
        cx.conditional_with(when, f)
    } else if expr::truthy(&cx.eval(when)) {
        f(cx)
    } else {
        Ok(())
    }
}

/// Whether the item's `type` picks the template for `own` (`None`: the default
/// template, for every type no other template declares).
fn type_test(var: &str, own: Option<&str>, declared: &[&str]) -> Val {
    match own {
        Some(t) => tpl(&format!("{{{{ {var}.type == '{t}' }}}}")),
        None if declared.is_empty() => Val::Lit(Json::Bool(true)),
        None => {
            let any = declared.iter().map(|t| format!("{var}.type == '{t}'")).collect::<Vec<_>>().join(" || ");
            tpl(&format!("{{{{ !({any}) }}}}"))
        }
    }
}

fn tpl(src: &str) -> Val {
    Val::Tpl(Template::parse(src).expect("the kit's own templates parse"))
}

fn node_item(cx: &mut RenderCx<'_>, flow: &Node, template: Option<&Node>) -> Result<(), RenderError> {
    let id = tpl("{{ node.id }}");
    let mut el = match template {
        Some(t) => cx.element("div", t),
        None => Element::new("div"),
    };
    el.base_style_ref(&NODE_BOX);
    let plain = template.and_then(|t| cx.prop_string(t, "variant")).as_deref() == Some("plain");
    // A shape: the kit draws its outline behind the content, so the box itself has
    // no border or background. The shape may come from the node, e.g. its data.
    let shape = template.and_then(|t| t.prop("shape")).filter(|v| !matches!(v, Val::Lit(Json::Null)));
    if let Some(Val::Lit(Json::String(s))) = shape
        && !SHAPES.contains(&s.as_str())
    {
        return Err(RenderError::new(template.unwrap_or(flow), format!("unknown shape `{s}`; use one of {}", SHAPES.join(", "))));
    }
    if !plain {
        el.base_style_ref(if shape.is_some() { &SHAPED_CARD } else { &CARD });
    }
    if let Some(shape) = shape {
        el.attr("data-shape", shape.clone());
    }
    if let Some(points) = template.and_then(|t| t.prop("points")) {
        el.attr("data-points", points.clone());
    }
    el.attr("data-flow-node", id.clone())
        .attr("data-type", tpl("{{ node.type }}"))
        .attr("role", "group")
        .attr("aria-roledescription", cx.label("node"))
        .attr("aria-label", tpl("{{ node.data.label }}"));
    // Without the script, nodes at least sit where they were saved.
    if !cx.is_reactive(&id) {
        let at = |axis: &str| expr::to_num(&cx.lookup(&[NODE.into(), "position".into(), axis.into()]));
        let (x, y) = (at("x"), at("y"));
        if x.is_finite() && y.is_finite() {
            el.attr("style", format!("transform: translate({x}px, {y}px)"));
        }
    }
    let host = template.unwrap_or(flow);
    cx.open(&el, host)?;
    if let (Some(t), Some(_)) = (template, shape) {
        // Sized and drawn by the script, in the node's own pixels.
        let mut svg = Element::new("svg");
        svg.base_style_ref(&SHAPE_SVG).attr("data-flow-shape", "").attr("aria-hidden", "true");
        cx.open(&svg, t)?;
        let mut outline = cx.part("path", t, "shape");
        outline.base_style_ref(&OUTLINE);
        cx.open(&outline, t)?;
        cx.close("path");
        cx.close("svg");
    }
    match template {
        Some(t) => {
            cx.with_context("ui-flow-node", Some(Val::Lit(Json::Bool(true))), |cx| cx.children(t))?;
            if cx.prop(t, "resizable") == Some(Json::Bool(true)) {
                let mut handle = Element::new("span");
                handle.base_style_ref(&RESIZE).attr("data-flow-resize", "").attr("aria-hidden", "true");
                cx.open(&handle, t)?;
                cx.close("span");
            }
        }
        None => {
            let label = tpl("{{ node.data.label }}");
            let mut text = Element::new("span");
            text.text(Cow::Owned(label));
            cx.open(&text, host)?;
            cx.close("span");
            // React Flow's `input` nodes have no target, and `output` nodes no source.
            for (kind, without) in [("target", "input"), ("source", "output")] {
                branch(cx, &tpl(&format!("{{{{ node.type != '{without}' }}}}")), |cx| {
                    let mut port = Element::new("span");
                    port.base_style_ref(&PORT).attr("data-flow-port", "").attr("data-port-type", kind);
                    cx.open(&port, host)?;
                    cx.close("span");
                    Ok(())
                })?;
            }
        }
    }
    cx.close("div");
    Ok(())
}

fn edge_item(cx: &mut RenderCx<'_>, flow: &Node, template: Option<&Node>) -> Result<(), RenderError> {
    let id = tpl(EDGE_ID);
    let at = template.unwrap_or(flow);
    let mut svg = Element::new("svg");
    svg.base_style_ref(&EDGE_SVG).attr("data-flow-edge", id.clone()).attr("aria-hidden", "true");
    cx.open(&svg, at)?;
    let mut path = match template {
        Some(t) => cx.element("path", t),
        None => Element::new("path"),
    };
    path.base_style_ref(&PATH).attr("data-flow-path", "");
    cx.open(&path, at)?;
    cx.close("path");
    let mut hit = Element::new("path");
    hit.base_style_ref(&HIT).attr("data-flow-hit", "");
    cx.open(&hit, at)?;
    cx.close("path");
    cx.close("svg");

    let mut label = match template {
        Some(t) => cx.part("div", t, "label"),
        None => Element::new("div"),
    };
    label.base_style_ref(&LABEL).attr("data-flow-label", id);
    let custom = template.filter(|t| !t.children.is_empty());
    if custom.is_none() {
        label.text(Cow::Owned(tpl("{{ edge.label }}")));
    }
    cx.open(&label, at)?;
    if let Some(t) = custom {
        cx.with_context("ui-flow-edge", Some(Val::Lit(Json::Bool(true))), |cx| cx.children(t))?;
    }
    cx.close("div");
    Ok(())
}

/// A flow's `ui-flow-node`, `ui-flow-edge` and `ui-flow-panel` children, with
/// their types.
struct Templates<'n> {
    nodes: Vec<(Option<String>, &'n Node)>,
    edges: Vec<(Option<String>, &'n Node)>,
    panels: Vec<&'n Node>,
}

impl<'n> Templates<'n> {
    fn collect(flow: &'n Node) -> Result<Self, RenderError> {
        let mut t = Templates { nodes: Vec::new(), edges: Vec::new(), panels: Vec::new() };
        for child in &flow.children {
            let list = match child.kind.as_str() {
                "ui-flow-node" => &mut t.nodes,
                "ui-flow-edge" => &mut t.edges,
                "ui-flow-panel" => {
                    t.panels.push(child);
                    continue;
                }
                other => {
                    return Err(RenderError::new(
                        child,
                        format!("`ui-flow` takes `ui-flow-node`, `ui-flow-edge` and `ui-flow-panel` children, not `{other}`"),
                    ));
                }
            };
            let ty = match child.prop("type") {
                None => None,
                Some(Val::Lit(Json::String(s))) if s == "default" => None,
                Some(Val::Lit(Json::String(s)))
                    if !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_')) =>
                {
                    Some(s.clone())
                }
                Some(_) => {
                    return Err(RenderError::new(child, "`type` must be a fixed name of letters, digits, `-` and `_`"));
                }
            };
            if list.iter().any(|(t, _)| *t == ty) {
                let name = ty.as_deref().unwrap_or("default");
                return Err(RenderError::new(child, format!("there is already a `{}` template of type `{name}`", child.kind)));
            }
            list.push((ty, child));
        }
        Ok(t)
    }
}

// ------------------------------------------------------- the child components

#[derive(Clone, Copy)]
enum Kind {
    Node,
    Edge,
}

/// `ui-flow-node` and `ui-flow-edge`. `ui-flow` renders them itself, once per
/// item; on their own they are an error.
struct Template_(Kind);

const NODE_TEMPLATE_PROPS: &[PropDef] = &[
    PropDef::new("type", PropKind::String, "The node type this template draws. Leave it out (or use `default`) for every type no other template declares"),
    PropDef::new("variant", PropKind::Enum(&["card", "plain"]), "`card` (default) gives the node a border, background and padding; `plain` leaves all of it to `style`"),
    PropDef::new("shape", PropKind::Any, "The node's outline: `rect`, `rounded`, `pill`, `circle`, `ellipse`, `diamond`, `hexagon`, `parallelogram`, `triangle` or `cylinder`; or `polygon` with `points`. May use the node, e.g. `{{ node.data.shape }}`. Style it through `parts.shape` (`fill`, `stroke`)"),
    PropDef::new("points", PropKind::Any, "With `shape: polygon`: the corners as percentages of the box, e.g. `50,0 100,50 50,100 0,50`"),
    PropDef::new("container", PropKind::Bool, "Other nodes can sit inside it: dragging a node onto it makes it the node's parent, and off it, takes it out"),
    PropDef::new("resizable", PropKind::Bool, "A handle in the bottom-right corner resizes the node; the size is saved as its `width` and `height`"),
    PropDef::new("minWidth", PropKind::Number, "Smallest width when resizing (default 80)"),
    PropDef::new("minHeight", PropKind::Number, "Smallest height when resizing (default 40)"),
];

const EDGE_TEMPLATE_PROPS: &[PropDef] = &[
    PropDef::new("type", PropKind::String, "The edge type this template draws. Leave it out (or use `default`) for every type no other template declares"),
    PropDef::new("path", PropKind::Enum(PATHS), "Path shape; default: the edge's own type when it is one of these, else the flow's `edgePath`"),
    PropDef::new("marker", PropKind::Enum(&["none", "arrow", "arrowclosed"]), "Arrowhead at the target, unless the edge has `markerEnd`"),
    PropDef::new("markerStart", PropKind::Enum(&["none", "arrow", "arrowclosed"]), "Arrowhead at the source, unless the edge has `markerStart`"),
    PropDef::new("animated", PropKind::Bool, "Moving dashes, unless the edge sets `animated`"),
    PropDef::new("floating", PropKind::Bool, "Attach to the nodes' borders, facing each other, instead of to ports"),
];

impl Component for Template_ {
    fn name(&self) -> &'static str {
        match self.0 {
            Kind::Node => "ui-flow-node",
            Kind::Edge => "ui-flow-edge",
        }
    }

    fn info(&self) -> ComponentInfo {
        match self.0 {
            Kind::Node => ComponentInfo {
                doc: "A node type of the `ui-flow` it is in: its children are drawn for every node of that `type`, with the node in scope as `node`. Place `ui-flow-port`s inside.",
                props: NODE_TEMPLATE_PROPS,
                parts: &["shape"],
                children: true,
                open_props: false,
            },
            Kind::Edge => ComponentInfo {
                doc: "An edge type of the `ui-flow` it is in. Its `style` styles the path (`stroke`, `stroke-width`, …); its children, with the edge in scope as `edge`, are the label. Without children the label is `edge.label`.",
                props: EDGE_TEMPLATE_PROPS,
                parts: &["label"],
                children: true,
                open_props: false,
            },
        }
    }

    fn render(&self, node: &Node, _cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        Err(RenderError::new(node, format!("`{}` must be a direct child of `ui-flow`", self.name())))
    }
}

const PORT_PROPS: &[PropDef] = &[
    PropDef::new("id", PropKind::Any, "The port's id, matched against an edge's `sourceHandle` / `targetHandle`. May use the node, e.g. `{{ c.handle }}`"),
    PropDef::new("type", PropKind::Enum(&["source", "target"]), "Edges leave from `source` ports (default) and arrive at `target` ports"),
    PropDef::new("side", PropKind::Any, "`left`, `right`, `top` or `bottom`. Default: from the flow's `direction`, e.g. targets left and sources right for `LR`"),
    PropDef::new("offset", PropKind::Any, "Where along the side, e.g. `30%` or `24px`. Default: ports on one side share it evenly"),
    PropDef::new("accepts", PropKind::Any, "Node types allowed at the other end of a new connection, e.g. `[step, merge]`"),
    PropDef::new("max", PropKind::Any, "How many edges this port takes at most"),
    PropDef::new("scope", PropKind::Enum(&["children"]), "`children`: connects only to nodes inside this node (a container), whatever `sameParent` says"),
    PropDef::new("connectable", PropKind::Any, "Whether new connections can start or end here (default true)"),
];

/// `ui-flow-port`: where edges attach to a node.
struct Port;

impl Component for Port {
    fn name(&self) -> &'static str {
        "ui-flow-port"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Where edges attach to a node, in a `ui-flow-node`. It is placed on its side of the nearest positioned element: the node, or e.g. a table row with `position: relative`.",
            props: PORT_PROPS,
            parts: &[],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        if cx.context("ui-flow-node").is_none() {
            return Err(RenderError::new(node, "`ui-flow-port` must be inside a `ui-flow-node`"));
        }
        let kind = cx.prop_string(node, "type").unwrap_or_else(|| "source".into());
        if kind != "source" && kind != "target" {
            return Err(RenderError::new(node, "`type` must be `source` or `target`"));
        }
        let mut el = cx.element("span", node);
        el.base_style_ref(&PORT)
            .attr("data-flow-port", node.prop("id").cloned().unwrap_or_else(|| Val::from("")))
            .attr("data-port-type", kind);
        for (prop, attr) in [
            ("side", "data-side"),
            ("offset", "data-offset"),
            ("accepts", "data-accepts"),
            ("max", "data-max"),
            ("scope", "data-scope"),
        ] {
            if let Some(v) = node.prop(prop) {
                el.attr(attr, v.clone());
            }
        }
        // Present when the port takes no new connections. (A `false` attribute isn't
        // written at all, so the attribute says "not" rather than "connectable".)
        if let Some(v) = node.prop("connectable") {
            el.attr("data-noconnect", not(v));
        }
        cx.open(&el, node)?;
        cx.close("span");
        Ok(())
    }
}

/// `!v`, as a value that stays live when `v` is.
fn not(v: &Val) -> Val {
    match v {
        Val::Lit(j) => Val::Lit(Json::Bool(!expr::truthy(j))),
        Val::Tpl(t) => {
            let e = match t.as_single_expr() {
                Some(e) => e.clone(),
                // A template with text in it is never empty, so always truthy.
                None => Expr::Lit(Json::Bool(true)),
            };
            Val::Tpl(Template { parts: vec![Part::Expr(Expr::Unary(UnOp::Not, Box::new(e)))] })
        }
    }
}

const PANEL_POSITIONS: &[&str] = &["top-left", "top-center", "top-right", "bottom-left", "bottom-center", "bottom-right"];

const PANEL_PROPS: &[PropDef] =
    &[PropDef::new("position", PropKind::Enum(PANEL_POSITIONS), "Corner or edge of the flow (default top-left)")];

/// `ui-flow-panel`: content floating above the flow, which doesn't pan or zoom.
struct Panel;

impl Component for Panel {
    fn name(&self) -> &'static str {
        "ui-flow-panel"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Content that floats above a `ui-flow`, e.g. a toolbar. It doesn't pan or zoom.",
            props: PANEL_PROPS,
            parts: &[],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, _cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        Err(RenderError::new(node, "`ui-flow-panel` must be a direct child of `ui-flow`"))
    }
}

const ITEM_PROPS: &[PropDef] = &[
    PropDef::new("node", PropKind::Any, "The node to add: `{ type, data }`, and `id` if it should keep one").required(),
    PropDef::new("flow", PropKind::String, "The element id of the flow to add to. Default: the flow it is in, or the page's only flow"),
];

/// `ui-flow-item`: a palette entry, dragged onto a flow or clicked to add a node.
struct Item;

impl Component for Item {
    fn name(&self) -> &'static str {
        "ui-flow-item"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A palette entry: drag it onto a `ui-flow` to add its `node` there, or click it to add the node in the middle of the view. The flow needs `bindNodes`.",
            props: ITEM_PROPS,
            parts: &[],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let Some(template) = node.prop("node") else {
            return Err(RenderError::new(node, "`ui-flow-item` needs a `node`"));
        };
        let (value, _) = dynamic(cx, node, "node", template)?;
        cx.global_js("ui-flow", SCRIPT);
        let mut el = cx.element("div", node);
        el.base_style_ref(&ITEM)
            .attr("role", "button")
            .attr("tabindex", "0")
            .attr("draggable", "true")
            .behavior("ui-flow-item", json!({ "node": value, "flow": cx.prop_string(node, "flow") }));
        cx.open(&el, node)?;
        cx.children(node)?;
        cx.close("div");
        Ok(())
    }
}

// ------------------------------------------------------------------- props

/// A prop as the browser needs it: its value, with every template that reads state
/// (at the top or anywhere inside an object literal) left as `{ "$irender": js }`
/// and every other template evaluated now. Also says whether any template was live.
fn dynamic(cx: &RenderCx<'_>, node: &Node, name: &str, v: &Val) -> Result<(Json, bool), RenderError> {
    match v {
        Val::Tpl(_) if cx.is_reactive(v) => Ok((json!({ "$irender": cx.val_js(v) }), true)),
        Val::Tpl(_) => Ok((cx.eval(v), false)),
        Val::Lit(lit) => {
            let mut live = false;
            let out = nested(cx, node, name, lit, &mut live)?;
            Ok((out, live))
        }
    }
}

fn nested(cx: &RenderCx<'_>, node: &Node, name: &str, v: &Json, live: &mut bool) -> Result<Json, RenderError> {
    Ok(match v {
        Json::String(s) if Template::is_template(s) => {
            let t = Template::parse(s).map_err(|e| RenderError::new(node, format!("`{name}`: {e}")))?;
            let (value, l) = dynamic(cx, node, name, &Val::Tpl(t))?;
            *live |= l;
            value
        }
        Json::Array(items) => Json::Array(items.iter().map(|i| nested(cx, node, name, i, live)).collect::<Result<_, _>>()?),
        Json::Object(o) => Json::Object(
            o.iter().map(|(k, v)| Ok((k.clone(), nested(cx, node, name, v, live)?))).collect::<Result<_, RenderError>>()?,
        ),
        other => other.clone(),
    })
}

// ------------------------------------------------------------------ styles

const BORDER: &str = "var(--color-border, #e4e4e7)";
const SURFACE: &str = "var(--color-surface, #ffffff)";
const TEXT: &str = "var(--color-text, #09090b)";
const MUTED: &str = "var(--color-muted, #71717a)";
const RING: &str = "var(--color-ring, #a1a1aa)";
const PRIMARY: &str = "var(--color-primary, #18181b)";

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

static ROOT: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        // Positions are geometry: left to right on any page.
        ("direction", "ltr"),
        ("position", "relative"),
        ("overflow", "hidden"),
        ("width", "100%"),
        ("box-sizing", "border-box"),
        ("touch-action", "none"),
        ("outline", "none"),
        ("color", TEXT),
        ("font-size", "calc(13px * var(--font-scale, 1))"),
        ("cursor", "grab"),
        ("background-size", "var(--flow-gap, 20px) var(--flow-gap, 20px)"),
        ("background-position", "var(--flow-bg-x, 0) var(--flow-bg-y, 0)"),
    ])
    .nest("&[data-background=dots]", pairs(&[("background-image", &format!("radial-gradient(circle, {BORDER} 1px, transparent 1.2px)"))]))
    .nest(
        "&[data-background=lines]",
        pairs(&[(
            "background-image",
            &format!("linear-gradient(to right, {BORDER} 1px, transparent 1px), linear-gradient(to bottom, {BORDER} 1px, transparent 1px)"),
        )]),
    )
    .nest("&[data-panning]", pairs(&[("cursor", "grabbing"), ("user-select", "none")]))
    .nest("&:fullscreen", pairs(&[("height", "100vh"), ("background-color", SURFACE)]))
    .nest("&:focus-visible", pairs(&[("box-shadow", &format!("inset 0 0 0 2px {RING}"))]))
});

static PANE: LazyLock<StyleBlock> =
    LazyLock::new(|| pairs(&[("position", "absolute"), ("inset", "0"), ("transform-origin", "0 0")]));

static LAYER: LazyLock<StyleBlock> =
    LazyLock::new(|| pairs(&[("position", "absolute"), ("inset", "0"), ("pointer-events", "none")]));

static NODE_BOX: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("position", "absolute"),
        ("left", "0"),
        ("top", "0"),
        ("box-sizing", "border-box"),
        ("pointer-events", "auto"),
        ("cursor", "default"),
        ("outline", "none"),
        ("transition", "opacity .15s"),
    ])
    .nest("&[data-selectable]", pairs(&[("cursor", "pointer")]))
    .nest("&[data-draggable]", pairs(&[("cursor", "grab")]))
    .nest("&[data-dragging]", pairs(&[("cursor", "grabbing")]))
    .nest("&[data-drop-target]", pairs(&[("outline", &format!("2px dashed {PRIMARY}")), ("outline-offset", "4px")]))
    .nest("&[data-active]", pairs(&[("outline", "2px solid #f59e0b"), ("outline-offset", "3px")]))
    .nest("&[data-selected]", pairs(&[("outline", &format!("2px solid {PRIMARY}")), ("outline-offset", "2px")]))
    .nest("&:focus-visible", pairs(&[("outline", &format!("2px solid {RING}")), ("outline-offset", "2px")]))
    .nest("&[data-dimmed]", pairs(&[("opacity", "0.3")]))
    // A shaped node shows its state on its outline, not as a box around it.
    .nest("&[data-shape]", pairs(&[("isolation", "isolate")]))
    .nest("&[data-shape][data-selected]", pairs(&[("outline", "none")]))
    .nest("&[data-shape]:focus-visible", pairs(&[("outline", "none")]))
    .nest("&[data-shape][data-selected] > [data-flow-shape] > path", pairs(&[("stroke", PRIMARY), ("stroke-width", "2")]))
    .nest("&[data-shape]:focus-visible > [data-flow-shape] > path", pairs(&[("stroke", RING), ("stroke-width", "2")]))
    .nest("&[data-shape][data-drop-target]", pairs(&[("outline", "none")]))
    .nest("&[data-neighbor]", pairs(&[("opacity", "0.8"), ("outline", &format!("1px dashed {MUTED}")), ("outline-offset", "3px")]))
});

static CARD: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("min-width", "120px"),
        ("padding", "calc(10px * var(--space-scale, 1) * var(--space-inset, 1)) calc(14px * var(--space-scale, 1) * var(--space-inset, 1))"),
        ("border", &format!("1px solid {BORDER}")),
        ("border-radius", "var(--radius-md, 8px)"),
        ("background", SURFACE),
        ("color", TEXT),
        ("font-size", "calc(12px * var(--font-scale, 1))"),
        ("line-height", "1.4"),
        ("text-align", "center"),
        ("box-shadow", "0 1px 2px rgb(0 0 0 / 0.06)"),
    ])
});

/// A shaped node's content: centred, with the shape's outline behind it.
static SHAPED_CARD: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("min-width", "100px"),
        ("min-height", "56px"),
        ("padding", "calc(12px * var(--space-scale, 1) * var(--space-inset, 1)) calc(18px * var(--space-scale, 1) * var(--space-inset, 1))"),
        ("display", "grid"),
        ("place-items", "center"),
        ("text-align", "center"),
        ("color", TEXT),
        ("font-size", "calc(12px * var(--font-scale, 1))"),
        ("line-height", "1.4"),
    ])
});

/// The outline's canvas, behind the node's content.
static SHAPE_SVG: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("position", "absolute"),
        ("left", "0"),
        ("top", "0"),
        ("width", "100%"),
        ("height", "100%"),
        ("overflow", "visible"),
        ("z-index", "-1"),
        ("pointer-events", "none"),
    ])
});

static OUTLINE: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("fill", SURFACE), ("stroke", "var(--color-border, #d4d4d8)"), ("stroke-width", "1.5"), ("stroke-linejoin", "round")])
});

static PORT: LazyLock<StyleBlock> = LazyLock::new(|| {
    let at = |a: &str, b: &str| pairs(&[(a, b), (if a == "left" { "top" } else { "left" }, "var(--flow-port-offset, 50%)")]);
    pairs(&[
        ("position", "absolute"),
        ("z-index", "1"),
        ("width", "8px"),
        ("height", "8px"),
        ("box-sizing", "border-box"),
        ("border-radius", "50%"),
        ("background", MUTED),
        ("border", &format!("1.5px solid {SURFACE}")),
        ("transform", "translate(-50%, -50%)"),
        ("pointer-events", "none"),
    ])
    .nest("&[data-side=left]", at("left", "0"))
    .nest("&[data-side=right]", at("left", "100%"))
    .nest("&[data-side=top]", at("top", "0"))
    .nest("&[data-side=bottom]", at("top", "100%"))
    // Ports take connections while the flow is connectable.
    .nest("[data-connectable] &", pairs(&[("pointer-events", "auto"), ("cursor", "crosshair"), ("width", "10px"), ("height", "10px")]))
    .nest("[data-connectable] &[data-noconnect]", pairs(&[("pointer-events", "none"), ("cursor", "default")]))
    .nest("[data-connectable] &:hover", pairs(&[("background", PRIMARY)]))
    .nest("&[data-connect=valid]", pairs(&[("background", "#16a34a"), ("width", "12px"), ("height", "12px")]))
    .nest("&[data-connect=invalid]", pairs(&[("background", "var(--color-destructive, #dc2626)")]))
});

static EDGE_SVG: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("position", "absolute"),
        ("left", "0"),
        ("top", "0"),
        ("width", "1px"),
        ("height", "1px"),
        ("overflow", "visible"),
        ("pointer-events", "none"),
        ("transition", "opacity .15s"),
    ])
    .nest("&[data-dimmed]", pairs(&[("opacity", "0.15")]))
    .nest("&[hidden]", pairs(&[("display", "none")]))
});

static PATH: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("fill", "none"), ("stroke", "var(--color-muted, #a1a1aa)"), ("stroke-width", "1.5")])
        .nest("&[data-animated]", pairs(&[("stroke-dasharray", "5"), ("animation", "ui-flow-dash .5s linear infinite")]))
        .nest("&[data-highlighted]", pairs(&[("stroke", PRIMARY), ("stroke-width", "2")]))
        .nest("&[data-selected]", pairs(&[("stroke", PRIMARY), ("stroke-width", "2.5")]))
});

/// The corner handle of a resizable node, shown while it is hovered or selected.
static RESIZE: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("position", "absolute"),
        ("right", "0"),
        ("bottom", "0"),
        ("width", "12px"),
        ("height", "12px"),
        ("cursor", "nwse-resize"),
        ("pointer-events", "auto"),
        ("opacity", "0"),
        ("transition", "opacity .15s"),
        ("background", &format!("linear-gradient(135deg, transparent 50%, {MUTED} 50%, {MUTED} 60%, transparent 60%, transparent 75%, {MUTED} 75%, {MUTED} 85%, transparent 85%)")),
    ])
    .nest("[data-flow-node]:hover > &", pairs(&[("opacity", "1")]))
    .nest("[data-flow-node][data-selected] > &", pairs(&[("opacity", "1")]))
});

/// The invisible, wider copy of an edge's path that takes clicks.
static HIT: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("fill", "none"), ("stroke", "transparent"), ("stroke-width", "14"), ("pointer-events", "none"), ("cursor", "pointer")])
});

static ITEM: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("cursor", "grab"), ("user-select", "none"), ("touch-action", "manipulation")])
        .nest("&:focus-visible", pairs(&[("outline", &format!("2px solid {RING}")), ("outline-offset", "2px")]))
});

static LABEL: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("position", "absolute"),
        ("left", "0"),
        ("top", "0"),
        ("pointer-events", "auto"),
        ("font-size", "calc(11px * var(--font-scale, 1))"),
        ("line-height", "1.3"),
        ("padding", "calc(2px * var(--space-scale, 1) * var(--space-inset, 1)) calc(6px * var(--space-scale, 1) * var(--space-inset, 1))"),
        ("border-radius", "4px"),
        ("background", SURFACE),
        ("color", MUTED),
        ("white-space", "nowrap"),
        ("transition", "opacity .15s"),
    ])
    .nest("&:empty", pairs(&[("display", "none")]))
    .nest("&[hidden]", pairs(&[("display", "none")]))
    .nest("&[data-dimmed]", pairs(&[("opacity", "0.15")]))
});

static PANEL: LazyLock<StyleBlock> = LazyLock::new(|| {
    let at = |p: &[(&str, &str)]| pairs(p);
    pairs(&[("position", "absolute"), ("z-index", "4"), ("cursor", "default")])
        .nest("&[data-position=top-left]", at(&[("top", "12px"), ("left", "12px")]))
        .nest("&[data-position=top-center]", at(&[("top", "12px"), ("left", "50%"), ("transform", "translateX(-50%)")]))
        .nest("&[data-position=top-right]", at(&[("top", "12px"), ("right", "12px")]))
        .nest("&[data-position=bottom-left]", at(&[("bottom", "12px"), ("left", "12px")]))
        .nest("&[data-position=bottom-center]", at(&[("bottom", "12px"), ("left", "50%"), ("transform", "translateX(-50%)")]))
        .nest("&[data-position=bottom-right]", at(&[("bottom", "12px"), ("right", "12px")]))
});

static CONTROLS: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("position", "absolute"),
        ("z-index", "4"),
        ("left", "12px"),
        ("bottom", "12px"),
        ("display", "flex"),
        ("flex-direction", "column"),
        ("border", &format!("1px solid {BORDER}")),
        ("border-radius", "var(--radius-md, 8px)"),
        ("overflow", "hidden"),
        ("background", SURFACE),
        ("box-shadow", "0 1px 2px rgb(0 0 0 / 0.06)"),
        ("cursor", "default"),
    ])
});

static CONTROL: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("display", "grid"),
        ("place-items", "center"),
        ("width", "28px"),
        ("height", "28px"),
        ("padding", "0"),
        ("border", "none"),
        ("border-bottom", &format!("1px solid {BORDER}")),
        ("background", "transparent"),
        ("color", TEXT),
        ("cursor", "pointer"),
    ])
    .nest("&:last-child", pairs(&[("border-bottom", "none")]))
    .nest("&:hover", pairs(&[("background", "var(--color-subtle, #f4f4f5)")]))
    .nest("&:focus-visible", pairs(&[("outline", &format!("2px solid {RING}")), ("outline-offset", "-2px")]))
});

static MINIMAP: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("position", "absolute"),
        ("z-index", "4"),
        ("right", "12px"),
        ("bottom", "12px"),
        ("width", "180px"),
        ("height", "120px"),
        ("border", &format!("1px solid {BORDER}")),
        ("border-radius", "var(--radius-md, 8px)"),
        ("overflow", "hidden"),
        ("background", SURFACE),
        ("box-shadow", "0 1px 2px rgb(0 0 0 / 0.06)"),
        ("cursor", "pointer"),
    ])
    .nest("& rect", pairs(&[("fill", "var(--color-border, #d4d4d8)")]))
    .nest("& rect[data-selected]", pairs(&[("fill", MUTED)]))
    .nest("& path", pairs(&[("fill", "color-mix(in srgb, var(--color-text, #09090b) 8%, transparent)")]))
});

/// The animated dashes, once per page.
const GLOBAL_CSS: &str = "@keyframes ui-flow-dash{to{stroke-dashoffset:-10}}\
@media (prefers-reduced-motion:reduce){[data-flow-path]{animation:none!important}}";
