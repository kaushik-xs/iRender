//! `ui-form` (rules checked as fields are left and on submit, unsaved changes, and a
//! question before leaving with them), `ui-error-summary` and `ui-save-bar`. `form.js`
//! does all three: the summary and the bar belong to the form they're in.

use crate::{BG_CARD, BG_DIALOG, BORDER, DESTRUCTIVE, MUTED, TEXT, forms, lit, synthetic};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, Val, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("form.js");
/// Set inside a form, so its parts know they're in one.
const IN_FORM: &str = "ui-form";

pub struct Form;
pub struct ErrorSummary;
pub struct SaveBar;

const PROPS: &[PropDef] = &[
    PropDef::new(
        "rules",
        PropKind::Any,
        "What each state key must be: `{ email: { label, required, type: email|url|number|date, min, max, minLength, maxLength, pattern, match: otherKey, message, remote } }`. \
         Any of them may use state, e.g. `required: \"{{ kind == 'company' }}\"`. \
         `remote: /api/available` (or `{ url, param, wait, message, headers }`) asks the server once the other rules pass, `wait` ms (default 400) after the value last changed: \
         GET `url?value=…` (`param` renames it; `{value}` in the url is replaced instead), answered by `true`, `{ valid: true }`, or what's wrong (`false`, a text, `{ valid: false, message }`). \
         A submit waits for it; if the server can't be reached the value passes (it's checked again when sent) and the form fires `failed`",
    ),
    PropDef::new("bindErrors", PropKind::StateKey, "State key that gets `{ key: message }` for the fields left so far (all of them once submitting was tried); give it to each `ui-field`'s `error`"),
    PropDef::new("bindValid", PropKind::StateKey, "State key that gets true while every rule is met (not while a `remote` one is being asked)"),
    PropDef::new("bindChecking", PropKind::StateKey, "State key that gets the keys whose `remote` rule is being asked, e.g. for a field's `description: \"{{ includes(checking, 'handle') ? 'Checking…' : '' }}\"`"),
    PropDef::new("bindTried", PropKind::StateKey, "State key that gets true once submitting was tried with something wrong"),
    PropDef::new("keys", PropKind::Any, "State keys whose changes are unsaved changes (default: those with `rules`)"),
    PropDef::new("bindDirty", PropKind::StateKey, "State key that gets true while there are unsaved changes; set it to false once saved (a `submit` that succeeds does)"),
    PropDef::new("guard", PropKind::Bool, "Asks before leaving the page with unsaved changes"),
    PropDef::new("guardText", PropKind::Any, "What's asked (default `You have unsaved changes that will be lost.`)"),
    PropDef::new("disabled", PropKind::Any, "Nothing can be changed or submitted, e.g. `{{ saving }}`"),
    crate::form_layout::COLUMNS_PROP,
];

const SUMMARY_PROPS: &[PropDef] = &[
    PropDef::new("title", PropKind::Any, "Default `There's a problem` / `There are {n} problems`"),
    PropDef::new("errors", PropKind::Any, "More messages than the form's own, e.g. from the server: `{ key: message }` or a list of texts"),
];

const BAR_PROPS: &[PropDef] = &[
    PropDef::new("text", PropKind::Any, "Default `Unsaved changes`"),
    PropDef::new("saveLabel", PropKind::Any, "The button that submits the form (default `Save`)"),
    PropDef::new("discardLabel", PropKind::Any, "The button that puts the fields back as they were (default `Discard`); false for none"),
    PropDef::new("saving", PropKind::Any, "Being saved: both buttons wait, e.g. `{{ saving }}`"),
    PropDef::new("disabled", PropKind::Any, "It can't be saved as it is, e.g. `{{ problems.length > 0 }}`; Discard still works"),
    PropDef::new("position", PropKind::Enum(&["sticky", "fixed", "inline"]), "Default `sticky`, at the foot of the window while the form is on it"),
];

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

