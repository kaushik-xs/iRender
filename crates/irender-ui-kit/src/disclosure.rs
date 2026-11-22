//! Components that show or switch content: accordion and collapsible (native
//! `<details>`), tabs, toggle and toggle group (state and actions), and the theme
//! toggle (a client behaviour).

use crate::{
    ACCENT, MUTED, SUBTLE, SURFACE, TEXT, choice, context, expr_val, field, id_with, instance_id, lit, items, each_item, field_or,
    state, state_eq, state_not, synthetic,
};
use irender_core::{
    Action, ComponentInfo, Node, PropDef, PropKind, StyleBlock, Val,
    expr::{self, BinOp, Expr, Func, UnOp},
};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const fn info(doc: &'static str, props: &'static [PropDef], parts: &'static [&'static str], children: bool) -> ComponentInfo {
    ComponentInfo { doc, props, parts, children, open_props: false }
}

fn focus_outline() -> StyleBlock {
    StyleBlock::from_pairs(&[("outline", "2px solid var(--color-ring, var(--color-muted))"), ("outline-offset", "2px")])
}

/// `<summary>` styles shared by accordion items and collapsibles: no marker, and a
/// chevron that turns when the `<details>` opens.
fn summary_rules(block: StyleBlock) -> StyleBlock {
    block
        .nest(
            "& > summary",
            StyleBlock::from_pairs(&[
                ("display", "flex"),
                ("align-items", "center"),
                ("justify-content", "space-between"),
                ("gap", gap!(16)),
                ("list-style", "none"),
                ("cursor", "pointer"),
                ("font-weight", "var(--font-weight-medium, 500)"),
            ]),
        )
        .nest("& > summary::-webkit-details-marker", StyleBlock::from_pairs(&[("display", "none")]))
        .nest(
            "& > summary::after",
            StyleBlock::from_pairs(&[("content", "'⌄'"), ("color", MUTED), ("line-height", "1"), ("transition", "transform .2s")]),
        )
        .nest("&[open] > summary::after", StyleBlock::from_pairs(&[("transform", "rotate(180deg)")]))
        .nest("& > summary:focus-visible", focus_outline())
}

// ---------------------------------------------------------------- accordion

pub struct Accordion;

const ACCORDION_PROPS: &[PropDef] = &[
    PropDef::new("multiple", PropKind::Bool, "Let several items be open at once (default: one at a time)"),
    PropDef::new("id", PropKind::Any, "Element id (default: derived from the document)"),
];

static ACCORDION: LazyLock<StyleBlock> = LazyLock::new(|| StyleBlock::from_pairs(&[("width", "100%")]));

impl Component for Accordion {
    fn name(&self) -> &'static str {
        "ui-accordion"
    }
    fn info(&self) -> ComponentInfo {
        info("Stacked sections that expand one at a time. Children are `ui-accordion-item`s.", ACCORDION_PROPS, &[], true)
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let id = instance_id(cx, node, "ui-accordion");
        let multiple = cx.prop(node, "multiple").is_some_and(|m| expr::truthy(&m));
        // Items name their `<details>` after the accordion, which makes them exclusive.
        let group = (!multiple).then(|| id.clone());
        let mut el = cx.element("div", node);
        el.base_style_ref(&ACCORDION).attr("id", id);
        cx.open(&el, node)?;
        cx.with_context("ui-accordion", group, |cx| cx.children(node))?;
        cx.close("div");
        Ok(())
    }
}

pub struct AccordionItem;

const ACCORDION_ITEM_PROPS: &[PropDef] = &[
    PropDef::new("title", PropKind::Any, "The always-visible heading").required(),
    PropDef::new("open", PropKind::Any, "Start open"),
];

static ACCORDION_ITEM: LazyLock<StyleBlock> = LazyLock::new(|| {
    summary_rules(StyleBlock::from_pairs(&[("border-bottom", "var(--border-width, 1px) solid var(--color-border)")]))
        .nest("& > summary", StyleBlock::from_pairs(&[("padding", pad!(16, 0)), ("font-size", fs!(14))]))
        .nest("& > summary:hover", StyleBlock::from_pairs(&[("text-decoration", "underline")]))
});
static DISCLOSURE_CONTENT: LazyLock<StyleBlock> =
    LazyLock::new(|| StyleBlock::from_pairs(&[("padding-bottom", pad!(16)), ("font-size", fs!(14))]));

/// `<details>` with a `<summary>` holding `title`, then the children.
fn details(node: &Node, cx: &mut RenderCx<'_>, base: &'static StyleBlock, title: &Val, name: Option<Val>) -> Result<(), RenderError> {
    let mut el = cx.element("details", node);
    el.base_style_ref(base);
    if let Some(n) = name {
        el.attr("name", n);
    }
    if let Some(o) = node.prop("open") {
        el.attr("open", o.clone());
    }
    cx.open(&el, node)?;
    let mut summary = cx.part("summary", node, "trigger");
    summary.text(title);
    cx.open(&summary, node)?;
    cx.close("summary");
    let mut content = cx.part("div", node, "content");
    content.base_style_ref(&DISCLOSURE_CONTENT);
    cx.open(&content, node)?;
    cx.children(node)?;
    cx.close("div");
    cx.close("details");
    Ok(())
}

impl Component for AccordionItem {
    fn name(&self) -> &'static str {
        "ui-accordion-item"
    }
    fn info(&self) -> ComponentInfo {
        info("One section of a `ui-accordion`.", ACCORDION_ITEM_PROPS, &["trigger", "content"], true)
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let title = node.prop("title").cloned().unwrap_or_else(|| lit(""));
        details(node, cx, &ACCORDION_ITEM, &title, context(cx, "ui-accordion"))
    }
}

