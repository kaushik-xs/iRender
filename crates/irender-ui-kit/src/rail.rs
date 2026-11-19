//! `ui-rail-layout`: an app shell with a rail of icons down the left, each opening its
//! own panel beside it (`ui-rail-panel` children), and the page's main content after.
//! The section shown is state (`bind`), and so is whether the panel is open
//! (`bindOpen`), so the server draws them as they are and the page can change them.
//! `rail.js` handles clicks (a click on the open section folds its panel) and the keys.

use crate::{ACCENT, MUTED, PRIMARY_FILL, SURFACE, TEXT, bin, cond, expr_val, forms, icon_kit, icon_name, id_with, instance_id, lit, path};
use irender_core::{
    ComponentInfo, Node, Part, PropDef, PropKind, StyleBlock, StyleValue, Template, Val,
    expr::{self, BinOp, Expr, UnOp},
};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("rail.js");

pub struct RailLayout;
pub struct RailPanel;

const PROPS: &[PropDef] = &[
    PropDef::new(
        "items",
        PropKind::Any,
        "The rail's sections: `{ id, label, icon, badge, bottom, href }`. `icon` is an icon kit name or a character; `bottom` puts it at the foot; `href` makes it a link",
    )
    .required(),
    PropDef::new("bind", PropKind::StateKey, "State key for the section shown (an item's `id`)").required(),
    PropDef::new("bindOpen", PropKind::StateKey, "State key for whether the panel is open (true or false): a click on the open section folds it"),
    PropDef::new("label", PropKind::Any, "Names the rail for screen readers (default `Sections`)"),
    PropDef::new("panelWidth", PropKind::Number, "The panel's width in px (default 260)"),
    PropDef::new("resizable", PropKind::Bool, "A handle on the panel's edge to drag it wider or narrower (or focus it and use the arrow keys)"),
    PropDef::new("bindWidth", PropKind::StateKey, "State key for the panel's width in px, written after each resize"),
    PropDef::new("minPanelWidth", PropKind::Number, "The narrowest a resize makes it, in px (default 180)"),
    PropDef::new("maxPanelWidth", PropKind::Number, "The widest a resize makes it, in px (default 560)"),
    PropDef::new("height", PropKind::Any, "The shell's height (default `100svh`)"),
];
const PANEL_PROPS: &[PropDef] = &[
    PropDef::new("for", PropKind::String, "The `id` of the rail item it belongs to").required(),
    PropDef::new("title", PropKind::Any, "A heading at its top"),
];

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

