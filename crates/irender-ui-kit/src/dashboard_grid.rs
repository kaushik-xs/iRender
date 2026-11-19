//! `ui-dashboard-grid`: tiles (`ui-tile` children) on a grid of columns and rows, to
//! drag by their header and resize from their corner. The server places them from
//! their `x`, `y`, `w` and `h` (or from `bind`'s layout), packs them upward, and draws
//! them there; `dashboard_grid.js` moves them, pushing the others out of the way, and
//! writes the layout back to `bind`. Under `stackBelow` the tiles stack in one column.

use crate::{BG_CARD, BORDER, MUTED, RING, TEXT, forms, instance_id, id_with, lit};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("dashboard_grid.js");

pub struct DashboardGrid;
pub struct Tile;

const PROPS: &[PropDef] = &[
    PropDef::new("columns", PropKind::Number, "Columns across (default 12)"),
    PropDef::new("rowHeight", PropKind::Number, "Each row's height in px (default 80)"),
    PropDef::new("gap", PropKind::Number, "Space between tiles in px (default 16)"),
    PropDef::new("bind", PropKind::StateKey, "State key for the layout, `[{ id, x, y, w, h }]`: read to place the tiles, written after each move"),
    PropDef::new("editable", PropKind::Bool, "Tiles can be moved and resized (default true)"),
    PropDef::new("compact", PropKind::Bool, "Tiles float up into the space above them (default true)"),
    PropDef::new("stackBelow", PropKind::Number, "Below this width in px the tiles stack in one column, in order (default 640)"),
    PropDef::new("label", PropKind::Any, "Names the grid for screen readers (default `Dashboard`)"),
];
const TILE_PROPS: &[PropDef] = &[
    PropDef::new("id", PropKind::String, "Names the tile in the layout (default `tile-1`, `tile-2`, … in order)"),
    PropDef::new("title", PropKind::Any, "Heading in its header, which is what you drag it by"),
    PropDef::new("x", PropKind::Number, "Its column, from 0 (default: the first free place)"),
    PropDef::new("y", PropKind::Number, "Its row, from 0"),
    PropDef::new("w", PropKind::Number, "Its width in columns (default 4)"),
    PropDef::new("h", PropKind::Number, "Its height in rows (default 2)"),
    PropDef::new("minW", PropKind::Number, "The narrowest it can be resized to (default 1)"),
    PropDef::new("minH", PropKind::Number, "The shortest (default 1)"),
    PropDef::new("maxW", PropKind::Number, "The widest"),
    PropDef::new("maxH", PropKind::Number, "The tallest"),
    PropDef::new("locked", PropKind::Bool, "Stays where it is: it can't be moved or resized, and others go around it"),
];

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

