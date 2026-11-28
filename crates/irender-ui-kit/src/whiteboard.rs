//! `ui-whiteboard`: a board without edges to draw on, with sticky notes, shapes, arrows and
//! text, moved and zoomed around. The items are a list in state, in the board's own units;
//! `whiteboard.js` draws them as SVG, with the tools, selecting, moving, resizing, editing
//! text, undo, and the view (`{ x, y, zoom }`), and writes every change back.

use crate::{MUTED, PRIMARY, SUBTLE, TEXT, forms, instance_id, lit, widget};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("whiteboard.js");

/// The tools, in the toolbar's order.
pub(crate) const TOOLS: &[&str] = &["select", "hand", "pen", "note", "rect", "ellipse", "arrow", "text", "frame"];
/// What an item may be.
pub(crate) const KINDS: &[&str] = &["pen", "note", "rect", "ellipse", "arrow", "text", "frame", "image"];

const PROPS: &[PropDef] = &[
    PropDef::new(
        "bind",
        PropKind::StateKey,
        "State key holding the items, in the board's units: `{ id, type, x, y, w, h, points, text, color, src, group, locked }`; `type` is pen, note, rect, ellipse, arrow, text, frame (a named area, `text` its name, that carries what's inside it) or image (`src`: http(s), a path or an image's data); `group` ties items to move together, `locked` keeps one still",
    )
    .required(),
    PropDef::new("bindView", PropKind::StateKey, "State key holding the view, `{ x, y, zoom }`, both ways"),
    PropDef::new("tools", PropKind::Any, "The tools offered, from select, hand, pen, note, rect, ellipse, arrow, text, frame (default all)"),
    PropDef::new("colors", PropKind::Any, "The colours to pick from: CSS colours (default eight)"),
    PropDef::new("readOnly", PropKind::Bool, "Look and move around only"),
    PropDef::new("height", PropKind::Any, "Its height: px or a CSS length (default 480)"),
    PropDef::new("label", PropKind::Any, "Names it for screen readers (default `Whiteboard`)"),
    PropDef::new("id", PropKind::Any, "Element id (default: derived from the document)"),
];

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

const BORDER: &str = "var(--border-width, 1px) solid var(--color-border)";

