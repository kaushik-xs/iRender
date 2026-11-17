//! Breadcrumb and pagination: lists of links, rebuilt in the browser when they use state.

use crate::{ACCENT, MUTED, TEXT, bin, branch, cond, each_item, expr_val, field_or, items, lit, num, path, val_expr};
use irender_core::{
    ComponentInfo, Node, Part, PropDef, PropKind, StyleBlock, Val,
    expr::{self, BinOp, Expr, UnOp},
};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("pagination.js");

// ---------------------------------------------------------------- breadcrumb

pub struct Breadcrumb;

const BREADCRUMB_PROPS: &[PropDef] = &[
    PropDef::new("items", PropKind::Any, "Array of `{ label, href }`; the last is the current page. May use state").required(),
    PropDef::new("separator", PropKind::Any, "Text between items (default `›`)"),
];

static BREADCRUMB_LIST: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("display", "flex"),
        ("flex-wrap", "wrap"),
        ("align-items", "center"),
        ("gap", sp!(6)),
        ("margin", "0"),
        ("padding", "0"),
        ("list-style", "none"),
        ("font-size", fs!(14)),
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
        let crumbs = items(cx, node, "items")?;
        // The last item is the current page. In a list the browser rebuilds, that needs
        // the list's length, which only a path to it can give.
        let count = match crumbs.1 {
            None => num(crumbs.0.len()),
            Some(Val::Tpl(t)) if matches!(t.parts.as_slice(), [Part::Expr(Expr::Path(_))]) => {
                let Part::Expr(Expr::Path(p)) = &t.parts[0] else { unreachable!() };
                Expr::Path(p.iter().cloned().chain(["length".to_string()]).collect())
            }
            Some(_) => {
                return Err(RenderError::new(node, "`items` that use state must be a path to them, like `{{ crumbs }}`"));
            }
        };
        let separator = cx.prop_string(node, "separator").unwrap_or_else(|| "›".into());

        let index = path(&[INDEX]);
        let last = bin(BinOp::Eq, index.clone(), bin(BinOp::Sub, count, num(1)));
        let href = path(&[CRUMB, "href"]);
        let link = expr_val(bin(BinOp::And, bin(BinOp::Ne, href.clone(), Expr::Lit(Json::Null)), Expr::Unary(UnOp::Not, Box::new(last.clone()))));
        let not_link = expr_val(Expr::Unary(UnOp::Not, Box::new(val_expr(&link))));
        let label = expr_val(field_or(CRUMB, "label", path(&[CRUMB])));
        let current = expr_val(cond(last, Expr::Lit(json!("page")), Expr::Lit(Json::Null)));
        let after_first = expr_val(bin(BinOp::Gt, index, num(0)));
        let href = expr_val(href);
        let crumb = |cx: &mut RenderCx<'_>| {
            branch(cx, &after_first, |cx| {
                let mut sep = Element::new("li");
                sep.attr("role", "presentation").attr("aria-hidden", "true").text(lit(separator.clone()));
                cx.open(&sep, node)?;
                cx.close("li");
                Ok(())
            })?;
            cx.open(&Element::new("li"), node)?;
            branch(cx, &link, |cx| {
                let mut a = Element::new("a");
                a.base_style_ref(&BREADCRUMB_LINK).attr("href", href.clone()).text(label.clone());
                cx.open(&a, node)?;
                cx.close("a");
                Ok(())
            })?;
            branch(cx, &not_link, |cx| {
                let mut span = Element::new("span");
                span.base_style_ref(&BREADCRUMB_CURRENT).attr("aria-current", current.clone()).text(label.clone());
                cx.open(&span, node)?;
                cx.close("span");
                Ok(())
            })?;
            cx.close("li");
            Ok(())
        };

        let mut nav = cx.element("nav", node);
        nav.attr("aria-label", "breadcrumb");
        cx.open(&nav, node)?;
        let mut ol = cx.part("ol", node, "list");
        ol.base_style_ref(&BREADCRUMB_LIST);
        each_item(cx, node, ol, crumbs, CRUMB, INDEX, &expr_val(path(&[INDEX])), crumb)?;
        cx.close("ol");
        cx.close("nav");
        Ok(())
    }
}

/// The names of the current item and its index while rendering a list's items.
const CRUMB: &str = "ui_crumb";
const INDEX: &str = "ui_index";

// ---------------------------------------------------------------- pagination

pub struct Pagination;

const PAGINATION_PROPS: &[PropDef] = &[
    PropDef::new("page", PropKind::Any, "Current page, from 1; may use state, like `pages` and `siblings`").required(),
    PropDef::new("pages", PropKind::Any, "Number of pages").required(),
    PropDef::new("href", PropKind::Any, "Link for page `p`, e.g. `\"/orders?page={{ p }}\"`").required(),
    PropDef::new("siblings", PropKind::Any, "Pages shown on each side of the current one (default 1)"),
];

static PAGINATION_LIST: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("display", "flex"),
        ("flex-wrap", "wrap"),
        ("align-items", "center"),
        ("gap", sp!(4)),
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
        ("gap", sp!(4)),
        ("min-width", sp!(36)),
        ("height", sp!(36)),
        ("padding", sp!(0, 10)),
        ("border", "1px solid transparent"),
        ("border-radius", "var(--radius-md)"),
        ("color", TEXT),
        ("font-size", fs!(14)),
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
            doc: "Links to the previous, next and nearby pages. Follows `page`, `pages` and `siblings` when they \
                  use state.",
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
        // With a page or page count from state, a behaviour redraws the links as it
        // changes, from these templates and the same window (pagination.js).
        let live = ["page", "pages", "siblings"].iter().any(|p| node.prop(p).is_some_and(|v| cx.is_reactive(v)));
        if live {
            let js = |cx: &RenderCx<'_>, name: &str, default: u64| node.prop(name).map_or(json!(default), |v| cx.val_js(v));
            let config = json!({
                "page": js(cx, "page", 1),
                "pages": js(cx, "pages", 1),
                "siblings": js(cx, "siblings", 1),
                "href": node.prop("href").map_or(Json::Null, |h| cx.item_val_js(h, "p", "ui_index")),
            });
            cx.global_js("ui-pagination", SCRIPT);
            ul.behavior("ui-pagination", config);
        }
        cx.open(&ul, node)?;
        if live {
            for (tag, style) in [("a", &*PAGE_LINK), ("span", &*ELLIPSIS)] {
                cx.open(&Element::new("template"), node)?;
                cx.open(&Element::new("li"), node)?;
                let mut inner = Element::new(tag);
                inner.base_style_ref(style);
                if tag == "span" {
                    inner.attr("aria-hidden", "true").text(lit("…"));
                }
                cx.open(&inner, node)?;
                cx.close(tag);
                cx.close("li");
                cx.close("template");
            }
        }

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
