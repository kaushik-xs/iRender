//! `ui-kanban`: a board of columns with cards to move between them. The server draws the
//! starting board from `columns` and `cards`; `kanban.js` draws it again whenever they
//! change, and moves cards by dragging or with Alt and the arrow keys, writing the new
//! order (and each card's `column`) to `bindCards`.

use crate::data_table::dynamic;
use crate::status::tone_rules;
use crate::{ACCENT, BORDER, MUTED, PRIMARY, TEXT, forms, lit};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("kanban.js");

pub struct Kanban;

const PROPS: &[PropDef] = &[
    PropDef::new("columns", PropKind::Any, "The columns, in order: `{ id, title, tone, limit }` (or plain names). `limit` marks a column over it").required(),
    PropDef::new(
        "cards",
        PropKind::Any,
        "The cards, in order: `{ id, column, title, description, tags, assignee, due, priority }`, usually from state, e.g. `{{ tasks }}`",
    )
    .required(),
    PropDef::new("bindCards", PropKind::StateKey, "State key the cards are in: moves and new cards are written back to it"),
    PropDef::new("bindCard", PropKind::StateKey, "State key that gets the card clicked or opened with Enter"),
    PropDef::new("movable", PropKind::Bool, "Drag cards, or move the focused one with Alt and the arrow keys (default true)"),
    PropDef::new("addable", PropKind::Bool, "An Add card button under each column, which asks for a title (needs `bindCards`)"),
    PropDef::new("collapsible", PropKind::Bool, "A button on each column to fold it away"),
    PropDef::new("collapsed", PropKind::Any, "Columns folded to start with, by id"),
    PropDef::new("addLabel", PropKind::Any, "The add button's text (default `Add card`)"),
    PropDef::new("emptyText", PropKind::String, "In a column with no cards (default `No cards`)"),
    PropDef::new("columnWidth", PropKind::Number, "Each column's width in px (default 280)"),
    PropDef::new("label", PropKind::Any, "Names the board for screen readers (default `Board`)"),
];

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

