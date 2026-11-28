//! `ui-gauge`: a figure on a dial between a minimum and a maximum, with coloured bands
//! (good, warning, bad), a target and a needle. Drawn on the server as SVG, without a
//! script: the value's arc, the needle and the target turn with CSS variables worked
//! out in the browser from the same expressions the server used, and so does the tone
//! of the band the value is in. The chart kit's `gauge` type is the full one; this is
//! the light one, for a figure in a tile.

use crate::{MUTED, TEXT, bin, cond, expr_val, lit, num, status, val_expr};
use irender_core::{
    ComponentInfo, Node, Part, PropDef, PropKind, StyleBlock, StyleValue, Template, Val,
    expr::{self, BinOp, Expr},
};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const PROPS: &[PropDef] = &[
    PropDef::new("value", PropKind::Any, "The figure. Follows state when it uses it, e.g. `{{ line.oee }}`").required(),
    PropDef::new("min", PropKind::Number, "The dial's start (default 0)"),
    PropDef::new("max", PropKind::Number, "The dial's end (default 100)"),
    PropDef::new("unit", PropKind::Any, "After the figure, smaller, e.g. `%` or `°C`"),
    PropDef::new("text", PropKind::Any, "In the middle in place of the value, e.g. a formatted one; empty for none"),
    PropDef::new("decimals", PropKind::Number, "The figure rounded to this many decimals, 0–10 (also when it follows state)"),
    PropDef::new("ticks", PropKind::Number, "Marks dividing the dial into this many equal parts, 2–20"),
    PropDef::new("tickLabels", PropKind::Bool, "With `ticks`: each mark's value inside it (the ends' too when `scale` is off), rounded to `decimals`"),
    PropDef::new(
        "bands",
        PropKind::Any,
        "Coloured stretches of the dial, in order: `{ to, tone, label }`, the last without `to` running to `max`. The value takes the tone of the band it's in",
    ),
    PropDef::new("tone", PropKind::Enum(status::TONES), "The value's colour (default: its band's, else `primary`)"),
    PropDef::new("target", PropKind::Any, "A mark across the dial at this value; may use state"),
    PropDef::new("needle", PropKind::Bool, "A needle pointing at the value, as well as the arc"),
    PropDef::new("shape", PropKind::Enum(&["arc", "half"]), "`arc` (default, 240°) or `half` (a semicircle)"),
    PropDef::new("size", PropKind::Number, "Width in px (default 160)"),
    PropDef::new("thickness", PropKind::Number, "The dial's width in px (default: a twelfth of the size)"),
    PropDef::new("scale", PropKind::Bool, "`min` and `max` at the dial's ends (default true)"),
    PropDef::new("caption", PropKind::Any, "Text under the dial, e.g. what it measures"),
    PropDef::new("label", PropKind::Any, "What it measures, for screen readers (default: `caption`)"),
];

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

/// A value at `--p` (and the target at `--t`) of the way round, clamped so bad values
/// stay on the dial.
const P: &str = "min(100, max(0, var(--p, 0)))";
const T: &str = "min(100, max(0, var(--t, 0)))";

