//! `ui-countdown`: the time left to a deadline (a ticket's SLA, an offer, a launch),
//! turning to warning and danger tones as thresholds pass and counting up once it's
//! over, as text or as big tiles, with a bar of the time used when it knows the start.
//! The server renders the frame and the deadline in a `<time>`; `countdown.js` counts.

use crate::{MUTED, TEXT, lit, status, widget};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("countdown.js");

const PROPS: &[PropDef] = &[
    PropDef::new("to", PropKind::Any, "The deadline: a date and time (`2026-12-01T17:00`, `2026-12-01T17:00:00Z`), milliseconds, or `+7h 40m` from when the page opens; may use state").required(),
    PropDef::new("from", PropKind::Any, "When the time started, as `to` (`-16h 20m`: before the page opened): a bar shows how much of it is used"),
    PropDef::new("warn", PropKind::Any, "Time left when it turns to the warning tone: seconds, or `90s`, `30m`, `4h`, `2d`"),
    PropDef::new("danger", PropKind::Any, "Time left when it turns to the danger tone, as `warn`"),
    PropDef::new("overdue", PropKind::Bool, "Past the deadline, count up as overdue (default true); false shows `endText`"),
    PropDef::new("endText", PropKind::Any, "Shown once it's over, with `overdue: false` (default `Time's up`)"),
    PropDef::new("format", PropKind::Enum(&["units", "clock"]), "`units` (default: `2d 4h`, `13m 05s`) or `clock` (`2:04:13`, `3d 02:04:13`)"),
    PropDef::new("variant", PropKind::Enum(&["inline", "tiles"]), "`inline` (default: text with a dot in its tone) or `tiles` (days, hours, minutes and seconds in boxes)"),
    PropDef::new("paused", PropKind::Any, "While true (e.g. `{{ ticket.status == 'waiting' }}`), the time left stands still and it says Paused; on resuming the deadline moves out by the pause. A new `to` from the page replaces the moved one"),
    PropDef::new("workdays", PropKind::Any, "Count working time only: `true` for Monday to Friday, or the weekdays that work (0 is Sunday), e.g. `[0, 1, 2, 3, 4]`"),
    PropDef::new("hours", PropKind::Any, "Working hours each working day, e.g. `09:00-17:30` (local time), or a shift past midnight, `22:00-06:00`; outside them it stands still. Alone, every day works. Or hours by weekday, `{ mon: 09:00-17:00, sat: 10:00-14:00 }` (also `0`–`6`, 0 Sunday): the days not named don't work"),
    PropDef::new("holidays", PropKind::Any, "Dates that don't count either, e.g. `[\"2026-12-25\"]`"),
    PropDef::new("bindLeft", PropKind::StateKey, "State key that gets the seconds left (below zero once overdue), every second"),
    PropDef::new("bindStatus", PropKind::StateKey, "State key that gets `ok`, `warning`, `danger` or `over` as it changes"),
    PropDef::new("label", PropKind::Any, "What it counts down to, for screen readers, e.g. `First reply due`"),
];

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