static GRID: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("position", "relative"), ("container-type", "inline-size"), ("color", TEXT)])
        .nest(
            "& [data-dg-grid]",
            pairs(&[
                ("position", "relative"),
                ("display", "grid"),
                ("grid-template-columns", "repeat(var(--dg-cols), minmax(0, 1fr))"),
                ("grid-auto-rows", "var(--dg-row)"),
                ("gap", "var(--dg-gap)"),
            ]),
        )
        .nest(
            "& [data-dg-tile]",
            pairs(&[
                ("position", "relative"),
                ("display", "flex"),
                ("flex-direction", "column"),
                ("min-width", "0"),
                ("min-height", "0"),
                ("box-sizing", "border-box"),
                ("border", "1px solid var(--color-border)"),
                ("border-radius", "var(--radius-lg)"),
                ("--ui-bg", BG_CARD),
                ("background", BG_CARD),
                ("box-shadow", "0 1px 2px rgb(0 0 0 / 0.05)"),
                ("overflow", "hidden"),
            ]),
        )
        .nest("& [data-dg-tile][data-dragging]", pairs(&[("z-index", "3"), ("box-shadow", "0 12px 32px rgb(0 0 0 / 0.18)"), ("opacity", ".92"), ("cursor", "grabbing")]))
        .nest("& [data-dg-tile][data-resizing]", pairs(&[("z-index", "3")]))
        .nest(
            "& [data-dg-head]",
            pairs(&[
                ("display", "flex"),
                ("align-items", "center"),
                ("gap", sp!(8)),
                ("min-height", "40px"),
                ("padding", sp!(8, 16)),
                ("box-sizing", "border-box"),
                ("user-select", "none"),
                ("touch-action", "none"),
                ("outline", "none"),
                ("border-radius", "var(--radius-lg) var(--radius-lg) 0 0"),
            ]),
        )
        .nest("& [data-dg-head][data-dg-handle]", pairs(&[("cursor", "grab")]))
        .nest("& [data-dg-head]:focus-visible", forms::focus_ring().set("border-color", BORDER))
        .nest("& [data-dg-head] h3", pairs(&[("margin", "0"), ("flex", "1"), ("min-width", "0"), ("font-size", fs!(14)), ("font-weight", "600"), ("overflow", "hidden"), ("text-overflow", "ellipsis"), ("white-space", "nowrap")]))
        .nest("& [data-dg-head] [data-dg-grip]", pairs(&[("flex", "none"), ("display", "grid"), ("color", MUTED), ("opacity", ".6")]))
        .nest(
            "& [data-dg-body]",
            pairs(&[("flex", "1"), ("min-height", "0"), ("display", "flex"), ("flex-direction", "column"), ("gap", sp!(12)), ("padding", sp!(0, 16, 16)), ("overflow", "auto")]),
        )
        .nest("& [data-dg-tile]:not(:has(> [data-dg-head])) > [data-dg-body]", pairs(&[("padding", sp!(16))]))
        // A tile with no title is dragged by a grip in its corner, shown on hover and focus.
        .nest(
            "& [data-dg-grip-only]",
            pairs(&[
                ("position", "absolute"),
                ("top", "6px"),
                ("inset-inline-end", "6px"),
                ("z-index", "1"),
                ("display", "grid"),
                ("place-items", "center"),
                ("width", "24px"),
                ("height", "24px"),
                ("border-radius", "var(--radius-sm)"),
                ("color", MUTED),
                ("background", BG_CARD),
                ("cursor", "grab"),
                ("touch-action", "none"),
                ("opacity", "0"),
                ("transition", "opacity .15s"),
                ("outline", "none"),
            ]),
        )
        .nest("& [data-dg-tile]:is(:hover, :focus-within) [data-dg-grip-only]", pairs(&[("opacity", "1")]))
        .nest("& [data-dg-grip-only]:focus-visible", forms::focus_ring())
        .nest("@media (hover: none)", pairs(&[]).nest("& [data-dg-grip-only]", pairs(&[("opacity", "1")])))
        .nest(
            "& [data-dg-resize]",
            pairs(&[
                ("position", "absolute"),
                ("bottom", "0"),
                ("inset-inline-end", "0"),
                ("z-index", "1"),
                ("display", "grid"),
                ("place-items", "center"),
                ("width", "20px"),
                ("height", "20px"),
                ("color", MUTED),
                ("cursor", "nwse-resize"),
                ("touch-action", "none"),
                ("opacity", ".5"),
            ]),
        )
        .nest("& [data-dg-resize]:dir(rtl)", pairs(&[("cursor", "nesw-resize")]))
        .nest("& [data-dg-resize]:dir(rtl) svg", pairs(&[("scale", "-1 1")]))
        .nest("& [data-dg-tile]:hover [data-dg-resize]", pairs(&[("opacity", "1")]))
        // Where a dragged tile will land.
        .nest(
            "& [data-dg-ghost]",
            pairs(&[
                ("border-radius", "var(--radius-lg)"),
                ("background", "color-mix(in srgb, var(--color-primary) 10%, transparent)"),
                ("border", "1.5px dashed color-mix(in srgb, var(--color-primary) 45%, transparent)"),
                ("pointer-events", "none"),
            ]),
        )
        .nest("& [data-dg-ghost][hidden]", pairs(&[("display", "none")]))
        .nest(
            "& :is([data-dg-live], [data-dg-help])",
            pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("overflow", "hidden"), ("clip-path", "inset(50%)"), ("white-space", "nowrap")]),
        )
        .nest("& [data-dg-tile]:has([data-dg-head]:focus-visible)", pairs(&[("border-color", RING)]))
});

