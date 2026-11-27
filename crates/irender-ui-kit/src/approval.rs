//! `ui-approval`: a sign-off in stages (a manager, then finance, then a director), each
//! approved when its rule is met (all its approvers, any one, or a number of them), with
//! who decided what, when and why. Whoever's looking (`me`), when it's their turn, gets
//! Approve, Request changes and Reject (the last two with a comment); the one who sent
//! it (`owner`) can send it again after changes or a rejection. The stages are state;
//! `approval.js` works out where it stands, draws it and writes each decision back.

use crate::{MUTED, PRIMARY, TEXT, forms, instance_id, lit, status, widget};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("approval.js");

/// What an approver may decide.
pub(crate) const DECISIONS: &[&str] = &["approved", "changes", "rejected"];

const PROPS: &[PropDef] = &[
    PropDef::new(
        "bind",
        PropKind::StateKey,
        "State key holding the stages, in order: `{ id, title, rule, due, approvers: [{ id, name, avatar, decision, comment, at, for }], rounds }`; `rule` is `all` (default), `any` or a number, `decision` approved, changes or rejected, `due` a date it's due by, `for` the approver who handed it on, `rounds` earlier rounds' approvers (kept when it's sent again)",
    )
    .required(),
    PropDef::new("me", PropKind::Any, "The `id` of whoever's looking: on their turn, they get the buttons to decide"),
    PropDef::new("owner", PropKind::Any, "The `id` of whoever sent it: after changes or a rejection, they can send it again"),
    PropDef::new("bindStatus", PropKind::StateKey, "State key that gets `pending`, `changes`, `rejected` or `approved`, as it changes"),
    PropDef::new("people", PropKind::Any, "Who an approver can hand their turn to: `[{ id, name, avatar }]`"),
    PropDef::new("readOnly", PropKind::Bool, "Only shows where it stands: no buttons"),
    PropDef::new("locale", PropKind::String, "How times are written (default: the document's `lang`)"),
    PropDef::new("label", PropKind::Any, "Names it for screen readers (default `Approval`)"),
    PropDef::new("id", PropKind::Any, "Element id (default: derived from the document)"),
];

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

const BORDER: &str = "var(--border-width, 1px) solid var(--color-border)";

