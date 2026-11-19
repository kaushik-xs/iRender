//! `ui-calendar` and `ui-date-picker`: a calendar grid, inline or in a popover behind
//! a button, for a day (with a time if asked), a month, a quarter or a year, one, a
//! range or several, with limits, disabled days, presets and financial years.
//!
//! The server renders the frame (the picker's button with the value as it stands,
//! the popover, hidden form inputs); `calendar.js` (shipped once per page through
//! `global_js`) draws the grid and handles the mouse and keyboard, following the
//! WAI-ARIA date picker dialog pattern.

use crate::{BG_CARD, BG_POPOVER, MUTED, PRIMARY, RING, SUBTLE, SURFACE, TEXT, forms, id_with, instance_id, lit};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, Template, Val, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("calendar.js");

pub(crate) const PICKERS: &[&str] = &["day", "month", "quarter", "year"];
pub(crate) const MODES: &[&str] = &["single", "range", "multiple"];

/// Props both components take.
macro_rules! calendar_props {
    ($($extra:expr),* $(,)?) => {
        &[
            PropDef::new("bind", PropKind::StateKey, "State key holding the value: `2026-03-01` (with `time`, `2026-03-01T14:30`; `2026-03`, `2026-Q1` or `2026` for the other pickers), `{ from, to }` with `mode: range`, a list with `multiple`"),
            PropDef::new("value", PropKind::Any, "Starting value, when not bound"),
            PropDef::new("picker", PropKind::Enum(PICKERS), "What is picked: `day` (default), `month`, `quarter` or `year`"),
            PropDef::new("mode", PropKind::Enum(MODES), "`single` (default), `range` or `multiple`"),
            PropDef::new("time", PropKind::Bool, "A time as well as the day (single days only)"),
            PropDef::new("step", PropKind::Number, "With `time`, minutes between times (default 1)"),
            PropDef::new("min", PropKind::Any, "Earliest date: `2026-01-01`, or `today`, `today-30d`, `today+1y`; may use state"),
            PropDef::new("max", PropKind::Any, "Latest date, as `min`"),
            PropDef::new("disabledDates", PropKind::Any, "Dates that can't be picked: `2026-12-25`, or `{ from, to }`; may use state"),
            PropDef::new("disabledDays", PropKind::Any, "Weekdays that can't be picked: 0 Sunday … 6 Saturday, e.g. `[0, 6]`"),
            PropDef::new("months", PropKind::Number, "Months shown side by side (default 1; 2 suits ranges)"),
            PropDef::new("weekStart", PropKind::Number, "First day of the week, 0 Sunday … 6 Saturday (default: the locale's)"),
            PropDef::new("fiscalStart", PropKind::Number, "Month a financial year starts, 1–12 (default 1); quarters and years follow it"),
            PropDef::new("presets", PropKind::Any, "Shortcuts beside the calendar: names (`last7`, `thisMonth`, `lastQuarter`, `yearToDate`, …) or `{ label, from, to }`"),
            PropDef::new("dropdowns", PropKind::Bool, "Month and year menus in place of the caption, for dates far away such as birthdays"),
            PropDef::new("bindRange", PropKind::StateKey, "State key that gets the dates the value covers, `{ from, to }`: a chart's or a table's filter"),
            PropDef::new("name", PropKind::String, "Form field name: hidden inputs carry the value (`name[from]` and `name[to]` for a range)"),
            PropDef::new("locale", PropKind::String, "Locale for names and formats (default: the document's `lang`, else the browser's)"),
            PropDef::new("label", PropKind::Any, "Accessible name"),
            PropDef::new("id", PropKind::Any, "Element id (default: derived from the document)"),
            $($extra),*
        ]
    };
}

const CALENDAR_PROPS: &[PropDef] = calendar_props![PropDef::new("clearable", PropKind::Bool, "A Clear button under the calendar")];

