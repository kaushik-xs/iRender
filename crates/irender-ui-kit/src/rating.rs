//! `ui-rating` (stars to pick, or to show an average) and `ui-password-checklist` (a
//! password's rules, ticked as they're met). The rating is a group of radio buttons,
//! so the arrow keys move along it; `rating.js` keeps it in step with state. A
//! read-only rating needs no script: a second row of stars is cut to the value's width.
//! `password.js` checks the rules as the password changes.

use crate::{MUTED, TEXT, forms, instance_id, lit};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, StyleValue, Val, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const RATING_SCRIPT: &str = include_str!("rating.js");
const PASSWORD_SCRIPT: &str = include_str!("password.js");

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

const STAR: &str = r#"<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 2.8l2.8 5.9 6.4.8-4.7 4.4 1.2 6.4L12 17.2l-5.7 3.1 1.2-6.4-4.7-4.4 6.4-.8z"/></svg>"#;
const AMBER: &str = "var(--color-rating, #f59e0b)";

// ---------------------------------------------------------------- rating

pub struct Rating;

const RATING_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key for the rating, a number (0 or null: none)"),
    PropDef::new("value", PropKind::Any, "A rating to show, read-only, e.g. an average `{{ product.rating }}`: fractions fill part of a star"),
    PropDef::new("max", PropKind::Number, "How many stars (default 5)"),
    PropDef::new("half", PropKind::Bool, "Half stars can be picked"),
    PropDef::new("size", PropKind::Number, "Star size in px (default 20)"),
    PropDef::new("label", PropKind::Any, "Names it for screen readers (default `Rating`)"),
    PropDef::new("clearable", PropKind::Bool, "Clicking the chosen star again clears the rating (default true)"),
    PropDef::new("disabled", PropKind::Any, "Boolean or expression"),
    PropDef::new("name", PropKind::String, "Form field name"),
];

static RATING: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("display", "inline-flex"), ("align-items", "center"), ("gap", "2px"), ("color", TEXT), ("line-height", "0")])
        .nest("& svg", pairs(&[("width", "var(--star, 20px)"), ("height", "var(--star, 20px)"), ("fill", "currentColor")]))
        // Picking: each star is a label around a radio, which stays focusable.
        .nest("& label", pairs(&[("position", "relative"), ("display", "inline-flex"), ("padding", "2px"), ("border-radius", "var(--radius-sm)"), ("color", "color-mix(in srgb, var(--color-text) 18%, transparent)"), ("cursor", "pointer"), ("transition", "color .1s, transform .1s")]))
        .nest("& label[data-on]", pairs(&[("color", AMBER)]))
        .nest("&:not([data-ro]) label:hover", pairs(&[("transform", "scale(1.12)")]))
        .nest("& label:has(input:focus-visible)", forms::focus_ring())
        .nest("& input", pairs(&[("position", "absolute"), ("inset", "0"), ("opacity", "0"), ("margin", "0"), ("cursor", "pointer")]))
        .nest("&:has(input:disabled)", pairs(&[("opacity", "0.5")]))
        .nest("&:has(input:disabled) label", pairs(&[("cursor", "not-allowed")]))
        .nest("@media (prefers-reduced-motion: reduce)", pairs(&[]).nest("& label", pairs(&[("transition", "none")])))
        // Half stars: a label per half, the second showing its star's other side.
        .nest("&[data-half] label", pairs(&[("width", "calc(var(--star, 20px) / 2)"), ("padding", "2px 0"), ("overflow", "hidden"), ("border-radius", "0")]))
        .nest("&[data-half] label svg", pairs(&[("flex", "none")]))
        .nest("&[data-half] label:nth-child(even)", pairs(&[("margin-inline-end", "4px")]))
        .nest("&[data-half] label:nth-child(even) svg", pairs(&[("margin-inline-start", "calc(var(--star, 20px) / -2)")]))
        .nest("&[data-half]:not([data-ro]) label:hover", pairs(&[("transform", "none")]))
        .nest("&[data-half]", pairs(&[("gap", "0")]))
        // Showing: the filled row cut to `--ui-value` of the way across.
        .nest("& [data-stars]", pairs(&[("position", "relative"), ("display", "inline-flex"), ("gap", "2px")]))
        .nest("& [data-stars] > span", pairs(&[("display", "inline-flex"), ("gap", "2px")]))
        .nest("& [data-empty]", pairs(&[("color", "color-mix(in srgb, var(--color-text) 18%, transparent)")]))
        .nest(
            "& [data-full]",
            pairs(&[("position", "absolute"), ("inset", "0"), ("color", AMBER), ("clip-path", "inset(0 calc(100% - min(100%, max(0%, var(--ui-value) * 1%))) 0 0)")]),
        )
        .nest("& [data-score]", pairs(&[("margin-inline-start", gap!(6)), ("color", MUTED), ("font-size", fs!(13)), ("line-height", "1"), ("font-variant-numeric", "tabular-nums")]))
});

