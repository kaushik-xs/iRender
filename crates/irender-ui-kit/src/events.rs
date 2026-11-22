//! `ui-event-calendar`: events in a month grid, a week of hours, or a list by day, with a
//! toolbar to move through time and switch views. The server draws the frame and the
//! toolbar; `events.js` draws the view from `events` as they change.

use crate::data_table::dynamic;
use crate::status::tone_rules;
use crate::{ACCENT, MUTED, PRIMARY, TEXT, forms, lit};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("events.js");

pub(crate) const VIEWS: &[&str] = &["day", "week", "month", "year", "list"];

pub struct EventCalendar;

const PROPS: &[PropDef] = &[
    PropDef::new(
        "events",
        PropKind::Any,
        "The events: `{ id, title, start, end, allDay, category, tone, description }`, with `start` and `end` as dates (`2026-11-04`) or local times (`2026-11-04T09:30`)",
    )
    .required(),
    PropDef::new("view", PropKind::Enum(VIEWS), "The view to start in: `day`, `week`, `month` (default), `year` or `list`"),
    PropDef::new("views", PropKind::Any, "Which views to offer, in order (default all five: day, week, month, year, list); one hides the switch"),
    PropDef::new("date", PropKind::Any, "The day to show to start with (default today)"),
    PropDef::new("categories", PropKind::Any, "Category → tone (`info`, `success`, `warning`, `danger`, `primary`, `neutral`), shown as a legend"),
    PropDef::new("bindEvent", PropKind::StateKey, "State key that gets the event clicked"),
    PropDef::new(
        "bindEvents",
        PropKind::StateKey,
        "State key holding the events: makes them movable (drag them, or Alt and the arrow keys) and writes the list back",
    ),
    PropDef::new("snap", PropKind::Number, "Minutes a move or a resize snaps to (default 15)"),
    PropDef::new("bindDay", PropKind::StateKey, "State key that gets the day picked (`2026-11-04`), e.g. to add an event on it"),
    PropDef::new("bindView", PropKind::StateKey, "State key that gets the view when it changes"),
    PropDef::new("hours", PropKind::Any, "The week view's hours, `[from, to]` (default `[7, 20]`)"),
    PropDef::new("weekStart", PropKind::Number, "0 for Sunday, 1 for Monday (default)"),
    PropDef::new("maxPerDay", PropKind::Number, "Events shown in a month's day before `+N more` (default 3)"),
    PropDef::new("today", PropKind::String, "The date to treat as today (default: the real one)"),
    PropDef::new("locale", PropKind::String, "Locale for dates and times (default: the document's `lang`, else the browser's)"),
    PropDef::new("emptyText", PropKind::Any, "The list view's text for a month with nothing in it"),
    PropDef::new("label", PropKind::Any, "Names it for screen readers (default `Calendar`)"),
];

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