const GRIP: &str = r#"<svg width="10" height="14" viewBox="0 0 10 14" fill="currentColor" aria-hidden="true"><circle cx="2.5" cy="2.5" r="1.2"/><circle cx="7.5" cy="2.5" r="1.2"/><circle cx="2.5" cy="7" r="1.2"/><circle cx="7.5" cy="7" r="1.2"/><circle cx="2.5" cy="11.5" r="1.2"/><circle cx="7.5" cy="11.5" r="1.2"/></svg>"#;
const CORNER: &str = r#"<svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" aria-hidden="true"><path d="M9 3 3 9M9 7 7 9"/></svg>"#;

/// A tile's place and limits, in grid cells.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Item {
    pub id: String,
    pub x: i64,
    pub y: i64,
    pub w: i64,
    pub h: i64,
    pub min_w: i64,
    pub min_h: i64,
    pub max_w: i64,
    pub max_h: i64,
    pub locked: bool,
}

fn collides(a: &Item, b: &Item) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

/// The layout with every tile on the grid and none overlapping: tiles without a place
/// take the first free one after the others have settled, overlapping tiles move down, and with `compact` each tile
/// floats up as far as it can. Locked tiles stay put. `dashboard_grid.js` does the same.
pub(crate) fn place(items: &mut [Item], placed: &[bool], cols: i64, compact: bool) {
    for it in items.iter_mut() {
        let (hi_w, hi_h) = (it.max_w.min(cols).max(1), it.max_h.max(1));
        it.w = it.w.clamp(it.min_w.clamp(1, hi_w), hi_w);
        it.h = it.h.clamp(it.min_h.clamp(1, hi_h), hi_h);
        it.x = it.x.clamp(0, cols - it.w);
        it.y = it.y.max(0);
    }
    // The tiles with a place settle first; then those without one, in order, take the
    // first spot, row by row, where they fit.
    let mut fixed: Vec<Item> = items.iter().zip(placed).filter(|(_, p)| **p).map(|(t, _)| t.clone()).collect();
    settle(&mut fixed, compact);
    let mut k = 0;
    for (it, p) in items.iter_mut().zip(placed) {
        if *p {
            *it = fixed[k].clone();
            k += 1;
        }
    }
    for i in 0..items.len() {
        if placed[i] {
            continue;
        }
        let others: Vec<Item> = items.iter().enumerate().filter(|(j, _)| placed[*j] || *j < i).map(|(_, t)| t.clone()).collect();
        let mut it = items[i].clone();
        'find: for y in 0.. {
            for x in 0..=cols - it.w {
                it.x = x;
                it.y = y;
                if !others.iter().any(|o| collides(o, &it)) {
                    break 'find;
                }
            }
        }
        items[i] = it;
    }
    settle(items, compact);
}

/// Locked tiles first, then the rest by row and column: each floats up (with `compact`)
/// and then down past anything it overlaps.
fn settle(items: &mut [Item], compact: bool) {
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by_key(|&i| (!items[i].locked, items[i].y, items[i].x));
    let mut done: Vec<Item> = Vec::new();
    for i in order {
        let mut it = items[i].clone();
        if !it.locked {
            if compact {
                while it.y > 0 && !done.iter().any(|d| collides(d, &Item { y: it.y - 1, ..it.clone() })) {
                    it.y -= 1;
                }
            }
            while let Some(d) = done.iter().find(|d| collides(d, &it)) {
                it.y = d.y + d.h;
            }
        }
        items[i] = it.clone();
        done.push(it);
    }
}

fn num(cx: &RenderCx<'_>, node: &Node, name: &str) -> Option<f64> {
    cx.prop(node, name).filter(|v| !v.is_null()).map(|v| expr::to_num(&v)).filter(|n| n.is_finite())
}

