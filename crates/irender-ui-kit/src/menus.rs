//! Menus beyond the dropdown: submenus, checkbox and radio items (for every menu),
//! `ui-menubar` (an application's File / Edit / View bar, the WAI-ARIA menubar pattern)
//! and `ui-navigation-menu` (a site's navigation, with panels of links that open on
//! hover or click, the disclosure navigation pattern).
//!
//! Every menu uses `menu.js`, which also opens submenus and toggles checkbox and radio
//! items; `menubar.js` moves between a menubar's menus, and `navigation_menu.js` opens
//! and closes the navigation's panels.

use crate::{
    ACCENT, MUTED, PRIMARY, SUBTLE, TEXT, context, expr_val, id_after, instance_id, lit,
    overlay::{MENU, MENU_ITEM, MENU_SCRIPT, SHORTCUT, popover_base},
    server_array, state,
};
use irender_core::{ComponentInfo, Expr, Node, PropDef, PropKind, StyleBlock, Val, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const MENUBAR_SCRIPT: &str = include_str!("menubar.js");
const NAVIGATION_SCRIPT: &str = include_str!("navigation_menu.js");

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

const CHEVRON_RIGHT: &str = r#"<svg data-flip width="14" height="14" style="width:calc(14px * var(--size-icon, 1));height:calc(14px * var(--size-icon, 1))" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M6 3.5 10.5 8 6 12.5"/></svg>"#;
const CHEVRON_DOWN: &str = r#"<svg width="12" height="12" style="width:calc(12px * var(--size-icon, 1));height:calc(12px * var(--size-icon, 1))" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" data-ui-chevron><path d="M3.5 6 8 10.5 12.5 6"/></svg>"#;

/// The space before an item's text where a check or a dot shows.
static INDICATOR: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("display", "inline-flex"), ("justify-content", "center"), ("width", "calc(16px * var(--size-icon, 1))"), ("flex-shrink", "0"), ("color", PRIMARY)])
});
/// Checkbox and radio items: the indicator shows only when checked.
static CHECKABLE: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("padding-inline-start", pad!(6))])
        .nest("& [data-ui-indicator]::before", pairs(&[("content", "''")]))
        .nest("&[role=menuitemcheckbox][aria-checked=true] [data-ui-indicator]::before", pairs(&[("content", "'✓'")]))
        .nest("&[role=menuitemradio][aria-checked=true] [data-ui-indicator]::before", pairs(&[("content", "'●'"), ("font-size", fs!(8))]))
});

/// A menu item's text and shortcut.
fn item_text(cx: &mut RenderCx<'_>, node: &Node) -> Result<(), RenderError> {
    let mut label = Element::new("span");
    label.text(node.prop("label").cloned().unwrap_or_else(|| lit("")));
    cx.open(&label, node)?;
    cx.close("span");
    if let Some(s) = node.prop("shortcut") {
        let mut k = cx.part("span", node, "shortcut");
        k.base_style_ref(&SHORTCUT).text(s);
        cx.open(&k, node)?;
        cx.close("span");
    }
    Ok(())
}

/// `keepOpen`: choosing the item leaves the menu open.
fn keep_open(cx: &RenderCx<'_>, node: &Node) -> bool {
    cx.prop(node, "keepOpen").is_some_and(|v| expr::truthy(&v))
}

/// The menu the item is in closes when it's chosen.
fn closes_menu(cx: &RenderCx<'_>, el: &mut Element<'_>) {
    if let Some(menu) = context(cx, "ui-popover") {
        el.attr("popovertarget", menu).attr("popovertargetaction", "hide");
    }
}

// ---------------------------------------------------------------- submenu

pub struct MenuSub;

const SUB_PROPS: &[PropDef] = &[
    PropDef::new("label", PropKind::Any, "The item that opens the submenu").required(),
    PropDef::new("disabled", PropKind::Any, "Boolean or expression"),
    PropDef::new("id", PropKind::Any, "The submenu's element id (default: derived from the document)"),
];

