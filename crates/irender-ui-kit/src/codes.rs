//! `ui-qr-code`, `ui-barcode` and `ui-code-scanner`. The codes are worked out and drawn
//! as SVG by `codes.js`, without libraries; the scanner uses the browser's own detector.

use crate::{
    BORDER, MUTED, SUBTLE, TEXT, forms, instance_id, lit,
    widget::{self, flag, pairs},
};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const ZXING: &str = "https://cdn.jsdelivr.net/npm/zxing-wasm@3.1.6/dist/iife/reader/index.js";
const ZXING_WASM: &str = "https://cdn.jsdelivr.net/npm/zxing-wasm@3.1.6/dist/reader/zxing_reader.wasm";
const SCRIPT: &str = include_str!("codes.js");

pub struct QrCode;
pub struct Barcode;
pub struct CodeScanner;

const QR_PROPS: &[PropDef] = &[
    PropDef::new("value", PropKind::Any, "What it holds: an address or any text; may use state").required(),
    PropDef::new("size", PropKind::Any, "Width and height: px or a CSS length (default 160)"),
    PropDef::new("level", PropKind::Enum(&["L", "M", "Q", "H"]), "How much of it can be damaged and still read: 7, 15 (default), 25 or 30%"),
    PropDef::new("logo", PropKind::Any, "A picture in the middle: an address (a path, http(s) or `data:`), best an SVG or a square PNG; may use state. Uses level H, so the code still reads"),
    PropDef::new("logoIcon", PropKind::Any, "Or an icon (with the icon kit), e.g. `star`"),
    PropDef::new("logoText", PropKind::Any, "Or a few letters, e.g. initials; may use state"),
    PropDef::new("logoSize", PropKind::Number, "Its width as a part of the code's, 0.1 to 0.3 (default 0.22)"),
    PropDef::new("logoShape", PropKind::Enum(&["rounded", "circle", "square"]), "Of the space around it, and what it's cut to (default `rounded`)"),
    PropDef::new("logoBackground", PropKind::Any, "Behind it (default: the code's background); `none` leaves only the space"),
    PropDef::new("logoColor", PropKind::Any, "Of an icon or letters (default: the modules')"),
    PropDef::new("dots", PropKind::Enum(&["square", "round"]), "Default `square`"),
    PropDef::new("color", PropKind::Any, "Of the modules (default black)"),
    PropDef::new("background", PropKind::Any, "Behind them (default white); codes need to be dark on light to be read"),
    PropDef::new("download", PropKind::String, "A button saves it under this name, ending in `.svg` or `.png`"),
    PropDef::new("label", PropKind::Any, "Names it for screen readers (default `QR code: {value}`)"),
];
const KINDS: &[&str] = &["code128", "code39", "ean13", "ean8", "upca", "itf"];
const BAR_PROPS: &[PropDef] = &[
    PropDef::new("value", PropKind::Any, "What it holds; may use state. A missing check digit (EAN, UPC) is added").required(),
    PropDef::new("type", PropKind::Enum(KINDS), "Default `code128`"),
    PropDef::new("height", PropKind::Number, "Of the bars in px (default 64)"),
    PropDef::new("barWidth", PropKind::Number, "Of the narrowest bar in px (default 2)"),
    PropDef::new("showText", PropKind::Bool, "The value under it (default true)"),
    PropDef::new("download", PropKind::String, "A button saves it under this name, ending in `.svg` or `.png`"),
    PropDef::new("label", PropKind::Any, "Names it for screen readers (default `Barcode: {value}`)"),
];
const SCAN_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key that gets the last code read, as text").required(),
    PropDef::new("bindCode", PropKind::StateKey, "State key that gets `{ value, format }`"),
    PropDef::new("bindList", PropKind::StateKey, "State key that collects each different code read, a list of texts"),
    PropDef::new("formats", PropKind::Any, "Kinds of codes, e.g. `[qr_code, ean_13, code_128]` (default all)"),
    PropDef::new("continuous", PropKind::Bool, "Keeps reading after a code"),
    PropDef::new("manual", PropKind::Bool, "A field to type the code in, for cameras and browsers that can't read it (default true)"),
    PropDef::new("sound", PropKind::Bool, "A short beep when one is read"),
    PropDef::new("picture", PropKind::Bool, "A code can be read from a picture on the device too (default true)"),
    PropDef::new("decoder", PropKind::Enum(&["auto", "native", "wasm"]), "`auto` (default) is the browser's own where it has one, else a decoder in WebAssembly, loaded the first time (Safari and Firefox)"),
    PropDef::new("library", PropKind::String, "Where that decoder's script is (default: zxing-wasm on jsDelivr, a fixed version)"),
    PropDef::new("wasm", PropKind::String, "Where its `.wasm` is"),
    PropDef::new("label", PropKind::Any, "The button that starts the camera (default `Scan a code`)"),
];

