//! `ui-diff` (two versions compared, as text or field by field), `ui-log` (lines that
//! follow as they arrive) and `ui-code-editor` (code typed over its highlighted copy).

use crate::{
    BG_POPOVER, BORDER, MUTED, RING, SUBTLE, SURFACE, TEXT, forms, instance_id, lit,
    widget::{self, flag, pairs},
};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Map, Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("devtools.js");
const FONT: &str = "var(--font-mono, ui-monospace, SFMono-Regular, Menlo, Consolas, monospace)";

fn frame() -> StyleBlock {
    forms::native_scheme(pairs(&[("display", "flex"), ("flex-direction", "column"), ("min-width", "0"), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-card, var(--radius-lg))"), ("background", SURFACE), ("color", TEXT), ("font-size", fs!(13)), ("overflow", "hidden")]))
        .nest("& [hidden]", pairs(&[("display", "none !important")]))
        .nest("& [data-d-bar]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("align-items", "center"), ("gap", gap!(6)), ("padding", pad!(6, 10)), ("border-bottom", "var(--border-width, 1px) solid var(--color-border)"), ("color", MUTED)]))
        .nest("& [data-d-title]", pairs(&[("color", TEXT), ("font-weight", "var(--font-weight-medium, 500)")]))
        .nest("& [data-push]", pairs(&[("margin-inline-start", "auto")]))
        .nest("& [data-d-bar] button", pairs(&[("display", "inline-flex"), ("align-items", "center"), ("gap", gap!(4)), ("height", "28px"), ("padding", pad!(0, 8)), ("border", "0"), ("border-radius", "var(--radius-sm)"), ("background", "none"), ("color", MUTED), ("font", "inherit"), ("cursor", "pointer")]))
        .nest("& [data-d-bar] button:hover", pairs(&[("background", SUBTLE), ("color", TEXT)]))
        .nest("& [data-d-bar] button[aria-pressed=true]", pairs(&[("background", SUBTLE), ("color", TEXT), ("font-weight", "var(--font-weight-medium, 500)")]))
        .nest("& :is(button, input):focus-visible", forms::focus_ring())
        .nest("& [data-d-scroll]", pairs(&[("overflow", "auto"), ("max-height", "var(--d-h, none)"), ("outline", "none")]))
        .nest("& [data-d-scroll]:focus-visible", pairs(&[("box-shadow", "inset 0 0 0 2px var(--color-ring, var(--color-muted))")]))
}

fn height(cx: &RenderCx<'_>, node: &Node) -> Result<StyleBlock, RenderError> {
    let mut s = StyleBlock::new();
    if let Some(h) = node.prop("height")
        && let Some(v) = cx.style_value(node, "max-height", h)?
    {
        s.push("--d-h", v);
    }
    Ok(s)
}

fn scripts(cx: &mut RenderCx<'_>) {
    widget::kit(cx);
    cx.global_js(irender_html::CODE_JS_KEY, irender_html::CODE_JS);
    cx.global_js("ui-devtools", SCRIPT);
}

fn mount(cx: &mut RenderCx<'_>, node: &Node, style: &'static StyleBlock, name: &'static str, config: Map<String, Json>, label: &str) -> Result<(), RenderError> {
    scripts(cx);
    let h = height(cx, node)?;
    let mut el = cx.element("div", node);
    el.base_style_ref(style).base_style(h).attr("role", "group").attr("aria-label", node.prop("label").cloned().or(node.prop("title").cloned()).unwrap_or_else(|| lit(cx.label(label)))).behavior(name, Json::Object(config));
    cx.open(&el, node)?;
    cx.close("div");
    Ok(())
}

// ---------------------------------------------------------------- diff

pub struct Diff;

const DIFF_PROPS: &[PropDef] = &[
    PropDef::new("before", PropKind::Any, "The earlier version: text, or an object or list; may use state").required(),
    PropDef::new("after", PropKind::Any, "The later version").required(),
    PropDef::new("mode", PropKind::Enum(&["text", "fields"]), "Line by line (default), or `fields`: two objects, a row per field"),
    PropDef::new("view", PropKind::Enum(&["split", "unified"]), "Side by side (default; one column on a narrow screen) or one under the other"),
    PropDef::new("bindView", PropKind::StateKey, "State key for the view, set by its buttons"),
    PropDef::new("language", PropKind::String, "Highlights code, as `ui-code-block`'s `language`"),
    PropDef::new("context", PropKind::Number, "Unchanged lines kept around a change (default 3); the rest fold"),
    PropDef::new("labels", PropKind::Any, "The versions' names (default `[Before, After]`), and with `fields`, use `fieldLabels`"),
    PropDef::new("fieldLabels", PropKind::Any, "With `fields`: `{ field: label }`"),
    PropDef::new("unchanged", PropKind::Bool, "With `fields`: show fields that didn't change too"),
    PropDef::new("wrap", PropKind::Bool, "Wrap long lines (default true)"),
    PropDef::new("title", PropKind::Any, "Shown in the bar, e.g. a file or record's name"),
    PropDef::new("height", PropKind::Any, "Most it grows to before it scrolls"),
    PropDef::new("label", PropKind::Any, "Names it for screen readers"),
];

static DIFF: LazyLock<StyleBlock> = LazyLock::new(|| {
    irender_html::code_token_styles(frame())
        .nest("& table", pairs(&[("width", "100%"), ("border-collapse", "collapse"), ("font-family", FONT), ("font-size", fs!(12)), ("line-height", "1.55")]))
        .nest("& table[data-fields]", pairs(&[("font-family", "inherit"), ("font-size", fs!(14))]))
        .nest("& [data-no]", pairs(&[("width", "1%"), ("min-width", "32px"), ("padding", pad!(0, 8)), ("color", MUTED), ("text-align", "end"), ("vertical-align", "top"), ("user-select", "none"), ("font-variant-numeric", "tabular-nums")]))
        .nest("& [data-line]", pairs(&[("padding", pad!(0, 8)), ("vertical-align", "top"), ("white-space", "pre"), ("tab-size", "4")]))
        .nest("&[data-wrap] [data-line]", pairs(&[("white-space", "pre-wrap"), ("overflow-wrap", "anywhere")]))
        .nest("&[data-view=split] [data-line]", pairs(&[("width", "49%")]))
        .nest("&[data-view=split] [data-no]:nth-of-type(3)", pairs(&[("border-inline-start", "var(--border-width, 1px) solid var(--color-border)")]))
        .nest("& [data-k=add]", pairs(&[("background", "color-mix(in srgb, var(--color-success, #16a34a) 12%, transparent)")]))
        .nest("& [data-k=del]", pairs(&[("background", "color-mix(in srgb, var(--color-destructive, #dc2626) 11%, transparent)")]))
        .nest("& [data-k=none]", pairs(&[("background", SUBTLE)]))
        .nest("& ins, & del", pairs(&[("text-decoration", "none"), ("border-radius", "3px")]))
        .nest("& ins", pairs(&[("background", "color-mix(in srgb, var(--color-success, #16a34a) 32%, transparent)")]))
        .nest("& del", pairs(&[("background", "color-mix(in srgb, var(--color-destructive, #dc2626) 28%, transparent)")]))
        .nest("& [data-fold] td", pairs(&[("padding", "0"), ("background", SUBTLE)]))
        .nest("& [data-fold] button", pairs(&[("width", "100%"), ("padding", pad!(4, 12)), ("border", "0"), ("background", "none"), ("color", MUTED), ("font", "inherit"), ("text-align", "start"), ("cursor", "pointer")]))
        .nest("& [data-fold] button:hover", pairs(&[("color", TEXT)]))
        .nest("& [data-added]", pairs(&[("color", "var(--color-success, #16a34a)")]))
        .nest("& [data-removed]", pairs(&[("color", "var(--color-destructive, #dc2626)")]))
        .nest("& [data-fields] :is(th, td)", pairs(&[("padding", pad!(8, 12)), ("border-bottom", "var(--border-width, 1px) solid var(--color-border)"), ("text-align", "start"), ("vertical-align", "top"), ("overflow-wrap", "anywhere")]))
        .nest("& [data-fields] thead th", pairs(&[("color", MUTED), ("font-weight", "var(--font-weight-medium, 500)"), ("font-size", fs!(13))]))
        .nest("& [data-fields] tbody th", pairs(&[("width", "22%"), ("overflow-wrap", "normal"), ("font-weight", "var(--font-weight-medium, 500)")]))
        .nest("& [data-fields] tbody tr:last-child > *", pairs(&[("border-bottom", "0")]))
        .nest("& [data-none]", pairs(&[("margin", "0"), ("padding", pad!(20)), ("color", MUTED), ("text-align", "center")]))
        .nest("& [data-sign]", pairs(&[("display", "inline-block"), ("width", "1.2em"), ("color", MUTED), ("user-select", "none")]))
});

impl Component for Diff {
    fn name(&self) -> &'static str {
        "ui-diff"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Two versions compared: text or code line by line, with the words that changed, or two records field by field.", props: DIFF_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut config = widget::config(cx, node, &["before", "after", "labels", "fieldLabels", "title"], &["mode", "view", "language", "context"], &["bindView"])?;
        config.insert("wrap".into(), json!(flag(cx, node, "wrap", true)));
        config.insert("unchanged".into(), json!(flag(cx, node, "unchanged", false)));
        mount(cx, node, &DIFF, "ui-diff", config, "Changes")
    }
}

// ---------------------------------------------------------------- log

pub struct Log;

const LOG_PROPS: &[PropDef] = &[
    PropDef::new("lines", PropKind::Any, "The lines: texts, or `{ time, level, text, source }`; from state, e.g. fed by `ui-live-data`").required(),
    PropDef::new("levels", PropKind::Any, "The levels offered as filters, e.g. `[debug, info, warn, error]` (default: those in the lines)"),
    PropDef::new("search", PropKind::Bool, "A search box that marks matches and hides other lines (default true)"),
    PropDef::new("time", PropKind::Enum(&["time", "datetime", "relative", "none"]), "How a line's time reads (default `time`)"),
    PropDef::new("follow", PropKind::Bool, "Keeps to the newest line until scrolled up (default true)"),
    PropDef::new("wrap", PropKind::Bool, "Wrap long lines (default true; a button changes it)"),
    PropDef::new("maxLines", PropKind::Number, "The most drawn, the newest (default 2000)"),
    PropDef::new("bindLine", PropKind::StateKey, "State key that gets the line clicked"),
    PropDef::new("download", PropKind::String, "A button saves the lines under this name"),
    PropDef::new("title", PropKind::Any, "Shown in the bar"),
    PropDef::new("emptyText", PropKind::Any, "Default `Nothing yet.`"),
    PropDef::new("height", PropKind::Any, "Its lines' height (default 320)"),
    PropDef::new("label", PropKind::Any, "Names it for screen readers"),
    PropDef::new("locale", PropKind::String, "Default: the document's `lang`, else the browser's"),
];

static LOG: LazyLock<StyleBlock> = LazyLock::new(|| {
    frame()
        .nest("& [data-log-lines]", pairs(&[("height", "var(--d-h, 320px)"), ("padding", pad!(6, 0)), ("font-family", FONT), ("font-size", fs!(12)), ("line-height", "1.6")]))
        .nest("& [data-log-line]", pairs(&[("display", "flex"), ("gap", gap!(10)), ("padding", pad!(0, 10)), ("border-inline-start", "3px solid transparent"), ("content-visibility", "auto"), ("contain-intrinsic-size", "auto 19px")]))
        .nest("& [data-log-line][data-click]", pairs(&[("cursor", "pointer")]))
        .nest("& [data-log-line]:hover", pairs(&[("background", SUBTLE)]))
        .nest("& [data-log-line][aria-current=true]", pairs(&[("background", "color-mix(in srgb, var(--color-primary) 12%, transparent)")]))
        .nest("& [data-log-line] time", pairs(&[("flex", "none"), ("color", MUTED), ("user-select", "none")]))
        .nest("& [data-log-level]", pairs(&[("flex", "none"), ("min-width", "5ch"), ("font-weight", "600"), ("text-transform", "uppercase"), ("color", "var(--lv, var(--color-muted))")]))
        .nest("& [data-log-source]", pairs(&[("flex", "none"), ("max-width", "18ch"), ("overflow", "hidden"), ("text-overflow", "ellipsis"), ("white-space", "nowrap"), ("color", MUTED)]))
        .nest("& [data-log-text]", pairs(&[("flex", "1"), ("min-width", "0"), ("white-space", "pre")]))
        .nest("&[data-wrap] [data-log-text]", pairs(&[("white-space", "pre-wrap"), ("overflow-wrap", "anywhere")]))
        .nest("& [data-lv=info]", pairs(&[("--lv", "var(--color-info, #2563eb)")]))
        .nest("& [data-lv=success]", pairs(&[("--lv", "var(--color-success, #16a34a)")]))
        .nest("& [data-lv=warn]", pairs(&[("--lv", "var(--color-warning, #d97706)"), ("border-inline-start-color", "var(--lv)")]))
        .nest("& [data-lv=error]", pairs(&[("--lv", "var(--color-destructive, #dc2626)"), ("border-inline-start-color", "var(--lv)"), ("background", "color-mix(in srgb, var(--color-destructive, #dc2626) 6%, transparent)")]))
        .nest("& mark", pairs(&[("border-radius", "2px"), ("background", "color-mix(in srgb, var(--color-warning, #f59e0b) 45%, transparent)"), ("color", "inherit")]))
        .nest("& [data-d-bar] input", pairs(&[("height", "28px"), ("min-width", "0"), ("width", "170px"), ("padding", pad!(0, 8)), ("border", "var(--border-width, 1px) solid var(--color-border)"), ("border-radius", "var(--radius-sm)"), ("background", "transparent"), ("color", TEXT), ("font", "inherit")]))
        .nest("& [data-log-wrap]", pairs(&[("position", "relative")]))
        .nest("& [data-log-latest]", pairs(&[("position", "absolute"), ("bottom", "10px"), ("inset-inline-end", "14px"), ("display", "inline-flex"), ("align-items", "center"), ("gap", gap!(4)), ("height", "30px"), ("padding", pad!(0, 12)), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "999px"), ("background", BG_POPOVER), ("color", TEXT), ("font", "inherit"), ("cursor", "pointer"), ("box-shadow", "var(--elevation-popover, 0 4px 12px rgb(0 0 0 / 0.15))")]))
        .nest("& [data-none]", pairs(&[("padding", pad!(20)), ("color", MUTED), ("text-align", "center")]))
});

impl Component for Log {
    fn name(&self) -> &'static str {
        "ui-log"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Lines that follow as they arrive: levels to filter by, a search, wrapping, and saving. Terminal colours in the text are drawn.", props: LOG_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut config = widget::config(cx, node, &["lines", "levels", "title", "emptyText"], &["time", "maxLines", "download"], &["bindLine"])?;
        for (f, d) in [("search", true), ("follow", true), ("wrap", true)] {
            config.insert(f.into(), json!(flag(cx, node, f, d)));
        }
        config.insert("locale".into(), json!(crate::locale(cx, node)));
        crate::sheets(cx);
        mount(cx, node, &LOG, "ui-log", config, "Log")
    }
}

