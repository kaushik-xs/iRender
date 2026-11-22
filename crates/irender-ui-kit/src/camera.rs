//! `ui-camera`, `ui-permission-manager` and `ui-photo-annotate`. `camera.js` runs them.

use crate::{
    BORDER, MUTED, PRIMARY, SUBTLE, lit,
    widget::{self, pairs},
};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock};
use irender_html::{Component, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("camera.js");

fn scripts(cx: &mut RenderCx<'_>) {
    widget::device_kit(cx);
    cx.global_js("ui-camera", SCRIPT);
}

// ---------------------------------------------------------------- camera

pub struct Camera;

const CAMERA_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key for the photo, `{ id, url, name, type, size, width, height, at }` (`url` is a `data:` address), or with `multiple` a list of them").required(),
    PropDef::new("multiple", PropKind::Bool, "Several photos"),
    PropDef::new("max", PropKind::Number, "At most this many"),
    PropDef::new("facing", PropKind::Enum(&["back", "front"]), "The camera it starts with (default `back`)"),
    PropDef::new("ratio", PropKind::Any, "The photo's shape, e.g. `1` or `\"4 / 3\"` (default the camera's)"),
    PropDef::new("maxSize", PropKind::Number, "The photo's longer side at most, in px (default 1600)"),
    PropDef::new("quality", PropKind::Number, "0 to 1 (default 0.8)"),
    PropDef::new("type", PropKind::Enum(&["jpeg", "webp", "png"]), "Default `jpeg`"),
    PropDef::new("review", PropKind::Bool, "Each photo is looked at first, to use or retake (default true, or false with `multiple`)"),
    PropDef::new("grid", PropKind::Bool, "Lines for composing start shown"),
    PropDef::new("timer", PropKind::Number, "Seconds between the button and the photo (default 0)"),
    PropDef::new("guide", PropKind::Enum(&["none", "card", "face", "square"]), "An outline to fill"),
    PropDef::new("stamp", PropKind::Any, "Written along the foot of the photo, e.g. `{{ site }} · {{ now }}`; `time` is the date and time"),
    PropDef::new("files", PropKind::Bool, "Photos can be chosen from the device too (default true)"),
    PropDef::new("autoStart", PropKind::Bool, "The camera starts by itself, where it's been allowed"),
    PropDef::new("label", PropKind::Any, "The button that starts it (default `Open the camera`)"),
    PropDef::new("disabled", PropKind::Any, "Shown, not changed"),
];