impl Component for MenuSub {
    fn name(&self) -> &'static str {
        "ui-menu-sub"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "An item that opens a submenu beside it, on hover, Enter, Space or Right; Left or Escape closes it. Children are menu items, as in any menu.",
            props: SUB_PROPS,
            parts: &["trigger", "menu"],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        crate::anchors(cx);
        crate::flips(cx);
        if context(cx, "ui-menu").is_none() {
            return Err(RenderError::new(node, "`ui-menu-sub` goes inside a menu: `ui-dropdown-menu`, `ui-context-menu`, `ui-menubar-menu` or another `ui-menu-sub`"));
        }
        let id = instance_id(cx, node, "submenu");
        let anchor = id_after("--", &id);
        let mut trigger = cx.part("button", node, "trigger");
        let anchor = expr::stringify(&cx.eval(&anchor));
        let style = StyleBlock::new().set("anchor-name", anchor.clone());
        trigger
            .base_style_ref(&MENU_ITEM[0])
            .base_style(style)
            .attr("type", "button")
            .attr("role", "menuitem")
            .attr("tabindex", "-1")
            .attr("aria-haspopup", "menu")
            .attr("aria-expanded", "false")
            .attr("aria-controls", id.clone())
            .attr("data-ui-sub-trigger", true);
        if let Some(d) = node.prop("disabled") {
            trigger.attr("disabled", d.clone());
        }
        cx.open(&trigger, node)?;
        let mut label = Element::new("span");
        label.text(node.prop("label").cloned().unwrap_or_else(|| lit("")));
        cx.open(&label, node)?;
        cx.close("span");
        let mut arrow = Element::new("span");
        arrow.base_style(pairs(&[("display", "inline-flex"), ("margin-inline-start", "auto"), ("color", MUTED)]));
        cx.open(&arrow, node)?;
        cx.markup(CHEVRON_RIGHT);
        cx.close("span");
        cx.close("button");

        let mut menu = cx.part("div", node, "menu");
        let place = crate::with_area(StyleBlock::new().set("position-anchor", anchor).set("position-try-fallbacks", "flip-inline").set("margin", gap!(-5, 4)), "right span-bottom");
        menu.base_style_ref(&MENU)
            .base_style(place)
            .attr("popover", "auto")
            .attr("id", id)
            .attr("role", "menu")
            .attr("data-ui-submenu", true)
            .behavior("ui-dropdown-menu", json!({}));
        cx.global_js("ui-dropdown-menu", MENU_SCRIPT);
        cx.open(&menu, node)?;
        // Items keep closing the outermost menu, which closes this one with it.
        cx.children(node)?;
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- checkbox and radio items

pub struct MenuCheckbox;

const CHECKBOX_PROPS: &[PropDef] = &[
    PropDef::new("label", PropKind::Any, "Item text").required(),
    PropDef::new("bind", PropKind::StateKey, "State key holding `true` / `false`; choosing the item flips it").required(),
    PropDef::new("shortcut", PropKind::Any, "Keyboard hint shown on the right"),
    PropDef::new("disabled", PropKind::Any, "Boolean or expression"),
    PropDef::new("keepOpen", PropKind::Bool, "The menu stays open when it's chosen, for flipping several settings in a row"),
];

impl Component for MenuCheckbox {
    fn name(&self) -> &'static str {
        "ui-menu-checkbox"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A menu item that turns a setting on and off (`role=menuitemcheckbox`), with a check while it's on. Choosing it flips `bind` and closes the menu (unless `keepOpen`).",
            props: CHECKBOX_PROPS,
            parts: &["shortcut"],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        crate::anchors(cx);
        crate::flips(cx);
        let key = cx.prop_string(node, "bind").ok_or_else(|| RenderError::new(node, "`bind` is required"))?;
        let mut el = cx.element("button", node);
        el.base_style_ref(&MENU_ITEM[0])
            .base_style_ref(&CHECKABLE)
            .attr("type", "button")
            .attr("role", "menuitemcheckbox")
            .attr("tabindex", "-1")
            .attr("aria-checked", expr_val(Expr::Unary(expr::UnOp::Not, Box::new(Expr::Unary(expr::UnOp::Not, Box::new(state(&key)))))))
            .attr("data-ui-toggle", key);
        if let Some(d) = node.prop("disabled") {
            el.attr("disabled", d.clone());
        }
        if !keep_open(cx, node) {
            closes_menu(cx, &mut el);
        }
        cx.open(&el, node)?;
        let mut mark = Element::new("span");
        mark.base_style_ref(&INDICATOR).attr("data-ui-indicator", true).attr("aria-hidden", "true");
        cx.open(&mark, node)?;
        cx.close("span");
        item_text(cx, node)?;
        cx.close("button");
        Ok(())
    }
}

