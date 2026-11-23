//! `ui-presence`: who's here now, as avatars stacked over each other with a dot for
//! each one's status and a ring in a colour of their own, then `+3` for the rest. The
//! stack is a button that opens everyone in a list: their status and what they're on.
//! The people may come from state and are the runtime's keyed lists, so it needs no
//! script; with `announce`, `presence.js` says who joins and leaves.

use crate::{BG_POPOVER, MUTED, TEXT, bin, branch, cond, each_item, expr_val, field_or, forms, instance_id, id_with, items, lit, num, path, widget};
use irender_core::{
    ComponentInfo, Node, PropDef, PropKind, StyleBlock, Template, Val,
    expr::{BinOp, Expr, UnOp},
};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("presence.js");

const PERSON: &str = "ui_person";
const PERSON_INDEX: &str = "ui_person_index";

/// The statuses a person may have, each with its dot's colour.
pub(crate) const STATUSES: &[(&str, &str)] = &[
    ("active", "var(--color-success, #16a34a)"),
    ("idle", "var(--color-warning, #d97706)"),
    ("busy", "var(--color-destructive, #dc2626)"),
    ("away", "color-mix(in srgb, var(--color-text) 35%, transparent)"),
    ("offline", "transparent"),
];

const PROPS: &[PropDef] = &[
    PropDef::new(
        "people",
        PropKind::Any,
        "Who's here: `{ id, name, avatar, initials, status, color, where }`; `status` is active, idle, busy, away or offline, `color` 1–8 (default: by place), `where` what they're on. From state, it follows",
    )
    .required(),
    PropDef::new("max", PropKind::Number, "Avatars shown before `+3` (default 4)"),
    PropDef::new("size", PropKind::Number, "Avatar size in px (default 28)"),
    PropDef::new("me", PropKind::Any, "The `id` of whoever's looking: `(you)` after their name"),
    PropDef::new("announce", PropKind::Bool, "Say who joins and leaves, for screen readers"),
    PropDef::new("align", PropKind::Enum(&["start", "end"]), "The list's edge lined up with the avatars (default `end`)"),
    PropDef::new("label", PropKind::Any, "What it shows (default `People here`)"),
    PropDef::new("id", PropKind::Any, "Element id (default: derived from the document)"),
];

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

