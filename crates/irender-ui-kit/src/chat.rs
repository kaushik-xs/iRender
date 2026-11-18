//! AI chat: a conversation that keeps to its latest message, message bubbles with
//! reply threads, a typing indicator and a prompt input that sends to an endpoint
//! and streams the reply in. Messages are `{ id, role, content, name, avatar,
//! initials, time, replies }`, the shape most chat APIs use, so an array of them can
//! sit in state and be appended to as the conversation goes on.

use crate::{
    PRIMARY_FILL, BG_CARD, MUTED, ON_PRIMARY, ON_SECONDARY, PRIMARY, RING, SECONDARY, SUBTLE, SURFACE, TEXT, bin, branch, cond, each_item,
    expr_val, field_or, items, lit, num, path, server_array, synthetic, val_expr,
};
use irender_core::{
    ComponentInfo, Node, Part, PropDef, PropKind, StyleBlock, Val,
    expr::{BinOp, Expr, UnOp},
};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const CONVERSATION_SCRIPT: &str = include_str!("conversation.js");
const INPUT_SCRIPT: &str = include_str!("chat_input.js");
const HTML_SCRIPT: &str = include_str!("message_html.js");

/// The three dots' bounce, added to the page once by the first typing indicator.
const BOUNCE: &str = "@keyframes ui-typing {\n  0%, 60%, 100% { transform: none; opacity: 0.4; }\n  30% { transform: translateY(-3px); opacity: 1; }\n}";

/// The names of the current message and reply while rendering a list of them.
const MESSAGE: &str = "ui_message";
const MESSAGE_INDEX: &str = "ui_message_index";
const REPLY: &str = "ui_reply";
const REPLY_INDEX: &str = "ui_reply_index";

/// The message fields a list item passes on to its `ui-message`.
/// `role` comes first.
const FIELDS: &[&str] = &["role", "content", "name", "avatar", "initials", "time", "html", "ui"];

const ROLES: &[&str] = &["assistant", "user", "system"];

/// The prop as a value, null when it's missing.
fn prop_or_null(node: &Node, name: &str) -> Val {
    node.prop(name).cloned().unwrap_or_else(|| lit(Json::Null))
}

fn or(a: &Val, b: &Val) -> Val {
    expr_val(bin(BinOp::Or, val_expr(a), val_expr(b)))
}

/// The length of the array prop `prop` as an expression, for an array from state
/// (which must then be a path to it, like `{{ messages }}`) or one known now.
fn length(node: &Node, prop: &str, now: usize, live: Option<&Val>) -> Result<Expr, RenderError> {
    match live {
        None => Ok(num(now)),
        Some(Val::Tpl(t)) => match t.parts.as_slice() {
            [Part::Expr(Expr::Path(p))] => Ok(Expr::Path(p.iter().cloned().chain(["length".to_string()]).collect())),
            _ => Err(RenderError::new(node, format!("`{prop}` that uses state must be a path to it, like `{{{{ {prop} }}}}`"))),
        },
        Some(Val::Lit(_)) => Ok(num(now)),
    }
}

/// A `ui-message` for the list item `var`, carrying `parts` from the list's node so
/// one `parts.bubble` styles every message.
fn item_message(node: &Node, var: &str, replies: bool) -> Node {
    let mut props: Vec<(&str, Val)> = FIELDS.iter().map(|f| (*f, expr_val(path(&[var, f])))).collect();
    props[0].1 = expr_val(field_or(var, "role", Expr::Lit(json!("assistant"))));
    if let Some(r) = node.prop("render") {
        props.push(("render", r.clone()));
    }
    if replies {
        props.push(("replies", expr_val(path(&[var, "replies"]))));
    }
    let mut m = synthetic(node, "ui-message", props, Vec::new(), Vec::new());
    m.parts = node.parts.clone();
    m
}

// ---------------------------------------------------------------- message

pub struct Message;