// ---------------------------------------------------------------- collapsible

pub struct Collapsible;

const COLLAPSIBLE_PROPS: &[PropDef] = &[
    PropDef::new("trigger", PropKind::Any, "Text of the row that opens and closes it").required(),
    PropDef::new("open", PropKind::Any, "Start open"),
];

static COLLAPSIBLE: LazyLock<StyleBlock> = LazyLock::new(|| {
    summary_rules(StyleBlock::new())
        .nest(
            "& > summary",
            StyleBlock::from_pairs(&[("padding", pad!(8, 12)), ("border-radius", "var(--radius-control, var(--radius-md))"), ("font-size", fs!(14))]),
        )
        .nest("& > summary:hover", StyleBlock::from_pairs(&[("background", ACCENT)]))
        .nest("& > div", StyleBlock::from_pairs(&[("padding", pad!(8, 12, 0))]))
});

impl Component for Collapsible {
    fn name(&self) -> &'static str {
        "ui-collapsible"
    }
    fn info(&self) -> ComponentInfo {
        info("Content behind a row that opens and closes it.", COLLAPSIBLE_PROPS, &["trigger", "content"], true)
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let trigger = node.prop("trigger").cloned().unwrap_or_else(|| lit(""));
        details(node, cx, &COLLAPSIBLE, &trigger, None)
    }
}

// ---------------------------------------------------------------- tabs

pub struct Tabs;
pub struct TabPanel;

const TABS_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key holding the `value` of the open tab").required(),
    PropDef::new("orientation", PropKind::Enum(&["horizontal", "vertical"]), "`vertical`: the tabs in a column beside their panel, across on a narrow screen (default `horizontal`)"),
    PropDef::new("activation", PropKind::Enum(&["automatic", "manual"]), "`manual`: the arrow keys move between tabs, and Enter or Space opens one, for panels that are slow to show"),
    PropDef::new("id", PropKind::Any, "Element id (default: derived from the document)"),
];
const TAB_PANEL_PROPS: &[PropDef] = &[
    PropDef::new("value", PropKind::Any, "What `bind` holds when this tab is open").required(),
    PropDef::new("label", PropKind::Any, "Tab text (default: the value)"),
];

static TABS: LazyLock<StyleBlock> =
    LazyLock::new(|| StyleBlock::from_pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(8)), ("min-width", "0")]));
static TABS_VERTICAL: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::new().nest(
        "@media (min-width: 641px)",
        StyleBlock::from_pairs(&[("display", "grid"), ("grid-template-columns", "max-content minmax(0, 1fr)"), ("gap", gap!(8, 20)), ("align-items", "start")])
            .nest("& > [role=tabpanel]", StyleBlock::from_pairs(&[("grid-column", "2")]))
            .nest("& > [role=tablist]", StyleBlock::from_pairs(&[("grid-column", "1"), ("grid-row", "1 / span 100")])),
    )
});
static TAB_LIST_VERTICAL: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::new().nest(
        "@media (min-width: 641px)",
        StyleBlock::from_pairs(&[("flex-direction", "column"), ("align-items", "stretch"), ("max-height", "100%"), ("overflow", "visible")]).nest("& > button", StyleBlock::from_pairs(&[("justify-content", "flex-start")])),
    )
});
const TABS_SCRIPT: &str = include_str!("tabs.js");
// More tabs than fit scroll within the list, which fades on the sides with more.
static TAB_LIST: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("box-sizing", "border-box"),
        ("display", "inline-flex"),
        ("align-items", "center"),
        ("gap", gap!(2)),
        ("width", "fit-content"),
        ("max-width", "100%"),
        ("overflow-x", "auto"),
        ("scrollbar-width", "none"),
        ("padding", pad!(3)),
        ("border-radius", "var(--radius-card, var(--radius-lg))"),
        ("background", SUBTLE),
        ("color", MUTED),
    ])
    .nest("&::-webkit-scrollbar", StyleBlock::from_pairs(&[("display", "none")]))
    .nest("&[data-more=left]", StyleBlock::from_pairs(&[("mask-image", "linear-gradient(to right, transparent, #000 32px)")]))
    .nest("&[data-more=right]", StyleBlock::from_pairs(&[("mask-image", "linear-gradient(to left, transparent, #000 32px)")]))
    .nest(
        "&[data-more=both]",
        StyleBlock::from_pairs(&[("mask-image", "linear-gradient(to right, transparent, #000 32px, #000 calc(100% - 32px), transparent)")]),
    )
});
static TAB: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("display", "inline-flex"),
        ("flex-shrink", "0"),
        ("align-items", "center"),
        ("justify-content", "center"),
        ("height", ctl!(30)),
        ("padding", pad!(0, 12)),
        ("border", "var(--border-width, 1px) solid transparent"),
        ("border-radius", "var(--radius-control, var(--radius-md))"),
        ("background", "transparent"),
        ("color", "inherit"),
        ("font-family", "inherit"),
        ("font-size", fs!(14)),
        ("font-weight", "var(--font-weight-medium, 500)"),
        ("white-space", "nowrap"),
        ("cursor", "pointer"),
        ("transition", "background-color .15s, color .15s"),
    ])
    .nest(":hover", StyleBlock::from_pairs(&[("color", TEXT)]))
    // Within the list's padding, which the scrolling would clip it to.
    .nest(":focus-visible", focus_outline().set("outline-offset", "0"))
    .nest(
        "&[aria-selected=true]",
        StyleBlock::from_pairs(&[("background", SURFACE), ("color", TEXT), ("box-shadow", "0 1px 2px rgb(0 0 0 / 0.08)")]),
    )
});
static TAB_PANEL: LazyLock<StyleBlock> = LazyLock::new(|| StyleBlock::new().nest(":focus-visible", focus_outline()));