const PICKER_PROPS: &[PropDef] = calendar_props![
    PropDef::new("input", PropKind::Bool, "A text field to type the value in (`15/03/2026`, `Mar 15`, `today`), with a button beside it for the calendar"),
    PropDef::new("bindError", PropKind::StateKey, "With `input`, a state key that gets why the typed text isn't a value, or null: a `ui-field`'s `error`"),
    PropDef::new("placeholder", PropKind::Any, "Shown while nothing is picked (default `Pick a date`, `Pick a range`, …)"),
    PropDef::new("clearable", PropKind::Bool, "A button in the field that clears the value"),
    PropDef::new("closeOnSelect", PropKind::Bool, "Close once a value is complete (default true; with `time` or `multiple`, false)"),
    PropDef::new("align", PropKind::Enum(&["start", "end"]), "The popover's edge lined up with the button (default `start`)"),
    PropDef::new("disabled", PropKind::Any, "Boolean or expression"),
];

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

const TINT: &str = "color-mix(in srgb, var(--color-primary) 12%, var(--ui-bg, var(--color-background, var(--color-surface))))";

/// The calendar itself, inline or in the popover. Everything is styled through
/// attributes, because the behaviour draws it.
fn calendar_style() -> StyleBlock {
    pairs(&[("color", TEXT), ("font-size", fs!(14))])
        .nest("& [data-cal-wrap]", pairs(&[("display", "flex"), ("gap", sp!(12)), ("align-items", "flex-start")]))
        .nest(
            "& [data-cal-presets]",
            pairs(&[
                ("display", "flex"),
                ("flex-direction", "column"),
                ("gap", sp!(2)),
                ("min-width", sp!(132)),
                ("padding-inline-end", sp!(12)),
                ("border-inline-end", "1px solid var(--color-border)"),
            ]),
        )
        .nest(
            "& [data-cal-presets] button",
            pairs(&[
                ("padding", sp!(6, 8)),
                ("border", "0"),
                ("border-radius", "var(--radius-sm)"),
                ("background", "none"),
                ("color", "inherit"),
                ("font", "inherit"),
                ("text-align", "start"),
                ("white-space", "nowrap"),
                ("cursor", "pointer"),
            ]),
        )
        .nest("& [data-cal-presets] button:hover", pairs(&[("background", SUBTLE)]))
        .nest("& [data-cal-presets] button[aria-pressed=true]", pairs(&[("background", TINT), ("font-weight", "500")]))
        .nest("& [data-cal-main]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", sp!(8))]))
        .nest("& [data-cal-head]", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", sp!(8))]))
        .nest("& [data-cal-captions]", pairs(&[("display", "flex"), ("flex", "1"), ("justify-content", "space-around"), ("gap", sp!(24))]))
        .nest("& [data-cal-caption]", pairs(&[("font-weight", "500"), ("display", "flex"), ("gap", sp!(4))]))
        .nest(
            "& [data-cal-caption] select",
            pairs(&[
                ("height", sp!(28)),
                ("padding", sp!(0, 4)),
                ("border", "1px solid var(--color-border)"),
                ("border-radius", "var(--radius-sm)"),
                ("background", SURFACE),
                ("color", TEXT),
                ("font", "inherit"),
                ("font-weight", "500"),
            ]),
        )
        .nest(
            "& [data-cal-head] > button",
            pairs(&[
                ("display", "inline-flex"),
                ("align-items", "center"),
                ("justify-content", "center"),
                ("width", sp!(28)),
                ("height", sp!(28)),
                ("padding", "0"),
                ("border", "1px solid var(--color-border)"),
                ("border-radius", "var(--radius-sm)"),
                ("background", "transparent"),
                ("color", "inherit"),
                ("cursor", "pointer"),
            ]),
        )
        .nest("& [data-cal-head] > button:hover", pairs(&[("background", SUBTLE)]))
        .nest("& [data-cal-head] > button:disabled", pairs(&[("opacity", "0.4"), ("cursor", "default")]))
        .nest("& [data-cal-grids]", pairs(&[("display", "flex"), ("gap", sp!(24)), ("flex-wrap", "wrap")]))
        .nest("& [data-cal-grids] table", pairs(&[("border-collapse", "separate"), ("border-spacing", sp!(0, 2))]))
        .nest(
            "& [data-cal-grids] th",
            pairs(&[("width", sp!(36)), ("padding", sp!(0, 0, 4)), ("color", MUTED), ("font-size", fs!(12)), ("font-weight", "400")]),
        )
        .nest("& [data-cal-grids] td", pairs(&[("padding", "0"), ("text-align", "center")]))
        .nest(
            "& [data-cal-grids] td > button",
            pairs(&[
                ("width", sp!(36)),
                ("height", sp!(36)),
                ("padding", "0"),
                ("border", "0"),
                ("border-radius", "var(--radius-md)"),
                ("background", "transparent"),
                ("color", "inherit"),
                ("font", "inherit"),
                ("font-variant-numeric", "tabular-nums"),
                ("cursor", "pointer"),
            ]),
        )
        // Months, quarters and years are wider cells.
        .nest("&[data-picker=month] td > button, &[data-picker=year] td > button", pairs(&[("width", sp!(84))]))
        .nest("&[data-picker=quarter] td > button", pairs(&[("width", sp!(64))]))
        .nest("& [data-cal-grids] td > button:hover", pairs(&[("background", SUBTLE)]))
        .nest(
            "& [data-cal-grids] td > button:focus-visible",
            pairs(&[("outline", "2px solid var(--color-ring, var(--color-muted))"), ("outline-offset", "-2px")]),
        )
        .nest("& [data-cal-grids] td > button[data-outside]", pairs(&[("color", MUTED), ("opacity", "0.6")]))
        .nest("& [data-cal-grids] td > button[data-today]", pairs(&[("background", SUBTLE), ("font-weight", "600")]))
        .nest("& [data-cal-grids] td > button[data-in-range]", pairs(&[("background", TINT), ("border-radius", "0")]))
        .nest(
            "& [data-cal-grids] td > button[data-range-end], & [data-cal-grids] td > button[data-range-start]",
            pairs(&[("background", "color-mix(in srgb, var(--color-primary) 30%, var(--ui-bg, var(--color-background, var(--color-surface))))")]),
        )
        .nest(
            "& [data-cal-grids] td > button[aria-pressed=true]",
            pairs(&[("background", PRIMARY), ("color", "var(--color-on-primary)"), ("font-weight", "500")]),
        )
        .nest("& [data-cal-grids] td > button[aria-pressed=true]:hover", pairs(&[("background", "var(--color-primary-hover, var(--color-primary))")]))
        .nest(
            "& [data-cal-grids] td > button[aria-disabled=true]",
            pairs(&[("opacity", "0.35"), ("cursor", "not-allowed"), ("text-decoration", "line-through"), ("background", "transparent")]),
        )
        .nest("& [data-cal-time]", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", sp!(8)), ("color", MUTED)]))
        .nest(
            "& [data-cal-time] input",
            pairs(&[
                ("height", sp!(32)),
                ("padding", sp!(0, 8)),
                ("border", "1px solid var(--color-border)"),
                ("border-radius", "var(--radius-md)"),
                ("background", "transparent"),
                ("color", TEXT),
                ("font", "inherit"),
            ]),
        )
        .nest("& [data-cal-time] input:focus-visible", pairs(&[("outline", "none"), ("border-color", RING)]))
        .nest("& [data-cal-foot]", pairs(&[("display", "flex"), ("justify-content", "space-between"), ("gap", sp!(8))]))
        .nest("& [data-cal-foot]:empty", pairs(&[("display", "none")]))
        .nest(
            "& [data-cal-clear]",
            pairs(&[("padding", "0"), ("border", "0"), ("background", "none"), ("color", PRIMARY), ("font", "inherit"), ("cursor", "pointer")]),
        )
        .nest(
            "& [data-cal-live]",
            pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("overflow", "hidden"), ("clip-path", "inset(50%)"), ("white-space", "nowrap")]),
        )
}