static PRESENCE: LazyLock<StyleBlock> = LazyLock::new(|| {
    let mut b = pairs(&[("display", "inline-flex"), ("color", TEXT), ("font-size", fs!(14))])
        .nest(
            "& [data-pr-stack]",
            pairs(&[
                ("display", "inline-flex"),
                ("align-items", "center"),
                ("padding", pad!(2, 4)),
                ("border", "0"),
                ("border-radius", "999px"),
                ("background", "transparent"),
                ("color", "inherit"),
                ("font", "inherit"),
                ("cursor", "pointer"),
            ]),
        )
        .nest("& [data-pr-stack]:hover", pairs(&[("background", "color-mix(in srgb, var(--color-text) 6%, transparent)")]))
        .nest("& [data-pr-stack]:focus-visible", forms::focus_ring())
        .nest("& [data-pr-stack] [data-pr-av] + [data-pr-av], & [data-pr-stack] [data-pr-more]", pairs(&[("margin-inline-start", "calc(var(--pr-size) * -0.25)")]))
        .nest("& [data-pr-stack]:hover [data-pr-av] + [data-pr-av]", pairs(&[("margin-inline-start", "calc(var(--pr-size) * -0.15)")]))
        .nest(
            "& [data-pr-av]",
            pairs(&[
                ("position", "relative"),
                ("display", "inline-grid"),
                ("place-items", "center"),
                ("flex", "none"),
                ("width", "var(--pr-size)"),
                ("height", "var(--pr-size)"),
                ("border-radius", "50%"),
                ("background", "color-mix(in srgb, var(--pr-ring) 18%, var(--ui-bg, var(--color-surface, #fff)))"),
                ("color", "color-mix(in srgb, var(--pr-ring) 70%, var(--color-text))"),
                ("box-shadow", "0 0 0 2px var(--ui-bg, var(--color-surface, #fff)), 0 0 0 3.5px var(--pr-ring)"),
                ("font-size", "calc(var(--pr-size) * 0.38)"),
                ("font-weight", "var(--font-weight-semibold, 600)"),
                ("line-height", "1"),
                ("transition", "margin .15s"),
            ]),
        )
        .nest("& [data-pr-av] img", pairs(&[("width", "100%"), ("height", "100%"), ("border-radius", "50%"), ("object-fit", "cover")]))
        .nest("& [data-pr-first]", pairs(&[("display", "block"), ("font-size", "0"), ("text-transform", "uppercase")]))
        .nest("& [data-pr-first]::first-letter", pairs(&[("font-size", "calc(var(--pr-size) * 0.38)")]))
        // The status: a dot at the foot, on the avatar's edge.
        .nest(
            "& [data-pr-av]::after",
            pairs(&[
                ("content", "''"),
                ("position", "absolute"),
                ("inset-inline-end", "-1px"),
                ("bottom", "-1px"),
                ("width", "calc(var(--pr-size) * 0.32)"),
                ("height", "calc(var(--pr-size) * 0.32)"),
                ("border-radius", "50%"),
                ("background", "var(--pr-dot, transparent)"),
                ("box-shadow", "0 0 0 2px var(--ui-bg, var(--color-surface, #fff))"),
            ]),
        )
        .nest("& [data-pr-av]:not([data-status])::after, & [data-pr-av][data-status=offline]::after", pairs(&[("display", "none")]))
        .nest("& [data-pr-av][data-status=offline]", pairs(&[("opacity", "0.55")]))
        .nest(
            "& [data-pr-more]",
            pairs(&[
                ("position", "relative"),
                ("display", "inline-grid"),
                ("place-items", "center"),
                ("min-width", "var(--pr-size)"),
                ("height", "var(--pr-size)"),
                ("padding", "0 6px"),
                ("box-sizing", "border-box"),
                ("border-radius", "999px"),
                ("background", "color-mix(in srgb, var(--color-text) 10%, var(--ui-bg, var(--color-surface, #fff)))"),
                ("box-shadow", "0 0 0 2px var(--ui-bg, var(--color-surface, #fff))"),
                ("color", MUTED),
                ("font-size", "calc(var(--pr-size) * 0.38)"),
                ("font-weight", "var(--font-weight-semibold, 600)"),
            ]),
        )
        .nest("& [data-pr-more][hidden]", pairs(&[("display", "none")]))
        // Everyone, in the popover.
        .nest(
            "& [data-pr-pop]",
            pairs(&[
                ("box-sizing", "border-box"),
                ("inset", "auto"),
                ("min-width", sp!(240)),
                ("max-width", "min(340px, calc(100vw - 16px))"),
                ("max-height", sp!(360)),
                ("overflow", "auto"),
                ("margin", concat!(gap!(6), " 0")),
                ("padding", pad!(6)),
                ("border", "var(--border-width, 1px) solid var(--color-border)"),
                ("border-radius", "var(--radius-card, var(--radius-md))"),
                ("--ui-bg", BG_POPOVER),
                ("background", BG_POPOVER),
                ("color", TEXT),
                ("box-shadow", "var(--elevation-popover, 0 6px 18px rgb(0 0 0 / 0.12))"),
                ("position-try-fallbacks", "flip-block, flip-inline"),
            ]),
        )
        .nest("& [data-pr-pop] h3", pairs(&[("margin", concat!("4px 8px ", "6px")), ("color", MUTED), ("font-size", fs!(12)), ("font-weight", "var(--font-weight-semibold, 600)")]))
        .nest("& [data-pr-list]", pairs(&[("margin", "0"), ("padding", "0"), ("list-style", "none")]))
        .nest("& [data-pr-list] li", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", gap!(10)), ("padding", pad!(6, 8)), ("border-radius", "var(--radius-sm)")]))
        .nest("& [data-pr-list] li > div", pairs(&[("display", "flex"), ("flex-direction", "column"), ("min-width", "0")]))
        .nest("& [data-pr-name]", pairs(&[("font-weight", "var(--font-weight-medium, 500)")]))
        .nest("& [data-pr-you]", pairs(&[("color", MUTED), ("font-weight", "400")]))
        .nest("& [data-pr-where]", pairs(&[("color", MUTED), ("font-size", fs!(12)), ("overflow", "hidden"), ("text-overflow", "ellipsis"), ("white-space", "nowrap")]))
        .nest("& [data-pr-where]:empty", pairs(&[("display", "none")]))
        .nest(
            "& [data-pr-live]",
            pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("overflow", "hidden"), ("clip-path", "inset(50%)"), ("white-space", "nowrap")]),
        );
    for (s, color) in STATUSES {
        b = b.nest(format!("& [data-status={s}]"), pairs(&[("--pr-dot", color)]));
    }
    // The theme's chart colours, else the chart kit's own.
    for (n, fallback) in ["#2a78d6", "#eb6834", "#1baf7a", "#eda100", "#e87ba4", "#008300", "#6250d6", "#e34948"].iter().enumerate() {
        b = b.nest(format!("& [data-color='{}']", n + 1), pairs(&[("--pr-ring", &format!("var(--color-chart-{}, {fallback})", n + 1))]));
    }
    b
});