impl Component for Tabs {
    fn name(&self) -> &'static str {
        "ui-tabs"
    }
    fn info(&self) -> ComponentInfo {
        info(
            "Panels shown one at a time, picked from a row of tabs. Children are `ui-tab-panel`s.",
            TABS_PROPS,
            &["list"],
            true,
        )
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let Some(key) = cx.prop_string(node, "bind") else { return Err(RenderError::new(node, "`ui-tabs` needs `bind`")) };
        if let Some(other) = node.children.iter().find(|c| c.kind != "ui-tab-panel") {
            return Err(RenderError::new(other, "children of `ui-tabs` must be `ui-tab-panel`s"));
        }
        let id = instance_id(cx, node, "ui-tabs");
        let panels: Vec<(Json, Val)> = node
            .children
            .iter()
            .map(|p| {
                let value = cx.prop(p, "value").unwrap_or(Json::Null);
                let label = p.prop("label").cloned().unwrap_or_else(|| lit(expr::stringify(&value)));
                (value, label)
            })
            .collect();

        let vertical = cx.prop_string(node, "orientation").as_deref() == Some("vertical");
        let manual = cx.prop_string(node, "activation").as_deref() == Some("manual");
        let mut el = cx.element("div", node);
        el.base_style_ref(&TABS);
        if vertical {
            el.base_style_ref(&TABS_VERTICAL);
        }
        cx.open(&el, node)?;
        let mut list = cx.part("div", node, "list");
        list.base_style_ref(&TAB_LIST).attr("role", "tablist").behavior("ui-tabs", json!({ "vertical": vertical, "manual": manual }));
        if vertical {
            list.base_style_ref(&TAB_LIST_VERTICAL).attr("aria-orientation", "vertical");
        }
        cx.global_js("ui-tabs", TABS_SCRIPT);
        cx.open(&list, node)?;
        for (i, (value, label)) in panels.iter().enumerate() {
            let tab = synthetic(
                node,
                "button",
                Vec::new(),
                vec![
                    ("type", lit("button")),
                    ("role", lit("tab")),
                    ("id", id_with(&id, &format!("-tab-{i}"))),
                    ("aria-controls", id_with(&id, &format!("-panel-{i}"))),
                    ("aria-selected", state_eq(&key, value.clone())),
                ],
                vec![("click", vec![Action::SetState { key: key.clone(), value: Val::Lit(value.clone()) }])],
            );
            let mut b = cx.element("button", &tab);
            b.base_style_ref(&TAB).text(label);
            cx.open(&b, &tab)?;
            cx.close("button");
        }
        cx.close("div");
        for (i, (panel, (value, _))) in node.children.iter().zip(&panels).enumerate() {
            let mut p = cx.element("div", panel);
            p.base_style_ref(&TAB_PANEL);
            p.attr("role", "tabpanel")
                .attr("id", id_with(&id, &format!("-panel-{i}")))
                .attr("aria-labelledby", id_with(&id, &format!("-tab-{i}")))
                .attr("tabindex", lit(0))
                .attr("hidden", expr_val(Expr::Binary(BinOp::Ne, Box::new(state(&key)), Box::new(Expr::Lit(value.clone())))));
            cx.open(&p, panel)?;
            cx.children(panel)?;
            cx.close("div");
        }
        cx.close("div");
        Ok(())
    }
}

impl Component for TabPanel {
    fn name(&self) -> &'static str {
        "ui-tab-panel"
    }
    fn info(&self) -> ComponentInfo {
        info("One panel of a `ui-tabs`.", TAB_PANEL_PROPS, &[], true)
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        // Outside `ui-tabs` it is just a container.
        let el = cx.element("div", node);
        cx.open(&el, node)?;
        cx.children(node)?;
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- toggle

pub struct Toggle;
pub struct ToggleGroup;

const TOGGLE_VARIANTS: &[&str] = &["default", "outline"];
const TOGGLE_SIZES: &[&str] = &["default", "sm", "lg"];
const TOGGLE_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key holding `true` / `false`").required(),
    PropDef::new("label", PropKind::Any, "Button text"),
    PropDef::new("variant", PropKind::Enum(TOGGLE_VARIANTS), "Default `default`"),
    PropDef::new("size", PropKind::Enum(TOGGLE_SIZES), "Default `default`"),
    PropDef::new("disabled", PropKind::Any, "Boolean or expression"),
];
const TOGGLE_GROUP_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key holding the pressed value, or null (a list of them with `multiple`)").required(),
    PropDef::new("options", PropKind::Any, "Array of values or `{ value, label, disabled }`; may use state").required(),
    PropDef::new("multiple", PropKind::Bool, "Let several be pressed; `bind` holds a list"),
    PropDef::new("variant", PropKind::Enum(TOGGLE_VARIANTS), "Default `default`"),
    PropDef::new("size", PropKind::Enum(TOGGLE_SIZES), "Default `default`"),
    PropDef::new("disabled", PropKind::Any, "Disable every option"),
];

