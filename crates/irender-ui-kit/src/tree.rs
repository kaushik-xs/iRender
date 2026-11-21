//! `ui-tree`: a hierarchy of nodes (`{ id, label, children }`) to browse, select, check,
//! search, add to, delete from and rearrange. The server draws it from the starting
//! nodes; `tree.js` draws it again whenever they change, and handles the keyboard
//! (the ARIA tree pattern), dragging, and moves with Alt and the arrow keys. Moves and
//! deletions are written back to `bindNodes`, and every change fires an event.

use crate::data_table::dynamic;
use crate::{ACCENT, MUTED, PRIMARY, PRIMARY_FILL, TEXT, forms, lit};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::collections::HashSet;
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("tree.js");

pub struct Tree;

const PROPS: &[PropDef] = &[
    PropDef::new("nodes", PropKind::Any, "The nodes: `{ id, label (or name), icon, children }`, usually from state, e.g. `{{ locations }}`").required(),
    PropDef::new("bindNodes", PropKind::StateKey, "State key the nodes are in: moves and deletions are written back to it"),
    PropDef::new("bind", PropKind::StateKey, "State key for the selected node's id"),
    PropDef::new("bindNode", PropKind::StateKey, "State key for the selected node itself, to show its fields beside the tree"),
    PropDef::new("checkable", PropKind::Bool, "A checkbox per node; checking a node checks the nodes under it"),
    PropDef::new("bindChecked", PropKind::StateKey, "State key for the checked nodes' ids"),
    PropDef::new(
        "addable",
        PropKind::Bool,
        "An Add button, and one on each node. With `bindNodes` they ask for a name and add the node; either way they write the parent's id (null for the top) to `bindParent` and fire `add`",
    ),
    PropDef::new("bindParent", PropKind::StateKey, "State key that gets the parent's id when Add is pressed"),
    PropDef::new("deletable", PropKind::Bool, "A delete button on each node, Delete on the keyboard, and Delete for the checked nodes"),
    PropDef::new("movable", PropKind::Bool, "Drag nodes to a new place, or move the focused one with Alt and the arrow keys"),
    PropDef::new("renamable", PropKind::Bool, "Rename a node in place: F2, or a double click on its name"),
    PropDef::new(
        "childrenUrl",
        PropKind::Any,
        "Loads a node's children when it's first opened, for nodes with `children: true`: GET with `?parent=<id>`, answering a list of nodes (or `{ nodes }`)",
    ),
    PropDef::new("search", PropKind::Any, "A search box above the tree: `true`, or its placeholder. Shows matches with the nodes above them"),
    PropDef::new("counts", PropKind::Bool, "Show how many nodes are under each node"),
    PropDef::new("expanded", PropKind::Any, "Open to start with: `true` for every node, a number of levels, or a list of ids (default: the selected node's parents)"),
    PropDef::new("icons", PropKind::Any, "Icon names nodes loaded later may use (from `childrenUrl` or state), so the icon kit draws them"),
    PropDef::new("title", PropKind::Any, "A heading above the tree, which also names it for screen readers"),
    PropDef::new("label", PropKind::Any, "Names the tree for screen readers when there's no `title` (default `Tree`)"),
    PropDef::new("addLabel", PropKind::Any, "The Add button's text (default `Add`)"),
    PropDef::new("emptyText", PropKind::Any, "Shown when there are no nodes (default `Nothing here yet.`)"),
    PropDef::new("height", PropKind::Any, "Most it grows to before it scrolls: a number (px) or CSS length"),
];

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

