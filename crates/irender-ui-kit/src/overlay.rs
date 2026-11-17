//! Overlays built on the platform: `<dialog>` opened with invoker commands
//! (`command="show-modal"`), the Popover API placed with CSS anchor positioning,
//! and CSS-only tooltips and hover cards. None of them adds JavaScript.

use crate::{
    ACCENT, DESTRUCTIVE, MUTED, ON_PRIMARY, PRIMARY, SURFACE, TEXT, choice, context, disclosure::merge_node, forms,
    id_after, id_with, instance_id, lit, synthetic,
};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, StyleValue, Val, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::json;
use std::sync::LazyLock;

const BUTTON_VARIANTS: PropKind = PropKind::Enum(&["default", "secondary", "destructive", "outline", "ghost", "link"]);

/// A value as a style value: literal as is, template evaluated live.
fn style_of(v: &Val) -> StyleValue {
    match v {
        Val::Lit(j) => StyleValue::Static(expr::stringify(j)),
        Val::Tpl(t) => StyleValue::Dynamic { tpl: t.clone(), px: false },
    }
}

/// The kit's button, opening or closing something through `attrs`.
fn trigger_button(node: &Node, cx: &mut RenderCx<'_>, attrs: Vec<(&str, Val)>, style: StyleBlock) -> Result<(), RenderError> {
    let Some(label) = node.prop("trigger") else { return Ok(()) };
    let variant = node.prop("triggerVariant").cloned().unwrap_or_else(|| lit("outline"));
    let mut b = synthetic(node, "ui-button", vec![("label", label.clone()), ("variant", variant)], attrs, Vec::new());
    if let Some(p) = node.parts.get("trigger") {
        b.attrs.extend(p.attrs.clone());
        b.class = p.class.clone();
        b.style = p.style.clone();
    }
    for decl in style.decls {
        b.style.decls.push(decl);
    }
    forms::Button.render(&b, cx)
}

fn focus_ring() -> StyleBlock {
    StyleBlock::from_pairs(&[("outline", "2px solid var(--color-ring, var(--color-muted))"), ("outline-offset", "2px")])
}

// ---------------------------------------------------------------- dialog, alert dialog, sheet

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Dialog,
    Alert,
    Sheet,
}

pub struct Dialog {
    name: &'static str,
    kind: Kind,
}

impl Dialog {
    pub const DIALOG: Dialog = Dialog { name: "ui-dialog", kind: Kind::Dialog };
    pub const ALERT: Dialog = Dialog { name: "ui-alert-dialog", kind: Kind::Alert };
    pub const SHEET: Dialog = Dialog { name: "ui-sheet", kind: Kind::Sheet };
}

const DIALOG_PROPS: &[PropDef] = &[
    PropDef::new("trigger", PropKind::Any, "Text of a button that opens it (or open it from your own button, see README)"),
    PropDef::new("triggerVariant", BUTTON_VARIANTS, "The trigger button's variant (default `outline`)"),
    PropDef::new("title", PropKind::Any, "Heading"),
    PropDef::new("description", PropKind::Any, "Text under the heading"),
    PropDef::new("id", PropKind::Any, "Element id (default: derived from the document)"),
];
const SIDES: &[&str] = &["right", "left", "top", "bottom"];
const SHEET_PROPS: &[PropDef] = &[
    PropDef::new("trigger", PropKind::Any, "Text of a button that opens it"),
    PropDef::new("triggerVariant", BUTTON_VARIANTS, "The trigger button's variant (default `outline`)"),
    PropDef::new("title", PropKind::Any, "Heading"),
    PropDef::new("description", PropKind::Any, "Text under the heading"),
    PropDef::new("side", PropKind::Enum(SIDES), "Edge it slides from (default `right`)"),
    PropDef::new("id", PropKind::Any, "Element id (default: derived from the document)"),
];