const MESSAGE_PROPS: &[PropDef] = &[
    PropDef::new("role", PropKind::Enum(ROLES), "Who it's from: `assistant` (default, on the left), `user` (on the right) or `system` (centred)"),
    PropDef::new("content", PropKind::Any, "The text; line breaks are kept"),
    PropDef::new("name", PropKind::Any, "Sender's name, shown above the bubble"),
    PropDef::new("time", PropKind::Any, "When it was sent, as text, shown after the name"),
    PropDef::new("avatar", PropKind::Any, "Avatar image URL"),
    PropDef::new("initials", PropKind::Any, "Shown in the avatar when there's no image"),
    PropDef::new(
        "html",
        PropKind::Any,
        "UI after the text, rendered on a server: by `render_ui` (the CLI renders each message's `ui` into it), or \
         `irender render --embed`. Shown only in trusted documents",
    ),
    PropDef::new("ui", PropKind::Any, "UI after the text as an iRender document or node, for `render` to render when there's no `html`"),
    PropDef::new(
        "render",
        PropKind::Any,
        "How a `ui` without `html` is rendered: `{ browser, url }`. `browser` is irender-wasm's `.wasm` file, to render it \
         in the page; `url` is your server's render endpoint, which gets `{ ui }` and answers with the markup, used when \
         there's no `browser` or it can't render it",
    ),
    PropDef::new("replies", PropKind::Any, "A thread: an array of messages shown under it, folded away behind `N replies`. May use state"),
    PropDef::new("open", PropKind::Bool, "Show the thread unfolded"),
];

static MESSAGE_ROW: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[("display", "flex"), ("align-items", "flex-start"), ("gap", sp!(12)), ("min-width", "0")])
        .nest("&[data-role=user]", StyleBlock::from_pairs(&[("flex-direction", "row-reverse")]))
        .nest("&[data-role=system]", StyleBlock::from_pairs(&[("justify-content", "center")]))
        // A user's bubble, in the primary colour.
        .nest("&[data-role=user] [data-bubble]", StyleBlock::from_pairs(&[("background", PRIMARY_FILL), ("color", ON_PRIMARY)]))
        .nest("&[data-role=user] > [data-body]", StyleBlock::from_pairs(&[("align-items", "flex-end")]))
        // A system note: small, muted and centred, without a bubble.
        .nest("&[data-role=system] > [data-body]", StyleBlock::from_pairs(&[("align-items", "center"), ("max-width", "100%")]))
        .nest(
            "&[data-role=system] [data-bubble]",
            StyleBlock::from_pairs(&[("padding", "0"), ("background", "transparent"), ("color", MUTED), ("font-size", fs!(12)), ("text-align", "center")]),
        )
});
static MESSAGE_AVATAR: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("position", "relative"),
        ("display", "inline-flex"),
        ("align-items", "center"),
        ("justify-content", "center"),
        ("flex-shrink", "0"),
        ("width", "32px"),
        ("height", "32px"),
        ("overflow", "hidden"),
        ("border-radius", "999px"),
        ("background", SUBTLE),
        ("color", MUTED),
        ("font-size", fs!(12)),
        ("font-weight", "500"),
    ])
    .nest("& > img", StyleBlock::from_pairs(&[("position", "absolute"), ("inset", "0"), ("width", "100%"), ("height", "100%"), ("object-fit", "cover")]))
});
static MESSAGE_BODY: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("display", "flex"),
        ("flex-direction", "column"),
        ("align-items", "flex-start"),
        ("gap", sp!(4)),
        ("min-width", "0"),
        ("max-width", "min(80%, 640px)"),
    ])
    // UI in a message gets the width it needs.
    .nest("&:has(> [data-ui]:not(:empty))", StyleBlock::from_pairs(&[("width", "100%"), ("max-width", "min(100%, 720px)")]))
});
static MESSAGE_UI: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[("width", "100%"), ("min-width", "0"), ("font-size", fs!(14))])
        .nest("&:empty", StyleBlock::from_pairs(&[("display", "none")]))
        .nest("& > [data-ui-note]", StyleBlock::from_pairs(&[("margin", "0"), ("font-size", fs!(12)), ("color", MUTED)]))
});
static MESSAGE_META: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[("display", "flex"), ("align-items", "baseline"), ("gap", sp!(6)), ("font-size", fs!(12)), ("color", MUTED)])
        .nest("& > :first-child", StyleBlock::from_pairs(&[("font-weight", "500"), ("color", TEXT)]))
        .nest("& > :empty", StyleBlock::from_pairs(&[("display", "none")]))
});
static BUBBLE: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("padding", sp!(8, 12)),
        ("border-radius", "var(--radius-lg)"),
        ("background", SECONDARY),
        ("color", ON_SECONDARY),
        ("font-size", fs!(14)),
        ("line-height", "1.5"),
        ("white-space", "pre-wrap"),
        ("overflow-wrap", "anywhere"),
    ])
    // A reply that hasn't started yet has no bubble.
    .nest("&:empty", StyleBlock::from_pairs(&[("display", "none")]))
    .nest("& > *", StyleBlock::from_pairs(&[("white-space", "normal")]))
});
static THREAD: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[("width", "100%"), ("font-size", fs!(12))])
        .nest(
            "& > summary",
            StyleBlock::from_pairs(&[
                ("width", "fit-content"),
                ("padding", sp!(2, 4)),
                ("border-radius", "var(--radius-sm)"),
                ("color", PRIMARY),
                ("font-weight", "500"),
                ("cursor", "pointer"),
                ("list-style", "none"),
            ]),
        )
        .nest("& > summary::-webkit-details-marker", StyleBlock::from_pairs(&[("display", "none")]))
        .nest("& > summary::before", StyleBlock::from_pairs(&[("content", "\"▸ \"")]))
        .nest("&[open] > summary::before", StyleBlock::from_pairs(&[("content", "\"▾ \"")]))
        .nest("& > summary:hover", StyleBlock::from_pairs(&[("text-decoration", "underline")]))
        .nest(
            "& > summary:focus-visible",
            StyleBlock::from_pairs(&[("outline", format!("2px solid {RING}").as_str()), ("outline-offset", "2px")]),
        )
});
static REPLIES: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("display", "flex"),
        ("flex-direction", "column"),
        ("gap", sp!(12)),
        ("margin-top", sp!(8)),
        ("padding-left", sp!(12)),
        ("border-left", "2px solid var(--color-border)"),
    ])
});