static CALENDAR: LazyLock<StyleBlock> = LazyLock::new(|| {
    calendar_style().set("display", "inline-block").set("width", "fit-content").set("position", "relative").set("padding", sp!(12)).set("border", "1px solid var(--color-border)").set("border-radius", "var(--radius-md)").set("--ui-bg", BG_CARD).set("background", BG_CARD)
});

static PICKER: LazyLock<StyleBlock> = LazyLock::new(|| {
    calendar_style()
        .set("position", "relative")
        .set("display", "inline-flex")
        .set("width", "100%")
        .set("max-width", "100%")
        .nest(
            "& [data-cal-trigger]",
            pairs(&[("width", "100%"), ("justify-content", "flex-start"), ("font-weight", "400"), ("text-align", "start"), ("padding-inline-end", sp!(32))]),
        )
        .nest("& [data-cal-trigger] svg", pairs(&[("flex-shrink", "0"), ("color", MUTED)]))
        .nest("& [data-cal-text]", pairs(&[("overflow", "hidden"), ("text-overflow", "ellipsis"), ("white-space", "nowrap")]))
        .nest("&[data-empty] [data-cal-text]", pairs(&[("color", MUTED)]))
        .nest(
            "& [data-cal-x]",
            pairs(&[
                ("position", "absolute"),
                ("inset-inline-end", sp!(6)),
                ("top", "50%"),
                ("transform", "translateY(-50%)"),
                ("display", "inline-flex"),
                ("align-items", "center"),
                ("justify-content", "center"),
                ("width", "22px"),
                ("height", "22px"),
                ("padding", "0"),
                ("border", "0"),
                ("border-radius", "var(--radius-sm)"),
                ("background", "transparent"),
                ("color", MUTED),
                ("font-size", fs!(11)),
                ("cursor", "pointer"),
            ]),
        )
        .nest("& [data-cal-x]:hover", pairs(&[("background", SUBTLE), ("color", TEXT)]))
        .nest("& [data-cal-x][hidden]", pairs(&[("display", "none")]))
        .nest(
            "& [data-cal-pop]",
            pairs(&[
                ("box-sizing", "border-box"),
                ("inset", "auto"),
                ("margin", "6px 0"),
                ("padding", sp!(12)),
                ("border", "1px solid var(--color-border)"),
                ("border-radius", "var(--radius-md)"),
                ("--ui-bg", BG_POPOVER),
                ("background", BG_POPOVER),
                ("color", TEXT),
                ("box-shadow", "0 6px 18px rgb(0 0 0 / 0.12)"),
                ("position-try-fallbacks", "flip-block, flip-inline"),
            ]),
        )
});