fn dialog_base() -> StyleBlock {
    StyleBlock::from_pairs(&[
        ("box-sizing", "border-box"),
        ("padding", "24px"),
        ("border", "1px solid var(--color-border)"),
        ("background", SURFACE),
        ("color", TEXT),
        ("font-family", "inherit"),
        ("box-shadow", "0 10px 30px rgb(0 0 0 / 0.2)"),
        ("overflow", "auto"),
    ])
    .nest("&[open]", StyleBlock::from_pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", "16px")]))
    .nest("::backdrop", StyleBlock::from_pairs(&[("background", "rgb(0 0 0 / 0.5)")]))
}

static DIALOG: LazyLock<StyleBlock> = LazyLock::new(|| {
    dialog_base()
        .set("width", "min(512px, calc(100% - 32px))")
        .set("max-height", "calc(100dvh - 32px)")
        .set("border-radius", "var(--radius-lg)")
});
static SHEETS: LazyLock<[StyleBlock; 4]> = LazyLock::new(|| {
    let side = |pairs: &[(&str, &str)]| {
        let mut b = dialog_base().set("border-radius", "0").set("max-width", "none").set("max-height", "none");
        for (k, v) in pairs {
            b = b.set(k, *v);
        }
        b
    };
    let tall = [("height", "100dvh"), ("width", "min(384px, 85vw)")];
    [
        side(&[tall[0], tall[1], ("margin", "0 0 0 auto"), ("border-width", "0 0 0 1px")]),
        side(&[tall[0], tall[1], ("margin", "0 auto 0 0"), ("border-width", "0 1px 0 0")]),
        side(&[("width", "100%"), ("margin", "0 0 auto"), ("border-width", "0 0 1px")]),
        side(&[("width", "100%"), ("margin", "auto 0 0"), ("border-width", "1px 0 0")]),
    ]
});
static DIALOG_HEADER: LazyLock<StyleBlock> =
    LazyLock::new(|| StyleBlock::from_pairs(&[("display", "grid"), ("gap", "6px"), ("padding-right", "24px")]));
static DIALOG_TITLE: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[("margin", "0"), ("font-size", "18px"), ("font-weight", "600"), ("line-height", "1.2")])
});
static DIALOG_DESCRIPTION: LazyLock<StyleBlock> =
    LazyLock::new(|| StyleBlock::from_pairs(&[("margin", "0"), ("font-size", "14px"), ("color", MUTED)]));
static DIALOG_X: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("position", "absolute"),
        ("top", "14px"),
        ("right", "14px"),
        ("width", "28px"),
        ("height", "28px"),
        ("border", "0"),
        ("border-radius", "var(--radius-sm)"),
        ("background", "transparent"),
        ("color", MUTED),
        ("font-size", "16px"),
        ("cursor", "pointer"),
    ])
    .nest(":hover", StyleBlock::from_pairs(&[("background", ACCENT), ("color", TEXT)]))
    .nest(":focus-visible", focus_ring())
});

impl Component for Dialog {
    fn name(&self) -> &'static str {
        self.name
    }

    fn info(&self) -> ComponentInfo {
        let (doc, props) = match self.kind {
            Kind::Dialog => ("A modal window. Escape, the ✕ or a click outside closes it.", DIALOG_PROPS),
            Kind::Alert => ("A modal that needs an answer: only its own buttons close it.", DIALOG_PROPS),
            Kind::Sheet => ("A modal panel along one edge of the screen.", SHEET_PROPS),
        };
        ComponentInfo { doc, props, parts: &["trigger", "header", "title", "description"], children: true, open_props: false }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let id = instance_id(cx, node, &self.name["ui-".len()..]);
        trigger_button(
            node,
            cx,
            vec![("command", lit("show-modal")), ("commandfor", id.clone()), ("aria-haspopup", lit("dialog"))],
            StyleBlock::new(),
        )?;

        let base = match self.kind {
            Kind::Sheet => &SHEETS[choice(cx, node, "side", SIDES)],
            _ => &*DIALOG,
        };
        let (title, description) = (node.prop("title"), node.prop("description"));
        let mut el = cx.element("dialog", node);
        el.base_style_ref(base).attr("id", id.clone());
        if self.kind == Kind::Alert {
            el.attr("role", "alertdialog").attr("closedby", "none");
        } else {
            el.attr("closedby", "any");
        }
        if title.is_some() {
            el.attr("aria-labelledby", id_with(&id, "-title"));
        }
        if description.is_some() {
            el.attr("aria-describedby", id_with(&id, "-description"));
        }
        cx.open(&el, node)?;
        if title.is_some() || description.is_some() {
            let mut header = cx.part("div", node, "header");
            header.base_style_ref(&DIALOG_HEADER);
            cx.open(&header, node)?;
            if let Some(t) = title {
                let mut h = cx.part("h2", node, "title");
                h.base_style_ref(&DIALOG_TITLE).attr("id", id_with(&id, "-title")).text(t);
                cx.open(&h, node)?;
                cx.close("h2");
            }
            if let Some(d) = description {
                let mut p = cx.part("p", node, "description");
                p.base_style_ref(&DIALOG_DESCRIPTION).attr("id", id_with(&id, "-description")).text(d);
                cx.open(&p, node)?;
                cx.close("p");
            }
            cx.close("div");
        }
        // Close buttons inside read the dialog's id from scope.
        let scoped = expr::stringify(&cx.eval(&id));
        cx.with_scope(vec![("ui_dialog".into(), json!(scoped))], |cx| cx.children(node))?;
        if self.kind != Kind::Alert {
            let mut x = Element::new("button");
            x.base_style_ref(&DIALOG_X)
                .attr("type", "button")
                .attr("command", "close")
                .attr("commandfor", id.clone())
                .attr("aria-label", "Close")
                .text(lit("✕"));
            cx.open(&x, node)?;
            cx.close("button");
        }
        cx.close("dialog");
        Ok(())
    }
}