static CAMERA: LazyLock<StyleBlock> = LazyLock::new(|| {
    widget::device(widget::column(gap!(10), "560px"))
        .nest("& [data-cam-view]", pairs(&[("position", "relative"), ("overflow", "hidden"), ("aspect-ratio", "var(--cam-ratio, 4 / 3)"), ("border-radius", "var(--radius-card, var(--radius-lg))"), ("background", "#000"), ("color", "#fff"), ("touch-action", "none")]))
        .nest("& [data-cam-view] video, & [data-cam-view] > img", pairs(&[("position", "absolute"), ("inset", "0"), ("width", "100%"), ("height", "100%"), ("object-fit", "cover")]))
        .nest("& [data-cam-view] > img", pairs(&[("object-fit", "contain"), ("background", "#000")]))
        .nest("& video[data-front]", pairs(&[("scale", "-1 1")]))
        .nest("& [data-cam-grid]", pairs(&[("position", "absolute"), ("inset", "0"), ("pointer-events", "none"), ("background", "linear-gradient(to right, transparent calc(33.33% - 0.5px), rgb(255 255 255 / 0.5) 33.33%, transparent calc(33.33% + 0.5px), transparent calc(66.66% - 0.5px), rgb(255 255 255 / 0.5) 66.66%, transparent calc(66.66% + 0.5px)), linear-gradient(to bottom, transparent calc(33.33% - 0.5px), rgb(255 255 255 / 0.5) 33.33%, transparent calc(33.33% + 0.5px), transparent calc(66.66% - 0.5px), rgb(255 255 255 / 0.5) 66.66%, transparent calc(66.66% + 0.5px))")]))
        .nest("& [data-cam-guide]", pairs(&[("position", "absolute"), ("pointer-events", "none"), ("border", "2px solid rgb(255 255 255 / 0.9)"), ("box-shadow", "0 0 0 999px rgb(0 0 0 / 0.4)")]))
        .nest("& [data-cam-guide=card]", pairs(&[("inset", "0 0 64px"), ("margin", "auto"), ("width", "76%"), ("height", "fit-content"), ("aspect-ratio", "1.586"), ("border-radius", "14px")]))
        .nest("& [data-cam-guide=square]", pairs(&[("inset", "0 0 64px"), ("margin", "auto"), ("height", "68%"), ("width", "fit-content"), ("aspect-ratio", "1"), ("border-radius", "10px")]))
        .nest("& [data-cam-guide=face]", pairs(&[("inset", "0 0 64px"), ("margin", "auto"), ("height", "72%"), ("width", "fit-content"), ("aspect-ratio", "0.76"), ("border-radius", "50%")]))
        .nest("& [data-cam-flash]", pairs(&[("position", "absolute"), ("inset", "0"), ("background", "#fff"), ("opacity", "0"), ("pointer-events", "none"), ("transition", "opacity .25s")]))
        .nest("& [data-cam-count]", pairs(&[("position", "absolute"), ("inset", "0"), ("display", "grid"), ("place-items", "center"), ("font-size", "84px"), ("font-weight", "600"), ("text-shadow", "0 2px 12px rgb(0 0 0 / 0.6)"), ("pointer-events", "none")]))
        .nest("& [data-cam-top], & [data-cam-foot]", pairs(&[("position", "absolute"), ("inset-inline", "10px"), ("display", "flex"), ("align-items", "center"), ("gap", "8px")]))
        .nest("& [data-cam-top]", pairs(&[("top", "10px")]))
        .nest("& [data-cam-foot]", pairs(&[("bottom", "12px"), ("justify-content", "space-between")]))
        .nest("& [data-cam-view] button", pairs(&[("display", "inline-grid"), ("place-items", "center"), ("width", "40px"), ("height", "40px"), ("padding", "0"), ("border", "0"), ("border-radius", "50%"), ("background", "rgb(0 0 0 / 0.5)"), ("color", "#fff"), ("font", "inherit"), ("font-size", "13px"), ("cursor", "pointer")]))
        .nest("& [data-cam-view] button[aria-pressed=true]", pairs(&[("background", "#facc15"), ("color", "#18181b")]))
        .nest("& [data-cam-view] button[data-shutter]", pairs(&[("width", "62px"), ("height", "62px"), ("border", "4px solid #fff"), ("background", "rgb(255 255 255 / 0.35)")]))
        .nest("& [data-cam-view] button[data-shutter]:active", pairs(&[("background", "#fff")]))
        .nest("& [data-cam-view] button[data-wide]", pairs(&[("width", "auto"), ("padding", "0 16px"), ("border-radius", "999px"), ("font-weight", "500")]))
        .nest("& [data-cam-view] button[data-wide=use]", pairs(&[("background", "#fff"), ("color", "#18181b")]))
        .nest("& [data-cam-zoom]", pairs(&[("flex", "1"), ("min-width", "0"), ("max-width", "180px"), ("margin-inline", "auto"), ("accent-color", "#fff")]))
        .nest("& [data-cam-shots]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("gap", gap!(8)), ("margin", "0"), ("padding", "0"), ("list-style", "none")]))
        .nest("& [data-cam-shots] li", pairs(&[("position", "relative")]))
        .nest("& [data-cam-shots] img", pairs(&[("display", "block"), ("width", "84px"), ("height", "84px"), ("object-fit", "cover"), ("border", "1px solid"), ("border-color", BORDER), ("border-radius", "var(--radius-md)")]))
        .nest("& [data-cam-shots] button", pairs(&[("position", "absolute"), ("top", "-6px"), ("inset-inline-end", "-6px"), ("display", "grid"), ("place-items", "center"), ("width", "24px"), ("height", "24px"), ("padding", "0"), ("border", "0"), ("border-radius", "50%"), ("background", "#18181b"), ("color", "#fff"), ("cursor", "pointer")]))
        .nest("& label[data-w-btn] input", pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("opacity", "0")]))
        .nest("& label[data-w-btn]:focus-within", crate::forms::focus_ring())
        .nest("& label[data-w-btn]", pairs(&[("position", "relative")]))
});