/// The text field and the calendar's button in one border, like `ui-input-group`.
static INPUT_GROUP: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("box-sizing", "border-box"),
        ("display", "flex"),
        ("align-items", "center"),
        ("width", "100%"),
        ("height", sp!(36)),
        ("border", "1px solid var(--color-border)"),
        ("border-radius", "var(--radius-md)"),
        ("background", "transparent"),
        ("box-shadow", "0 1px 2px rgb(0 0 0 / 0.05)"),
        ("transition", "border-color .15s, box-shadow .15s"),
    ])
    .nest(
        "&:focus-within",
        pairs(&[("border-color", RING), ("box-shadow", "0 0 0 3px color-mix(in srgb, var(--color-ring, var(--color-muted)) 50%, transparent)")]),
    )
    .nest(
        "&:has([aria-invalid=true])",
        pairs(&[("border-color", "var(--color-destructive, #dc2626)")]),
    )
    .nest(
        "& [data-cal-input]",
        pairs(&[
            ("flex", "1"),
            ("min-width", "0"),
            ("height", "100%"),
            ("padding", sp!(0, 12)),
            ("border", "0"),
            ("outline", "none"),
            ("background", "transparent"),
            ("color", TEXT),
            ("font", "inherit"),
            ("font-size", fs!(14)),
        ]),
    )
    .nest("& [data-cal-input]::placeholder", pairs(&[("color", MUTED)]))
    .nest("& [data-cal-x]", pairs(&[("position", "static"), ("transform", "none")]))
    .nest(
        "& [data-cal-trigger]",
        pairs(&[
            ("display", "inline-flex"),
            ("align-items", "center"),
            ("justify-content", "center"),
            ("width", sp!(32)),
            ("height", sp!(28)),
            ("margin-inline-end", sp!(3)),
            ("padding", "0"),
            ("border", "0"),
            ("border-radius", "var(--radius-sm)"),
            ("background", "transparent"),
            ("color", MUTED),
            ("cursor", "pointer"),
        ]),
    )
    .nest("& [data-cal-trigger]:hover", pairs(&[("background", SUBTLE), ("color", TEXT)]))
    .nest("& [data-cal-trigger]:focus-visible", pairs(&[("outline", "2px solid var(--color-ring, var(--color-muted))")]))
});