static ROOT: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(8)), ("min-width", "0"), ("color", TEXT), ("font-size", fs!(14))])
        .nest("& [data-tree-head]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("align-items", "center"), ("gap", gap!(8))]))
        .nest("& [data-tree-title]", pairs(&[("margin-block", "0 0"), ("margin-inline", "0 auto"), ("font-size", fs!(15)), ("font-weight", "600")]))
        .nest("& [data-tree-search]", pairs(&[("flex", "1 1 160px")]))
        .nest(
            "& [data-tree-bulk]",
            pairs(&[
                ("display", "flex"),
                ("align-items", "center"),
                ("gap", gap!(8)),
                ("padding-block", pad!(4, 4)), ("padding-inline", pad!(12, 4)),
                ("border-radius", "var(--radius-md)"),
                ("background", ACCENT),
                ("font-size", fs!(13)),
            ]),
        )
        .nest("& [data-tree-bulk] > span", pairs(&[("margin-inline-end", "auto"), ("font-weight", "500")]))
        .nest("& [hidden]", pairs(&[("display", "none")]))
        .nest("& [data-tree-empty]", pairs(&[("margin", "0"), ("padding", pad!(16, 8)), ("color", MUTED), ("text-align", "center")]))
        .nest(
            "& [data-tree-live]",
            pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("overflow", "hidden"), ("clip-path", "inset(50%)"), ("white-space", "nowrap")]),
        )
});

