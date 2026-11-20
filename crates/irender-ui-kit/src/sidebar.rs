//! The sidebar: a page layout with a collapsible navigation panel, in the style of
//! shadcn/ui's. `ui-sidebar-layout` holds the page and its open state; its first
//! `ui-sidebar` child is the panel and the other children are the main area.
//! Collapsing is CSS on the layout's `data-state`, which follows `bind`. On a phone the
//! panel is a sheet over the page, and ⌘B / Ctrl+B toggles it (sidebar.js).

use crate::{BG_PAGE, ACCENT, MUTED, RING, SURFACE, TEXT, choice, expr_val, icon, lit, state};
use irender_core::{
    Action, ComponentInfo, Node, PropDef, PropKind, StyleBlock, Val,
    expr::{Expr, UnOp},
};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

/// The panel's background: a step off the page's, in light and dark.
pub(crate) const SIDEBAR_BG: &str = "var(--color-sidebar, color-mix(in srgb, var(--color-text) 3%, var(--color-surface)))";
const COLLAPSIBLE: &[&str] = &["offcanvas", "icon", "none"];
const VARIANTS: &[&str] = &["sidebar", "inset", "floating"];
const SIDES: &[&str] = &["left", "right"];

/// While collapsed to icons.
const ICONS: &str = "[data-collapsible=icon][data-state=collapsed]";
/// Open and collapsed apply on wide screens. On a phone the panel is a sheet over the
/// page instead, opened by the trigger (sidebar.js) whatever `bind` says.
const WIDE: &str = "@media (min-width: 768px)";
const PHONE: &str = "@media (max-width: 767px)";
/// A layout whose sidebar can close, so it becomes a sheet on a phone.
const CLOSES: &str = "[data-collapsible]:not([data-collapsible=none])";

const SCRIPT: &str = include_str!("sidebar.js");

/// `block` under `key`, for nesting in a media query.
fn wide(key: impl Into<String>, block: StyleBlock) -> StyleBlock {
    StyleBlock::new().nest(key, block)
}

fn info(doc: &'static str, props: &'static [PropDef], parts: &'static [&'static str]) -> ComponentInfo {
    ComponentInfo { doc, props, parts, children: true, open_props: false }
}

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

/// A container with the node's children and a fixed style.
fn container(cx: &mut RenderCx<'_>, node: &Node, tag: &str, base: &'static StyleBlock, marker: &str) -> Result<(), RenderError> {
    let mut el = cx.element(tag, node);
    el.base_style_ref(base).attr(marker, true);
    cx.open(&el, node)?;
    cx.children(node)?;
    cx.close(tag);
    Ok(())
}

// ---------------------------------------------------------------- layout

pub struct SidebarLayout;

const LAYOUT_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key holding whether the sidebar is open (`true` / `false`)"),
    PropDef::new("collapsible", PropKind::Enum(COLLAPSIBLE), "How it closes: `offcanvas` (slides away, default), `icon` (narrows to icons) or `none`"),
    PropDef::new("variant", PropKind::Enum(VARIANTS), "`sidebar` (default), `inset` (the main area is a raised panel) or `floating`"),
    PropDef::new("side", PropKind::Enum(SIDES), "Default `left`"),
    PropDef::new("width", PropKind::Any, "The open sidebar's width (default 16rem)"),
    PropDef::new(
        "remember",
        PropKind::Any,
        "Keep open or closed across reloads, in the browser's storage: `true` (under `bind`'s name) or a name shared by the pages that should agree",
    ),
];

