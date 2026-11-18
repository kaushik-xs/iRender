//! Showing where things stand, without scripts: `ui-status-badge` (a status in its
//! colour), `ui-progress-ring` (a circular progress) and `ui-timeline` with
//! `ui-timeline-item` (who did what and when, with field changes). Each follows state as
//! it changes: the badge's tone and the ring's arc are worked out in the browser from
//! the same expressions the server used.

use crate::{
    BORDER, MUTED, PRIMARY, TEXT, bin, cond, each_item, expr_val, field_or, items, lit, path, synthetic, val_expr,
};
use irender_core::{
    ComponentInfo, Node, Part, PropDef, PropKind, StyleBlock, StyleValue, Template, Val,
    expr::{self, BinOp, Expr},
};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

pub(crate) const TONES: &[&str] = &["neutral", "info", "success", "warning", "danger", "primary"];

/// Each tone's colour, from the theme when it has one.
pub(crate) fn tone_color(tone: &str) -> &'static str {
    match tone {
        "info" => "var(--color-info, #2563eb)",
        "success" => "var(--color-success, #16a34a)",
        "warning" => "var(--color-warning, #d97706)",
        "danger" => "var(--color-destructive, #dc2626)",
        "primary" => PRIMARY,
        _ => MUTED,
    }
}

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

/// Rules giving `--tone` for each `[data-tone=…]`, under `scope` (`&` for the element).
pub(crate) fn tone_rules(mut b: StyleBlock, scope: &str) -> StyleBlock {
    for t in TONES {
        b = b.nest(format!("{scope}[data-tone={t}]"), pairs(&[("--tone", tone_color(t))]));
    }
    b
}

/// Statuses most apps use, and their tone. Matched ignoring case, spaces, `-` and `_`.
const KNOWN: &[(&str, &str)] = &[
    ("done", "success"),
    ("complete", "success"),
    ("completed", "success"),
    ("paid", "success"),
    ("approved", "success"),
    ("active", "success"),
    ("resolved", "success"),
    ("delivered", "success"),
    ("published", "success"),
    ("online", "success"),
    ("open", "info"),
    ("new", "info"),
    ("todo", "info"),
    ("scheduled", "info"),
    ("inprogress", "primary"),
    ("inreview", "warning"),
    ("review", "warning"),
    ("pending", "warning"),
    ("waiting", "warning"),
    ("onhold", "warning"),
    ("draft", "neutral"),
    ("partial", "warning"),
    ("overdue", "danger"),
    ("blocked", "danger"),
    ("failed", "danger"),
    ("rejected", "danger"),
    ("cancelled", "danger"),
    ("canceled", "danger"),
    ("unpaid", "danger"),
    ("error", "danger"),
    ("offline", "danger"),
    ("closed", "neutral"),
    ("archived", "neutral"),
    ("inactive", "neutral"),
];

/// A status as `KNOWN` writes it: lower case, without spaces, `-` or `_`.
fn squash(s: &str) -> String {
    s.chars().filter(|c| !matches!(c, ' ' | '-' | '_')).flat_map(char::to_lowercase).collect()
}

/// The tone for `status`: `tones` first (exact, then ignoring case and spacing), then
/// the known statuses, then `neutral`.
fn tone_of(status: &str, tones: &[(String, String)]) -> String {
    if let Some((_, t)) = tones.iter().find(|(k, _)| k == status).or_else(|| tones.iter().find(|(k, _)| squash(k) == squash(status))) {
        return t.clone();
    }
    KNOWN.iter().find(|(k, _)| *k == squash(status)).map_or("neutral", |(_, t)| t).to_string()
}

// ---------------------------------------------------------------- status badge

pub struct StatusBadge;

const BADGE_PROPS: &[PropDef] = &[
    PropDef::new("status", PropKind::Any, "The status, e.g. `In review` or `{{ ticket.status }}`").required(),
    PropDef::new(
        "tones",
        PropKind::Any,
        "Status → tone (`neutral`, `info`, `success`, `warning`, `danger`, `primary`), over the built-in ones, e.g. `{ Shipped: success }`",
    ),
    PropDef::new("labels", PropKind::Any, "Status → text to show, e.g. `{ in_progress: In progress }` (default: the status)"),
    PropDef::new("dot", PropKind::Bool, "A dot before the text (default true)"),
];