static TREE: LazyLock<StyleBlock> = LazyLock::new(|| {
    let row = pairs(&[
        ("display", "flex"),
        ("align-items", "center"),
        ("gap", gap!(6)),
        ("min-height", ctl!(32)),
        ("padding", pad!(0, 4)),
        ("padding-inline-start", "calc(var(--depth, 0) * 18px * var(--space-scale, 1) + 4px)"),
        ("border-radius", "var(--radius-md)"),
        ("cursor", "pointer"),
        ("user-select", "none"),
    ]);
    pairs(&[("margin", "0"), ("padding", "0"), ("list-style", "none"), ("overflow-y", "auto"), ("scrollbar-width", "thin")])
        .nest("& ul", pairs(&[("margin", "0"), ("padding", "0"), ("list-style", "none")]))
        .nest("& [role=treeitem]", pairs(&[("outline", "none")]))
        .nest("& [data-tree-row]", row)
        .nest("& [data-tree-row]:hover", pairs(&[("background", ACCENT)]))
        // A long press picks a node up, rather than selecting its text.
        .nest("@media (pointer: coarse)", pairs(&[]).nest("& [data-tree-row][draggable=true]", pairs(&[("user-select", "none"), ("-webkit-touch-callout", "none")])))
        .nest("& [aria-selected=true] > [data-tree-row]", pairs(&[("background", ACCENT), ("color", PRIMARY), ("font-weight", "500")]))
        .nest("& [role=treeitem]:focus-visible > [data-tree-row]", forms::focus_ring().set("outline-offset", "-2px"))
        .nest(
            "& [data-tree-twisty]",
            pairs(&[("display", "inline-flex"), ("flex", "none"), ("width", "calc(16px * var(--size-icon, 1))"), ("justify-content", "center"), ("color", MUTED), ("transition", "transform .15s")]),
        )
        .nest("& [aria-expanded=true] > [data-tree-row] [data-tree-twisty]", pairs(&[("transform", "rotate(90deg)")]))
        // Its icon is mirrored right to left (pointing left): it turns the other way to point down.
        .nest("&:dir(rtl) [aria-expanded=true] > [data-tree-row] [data-tree-twisty]", pairs(&[("transform", "rotate(-90deg)")]))
        .nest("@media (prefers-reduced-motion: reduce)", pairs(&[]).nest("& [data-tree-twisty]", pairs(&[("transition", "none")])))
        // Loading its children: the chevron faded and turning.
        .nest("& [data-tree-twisty][data-loading] svg", pairs(&[("opacity", "0.5"), ("animation", "ui-spin 0.8s linear infinite")]))
        .nest("& span[data-tree-icon]", pairs(&[("display", "inline-flex"), ("flex", "none"), ("align-items", "center"), ("color", MUTED), ("line-height", "1")]))
        .nest("& [data-tree-rename]", pairs(&[("flex", "1 1 auto"), ("min-width", "0"), ("height", "26px"), ("padding-block", "0")]))
        .nest(
            "& [data-tree-check]",
            pairs(&[
                ("display", "inline-flex"),
                ("flex", "none"),
                ("align-items", "center"),
                ("justify-content", "center"),
                ("width", "16px"),
                ("height", "16px"),
                ("box-sizing", "border-box"),
                ("border", "1px solid var(--color-border)"),
                ("border-radius", "4px"),
                ("color", "var(--color-on-primary, #fff)"),
            ]),
        )
        .nest(
            "& :is([aria-checked=true], [aria-checked=mixed]) > [data-tree-row] [data-tree-check]",
            pairs(&[("border-color", PRIMARY_FILL), ("background", PRIMARY_FILL)]),
        )
        .nest("& [data-tree-check] svg", pairs(&[("display", "none")]))
        .nest("& [aria-checked=true] > [data-tree-row] [data-tree-check] [data-on]", pairs(&[("display", "block")]))
        .nest("& [aria-checked=mixed] > [data-tree-row] [data-tree-check] [data-some]", pairs(&[("display", "block")]))
        .nest("& [data-tree-label]", pairs(&[("flex", "1 1 auto"), ("min-width", "0"), ("overflow", "hidden"), ("text-overflow", "ellipsis"), ("white-space", "nowrap")]))
        .nest("& [data-tree-name]", pairs(&[("flex", "1 1 auto"), ("height", ctl!(28)), ("margin", concat!(gap!(2), " 0"))]))
        .nest("& [data-tree-label] mark", pairs(&[("background", "color-mix(in srgb, var(--color-primary) 18%, transparent)"), ("color", "inherit"), ("border-radius", "2px")]))
        .nest("& [data-tree-count]", pairs(&[("flex", "none"), ("color", MUTED), ("font-size", fs!(12)), ("font-weight", "400"), ("font-variant-numeric", "tabular-nums")]))
        .nest("& [data-tree-actions]", pairs(&[("display", "flex"), ("flex", "none"), ("gap", gap!(2)), ("opacity", "0")]))
        .nest("& [data-tree-row]:hover [data-tree-actions], & [role=treeitem]:focus > [data-tree-row] [data-tree-actions]", pairs(&[("opacity", "1")]))
        .nest("@media (hover: none)", pairs(&[]).nest("& [data-tree-actions]", pairs(&[("opacity", "1")])))
        .nest(
            "& [data-tree-actions] button",
            pairs(&[
                ("display", "inline-flex"),
                ("align-items", "center"),
                ("justify-content", "center"),
                ("width", "24px"),
                ("height", "24px"),
                ("padding", "0"),
                ("border", "0"),
                ("border-radius", "var(--radius-sm)"),
                ("background", "transparent"),
                ("color", MUTED),
                ("cursor", "pointer"),
            ]),
        )
        .nest("& [data-tree-actions] button:hover", pairs(&[("background", "color-mix(in srgb, var(--color-text) 8%, transparent)"), ("color", TEXT)]))
        .nest("& [data-tree-actions] [data-tree-delete]:hover", pairs(&[("color", "var(--color-destructive, #dc2626)")]))
        // Where a dragged node would land.
        .nest("& [data-drop=inside] > [data-tree-row]", pairs(&[("box-shadow", "inset 0 0 0 2px var(--color-primary)")]))
        .nest("& [data-drop=before] > [data-tree-row]", pairs(&[("box-shadow", "inset 0 2px 0 var(--color-primary)")]))
        .nest("& [data-drop=after] > [data-tree-row]", pairs(&[("box-shadow", "inset 0 -2px 0 var(--color-primary)")]))
        .nest("& [data-dragging] > [data-tree-row]", pairs(&[("opacity", "0.5")]))
});