static LAYOUT: LazyLock<StyleBlock> = LazyLock::new(|| {
    // Native controls on the page (checkboxes, scrollbars) follow the theme's scheme.
    crate::forms::native_scheme(pairs(&[
        ("--sidebar-width", "16rem"),
        ("--sidebar-width-icon", "3rem"),
        ("display", "flex"),
        ("min-height", "100svh"),
        ("width", "100%"),
        ("--ui-bg", BG_PAGE),
        ("background", BG_PAGE),
        ("color", TEXT),
        ("font-family", "var(--font-body, system-ui, sans-serif)"),
    ]))
    .nest("&[data-side=right]", pairs(&[("flex-direction", "row-reverse")]))
    .nest("&[data-variant=inset]", pairs(&[("background", SIDEBAR_BG)]))
    // Restored from storage as the page loads: no sliding.
    .nest("&[data-restoring], &[data-restoring] *", pairs(&[("transition", "none !important")]))
    // The backdrop behind the sheet on a phone; a click on it closes the sheet.
    .nest(
        "&[data-mobile-open]::before",
        pairs(&[("content", "\"\""), ("position", "fixed"), ("inset", "0"), ("z-index", "40"), ("background", "rgb(0 0 0 / 0.5)")]),
    )
});
static MAIN: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("position", "relative"), ("display", "flex"), ("flex-direction", "column"), ("flex", "1"), ("min-width", "0"), ("--ui-bg", BG_PAGE), ("background", BG_PAGE)])
        // Inset: the main area is a raised panel next to the sidebar.
        .nest(
            "[data-variant=inset] > &",
            pairs(&[("margin", sp!(8)), ("border-radius", "var(--radius-lg)"), ("box-shadow", "0 1px 3px rgb(0 0 0 / 0.08)")]),
        )
        .nest(WIDE, wide("[data-variant=inset][data-state=expanded][data-side=left] > &", pairs(&[("margin-inline-start", "0")])))
        .nest(WIDE, wide("[data-variant=inset][data-state=expanded][data-side=right] > &", pairs(&[("margin-inline-end", "0")])))
});