pub struct DialogClose;

const DIALOG_CLOSE_PROPS: &[PropDef] = &[
    PropDef::new("label", PropKind::Any, "Button text").required(),
    PropDef::new("variant", BUTTON_VARIANTS, "Default `default`"),
];

impl Component for DialogClose {
    fn name(&self) -> &'static str {
        "ui-dialog-close"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A button that closes the dialog, alert dialog or sheet it is in. Its own `on.click` actions run too.",
            props: DIALOG_CLOSE_PROPS,
            parts: &[],
            children: true,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let Some(dialog) = context(cx, "ui_dialog") else {
            return Err(RenderError::new(node, "`ui-dialog-close` must be inside a `ui-dialog`, `ui-alert-dialog` or `ui-sheet`"));
        };
        let mut props = vec![("label", node.prop("label").cloned().unwrap_or_else(|| lit("Close")))];
        if let Some(v) = node.prop("variant") {
            props.push(("variant", v.clone()));
        }
        let mut b = synthetic(node, "ui-button", props, vec![("command", lit("close")), ("commandfor", dialog)], Vec::new());
        merge_node(&mut b, node);
        b.children = node.children.clone();
        forms::Button.render(&b, cx)
    }
}

pub struct DialogFooter;

static DIALOG_FOOTER: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("display", "flex"),
        ("flex-wrap", "wrap"),
        ("justify-content", "flex-end"),
        ("gap", "8px"),
        ("margin-top", "8px"),
    ])
});

impl Component for DialogFooter {
    fn name(&self) -> &'static str {
        "ui-dialog-footer"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "A row of buttons at the bottom of a dialog.", props: &[], parts: &[], children: true, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut el = cx.element("div", node);
        el.base_style_ref(&DIALOG_FOOTER);
        cx.open(&el, node)?;
        cx.children(node)?;
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- popover and dropdown menu

pub struct Popover {
    name: &'static str,
    menu: bool,
}

impl Popover {
    pub const POPOVER: Popover = Popover { name: "ui-popover", menu: false };
    pub const MENU: Popover = Popover { name: "ui-dropdown-menu", menu: true };
}

const POP_SIDES: &[&str] = &["bottom", "top", "right", "left"];
const ALIGNS: &[&str] = &["center", "start", "end"];
const POPOVER_PROPS: &[PropDef] = &[
    PropDef::new("trigger", PropKind::Any, "Text of the button that opens it").required(),
    PropDef::new("triggerVariant", BUTTON_VARIANTS, "The trigger button's variant (default `outline`)"),
    PropDef::new("side", PropKind::Enum(POP_SIDES), "Where it opens, next to the trigger (default `bottom`)"),
    PropDef::new("align", PropKind::Enum(ALIGNS), "Alignment along that side (default `center`; menus `start`)"),
    PropDef::new("id", PropKind::Any, "Element id (default: derived from the document)"),
];

fn popover_base() -> StyleBlock {
    StyleBlock::from_pairs(&[
        ("box-sizing", "border-box"),
        // The UA centres popovers with `inset: 0; margin: auto`; anchoring needs neither.
        ("inset", "auto"),
        ("margin", "0"),
        ("border", "1px solid var(--color-border)"),
        ("border-radius", "var(--radius-md)"),
        ("background", SURFACE),
        ("color", TEXT),
        ("font-family", "inherit"),
        ("box-shadow", "0 6px 18px rgb(0 0 0 / 0.12)"),
    ])
}

static POPOVER: LazyLock<StyleBlock> =
    LazyLock::new(|| popover_base().set("width", "288px").set("padding", "16px").set("font-size", "14px"));
static MENU: LazyLock<StyleBlock> = LazyLock::new(|| {
    popover_base()
        .set("min-width", "176px")
        .set("padding", "4px")
        .nest(":popover-open", StyleBlock::from_pairs(&[("display", "flex"), ("flex-direction", "column")]))
});

/// `position-area` for a side and alignment: `bottom`, `bottom span-right`, …
fn position_area(side: &str, align: &str) -> String {
    let span = match (side, align) {
        (_, "center") => return side.to_string(),
        ("top" | "bottom", "start") => "span-right",
        ("top" | "bottom", _) => "span-left",
        (_, "start") => "span-bottom",
        _ => "span-top",
    };
    format!("{side} {span}")
}

impl Component for Popover {
    fn name(&self) -> &'static str {
        self.name
    }