impl Component for Message {
    fn name(&self) -> &'static str {
        "ui-message"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "One chat message: a bubble with the sender's avatar, name and time, and an optional thread of replies. \
                  Children go in the bubble after the text.",
            props: MESSAGE_PROPS,
            parts: &["avatar", "body", "meta", "bubble", "ui", "thread", "replies"],
            children: true,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let (avatar, initials) = (prop_or_null(node, "avatar"), prop_or_null(node, "initials"));
        let (name, time) = (prop_or_null(node, "name"), prop_or_null(node, "time"));
        let mut row = cx.element("div", node);
        row.base_style_ref(&MESSAGE_ROW).attr("data-role", node.prop("role").cloned().unwrap_or_else(|| lit("assistant")));
        cx.open(&row, node)?;

        branch(cx, &or(&avatar, &initials), |cx| {
            let mut a = cx.part("span", node, "avatar");
            a.base_style_ref(&MESSAGE_AVATAR).attr("aria-hidden", "true").text(initials.clone());
            cx.open(&a, node)?;
            branch(cx, &avatar, |cx| {
                let mut img = Element::new("img");
                img.attr("src", avatar.clone()).attr("alt", "");
                cx.open(&img, node)
            })?;
            cx.close("span");
            Ok(())
        })?;

        let mut body = cx.part("div", node, "body");
        body.base_style_ref(&MESSAGE_BODY).attr("data-body", lit(""));
        cx.open(&body, node)?;
        branch(cx, &or(&name, &time), |cx| {
            let mut meta = cx.part("div", node, "meta");
            meta.base_style_ref(&MESSAGE_META);
            cx.open(&meta, node)?;
            for v in [&name, &time] {
                cx.open(Element::new("span").text(v), node)?;
                cx.close("span");
            }
            cx.close("div");
            Ok(())
        })?;
        let mut bubble = cx.part("div", node, "bubble");
        bubble.base_style_ref(&BUBBLE).attr("data-bubble", lit(""));
        if let Some(c) = node.prop("content") {
            bubble.text(c);
        }
        cx.open(&bubble, node)?;
        cx.children(node)?;
        cx.close("div");
        // Markup can do anything a script can, so a document that may not add scripts
        // may not show it either.
        if (node.prop("html").is_some() || node.prop("ui").is_some()) && !cx.policy().blocked_tags.contains("script") {
            cx.global_js("ui-message-html", HTML_SCRIPT);
            let render = cx.prop(node, "render").unwrap_or(Json::Null);
            let config = json!({ "browser": render.get("browser"), "url": render.get("url") });
            let mut ui = cx.part("div", node, "ui");
            ui.base_style_ref(&MESSAGE_UI).attr("data-ui", lit("")).behavior("ui-message-html", config);
            if let Some(html) = node.prop("html") {
                ui.attr("data-html", html.clone());
            }
            if let Some(json) = node.prop("ui") {
                ui.attr("data-ui-json", json.clone());
            }
            cx.open(&ui, node)?;
            cx.close("div");
        }
        if node.prop("replies").is_some() {
            thread(cx, node)?;
        }
        cx.close("div");
        cx.close("div");
        Ok(())
    }
}

