//! Code: `ui-code-block`, highlighted code with line numbers, marked lines and a copy
//! button, and `ui-json-editor`, a JSON text area that checks as you type and writes the
//! value to state while it's valid. The server draws the code as plain lines (it reads
//! fine without the script); `code.js` highlights it and does the rest.

use crate::{ACCENT, BORDER, MUTED, SURFACE, TEXT, data_table::dynamic, expr_val, forms, id_with, instance_id, lit};
use irender_core::{
    ComponentInfo, Node, PropDef, PropKind, StyleBlock,
    expr::{self, Expr},
};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("code.js");

pub struct CodeBlock;
pub struct JsonEditor;

/// The languages the highlighter knows, with their other names; any other is plain text.
const LANGUAGES: &[&str] = &[
    "text", "json", "jsonc", "yaml", "yml", "js", "javascript", "ts", "typescript", "jsx", "tsx", "rust", "rs", "python", "py", "bash", "sh",
    "shell", "zsh", "console", "html", "xml", "svg", "css", "sql", "postgres", "postgresql",
];

const PROPS: &[PropDef] = &[
    PropDef::new("code", PropKind::Any, "The code. Anything not a string (an object from state, say) shows as JSON").required(),
    PropDef::new("language", PropKind::String, "json, yaml, js (ts), rust, python, bash, html, css, sql or text (the default)"),
    PropDef::new("title", PropKind::Any, "A file name or caption in the bar above"),
    PropDef::new("lineNumbers", PropKind::Bool, "Numbers the lines"),
    PropDef::new("highlight", PropKind::Any, "Lines to mark: numbers and ranges, e.g. `[2, \"5-7\"]`"),
    PropDef::new("copy", PropKind::Bool, "A copy button (default true)"),
    PropDef::new("wrap", PropKind::Bool, "Wraps long lines instead of scrolling"),
    PropDef::new("maxHeight", PropKind::Any, "Scrolls past this height"),
    PropDef::new("label", PropKind::Any, "Names the code for screen readers (default: the title, or `Code`)"),
];
const EDITOR_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key for the value: written while the text is valid JSON, shown formatted when it changes from outside").required(),
    PropDef::new("bindValid", PropKind::StateKey, "State key set to whether the text is valid"),
    PropDef::new("type", PropKind::String, "What the value must be: `object`, `array` or `any` (the default)"),
    PropDef::new("rows", PropKind::Number, "Lines tall (default 10)"),
    PropDef::new("indent", PropKind::Number, "Spaces per indent for Tab and Format (default 2)"),
    PropDef::new("label", PropKind::Any, "Names the editor (default `JSON`); a `ui-field` around it names it too"),
    PropDef::new("format", PropKind::Bool, "A Format button (default true)"),
];

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

const FONT: &str = "var(--font-mono, ui-monospace, SFMono-Regular, Menlo, Consolas, monospace)";
const CODE_BG: &str = "var(--color-code, color-mix(in srgb, var(--color-text) 4%, var(--color-surface, #fff)))";