static TOGGLE: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("display", "inline-flex"),
        ("align-items", "center"),
        ("justify-content", "center"),
        ("gap", gap!(8)),
        ("border", "var(--border-width, 1px) solid transparent"),
        ("border-radius", "var(--radius-control, var(--radius-md))"),
        ("background", "transparent"),
        ("color", TEXT),
        ("font-family", "inherit"),
        ("font-size", fs!(14)),
        ("font-weight", "var(--font-weight-medium, 500)"),
        ("white-space", "nowrap"),
        ("cursor", "pointer"),
        ("transition", "background-color .15s, color .15s"),
    ])
    .nest(":hover", StyleBlock::from_pairs(&[("background", ACCENT), ("color", MUTED)]))
    .nest("&[aria-pressed=true]", StyleBlock::from_pairs(&[("background", ACCENT), ("color", TEXT)]))
    .nest(":focus-visible", StyleBlock::from_pairs(&[("outline", "none"), ("box-shadow", "0 0 0 3px color-mix(in srgb, var(--color-ring, var(--color-muted)) 50%, transparent)")]))
    .nest(":disabled", StyleBlock::from_pairs(&[("opacity", "0.5"), ("pointer-events", "none")]))
});
static TOGGLE_VARIANT_STYLES: LazyLock<[StyleBlock; 2]> = LazyLock::new(|| {
    [
        StyleBlock::new(),
        StyleBlock::from_pairs(&[("border-color", "var(--color-border)"), ("box-shadow", "var(--elevation-control, 0 1px 2px rgb(0 0 0 / 0.05))")]),
    ]
});
static TOGGLE_SIZE_STYLES: LazyLock<[StyleBlock; 3]> = LazyLock::new(|| {
    [
        StyleBlock::from_pairs(&[("height", ctl!(36)), ("min-width", ctl!(36)), ("padding", pad!(0, 10))]),
        StyleBlock::from_pairs(&[("height", ctl!(32)), ("min-width", ctl!(32)), ("padding", pad!(0, 8))]),
        StyleBlock::from_pairs(&[("height", ctl!(40)), ("min-width", ctl!(40)), ("padding", pad!(0, 12))]),
    ]
});
static TOGGLE_GROUP: LazyLock<StyleBlock> = LazyLock::new(|| {
    // Wraps when it's wider than what it's in, but isn't squeezed by what's beside it.
    StyleBlock::from_pairs(&[
        ("display", "inline-flex"),
        ("flex-wrap", "wrap"),
        ("flex-shrink", "0"),
        ("align-items", "center"),
        ("gap", gap!(4)),
        ("width", "fit-content"),
        ("max-width", "100%"),
    ])
});

impl Component for Toggle {
    fn name(&self) -> &'static str {
        "ui-toggle"
    }
    fn info(&self) -> ComponentInfo {
        info("A button that stays pressed. `bind` holds whether it is.", TOGGLE_PROPS, &[], true)
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let Some(key) = cx.prop_string(node, "bind") else { return Err(RenderError::new(node, "`ui-toggle` needs `bind`")) };
        let pressed = expr_val(Expr::Unary(UnOp::Not, Box::new(Expr::Unary(UnOp::Not, Box::new(state(&key))))));
        let mut button = synthetic(
            node,
            "button",
            Vec::new(),
            vec![("type", lit("button")), ("aria-pressed", pressed)],
            vec![("click", vec![Action::SetState { key: key.clone(), value: state_not(&key) }])],
        );
        merge_node(&mut button, node);
        let mut el = cx.element("button", &button);
        el.base_style_ref(&TOGGLE)
            .base_style_ref(&TOGGLE_VARIANT_STYLES[choice(cx, node, "variant", TOGGLE_VARIANTS)])
            .base_style_ref(&TOGGLE_SIZE_STYLES[choice(cx, node, "size", TOGGLE_SIZES)]);
        if let Some(d) = node.prop("disabled") {
            el.attr("disabled", d.clone());
        }
        if let Some(label) = node.prop("label") {
            el.text(label);
        }
        cx.open(&el, &button)?;
        cx.children(node)?;
        cx.close("button");
        Ok(())
    }
}

impl Component for ToggleGroup {
    fn name(&self) -> &'static str {
        "ui-toggle-group"
    }
    fn info(&self) -> ComponentInfo {
        info(
            "A row of toggles where one at most is pressed; pressing it again clears it. With `multiple`, any \
             number can be pressed.",
            TOGGLE_GROUP_PROPS,
            &[],
            false,
        )
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let Some(key) = cx.prop_string(node, "bind") else {
            return Err(RenderError::new(node, "`ui-toggle-group` needs `bind`"));
        };
        let options = items(cx, node, "options")?;
        let variant = &TOGGLE_VARIANT_STYLES[choice(cx, node, "variant", TOGGLE_VARIANTS)];
        let size = &TOGGLE_SIZE_STYLES[choice(cx, node, "size", TOGGLE_SIZES)];
        let disabled_all = cx.prop(node, "disabled").is_some_and(|d| expr::truthy(&d));
        let multiple = cx.prop(node, "multiple").is_some_and(|m| expr::truthy(&m));
        // Each option, from expressions over it (see `ui-radio-group`).
        let value = field_or(OPTION, "value", Expr::Path(vec![OPTION.into()]));
        let label = expr_val(field_or(OPTION, "label", value.clone()));
        let disabled = if disabled_all { lit(true) } else { expr_val(Expr::Path(vec![OPTION.into(), "disabled".into()])) };
        let (pressed, next) = if multiple {
            // Pressing adds the option to the list, or takes it out.
            let call = |f| expr_val(Expr::Call(f, vec![state(&key), value.clone()]));
            (call(Func::Includes), call(Func::Toggle))
        } else {
            // Pressing the pressed option clears the choice.
            let is = Expr::Binary(BinOp::Eq, Box::new(state(&key)), Box::new(value.clone()));
            let next = expr_val(Expr::Cond(Box::new(is.clone()), Box::new(Expr::Lit(Json::Null)), Box::new(value.clone())));
            (expr_val(is), next)
        };
        let option = |cx: &mut RenderCx<'_>| {
            let item = synthetic(
                node,
                "button",
                Vec::new(),
                vec![("type", lit("button")), ("aria-pressed", pressed.clone())],
                vec![("click", vec![Action::SetState { key: key.clone(), value: next.clone() }])],
            );
            let mut el = cx.element("button", &item);
            el.base_style_ref(&TOGGLE).base_style_ref(variant).base_style_ref(size).text(label.clone());
            el.attr("disabled", disabled.clone());
            cx.open(&el, &item)?;
            cx.close("button");
            Ok(())
        };
        let mut group = cx.element("div", node);
        group.base_style_ref(&TOGGLE_GROUP).attr("role", "group");
        each_item(cx, node, group, options, OPTION, "ui_index", &expr_val(value.clone()), option)?;
        cx.close("div");
        Ok(())
    }
}

