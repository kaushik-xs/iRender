//! Markdown for iRender documents: `ui-markdown` shows it, and `ui-markdown-editor`
//! writes it, as formatted text anyone can edit or as Markdown with a live preview.
//!
//! ```no_run
//! use irender_core::Format;
//! use irender_html::Engine;
//!
//! let mut engine = Engine::new();
//! irender_markdown_kit::register(&mut engine.registry);
//! let doc = irender_core::parse(r#"{ "version": 1, "root": { "type": "ui-markdown",
//!     "props": { "text": "**Hello**, _world_." } } }"#, Format::Json)?;
//! let page = engine.render(&engine.resolve(&doc)?, &serde_json::json!({}))?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! The server renders the Markdown with [`render`]; `markdown.js`, the same renderer
//! in the browser, re-renders text from state and the editor's preview. Both escape
//! every piece of text and keep only safe link and image URLs, so Markdown from
//! anyone (a chat message, a comment) can be shown.
//!
//! Colours, radii and sizes come from the theme's tokens (`color.text`, `color.muted`,
//! `color.border`, `color.primary`, `radius.md`, `space.scale`, `font.scale`), with
//! fallbacks, so it fits irender-ui-kit's look and its theme options.

mod markdown;

pub use markdown::render;

use indexmap::IndexMap;
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, Val, expr};
use irender_html::{Component, Element, RenderCx, RenderError, Registry};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

/// The renderer for the browser (`window.__irenderMarkdown.render`).
const RENDERER: &str = include_str!("markdown.js");
const VIEW: &str = include_str!("view.js");
const EDITOR: &str = include_str!("editor.js");

/// Adds `ui-markdown` and `ui-markdown-editor` to `registry`.
pub fn register(registry: &mut Registry) {
    registry.register(Markdown).register(MarkdownEditor);
}

/// A size that follows the theme's `space.scale`, like irender-ui-kit's.
macro_rules! sp {
    ($n:literal) => {
        concat!("calc(", $n, "px * var(--space-scale, 1))")
    };
}
/// A padding: also `space.inset`.
macro_rules! pad {
    ($n:literal) => {
        concat!("calc(", $n, "px * var(--space-scale, 1) * var(--space-inset, 1))")
    };
}
/// A gap: also `space.gap`.
macro_rules! gap {
    ($n:literal) => {
        concat!("calc(", $n, "px * var(--space-scale, 1) * var(--space-gap, 1))")
    };
}
/// A control's height or width: also `size.control`.
macro_rules! ctl {
    ($n:literal) => {
        concat!("calc(", $n, "px * var(--space-scale, 1) * var(--size-control, 1))")
    };
}
/// A font size that follows the theme's `font.scale`.
macro_rules! fs {
    ($n:literal) => {
        concat!("calc(", $n, "px * var(--font-scale, 1))")
    };
}

const TEXT: &str = "var(--color-text, inherit)";
const MUTED: &str = "var(--color-muted, #71717a)";
const BORDER: &str = "var(--color-border, #e4e4e7)";
const PRIMARY: &str = "var(--color-primary, currentColor)";
const SUBTLE: &str = "var(--color-subtle, color-mix(in srgb, currentColor 6%, transparent))";

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

