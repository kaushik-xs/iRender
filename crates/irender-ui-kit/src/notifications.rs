//! `ui-notifications`: a bell with an unread count that opens a panel of notifications
//! (or the panel on its own, `inline`), with Unread and All, mark read or unread,
//! archive, and Mark all as read. The server draws the bell and the frame;
//! `notifications.js` draws the list from `items` as they change.

use crate::data_table::dynamic;
use crate::overlay::popover_base;
use crate::status::tone_rules;
use crate::{ACCENT, MUTED, PRIMARY, PRIMARY_FILL, TEXT, forms, id_with, instance_id, lit};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("notifications.js");

pub struct Notifications;

const PROPS: &[PropDef] = &[
    PropDef::new(
        "items",
        PropKind::Any,
        "The notifications, newest first: `{ id, title, body, who, when, read, archived, tone }`, with `who` a name or `{ name, initials, avatar }`",
    )
    .required(),
    PropDef::new("bindItems", PropKind::StateKey, "State key the items are in: read and archived are written back to it"),
    PropDef::new("bindItem", PropKind::StateKey, "State key that gets the notification clicked"),
    PropDef::new("inline", PropKind::Bool, "The panel in the page, with no bell (for a notifications page)"),
    PropDef::new("tab", PropKind::Any, "The tab to start in: `unread` (default) or `all`"),
    PropDef::new("title", PropKind::Any, "The panel's heading (default `Notifications`)"),
    PropDef::new("label", PropKind::Any, "Names the bell for screen readers (default `Notifications`)"),
    PropDef::new("emptyText", PropKind::Any, "With nothing at all (default `No notifications.`)"),
    PropDef::new("caughtUp", PropKind::Any, "In Unread with nothing unread (default `You're all caught up.`)"),
    PropDef::new("allHref", PropKind::Any, "A link under the list to every notification, e.g. `/notifications`"),
    PropDef::new("closeOnOpen", PropKind::Bool, "Close the panel when a notification is clicked (default true)"),
];

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

