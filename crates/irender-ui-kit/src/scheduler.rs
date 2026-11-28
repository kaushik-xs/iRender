//! `ui-scheduler`: bookings of people, rooms or machines against time, one row each, drawn
//! by `scheduler.js`. Bookings are moved, stretched and made by dragging or the keys, and
//! clashes are marked.

use crate::{
    BG_UNDER, BORDER, MUTED, PRIMARY, SUBTLE, SURFACE, TEXT, forms, lit,
    widget::{self, flag, pairs},
};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock};
use irender_html::{Component, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("scheduler.js");

pub struct Scheduler;

const PROPS: &[PropDef] = &[
    PropDef::new("resources", PropKind::Any, "The rows: `[{ id, title, subtitle, avatar, group, hours: [{ from, to }], capacity }]`; may use state").required(),
    PropDef::new("events", PropKind::Any, "The bookings: `[{ id, resource, title, start, end, color, status, locked, note }]`, times as `YYYY-MM-DDTHH:MM`; `repeat`, as `ui-event-calendar`'s, makes one come again: a day of it moved (or stretched) on its own is left out of the series (`repeat.except`) and becomes a booking of its own, with `series` and `occurrence` (the day it was); may use state").required(),
    PropDef::new("bindEvents", PropKind::StateKey, "State key the bookings are in: moves, stretches and new ones are written back"),
    PropDef::new("bindEvent", PropKind::StateKey, "State key that gets the booking clicked (or opened with Enter)"),
    PropDef::new("bindDate", PropKind::StateKey, "State key for the first day shown, `YYYY-MM-DD` (default: today)"),
    PropDef::new("bindView", PropKind::StateKey, "State key for the view: `day`, `week` or `month`"),
    PropDef::new("view", PropKind::Enum(&["day", "week", "month"]), "The view to start with (default `day`)"),
    PropDef::new("views", PropKind::Any, "The views offered (default all three)"),
    PropDef::new("from", PropKind::Any, "First hour shown in a day, e.g. `08:00` (default `00:00`)"),
    PropDef::new("to", PropKind::Any, "Last hour shown, e.g. `18:00` (default `24:00`)"),
    PropDef::new("snap", PropKind::Number, "Minutes moves and stretches go by in the day view (default 15)"),
    PropDef::new("overlap", PropKind::Enum(&["allow", "mark", "prevent"]), "Bookings of a resource at the same time: side by side, also marked as a clash (default), or not let happen"),
    PropDef::new("creatable", PropKind::Bool, "Dragging over a free stretch makes a booking, an event `create` and, with `bindEvents`, a new one"),
    PropDef::new("newEvent", PropKind::Any, "What a new booking starts with, e.g. `{ title: New booking }`"),
    PropDef::new("moveResources", PropKind::Bool, "Bookings can be moved to another row (default true)"),
    PropDef::new("bindClashes", PropKind::StateKey, "State key that gets the ids of bookings that clash"),
    PropDef::new("rowLabel", PropKind::Any, "The heading over the rows, e.g. `Room`"),
    PropDef::new("rowHeight", PropKind::Number, "A row's least height in px (default 52)"),
    PropDef::new("height", PropKind::Any, "Most it grows to before it scrolls"),
    PropDef::new("now", PropKind::Any, "Marks this time instead of the present, or false for no mark"),
    PropDef::new("label", PropKind::Any, "Names it for screen readers (default `Schedule`)"),
    PropDef::new("locale", PropKind::String, "Default: the document's `lang`, else the browser's"),
];

static ROOT: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(10)), ("min-width", "0"), ("color", TEXT), ("font-size", fs!(14))])
        .nest("& [data-sc-bar]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("align-items", "center"), ("gap", gap!(6))]))
        .nest("& [data-sc-title]", pairs(&[("margin-inline", "6px auto"), ("font-weight", "var(--font-weight-semibold, 600)")]))
        .nest("& [data-sc-bar] button", pairs(&[("display", "inline-grid"), ("place-items", "center"), ("min-width", ctl!(32)), ("height", ctl!(32)), ("padding", pad!(0, 10)), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-control, var(--radius-md))"), ("background", SURFACE), ("color", TEXT), ("font", "inherit"), ("cursor", "pointer")]))
        .nest("& [data-sc-bar] button:hover", pairs(&[("background", SUBTLE)]))
        .nest("& [data-sc-bar] button[aria-pressed=true]", pairs(&[("border-color", PRIMARY), ("background", "color-mix(in srgb, var(--color-primary) 12%, transparent)")]))
        .nest("& button:focus-visible", forms::focus_ring())
        .nest("& [data-sc-scroll]", pairs(&[("position", "relative"), ("overflow", "auto"), ("max-height", "var(--sc-h, none)"), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-card, var(--radius-md))")]))
        .nest("& [data-sc-grid]", pairs(&[("position", "relative"), ("display", "grid"), ("grid-template-columns", "var(--sc-side, 180px) minmax(var(--sc-w, 0px), 1fr)"), ("min-width", "max-content")]))
        .nest("& [data-sc-head]", pairs(&[("position", "sticky"), ("top", "0"), ("z-index", "4"), ("display", "flex"), ("background", BG_UNDER), ("border-bottom", "var(--border-width, 1px) solid var(--color-border)"), ("color", MUTED), ("font-size", fs!(12))]))
        .nest("& [data-sc-head] > span", pairs(&[("flex", "1 1 0"), ("min-width", "0"), ("padding", pad!(6)), ("border-inline-start", "var(--border-width, 1px) solid var(--color-border)"), ("text-align", "start"), ("white-space", "nowrap"), ("overflow", "hidden")]))
        .nest("& [data-sc-head] > span[data-today]", pairs(&[("color", PRIMARY), ("font-weight", "var(--font-weight-semibold, 600)")]))
        .nest("& [data-sc-corner]", pairs(&[("position", "sticky"), ("inset-inline-start", "0"), ("top", "0"), ("z-index", "5"), ("padding", pad!(6, 12)), ("background", BG_UNDER), ("border-bottom", "var(--border-width, 1px) solid var(--color-border)"), ("color", MUTED), ("font-size", fs!(12))]))
        .nest("& [data-sc-group]", pairs(&[("grid-column", "1 / -1"), ("position", "relative"), ("padding", pad!(4, 12)), ("background", SUBTLE), ("color", MUTED), ("font-size", fs!(12)), ("font-weight", "var(--font-weight-semibold, 600)"), ("border-bottom", "var(--border-width, 1px) solid var(--color-border)")]))
        .nest("& [data-sc-group] > span", pairs(&[("position", "sticky"), ("inset-inline-start", "12px")]))
        .nest("& [data-sc-side]", pairs(&[("position", "sticky"), ("inset-inline-start", "0"), ("z-index", "3"), ("display", "flex"), ("align-items", "center"), ("gap", gap!(8)), ("box-sizing", "border-box"), ("min-width", "0"), ("padding", pad!(6, 12)), ("background", BG_UNDER), ("border-bottom", "var(--border-width, 1px) solid var(--color-border)"), ("border-inline-end", "var(--border-width, 1px) solid var(--color-border)")]))
        .nest("& [data-sc-side] > span:last-child", pairs(&[("min-width", "0"), ("overflow", "hidden"), ("text-overflow", "ellipsis"), ("white-space", "nowrap"), ("font-weight", "var(--font-weight-medium, 500)")]))
        .nest("& [data-sc-side] small", pairs(&[("display", "block"), ("color", MUTED), ("font-weight", "400"), ("overflow", "hidden"), ("text-overflow", "ellipsis")]))
        .nest("& [data-avatar]", pairs(&[("flex", "none"), ("display", "grid"), ("place-items", "center"), ("width", "28px"), ("height", "28px"), ("overflow", "hidden"), ("border-radius", "50%"), ("background", SUBTLE), ("color", MUTED), ("font-size", fs!(11))]))
        .nest("& [data-avatar] img", pairs(&[("width", "100%"), ("height", "100%"), ("object-fit", "cover")]))
        .nest("& [data-sc-row]", pairs(&[("position", "relative"), ("min-height", "var(--sc-row, 52px)"), ("border-bottom", "var(--border-width, 1px) solid var(--color-border)"), ("background-image", "linear-gradient(to right, var(--color-border) 1px, transparent 1px)"), ("background-size", "var(--sc-step, 10%) 100%"), ("touch-action", "pan-x pan-y")]))
        .nest("& [data-sc-row]:dir(rtl)", pairs(&[("background-image", "linear-gradient(to left, var(--color-border) 1px, transparent 1px)"), ("background-position", "right")]))
        .nest("& [data-sc-row][data-over]", pairs(&[("background-color", "color-mix(in srgb, var(--color-primary) 6%, transparent)")]))
        .nest("& [data-sc-off]", pairs(&[("position", "absolute"), ("top", "0"), ("bottom", "0"), ("background", "repeating-linear-gradient(-45deg, color-mix(in srgb, var(--color-text) 7%, transparent) 0 4px, transparent 4px 8px)"), ("pointer-events", "none")]))
        .nest("& [data-sc-now]", pairs(&[("position", "absolute"), ("top", "0"), ("bottom", "0"), ("width", "2px"), ("margin-inline-start", "-1px"), ("background", "var(--color-destructive, #dc2626)"), ("z-index", "2"), ("pointer-events", "none")]))
        .nest(
            "& [data-sc-event]",
            pairs(&[
                ("--c", PRIMARY),
                ("position", "absolute"),
                ("z-index", "1"),
                ("box-sizing", "border-box"),
                ("overflow", "hidden"),
                ("padding", pad!(3, 8)),
                ("border", "1px solid color-mix(in srgb, var(--c) 45%, transparent)"),
                ("border-inline-start", "3px solid var(--c)"),
                ("border-radius", "var(--radius-sm)"),
                ("background", "color-mix(in srgb, var(--c) 16%, var(--ui-bg, var(--color-surface)))"),
                ("color", TEXT),
                ("font-size", fs!(13)),
                ("line-height", "1.3"),
                ("text-align", "start"),
                ("cursor", "pointer"),
                ("user-select", "none"),
            ]),
        )
        .nest("& [data-sc-event][data-movable]", pairs(&[("cursor", "grab"), ("touch-action", "none")]))
        .nest("& [data-sc-event]:focus-visible", forms::focus_ring().set("z-index", "3"))
        .nest("& [data-sc-event] > b, & [data-sc-event] > small", pairs(&[("display", "block"), ("overflow", "hidden"), ("text-overflow", "ellipsis"), ("white-space", "nowrap")]))
        .nest("& [data-sc-event] > b", pairs(&[("font-weight", "var(--font-weight-medium, 500)")]))
        .nest("& [data-sc-event] > small", pairs(&[("color", MUTED), ("font-size", fs!(12))]))
        .nest("& [data-sc-event][data-clash]", pairs(&[("--c", "var(--color-destructive, #dc2626)"), ("outline", "1.5px solid var(--color-destructive, #dc2626)"), ("outline-offset", "-1px")]))
        .nest("& [data-sc-event][aria-current=true]", pairs(&[("box-shadow", "0 0 0 2px var(--color-ring, var(--color-muted))")]))
        .nest("& [data-sc-event][data-dragging]", pairs(&[("opacity", "0.75"), ("z-index", "6"), ("cursor", "grabbing"), ("box-shadow", "var(--elevation-popover, 0 6px 18px rgb(0 0 0 / 0.2))")]))
        .nest("& [data-sc-event][data-status=tentative]", pairs(&[("background", "repeating-linear-gradient(-45deg, color-mix(in srgb, var(--c) 18%, var(--ui-bg, var(--color-surface))) 0 6px, color-mix(in srgb, var(--c) 8%, var(--ui-bg, var(--color-surface))) 6px 12px)")]))
        .nest("& [data-sc-edge]", pairs(&[("position", "absolute"), ("top", "0"), ("bottom", "0"), ("width", "8px"), ("cursor", "ew-resize")]))
        .nest("& [data-sc-edge=start]", pairs(&[("inset-inline-start", "0")]))
        .nest("& [data-sc-edge=end]", pairs(&[("inset-inline-end", "0")]))
        .nest("& [data-sc-new]", pairs(&[("position", "absolute"), ("top", "4px"), ("bottom", "4px"), ("border", "1.5px dashed"), ("border-color", PRIMARY), ("border-radius", "var(--radius-sm)"), ("background", "color-mix(in srgb, var(--color-primary) 10%, transparent)"), ("color", PRIMARY), ("font-size", fs!(12)), ("padding", "2px 6px"), ("pointer-events", "none")]))
        .nest("@media (max-width: 640px)", pairs(&[("--sc-side", "112px")]).nest("& [data-avatar]", pairs(&[("display", "none")])))
});

impl Component for Scheduler {
    fn name(&self) -> &'static str {
        "ui-scheduler"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Bookings against time, a row per person, room, vehicle or machine: moved, stretched and made by dragging or the keys, with clashes marked.", props: PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut config = widget::config(cx, node, &["resources", "events", "now", "from", "to"], &["view", "views", "snap", "overlap", "newEvent", "rowLabel", "rowHeight"], &["bindEvents", "bindEvent", "bindDate", "bindView", "bindClashes"])?;
        config.insert("creatable".into(), json!(flag(cx, node, "creatable", false)));
        config.insert("moveResources".into(), json!(flag(cx, node, "moveResources", true)));
        config.insert("locale".into(), json!(crate::locale(cx, node)));
        widget::kit(cx);
        cx.global_js("ui-scheduler", SCRIPT);
        let mut s = StyleBlock::new();
        if let Some(h) = node.prop("height")
            && let Some(v) = cx.style_value(node, "max-height", h)?
        {
            s.push("--sc-h", v);
        }
        if let Some(h) = widget::number(cx, node, "rowHeight") {
            s.push("--sc-row", irender_core::StyleValue::Static(format!("{}px", h.clamp(28.0, 200.0))));
        }
        let mut el = cx.element("div", node);
        el.base_style_ref(&ROOT).base_style(s).attr("role", "group").attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Schedule")))).behavior("ui-scheduler", Json::Object(config));
        cx.open(&el, node)?;
        cx.close("div");
        Ok(())
    }
}
