//! `ui-signature`, `ui-image-cropper`, `ui-plan` (things placed on a drawing, zoomed and
//! moved) and `ui-pdf` (pages drawn by pdf.js, with notes on them). `media.js` runs them.

use crate::{
    BG_POPOVER, BORDER, MUTED, PRIMARY, RING, SUBTLE, SURFACE, TEXT, forms, instance_id, lit, status,
    widget::{self, flag, pairs},
};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("media.js");
const PANZOOM: &str = include_str!("panzoom.js");

fn scripts(cx: &mut RenderCx<'_>) {
    widget::kit(cx);
    cx.global_js("ui-panzoom", PANZOOM);
    cx.global_js("ui-media", SCRIPT);
}

fn bar(b: StyleBlock) -> StyleBlock {
    b.nest("& [hidden]", pairs(&[("display", "none !important")]))
        .nest("& [data-m-bar]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("align-items", "center"), ("gap", gap!(6))]))
        .nest("& [data-m-bar] button", pairs(&[("display", "inline-flex"), ("align-items", "center"), ("justify-content", "center"), ("gap", gap!(4)), ("min-width", ctl!(32)), ("height", ctl!(32)), ("padding", pad!(0, 10)), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-control, var(--radius-md))"), ("background", SURFACE), ("color", TEXT), ("font", "inherit"), ("font-size", fs!(13)), ("cursor", "pointer")]))
        .nest("& [data-m-bar] button:hover", pairs(&[("background", SUBTLE)]))
        .nest("& [data-m-bar] button[aria-pressed=true]", pairs(&[("border-color", PRIMARY), ("background", "color-mix(in srgb, var(--color-primary) 12%, transparent)")]))
        .nest("& [data-m-bar] button:disabled", pairs(&[("opacity", "0.5"), ("cursor", "default")]))
        .nest("& button:focus-visible", forms::focus_ring())
        .nest("& [data-push]", pairs(&[("margin-inline-start", "auto")]))
}

// ---------------------------------------------------------------- signature

pub struct Signature;

const SIGN_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key for the signature as a picture (a `data:` address; null when empty)").required(),
    PropDef::new("bindMeta", PropKind::StateKey, "State key that gets `{ method: drawn|typed, name, at }`"),
    PropDef::new("methods", PropKind::Any, "How it can be made: `[draw, type]` (default both)"),
    PropDef::new("format", PropKind::Enum(&["png", "svg"]), "Default `png`"),
    PropDef::new("height", PropKind::Number, "Of the pad in px (default 160)"),
    PropDef::new("color", PropKind::Any, "The pen's (default near black)"),
    PropDef::new("hint", PropKind::Any, "Under the line (default `Sign here`)"),
    PropDef::new("label", PropKind::Any, "Names it (default `Signature`)"),
    PropDef::new("disabled", PropKind::Any, "Shown, not changed"),
];

static SIGN: LazyLock<StyleBlock> = LazyLock::new(|| {
    bar(forms::native_scheme(pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(8)), ("width", "100%"), ("max-width", "560px"), ("color", TEXT), ("font-size", fs!(14))])))
        // A paper of its own, so the ink reads in any theme.
        .nest("& [data-sg-pad]", pairs(&[("position", "relative"), ("overflow", "hidden"), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-card, var(--radius-md))"), ("background", "#fff"), ("color", "#18181b"), ("touch-action", "none")]))
        .nest("& [data-sg-pad]:focus-within", forms::focus_ring().set("border-color", RING))
        .nest("& canvas", pairs(&[("display", "block"), ("width", "100%"), ("height", "var(--sg-h, 160px)"), ("cursor", "crosshair")]))
        .nest("& [data-sg-line]", pairs(&[("position", "absolute"), ("inset-inline", "24px"), ("bottom", "36px"), ("border-bottom", "1px solid #d4d4d8"), ("pointer-events", "none")]))
        .nest("& [data-sg-hint]", pairs(&[("position", "absolute"), ("inset-inline-start", "24px"), ("bottom", "12px"), ("color", "#71717a"), ("font-size", fs!(12)), ("pointer-events", "none")]))
        .nest("& [data-sg-type]", pairs(&[("box-sizing", "border-box"), ("display", "block"), ("width", "100%"), ("height", "var(--sg-h, 160px)"), ("padding", "0 28px 24px"), ("border", "0"), ("outline", "none"), ("background", "transparent"), ("color", "#18181b"), ("font-family", "'Snell Roundhand', 'Segoe Script', 'Bradley Hand', 'Brush Script MT', 'Apple Chancery', cursive"), ("font-size", "42px"), ("text-align", "center")]))
        .nest("& [data-sg-saved]", pairs(&[("display", "block"), ("width", "100%"), ("height", "var(--sg-h, 160px)"), ("object-fit", "contain")]))
        .nest("&[data-off] [data-sg-pad]", pairs(&[("pointer-events", "none"), ("opacity", "0.7")]))
});

impl Component for Signature {
    fn name(&self) -> &'static str {
        "ui-signature"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "A signature, drawn with a finger, pen or mouse, or typed; state gets it as a picture.", props: SIGN_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        if cx.prop_string(node, "bind").is_none() {
            return Err(RenderError::new(node, "`ui-signature` needs `bind`"));
        }
        let config = widget::config(cx, node, &["disabled", "color", "hint"], &["methods", "format", "height"], &["bind", "bindMeta"])?;
        scripts(cx);
        let h = widget::number(cx, node, "height").unwrap_or(160.0).clamp(80.0, 600.0);
        let mut el = cx.element("div", node);
        el.base_style_ref(&SIGN)
            .base_style(StyleBlock::new().set("--sg-h", format!("{h}px")))
            .attr("role", "group")
            .attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Signature"))))
            .behavior("ui-signature", Json::Object(config));
        cx.open(&el, node)?;
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- cropper

pub struct ImageCropper;

const CROP_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key that gets the cropped picture (a `data:` address) when Crop is pressed").required(),
    PropDef::new("src", PropKind::Any, "The picture to crop; without it, one is picked or dropped"),
    PropDef::new("ratio", PropKind::Any, "The crop's shape: `1`, `\"16 / 9\"`, `free`, or a list to choose from (default `free`)"),
    PropDef::new("round", PropKind::Bool, "Shown through a circle, for a profile picture"),
    PropDef::new("width", PropKind::Number, "The cropped picture's width in px (default: as many as there are)"),
    PropDef::new("maxWidth", PropKind::Number, "Its most (default 2048)"),
    PropDef::new("type", PropKind::Enum(&["jpeg", "png", "webp"]), "Default `jpeg`"),
    PropDef::new("quality", PropKind::Number, "0 to 1 (default 0.9)"),
    PropDef::new("bindCrop", PropKind::StateKey, "State key that gets the crop as parts of the picture: `{ x, y, width, height, rotation }`, 0 to 1"),
    PropDef::new("height", PropKind::Number, "Of the stage in px (default 320)"),
    PropDef::new("label", PropKind::Any, "Names it (default `Crop image`)"),
];

static CROP: LazyLock<StyleBlock> = LazyLock::new(|| {
    bar(forms::native_scheme(pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(10)), ("width", "100%"), ("min-width", "0"), ("color", TEXT), ("font-size", fs!(14))])))
        .nest("& [data-cr-drop]", pairs(&[("position", "relative"), ("display", "flex"), ("flex-direction", "column"), ("align-items", "center"), ("justify-content", "center"), ("gap", gap!(6)), ("min-height", "180px"), ("border", "1.5px dashed var(--color-border)"), ("border-radius", "var(--radius-card, var(--radius-lg))"), ("color", MUTED), ("text-align", "center"), ("cursor", "pointer")]))
        .nest("& [data-cr-drop]:is(:hover, [data-over])", pairs(&[("border-color", PRIMARY), ("background", SUBTLE)]))
        .nest("& [data-cr-drop]:focus-within", forms::focus_ring())
        .nest("& [data-cr-drop] input", pairs(&[("position", "absolute"), ("inset", "0"), ("opacity", "0"), ("cursor", "pointer")]))
        .nest("& [data-cr-stage]", pairs(&[("position", "relative"), ("height", "var(--cr-h, 320px)"), ("overflow", "hidden"), ("border-radius", "var(--radius-md)"), ("background", "repeating-conic-gradient(#80808022 0 25%, transparent 0 50%) 0 0 / 18px 18px"), ("touch-action", "none"), ("user-select", "none")]))
        .nest("& [data-cr-stage] canvas", pairs(&[("position", "absolute"), ("inset", "0"), ("width", "100%"), ("height", "100%")]))
        .nest("& [data-cr-box]", pairs(&[("position", "absolute"), ("box-sizing", "border-box"), ("border", "1.5px solid #fff"), ("box-shadow", "0 0 0 9999px rgb(0 0 0 / 0.5), 0 0 0 1px rgb(0 0 0 / 0.4)"), ("cursor", "move"), ("outline", "none")]))
        .nest("& [data-cr-box]:focus-visible", pairs(&[("border-color", PRIMARY)]))
        .nest("& [data-cr-box][data-round]", pairs(&[("border-radius", "50%")]))
        .nest("& [data-cr-box]::before, & [data-cr-box]::after", pairs(&[("content", "''"), ("position", "absolute"), ("pointer-events", "none"), ("border", "0 solid rgb(255 255 255 / 0.4)")]))
        .nest("& [data-cr-box]::before", pairs(&[("inset", "0 33.3%"), ("border-width", "0 1px")]))
        .nest("& [data-cr-box]::after", pairs(&[("inset", "33.3% 0"), ("border-width", "1px 0")]))
        .nest("& [data-cr-h]", pairs(&[("position", "absolute"), ("z-index", "1"), ("width", "22px"), ("height", "22px"), ("margin", "-11px")]))
        .nest("& [data-cr-h]::after", pairs(&[("content", "''"), ("position", "absolute"), ("inset", "6px"), ("border-radius", "50%"), ("background", "#fff"), ("box-shadow", "0 0 0 1px rgb(0 0 0 / 0.5)")]))
        .nest("& input[type=range]", pairs(&[("flex", "0 1 160px"), ("min-width", "80px"), ("accent-color", PRIMARY)]))
        .nest("& [data-cr-error]", pairs(&[("margin", "0"), ("color", "var(--color-destructive, #dc2626)")]))
});

