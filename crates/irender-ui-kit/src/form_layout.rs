//! How a form is laid out: `ui-form`'s `columns`, `ui-form-section` (a titled group of
//! fields with columns of its own, its title above them or beside them) and the `span` of
//! what's in them. They nest, so a form of two columns can hold a section of three. The
//! columns fall to two and then one as the space they're in narrows (container queries,
//! so a form in a drawer stacks too).

use crate::{MUTED, TEXT, id_with, instance_id, lit};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::Value as Json;
use std::sync::LazyLock;

/// The columns of the grid a field is in, so its `span` can't go past them.
const COLUMNS: &str = "ui-form-columns";

pub struct FormSection;

const PROPS: &[PropDef] = &[
    PropDef::new("title", PropKind::Any, "Its heading"),
    PropDef::new("description", PropKind::Any, "Text under the heading"),
    PropDef::new("columns", PropKind::Number, "Columns for what's in it, 1–6 (default 1); two and then one as it narrows"),
    PropDef::new("layout", PropKind::Enum(&["stacked", "aside"]), "The heading above the fields (default), or `aside`: beside them (up to 240px wide) while there's room"),
    PropDef::new("span", PropKind::Any, "Columns it takes in the form or section it's in: a number or `full`"),
    PropDef::new("level", PropKind::Any, "Its heading's level, 2–6 (default 3)"),
];

pub(crate) const COLUMNS_PROP: PropDef =
    PropDef::new("columns", PropKind::Number, "Lays what's in it out in columns, 1–6; two and then one as it narrows. Children take more with `span` (`ui-field`, `ui-form-section`) or `style: { grid-column: 1 / -1 }`");
pub(crate) const SPAN_PROP: PropDef = PropDef::new("span", PropKind::Any, "Columns it takes in a `ui-form` or `ui-form-section` with `columns`: a number or `full`");

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

/// A container whose one child is the grid, which asks the container how wide it is.
static GRID: LazyLock<StyleBlock> = LazyLock::new(|| {
    let mut s = pairs(&[("container-type", "inline-size"), ("min-width", "0")]).nest(
        "& > [data-form-grid]",
        pairs(&[("display", "grid"), ("grid-template-columns", "repeat(var(--form-cols, 1), minmax(0, 1fr))"), ("gap", gap!(16, 20)), ("align-items", "start")]),
    );
    for n in 2..=6 {
        s = s.nest(format!("& > [data-form-grid] > [data-span=\"{n}\"]"), pairs(&[("grid-column", &format!("span {n}"))]));
    }
    s.nest("& > [data-form-grid] > :is([data-span=full], [role=alert], [data-form-bar])", pairs(&[("grid-column", "1 / -1")]))
        .nest(
            "@container (max-width: 639.98px)",
            pairs(&[])
                .nest("& > [data-form-grid]:not([data-cols=\"1\"], [data-cols=\"2\"])", pairs(&[("grid-template-columns", "repeat(2, minmax(0, 1fr))")]))
                .nest("& > [data-form-grid] > [data-span]:not([data-span=\"2\"])", pairs(&[("grid-column", "1 / -1")])),
        )
        .nest(
            "@container (max-width: 399.98px)",
            pairs(&[]).nest("& > [data-form-grid][data-cols]", pairs(&[("grid-template-columns", "minmax(0, 1fr)")])).nest("& > [data-form-grid] > [data-span]", pairs(&[("grid-column", "1 / -1")])),
        )
});

static SECTION: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("container-type", "inline-size"), ("min-width", "0"), ("color", TEXT)])
        .nest("& > [data-section-in]", pairs(&[("display", "grid"), ("grid-template-columns", "minmax(0, 1fr)"), ("gap", gap!(16)), ("align-items", "start")]))
        .nest("& > [data-section-in] > [data-section-head]", pairs(&[("display", "grid"), ("gap", gap!(4))]))
        .nest("& > [data-section-in] > [data-section-head] > :first-child", pairs(&[("margin", "0"), ("font-size", fs!(16)), ("font-weight", "var(--font-weight-semibold, 600)"), ("line-height", "1.4")]))
        .nest("& > [data-section-in] > [data-section-head] > p", pairs(&[("margin", "0"), ("color", MUTED), ("font-size", fs!(14))]))
        .nest("@container (min-width: 640px)", pairs(&[]).nest("&[data-layout=aside] > [data-section-in]", pairs(&[("grid-template-columns", "minmax(0, 240px) minmax(0, 1fr)"), ("gap", gap!(16, 32))])))
});