static CALENDAR: LazyLock<StyleBlock> = LazyLock::new(|| {
    let chip = pairs(&[
        ("--tone", PRIMARY),
        ("display", "flex"),
        ("align-items", "baseline"),
        ("gap", gap!(4)),
        ("width", "100%"),
        ("min-width", "0"),
        ("box-sizing", "border-box"),
        ("padding", pad!(1, 6)),
        ("border", "0"),
        ("border-inline-start", "3px solid var(--tone)"),
        ("border-radius", "4px"),
        ("background", "color-mix(in srgb, var(--tone) 13%, var(--color-surface, #fff))"),
        ("color", "color-mix(in srgb, var(--tone) 70%, var(--color-text))"),
        ("font", "inherit"),
        ("font-size", fs!(12)),
        ("line-height", "1.5"),
        ("text-align", "start"),
        ("cursor", "pointer"),
        ("white-space", "nowrap"),
        ("overflow", "hidden"),
    ]);
    let b = pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(12)), ("min-width", "0"), ("color", TEXT), ("font-size", fs!(14))])
        .nest("& [data-ev-bar]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("align-items", "center"), ("gap", gap!(8))]))
        .nest("& [data-ev-title]", pairs(&[("margin-block", "0 0"), ("margin-inline", concat!(gap!(4), " auto")), ("font-size", fs!(17)), ("font-weight", "var(--font-weight-semibold, 600)"), ("min-width", "0")]))
        .nest("& [data-ev-views]", pairs(&[("display", "inline-flex"), ("padding", "2px"), ("border-radius", "var(--radius-control, var(--radius-md))"), ("background", "color-mix(in srgb, var(--color-text) 6%, transparent)")]))
        .nest(
            "& [data-ev-views] button",
            pairs(&[("padding", pad!(4, 12)), ("border", "0"), ("border-radius", "calc(var(--radius-control, var(--radius-md)) - 2px)"), ("background", "transparent"), ("color", MUTED), ("font", "inherit"), ("font-size", fs!(13)), ("cursor", "pointer")]),
        )
        .nest("& [data-ev-views] button[aria-pressed=true]", pairs(&[("background", "var(--color-surface, #fff)"), ("color", TEXT), ("font-weight", "var(--font-weight-medium, 500)"), ("box-shadow", "0 1px 2px rgb(0 0 0 / 0.08)")]))
        .nest("& [data-ev-views] button:focus-visible", forms::focus_ring())
        .nest("& [data-ev-legend]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("gap", gap!(12)), ("color", MUTED), ("font-size", fs!(12))]))
        .nest("& [data-ev-legend] > span", pairs(&[("--tone", PRIMARY), ("display", "inline-flex"), ("align-items", "center"), ("gap", gap!(6))]))
        .nest("& [data-ev-legend] i", pairs(&[("width", "10px"), ("height", "10px"), ("border-radius", "3px"), ("background", "var(--tone)")]))
        .nest("& [data-ev-event]", chip)
        .nest("& [data-ev-event] time", pairs(&[("flex", "none"), ("opacity", "0.8"), ("font-variant-numeric", "tabular-nums")]))
        .nest("& [data-ev-event] > span", pairs(&[("overflow", "hidden"), ("text-overflow", "ellipsis")]))
        .nest("& [data-ev-event]:hover", pairs(&[("background", "color-mix(in srgb, var(--tone) 22%, var(--color-surface, #fff))")]))
        .nest("& [data-ev-event]:focus-visible, & [data-ev-more]:focus-visible", forms::focus_ring())
        .nest("& [data-ev-event][data-from-before]", pairs(&[("border-inline-start-color", "transparent"), ("border-top-left-radius", "0"), ("border-bottom-left-radius", "0")]))
        .nest("& [data-ev-event][data-on-after]", pairs(&[("border-top-right-radius", "0"), ("border-bottom-right-radius", "0")]))
        // Month: a grid of days.
        .nest("& [data-ev-month]", pairs(&[("display", "grid"), ("border", "var(--border-width, 1px) solid var(--color-border)"), ("border-radius", "var(--radius-card, var(--radius-lg))"), ("overflow", "hidden")]))
        .nest("& [data-ev-weekdays], & [data-ev-week]", pairs(&[("display", "grid"), ("grid-template-columns", "repeat(7, minmax(0, 1fr))")]))
        .nest("& [data-ev-weekdays] > div", pairs(&[("padding", pad!(6, 8)), ("color", MUTED), ("font-size", fs!(12)), ("font-weight", "var(--font-weight-medium, 500)"), ("border-bottom", "var(--border-width, 1px) solid var(--color-border)")]))
        .nest(
            "& [data-ev-week] > [data-ev-day]",
            pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(3)), ("min-height", sp!(96)), ("min-width", "0"), ("padding", pad!(4)), ("border-top", "1px solid var(--color-border)"), ("border-inline-start", "1px solid var(--color-border)"), ("outline", "none"), ("cursor", "pointer")]),
        )
        .nest("& [data-ev-week]:first-of-type > [data-ev-day]", pairs(&[]))
        .nest("& [data-ev-week] > [data-ev-day]:first-child", pairs(&[("border-inline-start", "0")]))
        .nest("& [data-ev-month] [data-ev-day][data-outside]", pairs(&[("background", "color-mix(in srgb, var(--color-text) 2.5%, transparent)")]))
        .nest("& [data-ev-month] [data-ev-day][data-outside] [data-ev-date]", pairs(&[("color", MUTED)]))
        .nest("& [data-ev-month] [data-ev-day]:hover", pairs(&[("background", ACCENT)]))
        .nest("& [data-ev-month] [data-ev-day]:focus-visible", pairs(&[("box-shadow", "inset 0 0 0 2px var(--color-ring, var(--color-primary))")]))
        .nest("& [data-ev-month] [data-ev-day][data-picked]", pairs(&[("background", "color-mix(in srgb, var(--color-primary) 7%, transparent)")]))
        .nest("& [data-ev-date]", pairs(&[("align-self", "flex-start"), ("display", "grid"), ("place-items", "center"), ("min-width", "24px"), ("height", "24px"), ("padding", pad!(0, 4)), ("box-sizing", "border-box"), ("border-radius", "999px"), ("font-size", fs!(12)), ("font-weight", "var(--font-weight-medium, 500)"), ("font-variant-numeric", "tabular-nums")]))
        .nest("& [data-today] > [data-ev-date]", pairs(&[("background", PRIMARY), ("color", "var(--color-on-primary, #fff)")]))
        .nest("& [data-ev-chips]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", "2px"), ("min-width", "0")]))
        .nest(
            "& [data-ev-more]",
            pairs(&[("align-self", "flex-start"), ("padding", pad!(0, 6)), ("border", "0"), ("border-radius", "4px"), ("background", "transparent"), ("color", MUTED), ("font", "inherit"), ("font-size", fs!(12)), ("font-weight", "var(--font-weight-medium, 500)"), ("cursor", "pointer")]),
        )
        .nest("& [data-ev-more]:hover", pairs(&[("background", ACCENT), ("color", TEXT)]))
        // Week: days across, hours down.
        .nest("& [data-ev-weekview]", pairs(&[("border", "var(--border-width, 1px) solid var(--color-border)"), ("border-radius", "var(--radius-card, var(--radius-lg))"), ("overflow", "hidden")]))
        .nest(
            "& :is([data-ev-week-head], [data-ev-allday], [data-ev-hours])",
            pairs(&[("display", "grid"), ("grid-template-columns", "56px repeat(var(--days, 7), minmax(0, 1fr))")]),
        )
        .nest(
            "& [data-ev-week-head] > button",
            pairs(&[("display", "flex"), ("flex-direction", "column"), ("align-items", "center"), ("gap", "2px"), ("padding", pad!(6, 4)), ("border", "0"), ("border-inline-start", "var(--border-width, 1px) solid var(--color-border)"), ("background", "transparent"), ("color", MUTED), ("font", "inherit"), ("font-size", fs!(12)), ("cursor", "pointer")]),
        )
        .nest("& [data-ev-week-head] b", pairs(&[("display", "grid"), ("place-items", "center"), ("min-width", "28px"), ("height", "28px"), ("border-radius", "999px"), ("color", TEXT), ("font-size", fs!(15)), ("font-weight", "var(--font-weight-semibold, 600)")]))
        .nest("& [data-ev-week-head] [data-today] b", pairs(&[("background", PRIMARY), ("color", "var(--color-on-primary, #fff)")]))
        .nest("& [data-ev-week-head] > button:focus-visible", pairs(&[("outline", "none"), ("box-shadow", "inset 0 0 0 2px var(--color-ring, var(--color-primary))")]))
        .nest("& [data-ev-allday]", pairs(&[("border-top", "var(--border-width, 1px) solid var(--color-border)"), ("border-bottom", "var(--border-width, 1px) solid var(--color-border)")]))
        .nest("& [data-ev-allday-cell]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", "2px"), ("min-width", "0"), ("padding", pad!(3)), ("border-inline-start", "var(--border-width, 1px) solid var(--color-border)"), ("min-height", "26px")]))
        .nest("& [data-ev-hour]", pairs(&[("padding", pad!(2, 6)), ("color", MUTED), ("font-size", fs!(11)), ("text-align", "end"), ("white-space", "nowrap")]))
        .nest("& [data-ev-hours]", pairs(&[("height", "calc(var(--hours) * 44px)"), ("max-height", "560px"), ("overflow-y", "auto"), ("position", "relative")]))
        .nest("& [data-ev-labels] [data-ev-hour]", pairs(&[("height", "44px"), ("box-sizing", "border-box"), ("transform", "translateY(-7px)")]))
        .nest("& [data-ev-labels] [data-ev-hour]:first-child", pairs(&[("visibility", "hidden")]))
        .nest(
            "& [data-ev-col]",
            pairs(&[
                ("position", "relative"),
                ("height", "calc(var(--hours) * 44px)"),
                ("border-inline-start", "var(--border-width, 1px) solid var(--color-border)"),
                ("background", "repeating-linear-gradient(to bottom, transparent 0 43px, var(--color-border) 43px 44px)"),
            ]),
        )
        .nest("& [data-ev-col][data-today]", pairs(&[("background-color", "color-mix(in srgb, var(--color-primary) 4%, transparent)")]))
        .nest("& [data-ev-timed]", pairs(&[("position", "absolute"), ("flex-direction", "column"), ("align-items", "stretch"), ("gap", "0"), ("padding", pad!(2, 6)), ("white-space", "normal"), ("z-index", "1")]))
        .nest("& [data-ev-timed] > span", pairs(&[("font-weight", "var(--font-weight-medium, 500)")]))
        // Movable events: grabbed, dragged, and resized by the strip at a timed one's foot.
        .nest("&[data-editable] [data-ev-event]:not([data-ev-list] *)", pairs(&[("cursor", "grab"), ("touch-action", "none")]))
        .nest("& [data-ev-event][data-dragging]", pairs(&[("z-index", "5"), ("cursor", "grabbing"), ("opacity", ".9"), ("box-shadow", "0 6px 16px rgb(0 0 0 / .18)"), ("pointer-events", "none")]))
        .nest("& [data-ev-resize]", pairs(&[("position", "absolute"), ("inset-inline-start", "0"), ("inset-inline-end", "0"), ("bottom", "0"), ("height", "7px"), ("cursor", "ns-resize")]))
        .nest("&[data-editable] :is([data-ev-month] [data-ev-day], [data-ev-cell-date])[data-ev-drop]", pairs(&[("outline", "2px dashed var(--color-ring, var(--color-muted))"), ("outline-offset", "-2px")]))
        .nest("& [data-ev-timed][data-short]", pairs(&[("flex-direction", "row"), ("align-items", "center"), ("gap", gap!(4)), ("padding-top", "0"), ("padding-bottom", "0"), ("white-space", "nowrap")]))
        .nest("& [data-ev-now]", pairs(&[("position", "absolute"), ("inset-inline-start", "0"), ("inset-inline-end", "0"), ("height", "2px"), ("background", "var(--color-destructive, #dc2626)"), ("z-index", "2")]))
        .nest("& [data-ev-now]::before", pairs(&[("content", "''"), ("position", "absolute"), ("inset-inline-start", "-4px"), ("top", "-3px"), ("width", "8px"), ("height", "8px"), ("border-radius", "50%"), ("background", "var(--color-destructive, #dc2626)")]))
        // Year: twelve small months.
        .nest("& [data-ev-year]", pairs(&[("display", "grid"), ("grid-template-columns", "repeat(auto-fill, minmax(196px, 1fr))"), ("gap", gap!(20, 24))]))
        .nest("& [data-ev-mini]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(4))]))
        .nest(
            "& [data-ev-month-open]",
            pairs(&[("align-self", "flex-start"), ("padding", pad!(2, 6)), ("border", "0"), ("border-radius", "var(--radius-sm)"), ("background", "transparent"), ("color", TEXT), ("font", "inherit"), ("font-size", fs!(14)), ("font-weight", "var(--font-weight-semibold, 600)"), ("cursor", "pointer")]),
        )
        .nest("& [data-ev-month-open]:hover", pairs(&[("background", ACCENT)]))
        .nest("& [data-ev-month-open]:focus-visible", forms::focus_ring())
        .nest("& [data-ev-mini] [role=row]", pairs(&[("display", "grid"), ("grid-template-columns", "repeat(7, 1fr)")]))
        .nest("& [data-ev-mini-head] span", pairs(&[("color", MUTED), ("font-size", fs!(11)), ("text-align", "center"), ("padding", "2px 0")]))
        .nest(
            "& [data-ev-mini] [data-ev-day]",
            pairs(&[
                ("--tone", PRIMARY),
                ("position", "relative"),
                ("display", "grid"),
                ("place-items", "center"),
                ("height", "26px"),
                ("border-radius", "999px"),
                ("font-size", fs!(12)),
                ("font-variant-numeric", "tabular-nums"),
                ("cursor", "pointer"),
                ("outline", "none"),
            ]),
        )
        .nest("& [data-ev-mini] [data-ev-day]:hover", pairs(&[("background", ACCENT)]))
        .nest("& [data-ev-mini] [data-ev-day]:focus-visible", pairs(&[("box-shadow", "inset 0 0 0 2px var(--color-ring, var(--color-primary))")]))
        .nest("& [data-ev-mini] [data-busy]", pairs(&[("background", "color-mix(in srgb, var(--tone) 16%, transparent)"), ("color", "color-mix(in srgb, var(--tone) 80%, var(--color-text))"), ("font-weight", "var(--font-weight-semibold, 600)")]))
        .nest("& [data-ev-mini] [data-busy]::after", pairs(&[("content", "''"), ("position", "absolute"), ("bottom", "2px"), ("width", "4px"), ("height", "4px"), ("border-radius", "50%"), ("background", "var(--tone)")]))
        .nest("& [data-ev-mini] [data-today]", pairs(&[("background", PRIMARY), ("color", "var(--color-on-primary, #fff)")]))
        // List: days with their events.
        .nest("& [data-ev-list]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("border", "var(--border-width, 1px) solid var(--color-border)"), ("border-radius", "var(--radius-card, var(--radius-lg))"), ("overflow", "hidden")]))
        .nest("& [data-ev-group]", pairs(&[("display", "grid"), ("grid-template-columns", "140px minmax(0, 1fr)"), ("border-top", "var(--border-width, 1px) solid var(--color-border)")]))
        .nest("& [data-ev-group]:first-child", pairs(&[("border-top", "0")]))
        .nest("& [data-ev-group] h4", pairs(&[("display", "flex"), ("align-items", "baseline"), ("gap", gap!(8)), ("margin", "0"), ("padding", pad!(12)), ("font-size", fs!(13)), ("font-weight", "400"), ("color", MUTED)]))
        .nest("& [data-ev-group] h4 b", pairs(&[("color", TEXT), ("font-size", fs!(18)), ("font-weight", "var(--font-weight-semibold, 600)"), ("font-variant-numeric", "tabular-nums")]))
        .nest("& [data-ev-group][data-today] h4 b", pairs(&[("color", PRIMARY)]))
        .nest("& [data-ev-group] ul", pairs(&[("margin", "0"), ("padding", pad!(6)), ("list-style", "none"), ("display", "flex"), ("flex-direction", "column"), ("gap", "2px")]))
        .nest(
            "& [data-ev-list] [data-ev-event]",
            pairs(&[("display", "grid"), ("grid-template-columns", "130px 10px minmax(0, 1fr) auto"), ("align-items", "center"), ("gap", gap!(10)), ("padding", pad!(8, 10)), ("border-inline-start", "0"), ("background", "transparent"), ("color", TEXT), ("font-size", fs!(14))]),
        )
        .nest("& [data-ev-list] [data-ev-event]:hover", pairs(&[("background", ACCENT)]))
        .nest("& [data-ev-list] [data-ev-event] time", pairs(&[("color", MUTED), ("font-size", fs!(13))]))
        .nest("& [data-ev-dot]", pairs(&[("width", "10px"), ("height", "10px"), ("border-radius", "50%"), ("background", "var(--tone)")]))
        .nest("& [data-ev-list] small", pairs(&[("color", MUTED), ("font-size", fs!(12))]))
        .nest("& [data-ev-empty]", pairs(&[("margin", "0"), ("padding", pad!(24)), ("color", MUTED), ("text-align", "center")]))
        .nest("@media (max-width: 640px)", pairs(&[])
            .nest("& [data-ev-week] > [data-ev-day]", pairs(&[("min-height", "64px")]))
            .nest("& [data-ev-month] [data-ev-event] > :is(span, time)", pairs(&[("display", "none")]))
            .nest("& [data-ev-month] [data-ev-event]", pairs(&[("height", "6px"), ("padding", "0"), ("border-inline-start-width", "0"), ("background", "var(--tone)")]))
            .nest("& [data-ev-group]", pairs(&[("grid-template-columns", "1fr")]))
            .nest("& [data-ev-list] [data-ev-event]", pairs(&[("grid-template-columns", "90px 10px minmax(0, 1fr)")]))
            .nest("& [data-ev-list] small", pairs(&[("display", "none")])))
        .nest(
            "& [data-ev-live]",
            pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("overflow", "hidden"), ("clip-path", "inset(50%)"), ("white-space", "nowrap")]),
        );
    tone_rules(tone_rules(tone_rules(b, "& [data-ev-event]"), "& [data-ev-legend] > span"), "& [data-ev-mini] [data-ev-day]")
});