pub struct MenuRadioGroup;

const RADIO_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key holding the chosen value").required(),
    PropDef::new("options", PropKind::Any, "Values, or `{ value, label, shortcut, disabled }`").required(),
    PropDef::new("label", PropKind::Any, "Accessible name of the group"),
    PropDef::new("keepOpen", PropKind::Bool, "The menu stays open when one is chosen"),
];

impl Component for MenuRadioGroup {
    fn name(&self) -> &'static str {
        "ui-menu-radio-group"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Menu items that pick one of several values (`role=menuitemradio`), with a dot by the chosen one. Choosing one writes its value to `bind` and closes the menu (unless `keepOpen`).",
            props: RADIO_PROPS,
            parts: &[],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        crate::anchors(cx);
        crate::flips(cx);
        let key = cx.prop_string(node, "bind").ok_or_else(|| RenderError::new(node, "`bind` is required"))?;
        let options = server_array(cx, node, "options")?;
        let mut group = cx.element("div", node);
        group.attr("role", "group");
        if let Some(l) = node.prop("label") {
            group.attr("aria-label", l.clone());
        }
        cx.open(&group, node)?;
        for o in options {
            let (value, label, shortcut, disabled) = match &o {
                Json::Object(m) => (
                    m.get("value").cloned().unwrap_or(Json::Null),
                    m.get("label").or(m.get("value")).map(expr::stringify).unwrap_or_default(),
                    m.get("shortcut").map(expr::stringify),
                    m.get("disabled").is_some_and(expr::truthy),
                ),
                v => (v.clone(), expr::stringify(v), None, false),
            };
            let mut el = Element::new("button");
            el.base_style_ref(&MENU_ITEM[0])
                .base_style_ref(&CHECKABLE)
                .attr("type", "button")
                .attr("role", "menuitemradio")
                .attr("tabindex", "-1")
                .attr("aria-checked", crate::state_eq(&key, value.clone()))
                .attr("data-ui-pick", key.clone())
                .attr("data-ui-value", value.to_string())
                .attr("disabled", disabled);
            if !keep_open(cx, node) {
                closes_menu(cx, &mut el);
            }
            cx.open(&el, node)?;
            let mut mark = Element::new("span");
            mark.base_style_ref(&INDICATOR).attr("data-ui-indicator", true).attr("aria-hidden", "true");
            cx.open(&mark, node)?;
            cx.close("span");
            let mut text = Element::new("span");
            text.text(lit(label));
            cx.open(&text, node)?;
            cx.close("span");
            if let Some(s) = shortcut {
                let mut k = Element::new("span");
                k.base_style_ref(&SHORTCUT).text(lit(s));
                cx.open(&k, node)?;
                cx.close("span");
            }
            cx.close("button");
        }
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- menubar

pub struct Menubar;
pub struct MenubarMenu;

const MENUBAR_PROPS: &[PropDef] = &[
    PropDef::new("label", PropKind::Any, "Accessible name (default `Menu bar`)"),
];
const MENUBAR_MENU_PROPS: &[PropDef] = &[
    PropDef::new("label", PropKind::Any, "The menu's name in the bar, e.g. `File`").required(),
    PropDef::new("disabled", PropKind::Any, "Boolean or expression"),
    PropDef::new("id", PropKind::Any, "The menu's element id (default: derived from the document)"),
];

static MENUBAR: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("display", "flex"),
        ("align-items", "center"),
        ("gap", gap!(2)),
        ("width", "fit-content"),
        ("max-width", "100%"),
        ("padding", pad!(4)),
        ("border", "1px solid var(--color-border)"),
        ("border-radius", "var(--radius-md)"),
        ("background", "var(--color-surface)"),
        ("box-shadow", "0 1px 2px rgb(0 0 0 / 0.05)"),
    ])
    .nest(
        "& > [data-ui-menubar-trigger]",
        pairs(&[
            ("padding", pad!(4, 10)),
            ("border", "0"),
            ("border-radius", "var(--radius-sm)"),
            ("background", "transparent"),
            ("color", TEXT),
            ("font", "inherit"),
            ("font-size", fs!(14)),
            ("font-weight", "500"),
            ("cursor", "pointer"),
        ]),
    )
    .nest("& > [data-ui-menubar-trigger]:hover", pairs(&[("background", ACCENT)]))
    .nest("& > [data-ui-menubar-trigger][aria-expanded=true]", pairs(&[("background", ACCENT)]))
    .nest(
        "& > [data-ui-menubar-trigger]:focus-visible",
        pairs(&[("background", ACCENT), ("outline", "2px solid var(--color-ring, var(--color-muted))"), ("outline-offset", "-2px")]),
    )
    .nest("& > [data-ui-menubar-trigger]:disabled", pairs(&[("opacity", "0.5"), ("pointer-events", "none")]))
});