/// `columns` of `node`, 1–6, if it has them.
fn columns(cx: &RenderCx<'_>, node: &Node) -> Option<u64> {
    crate::widget::number(cx, node, "columns").map(|c| c.round().clamp(1.0, 6.0) as u64)
}

/// The grid of `columns` holding what `f` renders; what's in it can `span` them.
pub(crate) fn grid(cx: &mut RenderCx<'_>, node: &Node, columns: u64, f: impl FnOnce(&mut RenderCx<'_>) -> Result<(), RenderError>) -> Result<(), RenderError> {
    let mut wrap = Element::new("div");
    wrap.attr("data-form-columns", true).base_style_ref(&GRID);
    cx.open(&wrap, node)?;
    let mut g = Element::new("div");
    g.attr("data-form-grid", true).attr("data-cols", columns.to_string()).base_style(StyleBlock::new().set("--form-cols", columns.to_string()));
    cx.open(&g, node)?;
    cx.with_context(COLUMNS, Some(lit(columns)), f)?;
    cx.close("div");
    cx.close("div");
    Ok(())
}

/// Grid `columns` for a `ui-form`, or its children as they are.
pub(crate) fn children(cx: &mut RenderCx<'_>, node: &Node) -> Result<(), RenderError> {
    match columns(cx, node) {
        Some(c) => grid(cx, node, c, |cx| cx.children(node)),
        None => cx.children(node),
    }
}

/// `data-span` for the columns `node` takes, no more than the grid it's in has.
pub(crate) fn span(cx: &RenderCx<'_>, node: &Node, el: &mut Element<'_>) -> Result<(), RenderError> {
    let Some(v) = cx.prop(node, "span").filter(|v| !v.is_null()) else { return Ok(()) };
    let n = match &v {
        Json::String(s) if s == "full" => u64::MAX,
        v if expr::to_num(v) >= 1.0 => expr::to_num(v).round() as u64,
        _ => return Err(RenderError::new(node, "`span` must be a number of columns or `full`")),
    };
    let cols = crate::context(cx, COLUMNS).map(|c| expr::to_num(&cx.eval(&c)) as u64).unwrap_or(6);
    match n {
        1 => {}
        n if n >= cols => {
            el.attr("data-span", "full");
        }
        n => {
            el.attr("data-span", n.to_string());
        }
    }
    Ok(())
}

impl Component for FormSection {
    fn name(&self) -> &'static str {
        "ui-form-section"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A titled group of fields, with columns of its own. Sections go in a `ui-form` (which may have `columns` too, to put sections side by side) or in each other; \
                  children take more columns with `span`.",
            props: PROPS,
            parts: &[],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let id = instance_id(cx, node, "section");
        let (title, description) = (node.prop("title"), node.prop("description"));
        let mut el = cx.element("div", node);
        el.base_style_ref(&SECTION).attr("role", "group").attr("data-form-section", true).attr("data-layout", cx.prop_string(node, "layout").unwrap_or_else(|| "stacked".into()));
        if title.is_some() {
            el.attr("aria-labelledby", id_with(&id, "-title"));
        }
        if description.is_some() {
            el.attr("aria-describedby", id_with(&id, "-description"));
        }
        span(cx, node, &mut el)?;
        cx.open(&el, node)?;
        let mut inner = Element::new("div");
        inner.attr("data-section-in", true);
        cx.open(&inner, node)?;
        if title.is_some() || description.is_some() {
            let mut head = Element::new("div");
            head.attr("data-section-head", true);
            cx.open(&head, node)?;
            if let Some(t) = title {
                let level = cx.prop(node, "level").map(|l| expr::to_num(&l)).filter(|l| (2.0..=6.0).contains(l)).unwrap_or(3.0) as u8;
                let tag = format!("h{level}");
                let mut h = Element::new(&tag);
                h.attr("id", id_with(&id, "-title")).text(t.clone());
                cx.open(&h, node)?;
                cx.close(&tag);
            }
            if let Some(d) = description {
                let mut p = Element::new("p");
                p.attr("id", id_with(&id, "-description")).text(d.clone());
                cx.open(&p, node)?;
                cx.close("p");
            }
            cx.close("div");
        }
        grid(cx, node, columns(cx, node).unwrap_or(1), |cx| cx.children(node))?;
        cx.close("div");
        cx.close("div");
        Ok(())
    }
}