static APPROVAL: LazyLock<StyleBlock> = LazyLock::new(|| {
    let mut b = pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(14)), ("color", TEXT), ("font-size", fs!(14))])
        // ---- where it stands, at the top
        .nest(
            "& [data-ap-summary]",
            pairs(&[
                ("display", "flex"),
                ("align-items", "flex-start"),
                ("gap", gap!(10)),
                ("padding", pad!(10, 12)),
                ("border-radius", "var(--radius-control, var(--radius-md))"),
                ("background", "color-mix(in srgb, var(--tone) 10%, transparent)"),
                ("border", "1px solid color-mix(in srgb, var(--tone) 30%, transparent)"),
            ]),
        )
        .nest("& [data-ap-summary] b", pairs(&[("display", "block"), ("color", "color-mix(in srgb, var(--tone) 80%, var(--color-text))")]))
        .nest("& [data-ap-summary] small", pairs(&[("color", MUTED), ("font-size", fs!(13))]))
        .nest("& [data-ap-view]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(16))]))
        // ---- the stages, as a rail of marks
        .nest("& [data-ap-stages]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("margin", "0"), ("padding", "0"), ("list-style", "none")]))
        .nest("& [data-ap-stage]", pairs(&[("position", "relative"), ("display", "grid"), ("grid-template-columns", "28px minmax(0, 1fr)"), ("column-gap", gap!(12)), ("padding-bottom", pad!(18))]))
        .nest("& [data-ap-stage]::before", pairs(&[("content", "''"), ("position", "absolute"), ("inset-inline-start", "13.5px"), ("top", "30px"), ("bottom", "2px"), ("width", "1px"), ("background", "var(--color-border)")]))
        .nest("& [data-ap-stage]:last-child::before", pairs(&[("display", "none")]))
        .nest("& [data-ap-stage]:last-child", pairs(&[("padding-bottom", "0")]))
        .nest(
            "& [data-ap-mark]",
            pairs(&[
                ("display", "grid"),
                ("place-items", "center"),
                ("width", "28px"),
                ("height", "28px"),
                ("border-radius", "50%"),
                ("border", "2px solid var(--tone)"),
                ("background", "var(--ui-bg, var(--color-surface, #fff))"),
                ("color", "var(--tone)"),
                ("font-size", fs!(13)),
                ("font-weight", "var(--font-weight-bold, 700)"),
            ]),
        )
        .nest("& [data-ap-stage][data-state=approved] [data-ap-mark], & [data-ap-stage][data-state=rejected] [data-ap-mark]", pairs(&[("background", "var(--tone)"), ("color", "#fff")]))
        .nest("& [data-ap-stage][data-state=waiting]", pairs(&[("opacity", "0.6")]))
        .nest("& [data-ap-body]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(8)), ("min-width", "0"), ("padding-top", pad!(3))]))
        .nest("& [data-ap-head]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("align-items", "baseline"), ("gap", gap!(4, 8))]))
        .nest("& [data-ap-head] h3", pairs(&[("margin", "0"), ("font-size", fs!(15)), ("font-weight", "var(--font-weight-semibold, 600)")]))
        .nest("& [data-ap-rule]", pairs(&[("color", MUTED), ("font-size", fs!(13))]))
        // When it's due: overdue in the danger tone.
        .nest("& [data-ap-due]", pairs(&[("color", MUTED), ("font-size", fs!(12))]))
        .nest("& [data-ap-due][data-overdue]", pairs(&[("color", "var(--color-destructive, #dc2626)"), ("font-weight", "var(--font-weight-medium, 500)")]))
        .nest("& [data-ap-for]", pairs(&[("display", "block"), ("color", MUTED), ("font-size", fs!(12))]))
        // Earlier rounds, folded under the stage.
        .nest("& [data-ap-rounds]", pairs(&[("margin-top", gap!(8)), ("font-size", fs!(13))]))
        .nest("& [data-ap-rounds] > summary", pairs(&[("cursor", "pointer"), ("color", MUTED)]))
        .nest("& [data-ap-rounds] ol", pairs(&[("margin", "6px 0 0"), ("padding-inline-start", "18px"), ("display", "grid"), ("gap", gap!(4))]))
        .nest("& [data-ap-handoff]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("align-items", "center"), ("gap", gap!(8))]))
        .nest("& [data-ap-handoff] select", pairs(&[("min-height", ctl!(32)), ("padding", pad!(0, 8)), ("border", "var(--border-width, 1px) solid var(--color-border)"), ("border-radius", "var(--radius-control, var(--radius-md))"), ("background", "var(--ui-bg, transparent)"), ("color", "inherit"), ("font", "inherit")]))
        // ---- the approvers
        .nest("& [data-ap-people]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(8)), ("margin", "0"), ("padding", "0"), ("list-style", "none")]))
        .nest("& [data-ap-person]", pairs(&[("display", "grid"), ("grid-template-columns", "24px minmax(0, 1fr) auto"), ("align-items", "center"), ("column-gap", gap!(8)), ("row-gap", gap!(4))]))
        .nest(
            "& [data-ap-avatar]",
            pairs(&[("display", "grid"), ("place-items", "center"), ("width", "24px"), ("height", "24px"), ("border-radius", "50%"), ("overflow", "hidden"), ("background", "color-mix(in srgb, var(--color-text) 8%, transparent)"), ("color", MUTED), ("font-size", fs!(10)), ("font-weight", "var(--font-weight-semibold, 600)")]),
        )
        .nest("& [data-ap-avatar] img", pairs(&[("width", "100%"), ("height", "100%"), ("object-fit", "cover")]))
        .nest("& [data-ap-name] small", pairs(&[("margin-inline-start", gap!(6)), ("color", MUTED), ("font-size", fs!(12))]))
        .nest(
            "& [data-ap-decision]",
            pairs(&[("padding", pad!(1, 8)), ("border-radius", "999px"), ("background", "color-mix(in srgb, var(--tone) 12%, transparent)"), ("color", "color-mix(in srgb, var(--tone) 80%, var(--color-text))"), ("font-size", fs!(12)), ("font-weight", "var(--font-weight-medium, 500)"), ("white-space", "nowrap")]),
        )
        .nest(
            "& [data-ap-person] blockquote",
            pairs(&[("grid-column", "2 / -1"), ("margin", "0"), ("padding", pad!(6, 10)), ("border-inline-start", "2px solid color-mix(in srgb, var(--tone, var(--color-border)) 60%, var(--color-border))"), ("color", TEXT), ("white-space", "pre-wrap"), ("overflow-wrap", "anywhere")]),
        )
        // ---- deciding
        .nest("& [data-ap-decide]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(8)), ("padding", pad!(12)), ("border", BORDER), ("border-radius", "var(--radius-control, var(--radius-md))")]))
        .nest("& [data-ap-decide] label", pairs(&[("font-weight", "var(--font-weight-medium, 500)")]))
        .nest(
            "& [data-ap-decide] textarea",
            pairs(&[("box-sizing", "border-box"), ("width", "100%"), ("min-height", sp!(64)), ("padding", pad!(8, 10)), ("border", BORDER), ("border-radius", "var(--radius-control, var(--radius-md))"), ("background", "transparent"), ("color", TEXT), ("font", "inherit"), ("resize", "vertical")]),
        )
        .nest("& [data-ap-decide] textarea[aria-invalid=true]", pairs(&[("border-color", "var(--color-destructive, #dc2626)")]))
        .nest("& [data-ap-error]", pairs(&[("color", "var(--color-destructive, #dc2626)"), ("font-size", fs!(13))]))
        .nest("& [data-ap-buttons]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("gap", gap!(8))]))
        .nest(
            "& [data-ap-buttons] button",
            pairs(&[("height", ctl!(34)), ("padding", pad!(0, 14)), ("border", BORDER), ("border-radius", "var(--radius-control, var(--radius-md))"), ("background", "transparent"), ("color", "inherit"), ("font", "inherit"), ("font-weight", "var(--font-weight-medium, 500)"), ("cursor", "pointer")]),
        )
        .nest("& [data-ap-buttons] button[data-ap-act=approved]", pairs(&[("border-color", "transparent"), ("background", "var(--fill-primary, var(--color-primary))"), ("color", "var(--color-on-primary)")]))
        .nest("& [data-ap-buttons] button[data-ap-act=rejected]", pairs(&[("color", "var(--color-destructive, #dc2626)")]))
        .nest("& [data-ap-buttons] button:focus-visible, & textarea:focus-visible", forms::focus_ring())
        .nest(
            "& [data-ap-live]",
            pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("overflow", "hidden"), ("clip-path", "inset(50%)"), ("white-space", "nowrap")]),
        );
    // The tones of a stage's mark, a decision and the summary.
    for (sel, tone) in [("approved", "success"), ("rejected", "danger"), ("changes", "warning"), ("current", "primary"), ("pending", "primary"), ("waiting", "neutral")] {
        b = b.nest(format!("& [data-state={sel}], & [data-decision={sel}]"), pairs(&[("--tone", status::tone_color(tone))]));
    }
    b.nest("& [data-decision=none]", pairs(&[("--tone", MUTED)])).nest("& [data-state=current] [data-ap-mark]", pairs(&[("box-shadow", &format!("0 0 0 3px color-mix(in srgb, {PRIMARY} 18%, transparent)"))]))
});

pub struct Approval;

impl Component for Approval {
    fn name(&self) -> &'static str {
        "ui-approval"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A sign-off in stages, each approved when its rule is met: who decided what, when and why; Approve, Request changes and Reject on your turn.",
            props: PROPS,
            parts: &[],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let key = cx.prop_string(node, "bind").filter(|k| !k.is_empty()).ok_or_else(|| RenderError::new(node, "`bind` is the state key holding the stages"))?;
        let stages = match cx.lookup(std::slice::from_ref(&key)) {
            Json::Array(a) => a,
            Json::Null => Vec::new(),
            _ => return Err(RenderError::new(node, format!("`{key}` must hold a list of stages"))),
        };
        for (i, s) in stages.iter().enumerate() {
            let n = i + 1;
            let Some(people) = s.get("approvers").and_then(Json::as_array).filter(|a| !a.is_empty()) else {
                return Err(RenderError::new(node, format!("stage {n} needs `approvers`, a list of `{{ id, name }}`")));
            };
            match s.get("rule") {
                None | Some(Json::Null) => {}
                Some(Json::String(r)) if r == "all" || r == "any" => {}
                Some(Json::Number(k)) if k.as_f64().is_some_and(|k| k >= 1.0 && k.fract() == 0.0 && k as usize <= people.len()) => {}
                Some(r) => return Err(RenderError::new(node, format!("stage {n}: `rule` is `all`, `any` or a number up to its approvers, got `{}`", expr::stringify(r)))),
            }
            for p in people {
                if p.get("id").is_none_or(Json::is_null) || p.get("name").is_none_or(Json::is_null) {
                    return Err(RenderError::new(node, format!("stage {n}: each approver needs an `id` and a `name`")));
                }
                if let Some(d) = p.get("decision").and_then(Json::as_str)
                    && !DECISIONS.contains(&d)
                {
                    return Err(RenderError::new(node, format!("stage {n}: a decision is {}, got `{d}`", DECISIONS.join(", "))));
                }
            }
        }
        let mut config = widget::config(cx, node, &["me", "owner", "people"], &[], &["bind", "bindStatus"])?;
        config.insert("readOnly".into(), json!(widget::flag(cx, node, "readOnly", false)));
        config.insert("locale".into(), json!(crate::locale(cx, node)));
        let id = instance_id(cx, node, "approval");
        config.insert("id".into(), json!(expr::stringify(&cx.eval(&id))));
        widget::kit(cx);
        cx.global_js("ui-approval", SCRIPT);

        let mut root = cx.element("section", node);
        root.base_style_ref(&APPROVAL)
            .behavior("ui-approval", Json::Object(config))
            .attr("id", id)
            .attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Approval"))));
        cx.open(&root, node)?;
        // The behaviour draws the summary, the stages and the buttons.
        for part in ["data-ap-summary", "data-ap-view"] {
            let mut d = Element::new("div");
            d.attr(part, true);
            cx.open(&d, node)?;
            cx.close("div");
        }
        let mut live = Element::new("div");
        live.attr("data-ap-live", true).attr("role", "status").attr("aria-live", "polite");
        cx.open(&live, node)?;
        cx.close("div");
        cx.close("section");
        Ok(())
    }
}
