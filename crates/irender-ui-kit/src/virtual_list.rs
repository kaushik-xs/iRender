//! `ui-virtual-list`: a long list (thousands of items, or a hundred thousand) that keeps
//! only the items in view in the page, with space above and below standing in for the
//! rest, so it scrolls smoothly. Each item is the component's children, with the item and
//! its place in scope. The items are the runtime's keyed list, supplied a window at a
//! time by `virtual_list.js`, which measures them as they're drawn (they may differ in
//! height) and says when the end is near, to load more.

use crate::{MUTED, TEXT, cond, bin, expr_val, field_or, forms, items, lit, path, widget};
use irender_core::{
    ComponentInfo, Node, PropDef, PropKind, StyleBlock, Val,
    expr::{BinOp, Expr},
};
use irender_html::{ClientList, Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("virtual_list.js");

/// The names an item and its place are in scope under, in the children.
const ITEM: &str = "item";
const INDEX: &str = "index";
/// Items the server writes, for a page read before (or without) the script.
const FIRST: usize = 30;

const PROPS: &[PropDef] = &[
    PropDef::new("items", PropKind::Any, "The items: a list, usually from state (`{{ rows }}`); it follows").required(),
    PropDef::new("itemKey", PropKind::String, "The field that tells items apart (default `id`, else their place)"),
    PropDef::new("height", PropKind::Any, "The scrolling area's height: px or a CSS length (default 400)"),
    PropDef::new("itemHeight", PropKind::Number, "An item's height in px to start from, before items are measured (default 40)"),
    PropDef::new("gap", PropKind::Number, "Space between items in px (default 0)"),
    PropDef::new("bindRange", PropKind::StateKey, "State key that gets the places in view, `{ from, to, total }`, as it scrolls"),
    PropDef::new("endThreshold", PropKind::Number, "How many items from the end `end` fires, to load more (default 10)"),
    PropDef::new("emptyText", PropKind::Any, "Shown while there are no items (default `Nothing to show`)"),
    PropDef::new("label", PropKind::Any, "Names the list for screen readers"),
];

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

static LIST: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("position", "relative"),
        ("overflow", "auto"),
        ("overscroll-behavior", "contain"),
        ("border", "var(--border-width, 1px) solid var(--color-border)"),
        ("border-radius", "var(--radius-card, var(--radius-lg))"),
        ("color", TEXT),
        ("font-size", fs!(14)),
    ])
    .nest("&:focus-visible", forms::focus_ring())
    .nest("& [data-vl-items]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", "var(--vl-gap, 0px)")]))
    .nest("& [data-vl-pad]", pairs(&[("flex", "none")]))
    .nest("& [data-vl-empty]", pairs(&[("margin", "0"), ("padding", pad!(24)), ("color", MUTED), ("text-align", "center")]))
    .nest("& [data-vl-items]:has(> [data-vl-item]) + [data-vl-empty], & [data-vl-empty][hidden]", pairs(&[("display", "none")]))
});

pub struct VirtualList;

impl Component for VirtualList {
    fn name(&self) -> &'static str {
        "ui-virtual-list"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A long list that keeps only the items in view in the page, so thousands scroll smoothly; children are each item, with `item` and `index` in scope.",
            props: PROPS,
            parts: &[],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        if node.children.is_empty() {
            return Err(RenderError::new(node, "a virtual list's children are each item, with `item` and `index` in scope"));
        }
        let (values, _) = items(cx, node, "items")?;
        let height = match cx.prop(node, "height").filter(|v| !v.is_null()) {
            None => "400px".to_string(),
            Some(Json::Number(n)) => format!("{n}px"),
            Some(v) => irender_core::expr::stringify(&v),
        };
        let item_height = widget::number(cx, node, "itemHeight").unwrap_or(40.0);
        if !(4.0..=2000.0).contains(&item_height) {
            return Err(RenderError::new(node, "`itemHeight` is px, 4 to 2000"));
        }
        let gap = widget::number(cx, node, "gap").unwrap_or(0.0);
        let item_key = cx.prop_string(node, "itemKey").unwrap_or_else(|| "id".into());
        let mut config = widget::config(cx, node, &["items"], &[], &["bindRange"])?;
        config.insert("itemKey".into(), json!(item_key));
        config.insert("itemHeight".into(), json!(item_height));
        config.insert("gap".into(), json!(gap));
        config.insert("endThreshold".into(), json!(widget::number(cx, node, "endThreshold").unwrap_or(10.0)));
        widget::kit(cx);
        cx.global_js("ui-virtual-list", SCRIPT);

        let mut root = cx.element("div", node);
        root.base_style_ref(&LIST)
            .base_style(StyleBlock::new().set("height", height).set("--vl-gap", format!("{gap}px")))
            .behavior("ui-virtual-list", Json::Object(config))
            // The scrolling area takes focus, so the keys scroll it.
            .attr("tabindex", "0")
            .attr("data-vl", true);
        if let Some(l) = node.prop("label") {
            root.attr("aria-label", l.clone()).attr("role", "region");
        }
        cx.open(&root, node)?;
        let mut top = Element::new("div");
        top.attr("data-vl-pad", "top").attr("aria-hidden", "true");
        cx.open(&top, node)?;
        cx.close("div");

        // The behaviour supplies the items in view; the runtime keeps the nodes of those
        // that stay. Each is wrapped with its place in the whole list.
        let none = Val::Lit(Json::Array(Vec::new()));
        let key = expr_val(cond(
            bin(BinOp::Ne, path(&[ITEM, &item_key]), Expr::Lit(Json::Null)),
            path(&[ITEM, &item_key]),
            Expr::Path(vec![INDEX.into()]),
        ));
        let key = cx.item_val_js(&key, ITEM, INDEX);
        let total = values.len();
        let mut list = Element::new("div");
        list.attr("data-vl-items", true).attr("role", "list");
        if let Some(l) = node.prop("label") {
            list.attr("aria-label", l.clone());
        }
        list.list(ClientList { items: &none, item: ITEM.into(), index: INDEX.into(), key: Some(key) });
        cx.open(&list, node)?;
        let render_item = |cx: &mut RenderCx<'_>| -> Result<(), RenderError> {
            let mut wrap = Element::new("div");
            wrap.attr("data-vl-item", expr_val(field_or(ITEM, &item_key, Expr::Path(vec![INDEX.into()]))))
                .attr("role", "listitem")
                .attr("aria-posinset", expr_val(bin(BinOp::Add, Expr::Path(vec![INDEX.into()]), crate::num(1))));
            cx.open(&wrap, node)?;
            cx.children(node)?;
            cx.close("div");
            Ok(())
        };
        cx.item_template_with(ITEM, INDEX, render_item)?;
        // The first items, for a page read before the script runs; it replaces them.
        for (i, v) in values.into_iter().take(FIRST).enumerate() {
            cx.with_scope(vec![(ITEM.into(), v), (INDEX.into(), json!(i))], render_item)?;
        }
        cx.close("div");
        let mut empty = Element::new("p");
        empty.attr("data-vl-empty", true).attr("hidden", total > 0).text(node.prop("emptyText").cloned().unwrap_or_else(|| lit(cx.label("Nothing to show"))));
        cx.open(&empty, node)?;
        cx.close("p");
        let mut bottom = Element::new("div");
        bottom.attr("data-vl-pad", "bottom").attr("aria-hidden", "true").base_style(StyleBlock::new().set("height", format!("{}px", (total.saturating_sub(FIRST)) as f64 * (item_height + gap))));
        cx.open(&bottom, node)?;
        cx.close("div");
        cx.close("div");
        Ok(())
    }
}
