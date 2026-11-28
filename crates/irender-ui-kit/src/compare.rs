//! `ui-compare-slider`: two pictures of the same thing, before and after, one over the
//! other, with a divider dragged across to show more of either. The divider is a native
//! range input over the whole picture, so a mouse, a finger, a pen and the keyboard (the
//! arrows, Home, End, Page Up and Down) all move it, and screen readers know what it is;
//! `compare.js` keeps the picture, the spoken value and `bind` in step with it. With
//! `pictures`, three or four are side by side with a divider between each two; with
//! `zoom`, both (or all) are magnified and moved together, the dividers staying put.

use crate::{forms, lit, widget};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, StyleValue, Val, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("compare.js");

const PROPS: &[PropDef] = &[
    PropDef::new("before", PropKind::Any, "The first picture's address (or use `pictures`)"),
    PropDef::new("after", PropKind::Any, "The second picture's address, the same size and framing"),
    PropDef::new(
        "pictures",
        PropKind::Any,
        "In place of `before` and `after`: two to four pictures, `{ src, alt, label }`, side by side with a divider between each two (e.g. before, during, after)",
    ),
    PropDef::new("zoom", PropKind::Any, "Magnify all the pictures together: `true` (up to 4×) or the most, 2–10. Ctrl or ⌘ and the wheel, a pinch, a double click, the + and − keys or the buttons; dragging moves the view, and a divider when it's near one"),
    PropDef::new("bindZoom", PropKind::StateKey, "With `zoom`: state key holding how much it's magnified (1 is not at all), both ways"),
    PropDef::new("beforeAlt", PropKind::Any, "What the first picture shows, for screen readers"),
    PropDef::new("afterAlt", PropKind::Any, "What the second picture shows, for screen readers"),
    PropDef::new("beforeLabel", PropKind::Any, "On the first picture's side (default `Before`); empty for none"),
    PropDef::new("afterLabel", PropKind::Any, "On the second picture's side (default `After`); empty for none"),
    PropDef::new("position", PropKind::Any, "Where the divider starts, 0–100 from the first side (default 50); with `pictures`, a list, one a divider (default: evenly apart)"),
    PropDef::new("bind", PropKind::StateKey, "State key holding the divider's place, 0–100, both ways; with three or four `pictures`, a list of them"),
    PropDef::new("orientation", PropKind::Enum(&["horizontal", "vertical"]), "`horizontal` (default: the divider moves left and right) or `vertical` (up and down)"),
    PropDef::new("mode", PropKind::Enum(&["split", "fade"]), "`split` (default: a divider shows one side of each) or `fade`: the second picture fades in over the first as the slider moves"),
    PropDef::new("hover", PropKind::Bool, "The divider follows a mouse without pressing"),
    PropDef::new("ratio", PropKind::Any, "Width to height, e.g. `16/9` or `1`: both pictures fill it, cropped (default: the first picture's own)"),
    PropDef::new("caption", PropKind::Any, "Text under the pictures"),
    PropDef::new("label", PropKind::Any, "Names the comparison for screen readers (default `Before and after`)"),
];

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

/// The divider's place, clamped: a layer's and a line's own (`--at`, which is `--pos`,
/// `--pos2` or `--pos3`).
const POS: &str = "min(100, max(0, var(--at, 50)))";