static BOARD: LazyLock<StyleBlock> = LazyLock::new(|| {
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
    // ---- the toolbar
    .nest("& [data-wb-bar]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("align-items", "center"), ("gap", gap!(4)), ("padding", pad!(6, 8)), ("border-bottom", BORDER)]))
    .nest("& [data-wb-sep]", pairs(&[("width", "1px"), ("align-self", "stretch"), ("margin", "0 4px"), ("background", "var(--color-border)")]))
    .nest(
        "& [data-wb-bar] button",
        pairs(&[
            ("display", "inline-grid"),
            ("place-items", "center"),
            ("min-width", ctl!(32)),
            ("height", ctl!(32)),
            ("padding", pad!(0, 6)),
            ("border", "1px solid transparent"),
            ("border-radius", "var(--radius-sm)"),
            ("background", "transparent"),
            ("color", "inherit"),
            ("font", "inherit"),
            ("cursor", "pointer"),
        ]),
    )
    .nest("& [data-wb-bar] button:hover", pairs(&[("background", SUBTLE)]))
    .nest("& [data-wb-bar] button[aria-pressed=true]", pairs(&[("background", "color-mix(in srgb, var(--color-primary) 14%, transparent)"), ("border-color", PRIMARY)]))
    .nest("& [data-wb-bar] button:disabled", pairs(&[("opacity", "0.4"), ("cursor", "default")]))
    .nest("& [data-wb-bar] button:focus-visible", forms::focus_ring())
    .nest("& [data-wb-bar] svg", pairs(&[("width", "18px"), ("height", "18px")]))
    .nest(
        "& [data-wb-swatch]",
        pairs(&[("width", "18px"), ("height", "18px"), ("border-radius", "50%"), ("box-shadow", "inset 0 0 0 1px rgb(0 0 0 / 0.15)")]),
    )
    .nest("& [data-wb-zoom]", pairs(&[("min-width", "44px"), ("color", MUTED), ("font-size", fs!(12)), ("font-variant-numeric", "tabular-nums"), ("text-align", "center")]))
    // ---- the board
    .nest(
        "& [data-wb-stage]",
        pairs(&[
            ("position", "relative"),
            ("flex", "1"),
            ("min-height", "0"),
            ("overflow", "hidden"),
            ("outline", "none"),
            ("touch-action", "none"),
            ("background-image", "radial-gradient(color-mix(in srgb, var(--color-text) 16%, transparent) 1px, transparent 1px)"),
            ("background-size", "var(--wb-grid, 24px) var(--wb-grid, 24px)"),
            ("background-position", "var(--wb-gx, 0) var(--wb-gy, 0)"),
        ]),
    )
    .nest("& [data-wb-stage]:focus-visible", pairs(&[("box-shadow", "inset 0 0 0 2px var(--color-ring, var(--color-muted))")]))
    .nest("& [data-wb-stage] > svg", pairs(&[("position", "absolute"), ("inset", "0"), ("width", "100%"), ("height", "100%"), ("overflow", "visible")]))
    .nest("&[data-tool=hand] [data-wb-stage], & [data-wb-stage][data-panning]", pairs(&[("cursor", "grab")]))
    .nest("& [data-wb-stage][data-panning=on]", pairs(&[("cursor", "grabbing")]))
    .nest("&:is([data-tool=pen], [data-tool=rect], [data-tool=ellipse], [data-tool=arrow], [data-tool=note], [data-tool=text]) [data-wb-stage]", pairs(&[("cursor", "crosshair")]))
    .nest("& [data-wb-item]", pairs(&[("cursor", "move")]))
    .nest("&[data-readonly] [data-wb-item]", pairs(&[("cursor", "default")]))
    .nest("& [data-wb-note] div", pairs(&[("box-sizing", "border-box"), ("width", "100%"), ("height", "100%"), ("padding", "10px 12px"), ("overflow", "hidden"), ("color", "#1f1f1f"), ("font", "15px/1.35 var(--font-body, system-ui, sans-serif)"), ("white-space", "pre-wrap"), ("overflow-wrap", "anywhere")]))
    .nest("& [data-wb-text] div", pairs(&[("box-sizing", "border-box"), ("width", "100%"), ("height", "100%"), ("font", "18px/1.3 var(--font-body, system-ui, sans-serif)"), ("white-space", "pre-wrap"), ("overflow-wrap", "anywhere")]))
    .nest(
        "& [data-wb-edit]",
        pairs(&[
            ("position", "absolute"),
            ("box-sizing", "border-box"),
            ("margin", "0"),
            ("border", &format!("2px solid {PRIMARY}")),
            ("border-radius", "4px"),
            ("background", "transparent"),
            ("color", "inherit"),
            ("resize", "none"),
            ("outline", "none"),
            ("overflow", "hidden"),
            ("transform-origin", "0 0"),
        ]),
    )
    .nest("& [data-wb-empty]", pairs(&[("position", "absolute"), ("inset", "auto 0 16px"), ("margin", "0"), ("color", MUTED), ("text-align", "center"), ("pointer-events", "none")]))
    .nest(
        "& [data-wb-live]",
        pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("overflow", "hidden"), ("clip-path", "inset(50%)"), ("white-space", "nowrap")]),
    )
});

pub struct Whiteboard;