/// How rendered Markdown looks, on the element that holds it.
static PROSE: LazyLock<StyleBlock> = LazyLock::new(|| {
    irender_html::code_token_styles(pairs(&[("color", TEXT), ("font-size", fs!(14)), ("line-height", "calc(var(--font-leading, 1.5) + 0.15)"), ("overflow-wrap", "anywhere")]))
        .nest("& > :first-child", pairs(&[("margin-top", "0")]))
        .nest("& > :last-child", pairs(&[("margin-bottom", "0")]))
        .nest("& :is(p, ul, ol, blockquote, pre, table)", pairs(&[("margin", "0.75em 0")]))
        .nest("& :is(h1, h2, h3, h4, h5, h6)", pairs(&[("margin", "1.2em 0 0.5em"), ("font-family", "var(--font-heading, inherit)"), ("line-height", "1.25"), ("font-weight", "var(--font-weight-semibold, 600)")]))
        .nest("& h1", pairs(&[("font-size", "1.75em")]))
        .nest("& h2", pairs(&[("font-size", "1.4em"), ("padding-bottom", "0.2em"), ("border-bottom", "var(--border-width, 1px) solid var(--color-border, #e4e4e7)")]))
        .nest("& h3", pairs(&[("font-size", "1.2em")]))
        .nest("& :is(h4, h5, h6)", pairs(&[("font-size", "1em")]))
        .nest("& :is(ul, ol)", pairs(&[("padding-inline-start", "1.5em")]))
        .nest("& li + li", pairs(&[("margin-top", "0.25em")]))
        .nest("& li.task", pairs(&[("list-style", "none"), ("margin-inline-start", "-1.3em")]))
        .nest("& li.task > input", pairs(&[("margin-block", "0 0"), ("margin-inline", "0 0.4em"), ("vertical-align", "-0.1em")]))
        .nest("& a", pairs(&[("color", PRIMARY), ("text-decoration", "underline"), ("text-underline-offset", "3px")]))
        .nest(
            "& code",
            pairs(&[
                ("padding", "0.15em 0.35em"),
                ("border-radius", "var(--radius-sm, 4px)"),
                ("background", SUBTLE),
                ("font-family", "ui-monospace, SFMono-Regular, Menlo, monospace"),
                ("font-size", "0.875em"),
            ]),
        )
        .nest(
            "& pre",
            pairs(&[("padding", pad!(12)), ("overflow-x", "auto"), ("border-radius", "var(--radius-md, 8px)"), ("background", SUBTLE)]),
        )
        .nest("& pre code", pairs(&[("padding", "0"), ("background", "none"), ("font-size", "0.85em"), ("overflow-wrap", "normal")]))
        // A code fence, once highlighted: a box with a copy button in its corner, shown on
        // hover and focus (always, where there's no hover).
        .nest("& [data-fence]", pairs(&[("position", "relative"), ("margin", "0.75em 0")]))
        .nest("& [data-fence] > pre", pairs(&[("margin", "0")]))
        .nest(
            "& [data-fence-copy]",
            pairs(&[
                ("position", "absolute"),
                ("top", "6px"),
                ("inset-inline-end", "6px"),
                ("display", "grid"),
                ("place-items", "center"),
                ("width", "26px"),
                ("height", "26px"),
                ("padding", "0"),
                ("border", "var(--border-width, 1px) solid var(--color-border, #e4e4e7)"),
                ("border-radius", "var(--radius-sm, 4px)"),
                ("background", "var(--color-surface, #fff)"),
                ("color", MUTED),
                ("cursor", "pointer"),
                ("opacity", "0"),
                ("transition", "opacity .15s"),
            ]),
        )
        .nest("& [data-fence]:hover [data-fence-copy], & [data-fence-copy]:focus-visible, & [data-fence-copy][data-copied]", pairs(&[("opacity", "1")]))
        .nest("@media (hover: none)", pairs(&[]).nest("& [data-fence-copy]", pairs(&[("opacity", "1")])))
        .nest("& [data-fence-copy]:hover", pairs(&[("color", TEXT)]))
        .nest("& [data-fence-copy]:focus-visible", pairs(&[("outline", "2px solid var(--color-ring, var(--color-muted, #71717a))"), ("outline-offset", "1px")]))
        .nest("& [data-fence-copy] [data-done], & [data-fence-copy][data-copied] [data-idle]", pairs(&[("display", "none")]))
        .nest("& [data-fence-copy][data-copied] [data-done]", pairs(&[("display", "block"), ("color", "var(--color-success, #16a34a)")]))
        .nest(
            "& [data-fence-live]",
            pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("overflow", "hidden"), ("clip-path", "inset(50%)"), ("white-space", "nowrap")]),
        )
        .nest("& blockquote", pairs(&[("padding-inline-start", "1em"), ("border-inline-start", "3px solid var(--color-border, #e4e4e7)"), ("color", MUTED)]))
        .nest("& hr", pairs(&[("margin", "1.5em 0"), ("border", "0"), ("border-top", "var(--border-width, 1px) solid var(--color-border, #e4e4e7)")]))
        .nest("& img", pairs(&[("max-width", "100%"), ("border-radius", "var(--radius-md, 8px)")]))
        .nest("& table", pairs(&[("width", "100%"), ("border-collapse", "collapse"), ("font-size", "0.95em")]))
        .nest("& :is(th, td)", pairs(&[("padding", "0.4em 0.75em"), ("border", "var(--border-width, 1px) solid var(--color-border, #e4e4e7)"), ("text-align", "start")]))
        .nest("& th", pairs(&[("font-weight", "var(--font-weight-semibold, 600)"), ("background", SUBTLE)]))
});