const ICON: &str = r#"<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="2" y="3" width="12" height="11" rx="1.5"/><path d="M2 6.5h12M5.5 1.5v3M10.5 1.5v3"/></svg>"#;

/// A prop for the behaviour: templates that read state (anywhere inside) left for the
/// browser as `{ "$irender": js }`, the rest evaluated now.
fn dynamic(cx: &RenderCx<'_>, node: &Node, name: &str, v: &Val) -> Result<Json, RenderError> {
    match v {
        Val::Tpl(_) if cx.is_reactive(v) => Ok(json!({ "$irender": cx.val_js(v) })),
        Val::Tpl(_) => Ok(cx.eval(v)),
        Val::Lit(Json::String(s)) if Template::is_template(s) => {
            let t = Template::parse(s).map_err(|e| RenderError::new(node, format!("`{name}`: {e}")))?;
            dynamic(cx, node, name, &Val::Tpl(t))
        }
        Val::Lit(Json::Array(items)) => {
            Ok(Json::Array(items.iter().map(|i| dynamic(cx, node, name, &Val::Lit(i.clone()))).collect::<Result<_, _>>()?))
        }
        Val::Lit(Json::Object(o)) => Ok(Json::Object(
            o.iter().map(|(k, v)| Ok((k.clone(), dynamic(cx, node, name, &Val::Lit(v.clone()))?))).collect::<Result<_, RenderError>>()?,
        )),
        Val::Lit(other) => Ok(other.clone()),
    }
}

/// The names `presets` may use.
pub(crate) const PRESETS: &[&str] = &[
    "today", "yesterday", "tomorrow", "last7", "last30", "last90", "next7", "next30", "thisWeek", "lastWeek", "thisMonth",
    "lastMonth", "nextMonth", "thisQuarter", "lastQuarter", "thisYear", "lastYear", "yearToDate", "monthToDate", "quarters",
];

