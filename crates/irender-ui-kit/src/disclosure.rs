//! Components that show or switch content: accordion and collapsible (native
//! `<details>`), tabs, toggle and toggle group (state and actions), and the theme
//! toggle (a client behaviour).

use crate::{
    ACCENT, MUTED, SUBTLE, SURFACE, TEXT, choice, context, expr_val, field, id_with, instance_id, lit, server_array,
    state, state_eq, state_not, synthetic,
};
use irender_core::{
    Action, ComponentInfo, Node, PropDef, PropKind, StyleBlock, Val,
    expr::{self, BinOp, Expr, UnOp},
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
                ("gap", "16px"),
                ("list-style", "none"),
                ("cursor", "pointer"),
                ("font-weight", "500"),
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
    summary_rules(StyleBlock::from_pairs(&[("border-bottom", "1px solid var(--color-border)")]))
        .nest("& > summary", StyleBlock::from_pairs(&[("padding", "16px 0"), ("font-size", "14px")]))
        .nest("& > summary:hover", StyleBlock::from_pairs(&[("text-decoration", "underline")]))
});
static DISCLOSURE_CONTENT: LazyLock<StyleBlock> =
    LazyLock::new(|| StyleBlock::from_pairs(&[("padding-bottom", "16px"), ("font-size", "14px")]));

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
            StyleBlock::from_pairs(&[("padding", "8px 12px"), ("border-radius", "var(--radius-md)"), ("font-size", "14px")]),
        )
        .nest("& > summary:hover", StyleBlock::from_pairs(&[("background", ACCENT)]))
        .nest("& > div", StyleBlock::from_pairs(&[("padding", "8px 12px 0")]))
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
    PropDef::new("id", PropKind::Any, "Element id (default: derived from the document)"),
];
const TAB_PANEL_PROPS: &[PropDef] = &[
    PropDef::new("value", PropKind::Any, "What `bind` holds when this tab is open").required(),
    PropDef::new("label", PropKind::Any, "Tab text (default: the value)"),
];

static TABS: LazyLock<StyleBlock> =
    LazyLock::new(|| StyleBlock::from_pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", "8px")]));
static TAB_LIST: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("display", "inline-flex"),
        ("align-items", "center"),
        ("gap", "2px"),
        ("width", "fit-content"),
        ("padding", "3px"),
        ("border-radius", "var(--radius-lg)"),
        ("background", SUBTLE),
        ("color", MUTED),
    ])
});
static TAB: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("display", "inline-flex"),
        ("align-items", "center"),
        ("justify-content", "center"),
        ("height", "30px"),
        ("padding", "0 12px"),
        ("border", "1px solid transparent"),
        ("border-radius", "var(--radius-md)"),
        ("background", "transparent"),
        ("color", "inherit"),
        ("font-family", "inherit"),
        ("font-size", "14px"),
        ("font-weight", "500"),
        ("white-space", "nowrap"),
        ("cursor", "pointer"),
        ("transition", "background-color .15s, color .15s"),
    ])
    .nest(":hover", StyleBlock::from_pairs(&[("color", TEXT)]))
    .nest(":focus-visible", focus_outline())
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

        let mut el = cx.element("div", node);
        el.base_style_ref(&TABS);
        cx.open(&el, node)?;
        let mut list = cx.part("div", node, "list");
        list.base_style_ref(&TAB_LIST).attr("role", "tablist");
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
    PropDef::new("bind", PropKind::StateKey, "State key holding the pressed value, or null").required(),
    PropDef::new("options", PropKind::Any, "Array of values or `{ value, label, disabled }`; fixed on the server").required(),
    PropDef::new("variant", PropKind::Enum(TOGGLE_VARIANTS), "Default `default`"),
    PropDef::new("size", PropKind::Enum(TOGGLE_SIZES), "Default `default`"),
    PropDef::new("disabled", PropKind::Any, "Disable every option"),
];