static COUNTDOWN: LazyLock<StyleBlock> = LazyLock::new(|| {
    status::tone_rules(
        pairs(&[("--tone", MUTED), ("display", "inline-flex"), ("flex-direction", "column"), ("gap", gap!(6)), ("color", TEXT), ("font-variant-numeric", "tabular-nums")])
            .nest("& [data-cd-line]", pairs(&[("display", "inline-flex"), ("align-items", "center"), ("gap", gap!(6)), ("white-space", "nowrap")]))
            .nest("& [data-cd-dot]", pairs(&[("flex", "none"), ("width", "8px"), ("height", "8px"), ("border-radius", "50%"), ("background", "var(--tone)")]))
            .nest("&[data-status=over] [data-cd-dot], &[data-status=danger] [data-cd-dot]", pairs(&[("animation", "ui-cd-pulse 1.2s ease-in-out infinite")]))
            // Paused: still, and muted, whatever the time left.
            .nest("&:is([data-paused], [data-closed]) [data-cd-dot]", pairs(&[("animation", "none"), ("background", MUTED)]))
            .nest("&:not([data-status=ok]) [data-cd-text]", pairs(&[("color", "color-mix(in srgb, var(--tone) 85%, var(--color-text))"), ("font-weight", "var(--font-weight-medium, 500)")]))
            .nest(
                "& [data-cd-bar]",
                pairs(&[("height", "4px"), ("min-width", sp!(120)), ("border-radius", "999px"), ("overflow", "hidden"), ("background", "color-mix(in srgb, var(--color-text) 10%, transparent)")]),
            )
            .nest("& [data-cd-bar] > span", pairs(&[("display", "block"), ("height", "100%"), ("width", "calc(var(--used, 0) * 1%)"), ("border-radius", "inherit"), ("background", "var(--tone)")]))
            // Moving with the seconds, but not sweeping in from nothing when the page opens.
            .nest("&[data-cd-ready] [data-cd-bar] > span", pairs(&[("transition", "width 1s linear")]))
            .nest("&[data-status=ok] [data-cd-bar] > span", pairs(&[("background", "var(--color-primary)")]))
            // Tiles: a box per unit, the number big and its name under it.
            .nest("& [data-cd-tiles]", pairs(&[("display", "flex"), ("gap", gap!(8))]))
            .nest(
                "& [data-cd-tile]",
                pairs(&[
                    ("display", "flex"),
                    ("flex-direction", "column"),
                    ("align-items", "center"),
                    ("min-width", sp!(64)),
                    ("padding", pad!(10, 8)),
                    ("border", "var(--border-width, 1px) solid var(--color-border)"),
                    ("border-radius", "var(--radius-card, var(--radius-lg))"),
                    ("background", "var(--color-card, var(--color-surface))"),
                ]),
            )
            .nest("&:not([data-status=ok]) [data-cd-tile]", pairs(&[("border-color", "color-mix(in srgb, var(--tone) 45%, var(--color-border))")]))
            .nest("& [data-cd-tile] b", pairs(&[("font-size", fs!(28)), ("font-weight", "var(--font-weight-semibold, 600)"), ("line-height", "1.1"), ("letter-spacing", "-0.02em")]))
            .nest("&:not([data-status=ok]) [data-cd-tile] b", pairs(&[("color", "var(--tone)")]))
            .nest("& [data-cd-tile] small", pairs(&[("color", MUTED), ("font-size", fs!(12))]))
            .nest("& [data-cd-tiles] ~ [data-cd-line]", pairs(&[("color", MUTED), ("font-size", fs!(13))]))
            .nest("& time", pairs(&[("color", "inherit")]))
            .nest(
                "& [data-cd-live]",
                pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("overflow", "hidden"), ("clip-path", "inset(50%)"), ("white-space", "nowrap")]),
            )
            .nest("@keyframes ui-cd-pulse", pairs(&[]).nest("50%", pairs(&[("opacity", "0.35")])))
            .nest("@media (prefers-reduced-motion: reduce)", pairs(&[]).nest("& [data-cd-dot]", pairs(&[("animation", "none")])).nest("& [data-cd-bar] > span", pairs(&[("transition", "none")]))),
        "&",
    )
});

/// A duration in seconds: a number, or `90s`, `30m`, `4h`, `2d`.
fn seconds(node: &Node, name: &str, v: &Json) -> Result<f64, RenderError> {
    let bad = || RenderError::new(node, format!("`{name}` is seconds, or a number with s, m, h or d: `30m`, `4h`"));
    let n = match v {
        Json::Number(n) => n.as_f64().unwrap_or(-1.0),
        Json::String(s) => {
            let s = s.trim();
            let (num, unit) = s.split_at(s.find(|c: char| c.is_ascii_alphabetic()).unwrap_or(s.len()));
            let n: f64 = num.trim().parse().map_err(|_| bad())?;
            n * match unit.trim() {
                "" | "s" => 1.0,
                "m" => 60.0,
                "h" => 3600.0,
                "d" => 86400.0,
                _ => return Err(bad()),
            }
        }
        _ => return Err(bad()),
    };
    if n.is_finite() && n >= 0.0 { Ok(n) } else { Err(bad()) }
}

pub struct Countdown;

impl Component for Countdown {
    fn name(&self) -> &'static str {
        "ui-countdown"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "The time left to a deadline, turning to warning and danger as thresholds pass and counting up once overdue; as text or tiles, with a bar of the time used.",
            props: PROPS,
            parts: &[],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut config = widget::config(cx, node, &["to", "from", "paused"], &["format", "variant", "workdays", "holidays"], &["bindLeft", "bindStatus"])?;
        let mut limit = |name: &str| -> Result<Option<f64>, RenderError> {
            match cx.prop(node, name).filter(|v| !v.is_null()) {
                Some(v) => {
                    let s = seconds(node, name, &v)?;
                    config.insert(name.into(), json!(s));
                    Ok(Some(s))
                }
                None => Ok(None),
            }
        };
        let (warn, danger) = (limit("warn")?, limit("danger")?);
        if let (Some(w), Some(d)) = (warn, danger)
            && d > w
        {
            return Err(RenderError::new(node, "`danger` comes after `warn`: it must be the shorter time left"));
        }
        match cx.prop(node, "hours").filter(|v| !v.is_null()) {
            None => {}
            // By weekday: `{ mon: 09:00-17:00 }`, as `{ "1": [540, 1020] }`.
            Some(Json::Object(m)) => {
                let mut by = serde_json::Map::new();
                for (k, v) in m {
                    let day = DAYS.iter().position(|d| k.to_ascii_lowercase().starts_with(d)).or_else(|| k.parse::<usize>().ok().filter(|d| *d < 7));
                    let Some(day) = day else {
                        return Err(RenderError::new(node, format!("`hours` by weekday takes `sun` to `sat` (or 0 to 6), not `{k}`")));
                    };
                    by.insert(day.to_string(), json!(hours(node, &expr::stringify(&v))?));
                }
                config.insert("hours".into(), Json::Object(by));
            }
            Some(h) => {
                config.insert("hours".into(), json!(hours(node, &expr::stringify(&h))?));
            }
        }
        if let Some(d) = config.get("workdays").filter(|d| !matches!(d, Json::Bool(_))) {
            let ok = d.as_array().is_some_and(|l| !l.is_empty() && l.iter().all(|n| n.as_f64().is_some_and(|n| n.fract() == 0.0 && (0.0..=6.0).contains(&n))));
            if !ok {
                return Err(RenderError::new(node, "`workdays` must be `true` or weekdays from 0 (Sunday) to 6, e.g. `[1, 2, 3, 4, 5]`"));
            }
        }
        if config.get("holidays").is_some_and(|h| !h.is_array()) {
            return Err(RenderError::new(node, "`holidays` must be a list of dates, e.g. `[\"2026-12-25\"]`"));
        }
        config.insert("overdue".into(), json!(widget::flag(cx, node, "overdue", true)));
        config.insert("endText".into(), json!(cx.prop(node, "endText").filter(|v| !v.is_null()).map_or_else(|| cx.label("Time's up"), |v| expr::stringify(&v))));
        let tiles = cx.prop_string(node, "variant").as_deref() == Some("tiles");
        widget::kit(cx);
        cx.global_js("ui-countdown", SCRIPT);