static BADGE: LazyLock<StyleBlock> = LazyLock::new(|| {
    tone_rules(
        pairs(&[
            ("--tone", MUTED),
            ("display", "inline-flex"),
            ("align-items", "center"),
            ("gap", sp!(6)),
            ("width", "fit-content"),
            ("padding", sp!(2, 8)),
            ("border-radius", "999px"),
            ("background", "color-mix(in srgb, var(--tone) 12%, transparent)"),
            ("color", "color-mix(in srgb, var(--tone) 80%, var(--color-text))"),
            ("font-size", fs!(12)),
            ("font-weight", "500"),
            ("line-height", "1.5"),
            ("white-space", "nowrap"),
        ])
        .nest("& [data-dot]", pairs(&[("width", "6px"), ("height", "6px"), ("border-radius", "50%"), ("background", "var(--tone)"), ("flex", "none")])),
        "&",
    )
});

/// `{ status: value }` from a prop holding an object of text.
fn text_map(cx: &RenderCx<'_>, node: &Node, prop: &str) -> Result<Vec<(String, String)>, RenderError> {
    match cx.prop(node, prop) {
        None | Some(Json::Null) => Ok(Vec::new()),
        Some(Json::Object(m)) => Ok(m.iter().map(|(k, v)| (k.clone(), expr::stringify(v))).collect()),
        Some(_) => Err(RenderError::new(node, format!("`{prop}` must be an object of status → text"))),
    }
}

impl Component for StatusBadge {
    fn name(&self) -> &'static str {
        "ui-status-badge"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A status in its colour: common ones (Done, Paid, Open, In review, Blocked, Overdue, …) know theirs, and `tones` sets the rest.",
            props: BADGE_PROPS,
            parts: &[],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let tones = text_map(cx, node, "tones")?;
        if let Some((k, t)) = tones.iter().find(|(_, t)| !TONES.contains(&t.as_str())) {
            return Err(RenderError::new(node, format!("`tones`: `{k}` has no tone `{t}` (tones: {})", TONES.join(", "))));
        }
        let labels = text_map(cx, node, "labels")?;
        let status = node.prop("status").cloned().unwrap_or_else(|| lit(""));
        let (tone, text) = if cx.is_reactive(&status) {
            // Worked out in the browser too: every status we know of, as written and in
            // the usual casings, falling back to neutral.
            let s = val_expr(&status);
            let mut known: Vec<(String, String)> = tones.clone();
            for (k, t) in KNOWN {
                let title = k[..1].to_uppercase() + &k[1..];
                for form in [k.to_string(), title, k.to_uppercase()] {
                    if !known.iter().any(|(x, _)| *x == form) {
                        known.push((form, t.to_string()));
                    }
                }
            }
            for (spaced, t) in [("In progress", "primary"), ("In review", "warning"), ("On hold", "warning"), ("in_progress", "primary"), ("in_review", "warning"), ("on_hold", "warning"), ("in-progress", "primary"), ("To do", "info")] {
                if !known.iter().any(|(x, _)| x == spaced) {
                    known.push((spaced.into(), t.into()));
                }
            }
            let tone = known.iter().rev().fold(Expr::Lit(json!("neutral")), |rest, (k, t)| {
                cond(bin(BinOp::Eq, s.clone(), Expr::Lit(json!(k))), Expr::Lit(json!(t)), rest)
            });
            let text = labels.iter().rev().fold(s.clone(), |rest, (k, l)| cond(bin(BinOp::Eq, s.clone(), Expr::Lit(json!(k))), Expr::Lit(json!(l)), rest));
            (expr_val(tone), expr_val(text))
        } else {
            let s = expr::stringify(&cx.eval(&status));
            let text = labels.iter().find(|(k, _)| *k == s).map_or(s.clone(), |(_, l)| l.clone());
            (lit(tone_of(&s, &tones)), lit(text))
        };
        let mut el = cx.element("span", node);
        el.base_style_ref(&BADGE).attr("data-tone", tone);
        cx.open(&el, node)?;
        if cx.prop(node, "dot").is_none_or(|v| expr::truthy(&v)) {
            let mut dot = Element::new("span");
            dot.attr("data-dot", true).attr("aria-hidden", "true");
            cx.open(&dot, node)?;
            cx.close("span");
        }
        let mut t = Element::new("span");
        t.text(text);
        cx.open(&t, node)?;
        cx.close("span");
        cx.close("span");
        Ok(())
    }
}