static LAYOUT: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("position", "relative"),
        ("display", "grid"),
        ("grid-template-columns", "56px auto minmax(0, 1fr)"),
        ("height", "var(--rail-height, 100svh)"),
        ("min-height", "0"),
        ("color", TEXT),
        ("background", "var(--color-background, var(--color-surface))"),
    ])
    .nest(
        "& [data-rail]",
        pairs(&[
            ("display", "flex"),
            ("flex-direction", "column"),
            ("align-items", "center"),
            ("gap", sp!(4)),
            ("padding", sp!(10, 0)),
            ("border-inline-end", "1px solid var(--color-border)"),
            ("background", "var(--color-sidebar, color-mix(in srgb, var(--color-text) 3%, var(--color-surface, #fff)))"),
            ("min-height", "0"),
            // Not a scroll box: that would clip the labels shown beside the icons.
            ("position", "relative"),
            ("z-index", "25"),
        ]),
    )
    .nest("& [data-rail-foot]", pairs(&[("margin-top", "auto"), ("display", "flex"), ("flex-direction", "column"), ("align-items", "center"), ("gap", sp!(4))]))
    .nest(
        "& [data-rail-item]",
        pairs(&[
            ("position", "relative"),
            ("display", "grid"),
            ("place-items", "center"),
            ("width", "40px"),
            ("height", "40px"),
            ("padding", "0"),
            ("border", "0"),
            ("border-radius", "var(--radius-md)"),
            ("background", "transparent"),
            ("color", MUTED),
            ("font", "inherit"),
            ("font-size", fs!(15)),
            ("font-weight", "600"),
            ("text-decoration", "none"),
            ("cursor", "pointer"),
            ("transition", "background .15s, color .15s"),
        ]),
    )
    .nest("& [data-rail-item]:hover", pairs(&[("background", ACCENT), ("color", TEXT)]))
    .nest("& [data-rail-item]:focus-visible", forms::focus_ring().set("outline-offset", "-2px"))
    .nest("& [data-rail-item][aria-selected=true]", pairs(&[("background", "color-mix(in srgb, var(--color-text) 9%, transparent)"), ("color", TEXT)]))
    // The open section: a bar on the rail's edge.
    .nest(
        "& [data-rail-item][aria-selected=true]::before",
        pairs(&[("content", "''"), ("position", "absolute"), ("inset-inline-start", "-8px"), ("top", "10px"), ("bottom", "10px"), ("width", "3px"), ("border-start-end-radius", "3px"), ("border-end-end-radius", "3px"), ("background", PRIMARY_FILL)]),
    )
    // Its label beside it on hover and focus; `/ ""` keeps the copy from screen readers.
    .nest(
        "& [data-rail-item]::after",
        pairs(&[
            ("content", "attr(data-tip) / \"\""),
            ("position", "absolute"),
            ("inset-inline-start", "calc(100% + 10px)"),
            ("top", "50%"),
            ("z-index", "30"),
            ("padding", sp!(4, 8)),
            ("border-radius", "var(--radius-sm)"),
            ("background", TEXT),
            ("color", SURFACE),
            ("font-size", fs!(12)),
            ("font-weight", "500"),
            ("white-space", "nowrap"),
            ("transform", "translateY(-50%)"),
            ("opacity", "0"),
            ("pointer-events", "none"),
            ("transition", "opacity .1s"),
        ]),
    )
    .nest("& [data-rail-item]:is(:hover, :focus-visible)::after", pairs(&[("opacity", "1")]))
    .nest(
        "& [data-rail-badge]",
        pairs(&[
            ("position", "absolute"),
            ("top", "3px"),
            ("inset-inline-end", "1px"),
            ("min-width", "16px"),
            ("height", "16px"),
            ("padding", "0 4px"),
            ("box-sizing", "border-box"),
            ("border", "2px solid var(--color-sidebar, var(--color-surface, #fff))"),
            ("border-radius", "999px"),
            ("background", "var(--color-destructive, #dc2626)"),
            ("color", "#fff"),
            ("font-size", "9px"),
            ("font-weight", "700"),
            ("line-height", "12px"),
            ("text-align", "center"),
        ]),
    )
    // The panel: as wide as `--rail-panel`, folded to nothing while closed.
    .nest(
        "& [data-rail-panels]",
        pairs(&[
            ("width", "var(--rail-panel, 260px)"),
            ("min-height", "0"),
            ("overflow", "hidden"),
            ("border-inline-end", "1px solid var(--color-border)"),
            ("background", "var(--color-surface, #fff)"),
            ("transition", "width .2s ease"),
        ]),
    )
    .nest("&[data-closed] [data-rail-panels]", pairs(&[("width", "0"), ("border-inline-end-width", "0")]))
    .nest("&[data-resizing] [data-rail-panels]", pairs(&[("transition", "none")]))
    .nest("&[data-resizing]", pairs(&[("cursor", "col-resize"), ("user-select", "none")]))
    // The resize handle: a strip on the panel's edge, a line when hovered or focused.
    .nest(
        "& [data-rail-resize]",
        pairs(&[("position", "absolute"), ("z-index", "21"), ("top", "0"), ("bottom", "0"), ("inset-inline-start", "calc(56px + var(--rail-panel, 260px) - 3px)"), ("width", "7px"), ("cursor", "col-resize"), ("touch-action", "none"), ("outline", "none")]),
    )
    .nest("& [data-rail-resize]::after", pairs(&[("content", "''"), ("position", "absolute"), ("top", "0"), ("bottom", "0"), ("inset-inline-start", "3px"), ("width", "1px"), ("transition", "background .15s")]))
    .nest("& [data-rail-resize]:is(:hover, :focus-visible)::after, &[data-resizing] [data-rail-resize]::after", pairs(&[("inset-inline-start", "2px"), ("width", "3px"), ("background", "var(--color-ring, var(--color-muted))")]))
    .nest("&[data-closed] [data-rail-resize]", pairs(&[("display", "none")]))
    .nest("@media (prefers-reduced-motion: reduce)", pairs(&[]).nest("& [data-rail-panels]", pairs(&[("transition", "none")])))
    .nest("& [data-rail-main]", pairs(&[("min-width", "0"), ("min-height", "0"), ("overflow", "auto")]))
    // On a phone: the panel over the page, beside the rail.
    .nest(
        "@media (max-width: 720px)",
        pairs(&[("grid-template-columns", "56px minmax(0, 1fr)")])
            .nest(
                "& [data-rail-panels]",
                pairs(&[("position", "absolute"), ("z-index", "20"), ("top", "0"), ("bottom", "0"), ("inset-inline-start", "56px"), ("box-shadow", "8px 0 24px rgb(0 0 0 / 0.12)")]),
            )
            .nest("&[data-closed] [data-rail-panels]", pairs(&[("box-shadow", "none")])),
    )
});
static PANEL: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", sp!(8)), ("box-sizing", "border-box"), ("width", "var(--rail-panel, 260px)"), ("height", "100%"), ("padding", sp!(14, 12)), ("overflow-y", "auto")])
        .nest("&[hidden]", pairs(&[("display", "none")]))
        .nest("& [data-rail-head]", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", sp!(6)), ("padding", sp!(0, 4, 4))]))
        .nest("& [data-rail-head] h2", pairs(&[("margin-block", "0 0"), ("margin-inline", "0 auto"), ("font-size", fs!(15)), ("font-weight", "600")]))
        .nest(
            "& [data-rail-fold]",
            pairs(&[("display", "grid"), ("place-items", "center"), ("width", "28px"), ("height", "28px"), ("padding", "0"), ("border", "0"), ("border-radius", "var(--radius-sm)"), ("background", "transparent"), ("color", MUTED), ("cursor", "pointer")]),
        )
        .nest("& [data-rail-fold]:hover", pairs(&[("background", ACCENT), ("color", TEXT)]))
        .nest("& [data-rail-fold]:focus-visible", forms::focus_ring())
});