    fn info(&self) -> ComponentInfo {
        let doc = if self.menu {
            "A button that opens a list of actions. Children are `ui-menu-item`, `ui-menu-label` and `ui-menu-separator`."
        } else {
            "A button that opens floating content next to it. A click outside or Escape closes it."
        };
        ComponentInfo { doc, props: POPOVER_PROPS, parts: &["trigger"], children: true, open_props: false }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let id = instance_id(cx, node, &self.name["ui-".len()..]);
        let anchor = id_after("--", &id);
        let mut anchor_style = StyleBlock::new();
        anchor_style.push("anchor-name", style_of(&anchor));
        let haspopup = if self.menu { "menu" } else { "dialog" };
        trigger_button(node, cx, vec![("popovertarget", id.clone()), ("aria-haspopup", lit(haspopup))], anchor_style)?;

        let side = POP_SIDES[choice(cx, node, "side", POP_SIDES)];
        let default_align = if self.menu { "start" } else { "center" };
        let align = cx.prop_string(node, "align").filter(|a| ALIGNS.contains(&a.as_str())).unwrap_or(default_align.into());
        let mut placement = StyleBlock::new();
        placement.push("position-anchor", style_of(&anchor));
        // The gap to the trigger goes on the axis facing it only, so alignment stays exact.
        let vertical = matches!(side, "top" | "bottom");
        placement = placement
            .set("position-area", position_area(side, &align))
            .set("position-try-fallbacks", if vertical { "flip-block" } else { "flip-inline" })
            .set("margin", if vertical { "6px 0" } else { "0 6px" });

        let mut el = cx.element("div", node);
        el.base_style_ref(if self.menu { &MENU } else { &POPOVER }).base_style(placement);
        el.attr("popover", "auto").attr("id", id.clone());
        cx.open(&el, node)?;
        let scoped = expr::stringify(&cx.eval(&id));
        cx.with_scope(vec![("ui_popover".into(), json!(scoped))], |cx| cx.children(node))?;
        cx.close("div");
        Ok(())
    }
}

pub struct MenuItem;
pub struct MenuSeparator;
pub struct MenuLabel;

const MENU_ITEM_PROPS: &[PropDef] = &[
    PropDef::new("label", PropKind::Any, "Item text").required(),
    PropDef::new("shortcut", PropKind::Any, "Keyboard hint shown on the right, e.g. `⌘S`"),
    PropDef::new("variant", PropKind::Enum(&["default", "destructive"]), "Default `default`"),
    PropDef::new("href", PropKind::Any, "Make it a link"),
    PropDef::new("disabled", PropKind::Any, "Boolean or expression"),
];