static GAUGE: LazyLock<StyleBlock> = LazyLock::new(|| {
    status::tone_rules(
        pairs(&[
            ("--tone", "var(--color-primary)"),
            ("position", "relative"),
            ("display", "inline-flex"),
            ("flex-direction", "column"),
            ("align-items", "center"),
            ("flex", "none"),
            ("color", TEXT),
        ])
        .nest("& [data-dial]", pairs(&[("position", "relative")]))
        .nest("& svg", pairs(&[("display", "block"), ("overflow", "visible")]))
        .nest("& [data-track]", pairs(&[("stroke", "color-mix(in srgb, var(--color-text) 10%, transparent)")]))
        .nest("& [data-band]", pairs(&[("stroke", "color-mix(in srgb, var(--band) 35%, transparent)")]))
        .nest(
            "& [data-arc]",
            pairs(&[("stroke", "var(--tone)"), ("stroke-dasharray", "100 200"), ("stroke-dashoffset", &format!("calc((100 - {P}) * 1px)")), ("transition", "stroke-dashoffset .4s, stroke .4s")]),
        )
        // Needle and target point up at rest and turn about the dial's centre.
        .nest("& [data-needle], & [data-target]", pairs(&[("transform-box", "view-box"), ("transition", "transform .4s")]))
        .nest("&[data-shape=arc] [data-needle]", pairs(&[("transform", &format!("rotate(calc(-120deg + {P} * 2.4deg))"))]))
        .nest("&[data-shape=arc] [data-target]", pairs(&[("transform", &format!("rotate(calc(-120deg + {T} * 2.4deg))"))]))
        .nest("&[data-shape=half] [data-needle]", pairs(&[("transform", &format!("rotate(calc(-90deg + {P} * 1.8deg))"))]))
        .nest("&[data-shape=half] [data-target]", pairs(&[("transform", &format!("rotate(calc(-90deg + {T} * 1.8deg))"))]))
        .nest("& [data-needle]", pairs(&[("stroke", "var(--color-text)"), ("fill", "var(--color-text)")]))
        .nest("& [data-target]", pairs(&[("stroke", "var(--color-text)")]))
        .nest("& [data-tick]", pairs(&[("stroke", "color-mix(in srgb, var(--color-text) 35%, transparent)")]))
        .nest("@media (prefers-reduced-motion: reduce)", pairs(&[]).nest("& [data-arc], & [data-needle], & [data-target]", pairs(&[("transition", "none")])))
        .nest(
            "& [data-gauge-value]",
            pairs(&[
                ("position", "absolute"),
                ("left", "50%"),
                ("display", "flex"),
                ("align-items", "baseline"),
                ("gap", "2px"),
                ("font-weight", "var(--font-weight-semibold, 600)"),
                ("font-variant-numeric", "tabular-nums"),
                ("line-height", "1"),
                ("letter-spacing", "-0.02em"),
                ("white-space", "nowrap"),
                ("transform", "translate(-50%, -50%)"),
            ]),
        )
        .nest("&[data-shape=half] [data-gauge-value]", pairs(&[("transform", "translate(-50%, -100%)")]))
        // With a needle, the figure sits under the hub rather than across the needle.
        .nest("&[data-shape=half][data-with-needle] [data-gauge-value]", pairs(&[("transform", "translateX(-50%)")]))
        .nest("& [data-gauge-unit]", pairs(&[("font-size", "0.5em"), ("font-weight", "var(--font-weight-medium, 500)"), ("color", MUTED), ("letter-spacing", "0")]))
        .nest(
            "& [data-gauge-end]",
            pairs(&[("position", "absolute"), ("transform", "translateX(-50%)"), ("color", MUTED), ("font-size", fs!(11)), ("font-variant-numeric", "tabular-nums"), ("line-height", "1")]),
        )
        .nest("& [data-gauge-tick]", pairs(&[("position", "absolute"), ("transform", "translate(-50%, -50%)"), ("color", MUTED), ("font-variant-numeric", "tabular-nums"), ("line-height", "1"), ("white-space", "nowrap")]))
        .nest("& [data-gauge-caption]", pairs(&[("margin-top", gap!(4)), ("color", MUTED), ("font-size", fs!(13)), ("text-align", "center")])),
        "&",
    )
});

/// A number to two decimals at most, as SVG and CSS take it.
fn n(x: f64) -> String {
    let r = (x * 100.0).round() / 100.0;
    if r == r.trunc() { format!("{}", r as i64) } else { format!("{r}") }
}

/// `(v - min) * 100 / (max - min)`: how far round the dial `v` is, as a percentage.
fn percent(v: &Val, min: f64, max: f64) -> StyleValue {
    match v {
        Val::Tpl(_) => {
            let e = bin(BinOp::Div, bin(BinOp::Mul, bin(BinOp::Sub, val_expr(v), num(min)), num(100)), num(max - min));
            StyleValue::Dynamic { tpl: Template { parts: vec![Part::Expr(e)] }, px: false }
        }
        Val::Lit(j) => StyleValue::Static(n((expr::to_num(j) - min) * 100.0 / (max - min))),
    }
}

/// A band of the dial, checked: up to `to` (the last up to `max`), in a tone.
struct Band {
    to: f64,
    tone: String,
    label: Option<String>,
}