// ---------------------------------------------------------------- code editor

pub struct CodeEditor;

const EDITOR_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key for the text").required(),
    PropDef::new("language", PropKind::Any, "Highlights it, as `ui-code-block`'s `language`; may use state"),
    PropDef::new("rows", PropKind::Number, "Lines tall to start with (default 12)"),
    PropDef::new("maxRows", PropKind::Number, "Grows to this many lines, then scrolls (default 30)"),
    PropDef::new("lineNumbers", PropKind::Bool, "Default true"),
    PropDef::new("wrap", PropKind::Bool, "Wrap long lines (default false)"),
    PropDef::new("indent", PropKind::Number, "Spaces a Tab adds (default 2)"),
    PropDef::new("diagnostics", PropKind::Any, "What's wrong, from the page: `[{ line, message, severity }]` (`error`, `warning` or `info`), marked and listed; may use state"),
    PropDef::new("completions", PropKind::Any, "Words offered as it's typed, or with Ctrl+Space: texts or `{ label, insert, detail }`; may use state"),
    PropDef::new("bindCursor", PropKind::StateKey, "State key that gets `{ line, column }`"),
    PropDef::new("placeholder", PropKind::Any, "Shown while it's empty"),
    PropDef::new("readOnly", PropKind::Any, "Shown, not changed"),
    PropDef::new("label", PropKind::Any, "Names the editor (default `Code`)"),
    PropDef::new("name", PropKind::String, "Form field name"),
];

