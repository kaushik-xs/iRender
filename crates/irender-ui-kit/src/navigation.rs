//! Breadcrumb and pagination: lists of links expanded on the server.

use crate::{ACCENT, MUTED, TEXT, field, lit, server_array};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

// ---------------------------------------------------------------- breadcrumb

pub struct Breadcrumb;

const BREADCRUMB_PROPS: &[PropDef] = &[
    PropDef::new("items", PropKind::Any, "Array of `{ label, href }`; the last is the current page").required(),
    PropDef::new("separator", PropKind::Any, "Text between items (default `›`)"),
];

static BREADCRUMB_LIST: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("display", "flex"),
        ("flex-wrap", "wrap"),
        ("align-items", "center"),
        ("gap", "6px"),
        ("margin", "0"),
        ("padding", "0"),
        ("list-style", "none"),
        ("font-size", "14px"),
        ("color", MUTED),
    ])
});
static BREADCRUMB_LINK: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[("color", "inherit"), ("text-decoration", "none"), ("transition", "color .15s")])
        .nest(":hover", StyleBlock::from_pairs(&[("color", TEXT)]))
});
static BREADCRUMB_CURRENT: LazyLock<StyleBlock> = LazyLock::new(|| StyleBlock::from_pairs(&[("color", TEXT)]));

impl Component for Breadcrumb {
    fn name(&self) -> &'static str {
        "ui-breadcrumb"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Where the current page sits in the site's hierarchy.",
            props: BREADCRUMB_PROPS,
            parts: &["list"],
            children: false,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let items = server_array(cx, node, "items")?;
        let separator = cx.prop_string(node, "separator").unwrap_or_else(|| "›".into());
        let mut nav = cx.element("nav", node);
        nav.attr("aria-label", "breadcrumb");
        cx.open(&nav, node)?;
        let mut ol = cx.part("ol", node, "list");
        ol.base_style_ref(&BREADCRUMB_LIST);
        cx.open(&ol, node)?;
        for (i, item) in items.iter().enumerate() {
            let last = i + 1 == items.len();
            if i > 0 {
                let mut sep = Element::new("li");
                sep.attr("role", "presentation").attr("aria-hidden", "true").text(lit(separator.clone()));
                cx.open(&sep, node)?;
                cx.close("li");
            }
            let (label, href) = match item {
                Json::Object(_) => (field(item, "label").unwrap_or_default(), field(item, "href")),
                other => (expr::stringify(other), None),
            };
            cx.open(&Element::new("li"), node)?;
            let mut el = match (&href, last) {
                (Some(h), false) => {
                    let mut a = Element::new("a");
                    a.base_style_ref(&BREADCRUMB_LINK).attr("href", h.clone());
                    a
                }
                _ => {
                    let mut span = Element::new("span");
                    span.base_style_ref(&BREADCRUMB_CURRENT);
                    if last {
                        span.attr("aria-current", "page");
                    }
                    span
                }
            };
            el.text(lit(label));
            let tag = el.tag().to_string();
            cx.open(&el, node)?;
            cx.close(&tag);
            cx.close("li");
        }
        cx.close("ol");
        cx.close("nav");
        Ok(())
    }
}

// ---------------------------------------------------------------- pagination

pub struct Pagination;

const PAGINATION_PROPS: &[PropDef] = &[
    PropDef::new("page", PropKind::Any, "Current page, from 1").required(),
    PropDef::new("pages", PropKind::Any, "Number of pages").required(),
    PropDef::new("href", PropKind::Any, "Link for page `p`, e.g. `\"/orders?page={{ p }}\"`").required(),
    PropDef::new("siblings", PropKind::Any, "Pages shown on each side of the current one (default 1)"),
];

static PAGINATION_LIST: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("display", "flex"),
        ("flex-wrap", "wrap"),
        ("align-items", "center"),
        ("gap", "4px"),
        ("margin", "0"),
        ("padding", "0"),
        ("list-style", "none"),
    ])
});
static PAGE_LINK: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("display", "inline-flex"),
        ("align-items", "center"),
        ("justify-content", "center"),
        ("gap", "4px"),
        ("min-width", "36px"),
        ("height", "36px"),
        ("padding", "0 10px"),
        ("border", "1px solid transparent"),
        ("border-radius", "var(--radius-md)"),
        ("color", TEXT),
        ("font-size", "14px"),
        ("font-weight", "500"),
        ("text-decoration", "none"),
        ("transition", "background-color .15s"),
    ])
    .nest(":hover", StyleBlock::from_pairs(&[("background", ACCENT)]))
    .nest(
        "&[aria-current=page]",
        StyleBlock::from_pairs(&[("border-color", "var(--color-border)"), ("box-shadow", "0 1px 2px rgb(0 0 0 / 0.05)")]),
    )
    .nest("&[aria-disabled=true]", StyleBlock::from_pairs(&[("opacity", "0.5"), ("pointer-events", "none")]))
});
static ELLIPSIS: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[("display", "inline-flex"), ("width", "36px"), ("justify-content", "center"), ("color", MUTED)])
});

