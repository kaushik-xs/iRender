//! What an assistant's replies show beside their text: `ui-attachment` (files),
//! `ui-sources` (what it's based on), `ui-tool-call` (what it did), `ui-trace` (the steps
//! of a run, against time) and `ui-questionnaire` (what it asks). `agent.js` draws them.

use crate::{
    BG_CARD, BG_POPOVER, BORDER, MUTED, PRIMARY, PRIMARY_FILL, SUBTLE, SURFACE, TEXT, forms, lit,
    widget::{self, flag, pairs},
};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock};
use irender_html::{Component, RenderCx, RenderError};
use serde_json::{Map, Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("agent.js");
const FONT: &str = "var(--font-mono, ui-monospace, SFMono-Regular, Menlo, Consolas, monospace)";

fn mount(cx: &mut RenderCx<'_>, node: &Node, style: &'static StyleBlock, extra: StyleBlock, name: &'static str, config: Map<String, Json>, label: String) -> Result<(), RenderError> {
    widget::kit(cx);
    cx.global_js(irender_html::CODE_JS_KEY, irender_html::CODE_JS);
    cx.global_js("ui-agent", SCRIPT);
    let mut el = cx.element("div", node);
    el.base_style_ref(style).base_style(extra).attr("role", "group").attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(label))).behavior(name, Json::Object(config));
    cx.open(&el, node)?;
    cx.close("div");
    Ok(())
}

fn base() -> StyleBlock {
    forms::native_scheme(pairs(&[("min-width", "0"), ("color", TEXT), ("font-size", fs!(14))]))
        .nest("& [hidden]", pairs(&[("display", "none !important")]))
        .nest("& :is(button, a):focus-visible", forms::focus_ring())
        .nest("& [data-tool]", widget::tool())
        .nest("& [data-tool]:hover", pairs(&[("background", SUBTLE), ("color", TEXT)]))
}

// ---------------------------------------------------------------- attachment

pub struct Attachment;

const ATTACH_PROPS: &[PropDef] = &[
    PropDef::new("files", PropKind::Any, "`[{ id, name, type, size, url, thumb, status, progress, error, text }]`; `status`: uploading, processing, done or error; may use state").required(),
    PropDef::new("bindFiles", PropKind::StateKey, "State key the files are in: they can be removed"),
    PropDef::new("layout", PropKind::Enum(&["chips", "cards"]), "Default `chips`"),
    PropDef::new("label", PropKind::Any, "Names them for screen readers (default `Attachments`)"),
];

static ATTACH: LazyLock<StyleBlock> = LazyLock::new(|| {
    base()
        .nest("& ul", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("gap", gap!(8)), ("margin", "0"), ("padding", "0"), ("list-style", "none")]))
        .nest("& li", pairs(&[("position", "relative"), ("display", "flex"), ("align-items", "center"), ("gap", gap!(8)), ("box-sizing", "border-box"), ("min-width", "0"), ("max-width", "260px"), ("padding-block", pad!(6, 6)), ("padding-inline", pad!(6, 8)), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-md)"), ("background", SURFACE), ("overflow", "hidden")]))
        .nest("&[data-layout=cards] li", pairs(&[("flex-direction", "column"), ("align-items", "stretch"), ("width", "160px"), ("padding", "0")]))
        .nest("&[data-layout=cards] [data-thumb]", pairs(&[("width", "100%"), ("height", "96px"), ("border-radius", "0")]))
        .nest("&[data-layout=cards] [data-text]", pairs(&[("padding", pad!(8))]))
        .nest("&[data-layout=cards] [data-tool]", pairs(&[("position", "absolute"), ("top", "4px"), ("inset-inline-end", "4px"), ("background", "color-mix(in srgb, var(--color-surface) 85%, transparent)")]))
        .nest("& [data-thumb]", pairs(&[("flex", "none"), ("display", "grid"), ("place-items", "center"), ("width", "36px"), ("height", "36px"), ("overflow", "hidden"), ("border-radius", "var(--radius-sm)"), ("background", SUBTLE), ("color", MUTED), ("font-size", fs!(10)), ("font-weight", "600")]))
        .nest("& [data-thumb] img", pairs(&[("width", "100%"), ("height", "100%"), ("object-fit", "cover")]))
        .nest("& [data-text]", pairs(&[("min-width", "0"), ("flex", "1")]))
        .nest("& [data-name]", pairs(&[("display", "block"), ("overflow", "hidden"), ("text-overflow", "ellipsis"), ("white-space", "nowrap"), ("color", "inherit"), ("font-weight", "var(--font-weight-medium, 500)"), ("font-size", fs!(13)), ("text-decoration", "none")]))
        .nest("& a[data-name]:hover", pairs(&[("text-decoration", "underline")]))
        .nest("& [data-text] small", pairs(&[("display", "block"), ("color", MUTED), ("font-size", fs!(12)), ("overflow", "hidden"), ("text-overflow", "ellipsis"), ("white-space", "nowrap")]))
        .nest("& li[data-status=error]", pairs(&[("border-color", "var(--color-destructive, #dc2626)")]))
        .nest("& li[data-status=error] small", pairs(&[("color", "var(--color-destructive, #dc2626)")]))
        .nest("& [data-bar]", pairs(&[("position", "absolute"), ("inset-inline-start", "0"), ("bottom", "0"), ("height", "3px"), ("background", PRIMARY_FILL), ("transition", "width .2s")]))
        .nest("& [data-bar][data-busy]", pairs(&[("width", "40%"), ("animation", "ui-agent-busy 1.2s ease-in-out infinite")]))
        .nest("@media (prefers-reduced-motion: reduce)", pairs(&[]).nest("& [data-bar][data-busy]", pairs(&[("animation", "none"), ("width", "100%"), ("opacity", "0.5")])))
});

