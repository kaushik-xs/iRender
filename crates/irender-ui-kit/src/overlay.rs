//! Overlays built on the platform: `<dialog>` opened with invoker commands
//! (`command="show-modal"`), the Popover API placed with CSS anchor positioning,
//! and CSS-only tooltips and hover cards. Only the dropdown menu adds JavaScript, for
//! its arrow keys (menu.js).

use crate::{
    BG_DIALOG, BG_POPOVER, BG_SHEET, ACCENT, DESTRUCTIVE, MUTED, ON_PRIMARY, PRIMARY, TEXT, choice, context, describe, disclosure::merge_node,
    forms, id_after, id_with, instance_id, lit, synthetic,
};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, StyleValue, Val, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

pub(crate) const MENU_SCRIPT: &str = include_str!("menu.js");

const BUTTON_VARIANTS: PropKind = PropKind::Enum(&["default", "secondary", "destructive", "outline", "ghost", "link", "brand"]);
const BUTTON_SIZES: PropKind = PropKind::Enum(&["default", "sm", "lg", "icon"]);

/// A value as a style value: literal as is, template evaluated live.
fn style_of(v: &Val) -> StyleValue {
    match v {
        Val::Lit(j) => StyleValue::Static(expr::stringify(j)),
        Val::Tpl(t) => StyleValue::Dynamic { tpl: t.clone(), px: false },
    }
}

/// The kit's button, opening or closing something through `attrs`.
pub(crate) fn trigger_button(node: &Node, cx: &mut RenderCx<'_>, attrs: Vec<(&str, Val)>, style: StyleBlock) -> Result<(), RenderError> {
    let Some(label) = node.prop("trigger") else { return Ok(()) };
    let variant = node.prop("triggerVariant").cloned().unwrap_or_else(|| lit("outline"));
    let mut props = vec![("label", label.clone()), ("variant", variant)];
    if let Some(size) = node.prop("triggerSize") {
        props.push(("size", size.clone()));
    }
    let mut b = synthetic(node, "ui-button", props, attrs, Vec::new());
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
    Drawer,
}

pub struct Dialog {
    name: &'static str,
    kind: Kind,
}