static DIALOG: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("box-sizing", "border-box"),
        ("width", "min(420px, calc(100vw - 32px))"),
        ("padding", pad!(20)),
        ("border", "var(--border-width, 1px) solid var(--color-border)"),
        ("border-radius", "var(--radius-card, var(--radius-lg))"),
        ("background", BG_DIALOG),
        ("color", TEXT),
        ("font-size", fs!(14)),
        ("box-shadow", "var(--elevation-dialog, 0 16px 40px rgb(0 0 0 / 0.2))"),
    ])
    .nest("&::backdrop", pairs(&[("background", "rgb(0 0 0 / 0.5)")]))
    .nest("& h2", pairs(&[("margin", "0"), ("font-size", fs!(17)), ("font-weight", "var(--font-weight-semibold, 600)")]))
    .nest("& p", pairs(&[("margin", concat!(gap!(8), " 0 ", gap!(20))), ("color", MUTED)]))
    .nest("& [data-form-choices]", pairs(&[("display", "flex"), ("justify-content", "flex-end"), ("gap", gap!(8))]))
});

static SUMMARY: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("padding", pad!(12, 16)),
        ("border", "var(--border-width, 1px) solid"),
        ("border-color", "color-mix(in srgb, var(--color-destructive, #dc2626) 45%, var(--color-border))"),
        ("border-radius", "var(--radius-card, var(--radius-md))"),
        ("background", "color-mix(in srgb, var(--color-destructive, #dc2626) 6%, transparent)"),
        ("color", TEXT),
        ("font-size", fs!(14)),
    ])
    .nest("&[hidden]", pairs(&[("display", "none")]))
    .nest("&:focus", pairs(&[("outline", "none")]))
    .nest("&:focus-visible", forms::focus_ring())
    .nest("& strong", pairs(&[("display", "block"), ("color", DESTRUCTIVE), ("font-weight", "var(--font-weight-semibold, 600)")]))
    .nest("& ul", pairs(&[("margin", concat!(gap!(6), " 0 0")), ("padding-inline-start", "1.2em")]))
    .nest("& li button", pairs(&[("padding", "0"), ("border", "0"), ("background", "none"), ("color", "inherit"), ("font", "inherit"), ("text-align", "start"), ("text-decoration", "underline"), ("text-underline-offset", "3px"), ("cursor", "pointer")]))
    .nest("& li button:focus-visible", forms::focus_ring())
});

static BAR: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("display", "flex"),
        ("flex-wrap", "wrap"),
        ("align-items", "center"),
        ("gap", gap!(8)),
        ("padding", pad!(10, 16)),
        ("border", "var(--border-width, 1px) solid"),
        ("border-color", BORDER),
        ("border-radius", "var(--radius-card, var(--radius-lg))"),
        ("--ui-bg", BG_CARD),
        ("background", BG_CARD),
        ("color", TEXT),
        ("font-size", fs!(14)),
    ])
    .nest("&[hidden]", pairs(&[("display", "none")]))
    .nest("& [data-bar-text]", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", gap!(8)), ("margin-inline-end", "auto"), ("font-weight", "var(--font-weight-medium, 500)")]))
    .nest("& [data-bar-text]::before", pairs(&[("content", "''"), ("width", "8px"), ("height", "8px"), ("border-radius", "50%"), ("background", "var(--color-warning, #d97706)")]))
    .nest("&[data-position=sticky]", pairs(&[("position", "sticky"), ("bottom", "16px"), ("z-index", "20"), ("box-shadow", "var(--elevation-popover, 0 6px 18px rgb(0 0 0 / 0.12))")]))
    .nest(
        "&[data-position=fixed]",
        pairs(&[("position", "fixed"), ("bottom", "16px"), ("inset-inline", "16px"), ("z-index", "40"), ("max-width", "720px"), ("margin-inline", "auto"), ("box-shadow", "var(--elevation-popover, 0 6px 18px rgb(0 0 0 / 0.12))")]),
    )
    .nest("@media (prefers-reduced-motion: no-preference)", pairs(&[("animation", "ui-save-bar .18s ease-out")]))
});