pub struct ThemeToggle;

/// The name of the current option while rendering a toggle group's options.
const OPTION: &str = "ui_option";

/// Applies the stored scheme; [`crate::register`] adds it to every page.
pub(crate) const THEME_EARLY_SCRIPT: &str = include_str!("theme_early.js");
const THEME_TOGGLE_SCRIPT: &str = include_str!("theme_toggle.js");
const THEME_TOGGLE_PROPS: &[PropDef] = &[
    PropDef::new("labels", PropKind::Any, "Button text as `{ system, light, dark }`; default System, Light, Dark"),
    PropDef::new("variant", PropKind::Enum(TOGGLE_VARIANTS), "Default `outline`"),
    PropDef::new("size", PropKind::Enum(TOGGLE_SIZES), "Default `sm`"),
];
// Labels: "System", "Light", "Dark"
const SCHEMES: [(&str, &str); 3] = [("system", "System"), ("light", "Light"), ("dark", "Dark")];

impl Component for ThemeToggle {
    fn name(&self) -> &'static str {
        "ui-theme-toggle"
    }
    fn info(&self) -> ComponentInfo {
        info(
            "Switches the page between the system's colour scheme, light and dark, and remembers the choice in the browser.",
            THEME_TOGGLE_PROPS,
            &[],
            false,
        )
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let variant = &TOGGLE_VARIANT_STYLES[cx.prop_string(node, "variant").map_or(1, |_| choice(cx, node, "variant", TOGGLE_VARIANTS))];
        let size = &TOGGLE_SIZE_STYLES[cx.prop_string(node, "size").map_or(1, |_| choice(cx, node, "size", TOGGLE_SIZES))];
        let labels = cx.prop(node, "labels").unwrap_or(Json::Null);
        cx.early_js("ui-theme", THEME_EARLY_SCRIPT);
        cx.global_js("ui-theme-toggle", THEME_TOGGLE_SCRIPT);
        let mut group = cx.element("div", node);
        group
            .base_style_ref(&TOGGLE_GROUP)
            .attr("role", "group")
            .attr("aria-label", cx.label("Colour scheme"))
            .behavior("ui-theme-toggle", json!({}));
        cx.open(&group, node)?;
        // The server can't know the stored choice; the behaviour corrects `aria-pressed`.
        for (scheme, default) in SCHEMES {
            let mut button = Element::new("button");
            button
                .base_style_ref(&TOGGLE)
                .base_style_ref(variant)
                .base_style_ref(size)
                .attr("type", "button")
                .attr("data-scheme", scheme)
                .attr("aria-pressed", if scheme == "system" { "true" } else { "false" })
                .text(lit(field(&labels, scheme).unwrap_or_else(|| cx.label(default))));
            cx.open(&button, node)?;
            cx.close("button");
        }
        cx.close("div");
        Ok(())
    }
}

pub struct ThemeOption;

const THEME_OPTION_SCRIPT: &str = include_str!("theme_option.js");
const THEME_OPTION_PROPS: &[PropDef] = &[
    PropDef::new("option", PropKind::String, "The theme option it picks, from `theme.options`, e.g. `palette`").required(),
    PropDef::new("bind", PropKind::StateKey, "State key that gets the choice, e.g. `rose`, to show it or use it on the page; setting it picks that choice"),
    PropDef::new("swatch", PropKind::Any, "A token each choice is shown with as a colour dot, e.g. `color.primary`, or a list of them"),
    PropDef::new("label", PropKind::Any, "The group's name for screen readers (default: the option's label)"),
    PropDef::new("variant", PropKind::Enum(TOGGLE_VARIANTS), "Default `outline`"),
    PropDef::new("size", PropKind::Enum(TOGGLE_SIZES), "Default `sm`"),
];

/// The swatch of a `custom` choice: any colour.
static RAINBOW: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[("background", "conic-gradient(#ef4444, #eab308, #22c55e, #06b6d4, #3b82f6, #a855f7, #ef4444)")])
});

/// Several swatches overlap a little, like a fanned palette.
static SWATCHES: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[("display", "inline-flex"), ("flex-shrink", "0")])
        .nest("& > * + *", StyleBlock::from_pairs(&[("margin-inline-start", "-4px")]))
});
static SWATCH: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("width", "12px"),
        ("height", "12px"),
        ("flex-shrink", "0"),
        ("border-radius", "999px"),
        ("box-shadow", "inset 0 0 0 1px color-mix(in srgb, currentColor 20%, transparent)"),
    ])
});