// ---------------------------------------------------------------- ui-markdown

pub struct Markdown;

const MARKDOWN_PROPS: &[PropDef] = &[
    PropDef::new("text", PropKind::Any, "The Markdown; may use state, and then follows it in the browser").required(),
    PropDef::new("tag", PropKind::Tag, "HTML tag (default `div`)"),
    PropDef::new(
        "streaming",
        PropKind::Any,
        "The text is still arriving (a streamed reply): in the browser, formatting left open at its end shows as it will once closed. May use state",
    ),
];

impl Component for Markdown {
    fn name(&self) -> &'static str {
        "ui-markdown"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Markdown shown as formatted text: headings, lists, tables, code, links. Safe for text from anyone: \
                  HTML in it shows as text and only safe links are kept.",
            props: MARKDOWN_PROPS,
            parts: &[],
            children: false,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let tag = cx.prop_string(node, "tag").unwrap_or_else(|| "div".into());
        let text = node.prop("text").cloned().unwrap_or(Val::Lit(Json::Null));
        let html = render(&text_of(&cx.eval(&text)));
        let mut el = cx.element(&tag, node);
        el.base_style_ref(&PROSE).attr("data-markdown", true);
        if cx.is_reactive(&text) {
            cx.global_js("ui-markdown-renderer", RENDERER);
        }
        // From state, or with code fences to highlight: the behaviour, and the highlighter.
        if cx.is_reactive(&text) || html.contains("<pre><code") {
            cx.global_js(irender_html::CODE_JS_KEY, irender_html::CODE_JS);
            cx.global_js("ui-markdown", VIEW);
            let mut config = json!({ "text": cx.is_reactive(&text).then(|| cx.val_js(&text)) });
            if let Some(s) = node.prop("streaming").filter(|_| cx.is_reactive(&text)) {
                config["streaming"] = json!(cx.val_js(s));
            }
            el.behavior("ui-markdown", config);
        }
        cx.open(&el, node)?;
        cx.markup(&html);
        cx.close(&tag);
        Ok(())
    }
}

fn text_of(v: &Json) -> String {
    if v.is_null() { String::new() } else { expr::stringify(v) }
}

// ---------------------------------------------------------------- ui-markdown-editor

pub struct MarkdownEditor;

const MODES: &[&str] = &["rich", "tabs", "split"];
const EDITOR_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key holding the Markdown").required(),
    PropDef::new(
        "mode",
        PropKind::Enum(MODES),
        "`rich`: edit the formatted text, with a Markdown tab (default); `tabs`: Markdown with Write and Preview tabs; \
         `split`: Markdown and its preview side by side",
    ),
    PropDef::new("placeholder", PropKind::Any, "Shown while it's empty"),
    PropDef::new("label", PropKind::Any, "The text area's name for screen readers (default `Markdown`)"),
    PropDef::new("rows", PropKind::Any, "Visible lines (default 10)"),
    PropDef::new("maxLength", PropKind::Any, "A limit, shown as a count under it"),
    PropDef::new("toolbar", PropKind::Bool, "Show the formatting toolbar (default true)"),
    PropDef::new("name", PropKind::String, "Form field name"),
    PropDef::new("disabled", PropKind::Any, "Boolean or expression"),
];

