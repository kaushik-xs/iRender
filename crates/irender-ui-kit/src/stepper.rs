//! `ui-stepper` + `ui-step`: where someone is in a process, and its panels one at a time.
//! Everything follows the state key in expressions, so it needs no script: a step is done
//! once a later one is current, and a step can be gone to when the ones before it allow.
//! With `bindFurthest` it also keeps the furthest step reached, so going back doesn't
//! mean pressing Next through the steps again.

use crate::{BORDER, DESTRUCTIVE, MUTED, ON_PRIMARY, PRIMARY, PRIMARY_FILL, TEXT, bin, cond, expr_val, forms, id_with, instance_id, lit, num, state, synthetic, val_expr};
use irender_core::{
    Action, ComponentInfo, Node, PropDef, PropKind, StyleBlock, Val,
    expr::{self, BinOp, Expr, UnOp},
};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

pub struct Stepper;
pub struct Step;

const PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key holding the current step's `value`").required(),
    PropDef::new("bindFurthest", PropKind::StateKey, "State key holding the `value` of the furthest step reached: steps up to it can be gone to again, those before it show as done"),
    PropDef::new("orientation", PropKind::Enum(&["horizontal", "vertical"]), "Default `horizontal`; `vertical` opens each step's panel under it"),
    PropDef::new("linear", PropKind::Bool, "A later step can't be gone to until the ones before it are done, or `valid` (default true)"),
    PropDef::new("clickable", PropKind::Bool, "Steps are buttons that go to them (default true)"),
    PropDef::new("buttons", PropKind::Bool, "Back and Next (then Finish) under the panel (default true when steps have panels)"),
    PropDef::new("nextLabel", PropKind::Any, "Default `Next`"),
    PropDef::new("backLabel", PropKind::Any, "Default `Back`"),
    PropDef::new("finishLabel", PropKind::Any, "Default `Finish`; runs the stepper's `on: { finish: … }`"),
    PropDef::new("complete", PropKind::Any, "Every step is done, e.g. `{{ submitted }}`"),
    PropDef::new("label", PropKind::Any, "Names the steps for screen readers (default `Steps`)"),
    PropDef::new("id", PropKind::Any, "Element id (default: derived from the document)"),
];

const STEP_PROPS: &[PropDef] = &[
    PropDef::new("value", PropKind::Any, "What `bind` holds at this step (default: its number, from 1)"),
    PropDef::new("label", PropKind::Any, "The step's name").required(),
    PropDef::new("description", PropKind::Any, "Smaller text under it"),
    PropDef::new("icon", PropKind::Any, "An icon name (with the icon kit) or a character, in place of its number"),
    PropDef::new("optional", PropKind::Bool, "Marked Optional"),
    PropDef::new("valid", PropKind::Any, "Next (and the steps after it) wait for this, e.g. `{{ name != '' }}`"),
    PropDef::new("error", PropKind::Any, "Marks the step as needing attention"),
    PropDef::new("disabled", PropKind::Any, "Can't be gone to"),
];

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