fn bands(cx: &RenderCx<'_>, node: &Node, min: f64, max: f64) -> Result<Vec<Band>, RenderError> {
    let list = match cx.prop(node, "bands") {
        None | Some(Json::Null) => return Ok(Vec::new()),
        Some(Json::Array(l)) => l,
        Some(_) => return Err(RenderError::new(node, "`bands` must be a list of `{ to, tone, label }`")),
    };
    let mut out: Vec<Band> = Vec::new();
    for (i, b) in list.iter().enumerate() {
        let Some(o) = b.as_object() else { return Err(RenderError::new(node, "each band is `{ to, tone, label }`")) };
        let tone = o.get("tone").map(expr::stringify).unwrap_or_else(|| "neutral".into());
        if !status::TONES.contains(&tone.as_str()) {
            return Err(RenderError::new(node, format!("band {}: no tone `{tone}` (tones: {})", i + 1, status::TONES.join(", "))));
        }
        let last = i + 1 == list.len();
        let to = match o.get("to") {
            Some(v) if !v.is_null() => expr::to_num(v),
            _ if last => max,
            _ => return Err(RenderError::new(node, format!("band {}: only the last band may leave out `to`", i + 1))),
        };
        let from = out.last().map_or(min, |b| b.to);
        if !(to > from && to <= max) {
            return Err(RenderError::new(node, format!("band {}: `to` must be above {} and at most `max` ({})", i + 1, n(from), n(max))));
        }
        out.push(Band { to, tone, label: o.get("label").filter(|v| !v.is_null()).map(expr::stringify) });
    }
    Ok(out)
}

pub struct Gauge;

impl Component for Gauge {
    fn name(&self) -> &'static str {
        "ui-gauge"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A figure on a dial from `min` to `max`, with coloured bands, a target and a needle; follows state without a script.",
            props: PROPS,
            parts: &[],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let number = |name: &str, dflt: f64| cx.prop(node, name).filter(|v| !v.is_null()).map_or(dflt, |v| expr::to_num(&v));
        let (min, max) = (number("min", 0.0), number("max", 100.0));
        if !(min.is_finite() && max.is_finite() && max > min) {
            return Err(RenderError::new(node, "`max` must be above `min`"));
        }
        let size = number("size", 160.0);
        if !(48.0..=640.0).contains(&size) {
            return Err(RenderError::new(node, "`size` must be from 48 to 640"));
        }
        let thick = number("thickness", (size / 12.0).max(4.0));
        if !(2.0..=size / 4.0).contains(&thick) {
            return Err(RenderError::new(node, "`thickness` must be from 2 to a quarter of the size"));
        }
        let half = cx.prop_string(node, "shape").as_deref() == Some("half");
        let bands = bands(cx, node, min, max)?;
        let value = node.prop("value").cloned().unwrap_or_else(|| lit(min));
        let dynamic = cx.is_reactive(&value);
        // Room under the dial for `min` and `max`, and under a half's hub for the figure.
        let scale = cx.prop(node, "scale").is_none_or(|v| expr::truthy(&v));
        let needle = cx.prop(node, "needle").is_some_and(|v| expr::truthy(&v));
        let font = (size * 0.2).round().max(12.0);
        let hub = (thick * 0.6).max(3.0);
        let below = if half && needle { hub + 6.0 + font } else if scale { 18.0 } else { 0.0 };

        // The tone: given, or the band the value is in (worked out in the browser too).
        let in_band = |pick: &dyn Fn(&Band) -> Json, other: Json| -> Val {
            if dynamic {
                let v = val_expr(&value);
                expr_val(bands.iter().rev().skip(1).fold(Expr::Lit(bands.last().map_or(other.clone(), pick)), |rest, b| {
                    cond(bin(BinOp::Le, v.clone(), num(b.to)), Expr::Lit(pick(b)), rest)
                }))
            } else {
                let x = expr::to_num(&cx.eval(&value));
                lit(bands.iter().find(|b| x <= b.to).or(bands.last()).map_or(other, pick))
            }
        };
        let tone = match cx.prop_string(node, "tone") {
            Some(t) => lit(t),
            None if bands.is_empty() => lit("primary"),
            None => in_band(&|b| json!(b.tone), json!("primary")),
        };

        let unit = node.prop("unit").cloned();
        let decimals = match crate::widget::number(cx, node, "decimals") {
            Some(d) if !(0.0..=10.0).contains(&d) || d.fract() != 0.0 => return Err(RenderError::new(node, "`decimals` is a whole number from 0 to 10")),
            d => d,
        };
        let ticks = match crate::widget::number(cx, node, "ticks") {
            Some(t) if !(2.0..=20.0).contains(&t) || t.fract() != 0.0 => return Err(RenderError::new(node, "`ticks` is a whole number of parts from 2 to 20")),
            t => t.map(|t| t as u32),
        };
        let text = match node.prop("text") {
            Some(t) => t.clone(),
            None => match (&value, decimals) {
                (Val::Lit(j), d) => {
                    let x = expr::to_num(j);
                    let x = d.map_or(x, |d| expr::to_num(&Expr::Call(irender_core::expr::Func::Round, vec![num(x), num(d)]).eval(&|_| Json::Null)));
                    lit(irender_core::labels::number(cx.labels().lang.as_deref(), x))
                }
                (v, Some(d)) => expr_val(Expr::Call(irender_core::expr::Func::Round, vec![val_expr(v), num(d)])),
                (v, None) => v.clone(),
            },
        };
        let label = node.prop("label").or_else(|| node.prop("caption")).cloned();

