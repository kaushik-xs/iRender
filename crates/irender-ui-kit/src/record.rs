//! `ui-audio-recorder`, `ui-video-recorder`, `ui-voice-input`, `ui-screen-capture`,
//! `ui-read-aloud`, `ui-wake-lock`, `ui-swipe-actions` (`record.js` runs them) and
//! `ui-document-scanner` (`scan.js`).

use crate::{
    BG_CARD, BORDER, MUTED, PRIMARY, SUBTLE, TEXT, forms, lit,
    widget::{self, pairs},
};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock};
use irender_html::{Component, RenderCx, RenderError};
use serde_json::Value as Json;
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("record.js");
const SCAN: &str = include_str!("scan.js");

fn scripts(cx: &mut RenderCx<'_>) {
    widget::device_kit(cx);
    cx.global_js("ui-record", SCRIPT);
}

static PANEL: LazyLock<StyleBlock> = LazyLock::new(|| {
    widget::device(widget::column(gap!(8), "560px"))
        .nest("& [data-rc-wave]", pairs(&[("display", "block"), ("flex", "1"), ("min-width", "80px"), ("width", "100%"), ("height", "44px"), ("color", PRIMARY), ("cursor", "pointer"), ("border-radius", "var(--radius-sm)")]))
        .nest("& [data-rc-time]", pairs(&[("font-variant-numeric", "tabular-nums"), ("color", MUTED), ("font-size", fs!(13)), ("min-width", "3em")]))
        .nest("& [data-rc-dot]", pairs(&[("width", "10px"), ("height", "10px"), ("border-radius", "50%"), ("background", "var(--color-destructive, #dc2626)")]))
        .nest("& [data-rc-dot][data-on]", pairs(&[("animation", "ui-blink 1s infinite")]))
        .nest("& [data-rc-view]", pairs(&[("position", "relative"), ("overflow", "hidden"), ("aspect-ratio", "16 / 9"), ("border-radius", "var(--radius-card, var(--radius-lg))"), ("background", "#000")]))
        .nest("& [data-rc-view] :is(video, img)", pairs(&[("position", "absolute"), ("inset", "0"), ("width", "100%"), ("height", "100%"), ("object-fit", "contain")]))
        .nest("& video[data-front]", pairs(&[("scale", "-1 1")]))
        .nest("& [data-rc-badge]", pairs(&[("position", "absolute"), ("z-index", "1"), ("top", "10px"), ("inset-inline-start", "10px"), ("display", "flex"), ("align-items", "center"), ("gap", "7px"), ("padding", "4px 10px"), ("border-radius", "999px"), ("background", "rgb(0 0 0 / 0.6)"), ("color", "#fff"), ("font-size", "13px"), ("font-variant-numeric", "tabular-nums")]))
        .nest("& [data-vi-text]", pairs(&[("min-height", "1.5em"), ("color", MUTED), ("font-style", "italic"), ("overflow-wrap", "anywhere")]))
        .nest("& [data-vi-text]:empty", pairs(&[("display", "none")]))
        .nest("& [data-listening]", pairs(&[("position", "relative")]))
        .nest("& [data-listening]::after", pairs(&[("content", "''"), ("position", "absolute"), ("inset", "-5px"), ("border", "2px solid var(--color-destructive, #dc2626)"), ("border-radius", "inherit"), ("animation", "ui-blink 1.2s infinite"), ("pointer-events", "none")]))
        .nest("@media (prefers-reduced-motion: reduce)", pairs(&[]).nest("& [data-rc-dot], & [data-listening]::after", pairs(&[("animation", "none !important")])))
});

const BLINK: &str = "@keyframes ui-blink { 50% { opacity: 0.3; } }";

/// A component the panel's script draws.
fn panel(cx: &mut RenderCx<'_>, node: &Node, name: &'static str, config: serde_json::Map<String, Json>, label: &str) -> Result<(), RenderError> {
    scripts(cx);
    cx.global_css("ui-blink", BLINK);
    let label = lit(cx.label(label));
    widget::drawn(cx, node, "div", &PANEL, name, config, vec![("role", lit("group")), ("aria-label", label)])
}

// ---------------------------------------------------------------- audio, video, screen

pub struct AudioRecorder;
pub struct VideoRecorder;
pub struct ScreenCapture;

