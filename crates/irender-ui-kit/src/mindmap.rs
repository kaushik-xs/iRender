//! `ui-mindmap`: a mind map, a central topic with branches on both sides (or on the
//! right), each first branch in a colour of its own. Topics are added, written, folded,
//! moved and deleted from the keyboard and the mouse; it pans and zooms, undoes, and
//! exports as SVG. The topics are a tree in state, `{ id, label, children }` as `ui-tree`
//! has them; `mindmap.js` lays it out, draws it and writes every change back.

use crate::{MUTED, PRIMARY, SUBTLE, TEXT, forms, instance_id, lit, widget};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::collections::HashSet;
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("mindmap.js");

const PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key holding the map: the central topic, `{ id, label, icon, note, href, children, collapsed, color, links }`, children the same; `icon` is a character or emoji before the label, `note` longer text for the topic (edited under the map), `href` a link it opens (⌘/Ctrl+Enter, or its ↗), and the central topic's `links` lines between any two topics, `[{ from, to, label, color }]`").required(),
    PropDef::new("bindSelected", PropKind::StateKey, "State key that gets the `id` of the topic selected"),
    PropDef::new("layout", PropKind::Enum(&["balanced", "right", "down"]), "`balanced` (default: branches on both sides), `right`, or `down` (a tree from the top)"),
    PropDef::new("colors", PropKind::Any, "Colours for the first branches, in turn (default eight)"),
    PropDef::new("readOnly", PropKind::Bool, "Look, fold and move around only"),
    PropDef::new("height", PropKind::Any, "Its height: px or a CSS length (default 480)"),
    PropDef::new("label", PropKind::Any, "Names it for screen readers (default: the central topic)"),
    PropDef::new("id", PropKind::Any, "Element id (default: derived from the document)"),
];

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

const BORDER: &str = "var(--border-width, 1px) solid var(--color-border)";

static MAP: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("position", "relative"),
        ("display", "flex"),
        ("flex-direction", "column"),
        ("overflow", "hidden"),
        ("border", BORDER),
        ("border-radius", "var(--radius-card, var(--radius-lg))"),
        ("--ui-bg", "var(--color-card, var(--color-surface))"),
        ("background", "var(--ui-bg)"),
        ("color", TEXT),
        ("font-size", fs!(14)),
    ])
    .nest("& [data-mm-note]", pairs(&[("display", "grid"), ("gap", gap!(4)), ("padding", pad!(8)), ("border-top", BORDER), ("font-size", fs!(13))]))
    .nest("& [data-mm-note][hidden]", pairs(&[("display", "none")]))
    .nest(
        "& [data-mm-note] textarea",
        pairs(&[("min-height", "56px"), ("padding", pad!(6, 8)), ("border", BORDER), ("border-radius", "var(--radius-control, var(--radius-md))"), ("background", "var(--ui-bg, transparent)"), ("color", "inherit"), ("font", "inherit"), ("resize", "vertical")]),
    )
    .nest("& [data-mm-note] textarea:focus-visible", crate::forms::focus_ring())
    .nest("& [data-mm-bar]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("align-items", "center"), ("gap", gap!(4)), ("padding", pad!(6, 8)), ("border-bottom", BORDER)]))
    .nest("& [data-mm-sep]", pairs(&[("width", "1px"), ("align-self", "stretch"), ("margin", "0 4px"), ("background", "var(--color-border)")]))
    .nest(
        "& [data-mm-bar] button",
        pairs(&[
            ("display", "inline-flex"),
            ("align-items", "center"),
            ("gap", gap!(6)),
            ("height", ctl!(32)),
            ("padding", pad!(0, 8)),
            ("border", "1px solid transparent"),
            ("border-radius", "var(--radius-sm)"),
            ("background", "transparent"),
            ("color", "inherit"),
            ("font", "inherit"),
            ("cursor", "pointer"),
        ]),
    )
    .nest("& [data-mm-bar] button:hover", pairs(&[("background", SUBTLE)]))
    .nest("& [data-mm-bar] button:disabled", pairs(&[("opacity", "0.4"), ("cursor", "default"), ("background", "transparent")]))
    .nest("& [data-mm-bar] button:focus-visible", forms::focus_ring())
    .nest("& [data-mm-bar] svg", pairs(&[("width", "18px"), ("height", "18px")]))
    .nest("& [data-mm-zoom]", pairs(&[("min-width", "44px"), ("color", MUTED), ("font-size", fs!(12)), ("font-variant-numeric", "tabular-nums"), ("text-align", "center")]))
    .nest(
        "& [data-mm-stage]",
        pairs(&[("position", "relative"), ("flex", "1"), ("min-height", "0"), ("overflow", "hidden"), ("outline", "none"), ("touch-action", "none"), ("cursor", "grab"), ("user-select", "none")]),
    )
    .nest("& [data-mm-stage]:focus-visible", pairs(&[("box-shadow", "inset 0 0 0 2px var(--color-ring, var(--color-muted))")]))
    .nest("& [data-mm-stage][data-panning]", pairs(&[("cursor", "grabbing")]))
    .nest("& [data-mm-stage] > svg", pairs(&[("position", "absolute"), ("inset", "0"), ("width", "100%"), ("height", "100%"), ("overflow", "visible")]))
    .nest("& [data-mm-topic]", pairs(&[("cursor", "pointer")]))
    .nest("& [data-mm-topic] text", pairs(&[("font-family", "var(--font-body, system-ui, sans-serif)"), ("dominant-baseline", "central"), ("pointer-events", "none")]))
    .nest(
        "& [data-mm-edit]",
        pairs(&[
            ("position", "absolute"),
            ("box-sizing", "border-box"),
            ("margin", "0"),
            ("padding", "0 12px"),
            ("border", &format!("2px solid {PRIMARY}")),
            ("border-radius", "8px"),
            ("background", "var(--ui-bg)"),
            ("color", "inherit"),
            ("font", "14px var(--font-body, system-ui, sans-serif)"),
            ("outline", "none"),
            ("transform-origin", "0 0"),
        ]),
    )
    .nest(
        "& [data-mm-live]",
        pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("overflow", "hidden"), ("clip-path", "inset(50%)"), ("white-space", "nowrap")]),
    )
});