/// The message's replies, folded away behind `N replies` while there are any.
fn thread(cx: &mut RenderCx<'_>, node: &Node) -> Result<(), RenderError> {
    let replies = items(cx, node, "replies")?;
    let count = length(node, "replies", replies.0.len(), replies.1)?;
    let summary = expr_val(cond(
        bin(BinOp::Eq, count.clone(), num(1)),
        Expr::Lit(json!("1 reply")),
        bin(BinOp::Add, count.clone(), Expr::Lit(json!(" replies"))),
    ));
    let any = expr_val(bin(BinOp::Gt, count, num(0)));
    let open = cx.prop(node, "open").is_some_and(|o| irender_core::expr::truthy(&o));
    branch(cx, &any, |cx| {
        let mut details = cx.part("details", node, "thread");
        details.base_style_ref(&THREAD);
        if open {
            details.attr("open", lit(true));
        }
        cx.open(&details, node)?;
        cx.open(Element::new("summary").text(&summary), node)?;
        cx.close("summary");
        let mut list = cx.part("div", node, "replies");
        list.base_style_ref(&REPLIES);
        let key = expr_val(field_or(REPLY, "id", path(&[REPLY_INDEX])));
        let reply = item_message(node, REPLY, false);
        each_item(cx, node, list, replies, REPLY, REPLY_INDEX, &key, |cx| cx.render(&reply))?;
        cx.close("div");
        cx.close("details");
        Ok(())
    })
}

// ---------------------------------------------------------------- typing indicator

pub struct TypingIndicator;

const TYPING_PROPS: &[PropDef] =
    &[PropDef::new("label", PropKind::Any, "What's happening, for screen readers; default `Typing`")];

static TYPING: LazyLock<StyleBlock> = LazyLock::new(|| {
    let dot = |delay: &str| StyleBlock::from_pairs(&[("animation-delay", delay)]);
    StyleBlock::from_pairs(&[("display", "inline-flex"), ("align-items", "center"), ("gap", "4px"), ("height", "1.5em")])
        .nest(
            "& > span",
            StyleBlock::from_pairs(&[
                ("width", "6px"),
                ("height", "6px"),
                ("border-radius", "999px"),
                ("background", "currentColor"),
                ("opacity", "0.4"),
                ("animation", "ui-typing 1.2s ease-in-out infinite"),
            ]),
        )
        .nest("& > span:nth-child(2)", dot(".15s"))
        .nest("& > span:nth-child(3)", dot(".3s"))
        .nest("@media (prefers-reduced-motion: reduce)", StyleBlock::from_pairs(&[]).nest("& > span", StyleBlock::from_pairs(&[("animation-duration", "3s")])))
});

impl Component for TypingIndicator {
    fn name(&self) -> &'static str {
        "ui-typing-indicator"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Three bouncing dots that say a reply is on its way. Takes the text colour.",
            props: TYPING_PROPS,
            parts: &[],
            children: false,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        cx.global_css("ui-typing", BOUNCE);
        let mut el = cx.element("span", node);
        el.base_style_ref(&TYPING).attr("role", "status");
        el.attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit("Typing")));
        cx.open(&el, node)?;
        for _ in 0..3 {
            cx.open(&Element::new("span"), node)?;
            cx.close("span");
        }
        cx.close("span");
        Ok(())
    }
}

// ---------------------------------------------------------------- conversation

pub struct Conversation;

const CONVERSATION_PROPS: &[PropDef] = &[
    PropDef::new("messages", PropKind::Any, "Array of messages, after any children. From state, like `{{ messages }}`, it redraws as it changes"),
    PropDef::new("typing", PropKind::Any, "Show a typing indicator after the last message, e.g. `{{ waiting }}`"),
    PropDef::new("render", PropKind::Any, "How messages' `ui` without `html` is rendered: `{ browser, url }`, as on `ui-message`"),
    PropDef::new("assistant", PropKind::Any, "The typing indicator's sender: `{ name, avatar, initials }`"),
    PropDef::new("empty", PropKind::Any, "Text shown while `messages` is empty"),
    PropDef::new("label", PropKind::Any, "Names the conversation for screen readers (default `Conversation`)"),
    PropDef::new("height", PropKind::Any, "Number (px) or CSS length"),
    PropDef::new("maxHeight", PropKind::Any, "Number (px) or CSS length: grows with the messages up to this"),
];