impl Component for Menubar {
    fn name(&self) -> &'static str {
        "ui-menubar"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A bar of menus, as at the top of an application: `ui-menubar-menu` children. Left and Right move along the bar (an open menu follows), Down opens a menu, and hovering moves between open menus.",
            props: MENUBAR_PROPS,
            parts: &[],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        crate::anchors(cx);
        crate::flips(cx);
        cx.global_js("ui-dropdown-menu", MENU_SCRIPT);
        cx.global_js("ui-menubar", MENUBAR_SCRIPT);
        let mut root = cx.element("div", node);
        root.base_style_ref(&MENUBAR)
            .attr("role", "menubar")
            .attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Menu bar"))))
            .behavior("ui-menubar", json!({}));
        cx.open(&root, node)?;
        for (i, menu) in node.children.iter().enumerate() {
            if menu.kind != "ui-menubar-menu" {
                return Err(RenderError::new(menu, format!("`ui-menubar` takes `ui-menubar-menu` children, not `{}`", menu.kind)));
            }
            let id = crate::own_id(cx, menu, "menubar-menu");
            let anchor = expr::stringify(&cx.eval(&id_after("--", &id)));
            let mut t = Element::new("button");
            t.attr("type", "button")
                .attr("role", "menuitem")
                .attr("tabindex", if i == 0 { "0" } else { "-1" })
                .attr("aria-haspopup", "menu")
                .attr("aria-expanded", "false")
                .attr("popovertarget", id.clone())
                .attr("data-ui-menubar-trigger", true)
                .base_style(StyleBlock::new().set("anchor-name", anchor.clone()))
                .text(menu.prop("label").cloned().unwrap_or_else(|| lit("")));
            if let Some(d) = menu.prop("disabled") {
                t.attr("disabled", d.clone());
            }
            cx.open(&t, menu)?;
            cx.close("button");
            let mut m = cx.element("div", menu);
            m.base_style_ref(&MENU)
                .base_style(crate::with_area(StyleBlock::new().set("position-anchor", anchor).set("position-try-fallbacks", "flip-block, flip-inline").set("margin", concat!(gap!(6), " 0")), "bottom span-right"))
                .attr("popover", "auto")
                .attr("id", id.clone())
                .attr("role", "menu")
                .attr("aria-label", menu.prop("label").cloned().unwrap_or_else(|| lit("")))
                .attr("data-ui-menubar-menu", true)
                .behavior("ui-dropdown-menu", json!({}));
            cx.open(&m, menu)?;
            cx.with_context("ui-popover", Some(id), |cx| cx.with_context("ui-menu", Some(lit(true)), |cx| cx.children(menu)))?;
            cx.close("div");
        }
        cx.close("div");
        Ok(())
    }
}

impl Component for MenubarMenu {
    fn name(&self) -> &'static str {
        "ui-menubar-menu"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "One menu of a `ui-menubar`: its name in the bar, and menu items as children (`ui-menu-item`, `-checkbox`, `-radio-group`, `-sub`, `-label`, `-separator`).",
            props: MENUBAR_MENU_PROPS,
            parts: &[],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, _cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        Err(RenderError::new(node, "`ui-menubar-menu` must be a direct child of `ui-menubar`"))
    }
}

// ---------------------------------------------------------------- navigation menu

pub struct NavigationMenu;
pub struct NavigationItem;
pub struct NavigationLink;