const CHEVRON_LEFT: &str = r#"<svg data-flip width="16" height="16" style="width:calc(16px * var(--size-icon, 1));height:calc(16px * var(--size-icon, 1))" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M10 3.5 5.5 8 10 12.5"/></svg>"#;
const CHEVRON_RIGHT: &str = r#"<svg data-flip width="16" height="16" style="width:calc(16px * var(--size-icon, 1));height:calc(16px * var(--size-icon, 1))" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M6 3.5 10.5 8 6 12.5"/></svg>"#;

impl Component for EventCalendar {
    fn name(&self) -> &'static str {
        "ui-event-calendar"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Events in a month grid, a week of hours, or a list by day, coloured by category, with a toolbar to move through \
                  time. Clicks go to `bindEvent` (an event) and `bindDay` (a day, e.g. to add one).",
            props: PROPS,
            parts: &["toolbar"],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        crate::flips(cx);
        let views: Vec<String> = match cx.prop(node, "views") {
            None | Some(Json::Null) => VIEWS.iter().map(|v| v.to_string()).collect(),
            Some(Json::Array(vs)) if !vs.is_empty() => {
                let vs: Vec<String> = vs.iter().map(expr::stringify).collect();
                if let Some(bad) = vs.iter().find(|v| !VIEWS.contains(&v.as_str())) {
                    return Err(RenderError::new(node, format!("`views` has `{bad}`; the views are day, week, month, year and list")));
                }
                vs
            }
            Some(_) => return Err(RenderError::new(node, "`views` must be a list: day, week, month, year, list")),
        };
        // Month to start with when it's offered, else the first.
        let view = cx.prop_string(node, "view").unwrap_or_else(|| if views.iter().any(|v| v == "month") { "month".into() } else { views[0].clone() });
        if !views.contains(&view) {
            return Err(RenderError::new(node, format!("`view` is `{view}`, which `views` leaves out")));
        }
        let hours = match cx.prop(node, "hours") {
            None | Some(Json::Null) => json!([7, 20]),
            Some(Json::Array(h)) if h.len() == 2 => {
                let (a, b) = (expr::to_num(&h[0]), expr::to_num(&h[1]));
                if !(0.0..24.0).contains(&a) || !(a + 1.0..=24.0).contains(&b) || a.fract() != 0.0 || b.fract() != 0.0 {
                    return Err(RenderError::new(node, "`hours` must be `[from, to]`, whole hours from 0 to 24, `from` first"));
                }
                json!([a as i64, b as i64])
            }
            Some(_) => return Err(RenderError::new(node, "`hours` must be `[from, to]`, e.g. `[8, 18]`")),
        };
        let week_start = cx.prop(node, "weekStart").map_or(1.0, |v| expr::to_num(&v));
        if week_start != 0.0 && week_start != 1.0 {
            return Err(RenderError::new(node, "`weekStart` must be 0 (Sunday) or 1 (Monday)"));
        }
        let categories = match node.prop("categories") {
            Some(c) => dynamic(cx, node, "categories", c)?,
            None => Json::Null,
        };
        cx.global_js("ui-event-calendar", SCRIPT);
        let config = json!({
            "events": dynamic(cx, node, "events", node.prop("events").expect("required"))?,
            "categories": categories,
            "view": view,
            "views": views,
            "date": node.prop("date").map(|d| dynamic(cx, node, "date", d)).transpose()?,
            "bindEvent": cx.prop_string(node, "bindEvent"),
            "bindEvents": cx.prop_string(node, "bindEvents"),
            "snap": cx.prop(node, "snap").map(|v| expr::to_num(&v).clamp(1.0, 240.0) as i64),
            "bindDay": cx.prop_string(node, "bindDay"),
            "bindView": cx.prop_string(node, "bindView"),
            "hours": hours,
            "weekStart": week_start as i64,
            "maxPerDay": cx.prop(node, "maxPerDay").map(|v| expr::to_num(&v).max(1.0) as i64),
            "today": cx.prop_string(node, "today"),
            "locale": crate::locale(cx, node),
            "emptyText": node.prop("emptyText").map(|v| expr::stringify(&cx.eval(v))),
        });
        let mut root = cx.element("div", node);
        if cx.prop_string(node, "bindEvents").is_some() {
            root.attr("data-editable", true);
        }
        root.base_style_ref(&CALENDAR)
            .behavior("ui-event-calendar", config)
            .attr("role", "region")
            .attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Calendar"))));
        cx.open(&root, node)?;