// ---------------------------------------------------------------- progress ring

pub struct ProgressRing;

const RING_PROPS: &[PropDef] = &[
    PropDef::new("value", PropKind::Any, "0–100. Follows state when it uses it, e.g. `{{ project.done }}`").required(),
    PropDef::new("size", PropKind::Number, "Width and height in px (default 48)"),
    PropDef::new("thickness", PropKind::Number, "The ring's width in px (default: a tenth of the size)"),
    PropDef::new("text", PropKind::Any, "In the middle (default: the value with `%`); empty for none"),
    PropDef::new("label", PropKind::Any, "What it measures, for screen readers, e.g. `Project progress`"),
    PropDef::new("tone", PropKind::Enum(TONES), "Its colour (default `primary`)"),
];

static RING: LazyLock<StyleBlock> = LazyLock::new(|| {
    tone_rules(
        pairs(&[
            ("--tone", PRIMARY),
            ("position", "relative"),
            ("display", "inline-grid"),
            ("place-items", "center"),
            ("flex", "none"),
            ("color", TEXT),
        ])
        .nest("& svg", pairs(&[("position", "absolute"), ("inset", "0"), ("transform", "rotate(-90deg)")]))
        .nest("& [data-track]", pairs(&[("stroke", "color-mix(in srgb, var(--tone) 15%, transparent)")]))
        .nest(
            "& [data-arc]",
            pairs(&[
                ("stroke", "var(--tone)"),
                ("stroke-dasharray", "100"),
                // `--ui-value` is set per element; clamped so bad values stay in range.
                ("stroke-dashoffset", "calc(100 - min(100, max(0, var(--ui-value))))"),
                ("transition", "stroke-dashoffset .3s"),
            ]),
        )
        .nest("@media (prefers-reduced-motion: reduce)", pairs(&[]).nest("& [data-arc]", pairs(&[("transition", "none")])))
        .nest("& [data-ring-text]", pairs(&[("position", "relative"), ("font-weight", "600"), ("font-variant-numeric", "tabular-nums"), ("line-height", "1")])),
        "&",
    )
});

impl Component for ProgressRing {
    fn name(&self) -> &'static str {
        "ui-progress-ring"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "A circular progress with the value in the middle.", props: RING_PROPS, parts: &[], children: false, open_props: false }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let size = cx.prop(node, "size").map_or(48.0, |v| expr::to_num(&v));
        if !(16.0..=512.0).contains(&size) {
            return Err(RenderError::new(node, "`size` must be from 16 to 512"));
        }
        let thickness = cx.prop(node, "thickness").map_or((size / 10.0).max(3.0), |v| expr::to_num(&v));
        if !(1.0..=size / 2.0).contains(&thickness) {
            return Err(RenderError::new(node, "`thickness` must be from 1 to half the size"));
        }
        let value = node.prop("value").cloned().unwrap_or_else(|| lit(0));
        let number = match &value {
            Val::Tpl(t) => StyleValue::Dynamic { tpl: t.clone(), px: false },
            Val::Lit(v) => StyleValue::Static(expr::to_num(v).to_string()),
        };
        let text = match node.prop("text") {
            Some(t) => t.clone(),
            None => match &value {
                // The value rounded, with a %: a template gains it, a literal becomes it.
                Val::Tpl(t) => {
                    let mut tpl: Template = t.clone();
                    tpl.parts.push(Part::Lit("%".into()));
                    Val::Tpl(tpl)
                }
                Val::Lit(v) => lit(format!("{}%", expr::to_num(v).round())),
            },
        };
        let mut style = StyleBlock::new();
        style.push("--ui-value", number);
        style.push("width", StyleValue::Static(format!("{size}px")));
        style.push("height", StyleValue::Static(format!("{size}px")));
        style.push("font-size", StyleValue::Static(format!("{}px", (size * 0.22).round().max(10.0))));
        let mut el = cx.element("div", node);
        el.base_style_ref(&RING)
            .base_style(style)
            .attr("role", "progressbar")
            .attr("aria-valuemin", lit(0))
            .attr("aria-valuemax", lit(100))
            .attr("aria-valuenow", value.clone());
        if let Some(l) = node.prop("label") {
            el.attr("aria-label", l.clone());
        }
        if let Some(t) = node.prop("tone") {
            el.attr("data-tone", t.clone());
        }
        cx.open(&el, node)?;
        let r = (size - thickness) / 2.0;
        let c = size / 2.0;
        cx.markup(&format!(
            r#"<svg width="{size}" height="{size}" viewBox="0 0 {size} {size}" fill="none" aria-hidden="true"><circle data-track cx="{c}" cy="{c}" r="{r}" stroke-width="{thickness}"/><circle data-arc cx="{c}" cy="{c}" r="{r}" stroke-width="{thickness}" stroke-linecap="round" pathLength="100"/></svg>"#
        ));
        let mut t = Element::new("span");
        t.attr("data-ring-text", true).attr("aria-hidden", "true").text(text);
        cx.open(&t, node)?;
        cx.close("span");
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- timeline

