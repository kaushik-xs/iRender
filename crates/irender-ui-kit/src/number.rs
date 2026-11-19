//! `ui-number-input`: a number or an amount, shown with the locale's digit grouping as
//! it's typed ("1,25,000" in en-IN), with a currency symbol or a prefix and suffix, and
//! optional − and + buttons. State gets a plain number (null when empty); `number.js`
//! formats, parses, steps and clamps.

use crate::{MUTED, TEXT, forms, instance_id, lit};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::json;
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("number.js");

pub struct NumberInput;

const PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key for the number (null when empty)").required(),
    PropDef::new("currency", PropKind::String, "An ISO currency code, e.g. `INR`: its symbol goes before the number, and its decimals apply"),
    PropDef::new("prefix", PropKind::Any, "Text before the number, e.g. `~` (instead of a currency's symbol)"),
    PropDef::new("suffix", PropKind::Any, "Text after the number, e.g. `kg` or `%`"),
    PropDef::new("locale", PropKind::String, "How the number is written, e.g. `en-IN` (default: the document's `lang`, else the browser's)"),
    PropDef::new("min", PropKind::Number, "The smallest it can be; leaving the field raises it to this"),
    PropDef::new("max", PropKind::Number, "The largest it can be"),
    PropDef::new("step", PropKind::Number, "How much Up, Down and the buttons change it (default 1)"),
    PropDef::new("decimals", PropKind::Number, "Digits after the decimal point (default: the currency's, or as typed)"),
    PropDef::new("stepper", PropKind::Bool, "− and + buttons beside it"),
    PropDef::new("placeholder", PropKind::Any, "Shown while it's empty"),
    PropDef::new("label", PropKind::Any, "Accessible name, when no visible label (such as a `ui-field`'s) names it"),
    PropDef::new("name", PropKind::String, "Form field name"),
    PropDef::new("disabled", PropKind::Any, "Boolean or expression"),
    PropDef::new("invalid", PropKind::Any, "Mark it wrong, e.g. `{{ amount > budget }}`"),
    PropDef::new("id", PropKind::Any, "The input's id (default: derived from the document)"),
];

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

static ROOT: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("display", "flex"),
        ("align-items", "center"),
        ("box-sizing", "border-box"),
        ("width", "100%"),
        ("height", "36px"),
        ("border", "1px solid var(--color-border)"),
        ("border-radius", "var(--radius-md)"),
        ("background", "transparent"),
        ("color", TEXT),
        ("box-shadow", "0 1px 2px rgb(0 0 0 / 0.05)"),
        ("overflow", "hidden"),
        ("transition", "border-color .15s, box-shadow .15s"),
    ])
    .nest("&:focus-within", forms::focus_ring().set("border-color", crate::RING))
    .nest("&:has([aria-invalid=true])", pairs(&[("border-color", "var(--color-destructive, #dc2626)")]))
    .nest("&:has(input:disabled)", pairs(&[("opacity", "0.5"), ("cursor", "not-allowed")]))
    .nest(
        "& input",
        pairs(&[
            ("flex", "1 1 auto"),
            ("min-width", "0"),
            ("height", "100%"),
            ("padding", sp!(0, 12)),
            ("border", "0"),
            ("outline", "none"),
            ("background", "transparent"),
            ("color", "inherit"),
            ("font", "inherit"),
            ("font-size", fs!(14)),
            ("font-variant-numeric", "tabular-nums"),
            ("text-align", "end"),
        ]),
    )
    .nest("& input::placeholder", pairs(&[("color", MUTED)]))
    .nest("& [data-num-prefix] + input", pairs(&[("padding-inline-start", sp!(6))]))
    .nest("& input:has(+ [data-num-suffix])", pairs(&[("padding-inline-end", sp!(6))]))
    .nest("& :is([data-num-prefix], [data-num-suffix])", pairs(&[("flex", "none"), ("color", MUTED), ("font-size", fs!(14)), ("white-space", "nowrap")]))
    .nest("& [data-num-prefix]", pairs(&[("padding-inline-start", sp!(12))]))
    .nest("& [data-num-suffix]", pairs(&[("padding-inline-end", sp!(12))]))
    .nest(
        "& [data-num-step]",
        pairs(&[
            ("flex", "none"),
            ("display", "grid"),
            ("place-items", "center"),
            ("width", "34px"),
            ("height", "100%"),
            ("padding", "0"),
            ("border", "0"),
            ("background", "color-mix(in srgb, var(--color-text) 4%, transparent)"),
            ("color", TEXT),
            ("font", "inherit"),
            ("font-size", fs!(16)),
            ("cursor", "pointer"),
        ]),
    )
    .nest("& [data-num-step='-1']", pairs(&[("border-inline-end", "1px solid var(--color-border)")]))
    .nest("& [data-num-step='1']", pairs(&[("border-inline-start", "1px solid var(--color-border)")]))
    .nest("& [data-num-step]:hover", pairs(&[("background", "color-mix(in srgb, var(--color-text) 9%, transparent)")]))
    .nest("& [data-num-step]:focus-visible", forms::focus_ring().set("outline-offset", "-2px"))
});