impl Dialog {
    pub const DIALOG: Dialog = Dialog { name: "ui-dialog", kind: Kind::Dialog };
    pub const ALERT: Dialog = Dialog { name: "ui-alert-dialog", kind: Kind::Alert };
    pub const SHEET: Dialog = Dialog { name: "ui-sheet", kind: Kind::Sheet };
    pub const DRAWER: Dialog = Dialog { name: "ui-drawer", kind: Kind::Drawer };
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

const DRAWER_PROPS: &[PropDef] = &[
    PropDef::new("trigger", PropKind::Any, "Text of a button that opens it"),
    PropDef::new("triggerVariant", BUTTON_VARIANTS, "The trigger button's variant (default `outline`)"),
    PropDef::new("title", PropKind::Any, "Heading"),
    PropDef::new("description", PropKind::Any, "Text under the heading"),
    PropDef::new("width", PropKind::Any, "Its widest, centred on a wide screen (default: the whole width)"),
    PropDef::new("id", PropKind::Any, "Element id (default: derived from the document)"),
];

const DRAWER_SCRIPT: &str = include_str!("drawer.js");

fn dialog_base() -> StyleBlock {
    StyleBlock::from_pairs(&[
        ("box-sizing", "border-box"),
        ("padding", sp!(24)),
        ("border", "1px solid var(--color-border)"),
        ("--ui-bg", BG_DIALOG),
        ("background", BG_DIALOG),
        ("color", TEXT),
        ("font-family", "inherit"),
        ("box-shadow", "0 10px 30px rgb(0 0 0 / 0.2)"),
        ("overflow", "auto"),
    ])
    .nest("&[open]", StyleBlock::from_pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", sp!(16))]))
    .nest("::backdrop", StyleBlock::from_pairs(&[("background", "rgb(0 0 0 / 0.5)")]))
}

static DIALOG: LazyLock<StyleBlock> = LazyLock::new(|| {
    dialog_base()
        .set("width", "min(512px, calc(100% - 32px))")
        .set("max-height", "calc(100dvh - 32px)")
        .set("border-radius", "var(--radius-lg)")
});
pub(crate) static SHEETS: LazyLock<[StyleBlock; 4]> = LazyLock::new(|| {
    let side = |pairs: &[(&str, &str)]| {
        let mut b = dialog_base().set("--ui-bg", BG_SHEET).set("background", BG_SHEET).set("border-radius", "0").set("max-width", "none").set("max-height", "none");
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
/// The drawer: a sheet from the bottom with rounded top corners and a grab bar, that
/// slides up and can be dragged down to close.
static DRAWER: LazyLock<StyleBlock> = LazyLock::new(|| {
    dialog_base()
        .set("--ui-bg", BG_SHEET)
        .set("background", BG_SHEET)
        .set("width", "min(100%, var(--drawer-width, 100%))")
        .set("max-width", "none")
        .set("max-height", "85dvh")
        .set("margin", "auto auto 0")
        .set("padding-top", sp!(28))
        .set("border-width", "1px 1px 0")
        .set("border-radius", "var(--radius-xl, 16px) var(--radius-xl, 16px) 0 0")
        .set("overscroll-behavior", "contain")
        .set("transform", "translateY(100%)")
        .set("transition", "transform .3s cubic-bezier(.32, .72, 0, 1), overlay .3s allow-discrete, display .3s allow-discrete")
        .nest("&[open]", StyleBlock::from_pairs(&[("transform", "none")]))
        .nest("@starting-style", StyleBlock::new().nest("&[open]", StyleBlock::from_pairs(&[("transform", "translateY(100%)")])))
        .nest("&[data-dragging]", StyleBlock::from_pairs(&[("transition", "none"), ("user-select", "none")]))
        .nest("::backdrop", StyleBlock::from_pairs(&[("transition", "opacity .3s, overlay .3s allow-discrete, display .3s allow-discrete"), ("opacity", "0")]))
        .nest("&[open]::backdrop", StyleBlock::from_pairs(&[("opacity", "1")]))
        .nest("@starting-style", StyleBlock::new().nest("&[open]::backdrop", StyleBlock::from_pairs(&[("opacity", "0")])))
        .nest("@media (prefers-reduced-motion: reduce)", StyleBlock::new().nest("&, &::backdrop", StyleBlock::from_pairs(&[("transition", "none")])))
        // The grab bar, and the strip around it that takes the drag.
        .nest(
            "& [data-drawer-grip]",
            StyleBlock::from_pairs(&[("position", "absolute"), ("top", "0"), ("left", "0"), ("right", "0"), ("height", "28px"), ("cursor", "grab"), ("touch-action", "none")]),
        )
        .nest(
            "& [data-drawer-grip]::after",
            StyleBlock::from_pairs(&[
                ("content", "''"),
                ("position", "absolute"),
                ("top", "10px"),
                ("left", "50%"),
                ("width", "40px"),
                ("height", "5px"),
                ("margin-left", "-20px"),
                ("border-radius", "999px"),
                ("background", "color-mix(in srgb, var(--color-text) 18%, transparent)"),
            ]),
        )
        .nest("&[data-dragging] [data-drawer-grip]", StyleBlock::from_pairs(&[("cursor", "grabbing")]))
});

pub(crate) static DIALOG_HEADER: LazyLock<StyleBlock> =
    LazyLock::new(|| StyleBlock::from_pairs(&[("display", "grid"), ("gap", sp!(6)), ("padding-right", sp!(24))]));
pub(crate) static DIALOG_TITLE: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[("margin", "0"), ("font-size", fs!(18)), ("font-weight", "600"), ("line-height", "1.2")])
});
static DIALOG_DESCRIPTION: LazyLock<StyleBlock> =
    LazyLock::new(|| StyleBlock::from_pairs(&[("margin", "0"), ("font-size", fs!(14)), ("color", MUTED)]));
pub(crate) static DIALOG_X: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("position", "absolute"),
        ("top", "14px"),
        ("right", "14px"),
        ("width", "28px"),
        ("height", sp!(28)),
        ("border", "0"),
        ("border-radius", "var(--radius-sm)"),
        ("background", "transparent"),
        ("color", MUTED),
        ("font-size", fs!(16)),
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
            Kind::Drawer => ("A panel that slides up from the bottom, and that a drag down (or Escape, or the ✕) closes.", DRAWER_PROPS),
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
            Kind::Drawer => &*DRAWER,
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
        if self.kind == Kind::Drawer {
            cx.global_js("ui-drawer", DRAWER_SCRIPT);
            el.behavior("ui-drawer", json!({}));
            if let Some(w) = cx.prop(node, "width") {
                el.base_style(StyleBlock::new().set("--drawer-width", if let Json::Number(n) = &w { format!("{n}px") } else { expr::stringify(&w) }));
            }
        }
        if title.is_some() {
            el.attr("aria-labelledby", id_with(&id, "-title"));
        }
        if description.is_some() {
            el.attr("aria-describedby", id_with(&id, "-description"));
        }
        cx.open(&el, node)?;
        if self.kind == Kind::Drawer {
            // Decorative: Escape and the ✕ do the same for the keyboard.
            let mut grip = Element::new("div");
            grip.attr("data-drawer-grip", true).attr("aria-hidden", "true");
            cx.open(&grip, node)?;
            cx.close("div");
        }
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
        // Close buttons inside target the dialog's id.
        cx.with_context("ui-dialog", Some(id.clone()), |cx| cx.children(node))?;
        if self.kind != Kind::Alert {
            let mut x = Element::new("button");
            x.base_style_ref(&DIALOG_X)
                .attr("type", "button")
                .attr("command", "close")
                .attr("commandfor", id.clone())
                .attr("aria-label", cx.label("Close"))
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
        let Some(dialog) = context(cx, "ui-dialog") else {
            return Err(RenderError::new(node, "`ui-dialog-close` must be inside a `ui-dialog`, `ui-alert-dialog` or `ui-sheet`"));
        };
        let mut props = vec![("label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Close"))))];
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
        ("gap", sp!(8)),
        ("margin-top", sp!(8)),
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
    PropDef::new("triggerSize", BUTTON_SIZES, "The trigger button's size (default `default`)"),
    PropDef::new("side", PropKind::Enum(POP_SIDES), "Where it opens, next to the trigger (default `bottom`)"),
    PropDef::new("align", PropKind::Enum(ALIGNS), "Alignment along that side (default `center`; menus `start`)"),
    PropDef::new("id", PropKind::Any, "Element id (default: derived from the document)"),
];

