//! `ui-color-picker`: a colour, picked on a saturation and brightness area with a hue
//! slider (and an opacity one), typed as any CSS colour, taken from the screen with the
//! eyedropper where the browser has one, or chosen from swatches. A button with the
//! colour opens it in a popover; `inline` shows it on the page.
//!
//! The server renders the frame (the button with the colour as it stands, the panel's
//! controls, a hidden form input); `color.js` (shipped once per page through
//! `global_js`) keeps them in step with state.

use crate::{BG_POPOVER, MUTED, RING, SUBTLE, TEXT, forms, id_with, instance_id, lit};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("color.js");

pub(crate) const FORMATS: &[&str] = &["hex", "rgb", "hsl"];

const PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key holding the colour as text in `format` (`#1e3a8a`), or null"),
    PropDef::new("value", PropKind::Any, "Starting colour, when not bound"),
    PropDef::new("format", PropKind::Enum(FORMATS), "How the colour is written: `hex` (default, `#1e3a8a`, `#1e3a8a80` with opacity), `rgb` (`rgb(30, 58, 138)`) or `hsl` (`hsl(224, 64%, 33%)`)"),
    PropDef::new("alpha", PropKind::Bool, "An opacity slider too"),
    PropDef::new("swatches", PropKind::Any, "Colours to pick with one click: CSS colours, or `{ value, label }`"),
    PropDef::new("recent", PropKind::Any, "The colours picked last, to pick again with one click: `true` (8) or how many; kept in the browser"),
    PropDef::new("recentKey", PropKind::String, "Which recent colours: pickers with the same key share them (default: all pickers share one list)"),
    PropDef::new("contrastWith", PropKind::Any, "A background (or text) colour to check the contrast against, as WCAG reads it: the ratio and AA / AAA; may use state"),
    PropDef::new("custom", PropKind::Bool, "The area, sliders and text field (default true); false leaves just the swatches"),
    PropDef::new("eyedropper", PropKind::Bool, "A button to take a colour from the screen, where the browser has one (default true)"),
    PropDef::new("inline", PropKind::Bool, "The panel on the page, without the button and popover"),
    PropDef::new("placeholder", PropKind::Any, "The button's text while there's no colour (default `Pick a colour`)"),
    PropDef::new("clearable", PropKind::Bool, "A button that clears the colour (to null)"),
    PropDef::new("align", PropKind::Enum(&["start", "end"]), "The popover's edge lined up with the button (default `start`)"),
    PropDef::new("name", PropKind::String, "Form field name: a hidden input carries the colour"),
    PropDef::new("label", PropKind::Any, "Accessible name"),
    PropDef::new("disabled", PropKind::Any, "Boolean or expression"),
    PropDef::new("id", PropKind::Any, "Element id (default: derived from the document)"),
];

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

/// A checkerboard under colours that may be see-through.
const CHECKS: &str = "repeating-conic-gradient(color-mix(in srgb, var(--color-text) 14%, transparent) 0 25%, transparent 0 50%) 0 0 / 8px 8px";

/// A range input's track and thumb, its track painted by `--track`.
fn slider(block: StyleBlock, sel: &str) -> StyleBlock {
    let thumb = [
        ("appearance", "none"),
        ("width", "16px"),
        ("height", "16px"),
        ("border", "2px solid #fff"),
        ("border-radius", "50%"),
        ("background", "transparent"),
        ("box-shadow", "0 0 0 1px rgb(0 0 0 / 0.3), inset 0 0 0 1px rgb(0 0 0 / 0.3)"),
        ("cursor", "grab"),
    ];
    block
        .nest(
            sel,
            pairs(&[("appearance", "none"), ("display", "block"), ("width", "100%"), ("height", "12px"), ("margin", "0"), ("border-radius", "999px"), ("background", "var(--track)"), ("cursor", "pointer")]),
        )
        .nest(format!("{sel}::-webkit-slider-thumb"), pairs(&thumb))
        .nest(format!("{sel}::-moz-range-thumb"), pairs(&thumb))
        .nest(format!("{sel}:focus-visible"), pairs(&[("outline", "2px solid var(--color-ring, var(--color-muted))"), ("outline-offset", "2px")]))
}