static CONVERSATION: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[("position", "relative"), ("display", "flex"), ("flex-direction", "column"), ("min-height", "0"), ("color", TEXT)])
});
static LOG: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("flex", "1 1 auto"),
        ("min-height", "0"),
        ("display", "flex"),
        ("flex-direction", "column"),
        ("gap", sp!(16)),
        ("padding", sp!(16)),
        ("overflow-y", "auto"),
        ("overscroll-behavior", "contain"),
        ("scrollbar-width", "thin"),
        ("scrollbar-color", "color-mix(in srgb, var(--color-text) 25%, transparent) transparent"),
    ])
    .nest(":focus-visible", StyleBlock::from_pairs(&[("outline", "none"), ("box-shadow", "inset 0 0 0 2px var(--color-ring, var(--color-muted))")]))
});
static MESSAGES: LazyLock<StyleBlock> = LazyLock::new(|| StyleBlock::from_pairs(&[("display", "contents")]));
static EMPTY: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[("margin", "auto"), ("padding", sp!(24)), ("color", MUTED), ("font-size", fs!(14)), ("text-align", "center")])
});
static LATEST: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("position", "absolute"),
        ("left", "50%"),
        ("bottom", sp!(12)),
        ("transform", "translateX(-50%)"),
        ("display", "inline-flex"),
        ("align-items", "center"),
        ("justify-content", "center"),
        ("width", "32px"),
        ("height", "32px"),
        ("padding", "0"),
        ("border", "1px solid var(--color-border)"),
        ("border-radius", "999px"),
        ("background", SURFACE),
        ("color", TEXT),
        ("box-shadow", "0 4px 12px rgb(0 0 0 / 0.12)"),
        ("cursor", "pointer"),
    ])
    .nest("&[hidden]", StyleBlock::from_pairs(&[("display", "none")]))
    .nest(":focus-visible", StyleBlock::from_pairs(&[("outline", "2px solid var(--color-ring, var(--color-muted))"), ("outline-offset", "2px")]))
});

impl Component for Conversation {
    fn name(&self) -> &'static str {
        "ui-conversation"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A scrolling chat log that keeps to its latest message as messages arrive, unless you've scrolled up to read; \
                  then a button jumps back down. Children are `ui-message`s, followed by `messages`. Its `parts.bubble`, \
                  `parts.avatar` and so on style the messages drawn from `messages`.",
            props: CONVERSATION_PROPS,
            parts: &["log", "empty", "latest", "avatar", "body", "meta", "bubble", "ui", "thread", "replies"],
            children: true,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut sizing = StyleBlock::new();
        for (prop, property) in [("height", "height"), ("maxHeight", "max-height")] {
            if let Some(v) = node.prop(prop)
                && let Some(v) = cx.style_value(node, property, v)?
            {
                sizing.push(property, v);
            }
        }
        cx.global_js("ui-conversation", CONVERSATION_SCRIPT);
        let mut el = cx.element("div", node);
        el.base_style_ref(&CONVERSATION).base_style(sizing).behavior("ui-conversation", json!({}));
        cx.open(&el, node)?;

        let mut log = cx.part("div", node, "log");
        log.base_style_ref(&LOG)
            .attr("role", "log")
            .attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit("Conversation")))
            .attr("tabindex", "0")
            .attr("data-log", lit(""));
        cx.open(&log, node)?;
        cx.children(node)?;
        if node.prop("messages").is_some() {
            let messages = items(cx, node, "messages")?;
            let count = length(node, "messages", messages.0.len(), messages.1)?;
            let key = expr_val(field_or(MESSAGE, "id", path(&[MESSAGE_INDEX])));
            let message = item_message(node, MESSAGE, true);
            let mut list = Element::new("div");
            list.base_style_ref(&MESSAGES);
            each_item(cx, node, list, messages, MESSAGE, MESSAGE_INDEX, &key, |cx| cx.render(&message))?;
            cx.close("div");
            if let Some(text) = node.prop("empty") {
                branch(cx, &expr_val(Expr::Unary(UnOp::Not, Box::new(count))), |cx| {
                    let mut empty = cx.part("div", node, "empty");
                    empty.base_style_ref(&EMPTY).text(text);
                    cx.open(&empty, node)?;
                    cx.close("div");
                    Ok(())
                })?;
            }
        }
        if let Some(typing) = node.prop("typing") {
            let who = |f: &str| match cx.prop(node, "assistant") {
                Some(a) => lit(a.get(f).cloned().unwrap_or(Json::Null)),
                None => lit(Json::Null),
            };
            let props: Vec<(&str, Val)> =
                ["name", "avatar", "initials"].into_iter().map(|f| (f, who(f))).chain([("role", lit("assistant"))]).collect();
            let mut shell = synthetic(node, "ui-message", props, Vec::new(), Vec::new());
            shell.parts = node.parts.clone();
            shell.children = vec![synthetic(node, "ui-typing-indicator", Vec::new(), Vec::new(), Vec::new())];
            branch(cx, typing, |cx| cx.render(&shell))?;
        }
        cx.close("div");

        let mut latest = cx.part("button", node, "latest");
        latest
            .base_style_ref(&LATEST)
            .attr("type", "button")
            .attr("aria-label", "Scroll to the latest message")
            .attr("hidden", lit(true))
            .attr("data-latest", lit(""))
            .text(lit("↓"));
        cx.open(&latest, node)?;
        cx.close("button");
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- chat input