static BOARD: LazyLock<StyleBlock> = LazyLock::new(|| {
    let tag = pairs(&[
        ("display", "inline-flex"),
        ("align-items", "center"),
        ("padding", "1px 8px"),
        ("border-radius", "999px"),
        ("background", "color-mix(in srgb, var(--tone) 12%, transparent)"),
        ("color", "color-mix(in srgb, var(--tone) 80%, var(--color-text))"),
        ("font-size", fs!(11)),
        ("font-weight", "500"),
        ("line-height", "1.6"),
    ]);
    let b = pairs(&[("color", TEXT), ("font-size", fs!(14))])
        .nest(
            "& [data-kb-board]",
            pairs(&[("display", "flex"), ("align-items", "flex-start"), ("gap", sp!(12)), ("overflow-x", "auto"), ("padding-bottom", sp!(8)), ("scrollbar-width", "thin")]),
        )
        .nest(
            "& [data-kb-column]",
            pairs(&[
                ("--tone", MUTED),
                ("flex", "0 0 var(--kb-width, 280px)"),
                ("display", "flex"),
                ("flex-direction", "column"),
                ("gap", sp!(8)),
                ("box-sizing", "border-box"),
                ("padding", sp!(10)),
                ("border-radius", "var(--radius-lg)"),
                ("background", "color-mix(in srgb, var(--color-text) 3.5%, var(--color-surface, #fff))"),
                ("transition", "background .15s, box-shadow .15s"),
            ]),
        )
        .nest("& [data-kb-column][data-over-drop]", pairs(&[("box-shadow", "inset 0 0 0 1.5px color-mix(in srgb, var(--color-primary) 45%, transparent)")]))
        .nest("& [data-kb-head]", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", sp!(8)), ("padding", sp!(2, 4))]))
        .nest("& [data-kb-head] h3", pairs(&[("margin", "0"), ("font-size", fs!(13)), ("font-weight", "600"), ("min-width", "0"), ("overflow", "hidden"), ("text-overflow", "ellipsis"), ("white-space", "nowrap")]))
        .nest("& [data-kb-dot]", pairs(&[("flex", "none"), ("width", "8px"), ("height", "8px"), ("border-radius", "50%"), ("background", "var(--tone)")]))
        .nest(
            "& [data-kb-count]",
            // "2 / 3" reads left to right on any page.
            pairs(&[("direction", "ltr"), ("unicode-bidi", "isolate"), ("padding", "0 7px"), ("border-radius", "999px"), ("background", "color-mix(in srgb, var(--color-text) 8%, transparent)"), ("color", MUTED), ("font-size", fs!(12)), ("font-variant-numeric", "tabular-nums")]),
        )
        .nest("& [data-over] [data-kb-count]", pairs(&[("background", "color-mix(in srgb, var(--color-destructive, #dc2626) 14%, transparent)"), ("color", "var(--color-destructive, #dc2626)"), ("font-weight", "600")]))
        .nest(
            "& [data-kb-fold]",
            pairs(&[("margin-inline-start", "auto"), ("display", "inline-flex"), ("padding", "4px"), ("border", "0"), ("border-radius", "var(--radius-sm)"), ("background", "transparent"), ("color", MUTED), ("cursor", "pointer")]),
        )
        .nest("& [data-kb-fold]:hover", pairs(&[("background", ACCENT), ("color", TEXT)]))
        .nest("& [data-kb-fold]:focus-visible", forms::focus_ring())
        // Folded: a narrow strip with the title down its side.
        .nest("& [data-kb-column][data-collapsed]", pairs(&[("flex-basis", "44px"), ("align-items", "center")]))
        .nest("& [data-collapsed] [data-kb-head]", pairs(&[("flex-direction", "column"), ("padding", "2px 0")]))
        .nest("& [data-collapsed] [data-kb-head] h3", pairs(&[("writing-mode", "vertical-rl"), ("overflow", "visible")]))
        .nest("& [data-collapsed] [data-kb-fold]", pairs(&[("margin", "0"), ("order", "-1"), ("transform", "rotate(180deg)")]))
        .nest("& [data-collapsed] :is([data-kb-cards], [data-kb-add], [data-kb-name])", pairs(&[("display", "none")]))
        .nest("& [data-kb-cards]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", sp!(8)), ("min-height", "48px")]))
        .nest("& [data-kb-empty]", pairs(&[("margin", "0"), ("padding", sp!(12, 4)), ("color", MUTED), ("font-size", fs!(13)), ("text-align", "center")]))
        .nest(
            "& [data-kb-card]",
            pairs(&[
                ("display", "flex"),
                ("flex-direction", "column"),
                ("gap", sp!(8)),
                ("padding", sp!(10, 12)),
                ("border", "1px solid var(--color-border)"),
                ("border-radius", "var(--radius-md)"),
                ("background", "var(--color-card, var(--color-surface, #fff))"),
                ("box-shadow", "0 1px 2px rgb(0 0 0 / 0.05)"),
                ("cursor", "pointer"),
                ("outline", "none"),
                ("transition", "box-shadow .15s, border-color .15s, opacity .15s"),
            ]),
        )
        .nest("& [data-kb-card][draggable=true]", pairs(&[("cursor", "grab")]))
        .nest("& [data-kb-card]:hover", pairs(&[("border-color", "color-mix(in srgb, var(--color-text) 22%, transparent)"), ("box-shadow", "0 2px 6px rgb(0 0 0 / 0.08)")]))
        .nest("& [data-kb-card]:focus-visible", forms::focus_ring().set("outline-offset", "1px"))
        .nest("& [data-kb-card][data-dragging]", pairs(&[("opacity", "0.35")]))
        .nest("& [data-kb-title]", pairs(&[("font-weight", "500"), ("line-height", "1.35"), ("overflow-wrap", "anywhere")]))
        .nest(
            "& [data-kb-desc]",
            pairs(&[("margin", "0"), ("color", MUTED), ("font-size", fs!(13)), ("line-height", "1.4"), ("display", "-webkit-box"), ("-webkit-line-clamp", "2"), ("-webkit-box-orient", "vertical"), ("overflow", "hidden")]),
        )
        .nest("& [data-kb-tags]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("gap", "4px")]))
        .nest("& [data-kb-tags] span, & [data-kb-priority]", tag)
        .nest("& [data-kb-foot]", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", sp!(8)), ("color", MUTED), ("font-size", fs!(12))]))
        .nest(
            "& [data-kb-avatar]",
            pairs(&[
                ("margin-inline-start", "auto"),
                ("display", "grid"),
                ("place-items", "center"),
                ("width", "24px"),
                ("height", "24px"),
                ("border-radius", "50%"),
                ("overflow", "hidden"),
                ("background", "color-mix(in srgb, var(--color-primary) 14%, var(--color-surface, #fff))"),
                ("color", TEXT),
                ("font-size", fs!(10)),
                ("font-weight", "600"),
            ]),
        )
        .nest("& [data-kb-avatar] img", pairs(&[("width", "100%"), ("height", "100%"), ("object-fit", "cover")]))
        .nest("& [data-kb-drop]", pairs(&[("height", "2px"), ("margin", "-5px 2px"), ("border-radius", "2px"), ("background", PRIMARY)]))
        .nest(
            "& [data-kb-add]",
            pairs(&[
                ("display", "flex"),
                ("align-items", "center"),
                ("gap", sp!(6)),
                ("padding", sp!(6, 8)),
                ("border", "0"),
                ("border-radius", "var(--radius-md)"),
                ("background", "transparent"),
                ("color", MUTED),
                ("font", "inherit"),
                ("font-size", fs!(13)),
                ("cursor", "pointer"),
            ]),
        )
        .nest("& [data-kb-add]:hover", pairs(&[("background", "color-mix(in srgb, var(--color-text) 6%, transparent)"), ("color", TEXT)]))
        .nest("& [data-kb-add]:focus-visible", forms::focus_ring())
        .nest(
            "& [data-kb-name]",
            pairs(&[
                ("box-sizing", "border-box"),
                ("width", "100%"),
                ("padding", sp!(8, 10)),
                ("border", "1px solid var(--color-border)"),
                ("border-radius", "var(--radius-md)"),
                ("background", "var(--color-card, var(--color-surface, #fff))"),
                ("color", TEXT),
                ("font", "inherit"),
                ("resize", "vertical"),
            ]),
        )
        .nest("& [data-kb-name]:focus-visible", forms::focus_ring().set("border-color", BORDER))
        .nest(
            "& [data-kb-live]",
            pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("overflow", "hidden"), ("clip-path", "inset(50%)"), ("white-space", "nowrap")]),
        )
        .nest("@media (prefers-reduced-motion: reduce)", pairs(&[]).nest("& *", pairs(&[("transition", "none !important")])));
    // Column dots, tags and priorities take their tone.
    tone_rules(tone_rules(b, "& [data-kb-column]"), "& :is([data-kb-tags] span, [data-kb-priority])")
});