static ROOT: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(20)), ("min-width", "0"), ("color", TEXT), ("font-size", fs!(14))])
        .nest("& > ol", pairs(&[("display", "flex"), ("margin", "0"), ("padding", "0"), ("list-style", "none"), ("min-width", "0")]))
        .nest("& > ol > li", pairs(&[("position", "relative"), ("min-width", "0")]))
        .nest(
            "& [data-step-head]",
            pairs(&[
                ("display", "flex"),
                ("align-items", "center"),
                ("gap", gap!(10)),
                ("min-width", "0"),
                ("padding", pad!(4)),
                ("border", "0"),
                ("border-radius", "var(--radius-control, var(--radius-md))"),
                ("background", "none"),
                ("color", MUTED),
                ("font", "inherit"),
                ("text-align", "start"),
            ]),
        )
        .nest("& button[data-step-head]", pairs(&[("cursor", "pointer")]))
        .nest("& button[data-step-head]:disabled", pairs(&[("cursor", "default")]))
        .nest("& button[data-step-head]:focus-visible", forms::focus_ring())
        .nest("& [data-step-head] :is([data-step-label], [data-step-more])", pairs(&[("display", "block")]))
        .nest("& [data-step-label]", pairs(&[("font-weight", "var(--font-weight-medium, 500)")]))
        .nest("& [data-step-more]", pairs(&[("font-size", fs!(12)), ("color", MUTED)]))
        .nest(
            "& [data-step-dot]",
            pairs(&[
                ("flex", "none"),
                ("display", "grid"),
                ("place-items", "center"),
                ("box-sizing", "border-box"),
                ("width", ctl!(30)),
                ("height", ctl!(30)),
                ("border-radius", "50%"),
                ("border", "1.5px solid var(--color-border)"),
                ("font-size", fs!(13)),
                ("font-weight", "var(--font-weight-medium, 500)"),
                ("font-variant-numeric", "tabular-nums"),
                ("transition", "background-color .2s, border-color .2s, color .2s"),
            ]),
        )
        .nest("& [data-step-dot] > [data-tick], & [data-step-dot] > [data-bang]", pairs(&[("display", "none")]))
        .nest("& li[data-status=current] [data-step-head]", pairs(&[("color", TEXT)]))
        .nest("& li[data-status=current] [data-step-dot]", pairs(&[("border-color", PRIMARY), ("color", PRIMARY), ("box-shadow", "0 0 0 3px color-mix(in srgb, var(--color-primary) 18%, transparent)")]))
        .nest("& li[data-status=done] [data-step-label]", pairs(&[("color", TEXT)]))
        .nest("& li[data-status=done] [data-step-dot]", pairs(&[("background", PRIMARY_FILL), ("border-color", "transparent"), ("color", ON_PRIMARY)]))
        .nest("& li:is([data-status=done], [data-error]) [data-step-dot] > [data-num]", pairs(&[("display", "none")]))
        .nest("& li[data-status=done]:not([data-error]) [data-step-dot] > [data-tick]", pairs(&[("display", "block")]))
        .nest("& li[data-error] [data-step-dot] > [data-bang]", pairs(&[("display", "block")]))
        .nest("& li[data-error] [data-step-dot]", pairs(&[("background", DESTRUCTIVE), ("border-color", "transparent"), ("color", crate::ON_DESTRUCTIVE)]))
        .nest("& li[data-error] [data-step-label]", pairs(&[("color", DESTRUCTIVE)]))
        .nest("& [data-step-state]", pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("overflow", "hidden"), ("clip-path", "inset(50%)")]))
        .nest("& [data-step-panel]:focus-visible", forms::focus_ring())
        .nest("& [data-step-panel][hidden]", pairs(&[("display", "none")]))
        .nest("& [data-step-buttons]", pairs(&[("display", "flex"), ("justify-content", "space-between"), ("gap", gap!(8)), ("margin-top", gap!(20))]))
        .nest("& [data-step-buttons] [hidden]", pairs(&[("display", "none")]))
        // Across: a line between each step and the next.
        .nest("&[data-orientation=horizontal] > ol", pairs(&[("gap", gap!(8))]))
        .nest("&[data-orientation=horizontal] > ol > li", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", gap!(8)), ("flex", "1 1 0")]))
        .nest("&[data-orientation=horizontal] > ol > li:last-child", pairs(&[("flex", "0 1 auto")]))
        .nest("&[data-orientation=horizontal] > ol > li:not(:last-child)::after", pairs(&[("content", "''"), ("flex", "1"), ("min-width", "12px"), ("height", "2px"), ("border-radius", "1px"), ("background", BORDER), ("transition", "background-color .2s")]))
        .nest("&[data-orientation=horizontal] > ol > li[data-status=done]:not(:last-child)::after", pairs(&[("background", PRIMARY)]))
        // Down: the line runs under each dot, beside its panel.
        .nest("&[data-orientation=vertical] > ol", pairs(&[("flex-direction", "column")]))
        .nest("&[data-orientation=vertical] > ol > li", pairs(&[("padding-bottom", pad!(16))]))
        .nest("&[data-orientation=vertical] > ol > li:last-child", pairs(&[("padding-bottom", "0")]))
        .nest(
            "&[data-orientation=vertical] > ol > li:not(:last-child)::before",
            pairs(&[("content", "''"), ("position", "absolute"), ("top", ctl!(42)), ("bottom", "0"), ("inset-inline-start", concat!("calc(", ctl!(15), " + 3px)")), ("width", "2px"), ("border-radius", "1px"), ("background", BORDER)]),
        )
        .nest("&[data-orientation=vertical] > ol > li[data-status=done]:not(:last-child)::before", pairs(&[("background", PRIMARY)]))
        .nest("&[data-orientation=vertical] [data-step-panel]", pairs(&[("margin-inline-start", concat!("calc(", ctl!(30), " + 14px)")), ("padding-top", pad!(8))]))
        // Narrow: the current step's name only, the others as their dots.
        .nest(
            "@media (max-width: 640px)",
            pairs(&[])
                .nest("&[data-orientation=horizontal] > ol > li:not([data-status=current]) [data-step-text]", pairs(&[("display", "none")]))
                .nest("&[data-orientation=horizontal] > ol > li[data-status=current]", pairs(&[("flex", "0 1 auto")])),
        )
        .nest("@media (prefers-reduced-motion: reduce)", pairs(&[]).nest("& [data-step-dot], & li::after", pairs(&[("transition", "none")])))
});