static CODE: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("display", "inline-flex"), ("flex-direction", "column"), ("align-items", "center"), ("gap", gap!(8)), ("max-width", "100%"), ("color", TEXT), ("font-size", fs!(14))])
        .nest("& svg[data-code]", pairs(&[("display", "block"), ("max-width", "100%"), ("border-radius", "4px")]))
        .nest("& [data-code-error]", pairs(&[("display", "grid"), ("place-items", "center"), ("box-sizing", "border-box"), ("padding", "8px"), ("border", "1px dashed"), ("border-color", BORDER), ("border-radius", "var(--radius-sm)"), ("color", MUTED), ("font-size", fs!(12)), ("text-align", "center")]))
        .nest("& [data-code-save]", pairs(&[("padding", pad!(4, 8)), ("border", "0"), ("border-radius", "var(--radius-sm)"), ("background", "none"), ("color", crate::PRIMARY), ("font", "inherit"), ("font-size", fs!(13)), ("cursor", "pointer")]))
        .nest("& [data-code-save]:hover", pairs(&[("background", SUBTLE)]))
        .nest("& [data-code-save]:focus-visible", forms::focus_ring())
});

static SCANNER: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(10)), ("width", "100%"), ("max-width", "420px"), ("color", TEXT), ("font-size", fs!(14))])
        .nest("& [hidden]", pairs(&[("display", "none !important")]))
        .nest("& [data-scan-view]", pairs(&[("position", "relative"), ("overflow", "hidden"), ("aspect-ratio", "4 / 3"), ("border-radius", "var(--radius-card, var(--radius-md))"), ("background", "#000")]))
        .nest("& video", pairs(&[("width", "100%"), ("height", "100%"), ("object-fit", "cover")]))
        .nest("& [data-scan-view]::after", pairs(&[("content", "''"), ("position", "absolute"), ("inset", "18% 14%"), ("border", "2px solid rgb(255 255 255 / 0.85)"), ("border-radius", "12px"), ("box-shadow", "0 0 0 999px rgb(0 0 0 / 0.35)"), ("pointer-events", "none")]))
        .nest("& [data-scan-view][data-hit]::after", pairs(&[("border-color", "#22c55e"), ("box-shadow", "0 0 0 999px rgb(34 197 94 / 0.25)")]))
        .nest("& [data-scan-tools]", pairs(&[("position", "absolute"), ("z-index", "1"), ("bottom", "8px"), ("inset-inline-end", "8px"), ("display", "flex"), ("gap", "6px")]))
        .nest("& [data-scan-tools] button", pairs(&[("height", "32px"), ("padding", "0 12px"), ("border", "0"), ("border-radius", "999px"), ("background", "rgb(0 0 0 / 0.6)"), ("color", "#fff"), ("font", "inherit"), ("font-size", fs!(13)), ("cursor", "pointer")]))
        .nest("& [data-scan-row]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("gap", gap!(8))]))
        .nest("& [data-scan-row] input", pairs(&[("flex", "1 1 140px"), ("width", "auto")]))
        .nest("& [data-scan-tools] button[aria-pressed=true]", pairs(&[("background", "#facc15"), ("color", "#18181b")]))
        .nest("& [data-scan-picture]", pairs(&[("position", "relative"), ("cursor", "pointer")]))
        .nest("& [data-scan-picture] input[type=file]", pairs(&[("position", "absolute"), ("flex", "none"), ("width", "1px"), ("height", "1px"), ("opacity", "0")]))
        .nest("& [data-scan-picture]:focus-within", forms::focus_ring())
        .nest("& [data-scan-note]", pairs(&[("margin", "0"), ("color", MUTED), ("font-size", fs!(13))]))
        .nest("& [data-scan-note]:empty", pairs(&[("display", "none")]))
});