pub struct Timeline;
pub struct TimelineItem;

const TIMELINE_PROPS: &[PropDef] = &[
    PropDef::new("items", PropKind::Any, "Entries with the `ui-timeline-item` props, after any children. From state, like `{{ history }}`, it redraws as it changes"),
    PropDef::new("label", PropKind::Any, "Names it for screen readers (default `Activity`)"),
];
const ITEM_PROPS: &[PropDef] = &[
    PropDef::new("who", PropKind::Any, "Who did it, shown first in bold"),
    PropDef::new("action", PropKind::Any, "What they did, after the name: `changed the status`, `commented`"),
    PropDef::new("when", PropKind::Any, "When, as text: `2 hours ago`, `4 Nov, 10:42`"),
    PropDef::new("initials", PropKind::Any, "In the avatar when there's no image (default: from `who`)"),
    PropDef::new("avatar", PropKind::Any, "Avatar image URL"),
    PropDef::new("changes", PropKind::Any, "Fields changed: `[{ field, from, to }]`, shown as from → to"),
    PropDef::new("comment", PropKind::Any, "Text they wrote, quoted under the line"),
    PropDef::new("tone", PropKind::Enum(TONES), "The dot's colour, for an event rather than a person (with no `who`)"),
];
/// The fields a list item passes to `ui-timeline-item`.
const FIELDS: &[&str] = &["who", "action", "when", "initials", "avatar", "changes", "comment", "tone"];
const ENTRY: &str = "ui_entry";
const ENTRY_INDEX: &str = "ui_entry_index";
const CHANGE: &str = "ui_change";
const CHANGE_INDEX: &str = "ui_change_index";