impl Component for SidebarLayout {
    fn name(&self) -> &'static str {
        "ui-sidebar-layout"
    }
    fn info(&self) -> ComponentInfo {
        info(
            "A page with a sidebar. Children: a `ui-sidebar`, then the main area's content. With `bind`, a \
             `ui-sidebar-trigger` opens and closes it.",
            LAYOUT_PROPS,
            &["main"],
        )
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        crate::flips(cx);
        let key = cx.prop_string(node, "bind");
        let collapsible = COLLAPSIBLE[choice(cx, node, "collapsible", COLLAPSIBLE)];
        let open = match &key {
            Some(k) if collapsible != "none" => {
                expr_val(Expr::Cond(Box::new(state(k)), Box::new(Expr::Lit(json!("expanded"))), Box::new(Expr::Lit(json!("collapsed")))))
            }
            _ => lit("expanded"),
        };
        let mut el = cx.element("div", node);
        el.base_style_ref(&LAYOUT)
            .attr("data-sidebar-layout", true)
            .attr("data-state", open)
            .attr("data-collapsible", collapsible)
            .attr("data-variant", VARIANTS[choice(cx, node, "variant", VARIANTS)])
            .attr("data-side", SIDES[choice(cx, node, "side", SIDES)]);
        if let Some(k) = key.as_ref().filter(|_| collapsible != "none") {
            cx.global_js("ui-sidebar", SCRIPT);
            let remember = match cx.prop(node, "remember") {
                None | Some(Json::Null) | Some(Json::Bool(false)) => None,
                Some(Json::Bool(true)) => Some(k.clone()),
                Some(v) => Some(irender_core::expr::stringify(&v)).filter(|n| !n.is_empty()),
            };
            let mut config = json!({ "key": k });
            if let Some(r) = remember {
                config["remember"] = json!(r);
            }
            el.behavior("ui-sidebar", config);
        }
        if let Some(w) = node.prop("width") {
            let mut width = StyleBlock::new();
            if let Some(v) = cx.style_value(node, "--sidebar-width", w)? {
                width.push("--sidebar-width", v);
            }
            el.base_style(width);
        }
        cx.open(&el, node)?;
        let (panels, rest): (Vec<&Node>, Vec<&Node>) = node.children.iter().partition(|c| c.kind == "ui-sidebar");
        cx.with_context("ui-sidebar", key.map(lit), |cx| {
            for p in panels {
                cx.render(p)?;
            }
            let mut main = cx.part("main", node, "main");
            main.base_style_ref(&MAIN);
            cx.open(&main, node)?;
            for c in rest {
                cx.render(c)?;
            }
            cx.close("main");
            Ok(())
        })?;
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- panel

pub struct Sidebar;

const PANEL_PROPS: &[PropDef] = &[PropDef::new("label", PropKind::Any, "Names the navigation for screen readers (default `Sidebar`)")];

static PANEL: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("position", "sticky"),
        ("top", "0"),
        ("align-self", "flex-start"),
        ("flex-shrink", "0"),
        ("box-sizing", "border-box"),
        ("width", "var(--sidebar-width)"),
        ("height", "100svh"),
        ("overflow", "hidden"),
        ("--ui-bg", SIDEBAR_BG),
        ("background", SIDEBAR_BG),
        ("color", TEXT),
        ("font-size", fs!(14)),
        ("transition", "width .2s ease-out"),
    ])
    .nest("[data-side=left]:not([data-variant=inset]):not([data-variant=floating]) > &", pairs(&[("border-inline-end", "1px solid var(--color-border)")]))
    .nest("[data-side=right]:not([data-variant=inset]):not([data-variant=floating]) > &", pairs(&[("border-inline-start", "1px solid var(--color-border)")]))
    .nest("[data-variant=floating] > &", pairs(&[("padding", sp!(8)), ("background", "transparent")]))
    // As specific as the border rules above, and after them, so it takes the border too.
    .nest(WIDE, wide("[data-collapsible=offcanvas][data-state=collapsed][data-side] > &", pairs(&[("width", "0"), ("border", "0"), ("padding", "0")])))
    .nest(WIDE, wide(format!("{ICONS} > &"), pairs(&[("width", "var(--sidebar-width-icon)"), ("overflow", "visible")])))
    .nest(WIDE, wide("[data-variant=floating][data-collapsible=icon][data-state=collapsed] > &", pairs(&[("width", "calc(var(--sidebar-width-icon) + 16px)")])))
    .nest(
        PHONE,
        wide(
            format!("{CLOSES} > &"),
            pairs(&[
                ("position", "fixed"),
                ("top", "0"),
                ("bottom", "0"),
                ("inset-inline-start", "0"),
                ("z-index", "50"),
                ("width", "min(var(--sidebar-width), 85vw)"),
                // Stretched between top and bottom: the sticky panel's `align-self:
                // flex-start` would shrink a fixed box to its content.
                ("align-self", "stretch"),
                ("height", "auto"),
                ("border-inline-end", "1px solid var(--color-border)"),
                ("background", SIDEBAR_BG),
                ("box-shadow", "0 10px 30px rgb(0 0 0 / 0.2)"),
                ("transform", "translateX(calc(-100% * var(--ui-dir, 1)))"),
                ("visibility", "hidden"),
                ("transition", "transform .2s ease-out, visibility .2s"),
            ]),
        )
        .nest(format!("[data-side=right]{CLOSES} > &"), pairs(&[("inset-inline-start", "auto"), ("inset-inline-end", "0"), ("border-inline-end", "0"), ("border-inline-start", "1px solid var(--color-border)"), ("transform", "translateX(calc(100% * var(--ui-dir, 1)))")]))
        .nest(format!("[data-mobile-open]{CLOSES} > &"), pairs(&[("transform", "none"), ("visibility", "visible")])),
    )
    .nest("@media (prefers-reduced-motion: reduce)", pairs(&[("transition", "none")]))
});
static INNER: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("display", "flex"), ("flex-direction", "column"), ("box-sizing", "border-box"), ("width", "100%"), ("height", "100%")])
        // Off-canvas, the panel keeps its width while it slides shut, so nothing reflows.
        .nest("[data-collapsible=offcanvas] > * > &", pairs(&[("width", "var(--sidebar-width)")]))
        .nest(
            "[data-variant=floating] > * > &",
            pairs(&[
                ("border", "1px solid var(--color-border)"),
                ("border-radius", "var(--radius-lg)"),
                ("background", SIDEBAR_BG),
                ("box-shadow", "0 1px 3px rgb(0 0 0 / 0.08)"),
            ]),
        )
        .nest("[data-variant=floating][data-collapsible=offcanvas] > * > &", pairs(&[("width", "calc(var(--sidebar-width) - 16px)")]))
});