/// What both components pass the behaviour, after checking the props.
fn config(cx: &RenderCx<'_>, node: &Node, picker: bool) -> Result<Json, RenderError> {
    let s = |name: &str| cx.prop_string(node, name).filter(|v| !v.is_empty());
    let flag = |name: &str| cx.prop(node, name).filter(|v| !v.is_null()).map(|v| expr::truthy(&v));
    let num = |name: &str| cx.prop(node, name).filter(|v| !v.is_null()).map(|v| expr::to_num(&v));
    let kind = s("picker").unwrap_or_else(|| "day".into());
    let mode = s("mode").unwrap_or_else(|| "single".into());
    if !PICKERS.contains(&kind.as_str()) {
        return Err(RenderError::new(node, format!("`picker` must be one of {}, got `{kind}`", PICKERS.join(", "))));
    }
    if !MODES.contains(&mode.as_str()) {
        return Err(RenderError::new(node, format!("`mode` must be one of {}, got `{mode}`", MODES.join(", "))));
    }
    let time = flag("time").unwrap_or(false);
    if picker && flag("input").unwrap_or(false) && mode == "multiple" {
        return Err(RenderError::new(node, "`input` takes one value or a range, not `multiple`"));
    }
    if time && (kind != "day" || mode != "single") {
        return Err(RenderError::new(node, "`time` goes with single days (`picker: day`, `mode: single`)"));
    }
    let fiscal = num("fiscalStart").unwrap_or(1.0);
    if !(1.0..=12.0).contains(&fiscal) || fiscal.fract() != 0.0 {
        return Err(RenderError::new(node, "`fiscalStart` must be a month, 1–12"));
    }
    if let Some(w) = num("weekStart")
        && (!(0.0..=6.0).contains(&w) || w.fract() != 0.0)
    {
        return Err(RenderError::new(node, "`weekStart` must be 0 (Sunday) to 6 (Saturday)"));
    }
    let presets = match cx.prop(node, "presets") {
        None | Some(Json::Null) => Json::Array(Vec::new()),
        Some(Json::Array(list)) => {
            for p in &list {
                match p {
                    Json::String(n) if !PRESETS.contains(&n.as_str()) => {
                        return Err(RenderError::new(node, format!("unknown preset `{n}`; use one of {}, or `{{ label, from, to }}`", PRESETS.join(", "))));
                    }
                    Json::String(_) => {}
                    Json::Object(o) if o.contains_key("label") && o.contains_key("from") => {}
                    _ => return Err(RenderError::new(node, "each preset is a name or `{ label, from, to }`")),
                }
            }
            // `quarters`: the quarter today is in and the three before it, each its own.
            Json::Array(list.into_iter().flat_map(|p| if p == json!("quarters") { (0..4).map(|n| json!(format!("quarter{n}"))).collect() } else { vec![p] }).collect())
        }
        Some(_) => return Err(RenderError::new(node, "`presets` must be a list")),
    };
    let months = num("months").unwrap_or(1.0).clamp(1.0, 3.0) as u8;
    let opt = |name: &str| node.prop(name).map(|v| dynamic(cx, node, name, v)).transpose();
    Ok(json!({
        "bind": s("bind"),
        "bindRange": s("bindRange"),
        "value": if s("bind").is_none() { cx.prop(node, "value").unwrap_or(Json::Null) } else { Json::Null },
        "picker": kind,
        "mode": mode,
        "time": time,
        "step": num("step").unwrap_or(1.0).max(1.0),
        "min": opt("min")?,
        "max": opt("max")?,
        "disabledDates": opt("disabledDates")?,
        "disabledDays": cx.prop(node, "disabledDays").unwrap_or(Json::Null),
        "months": months,
        "weekStart": num("weekStart"),
        "fiscalStart": fiscal,
        "presets": presets,
        "dropdowns": flag("dropdowns").unwrap_or(false),
        "clearable": flag("clearable").unwrap_or(false),
        "closeOnSelect": flag("closeOnSelect").unwrap_or(!time && mode != "multiple"),
        "name": s("name"),
        "locale": crate::locale(cx, node),
        "popover": picker,
        "input": picker && flag("input").unwrap_or(false),
        "bindError": s("bindError"),
    }))
}

/// The current value from state or `value`, for the server's text and hidden inputs.
fn current(cx: &RenderCx<'_>, node: &Node) -> Json {
    match cx.prop_string(node, "bind") {
        Some(k) => cx.lookup(&[k]),
        None => cx.prop(node, "value").unwrap_or(Json::Null),
    }
}

/// Hidden inputs that carry the value in a form; the behaviour keeps them current.
fn hidden_inputs(cx: &mut RenderCx<'_>, node: &Node, value: &Json) -> Result<(), RenderError> {
    let mut span = Element::new("span");
    span.attr("data-cal-hidden", true);
    cx.open(&span, node)?;
    if let Some(name) = cx.prop_string(node, "name") {
        let text = |v: Option<&Json>| v.filter(|v| !v.is_null()).map(expr::stringify).unwrap_or_default();
        let inputs: Vec<(String, String)> = match value {
            Json::Object(o) => vec![(format!("{name}[from]"), text(o.get("from"))), (format!("{name}[to]"), text(o.get("to")))],
            Json::Array(a) => a.iter().map(|v| (name.clone(), text(Some(v)))).collect(),
            v => vec![(name.clone(), text(Some(v)))],
        };
        for (n, v) in inputs {
            let mut i = Element::new("input");
            i.attr("type", "hidden").attr("name", n).attr("value", v);
            cx.open(&i, node)?;
        }
    }
    cx.close("span");
    Ok(())
}