impl Component for DashboardGrid {
    fn name(&self) -> &'static str {
        "ui-dashboard-grid"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Tiles (`ui-tile` children) on a grid, to drag by their header and resize from their corner, or move with the arrow keys; \
                  the others make way. `bind` keeps the layout in state. On a narrow screen the tiles stack.",
            props: PROPS,
            parts: &["tile", "header"],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        if let Some(c) = node.children.iter().find(|c| c.kind != "ui-tile") {
            return Err(RenderError::new(c, "`ui-dashboard-grid` takes only `ui-tile` children"));
        }
        let int = |name: &str, default: f64| num(cx, node, name).unwrap_or(default);
        let cols = int("columns", 12.0).round() as i64;
        if !(1..=48).contains(&cols) {
            return Err(RenderError::new(node, "`columns` must be from 1 to 48"));
        }
        let (row, gap, stack) = (int("rowHeight", 80.0).max(8.0), int("gap", 16.0).max(0.0), int("stackBelow", 640.0).max(0.0));
        let truthy = |name: &str, default: bool| cx.prop(node, name).map_or(default, |v| expr::truthy(&v));
        let (editable, compact) = (truthy("editable", true), truthy("compact", true));
        let key = cx.prop_string(node, "bind");

        // The tiles' places: from `bind`'s layout when it has them, else their own props.
        let saved = key.as_ref().map(|k| cx.lookup(std::slice::from_ref(k))).unwrap_or(Json::Null);
        let saved_at = |id: &str| saved.as_array().and_then(|a| a.iter().find(|l| l.get("id").map(expr::stringify).as_deref() == Some(id)));
        let mut items = Vec::new();
        let mut placed = Vec::new();
        let mut ids = std::collections::HashSet::new();
        for (i, t) in node.children.iter().enumerate() {
            let id = cx.prop_string(t, "id").unwrap_or_else(|| format!("tile-{}", i + 1));
            if !ids.insert(id.clone()) {
                return Err(RenderError::new(t, format!("two tiles have the id `{id}`")));
            }
            let n = |name: &str| num(cx, t, name).map(|v| v.round() as i64);
            let from = |name: &str| saved_at(&id).and_then(|l| l.get(name)).map(expr::to_num).filter(|v| v.is_finite()).map(|v| v.round() as i64);
            let (x, y) = (from("x").or(n("x")), from("y").or(n("y")));
            placed.push(x.is_some() && y.is_some());
            items.push(Item {
                x: x.unwrap_or(0),
                y: y.unwrap_or(0),
                w: from("w").or(n("w")).unwrap_or(4),
                h: from("h").or(n("h")).unwrap_or(2),
                min_w: n("minW").unwrap_or(1),
                min_h: n("minH").unwrap_or(1),
                max_w: n("maxW").unwrap_or(cols),
                max_h: n("maxH").unwrap_or(i64::MAX / 4),
                locked: cx.prop(t, "locked").is_some_and(|v| expr::truthy(&v)),
                id,
            });
        }
        // Where they go without a saved layout, for when `bind` is cleared.
        let mut defaults = items.clone();
        let mut own = placed.clone();
        for (i, t) in node.children.iter().enumerate() {
            let n = |name: &str| num(cx, t, name).map(|v| v.round() as i64);
            own[i] = n("x").is_some() && n("y").is_some();
            let d = &mut defaults[i];
            (d.x, d.y, d.w, d.h) = (n("x").unwrap_or(0), n("y").unwrap_or(0), n("w").unwrap_or(4), n("h").unwrap_or(2));
        }
        place(&mut items, &placed, cols, compact);
        place(&mut defaults, &own, cols, compact);