impl Component for ThemeOption {
    fn name(&self) -> &'static str {
        "ui-theme-option"
    }
    fn info(&self) -> ComponentInfo {
        info(
            "Picks one choice of a theme option (`theme.options`), such as a palette or a corner radius, for every page, \
             and remembers it in the browser.",
            THEME_OPTION_PROPS,
            &[],
            false,
        )
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let name = cx.prop_string(node, "option").unwrap_or_default();
        let Some(option) = cx.theme().options().get(&name).cloned() else {
            let known = cx.theme().options().keys().map(|k| format!("`{k}`")).collect::<Vec<_>>().join(", ");
            let known = if known.is_empty() { "none".into() } else { known };
            return Err(RenderError::new(node, format!("`{name}` is not a theme option (options: {known})")));
        };
        let variant = &TOGGLE_VARIANT_STYLES[cx.prop_string(node, "variant").map_or(1, |_| choice(cx, node, "variant", TOGGLE_VARIANTS))];
        let size = &TOGGLE_SIZE_STYLES[cx.prop_string(node, "size").map_or(1, |_| choice(cx, node, "size", TOGGLE_SIZES))];
        // One token or several: a palette's primary, or a chart scheme's first colours.
        let swatch: Vec<String> = match cx.prop(node, "swatch") {
            Some(Json::Array(list)) => list.iter().map(expr::stringify).collect(),
            Some(Json::String(t)) => vec![t],
            _ => Vec::new(),
        };
        cx.global_js("ui-theme-option", THEME_OPTION_SCRIPT);
        let mut group = cx.element("div", node);
        group
            .base_style_ref(&TOGGLE_GROUP)
            .attr("role", "group")
            .attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(option.label.clone())))
            .behavior("ui-theme-option", json!({ "option": name, "fallback": option.default, "bind": cx.prop_string(node, "bind") }));
        cx.open(&group, node)?;
        // The server can't know the stored choice; the behaviour corrects `aria-pressed`.
        for (key, c) in &option.choices {
            let mut button = Element::new("button");
            button
                .base_style_ref(&TOGGLE)
                .base_style_ref(variant)
                .base_style_ref(size)
                .attr("type", "button")
                .attr("data-choice", key.as_str())
                .attr("aria-pressed", if *key == option.default { "true" } else { "false" });
            cx.open(&button, node)?;
            // A choice that doesn't set a token shows the base value (the default's).
            let colors: Vec<String> = swatch
                .iter()
                .filter_map(|t| {
                    c.tokens.get(t).cloned().or_else(|| (*key == option.default).then(|| cx.theme().tokens().get(t).cloned()).flatten())
                })
                .collect();
            if colors.is_empty() && key == "custom" && !swatch.is_empty() {
                let mut dot = Element::new("span");
                dot.base_style_ref(&SWATCH).base_style_ref(&RAINBOW).attr("aria-hidden", "true");
                cx.open(&dot, node)?;
                cx.close("span");
            } else if !colors.is_empty() {
                let mut dots = Element::new("span");
                dots.base_style_ref(&SWATCHES).attr("aria-hidden", "true");
                cx.open(&dots, node)?;
                for color in colors {
                    let mut dot = Element::new("span");
                    let mut fill = StyleBlock::new();
                    if let Some(v) = cx.style_value(node, "background", &lit(color))? {
                        fill.push("background", v);
                    }
                    dot.base_style_ref(&SWATCH).base_style(fill);
                    cx.open(&dot, node)?;
                    cx.close("span");
                }
                cx.close("span");
            }
            cx.text(&c.label);
            cx.close("button");
        }
        cx.close("div");
        Ok(())
    }
}

pub struct ThemeColors;

const THEME_COLORS_SCRIPT: &str = include_str!("theme_colors.js");
const THEME_COLORS_PROPS: &[PropDef] = &[
    PropDef::new("option", PropKind::String, "The option whose `custom` choice the colours make (default `palette`)"),
    PropDef::new("gradients", PropKind::Bool, "Gradient ends for the primary and second colour (default true)"),
    PropDef::new("backgrounds", PropKind::Bool, "Background colours for the page, cards, popovers, dialogs, sheets and the sidebar, per mode (default true)"),
];

/// A custom colour's field.
struct CustomColor {
    /// The name it's stored under.
    name: &'static str,
    label: &'static str,
    /// The tokens the field starts from, the first the theme has; none for a gradient
    /// end, which starts empty.
    tokens: &'static [&'static str],
    /// The colour the kit shows for it, which the behaviour reads back in the browser.
    probe: &'static str,
}

const fn color(name: &'static str, label: &'static str, tokens: &'static [&'static str], probe: &'static str) -> CustomColor {
    CustomColor { name, label, tokens, probe }
}

const BRAND_COLORS: [CustomColor; 2] = [
    color("primary", "Primary", &["color.primary"], "var(--color-primary)"),
    color("brand-2", "Second colour", &["color.brand-2", "color.primary"], "var(--color-brand-2, var(--color-primary))"),
];
/// The gradient end after each brand colour: `primary-to`, `brand-2-to`.
const GRADIENT_ENDS: [CustomColor; 2] =
    [color("primary-to", "Primary gradient end", &[], ""), color("brand-2-to", "Second gradient end", &[], "")];
const BACKGROUND_COLORS: [CustomColor; 6] = [
    color("background", "Page", &["color.background", "color.surface"], crate::BG_PAGE),
    color("card", "Cards", &["color.card", "color.surface"], crate::BG_CARD),
    color("popover", "Popovers and menus", &["color.popover", "color.surface"], crate::BG_POPOVER),
    color("dialog", "Dialogs", &["color.dialog", "color.surface"], crate::BG_DIALOG),
    color("sheet", "Sheets", &["color.sheet", "color.dialog", "color.surface"], crate::BG_SHEET),
    color("sidebar", "Sidebar", &["color.sidebar", "color.surface"], crate::sidebar::SIDEBAR_BG),
];