impl Component for Sidebar {
    fn name(&self) -> &'static str {
        "ui-sidebar"
    }
    fn info(&self) -> ComponentInfo {
        info(
            "The sidebar's panel, inside a `ui-sidebar-layout`. Children: `ui-sidebar-header`, `ui-sidebar-content` \
             and `ui-sidebar-footer`.",
            PANEL_PROPS,
            &["inner"],
        )
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        crate::flips(cx);
        let mut el = cx.element("aside", node);
        el.base_style_ref(&PANEL).attr("data-sidebar", true);
        el.attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Sidebar"))));
        cx.open(&el, node)?;
        let mut inner = cx.part("div", node, "inner");
        inner.base_style_ref(&INNER);
        cx.open(&inner, node)?;
        cx.children(node)?;
        cx.close("div");
        cx.close("aside");
        Ok(())
    }
}

pub struct SidebarHeader;
pub struct SidebarFooter;
pub struct SidebarContent;

static HEADER: LazyLock<StyleBlock> =
    LazyLock::new(|| pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", sp!(8)), ("padding", sp!(8))]));
static CONTENT: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("display", "flex"),
        ("flex-direction", "column"),
        ("flex", "1"),
        ("min-height", "0"),
        ("gap", sp!(8)),
        ("overflow-y", "auto"),
        ("overflow-x", "hidden"),
        ("scrollbar-width", "thin"),
    ])
    .nest(WIDE, wide(format!("{ICONS} &"), pairs(&[("overflow", "visible")])))
});

impl Component for SidebarHeader {
    fn name(&self) -> &'static str {
        "ui-sidebar-header"
    }
    fn info(&self) -> ComponentInfo {
        info("The top of a `ui-sidebar`, e.g. a logo or a team switcher.", &[], &[])
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        crate::flips(cx);
        container(cx, node, "div", &HEADER, "data-sidebar-header")
    }
}

impl Component for SidebarFooter {
    fn name(&self) -> &'static str {
        "ui-sidebar-footer"
    }
    fn info(&self) -> ComponentInfo {
        info("The bottom of a `ui-sidebar`, e.g. the signed-in user.", &[], &[])
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        crate::flips(cx);
        container(cx, node, "div", &HEADER, "data-sidebar-footer")
    }
}

impl Component for SidebarContent {
    fn name(&self) -> &'static str {
        "ui-sidebar-content"
    }
    fn info(&self) -> ComponentInfo {
        info("The scrolling middle of a `ui-sidebar`. Children: `ui-sidebar-group`s.", &[], &[])
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        crate::flips(cx);
        container(cx, node, "div", &CONTENT, "data-sidebar-content")
    }
}

// ---------------------------------------------------------------- group

pub struct SidebarGroup;

const GROUP_PROPS: &[PropDef] = &[PropDef::new("label", PropKind::Any, "The group's heading, e.g. `Platform`")];

static GROUP: LazyLock<StyleBlock> =
    LazyLock::new(|| pairs(&[("display", "flex"), ("flex-direction", "column"), ("min-width", "0"), ("padding", sp!(8))]));
static GROUP_LABEL: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("display", "flex"),
        ("align-items", "center"),
        ("height", sp!(32)),
        ("padding", sp!(0, 8)),
        ("font-size", fs!(12)),
        ("font-weight", "500"),
        ("color", "color-mix(in srgb, var(--color-text) 70%, transparent)"),
        ("white-space", "nowrap"),
        ("transition", "margin .2s, opacity .2s"),
    ])
    .nest(WIDE, wide(format!("{ICONS} &"), pairs(&[("margin-top", sp!(-32)), ("opacity", "0")])))
});
static MENU: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", sp!(4)), ("min-width", "0"), ("margin", "0"), ("padding", "0"), ("list-style", "none")])
});

