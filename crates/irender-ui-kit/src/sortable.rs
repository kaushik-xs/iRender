//! `ui-sortable-list`: a list in state put in order by dragging each item's grip (mouse,
//! touch or pen) or from the keyboard, with items removed if asked. The items are the
//! runtime's own keyed list, so `sortable.js` only writes the new order to state and the
//! runtime moves the items; their content is the component's children, with the item and
//! its place in scope, or the item's label.

use crate::{MUTED, SUBTLE, TEXT, each_item, expr_val, forms, lit, state};
use irender_core::{
    ComponentInfo, Node, Part, PropDef, PropKind, StyleBlock, Template, Val,
    expr::{self, BinOp, Expr},
};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("sortable.js");

/// The names the item and its place are in scope under, in the children.
const ITEM: &str = "item";
const INDEX: &str = "index";

const PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key holding the list, in order: plain values or objects").required(),
    PropDef::new("itemKey", PropKind::String, "With objects, the field that tells them apart (default `id`)"),
    PropDef::new("labelField", PropKind::String, "With objects, the field shown and read out for each (default `label`)"),
    PropDef::new("group", PropKind::String, "Lists with the same group take items from each other: dragged across, or picked up and moved with ←/→ (`transfer` fires on both)"),
    PropDef::new("numbered", PropKind::Bool, "Each item's place before it: 1, 2, 3"),
    PropDef::new("removable", PropKind::Bool, "A button on each item that takes it out of the list"),
    PropDef::new("emptyText", PropKind::Any, "Shown while the list is empty (default `Nothing here yet`)"),
    PropDef::new("label", PropKind::Any, "Names the list for screen readers (default `List`)"),
    PropDef::new("disabled", PropKind::Any, "Boolean or expression: no moving or removing"),
    PropDef::new("id", PropKind::Any, "Element id (default: derived from the document)"),
];

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

static LIST: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(6)), ("color", TEXT), ("font-size", fs!(14))])
        .nest("& [data-sl-items]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(6)), ("margin", "0"), ("padding", "0"), ("list-style", "none")]))
        .nest(
            "& [data-sl-item]",
            pairs(&[
                ("position", "relative"),
                ("display", "flex"),
                ("align-items", "center"),
                ("gap", gap!(8)),
                ("min-height", ctl!(40)),
                ("padding", pad!(6, 8, 6, 4)),
                ("border", "var(--border-width, 1px) solid var(--color-border)"),
                ("border-radius", "var(--radius-control, var(--radius-md))"),
                ("background", "var(--color-card, var(--color-surface))"),
                ("transition", "box-shadow .15s, opacity .15s"),
            ]),
        )
        // The item dragged follows the pointer, over the others, which make way for it.
        .nest("& [data-sl-item][data-sl-dragging]", pairs(&[("position", "relative"), ("z-index", "2"), ("opacity", "0.9"), ("box-shadow", "var(--elevation-overlay, 0 8px 24px rgb(0 0 0 / 0.18))"), ("cursor", "grabbing")]))
        .nest("&[data-sl-active] [data-sl-item]:not([data-sl-dragging])", pairs(&[("transition", "transform .15s ease")]))
        .nest("@media (prefers-reduced-motion: reduce)", pairs(&[]).nest("&[data-sl-active] [data-sl-item]:not([data-sl-dragging])", pairs(&[("transition", "none")])))
        .nest(
            "& [data-sl-item][data-sl-picked]",
            pairs(&[("border-color", "var(--color-primary)"), ("box-shadow", "0 0 0 1px var(--color-primary), var(--elevation-popover, 0 6px 18px rgb(0 0 0 / 0.12))")]),
        )
        // Where a dragged item lands: a line in the gap before or after the item.
        .nest(
            "& [data-sl-item][data-sl-drop]::before",
            pairs(&[("content", "''"), ("position", "absolute"), ("left", "0"), ("right", "0"), ("height", "2px"), ("border-radius", "1px"), ("background", "var(--color-primary)"), ("pointer-events", "none")]),
        )
        .nest("& [data-sl-item][data-sl-drop=before]::before", pairs(&[("top", "-5px")]))
        .nest("& [data-sl-item][data-sl-drop=after]::before", pairs(&[("bottom", "-5px")]))
        .nest(
            "& [data-sl-grip]",
            pairs(&[
                ("display", "inline-flex"),
                ("align-items", "center"),
                ("justify-content", "center"),
                ("flex", "none"),
                ("width", "24px"),
                ("height", "28px"),
                ("padding", "0"),
                ("border", "0"),
                ("border-radius", "var(--radius-sm)"),
                ("background", "transparent"),
                ("color", MUTED),
                ("cursor", "grab"),
                ("touch-action", "none"),
            ]),
        )
        .nest("& [data-sl-grip]:hover", pairs(&[("background", SUBTLE), ("color", TEXT)]))
        .nest("& [data-sl-grip]:focus-visible", forms::focus_ring())
        .nest("& [data-sl-grip]:disabled", pairs(&[("cursor", "default"), ("opacity", "0.4")]))
        .nest("&[data-sl-active] [data-sl-grip]", pairs(&[("cursor", "grabbing")]))
        .nest("&[data-sl-active]", pairs(&[("user-select", "none")]))
        .nest("& [data-sl-place]", pairs(&[("flex", "none"), ("min-width", "1.5em"), ("color", MUTED), ("font-variant-numeric", "tabular-nums"), ("text-align", "end")]))
        .nest("& [data-sl-body]", pairs(&[("flex", "1"), ("min-width", "0")]))
        .nest(
            "& [data-sl-remove]",
            pairs(&[
                ("display", "inline-flex"),
                ("align-items", "center"),
                ("justify-content", "center"),
                ("flex", "none"),
                ("width", "24px"),
                ("height", "24px"),
                ("padding", "0"),
                ("border", "0"),
                ("border-radius", "var(--radius-sm)"),
                ("background", "transparent"),
                ("color", MUTED),
                ("font-size", fs!(12)),
                ("cursor", "pointer"),
            ]),
        )
        .nest("& [data-sl-remove]:hover", pairs(&[("background", SUBTLE), ("color", "var(--color-destructive, #dc2626)")]))
        .nest("& [data-sl-remove]:focus-visible", forms::focus_ring())
        .nest("& [data-sl-empty]", pairs(&[("padding", pad!(16)), ("border", "var(--border-width, 1px) dashed var(--color-border)"), ("border-radius", "var(--radius-control, var(--radius-md))"), ("color", MUTED), ("text-align", "center")]))
        // The list always holds the runtime's `<template>`: it's empty without an item.
        .nest("& [data-sl-items]:has(> li) + [data-sl-empty]", pairs(&[("display", "none")]))
        .nest("& [data-sl-help]", pairs(&[("display", "none")]))
        .nest(
            "& [data-sl-live]",
            pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("overflow", "hidden"), ("clip-path", "inset(50%)"), ("white-space", "nowrap")]),
        )
});

