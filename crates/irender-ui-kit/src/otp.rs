//! `ui-input-otp`: a one-time code in a row of boxes, after shadcn/ui's InputOTP.
//!
//! One real text field lies over the boxes, transparent: typing, pasting and the phone's
//! "code from Messages" suggestion (`autocomplete="one-time-code"`) all go to it, it's a
//! single stop in the tab order, and screen readers read it as one field. The boxes
//! draw its characters and where the caret is. `otp.js` keeps out characters the
//! `pattern` doesn't allow, writes the code to `bind`, and fires `complete` when it's full.

use crate::{MUTED, PRIMARY, RING, TEXT, instance_id, lit};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("otp.js");

pub(crate) const PATTERNS: &[&str] = &["digits", "letters", "alphanumeric"];

pub struct InputOtp;

const PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key holding the code, as text"),
    PropDef::new("length", PropKind::Number, "How many characters (default 6)"),
    PropDef::new("pattern", PropKind::Enum(PATTERNS), "What may be typed: `digits` (default), `letters` or `alphanumeric`; letters are kept in capitals"),
    PropDef::new("groups", PropKind::Any, "Boxes per group, with a dash between groups, e.g. `[3, 3]` (default: one group)"),
    PropDef::new("mask", PropKind::Bool, "Show dots rather than the characters, as for a PIN"),
    PropDef::new("label", PropKind::Any, "Accessible name, when no visible label (such as a `ui-field`'s) names it"),
    PropDef::new("name", PropKind::String, "Form field name"),
    PropDef::new("disabled", PropKind::Any, "Boolean or expression"),
    PropDef::new("invalid", PropKind::Any, "Mark it wrong, e.g. `{{ codeError != null }}`"),
    PropDef::new("id", PropKind::Any, "The field's element id (default: derived from the document)"),
];

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

static ROOT: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("position", "relative"), ("display", "inline-flex"), ("align-items", "center"), ("gap", sp!(8)), ("color", TEXT)])
        // The real field: over the boxes, taking every click and key, drawn by them.
        .nest(
            "& [data-otp-input]",
            pairs(&[
                ("position", "absolute"),
                ("inset", "0"),
                ("width", "100%"),
                ("height", "100%"),
                ("margin", "0"),
                ("padding", "0"),
                ("border", "0"),
                ("outline", "none"),
                ("background", "transparent"),
                ("color", "transparent"),
                ("caret-color", "transparent"),
                ("font-size", "16px"),
                ("letter-spacing", "-0.5em"),
                ("cursor", "text"),
                ("z-index", "1"),
            ]),
        )
        .nest("& [data-otp-input]::selection", pairs(&[("background", "transparent")]))
        .nest("& [data-otp-input]:disabled", pairs(&[("cursor", "not-allowed")]))
        .nest("&:has([data-otp-input]:disabled)", pairs(&[("opacity", "0.5")]))
        .nest("& [data-otp-group]", pairs(&[("display", "flex")]))
        .nest(
            "& [data-otp-slot]",
            pairs(&[
                ("position", "relative"),
                ("display", "flex"),
                ("align-items", "center"),
                ("justify-content", "center"),
                ("width", sp!(40)),
                ("height", sp!(40)),
                ("margin-left", "-1px"),
                ("border", "1px solid var(--color-border)"),
                ("background", "transparent"),
                ("font-size", fs!(16)),
                ("font-weight", "500"),
                ("font-variant-numeric", "tabular-nums"),
                ("transition", "border-color .15s, box-shadow .15s"),
            ]),
        )
        .nest("& [data-otp-slot]:first-child", pairs(&[("margin-left", "0"), ("border-radius", "var(--radius-md) 0 0 var(--radius-md)")]))
        .nest("& [data-otp-slot]:last-child", pairs(&[("border-radius", "0 var(--radius-md) var(--radius-md) 0")]))
        .nest("& [data-otp-slot]:only-child", pairs(&[("border-radius", "var(--radius-md)")]))
        .nest(
            "& [data-otp-slot][data-active]",
            pairs(&[
                ("z-index", "1"),
                ("border-color", RING),
                ("box-shadow", "0 0 0 3px color-mix(in srgb, var(--color-ring, var(--color-muted)) 50%, transparent)"),
            ]),
        )
        .nest("&[data-invalid] [data-otp-slot]", pairs(&[("border-color", "var(--color-destructive, #dc2626)")]))
        // The caret, in the active box while it's empty.
        .nest(
            "& [data-otp-slot][data-active][data-empty]::after",
            pairs(&[
                ("content", "''"),
                ("width", "1px"),
                ("height", "16px"),
                ("background", TEXT),
                ("animation", "ui-otp-caret 1s step-end infinite"),
            ]),
        )
        .nest("& [data-otp-sep]", pairs(&[("width", "10px"), ("height", "2px"), ("border-radius", "1px"), ("background", MUTED)]))
        .nest("& [data-otp-slot] [data-otp-dot]", pairs(&[("width", "8px"), ("height", "8px"), ("border-radius", "50%"), ("background", PRIMARY)]))
});