        cx.global_js("ui-dashboard-grid", SCRIPT);
        crate::flips(cx);
        let id = instance_id(cx, node, "dg");
        let help = id_with(&id, "-help");
        let mut root = cx.element("div", node);
        root.base_style_ref(&GRID).base_style(
            pairs(&[("--dg-cols", &cols.to_string()), ("--dg-row", &format!("{row}px")), ("--dg-gap", &format!("{gap}px"))]).nest(
                format!("@container (max-width: {}px)", stack - 0.02),
                pairs(&[])
                    .nest("& [data-dg-grid]", pairs(&[("grid-template-columns", "minmax(0, 1fr)")]))
                    .nest("& [data-dg-tile]", pairs(&[("grid-column", "1 / -1 !important"), ("grid-row", "auto / span var(--h) !important")]))
                    .nest("& :is([data-dg-resize], [data-dg-grip-only], [data-dg-head] [data-dg-grip], [data-dg-ghost])", pairs(&[("display", "none !important")]))
                    .nest("& [data-dg-head][data-dg-handle]", pairs(&[("cursor", "auto")])),
            ),
        );
        let max = |v: i64| if v >= i64::MAX / 8 { Json::Null } else { json!(v) };
        root.behavior(
            "ui-dashboard-grid",
            json!({
                "columns": cols,
                "gap": gap,
                "row": row,
                "stack": stack,
                "bind": key,
                "editable": editable,
                "compact": compact,
                "items": items.iter().map(|it| json!({
                    "id": it.id, "x": it.x, "y": it.y, "w": it.w, "h": it.h,
                    "minW": it.min_w, "minH": it.min_h, "maxW": max(it.max_w), "maxH": max(it.max_h), "locked": it.locked,
                })).collect::<Vec<_>>(),
                "defaults": defaults.iter().map(|it| json!({ "id": it.id, "x": it.x, "y": it.y, "w": it.w, "h": it.h })).collect::<Vec<_>>(),
            }),
        );
        cx.open(&root, node)?;
        let mut grid = Element::new("div");
        let label = cx.prop_string(node, "label").unwrap_or_else(|| cx.label("Dashboard"));
        grid.attr("data-dg-grid", true).attr("role", "list").attr("aria-label", label);
        cx.open(&grid, node)?;
        // Stacked, tiles go in reading order.
        let mut order: Vec<usize> = (0..items.len()).collect();
        order.sort_by_key(|&i| (items[i].y, items[i].x));
        for (i, (t, it)) in node.children.iter().zip(&items).enumerate() {
            let rank = order.iter().position(|&o| o == i).unwrap_or(i);
            tile(cx, node, t, it, rank, editable, &help)?;
        }
        let mut ghost = Element::new("div");
        ghost.attr("data-dg-ghost", true).attr("hidden", true).attr("aria-hidden", "true");
        cx.open(&ghost, node)?;
        cx.close("div");
        cx.close("div");
        if editable {
            let mut h = Element::new("p");
            h.attr("id", help).attr("data-dg-help", true).text(lit(cx.label("Arrow keys move the tile; Shift and the arrow keys resize it.")));
            cx.open(&h, node)?;
            cx.close("p");
        }
        let mut live = Element::new("div");
        live.attr("data-dg-live", true).attr("role", "status").attr("aria-live", "polite");
        cx.open(&live, node)?;
        cx.close("div");
        cx.close("div");
        Ok(())
    }
}