const FOLD: &str = r#"<svg data-flip width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="2" y="2.5" width="12" height="11" rx="2"/><path d="M6 2.5v11M10.5 6.5 9 8l1.5 1.5"/></svg>"#;

/// One rail item, read from `items`.
struct Item {
    id: String,
    label: String,
    icon: Option<Json>,
    badge: Option<Json>,
    bottom: bool,
    href: Option<String>,
}

fn items(cx: &RenderCx<'_>, node: &Node) -> Result<Vec<Item>, RenderError> {
    let Some(Json::Array(list)) = cx.prop(node, "items") else {
        return Err(RenderError::new(node, "`items` must be a list of `{ id, label, icon }`"));
    };
    let mut out: Vec<Item> = Vec::new();
    for it in &list {
        let id = it.get("id").map(expr::stringify).filter(|s| !s.is_empty()).ok_or_else(|| RenderError::new(node, "each rail item needs an `id`"))?;
        if out.iter().any(|x| x.id == id) {
            return Err(RenderError::new(node, format!("two rail items have the id `{id}`")));
        }
        let label = it.get("label").map(expr::stringify).filter(|s| !s.is_empty()).ok_or_else(|| RenderError::new(node, format!("rail item `{id}` needs a `label`: it's the item's name and tooltip")))?;
        out.push(Item {
            label,
            icon: it.get("icon").filter(|v| !v.is_null()).cloned(),
            badge: it.get("badge").filter(|v| !v.is_null() && **v != json!(0) && **v != json!("")).cloned(),
            bottom: it.get("bottom").is_some_and(expr::truthy),
            href: it.get("href").filter(|v| !v.is_null()).map(expr::stringify),
            id,
        });
    }
    Ok(out)
}

impl Component for RailLayout {
    fn name(&self) -> &'static str {
        "ui-rail-layout"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "An app shell: a rail of icons, each opening its own panel beside it (`ui-rail-panel` children), and the main \
                  content after. The section shown is in `bind`, the panel's open state in `bindOpen`.",
            props: PROPS,
            parts: &["rail", "panels", "main"],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        crate::flips(cx);
        let key = cx.prop_string(node, "bind").ok_or_else(|| RenderError::new(node, "`ui-rail-layout` needs `bind`"))?;
        let open_key = cx.prop_string(node, "bindOpen");
        let items = items(cx, node)?;
        let panels: Vec<&Node> = node.children.iter().filter(|c| c.kind == "ui-rail-panel").collect();
        for p in &panels {
            let f = cx.prop_string(p, "for").unwrap_or_default();
            if !items.iter().any(|i| i.id == f) {
                return Err(RenderError::new(p, format!("`ui-rail-panel` is for `{f}`, which isn't a rail item")));
            }
        }
        let shown = expr::Expr::Path(vec![key.clone()]);
        let open = open_key.as_ref().map(|k| path(&[k]));
        let id = instance_id(cx, node, "rail");
        let width = cx.prop(node, "panelWidth").map_or(260.0, |v| expr::to_num(&v));
        let resizable = cx.prop(node, "resizable").is_some_and(|v| expr::truthy(&v));
        let width_key = cx.prop_string(node, "bindWidth");
        let (min, max) = (cx.prop(node, "minPanelWidth").map_or(180.0, |v| expr::to_num(&v)), cx.prop(node, "maxPanelWidth").map_or(560.0, |v| expr::to_num(&v)));
        if resizable && !(min > 0.0 && min <= width && width <= max) {
            return Err(RenderError::new(node, "a resizable panel needs `minPanelWidth` ≤ `panelWidth` ≤ `maxPanelWidth`"));
        }

