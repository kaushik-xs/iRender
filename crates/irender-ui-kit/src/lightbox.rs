//! `ui-lightbox`: pictures as a grid, a row or a masonry, opened one at a time over the
//! page to zoom, move, and step through. The server draws the thumbnails as links to the
//! pictures, so they work without a script; `lightbox.js` opens them in place.

use crate::{
    MUTED, SUBTLE, TEXT, each_item, expr_val, forms, items, lit, path,
    widget::{self, flag, pairs},
};
use irender_core::{
    ComponentInfo, Node, PropDef, PropKind, StyleBlock,
    expr::{BinOp, Expr},
};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("lightbox.js");
const IMAGE: &str = "ui_image";
const INDEX: &str = "ui_image_index";

pub struct Lightbox;

const PROPS: &[PropDef] = &[
    PropDef::new("images", PropKind::Any, "`[{ src, thumb, alt, caption, download }]`, or addresses; may use state").required(),
    PropDef::new("layout", PropKind::Enum(&["grid", "row", "masonry", "stack"]), "`grid` (default), `row` (scrolls sideways), `masonry`, or `stack`: the first large, the rest counted on it"),
    PropDef::new("minWidth", PropKind::Number, "The narrowest a thumbnail gets, in px (default 140)"),
    PropDef::new("ratio", PropKind::Any, "A thumbnail's shape, e.g. `1` or `\"4 / 3\"` (default 1; `masonry` keeps each picture's own)"),
    PropDef::new("max", PropKind::Number, "Thumbnails shown; the last says how many more there are"),
    PropDef::new("captions", PropKind::Bool, "Captions under the thumbnails too"),
    PropDef::new("download", PropKind::Bool, "A button saves the picture shown"),
    PropDef::new("bind", PropKind::StateKey, "State key for the open picture's place, from 0 (null: closed); set it to open one"),
    PropDef::new("label", PropKind::Any, "Names the pictures for screen readers (default `Images`)"),
];