const BUSY: &str = "@keyframes ui-agent-busy {\n  from { inset-inline-start: -40%; }\n  to { inset-inline-start: 100%; }\n}";

impl Component for Attachment {
    fn name(&self) -> &'static str {
        "ui-attachment"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Files sent with a message or being sent: a name, size and picture, progress, what went wrong, and a button to remove each.", props: ATTACH_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let config = widget::config(cx, node, &["files"], &["layout"], &["bindFiles"])?;
        cx.global_css("ui-agent-busy", BUSY);
        let l = cx.label("Attachments");
        mount(cx, node, &ATTACH, StyleBlock::new(), "ui-attachment", config, l)
    }
}

// ---------------------------------------------------------------- sources

pub struct Sources;

const SOURCES_PROPS: &[PropDef] = &[
    PropDef::new("items", PropKind::Any, "`[{ title, url, text, site, date, page }]`, numbered in their order; may use state").required(),
    PropDef::new("text", PropKind::Any, "Text with `[1]`, `[2]` in it: each becomes a link that shows its source"),
    PropDef::new("layout", PropKind::Enum(&["list", "chips", "fold"]), "All of them (`list`), small (`chips`), or folded behind how many there are (`fold`, default)"),
    PropDef::new("label", PropKind::Any, "Default `Sources`"),
];