const PRIORITY: &[(&str, &str)] = &[("urgent", "danger"), ("critical", "danger"), ("high", "warning"), ("medium", "info"), ("normal", "info"), ("low", "neutral")];

fn text(v: Option<&Json>) -> String {
    v.map(expr::stringify).unwrap_or_default()
}

fn initials_of(name: &str) -> String {
    name.split_whitespace().take(2).filter_map(|w| w.chars().next()).flat_map(char::to_uppercase).collect()
}

impl Component for Kanban {
    fn name(&self) -> &'static str {
        "ui-kanban"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A board of columns with cards to drag between them (or move with Alt and the arrow keys), add to, and open. \
                  The new order goes to `bindCards`, with `move`, `add` and `open` events.",
            props: PROPS,
            parts: &["board"],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        crate::flips(cx);
        let flag = |cx: &RenderCx<'_>, name: &str, default: bool| cx.prop(node, name).map_or(default, |v| expr::truthy(&v));
        let (movable, addable, collapsible) = (flag(cx, "movable", true), flag(cx, "addable", false), flag(cx, "collapsible", false));
        let list = |cx: &RenderCx<'_>, name: &str| -> Result<Vec<Json>, RenderError> {
            match cx.prop(node, name) {
                Some(Json::Array(a)) => Ok(a),
                None | Some(Json::Null) => Ok(Vec::new()),
                Some(_) => Err(RenderError::new(node, format!("`{name}` must be a list"))),
            }
        };
        let columns = list(cx, "columns")?;
        let cards = list(cx, "cards")?;
        let collapsed: Vec<String> = list(cx, "collapsed")?.iter().map(expr::stringify).collect();
        let bind_cards = cx.prop_string(node, "bindCards");
        if addable && bind_cards.is_none() {
            return Err(RenderError::new(node, "`addable` needs `bindCards`, to add the new card to"));
        }
        let width = cx.prop(node, "columnWidth").map_or(280.0, |v| expr::to_num(&v));

        cx.global_js("ui-kanban", SCRIPT);
        let config = json!({
            "columns": dynamic(cx, node, "columns", node.prop("columns").expect("required"))?,
            "cards": dynamic(cx, node, "cards", node.prop("cards").expect("required"))?,
            "bindCards": bind_cards, "bindCard": cx.prop_string(node, "bindCard"),
            "movable": movable, "addable": addable, "collapsible": collapsible, "collapsed": collapsed,
            "addLabel": node.prop("addLabel").map(|v| expr::stringify(&cx.eval(v))),
            "emptyText": cx.prop_string(node, "emptyText"),
        });
        let mut root = cx.element("div", node);
        root.base_style_ref(&BOARD)
            .base_style(StyleBlock::new().set("--kb-width", format!("{width}px")))
            .behavior("ui-kanban", config)
            .attr("role", "region")
            .attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Board"))))
            .attr("aria-roledescription", cx.label("board"));
        cx.open(&root, node)?;
        let mut board = cx.part("div", node, "board");
        board.attr("data-kb-board", true);
        cx.open(&board, node)?;
        let empty = cx.prop_string(node, "emptyText").unwrap_or_else(|| cx.label("No cards"));
        let mut first = true;
        for col in &columns {
            let (id, title) = match col {
                Json::Object(m) => (text(m.get("id")), m.get("title").map(expr::stringify).unwrap_or_else(|| text(m.get("id")))),
                other => (expr::stringify(other), expr::stringify(other)),
            };
            let mine: Vec<&Json> = cards.iter().filter(|c| text(c.get("column")) == id).collect();
            let limit = col.get("limit").filter(|l| !l.is_null()).map(expr::to_num);
            let folded = collapsed.contains(&id);
            let mut section = Element::new("section");
            section
                .attr("data-kb-column", id.clone())
                .attr("aria-label", format!("{title}, {}", cx.label_count("{n} card", "{n} cards", mine.len() as f64)));
            if let Some(t) = col.get("tone").filter(|t| !t.is_null()) {
                section.attr("data-tone", expr::stringify(t));
            }
            if limit.is_some_and(|l| mine.len() as f64 > l) {
                section.attr("data-over", true);
            }
            if folded {
                section.attr("data-collapsed", true);
            }
            cx.open(&section, node)?;
            let mut head = Element::new("header");
            head.attr("data-kb-head", true);
            cx.open(&head, node)?;
            let mut dot = Element::new("span");
            dot.attr("data-kb-dot", true).attr("aria-hidden", "true");
            cx.open(&dot, node)?;
            cx.close("span");
            let mut h = Element::new("h3");
            h.text(lit(title.clone()));
            cx.open(&h, node)?;
            cx.close("h3");
            let mut count = Element::new("span");
            count.attr("data-kb-count", true).text(lit(match limit {
                Some(l) => format!("{} / {}", mine.len(), l),
                None => mine.len().to_string(),
            }));
            cx.open(&count, node)?;
            cx.close("span");
            if collapsible {
                let mut b = Element::new("button");
                b.attr("type", "button")
                    .attr("data-kb-fold", true)
                    .attr("aria-expanded", if folded { "false" } else { "true" })
                    .attr("aria-label", cx.label_with(if folded { "Show {name}" } else { "Hide {name}" }, &[("name", &title)]));
                cx.open(&b, node)?;
                cx.markup(r#"<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M10 3.5 5.5 8 10 12.5"/></svg>"#);
                cx.close("button");
            }
            cx.close("header");
            let mut list = Element::new("div");
            list.attr("data-kb-cards", true).attr("role", "list");
            cx.open(&list, node)?;
            for c in &mine {
                card(cx, node, c, movable, first)?;
                first = false;
            }
            if mine.is_empty() {
                let mut p = Element::new("p");
                p.attr("data-kb-empty", true).text(lit(empty.clone()));
                cx.open(&p, node)?;
                cx.close("p");
            }
            cx.close("div");
            if addable {
                let mut b = Element::new("button");
                b.attr("type", "button").attr("data-kb-add", true);
                cx.open(&b, node)?;
                cx.markup(r#"<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" aria-hidden="true"><path d="M8 3v10M3 8h10"/></svg>"#);
                let mut l = Element::new("span");
                l.text(node.prop("addLabel").cloned().unwrap_or_else(|| lit(cx.label("Add card"))));
                cx.open(&l, node)?;
                cx.close("span");
                cx.close("button");
            }
            cx.close("section");
        }
        cx.close("div");
        let mut live = Element::new("div");
        live.attr("data-kb-live", true).attr("role", "status").attr("aria-live", "polite");
        cx.open(&live, node)?;
        cx.close("div");
        cx.close("div");
        Ok(())
    }
}