pub(crate) fn popover_base() -> StyleBlock {
    StyleBlock::from_pairs(&[
        ("box-sizing", "border-box"),
        // The UA centres popovers with `inset: 0; margin: auto`; anchoring needs neither.
        ("inset", "auto"),
        ("margin", "0"),
        ("border", "1px solid var(--color-border)"),
        ("border-radius", "var(--radius-md)"),
        ("--ui-bg", BG_POPOVER),
        ("background", BG_POPOVER),
        ("color", TEXT),
        ("font-family", "inherit"),
        ("box-shadow", "0 6px 18px rgb(0 0 0 / 0.12)"),
    ])
}

static POPOVER: LazyLock<StyleBlock> =
    LazyLock::new(|| popover_base().set("width", "288px").set("padding", sp!(16)).set("font-size", fs!(14)));
pub(crate) static MENU: LazyLock<StyleBlock> = LazyLock::new(|| {
    popover_base()
        .set("min-width", sp!(176))
        .set("padding", sp!(4))
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
        if self.menu {
            // Items are reached with the arrow keys, so Tab leaves the menu.
            cx.global_js("ui-dropdown-menu", MENU_SCRIPT);
            el.attr("role", "menu").behavior("ui-dropdown-menu", json!({}));
        }
        cx.open(&el, node)?;
        let menu = self.menu.then(|| lit(true));
        cx.with_context("ui-popover", Some(id), |cx| cx.with_context("ui-menu", menu, |cx| cx.children(node)))?;
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- context menu

/// A menu opened on its area with a right-click, Shift+F10 or the menu key, at the
/// pointer. Its `ui-menu-*` children are the menu; the rest is the area. The menu is a
/// `ui-dropdown-menu`'s, keyboard included.
pub struct ContextMenu;

const CONTEXT_SCRIPT: &str = include_str!("context_menu.js");

const CONTEXT_PROPS: &[PropDef] = &[
    PropDef::new(
        "bind",
        PropKind::StateKey,
        "State key that gets what was right-clicked: the nearest `data-context` value (JSON or text), or a data table's row",
    ),
    PropDef::new("label", PropKind::Any, "The menu's accessible name (default `Actions`)"),
    PropDef::new("disabled", PropKind::Any, "Leave the browser's own menu in place, e.g. `{{ !editing }}`"),
    PropDef::new("id", PropKind::Any, "The menu's element id (default: derived from the document)"),
];

/// The children that make up the menu rather than the area.
const MENU_PARTS: &[&str] = &["ui-menu-item", "ui-menu-label", "ui-menu-separator"];

impl Component for ContextMenu {
    fn name(&self) -> &'static str {
        "ui-context-menu"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A menu opened with a right-click (or Shift+F10, or the menu key) on its area, at the pointer. \
                  `ui-menu-item`, `ui-menu-label` and `ui-menu-separator` children are the menu; any other children are the area.",
            props: CONTEXT_PROPS,
            parts: &["menu"],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let id = instance_id(cx, node, "context-menu");
        cx.global_js("ui-context-menu", CONTEXT_SCRIPT);
        cx.global_js("ui-dropdown-menu", MENU_SCRIPT);
        let mut root = cx.element("div", node);
        root.behavior("ui-context-menu", json!({ "bind": cx.prop_string(node, "bind") }));
        if let Some(d) = node.prop("disabled") {
            root.attr("data-disabled", d.clone());
        }
        cx.open(&root, node)?;
        for child in node.children.iter().filter(|c| !MENU_PARTS.contains(&c.kind.as_str())) {
            cx.render(child)?;
        }
        let mut menu = cx.part("div", node, "menu");
        menu.base_style_ref(&MENU)
            .attr("popover", "auto")
            .attr("id", id.clone())
            .attr("role", "menu")
            .attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Actions"))))
            .behavior("ui-dropdown-menu", json!({}));
        cx.open(&menu, node)?;
        cx.with_context("ui-popover", Some(id), |cx| {
            cx.with_context("ui-menu", Some(lit(true)), |cx| {
                for child in node.children.iter().filter(|c| MENU_PARTS.contains(&c.kind.as_str())) {
                    cx.render(child)?;
                }
                Ok(())
            })
        })?;
        cx.close("div");
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- split button

/// A primary action joined to a menu of secondary ones: a `ui-button` with the node's
/// own `on`, and a `ui-dropdown-menu` on an arrow next to it holding the children.
pub struct SplitButton;

const SPLIT_PROPS: &[PropDef] = &[
    PropDef::new("label", PropKind::Any, "The primary action's text").required(),
    PropDef::new("variant", BUTTON_VARIANTS, "Both buttons' variant (default `default`)"),
    PropDef::new("size", BUTTON_SIZES, "Both buttons' size (default `default`; not `icon`)"),
    PropDef::new("disabled", PropKind::Any, "Disable the primary action (the menu still opens)"),
    PropDef::new("menuLabel", PropKind::Any, "The arrow's name for screen readers (default `More actions`)"),
    PropDef::new("side", PropKind::Enum(POP_SIDES), "Where the menu opens (default `bottom`)"),
    PropDef::new("align", PropKind::Enum(ALIGNS), "The menu's alignment (default `end`: its right edge under the arrow)"),
    PropDef::new("id", PropKind::Any, "The menu's element id (default: derived from the document)"),
];

static SPLIT: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[("display", "inline-flex"), ("align-items", "stretch"), ("width", "fit-content")])
        // The buttons meet: square inner corners, one border between them, drawn as a
        // faint line so filled buttons show where one ends.
        .nest("& > button:first-of-type", StyleBlock::from_pairs(&[("border-start-end-radius", "0"), ("border-end-end-radius", "0")]))
        .nest(
            "& > button:last-of-type",
            StyleBlock::from_pairs(&[
                ("border-start-start-radius", "0"),
                ("border-end-start-radius", "0"),
                ("margin-inline-start", "-1px"),
                ("padding", sp!(0, 10)),
                ("font-size", fs!(16)),
                ("border-inline-start-color", "color-mix(in srgb, currentColor 25%, transparent)"),
            ]),
        )
        .nest("& > button:focus-visible", StyleBlock::from_pairs(&[("position", "relative"), ("z-index", "1")]))
});