/// The toolbar: command, its name, what it shows, and its shortcut.
// Labels (looked up where they're used): "Heading", "Bold", "Italic", "Strikethrough", "Quote", "Code", "Link", "Bulleted list", "Numbered list", "Task list", "Table", "Image", "Horizontal rule"
const TOOLS: &[(&str, &str, &str, &str)] = &[
    ("heading", "Heading", "H", "⌘⌥1–6"),
    ("bold", "Bold", "B", "⌘B"),
    ("italic", "Italic", "I", "⌘I"),
    ("strike", "Strikethrough", "S", "⌘⇧X"),
    ("|", "", "", ""),
    ("quote", "Quote", "❝", ""),
    ("code", "Code", "</>", "⌘E"),
    ("link", "Link", "🔗", "⌘K"),
    ("|", "", "", ""),
    ("bullets", "Bulleted list", "•", ""),
    ("numbers", "Numbered list", "1.", ""),
    ("tasks", "Task list", "☑", ""),
    ("|", "", "", ""),
    ("table", "Table", "▦", ""),
    ("image", "Image", "", ""),
    ("rule", "Horizontal rule", "―", ""),
];

const IMAGE_ICON: &str = r#"<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="2" y="3" width="12" height="10" rx="1.5"/><circle cx="6" cy="6.5" r="1.2"/><path d="m3 12 3.5-3.5 2.5 2.5 1.5-1.5L13 12"/></svg>"#;

static EDITOR_ROOT: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("display", "flex"),
        ("flex-direction", "column"),
        ("border", "var(--border-width, 1px) solid var(--color-border, #e4e4e7)"),
        ("border-radius", "var(--radius-md, 8px)"),
        ("background", "var(--color-surface, transparent)"),
        ("color", TEXT),
        ("overflow", "hidden"),
        ("transition", "border-color .15s, box-shadow .15s"),
    ])
    .nest(
        ":focus-within",
        pairs(&[
            ("border-color", "var(--color-ring, var(--color-muted))"),
            ("box-shadow", "0 0 0 3px color-mix(in srgb, var(--color-ring, var(--color-muted)) 50%, transparent)"),
        ]),
    )
});
static BAR: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("display", "flex"),
        ("flex-wrap", "wrap"),
        ("align-items", "center"),
        ("gap", gap!(2)),
        ("padding", pad!(4)),
        ("border-bottom", "var(--border-width, 1px) solid var(--color-border, #e4e4e7)"),
        ("background", SUBTLE),
    ])
});
static TOOL: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("display", "inline-flex"),
        ("align-items", "center"),
        ("justify-content", "center"),
        ("min-width", ctl!(28)),
        ("height", ctl!(28)),
        ("padding", "0 6px"),
        ("border", "0"),
        ("border-radius", "var(--radius-sm, 4px)"),
        ("background", "transparent"),
        ("color", "inherit"),
        ("font", "inherit"),
        ("font-size", fs!(13)),
        ("font-weight", "var(--font-weight-semibold, 600)"),
        ("cursor", "pointer"),
    ])
    .nest(":hover", pairs(&[("background", "color-mix(in srgb, var(--color-primary, currentColor) 10%, transparent)")]))
    .nest(":focus-visible", pairs(&[("outline", "2px solid var(--color-ring, var(--color-muted))"), ("outline-offset", "-2px")]))
    .nest("&[data-md=italic]", pairs(&[("font-style", "italic")]))
    .nest("&[data-md=strike]", pairs(&[("text-decoration", "line-through")]))
    .nest(
        "&:is([aria-selected=true], [aria-pressed=true])",
        pairs(&[("background", "var(--color-surface, #fff)"), ("box-shadow", "0 1px 2px rgb(0 0 0 / 0.08)")]),
    )
    .nest(":disabled", pairs(&[("opacity", "0.5"), ("pointer-events", "none")]))
});
static SEP: LazyLock<StyleBlock> =
    LazyLock::new(|| pairs(&[("width", "1px"), ("height", sp!(16)), ("margin", "0 4px"), ("background", BORDER)]));