static BLOCK: LazyLock<StyleBlock> = LazyLock::new(|| {
    irender_html::code_token_styles(pairs(&[
        ("position", "relative"),
        ("display", "flex"),
        ("flex-direction", "column"),
        ("margin", "0"),
        ("min-width", "0"),
        ("border", "1px solid var(--color-border)"),
        ("border-radius", "var(--radius-lg)"),
        ("background", CODE_BG),
        ("color", TEXT),
        ("overflow", "hidden"),
    ]))
    .nest(
        "& [data-code-head]",
        pairs(&[("display", "flex"), ("align-items", "center"), ("gap", gap!(8)), ("min-height", "36px"), ("padding-block", pad!(0, 0)), ("padding-inline", pad!(14, 6)), ("border-bottom", "1px solid var(--color-border)"), ("font-size", fs!(13)), ("color", MUTED)]),
    )
    .nest("& [data-code-title]", pairs(&[("color", TEXT), ("font-weight", "500"), ("overflow", "hidden"), ("text-overflow", "ellipsis"), ("white-space", "nowrap")]))
    .nest("& [data-code-lang]", pairs(&[("margin-inline-start", "auto"), ("font-family", FONT), ("font-size", fs!(12)), ("text-transform", "lowercase")]))
    // With no bar, the copy button floats in the corner.
    .nest("& > [data-code-copy]", pairs(&[("position", "absolute"), ("top", "6px"), ("inset-inline-end", "6px"), ("z-index", "1"), ("background", CODE_BG)]))
    .nest(
        "& [data-code-copy]",
        pairs(&[
            ("display", "grid"),
            ("place-items", "center"),
            ("width", "28px"),
            ("height", "28px"),
            ("padding", "0"),
            ("border", "0"),
            ("border-radius", "var(--radius-sm)"),
            ("background", "transparent"),
            ("color", MUTED),
            ("cursor", "pointer"),
        ]),
    )
    .nest("& [data-code-copy]:hover", pairs(&[("background", ACCENT), ("color", TEXT)]))
    .nest("& [data-code-copy]:focus-visible", forms::focus_ring())
    .nest("& [data-code-copy] [data-done]", pairs(&[("display", "none")]))
    .nest("& [data-code-copy][data-copied] [data-done]", pairs(&[("display", "block"), ("color", "var(--color-success, #16a34a)")]))
    .nest("& [data-code-copy][data-copied] [data-idle]", pairs(&[("display", "none")]))
    .nest(
        "& pre",
        pairs(&[
            ("margin", "0"),
            ("padding", pad!(12, 0)),
            ("overflow", "auto"),
            ("max-height", "var(--code-max, none)"),
            ("font-family", FONT),
            ("font-size", fs!(13)),
            ("line-height", "1.6"),
            ("tab-size", "2"),
            ("counter-reset", "ln"),
        ]),
    )
    .nest("& pre:focus-visible", forms::focus_ring().set("outline-offset", "-2px"))
    .nest("& code", pairs(&[("display", "block"), ("min-width", "max-content"), ("font", "inherit")]))
    .nest("&[data-wrap] code", pairs(&[("min-width", "0"), ("white-space", "pre-wrap"), ("overflow-wrap", "anywhere")]))
    .nest("& [data-ln]", pairs(&[("display", "block"), ("padding", pad!(0, 16)), ("min-height", "1.6em")]))
    .nest("& [data-ln][data-hl]", pairs(&[("background", "color-mix(in srgb, var(--color-primary, #2563eb) 10%, transparent)"), ("box-shadow", "inset 2px 0 var(--color-primary, #2563eb)")]))
    .nest("&:dir(rtl) [data-ln][data-hl]", pairs(&[("box-shadow", "inset -2px 0 var(--color-primary, #2563eb)")]))
    .nest("&[data-numbered] [data-ln]", pairs(&[("padding-inline-start", "0"), ("counter-increment", "ln")]))
    .nest(
        "&[data-numbered] [data-ln]::before",
        pairs(&[
            ("content", "counter(ln) / \"\""),
            ("display", "inline-block"),
            ("width", "var(--code-gutter, 3ch)"),
            ("margin-inline-end", gap!(16)),
            ("padding-inline-start", pad!(12)),
            ("text-align", "end"),
            ("color", MUTED),
            ("opacity", ".7"),
            ("user-select", "none"),
        ]),
    )
    .nest("&[data-wrap][data-numbered] [data-ln]", pairs(&[("padding-inline-start", "calc(var(--code-gutter, 3ch) + 28px)"), ("text-indent", "calc(-1 * (var(--code-gutter, 3ch) + 28px))")]))
    .nest(
        "& [data-code-live]",
        pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("overflow", "hidden"), ("clip-path", "inset(50%)"), ("white-space", "nowrap")]),
    )
});