/// Checks the map: every topic an object with an `id` and a `label`, the ids unique.
fn check(node: &Node, t: &Json, seen: &mut HashSet<String>, depth: usize) -> Result<(), RenderError> {
    if depth > 64 {
        return Err(RenderError::new(node, "the map is more than 64 levels deep"));
    }
    let Some(o) = t.as_object() else { return Err(RenderError::new(node, "each topic is `{ id, label, children }`")) };
    let id = o.get("id").filter(|v| !v.is_null()).map(expr::stringify).ok_or_else(|| RenderError::new(node, "each topic needs an `id` and a `label`"))?;
    if o.get("label").is_none_or(Json::is_null) {
        return Err(RenderError::new(node, format!("topic `{id}` needs a `label`")));
    }
    if !seen.insert(id.clone()) {
        return Err(RenderError::new(node, format!("two topics have the `id` `{id}`")));
    }
    match o.get("children") {
        None | Some(Json::Null) => Ok(()),
        Some(Json::Array(kids)) => kids.iter().try_for_each(|k| check(node, k, seen, depth + 1)),
        Some(_) => Err(RenderError::new(node, format!("topic `{id}`: `children` must be a list"))),
    }
}

pub struct MindMap;

impl Component for MindMap {
    fn name(&self) -> &'static str {
        "ui-mindmap"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A mind map: a central topic with coloured branches on both sides; add, write, fold, move and delete topics from the keyboard or the mouse, pan, zoom, undo and export.",
            props: PROPS,
            parts: &[],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let key = cx.prop_string(node, "bind").filter(|k| !k.is_empty()).ok_or_else(|| RenderError::new(node, "`bind` is the state key holding the map"))?;
        let map = cx.lookup(std::slice::from_ref(&key));
        if !map.is_null() {
            check(node, &map, &mut HashSet::new(), 0)?;
        }
        let colors: Vec<String> = match cx.prop(node, "colors") {
            None | Some(Json::Null) => ["#2563eb", "#e11d48", "#16a34a", "#d97706", "#7c3aed", "#0891b2", "#db2777", "#65a30d"].iter().map(|c| c.to_string()).collect(),
            Some(Json::Array(list)) if !list.is_empty() => list.iter().map(expr::stringify).collect(),
            Some(_) => return Err(RenderError::new(node, "`colors` must be a list of CSS colours")),
        };
        let height = match cx.prop(node, "height").filter(|v| !v.is_null()) {
            None => "480px".to_string(),
            Some(Json::Number(n)) => format!("{n}px"),
            Some(v) => expr::stringify(&v),
        };
        let mut config = widget::config(cx, node, &[], &["layout"], &["bind", "bindSelected"])?;
        config.insert("colors".into(), json!(colors));
        config.insert("readOnly".into(), json!(widget::flag(cx, node, "readOnly", false)));
        let id = instance_id(cx, node, "mindmap");
        config.insert("id".into(), json!(expr::stringify(&cx.eval(&id))));
        widget::kit(cx);
        cx.global_js("ui-mindmap", SCRIPT);

        let label = node.prop("label").cloned().unwrap_or_else(|| lit(map.get("label").map(expr::stringify).unwrap_or_else(|| cx.label("Mind map"))));
        let mut root = cx.element("div", node);
        root.base_style_ref(&MAP).base_style(StyleBlock::new().set("height", height)).behavior("ui-mindmap", Json::Object(config)).attr("id", id);
        cx.open(&root, node)?;
        let mut bar = Element::new("div");
        bar.attr("data-mm-bar", true).attr("role", "toolbar").attr("aria-label", cx.label("Mind map tools"));
        cx.open(&bar, node)?;
        cx.close("div");
        let mut stage = Element::new("div");
        stage.attr("data-mm-stage", true).attr("tabindex", "0").attr("role", "tree").attr("aria-label", label);
        cx.open(&stage, node)?;
        cx.close("div");
        let mut live = Element::new("div");
        live.attr("data-mm-live", true).attr("role", "status").attr("aria-live", "polite");
        cx.open(&live, node)?;
        cx.close("div");
        cx.close("div");
        Ok(())
    }
}