impl Component for Rating {
    fn name(&self) -> &'static str {
        "ui-rating"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Stars: to pick a rating (`bind`), from the mouse or the arrow keys, or to show one (`value`), fractions included.",
            props: RATING_PROPS,
            parts: &[],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let max = cx.prop(node, "max").map_or(5.0, |v| expr::to_num(&v));
        if !(1.0..=10.0).contains(&max) || max.fract() != 0.0 {
            return Err(RenderError::new(node, "`max` must be a whole number from 1 to 10"));
        }
        let max = max as usize;
        let size = cx.prop(node, "size").map_or(20.0, |v| expr::to_num(&v));
        let key = cx.prop_string(node, "bind");
        let shown = node.prop("value");
        if key.is_some() == shown.is_some() {
            return Err(RenderError::new(node, "a rating takes `bind` (to pick one) or `value` (to show one)"));
        }
        let label = node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Rating")));
        let mut style = StyleBlock::new();
        style.push("--star", StyleValue::Static(format!("{size}px")));

        if let Some(v) = shown {
            // Shown, not picked: one image, named with the value.
            let (percent, text) = match v {
                Val::Tpl(t) => (StyleValue::Dynamic { tpl: percent_tpl(t, max), px: false }, Val::Tpl(t.clone())),
                Val::Lit(j) => {
                    let n = expr::to_num(j);
                    (StyleValue::Static((n * 100.0 / max as f64).to_string()), lit(format!("{}", (n * 10.0).round() / 10.0)))
                }
            };
            style.push("--ui-value", percent);
            let mut el = cx.element("span", node);
            el.base_style_ref(&RATING).base_style(style).attr("data-ro", true).attr("role", "img");
            el.attr("aria-label", aria_value(cx, v, max, &label));
            cx.open(&el, node)?;
            let mut stars = Element::new("span");
            stars.attr("data-stars", true);
            cx.open(&stars, node)?;
            for which in ["data-empty", "data-full"] {
                let mut row = Element::new("span");
                row.attr(which, true);
                cx.open(&row, node)?;
                for _ in 0..max {
                    cx.markup(STAR);
                }
                cx.close("span");
            }
            cx.close("span");
            let mut score = Element::new("span");
            score.attr("data-score", true).attr("aria-hidden", "true").text(text);
            cx.open(&score, node)?;
            cx.close("span");
            cx.close("span");
            return Ok(());
        }

        let key = key.expect("checked");
        let current = expr::to_num(&cx.lookup(std::slice::from_ref(&key)));
        let name = cx.prop_string(node, "name").unwrap_or_else(|| expr::stringify(&cx.eval(&instance_id(cx, node, "rating"))));
        cx.global_js("ui-rating", RATING_SCRIPT);
        let clearable = cx.prop(node, "clearable").is_none_or(|v| expr::truthy(&v));
        let mut el = cx.element("div", node);
        el.base_style_ref(&RATING)
            .base_style(style)
            .attr("role", "radiogroup")
            .attr("aria-label", label)
            .behavior("ui-rating", json!({ "bind": key, "clearable": clearable }));
        let half = cx.prop(node, "half").is_some_and(|v| expr::truthy(&v));
        if half {
            el.attr("data-half", true);
        }
        cx.open(&el, node)?;
        let steps = if half { max * 2 } else { max };
        for i in 1..=steps {
            let value = if half { i as f64 / 2.0 } else { i as f64 };
            let mut l = Element::new("label");
            if value <= current {
                l.attr("data-on", true);
            }
            cx.open(&l, node)?;
            let mut input = Element::new("input");
            input
                .attr("type", "radio")
                .attr("name", name.clone())
                .attr("value", value.to_string())
                .attr("aria-label", if value == 1.0 { "1 star".to_string() } else { format!("{value} stars") });
            if current == value {
                input.attr("checked", true);
            }
            if let Some(d) = node.prop("disabled") {
                input.attr("disabled", d.clone());
            }
            cx.open(&input, node)?;
            cx.markup(STAR);
            cx.close("label");
        }
        cx.close("div");
        Ok(())
    }
}