static MENU_ITEM: LazyLock<[StyleBlock; 2]> = LazyLock::new(|| {
    let base = |color: &str| {
        StyleBlock::from_pairs(&[
            ("display", "flex"),
            ("align-items", "center"),
            ("gap", "8px"),
            ("width", "100%"),
            ("padding", "6px 8px"),
            ("border", "0"),
            ("border-radius", "var(--radius-sm)"),
            ("background", "transparent"),
            ("color", color),
            ("font-family", "inherit"),
            ("font-size", "14px"),
            ("text-align", "left"),
            ("text-decoration", "none"),
            ("cursor", "pointer"),
        ])
        .nest(":hover", StyleBlock::from_pairs(&[("background", ACCENT)]))
        .nest(":focus-visible", StyleBlock::from_pairs(&[("background", ACCENT), ("outline", "none")]))
        .nest(":disabled", StyleBlock::from_pairs(&[("opacity", "0.5"), ("pointer-events", "none")]))
    };
    [base(TEXT), base(DESTRUCTIVE)]
});
static SHORTCUT: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[("margin-left", "auto"), ("padding-left", "16px"), ("font-size", "12px"), ("letter-spacing", "0.1em"), ("color", MUTED)])
});
static MENU_SEPARATOR: LazyLock<StyleBlock> =
    LazyLock::new(|| StyleBlock::from_pairs(&[("height", "1px"), ("margin", "4px -4px"), ("background", "var(--color-border)")]));
static MENU_LABEL: LazyLock<StyleBlock> =
    LazyLock::new(|| StyleBlock::from_pairs(&[("padding", "6px 8px"), ("font-size", "14px"), ("font-weight", "600")]));

impl Component for MenuItem {
    fn name(&self) -> &'static str {
        "ui-menu-item"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "An action in a `ui-dropdown-menu`. Choosing it runs its `on.click` and closes the menu.",
            props: MENU_ITEM_PROPS,
            parts: &["shortcut"],
            children: false,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let href = node.prop("href");
        let tag = if href.is_some() { "a" } else { "button" };
        let mut el = cx.element(tag, node);
        el.base_style_ref(&MENU_ITEM[choice(cx, node, "variant", &["default", "destructive"])]);
        match href {
            Some(h) => {
                el.attr("href", h.clone());
            }
            None => {
                el.attr("type", "button");
                if let Some(menu) = context(cx, "ui_popover") {
                    el.attr("popovertarget", menu).attr("popovertargetaction", "hide");
                }
                if let Some(d) = node.prop("disabled") {
                    el.attr("disabled", d.clone());
                }
            }
        }
        cx.open(&el, node)?;
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
        cx.close(tag);
        Ok(())
    }
}

impl Component for MenuSeparator {
    fn name(&self) -> &'static str {
        "ui-menu-separator"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "A line between groups of menu items.", props: &[], parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut el = cx.element("div", node);
        el.base_style_ref(&MENU_SEPARATOR).attr("role", "separator");
        cx.open(&el, node)?;
        cx.close("div");
        Ok(())
    }
}

impl Component for MenuLabel {
    fn name(&self) -> &'static str {
        "ui-menu-label"
    }
    fn info(&self) -> ComponentInfo {
        const PROPS: &[PropDef] = &[PropDef::new("text", PropKind::Any, "Heading for the items below").required()];
        ComponentInfo { doc: "A heading inside a menu.", props: PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut el = cx.element("div", node);
        el.base_style_ref(&MENU_LABEL);
        if let Some(t) = node.prop("text") {
            el.text(t);
        }
        cx.open(&el, node)?;
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- tooltip and hover card

pub struct Tooltip;

const TIP_SIDES: &[&str] = &["top", "bottom", "left", "right"];
const TOOLTIP_PROPS: &[PropDef] = &[
    PropDef::new("text", PropKind::Any, "The tip").required(),
    PropDef::new("side", PropKind::Enum(TIP_SIDES), "Default `top`"),
];

static TIP_WRAP: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[("position", "relative"), ("display", "inline-flex")])
        .nest("&:hover > [role=tooltip]", StyleBlock::from_pairs(&[("opacity", "1"), ("transition-delay", "0.3s")]))
        .nest("&:focus-within > [role=tooltip]", StyleBlock::from_pairs(&[("opacity", "1")]))
});
static TIPS: LazyLock<[StyleBlock; 4]> = LazyLock::new(|| {
    let base = |pairs: &[(&str, &str)]| {
        let mut b = StyleBlock::from_pairs(&[
            ("position", "absolute"),
            ("z-index", "50"),
            ("padding", "6px 10px"),
            ("border-radius", "var(--radius-md)"),
            ("background", PRIMARY),
            ("color", ON_PRIMARY),
            ("font-size", "12px"),
            ("line-height", "1.2"),
            ("white-space", "nowrap"),
            ("opacity", "0"),
            ("pointer-events", "none"),
            ("transition", "opacity .15s"),
        ]);
        for (k, v) in pairs {
            b = b.set(k, *v);
        }
        b
    };
    [
        base(&[("bottom", "calc(100% + 6px)"), ("left", "50%"), ("transform", "translateX(-50%)")]),
        base(&[("top", "calc(100% + 6px)"), ("left", "50%"), ("transform", "translateX(-50%)")]),
        base(&[("right", "calc(100% + 6px)"), ("top", "50%"), ("transform", "translateY(-50%)")]),
        base(&[("left", "calc(100% + 6px)"), ("top", "50%"), ("transform", "translateY(-50%)")]),
    ]
});