fn check_download(cx: &RenderCx<'_>, node: &Node) -> Result<(), RenderError> {
    match cx.prop_string(node, "download") {
        Some(d) if !d.ends_with(".svg") && !d.ends_with(".png") => Err(RenderError::new(node, format!("`download` must end in .svg or .png, got `{d}`"))),
        _ => Ok(()),
    }
}

/// The element a code is drawn in; what it holds is there as text until it is.
fn code(cx: &mut RenderCx<'_>, node: &Node, name: &'static str, config: serde_json::Map<String, Json>, extra: StyleBlock, icon: Option<&irender_core::Val>) -> Result<(), RenderError> {
    check_download(cx, node)?;
    widget::kit(cx);
    crate::sheets(cx);
    cx.global_js("ui-codes", SCRIPT);
    let mut el = cx.element("div", node);
    el.base_style_ref(&CODE).base_style(extra).behavior(name, Json::Object(config));
    cx.open(&el, node)?;
    let text = node.prop("value").map(|v| expr::stringify(&cx.eval(v))).unwrap_or_default();
    let mut s = Element::new("span");
    s.attr("data-code-text", true).text(lit(text));
    cx.open(&s, node)?;
    cx.close("span");
    if let Some(i) = icon {
        let mut t = Element::new("template");
        t.attr("data-code-icon", true);
        cx.open(&t, node)?;
        crate::icon_kit(cx, node, i, 24)?;
        cx.close("template");
    }
    cx.close("div");
    Ok(())
}

impl Component for QrCode {
    fn name(&self) -> &'static str {
        "ui-qr-code"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "A QR code of any text, drawn in the page without a service, with a logo in it if there's one.", props: QR_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let config = widget::config(cx, node, &["value", "label", "logo", "logoText", "color", "background", "logoBackground", "logoColor"], &["level", "dots", "download", "logoSize", "logoShape"], &[])?;
        let given = ["logo", "logoIcon", "logoText"].iter().filter(|p| node.prop(p).is_some()).count();
        if given > 1 {
            return Err(RenderError::new(node, "give one of `logo`, `logoIcon` and `logoText`"));
        }
        if let Some(s) = widget::number(cx, node, "logoSize")
            && !(0.1..=0.3).contains(&s)
        {
            return Err(RenderError::new(node, "`logoSize` must be from 0.1 to 0.3: a larger one would hide too much of the code to read it"));
        }
        // An icon is drawn here, in a template the script copies into the code.
        let icon = match node.prop("logoIcon") {
            None => None,
            Some(i) => Some(crate::icon_name(cx, i).ok_or_else(|| RenderError::new(node, "`logoIcon` is an icon's name, and needs the icon kit"))?),
        };
        let size = node.prop("size").cloned().unwrap_or_else(|| lit(160));
        let mut s = StyleBlock::new();
        if let Some(v) = cx.style_value(node, "width", &size)? {
            s.push("--qr", v);
        }
        s = s.nest("& svg[data-code], & [data-code-error]", pairs(&[("width", "var(--qr)"), ("height", "auto"), ("aspect-ratio", "1")]));
        code(cx, node, "ui-qr-code", config, s, icon.as_ref())
    }
}