static TABS: LazyLock<StyleBlock> = LazyLock::new(|| pairs(&[("display", "flex"), ("gap", gap!(2)), ("margin-inline-start", "auto")]));
static BODY: LazyLock<StyleBlock> = LazyLock::new(|| {
    // Fills the editor, so a taller editor gets a taller text area.
    pairs(&[("display", "grid"), ("flex", "1"), ("min-height", "0")])
        .nest("&[data-mode=split]", pairs(&[("grid-template-columns", "1fr 1fr")]))
        .nest("&[data-mode=split] > [data-md-preview]", pairs(&[("border-inline-start", "var(--border-width, 1px) solid var(--color-border, #e4e4e7)")]))
        .nest("&[data-mode=tabs][data-view=write] > [data-md-preview]", pairs(&[("display", "none")]))
        .nest("&[data-mode=tabs][data-view=preview] > textarea", pairs(&[("display", "none")]))
        .nest("&[data-view=rich] > textarea", pairs(&[("display", "none")]))
        .nest("&[data-view=source] > [data-md-rich]", pairs(&[("display", "none")]))
});
/// The formatted text being edited: the preview's look, as a text box.
static RICH: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("position", "relative"), ("outline", "none"), ("cursor", "text")])
        .nest(
            "&[data-empty]::before",
            pairs(&[("content", "attr(data-placeholder)"), ("position", "absolute"), ("color", MUTED), ("pointer-events", "none")]),
        )
        .nest("& li.task > input", pairs(&[("cursor", "pointer"), ("accent-color", PRIMARY)]))
        .nest("&[contenteditable=false]", pairs(&[("opacity", "0.6"), ("cursor", "default")]))
});
static LINKBAR: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("display", "flex"),
        ("align-items", "center"),
        ("gap", gap!(4)),
        ("padding", pad!(4)),
        ("border-bottom", "var(--border-width, 1px) solid var(--color-border, #e4e4e7)"),
    ])
    .nest("&[hidden]", pairs(&[("display", "none")]))
    .nest(
        "& input",
        pairs(&[
            ("flex", "1"),
            ("min-width", "0"),
            ("height", ctl!(28)),
            ("padding", "0 8px"),
            ("border", "var(--border-width, 1px) solid var(--color-border, #e4e4e7)"),
            ("border-radius", "var(--radius-sm, 4px)"),
            ("background", "transparent"),
            ("color", "inherit"),
            ("font", "inherit"),
            ("font-size", fs!(13)),
        ]),
    )
});
static AREA: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("box-sizing", "border-box"),
        ("width", "100%"),
        ("height", "100%"),
        ("min-height", sp!(160)),
        ("padding", pad!(12)),
        ("border", "0"),
        ("outline", "none"),
        ("resize", "vertical"),
        ("background", "transparent"),
        ("color", "inherit"),
        ("font-family", "ui-monospace, SFMono-Regular, Menlo, monospace"),
        ("font-size", fs!(13)),
        ("line-height", "calc(var(--font-leading, 1.5) + 0.1)"),
        ("tab-size", "2"),
    ])
    .nest("::placeholder", pairs(&[("color", MUTED)]))
});
static PREVIEW: LazyLock<StyleBlock> = LazyLock::new(|| pairs(&[("min-height", sp!(160)), ("padding", pad!(12)), ("overflow", "auto")]));
static EMPTY: LazyLock<StyleBlock> = LazyLock::new(|| pairs(&[("color", MUTED)]));
static FOOT: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("display", "flex"),
        ("justify-content", "space-between"),
        ("gap", gap!(8)),
        ("padding", "4px 10px"),
        ("border-top", "var(--border-width, 1px) solid var(--color-border, #e4e4e7)"),
        ("color", MUTED),
        ("font-size", fs!(12)),
    ])
    .nest("& [data-md-count][data-over]", pairs(&[("color", "var(--color-destructive, #dc2626)"), ("font-weight", "var(--font-weight-semibold, 600)")]))
});