impl Component for SplitButton {
    fn name(&self) -> &'static str {
        "ui-split-button"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A primary action with an arrow that opens a menu of secondary ones. Its `on.click` is the \
                  primary action; children are the menu's `ui-menu-item`, `ui-menu-label` and `ui-menu-separator`.",
            props: SPLIT_PROPS,
            parts: &["primary", "trigger"],
            children: true,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let variant = node.prop("variant").cloned().unwrap_or_else(|| lit("default"));
        let mut primary = synthetic(node, "ui-button", vec![("label", node.prop("label").cloned().unwrap_or_else(|| lit(""))), ("variant", variant.clone())], Vec::new(), Vec::new());
        primary.on = node.on.clone();
        for name in ["size", "disabled"] {
            if let Some(v) = node.prop(name) {
                primary.props.insert(name.into(), v.clone());
            }
        }
        if let Some(p) = node.parts.get("primary") {
            primary.attrs.extend(p.attrs.clone());
            primary.class = p.class.clone();
            primary.style = p.style.clone();
        }

        let label = node.prop("menuLabel").cloned().unwrap_or_else(|| lit(cx.label("More actions")));
        let mut props = vec![("trigger", lit("▾")), ("triggerVariant", variant), ("align", lit("end"))];
        for name in ["side", "align", "id"] {
            if let Some(v) = node.prop(name) {
                props.retain(|(k, _)| *k != name);
                props.push((name, v.clone()));
            }
        }
        if let Some(size) = node.prop("size") {
            props.push(("triggerSize", size.clone()));
        }
        let mut menu = synthetic(node, "ui-dropdown-menu", props, Vec::new(), Vec::new());
        let mut trigger = node.parts.get("trigger").cloned().unwrap_or_default();
        trigger.attrs.entry("aria-label".into()).or_insert(label);
        menu.parts.insert("trigger".into(), trigger);
        menu.children = node.children.clone();