static EDITOR: LazyLock<StyleBlock> = LazyLock::new(|| {
    irender_html::code_token_styles(pairs(&[("display", "flex"), ("flex-direction", "column"), ("min-width", "0"), ("border", "1px solid var(--color-border)"), ("border-radius", "var(--radius-md)"), ("background", SURFACE), ("color", TEXT), ("overflow", "hidden")]))
        .nest("&:focus-within", pairs(&[("border-color", "var(--color-ring, var(--color-muted))"), ("box-shadow", "0 0 0 3px color-mix(in srgb, var(--color-ring, var(--color-muted)) 25%, transparent)")]))
        .nest("&[data-invalid]", pairs(&[("border-color", "var(--color-destructive, #dc2626)")]))
        .nest(
            "& [data-je-bar]",
            pairs(&[("display", "flex"), ("align-items", "center"), ("gap", gap!(8)), ("min-height", "34px"), ("padding-block", pad!(0, 0)), ("padding-inline", pad!(12, 6)), ("border-bottom", format!("1px solid {BORDER}").as_str()), ("background", CODE_BG), ("font-size", fs!(12)), ("color", MUTED)]),
        )
        .nest("& [data-je-status]", pairs(&[("display", "inline-flex"), ("align-items", "center"), ("gap", gap!(6))]))
        .nest("& [data-je-status]::before", pairs(&[("content", "''"), ("width", "7px"), ("height", "7px"), ("border-radius", "50%"), ("background", "var(--color-success, #16a34a)")]))
        .nest("&[data-invalid] [data-je-status]::before", pairs(&[("background", "var(--color-destructive, #dc2626)")]))
        .nest("& [data-je-format]", pairs(&[("margin-inline-start", "auto")]))
        .nest("& [data-je-area]", pairs(&[("position", "relative"), ("display", "flex"), ("min-height", "0"), ("font-family", FONT), ("font-size", fs!(13)), ("line-height", "20px")]))
        .nest(
            "& [data-je-gutter-box]",
            pairs(&[("flex", "0 0 auto"), ("min-width", "3ch"), ("padding-block", pad!(10, 10)), ("padding-inline", pad!(12, 10)), ("overflow", "hidden"), ("border-inline-end", format!("1px solid {BORDER}").as_str()), ("background", CODE_BG), ("color", MUTED), ("text-align", "end"), ("user-select", "none")]),
        )
        .nest("& [data-je-gutter] span", pairs(&[("display", "block"), ("opacity", ".7")]))
        .nest("& [data-je-gutter] span[data-err]", pairs(&[("opacity", "1"), ("color", "var(--color-destructive, #dc2626)"), ("font-weight", "700")]))
        .nest("& [data-je-stack]", pairs(&[("position", "relative"), ("flex", "1"), ("min-width", "0"), ("display", "grid")]))
        // The highlighted copy and the text area over it share every metric.
        .nest(
            "& :is(textarea, [data-je-pre])",
            pairs(&[
                ("grid-area", "1 / 1"),
                ("box-sizing", "border-box"),
                ("margin", "0"),
                ("padding", pad!(10, 12)),
                ("border", "0"),
                ("font", "inherit"),
                ("line-height", "inherit"),
                ("letter-spacing", "0"),
                ("tab-size", "2"),
                ("white-space", "pre"),
                ("overflow-wrap", "normal"),
            ]),
        )
        .nest("& [data-je-pre]", pairs(&[("overflow", "hidden"), ("pointer-events", "none")]))
        .nest("& [data-je-code]", pairs(&[("display", "block"), ("font", "inherit"), ("will-change", "transform")]))
        .nest("& [data-je-code] [data-ln]", pairs(&[("display", "block"), ("min-width", "max-content")]))
        .nest(
            "& [data-je-code] [data-ln][data-err]",
            pairs(&[("background", "color-mix(in srgb, var(--color-destructive, #dc2626) 10%, transparent)"), ("text-decoration", "underline wavy var(--color-destructive, #dc2626)"), ("text-underline-offset", "4px")]),
        )
        .nest(
            "& textarea",
            pairs(&[("position", "relative"), ("z-index", "1"), ("width", "100%"), ("height", "var(--je-height)"), ("min-height", "60px"), ("resize", "vertical"), ("overflow", "auto"), ("background", "transparent"), ("color", "transparent"), ("caret-color", TEXT), ("outline", "none")]),
        )
        .nest("& textarea::selection", pairs(&[("background", "color-mix(in srgb, var(--color-primary, #2563eb) 25%, transparent)"), ("color", "transparent")]))
        .nest("& textarea::placeholder", pairs(&[("color", MUTED)]))
        .nest(
            "& [data-je-error]",
            pairs(&[("margin", "0"), ("padding", pad!(6, 12)), ("border-top", "1px solid color-mix(in srgb, var(--color-destructive, #dc2626) 35%, transparent)"), ("background", "color-mix(in srgb, var(--color-destructive, #dc2626) 7%, var(--color-surface, #fff))"), ("color", "var(--color-destructive, #dc2626)"), ("font-size", fs!(12))]),
        )
        .nest("& [data-je-error]:empty", pairs(&[("display", "none")]))
        .nest("& [data-je-hint]", pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("overflow", "hidden"), ("clip-path", "inset(50%)"), ("white-space", "nowrap")]))
});