const AUDIO_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key for the recording, `{ id, url, name, type, size, seconds, peaks, at }` (`url` is a `data:` address), or with `multiple` a list of them").required(),
    PropDef::new("multiple", PropKind::Bool, "Several"),
    PropDef::new("max", PropKind::Number, "Seconds at most (default 300)"),
    PropDef::new("bitrate", PropKind::Number, "Bits a second (default 48000, good for voice)"),
    PropDef::new("label", PropKind::Any, "Default `Record`"),
    PropDef::new("disabled", PropKind::Any, "Played, not changed"),
];
const VIDEO_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key for the video, `{ id, url, name, type, size, seconds, at }` (`url` is a `data:` address)").required(),
    PropDef::new("max", PropKind::Number, "Seconds at most (default 30)"),
    PropDef::new("facing", PropKind::Enum(&["front", "back"]), "Default `front`"),
    PropDef::new("audio", PropKind::Bool, "With sound (default true)"),
    PropDef::new("bitrate", PropKind::Number, "Bits a second (default 1200000)"),
    PropDef::new("height", PropKind::Number, "Of the video in px (default 720)"),
    PropDef::new("label", PropKind::Any, "Default `Open the camera`"),
    PropDef::new("disabled", PropKind::Any, "Played, not changed"),
];
const SCREEN_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key for what's taken: a picture `{ id, url, name, type, size, width, height, at }`, or a video").required(),
    PropDef::new("modes", PropKind::Any, "`[picture, video]` (default both)"),
    PropDef::new("max", PropKind::Number, "Seconds of video at most (default 60)"),
    PropDef::new("audio", PropKind::Bool, "With the sound of what's shared, where it can be"),
];

impl Component for AudioRecorder {
    fn name(&self) -> &'static str {
        "ui-audio-recorder"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Voice notes: records with a moving waveform, pauses, plays back and goes to any point of it.", props: AUDIO_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        widget::need(cx, node, "bind")?;
        let mut config = widget::config(cx, node, &["label", "disabled"], &["max", "bitrate"], &["bind"])?;
        widget::flags(cx, node, &mut config, &[("multiple", false)]);
        panel(cx, node, "ui-audio-recorder", config, "Audio recorder")
    }
}

impl Component for VideoRecorder {
    fn name(&self) -> &'static str {
        "ui-video-recorder"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Records a short video with a time limit, pauses, and plays it back to keep or take again.", props: VIDEO_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        widget::need(cx, node, "bind")?;
        let mut config = widget::config(cx, node, &["label", "disabled"], &["max", "bitrate", "facing", "height"], &["bind"])?;
        widget::flags(cx, node, &mut config, &[("audio", true)]);
        panel(cx, node, "ui-video-recorder", config, "Video recorder")
    }
}

impl Component for ScreenCapture {
    fn name(&self) -> &'static str {
        "ui-screen-capture"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "A picture or a video of a screen, a window or a tab, which someone picks: for reporting a problem, say. Phones can't.", props: SCREEN_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        widget::need(cx, node, "bind")?;
        let mut config = widget::config(cx, node, &[], &["max", "modes"], &["bind"])?;
        if let Some(m) = config.get("modes")
            && !m.as_array().is_some_and(|m| !m.is_empty() && m.iter().all(|x| x == "picture" || x == "video"))
        {
            return Err(RenderError::new(node, "`modes` is a list of `picture` and `video`"));
        }
        widget::flags(cx, node, &mut config, &[("audio", false)]);
        panel(cx, node, "ui-screen-capture", config, "Screen capture")
    }
}

// ---------------------------------------------------------------- speech

pub struct VoiceInput;
pub struct ReadAloud;