pub struct ChatInput;

const CHAT_INPUT_PROPS: &[PropDef] = &[
    PropDef::new("messages", PropKind::StateKey, "State key of the messages array: sending appends the user's message, and the reply")
        .required(),
    PropDef::new(
        "endpoint",
        PropKind::Any,
        "URL to POST `{ messages }` to. The reply may be JSON (a message, or `{ content }`), plain text, or server-sent \
         events, which stream into the reply as they arrive",
    ),
    PropDef::new("bind", PropKind::StateKey, "State key holding the text being written"),
    PropDef::new("waiting", PropKind::StateKey, "State key set true from sending until the reply starts: give it to the conversation's `typing`"),
    PropDef::new(
        "threads",
        PropKind::StateKey,
        "State key of an array of threads, `{ id, title, messages }`: the messages live in them, and `messages` shows the current \
         one's. Needs `thread`",
    ),
    PropDef::new("thread", PropKind::StateKey, "State key of the current thread's id; null for a new one, started by the first message"),
    PropDef::new("placeholder", PropKind::Any, "Default `Send a message…`"),
    PropDef::new("label", PropKind::Any, "Names the text box for screen readers (default `Message`)"),
    PropDef::new("suggestions", PropKind::Any, "Prompts shown while the conversation is empty; clicking one sends it"),
    PropDef::new("disabled", PropKind::Any, "Boolean or expression"),
];

static CHAT_INPUT: LazyLock<StyleBlock> = LazyLock::new(|| StyleBlock::from_pairs(&[("display", "grid"), ("gap", sp!(8)), ("margin", "0")]));
static SUGGESTIONS: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("gap", sp!(8))])
        .nest("&[hidden]", StyleBlock::from_pairs(&[("display", "none")]))
        .nest(
            "& > button",
            StyleBlock::from_pairs(&[
                ("padding", sp!(6, 12)),
                ("border", "1px solid var(--color-border)"),
                ("border-radius", "999px"),
                ("background", SURFACE),
                ("color", TEXT),
                ("font", "inherit"),
                ("font-size", fs!(13)),
                ("cursor", "pointer"),
                ("transition", "background-color .15s"),
            ]),
        )
        .nest("& > button:hover", StyleBlock::from_pairs(&[("background", SUBTLE)]))
        .nest("& > button:focus-visible", StyleBlock::from_pairs(&[("outline", "2px solid var(--color-ring, var(--color-muted))"), ("outline-offset", "2px")]))
});
static BOX: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("display", "grid"),
        ("gap", sp!(8)),
        ("padding", sp!(8)),
        ("border", "1px solid var(--color-border)"),
        ("border-radius", "var(--radius-lg)"),
        ("--ui-bg", BG_CARD),
        ("background", BG_CARD),
        ("box-shadow", "0 1px 2px rgb(0 0 0 / 0.05)"),
        ("transition", "box-shadow .15s, border-color .15s"),
    ])
    .nest(
        ":focus-within",
        StyleBlock::from_pairs(&[
            ("border-color", RING),
            ("box-shadow", "0 0 0 3px color-mix(in srgb, var(--color-ring, var(--color-muted)) 50%, transparent)"),
        ]),
    )
});
static TEXTBOX: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("box-sizing", "border-box"),
        ("width", "100%"),
        ("min-height", sp!(40)),
        ("max-height", "200px"),
        ("padding", sp!(4, 6)),
        ("border", "0"),
        ("outline", "none"),
        ("resize", "none"),
        ("field-sizing", "content"),
        ("background", "transparent"),
        ("color", TEXT),
        ("font", "inherit"),
        ("font-size", fs!(14)),
        ("line-height", "1.5"),
    ])
    .nest("::placeholder", StyleBlock::from_pairs(&[("color", MUTED)]))
    .nest(":disabled", StyleBlock::from_pairs(&[("cursor", "not-allowed"), ("opacity", "0.5")]))
});
static TOOLBAR: LazyLock<StyleBlock> =
    LazyLock::new(|| StyleBlock::from_pairs(&[("display", "flex"), ("align-items", "center"), ("gap", sp!(4))]));