const COPY: &str = r#"<svg data-idle width="15" height="15" style="width:calc(15px * var(--size-icon, 1));height:calc(15px * var(--size-icon, 1))" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round" aria-hidden="true"><rect x="5.5" y="5.5" width="8" height="8" rx="1.5"/><path d="M10.5 3.5v-.5a1 1 0 0 0-1-1h-6a1 1 0 0 0-1 1v6a1 1 0 0 0 1 1h.5"/></svg><svg data-done width="15" height="15" style="width:calc(15px * var(--size-icon, 1));height:calc(15px * var(--size-icon, 1))" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3.5 8.5 6.5 11.5 12.5 4.5"/></svg>"#;

/// The code as text: strings as they are, anything else as JSON, two spaces an indent.
fn as_text(v: &Json) -> String {
    match v {
        Json::String(s) => s.clone(),
        Json::Null => String::new(),
        other => serde_json::to_string_pretty(other).unwrap_or_default(),
    }
}

/// Lines marked by `highlight`: numbers and `a-b` ranges.
fn marked(cx: &RenderCx<'_>, node: &Node) -> Result<Vec<(u64, u64)>, RenderError> {
    let Some(v) = cx.prop(node, "highlight") else { return Ok(Vec::new()) };
    let list = match v {
        Json::Array(l) => l,
        Json::Null => return Ok(Vec::new()),
        one => vec![one],
    };
    list.iter()
        .map(|m| {
            let s = expr::stringify(m);
            let (a, b) = s.split_once('-').unwrap_or((&s, &s));
            match (a.trim().parse::<u64>(), b.trim().parse::<u64>()) {
                (Ok(a), Ok(b)) if a >= 1 && a <= b => Ok((a, b)),
                _ => Err(RenderError::new(node, format!("`highlight`: `{s}` isn't a line number or a range like `3-5`"))),
            }
        })
        .collect()
}

impl Component for CodeBlock {
    fn name(&self) -> &'static str {
        "ui-code-block"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Code, highlighted, with line numbers, marked lines and a copy button. `code` can come from state: an object shows as JSON.",
            props: PROPS,
            parts: &["head", "pre", "copy"],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let raw = node.prop("code").ok_or_else(|| RenderError::new(node, "`ui-code-block` needs `code`"))?;
        let language = cx.prop_string(node, "language").unwrap_or_else(|| "text".into()).to_lowercase();
        if !LANGUAGES.contains(&language.as_str()) {
            return Err(RenderError::new(node, format!("`language` `{language}` isn't one the highlighter knows: {}", LANGUAGES.join(", "))));
        }
        let text = as_text(&cx.prop(node, "code").unwrap_or(Json::Null));
        let text = text.strip_suffix('\n').unwrap_or(&text).to_string();
        let marks = marked(cx, node)?;
        let numbered = cx.prop(node, "lineNumbers").is_some_and(|v| expr::truthy(&v));
        let copy = cx.prop(node, "copy").is_none_or(|v| expr::truthy(&v));
        // A title that comes to nothing (a list item without one, say) is no title.
        let title = node.prop("title").filter(|t| cx.is_reactive(t) || cx.prop_string(node, "title").is_some_and(|s| !s.is_empty())).cloned();
        let code = dynamic(cx, node, "code", raw)?;

        cx.global_js(irender_html::CODE_JS_KEY, irender_html::CODE_JS);
        cx.global_js("ui-code", SCRIPT);
        let mut root = cx.element("figure", node);
        root.base_style_ref(&BLOCK).behavior(
            "ui-code-block",
            json!({ "code": code, "language": language, "highlight": marks.iter().map(|(a, b)| format!("{a}-{b}")).collect::<Vec<_>>() }),
        );
        let mut vars = StyleBlock::new();
        if numbered {
            root.attr("data-numbered", true);
            // Room for the longest number.
            vars = vars.set("--code-gutter", format!("{}ch", text.lines().count().max(1).to_string().len().max(2)));
        }
        if let Some(h) = cx.prop(node, "maxHeight") {
            vars = vars.set("--code-max", if let Json::Number(n) = &h { format!("{n}px") } else { expr::stringify(&h) });
        }
        root.base_style(vars);
        if cx.prop(node, "wrap").is_some_and(|v| expr::truthy(&v)) {
            root.attr("data-wrap", true);
        }
        cx.open(&root, node)?;