impl Component for Camera {
    fn name(&self) -> &'static str {
        "ui-camera"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Takes photos: front and back camera, light, zoom, composing lines, a timer, retakes and several shots, made smaller before they're kept. Where there's no camera to use, the device's own picker.", props: CAMERA_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        widget::need(cx, node, "bind")?;
        let mut config = widget::config(cx, node, &["stamp", "label", "disabled"], &["max", "facing", "ratio", "maxSize", "quality", "type", "timer", "guide"], &["bind"])?;
        let multiple = widget::flag(cx, node, "multiple", false);
        widget::flags(cx, node, &mut config, &[("multiple", false), ("review", !multiple), ("grid", false), ("files", true), ("autoStart", false)]);
        scripts(cx);
        widget::drawn(cx, node, "div", &CAMERA, "ui-camera", config, vec![("role", lit("group")), ("aria-label", lit(cx.label("Camera")))])
    }
}

// ---------------------------------------------------------------- permissions

pub struct PermissionManager;

const KINDS: &[&str] = &["camera", "microphone", "geolocation", "notifications", "motion", "storage", "clipboard", "nfc"];
const PERMISSION_PROPS: &[PropDef] = &[
    PropDef::new("permissions", PropKind::Any, "What's used: names, or `{ name, label, reason, required }`: `camera`, `microphone`, `geolocation`, `notifications`, `motion`, `storage` (kept when space is short), `clipboard` or `nfc`").required(),
    PropDef::new("bind", PropKind::StateKey, "State key that gets each one's answer, e.g. `{ camera: granted, geolocation: prompt }`: `granted`, `denied`, `prompt` or `unsupported`"),
    PropDef::new("bindReady", PropKind::StateKey, "State key that's true once every one that's `required` is allowed"),
    PropDef::new("title", PropKind::Any, "Over them"),
];

static PERMISSIONS: LazyLock<StyleBlock> = LazyLock::new(|| {
    widget::device(widget::column(gap!(8), "100%"))
        .nest("& [data-perm-icon]", pairs(&[("display", "grid"), ("place-items", "center"), ("flex", "none"), ("width", "36px"), ("height", "36px"), ("border-radius", "var(--radius-md)"), ("background", SUBTLE)]))
        .nest("& [data-w-list] small", pairs(&[("color", MUTED), ("font-size", fs!(13)), ("line-height", "1.4")]))
        .nest("& [data-perm-name]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("align-items", "center"), ("gap", gap!(4, 8))]))
});

impl Component for PermissionManager {
    fn name(&self) -> &'static str {
        "ui-permission-manager"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "What the app may use (camera, location, notifications…): why, whether it's allowed, a button to ask, and how to allow what was blocked.", props: PERMISSION_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let config = widget::config(cx, node, &["title"], &["permissions"], &["bind", "bindReady"])?;
        let list = config.get("permissions").and_then(Json::as_array).cloned().unwrap_or_default();
        if list.is_empty() {
            return Err(RenderError::new(node, "`permissions` is a list of names"));
        }
        for p in &list {
            let name = p.as_str().or_else(|| p["name"].as_str()).unwrap_or_default();
            if !KINDS.contains(&name) {
                return Err(RenderError::new(node, format!("`{name}` isn't one of the permissions: {}", KINDS.join(", "))));
            }
        }
        scripts(cx);
        widget::drawn(cx, node, "section", &PERMISSIONS, "ui-permission-manager", config, vec![("aria-label", lit(cx.label("Permissions")))])
    }
}

// ---------------------------------------------------------------- annotate

pub struct PhotoAnnotate;