        cx.global_js("ui-rail-layout", SCRIPT);
        let mut root = cx.element("div", node);
        let mut vars = StyleBlock::new();
        match &width_key {
            // The width in state, when it's a number there.
            Some(k) => {
                let w = Expr::Path(vec![k.clone()]);
                let px = bin(BinOp::Add, w.clone(), Expr::Lit(json!("px")));
                let ok = bin(BinOp::Gt, w, Expr::Lit(json!(0)));
                vars.push("--rail-panel", StyleValue::Dynamic { tpl: Template { parts: vec![Part::Expr(cond(ok, px, Expr::Lit(json!(format!("{width}px")))))] }, px: false });
            }
            None => vars = vars.set("--rail-panel", format!("{width}px")),
        }
        if let Some(h) = cx.prop(node, "height") {
            vars = vars.set("--rail-height", if let Json::Number(n) = &h { format!("{n}px") } else { expr::stringify(&h) });
        }
        root.base_style_ref(&LAYOUT).base_style(vars).behavior(
            "ui-rail-layout",
            json!({ "bind": key, "bindOpen": open_key, "panels": panels.iter().map(|p| cx.prop_string(p, "for")).collect::<Vec<_>>(),
                "width": width, "bindWidth": width_key, "min": min, "max": max }),
        );
        // Closed: no panel for the section, or `bindOpen` false.
        let has_panel = panels.iter().fold(Expr::Lit(json!(false)), |acc, p| {
            bin(BinOp::Or, acc, bin(BinOp::Eq, shown.clone(), Expr::Lit(json!(cx.prop_string(p, "for").unwrap_or_default()))))
        });
        let closed = match &open {
            Some(o) => bin(BinOp::Or, Expr::Unary(UnOp::Not, Box::new(has_panel)), Expr::Unary(UnOp::Not, Box::new(o.clone()))),
            None => Expr::Unary(UnOp::Not, Box::new(has_panel)),
        };
        root.attr("data-closed", expr_val(closed));
        cx.open(&root, node)?;

        // The rail: a vertical tab list of sections (links for items with `href`).
        let mut rail = cx.part("nav", node, "rail");
        rail.attr("data-rail", true).attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Sections"))));
        cx.open(&rail, node)?;
        let mut list = Element::new("div");
        list.attr("role", "tablist").attr("aria-orientation", "vertical").attr("data-rail-tabs", true).base_style(pairs(&[("display", "contents")]));
        cx.open(&list, node)?;
        let (top, foot): (Vec<&Item>, Vec<&Item>) = items.iter().partition(|i| !i.bottom);
        for it in &top {
            item(cx, node, it, &shown, &id, panels.iter().any(|p| cx.prop_string(p, "for").as_deref() == Some(&it.id)))?;
        }
        if !foot.is_empty() {
            let mut f = Element::new("div");
            f.attr("data-rail-foot", true);
            cx.open(&f, node)?;
            for it in &foot {
                item(cx, node, it, &shown, &id, panels.iter().any(|p| cx.prop_string(p, "for").as_deref() == Some(&it.id)))?;
            }
            cx.close("div");
        }
        cx.close("div");
        cx.close("nav");