/// The person's field, as an expression.
fn of(field: &str) -> Expr {
    path(&[PERSON, field])
}

pub struct Presence;

impl Component for Presence {
    fn name(&self) -> &'static str {
        "ui-presence"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Who's here now: stacked avatars with a status dot and a colour each, then `+3`; a click lists everyone, their status and what they're on.",
            props: PROPS,
            parts: &["trigger", "popover"],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let max = widget::number(cx, node, "max").unwrap_or(4.0);
        if !(1.0..=20.0).contains(&max) || max.fract() != 0.0 {
            return Err(RenderError::new(node, "`max` is how many avatars show, 1 to 20"));
        }
        let size = widget::number(cx, node, "size").unwrap_or(28.0);
        if !(16.0..=64.0).contains(&size) {
            return Err(RenderError::new(node, "`size` is from 16 to 64"));
        }
        let people = items(cx, node, "people")?;
        if let Some((i, _)) = people.0.iter().enumerate().find(|(_, p)| !p.is_object() || p.get("name").is_none_or(Json::is_null)) {
            return Err(RenderError::new(node, format!("person {} needs a `name`", i + 1)));
        }
        let max = max as usize;
        // How many: `people.length`, read from state as it changes.
        let count = match people.1 {
            Some(v) => {
                let p = path_of(v);
                if p.is_empty() {
                    return Err(RenderError::new(node, "`people` from state is a key, like `{{ team }}` or `{{ doc.viewers }}`"));
                }
                Expr::Path(p.into_iter().chain(["length".to_string()]).collect())
            }
            None => num(people.0.len()),
        };
        let me = node.prop("me").cloned();
        let label = node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("People here")));
        let id = instance_id(cx, node, "presence");
        let pop = id_with(&id, "-list");
        let anchor = format!("--{}", irender_core::expr::stringify(&cx.eval(&id)));
        crate::anchors(cx);
        crate::flips(cx);
        let announce = widget::flag(cx, node, "announce", false);
        if announce {
            cx.global_js("ui-presence", SCRIPT);
        }

        let mut root = cx.element("div", node);
        root.base_style_ref(&PRESENCE)
            .base_style(
                StyleBlock::new()
                    .set("--pr-size", format!("{size}px"))
                    // Past `max`, the avatars give way to `+3`.
                    .nest(format!("& [data-pr-stack] [data-pr-av]:nth-of-type(n + {})", max + 1), pairs(&[("display", "none")])),
            )
            .attr("data-pr", true);
        if announce {
            root.behavior("ui-presence", json!({}));
        }
        cx.open(&root, node)?;

        // The stack: one button, named with how many are here.
        let named = Val::Tpl(Template {
            parts: vec![
                irender_core::Part::Expr(crate::val_expr(&label)),
                irender_core::Part::Lit(": ".into()),
                irender_core::Part::Expr(count.clone()),
            ],
        });
        let mut trigger = cx.part("button", node, "trigger");
        trigger
            .base_style(StyleBlock::new().set("anchor-name", anchor.clone()))
            .attr("type", "button")
            .attr("id", id.clone())
            .attr("data-pr-stack", true)
            .attr("popovertarget", pop.clone())
            .attr("aria-haspopup", "dialog")
            .attr("aria-label", named);
        cx.open(&trigger, node)?;
        let key = expr_val(field_or(PERSON, "id", path(&[PERSON, "name"])));
        let stack = Element::new("span");
        let mut stack = stack;
        stack.attr("data-pr-avatars", true).attr("aria-hidden", "true").base_style(StyleBlock::new().set("display", "contents"));
        each_item(cx, node, stack, people.clone(), PERSON, PERSON_INDEX, &key, |cx| avatar(cx, node, true))?;
        cx.close("span");
        let rest = bin(BinOp::Sub, count.clone(), num(max));
        let mut more = Element::new("span");
        more.attr("data-pr-more", true)
            .attr("aria-hidden", "true")
            .attr("hidden", expr_val(bin(BinOp::Le, count.clone(), num(max))))
            .text(expr_val(bin(BinOp::Add, num("+"), rest)));
        cx.open(&more, node)?;
        cx.close("span");
        cx.close("button");

        // Everyone, with their status and what they're on.
        let align = cx.prop_string(node, "align").unwrap_or_else(|| "end".into());
        let area = if align == "start" { "bottom span-right" } else { "bottom span-left" };
        let mut popover = cx.part("div", node, "popover");
        popover
            .base_style(crate::with_area(StyleBlock::new().set("position-anchor", anchor), area))
            .attr("popover", "auto")
            .attr("id", pop)
            .attr("role", "dialog")
            .attr("data-pr-pop", true)
            .attr("aria-label", label.clone());
        cx.open(&popover, node)?;
        let mut h = Element::new("h3");
        h.text(Val::Tpl(Template { parts: vec![irender_core::Part::Expr(crate::val_expr(&label)), irender_core::Part::Lit(" · ".into()), irender_core::Part::Expr(count)] }));
        cx.open(&h, node)?;
        cx.close("h3");
        let mut list = Element::new("ul");
        list.attr("data-pr-list", true);
        // Labels: "Active", "Idle", "Busy", "Away", "Offline"
        let words: Vec<(String, String)> = STATUSES.iter().map(|(s, _)| (s.to_string(), cx.label(&(s[..1].to_uppercase() + &s[1..])))).collect();
        let you = cx.label("(you)");
        each_item(cx, node, list, people, PERSON, PERSON_INDEX, &key, |cx| {
            let mut li = Element::new("li");
            li.attr("data-pr-person", expr_val(field_or(PERSON, "id", of("name"))));
            cx.open(&li, node)?;
            avatar(cx, node, false)?;
            let div = Element::new("div");
            cx.open(&div, node)?;
            let mut name = Element::new("span");
            name.attr("data-pr-name", true);
            cx.open(&name, node)?;
            let mut n = Element::new("span");
            n.text(expr_val(of("name")));
            cx.open(&n, node)?;
            cx.close("span");
            if let Some(m) = &me {
                let mine = expr_val(bin(BinOp::And, bin(BinOp::Ne, of("id"), num(Json::Null)), bin(BinOp::Eq, of("id"), crate::val_expr(m))));
                branch(cx, &mine, |cx| {
                    let mut y = Element::new("span");
                    y.attr("data-pr-you", true).text(lit(format!(" {you}")));
                    cx.open(&y, node)?;
                    cx.close("span");
                    Ok(())
                })?;
            }
            cx.close("span");
            // "Active · Editing Budget": the status in words, and where they are.
            let status = words.iter().rev().fold(Expr::Lit(json!("")), |rest, (s, w)| cond(bin(BinOp::Eq, of("status"), num(s.as_str())), num(w.as_str()), rest));
            let place = cond(of("where"), of("where"), num(""));
            let line = cond(
                bin(BinOp::And, status.clone(), place.clone()),
                bin(BinOp::Add, bin(BinOp::Add, status.clone(), num(" · ")), place.clone()),
                cond(status.clone(), status, place),
            );
            let mut w = Element::new("span");
            w.attr("data-pr-where", true).text(expr_val(line));
            cx.open(&w, node)?;
            cx.close("span");
            cx.close("div");
            cx.close("li");
            Ok(())
        })?;
        cx.close("ul");
        cx.close("div");
        if announce {
            let mut live = Element::new("span");
            live.attr("data-pr-live", true).attr("role", "status").attr("aria-live", "polite");
            cx.open(&live, node)?;
            cx.close("span");
        }
        cx.close("div");
        Ok(())
    }
}