        let copy_button = |cx: &mut RenderCx<'_>| -> Result<(), RenderError> {
            let mut b = cx.part("button", node, "copy");
            b.attr("type", "button").attr("data-code-copy", true).attr("aria-label", cx.label("Copy code")).attr("title", cx.label("Copy"));
            cx.open(&b, node)?;
            cx.markup(COPY);
            cx.close("button");
            Ok(())
        };
        let bar = title.is_some() || language != "text";
        if bar {
            let mut head = cx.part("figcaption", node, "head");
            head.attr("data-code-head", true);
            cx.open(&head, node)?;
            if let Some(t) = &title {
                let mut s = Element::new("span");
                s.attr("data-code-title", true).text(t.clone());
                cx.open(&s, node)?;
                cx.close("span");
            }
            let mut l = Element::new("span");
            l.attr("data-code-lang", true).text(lit(if language == "text" { String::new() } else { language.clone() }));
            cx.open(&l, node)?;
            cx.close("span");
            if copy {
                copy_button(cx)?;
            }
            cx.close("figcaption");
        } else if copy {
            copy_button(cx)?;
        }

        // A scrolling region, so it takes focus to scroll with the keys.
        let mut pre = cx.part("pre", node, "pre");
        let label = node.prop("label").cloned().or(title).unwrap_or_else(|| lit(cx.label("Code")));
        pre.attr("tabindex", "0").attr("aria-label", label);
        cx.open(&pre, node)?;
        let mut c = Element::new("code");
        c.attr("data-code-body", true);
        cx.open(&c, node)?;
        for (i, line) in text.split('\n').enumerate() {
            let n = i as u64 + 1;
            let mut s = Element::new("span");
            s.attr("data-ln", true);
            if marks.iter().any(|(a, b)| (*a..=*b).contains(&n)) {
                s.attr("data-hl", true);
            }
            s.text(lit(line));
            cx.open(&s, node)?;
            cx.close("span");
        }
        cx.close("code");
        cx.close("pre");
        if copy {
            let mut live = Element::new("span");
            live.attr("data-code-live", true).attr("aria-live", "polite");
            cx.open(&live, node)?;
            cx.close("span");
        }
        cx.close("figure");
        Ok(())
    }
}

impl Component for JsonEditor {
    fn name(&self) -> &'static str {
        "ui-json-editor"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A JSON editor: highlighted, with line numbers, a check as you type that marks the line in error, and Format. The value \
                  goes to `bind` while the text is valid.",
            props: EDITOR_PROPS,
            parts: &["bar", "textarea", "error"],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let key = cx.prop_string(node, "bind").ok_or_else(|| RenderError::new(node, "`ui-json-editor` needs `bind`"))?;
        let kind = cx.prop_string(node, "type").unwrap_or_else(|| "any".into());
        if !["any", "object", "array"].contains(&kind.as_str()) {
            return Err(RenderError::new(node, "`type` must be `object`, `array` or `any`"));
        }
        let rows = cx.prop(node, "rows").map_or(10.0, |v| expr::to_num(&v));
        if !(2.0..=200.0).contains(&rows) {
            return Err(RenderError::new(node, "`rows` must be from 2 to 200"));
        }
        let indent = cx.prop(node, "indent").map_or(2.0, |v| expr::to_num(&v));
        if !(1.0..=8.0).contains(&indent) {
            return Err(RenderError::new(node, "`indent` must be from 1 to 8"));
        }
        let indent = indent as usize;
        // The text: state's value, formatted with the indent asked for.
        let value = cx.eval(&expr_val(Expr::Path(vec![key.clone()])));
        let text = match &value {
            Json::Null => String::new(),
            // Two spaces an indent, made `indent` (a string's new lines are escaped, so
            // every leading space is indent).
            v => serde_json::to_string_pretty(v)
                .unwrap_or_default()
                .lines()
                .map(|l| {
                    let body = l.trim_start_matches(' ');
                    format!("{}{body}", " ".repeat((l.len() - body.len()) / 2 * indent))
                })
                .collect::<Vec<_>>()
                .join("\n"),
        };
        let id = instance_id(cx, node, "json");