static ROOT: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("display", "grid"), ("grid-template-columns", "repeat(auto-fill, minmax(min(var(--lb-min, 140px), 100%), 1fr))"), ("gap", gap!(8)), ("min-width", "0"), ("color", TEXT), ("font-size", fs!(14))])
        .nest("&[data-layout=row]", pairs(&[("display", "flex"), ("overflow-x", "auto"), ("scroll-snap-type", "x proximity")]))
        .nest("&[data-layout=row] [data-lb-figure]", pairs(&[("flex", "0 0 var(--lb-min, 140px)"), ("scroll-snap-align", "start")]))
        .nest("&[data-layout=masonry]", pairs(&[("display", "block"), ("columns", "var(--lb-min, 140px)"), ("column-gap", gap!(8))]))
        .nest("&[data-layout=masonry] [data-lb-figure]", pairs(&[("break-inside", "avoid"), ("margin-bottom", gap!(8))]))
        .nest("&[data-layout=masonry] [data-lb-open]", pairs(&[("aspect-ratio", "auto")]))
        .nest("&[data-layout=stack]", pairs(&[("grid-template-columns", "minmax(0, 1fr)")]))
        .nest("&[data-layout=stack] [data-lb-figure]:nth-of-type(n + 2)", pairs(&[("display", "none")]))
        .nest("& [data-lb-figure]", pairs(&[("margin", "0"), ("min-width", "0")]))
        .nest("& [data-lb-figure][hidden]", pairs(&[("display", "none")]))
        .nest("& [data-lb-figure] > figcaption", pairs(&[("margin-top", gap!(4)), ("color", MUTED), ("font-size", fs!(13))]))
        .nest("& [data-lb-open]", pairs(&[("position", "relative"), ("display", "block"), ("overflow", "hidden"), ("aspect-ratio", "var(--lb-ratio, 1)"), ("border-radius", "var(--radius-md)"), ("background", SUBTLE), ("cursor", "zoom-in")]))
        .nest("& [data-lb-open] img", pairs(&[("display", "block"), ("width", "100%"), ("height", "100%"), ("object-fit", "cover"), ("transition", "transform .25s")]))
        .nest("& [data-lb-open]:hover img", pairs(&[("transform", "scale(1.03)")]))
        .nest("& [data-lb-open]:focus-visible", forms::focus_ring())
        .nest("& [data-lb-more]", pairs(&[("position", "absolute"), ("inset", "0"), ("display", "grid"), ("place-items", "center"), ("background", "rgb(0 0 0 / 0.55)"), ("color", "#fff"), ("font-size", fs!(20)), ("font-weight", "var(--font-weight-semibold, 600)")]))
        .nest("@media (prefers-reduced-motion: reduce)", pairs(&[]).nest("& img", pairs(&[("transition", "none !important")])))
        // Over the page: always dark, whatever the theme.
        .nest(
            "& dialog",
            pairs(&[("position", "fixed"), ("inset", "0"), ("width", "100vw"), ("height", "100dvh"), ("max-width", "none"), ("max-height", "none"), ("margin", "0"), ("padding", "0"), ("border", "0"), ("background", "rgb(10 10 12 / 0.94)"), ("color", "#fff"), ("overflow", "hidden")]),
        )
        .nest("& dialog[open]", pairs(&[("display", "grid"), ("grid-template-rows", "auto minmax(0, 1fr) auto")]))
        .nest("& dialog::backdrop", pairs(&[("background", "rgb(0 0 0 / 0.5)")]))
        .nest("& [data-lb-bar]", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", gap!(4)), ("padding", pad!(8, 12))]))
        .nest("& [data-lb-count]", pairs(&[("margin-inline-end", "auto"), ("color", "rgb(255 255 255 / 0.75)"), ("font-variant-numeric", "tabular-nums")]))
        .nest("& dialog :is(button, a[data-lb-save])", pairs(&[("display", "inline-grid"), ("place-items", "center"), ("width", "36px"), ("height", "36px"), ("padding", "0"), ("border", "0"), ("border-radius", "50%"), ("background", "rgb(255 255 255 / 0.1)"), ("color", "#fff"), ("cursor", "pointer")]))
        .nest("& dialog :is(button, a):hover", pairs(&[("background", "rgb(255 255 255 / 0.22)")]))
        .nest("& dialog :is(button, a):focus-visible", pairs(&[("outline", "2px solid #fff"), ("outline-offset", "2px")]))
        .nest("& dialog button:disabled", pairs(&[("opacity", "0.35"), ("cursor", "default")]))
        .nest("& [data-lb-stage]", pairs(&[("position", "relative"), ("overflow", "hidden"), ("touch-action", "none"), ("display", "grid"), ("place-items", "center"), ("cursor", "grab")]))
        .nest("& [data-lb-stage]:active", pairs(&[("cursor", "grabbing")]))
        .nest("& [data-lb-stage] > img", pairs(&[("max-width", "100%"), ("max-height", "100%"), ("user-select", "none"), ("-webkit-user-drag", "none")]))
        .nest("& [data-lb-stage] > img[data-loading]", pairs(&[("opacity", "0.3")]))
        .nest("& dialog [data-lb-nav]", pairs(&[("position", "absolute"), ("top", "calc(50% - 20px)"), ("z-index", "2"), ("width", "40px"), ("height", "40px")]))
        .nest("& [data-lb-nav='-1']", pairs(&[("inset-inline-start", "12px")]))
        .nest("& [data-lb-nav='1']", pairs(&[("inset-inline-end", "12px")]))
        .nest("& [data-lb-foot]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("align-items", "center"), ("gap", gap!(8)), ("padding", pad!(8, 12, 12))]))
        .nest("& [data-lb-caption]", pairs(&[("max-width", "70ch"), ("text-align", "center"), ("line-height", "1.4")]))
        .nest("& [data-lb-caption]:empty", pairs(&[("display", "none")]))
        .nest("& [data-lb-strip]", pairs(&[("display", "flex"), ("gap", gap!(6)), ("max-width", "100%"), ("overflow-x", "auto"), ("padding", "4px"), ("scrollbar-width", "none")]))
        .nest("& dialog [data-lb-strip] button", pairs(&[("flex", "none"), ("width", "52px"), ("height", "40px"), ("overflow", "hidden"), ("border-radius", "6px"), ("opacity", "0.55")]))
        .nest("& dialog [data-lb-strip] button[aria-current=true]", pairs(&[("opacity", "1"), ("box-shadow", "0 0 0 2px #fff")]))
        .nest("& [data-lb-strip] img", pairs(&[("width", "100%"), ("height", "100%"), ("object-fit", "cover")]))
        .nest("@media (max-width: 640px)", pairs(&[]).nest("& dialog [data-lb-nav]", pairs(&[("display", "none")])))
});