        // The wrapper takes the node's attrs, class and style; its `on` is the primary
        // button's.
        let outer = Node {
            kind: node.kind.clone(),
            path: node.path.clone(),
            props: Default::default(),
            attrs: node.attrs.clone(),
            class: node.class.clone(),
            style: node.style.clone(),
            parts: Default::default(),
            on: Default::default(),
            children: Vec::new(),
        };
        let mut el = cx.element("div", &outer);
        el.base_style_ref(&SPLIT).attr("role", "group");
        cx.open(&el, &outer)?;
        forms::Button.render(&primary, cx)?;
        Popover::MENU.render(&menu, cx)?;
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

pub(crate) static MENU_ITEM: LazyLock<[StyleBlock; 2]> = LazyLock::new(|| {
    let base = |color: &str| {
        StyleBlock::from_pairs(&[
            ("display", "flex"),
            ("align-items", "center"),
            ("gap", sp!(8)),
            ("width", "100%"),
            ("padding", sp!(6, 8)),
            ("border", "0"),
            ("border-radius", "var(--radius-sm)"),
            ("background", "transparent"),
            ("color", color),
            ("font-family", "inherit"),
            ("font-size", fs!(14)),
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
pub(crate) static SHORTCUT: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[("margin-left", "auto"), ("padding-left", sp!(16)), ("font-size", fs!(12)), ("letter-spacing", "0.1em"), ("color", MUTED)])
});
static MENU_SEPARATOR: LazyLock<StyleBlock> =
    LazyLock::new(|| StyleBlock::from_pairs(&[("height", "1px"), ("margin", sp!(4, -4)), ("background", "var(--color-border)")]));
static MENU_LABEL: LazyLock<StyleBlock> =
    LazyLock::new(|| StyleBlock::from_pairs(&[("padding", sp!(6, 8)), ("font-size", fs!(14)), ("font-weight", "600")]));

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
        if context(cx, "ui-menu").is_some() {
            el.attr("role", "menuitem").attr("tabindex", "-1");
        }
        match href {
            Some(h) => {
                el.attr("href", h.clone());
            }
            None => {
                el.attr("type", "button");
                if let Some(menu) = context(cx, "ui-popover") {
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
    PropDef::new("id", PropKind::Any, "The tip's element id (default: derived from the document)"),
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
            ("padding", sp!(6, 10)),
            ("border-radius", "var(--radius-md)"),
            ("background", PRIMARY),
            ("color", ON_PRIMARY),
            ("font-size", fs!(12)),
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
            doc: "A short tip shown when its children are hovered or focused. It describes the first child for \
                  screen readers.",
            props: TOOLTIP_PROPS,
            parts: &["tip"],
            children: true,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        // The first child (usually a button) is described by the tip, so screen readers
        // read the tip with it.
        let id = instance_id(cx, node, "tooltip");
        let mut children = node.children.clone();
        if let Some(first) = children.first_mut().filter(|c| c.kind != "text") {
            describe(first, std::slice::from_ref(&id));
        }
        let mut el = cx.element("span", node);
        el.base_style_ref(&TIP_WRAP);
        cx.open(&el, node)?;
        children.iter().try_for_each(|c| cx.render(c))?;
        let mut tip = cx.part("span", node, "tip");
        tip.base_style_ref(&TIPS[choice(cx, node, "side", TIP_SIDES)]).attr("role", "tooltip").attr("id", id);
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
        ("padding", sp!(16)),
        ("border", "1px solid var(--color-border)"),
        ("border-radius", "var(--radius-lg)"),
        ("--ui-bg", BG_POPOVER),
        ("background", BG_POPOVER),
        ("color", TEXT),
        ("font-size", fs!(14)),
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