        // ---- geometry: a circle of radius r about (c, c), the dial from a0 to a1 degrees
        // (clockwise from 3 o'clock), and the box down to the dial's lowest point.
        let c = size / 2.0;
        // Tick numbers go round the outside: the dial is drawn smaller to leave them room.
        let tick_labels = ticks.is_some() && cx.prop(node, "tickLabels").is_some_and(|v| expr::truthy(&v));
        let small = (font * 0.5).clamp(9.0, 13.0);
        let r = c - thick / 2.0 - 2.0 - if tick_labels { small * 1.4 } else { 0.0 };
        let (a0, a1, large) = if half { (180.0_f64, 360.0_f64, 0) } else { (150.0_f64, 390.0_f64, 1) };
        let at = |a: f64| (c + r * a.to_radians().cos(), c + r * a.to_radians().sin());
        let ((x0, y0), (x1, y1)) = (at(a0), at(a1));
        let height = (y0.max(c) + thick / 2.0 + 2.0).ceil();
        let arc = format!("M{} {}A{} {} 0 {large} 1 {} {}", n(x0), n(y0), n(r), n(r), n(x1), n(y1));

        let mut style = StyleBlock::new();
        style.push("--p", percent(&value, min, max));
        if let Some(t) = node.prop("target") {
            style.push("--t", percent(t, min, max));
        }
        let mut el = cx.element("div", node);
        el.base_style_ref(&GAUGE)
            .base_style(style)
            .attr("data-shape", if half { "half" } else { "arc" })
            .attr("data-with-needle", needle)
            .attr("data-tone", tone)
            .attr("role", "meter")
            .attr("aria-valuemin", lit(min))
            .attr("aria-valuemax", lit(max))
            .attr("aria-valuenow", value.clone());
        // "72 %, Warning": the figure, its unit and its band's name.
        let mut said = vec![val_expr(&value)];
        if let Some(u) = &unit {
            said.push(num(" "));
            said.push(val_expr(u));
        }
        if bands.iter().any(|b| b.label.is_some()) {
            let name = in_band(&|b| json!(b.label.clone().unwrap_or_default()), json!(""));
            said.push(cond(val_expr(&name), bin(BinOp::Add, num(", "), val_expr(&name)), num("")));
        }
        let said = said.into_iter().reduce(|a, b| bin(BinOp::Add, a, b)).expect("one");
        el.attr("aria-valuetext", if dynamic || unit.as_ref().is_some_and(|u| cx.is_reactive(u)) { expr_val(said) } else { lit(expr::stringify(&cx.eval(&expr_val(said)))) });
        if let Some(l) = &label {
            el.attr("aria-label", l.clone());
        }
        cx.open(&el, node)?;