static TIMELINE: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("margin", "0"), ("padding", "0"), ("list-style", "none"), ("display", "flex"), ("flex-direction", "column"), ("color", TEXT), ("font-size", fs!(14))])
        .nest("& [data-entries]", pairs(&[("display", "contents")]))
});
static ITEM: LazyLock<StyleBlock> = LazyLock::new(|| {
    tone_rules(
        pairs(&[
            ("--tone", MUTED),
            ("position", "relative"),
            ("display", "grid"),
            ("grid-template-columns", "28px minmax(0, 1fr)"),
            ("column-gap", sp!(12)),
            ("padding-bottom", sp!(18)),
        ])
        // The rail between entries.
        .nest("&::before", pairs(&[("content", "''"), ("position", "absolute"), ("left", "13.5px"), ("top", "30px"), ("bottom", "2px"), ("width", "1px"), ("background", BORDER)]))
        .nest("&:last-child::before", pairs(&[("display", "none")]))
        .nest("&:last-child", pairs(&[("padding-bottom", "0")]))
        .nest(
            "& [data-mark]",
            pairs(&[
                ("display", "grid"),
                ("place-items", "center"),
                ("width", "28px"),
                ("height", "28px"),
                ("border-radius", "50%"),
                ("overflow", "hidden"),
                ("background", "color-mix(in srgb, var(--tone) 14%, var(--color-surface, #fff))"),
                ("color", "color-mix(in srgb, var(--tone) 85%, var(--color-text))"),
                ("font-size", fs!(11)),
                ("font-weight", "600"),
            ]),
        )
        .nest("& [data-mark] img", pairs(&[("width", "100%"), ("height", "100%"), ("object-fit", "cover")]))
        .nest("& [data-first]", pairs(&[("display", "block"), ("font-size", "0"), ("text-transform", "uppercase")]))
        .nest("& [data-first]::first-letter", pairs(&[("font-size", fs!(11))]))
        .nest("& [data-mark][data-dot]::after", pairs(&[("content", "''"), ("width", "8px"), ("height", "8px"), ("border-radius", "50%"), ("background", "var(--tone)")]))
        .nest("& [data-body]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", sp!(6)), ("min-width", "0"), ("padding-top", "4px")]))
        .nest("& [data-line]", pairs(&[("line-height", "1.45")]))
        .nest("& [data-line] b", pairs(&[("font-weight", "600")]))
        .nest("& [data-line] time", pairs(&[("color", MUTED), ("font-size", fs!(12)), ("margin-left", sp!(6)), ("white-space", "nowrap")]))
        .nest("& [data-changes]", pairs(&[("margin", "0"), ("padding", "0"), ("list-style", "none"), ("display", "flex"), ("flex-direction", "column"), ("gap", sp!(4))]))
        .nest("& [data-changes] li", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("align-items", "center"), ("gap", sp!(6)), ("font-size", fs!(13))]))
        .nest("& [data-field]", pairs(&[("color", MUTED)]))
        .nest(
            "& :is([data-from], [data-to])",
            pairs(&[("padding", sp!(1, 6)), ("border-radius", "var(--radius-sm)"), ("background", "color-mix(in srgb, var(--color-text) 6%, transparent)"), ("overflow-wrap", "anywhere")]),
        )
        .nest("& [data-from]", pairs(&[("color", MUTED), ("text-decoration", "line-through")]))
        .nest("& [data-from]:empty, & [data-to]:empty", pairs(&[("display", "none")]))
        .nest("& [data-arrow]", pairs(&[("color", MUTED)]))
        .nest(
            "& blockquote",
            pairs(&[("margin", "0"), ("padding", sp!(8, 12)), ("border", "1px solid var(--color-border)"), ("border-radius", "var(--radius-md)"), ("white-space", "pre-wrap"), ("overflow-wrap", "anywhere")]),
        ),
        "&",
    )
});

/// Initials from a name: the first letters of its first two words.
fn initials_of(who: &str) -> String {
    who.split_whitespace().take(2).filter_map(|w| w.chars().next()).flat_map(char::to_uppercase).collect()
}

impl Component for Timeline {
    fn name(&self) -> &'static str {
        "ui-timeline"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Activity, newest first or as given: who did what and when, with field changes (old → new) and comments. \
                  Entries are `ui-timeline-item` children and/or `items`.",
            props: TIMELINE_PROPS,
            parts: &[],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut ol = cx.element("ol", node);
        ol.base_style_ref(&TIMELINE).attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit("Activity")));
        cx.open(&ol, node)?;
        cx.children(node)?;
        if node.prop("items").is_some() {
            let list = Element::new("div");
            let mut list = list;
            list.attr("data-entries", true).attr("role", "presentation");
            let key = expr_val(field_or(ENTRY, "id", path(&[ENTRY_INDEX])));
            let props: Vec<(&str, Val)> = FIELDS.iter().map(|f| (*f, expr_val(path(&[ENTRY, f])))).collect();
            let entry = synthetic(node, "ui-timeline-item", props, Vec::new(), Vec::new());
            each_item(cx, node, list, items(cx, node, "items")?, ENTRY, ENTRY_INDEX, &key, |cx| cx.render(&entry))?;
            cx.close("div");
        }
        cx.close("ol");
        Ok(())
    }
}