const VOICE_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key for the text").required(),
    PropDef::new("append", PropKind::Bool, "After what's there already (default true)"),
    PropDef::new("continuous", PropKind::Bool, "Goes on past a pause, until it's stopped"),
    PropDef::new("lang", PropKind::Any, "What's spoken, e.g. `en-GB` (default the document's)"),
    PropDef::new("transcribeUrl", PropKind::Any, "Where the browser can't (Firefox), what's said is recorded and POSTed here as `audio`; the reply is `{ text }`"),
    PropDef::new("iconOnly", PropKind::Bool, "Only a microphone, for beside a field"),
    PropDef::new("label", PropKind::Any, "Default `Speak`"),
];
const READ_PROPS: &[PropDef] = &[
    PropDef::new("text", PropKind::Any, "What's read; may use state"),
    PropDef::new("target", PropKind::String, "Or a part of the page, a CSS selector: each sentence is marked as it's read"),
    PropDef::new("lang", PropKind::Any, "Default the document's"),
    PropDef::new("voice", PropKind::Any, "A voice's name, or the start of it"),
    PropDef::new("rate", PropKind::Number, "0.5 to 2 (default 1)"),
    PropDef::new("controls", PropKind::Bool, "Speed can be changed"),
    PropDef::new("bindState", PropKind::StateKey, "State key that gets `playing`, `paused` or `stopped`"),
    PropDef::new("label", PropKind::Any, "Default `Read aloud`"),
];

impl Component for VoiceInput {
    fn name(&self) -> &'static str {
        "ui-voice-input"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Speech to text, shown as it's heard. Some browsers send what's said to their maker's servers to do it.", props: VOICE_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        widget::need(cx, node, "bind")?;
        widget::check_url(cx, node, "transcribeUrl")?;
        let mut config = widget::config(cx, node, &["lang", "label", "transcribeUrl"], &[], &["bind"])?;
        widget::flags(cx, node, &mut config, &[("append", true), ("continuous", false), ("iconOnly", false)]);
        panel(cx, node, "ui-voice-input", config, "Voice input")
    }
}

impl Component for ReadAloud {
    fn name(&self) -> &'static str {
        "ui-read-aloud"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Reads text, or a part of the page, out loud with the device's own voices, with pause and speed.", props: READ_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        if node.prop("text").is_none() && node.prop("target").is_none() {
            return Err(RenderError::new(node, "`ui-read-aloud` needs `text` or `target`"));
        }
        let mut config = widget::config(cx, node, &["text", "lang", "voice", "label"], &["target", "rate"], &["bindState"])?;
        widget::flags(cx, node, &mut config, &[("controls", false)]);
        cx.global_css("ui-reading", "::highlight(ui-reading) { background-color: color-mix(in srgb, var(--color-primary) 25%, transparent); }");
        panel(cx, node, "ui-read-aloud", config, "Read aloud")
    }
}

// ---------------------------------------------------------------- wake lock

pub struct WakeLock;

const WAKE_PROPS: &[PropDef] = &[
    PropDef::new("active", PropKind::Any, "The screen's kept on while this is true, e.g. `{{ step > 0 }}`"),
    PropDef::new("bind", PropKind::StateKey, "Or a state key, with a switch for it"),
    PropDef::new("label", PropKind::Any, "The switch's (default `Keep the screen on`)"),
    PropDef::new("bindHeld", PropKind::StateKey, "State key that's true while it's really kept on"),
];

static WAKE: LazyLock<StyleBlock> = LazyLock::new(|| {
    widget::device(pairs(&[("display", "inline-flex"), ("color", TEXT), ("font-size", fs!(14))]))
        .nest("& label", pairs(&[("display", "inline-flex"), ("align-items", "center"), ("gap", gap!(8)), ("cursor", "pointer")]))
        .nest("& input", pairs(&[("width", "16px"), ("height", "16px"), ("margin", "0"), ("accent-color", PRIMARY)]))
});

impl Component for WakeLock {
    fn name(&self) -> &'static str {
        "ui-wake-lock"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Keeps the screen on: a recipe, a score, a boarding pass, a timer.", props: WAKE_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        if node.prop("active").is_none() && node.prop("bind").is_none() {
            return Err(RenderError::new(node, "`ui-wake-lock` needs `active` or `bind`"));
        }
        let config = widget::config(cx, node, &["active", "label"], &[], &["bind", "bindHeld"])?;
        scripts(cx);
        widget::drawn(cx, node, "div", &WAKE, "ui-wake-lock", config, vec![])
    }
}

// ---------------------------------------------------------------- swipe

pub struct SwipeActions;