impl Component for Tooltip {
    fn name(&self) -> &'static str {
        "ui-tooltip"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A short tip shown when its children are hovered or focused.",
            props: TOOLTIP_PROPS,
            parts: &["tip"],
            children: true,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut el = cx.element("span", node);
        el.base_style_ref(&TIP_WRAP);
        cx.open(&el, node)?;
        cx.children(node)?;
        let mut tip = cx.part("span", node, "tip");
        tip.base_style_ref(&TIPS[choice(cx, node, "side", TIP_SIDES)]).attr("role", "tooltip");
        if let Some(t) = node.prop("text") {
            tip.text(t);
        }
        cx.open(&tip, node)?;
        cx.close("span");
        cx.close("span");
        Ok(())
    }
}

pub struct HoverCard;

const HOVER_CARD_PROPS: &[PropDef] = &[
    PropDef::new("trigger", PropKind::Any, "Text that shows the card when hovered or focused").required(),
    PropDef::new("href", PropKind::Any, "Make the trigger a link"),
];

static CARD_WRAP: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[("position", "relative"), ("display", "inline-block")])
        .nest(
            "&:hover > [data-ui-card]",
            StyleBlock::from_pairs(&[("opacity", "1"), ("visibility", "visible"), ("transition-delay", "0.4s")]),
        )
        .nest("&:focus-within > [data-ui-card]", StyleBlock::from_pairs(&[("opacity", "1"), ("visibility", "visible")]))
});
static CARD_TRIGGER: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("color", "inherit"),
        ("font-weight", "500"),
        ("text-decoration", "underline"),
        ("text-underline-offset", "4px"),
        ("cursor", "pointer"),
    ])
    .nest(":focus-visible", focus_ring())
});
static CARD_POP: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("box-sizing", "border-box"),
        ("position", "absolute"),
        ("z-index", "50"),
        ("top", "calc(100% + 8px)"),
        ("left", "0"),
        ("width", "320px"),
        ("padding", "16px"),
        ("border", "1px solid var(--color-border)"),
        ("border-radius", "var(--radius-lg)"),
        ("background", SURFACE),
        ("color", TEXT),
        ("font-size", "14px"),
        ("box-shadow", "0 6px 18px rgb(0 0 0 / 0.12)"),
        ("opacity", "0"),
        ("visibility", "hidden"),
        ("transition", "opacity .15s, visibility .15s"),
    ])
});

impl Component for HoverCard {
    fn name(&self) -> &'static str {
        "ui-hover-card"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A preview card shown when its trigger is hovered or focused. Children are the card.",
            props: HOVER_CARD_PROPS,
            parts: &["trigger", "card"],
            children: true,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut el = cx.element("span", node);
        el.base_style_ref(&CARD_WRAP);
        cx.open(&el, node)?;
        let href = node.prop("href");
        let tag = if href.is_some() { "a" } else { "span" };
        let mut t = cx.part(tag, node, "trigger");
        t.base_style_ref(&CARD_TRIGGER);
        match href {
            Some(h) => t.attr("href", h.clone()),
            None => t.attr("tabindex", lit(0)),
        };
        if let Some(label) = node.prop("trigger") {
            t.text(label);
        }
        cx.open(&t, node)?;
        cx.close(tag);
        let mut card = cx.part("div", node, "card");
        card.base_style_ref(&CARD_POP).attr("data-ui-card", true);
        cx.open(&card, node)?;
        cx.children(node)?;
        cx.close("div");
        cx.close("span");
        Ok(())
    }
}