/// `{{ v }}` as a percentage of `max` stars, for the clip.
fn percent_tpl(t: &irender_core::Template, max: usize) -> irender_core::Template {
    use irender_core::{Part, expr::BinOp};
    let e = crate::val_expr(&Val::Tpl(t.clone()));
    let e = crate::bin(BinOp::Div, crate::bin(BinOp::Mul, e, crate::num(100)), crate::num(max as u64));
    irender_core::Template { parts: vec![Part::Expr(e)] }
}

/// "4.5 out of 5 stars", following state when the value does.
fn aria_value(cx: &RenderCx<'_>, v: &Val, max: usize, label: &Val) -> Val {
    let label_text = match label {
        Val::Lit(j) => expr::stringify(j),
        _ => cx.label("Rating"),
    };
    let text = |value: &str| cx.label_with("{label}: {value} out of {max}", &[("label", &label_text), ("value", value), ("max", &max.to_string())]);
    match v {
        Val::Tpl(t) => {
            // The value follows state: the text around it stays.
            let whole = text("\u{0}");
            let (before, after) = whole.split_once('\u{0}').unwrap_or((&whole, ""));
            let mut t = t.clone();
            t.parts.insert(0, irender_core::Part::Lit(before.to_string()));
            t.parts.push(irender_core::Part::Lit(after.to_string()));
            Val::Tpl(t)
        }
        Val::Lit(j) => lit(text(&((expr::to_num(j) * 10.0).round() / 10.0).to_string())),
    }
}

// ---------------------------------------------------------------- password checklist

pub struct PasswordChecklist;

const CHECKLIST_PROPS: &[PropDef] = &[
    PropDef::new("value", PropKind::Any, "The password, e.g. `{{ password }}`").required(),
    PropDef::new(
        "rules",
        PropKind::Any,
        "The rules, in order: `length` (8 characters; `{ length: 12 }` for more), `lower`, `upper`, `number`, `symbol`, or \
         `{ label, pattern }` with a regular expression. Default: length, lower, upper, number, symbol",
    ),
    PropDef::new("bindValid", PropKind::StateKey, "State key that gets true while every rule is met, e.g. for a button's `disabled`"),
    PropDef::new("meter", PropKind::Bool, "A strength bar above the rules (default true)"),
    PropDef::new("label", PropKind::Any, "Names the list for screen readers (default `Password requirements`)"),
];

static CHECKLIST: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(8)), ("color", MUTED), ("font-size", fs!(13))])
        .nest("& [data-meter]", pairs(&[("display", "grid"), ("grid-template-columns", "repeat(4, 1fr)"), ("gap", gap!(4))]))
        .nest("& [data-meter] span", pairs(&[("height", "4px"), ("border-radius", "999px"), ("background", "color-mix(in srgb, var(--color-text) 12%, transparent)"), ("transition", "background .2s")]))
        .nest("&[data-strength='1'] [data-meter] span:nth-child(-n+1)", pairs(&[("background", "var(--color-destructive, #dc2626)")]))
        .nest("&[data-strength='2'] [data-meter] span:nth-child(-n+2)", pairs(&[("background", "var(--color-warning, #d97706)")]))
        .nest("&[data-strength='3'] [data-meter] span:nth-child(-n+3)", pairs(&[("background", "var(--color-warning, #d97706)")]))
        .nest("&[data-strength='4'] [data-meter] span", pairs(&[("background", "var(--color-success, #16a34a)")]))
        .nest("& [data-strength-text]", pairs(&[("font-weight", "var(--font-weight-medium, 500)")]))
        .nest("& ul", pairs(&[("margin", "0"), ("padding", "0"), ("list-style", "none"), ("display", "flex"), ("flex-direction", "column"), ("gap", gap!(4))]))
        .nest("& li", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", gap!(8)), ("transition", "color .15s")]))
        .nest("& li::before", pairs(&[("content", "''"), ("flex", "none"), ("width", "14px"), ("height", "14px"), ("border-radius", "50%"), ("border", "1.5px solid currentColor"), ("box-sizing", "border-box")]))
        .nest("& li[data-met]", pairs(&[("color", "var(--color-success, #16a34a)")]))
        .nest(
            "& li[data-met]::before",
            pairs(&[
                ("content", "'✓'"),
                ("display", "grid"),
                ("place-items", "center"),
                ("border", "0"),
                ("background", "var(--color-success, #16a34a)"),
                ("color", "var(--color-surface, #fff)"),
                ("font-size", fs!(10)),
                ("font-weight", "var(--font-weight-bold, 700)"),
                ("line-height", "1"),
            ]),
        )
});