// Decorative: the rows and buttons around them are labelled.
const CHEVRON: &str = r#"<svg data-flip width="14" height="14" style="width:calc(14px * var(--size-icon, 1));height:calc(14px * var(--size-icon, 1))" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M6 3.5 10.5 8 6 12.5"/></svg>"#;
const CHECK: &str = r#"<svg data-on width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3.5 8.5 6.5 11.5 12.5 4.5"/></svg><svg data-some width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" aria-hidden="true"><path d="M4 8h8"/></svg>"#;
const PLUS: &str = r#"<svg width="14" height="14" style="width:calc(14px * var(--size-icon, 1));height:calc(14px * var(--size-icon, 1))" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" aria-hidden="true"><path d="M8 3v10M3 8h10"/></svg>"#;
const TRASH: &str = r#"<svg width="14" height="14" style="width:calc(14px * var(--size-icon, 1));height:calc(14px * var(--size-icon, 1))" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M2.5 4.5h11M6.5 4.5V3h3v1.5M4 4.5l.7 8.5h6.6l.7-8.5"/></svg>"#;

/// What the server needs to draw the starting tree, and what the browser gets.
struct Draw<'a> {
    checkable: bool,
    addable: bool,
    deletable: bool,
    movable: bool,
    counts: bool,
    lazy: bool,
    selected: Option<String>,
    checked: HashSet<String>,
    open: &'a dyn Fn(&str, usize) -> bool,
}

/// The icon names the nodes use, once each, in order.
fn icon_names(nodes: &[Json], out: &mut Vec<String>) {
    for n in nodes {
        if let Some(i) = n.get("icon").filter(|v| !v.is_null()).map(expr::stringify)
            && !out.contains(&i)
        {
            out.push(i);
        }
        icon_names(children_of(n), out);
    }
}

fn id_of(n: &Json) -> Option<String> {
    n.get("id").filter(|v| !v.is_null()).map(expr::stringify)
}
fn label_of(n: &Json) -> String {
    n.get("label").or_else(|| n.get("name")).map(expr::stringify).unwrap_or_default()
}
fn children_of(n: &Json) -> &[Json] {
    n.get("children").and_then(Json::as_array).map_or(&[], Vec::as_slice)
}
fn descendants(n: &Json) -> usize {
    children_of(n).iter().map(|c| 1 + descendants(c)).sum()
}
/// Whether every node under `n` (and `n`) is checked, some are, or none.
fn check_state(n: &Json, checked: &HashSet<String>) -> &'static str {
    let me = id_of(n).is_some_and(|i| checked.contains(&i));
    let kids = children_of(n);
    if kids.is_empty() {
        return if me { "true" } else { "false" };
    }
    let states: Vec<_> = kids.iter().map(|c| check_state(c, checked)).collect();
    if states.iter().all(|s| *s == "true") {
        "true"
    } else if me || states.iter().any(|s| *s != "false") {
        "mixed"
    } else {
        "false"
    }
}
/// The ids from the top down to `id`, without it.
fn ancestors(nodes: &[Json], id: &str) -> Option<Vec<String>> {
    for n in nodes {
        let me = id_of(n).unwrap_or_default();
        if me == id {
            return Some(Vec::new());
        }
        if let Some(mut path) = ancestors(children_of(n), id) {
            path.insert(0, me);
            return Some(path);
        }
    }
    None
}