impl Component for SidebarGroup {
    fn name(&self) -> &'static str {
        "ui-sidebar-group"
    }
    fn info(&self) -> ComponentInfo {
        info(
            "A labelled list in a sidebar. Children: `ui-sidebar-item`s.",
            GROUP_PROPS,
            &["label", "list"],
        )
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        crate::flips(cx);
        let mut el = cx.element("div", node);
        el.base_style_ref(&GROUP).attr("data-sidebar-group", true);
        cx.open(&el, node)?;
        if let Some(label) = node.prop("label") {
            let mut l = cx.part("div", node, "label");
            l.base_style_ref(&GROUP_LABEL).text(label);
            cx.open(&l, node)?;
            cx.close("div");
        }
        let mut ul = cx.part("ul", node, "list");
        ul.base_style_ref(&MENU);
        cx.open(&ul, node)?;
        cx.with_context("ui-sidebar-sub", None, |cx| cx.children(node))?;
        cx.close("ul");
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- item

pub struct SidebarItem;

const ITEM_PROPS: &[PropDef] = &[
    PropDef::new("label", PropKind::Any, "The item's text").required(),
    PropDef::new("icon", PropKind::Any, "An icon name (with the icon kit) or a character, shown before the label"),
    PropDef::new("href", PropKind::Any, "Make it a link"),
    PropDef::new("active", PropKind::Any, "Marks the current page (boolean or expression)"),
    PropDef::new("badge", PropKind::Any, "A count or short text on the right"),
    PropDef::new("open", PropKind::Bool, "With children: start with its sub-menu open"),
];

/// The visually hidden style for text that screen readers still read.
const HIDDEN_TEXT: &[(&str, &str)] = &[
    ("position", "absolute"),
    ("width", "1px"),
    ("height", "1px"),
    ("overflow", "hidden"),
    ("clip-path", "inset(50%)"),
    ("white-space", "nowrap"),
];

static ITEM: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("display", "flex"),
        ("align-items", "center"),
        ("gap", sp!(8)),
        ("box-sizing", "border-box"),
        ("width", "100%"),
        ("min-height", sp!(32)),
        ("padding", sp!(6, 8)),
        ("border", "0"),
        ("border-radius", "var(--radius-md)"),
        ("background", "transparent"),
        ("color", "inherit"),
        ("font", "inherit"),
        ("text-align", "start"),
        ("text-decoration", "none"),
        ("white-space", "nowrap"),
        ("position", "relative"),
        ("cursor", "pointer"),
        ("list-style", "none"),
        ("transition", "width .2s, padding .2s"),
    ])
    .nest("&::-webkit-details-marker", pairs(&[("display", "none")]))
    .nest(":hover", pairs(&[("background", ACCENT)]))
    .nest(":focus-visible", pairs(&[("outline", "2px solid"), ("outline-color", RING), ("outline-offset", "-2px")]))
    .nest("&[aria-current=page]", pairs(&[("background", ACCENT), ("font-weight", "500")]))
    .nest("& [data-icon]", pairs(&[("display", "inline-flex"), ("width", "16px"), ("justify-content", "center"), ("flex-shrink", "0")]))
    // Collapsed to icons: a square button with the icon alone; the label stays for
    // screen readers.
    .nest(WIDE, wide(format!("{ICONS} &"), pairs(&[("width", "32px"), ("padding", sp!(8))])))
    // Its label beside it on hover or focus. `/ ""` keeps the copy out of what screen
    // readers hear: they read the hidden label already.
    .nest(WIDE, wide(format!("{ICONS} &::after"), pairs(&[
        ("content", "attr(data-tip) / \"\""),
        ("position", "absolute"),
        ("inset-inline-start", "calc(100% + 8px)"),
        ("top", "50%"),
        ("z-index", "20"),
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
    ])))
    .nest(WIDE, wide(format!("[data-side=right]{ICONS} &::after"), pairs(&[("inset-inline-start", "auto"), ("inset-inline-end", "calc(100% + 8px)")])))
    .nest(WIDE, wide(format!("{ICONS} &:hover::after, {ICONS} &:focus-visible::after"), pairs(&[("opacity", "1")])))
    .nest(WIDE, wide(format!("{ICONS} & > [data-label]"), pairs(HIDDEN_TEXT)))
    .nest(WIDE, wide(format!("{ICONS} & > [data-badge], {ICONS} & > [data-chevron]"), pairs(&[("display", "none")])))
});
static LABEL: LazyLock<StyleBlock> =
    LazyLock::new(|| pairs(&[("flex", "1"), ("min-width", "0"), ("overflow", "hidden"), ("text-overflow", "ellipsis")]));