const BAR_IN: &str = "@keyframes ui-save-bar {\n  from { opacity: 0; transform: translateY(8px); }\n}";

const RULES: &[&str] = &["label", "required", "type", "min", "max", "minLength", "maxLength", "pattern", "match", "message", "remote"];

impl Component for Form {
    fn name(&self) -> &'static str {
        "ui-form"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A form whose fields are checked by `rules` as they're left and when it's submitted (`on: { submit: … }` runs only when all is right), \
                  which knows its unsaved changes. A `remote` rule asks the server (is this name taken?). Put a `ui-error-summary` and a `ui-save-bar` in it.",
            props: PROPS,
            parts: &[],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let rules = match node.prop("rules") {
            None => json!({}),
            Some(v) => {
                if !matches!(cx.eval(v), Json::Object(_)) {
                    return Err(RenderError::new(node, "`rules` must be an object: `{ key: { required: true } }`"));
                }
                crate::data_table::dynamic(cx, node, "rules", v)?
            }
        };
        for (key, r) in rules.as_object().into_iter().flatten() {
            let Json::Object(r) = r else { return Err(RenderError::new(node, format!("the rule for `{key}` must be an object"))) };
            if let Some(bad) = r.keys().find(|k| !RULES.contains(&k.as_str())) {
                return Err(RenderError::new(node, format!("unknown rule `{bad}` for `{key}`; there are {}", RULES.join(", "))));
            }
            match r.get("remote") {
                None | Some(Json::String(_)) => {}
                Some(Json::Object(o)) if o.contains_key("url") || o.contains_key("$irender") => {}
                Some(_) => return Err(RenderError::new(node, format!("`remote` for `{key}` must be a url or `{{ url, param, wait, message }}`"))),
            }
        }
        let keys = match cx.prop(node, "keys") {
            None | Some(Json::Null) => rules.as_object().map(|m| m.keys().map(|k| json!(k)).collect()).unwrap_or_default(),
            Some(Json::Array(a)) => a,
            Some(_) => return Err(RenderError::new(node, "`keys` must be a list of state keys")),
        };
        let bind = |n: &str| cx.prop_string(node, n);
        let config = json!({
            "rules": rules, "keys": keys,
            "errors": bind("bindErrors"), "valid": bind("bindValid"), "tried": bind("bindTried"), "dirty": bind("bindDirty"), "checking": bind("bindChecking"),
            "guard": cx.prop(node, "guard").is_some_and(|v| expr::truthy(&v)),
            "disabled": node.prop("disabled").map(|d| crate::data_table::dynamic(cx, node, "disabled", d)).transpose()?,
            "locale": crate::locale(cx, node),
        });
        cx.global_js("ui-form", SCRIPT);
        let mut el = cx.element("form", node);
        el.attr("novalidate", true).behavior("ui-form", config);
        cx.open(&el, node)?;
        cx.with_context(IN_FORM, Some(lit(true)), |cx| crate::form_layout::children(cx, node))?;
        if cx.prop(node, "guard").is_some_and(|v| expr::truthy(&v)) {
            let mut d = Element::new("dialog");
            d.attr("data-form-guard", true).attr("aria-label", cx.label("Leave this page?")).base_style_ref(&DIALOG);
            cx.open(&d, node)?;
            let mut h = Element::new("h2");
            h.text(lit(cx.label("Leave this page?")));
            cx.open(&h, node)?;
            cx.close("h2");
            let mut p = Element::new("p");
            p.text(node.prop("guardText").cloned().unwrap_or_else(|| lit(cx.label("You have unsaved changes that will be lost."))));
            cx.open(&p, node)?;
            cx.close("p");
            let mut row = Element::new("div");
            row.attr("data-form-choices", true);
            cx.open(&row, node)?;
            for (attr, label, variant) in [("data-form-stay", cx.label("Keep editing"), "outline"), ("data-form-leave", cx.label("Leave"), "destructive")] {
                let b = synthetic(node, "ui-button", vec![("label", lit(label)), ("variant", lit(variant))], vec![(attr, lit(true))], Vec::new());
                cx.render(&b)?;
            }
            cx.close("div");
            cx.close("dialog");
        }
        cx.close("form");
        Ok(())
    }
}

