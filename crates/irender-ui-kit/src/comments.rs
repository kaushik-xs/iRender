//! `ui-comments` (threads with replies, mentions, reactions and resolving) and
//! `ui-mention-input` (a text area that names people with `@`). Comments live in state;
//! `comments.js` draws them, and `mentions.js` is the `@` list both use.

use crate::{
    BG_POPOVER, BORDER, MUTED, PRIMARY, SUBTLE, TEXT, forms, instance_id, lit,
    widget::{self, flag, pairs},
};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock};
use irender_html::{Component, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("comments.js");
const MENTIONS: &str = include_str!("mentions.js");

pub struct Comments;
pub struct MentionInput;

const PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key holding the comments: `[{ id, who, when, text, replies: [...], reactions: { \"👍\": [names] }, resolved, edited }]`; `who` is a name or `{ name, avatar, initials }`, `when` an ISO time").required(),
    PropDef::new("user", PropKind::Any, "Who's commenting: a name or `{ name, avatar }`; without it, comments are only read"),
    PropDef::new("people", PropKind::Any, "Who can be mentioned with `@`: names or `{ name, detail, avatar }`"),
    PropDef::new("reactions", PropKind::Any, "Emoji to react with (default 👍 ❤️ 🎉 👀; `[]` for none)"),
    PropDef::new("resolvable", PropKind::Bool, "A thread can be resolved, and opened again"),
    PropDef::new("replies", PropKind::Bool, "Comments can be replied to (default true)"),
    PropDef::new("editable", PropKind::Bool, "People can edit and delete their own (default true)"),
    PropDef::new("order", PropKind::Enum(&["oldest", "newest"]), "Default `oldest` first, with the box at the foot; `newest` puts it at the top"),
    PropDef::new("placeholder", PropKind::Any, "Default `Write a comment…`"),
    PropDef::new("emptyText", PropKind::Any, "Default `No comments yet.`"),
    PropDef::new("maxLength", PropKind::Number, "The most characters in a comment"),
    PropDef::new("markdown", PropKind::Bool, "Comments are read as a little Markdown: **bold**, *italic*, ~~struck~~, `code`, fenced code, [links](https://…), lists and > quotes (built as elements, never as HTML)"),
    PropDef::new("label", PropKind::Any, "Names it for screen readers (default `Comments`)"),
    PropDef::new("disabled", PropKind::Any, "Nothing can be added or changed"),
    PropDef::new("locale", PropKind::String, "How times are written (default: the document's `lang`, else the browser's)"),
];

const MENTION_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key for the text").required(),
    PropDef::new("people", PropKind::Any, "Who can be named: names or `{ name, detail, avatar }`; may use state").required(),
    PropDef::new("bindMentions", PropKind::StateKey, "State key that gets the names in the text, a list"),
    PropDef::new("trigger", PropKind::String, "What opens the list (default `@`)"),
    PropDef::new("placeholder", PropKind::Any, "Shown while it's empty"),
    PropDef::new("rows", PropKind::Number, "Lines it starts with (default 3); it grows as it's typed in"),
    PropDef::new("maxLength", PropKind::Number, "The most characters"),
    PropDef::new("label", PropKind::Any, "Accessible name, when no visible label names it"),
    PropDef::new("name", PropKind::String, "Form field name"),
    PropDef::new("disabled", PropKind::Any, "Boolean or expression"),
    PropDef::new("invalid", PropKind::Any, "Mark it wrong"),
];

/// The list of people that opens at the caret.
const LIST_CSS: &str = "[data-mention-list] { position: fixed; z-index: 60; inset: auto; margin: 0; box-sizing: border-box; min-width: 200px; max-width: 320px; max-height: 240px; overflow-y: auto; padding: 4px; border: var(--border-width, 1px) solid var(--color-border); border-radius: var(--radius-md); background: var(--color-popover, var(--color-surface)); color: var(--color-text); font-size: 14px; box-shadow: var(--elevation-popover, 0 6px 18px rgb(0 0 0 / 0.12)); }
[data-mention-list] [role=option] { display: flex; align-items: center; gap: 8px; padding: 6px 8px; border-radius: var(--radius-sm); cursor: pointer; }
[data-mention-list] [aria-selected=true] { background: var(--color-accent, color-mix(in srgb, var(--color-text) 7%, transparent)); }
[data-mention-list] small { display: block; color: var(--color-muted); font-size: 12px; }
[data-mention-list] [data-avatar] { flex: none; display: grid; place-items: center; width: 26px; height: 26px; overflow: hidden; border-radius: 50%; background: color-mix(in srgb, var(--color-text) 8%, transparent); color: var(--color-muted); font-size: 11px; font-weight: 500; }
[data-mention-list] [data-avatar] img { width: 100%; height: 100%; object-fit: cover; }";

