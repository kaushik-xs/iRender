//! `ui-resizable`: panels side by side (or stacked) with a handle between each two that
//! moves the line between them. Sizes are percentages of the group, from each panel's
//! `size` or from state (`bind`, which gets them back after each move), so the server
//! draws them as they are. `resizable.js` does the dragging and the keys.

use crate::{BORDER, MUTED, RING, bin, cond, instance_id, id_with, lit};
use irender_core::{
    ComponentInfo, Node, PropDef, PropKind, StyleBlock, StyleValue, Template,
    expr::{self, BinOp, Expr, Part},
};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("resizable.js");

pub struct Resizable;
pub struct ResizablePanel;

const PROPS: &[PropDef] = &[
    PropDef::new("direction", PropKind::String, "`horizontal` (side by side, the default) or `vertical` (stacked)"),
    PropDef::new("bind", PropKind::StateKey, "State key for the sizes: a list of percentages, one per panel, written after each move"),
    PropDef::new("handle", PropKind::Bool, "Shows a grip on each handle"),
    PropDef::new("height", PropKind::Any, "The group's height (default: as tall as its container lets it be)"),
    PropDef::new("label", PropKind::Any, "Names the handles for screen readers: `Resize <label>` (default `Resize panels`)"),
];
const PANEL_PROPS: &[PropDef] = &[
    PropDef::new("size", PropKind::Number, "Its starting size in % (the panels without one share what's left)"),
    PropDef::new("min", PropKind::Number, "The smallest it can be, in % (default 10)"),
    PropDef::new("max", PropKind::Number, "The largest it can be, in % (default 100)"),
    PropDef::new("collapsible", PropKind::Bool, "Can fold to nothing: dragged under half its `min`, or Enter on its handle"),
    PropDef::new("label", PropKind::Any, "Its name, read with the handle before it"),
];

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

static GROUP: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("display", "flex"), ("width", "100%"), ("height", "var(--rz-height, 100%)"), ("min-height", "0"), ("overflow", "hidden")])
        .nest("&[data-direction=vertical]", pairs(&[("flex-direction", "column")]))
        .nest("& > [data-rz-panel]", pairs(&[("flex", "var(--rz-size) 1 0px"), ("min-width", "0"), ("min-height", "0"), ("overflow", "auto")]))
        .nest("& > [data-rz-panel][data-collapsed]", pairs(&[("overflow", "hidden"), ("visibility", "hidden")]))
        // The handle: a line, with a wider area to grab.
        .nest(
            "& > [data-rz-handle]",
            pairs(&[
                ("position", "relative"),
                ("flex", "0 0 1px"),
                ("background", BORDER),
                ("cursor", "col-resize"),
                ("touch-action", "none"),
                ("user-select", "none"),
                ("min-width", "0"),
                ("min-height", "0"),
                ("z-index", "1"),
                ("outline", "none"),
                ("transition", "background .15s"),
            ]),
        )
        .nest("&[data-direction=vertical] > [data-rz-handle]", pairs(&[("cursor", "row-resize")]))
        .nest("& > [data-rz-handle]::before", pairs(&[("content", "''"), ("position", "absolute"), ("inset", "0 -5px")]))
        .nest("&[data-direction=vertical] > [data-rz-handle]::before", pairs(&[("inset", "-5px 0")]))
        .nest("& > [data-rz-handle]:is(:hover, [data-dragging])", pairs(&[("background", RING)]))
        .nest("& > [data-rz-handle]:focus-visible", pairs(&[("background", RING), ("box-shadow", "0 0 0 2px var(--color-ring, var(--color-muted))")]))
        .nest(
            "& > [data-rz-handle] [data-rz-grip]",
            pairs(&[
                ("position", "absolute"),
                ("inset-inline-start", "50%"),
                ("top", "50%"),
                ("translate", "-50% -50%"),
                ("display", "grid"),
                ("place-items", "center"),
                ("width", "12px"),
                ("height", "18px"),
                ("border", "1px solid var(--color-border)"),
                ("border-radius", "var(--radius-sm)"),
                ("background", "var(--color-surface)"),
                ("color", MUTED),
            ]),
        )
        .nest("&[data-direction=vertical] > [data-rz-handle] [data-rz-grip]", pairs(&[("width", "18px"), ("height", "12px"), ("transform", "rotate(90deg)")]))
        .nest("@media (prefers-reduced-motion: reduce)", pairs(&[]).nest("& > [data-rz-handle]", pairs(&[("transition", "none")])))
});