const SWIPE_PROPS: &[PropDef] = &[
    PropDef::new("start", PropKind::Any, "Under its start, shown by swiping that way: `[{ value, label, tone, icon }]`; each runs `on: { <value>: … }`"),
    PropDef::new("end", PropKind::Any, "Under its end"),
    PropDef::new("full", PropKind::Bool, "Swiping all the way runs the outermost one (default true)"),
    PropDef::new("menu", PropKind::Bool, "A button shows them too, for a mouse and the keyboard (default true)"),
    PropDef::new("label", PropKind::Any, "Names the row in that button, e.g. `{{ m.from }}`"),
    PropDef::new("disabled", PropKind::Any, "Doesn't swipe"),
];

static SWIPE: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("position", "relative"), ("overflow", "hidden"), ("color", TEXT), ("font-size", fs!(14))])
        .nest("& [hidden]", pairs(&[("display", "none !important")]))
        .nest("& > [data-sw-in]", pairs(&[("position", "relative"), ("z-index", "1"), ("background", BG_CARD), ("touch-action", "pan-y"), ("transition", "transform .22s")]))
        .nest("& > [data-sw-in][data-drag]", pairs(&[("transition", "none"), ("user-select", "none")]))
        .nest("& > [data-sw-acts]", pairs(&[("position", "absolute"), ("inset-block", "0"), ("display", "flex"), ("visibility", "hidden")]))
        .nest("& > [data-sw-acts][data-on]", pairs(&[("visibility", "visible")]))
        .nest("& > [data-sw-acts=start]", pairs(&[("inset-inline-start", "0")]))
        .nest("& > [data-sw-acts=end]", pairs(&[("inset-inline-end", "0")]))
        .nest("& > [data-sw-acts] button", pairs(&[("display", "flex"), ("flex-direction", "column"), ("align-items", "center"), ("justify-content", "center"), ("flex", "1"), ("gap", "4px"), ("min-width", "76px"), ("padding", "0 12px"), ("border", "0"), ("background", "var(--sw, var(--color-muted))"), ("color", "#fff"), ("font", "inherit"), ("font-size", fs!(13)), ("font-weight", "500"), ("cursor", "pointer")]))
        .nest("& button:focus-visible", forms::focus_ring().set("outline-offset", "-3px"))
        .nest("& [data-tone=danger]", pairs(&[("--sw", "var(--color-destructive, #dc2626)")]))
        .nest("& [data-tone=success]", pairs(&[("--sw", "var(--color-success, #16a34a)")]))
        .nest("& [data-tone=warning]", pairs(&[("--sw", "var(--color-warning, #d97706)")]))
        .nest("& [data-tone=info]", pairs(&[("--sw", "var(--color-info, #2563eb)")]))
        .nest("& [data-tone=primary]", pairs(&[("--sw", PRIMARY)]))
        .nest("& [data-sw-more]", widget::tool().set("position", "absolute").set("z-index", "2").set("top", "50%").set("inset-inline-end", "8px").set("translate", "0 -50%").set("opacity", "0"))
        .nest("&:is(:hover, :focus-within) [data-sw-more]", pairs(&[("opacity", "1")]))
        .nest("& [data-sw-more]:hover", pairs(&[("background", SUBTLE)]))
        .nest("@media (hover: none)", pairs(&[]).nest("& [data-sw-more]:not(:focus-visible)", pairs(&[("opacity", "0"), ("pointer-events", "none")])))
        .nest("@media (prefers-reduced-motion: reduce)", pairs(&[]).nest("& > [data-sw-in]", pairs(&[("transition", "none")])))
});

impl Component for SwipeActions {
    fn name(&self) -> &'static str {
        "ui-swipe-actions"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "A row that's swiped sideways to show what can be done with it: archive, delete, call. Its children are the row.", props: SWIPE_PROPS, parts: &["content"], children: true, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut config = widget::config(cx, node, &["start", "end", "label", "disabled"], &[], &[])?;
        if !config.contains_key("start") && !config.contains_key("end") {
            return Err(RenderError::new(node, "`ui-swipe-actions` needs `start` or `end`"));
        }
        widget::flags(cx, node, &mut config, &[("full", true), ("menu", true)]);
        scripts(cx);
        let mut el = cx.element("div", node);
        el.base_style_ref(&SWIPE).behavior("ui-swipe-actions", Json::Object(config));
        cx.open(&el, node)?;
        let mut inner = cx.part("div", node, "content");
        inner.attr("data-sw-in", true);
        cx.open(&inner, node)?;
        cx.children(node)?;
        cx.close("div");
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- documents

pub struct DocumentScanner;

const DOC_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key for the pages: `[{ id, url, width, height }]`, each a `data:` picture").required(),
    PropDef::new("bindPdf", PropKind::StateKey, "State key that gets them as one PDF, `{ url, name, type, size, pages }`"),
    PropDef::new("max", PropKind::Number, "Pages at most"),
    PropDef::new("filter", PropKind::Enum(&["colour", "grey", "bw", "none"]), "How it starts: brightened, grey, black and white, or as it was (default `colour`)"),
    PropDef::new("paper", PropKind::Enum(&["auto", "a4", "letter"]), "The PDF's pages (default `auto`: each picture's shape)"),
    PropDef::new("maxSize", PropKind::Number, "A page's longer side at most, in px (default 1800)"),
    PropDef::new("quality", PropKind::Number, "0 to 1 (default 0.75)"),
    PropDef::new("name", PropKind::Any, "The PDF's name (default `scan.pdf`)"),
    PropDef::new("label", PropKind::Any, "Default `Scan a page`"),
];