static ROOT: LazyLock<StyleBlock> = LazyLock::new(|| {
    let b = pairs(&[("position", "relative"), ("display", "inline-flex"), ("color", TEXT), ("font-size", fs!(14))])
        .nest(
            "& [data-nt-bell]",
            pairs(&[
                ("position", "relative"),
                ("display", "inline-grid"),
                ("place-items", "center"),
                ("width", "36px"),
                ("height", "36px"),
                ("padding", "0"),
                ("border", "1px solid var(--color-border)"),
                ("border-radius", "var(--radius-md)"),
                ("background", "transparent"),
                ("color", TEXT),
                ("cursor", "pointer"),
            ]),
        )
        .nest("& [data-nt-bell]:hover", pairs(&[("background", ACCENT)]))
        .nest("& [data-nt-bell]:focus-visible", forms::focus_ring())
        .nest(
            "& [data-nt-count]",
            pairs(&[
                ("position", "absolute"),
                ("top", "-6px"),
                ("inset-inline-end", "-6px"),
                ("min-width", "18px"),
                ("height", "18px"),
                ("padding", sp!(0, 5)),
                ("box-sizing", "border-box"),
                ("border", "2px solid var(--color-surface, #fff)"),
                ("border-radius", "999px"),
                ("background", "var(--color-destructive, #dc2626)"),
                ("color", "#fff"),
                ("font-size", fs!(10)),
                ("font-weight", "700"),
                ("line-height", "14px"),
                ("text-align", "center"),
            ]),
        )
        .nest("& [data-nt-count][hidden]", pairs(&[("display", "none")]))
        .nest("& [data-nt-panel][popover]", popover_base().set("width", "min(380px, calc(100vw - 24px))").set("padding", "0").set("margin", concat!(sp!(8), " 0")).set("position-area", "bottom span-left").set("position-try-fallbacks", "flip-block, flip-inline").nest("&:dir(rtl)", StyleBlock::new().set("position-area", "bottom span-right")))
        .nest("& [data-nt-panel][popover]:popover-open, & [data-nt-panel]:not([popover])", pairs(&[("display", "flex"), ("flex-direction", "column")]))
        .nest("& [data-nt-panel]:not([popover])", pairs(&[("width", "100%"), ("border", "1px solid var(--color-border)"), ("border-radius", "var(--radius-lg)")]))
        .nest("& [data-nt-head]", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", sp!(8)), ("padding", sp!(12, 14, 8))]))
        .nest("& [data-nt-head] h3", pairs(&[("margin-block", "0 0"), ("margin-inline", "0 auto"), ("font-size", fs!(15)), ("font-weight", "600")]))
        .nest(
            "& [data-nt-mark-all]",
            pairs(&[("padding", sp!(4, 6)), ("border", "0"), ("border-radius", "var(--radius-sm)"), ("background", "transparent"), ("color", PRIMARY), ("font", "inherit"), ("font-size", fs!(13)), ("font-weight", "500"), ("cursor", "pointer")]),
        )
        .nest("& [data-nt-mark-all]:hover", pairs(&[("background", ACCENT)]))
        .nest("& [data-nt-mark-all]:disabled", pairs(&[("color", MUTED), ("cursor", "default"), ("background", "transparent")]))
        .nest("& [data-nt-mark-all]:focus-visible, & [data-nt-tab]:focus-visible", forms::focus_ring())
        .nest("& [data-nt-tabs]", pairs(&[("display", "flex"), ("gap", sp!(4)), ("padding", sp!(0, 14, 8)), ("border-bottom", "1px solid var(--color-border)")]))
        .nest(
            "& [data-nt-tab]",
            pairs(&[("padding", sp!(4, 10)), ("border", "0"), ("border-radius", "999px"), ("background", "transparent"), ("color", MUTED), ("font", "inherit"), ("font-size", fs!(13)), ("cursor", "pointer")]),
        )
        .nest("& [data-nt-tab][aria-pressed=true]", pairs(&[("background", "color-mix(in srgb, var(--color-text) 8%, transparent)"), ("color", TEXT), ("font-weight", "500")]))
        .nest("& [data-nt-list]", pairs(&[("margin", "0"), ("padding", sp!(4)), ("list-style", "none"), ("max-height", "min(420px, 60vh)"), ("overflow-y", "auto")]))
        .nest("& [data-nt-panel]:not([popover]) [data-nt-list]", pairs(&[("max-height", "none")]))
        .nest("& [data-nt-item]", pairs(&[("--tone", PRIMARY), ("position", "relative"), ("display", "flex"), ("align-items", "flex-start"), ("border-radius", "var(--radius-md)")]))
        .nest("& [data-nt-item]:hover", pairs(&[("background", ACCENT)]))
        .nest(
            "& [data-nt-open]",
            pairs(&[
                ("flex", "1 1 auto"),
                ("min-width", "0"),
                ("display", "flex"),
                ("align-items", "flex-start"),
                ("gap", sp!(10)),
                ("padding", sp!(10)),
                ("border", "0"),
                ("border-radius", "var(--radius-md)"),
                ("background", "transparent"),
                ("color", TEXT),
                ("font", "inherit"),
                ("text-align", "start"),
                ("cursor", "pointer"),
            ]),
        )
        .nest("& [data-nt-open]:focus-visible", forms::focus_ring().set("outline-offset", "-2px"))
        .nest(
            "& [data-nt-mark]",
            pairs(&[
                ("flex", "none"),
                ("display", "grid"),
                ("place-items", "center"),
                ("width", "32px"),
                ("height", "32px"),
                ("border-radius", "50%"),
                ("overflow", "hidden"),
                ("background", "color-mix(in srgb, var(--tone) 14%, var(--color-surface, #fff))"),
                ("color", "color-mix(in srgb, var(--tone) 80%, var(--color-text))"),
                ("font-size", fs!(12)),
                ("font-weight", "600"),
            ]),
        )
        .nest("& [data-nt-mark] img", pairs(&[("width", "100%"), ("height", "100%"), ("object-fit", "cover")]))
        .nest("& [data-nt-mark][data-dot]::after", pairs(&[("content", "''"), ("width", "10px"), ("height", "10px"), ("border-radius", "50%"), ("background", "var(--tone)")]))
        .nest("& [data-nt-text]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", "2px"), ("min-width", "0"), ("padding-inline-end", sp!(48))]))
        .nest("& [data-nt-title]", pairs(&[("line-height", "1.35"), ("overflow-wrap", "anywhere")]))
        .nest("& [data-unread] [data-nt-title]", pairs(&[("font-weight", "600")]))
        .nest(
            "& [data-nt-body]",
            pairs(&[("color", MUTED), ("font-size", fs!(13)), ("line-height", "1.4"), ("display", "-webkit-box"), ("-webkit-line-clamp", "2"), ("-webkit-box-orient", "vertical"), ("overflow", "hidden")]),
        )
        .nest("& [data-nt-text] time", pairs(&[("color", MUTED), ("font-size", fs!(12))]))
        .nest("& [data-nt-actions]", pairs(&[("position", "absolute"), ("top", sp!(8)), ("inset-inline-end", sp!(6)), ("display", "flex"), ("gap", "2px")]))
        .nest(
            "& [data-nt-actions] button",
            pairs(&[("display", "grid"), ("place-items", "center"), ("width", "24px"), ("height", "24px"), ("padding", "0"), ("border", "0"), ("border-radius", "var(--radius-sm)"), ("background", "transparent"), ("color", MUTED), ("cursor", "pointer")]),
        )
        .nest("& [data-nt-actions] button:hover", pairs(&[("background", "color-mix(in srgb, var(--color-text) 8%, transparent)"), ("color", TEXT)]))
        .nest("& [data-nt-actions] button:focus-visible", forms::focus_ring())
        // The read toggle is the unread dot: filled while unread, a ring while read.
        .nest("& [data-nt-toggle] span", pairs(&[("width", "8px"), ("height", "8px"), ("border-radius", "50%"), ("box-sizing", "border-box"), ("border", "1.5px solid currentColor")]))
        .nest("& [data-unread] [data-nt-toggle] span", pairs(&[("border-color", PRIMARY_FILL), ("background", PRIMARY_FILL)]))
        // The archive button shows on hover and focus, and always on touch screens.
        .nest("& [data-nt-archive]", pairs(&[("opacity", "0")]))
        .nest("& [data-nt-item]:is(:hover, :focus-within) [data-nt-archive]", pairs(&[("opacity", "1")]))
        .nest("@media (hover: none)", pairs(&[]).nest("& [data-nt-archive]", pairs(&[("opacity", "1")])))
        .nest("& [data-nt-empty]", pairs(&[("margin", "0"), ("padding", sp!(28, 16)), ("color", MUTED), ("text-align", "center")]))
        .nest("& [data-nt-empty][hidden]", pairs(&[("display", "none")]))
        .nest("& [data-nt-foot]", pairs(&[("padding", sp!(8, 14)), ("border-top", "1px solid var(--color-border)"), ("text-align", "center")]))
        .nest("& [data-nt-foot] a", pairs(&[("color", PRIMARY), ("font-size", fs!(13)), ("font-weight", "500"), ("text-decoration", "none")]))
        .nest("& [data-nt-foot] a:hover", pairs(&[("text-decoration", "underline")]))
        .nest(
            "& [data-nt-live]",
            pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("overflow", "hidden"), ("clip-path", "inset(50%)"), ("white-space", "nowrap")]),
        );
    tone_rules(b, "& [data-nt-item]")
});
static INLINE: LazyLock<StyleBlock> = LazyLock::new(|| pairs(&[("display", "flex")]));