/// The panel, inline or in the popover.
fn panel_style() -> StyleBlock {
    let s = pairs(&[("color", TEXT), ("font-size", fs!(14))])
        .nest("& [data-color-panel]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(10)), ("width", sp!(240))]))
        .nest(
            "& [data-color-area]",
            pairs(&[
                ("position", "relative"),
                ("height", sp!(150)),
                ("border-radius", "var(--radius-sm)"),
                // White to the right's hue across, black up from the foot.
                ("background", "linear-gradient(to top, #000, transparent), linear-gradient(to right, #fff, transparent), hsl(var(--hue, 0) 100% 50%)"),
                ("cursor", "crosshair"),
                ("touch-action", "none"),
            ]),
        )
        .nest("& [data-color-area]:focus-visible", pairs(&[("outline", "2px solid var(--color-ring, var(--color-muted))"), ("outline-offset", "2px")]))
        .nest(
            "& [data-color-thumb]",
            pairs(&[
                ("position", "absolute"),
                ("left", "calc(var(--sat, 0) * 1%)"),
                ("top", "calc(100% - var(--bri, 0) * 1%)"),
                ("width", "14px"),
                ("height", "14px"),
                ("border", "2px solid #fff"),
                ("border-radius", "50%"),
                ("box-shadow", "0 0 0 1px rgb(0 0 0 / 0.3), inset 0 0 0 1px rgb(0 0 0 / 0.3)"),
                ("transform", "translate(-50%, -50%)"),
                ("pointer-events", "none"),
            ]),
        )
        .nest("& [data-color-hue]", pairs(&[("--track", "linear-gradient(to right, #f00, #ff0, #0f0, #0ff, #00f, #f0f, #f00)")]))
        .nest("& [data-color-alpha]", pairs(&[("--track", concat!("linear-gradient(to right, transparent, var(--opaque, #000)), ", "repeating-conic-gradient(color-mix(in srgb, var(--color-text) 14%, transparent) 0 25%, transparent 0 50%) 0 0 / 8px 8px"))]))
        .nest("& [data-color-row]", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", gap!(6))]))
        .nest(
            "& [data-color-text]",
            pairs(&[
                ("flex", "1"),
                ("min-width", "0"),
                ("height", ctl!(32)),
                ("padding", pad!(0, 8)),
                ("border", "var(--border-width, 1px) solid var(--color-border)"),
                ("border-radius", "var(--radius-control, var(--radius-md))"),
                ("background", "transparent"),
                ("color", TEXT),
                ("font", "inherit"),
                ("font-family", "var(--font-mono, ui-monospace, monospace)"),
                ("font-size", fs!(13)),
            ]),
        )
        .nest("& [data-color-text]:focus-visible", pairs(&[("outline", "none"), ("border-color", RING)]))
        .nest("& [data-color-text][aria-invalid=true]", pairs(&[("border-color", "var(--color-destructive, #dc2626)")]))
        .nest(
            "& [data-color-row] button, & [data-color-clear]",
            pairs(&[
                ("display", "inline-flex"),
                ("align-items", "center"),
                ("justify-content", "center"),
                ("flex-shrink", "0"),
                ("height", ctl!(32)),
                ("min-width", ctl!(32)),
                ("padding", pad!(0, 6)),
                ("border", "var(--border-width, 1px) solid var(--color-border)"),
                ("border-radius", "var(--radius-control, var(--radius-md))"),
                ("background", "transparent"),
                ("color", "inherit"),
                ("font", "inherit"),
                ("cursor", "pointer"),
            ]),
        )
        .nest("& [data-color-row] button:hover, & [data-color-clear]:hover", pairs(&[("background", SUBTLE)]))
        .nest("& [data-color-eyedropper][hidden]", pairs(&[("display", "none")]))
        .nest("& [data-color-swatches]", pairs(&[("display", "grid"), ("grid-template-columns", "repeat(auto-fill, minmax(24px, 1fr))"), ("gap", gap!(6))]))
        .nest(
            "& [data-color-swatches] button",
            pairs(&[
                ("aspect-ratio", "1"),
                ("padding", "0"),
                ("border", "var(--border-width, 1px) solid color-mix(in srgb, var(--color-text) 15%, transparent)"),
                ("border-radius", "var(--radius-sm)"),
                ("background", "var(--swatch), var(--checks)"),
                ("cursor", "pointer"),
            ]),
        )
        .nest("& [data-color-swatches] button:hover", pairs(&[("transform", "scale(1.1)")]))
        .nest("& [data-color-swatches] button:focus-visible", pairs(&[("outline", "2px solid var(--color-ring, var(--color-muted))"), ("outline-offset", "2px")]))
        .nest("& [data-color-recent][hidden]", pairs(&[("display", "none")]))
        .nest("& [data-color-contrast]", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", gap!(6)), ("margin", "0"), ("font-size", fs!(12)), ("color", MUTED), ("font-variant-numeric", "tabular-nums")]))
        .nest("& [data-color-contrast]:empty", pairs(&[("display", "none")]))
        .nest("& [data-color-contrast] [data-sample]", pairs(&[("display", "inline-grid"), ("place-items", "center"), ("width", "22px"), ("height", "22px"), ("border-radius", "var(--radius-sm)"), ("border", "1px solid var(--color-border)"), ("font-weight", "700"), ("font-size", fs!(12))]))
        .nest("& [data-color-contrast] [data-grade]", pairs(&[("margin-inline-start", "auto"), ("padding", pad!(1, 6)), ("border-radius", "999px"), ("font-weight", "600")]))
        .nest("& [data-color-contrast] [data-grade=fail]", pairs(&[("color", "var(--color-destructive, #dc2626)"), ("background", "color-mix(in srgb, var(--color-destructive, #dc2626) 12%, transparent)")]))
        .nest("& [data-color-contrast] [data-grade]:not([data-grade=fail])", pairs(&[("color", "var(--color-success, #15803d)"), ("background", "color-mix(in srgb, var(--color-success, #15803d) 12%, transparent)")]))
        .nest("& [data-color-swatches] button[aria-pressed=true]", pairs(&[("box-shadow", "0 0 0 2px var(--ui-bg, var(--color-surface)), 0 0 0 4px var(--color-text)")]))
        .nest(
            "& [data-color-live]",
            pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("overflow", "hidden"), ("clip-path", "inset(50%)"), ("white-space", "nowrap")]),
        )
        .set("--checks", CHECKS);
    let s = slider(s, "& [data-color-hue]");
    slider(s, "& [data-color-alpha]")
}

static INLINE: LazyLock<StyleBlock> = LazyLock::new(|| {
    panel_style()
        .set("display", "inline-block")
        .set("position", "relative")
        .set("padding", pad!(12))
        .set("border", "var(--border-width, 1px) solid var(--color-border)")
        .set("border-radius", "var(--radius-control, var(--radius-md))")
        .set("--ui-bg", "var(--color-card, var(--color-surface))")
        .set("background", "var(--ui-bg)")
});

static PICKER: LazyLock<StyleBlock> = LazyLock::new(|| {
    panel_style()
        .set("position", "relative")
        .set("display", "inline-flex")
        .set("width", "100%")
        .set("min-width", "0")
        .nest("& [data-color-trigger]", pairs(&[("width", "100%"), ("min-width", "0"), ("justify-content", "flex-start"), ("font-weight", "400"), ("padding-inline-end", pad!(32))]))
        .nest(
            "& [data-color-trigger] [data-color-chip]",
            pairs(&[
                ("flex-shrink", "0"),
                ("width", "18px"),
                ("height", "18px"),
                ("border-radius", "var(--radius-sm)"),
                ("border", "var(--border-width, 1px) solid color-mix(in srgb, var(--color-text) 15%, transparent)"),
                ("background", "var(--swatch, transparent), var(--checks)"),
            ]),
        )
        .nest("& [data-color-value]", pairs(&[("min-width", "0"), ("overflow", "hidden"), ("text-overflow", "ellipsis"), ("white-space", "nowrap"), ("font-variant-numeric", "tabular-nums")]))
        .nest("&[data-empty] [data-color-value]", pairs(&[("color", MUTED)]))
        .nest(
            "& [data-color-x]",
            pairs(&[
                ("position", "absolute"),
                ("inset-inline-end", pad!(6)),
                ("top", "50%"),
                ("transform", "translateY(-50%)"),
                ("display", "inline-flex"),
                ("align-items", "center"),
                ("justify-content", "center"),
                ("width", "22px"),
                ("height", "22px"),
                ("padding", "0"),
                ("border", "0"),
                ("border-radius", "var(--radius-sm)"),
                ("background", "transparent"),
                ("color", MUTED),
                ("font-size", fs!(11)),
                ("cursor", "pointer"),
            ]),
        )
        .nest("& [data-color-x]:hover", pairs(&[("background", SUBTLE), ("color", TEXT)]))
        .nest("& [data-color-x][hidden]", pairs(&[("display", "none")]))
        .nest(
            "& [data-color-pop]",
            pairs(&[
                ("box-sizing", "border-box"),
                ("inset", "auto"),
                ("margin", concat!(gap!(6), " 0")),
                ("padding", pad!(12)),
                ("border", "var(--border-width, 1px) solid var(--color-border)"),
                ("border-radius", "var(--radius-card, var(--radius-md))"),
                ("--ui-bg", BG_POPOVER),
                ("background", BG_POPOVER),
                ("color", TEXT),
                ("box-shadow", "var(--elevation-popover, 0 6px 18px rgb(0 0 0 / 0.12))"),
                ("position-try-fallbacks", "flip-block, flip-inline"),
            ]),
        )
});

const EYEDROPPER: &str = r#"<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m10.5 2.5 3 3M12 4l-1.5-1.5a1.4 1.4 0 0 0-2 0L7.5 3.5l5 5 1-1a1.4 1.4 0 0 0 0-2L12 4ZM9 5 3 11v2h2l6-6"/></svg>"#;

/// The swatches as `{ value, label }`, after checking them.
fn swatches(cx: &RenderCx<'_>, node: &Node) -> Result<Vec<Json>, RenderError> {
    match cx.prop(node, "swatches") {
        None | Some(Json::Null) => Ok(Vec::new()),
        Some(Json::Array(list)) => list
            .into_iter()
            .map(|s| match s {
                Json::String(v) if !v.trim().is_empty() => Ok(json!({ "value": v, "label": v })),
                Json::Object(o) if o.get("value").is_some_and(Json::is_string) => {
                    let label = o.get("label").cloned().unwrap_or_else(|| o["value"].clone());
                    Ok(json!({ "value": o["value"], "label": label }))
                }
                _ => Err(RenderError::new(node, "each swatch is a CSS colour or `{ value, label }`")),
            })
            .collect(),
        Some(_) => Err(RenderError::new(node, "`swatches` must be a list")),
    }
}

pub struct ColorPicker;

impl Component for ColorPicker {
    fn name(&self) -> &'static str {
        "ui-color-picker"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A colour: picked on an area with hue and opacity sliders, typed as any CSS colour, taken from the screen, or chosen from swatches. A button opens it in a popover; `inline` shows it on the page.",
            props: PROPS,
            parts: &["trigger", "input", "popover"],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let s = |name: &str| cx.prop_string(node, name).filter(|v| !v.is_empty());
        let flag = |name: &str, dflt: bool| cx.prop(node, name).filter(|v| !v.is_null()).map_or(dflt, |v| expr::truthy(&v));
        let format = s("format").unwrap_or_else(|| "hex".into());
        if !FORMATS.contains(&format.as_str()) {
            return Err(RenderError::new(node, format!("`format` must be one of {}, got `{format}`", FORMATS.join(", "))));
        }
        let swatches = swatches(cx, node)?;
        let custom = flag("custom", true);
        if !custom && swatches.is_empty() {
            return Err(RenderError::new(node, "`custom: false` leaves only the swatches, so it needs `swatches`"));
        }
        let inline = flag("inline", false);
        let value = match s("bind") {
            Some(k) => cx.lookup(&[k]),
            None => cx.prop(node, "value").unwrap_or(Json::Null),
        };
        let text = match &value {
            Json::String(v) if !v.trim().is_empty() => Some(v.clone()),
            _ => None,
        };
        let (bind, name, placeholder, align) = (s("bind"), s("name"), s("placeholder"), s("align"));
        let (alpha, eyedropper, clearable) = (flag("alpha", false), flag("eyedropper", true), flag("clearable", false));
        let recent = match cx.prop(node, "recent") {
            None | Some(Json::Null) | Some(Json::Bool(false)) => 0,
            Some(Json::Bool(true)) => 8,
            Some(v) => (expr::to_num(&v).clamp(0.0, 24.0)) as usize,
        };
        let contrast = node.prop("contrastWith").map(|v| crate::data_table::dynamic(cx, node, "contrastWith", v)).transpose()?;
        let config = json!({
            "recent": recent,
            "recentKey": s("recentKey"),
            "contrastWith": contrast,
            "bind": bind,
            "value": if bind.is_none() { value.clone() } else { Json::Null },
            "format": format,
            "alpha": alpha,
            "swatches": swatches,
            "custom": custom,
            "name": name,
            "popover": !inline,
        });
        crate::widget::kit(cx);
        cx.global_js("ui-color-picker", SCRIPT);
        let id = instance_id(cx, node, "color-picker");
        let label = node.prop("label").cloned();

        let mut root = cx.element("div", node);
        root.base_style_ref(if inline { &INLINE } else { &PICKER }).behavior("ui-color-picker", config);
        if text.is_none() {
            root.attr("data-empty", true);
        }
        if inline {
            root.attr("role", "group").attr("aria-label", label.clone().unwrap_or_else(|| lit(cx.label("Colour"))));
            cx.open(&root, node)?;
            panel(cx, node, Some(&id), &swatches, custom, alpha, eyedropper, clearable, recent > 0, contrast.is_some())?;
        } else {
            crate::anchors(cx);
            crate::flips(cx);
            cx.open(&root, node)?;
            let pop = id_with(&id, "-panel");
            let anchor = format!("--{}", expr::stringify(&cx.eval(&id)));
            let placeholder = placeholder.unwrap_or_else(|| cx.label("Pick a colour"));
            let mut trigger = cx.part("button", node, "trigger");
            forms::button_look(&mut trigger, "outline", "default");
            trigger
                .base_style(StyleBlock::new().set("anchor-name", anchor.clone()))
                .attr("type", "button")
                .attr("id", id.clone())
                .attr("data-color-trigger", true)
                .attr("popovertarget", pop.clone())
                .attr("aria-haspopup", "dialog");
            if let Some(d) = node.prop("disabled") {
                trigger.attr("disabled", d.clone());
            }
            cx.open(&trigger, node)?;
            // Colours reach CSS only through the behaviour (`style.setProperty`), which
            // takes nothing that isn't one.
            let mut chip = Element::new("span");
            chip.attr("data-color-chip", true).attr("aria-hidden", "true");
            cx.open(&chip, node)?;
            cx.close("span");
            let mut shown = Element::new("span");
            shown.attr("data-color-value", true).attr("data-placeholder", placeholder.clone()).text(lit(text.clone().unwrap_or(placeholder)));
            cx.open(&shown, node)?;
            cx.close("span");
            cx.close("button");
            if clearable {
                let mut x = Element::new("button");
                x.attr("type", "button").attr("data-color-x", true).attr("aria-label", cx.label("Clear")).attr("hidden", text.is_none()).text(lit("✕"));
                cx.open(&x, node)?;
                cx.close("button");
            }
            let align = align.unwrap_or_else(|| "start".into());
            let area = if align == "end" { "bottom span-left" } else { "bottom span-right" };
            let mut popover = cx.part("div", node, "popover");
            popover
                .base_style(crate::with_area(StyleBlock::new().set("position-anchor", anchor), area))
                .attr("popover", "auto")
                .attr("id", pop)
                .attr("role", "dialog")
                .attr("data-color-pop", true)
                .attr("aria-label", label.unwrap_or_else(|| lit(cx.label("Choose a colour"))));
            cx.open(&popover, node)?;
            panel(cx, node, None, &swatches, custom, alpha, eyedropper, false, recent > 0, contrast.is_some())?;
            cx.close("div");
        }
        let mut live = Element::new("div");
        live.attr("data-color-live", true).attr("role", "status").attr("aria-live", "polite");
        cx.open(&live, node)?;
        cx.close("div");
        if let Some(name) = name {
            let mut i = Element::new("input");
            i.attr("type", "hidden").attr("name", name).attr("value", text.unwrap_or_default());
            cx.open(&i, node)?;
        }
        cx.close("div");
        Ok(())
    }
}