/// A picture's field, or the picture itself when it's only an address.
fn of(field: &str) -> Expr {
    crate::cond(crate::bin(BinOp::Ne, path(&[IMAGE, "src"]), Expr::Lit(Json::Null)), path(&[IMAGE, field]), if field == "src" { path(&[IMAGE]) } else { Expr::Lit(Json::Null) })
}

impl Component for Lightbox {
    fn name(&self) -> &'static str {
        "ui-lightbox"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Pictures that open over the page: step with the arrows or a swipe, zoom with the wheel, a pinch or a double click, and save.", props: PROPS, parts: &[], children: false, open_props: false }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut config = widget::config(cx, node, &["images"], &["max", "layout"], &["bind"])?;
        config.insert("download".into(), json!(flag(cx, node, "download", false)));
        let layout = cx.prop_string(node, "layout").unwrap_or_else(|| "grid".into());
        widget::kit(cx);
        cx.global_js("ui-panzoom", include_str!("panzoom.js"));
        cx.global_js("ui-lightbox", SCRIPT);
        let mut style = StyleBlock::new().set("--lb-min", format!("{}px", widget::number(cx, node, "minWidth").unwrap_or(140.0).clamp(40.0, 800.0)));
        if let Some(r) = node.prop("ratio")
            && let Some(v) = cx.style_value(node, "aspect-ratio", r)?
        {
            style.push("--lb-ratio", v);
        }
        let captions = flag(cx, node, "captions", false);
        let max = widget::number(cx, node, "max").map(|n| n.max(1.0) as u64).or((layout == "stack").then_some(1));
        let (images, live) = items(cx, node, "images")?;
        let total = images.len() as u64;
        let mut el = cx.element("div", node);
        el.base_style_ref(&ROOT)
            .base_style(style)
            .attr("role", "group")
            .attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Images"))))
            .attr("data-layout", layout)
            .behavior("ui-lightbox", Json::Object(config));
        cx.open(&el, node)?;
        // The thumbnails are in a list of their own, which takes no space of its own.
        let mut list = Element::new("div");
        list.attr("data-lb-list", true).base_style(pairs(&[("display", "contents")]));
        let key = expr_val(of("src"));
        each_item(cx, node, list, (images, live), IMAGE, INDEX, &key, |cx| {
            let mut fig = Element::new("figure");
            fig.attr("data-lb-figure", true);
            if let Some(m) = max {
                fig.attr("hidden", expr_val(crate::bin(BinOp::Ge, path(&[INDEX]), crate::num(m))));
            }
            cx.open(&fig, node)?;
            let mut a = Element::new("a");
            a.attr("data-lb-open", true).attr("href", expr_val(of("src"))).attr("target", "_blank").attr("rel", "noreferrer");
            cx.open(&a, node)?;
            let mut img = Element::new("img");
            img.attr("src", expr_val(crate::cond(of("thumb"), of("thumb"), of("src")))).attr("alt", expr_val(crate::cond(of("alt"), of("alt"), crate::cond(of("caption"), of("caption"), Expr::Lit(json!("")))))).attr("loading", "lazy");
            cx.open(&img, node)?;
            if let Some(m) = max {
                let more = crate::bin(BinOp::And, crate::bin(BinOp::Eq, path(&[INDEX]), crate::num(m - 1)), crate::num(total.saturating_sub(m)));
                crate::branch(cx, &expr_val(more), |cx| {
                    let mut s = Element::new("span");
                    s.attr("data-lb-more", true).text(lit(format!("+{}", total.saturating_sub(m))));
                    cx.open(&s, node)?;
                    cx.close("span");
                    Ok(())
                })?;
            }
            cx.close("a");
            if captions {
                crate::branch(cx, &expr_val(of("caption")), |cx| {
                    let mut c = Element::new("figcaption");
                    c.text(expr_val(of("caption")));
                    cx.open(&c, node)?;
                    cx.close("figcaption");
                    Ok(())
                })?;
            }
            cx.close("figure");
            Ok(())
        })?;
        cx.close("div");
        cx.close("div");
        Ok(())
    }
}