/// The form a part is in, or an error.
fn in_form(cx: &RenderCx<'_>, node: &Node) -> Result<(), RenderError> {
    match crate::context(cx, IN_FORM) {
        Some(_) => Ok(()),
        None => Err(RenderError::new(node, format!("`{}` goes inside a `ui-form`", node.kind))),
    }
}

impl Component for ErrorSummary {
    fn name(&self) -> &'static str {
        "ui-error-summary"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "What's wrong in the `ui-form` it's in, listed once submitting was tried; each message goes to its field.",
            props: SUMMARY_PROPS,
            parts: &[],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        in_form(cx, node)?;
        cx.global_js("ui-form", SCRIPT);
        let config = json!({
            "title": node.prop("title").map(|t| crate::data_table::dynamic(cx, node, "title", t)).transpose()?,
            "errors": node.prop("errors").map(|e| crate::data_table::dynamic(cx, node, "errors", e)).transpose()?,
        });
        let mut el = cx.element("div", node);
        el.base_style_ref(&SUMMARY).attr("role", "alert").attr("tabindex", "-1").attr("hidden", true).behavior("ui-error-summary", config);
        cx.open(&el, node)?;
        cx.open(&Element::new("strong"), node)?;
        cx.close("strong");
        cx.open(&Element::new("ul"), node)?;
        cx.close("ul");
        cx.close("div");
        Ok(())
    }
}

impl Component for SaveBar {
    fn name(&self) -> &'static str {
        "ui-save-bar"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Shown while the `ui-form` it's in has unsaved changes: Save submits the form, Discard puts its fields back. Children go before the buttons.",
            props: BAR_PROPS,
            parts: &[],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        in_form(cx, node)?;
        cx.global_css("ui-save-bar", BAR_IN);
        let position = cx.prop_string(node, "position").unwrap_or_else(|| "sticky".into());
        let mut el = cx.element("div", node);
        el.base_style_ref(&BAR).attr("data-form-bar", true).attr("data-position", position).attr("role", "region").attr("aria-label", cx.label("Unsaved changes")).attr("hidden", true);
        cx.open(&el, node)?;
        let mut t = Element::new("span");
        t.attr("data-bar-text", true).attr("role", "status").text(node.prop("text").cloned().unwrap_or_else(|| lit(cx.label("Unsaved changes"))));
        cx.open(&t, node)?;
        cx.close("span");
        cx.children(node)?;
        let button = |cx: &mut RenderCx<'_>, label: Val, variant: &str, ty: &str, attr: &str| -> Result<(), RenderError> {
            let mut props = vec![("label", label), ("variant", lit(variant)), ("size", lit("sm")), ("type", lit(ty))];
            let off: Vec<_> = ["saving", "disabled"].into_iter().filter(|p| *p == "saving" || ty == "submit").filter_map(|p| node.prop(p)).map(crate::val_expr).collect();
            if let Some(off) = off.into_iter().reduce(|a, b| crate::bin(expr::BinOp::Or, a, b)) {
                props.push(("disabled", crate::expr_val(off)));
            }
            cx.render(&synthetic(node, "ui-button", props, vec![(attr, lit(true))], Vec::new()))
        };
        if cx.prop(node, "discardLabel") != Some(json!(false)) {
            let l = node.prop("discardLabel").cloned().unwrap_or_else(|| lit(cx.label("Discard")));
            button(cx, l, "outline", "button", "data-form-discard")?;
        }
        let l = node.prop("saveLabel").cloned().unwrap_or_else(|| lit(cx.label("Save")));
        button(cx, l, "default", "submit", "data-form-save")?;
        cx.close("div");
        Ok(())
    }
}