        let mut dial = Element::new("div");
        dial.attr("data-dial", true).base_style(StyleBlock::new().set("width", format!("{}px", n(size))).set("height", format!("{}px", n(height + below))).set("font-size", format!("{}px", n(font))));
        cx.open(&dial, node)?;
        let mut svg = format!(r#"<svg width="{w}" height="{h}" viewBox="0 0 {w} {h}" fill="none" aria-hidden="true"><path data-track d="{arc}" stroke-width="{t}"/>"#, w = n(size), h = n(height), t = n(thick));
        let mut from = 0.0;
        for b in &bands {
            let to = (b.to - min) * 100.0 / (max - min);
            svg.push_str(&format!(
                r#"<path data-band d="{arc}" stroke-width="{t}" pathLength="100" stroke-dasharray="0 {a} {len} 200" style="--band:{color}"/>"#,
                t = n(thick),
                a = n(from),
                len = n(to - from),
                color = status::tone_color(&b.tone)
            ));
            from = to;
        }
        // Ticks: short marks just inside the dial, at each division.
        if let Some(k) = ticks {
            let (inner, outer) = (r - thick / 2.0 - 2.0, r - thick / 2.0 - 2.0 - (thick * 0.6).max(4.0));
            for i in 0..=k {
                let a = (a0 + (a1 - a0) * f64::from(i) / f64::from(k)).to_radians();
                svg.push_str(&format!(
                    r#"<line data-tick x1="{}" y1="{}" x2="{}" y2="{}" stroke-width="1.5" stroke-linecap="round"/>"#,
                    n(c + inner * a.cos()),
                    n(c + inner * a.sin()),
                    n(c + outer * a.cos()),
                    n(c + outer * a.sin())
                ));
            }
        }
        svg.push_str(&format!(r#"<path data-arc d="{arc}" stroke-width="{t}" pathLength="100"/>"#, t = n(thick)));
        // Turned by CSS about the centre; drawn pointing up.
        let origin = format!("transform-origin:{}px {}px", n(c), n(c));
        if node.prop("target").is_some() {
            svg.push_str(&format!(r#"<line data-target x1="{c}" y1="{y1}" x2="{c}" y2="{y2}" stroke-width="2" style="{origin}"/>"#, c = n(c), y1 = n(c - r - thick / 2.0 - 3.0), y2 = n(c - r + thick / 2.0 + 3.0)));
        }
        if needle {
            svg.push_str(&format!(
                r#"<g data-needle style="{origin}"><line x1="{c}" y1="{c}" x2="{c}" y2="{tip}" stroke-width="2.5" stroke-linecap="round"/><circle cx="{c}" cy="{c}" r="{hub}"/></g>"#,
                c = n(c),
                tip = n(c - r + thick),
                hub = n(hub)
            ));
        }
        svg.push_str("</svg>");
        cx.markup(&svg);

        // The figure: in the middle of an arc, or sitting on a half's baseline.
        let mut fig = Element::new("span");
        fig.attr("data-gauge-value", true)
            .attr("aria-hidden", "true")
            .base_style(StyleBlock::new().set("top", format!("{}px", n(match (half, needle) {
                (true, true) => c + hub + 4.0,
                (true, false) => c - thick,
                (false, true) => c + r * 0.45,
                (false, false) => c,
            }))));
        cx.open(&fig, node)?;
        let mut t = Element::new("span");
        t.text(text);
        cx.open(&t, node)?;
        cx.close("span");
        if let Some(u) = unit {
            let mut s = Element::new("span");
            s.attr("data-gauge-unit", true).text(u);
            cx.open(&s, node)?;
            cx.close("span");
        }
        cx.close("span");

        // Each tick's value, round the outside; the ends are the scale's when it's shown.
        // Crowded, every other one (or fewer) is numbered.
        if let Some(k) = ticks.filter(|_| tick_labels) {
            let lang = cx.labels().lang.clone();
            let radius = r + thick / 2.0 + small * 0.8;
            let value_at = |i: u32| {
                let v = min + (max - min) * f64::from(i) / f64::from(k);
                decimals.map_or((v * 1e6).round() / 1e6, |d| (v * 10f64.powi(d as i32)).round() / 10f64.powi(d as i32))
            };
            let widest = (0..=k).map(|i| irender_core::labels::number(lang.as_deref(), value_at(i)).chars().count()).max().unwrap_or(1) as f64;
            let room = radius * (a1 - a0).to_radians() / f64::from(k);
            let step = ((widest * small * 0.62 + 6.0) / room).ceil().max(1.0) as u32;
            for i in 0..=k {
                if (scale && (i == 0 || i == k)) || i % step != 0 {
                    continue;
                }
                let v = value_at(i);
                let a = (a0 + (a1 - a0) * f64::from(i) / f64::from(k)).to_radians();
                let mut e = Element::new("span");
                e.attr("data-gauge-tick", true)
                    .attr("aria-hidden", "true")
                    .base_style(StyleBlock::new().set("left", format!("{}px", n(c + radius * a.cos()))).set("top", format!("{}px", n(c + radius * a.sin()))).set("font-size", format!("{}px", n(small))))
                    .text(lit(irender_core::labels::number(lang.as_deref(), v)));
                cx.open(&e, node)?;
                cx.close("span");
            }
        }
        if scale {
            let lang = cx.labels().lang.clone();
            for (x, v) in [(x0, min), (x1, max)] {
                let mut e = Element::new("span");
                e.attr("data-gauge-end", true)
                    .attr("aria-hidden", "true")
                    .base_style(StyleBlock::new().set("left", format!("{}px", n(x))).set("top", format!("{}px", n(y0 + thick / 2.0 + 4.0))))
                    .text(lit(irender_core::labels::number(lang.as_deref(), v)));
                cx.open(&e, node)?;
                cx.close("span");
            }
        }
        cx.close("div");

        if let Some(cap) = node.prop("caption") {
            let mut e = Element::new("div");
            e.attr("data-gauge-caption", true).attr("aria-hidden", "true").text(cap.clone());
            cx.open(&e, node)?;
            cx.close("div");
        }
        cx.close("div");
        Ok(())
    }
}