static SEND: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("display", "inline-flex"),
        ("align-items", "center"),
        ("justify-content", "center"),
        ("flex-shrink", "0"),
        ("width", "32px"),
        ("height", "32px"),
        ("margin-left", "auto"),
        ("padding", "0"),
        ("border", "0"),
        ("border-radius", "999px"),
        ("background", PRIMARY_FILL),
        ("color", ON_PRIMARY),
        ("font-size", fs!(16)),
        ("cursor", "pointer"),
    ])
    .nest(":disabled", StyleBlock::from_pairs(&[("opacity", "0.4"), ("cursor", "default")]))
    .nest(":focus-visible", StyleBlock::from_pairs(&[("outline", "2px solid var(--color-ring, var(--color-muted))"), ("outline-offset", "2px")]))
    .nest("& > span", StyleBlock::from_pairs(&[("display", "inline-flex")]))
    // One glyph shows at a time: send, or stop while a reply streams in.
    .nest("&:not([data-streaming]) > [data-stop], &[data-streaming] > [data-send]", StyleBlock::from_pairs(&[("display", "none")]))
});

impl Component for ChatInput {
    fn name(&self) -> &'static str {
        "ui-chat-input"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "The box a chat message is written in. Enter sends and Shift+Enter starts a new line. Sending appends \
                  `{ id, role: user, content }` to `messages`, runs the document's `on.send` actions, and with `endpoint` \
                  asks it for the reply, streaming it into a new assistant message; while it streams, the send button stops \
                  it. With `threads` and `thread` it keeps a list of conversations. Children go in the toolbar, left of the \
                  send button.",
            props: CHAT_INPUT_PROPS,
            parts: &["suggestions", "box", "input", "toolbar", "send"],
            children: true,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let Some(messages) = cx.prop_string(node, "messages") else {
            return Err(RenderError::new(node, "`ui-chat-input` needs `messages`"));
        };
        let suggestions = server_array(cx, node, "suggestions")?;
        let (threads, thread) = (cx.prop_string(node, "threads"), cx.prop_string(node, "thread"));
        if threads.is_some() != thread.is_some() {
            return Err(RenderError::new(node, "`threads` and `thread` go together"));
        }
        let config = json!({
            "messages": messages,
            "endpoint": cx.prop_string(node, "endpoint"),
            "bind": cx.prop_string(node, "bind"),
            "waiting": cx.prop_string(node, "waiting"),
            "threads": threads,
            "thread": thread,
        });
        cx.global_js("ui-chat-input", INPUT_SCRIPT);
        let mut form = cx.element("form", node);
        form.base_style_ref(&CHAT_INPUT).behavior("ui-chat-input", config);
        cx.open(&form, node)?;

        if !suggestions.is_empty() {
            let mut row = cx.part("div", node, "suggestions");
            row.base_style_ref(&SUGGESTIONS).attr("data-suggestions", lit(""));
            cx.open(&row, node)?;
            for s in &suggestions {
                let mut b = Element::new("button");
                b.attr("type", "button").attr("data-suggestion", lit("")).text(lit(irender_core::expr::stringify(s)));
                cx.open(&b, node)?;
                cx.close("button");
            }
            cx.close("div");
        }