static COMPARE: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("--at", "var(--pos, 50)"), ("display", "flex"), ("flex-direction", "column"), ("gap", gap!(8)), ("max-width", "100%")])
        .nest(
            "& [data-cmp-frame]",
            pairs(&[("position", "relative"), ("overflow", "hidden"), ("border-radius", "var(--radius-card, var(--radius-lg))"), ("background", "color-mix(in srgb, var(--color-text) 6%, transparent)"), ("user-select", "none")]),
        )
        .nest(
            "& [data-cmp-frame] img",
            pairs(&[
                ("display", "block"),
                ("width", "100%"),
                ("height", "auto"),
                ("pointer-events", "none"),
                // Zoomed, every picture alike; the layers' clips stay where the dividers are.
                ("transform-origin", "0 0"),
                ("transform", "translate(calc(var(--zx, 0) * 1px), calc(var(--zy, 0) * 1px)) scale(var(--zs, 1))"),
            ]),
        )
        .nest("& [data-cmp-after] img", pairs(&[("position", "absolute"), ("inset", "0"), ("height", "100%"), ("object-fit", "cover")]))
        .nest("& [data-cmp-frame][data-own-drag]", pairs(&[("touch-action", "pan-y")]))
        .nest("&[data-vertical] [data-cmp-frame][data-own-drag]", pairs(&[("touch-action", "pan-x")]))
        .nest("& [data-cmp-frame][data-own-drag] input", pairs(&[("pointer-events", "none")]))
        .nest("& [data-cmp-frame][data-zoomed]", pairs(&[("touch-action", "none"), ("cursor", "grab")]))
        .nest("& [data-cmp-frame][data-panning]", pairs(&[("cursor", "grabbing")]))
        .nest("& [data-cmp-frame][data-own-drag]:not([data-zoomed])", pairs(&[("cursor", "ew-resize")]))
        .nest("&[data-vertical] [data-cmp-frame][data-own-drag]:not([data-zoomed])", pairs(&[("cursor", "ns-resize")]))
        .nest(
            "& [data-cmp-zoombar]",
            pairs(&[
                ("position", "absolute"),
                ("bottom", "10px"),
                ("inset-inline-end", "10px"),
                ("display", "flex"),
                ("align-items", "center"),
                ("gap", "2px"),
                ("padding", "2px"),
                ("border-radius", "var(--radius-md)"),
                ("background", "rgb(0 0 0 / 0.55)"),
                ("color", "#fff"),
                ("font-size", fs!(12)),
                ("font-variant-numeric", "tabular-nums"),
                ("z-index", "1"),
            ]),
        )
        .nest(
            "& [data-cmp-zoombar] button",
            pairs(&[("display", "grid"), ("place-items", "center"), ("width", "26px"), ("height", "26px"), ("border", "0"), ("border-radius", "var(--radius-sm)"), ("background", "transparent"), ("color", "inherit"), ("cursor", "pointer")]),
        )
        .nest("& [data-cmp-zoombar] button:hover", pairs(&[("background", "rgb(255 255 255 / 0.18)")]))
        .nest("& [data-cmp-zoombar] button:disabled", pairs(&[("opacity", "0.4"), ("cursor", "default")]))
        .nest("& [data-cmp-zoombar] button:focus-visible", forms::focus_ring())
        .nest("& [data-cmp-zoombar] output", pairs(&[("min-width", "3.2em"), ("text-align", "center")]))
        // With a ratio, both fill it; without, the first sets the size and the second covers it.
        .nest("& [data-cmp-frame][data-ratio] img", pairs(&[("position", "absolute"), ("inset", "0"), ("height", "100%"), ("object-fit", "cover")]))
        .nest("& [data-cmp-after]", pairs(&[("position", "absolute"), ("inset", "0"), ("height", "100%"), ("object-fit", "cover")]))
        .nest("&:not([data-vertical]) [data-cmp-after]", pairs(&[("clip-path", &format!("inset(0 0 0 calc({POS} * 1%))"))]))
        .nest("&[data-vertical] [data-cmp-after]", pairs(&[("clip-path", &format!("inset(calc({POS} * 1%) 0 0 0)"))]))
        // The divider: a line with a round handle.
        .nest("& [data-cmp-line]", pairs(&[("position", "absolute"), ("background", "#fff"), ("box-shadow", "0 0 0 1px rgb(0 0 0 / 0.2), 0 0 8px rgb(0 0 0 / 0.3)"), ("pointer-events", "none")]))
        .nest("&:not([data-vertical]) [data-cmp-line]", pairs(&[("top", "0"), ("bottom", "0"), ("width", "2px"), ("left", &format!("calc({POS} * 1%)")), ("transform", "translateX(-50%)")]))
        .nest("&[data-vertical] [data-cmp-line]", pairs(&[("left", "0"), ("right", "0"), ("height", "2px"), ("top", &format!("calc({POS} * 1%)")), ("transform", "translateY(-50%)")]))
        .nest(
            "& [data-cmp-handle]",
            pairs(&[
                ("position", "absolute"),
                ("top", "50%"),
                ("left", "50%"),
                ("display", "grid"),
                ("place-items", "center"),
                ("width", "38px"),
                ("height", "38px"),
                ("border-radius", "50%"),
                ("background", "#fff"),
                ("color", "#18181b"),
                ("box-shadow", "0 2px 8px rgb(0 0 0 / 0.3)"),
                ("transform", "translate(-50%, -50%)"),
            ]),
        )
        .nest("&[data-vertical] [data-cmp-handle] svg", pairs(&[("transform", "rotate(90deg)")]))
        .nest("& [data-cmp-frame]:has(input:focus-visible) [data-cmp-handle]", forms::focus_ring())
        // The range input over the whole picture: invisible, but what's dragged and focused.
        .nest(
            "& [data-cmp-frame] input",
            pairs(&[("position", "absolute"), ("inset", "0"), ("width", "100%"), ("height", "100%"), ("margin", "0"), ("opacity", "0"), ("cursor", "ew-resize"), ("touch-action", "pan-y")]),
        )
        .nest("&[data-vertical] [data-cmp-frame] input", pairs(&[("writing-mode", "vertical-lr"), ("cursor", "ns-resize"), ("touch-action", "pan-x")]))
        .nest(
            "& [data-cmp-tag]",
            pairs(&[
                ("position", "absolute"),
                ("padding", pad!(2, 8)),
                ("border-radius", "var(--radius-sm)"),
                ("background", "rgb(0 0 0 / 0.55)"),
                ("color", "#fff"),
                ("font-size", fs!(12)),
                ("font-weight", "var(--font-weight-medium, 500)"),
                ("pointer-events", "none"),
                ("transition", "opacity .15s"),
            ]),
        )
        .nest("& [data-cmp-tag=before]", pairs(&[("top", "10px"), ("inset-inline-start", "10px")]))
        .nest("&:not([data-vertical]) [data-cmp-tag=after]", pairs(&[("top", "10px"), ("inset-inline-end", "10px")]))
        .nest("&[data-vertical] [data-cmp-tag=after]", pairs(&[("bottom", "10px"), ("inset-inline-start", "10px")]))
        // Three or four: each label at the start of its picture's part.
        .nest("&[data-many]:not([data-vertical]) [data-cmp-tag=after]", pairs(&[("inset-inline-end", "auto"), ("inset-inline-start", &format!("calc({POS} * 1% + 10px)"))]))
        .nest("&[data-many][data-vertical] [data-cmp-tag=after]", pairs(&[("bottom", "auto"), ("top", &format!("calc({POS} * 1% + 10px)"))]))
        // A label fades as the divider covers its side.
        .nest("&[data-near=before] [data-cmp-tag=before], &[data-near=after] [data-cmp-tag=after]", pairs(&[("opacity", "0")]))
        // Right to left, the first picture is on the right, as the range runs.
        .nest("&:dir(rtl):not([data-vertical]) [data-cmp-after]", pairs(&[("clip-path", &format!("inset(0 calc({POS} * 1%) 0 0)"))]))
        .nest("&:dir(rtl):not([data-vertical]) [data-cmp-line]", pairs(&[("left", &format!("calc(100% - {POS} * 1%)"))]))
        .nest("& figcaption", pairs(&[("color", "var(--color-muted)"), ("font-size", fs!(13))]))
        // Fading: the second picture whole, as opaque as the slider is far along, and a bar
        // along the foot for where it is.
        .nest("&[data-fade] [data-cmp-after]", pairs(&[("clip-path", "none"), ("opacity", &format!("calc({POS} / 100)"))]))
        .nest("&[data-fade]:dir(rtl) [data-cmp-after]", pairs(&[("clip-path", "none")]))
        .nest(
            "&[data-fade] [data-cmp-line]",
            pairs(&[("top", "auto"), ("bottom", "0"), ("left", "0"), ("right", "auto"), ("height", "4px"), ("width", &format!("calc({POS} * 1%)")), ("transform", "none"), ("background", "var(--color-primary)"), ("box-shadow", "none")]),
        )
        .nest("&[data-fade]:dir(rtl) [data-cmp-line]", pairs(&[("left", "auto"), ("right", "0")]))
        .nest("&[data-fade] [data-cmp-handle]", pairs(&[("display", "none")]))
        .nest("&[data-fade] [data-cmp-tag]", pairs(&[("transition", "opacity .1s")]))
        .nest("&[data-fade] [data-cmp-tag=before]", pairs(&[("opacity", &format!("calc(1 - {POS} / 100)"))]))
        .nest("&[data-fade] [data-cmp-tag=after]", pairs(&[("opacity", &format!("calc({POS} / 100)"))]))
});