const TICK: &str = r#"<svg data-tick width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m3.5 8.5 3 3 6-7"/></svg>"#;
const BANG: &str = r#"<svg data-bang width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M8 3v6M8 12.5h.01"/></svg>"#;

/// `a && b`.
fn and(a: Expr, b: Expr) -> Expr {
    bin(BinOp::And, a, b)
}

/// The furthest step reached: its state key, the value it holds (the current step's until
/// it's set) and that value's place.
struct Furthest {
    key: String,
    value: Expr,
    at: Expr,
}

impl Furthest {
    /// What going to step `j` writes to the furthest key: `j`'s value once it's further.
    fn go(&self, steps: &[At], j: usize) -> Action {
        let v = cond(bin(BinOp::Ge, self.at.clone(), num(j)), self.value.clone(), Expr::Lit(steps[j].value.clone()));
        Action::SetState { key: self.key.clone(), value: expr_val(v) }
    }
}

/// Going to step `j`: the furthest step first, which reads the current one when unset.
fn go(key: &str, furthest: Option<&Furthest>, steps: &[At], j: usize) -> Vec<Action> {
    furthest.map(|f| f.go(steps, j)).into_iter().chain([Action::SetState { key: key.into(), value: Val::Lit(steps[j].value.clone()) }]).collect()
}

struct At {
    value: Json,
    label: Val,
    valid: Option<Expr>,
    disabled: Option<Expr>,
}

impl Component for Stepper {
    fn name(&self) -> &'static str {
        "ui-stepper"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "The steps of a process and the current one's panel, with Back, Next and Finish. Children are `ui-step`s.",
            props: PROPS,
            parts: &["list", "buttons"],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let key = cx.prop_string(node, "bind").ok_or_else(|| RenderError::new(node, "`ui-stepper` needs `bind`"))?;
        if let Some(other) = node.children.iter().find(|c| c.kind != "ui-step") {
            return Err(RenderError::new(other, "children of `ui-stepper` must be `ui-step`s"));
        }
        if node.children.is_empty() {
            return Err(RenderError::new(node, "`ui-stepper` needs `ui-step` children"));
        }
        let flag = |cx: &RenderCx<'_>, n: &str, d: bool| cx.prop(node, n).map_or(d, |v| expr::truthy(&v));
        let vertical = cx.prop_string(node, "orientation").as_deref() == Some("vertical");
        let linear = flag(cx, "linear", true);
        let clickable = flag(cx, "clickable", true);
        let panels = node.children.iter().any(|c| !c.children.is_empty());
        let buttons = flag(cx, "buttons", panels);
        let id = instance_id(cx, node, "ui-stepper");
        let steps: Vec<At> = node
            .children
            .iter()
            .enumerate()
            .map(|(i, s)| At {
                value: cx.prop(s, "value").filter(|v| !v.is_null()).unwrap_or(json!(i + 1)),
                label: s.prop("label").cloned().unwrap_or_else(|| lit("")),
                valid: s.prop("valid").map(val_expr),
                disabled: s.prop("disabled").map(val_expr),
            })
            .collect();
        for (i, s) in steps.iter().enumerate() {
            if steps[..i].iter().any(|o| o.value == s.value) {
                return Err(RenderError::new(&node.children[i], format!("two steps have the value `{}`", expr::stringify(&s.value))));
            }
        }
        // The current step's place; the first when the state names none.
        let place = |of: Expr| steps.iter().enumerate().rev().fold(num(0), |rest, (i, s)| cond(bin(BinOp::Eq, of.clone(), Expr::Lit(s.value.clone())), num(i), rest));
        let at = place(state(&key));
        let furthest = cx.prop_string(node, "bindFurthest").map(|f| {
            let kept = cond(bin(BinOp::Eq, state(&f), Expr::Lit(Json::Null)), state(&key), state(&f));
            // Never behind the current step, should the page move that on its own.
            let (far, ahead) = (place(kept.clone()), bin(BinOp::Gt, place(kept.clone()), at.clone()));
            Furthest { key: f, value: cond(ahead.clone(), kept, state(&key)), at: cond(ahead, far, at.clone()) }
        });
        let complete = node.prop("complete").map(val_expr);
        let is = |i: usize| bin(BinOp::Eq, at.clone(), num(i));
        // Passed: a later step is current, or was reached.
        let past = |j: usize| match &furthest {
            Some(f) => bin(BinOp::Gt, f.at.clone(), num(j)),
            None => bin(BinOp::Gt, at.clone(), num(j)),
        };
        // Reached: every step before it is done, or says it's valid.
        let reach = |i: usize| {
            steps[..i].iter().enumerate().fold(Expr::Lit(json!(true)), |all, (j, s)| {
                let past = past(j);
                and(all, s.valid.clone().unwrap_or(past))
            })
        };