const GRIP: &str = r#"<svg width="6" height="10" viewBox="0 0 6 10" fill="currentColor" aria-hidden="true"><circle cx="1.5" cy="1.5" r="1"/><circle cx="4.5" cy="1.5" r="1"/><circle cx="1.5" cy="5" r="1"/><circle cx="4.5" cy="5" r="1"/><circle cx="1.5" cy="8.5" r="1"/><circle cx="4.5" cy="8.5" r="1"/></svg>"#;

struct Panel<'n> {
    node: &'n Node,
    size: f64,
    min: f64,
    max: f64,
    collapsible: bool,
}

/// The panels with their sizes: those given, the rest sharing what's left, then scaled
/// to add up to 100.
fn panels<'n>(cx: &RenderCx<'_>, node: &'n Node) -> Result<Vec<Panel<'n>>, RenderError> {
    let kids: Vec<&Node> = node.children.iter().collect();
    if let Some(c) = kids.iter().find(|c| c.kind != "ui-resizable-panel") {
        return Err(RenderError::new(c, "`ui-resizable` takes only `ui-resizable-panel` children"));
    }
    if kids.len() < 2 {
        return Err(RenderError::new(node, "`ui-resizable` needs two or more `ui-resizable-panel` children"));
    }
    let num = |n: &Node, k: &str| cx.prop(n, k).map(|v| expr::to_num(&v));
    let mut out = Vec::new();
    for k in &kids {
        let (min, max) = (num(k, "min").unwrap_or(10.0), num(k, "max").unwrap_or(100.0));
        if !(0.0..=100.0).contains(&min) || !(min..=100.0).contains(&max) {
            return Err(RenderError::new(k, "`min` and `max` must be from 0 to 100, `min` no more than `max`"));
        }
        let size = num(k, "size").unwrap_or(f64::NAN);
        if size.is_finite() && !(0.0..=100.0).contains(&size) {
            return Err(RenderError::new(k, "`size` must be from 0 to 100"));
        }
        out.push(Panel { node: k, size, min, max, collapsible: cx.prop(k, "collapsible").is_some_and(|v| expr::truthy(&v)) });
    }
    let given: f64 = out.iter().filter(|p| p.size.is_finite()).map(|p| p.size).sum();
    let rest = out.iter().filter(|p| !p.size.is_finite()).count();
    let share = if rest > 0 { ((100.0 - given) / rest as f64).max(0.0) } else { 0.0 };
    for p in &mut out {
        if !p.size.is_finite() {
            p.size = share;
        }
    }
    let total: f64 = out.iter().map(|p| p.size).sum();
    if total <= 0.0 {
        return Err(RenderError::new(node, "the panels' sizes add up to nothing"));
    }
    for p in &mut out {
        p.size = (p.size * 100.0 / total * 100.0).round() / 100.0;
    }
    Ok(out)
}