impl Component for TimelineItem {
    fn name(&self) -> &'static str {
        "ui-timeline-item"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "One entry of a `ui-timeline`. Children go under its line.", props: ITEM_PROPS, parts: &[], children: true, open_props: false }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let who = node.prop("who").cloned();
        let avatar = node.prop("avatar").cloned();
        let mut li = cx.element("li", node);
        li.base_style_ref(&ITEM);
        if let Some(t) = node.prop("tone") {
            li.attr("data-tone", t.clone());
        }
        cx.open(&li, node)?;

        // The mark: the avatar, else the initials, else (no `who`) a dot for an event.
        // Each is decided per entry, in the browser too for entries from state.
        let missing = |v: &Option<Val>| v.as_ref().map_or(Expr::Lit(Json::Null), val_expr);
        let not = |e: Expr| Expr::Unary(irender_core::expr::UnOp::Not, Box::new(e));
        let mut mark = Element::new("span");
        mark.attr("data-mark", true)
            .attr("aria-hidden", "true")
            .attr("data-dot", expr_val(bin(BinOp::And, not(missing(&who)), not(missing(&avatar)))));
        cx.open(&mark, node)?;
        if let Some(a) = &avatar {
            crate::branch(cx, a, |cx| {
                let mut img = Element::new("img");
                img.attr("src", a.clone()).attr("alt", "");
                cx.open(&img, node)
            })?;
        }
        if let Some(w) = &who {
            let when = expr_val(bin(BinOp::And, val_expr(w), not(missing(&avatar))));
            crate::branch(cx, &when, |cx| {
                let mut s = Element::new("span");
                let initials = node.prop("initials");
                if initials.is_some_and(|i| cx.is_reactive(i)) || cx.is_reactive(w) {
                    // From state: the initials when there are some, else the name with
                    // only its first letter shown (CSS can; the expressions can't split
                    // words).
                    let i = missing(&initials.cloned());
                    s.text(expr_val(cond(i.clone(), i.clone(), val_expr(w)))).attr("data-first", expr_val(not(i)));
                } else {
                    let given = initials.map(|i| expr::stringify(&cx.eval(i))).filter(|i| !i.is_empty());
                    s.text(lit(given.unwrap_or_else(|| initials_of(&expr::stringify(&cx.eval(w))))));
                }
                cx.open(&s, node)?;
                cx.close("span");
                Ok(())
            })?;
        }
        cx.close("span");

        let mut body = Element::new("div");
        body.attr("data-body", true);
        cx.open(&body, node)?;
        let mut line = Element::new("div");
        line.attr("data-line", true);
        cx.open(&line, node)?;
        if let Some(w) = &who {
            let mut b = Element::new("b");
            b.text(w.clone());
            cx.open(&b, node)?;
            cx.close("b");
            cx.text(" ");
        }
        if let Some(a) = node.prop("action") {
            let mut s = Element::new("span");
            s.text(a.clone());
            cx.open(&s, node)?;
            cx.close("span");
        }
        if let Some(w) = node.prop("when") {
            let mut t = Element::new("time");
            t.text(w.clone());
            cx.open(&t, node)?;
            cx.close("time");
        }
        cx.close("div");

        if node.prop("changes").is_some() {
            let mut list = Element::new("ul");
            list.attr("data-changes", true);
            let key = expr_val(path(&[CHANGE_INDEX]));
            each_item(cx, node, list, items(cx, node, "changes")?, CHANGE, CHANGE_INDEX, &key, |cx| {
                let li = Element::new("li");
                cx.open(&li, node)?;
                for (attr, value) in [("data-field", path(&[CHANGE, "field"])), ("data-from", path(&[CHANGE, "from"]))] {
                    let mut s = Element::new("span");
                    s.attr(attr, true).text(expr_val(value));
                    cx.open(&s, node)?;
                    cx.close("span");
                }
                let mut arrow = Element::new("span");
                arrow.attr("data-arrow", true).attr("aria-label", "to").text(lit("→"));
                cx.open(&arrow, node)?;
                cx.close("span");
                let mut to = Element::new("span");
                to.attr("data-to", true).text(expr_val(path(&[CHANGE, "to"])));
                cx.open(&to, node)?;
                cx.close("span");
                cx.close("li");
                Ok(())
            })?;
            cx.close("ul");
        }
        if let Some(c) = node.prop("comment") {
            crate::branch(cx, c, |cx| {
                let mut q = Element::new("blockquote");
                q.text(c.clone());
                cx.open(&q, node)?;
                cx.close("blockquote");
                Ok(())
            })?;
        }
        cx.children(node)?;
        cx.close("div");
        cx.close("li");
        Ok(())
    }
}