static ROOT: LazyLock<StyleBlock> = LazyLock::new(|| {
    forms::native_scheme(pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(16)), ("min-width", "0"), ("color", TEXT), ("font-size", fs!(14))]))
        .nest("& ol", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(16)), ("margin", "0"), ("padding", "0"), ("list-style", "none")]))
        .nest("& ol ol", pairs(&[("gap", gap!(12)), ("margin-top", gap!(12)), ("padding-inline-start", pad!(12)), ("border-inline-start", "2px solid var(--color-border)")]))
        .nest("& [data-comment]", pairs(&[("display", "grid"), ("grid-template-columns", "32px minmax(0, 1fr)"), ("gap", gap!(10))]))
        .nest("& ol ol [data-comment]", pairs(&[("grid-template-columns", "24px minmax(0, 1fr)")]))
        .nest("& [data-avatar]", pairs(&[("display", "grid"), ("place-items", "center"), ("width", "32px"), ("height", "32px"), ("overflow", "hidden"), ("border-radius", "50%"), ("background", SUBTLE), ("color", MUTED), ("font-size", fs!(12)), ("font-weight", "var(--font-weight-medium, 500)")]))
        .nest("& ol ol [data-avatar]", pairs(&[("width", "24px"), ("height", "24px"), ("font-size", fs!(10))]))
        .nest("& [data-avatar] img", pairs(&[("width", "100%"), ("height", "100%"), ("object-fit", "cover")]))
        .nest("& [data-c-head]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("align-items", "center"), ("gap", gap!(2, 8)), ("min-height", "28px")]))
        .nest("& [data-c-head] b", pairs(&[("font-weight", "var(--font-weight-semibold, 600)")]))
        .nest("& :is([data-c-head] time, [data-c-edited])", pairs(&[("color", MUTED), ("font-size", fs!(12))]))
        .nest("& [data-c-tools]", pairs(&[("display", "flex"), ("gap", gap!(2)), ("margin-inline-start", "auto")]))
        .nest("& [data-c-tools] button, & [data-c-react]", widget::tool())
        .nest("& [data-c-tools] button:hover, & [data-c-react]:hover", pairs(&[("background", SUBTLE), ("color", TEXT)]))
        .nest("& button:focus-visible", forms::focus_ring())
        .nest("& [data-c-text]", pairs(&[("margin", "0"), ("white-space", "pre-wrap"), ("overflow-wrap", "anywhere"), ("line-height", "1.5")]))
        .nest("& [data-c-text] a", pairs(&[("color", PRIMARY), ("text-underline-offset", "3px")]))
        // Markdown: its blocks spaced as in a message, code in the mono face.
        .nest("& [data-c-text][data-md]", pairs(&[("white-space", "normal"), ("display", "flex"), ("flex-direction", "column"), ("gap", gap!(6))]))
        .nest("& [data-c-text][data-md] > :is(p, ul, ol, pre, blockquote)", pairs(&[("margin", "0")]))
        .nest("& [data-c-text][data-md] :is(ul, ol)", pairs(&[("padding-inline-start", "1.4em")]))
        .nest("& [data-c-text] code", pairs(&[("padding", "1px 4px"), ("border-radius", "var(--radius-sm)"), ("background", "color-mix(in srgb, var(--color-text) 8%, transparent)"), ("font-family", "var(--font-mono, ui-monospace, SFMono-Regular, Menlo, monospace)"), ("font-size", "0.9em")]))
        .nest("& [data-c-text] pre", pairs(&[("padding", pad!(8, 10)), ("border-radius", "var(--radius-md)"), ("background", "color-mix(in srgb, var(--color-text) 6%, transparent)"), ("overflow-x", "auto"), ("white-space", "pre")]))
        .nest("& [data-c-text] pre code", pairs(&[("padding", "0"), ("background", "none")]))
        .nest("& [data-c-text] blockquote", pairs(&[("padding-inline-start", "10px"), ("border-inline-start", "3px solid var(--color-border)"), ("color", "var(--color-muted)")]))
        .nest("& [data-mention]", pairs(&[("padding", "0 3px"), ("border-radius", "4px"), ("background", "color-mix(in srgb, var(--color-primary) 12%, transparent)"), ("font-weight", "var(--font-weight-medium, 500)")]))
        .nest("& [data-mention=me]", pairs(&[("background", "color-mix(in srgb, var(--color-warning, #d97706) 22%, transparent)")]))
        .nest("& [data-c-foot]", pairs(&[("position", "relative"), ("display", "flex"), ("flex-wrap", "wrap"), ("align-items", "center"), ("gap", gap!(4)), ("margin-top", gap!(4))]))
        .nest("& [data-c-foot]:empty", pairs(&[("display", "none")]))
        .nest("& [data-c-chip]", pairs(&[("display", "inline-flex"), ("align-items", "center"), ("gap", gap!(4)), ("height", "24px"), ("padding", pad!(0, 8)), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "999px"), ("background", "none"), ("color", TEXT), ("font", "inherit"), ("font-size", fs!(12)), ("cursor", "pointer")]))
        .nest("& [data-c-chip][aria-pressed=true]", pairs(&[("border-color", PRIMARY), ("background", "color-mix(in srgb, var(--color-primary) 10%, transparent)")]))
        .nest("& [data-c-chip]:disabled", pairs(&[("cursor", "default")]))
        .nest("& [data-c-picker]", pairs(&[("position", "absolute"), ("z-index", "5"), ("bottom", "28px"), ("inset-inline-start", "0"), ("display", "flex"), ("gap", gap!(2)), ("padding", pad!(4)), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "999px"), ("background", BG_POPOVER), ("box-shadow", "var(--elevation-popover, 0 6px 18px rgb(0 0 0 / 0.12))")]))
        .nest("& [data-c-picker] button", pairs(&[("width", "32px"), ("height", "32px"), ("padding", "0"), ("border", "0"), ("border-radius", "50%"), ("background", "none"), ("font-size", "17px"), ("cursor", "pointer")]))
        .nest("& [data-c-picker] button:hover", pairs(&[("background", SUBTLE)]))
        .nest("& [data-resolved] > div > :is([data-c-text], ol, [data-c-foot])", pairs(&[("display", "none")]))
        .nest("& [data-resolved] > div > [data-c-open] ~ :is([data-c-text], ol, [data-c-foot])", pairs(&[("display", "block")]))
        .nest("& [data-c-badge]", pairs(&[("display", "inline-flex"), ("align-items", "center"), ("gap", gap!(4)), ("padding", pad!(1, 7)), ("border-radius", "999px"), ("background", "color-mix(in srgb, var(--color-success, #16a34a) 14%, transparent)"), ("color", "var(--color-success, #16a34a)"), ("font-size", fs!(12)), ("font-weight", "var(--font-weight-medium, 500)")]))
        .nest("& [data-c-link]", pairs(&[("padding", "0"), ("border", "0"), ("background", "none"), ("color", MUTED), ("font", "inherit"), ("font-size", fs!(13)), ("text-decoration", "underline"), ("text-underline-offset", "3px"), ("cursor", "pointer")]))
        .nest("& [data-c-empty]", pairs(&[("margin", "0"), ("padding", pad!(16)), ("color", MUTED), ("text-align", "center")]))
        .nest("& [data-c-box]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(8)), ("min-width", "0")]))
        .nest("& [data-c-box] textarea", pairs(&[("box-sizing", "border-box"), ("width", "100%"), ("min-height", "64px"), ("max-height", "320px"), ("padding", pad!(8, 12)), ("border", "var(--border-width, 1px) solid var(--color-border)"), ("border-radius", "var(--radius-control, var(--radius-md))"), ("background", "transparent"), ("color", TEXT), ("font", "inherit"), ("line-height", "1.5"), ("resize", "vertical"), ("field-sizing", "content")]))
        .nest("& [data-c-box] textarea:focus-visible", forms::focus_ring().set("border-color", crate::RING))
        .nest("& [data-c-actions]", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", gap!(8))]))
        .nest("& [data-c-actions] small", pairs(&[("margin-inline-end", "auto"), ("color", MUTED)]))
        .nest("& [data-c-actions] button", pairs(&[("height", ctl!(32)), ("padding", pad!(0, 12)), ("border", "var(--border-width, 1px) solid var(--color-border)"), ("border-radius", "var(--radius-control, var(--radius-md))"), ("background", "none"), ("color", TEXT), ("font", "inherit"), ("font-weight", "var(--font-weight-medium, 500)"), ("cursor", "pointer")]))
        .nest("& [data-c-actions] button[data-primary]", pairs(&[("border-color", "transparent"), ("background", crate::PRIMARY_FILL), ("color", crate::ON_PRIMARY)]))
        .nest("& [data-c-actions] button:disabled", pairs(&[("opacity", "0.5"), ("cursor", "default")]))
});