const TOOLS: &[&str] = &["select", "arrow", "box", "circle", "pen", "text", "measure", "blur"];
const ANNOTATE_PROPS: &[PropDef] = &[
    PropDef::new("src", PropKind::Any, "The picture: an address or `data:`; may use state. An object with a `url` (a `ui-camera` photo) will do").required(),
    PropDef::new("bind", PropKind::StateKey, "State key that gets the picture with the marks drawn on it (a `data:` address) as they change"),
    PropDef::new("bindMarks", PropKind::StateKey, "State key for the marks: `[{ id, type, points, text, color }]`, points as parts of the picture, 0 to 1"),
    PropDef::new("tools", PropKind::Any, "Which, in order: `[select, arrow, box, circle, pen, text, measure, blur]` (default all)"),
    PropDef::new("colors", PropKind::Any, "Default red, amber, green, blue, white and black"),
    PropDef::new("width", PropKind::Number, "Of the lines, in px of a 1000 px wide picture (default 6)"),
    PropDef::new("unit", PropKind::Any, "For measurements, e.g. `cm`: the first one's length is typed, and the others are worked out from it (default `cm`)"),
    PropDef::new("type", PropKind::Enum(&["jpeg", "png", "webp"]), "Default `jpeg`"),
    PropDef::new("quality", PropKind::Number, "0 to 1 (default 0.85)"),
    PropDef::new("maxSize", PropKind::Number, "The result's longer side at most, in px (default 2000)"),
    PropDef::new("readonly", PropKind::Bool, "The marks are shown, not changed"),
    PropDef::new("label", PropKind::Any, "Names it (default `Mark up the photo`)"),
];

static ANNOTATE: LazyLock<StyleBlock> = LazyLock::new(|| {
    widget::device(widget::column(gap!(8), "100%"))
        .nest("& [data-an-stage]", pairs(&[("position", "relative"), ("display", "grid"), ("place-items", "center"), ("overflow", "hidden"), ("min-height", "120px"), ("border-radius", "var(--radius-md)"), ("background", SUBTLE)]))
        .nest("& [data-an-stage] canvas", pairs(&[("display", "block"), ("max-width", "100%"), ("touch-action", "none"), ("outline-offset", "-3px"), ("cursor", "crosshair")]))
        .nest("& [data-an-stage] canvas[data-tool=select], & [data-an-stage] canvas[data-readonly]", pairs(&[("cursor", "default")]))
        .nest("& [data-an-text]", pairs(&[("position", "absolute"), ("z-index", "1"), ("width", "160px"), ("translate", "-50% -50%")]))
        .nest("& [data-an-swatch]", pairs(&[("box-sizing", "border-box"), ("width", "24px"), ("height", "24px"), ("padding", "0"), ("border", "2px solid"), ("border-color", BORDER), ("border-radius", "50%"), ("cursor", "pointer")]))
        .nest("& [data-an-swatch][aria-checked=true]", pairs(&[("outline", "2px solid"), ("outline-color", PRIMARY), ("outline-offset", "1px")]))
        .nest("& [data-an-sep]", pairs(&[("width", "1px"), ("height", "22px"), ("background", BORDER)]))
});

impl Component for PhotoAnnotate {
    fn name(&self) -> &'static str {
        "ui-photo-annotate"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Marks on a photo: arrows, boxes, circles, freehand, text, measurements and blurring (for faces and number plates), kept apart from the picture and drawn onto a copy of it.", props: ANNOTATE_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut config = widget::config(cx, node, &["src", "unit"], &["tools", "colors", "width", "type", "quality", "maxSize"], &["bind", "bindMarks"])?;
        if let Some(t) = config.get("tools").and_then(Json::as_array)
            && let Some(bad) = t.iter().find(|t| !TOOLS.contains(&t.as_str().unwrap_or_default()))
        {
            return Err(RenderError::new(node, format!("{bad} isn't one of the tools: {}", TOOLS.join(", "))));
        }
        if !config.contains_key("tools") {
            config.insert("tools".into(), json!(TOOLS));
        }
        widget::flags(cx, node, &mut config, &[("readonly", false)]);
        scripts(cx);
        let label = node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Mark up the photo")));
        widget::drawn(cx, node, "div", &ANNOTATE, "ui-photo-annotate", config, vec![("role", lit("group")), ("aria-label", label)])
    }
}