fn live(cx: &mut RenderCx<'_>, node: &Node) -> Result<(), RenderError> {
    let mut l = Element::new("div");
    l.attr("data-cal-live", true).attr("role", "status").attr("aria-live", "polite");
    cx.open(&l, node)?;
    cx.close("div");
    Ok(())
}

pub struct Calendar;

impl Component for Calendar {
    fn name(&self) -> &'static str {
        "ui-calendar"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "An inline calendar: days (with a time), months, quarters or years; one, a range or several; limits, presets, financial years and full keyboard support.",
            props: CALENDAR_PROPS,
            parts: &[],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        crate::flips(cx);
        let config = config(cx, node, false)?;
        let value = current(cx, node);
        cx.global_js("ui-calendar", SCRIPT);
        let id = instance_id(cx, node, "calendar");
        let mut root = cx.element("div", node);
        root.base_style_ref(&CALENDAR)
            .behavior("ui-calendar", config.clone())
            .attr("id", id)
            .attr("role", "group")
            .attr("data-picker", config["picker"].as_str().unwrap_or("day").to_string())
            .attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Calendar"))));
        cx.open(&root, node)?;
        let mut body = Element::new("div");
        body.attr("data-cal", true);
        cx.open(&body, node)?;
        cx.close("div");
        live(cx, node)?;
        hidden_inputs(cx, node, &value)?;
        cx.close("div");
        Ok(())
    }
}

pub struct DatePicker;

impl Component for DatePicker {
    fn name(&self) -> &'static str {
        "ui-date-picker"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A button that opens a calendar in a popover: a day (with a time), month, quarter or year; one, a range or several; presets, limits and financial years.",
            props: PICKER_PROPS,
            parts: &["trigger", "input", "popover"],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        crate::flips(cx);
        let config = config(cx, node, true)?;
        let value = current(cx, node);
        cx.global_js("ui-calendar", SCRIPT);
        let id = instance_id(cx, node, "date-picker");
        let pop = id_with(&id, "-calendar");
        let anchor = format!("--{}", expr::stringify(&cx.eval(&id)));
        let empty = match &value {
            Json::Null => true,
            Json::String(s) => s.is_empty(),
            Json::Array(a) => a.is_empty(),
            Json::Object(o) => o.get("from").is_none_or(Json::is_null),
            _ => false,
        };
        let placeholder = match cx.prop_string(node, "placeholder") {
            Some(p) => p,
            None => match (config["mode"].as_str(), config["picker"].as_str()) {
                (Some("range"), _) => cx.label("Pick a range"),
                (Some("multiple"), _) => cx.label("Pick dates"),
                (_, Some("month")) => cx.label("Pick a month"),
                (_, Some("quarter")) => cx.label("Pick a quarter"),
                (_, Some("year")) => cx.label("Pick a year"),
                _ => cx.label("Pick a date"),
            },
        };

        let mut root = cx.element("div", node);
        root.base_style_ref(&PICKER).behavior("ui-date-picker", config.clone()).attr("data-picker", config["picker"].as_str().unwrap_or("day").to_string());
        if empty {
            root.attr("data-empty", true);
        }
        cx.open(&root, node)?;