const ARROWS: &str = r#"<svg width="18" height="18" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M5.5 4.5 2 8l3.5 3.5M10.5 4.5 14 8l-3.5 3.5"/></svg>"#;

pub struct CompareSlider;

impl Component for CompareSlider {
    fn name(&self) -> &'static str {
        "ui-compare-slider"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Two pictures, before and after, one over the other, with a divider dragged across (or moved with the keys) to show more of either.",
            props: PROPS,
            parts: &[],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let pictures = pictures(cx, node)?;
        let n = pictures.len();
        let bind = cx.prop_string(node, "bind").filter(|k| !k.is_empty());
        let start = match &bind {
            Some(k) => cx.lookup(std::slice::from_ref(k)),
            None => cx.prop(node, "position").unwrap_or(Json::Null),
        };
        // One place a divider, in order; evenly apart when not given.
        let even = |k: usize| ((k as f64) * 100.0 / (n as f64)).round();
        let places: Vec<f64> = match &start {
            Json::Null => (1..n).map(even).collect(),
            Json::Array(l) if n > 2 => l.iter().map(expr::to_num).collect(),
            v if n == 2 => vec![expr::to_num(v)],
            _ => return Err(RenderError::new(node, format!("with {n} pictures, the dividers' places (`position`, or what `bind` holds) are a list of {}", n - 1))),
        };
        if places.len() != n - 1 || places.iter().any(|p| !(0.0..=100.0).contains(p)) || places.windows(2).any(|w| w[0] > w[1]) {
            return Err(RenderError::new(node, "the divider's place (`position`, or what `bind` holds) is from 0 to 100, one a divider, in order"));
        }
        let ratio = match cx.prop(node, "ratio").filter(|v| !v.is_null()) {
            None => None,
            Some(v) => {
                let text = expr::stringify(&v);
                let ok = text.split('/').map(str::trim).all(|p| p.parse::<f64>().is_ok_and(|n| n > 0.0)) && text.split('/').count() <= 2;
                if !ok {
                    return Err(RenderError::new(node, "`ratio` is width to height: `16/9`, `4/3`, `1`"));
                }
                Some(text.replace(' ', ""))
            }
        };
        let fade = cx.prop_string(node, "mode").as_deref() == Some("fade");
        let vertical = !fade && cx.prop_string(node, "orientation").as_deref() == Some("vertical");
        let hover = widget::flag(cx, node, "hover", false);
        if n > 2 && (fade || hover) {
            return Err(RenderError::new(node, format!("`{}` takes two pictures", if fade { "mode: fade" } else { "hover" })));
        }
        let zoom = match cx.prop(node, "zoom") {
            None | Some(Json::Null) | Some(Json::Bool(false)) => None,
            Some(Json::Bool(true)) => Some(4.0),
            Some(v) => match expr::to_num(&v) {
                z if (2.0..=10.0).contains(&z) => Some(z),
                _ => return Err(RenderError::new(node, "`zoom` is `true` or the most it magnifies, 2 to 10")),
            },
        };
        widget::kit(cx);
        cx.global_js("ui-compare-slider", SCRIPT);