        let mut wrap = cx.part("div", node, "panels");
        wrap.attr("data-rail-panels", true).attr("id", id_with(&id, "-panels"));
        cx.open(&wrap, node)?;
        for p in &panels {
            let f = cx.prop_string(p, "for").unwrap_or_default();
            let label = items.iter().find(|i| i.id == f).map(|i| i.label.clone()).unwrap_or_default();
            let mut el = cx.element("section", p);
            el.base_style_ref(&PANEL)
                .attr("id", id_with(&id, &format!("-panel-{f}")))
                .attr("role", "tabpanel")
                .attr("aria-labelledby", id_with(&id, &format!("-tab-{f}")))
                .attr("data-rail-panel", f.clone())
                .attr("hidden", expr_val(bin(BinOp::Ne, shown.clone(), Expr::Lit(json!(f)))));
            cx.open(&el, p)?;
            let mut head = Element::new("div");
            head.attr("data-rail-head", true);
            cx.open(&head, p)?;
            let mut h = Element::new("h2");
            h.text(p.prop("title").cloned().unwrap_or_else(|| lit(label.clone())));
            cx.open(&h, p)?;
            cx.close("h2");
            if open_key.is_some() {
                let mut b = Element::new("button");
                b.attr("type", "button").attr("data-rail-fold", true).attr("aria-label", cx.label_with("Hide {name}", &[("name", &label)])).attr("title", cx.label("Hide panel"));
                cx.open(&b, p)?;
                cx.markup(FOLD);
                cx.close("button");
            }
            cx.close("div");
            cx.children(p)?;
            cx.close("section");
        }
        cx.close("div");
        if resizable {
            // Beside the panels, not in them: they clip what overflows.
            let mut h = Element::new("div");
            h.attr("data-rail-resize", true)
                .attr("role", "separator")
                .attr("tabindex", "0")
                .attr("aria-orientation", "vertical")
                .attr("aria-label", cx.label("Resize the panel"))
                .attr("aria-controls", id_with(&id, "-panels"))
                .attr("aria-valuemin", lit(min))
                .attr("aria-valuemax", lit(max))
                .attr("aria-valuenow", lit(width));
            cx.open(&h, node)?;
            cx.close("div");
        }

        let mut main = cx.part("main", node, "main");
        main.attr("data-rail-main", true);
        cx.open(&main, node)?;
        for c in node.children.iter().filter(|c| c.kind != "ui-rail-panel") {
            cx.render(c)?;
        }
        cx.close("main");
        cx.close("div");
        Ok(())
    }
}

fn item(cx: &mut RenderCx<'_>, node: &Node, it: &Item, shown: &Expr, id: &Val, has_panel: bool) -> Result<(), RenderError> {
    let selected = bin(BinOp::Eq, shown.clone(), Expr::Lit(json!(it.id)));
    let mut el = Element::new(if it.href.is_some() { "a" } else { "button" });
    el.attr("data-rail-item", it.id.clone()).attr("data-tip", it.label.clone()).attr("aria-label", it.label.clone());
    if let Some(h) = &it.href {
        el.attr("href", h.clone());
    } else {
        el.attr("type", "button")
            .attr("role", "tab")
            .attr("id", id_with(id, &format!("-tab-{}", it.id)))
            .attr("aria-selected", expr_val(cond(selected.clone(), Expr::Lit(json!("true")), Expr::Lit(json!("false")))))
            .attr("tabindex", expr_val(cond(selected, Expr::Lit(json!(0)), Expr::Lit(json!(-1)))));
        if has_panel {
            el.attr("aria-controls", id_with(id, &format!("-panel-{}", it.id)));
        }
    }
    cx.open(&el, node)?;
    match it.icon.as_ref().map(|i| lit(i.clone())).as_ref().and_then(|v| icon_name(cx, v)) {
        Some(name) => icon_kit(cx, node, &name, 18)?,
        None => {
            // A character given as the icon, or the label's first letter.
            let text = it.icon.as_ref().map(expr::stringify).unwrap_or_else(|| it.label.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default());
            let mut s = Element::new("span");
            s.attr("aria-hidden", "true").text(lit(text));
            cx.open(&s, node)?;
            cx.close("span");
        }
    }
    if let Some(b) = &it.badge {
        let mut s = Element::new("span");
        s.attr("data-rail-badge", true).text(lit(expr::stringify(b)));
        cx.open(&s, node)?;
        cx.close("span");
    }
    cx.close(if it.href.is_some() { "a" } else { "button" });
    Ok(())
}

impl Component for RailPanel {
    fn name(&self) -> &'static str {
        "ui-rail-panel"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "The panel a `ui-rail-layout` item opens: its children, under a heading. Read by the layout, never rendered on its own.",
            props: PANEL_PROPS,
            parts: &[],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, _cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        Err(RenderError::new(node, "`ui-rail-panel` must be a direct child of `ui-rail-layout`"))
    }
}