        let mut bar = cx.part("div", node, "toolbar");
        bar.attr("data-ev-bar", true);
        cx.open(&bar, node)?;
        // Labels: "Previous", "Next"
        for (attr, label, icon) in [("data-ev-prev", "Previous", CHEVRON_LEFT), ("data-ev-next", "Next", CHEVRON_RIGHT)] {
            let mut b = Element::new("button");
            forms::button_look(&mut b, "outline", "sm");
            b.attr("type", "button").attr(attr, true).attr("aria-label", cx.label(label));
            cx.open(&b, node)?;
            cx.markup(icon);
            cx.close("button");
        }
        let mut today = Element::new("button");
        forms::button_look(&mut today, "outline", "sm");
        today.attr("type", "button").attr("data-ev-today", true).text(lit(cx.label("Today")));
        cx.open(&today, node)?;
        cx.close("button");
        let mut title = Element::new("h3");
        title.attr("data-ev-title", true).attr("aria-live", "polite");
        cx.open(&title, node)?;
        cx.close("h3");
        if views.len() > 1 {
            let mut group = Element::new("div");
            group.attr("data-ev-views", true).attr("role", "group").attr("aria-label", cx.label("View"));
            cx.open(&group, node)?;
            for v in &views {
                let mut b = Element::new("button");
                // Labels: "Day", "Week", "Month", "Year", "List"
                let label = cx.label(&(v[..1].to_uppercase() + &v[1..]));
                b.attr("type", "button").attr("data-ev-view", v.clone()).attr("aria-pressed", if *v == view { "true" } else { "false" }).text(lit(label));
                cx.open(&b, node)?;
                cx.close("button");
            }
            cx.close("div");
        }
        cx.close("div");
        if node.prop("categories").is_some() {
            let mut legend = Element::new("div");
            legend.attr("data-ev-legend", true).attr("aria-label", cx.label("Categories"));
            cx.open(&legend, node)?;
            cx.close("div");
        }
        let mut body = Element::new("div");
        body.attr("data-ev-body", true).attr("data-view", view);
        cx.open(&body, node)?;
        cx.close("div");
        let mut live = Element::new("div");
        live.attr("data-ev-live", true).attr("role", "status").attr("aria-live", "polite");
        cx.open(&live, node)?;
        cx.close("div");
        cx.close("div");
        Ok(())
    }
}