const NAV_PROPS: &[PropDef] = &[
    PropDef::new("label", PropKind::Any, "Accessible name of the navigation (default `Main`)"),
    PropDef::new("align", PropKind::Enum(&["start", "center", "end"]), "Where panels line up with their item (default `start`)"),
];
const NAV_ITEM_PROPS: &[PropDef] = &[
    PropDef::new("label", PropKind::Any, "The item's text in the bar").required(),
    PropDef::new("width", PropKind::Any, "The panel's width (default: its content's, at most the window's)"),
    PropDef::new("id", PropKind::Any, "The panel's element id (default: derived from the document)"),
];
const NAV_LINK_PROPS: &[PropDef] = &[
    PropDef::new("label", PropKind::Any, "Link text").required(),
    PropDef::new("href", PropKind::Any, "Where it goes").required(),
    PropDef::new("description", PropKind::Any, "A line under the text, in a panel"),
    PropDef::new("active", PropKind::Any, "The current page (`aria-current=page`)"),
];

static NAV: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("position", "relative"), ("font-size", fs!(14)), ("color", TEXT)])
        .nest("& > ul", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("align-items", "center"), ("gap", gap!(2)), ("margin", "0"), ("padding", "0"), ("list-style", "none")]))
        .nest(
            "& > ul > li > [data-ui-nav-trigger], & > ul > li > a",
            pairs(&[
                ("display", "inline-flex"),
                ("align-items", "center"),
                ("gap", gap!(4)),
                ("height", ctl!(36)),
                ("padding", pad!(0, 12)),
                ("border", "0"),
                ("border-radius", "var(--radius-md)"),
                ("background", "transparent"),
                ("color", "inherit"),
                ("font", "inherit"),
                ("font-weight", "500"),
                ("text-decoration", "none"),
                ("cursor", "pointer"),
            ]),
        )
        .nest("& > ul > li > [data-ui-nav-trigger]:hover, & > ul > li > a:hover", pairs(&[("background", ACCENT)]))
        .nest("& > ul > li > [aria-expanded=true]", pairs(&[("background", ACCENT)]))
        .nest("& > ul > li > a[aria-current=page]", pairs(&[("background", ACCENT)]))
        .nest(
            "& > ul > li > *:focus-visible",
            pairs(&[("outline", "2px solid var(--color-ring, var(--color-muted))"), ("outline-offset", "1px")]),
        )
        .nest("& [data-ui-chevron]", pairs(&[("transition", "transform .2s"), ("color", MUTED)]))
        .nest("& [aria-expanded=true] [data-ui-chevron]", pairs(&[("transform", "rotate(180deg)")]))
        .nest(
            "& [data-ui-nav-panel] [data-ui-nav-link]",
            pairs(&[
                ("display", "flex"),
                ("flex-direction", "column"),
                ("gap", gap!(4)),
                ("padding", pad!(10, 12)),
                ("border-radius", "var(--radius-md)"),
                ("color", "inherit"),
                ("text-decoration", "none"),
                ("line-height", "1.3"),
            ]),
        )
        .nest("& [data-ui-nav-panel] [data-ui-nav-link]:hover, & [data-ui-nav-panel] [data-ui-nav-link]:focus-visible", pairs(&[("background", SUBTLE), ("outline", "none")]))
        .nest("& [data-ui-nav-panel] [data-ui-nav-link] > span:first-child", pairs(&[("font-weight", "500")]))
        .nest("& [data-ui-nav-panel] [data-ui-nav-link] > span + span", pairs(&[("color", MUTED), ("font-size", fs!(13))]))
});

static NAV_PANEL: LazyLock<StyleBlock> = LazyLock::new(|| {
    popover_base()
        .set("max-width", "calc(100vw - 16px)")
        .set("padding", pad!(8))
        .set("margin", concat!(gap!(8), " 0"))
        .set("font-size", fs!(14))
        .set("position-try-fallbacks", "flip-inline")
});