const BELL: &str = r#"<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9"/><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/></svg>"#;

impl Component for Notifications {
    fn name(&self) -> &'static str {
        "ui-notifications"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A bell with an unread count that opens a panel of notifications, with Unread and All, mark read or unread, archive \
                  and Mark all as read. Changes go to `bindItems`; a click opens one into `bindItem`.",
            props: PROPS,
            parts: &["bell", "panel"],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        crate::anchors(cx);
        let inline = cx.prop(node, "inline").is_some_and(|v| expr::truthy(&v));
        let tab = cx.prop_string(node, "tab").unwrap_or_else(|| "unread".into());
        if tab != "unread" && tab != "all" {
            return Err(RenderError::new(node, "`tab` must be `unread` or `all`"));
        }
        let items = match cx.prop(node, "items") {
            Some(Json::Array(a)) => a,
            None | Some(Json::Null) => Vec::new(),
            Some(_) => return Err(RenderError::new(node, "`items` must be a list of notifications")),
        };
        let unread = items.iter().filter(|n| !n.get("read").is_some_and(expr::truthy) && !n.get("archived").is_some_and(expr::truthy)).count();
        let text = |cx: &RenderCx<'_>, name: &str| node.prop(name).map(|v| expr::stringify(&cx.eval(v)));
        let label = text(cx, "label").unwrap_or_else(|| cx.label("Notifications"));
        cx.global_js("ui-notifications", SCRIPT);
        let config = json!({
            "items": dynamic(cx, node, "items", node.prop("items").expect("required"))?,
            "bindItems": cx.prop_string(node, "bindItems"),
            "bindItem": cx.prop_string(node, "bindItem"),
            "tab": tab,
            "label": label,
            "emptyText": text(cx, "emptyText"),
            "caughtUp": text(cx, "caughtUp"),
            "closeOnOpen": cx.prop(node, "closeOnOpen").is_none_or(|v| expr::truthy(&v)),
        });
        let id = instance_id(cx, node, "notifications");
        let mut root = cx.element("div", node);
        root.base_style_ref(&ROOT).behavior("ui-notifications", config);
        if inline {
            root.base_style_ref(&INLINE);
        }
        cx.open(&root, node)?;
        let anchor = format!("--{}", expr::stringify(&cx.eval(&id)));
        if !inline {
            let mut bell = cx.part("button", node, "bell");
            bell.attr("type", "button")
                .attr("data-nt-bell", true)
                .attr("popovertarget", id.clone())
                .attr("aria-haspopup", "dialog")
                .attr("aria-label", if unread > 0 { cx.label_with("{label}, {n} unread", &[("label", &label), ("n", &unread.to_string())]) } else { label.clone() })
                .base_style(StyleBlock::new().set("anchor-name", anchor.clone()));
            cx.open(&bell, node)?;
            cx.markup(BELL);
            let mut count = Element::new("span");
            count.attr("data-nt-count", true).attr("aria-hidden", "true").text(lit(if unread > 99 { "99+".to_string() } else { unread.to_string() }));
            if unread == 0 {
                count.attr("hidden", true);
            }
            cx.open(&count, node)?;
            cx.close("span");
            cx.close("button");
        }
        let mut panel = cx.part("div", node, "panel");
        panel.attr("data-nt-panel", true).attr("id", id.clone()).attr("role", if inline { "region" } else { "dialog" }).attr("aria-labelledby", id_with(&id, "-title"));
        if !inline {
            panel.attr("popover", "auto").base_style(StyleBlock::new().set("position-anchor", anchor));
        }
        cx.open(&panel, node)?;
        let mut head = Element::new("div");
        head.attr("data-nt-head", true);
        cx.open(&head, node)?;
        let mut h = Element::new("h3");
        h.attr("id", id_with(&id, "-title")).text(node.prop("title").cloned().unwrap_or_else(|| lit(cx.label("Notifications"))));
        cx.open(&h, node)?;
        cx.close("h3");
        let mut all = Element::new("button");
        all.attr("type", "button").attr("data-nt-mark-all", true).text(lit(cx.label("Mark all as read")));
        if unread == 0 {
            all.attr("disabled", true);
        }
        cx.open(&all, node)?;
        cx.close("button");
        cx.close("div");
        let mut tabs = Element::new("div");
        tabs.attr("data-nt-tabs", true).attr("role", "group").attr("aria-label", cx.label("Show"));
        cx.open(&tabs, node)?;
        let unread_text = if unread > 0 { cx.label_with("Unread ({n})", &[("n", &unread.to_string())]) } else { cx.label("Unread") };
        for (value, text) in [("unread", unread_text), ("all", cx.label("All"))] {
            let mut b = Element::new("button");
            b.attr("type", "button").attr("data-nt-tab", value).attr("aria-pressed", if tab == value { "true" } else { "false" }).text(lit(text));
            cx.open(&b, node)?;
            cx.close("button");
        }
        cx.close("div");
        let mut list = Element::new("ul");
        list.attr("data-nt-list", true).attr("aria-label", label);
        cx.open(&list, node)?;
        cx.close("ul");
        let mut empty = Element::new("p");
        empty.attr("data-nt-empty", true).attr("hidden", true);
        cx.open(&empty, node)?;
        cx.close("p");
        if let Some(href) = node.prop("allHref") {
            let mut foot = Element::new("div");
            foot.attr("data-nt-foot", true);
            cx.open(&foot, node)?;
            let mut a = Element::new("a");
            a.attr("href", href.clone()).text(lit(cx.label("See all notifications")));
            cx.open(&a, node)?;
            cx.close("a");
            cx.close("div");
        }
        cx.close("div");
        let mut live = Element::new("div");
        live.attr("data-nt-live", true).attr("role", "status").attr("aria-live", "polite");
        cx.open(&live, node)?;
        cx.close("div");
        cx.close("div");
        Ok(())
    }
}