impl Component for Resizable {
    fn name(&self) -> &'static str {
        "ui-resizable"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Panels side by side or stacked (`ui-resizable-panel` children), with a handle between each two to resize them by \
                  dragging or with the arrow keys. Sizes are percentages; `bind` keeps them in state.",
            props: PROPS,
            parts: &["handle"],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let direction = cx.prop_string(node, "direction").unwrap_or_else(|| "horizontal".into());
        if direction != "horizontal" && direction != "vertical" {
            return Err(RenderError::new(node, "`direction` must be `horizontal` or `vertical`"));
        }
        let key = cx.prop_string(node, "bind");
        let list = panels(cx, node)?;
        let id = instance_id(cx, node, "rz");
        let grip = cx.prop(node, "handle").is_some_and(|v| expr::truthy(&v));
        let name = cx.prop_string(node, "label").unwrap_or_else(|| cx.label("panels"));

        cx.global_js("ui-resizable", SCRIPT);
        let mut root = cx.element("div", node);
        let mut vars = StyleBlock::new();
        if let Some(h) = cx.prop(node, "height") {
            vars = vars.set("--rz-height", if let Json::Number(n) = &h { format!("{n}px") } else { expr::stringify(&h) });
        }
        root.base_style_ref(&GROUP).base_style(vars).attr("data-direction", direction.clone()).behavior(
            "ui-resizable",
            json!({
                "direction": direction,
                "bind": key,
                "sizes": list.iter().map(|p| p.size).collect::<Vec<_>>(),
                "panels": list.iter().map(|p| json!({ "min": p.min, "max": p.max, "collapsible": p.collapsible })).collect::<Vec<_>>(),
            }),
        );
        cx.open(&root, node)?;
        let n = list.len();
        for (i, p) in list.iter().enumerate() {
            // Its size: from state when `bind` holds one per panel, otherwise as given.
            let size = match &key {
                Some(k) => {
                    let ok = bin(BinOp::Eq, Expr::Path(vec![k.clone(), "length".into()]), Expr::Lit(json!(n)));
                    StyleValue::Dynamic { tpl: Template { parts: vec![Part::Expr(cond(ok, Expr::Path(vec![k.clone(), i.to_string()]), Expr::Lit(json!(p.size))))] }, px: false }
                }
                None => StyleValue::Static(p.size.to_string()),
            };
            let mut style = StyleBlock::new();
            style.push("--rz-size", size);
            let mut el = cx.element("div", p.node);
            el.base_style(style).attr("id", id_with(&id, &format!("-{i}"))).attr("data-rz-panel", true);
            if p.size == 0.0 && key.is_none() {
                el.attr("data-collapsed", true);
            }
            cx.open(&el, p.node)?;
            cx.children(p.node)?;
            cx.close("div");
            if i + 1 < n {
                let mut h = cx.part("div", node, "handle");
                let label = cx.label_with("Resize {name}", &[("name", &cx.prop_string(p.node, "label").unwrap_or_else(|| name.clone()))]);
                h.attr("data-rz-handle", true)
                    .attr("role", "separator")
                    .attr("tabindex", "0")
                    // A line between side-by-side panels stands upright.
                    .attr("aria-orientation", if direction == "horizontal" { "vertical" } else { "horizontal" })
                    .attr("aria-controls", id_with(&id, &format!("-{i}")))
                    .attr("aria-label", label)
                    .attr("aria-valuemin", lit(p.min))
                    .attr("aria-valuemax", lit(p.max))
                    .attr("aria-valuenow", lit(p.size.round()));
                cx.open(&h, node)?;
                if grip {
                    let mut g = Element::new("span");
                    g.attr("data-rz-grip", true);
                    cx.open(&g, node)?;
                    cx.markup(GRIP);
                    cx.close("span");
                }
                cx.close("div");
            }
        }
        cx.close("div");
        Ok(())
    }
}

impl Component for ResizablePanel {
    fn name(&self) -> &'static str {
        "ui-resizable-panel"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "One panel of a `ui-resizable`: its children, at its size. Read by the group, never rendered on its own.",
            props: PANEL_PROPS,
            parts: &[],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, _cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        Err(RenderError::new(node, "`ui-resizable-panel` must be a direct child of `ui-resizable`"))
    }
}