/// A rule as `password.js` takes it: `{ label, min }` or `{ label, pattern }`.
fn rule(cx: &RenderCx<'_>, node: &Node, r: &Json) -> Result<Json, RenderError> {
    let preset = |name: &str, n: f64| -> Option<Json> {
        Some(match name {
            "length" => json!({ "label": cx.label_with("At least {n} characters", &[("n", &n.to_string())]), "min": n }),
            "lower" => json!({ "label": cx.label("A lowercase letter"), "pattern": "\\p{Ll}" }),
            "upper" => json!({ "label": cx.label("An uppercase letter"), "pattern": "\\p{Lu}" }),
            "number" => json!({ "label": cx.label("A number"), "pattern": "\\d" }),
            "symbol" => json!({ "label": cx.label("A symbol, like ! or #"), "pattern": "[^\\p{L}\\p{N}\\s]" }),
            _ => return None,
        })
    };
    let bad = || RenderError::new(node, "each rule is `length`, `lower`, `upper`, `number`, `symbol`, `{ length: n }` or `{ label, pattern }`");
    match r {
        Json::String(s) => preset(s, 8.0).ok_or_else(bad),
        Json::Object(m) if m.contains_key("length") => {
            let n = m.get("length").map(expr::to_num).unwrap_or(8.0);
            let mut out = preset("length", n).expect("known");
            if let Some(l) = m.get("label") {
                out["label"] = l.clone();
            }
            Ok(out)
        }
        Json::Object(m) if m.contains_key("pattern") && m.contains_key("label") => Ok(json!({ "label": m["label"], "pattern": m["pattern"] })),
        _ => Err(bad()),
    }
}

impl Component for PasswordChecklist {
    fn name(&self) -> &'static str {
        "ui-password-checklist"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A password's rules, ticked as they're met, with a strength bar. Put it under the password input; `bindValid` says when all are met.",
            props: CHECKLIST_PROPS,
            parts: &[],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let rules: Vec<Json> = match cx.prop(node, "rules") {
            None | Some(Json::Null) => ["length", "lower", "upper", "number", "symbol"].iter().map(|r| rule(cx, node, &json!(r))).collect::<Result<_, _>>()?,
            Some(Json::Array(rs)) if !rs.is_empty() => rs.iter().map(|r| rule(cx, node, r)).collect::<Result<_, _>>()?,
            Some(_) => return Err(RenderError::new(node, "`rules` must be a list")),
        };
        let value = node.prop("value").cloned().unwrap_or_else(|| lit(""));
        let meter = cx.prop(node, "meter").is_none_or(|v| expr::truthy(&v));
        cx.global_js("ui-password-checklist", PASSWORD_SCRIPT);
        let config = json!({
            "value": crate::data_table::dynamic(cx, node, "value", &value)?,
            "rules": rules,
            "bindValid": cx.prop_string(node, "bindValid"),
        });
        let mut el = cx.element("div", node);
        el.base_style_ref(&CHECKLIST).attr("data-strength", "0").behavior("ui-password-checklist", config);
        cx.open(&el, node)?;
        if meter {
            let mut m = Element::new("div");
            m.attr("data-meter", true).attr("aria-hidden", "true");
            cx.open(&m, node)?;
            for _ in 0..4 {
                cx.open(&Element::new("span"), node)?;
                cx.close("span");
            }
            cx.close("div");
            let mut t = Element::new("div");
            t.attr("data-strength-text", true).attr("aria-live", "polite").text(lit(cx.label("Enter a password")));
            cx.open(&t, node)?;
            cx.close("div");
        }
        let mut ul = Element::new("ul");
        ul.attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Password requirements"))));
        cx.open(&ul, node)?;
        for r in &rules {
            let mut li = Element::new("li");
            li.text(lit(expr::stringify(&r["label"])));
            cx.open(&li, node)?;
            // Read with each rule: met or not, for screen readers.
            let mut s = Element::new("span");
            s.attr("data-state", true).base_style(pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("overflow", "hidden"), ("clip-path", "inset(50%)")])).text(lit(", not met"));
            cx.open(&s, node)?;
            cx.close("span");
            cx.close("li");
        }
        cx.close("ul");
        cx.close("div");
        Ok(())
    }
}