static DOC: LazyLock<StyleBlock> = LazyLock::new(|| {
    widget::device(widget::column(gap!(10), "640px"))
        .nest("& [data-ds-view]", pairs(&[("position", "relative"), ("overflow", "hidden"), ("display", "grid"), ("place-items", "center"), ("min-height", "220px"), ("border-radius", "var(--radius-card, var(--radius-lg))"), ("background", "#18181b"), ("touch-action", "none")]))
        .nest("& [data-ds-view] video", pairs(&[("display", "block"), ("width", "100%"), ("max-height", "70vh"), ("object-fit", "contain")]))
        .nest("& [data-ds-view] canvas", pairs(&[("display", "block"), ("max-width", "100%")]))
        .nest("& [data-ds-view] svg[data-ds-over]", pairs(&[("position", "absolute"), ("overflow", "visible"), ("pointer-events", "none")]))
        .nest("& [data-ds-view] svg polygon", pairs(&[("fill", "rgb(37 99 235 / 0.2)"), ("stroke", "#60a5fa"), ("stroke-width", "2.5"), ("stroke-linejoin", "round")]))
        .nest("& [data-ds-corner]", pairs(&[("position", "absolute"), ("width", "32px"), ("height", "32px"), ("margin", "-16px 0 0 -16px"), ("padding", "0"), ("border", "0"), ("border-radius", "50%"), ("background", "radial-gradient(circle, #fff 0 7px, #2563eb 7px 10px, transparent 10px)"), ("cursor", "move"), ("touch-action", "none")]))
        .nest("& [data-ds-pages]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("gap", gap!(10)), ("margin", "0"), ("padding", "0"), ("list-style", "none")]))
        .nest("& [data-ds-pages] li", pairs(&[("position", "relative"), ("display", "flex"), ("flex-direction", "column"), ("gap", "4px"), ("align-items", "center")]))
        .nest("& [data-ds-pages] img", pairs(&[("display", "block"), ("width", "96px"), ("height", "128px"), ("object-fit", "contain"), ("border", "1px solid"), ("border-color", BORDER), ("border-radius", "var(--radius-sm)"), ("background", "#fff")]))
        .nest("& [data-ds-pages] li > div", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", "2px"), ("color", MUTED), ("font-size", fs!(12))]))
        .nest("& [data-ds-pages] button", widget::tool())
        .nest("& label[data-w-btn] input", pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("opacity", "0")]))
        .nest("& label[data-w-btn]", pairs(&[("position", "relative")]))
        .nest("& label[data-w-btn]:focus-within", forms::focus_ring())
});

impl Component for DocumentScanner {
    fn name(&self) -> &'static str {
        "ui-document-scanner"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Scans papers with the camera or from a picture: finds the page's edges, straightens it, cleans it up, and makes a PDF of several.", props: DOC_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        widget::need(cx, node, "bind")?;
        let config = widget::config(cx, node, &["label", "name"], &["max", "filter", "paper", "maxSize", "quality"], &["bind", "bindPdf"])?;
        widget::device_kit(cx);
        crate::sheets(cx);
        cx.global_js("ui-scan", SCAN);
        widget::drawn(cx, node, "div", &DOC, "ui-document-scanner", config, vec![("role", lit("group")), ("aria-label", lit(cx.label("Document scanner")))])
    }
}