/// The symbol for the common currencies, so the server's page shows it before the
/// script runs (the browser writes the rest with its own data).
pub(crate) fn symbol(code: &str) -> String {
    match code {
        "INR" => "₹",
        "USD" => "$",
        "EUR" => "€",
        "GBP" => "£",
        "JPY" | "CNY" => "¥",
        "AUD" => "A$",
        "CAD" => "CA$",
        "SGD" => "S$",
        "AED" => "AED",
        _ => code,
    }
    .to_string()
}

impl Component for NumberInput {
    fn name(&self) -> &'static str {
        "ui-number-input"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A number or an amount, grouped as it's typed (`1,25,000`), with a currency symbol, prefix or suffix, Up and Down to \
                  step it, and optional − and + buttons. State gets a plain number.",
            props: PROPS,
            parts: &["input"],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let key = cx.prop_string(node, "bind").ok_or_else(|| RenderError::new(node, "`ui-number-input` needs `bind`"))?;
        let num = |cx: &RenderCx<'_>, n: &str| cx.prop(node, n).filter(|v| !v.is_null()).map(|v| expr::to_num(&v));
        let (min, max, step, decimals) = (num(cx, "min"), num(cx, "max"), num(cx, "step"), num(cx, "decimals"));
        if let (Some(a), Some(b)) = (min, max)
            && a > b
        {
            return Err(RenderError::new(node, "`min` is more than `max`"));
        }
        if step.is_some_and(|s| s <= 0.0) {
            return Err(RenderError::new(node, "`step` must be more than 0"));
        }
        if decimals.is_some_and(|d| !(0.0..=10.0).contains(&d) || d.fract() != 0.0) {
            return Err(RenderError::new(node, "`decimals` must be a whole number from 0 to 10"));
        }
        let currency = cx.prop_string(node, "currency").map(|c| c.to_uppercase());
        if currency.as_ref().is_some_and(|c| c.len() != 3 || !c.chars().all(|ch| ch.is_ascii_alphabetic())) {
            return Err(RenderError::new(node, "`currency` must be a three-letter code, e.g. `INR`"));
        }
        let stepper = cx.prop(node, "stepper").is_some_and(|v| expr::truthy(&v));
        let current = cx.lookup(std::slice::from_ref(&key));
        cx.global_js("ui-number-input", SCRIPT);
        let config = json!({
            "bind": key, "currency": currency, "locale": crate::locale(cx, node),
            "min": min, "max": max, "step": step, "decimals": decimals.map(|d| d as i64),
        });
        let mut root = cx.element("div", node);
        root.base_style_ref(&ROOT).behavior("ui-number-input", config);
        cx.open(&root, node)?;
        let id = instance_id(cx, node, "number");
        let label_text = node.prop("label").map(|l| expr::stringify(&cx.eval(l)));
        if stepper {
            step_button(cx, node, -1, label_text.as_deref())?;
        }
        let prefix = node.prop("prefix").cloned().or_else(|| currency.as_ref().map(|c| lit(symbol(c))));
        if let Some(p) = prefix {
            let mut s = Element::new("span");
            s.attr("data-num-prefix", true).attr("aria-hidden", "true").text(p);
            cx.open(&s, node)?;
            cx.close("span");
        }
        let mut input = cx.part("input", node, "input");
        input
            .attr("type", "text")
            .attr("inputmode", if decimals == Some(0.0) { "numeric" } else { "decimal" })
            .attr("id", id)
            .attr("data-num-input", true)
            .attr("autocomplete", "off")
            .attr("value", if current.is_null() { String::new() } else { expr::stringify(&current) });
        if let Some(l) = node.prop("label") {
            input.attr("aria-label", l.clone());
        }
        if let Some(c) = &currency {
            // Read with the amount, since the symbol is hidden from screen readers.
            input.attr("aria-description", format!("in {c}"));
        }
        if let Some(p) = node.prop("placeholder") {
            input.attr("placeholder", p.clone());
        }
        if let Some(n) = cx.prop_string(node, "name") {
            input.attr("name", n);
        }
        if let Some(d) = node.prop("disabled") {
            input.attr("disabled", d.clone());
        }
        if let Some(i) = node.prop("invalid") {
            input.attr("aria-invalid", i.clone());
        }
        cx.open(&input, node)?;
        if let Some(s) = node.prop("suffix") {
            let mut el = Element::new("span");
            el.attr("data-num-suffix", true).text(s.clone());
            cx.open(&el, node)?;
            cx.close("span");
        }
        if stepper {
            step_button(cx, node, 1, label_text.as_deref())?;
        }
        cx.close("div");
        Ok(())
    }
}

fn step_button(cx: &mut RenderCx<'_>, node: &Node, dir: i32, label: Option<&str>) -> Result<(), RenderError> {
    let mut b = Element::new("button");
    let aria = match (dir < 0, label) {
        (true, Some(l)) => cx.label_with("Decrease {name}", &[("name", l)]),
        (false, Some(l)) => cx.label_with("Increase {name}", &[("name", l)]),
        (true, None) => cx.label("Decrease"),
        (false, None) => cx.label("Increase"),
    };
    b.attr("type", "button")
        .attr("data-num-step", dir.to_string())
        .attr("tabindex", lit(-1))
        .attr("aria-label", aria)
        .text(lit(if dir < 0 { "−" } else { "+" }));
    if let Some(d) = node.prop("disabled") {
        b.attr("disabled", d.clone());
    }
    cx.open(&b, node)?;
    cx.close("button");
    Ok(())
}