fn tile(cx: &mut RenderCx<'_>, grid: &Node, t: &Node, it: &Item, rank: usize, editable: bool, help: &irender_core::Val) -> Result<(), RenderError> {
    let movable = editable && !it.locked;
    let name = cx.prop_string(t, "title").unwrap_or_else(|| it.id.clone());
    let mut el = cx.part("section", grid, "tile");
    el.base_style(pairs(&[
        ("grid-column", &format!("{} / span {}", it.x + 1, it.w)),
        ("grid-row", &format!("{} / span {}", it.y + 1, it.h)),
        ("order", &rank.to_string()),
        ("--h", &it.h.to_string()),
    ]))
    .attr("data-dg-tile", true)
    .attr("data-id", it.id.clone())
    .attr("role", "listitem")
    .attr("aria-label", name.clone());
    if it.locked {
        el.attr("data-locked", true);
    }
    cx.open(&el, t)?;
    let handle = |cx: &RenderCx<'_>, h: &mut Element<'_>| {
        h.attr("data-dg-handle", true)
            .attr("tabindex", "0")
            .attr("role", "button")
            .attr("aria-roledescription", cx.label("movable tile"))
            .attr("aria-label", cx.label_with("Move {name}", &[("name", &name)]))
            .attr("aria-describedby", help.clone());
    };
    if let Some(title) = t.prop("title") {
        let mut head = cx.part("header", grid, "header");
        head.attr("data-dg-head", true);
        if movable {
            handle(cx, &mut head);
        }
        cx.open(&head, t)?;
        if movable {
            let mut g = Element::new("span");
            g.attr("data-dg-grip", true);
            cx.open(&g, t)?;
            cx.markup(GRIP);
            cx.close("span");
        }
        let mut h3 = Element::new("h3");
        h3.text(title);
        cx.open(&h3, t)?;
        cx.close("h3");
        cx.close("header");
    } else if movable {
        let mut g = Element::new("span");
        g.attr("data-dg-grip-only", true);
        handle(cx, &mut g);
        cx.open(&g, t)?;
        cx.markup(GRIP);
        cx.close("span");
    }
    let mut body = Element::new("div");
    body.attr("data-dg-body", true);
    cx.open(&body, t)?;
    cx.children(t)?;
    cx.close("div");
    if movable {
        let mut r = Element::new("span");
        r.attr("data-dg-resize", true).attr("aria-hidden", "true");
        cx.open(&r, t)?;
        cx.markup(CORNER);
        cx.close("span");
    }
    cx.close("section");
    Ok(())
}

impl Component for Tile {
    fn name(&self) -> &'static str {
        "ui-tile"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "One tile of a `ui-dashboard-grid`: its place (`x`, `y`, `w`, `h` in grid cells), a `title` to drag it by, and its children. \
                  Read by the grid, never rendered on its own.",
            props: TILE_PROPS,
            parts: &[],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, _cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        Err(RenderError::new(node, "`ui-tile` must be a direct child of `ui-dashboard-grid`"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, x: i64, y: i64, w: i64, h: i64) -> Item {
        Item { id: id.into(), x, y, w, h, min_w: 1, min_h: 1, max_w: 12, max_h: 100, locked: false }
    }

    #[test]
    fn tiles_float_up_and_overlaps_move_down() {
        let mut l = vec![item("a", 0, 3, 6, 2), item("b", 0, 3, 6, 2), item("c", 6, 9, 6, 1)];
        place(&mut l, &[true, true, true], 12, true);
        assert_eq!(l.iter().map(|i| (i.x, i.y)).collect::<Vec<_>>(), [(0, 0), (0, 2), (6, 0)]);
        // Without compacting, only the overlap is undone.
        let mut l = vec![item("a", 0, 3, 6, 2), item("b", 0, 3, 6, 2)];
        place(&mut l, &[true, true], 12, false);
        assert_eq!(l.iter().map(|i| i.y).collect::<Vec<_>>(), [3, 5]);
    }

    #[test]
    fn unplaced_tiles_take_the_first_free_spot_and_locked_ones_stay() {
        let mut l = vec![item("a", 0, 0, 8, 2), item("b", 0, 0, 4, 2), item("c", 0, 0, 6, 1)];
        place(&mut l, &[true, false, false], 12, true);
        assert_eq!(l.iter().map(|i| (i.x, i.y)).collect::<Vec<_>>(), [(0, 0), (8, 0), (0, 2)]);
        let mut l = vec![item("a", 0, 0, 12, 1), Item { locked: true, ..item("lock", 0, 1, 12, 1) }];
        l[0].y = 1;
        place(&mut l, &[true, true], 12, true);
        assert_eq!(l.iter().map(|i| (i.id.as_str(), i.y)).collect::<Vec<_>>(), [("a", 0), ("lock", 1)]);
        // Too wide is cut to the grid; past the edge comes back in.
        let mut l = vec![item("a", 10, 0, 20, 1), item("b", 11, 1, 4, 1)];
        place(&mut l, &[true, true], 12, true);
        assert_eq!(l.iter().map(|i| (i.x, i.w)).collect::<Vec<_>>(), [(0, 12), (8, 4)]);
    }
}