/// One card, as `kanban.js` draws it.
fn card(cx: &mut RenderCx<'_>, node: &Node, c: &Json, movable: bool, first: bool) -> Result<(), RenderError> {
    let title = text(c.get("title"));
    let who = c.get("assignee").filter(|a| !a.is_null() && *a != &json!(""));
    let who_name = who.map(|w| if w.is_object() { text(w.get("name")) } else { expr::stringify(w) });
    let priority = c.get("priority").filter(|p| !p.is_null()).map(expr::stringify);
    let due = c.get("due").filter(|p| !p.is_null()).map(expr::stringify);
    let label: Vec<String> = [
        Some(title.clone()),
        priority.as_ref().map(|p| cx.label_with("{priority} priority", &[("priority", p)])),
        due.as_ref().map(|d| cx.label_with("due {date}", &[("date", d)])),
        who_name.as_ref().filter(|n| !n.is_empty()).map(|n| cx.label_with("assigned to {name}", &[("name", n)])),
    ]
    .into_iter()
    .flatten()
    .collect();
    let mut a = Element::new("article");
    a.attr("data-kb-card", text(c.get("id")))
        .attr("tabindex", lit(if first { 0 } else { -1 }))
        .attr("aria-roledescription", cx.label("card"))
        .attr("aria-label", label.join(", "))
        .attr("role", "listitem");
    if movable {
        a.attr("draggable", "true");
    }
    cx.open(&a, node)?;
    let mut t = Element::new("div");
    t.attr("data-kb-title", true).text(lit(title));
    cx.open(&t, node)?;
    cx.close("div");
    if let Some(d) = c.get("description").filter(|d| !d.is_null() && *d != &json!("")) {
        let mut p = Element::new("p");
        p.attr("data-kb-desc", true).text(lit(expr::stringify(d)));
        cx.open(&p, node)?;
        cx.close("p");
    }
    if let Some(Json::Array(tags)) = c.get("tags").filter(|t| t.as_array().is_some_and(|a| !a.is_empty())) {
        let mut box_ = Element::new("div");
        box_.attr("data-kb-tags", true);
        cx.open(&box_, node)?;
        for tag in tags {
            let (label, tone) = match tag {
                Json::Object(m) => (text(m.get("label").or(m.get("text"))), m.get("tone").map(expr::stringify).unwrap_or_else(|| "neutral".into())),
                other => (expr::stringify(other), "neutral".into()),
            };
            let mut s = Element::new("span");
            s.attr("data-tone", tone).text(lit(label));
            cx.open(&s, node)?;
            cx.close("span");
        }
        cx.close("div");
    }
    if priority.is_some() || due.is_some() || who.is_some() {
        let mut f = Element::new("div");
        f.attr("data-kb-foot", true);
        cx.open(&f, node)?;
        if let Some(p) = &priority {
            let tone = PRIORITY.iter().find(|(k, _)| *k == p.to_lowercase()).map_or("neutral", |(_, t)| t);
            let mut s = Element::new("span");
            s.attr("data-kb-priority", true).attr("data-tone", tone).text(lit(p.clone()));
            cx.open(&s, node)?;
            cx.close("span");
        }
        if let Some(d) = &due {
            let mut s = Element::new("span");
            s.attr("data-kb-due", true).text(lit(d.clone()));
            cx.open(&s, node)?;
            cx.close("span");
        }
        if let Some(w) = who {
            let mut s = Element::new("span");
            s.attr("data-kb-avatar", true).attr("title", who_name.clone().unwrap_or_default());
            let avatar = w.get("avatar").filter(|a| !a.is_null()).map(expr::stringify);
            if avatar.is_none() {
                let initials = w.get("initials").map(expr::stringify).unwrap_or_else(|| initials_of(who_name.as_deref().unwrap_or("")));
                s.text(lit(initials));
            }
            cx.open(&s, node)?;
            if let Some(src) = avatar {
                let mut img = Element::new("img");
                img.attr("src", src).attr("alt", "");
                cx.open(&img, node)?;
            }
            cx.close("span");
        }
        cx.close("div");
    }
    cx.close("article");
    Ok(())
}