fn items(cx: &mut RenderCx<'_>, node: &Node, nodes: &[Json], depth: usize, d: &Draw<'_>, first: &mut bool) -> Result<(), RenderError> {
    for (i, n) in nodes.iter().enumerate() {
        let Some(id) = id_of(n) else { continue };
        let label = label_of(n);
        let kids = children_of(n);
        // `children: true` with a `childrenUrl`: it has some, loaded when it opens.
        let later = d.lazy && kids.is_empty() && (n.get("children") == Some(&Json::Bool(true)) || n.get("hasChildren") == Some(&Json::Bool(true)));
        let open = !kids.is_empty() && (d.open)(&id, depth);
        let selected = d.selected.as_deref() == Some(id.as_str());
        // One stop in the tab order: the selected node, or the first.
        let stop = selected || (d.selected.is_none() && *first);
        *first = false;
        let mut li = Element::new("li");
        li.attr("role", "treeitem")
            .attr("data-tree-id", id.clone())
            .attr("aria-level", lit(depth + 1))
            .attr("aria-setsize", lit(nodes.len()))
            .attr("aria-posinset", lit(i + 1))
            .attr("aria-selected", if selected { "true" } else { "false" })
            .attr("tabindex", lit(if stop { 0 } else { -1 }));
        if !kids.is_empty() || later {
            li.attr("aria-expanded", if open { "true" } else { "false" });
        }
        if d.checkable {
            li.attr("aria-checked", check_state(n, &d.checked));
        }
        cx.open(&li, node)?;
        let mut row = Element::new("div");
        row.attr("data-tree-row", true).base_style(StyleBlock::new().set("--depth", depth.to_string()));
        if d.movable {
            row.attr("draggable", "true");
        }
        cx.open(&row, node)?;
        let mut twisty = Element::new("span");
        twisty.attr("data-tree-twisty", true).attr("aria-hidden", "true");
        cx.open(&twisty, node)?;
        if !kids.is_empty() || later {
            cx.markup(CHEVRON);
        }
        cx.close("span");
        if d.checkable {
            let mut b = Element::new("span");
            b.attr("data-tree-check", true).attr("aria-hidden", "true");
            cx.open(&b, node)?;
            cx.markup(CHECK);
            cx.close("span");
        }
        if let Some(i) = n.get("icon").filter(|v| !v.is_null()) {
            let mut wrap = Element::new("span");
            wrap.attr("data-tree-icon", true).attr("aria-hidden", "true");
            cx.open(&wrap, node)?;
            crate::icon(cx, node, &lit(expr::stringify(i)), 16)?;
            cx.close("span");
        }
        let mut l = Element::new("span");
        l.attr("data-tree-label", true).text(lit(label.clone()));
        cx.open(&l, node)?;
        cx.close("span");
        if d.counts && !kids.is_empty() {
            let mut c = Element::new("span");
            c.attr("data-tree-count", true).attr("aria-hidden", "true").text(lit(descendants(n).to_string()));
            cx.open(&c, node)?;
            cx.close("span");
        }
        if d.addable || d.deletable {
            let mut a = Element::new("span");
            a.attr("data-tree-actions", true);
            cx.open(&a, node)?;
            // Labels: "Add under {name}", "Delete {name}"
            for (on, attr, name, icon) in [(d.addable, "data-tree-add", "Add under {name}", PLUS), (d.deletable, "data-tree-delete", "Delete {name}", TRASH)] {
                if on {
                    let mut b = Element::new("button");
                    let words = cx.label_with(name, &[("name", &label)]);
                    b.attr("type", "button").attr(attr, true).attr("tabindex", lit(-1)).attr("aria-label", words.clone()).attr("title", words);
                    cx.open(&b, node)?;
                    cx.markup(icon);
                    cx.close("button");
                }
            }
            cx.close("span");
        }
        cx.close("div");
        if !kids.is_empty() {
            let mut group = Element::new("ul");
            group.attr("role", "group");
            if !open {
                group.attr("hidden", true);
            }
            cx.open(&group, node)?;
            items(cx, node, kids, depth + 1, d, first)?;
            cx.close("ul");
        }
        cx.close("li");
    }
    Ok(())
}