        cx.global_js(irender_html::CODE_JS_KEY, irender_html::CODE_JS);
        cx.global_js("ui-code", SCRIPT);
        let mut root = cx.element("div", node);
        root.base_style_ref(&EDITOR)
            .base_style(StyleBlock::new().set("--je-height", format!("{}px", rows as u32 * 20 + 20)))
            .attr("data-je", true)
            .behavior("ui-json-editor", json!({ "bind": key, "bindValid": cx.prop_string(node, "bindValid"), "type": kind, "indent": indent }));
        cx.open(&root, node)?;

        let mut bar = cx.part("div", node, "bar");
        bar.attr("data-je-bar", true);
        cx.open(&bar, node)?;
        let mut st = Element::new("span");
        st.attr("data-je-status", true).text(lit(cx.label("JSON")));
        cx.open(&st, node)?;
        cx.close("span");
        if cx.prop(node, "format").is_none_or(|v| expr::truthy(&v)) {
            let mut f = Element::new("button");
            forms::button_look(&mut f, "ghost", "sm");
            f.attr("type", "button").attr("data-je-format", true).text(lit(cx.label("Format")));
            cx.open(&f, node)?;
            cx.close("button");
        }
        cx.close("div");

        let mut area = Element::new("div");
        area.attr("data-je-area", true);
        cx.open(&area, node)?;
        let mut gbox = Element::new("div");
        gbox.attr("data-je-gutter-box", true).attr("aria-hidden", "true");
        cx.open(&gbox, node)?;
        let mut g = Element::new("div");
        g.attr("data-je-gutter", true);
        cx.open(&g, node)?;
        for n in 1..=text.split('\n').count() {
            let mut s = Element::new("span");
            s.text(lit(n.to_string()));
            cx.open(&s, node)?;
            cx.close("span");
        }
        cx.close("div");
        cx.close("div");
        let mut stack = Element::new("div");
        stack.attr("data-je-stack", true);
        cx.open(&stack, node)?;
        let mut pre = Element::new("pre");
        pre.attr("data-je-pre", true).attr("aria-hidden", "true");
        cx.open(&pre, node)?;
        let mut code = Element::new("code");
        code.attr("data-je-code", true);
        cx.open(&code, node)?;
        for line in text.split('\n') {
            let mut s = Element::new("span");
            s.attr("data-ln", true).text(lit(if line.is_empty() { " " } else { line }));
            cx.open(&s, node)?;
            cx.close("span");
        }
        cx.close("code");
        cx.close("pre");
        let mut ta = cx.part("textarea", node, "textarea");
        ta.attr("spellcheck", "false")
            .attr("autocapitalize", "off")
            .attr("autocomplete", "off")
            .attr("wrap", "off")
            .attr("aria-describedby", match &id {
                irender_core::Val::Lit(v) => lit(format!("{0}-error {0}-hint", expr::stringify(v))),
                other => id_with(other, "-error"),
            })
            .attr("aria-invalid", "false")
            .text(lit(text.clone()));
        // A `ui-field` names it (a `<label for>` this part's id, or `aria-labelledby`);
        // otherwise `label` does.
        if !node.parts.get("textarea").is_some_and(|p| p.attrs.contains_key("id") || p.attrs.contains_key("aria-labelledby")) {
            ta.attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("JSON"))));
        }
        cx.open(&ta, node)?;
        cx.close("textarea");
        cx.close("div");
        let mut hint = Element::new("span");
        hint.attr("data-je-hint", true).attr("id", id_with(&id, "-hint")).text(lit(cx.label("Tab indents. Press Escape, then Tab, to leave the editor.")));
        cx.open(&hint, node)?;
        cx.close("span");
        cx.close("div");

        let mut err = cx.part("p", node, "error");
        err.attr("data-je-error", true).attr("id", id_with(&id, "-error")).attr("aria-live", "polite");
        cx.open(&err, node)?;
        cx.close("p");
        cx.close("div");
        Ok(())
    }
}