static SOURCES: LazyLock<StyleBlock> = LazyLock::new(|| {
    base()
        .nest("& [data-cited]", pairs(&[("margin", "0"), ("line-height", "1.6"), ("white-space", "pre-wrap"), ("overflow-wrap", "anywhere")]))
        .nest("& [data-cite]", pairs(&[("display", "inline-grid"), ("place-items", "center"), ("min-width", "17px"), ("height", "17px"), ("margin-inline", "2px"), ("padding", "0 4px"), ("border", "0"), ("border-radius", "999px"), ("background", "color-mix(in srgb, var(--color-primary) 14%, transparent)"), ("color", TEXT), ("font", "inherit"), ("font-size", fs!(11)), ("line-height", "1"), ("vertical-align", "super"), ("cursor", "pointer")]))
        .nest("& [data-cite]:hover", pairs(&[("background", PRIMARY_FILL), ("color", crate::ON_PRIMARY)]))
        .nest("& [data-pop]", pairs(&[("position", "fixed"), ("z-index", "60"), ("inset", "auto"), ("margin", "0"), ("box-sizing", "border-box"), ("width", "min(300px, calc(100vw - 16px))"), ("padding", pad!(12)), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-md)"), ("background", BG_POPOVER), ("color", TEXT), ("font-size", fs!(13)), ("box-shadow", "var(--elevation-popover, 0 6px 18px rgb(0 0 0 / 0.15))")]))
        .nest("& [data-fold]", pairs(&[("display", "inline-flex"), ("align-items", "center"), ("gap", gap!(6)), ("margin-top", gap!(8)), ("height", "28px"), ("padding", pad!(0, 10)), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "999px"), ("background", "none"), ("color", TEXT), ("font", "inherit"), ("font-size", fs!(13)), ("cursor", "pointer")]))
        .nest("& [data-fold]:hover", pairs(&[("background", SUBTLE)]))
        .nest("& [data-fold] svg", pairs(&[("transition", "rotate .15s")]))
        .nest("& [data-fold][aria-expanded=true] svg", pairs(&[("rotate", "90deg")]))
        .nest("& ol", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(6)), ("margin", concat!(gap!(8), " 0 0")), ("padding", "0"), ("list-style", "none")]))
        .nest("&[data-layout=chips] ol", pairs(&[("flex-direction", "row"), ("flex-wrap", "wrap")]))
        .nest("&[data-layout=chips] li", pairs(&[("max-width", "220px")]))
        .nest("&[data-layout=chips] li p", pairs(&[("display", "none")]))
        .nest("& li", pairs(&[("display", "flex"), ("gap", gap!(8)), ("min-width", "0"), ("padding", pad!(8, 10)), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-md)")]))
        .nest("& li[data-hit]", pairs(&[("border-color", PRIMARY)]))
        .nest("& li > i", pairs(&[("flex", "none"), ("color", MUTED), ("font-style", "normal"), ("font-size", fs!(12)), ("font-variant-numeric", "tabular-nums")]))
        .nest("& [data-src]", pairs(&[("min-width", "0")]))
        .nest("& [data-src] :is(a, b)", pairs(&[("display", "block"), ("overflow", "hidden"), ("text-overflow", "ellipsis"), ("white-space", "nowrap"), ("color", "inherit"), ("font-weight", "var(--font-weight-medium, 500)"), ("text-decoration", "none")]))
        .nest("& [data-src] a:hover", pairs(&[("text-decoration", "underline")]))
        .nest("& [data-src] small", pairs(&[("display", "block"), ("color", MUTED), ("font-size", fs!(12))]))
        .nest("& [data-src] p", pairs(&[("margin", "4px 0 0"), ("color", MUTED), ("font-size", fs!(13)), ("line-height", "1.45"), ("display", "-webkit-box"), ("-webkit-line-clamp", "3"), ("-webkit-box-orient", "vertical"), ("overflow", "hidden")]))
});

impl Component for Sources {
    fn name(&self) -> &'static str {
        "ui-sources"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "What an answer is based on: numbered sources, and the `[1]`s in its text as links that show them.", props: SOURCES_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let config = widget::config(cx, node, &["items", "text", "label"], &["layout"], &[])?;
        let l = cx.label("Sources");
        mount(cx, node, &SOURCES, StyleBlock::new(), "ui-sources", config, l)
    }
}

// ---------------------------------------------------------------- tool call

pub struct ToolCall;

const CALL_PROPS: &[PropDef] = &[
    PropDef::new("title", PropKind::Any, "What was done, e.g. `Searched the orders`").required(),
    PropDef::new("status", PropKind::Any, "`pending`, `running`, `done`, `error` or `approval`; may use state"),
    PropDef::new("name", PropKind::Any, "The tool's own name, smaller"),
    PropDef::new("input", PropKind::Any, "What it was given; shown as JSON, or as it is when text"),
    PropDef::new("output", PropKind::Any, "What it answered"),
    PropDef::new("error", PropKind::Any, "What went wrong"),
    PropDef::new("duration", PropKind::Any, "Milliseconds it took"),
    PropDef::new("open", PropKind::Bool, "Starts open (with `approval` it always does)"),
    PropDef::new("bindApproval", PropKind::StateKey, "With `status: approval`, state key that gets true or false; also `approve` and `deny` events"),
    PropDef::new("label", PropKind::Any, "Names it for screen readers"),
];