static COLORS_HEADING: LazyLock<StyleBlock> =
    LazyLock::new(|| StyleBlock::from_pairs(&[("margin-top", gap!(8)), ("color", MUTED), ("font-size", fs!(13))]));
static COLORS_ROW: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[("display", "grid"), ("grid-template-columns", "1fr auto 7.5em"), ("align-items", "center"), ("gap", gap!(8)), ("font-size", fs!(14))])
});
static COLOR_INPUT: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("width", ctl!(32)),
        ("height", ctl!(32)),
        ("padding", "2px"),
        ("border", "var(--border-width, 1px) solid var(--color-border)"),
        ("border-radius", "var(--radius-control, var(--radius-md))"),
        ("background", "transparent"),
        ("cursor", "pointer"),
    ])
});
static HEX_INPUT: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("box-sizing", "border-box"),
        ("width", "100%"),
        ("height", ctl!(32)),
        ("padding", pad!(0, 8)),
        ("border", "var(--border-width, 1px) solid var(--color-border)"),
        ("border-radius", "var(--radius-control, var(--radius-md))"),
        ("background", "transparent"),
        ("color", "inherit"),
        ("font-family", crate::MONO),
        ("font-size", fs!(13)),
    ])
    .nest("&[aria-invalid=true]", StyleBlock::from_pairs(&[("border-color", "var(--color-destructive, #dc2626)")]))
});

/// `#rgb` as `#rrggbb`, the only form a colour input takes; `None` for anything else.
fn hex6(v: &str) -> Option<String> {
    let h = v.trim().strip_prefix('#')?;
    if !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    match h.len() {
        6 => Some(format!("#{}", h.to_ascii_lowercase())),
        3 => Some(format!("#{}", h.chars().flat_map(|c| [c, c]).collect::<String>().to_ascii_lowercase())),
        _ => None,
    }
}

/// One custom colour: its label, a colour picker and a hex field. Until something is
/// stored (the behaviour fills that in), the theme's colour; a gradient end starts
/// empty, its picker on `base`'s colour.
fn color_row(cx: &mut RenderCx<'_>, node: &Node, c: &CustomColor, base: Option<&CustomColor>, per_mode: bool) -> Result<(), RenderError> {
    let theme_hex = |tokens: &[&str]| tokens.iter().find_map(|t| cx.theme().tokens().get(*t)).and_then(|v| hex6(v));
    let start = theme_hex(c.tokens);
    let pick_start = start.clone().or_else(|| base.and_then(|b| theme_hex(b.tokens))).unwrap_or_else(|| "#000000".into());
    let mut row = Element::new("label");
    row.base_style_ref(&COLORS_ROW).attr("data-color", c.name);
    if !c.probe.is_empty() {
        row.attr("data-probe", c.probe);
    }
    if per_mode {
        row.attr("data-per-mode", true);
    }
    cx.open(&row, node)?;
    cx.text(c.label);
    let mut pick = Element::new("input");
    pick.base_style_ref(&COLOR_INPUT).attr("type", "color").attr("value", pick_start.as_str()).attr("aria-label", format!("{}, picked", c.label));
    cx.open(&pick, node)?;
    let mut text = Element::new("input");
    text.base_style_ref(&HEX_INPUT)
        .attr("type", "text")
        .attr("value", start.unwrap_or_default().as_str())
        .attr("aria-label", format!("{}, as hex", c.label))
        .attr("spellcheck", "false")
        .attr("maxlength", "7")
        .attr("placeholder", if base.is_some() { "none" } else { "#rrggbb" });
    cx.open(&text, node)?;
    cx.close("label");
    Ok(())
}

impl Component for ThemeColors {
    fn name(&self) -> &'static str {
        "ui-theme-colors"
    }
    fn info(&self) -> ComponentInfo {
        info(
            "Colour inputs, typed as hex or picked: a primary and a second brand colour, each with an optional \
             gradient end, and the backgrounds of the page, cards, popovers, dialogs, sheets and the sidebar for \
             the mode showing. Changing one picks the option's `custom` choice; the colours are remembered with the \
             other theme choices.",
            THEME_COLORS_PROPS,
            &[],
            false,
        )
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let option = cx.prop_string(node, "option").unwrap_or_else(|| "palette".into());
        if !cx.theme().options().get(&option).is_some_and(|o| o.choices.contains_key("custom")) {
            return Err(RenderError::new(node, format!("`{option}` has no `custom` choice to apply the colours to")));
        }
        cx.global_js("ui-theme-colors", THEME_COLORS_SCRIPT);
        let mut el = cx.element("div", node);
        el.base_style_ref(&PANEL).behavior("ui-theme-colors", json!({ "option": option }));
        cx.open(&el, node)?;
        let on = |prop: &str| cx.prop(node, prop).is_none_or(|v| expr::truthy(&v));
        let (gradients, backgrounds) = (on("gradients"), on("backgrounds"));
        for (i, c) in BRAND_COLORS.iter().enumerate() {
            color_row(cx, node, c, None, false)?;
            if gradients {
                // A gradient end's picker opens on its colour.
                color_row(cx, node, &GRADIENT_ENDS[i], Some(c), false)?;
            }
        }
        if backgrounds {
            let mut heading = Element::new("div");
            heading.base_style_ref(&COLORS_HEADING);
            cx.open(&heading, node)?;
            let t = format!("{}, ", cx.label("Backgrounds"));
            cx.text(&t);
            let mut mode = Element::new("span");
            mode.attr("data-mode", true).text(lit(cx.label("light mode")));
            cx.open(&mode, node)?;
            cx.close("span");
            cx.close("div");
            for c in &BACKGROUND_COLORS {
                color_row(cx, node, c, None, true)?;
            }
        }
        let mut reset = Element::new("button");
        reset
            .base_style_ref(&TOGGLE)
            .base_style_ref(&TOGGLE_VARIANT_STYLES[0])
            .base_style_ref(&TOGGLE_SIZE_STYLES[1])
            .attr("type", "button")
            .attr("data-reset", true)
            .text(lit(cx.label("Reset colours")));
        cx.open(&reset, node)?;
        cx.close("button");
        cx.close("div");
        Ok(())
    }
}