        let mut el = cx.element("div", node);
        el.base_style_ref(&ROOT).attr("data-orientation", if vertical { "vertical" } else { "horizontal" }).attr("id", id.clone());
        cx.open(&el, node)?;
        // Done: before the current step, or reached after it and open to go to again.
        let done = |i: usize| match (&furthest, linear) {
            (Some(_), true) => bin(BinOp::Or, bin(BinOp::Gt, at.clone(), num(i)), and(past(i), reach(i))),
            _ => past(i),
        };
        let mut ol = cx.part("ol", node, "list");
        ol.attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Steps"))));
        cx.open(&ol, node)?;
        for (i, (s, step)) in steps.iter().zip(&node.children).enumerate() {
            let status = cond(
                complete.clone().unwrap_or(Expr::Lit(json!(false))),
                Expr::Lit(json!("done")),
                cond(is(i), Expr::Lit(json!("current")), cond(done(i), Expr::Lit(json!("done")), Expr::Lit(json!("upcoming")))),
            );
            let mut li = cx.element("li", step);
            li.attr("data-status", expr_val(status.clone()));
            if let Some(e) = step.prop("error") {
                li.attr("data-error", expr_val(cond(val_expr(e), Expr::Lit(json!(true)), Expr::Lit(Json::Null))));
            }
            cx.open(&li, step)?;
            // The head: the number (or a tick), the name and what's under it.
            let current = expr_val(cond(is(i), Expr::Lit(json!("step")), Expr::Lit(Json::Null)));
            let blocked = match (linear, &s.disabled) {
                (true, d) => {
                    let shut = Expr::Unary(UnOp::Not, Box::new(reach(i)));
                    Some(d.clone().map_or(shut.clone(), |d| bin(BinOp::Or, d, shut)))
                }
                (false, d) => d.clone(),
            };
            if clickable {
                let mut attrs = vec![("type", lit("button")), ("data-step-head", lit(true)), ("aria-current", current)];
                if let Some(b) = blocked {
                    attrs.push(("disabled", expr_val(b)));
                }
                let b = synthetic(step, "button", Vec::new(), attrs, vec![("click", go(&key, furthest.as_ref(), &steps, i))]);
                let el = cx.element("button", &b);
                cx.open(&el, &b)?;
            } else {
                let mut d = Element::new("div");
                d.attr("data-step-head", true).attr("aria-current", current);
                cx.open(&d, step)?;
            }
            let mut dot = Element::new("span");
            dot.attr("data-step-dot", true).attr("aria-hidden", "true");
            cx.open(&dot, step)?;
            let mut n = Element::new("span");
            n.attr("data-num", true);
            match step.prop("icon") {
                Some(icon) => {
                    cx.open(&n, step)?;
                    crate::icon(cx, step, icon, 16)?;
                }
                None => {
                    n.text(lit((i + 1).to_string()));
                    cx.open(&n, step)?;
                }
            }
            cx.close("span");
            cx.markup(TICK);
            cx.markup(BANG);
            cx.close("span");
            let mut text = Element::new("span");
            text.attr("data-step-text", true);
            cx.open(&text, step)?;
            let mut l = Element::new("span");
            l.attr("data-step-label", true).text(s.label.clone());
            cx.open(&l, step)?;
            cx.close("span");
            let optional = cx.prop(step, "optional").is_some_and(|v| expr::truthy(&v));
            if let Some(d) = step.prop("description").cloned().or_else(|| optional.then(|| lit(cx.label("Optional")))) {
                let mut m = Element::new("span");
                m.attr("data-step-more", true).text(d);
                cx.open(&m, step)?;
                cx.close("span");
            }
            cx.close("span");
            let mut st = Element::new("span");
            let word = |t: &str| Expr::Lit(json!(format!(" ({})", cx.label(t))));
            let mut said = cond(bin(BinOp::Eq, status, Expr::Lit(json!("done"))), word("done"), Expr::Lit(json!("")));
            if let Some(e) = step.prop("error") {
                said = cond(val_expr(e), word("needs attention"), said);
            }
            st.attr("data-step-state", true).text(expr_val(said));
            cx.open(&st, step)?;
            cx.close("span");
            cx.close(if clickable { "button" } else { "div" });
            if vertical {
                self.panel(cx, node, step, i, steps.len(), &id, &key, furthest.as_ref(), &steps, &is, buttons)?;
            }
            cx.close("li");
        }
        cx.close("ol");
        if !vertical {
            for (i, step) in node.children.iter().enumerate() {
                self.panel(cx, node, step, i, steps.len(), &id, &key, furthest.as_ref(), &steps, &is, buttons)?;
            }
        }
        cx.close("div");
        Ok(())
    }
}