        let mut bx = cx.part("div", node, "box");
        bx.base_style_ref(&BOX);
        cx.open(&bx, node)?;
        let mut input = cx.part("textarea", node, "input");
        input
            .base_style_ref(&TEXTBOX)
            .attr("rows", lit(1))
            .attr("placeholder", node.prop("placeholder").cloned().unwrap_or_else(|| lit("Send a message…")))
            .attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit("Message")))
            .attr("data-input", lit(""));
        if let Some(d) = node.prop("disabled") {
            input.attr("disabled", d.clone());
        }
        if let Some(key) = cx.prop_string(node, "bind") {
            // Initial text only: the runtime keeps the value in sync from then on.
            input.text(lit(cx.lookup(std::slice::from_ref(&key))));
            input.bind(key);
        }
        cx.open(&input, node)?;
        cx.close("textarea");

        let mut toolbar = cx.part("div", node, "toolbar");
        toolbar.base_style_ref(&TOOLBAR);
        cx.open(&toolbar, node)?;
        cx.children(node)?;
        let mut send = cx.part("button", node, "send");
        send.base_style_ref(&SEND).attr("type", "submit").attr("aria-label", "Send").attr("data-send-button", lit(""));
        if let Some(d) = node.prop("disabled") {
            send.attr("disabled", d.clone());
        }
        cx.open(&send, node)?;
        // The send arrow from the icon kit when it's registered.
        let arrow = crate::icon_name(cx, &lit("arrow-up"));
        let mut glyph = Element::new("span");
        glyph.attr("data-send", lit("")).attr("aria-hidden", "true");
        if arrow.is_none() {
            glyph.text(lit("↑"));
        }
        cx.open(&glyph, node)?;
        if let Some(n) = arrow {
            crate::icon_kit(cx, node, &n, 16)?;
        }
        cx.close("span");
        let mut stop = Element::new("span");
        stop.attr("data-stop", lit("")).attr("aria-hidden", "true").text(lit("■"));
        cx.open(&stop, node)?;
        cx.close("span");
        cx.close("button");
        cx.close("div");
        cx.close("div");
        cx.close("form");
        Ok(())
    }
}

// ---------------------------------------------------------------- message UI

/// A message's `ui` as a document: a node becomes a document's root, and a document
/// gets `version: 1` if it has none.
pub fn ui_document(ui: &Json) -> Json {
    match ui {
        Json::Object(o) if o.contains_key("root") => {
            let mut doc = o.clone();
            doc.entry("version").or_insert(json!(1));
            Json::Object(doc)
        }
        node => json!({ "version": 1, "root": node }),
    }
}

/// Renders a message's `ui` (a document or a node) as markup for its `html`: the one
/// function behind the CLI, a server's render endpoint and the browser renderer
/// (irender-wasm), so all three give the same result. `prepare` adds what the kits
/// need before resolving, e.g. their themes:
/// `|doc| { irender_ui_kit::apply_theme(doc); irender_chart_kit::apply_theme(doc); }`.
/// Use an engine with the untrusted policy for UI a model wrote.
pub fn render_ui(engine: &irender_html::Engine, ui: &Json, prepare: impl FnOnce(&mut irender_core::Document)) -> Result<String, String> {
    let mut doc = irender_core::parse(&ui_document(ui).to_string(), irender_core::Format::Json).map_err(|d| d.to_string())?;
    prepare(&mut doc);
    let ir = engine.resolve(&doc).map_err(|d| d.to_string())?;
    Ok(engine.render_embedded(&ir, &Json::Object(Default::default())).map_err(|e| e.to_string())?.embedded())
}

/// Renders the `ui` of every message in `doc`'s state into its `html`, which
/// `ui-message` shows. A message is an object with a `role` and a `ui`: an iRender
/// document or node, so a reply can be a form, a chart or a gallery rather than text.
/// Messages may sit anywhere in state, e.g. in threads. `render` turns a `ui` into
/// markup, usually with [`render_ui`]. Messages that already have `html`
/// are left alone. Errors name the message's place in state.
pub fn render_message_ui(
    doc: &mut irender_core::Document,
    mut render: impl FnMut(&Json) -> Result<String, String>,
) -> Result<(), String> {
    fn walk(v: &mut Json, at: &str, render: &mut dyn FnMut(&Json) -> Result<String, String>) -> Result<(), String> {
        match v {
            Json::Array(items) => {
                for (i, item) in items.iter_mut().enumerate() {
                    walk(item, &format!("{at}[{i}]"), render)?;
                }
            }
            Json::Object(o) => {
                if o.contains_key("role") && o.get("ui").is_some_and(Json::is_object) && !o.contains_key("html") {
                    let html = render(&o["ui"]).map_err(|e| format!("{at}.ui: {e}"))?;
                    o.insert("html".into(), json!(html));
                }
                for (k, v) in o.iter_mut() {
                    if k != "ui" {
                        walk(v, &format!("{at}.{k}"), render)?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
    for (key, value) in doc.state.iter_mut() {
        walk(value, &format!("state.{key}"), &mut render)?;
    }
    Ok(())
}