pub struct ThemePanel;

const THEME_PANEL_PROPS: &[PropDef] = &[
    PropDef::new("options", PropKind::Any, "The options to show, in order, e.g. `[palette, radius]` (default: every theme option, the palette first and chart colours last)"),
    PropDef::new("mode", PropKind::Bool, "Show the light / dark mode too (default true)"),
];

static PANEL: LazyLock<StyleBlock> = LazyLock::new(|| StyleBlock::from_pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(16))]));
static PANEL_ROW: LazyLock<StyleBlock> = LazyLock::new(|| StyleBlock::from_pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(8))]));
static PANEL_LABEL: LazyLock<StyleBlock> =
    LazyLock::new(|| StyleBlock::from_pairs(&[("font-size", fs!(14)), ("font-weight", "var(--font-weight-medium, 500)"), ("line-height", "1")]));

/// The token a picker shows as a swatch: the primary for a palette, the first series
/// colour for chart colours, or the first colour a choice sets.
fn swatch_for(option: &irender_core::theme::ThemeOption) -> Option<String> {
    let set: Vec<&String> = option.choices.values().flat_map(|c| c.tokens.keys()).collect();
    ["color.primary", "color.chart-1"]
        .iter()
        .find(|t| set.iter().any(|k| k == *t))
        .map(|t| t.to_string())
        .or_else(|| set.into_iter().find(|k| k.starts_with("color.")).cloned())
}

impl Component for ThemePanel {
    fn name(&self) -> &'static str {
        "ui-theme-panel"
    }
    fn info(&self) -> ComponentInfo {
        info(
            "A labelled `ui-theme-option` for each theme option (the kits' and the document's), and the mode. \
             Put it in a `ui-popover` for a Theme button.",
            THEME_PANEL_PROPS,
            &[],
            false,
        )
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let all = cx.theme().options().clone();
        let names: Vec<String> = match cx.prop(node, "options") {
            Some(Json::Array(list)) => list.iter().map(expr::stringify).collect(),
            // The palette first and chart colours last, whatever order the kits' themes
            // were merged in; the rest as the theme has them.
            _ => {
                let mut keys: Vec<String> = all.keys().cloned().collect();
                keys.sort_by_key(|k| match k.as_str() {
                    "palette" => 0,
                    "chart" => 2,
                    _ => 1,
                });
                keys
            }
        };
        let mut el = cx.element("div", node);
        el.base_style_ref(&PANEL);
        cx.open(&el, node)?;
        let row = |cx: &mut RenderCx<'_>, label: &str, picker: Node| -> Result<(), RenderError> {
            let mut r = Element::new("div");
            r.base_style_ref(&PANEL_ROW);
            cx.open(&r, node)?;
            let mut l = Element::new("div");
            l.base_style_ref(&PANEL_LABEL).attr("aria-hidden", "true").text(lit(label));
            cx.open(&l, node)?;
            cx.close("div");
            cx.render(&picker)?;
            cx.close("div");
            Ok(())
        };
        for name in names {
            let Some(option) = all.get(&name) else {
                return Err(RenderError::new(node, format!("`{name}` is not a theme option")));
            };
            let mut props = vec![("option", lit(name.clone()))];
            if let Some(t) = swatch_for(option) {
                // Chart schemes show their first four colours; anything else one.
                let shown = if t == "color.chart-1" { json!(["color.chart-1", "color.chart-2", "color.chart-3", "color.chart-4"]) } else { json!(t) };
                props.push(("swatch", lit(shown)));
            }
            let mut picker = synthetic(node, "ui-theme-option", props, Vec::new(), Vec::new());
            picker.style = StyleBlock::from_pairs(&[("flex-wrap", "wrap")]);
            row(cx, &option.label, picker)?;
            if name == "palette" && option.choices.contains_key("custom") {
                row(cx, "Custom colours", synthetic(node, "ui-theme-colors", Vec::new(), Vec::new(), Vec::new()))?;
            }
        }
        if cx.prop(node, "mode").is_none_or(|m| expr::truthy(&m)) {
            row(cx, "Mode", synthetic(node, "ui-theme-toggle", Vec::new(), Vec::new(), Vec::new()))?;
        }
        cx.close("div");
        Ok(())
    }
}

/// Gives a synthetic node the document's own `attrs`, `class`, `style` and `on`
/// for the element it stands in for. The document's attributes win; its actions
/// run after the kit's.
pub(crate) fn merge_node(synthetic: &mut Node, node: &Node) {
    for (k, v) in &node.attrs {
        synthetic.attrs.insert(k.clone(), v.clone());
    }
    synthetic.class = node.class.clone();
    synthetic.style = node.style.clone();
    for (event, actions) in &node.on {
        synthetic.on.entry(event.clone()).or_default().extend(actions.iter().cloned());
    }
}