        let mut el = cx.element("div", node);
        el.base_style_ref(&COUNTDOWN).behavior("ui-countdown", Json::Object(config)).attr("data-status", "ok").attr("role", "timer");
        if let Some(l) = node.prop("label") {
            el.attr("aria-label", l.clone());
        }
        cx.open(&el, node)?;
        if tiles {
            let mut t = Element::new("div");
            t.attr("data-cd-tiles", true).attr("aria-hidden", "true");
            cx.open(&t, node)?;
            // Labels: "Days", "Hours", "Minutes", "Seconds"
            for unit in ["Days", "Hours", "Minutes", "Seconds"] {
                let mut tile = Element::new("span");
                tile.attr("data-cd-tile", unit.to_lowercase());
                cx.open(&tile, node)?;
                let mut b = Element::new("b");
                b.text(lit("--"));
                cx.open(&b, node)?;
                cx.close("b");
                let mut s = Element::new("small");
                s.text(lit(cx.label(unit)));
                cx.open(&s, node)?;
                cx.close("small");
                cx.close("span");
            }
            cx.close("div");
        }
        // The deadline as written, until the behaviour counts (and for a page without it).
        let mut line = Element::new("span");
        line.attr("data-cd-line", true);
        cx.open(&line, node)?;
        if !tiles {
            let mut dot = Element::new("span");
            dot.attr("data-cd-dot", true).attr("aria-hidden", "true");
            cx.open(&dot, node)?;
            cx.close("span");
        }
        // Written once, not bound: the behaviour owns the text from then on.
        let to = cx.prop(node, "to").map(|v| expr::stringify(&v)).unwrap_or_default();
        let mut time = Element::new("time");
        time.attr("data-cd-text", true).attr("datetime", to.clone()).text(lit(to));
        cx.open(&time, node)?;
        cx.close("time");
        cx.close("span");
        if node.prop("from").is_some() {
            let mut bar = Element::new("span");
            bar.attr("data-cd-bar", true).attr("aria-hidden", "true");
            cx.open(&bar, node)?;
            cx.open(&Element::new("span"), node)?;
            cx.close("span");
            cx.close("span");
        }
        let mut live = Element::new("span");
        live.attr("data-cd-live", true).attr("role", "status").attr("aria-live", "polite");
        cx.open(&live, node)?;
        cx.close("span");
        cx.close("div");
        Ok(())
    }
}

/// `hours` as minutes into the day it starts and ends, `09:00-17:30` → `[540, 1050]`.
/// Weekdays by name, Sunday first.
const DAYS: [&str; 7] = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"];

/// A start and an end in minutes; an end before the start is the next day's (a night shift).
fn hours(node: &Node, h: &str) -> Result<[u32; 2], RenderError> {
    let bad = || RenderError::new(node, format!("`hours` must be a start and an end in a day, e.g. `09:00-17:30` (or `22:00-06:00` past midnight), not `{h}`"));
    let at = |t: &str| -> Option<u32> {
        let (a, b) = t.trim().split_once(':')?;
        let (a, b): (u32, u32) = (a.parse().ok()?, b.parse().ok()?);
        (b < 60 && (a < 24 || (a == 24 && b == 0)) && b.to_string().len() <= 2).then_some(a * 60 + b)
    };
    let (a, b) = h.split_once(['-', '–']).ok_or_else(bad)?;
    match (at(a), at(b)) {
        (Some(a), Some(b)) if a < b => Ok([a, b]),
        (Some(a), Some(b)) if a > b && a < 1440 => Ok([a, b + 1440]),
        _ => Err(bad()),
    }
}