const GRIP: &str = r#"<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true"><circle cx="6" cy="3.5" r="1.3"/><circle cx="10" cy="3.5" r="1.3"/><circle cx="6" cy="8" r="1.3"/><circle cx="10" cy="8" r="1.3"/><circle cx="6" cy="12.5" r="1.3"/><circle cx="10" cy="12.5" r="1.3"/></svg>"#;

/// `item.field`, or the item itself when the list holds plain values.
fn field(objects: bool, name: &str) -> Expr {
    Expr::Path(if objects { vec![ITEM.into(), name.into()] } else { vec![ITEM.into()] })
}

/// `before {name} after`, from a label with a `{name}` in it, as a template.
fn with_name(cx: &RenderCx<'_>, text: &str, name: &Expr) -> Val {
    let whole = cx.label_with(text, &[("name", "\u{0}")]);
    let (before, after) = whole.split_once('\u{0}').unwrap_or((&whole, ""));
    Val::Tpl(Template { parts: vec![Part::Lit(before.into()), Part::Expr(name.clone()), Part::Lit(after.into())] })
}

pub struct SortableList;

impl Component for SortableList {
    fn name(&self) -> &'static str {
        "ui-sortable-list"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A list in state put in order by dragging each item's grip, or from the keyboard; children are each item's content, with `item` and `index` in scope.",
            props: PROPS,
            parts: &[],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let key = cx.prop_string(node, "bind").filter(|k| !k.is_empty()).ok_or_else(|| RenderError::new(node, "`bind` is the state key holding the list"))?;
        let values = match cx.lookup(std::slice::from_ref(&key)) {
            Json::Array(a) => a,
            Json::Null => Vec::new(),
            _ => return Err(RenderError::new(node, format!("`{key}` must hold a list"))),
        };
        let item_key = cx.prop_string(node, "itemKey").unwrap_or_else(|| "id".into());
        let label_field = cx.prop_string(node, "labelField").unwrap_or_else(|| "label".into());
        // Objects or plain values: what the first item is (an empty list, objects).
        let objects = values.first().is_none_or(Json::is_object);
        if objects && let Some(i) = values.iter().position(|v| v.get(&item_key).is_none_or(Json::is_null)) {
            return Err(RenderError::new(node, format!("item {} has no `{item_key}`: give each item one, or set `itemKey`", i + 1)));
        }
        let flag = |name: &str| cx.prop(node, name).is_some_and(|v| expr::truthy(&v));
        let (numbered, removable) = (flag("numbered"), flag("removable"));
        let group = cx.prop_string(node, "group").filter(|g| !g.is_empty());
        cx.global_js("ui-sortable-list", SCRIPT);