static BADGE: LazyLock<StyleBlock> =
    LazyLock::new(|| pairs(&[("margin-inline-start", "auto"), ("font-size", fs!(12)), ("font-variant-numeric", "tabular-nums"), ("color", MUTED)]));
static CHEVRON: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("display", "inline-flex"), ("margin-inline-start", "auto"), ("color", MUTED), ("transition", "transform .2s")])
        .nest("details[open] > summary > &", pairs(&[("transform", "rotate(90deg)")]))
        // Its icon is mirrored right to left (pointing left): it turns the other way to point down.
        .nest("details[open] > summary > &:dir(rtl)", pairs(&[("transform", "rotate(-90deg)")]))
});
static SUB: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("display", "flex"),
        ("flex-direction", "column"),
        ("gap", sp!(4)),
        ("margin-block", sp!(4, 0)), ("margin-inline", sp!(14, 0)),
        ("padding-block", sp!(2, 2)), ("padding-inline", sp!(10, 0)),
        ("border-inline-start", "1px solid var(--color-border)"),
        ("list-style", "none"),
    ])
    .nest(WIDE, wide(format!("{ICONS} &"), pairs(&[("display", "none")])))
});
static SUB_ITEM: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("min-height", sp!(28)), ("padding", sp!(4, 8)), ("color", "color-mix(in srgb, var(--color-text) 85%, transparent)")])
});

const CHEVRON_SVG: &str = r#"<svg data-flip width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M6 3.5 10.5 8 6 12.5"/></svg>"#;

impl Component for SidebarItem {
    fn name(&self) -> &'static str {
        "ui-sidebar-item"
    }
    fn info(&self) -> ComponentInfo {
        info(
            "An entry in a sidebar group: a link, or a button that runs its `on.click`. Children are a sub-menu of \
             more `ui-sidebar-item`s, opened and closed by the item.",
            ITEM_PROPS,
            &["label", "list"],
        )
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        crate::flips(cx);
        let sub = cx.context("ui-sidebar-sub").is_some();
        let has_children = !node.children.is_empty();
        cx.open(&Element::new("li"), node)?;
        let tag = if has_children {
            let mut details = Element::new("details");
            if cx.prop(node, "open").is_some_and(|o| irender_core::expr::truthy(&o)) {
                details.attr("open", true);
            }
            cx.open(&details, node)?;
            "summary"
        } else if node.prop("href").is_some() {
            "a"
        } else {
            "button"
        };
        let mut el = cx.element(tag, node);
        el.base_style_ref(&ITEM).attr("data-sidebar-item", true);
        if !sub && let Some(l) = node.prop("label") {
            el.attr("data-tip", l.clone());
        }
        if sub {
            el.base_style_ref(&SUB_ITEM);
        }
        match tag {
            "a" => {
                el.attr("href", node.prop("href").cloned().unwrap_or_else(|| lit("")));
            }
            "button" => {
                el.attr("type", "button");
            }
            _ => {}
        }
        if let Some(a) = node.prop("active") {
            let current = Expr::Cond(
                Box::new(Expr::Unary(UnOp::Not, Box::new(Expr::Unary(UnOp::Not, Box::new(crate::val_expr(a)))))),
                Box::new(Expr::Lit(json!("page"))),
                Box::new(Expr::Lit(serde_json::Value::Null)),
            );
            el.attr("aria-current", expr_val(current));
        }
        cx.open(&el, node)?;
        if let Some(i) = node.prop("icon") {
            icon(cx, node, i, 16)?;
        }
        let mut label = cx.part("span", node, "label");
        label.base_style_ref(&LABEL).attr("data-label", true);
        if let Some(l) = node.prop("label") {
            label.text(l);
        }
        cx.open(&label, node)?;
        cx.close("span");
        if let Some(b) = node.prop("badge") {
            let mut badge = Element::new("span");
            badge.base_style_ref(&BADGE).attr("data-badge", true).text(b);
            cx.open(&badge, node)?;
            cx.close("span");
        }
        if has_children {
            let mut chevron = Element::new("span");
            chevron.base_style_ref(&CHEVRON).attr("data-chevron", true);
            cx.open(&chevron, node)?;
            cx.markup(CHEVRON_SVG);
            cx.close("span");
        }
        cx.close(tag);
        if has_children {
            let mut ul = cx.part("ul", node, "list");
            ul.base_style_ref(&SUB);
            cx.open(&ul, node)?;
            cx.with_context("ui-sidebar-sub", Some(lit(true)), |cx| cx.children(node))?;
            cx.close("ul");
            cx.close("details");
        }
        cx.close("li");
        Ok(())
    }
}