static EDITOR: LazyLock<StyleBlock> = LazyLock::new(|| {
    let layer = || pairs(&[("box-sizing", "border-box"), ("margin", "0"), ("padding", "8px 12px"), ("border", "0"), ("font-family", FONT), ("font-size", fs!(13)), ("line-height", "1.55"), ("letter-spacing", "0"), ("tab-size", "var(--ce-tab, 2)"), ("white-space", "pre"), ("overflow-wrap", "normal")]);
    irender_html::code_token_styles(frame())
        .nest("&:focus-within", forms::focus_ring().set("border-color", RING))
        .nest("& [data-ce-body]", pairs(&[("display", "flex"), ("min-width", "0"), ("overflow", "auto"), ("max-height", "var(--ce-max, 480px)")]))
        .nest("& [data-ce-gutter]", layer().set("flex", "none").set("position", "sticky").set("inset-inline-start", "0").set("z-index", "2").set("padding-inline", "10px").set("min-width", "44px").set("background", SUBTLE).set("color", MUTED).set("text-align", "end").set("user-select", "none"))
        .nest("& [data-ce-gutter] > span", pairs(&[("display", "block")]))
        .nest("& [data-ce-gutter] > span[data-sev=error]", pairs(&[("color", "#fff"), ("background", "var(--color-destructive, #dc2626)"), ("border-radius", "3px"), ("margin-inline", "-6px"), ("padding-inline", "6px")]))
        .nest("& [data-ce-gutter] > span[data-sev=warning]", pairs(&[("color", "#fff"), ("background", "var(--color-warning, #d97706)"), ("border-radius", "3px"), ("margin-inline", "-6px"), ("padding-inline", "6px")]))
        .nest("& [data-ce-main]", pairs(&[("position", "relative"), ("flex", "1 0 auto"), ("min-width", "0"), ("min-height", "calc(var(--ce-rows, 12) * 1.55em + 16px)"), ("font-size", fs!(13))]))
        .nest("&[data-wrap] [data-ce-main]", pairs(&[("flex", "1 1 0")]))
        .nest("& [data-ce-code]", layer().set("min-height", "100%").set("pointer-events", "none"))
        .nest("& [data-ce-code] [data-ln]", pairs(&[("display", "block"), ("min-height", "1.55em")]))
        .nest("& [data-ce-code] [data-ln][data-cur]", pairs(&[("background", "color-mix(in srgb, var(--color-text) 4%, transparent)"), ("margin-inline", "-12px"), ("padding-inline", "12px")]))
        .nest("& [data-ce-code] [data-ln][data-sev=error]", pairs(&[("text-decoration", "underline wavy var(--color-destructive, #dc2626)"), ("text-decoration-skip-ink", "none")]))
        .nest("& [data-ce-code] [data-ln][data-sev=warning]", pairs(&[("text-decoration", "underline wavy var(--color-warning, #d97706)")]))
        .nest("& textarea", layer().set("position", "absolute").set("inset", "0").set("width", "100%").set("height", "100%").set("resize", "none").set("outline", "none").set("overflow", "hidden").set("background", "transparent").set("color", "transparent").set("caret-color", TEXT))
        .nest("& textarea::placeholder", pairs(&[("color", MUTED)]))
        .nest("& textarea::selection", pairs(&[("background", "color-mix(in srgb, var(--color-primary) 28%, transparent)")]))
        .nest("&[data-wrap] :is([data-ce-code], textarea)", pairs(&[("white-space", "pre-wrap"), ("overflow-wrap", "anywhere")]))
        .nest("& [data-ce-list]", pairs(&[("position", "absolute"), ("z-index", "5"), ("box-sizing", "border-box"), ("min-width", "180px"), ("max-height", "200px"), ("overflow-y", "auto"), ("padding", "4px"), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-md)"), ("background", BG_POPOVER), ("box-shadow", "var(--elevation-popover, 0 6px 18px rgb(0 0 0 / 0.12))"), ("font-size", fs!(13))]))
        .nest("& [data-ce-list] [role=option]", pairs(&[("display", "flex"), ("gap", "12px"), ("padding", "4px 8px"), ("border-radius", "var(--radius-sm)"), ("font-family", FONT), ("cursor", "pointer"), ("white-space", "nowrap")]))
        .nest("& [data-ce-list] [aria-selected=true]", pairs(&[("background", SUBTLE)]))
        .nest("& [data-ce-list] small", pairs(&[("margin-inline-start", "auto"), ("color", MUTED), ("font-family", "var(--font-body, system-ui, sans-serif)")]))
        .nest("& [data-ce-problems]", pairs(&[("margin", "0"), ("padding", pad!(4, 10)), ("list-style", "none"), ("max-height", "120px"), ("overflow-y", "auto"), ("border-top", "var(--border-width, 1px) solid var(--color-border)")]))
        .nest("& [data-ce-problems]:empty", pairs(&[("display", "none")]))
        .nest("& [data-ce-problems] button", pairs(&[("padding", "2px 0"), ("border", "0"), ("background", "none"), ("color", "var(--color-destructive, #dc2626)"), ("font", "inherit"), ("text-align", "start"), ("cursor", "pointer")]))
        .nest("& [data-ce-problems] [data-sev=warning] button", pairs(&[("color", "var(--color-warning, #d97706)")]))
        .nest("& [data-ce-problems] [data-sev=info] button", pairs(&[("color", MUTED)]))
        .nest("& [data-ce-foot]", pairs(&[("display", "flex"), ("justify-content", "flex-end"), ("gap", gap!(12)), ("padding", pad!(2, 10)), ("border-top", "var(--border-width, 1px) solid var(--color-border)"), ("color", MUTED), ("font-size", fs!(12)), ("font-variant-numeric", "tabular-nums")]))
});

impl Component for CodeEditor {
    fn name(&self) -> &'static str {
        "ui-code-editor"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Code or a template, typed over its highlighted copy: line numbers, indenting, closing brackets, comments, words offered, and problems marked.", props: EDITOR_PROPS, parts: &["textarea"], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let key = cx.prop_string(node, "bind").ok_or_else(|| RenderError::new(node, "`ui-code-editor` needs `bind`"))?;
        let rows = widget::number(cx, node, "rows").unwrap_or(12.0).clamp(1.0, 200.0);
        let max = widget::number(cx, node, "maxRows").unwrap_or(30.0).max(rows);
        let indent = widget::number(cx, node, "indent").unwrap_or(2.0).clamp(1.0, 8.0);
        let numbers = flag(cx, node, "lineNumbers", true);
        let mut config = widget::config(cx, node, &["language", "diagnostics", "completions", "readOnly"], &[], &["bind", "bindCursor"])?;
        config.insert("indent".into(), json!(indent));
        scripts(cx);
        let mut el = cx.element("div", node);
        el.base_style_ref(&EDITOR)
            .base_style(StyleBlock::new().set("--ce-rows", rows.to_string()).set("--ce-max", format!("calc({max} * 13px * 1.55 * var(--font-scale, 1) + 16px)")).set("--ce-tab", indent.to_string()))
            .behavior("ui-code-editor", Json::Object(config));
        if flag(cx, node, "wrap", false) {
            el.attr("data-wrap", true);
        }
        cx.open(&el, node)?;
        let mut body = Element::new("div");
        body.attr("data-ce-body", true);
        cx.open(&body, node)?;
        if numbers {
            let mut g = Element::new("div");
            g.attr("data-ce-gutter", true).attr("aria-hidden", "true");
            cx.open(&g, node)?;
            cx.close("div");
        }
        let mut main = Element::new("div");
        main.attr("data-ce-main", true);
        cx.open(&main, node)?;
        let mut pre = Element::new("pre");
        pre.attr("data-ce-code", true).attr("aria-hidden", "true");
        cx.open(&pre, node)?;
        cx.close("pre");
        let mut t = cx.part("textarea", node, "textarea");
        t.attr("id", instance_id(cx, node, "code"))
            .attr("spellcheck", "false")
            .attr("autocapitalize", "off")
            .attr("autocomplete", "off")
            .attr("autocorrect", "off")
            .attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Code"))))
            .attr("aria-description", cx.label("Escape, then Tab, leaves the editor"))
            .text(lit(expr::stringify(&cx.lookup(&[key]))));
        for (prop, attr) in [("placeholder", "placeholder"), ("name", "name"), ("readOnly", "readonly")] {
            if let Some(v) = node.prop(prop) {
                t.attr(attr, v.clone());
            }
        }
        cx.open(&t, node)?;
        cx.close("textarea");
        cx.close("div");
        cx.close("div");
        let mut ul = Element::new("ul");
        ul.attr("data-ce-problems", true).attr("aria-label", cx.label("Problems"));
        cx.open(&ul, node)?;
        cx.close("ul");
        let mut foot = Element::new("div");
        foot.attr("data-ce-foot", true).attr("aria-hidden", "true");
        cx.open(&foot, node)?;
        cx.close("div");
        cx.close("div");
        Ok(())
    }
}