impl Component for Tree {
    fn name(&self) -> &'static str {
        "ui-tree"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A tree of nodes to browse, select, check, search, add to, delete from and rearrange by dragging or with Alt and \
                  the arrow keys. Nodes can be renamed in place (`renamable`), carry an `icon`, and load their children \
                  from an API when first opened (`childrenUrl`). Changes go to state (`bindNodes`) and fire `select`, `add`, `delete`, \
                  `move` and `rename` events.",
            props: PROPS,
            parts: &["tree"],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        crate::flips(cx);
        let flag = |cx: &RenderCx<'_>, name: &str| cx.prop(node, name).is_some_and(|v| expr::truthy(&v));
        let (checkable, addable, deletable, movable, counts, renamable) =
            (flag(cx, "checkable"), flag(cx, "addable"), flag(cx, "deletable"), flag(cx, "movable"), flag(cx, "counts"), flag(cx, "renamable"));
        let children_url = node.prop("childrenUrl").map(|v| dynamic(cx, node, "childrenUrl", v)).transpose()?;
        let key = |name: &str| cx.prop_string(node, name);
        let (bind, bind_checked, bind_nodes, bind_parent, bind_node) = (key("bind"), key("bindChecked"), key("bindNodes"), key("bindParent"), key("bindNode"));
        let nodes = match cx.prop(node, "nodes") {
            Some(Json::Array(a)) => a,
            None | Some(Json::Null) => Vec::new(),
            Some(_) => return Err(RenderError::new(node, "`nodes` must be a list of `{ id, label, children }`")),
        };
        let expanded = cx.prop(node, "expanded").unwrap_or(Json::Null);
        let selected = bind.as_ref().map(|k| cx.lookup(std::slice::from_ref(k))).filter(|v| !v.is_null()).map(|v| expr::stringify(&v));
        let checked: HashSet<String> = match bind_checked.as_ref().map(|k| cx.lookup(std::slice::from_ref(k))) {
            Some(Json::Array(ids)) => ids.iter().map(expr::stringify).collect(),
            _ => HashSet::new(),
        };
        let path: HashSet<String> = selected.as_deref().and_then(|s| ancestors(&nodes, s)).unwrap_or_default().into_iter().collect();
        let ids: HashSet<String> = match &expanded {
            Json::Array(a) => a.iter().map(expr::stringify).collect(),
            _ => HashSet::new(),
        };
        let open = |id: &str, depth: usize| -> bool {
            path.contains(id)
                || ids.contains(id)
                || match &expanded {
                    Json::Bool(b) => *b,
                    Json::Number(n) => (depth as f64) < n.as_f64().unwrap_or(0.0),
                    _ => false,
                }
        };
        let search = match cx.prop(node, "search") {
            None | Some(Json::Null) | Some(Json::Bool(false)) => None,
            Some(Json::Bool(true)) => Some(cx.label("Search…")),
            Some(v) => Some(expr::stringify(&v)),
        };
        let title = node.prop("title").cloned();

        crate::touch_drag(cx);
        cx.global_js("ui-tree", SCRIPT);
        if children_url.is_some() {
            cx.global_css("ui-spin", crate::SPIN);
        }
        let config = json!({
            "nodes": dynamic(cx, node, "nodes", node.prop("nodes").expect("required"))?,
            "bind": bind, "bindChecked": bind_checked, "bindNodes": bind_nodes, "bindParent": bind_parent, "bindNode": bind_node,
            "checkable": checkable, "addable": addable, "deletable": deletable, "movable": movable, "counts": counts,
            "renamable": renamable, "childrenUrl": children_url, "expanded": expanded,
        });
        let mut root = cx.element("div", node);
        root.base_style_ref(&ROOT).behavior("ui-tree", config);
        if let Some(h) = cx.prop(node, "height") {
            let h = if let Json::Number(n) = &h { format!("{n}px") } else { expr::stringify(&h) };
            root.base_style(StyleBlock::new().set("--tree-height", h));
        }
        cx.open(&root, node)?;