        let mut root = cx.element("div", node);
        root.base_style_ref(&LIST).behavior(
            "ui-sortable-list",
            json!({ "bind": key, "group": group, "itemKey": if objects { Json::String(item_key.clone()) } else { Json::Null }, "labelField": if objects { Json::String(label_field.clone()) } else { Json::Null } }),
        );
        if let Some(g) = &group {
            root.attr("data-sl-group", g.clone());
        }
        if let Some(d) = node.prop("disabled") {
            root.attr("data-disabled", d.clone());
        }
        cx.open(&root, node)?;
        // How to move an item, read with each grip.
        let help = instance_help_id(cx, node);
        let mut h = Element::new("span");
        let help_text = if group.is_some() {
            cx.label("Space picks it up; the arrow keys move it, left and right to the other lists; Space puts it down, Escape puts it back. Alt and the arrow keys move it straight away.")
        } else {
            cx.label("Space picks it up; the arrow keys move it; Space puts it down, Escape puts it back. Alt and the arrow keys move it straight away.")
        };
        h.attr("id", help.clone()).attr("data-sl-help", true).text(lit(help_text));
        cx.open(&h, node)?;
        cx.close("span");

        let name = field(objects, &label_field);
        let id_of = expr_val(field(objects, &item_key));
        let move_label = with_name(cx, "Move {name}", &name);
        let remove_label = with_name(cx, "Remove {name}", &name);
        let items_val = expr_val(state(&key));
        let mut list = Element::new("ol");
        list.attr("data-sl-items", true).attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("List"))));
        let disabled = node.prop("disabled").cloned();
        each_item(cx, node, list, (values, Some(&items_val)), ITEM, INDEX, &id_of, |cx| {
            let mut li = Element::new("li");
            li.attr("data-sl-item", id_of.clone());
            cx.open(&li, node)?;
            let mut grip = Element::new("button");
            grip.attr("type", "button").attr("data-sl-grip", true).attr("aria-label", move_label.clone()).attr("aria-describedby", help.clone());
            if let Some(d) = &disabled {
                grip.attr("disabled", d.clone());
            }
            cx.open(&grip, node)?;
            cx.markup(GRIP);
            cx.close("button");
            if numbered {
                let mut p = Element::new("span");
                p.attr("data-sl-place", true).attr("aria-hidden", "true").text(expr_val(crate::bin(BinOp::Add, Expr::Path(vec![INDEX.into()]), crate::num(1))));
                cx.open(&p, node)?;
                cx.close("span");
            }
            let mut body = Element::new("div");
            body.attr("data-sl-body", true);
            if node.children.is_empty() {
                body.text(expr_val(name.clone()));
            }
            cx.open(&body, node)?;
            if !node.children.is_empty() {
                cx.children(node)?;
            }
            cx.close("div");
            if removable {
                let mut x = Element::new("button");
                x.attr("type", "button").attr("data-sl-remove", true).attr("aria-label", remove_label.clone()).text(lit("✕"));
                if let Some(d) = &disabled {
                    x.attr("disabled", d.clone());
                }
                cx.open(&x, node)?;
                cx.close("button");
            }
            cx.close("li");
            Ok(())
        })?;
        cx.close("ol");
        let mut empty = Element::new("p");
        empty.attr("data-sl-empty", true).text(node.prop("emptyText").cloned().unwrap_or_else(|| lit(cx.label("Nothing here yet"))));
        cx.open(&empty, node)?;
        cx.close("p");
        let mut live = Element::new("div");
        live.attr("data-sl-live", true).attr("role", "status").attr("aria-live", "assertive");
        cx.open(&live, node)?;
        cx.close("div");
        cx.close("div");
        Ok(())
    }
}

/// An id for the list's instructions, unique on the page.
fn instance_help_id(cx: &RenderCx<'_>, node: &Node) -> Val {
    crate::id_with(&crate::instance_id(cx, node, "sortable"), "-help")
}