const CARET: &str = "@keyframes ui-otp-caret {\n  50% { opacity: 0; }\n}\n@media (prefers-reduced-motion: reduce) {\n  [data-otp-slot]::after { animation: none !important; }\n}";

impl Component for InputOtp {
    fn name(&self) -> &'static str {
        "ui-input-otp"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A one-time code in a row of boxes: type, paste or take it from the phone's suggestion; fires `complete` when full.",
            props: PROPS,
            parts: &["input"],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let length = cx.prop(node, "length").map_or(6.0, |v| expr::to_num(&v));
        if !(1.0..=12.0).contains(&length) || length.fract() != 0.0 {
            return Err(RenderError::new(node, "`length` must be a whole number from 1 to 12"));
        }
        let length = length as usize;
        let groups: Vec<usize> = match cx.prop(node, "groups") {
            None | Some(Json::Null) => vec![length],
            Some(Json::Array(g)) => g.iter().map(|v| expr::to_num(v).max(0.0) as usize).collect(),
            Some(_) => return Err(RenderError::new(node, "`groups` must be a list of numbers, e.g. `[3, 3]`")),
        };
        if groups.iter().sum::<usize>() != length || groups.contains(&0) {
            return Err(RenderError::new(node, format!("`groups` must add up to `length` ({length})")));
        }
        let pattern = cx.prop_string(node, "pattern").unwrap_or_else(|| "digits".into());
        let mask = cx.prop(node, "mask").is_some_and(|v| expr::truthy(&v));
        let key = cx.prop_string(node, "bind");
        let value = match &key {
            Some(k) => expr::stringify(&cx.lookup(std::slice::from_ref(k))),
            None => String::new(),
        };
        let chars: Vec<char> = value.chars().take(length).collect();

        cx.global_js("ui-input-otp", SCRIPT);
        cx.global_css("ui-otp-caret", CARET);
        let mut root = cx.element("div", node);
        root.base_style_ref(&ROOT).behavior("ui-input-otp", json!({ "bind": key, "length": length, "pattern": pattern, "mask": mask }));
        if let Some(i) = node.prop("invalid") {
            root.attr("data-invalid", i.clone());
        }
        cx.open(&root, node)?;

        let id = instance_id(cx, node, "otp");
        let mut input = cx.part("input", node, "input");
        input
            .attr("type", if mask { "password" } else { "text" })
            .attr("id", id)
            .attr("data-otp-input", true)
            .attr("value", value.clone())
            .attr("maxlength", length.to_string())
            .attr("autocomplete", "one-time-code")
            .attr("inputmode", if pattern == "digits" { "numeric" } else { "text" })
            .attr("autocapitalize", "characters")
            .attr("spellcheck", "false");
        // Named by a visible label (a `ui-field`'s) or by `label`, which would override it.
        if let Some(l) = node.prop("label") {
            input.attr("aria-label", l.clone());
        }
        if pattern == "digits" {
            input.attr("pattern", format!("[0-9]{{{length}}}"));
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

        // The boxes, drawn from the value; the behaviour redraws them as it changes.
        let mut at = 0;
        for (g, size) in groups.iter().enumerate() {
            if g > 0 {
                let mut sep = Element::new("div");
                sep.attr("data-otp-sep", true).attr("aria-hidden", "true");
                cx.open(&sep, node)?;
                cx.close("div");
            }
            let mut group = Element::new("div");
            group.attr("data-otp-group", true).attr("aria-hidden", "true");
            cx.open(&group, node)?;
            for _ in 0..*size {
                let mut slot = Element::new("div");
                slot.attr("data-otp-slot", true);
                match chars.get(at) {
                    Some(c) if !mask => {
                        slot.text(lit(c.to_string()));
                    }
                    Some(_) => {}
                    None => {
                        slot.attr("data-empty", true);
                    }
                }
                cx.open(&slot, node)?;
                if mask && chars.get(at).is_some() {
                    let mut dot = Element::new("span");
                    dot.attr("data-otp-dot", true);
                    cx.open(&dot, node)?;
                    cx.close("span");
                }
                cx.close("div");
                at += 1;
            }
            cx.close("div");
        }
        cx.close("div");
        Ok(())
    }
}