impl Component for Comments {
    fn name(&self) -> &'static str {
        "ui-comments"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Comments in threads: replies, mentions, reactions, resolving, editing one's own. Each change is written to `bind` and fires an event.", props: PROPS, parts: &[], children: false, open_props: false }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        if cx.prop_string(node, "bind").is_none() {
            return Err(RenderError::new(node, "`ui-comments` needs `bind`"));
        }
        let mut config = widget::config(cx, node, &["user", "people", "placeholder", "emptyText", "disabled"], &["order", "maxLength", "reactions"], &["bind"])?;
        for (name, default) in [("resolvable", false), ("replies", true), ("editable", true), ("markdown", false)] {
            config.insert(name.into(), json!(flag(cx, node, name, default)));
        }
        config.insert("locale".into(), json!(crate::locale(cx, node)));
        widget::kit(cx);
        cx.global_css("ui-mentions", LIST_CSS);
        cx.global_js("ui-mentions", MENTIONS);
        cx.global_js("ui-comments", SCRIPT);
        let mut el = cx.element("section", node);
        el.base_style_ref(&ROOT).attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Comments")))).behavior("ui-comments", Json::Object(config));
        cx.open(&el, node)?;
        cx.close("section");
        Ok(())
    }
}

impl Component for MentionInput {
    fn name(&self) -> &'static str {
        "ui-mention-input"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "A text area that names people with `@`, from a list that opens at the caret. State gets the text, and the names in it.", props: MENTION_PROPS, parts: &["input"], children: false, open_props: false }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let key = cx.prop_string(node, "bind").ok_or_else(|| RenderError::new(node, "`ui-mention-input` needs `bind`"))?;
        let mut config = widget::config(cx, node, &["people"], &["trigger"], &["bind", "bindMentions"])?;
        config.insert("locale".into(), json!(crate::locale(cx, node)));
        widget::kit(cx);
        cx.global_css("ui-mentions", LIST_CSS);
        cx.global_js("ui-mentions", MENTIONS);
        let mut root = cx.element("div", node);
        root.base_style(pairs(&[("position", "relative"), ("display", "flex"), ("width", "100%")])).behavior("ui-mention-input", Json::Object(config));
        cx.open(&root, node)?;
        let mut t = cx.part("textarea", node, "input");
        t.base_style_ref(forms::input_look())
            .base_style(pairs(&[("width", "100%"), ("height", "auto"), ("max-height", "320px"), ("padding", "8px 12px"), ("line-height", "1.5"), ("resize", "vertical"), ("field-sizing", "content")]))
            .attr("id", instance_id(cx, node, "mention"))
            .bind(key);
        let rows = widget::number(cx, node, "rows").unwrap_or(3.0).max(1.0);
        t.attr("rows", rows.to_string()).base_style(StyleBlock::new().set("min-height", format!("calc({rows} * 1.5em + 18px)")));
        if let Some(n) = widget::number(cx, node, "maxLength") {
            t.attr("maxlength", n.to_string());
        }
        for (prop, attr) in [("placeholder", "placeholder"), ("label", "aria-label"), ("name", "name"), ("disabled", "disabled"), ("invalid", "aria-invalid")] {
            if let Some(v) = node.prop(prop) {
                t.attr(attr, v.clone());
            }
        }
        cx.open(&t, node)?;
        cx.close("textarea");
        cx.close("div");
        Ok(())
    }
}