        if config["input"] == json!(true) {
            // A text field and, beside it inside the same border, the calendar's button.
            let mut group = Element::new("div");
            group.base_style_ref(&INPUT_GROUP).base_style(StyleBlock::new().set("anchor-name", anchor.clone())).attr("data-cal-group", true);
            cx.open(&group, node)?;
            let mut input = cx.part("input", node, "input");
            let typed = match &value {
                Json::Object(o) if !empty => format!("{} – {}", expr::stringify(o.get("from").unwrap_or(&Json::Null)), o.get("to").map(expr::stringify).unwrap_or_default()),
                v if !empty => expr::stringify(v),
                _ => String::new(),
            };
            input
                .attr("type", "text")
                .attr("id", id.clone())
                .attr("data-cal-input", true)
                .attr("value", typed)
                .attr("autocomplete", "off")
                .attr("spellcheck", "false");
            if let Some(p) = cx.prop_string(node, "placeholder") {
                input.attr("placeholder", p);
            }
            if let Some(l) = node.prop("label") {
                input.attr("aria-label", l.clone());
            }
            if let Some(d) = node.prop("disabled") {
                input.attr("disabled", d.clone());
            }
            cx.open(&input, node)?;
            if config["clearable"] == json!(true) {
                let mut x = Element::new("button");
                x.attr("type", "button").attr("data-cal-x", true).attr("aria-label", cx.label("Clear")).attr("hidden", empty).text(lit("✕"));
                cx.open(&x, node)?;
                cx.close("button");
            }
            let mut b = cx.part("button", node, "trigger");
            b.attr("type", "button")
                .attr("data-cal-trigger", true)
                .attr("popovertarget", pop.clone())
                .attr("aria-haspopup", "dialog")
                // Labels: "Choose dates", "Choose a date"
                .attr("aria-label", cx.label(if config["mode"] == "range" { "Choose dates" } else { "Choose a date" }));
            if let Some(d) = node.prop("disabled") {
                b.attr("disabled", d.clone());
            }
            cx.open(&b, node)?;
            cx.markup(ICON);
            cx.close("button");
            cx.close("div");
            return finish(cx, node, config, &pop, &anchor, &value);
        }

        let mut trigger = cx.part("button", node, "trigger");
        forms::button_look(&mut trigger, "outline", "default");
        trigger
            .base_style(StyleBlock::new().set("anchor-name", anchor.clone()))
            .attr("type", "button")
            .attr("id", id.clone())
            .attr("data-cal-trigger", true)
            .attr("popovertarget", pop.clone())
            .attr("aria-haspopup", "dialog");
        if let Some(d) = node.prop("disabled") {
            trigger.attr("disabled", d.clone());
        }
        cx.open(&trigger, node)?;
        cx.markup(ICON);
        let mut text = Element::new("span");
        // The value as stored until the behaviour formats it.
        let shown = if empty {
            placeholder.clone()
        } else {
            match &value {
                Json::Object(o) => format!("{} – {}", expr::stringify(o.get("from").unwrap_or(&Json::Null)), o.get("to").map(expr::stringify).unwrap_or_default()),
                Json::Array(a) => cx.label_with("{n} dates", &[("n", &a.len().to_string())]),
                v => expr::stringify(v),
            }
        };
        text.attr("data-cal-text", true).attr("data-placeholder", placeholder).text(lit(shown));
        cx.open(&text, node)?;
        cx.close("span");
        cx.close("button");
        if config["clearable"] == json!(true) {
            let mut x = Element::new("button");
            x.attr("type", "button").attr("data-cal-x", true).attr("aria-label", cx.label("Clear")).attr("hidden", empty).text(lit("✕"));
            cx.open(&x, node)?;
            cx.close("button");
        }

        finish(cx, node, config, &pop, &anchor, &value)
    }
}

/// The popover holding the calendar, the live region and the hidden inputs, then the
/// end of the picker's element.
fn finish(cx: &mut RenderCx<'_>, node: &Node, config: Json, pop: &Val, anchor: &str, value: &Json) -> Result<(), RenderError> {
    let align = cx.prop_string(node, "align").unwrap_or_else(|| "start".into());
    let area = if align == "end" { "bottom span-left" } else { "bottom span-right" };
    let mut popover = cx.part("div", node, "popover");
    popover
        .base_style(crate::with_area(StyleBlock::new().set("position-anchor", anchor.to_string()), area))
        .attr("popover", "auto")
        .attr("id", pop.clone())
        .attr("role", "dialog")
        .attr("data-cal-pop", true)
        .attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label(if config["mode"] == "range" { "Choose dates" } else { "Choose a date" }))));
    cx.open(&popover, node)?;
    let mut body = Element::new("div");
    body.attr("data-cal", true);
    cx.open(&body, node)?;
    cx.close("div");
    cx.close("div");
    live(cx, node)?;
    hidden_inputs(cx, node, value)?;
    cx.close("div");
    Ok(())
}