// ---------------------------------------------------------------- trigger

pub struct SidebarTrigger;

const TRIGGER_PROPS: &[PropDef] =
    &[PropDef::new("label", PropKind::Any, "Its name for screen readers (default `Toggle sidebar`)")];

static TRIGGER: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("display", "inline-flex"),
        ("align-items", "center"),
        ("justify-content", "center"),
        ("flex-shrink", "0"),
        ("width", "28px"),
        ("height", sp!(28)),
        ("padding", "0"),
        ("border", "0"),
        ("border-radius", "var(--radius-md)"),
        ("background", "transparent"),
        ("color", "inherit"),
        ("cursor", "pointer"),
    ])
    .nest(":hover", pairs(&[("background", ACCENT)]))
    .nest(":focus-visible", pairs(&[("outline", "2px solid"), ("outline-color", RING), ("outline-offset", "2px")]))
});

const PANEL_SVG: &str = r#"<svg data-flip width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round" aria-hidden="true"><rect x="2" y="2.5" width="12" height="11" rx="2"/><path d="M6 2.5v11"/></svg>"#;

impl Component for SidebarTrigger {
    fn name(&self) -> &'static str {
        "ui-sidebar-trigger"
    }
    fn info(&self) -> ComponentInfo {
        info(
            "A button that opens and closes the sidebar of the `ui-sidebar-layout` it is in (one with `bind`).",
            TRIGGER_PROPS,
            &[],
        )
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        crate::flips(cx);
        let Some(Val::Lit(serde_json::Value::String(key))) = cx.context("ui-sidebar").cloned() else {
            return Err(RenderError::new(node, "`ui-sidebar-trigger` must be inside a `ui-sidebar-layout` with `bind`"));
        };
        let toggle = crate::synthetic(
            node,
            "button",
            Vec::new(),
            vec![("type", lit("button")), ("aria-expanded", expr_val(Expr::Unary(UnOp::Not, Box::new(Expr::Unary(UnOp::Not, Box::new(state(&key)))))))],
            vec![("click", vec![Action::SetState { key: key.clone(), value: crate::state_not(&key) }])],
        );
        let mut merged = toggle;
        crate::disclosure::merge_node(&mut merged, node);
        let mut el = cx.element("button", &merged);
        el.base_style_ref(&TRIGGER)
            .attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Toggle sidebar"))))
            .attr("data-sidebar-trigger", true);
        cx.open(&el, &merged)?;
        cx.markup(PANEL_SVG);
        cx.close("button");
        Ok(())
    }
}