/// A built-in `textarea`, so the text binds to state like any other field.
fn textarea(node: &Node, props: Vec<(&str, Val)>, attrs: Vec<(&str, Val)>) -> Node {
    Node {
        kind: "textarea".into(),
        path: node.path.clone(),
        props: props.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        attrs: attrs.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        class: Vec::new(),
        style: StyleBlock::new(),
        parts: IndexMap::new(),
        on: IndexMap::new(),
        children: Vec::new(),
    }
}

impl Component for MarkdownEditor {
    fn name(&self) -> &'static str {
        "ui-markdown-editor"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A Markdown editor anyone can use: by default the text is edited formatted, as readers see it, with \
                  a toolbar, shortcuts (⌘B, ⌘I, ⌘K, ⌘E) and a Markdown tab; `mode: tabs` or `split` edit the Markdown \
                  with a live preview. The Markdown is in `bind`.",
            props: EDITOR_PROPS,
            parts: &["toolbar", "rich", "input", "preview", "footer"],
            children: false,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let Some(key) = cx.prop_string(node, "bind") else {
            return Err(RenderError::new(node, "`ui-markdown-editor` needs `bind`"));
        };
        let mode = cx.prop_string(node, "mode").filter(|m| MODES.contains(&m.as_str())).unwrap_or_else(|| "rich".into());
        let toolbar = cx.prop(node, "toolbar").is_none_or(|t| expr::truthy(&t));
        let max = cx.prop(node, "maxLength").map(|m| expr::to_num(&m)).filter(|m| m.is_finite() && *m > 0.0).map(|m| m as u64);
        let text = text_of(&cx.lookup(std::slice::from_ref(&key)));
        cx.global_js("ui-markdown-renderer", RENDERER);
        cx.global_js(irender_html::CODE_JS_KEY, irender_html::CODE_JS);
        cx.global_js("ui-markdown-editor", EDITOR);

        let mut root = cx.element("div", node);
        root.base_style_ref(&EDITOR_ROOT).attr("data-md-editor", true).behavior("ui-markdown-editor", json!({ "key": key, "max": max }));
        cx.open(&root, node)?;