static TOGGLE: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("display", "inline-flex"),
        ("align-items", "center"),
        ("justify-content", "center"),
        ("gap", "8px"),
        ("border", "1px solid transparent"),
        ("border-radius", "var(--radius-md)"),
        ("background", "transparent"),
        ("color", TEXT),
        ("font-family", "inherit"),
        ("font-size", "14px"),
        ("font-weight", "500"),
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
        StyleBlock::from_pairs(&[("border-color", "var(--color-border)"), ("box-shadow", "0 1px 2px rgb(0 0 0 / 0.05)")]),
    ]
});
static TOGGLE_SIZE_STYLES: LazyLock<[StyleBlock; 3]> = LazyLock::new(|| {
    [
        StyleBlock::from_pairs(&[("height", "36px"), ("min-width", "36px"), ("padding", "0 10px")]),
        StyleBlock::from_pairs(&[("height", "32px"), ("min-width", "32px"), ("padding", "0 8px")]),
        StyleBlock::from_pairs(&[("height", "40px"), ("min-width", "40px"), ("padding", "0 12px")]),
    ]
});
static TOGGLE_GROUP: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[("display", "inline-flex"), ("align-items", "center"), ("gap", "4px"), ("width", "fit-content")])
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
        info("A row of toggles where one at most is pressed; pressing it again clears it.", TOGGLE_GROUP_PROPS, &[], false)
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let Some(key) = cx.prop_string(node, "bind") else {
            return Err(RenderError::new(node, "`ui-toggle-group` needs `bind`"));
        };
        let options = server_array(cx, node, "options")?;
        let variant = &TOGGLE_VARIANT_STYLES[choice(cx, node, "variant", TOGGLE_VARIANTS)];
        let size = &TOGGLE_SIZE_STYLES[choice(cx, node, "size", TOGGLE_SIZES)];
        let disabled_all = cx.prop(node, "disabled").is_some_and(|d| expr::truthy(&d));
        let mut group = cx.element("div", node);
        group.base_style_ref(&TOGGLE_GROUP).attr("role", "group");
        cx.open(&group, node)?;
        for o in &options {
            let (value, label) = match o {
                Json::Object(m) => {
                    let v = m.get("value").cloned().unwrap_or(Json::Null);
                    let l = m.get("label").filter(|l| !l.is_null()).cloned().unwrap_or_else(|| v.clone());
                    (v, l)
                }
                other => (other.clone(), other.clone()),
            };
            // Pressing the pressed option clears the choice.
            let next = expr_val(Expr::Cond(
                Box::new(Expr::Binary(BinOp::Eq, Box::new(state(&key)), Box::new(Expr::Lit(value.clone())))),
                Box::new(Expr::Lit(Json::Null)),
                Box::new(Expr::Lit(value.clone())),
            ));
            let item = synthetic(
                node,
                "button",
                Vec::new(),
                vec![("type", lit("button")), ("aria-pressed", state_eq(&key, value.clone()))],
                vec![("click", vec![Action::SetState { key: key.clone(), value: next }])],
            );
            let mut el = cx.element("button", &item);
            el.base_style_ref(&TOGGLE).base_style_ref(variant).base_style_ref(size).text(Val::Lit(json!(expr::stringify(&label))));
            if disabled_all || o.get("disabled").is_some_and(expr::truthy) {
                el.attr("disabled", true);
            }
            cx.open(&el, &item)?;
            cx.close("button");
        }
        cx.close("div");
        Ok(())
    }
}

pub struct ThemeToggle;

const THEME_EARLY_SCRIPT: &str = include_str!("theme_early.js");
const THEME_TOGGLE_SCRIPT: &str = include_str!("theme_toggle.js");
const THEME_TOGGLE_PROPS: &[PropDef] = &[
    PropDef::new("labels", PropKind::Any, "Button text as `{ system, light, dark }`; default System, Light, Dark"),
    PropDef::new("variant", PropKind::Enum(TOGGLE_VARIANTS), "Default `outline`"),
    PropDef::new("size", PropKind::Enum(TOGGLE_SIZES), "Default `sm`"),
];
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
            .attr("aria-label", "Colour scheme")
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
                .text(lit(field(&labels, scheme).unwrap_or_else(|| default.into())));
            cx.open(&button, node)?;
            cx.close("button");
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