impl Stepper {
    /// A step's panel, with its buttons: shown while the step is current.
    #[allow(clippy::too_many_arguments)]
    fn panel(&self, cx: &mut RenderCx<'_>, node: &Node, step: &Node, i: usize, n: usize, id: &Val, key: &str, furthest: Option<&Furthest>, steps: &[At], is: &dyn Fn(usize) -> Expr, buttons: bool) -> Result<(), RenderError> {
        if step.children.is_empty() && !buttons {
            return Ok(());
        }
        let mut p = Element::new("div");
        p.attr("data-step-panel", true)
            .attr("id", id_with(id, &format!("-panel-{}", i + 1)))
            .attr("role", "group")
            .attr("aria-label", steps[i].label.clone())
            .attr("hidden", expr_val(Expr::Unary(UnOp::Not, Box::new(is(i)))));
        cx.open(&p, step)?;
        cx.children(step)?;
        if buttons {
            let mut bar = cx.part("div", node, "buttons");
            bar.attr("data-step-buttons", true);
            cx.open(&bar, node)?;
            let text = |cx: &RenderCx<'_>, prop: &str, default: &str| node.prop(prop).cloned().unwrap_or_else(|| lit(cx.label(default)));
            // Labels: "Back", "Next", "Finish"
            if i > 0 {
                let back = synthetic(
                    node,
                    "ui-button",
                    vec![("label", text(cx, "backLabel", "Back")), ("variant", lit("outline"))],
                    Vec::new(),
                    vec![("click", go(key, furthest, steps, i - 1))],
                );
                cx.render(&back)?;
            } else {
                cx.open(&Element::new("span"), node)?;
                cx.close("span");
            }
            let mut props = Vec::new();
            if let Some(v) = &steps[i].valid {
                props.push(("disabled", expr_val(Expr::Unary(UnOp::Not, Box::new(v.clone())))));
            }
            let actions = if i + 1 < n {
                props.push(("label", text(cx, "nextLabel", "Next")));
                go(key, furthest, steps, i + 1)
            } else {
                props.push(("label", text(cx, "finishLabel", "Finish")));
                node.on.get("finish").cloned().unwrap_or_default()
            };
            cx.render(&synthetic(node, "ui-button", props, Vec::new(), vec![("click", actions)]))?;
            cx.close("div");
        }
        cx.close("div");
        Ok(())
    }
}

impl Component for Step {
    fn name(&self) -> &'static str {
        "ui-step"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "One step of a `ui-stepper`; its children are its panel.", props: STEP_PROPS, parts: &[], children: true, open_props: false }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        // Drawn by its stepper; on its own it's just a container.
        let el = cx.element("div", node);
        cx.open(&el, node)?;
        cx.children(node)?;
        cx.close("div");
        Ok(())
    }
}