        // Labels: "Rich text", "Markdown", "Write", "Preview"
        let views: &[(&str, &str)] = match mode.as_str() {
            "rich" => &[("rich", "Rich text"), ("source", "Markdown")],
            "tabs" => &[("write", "Write"), ("preview", "Preview")],
            _ => &[],
        };
        if toolbar || !views.is_empty() {
            let mut bar = cx.part("div", node, "toolbar");
            bar.base_style_ref(&BAR).attr("role", "toolbar").attr("aria-label", cx.label("Formatting"));
            cx.open(&bar, node)?;
            if toolbar {
                for (cmd, name, shown, keys) in TOOLS {
                    if *cmd == "|" {
                        let mut sep = Element::new("span");
                        sep.base_style_ref(&SEP).attr("aria-hidden", "true");
                        cx.open(&sep, node)?;
                        cx.close("span");
                        continue;
                    }
                    let name = cx.label(name);
                    let title = if keys.is_empty() { name.clone() } else { format!("{name} ({keys})") };
                    let mut b = Element::new("button");
                    b.base_style_ref(&TOOL).attr("type", "button").attr("data-md", *cmd).attr("aria-label", name.as_str()).attr("title", title.as_str());
                    if !shown.is_empty() {
                        b.text(Val::Lit(json!(shown)));
                    }
                    if let Some(d) = node.prop("disabled") {
                        b.attr("disabled", d.clone());
                    }
                    cx.open(&b, node)?;
                    // A picture, in the text's colour (an emoji would be in its own).
                    if *cmd == "image" {
                        cx.markup(IMAGE_ICON);
                    }
                    cx.close("button");
                }
            }
            if !views.is_empty() {
                let mut tabs = Element::new("div");
                tabs.base_style_ref(&TABS).attr("role", "tablist").attr("aria-label", cx.label("View"));
                cx.open(&tabs, node)?;
                for (i, (view, label)) in views.iter().enumerate() {
                    let mut t = Element::new("button");
                    t.base_style_ref(&TOOL)
                        .attr("type", "button")
                        .attr("role", "tab")
                        .attr("data-md-view", *view)
                        .attr("aria-selected", if i == 0 { "true" } else { "false" })
                        .text(Val::Lit(json!(cx.label(label))));
                    cx.open(&t, node)?;
                    cx.close("button");
                }
                cx.close("div");
            }
            cx.close("div");
        }
        if mode == "rich" && toolbar {
            // Where a link's address is typed, under the toolbar.
            let mut bar = Element::new("div");
            bar.base_style_ref(&LINKBAR).attr("data-md-linkbar", true).attr("hidden", true);
            cx.open(&bar, node)?;
            let mut url = Element::new("input");
            url.attr("type", "url").attr("aria-label", cx.label("Link address")).attr("placeholder", cx.label("Paste or type a link"));
            cx.open(&url, node)?;
            // Labels: "Apply", "Remove link", "Cancel"
            for (act, label) in [("apply", "Apply"), ("remove", "Remove link"), ("cancel", "Cancel")] {
                let mut b = Element::new("button");
                b.base_style_ref(&TOOL).attr("type", "button").attr("data-md-link", act).text(Val::Lit(json!(cx.label(label))));
                cx.open(&b, node)?;
                cx.close("button");
            }
            cx.close("div");
            // Where an image's address and description are typed.
            let mut bar = Element::new("div");
            bar.base_style_ref(&LINKBAR).attr("data-md-imagebar", true).attr("hidden", true);
            cx.open(&bar, node)?;
            let mut url = Element::new("input");
            url.attr("type", "url").attr("data-md-image-src", true).attr("aria-label", cx.label("Image address")).attr("placeholder", cx.label("Paste or type the image's address"));
            cx.open(&url, node)?;
            let mut alt = Element::new("input");
            alt.attr("type", "text").attr("data-md-image-alt", true).attr("aria-label", cx.label("Image description")).attr("placeholder", cx.label("Describe the image"));
            cx.open(&alt, node)?;
            // Labels: "Insert", "Cancel"
            for (act, label) in [("insert", "Insert"), ("cancel", "Cancel")] {
                let mut b = Element::new("button");
                b.base_style_ref(&TOOL).attr("type", "button").attr("data-md-image", act).text(Val::Lit(json!(cx.label(label))));
                cx.open(&b, node)?;
                cx.close("button");
            }
            cx.close("div");
        }