        let mut style = StyleBlock::new();
        for (k, p) in places.iter().enumerate() {
            style.push(&pos_var(k), StyleValue::Static(p.to_string()));
        }
        let mut root = cx.element("figure", node);
        root.base_style_ref(&COMPARE)
            .base_style(style)
            .base_style(StyleBlock::new().set("margin", "0"))
            .behavior("ui-compare-slider", json!({ "bind": bind, "hover": hover, "vertical": vertical, "zoom": zoom, "bindZoom": cx.prop_string(node, "bindZoom").filter(|k| !k.is_empty()) }))
            .attr("role", "group")
            // Labels: "Comparison", "Before and after"
            .attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label(if n > 2 { "Comparison" } else { "Before and after" }))));
        if vertical {
            root.attr("data-vertical", true);
        }
        if fade {
            root.attr("data-fade", true);
        }
        if n > 2 {
            root.attr("data-many", true);
        }
        cx.open(&root, node)?;
        let mut frame = Element::new("div");
        frame.attr("data-cmp-frame", true);
        // Its own dragging (the inputs left to the keys): to pick the nearest of several
        // dividers, and to move a zoomed view.
        if n > 2 || zoom.is_some() {
            frame.attr("data-own-drag", true);
        }
        if let Some(r) = &ratio {
            frame.attr("data-ratio", true).base_style(StyleBlock::new().set("aspect-ratio", r.clone()));
        }
        cx.open(&frame, node)?;
        // The first picture sets the size; each after it is a layer from its divider on.
        for (i, pic) in pictures.iter().enumerate() {
            let mut img = Element::new("img");
            img.attr("src", pic.src.clone()).attr("alt", pic.alt.clone().unwrap_or_else(|| lit(""))).attr("draggable", "false").attr("decoding", "async");
            if i == 0 {
                img.attr("data-cmp-before", true);
                cx.open(&img, node)?;
                continue;
            }
            let mut layer = Element::new("span");
            layer.attr("data-cmp-after", i.to_string());
            if i > 1 {
                layer.base_style(StyleBlock::new().set("--at", format!("var({})", pos_var(i - 1))));
            }
            cx.open(&layer, node)?;
            cx.open(&img, node)?;
            cx.close("span");
        }
        for (i, pic) in pictures.iter().enumerate() {
            if matches!(&pic.label, Val::Lit(Json::String(s)) if s.is_empty()) {
                continue;
            }
            let mut tag = Element::new("span");
            tag.attr("data-cmp-tag", if i == 0 { "before" } else { "after" }).attr("aria-hidden", "true").text(pic.label.clone());
            if i > 1 {
                tag.base_style(StyleBlock::new().set("--at", format!("var({})", pos_var(i - 1))));
            }
            cx.open(&tag, node)?;
            cx.close("span");
        }
        for k in 0..n - 1 {
            let mut line = Element::new("span");
            line.attr("data-cmp-line", true).attr("aria-hidden", "true");
            if k > 0 {
                line.base_style(StyleBlock::new().set("--at", format!("var({})", pos_var(k))));
            }
            cx.open(&line, node)?;
            let mut handle = Element::new("span");
            handle.attr("data-cmp-handle", true);
            cx.open(&handle, node)?;
            cx.markup(ARROWS);
            cx.close("span");
            cx.close("span");
        }
        for (k, p) in places.iter().enumerate() {
            let mut input = Element::new("input");
            input.attr("type", "range").attr("min", "0").attr("max", "100").attr("step", "1").attr("value", p.to_string());
            // Labels: "Divider {n}"
            let name = if n > 2 { cx.label_with("Divider {n}", &[("n", &(k + 1).to_string())]) } else { cx.label("Divider") };
            input.attr("aria-label", name).attr("aria-orientation", if vertical { "vertical" } else { "horizontal" });
            cx.open(&input, node)?;
        }
        cx.close("div");
        if let Some(c) = node.prop("caption") {
            let mut cap = Element::new("figcaption");
            cap.text(c.clone());
            cx.open(&cap, node)?;
            cx.close("figcaption");
        }
        cx.close("figure");
        Ok(())
    }
}