impl Component for NavigationMenu {
    fn name(&self) -> &'static str {
        "ui-navigation-menu"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A site's navigation: `ui-navigation-link` children are links, and `ui-navigation-item` children open a panel of links (or anything) on hover or click. Down opens a panel and moves into it; Escape closes it.",
            props: NAV_PROPS,
            parts: &[],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        crate::anchors(cx);
        crate::flips(cx);
        cx.global_js("ui-navigation-menu", NAVIGATION_SCRIPT);
        let align = cx.prop_string(node, "align").unwrap_or_else(|| "start".into());
        let area = match align.as_str() {
            "center" => "bottom",
            "end" => "bottom span-left",
            _ => "bottom span-right",
        };
        let mut root = cx.element("nav", node);
        root.base_style_ref(&NAV).attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Main")))).behavior("ui-navigation-menu", json!({}));
        cx.open(&root, node)?;
        cx.open(&Element::new("ul"), node)?;
        for child in &node.children {
            cx.open(&Element::new("li"), node)?;
            match child.kind.as_str() {
                "ui-navigation-link" => cx.render(child)?,
                "ui-navigation-item" => {
                    let id = crate::own_id(cx, child, "nav-panel");
                    let anchor = expr::stringify(&cx.eval(&id_after("--", &id)));
                    let mut t = Element::new("button");
                    t.attr("type", "button")
                        .attr("aria-expanded", "false")
                        .attr("aria-controls", id.clone())
                        .attr("data-ui-nav-trigger", true)
                        .base_style(StyleBlock::new().set("anchor-name", anchor.clone()));
                    cx.open(&t, child)?;
                    let mut label = Element::new("span");
                    label.text(child.prop("label").cloned().unwrap_or_else(|| lit("")));
                    cx.open(&label, child)?;
                    cx.close("span");
                    cx.markup(CHEVRON_DOWN);
                    cx.close("button");
                    let mut size = StyleBlock::new();
                    if let Some(w) = child.prop("width")
                        && let Some(v) = cx.style_value(child, "width", w)?
                    {
                        size.push("width", v);
                    }
                    let mut p = cx.element("div", child);
                    p.base_style_ref(&NAV_PANEL)
                        .base_style(crate::with_area(StyleBlock::new().set("position-anchor", anchor), area))
                        .base_style(size)
                        .attr("popover", "auto")
                        .attr("id", id)
                        .attr("data-ui-nav-panel", true);
                    cx.open(&p, child)?;
                    cx.with_context("ui-navigation-panel", Some(lit(true)), |cx| cx.children(child))?;
                    cx.close("div");
                }
                other => {
                    return Err(RenderError::new(
                        child,
                        format!("`ui-navigation-menu` takes `ui-navigation-item` and `ui-navigation-link` children, not `{other}`"),
                    ));
                }
            }
            cx.close("li");
        }
        cx.close("ul");
        cx.close("nav");
        Ok(())
    }
}

impl Component for NavigationItem {
    fn name(&self) -> &'static str {
        "ui-navigation-item"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "An item of a `ui-navigation-menu` that opens a panel: its children, usually `ui-navigation-link`s with descriptions, laid out as you like.",
            props: NAV_ITEM_PROPS,
            parts: &[],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, _cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        Err(RenderError::new(node, "`ui-navigation-item` must be a direct child of `ui-navigation-menu`"))
    }
}

impl Component for NavigationLink {
    fn name(&self) -> &'static str {
        "ui-navigation-link"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A link in a `ui-navigation-menu`: in the bar, or in an item's panel, where it shows its `description` under the text.",
            props: NAV_LINK_PROPS,
            parts: &[],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        crate::anchors(cx);
        crate::flips(cx);
        let mut a = cx.element("a", node);
        a.attr("href", node.prop("href").cloned().unwrap_or_else(|| lit("#")));
        if let Some(act) = node.prop("active") {
            let current = match act {
                Val::Lit(v) => lit(if expr::truthy(v) { json!("page") } else { Json::Null }),
                Val::Tpl(_) => {
                    let c = crate::val_expr(act);
                    expr_val(crate::cond(c, Expr::Lit(json!("page")), Expr::Lit(Json::Null)))
                }
            };
            a.attr("aria-current", current);
        }
        let in_panel = context(cx, "ui-navigation-panel").is_some();
        if in_panel {
            a.attr("data-ui-nav-link", true);
            cx.open(&a, node)?;
            let mut t = Element::new("span");
            t.text(node.prop("label").cloned().unwrap_or_else(|| lit("")));
            cx.open(&t, node)?;
            cx.close("span");
            if let Some(d) = node.prop("description") {
                let mut s = Element::new("span");
                s.text(d);
                cx.open(&s, node)?;
                cx.close("span");
            }
        } else {
            a.text(node.prop("label").cloned().unwrap_or_else(|| lit("")));
            cx.open(&a, node)?;
        }
        cx.close("a");
        Ok(())
    }
}