        let first = views.first().map_or("write", |v| v.0);
        let mut body = Element::new("div");
        body.base_style_ref(&BODY).attr("data-md-body", true).attr("data-mode", mode.as_str()).attr("data-view", first);
        cx.open(&body, node)?;
        let mut props: Vec<(&str, Val)> = vec![("bind", Val::Lit(json!(key)))];
        props.push(("rows", node.prop("rows").cloned().unwrap_or(Val::Lit(json!(10)))));
        for name in ["placeholder", "name", "disabled"] {
            if let Some(v) = node.prop(name) {
                props.push((name, v.clone()));
            }
        }
        let mut attrs: Vec<(&str, Val)> = vec![
            ("aria-label", node.prop("label").cloned().unwrap_or_else(|| Val::Lit(json!(cx.label("Markdown"))))),
            ("spellcheck", Val::Lit(json!("true"))),
        ];
        if let Some(m) = max {
            attrs.push(("maxlength", Val::Lit(json!(m))));
        }
        let mut area = textarea(node, props, attrs);
        if let Some(p) = node.parts.get("input") {
            area.attrs.extend(p.attrs.clone());
            area.class = p.class.clone();
            area.style = p.style.clone();
        }
        irender_html::Textarea::render_styled(&area, cx, &AREA)?;

        if mode == "rich" {
            // The text shows formatted, as readers will see it, and is edited there.
            let rows = cx.prop(node, "rows").map(|r| expr::to_num(&r)).filter(|r| r.is_finite() && *r > 0.0).unwrap_or(10.0);
            let mut editor = cx.part("div", node, "rich");
            let disabled = node.prop("disabled").is_some_and(|d| !cx.is_reactive(d) && expr::truthy(&cx.eval(d)));
            editor
                .base_style_ref(&PREVIEW)
                .base_style_ref(&PROSE)
                .base_style_ref(&RICH)
                .base_style(pairs(&[("min-height", &format!("calc({rows} * 1.65em)"))]))
                .attr("data-md-rich", true)
                .attr("contenteditable", if disabled { "false" } else { "true" })
                .attr("role", "textbox")
                .attr("aria-multiline", "true")
                .attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| Val::Lit(json!(cx.label("Text")))))
                .attr("spellcheck", "true");
            if let Some(p) = node.prop("placeholder") {
                editor.attr("data-placeholder", p.clone());
            }
            if text.trim().is_empty() {
                editor.attr("data-empty", true);
            }
            cx.open(&editor, node)?;
            cx.markup(if text.trim().is_empty() { "<p><br></p>".into() } else { render(&text) }.as_str());
            cx.close("div");
            cx.close("div");
            return self.footer(node, cx, &text, max, &mode);
        }

        let mut preview = cx.part("div", node, "preview");
        preview.base_style_ref(&PREVIEW).base_style_ref(&PROSE).attr("data-md-preview", true).attr("aria-live", "polite");
        if mode == "tabs" {
            preview.attr("role", "tabpanel");
        }
        cx.open(&preview, node)?;
        if text.trim().is_empty() {
            let mut e = Element::new("p");
            e.base_style_ref(&EMPTY).text(Val::Lit(json!(cx.label("Nothing to preview"))));
            cx.open(&e, node)?;
            cx.close("p");
        } else {
            cx.markup(&render(&text));
        }
        cx.close("div");
        cx.close("div");
        self.footer(node, cx, &text, max, &mode)
    }
}

impl MarkdownEditor {
    /// The note under the editor, and the count when there's a limit; closes the editor.
    fn footer(&self, node: &Node, cx: &mut RenderCx<'_>, text: &str, max: Option<u64>, mode: &str) -> Result<(), RenderError> {
        let note = if mode == "rich" { cx.label("Start a line with #, - or > to format it") } else { cx.label("Markdown supported") };
        let mut foot = cx.part("div", node, "footer");
        foot.base_style_ref(&FOOT);
        cx.open(&foot, node)?;
        cx.open(Element::new("span").text(Val::Lit(json!(note))), node)?;
        cx.close("span");
        if let Some(m) = max {
            let mut count = Element::new("span");
            count.attr("data-md-count", true).attr("aria-live", "polite").text(Val::Lit(json!(format!("{}/{m}", text.chars().count()))));
            cx.open(&count, node)?;
            cx.close("span");
        }
        cx.close("div");
        cx.close("div");
        Ok(())
    }
}