/// `--pos`, `--pos2`, `--pos3`: divider `k`'s place.
fn pos_var(k: usize) -> String {
    if k == 0 { "--pos".into() } else { format!("--pos{}", k + 1) }
}

/// A picture to compare, from `pictures` or `before` and `after`.
struct Picture {
    src: Val,
    alt: Option<Val>,
    label: Val,
}

fn pictures(cx: &RenderCx<'_>, node: &Node) -> Result<Vec<Picture>, RenderError> {
    let Some(list) = node.prop("pictures") else {
        let (Some(before), Some(after)) = (node.prop("before"), node.prop("after")) else {
            return Err(RenderError::new(node, "needs `before` and `after`, or `pictures`"));
        };
        return Ok(vec![
            Picture { src: before.clone(), alt: node.prop("beforeAlt").cloned(), label: node.prop("beforeLabel").cloned().unwrap_or_else(|| lit(cx.label("Before"))) },
            Picture { src: after.clone(), alt: node.prop("afterAlt").cloned(), label: node.prop("afterLabel").cloned().unwrap_or_else(|| lit(cx.label("After"))) },
        ]);
    };
    if node.prop("before").is_some() || node.prop("after").is_some() {
        return Err(RenderError::new(node, "`pictures` takes the place of `before` and `after`: give one or the other"));
    }
    let items = match list {
        Val::Lit(Json::Array(items)) if (2..=4).contains(&items.len()) => items,
        _ => return Err(RenderError::new(node, "`pictures` is a list of two to four `{ src, alt, label }`")),
    };
    items
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let val = |v: &Json| Val::parse(v).map_err(|e| RenderError::new(node, format!("picture {}: {e}", i + 1)));
            let get = |k: &str| p.get(k).filter(|v| !v.is_null()).map(val).transpose();
            let src = match p {
                Json::String(_) => val(p)?,
                _ => get("src")?.ok_or_else(|| RenderError::new(node, format!("picture {}: needs `src`", i + 1)))?,
            };
            Ok(Picture { src, alt: get("alt")?, label: get("label")?.unwrap_or_else(|| lit("")) })
        })
        .collect()
}