/// The area, sliders, text field and swatches; the behaviour sets their values. Inline,
/// the text field takes the picker's `id`, which a `ui-field`'s label points at.
#[allow(clippy::too_many_arguments)]
fn panel(cx: &mut RenderCx<'_>, node: &Node, id: Option<&irender_core::Val>, swatches: &[Json], custom: bool, alpha: bool, eyedropper: bool, clearable: bool, recent: bool, contrast: bool) -> Result<(), RenderError> {
    let mut p = Element::new("div");
    p.attr("data-color-panel", true);
    cx.open(&p, node)?;
    if custom {
        let mut area = Element::new("div");
        area.attr("data-color-area", true).attr("role", "slider").attr("tabindex", "0").attr("aria-label", cx.label("Saturation and brightness"));
        cx.open(&area, node)?;
        let mut thumb = Element::new("span");
        thumb.attr("data-color-thumb", true);
        cx.open(&thumb, node)?;
        cx.close("span");
        cx.close("div");
        let mut hue = Element::new("input");
        hue.attr("type", "range").attr("min", "0").attr("max", "360").attr("step", "1").attr("value", "0").attr("data-color-hue", true).attr("aria-label", cx.label("Hue"));
        cx.open(&hue, node)?;
        if alpha {
            let mut a = Element::new("input");
            a.attr("type", "range").attr("min", "0").attr("max", "100").attr("step", "1").attr("value", "100").attr("data-color-alpha", true).attr("aria-label", cx.label("Opacity"));
            cx.open(&a, node)?;
        }
        let mut row = Element::new("div");
        row.attr("data-color-row", true);
        cx.open(&row, node)?;
        let mut input = cx.part("input", node, "input");
        if let Some(id) = id {
            input.attr("id", id.clone());
        }
        input.attr("type", "text").attr("data-color-text", true).attr("spellcheck", "false").attr("autocomplete", "off");
        // A `ui-field` labels it inline (it gave the field its id); otherwise it's named here.
        let labelled = id.is_some() && node.parts.get("input").is_some_and(|p| p.attrs.contains_key("id"));
        if !labelled {
            input.attr("aria-label", cx.label("Colour value"));
        }
        cx.open(&input, node)?;
        if eyedropper {
            // Shown by the behaviour where the browser has an eyedropper.
            let mut b = Element::new("button");
            b.attr("type", "button").attr("data-color-eyedropper", true).attr("hidden", true).attr("aria-label", cx.label("Pick a colour from the screen"));
            cx.open(&b, node)?;
            cx.markup(EYEDROPPER);
            cx.close("button");
        }
        cx.close("div");
    }
    if !swatches.is_empty() {
        let mut list = Element::new("div");
        list.attr("data-color-swatches", true).attr("role", "group").attr("aria-label", cx.label("Swatches"));
        cx.open(&list, node)?;
        for (i, sw) in swatches.iter().enumerate() {
            let mut b = Element::new("button");
            b.attr("type", "button")
                .attr("data-swatch", i.to_string())
                .attr("aria-label", expr::stringify(&sw["label"]))
                .attr("title", expr::stringify(&sw["label"]))
                .attr("aria-pressed", "false");
            cx.open(&b, node)?;
            cx.close("button");
        }
        cx.close("div");
    }
    // The colours picked last, drawn by the behaviour; hidden until there are some.
    if recent {
        let mut list = Element::new("div");
        list.attr("data-color-swatches", true).attr("data-color-recent", true).attr("role", "group").attr("aria-label", cx.label("Recent colours")).attr("hidden", true);
        cx.open(&list, node)?;
        cx.close("div");
    }
    if contrast {
        let mut c = Element::new("p");
        c.attr("data-color-contrast", true).attr("aria-live", "polite");
        cx.open(&c, node)?;
        cx.close("p");
    }
    if clearable {
        let mut c = Element::new("button");
        c.attr("type", "button").attr("data-color-clear", true).text(lit(cx.label("Clear")));
        cx.open(&c, node)?;
        cx.close("button");
    }
    cx.close("div");
    Ok(())
}