static CALL: LazyLock<StyleBlock> = LazyLock::new(|| {
    base()
        .nest("&", pairs(&[("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-md)"), ("background", BG_CARD), ("overflow", "hidden")]))
        .nest("& [data-head]", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", gap!(8)), ("width", "100%"), ("padding", pad!(8, 12)), ("border", "0"), ("background", "none"), ("color", "inherit"), ("font", "inherit"), ("text-align", "start"), ("cursor", "pointer")]))
        .nest("& [data-head]:hover", pairs(&[("background", SUBTLE)]))
        .nest("& [data-head] b", pairs(&[("font-weight", "var(--font-weight-medium, 500)"), ("min-width", "0"), ("overflow", "hidden"), ("text-overflow", "ellipsis"), ("white-space", "nowrap")]))
        .nest("& [data-head] code", pairs(&[("color", MUTED), ("font-family", FONT), ("font-size", fs!(12)), ("white-space", "nowrap")]))
        .nest("& [data-head] small", pairs(&[("margin-inline-start", "auto"), ("color", MUTED), ("font-size", fs!(12)), ("white-space", "nowrap"), ("font-variant-numeric", "tabular-nums")]))
        .nest("& [data-head] [data-chevron]", pairs(&[("display", "grid"), ("color", MUTED), ("transition", "rotate .15s")]))
        .nest("& [data-head][aria-expanded=true] [data-chevron]", pairs(&[("rotate", "90deg")]))
        .nest("& [data-mark]", pairs(&[("flex", "none"), ("display", "grid"), ("place-items", "center"), ("width", "18px"), ("height", "18px"), ("border-radius", "50%"), ("color", "#fff"), ("background", MUTED)]))
        .nest("&[data-status=done] [data-mark]", pairs(&[("background", "var(--color-success, #16a34a)")]))
        .nest("&[data-status=error] [data-mark]", pairs(&[("background", "var(--color-destructive, #dc2626)")]))
        .nest("&[data-status=approval] [data-mark]", pairs(&[("background", "var(--color-warning, #d97706)")]))
        .nest("&[data-status=pending] [data-mark]", pairs(&[("background", "none"), ("border", "1.5px dashed"), ("border-color", MUTED), ("box-sizing", "border-box")]))
        .nest("&[data-status=running] [data-mark]", pairs(&[("box-sizing", "border-box"), ("background", "none"), ("border", "2px solid color-mix(in srgb, var(--color-primary) 25%, transparent)"), ("border-top-color", PRIMARY), ("animation", "ui-spin .8s linear infinite")]))
        .nest("& [data-body]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(8)), ("padding", pad!(4, 12, 12)), ("border-top", "var(--border-width, 1px) solid var(--color-border)")]))
        .nest("& [data-part] > small", pairs(&[("display", "block"), ("margin", "8px 0 4px"), ("color", MUTED), ("font-size", fs!(12))]))
        .nest("& pre", pairs(&[("margin", "0"), ("max-height", "260px"), ("overflow", "auto"), ("padding", pad!(8, 10)), ("border-radius", "var(--radius-sm)"), ("background", SUBTLE), ("font-family", FONT), ("font-size", fs!(12)), ("line-height", "1.5"), ("white-space", "pre-wrap"), ("overflow-wrap", "anywhere")]))
        .nest("& [data-error]", pairs(&[("margin", "8px 0 0"), ("color", "var(--color-destructive, #dc2626)")]))
        .nest("& [data-ask]", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", gap!(8)), ("padding-top", "8px")]))
        .nest("& [data-ask] span", pairs(&[("margin-inline-end", "auto")]))
        .nest("& [data-ask] button", pairs(&[("height", ctl!(32)), ("padding", pad!(0, 12)), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-control, var(--radius-md))"), ("background", SURFACE), ("color", TEXT), ("font", "inherit"), ("font-weight", "var(--font-weight-medium, 500)"), ("cursor", "pointer")]))
        .nest("& [data-ask] button[data-yes]", pairs(&[("border-color", "transparent"), ("background", PRIMARY_FILL), ("color", crate::ON_PRIMARY)]))
        .nest("@media (prefers-reduced-motion: reduce)", pairs(&[]).nest("&[data-status=running] [data-mark]", pairs(&[("animation-duration", "3s")])))
});

impl Component for ToolCall {
    fn name(&self) -> &'static str {
        "ui-tool-call"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Something an assistant or a job did, or asks leave to do: what, how it went, and what went in and out.", props: CALL_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut config = widget::config(cx, node, &["title", "status", "name", "input", "output", "error", "duration"], &[], &["bindApproval"])?;
        config.insert("open".into(), json!(flag(cx, node, "open", false)));
        config.insert("locale".into(), json!(crate::locale(cx, node)));
        cx.global_css("ui-spin", crate::SPIN);
        let l = cx.label("Action");
        mount(cx, node, &CALL, StyleBlock::new(), "ui-tool-call", config, l)
    }
}

// ---------------------------------------------------------------- trace

pub struct Trace;

const TRACE_PROPS: &[PropDef] = &[
    PropDef::new("spans", PropKind::Any, "The steps: `[{ id, parent, name, kind, start, duration, status, input, output, error, tokens, cost }]`, times in milliseconds from any start; may use state").required(),
    PropDef::new("kinds", PropKind::Any, "Colours by `kind`: `{ model: \"#7c3aed\" }`"),
    PropDef::new("bindSpan", PropKind::StateKey, "State key that gets the step clicked"),
    PropDef::new("expanded", PropKind::Any, "How deep it starts open (default every step)"),
    PropDef::new("height", PropKind::Any, "Most it grows to before it scrolls"),
    PropDef::new("title", PropKind::Any, "Shown over it"),
    PropDef::new("label", PropKind::Any, "Names it for screen readers (default `Trace`)"),
];

static TRACE: LazyLock<StyleBlock> = LazyLock::new(|| {
    base()
        .nest("&", pairs(&[("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-card, var(--radius-md))"), ("overflow", "hidden")]))
        .nest("& [data-head]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("gap", gap!(4, 14)), ("padding", pad!(8, 12)), ("border-bottom", "var(--border-width, 1px) solid var(--color-border)"), ("color", MUTED)]))
        .nest("& [data-head] b", pairs(&[("margin-inline-end", "auto"), ("color", TEXT), ("font-weight", "var(--font-weight-medium, 500)")]))
        .nest("& [role=tree]", pairs(&[("max-height", "var(--tr-h, none)"), ("overflow", "auto")]))
        .nest("& [role=treeitem]", pairs(&[("display", "grid"), ("grid-template-columns", "minmax(150px, 34%) minmax(0, 1fr) 64px"), ("align-items", "center"), ("gap", gap!(8)), ("min-height", "30px"), ("padding", pad!(2, 12)), ("cursor", "pointer"), ("outline", "none")]))
        .nest("& [role=treeitem]:hover", pairs(&[("background", SUBTLE)]))
        .nest("& [role=treeitem]:focus-visible", pairs(&[("box-shadow", "inset 0 0 0 2px var(--color-ring, var(--color-muted))")]))
        .nest("& [role=treeitem][aria-selected=true]", pairs(&[("background", "color-mix(in srgb, var(--color-primary) 10%, transparent)")]))
        .nest("& [data-name]", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", gap!(4)), ("min-width", "0"), ("padding-inline-start", "calc(var(--d, 0) * 16px)")]))
        .nest("& [data-name] > span", pairs(&[("overflow", "hidden"), ("text-overflow", "ellipsis"), ("white-space", "nowrap")]))
        .nest("& [data-twist]", pairs(&[("flex", "none"), ("width", "16px"), ("display", "grid"), ("color", MUTED), ("transition", "rotate .15s")]))
        .nest("& [aria-expanded=true] [data-twist]", pairs(&[("rotate", "90deg")]))
        .nest("& [data-lane]", pairs(&[("position", "relative"), ("height", "14px"), ("border-radius", "4px"), ("background", "repeating-linear-gradient(to right, color-mix(in srgb, var(--color-text) 8%, transparent) 0 1px, transparent 1px 25%)")]))
        .nest("& [data-span]", pairs(&[("--c", PRIMARY), ("position", "absolute"), ("top", "2px"), ("bottom", "2px"), ("min-width", "3px"), ("border-radius", "3px"), ("background", "var(--c)"), ("opacity", "0.85")]))
        .nest("& [data-span][data-status=running]", pairs(&[("background", "repeating-linear-gradient(-45deg, var(--c) 0 5px, color-mix(in srgb, var(--c) 60%, transparent) 5px 10px)")]))
        .nest("& [data-status=error] [data-span]", pairs(&[("--c", "var(--color-destructive, #dc2626)")]))
        .nest("& [data-status=error] [data-name] > span", pairs(&[("color", "var(--color-destructive, #dc2626)")]))
        .nest("& [data-took]", pairs(&[("color", MUTED), ("font-size", fs!(12)), ("text-align", "end"), ("font-variant-numeric", "tabular-nums")]))
        .nest("& [data-detail]", pairs(&[("padding", pad!(8, 12, 12)), ("border-top", "var(--border-width, 1px) solid var(--color-border)"), ("background", SUBTLE)]))
        .nest("& [data-detail] dl", pairs(&[("display", "grid"), ("grid-template-columns", "max-content minmax(0, 1fr)"), ("gap", gap!(4, 12)), ("margin", "0")]))
        .nest("& [data-detail] dt", pairs(&[("color", MUTED)]))
        .nest("& [data-detail] dd", pairs(&[("margin", "0"), ("overflow-wrap", "anywhere")]))
        .nest("& pre", pairs(&[("margin", "0"), ("max-height", "200px"), ("overflow", "auto"), ("font-family", FONT), ("font-size", fs!(12)), ("white-space", "pre-wrap"), ("overflow-wrap", "anywhere")]))
        .nest("@media (max-width: 560px)", pairs(&[]).nest("& [role=treeitem]", pairs(&[("grid-template-columns", "minmax(0, 1fr) 64px")])).nest("& [data-lane]", pairs(&[("display", "none")])))
});

impl Component for Trace {
    fn name(&self) -> &'static str {
        "ui-trace"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "The steps of a run, nested and against time: an assistant's, a job's, a request's. A step opens to what went in and out of it.", props: TRACE_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut config = widget::config(cx, node, &["spans", "title"], &["kinds", "expanded"], &["bindSpan"])?;
        config.insert("locale".into(), json!(crate::locale(cx, node)));
        let mut extra = StyleBlock::new();
        if let Some(h) = node.prop("height")
            && let Some(v) = cx.style_value(node, "max-height", h)?
        {
            extra.push("--tr-h", v);
        }
        let l = cx.label("Trace");
        mount(cx, node, &TRACE, extra, "ui-trace", config, l)
    }
}

// ---------------------------------------------------------------- questionnaire

pub struct Questionnaire;

const QUESTION_PROPS: &[PropDef] = &[
    PropDef::new("questions", PropKind::Any, "`[{ id, question, description, type, options, other, required, min, max, labels }]`; `type`: choice (default), choices, text, number, scale, yesno; may use state").required(),
    PropDef::new("bind", PropKind::StateKey, "State key for the answers, `{ id: answer }`").required(),
    PropDef::new("mode", PropKind::Enum(&["steps", "all"]), "One question at a time (default), or all of them"),
    PropDef::new("bindDone", PropKind::StateKey, "State key that gets true once it's submitted; also a `submit` event with the answers"),
    PropDef::new("skippable", PropKind::Bool, "Questions that aren't required can be skipped (default true)"),
    PropDef::new("submitLabel", PropKind::Any, "Default `Send`"),
    PropDef::new("doneText", PropKind::Any, "Shown once it's submitted, over the answers"),
    PropDef::new("title", PropKind::Any, "Shown over it"),
    PropDef::new("label", PropKind::Any, "Names it for screen readers (default `Questions`)"),
];

static QUESTIONS: LazyLock<StyleBlock> = LazyLock::new(|| {
    base()
        .nest("&", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(12)), ("padding", pad!(16)), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-card, var(--radius-lg))"), ("background", BG_CARD)]))
        .nest("& [data-q-title]", pairs(&[("display", "flex"), ("gap", gap!(8)), ("align-items", "baseline"), ("font-weight", "var(--font-weight-semibold, 600)")]))
        .nest("& [data-q-title] small", pairs(&[("margin-inline-start", "auto"), ("color", MUTED), ("font-weight", "400")]))
        .nest("& fieldset", pairs(&[("min-width", "0"), ("margin", "0"), ("padding", "0"), ("border", "0"), ("display", "flex"), ("flex-direction", "column"), ("gap", gap!(6))]))
        .nest("& legend", pairs(&[("padding", "0"), ("margin-bottom", gap!(6)), ("font-weight", "var(--font-weight-medium, 500)")]))
        .nest("& legend [data-req]", pairs(&[("color", "var(--color-destructive, #dc2626)")]))
        .nest("& [data-q-more]", pairs(&[("margin", "-4px 0 2px"), ("color", MUTED), ("font-size", fs!(13))]))
        .nest("& label[data-opt]", pairs(&[("display", "flex"), ("align-items", "flex-start"), ("gap", gap!(10)), ("padding", pad!(9, 12)), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-md)"), ("cursor", "pointer")]))
        .nest("& label[data-opt]:hover", pairs(&[("background", SUBTLE)]))
        .nest("& label[data-opt]:has(input:checked)", pairs(&[("border-color", PRIMARY), ("background", "color-mix(in srgb, var(--color-primary) 8%, transparent)")]))
        .nest("& label[data-opt]:has(input:focus-visible)", forms::focus_ring())
        .nest("& label[data-opt] input", pairs(&[("flex", "none"), ("margin", "3px 0 0"), ("accent-color", PRIMARY)]))
        .nest("& label[data-opt] small", pairs(&[("display", "block"), ("color", MUTED)]))
        .nest("& [data-scale]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("gap", gap!(6))]))
        .nest("& [data-scale] label", pairs(&[("position", "relative"), ("display", "grid"), ("place-items", "center"), ("min-width", "40px"), ("height", "40px"), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-md)"), ("cursor", "pointer")]))
        .nest("& [data-scale] input", pairs(&[("position", "absolute"), ("inset", "0"), ("opacity", "0"), ("cursor", "pointer"), ("margin", "0")]))
        .nest("& [data-scale] label:has(input:checked)", pairs(&[("border-color", "transparent"), ("background", PRIMARY_FILL), ("color", crate::ON_PRIMARY)]))
        .nest("& [data-scale] label:has(input:focus-visible)", forms::focus_ring())
        .nest("& [data-ends]", pairs(&[("display", "flex"), ("justify-content", "space-between"), ("color", MUTED), ("font-size", fs!(12))]))
        .nest("& :is(textarea, input[type=text])", pairs(&[("box-sizing", "border-box"), ("width", "100%"), ("padding", pad!(8, 12)), ("border", "var(--border-width, 1px) solid var(--color-border)"), ("border-radius", "var(--radius-control, var(--radius-md))"), ("background", "transparent"), ("color", TEXT), ("font", "inherit")]))
        .nest("& :is(textarea, input[type=text]):focus-visible", forms::focus_ring())
        .nest("& [data-error]", pairs(&[("margin", "0"), ("color", "var(--color-destructive, #dc2626)"), ("font-size", fs!(13))]))
        .nest("& [data-foot]", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", gap!(8))]))
        .nest("& [data-foot] [data-push]", pairs(&[("margin-inline-start", "auto")]))
        .nest("& [data-foot] button", pairs(&[("height", ctl!(34)), ("padding", pad!(0, 14)), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-control, var(--radius-md))"), ("background", SURFACE), ("color", TEXT), ("font", "inherit"), ("font-weight", "var(--font-weight-medium, 500)"), ("cursor", "pointer")]))
        .nest("& [data-foot] button[data-primary]", pairs(&[("border-color", "transparent"), ("background", PRIMARY_FILL), ("color", crate::ON_PRIMARY)]))
        .nest("& progress", pairs(&[("width", "100%"), ("height", "4px"), ("accent-color", PRIMARY)]))
        .nest("& dl", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(8)), ("margin", "0")]))
        .nest("& dt", pairs(&[("color", MUTED), ("font-size", fs!(13))]))
        .nest("& dd", pairs(&[("margin", "0"), ("white-space", "pre-wrap")]))
});

impl Component for Questionnaire {
    fn name(&self) -> &'static str {
        "ui-questionnaire"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Questions answered one at a time or together: choices, text, numbers, a scale. For an assistant's questions, surveys and intake forms.", props: QUESTION_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        if cx.prop_string(node, "bind").is_none() {
            return Err(RenderError::new(node, "`ui-questionnaire` needs `bind`"));
        }
        let mut config = widget::config(cx, node, &["questions", "submitLabel", "doneText", "title"], &["mode"], &["bind", "bindDone"])?;
        config.insert("skippable".into(), json!(flag(cx, node, "skippable", true)));
        let l = cx.label("Questions");
        mount(cx, node, &QUESTIONS, StyleBlock::new(), "ui-questionnaire", config, l)
    }
}