/// The state path a `{{ key }}` (or `{{ a.b }}`) value reads.
fn path_of(v: &Val) -> Vec<String> {
    match crate::val_expr(v) {
        Expr::Path(p) => p,
        _ => Vec::new(),
    }
}

/// One person's avatar: their picture, else their initials (or their name's first letter),
/// with a dot for their status and a ring in their colour.
fn avatar(cx: &mut RenderCx<'_>, node: &Node, stacked: bool) -> Result<(), RenderError> {
    let not = |e: Expr| Expr::Unary(UnOp::Not, Box::new(e));
    // The colour given (1–8), else one by place.
    let color = cond(of("color"), of("color"), bin(BinOp::Add, bin(BinOp::Rem, Expr::Path(vec![PERSON_INDEX.into()]), num(8)), num(1)));
    let mut av = Element::new("span");
    av.attr("data-pr-av", true).attr("data-status", expr_val(of("status"))).attr("data-color", expr_val(color));
    if stacked {
        av.attr("title", expr_val(of("name")));
    } else {
        av.attr("aria-hidden", "true");
    }
    cx.open(&av, node)?;
    let pic = expr_val(of("avatar"));
    branch(cx, &pic, |cx| {
        let mut img = Element::new("img");
        img.attr("src", expr_val(of("avatar"))).attr("alt", "");
        cx.open(&img, node)
    })?;
    branch(cx, &expr_val(not(of("avatar"))), |cx| {
        let mut s = Element::new("span");
        let initials = of("initials");
        s.text(expr_val(cond(initials.clone(), initials.clone(), of("name")))).attr("data-pr-first", expr_val(not(initials)));
        cx.open(&s, node)?;
        cx.close("span");
        Ok(())
    })?;
    cx.close("span");
    Ok(())
}