        if title.is_some() || search.is_some() || addable {
            let mut head = Element::new("div");
            head.attr("data-tree-head", true);
            cx.open(&head, node)?;
            if let Some(t) = &title {
                let mut h = Element::new("h3");
                h.attr("data-tree-title", true).text(t.clone());
                cx.open(&h, node)?;
                cx.close("h3");
            }
            if let Some(p) = &search {
                let mut s = Element::new("input");
                s.base_style_ref(forms::input_look())
                    .attr("type", "search")
                    .attr("data-tree-search", true)
                    .attr("placeholder", p.clone())
                    .attr("aria-label", p.trim_end_matches('…').to_string());
                cx.open(&s, node)?;
            }
            if addable {
                let mut b = Element::new("button");
                forms::button_look(&mut b, "outline", "sm");
                b.attr("type", "button").attr("data-tree-add-root", true);
                cx.open(&b, node)?;
                cx.markup(PLUS);
                let mut l = Element::new("span");
                l.text(node.prop("addLabel").cloned().unwrap_or_else(|| lit(cx.label("Add"))));
                cx.open(&l, node)?;
                cx.close("span");
                cx.close("button");
            }
            cx.close("div");
        }
        if checkable && deletable {
            let mut bulk = Element::new("div");
            bulk.attr("data-tree-bulk", true);
            if checked.is_empty() {
                bulk.attr("hidden", true);
            }
            cx.open(&bulk, node)?;
            let mut n = Element::new("span");
            n.attr("data-tree-checked", true).text(lit(cx.label_with("{n} selected", &[("n", &checked.len().to_string())])));
            cx.open(&n, node)?;
            cx.close("span");
            // Labels: "Delete", "Clear"
            for (attr, text, variant) in [("data-tree-delete-checked", "Delete", "destructive"), ("data-tree-uncheck", "Clear", "ghost")] {
                let mut b = Element::new("button");
                forms::button_look(&mut b, variant, "sm");
                b.attr("type", "button").attr(attr, true).text(lit(cx.label(text)));
                cx.open(&b, node)?;
                cx.close("button");
            }
            cx.close("div");
        }

        let mut ul = cx.part("ul", node, "tree");
        ul.base_style_ref(&TREE).base_style(StyleBlock::new().set("max-height", "var(--tree-height, none)")).attr("role", "tree");
        match &title {
            Some(t) => {
                ul.attr("aria-label", t.clone());
            }
            None => {
                ul.attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Tree"))));
            }
        }
        if nodes.is_empty() {
            ul.attr("hidden", true);
        }
        cx.open(&ul, node)?;
        let draw = Draw { checkable, addable, deletable, movable, counts, lazy: children_url.is_some(), selected, checked, open: &open };
        items(cx, node, &nodes, 0, &draw, &mut true)?;
        cx.close("ul");

        // The icon kit's drawing of each icon name the nodes use, for the script to copy
        // when it draws the tree again.
        if cx.has_component("ui-icon") {
            let mut names = Vec::new();
            icon_names(&nodes, &mut names);
            for name in names.into_iter().chain(cx.prop(node, "icons").and_then(|v| v.as_array().cloned()).unwrap_or_default().iter().map(expr::stringify)) {
                let is_name = !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == ':');
                if !is_name {
                    continue;
                }
                let mut t = Element::new("template");
                t.attr("data-tree-icon", name.clone());
                cx.open(&t, node)?;
                crate::icon(cx, node, &lit(name), 16)?;
                cx.close("template");
            }
        }

        let mut empty = Element::new("p");
        empty.attr("data-tree-empty", true).text(node.prop("emptyText").cloned().unwrap_or_else(|| lit(cx.label("Nothing here yet."))));
        if !nodes.is_empty() {
            empty.attr("hidden", true);
        }
        cx.open(&empty, node)?;
        cx.close("p");
        let mut live = Element::new("div");
        live.attr("data-tree-live", true).attr("role", "status").attr("aria-live", "polite");
        cx.open(&live, node)?;
        cx.close("div");
        cx.close("div");
        Ok(())
    }
}