impl Component for Whiteboard {
    fn name(&self) -> &'static str {
        "ui-whiteboard"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A board without edges: draw with a pen, add sticky notes, shapes, arrows and text, select, move, resize and edit them, pan and zoom, undo, and export it as SVG.",
            props: PROPS,
            parts: &[],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let key = cx.prop_string(node, "bind").filter(|k| !k.is_empty()).ok_or_else(|| RenderError::new(node, "`bind` is the state key holding the items"))?;
        let items = match cx.lookup(std::slice::from_ref(&key)) {
            Json::Array(a) => a,
            Json::Null => Vec::new(),
            _ => return Err(RenderError::new(node, format!("`{key}` must hold a list of items"))),
        };
        for (i, it) in items.iter().enumerate() {
            let kind = it.get("type").and_then(Json::as_str).unwrap_or("");
            if !KINDS.contains(&kind) {
                return Err(RenderError::new(node, format!("item {}: `type` is {}, got `{kind}`", i + 1, KINDS.join(", "))));
            }
            if it.get("id").is_none_or(Json::is_null) {
                return Err(RenderError::new(node, format!("item {} needs an `id`", i + 1)));
            }
            // A picture's address: http(s), a path, or an image's data.
            if kind == "image" {
                let src = it.get("src").and_then(Json::as_str).unwrap_or("").trim();
                let data = src.to_ascii_lowercase().starts_with("data:image/");
                if !data && !(crate::combobox::safe_search_url(src) && cx.policy().url_allowed(src)) {
                    return Err(RenderError::new(node, format!("item {}: a picture's `src` is an http(s) address, a path or an image's data, got `{src}`", i + 1)));
                }
            }
        }
        let tools: Vec<String> = match cx.prop(node, "tools") {
            None | Some(Json::Null) => TOOLS.iter().map(|t| t.to_string()).collect(),
            Some(Json::Array(list)) if !list.is_empty() => {
                let mut out = Vec::new();
                for t in &list {
                    match t.as_str() {
                        Some(t) if TOOLS.contains(&t) => out.push(t.to_string()),
                        _ => return Err(RenderError::new(node, format!("`tools` are {}, got `{}`", TOOLS.join(", "), expr::stringify(t)))),
                    }
                }
                out
            }
            Some(_) => return Err(RenderError::new(node, "`tools` must be a list")),
        };
        let colors: Vec<String> = match cx.prop(node, "colors") {
            None | Some(Json::Null) => ["#1f2937", "#e11d48", "#ea580c", "#ca8a04", "#16a34a", "#0891b2", "#2563eb", "#7c3aed"].iter().map(|c| c.to_string()).collect(),
            Some(Json::Array(list)) if !list.is_empty() => list.iter().map(expr::stringify).collect(),
            Some(_) => return Err(RenderError::new(node, "`colors` must be a list of CSS colours")),
        };
        let height = match cx.prop(node, "height").filter(|v| !v.is_null()) {
            None => "480px".to_string(),
            Some(Json::Number(n)) => format!("{n}px"),
            Some(v) => expr::stringify(&v),
        };
        let mut config = widget::config(cx, node, &[], &[], &["bind", "bindView"])?;
        config.insert("tools".into(), json!(tools));
        config.insert("colors".into(), json!(colors));
        let read_only = widget::flag(cx, node, "readOnly", false);
        config.insert("readOnly".into(), json!(read_only));
        let id = instance_id(cx, node, "whiteboard");
        config.insert("id".into(), json!(expr::stringify(&cx.eval(&id))));
        widget::kit(cx);
        cx.global_js("ui-whiteboard", SCRIPT);

        let mut root = cx.element("div", node);
        root.base_style_ref(&BOARD)
            .base_style(StyleBlock::new().set("height", height))
            .behavior("ui-whiteboard", Json::Object(config))
            .attr("id", id)
            .attr("data-tool", if read_only { "hand" } else { "select" });
        if read_only {
            root.attr("data-readonly", true);
        }
        cx.open(&root, node)?;
        let mut bar = Element::new("div");
        bar.attr("data-wb-bar", true).attr("role", "toolbar").attr("aria-label", cx.label("Whiteboard tools"));
        cx.open(&bar, node)?;
        cx.close("div");
        let mut stage = Element::new("div");
        stage
            .attr("data-wb-stage", true)
            .attr("tabindex", "0")
            .attr("role", "application")
            .attr("aria-roledescription", cx.label("whiteboard"))
            .attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Whiteboard"))));
        cx.open(&stage, node)?;
        cx.close("div");
        let mut live = Element::new("div");
        live.attr("data-wb-live", true).attr("role", "status").attr("aria-live", "polite");
        cx.open(&live, node)?;
        cx.close("div");
        cx.close("div");
        Ok(())
    }
}