impl Component for Barcode {
    fn name(&self) -> &'static str {
        "ui-barcode"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "A barcode: Code 128, Code 39, EAN, UPC or ITF.", props: BAR_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut config = widget::config(cx, node, &["value", "label"], &["type", "height", "barWidth", "download"], &[])?;
        config.insert("showText".into(), json!(flag(cx, node, "showText", true)));
        code(cx, node, "ui-barcode", config, StyleBlock::new(), None)
    }
}

impl Component for CodeScanner {
    fn name(&self) -> &'static str {
        "ui-code-scanner"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Reads QR codes and barcodes with the camera in any browser, from a picture, or from a field (a handheld scanner types into it).", props: SCAN_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        if cx.prop_string(node, "bind").is_none() {
            return Err(RenderError::new(node, "`ui-code-scanner` needs `bind`"));
        }
        for u in ["library", "wasm"] {
            widget::check_url(cx, node, u)?;
        }
        let mut config = widget::config(cx, node, &[], &["formats", "decoder"], &["bind", "bindCode", "bindList"])?;
        config.insert("library".into(), json!(cx.prop_string(node, "library").unwrap_or_else(|| ZXING.into())));
        config.insert("wasm".into(), json!(cx.prop_string(node, "wasm").unwrap_or_else(|| ZXING_WASM.into())));
        config.insert("continuous".into(), json!(flag(cx, node, "continuous", false)));
        config.insert("sound".into(), json!(flag(cx, node, "sound", false)));
        widget::device_kit(cx);
        cx.global_js("ui-codes", SCRIPT);
        let mut el = cx.element("div", node);
        el.base_style_ref(&SCANNER).behavior("ui-code-scanner", Json::Object(config));
        cx.open(&el, node)?;
        let mut view = Element::new("div");
        view.attr("data-scan-view", true).attr("hidden", true);
        cx.open(&view, node)?;
        let mut v = Element::new("video");
        v.attr("playsinline", true).attr("muted", true).attr("aria-label", cx.label("Camera"));
        cx.open(&v, node)?;
        cx.close("video");
        let mut tools = Element::new("div");
        tools.attr("data-scan-tools", true);
        cx.open(&tools, node)?;
        // Labels: "Light", "Switch camera", "Stop"
        for (attr, text) in [("data-scan-torch", "Light"), ("data-scan-flip", "Switch camera"), ("data-scan-stop", "Stop")] {
            let mut b = Element::new("button");
            if attr == "data-scan-torch" {
                b.attr("aria-pressed", "false").attr("hidden", true);
            }
            b.attr("type", "button").attr(attr, true).text(lit(cx.label(text)));
            cx.open(&b, node)?;
            cx.close("button");
        }
        cx.close("div");
        cx.close("div");
        let mut row = Element::new("div");
        row.attr("data-scan-row", true);
        cx.open(&row, node)?;
        let mut b = Element::new("button");
        forms::button_look(&mut b, "default", "default");
        b.attr("type", "button").attr("data-scan-start", true).text(node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Scan a code"))));
        cx.open(&b, node)?;
        cx.close("button");
        if flag(cx, node, "picture", true) {
            let mut l = Element::new("label");
            forms::button_look(&mut l, "outline", "default");
            l.attr("data-scan-picture", true);
            cx.open(&l, node)?;
            let mut i = Element::new("input");
            i.attr("type", "file").attr("accept", "image/*").attr("data-scan-pick", true);
            cx.open(&i, node)?;
            cx.text(&cx.label("From a picture"));
            cx.close("label");
        }
        if flag(cx, node, "manual", true) {
            let mut i = Element::new("input");
            i.base_style_ref(forms::input_look())
                .attr("type", "text")
                .attr("id", instance_id(cx, node, "scan"))
                .attr("autocomplete", "off")
                .attr("data-scan-input", true)
                .attr("placeholder", cx.label("Or type it and press Enter"))
                .attr("aria-label", cx.label("Code"));
            cx.open(&i, node)?;
        }
        cx.close("div");
        let mut p = Element::new("p");
        p.attr("data-scan-note", true).attr("role", "status");
        cx.open(&p, node)?;
        cx.close("p");
        cx.close("div");
        Ok(())
    }
}