impl Component for ImageCropper {
    fn name(&self) -> &'static str {
        "ui-image-cropper"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "A picture cropped, turned and zoomed in the browser, by dragging or the keys.", props: CROP_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        if cx.prop_string(node, "bind").is_none() {
            return Err(RenderError::new(node, "`ui-image-cropper` needs `bind`"));
        }
        let mut config = widget::config(cx, node, &["src"], &["ratio", "width", "maxWidth", "type", "quality"], &["bind", "bindCrop"])?;
        config.insert("round".into(), json!(flag(cx, node, "round", false)));
        scripts(cx);
        let h = widget::number(cx, node, "height").unwrap_or(320.0).clamp(120.0, 900.0);
        let mut el = cx.element("div", node);
        el.base_style_ref(&CROP)
            .base_style(StyleBlock::new().set("--cr-h", format!("{h}px")))
            .attr("role", "group")
            .attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Crop image"))))
            .behavior("ui-image-cropper", Json::Object(config));
        cx.open(&el, node)?;
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- plan

pub struct Plan;

const PLAN_PROPS: &[PropDef] = &[
    PropDef::new("items", PropKind::Any, "What's on it: `[{ id, x, y, width, height, shape, rotation, label, sublabel, tone, color, status, points, locked }]` in the plan's units; `shape`: rect (default), circle, pin or polygon (with `points`); may use state").required(),
    PropDef::new("width", PropKind::Number, "The plan's width in its own units (default 1000)"),
    PropDef::new("height", PropKind::Number, "Its height (default 600)"),
    PropDef::new("image", PropKind::Any, "A drawing behind everything: a floor plan, a site map, a photo"),
    PropDef::new("bindItems", PropKind::StateKey, "State key the items are in: moved ones are written back"),
    PropDef::new("bind", PropKind::StateKey, "State key for the picked item's id"),
    PropDef::new("bindItem", PropKind::StateKey, "State key that gets the picked item itself"),
    PropDef::new("tones", PropKind::Any, "A tone per `status`: `{ Free: success, Occupied: danger }`"),
    PropDef::new("grid", PropKind::Number, "Lines drawn this far apart, which moved items keep to"),
    PropDef::new("filter", PropKind::Any, "Items that don't pass are faded: `{ field: condition }`"),
    PropDef::new("search", PropKind::Bool, "A box that finds an item and goes to it"),
    PropDef::new("legend", PropKind::Bool, "What the tones mean, with counts"),
    PropDef::new("viewHeight", PropKind::Any, "Its height on the page (default 420)"),
    PropDef::new("label", PropKind::Any, "Names it for screen readers (default `Plan`)"),
];

static PLAN: LazyLock<StyleBlock> = LazyLock::new(|| {
    status::tone_rules(
        bar(pairs(&[("position", "relative"), ("display", "flex"), ("flex-direction", "column"), ("gap", gap!(8)), ("min-width", "0"), ("color", TEXT), ("font-size", fs!(14))])),
        "& [data-pl-item]",
    )
    .nest("& [data-pl-frame]", pairs(&[("position", "relative"), ("height", "var(--pl-h, 420px)"), ("overflow", "hidden"), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-card, var(--radius-md))"), ("background", SUBTLE), ("touch-action", "none"), ("cursor", "grab")]))
    .nest("& [data-pl-frame]:active", pairs(&[("cursor", "grabbing")]))
    .nest("& [data-pl-sheet]", pairs(&[("position", "absolute"), ("top", "0"), ("left", "0")]))
    .nest("& svg[data-pl]", pairs(&[("display", "block"), ("overflow", "visible")]))
    .nest("& [data-pl-item]", pairs(&[("--tone", MUTED), ("cursor", "pointer"), ("outline", "none")]))
    .nest("& [data-pl-item] > :is(rect, circle, polygon, path)", pairs(&[("fill", "color-mix(in srgb, var(--tone) 22%, var(--color-surface))"), ("stroke", "var(--tone)"), ("stroke-width", "1.5"), ("vector-effect", "non-scaling-stroke"), ("transition", "fill .15s")]))
    .nest("& [data-pl-item]:hover > :is(rect, circle, polygon, path)", pairs(&[("fill", "color-mix(in srgb, var(--tone) 38%, var(--color-surface))")]))
    .nest("& [data-pl-item]:focus-visible > :is(rect, circle, polygon, path)", pairs(&[("stroke-width", "3")]))
    .nest("& [data-pl-item][aria-pressed=true] > :is(rect, circle, polygon, path)", pairs(&[("stroke-width", "3"), ("fill", "color-mix(in srgb, var(--tone) 45%, var(--color-surface))")]))
    .nest("& [data-pl-item][data-faded]", pairs(&[("opacity", "0.25")]))
    .nest("& [data-pl-item][data-dragging]", pairs(&[("opacity", "0.7")]))
    .nest("& [data-pl-item] text", pairs(&[("fill", TEXT), ("text-anchor", "middle"), ("dominant-baseline", "central"), ("pointer-events", "none"), ("font-weight", "500")]))
    .nest("& [data-pl-item] text[data-sub]", pairs(&[("fill", MUTED), ("font-weight", "400")]))
    .nest("& [data-pl-grid]", pairs(&[("stroke", BORDER), ("stroke-width", "1"), ("vector-effect", "non-scaling-stroke"), ("fill", "none")]))
    .nest("& [data-pl-zoom]", pairs(&[("position", "absolute"), ("z-index", "2"), ("bottom", "10px"), ("inset-inline-end", "10px"), ("display", "flex"), ("flex-direction", "column"), ("overflow", "hidden"), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-md)"), ("background", BG_POPOVER)]))
    .nest("& [data-pl-zoom] button", pairs(&[("display", "grid"), ("place-items", "center"), ("width", "32px"), ("height", "32px"), ("padding", "0"), ("border", "0"), ("background", "none"), ("color", TEXT), ("cursor", "pointer")]))
    .nest("& [data-pl-zoom] button:hover", pairs(&[("background", SUBTLE)]))
    .nest("& [data-pl-legend]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("gap", gap!(4, 14)), ("margin", "0"), ("padding", "0"), ("list-style", "none"), ("color", MUTED), ("font-size", fs!(13))]))
    .nest("& [data-pl-legend] li", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", gap!(6))]))
    .nest("& [data-pl-legend] i", pairs(&[("width", "10px"), ("height", "10px"), ("border-radius", "3px"), ("background", "var(--tone)")]))
    .nest("& [data-pl-search]", pairs(&[("position", "relative"), ("max-width", "300px")]))
    .nest("& [data-pl-search] ul", pairs(&[("position", "absolute"), ("z-index", "5"), ("top", "calc(100% + 4px)"), ("inset-inline", "0"), ("margin", "0"), ("padding", "4px"), ("list-style", "none"), ("max-height", "220px"), ("overflow-y", "auto"), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-md)"), ("background", BG_POPOVER), ("box-shadow", "var(--elevation-popover, 0 6px 18px rgb(0 0 0 / 0.12))")]))
    .nest("& [data-pl-search] li button", pairs(&[("width", "100%"), ("padding", pad!(6, 8)), ("border", "0"), ("border-radius", "var(--radius-sm)"), ("background", "none"), ("color", TEXT), ("font", "inherit"), ("text-align", "start"), ("cursor", "pointer")]))
    .nest("& [data-pl-search] li button:is(:hover, :focus-visible)", pairs(&[("background", SUBTLE)]))
    .nest("& [data-pl-search] small", pairs(&[("color", MUTED), ("margin-inline-start", "6px")]))
});

impl Component for Plan {
    fn name(&self) -> &'static str {
        "ui-plan"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Things placed on a drawing: rooms, seats, bays, beds, plots, machines. Zoomed and moved, picked, coloured by status, and rearranged by dragging.", props: PLAN_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut config = widget::config(cx, node, &["items", "filter", "image"], &["width", "height", "grid", "tones"], &["bindItems", "bind", "bindItem"])?;
        for f in ["search", "legend"] {
            config.insert(f.into(), json!(flag(cx, node, f, false)));
        }
        scripts(cx);
        let mut s = StyleBlock::new();
        if let Some(h) = node.prop("viewHeight")
            && let Some(v) = cx.style_value(node, "height", h)?
        {
            s.push("--pl-h", v);
        }
        let mut el = cx.element("div", node);
        el.base_style_ref(&PLAN).base_style(s).attr("role", "group").attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Plan")))).behavior("ui-plan", Json::Object(config));
        cx.open(&el, node)?;
        if flag(cx, node, "search", false) {
            let mut d = Element::new("div");
            d.attr("data-pl-search", true);
            cx.open(&d, node)?;
            let mut i = Element::new("input");
            i.base_style_ref(forms::input_look()).attr("type", "search").attr("id", instance_id(cx, node, "plan")).attr("placeholder", cx.label("Find…")).attr("aria-label", cx.label("Find"));
            cx.open(&i, node)?;
            cx.close("div");
        }
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- pdf

pub struct Pdf;

const PDF_PROPS: &[PropDef] = &[
    PropDef::new("src", PropKind::Any, "The PDF's address; may use state").required(),
    PropDef::new("bindPage", PropKind::StateKey, "State key for the page shown, from 1"),
    PropDef::new("bindPages", PropKind::StateKey, "State key that gets how many pages there are"),
    PropDef::new("notes", PropKind::Any, "What's marked on it: `[{ id, page, x, y, width, height, type, text, color, who }]`, places as parts of the page 0 to 1; `type`: highlight, box, note or signature; may use state"),
    PropDef::new("bindNotes", PropKind::StateKey, "State key the notes are in, to add, move and remove them"),
    PropDef::new("tools", PropKind::Any, "With `bindNotes`, what can be added: `[highlight, box, note, signature]` (default the first three)"),
    PropDef::new("signature", PropKind::Any, "With the `signature` tool, the picture put where it's clicked, e.g. `{{ signed }}`"),
    PropDef::new("user", PropKind::Any, "Who adds notes, kept with each"),
    PropDef::new("bindNote", PropKind::StateKey, "State key that gets the note picked"),
    PropDef::new("download", PropKind::Bool, "A link saves the file (default true)"),
    PropDef::new("library", PropKind::String, "Where pdf.js is (default: jsDelivr, a fixed version); its worker is beside it"),
    PropDef::new("height", PropKind::Any, "Its height (default 640)"),
    PropDef::new("title", PropKind::Any, "Shown in the bar, and names it"),
];

static PDF: LazyLock<StyleBlock> = LazyLock::new(|| {
    bar(forms::native_scheme(pairs(&[("display", "flex"), ("flex-direction", "column"), ("min-width", "0"), ("height", "var(--pdf-h, 640px)"), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-card, var(--radius-lg))"), ("overflow", "hidden"), ("background", SURFACE), ("color", TEXT), ("font-size", fs!(14))])))
        .nest("& [data-m-bar]", pairs(&[("padding", pad!(6, 8)), ("border-bottom", "var(--border-width, 1px) solid var(--color-border)")]))
        .nest("& [data-m-bar] :is(button, a)", pairs(&[("height", "30px"), ("min-width", "30px"), ("padding", pad!(0, 8)), ("border-color", "transparent")]))
        .nest("& [data-m-bar] a", pairs(&[("display", "inline-grid"), ("place-items", "center"), ("color", TEXT), ("border-radius", "var(--radius-sm)")]))
        .nest("& [data-pdf-title]", pairs(&[("font-weight", "var(--font-weight-medium, 500)"), ("overflow", "hidden"), ("text-overflow", "ellipsis"), ("white-space", "nowrap"), ("max-width", "30ch")]))
        .nest("& [data-m-bar] input", pairs(&[("width", "44px"), ("height", "28px"), ("border", "var(--border-width, 1px) solid var(--color-border)"), ("border-radius", "var(--radius-sm)"), ("background", "transparent"), ("color", TEXT), ("font", "inherit"), ("text-align", "center")]))
        .nest("& [data-pdf-pages]", pairs(&[("flex", "1"), ("min-height", "0"), ("overflow", "auto"), ("padding", "16px"), ("background", "color-mix(in srgb, var(--color-text) 9%, var(--color-surface))"), ("outline", "none")]))
        .nest("& [data-pdf-page]", pairs(&[("position", "relative"), ("margin", "0 auto 14px"), ("background", "#fff"), ("box-shadow", "0 1px 6px rgb(0 0 0 / 0.22)")]))
        .nest("& [data-pdf-page] canvas", pairs(&[("display", "block"), ("width", "100%"), ("height", "100%")]))
        .nest("& [data-pdf-page][data-tool]", pairs(&[("cursor", "crosshair"), ("touch-action", "none")]))
        .nest("& [data-pdf-text]", pairs(&[("position", "absolute"), ("inset", "0"), ("overflow", "hidden"), ("line-height", "1"), ("opacity", "1")]))
        .nest("& [data-pdf-text] span", pairs(&[("position", "absolute"), ("color", "transparent"), ("white-space", "pre"), ("transform-origin", "0 0"), ("cursor", "text")]))
        .nest("& [data-pdf-text] ::selection", pairs(&[("background", "rgb(37 99 235 / 0.3)")]))
        .nest("& [data-pdf-page][data-tool] [data-pdf-text]", pairs(&[("pointer-events", "none")]))
        .nest("& [data-note]", pairs(&[("--n", "#f59e0b"), ("position", "absolute"), ("z-index", "2"), ("box-sizing", "border-box"), ("padding", "0"), ("border", "0"), ("border-radius", "2px"), ("cursor", "pointer"), ("font", "inherit")]))
        .nest("& [data-note=highlight]", pairs(&[("background", "color-mix(in srgb, var(--n) 38%, transparent)"), ("mix-blend-mode", "multiply")]))
        .nest("& [data-note=box]", pairs(&[("background", "transparent"), ("border", "2px solid var(--n)")]))
        .nest("& [data-note=note]", pairs(&[("display", "grid"), ("place-items", "center"), ("width", "26px !important"), ("height", "26px !important"), ("border-radius", "50% 50% 50% 0"), ("background", "var(--n)"), ("color", "#fff"), ("font-size", "13px"), ("font-weight", "700"), ("translate", "0 -100%"), ("box-shadow", "0 1px 4px rgb(0 0 0 / 0.35)")]))
        .nest("& [data-note=signature]", pairs(&[("background", "none center / contain no-repeat")]))
        .nest("& [data-note]:is(:focus-visible, [aria-pressed=true])", pairs(&[("outline", "2px solid var(--color-ring, #2563eb)"), ("outline-offset", "2px")]))
        .nest("& [data-note-pop]", pairs(&[("position", "absolute"), ("z-index", "4"), ("display", "flex"), ("flex-direction", "column"), ("gap", "6px"), ("width", "230px"), ("padding", "10px"), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-md)"), ("background", BG_POPOVER), ("color", TEXT), ("box-shadow", "var(--elevation-popover, 0 6px 18px rgb(0 0 0 / 0.2))"), ("font-size", fs!(13))]))
        .nest("& [data-note-pop] textarea", pairs(&[("box-sizing", "border-box"), ("width", "100%"), ("min-height", "60px"), ("padding", "6px 8px"), ("border", "var(--border-width, 1px) solid var(--color-border)"), ("border-radius", "var(--radius-sm)"), ("background", "transparent"), ("color", TEXT), ("font", "inherit"), ("resize", "vertical")]))
        .nest("& [data-note-pop] small", pairs(&[("color", MUTED)]))
        .nest("& [data-note-pop] div", pairs(&[("display", "flex"), ("justify-content", "space-between"), ("gap", "6px")]))
        .nest("& [data-note-pop] button", pairs(&[("height", "28px"), ("padding", "0 10px"), ("border", "var(--border-width, 1px) solid var(--color-border)"), ("border-radius", "var(--radius-sm)"), ("background", "none"), ("color", TEXT), ("font", "inherit"), ("cursor", "pointer")]))
        .nest("& [data-pdf-none]", pairs(&[("margin", "0"), ("padding", "40px 16px"), ("color", MUTED), ("text-align", "center")]))
});

/// pdf.js, as an ES module with its worker beside it.
const PDFJS: &str = "https://cdn.jsdelivr.net/npm/pdfjs-dist@4.10.38/build/pdf.min.mjs";

impl Component for Pdf {
    fn name(&self) -> &'static str {
        "ui-pdf"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "A PDF on the page: zoomed, its text selected and found with the browser, and marked with highlights, boxes, notes and signatures. Drawn by pdf.js, loaded when there's one to show.", props: PDF_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        widget::check_url(cx, node, "src")?;
        widget::check_url(cx, node, "library")?;
        let mut config = widget::config(cx, node, &["src", "notes", "signature", "user", "title"], &["tools"], &["bindPage", "bindPages", "bindNotes", "bindNote"])?;
        config.insert("library".into(), json!(cx.prop_string(node, "library").unwrap_or_else(|| PDFJS.into())));
        config.insert("download".into(), json!(flag(cx, node, "download", true)));
        scripts(cx);
        let mut s = StyleBlock::new();
        if let Some(h) = node.prop("height")
            && let Some(v) = cx.style_value(node, "height", h)?
        {
            s.push("--pdf-h", v);
        }
        let mut el = cx.element("div", node);
        el.base_style_ref(&PDF).base_style(s).attr("role", "group").attr("aria-label", node.prop("title").cloned().unwrap_or_else(|| lit(cx.label("PDF")))).behavior("ui-pdf", Json::Object(config));
        cx.open(&el, node)?;
        // Without the script, a link to it.
        let mut a = Element::new("a");
        a.attr("href", node.prop("src").cloned().unwrap_or_else(|| lit(""))).text(node.prop("title").cloned().unwrap_or_else(|| lit(cx.label("Open the PDF"))));
        cx.open(&a, node)?;
        cx.close("a");
        cx.close("div");
        Ok(())
    }
}