/// The pages to show: the first, the last, and `siblings` around `page`, with
/// `None` where a gap becomes an ellipsis.
fn page_window(page: u64, pages: u64, siblings: u64) -> Vec<Option<u64>> {
    let mut out = Vec::new();
    let (lo, hi) = (page.saturating_sub(siblings).max(1), (page + siblings).min(pages));
    let push = |out: &mut Vec<Option<u64>>, p: u64| {
        if let Some(Some(prev)) = out.last()
            && p > prev + 1
        {
            // A gap of one page shows the page itself instead of `…`.
            out.push(if p == prev + 2 { Some(prev + 1) } else { None });
        }
        out.push(Some(p));
    };
    for p in std::iter::once(1).chain(lo..=hi).chain(std::iter::once(pages)) {
        if out.last() != Some(&Some(p)) && !out.contains(&Some(p)) {
            push(&mut out, p);
        }
    }
    out
}

impl Component for Pagination {
    fn name(&self) -> &'static str {
        "ui-pagination"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Links to the previous, next and nearby pages. Rendered on the server.",
            props: PAGINATION_PROPS,
            parts: &["list"],
            children: false,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let num = |cx: &RenderCx<'_>, name: &str, default: f64| {
            cx.prop(node, name).map(|v| expr::to_num(&v)).filter(|n| n.is_finite()).unwrap_or(default).max(0.0) as u64
        };
        let pages = num(cx, "pages", 1.0).max(1);
        let page = num(cx, "page", 1.0).clamp(1, pages);
        let siblings = num(cx, "siblings", 1.0);
        let href = |cx: &mut RenderCx<'_>, p: u64| {
            cx.with_scope(vec![("p".into(), json!(p))], |cx| Ok(cx.prop(node, "href").unwrap_or(Json::Null)))
        };

        let mut nav = cx.element("nav", node);
        nav.attr("aria-label", "pagination");
        cx.open(&nav, node)?;
        let mut ul = cx.part("ul", node, "list");
        ul.base_style_ref(&PAGINATION_LIST);
        cx.open(&ul, node)?;

        let link = |cx: &mut RenderCx<'_>, target: Option<u64>, label: &str, aria: Option<&str>, current: bool| {
            cx.open(&Element::new("li"), node)?;
            let mut a = Element::new("a");
            a.base_style_ref(&PAGE_LINK).text(lit(label));
            match target {
                Some(p) => {
                    a.attr("href", href(cx, p)?);
                }
                None => {
                    a.attr("aria-disabled", "true");
                }
            }
            if let Some(l) = aria {
                a.attr("aria-label", l);
            }
            if current {
                a.attr("aria-current", "page");
            }
            cx.open(&a, node)?;
            cx.close("a");
            cx.close("li");
            Ok::<(), RenderError>(())
        };

        link(cx, (page > 1).then(|| page - 1), "‹ Previous", Some("Go to previous page"), false)?;
        for p in page_window(page, pages, siblings) {
            match p {
                Some(p) => link(cx, Some(p), &p.to_string(), None, p == page)?,
                None => {
                    cx.open(&Element::new("li"), node)?;
                    let mut gap = Element::new("span");
                    gap.base_style_ref(&ELLIPSIS).attr("aria-hidden", "true").text(lit("…"));
                    cx.open(&gap, node)?;
                    cx.close("span");
                    cx.close("li");
                }
            }
        }
        link(cx, (page < pages).then(|| page + 1), "Next ›", Some("Go to next page"), false)?;
        cx.close("ul");
        cx.close("nav");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::page_window;

    fn show(page: u64, pages: u64) -> String {
        page_window(page, pages, 1).iter().map(|p| p.map_or("…".into(), |p| p.to_string())).collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn windows() {
        assert_eq!(show(1, 1), "1");
        assert_eq!(show(1, 3), "1 2 3");
        assert_eq!(show(1, 10), "1 2 … 10");
        assert_eq!(show(5, 10), "1 … 4 5 6 … 10");
        assert_eq!(show(3, 10), "1 2 3 4 … 10");
        assert_eq!(show(10, 10), "1 … 9 10");
        assert_eq!(show(4, 7), "1 2 3 4 5 6 7");
    }
}
