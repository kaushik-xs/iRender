//! Buttons and form controls. The inputs reuse the built-ins' state binding and
//! only change how they look. Button groups, input groups and fields arrange them.

use crate::{
    ACCENT, CONTROLS, DESTRUCTIVE, MUTED, ON_DESTRUCTIVE, ON_PRIMARY, ON_SECONDARY, PRIMARY, PRIMARY_HOVER, RING,
    SECONDARY, SURFACE, TEXT, bin, choice, describe, each_item, field_or, focus_attr, focus_attrs, id_with, instance_id,
    expr_val, items, lit, path,
};
use indexmap::IndexMap;
use irender_core::{
    ComponentInfo, Node, PropDef, PropKind, StyleBlock, Val,
    expr::{self, BinOp, Expr},
};
use irender_html::{Component, RenderCx, RenderError};
use serde_json::json;
use std::sync::LazyLock;

fn focus_ring() -> StyleBlock {
    StyleBlock::from_pairs(&[
        ("outline", "none"),
        ("box-shadow", "0 0 0 3px color-mix(in srgb, var(--color-ring, var(--color-muted)) 50%, transparent)"),
    ])
}

// ---------------------------------------------------------------- button

pub struct Button;

const BUTTON_VARIANTS: &[&str] = &["default", "secondary", "destructive", "outline", "ghost", "link"];
const BUTTON_SIZES: &[&str] = &["default", "sm", "lg", "icon"];
const BUTTON_PROPS: &[PropDef] = &[
    PropDef::new("label", PropKind::Any, "Button text"),
    PropDef::new("icon", PropKind::Any, "An icon name (with the icon kit) or a character, before the label"),
    PropDef::new("variant", PropKind::Enum(BUTTON_VARIANTS), "Default `default`"),
    PropDef::new("size", PropKind::Enum(BUTTON_SIZES), "Default `default`"),
    PropDef::new("href", PropKind::Any, "Render a link that looks like a button"),
    PropDef::new("type", PropKind::Enum(&["button", "submit", "reset"]), "Default `button`"),
    PropDef::new("disabled", PropKind::Any, "Boolean or expression"),
];

static BUTTON: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("display", "inline-flex"),
        ("align-items", "center"),
        ("justify-content", "center"),
        ("gap", sp!(8)),
        ("flex-shrink", "0"),
        ("white-space", "nowrap"),
        ("border", "1px solid transparent"),
        ("border-radius", "var(--radius-md)"),
        ("font-family", "inherit"),
        ("font-size", fs!(14)),
        ("font-weight", "500"),
        ("line-height", "1"),
        ("text-decoration", "none"),
        ("cursor", "pointer"),
        ("transition", "background-color .15s, color .15s, box-shadow .15s"),
    ])
    .nest(":focus-visible", focus_ring())
    .nest(":disabled", StyleBlock::from_pairs(&[("opacity", "0.5"), ("pointer-events", "none")]))
    .nest("&[aria-disabled=true]", StyleBlock::from_pairs(&[("opacity", "0.5"), ("pointer-events", "none")]))
});

static BUTTON_VARIANT_STYLES: LazyLock<[StyleBlock; 6]> = LazyLock::new(|| {
    let v = |rest: &[(&str, &str)], hover: &[(&str, &str)]| {
        StyleBlock::from_pairs(rest).nest(":hover", StyleBlock::from_pairs(hover))
    };
    [
        v(&[("background", PRIMARY), ("color", ON_PRIMARY)], &[("background", PRIMARY_HOVER)]),
        v(&[("background", SECONDARY), ("color", ON_SECONDARY)], &[("filter", "brightness(0.95)")]),
        v(&[("background", DESTRUCTIVE), ("color", ON_DESTRUCTIVE)], &[("filter", "brightness(0.9)")]),
        v(
            &[("background", SURFACE), ("color", TEXT), ("border-color", "var(--color-border)"),
              ("box-shadow", "0 1px 2px rgb(0 0 0 / 0.05)")],
            &[("background", ACCENT)],
        ),
        v(&[("background", "transparent"), ("color", TEXT)], &[("background", ACCENT)]),
        v(
            &[("background", "transparent"), ("color", PRIMARY), ("text-underline-offset", "4px")],
            &[("text-decoration", "underline")],
        ),
    ]
});

static BUTTON_SIZE_STYLES: LazyLock<[StyleBlock; 4]> = LazyLock::new(|| {
    [
        StyleBlock::from_pairs(&[("height", sp!(36)), ("padding", sp!(0, 16))]),
        StyleBlock::from_pairs(&[("height", sp!(32)), ("padding", sp!(0, 12)), ("gap", sp!(6))]),
        StyleBlock::from_pairs(&[("height", sp!(40)), ("padding", sp!(0, 24))]),
        StyleBlock::from_pairs(&[("height", sp!(36)), ("width", "36px"), ("padding", "0")]),
    ]
});
static LINK_SIZE: LazyLock<StyleBlock> = LazyLock::new(|| StyleBlock::from_pairs(&[("height", "auto"), ("padding", "0")]));

impl Component for Button {
    fn name(&self) -> &'static str {
        "ui-button"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A button in six variants and four sizes. With `href` it is a link that looks like a button.",
            props: BUTTON_PROPS,
            parts: &[],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let variant = choice(cx, node, "variant", BUTTON_VARIANTS);
        let size =
            if BUTTON_VARIANTS[variant] == "link" { &*LINK_SIZE } else { &BUTTON_SIZE_STYLES[choice(cx, node, "size", BUTTON_SIZES)] };
        let href = node.prop("href");
        let tag = if href.is_some() { "a" } else { "button" };
        let mut el = cx.element(tag, node);
        el.base_style_ref(&BUTTON).base_style_ref(&BUTTON_VARIANT_STYLES[variant]).base_style_ref(size);
        match href {
            Some(h) => {
                el.attr("href", h.clone());
                if let Some(d) = node.prop("disabled") {
                    el.attr("aria-disabled", d.clone());
                }
            }
            None => {
                el.attr("type", cx.prop_string(node, "type").unwrap_or_else(|| "button".into()));
                if let Some(d) = node.prop("disabled") {
                    el.attr("disabled", d.clone());
                }
            }
        }
        let icon = node.prop("icon");
        if icon.is_none()
            && let Some(label) = node.prop("label")
        {
            el.text(label);
        }
        cx.open(&el, node)?;
        if let Some(i) = icon {
            crate::icon(cx, node, i, 16)?;
            if let Some(label) = node.prop("label") {
                let mut text = irender_html::Element::new("span");
                text.text(label);
                cx.open(&text, node)?;
                cx.close("span");
            }
        }
        cx.children(node)?;
        cx.close(tag);
        Ok(())
    }
}

// ---------------------------------------------------------------- label

pub struct Label;

const LABEL_PROPS: &[PropDef] = &[
    PropDef::new("text", PropKind::Any, "Label text"),
    PropDef::new("for", PropKind::Any, "`id` of the control it labels (or put the control inside)"),
];

static LABEL: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("display", "flex"),
        ("align-items", "center"),
        ("gap", sp!(8)),
        ("font-size", fs!(14)),
        ("font-weight", "500"),
        ("line-height", "1"),
        ("color", TEXT),
        ("user-select", "none"),
    ])
});

impl Component for Label {
    fn name(&self) -> &'static str {
        "ui-label"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "A form label.", props: LABEL_PROPS, parts: &[], children: true, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut el = cx.element("label", node);
        el.base_style_ref(&LABEL);
        if let Some(f) = node.prop("for") {
            el.attr("for", f.clone());
        }
        if let Some(t) = node.prop("text") {
            el.text(t);
        }
        cx.open(&el, node)?;
        cx.children(node)?;
        cx.close("label");
        Ok(())
    }
}

// ---------------------------------------------------------------- text fields

/// Native parts (a select's arrow and menu, a checkbox, a slider's track) follow the
/// page's colour scheme: light, and dark under `@dark`, which `ui-theme-toggle`
/// redirects to the chosen scheme. (`light dark` would follow only the system.)
pub(crate) fn native_scheme(b: StyleBlock) -> StyleBlock {
    b.set("color-scheme", "light").nest("@dark", StyleBlock::from_pairs(&[("color-scheme", "dark")]))
}

fn field_base(height: Option<&str>) -> StyleBlock {
    let mut b = StyleBlock::from_pairs(&[
        // Padding and border count inside `width: 100%` and `height`.
        ("box-sizing", "border-box"),
        ("width", "100%"),
        ("min-width", "0"),
        ("padding", sp!(4, 12)),
        ("border", "1px solid var(--color-border)"),
        ("border-radius", "var(--radius-md)"),
        ("background", "transparent"),
        ("color", TEXT),
        ("font-family", "inherit"),
        ("font-size", fs!(14)),
        ("box-shadow", "0 1px 2px rgb(0 0 0 / 0.05)"),
        ("transition", "border-color .15s, box-shadow .15s"),
    ])
    .nest("::placeholder", StyleBlock::from_pairs(&[("color", MUTED)]))
    .nest(":focus-visible", focus_ring().set("border-color", RING))
    .nest(":disabled", StyleBlock::from_pairs(&[("opacity", "0.5"), ("cursor", "not-allowed")]))
    .nest("&[aria-invalid=true]", StyleBlock::from_pairs(&[("border-color", DESTRUCTIVE)]));
    b = native_scheme(b);
    if let Some(h) = height {
        b = b.set("height", h);
    }
    b
}

static INPUT: LazyLock<StyleBlock> = LazyLock::new(|| field_base(Some("36px")));
static TEXTAREA: LazyLock<StyleBlock> =
    LazyLock::new(|| field_base(None).set("min-height", sp!(64)).set("padding", sp!(8, 12)).set("resize", "vertical"));
static SELECT: LazyLock<StyleBlock> = LazyLock::new(|| field_base(Some("36px")).set("padding", sp!(4, 8)).set("cursor", "pointer"));

pub struct Input;

impl Component for Input {
    fn name(&self) -> &'static str {
        "ui-input"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "A text field. Same props as `input`.", ..irender_html::Input.info() }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        irender_html::Input::render_styled(node, cx, &INPUT)
    }
}

pub struct Textarea;

impl Component for Textarea {
    fn name(&self) -> &'static str {
        "ui-textarea"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "A multi-line text field. Same props as `textarea`.", ..irender_html::Textarea.info() }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        irender_html::Textarea::render_styled(node, cx, &TEXTAREA)
    }
}

pub struct Select;

impl Component for Select {
    fn name(&self) -> &'static str {
        "ui-select"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "A dropdown. Same props as `select`.", ..irender_html::Select.info() }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        irender_html::Select::render_styled(node, cx, &SELECT)
    }
}

// ---------------------------------------------------------------- choice controls

/// An `<input>` node for a checkbox, switch or radio, rendered by the built-in
/// `input` so it binds to state the same way. `parts.control` styles it.
fn control(node: &Node, props: Vec<(&str, Val)>, extra_attrs: &[(&str, Val)]) -> Node {
    let part = node.parts.get("control");
    let mut attrs: IndexMap<String, Val> = part.map(|p| p.attrs.clone()).unwrap_or_default();
    for (k, v) in extra_attrs {
        attrs.entry(k.to_string()).or_insert_with(|| v.clone());
    }
    Node {
        kind: "input".into(),
        path: node.path.clone(),
        props: props.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        attrs,
        class: part.map(|p| p.class.clone()).unwrap_or_default(),
        style: part.map(|p| p.style.clone()).unwrap_or_default(),
        parts: IndexMap::new(),
        on: IndexMap::new(),
        children: Vec::new(),
    }
}

/// The node's props that pass straight through to the inner `<input>`.
fn passthrough(node: &Node, ty: &str) -> Vec<(&'static str, Val)> {
    let mut props = vec![("type", lit(ty))];
    for name in ["bind", "name", "disabled"] {
        if let Some(v) = node.prop(name) {
            props.push((name, v.clone()));
        }
    }
    props
}

static CHOICE_ROW: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("display", "inline-flex"),
        ("align-items", "center"),
        ("gap", sp!(8)),
        ("font-size", fs!(14)),
        ("font-weight", "500"),
        ("line-height", "1.2"),
        ("color", TEXT),
        ("cursor", "pointer"),
    ])
    .nest("&:has(:disabled)", StyleBlock::from_pairs(&[("opacity", "0.6"), ("cursor", "not-allowed")]))
});

static CHECKBOX: LazyLock<StyleBlock> = LazyLock::new(|| {
    native_scheme(StyleBlock::from_pairs(&[
        ("width", "16px"),
        ("height", "16px"),
        ("margin", "0"),
        ("flex-shrink", "0"),
        ("accent-color", PRIMARY),
        ("cursor", "inherit"),
    ])
    .nest(":focus-visible", StyleBlock::from_pairs(&[("outline", "2px solid var(--color-ring, var(--color-muted))"), ("outline-offset", "2px")])))
});

static SWITCH: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("appearance", "none"),
        ("position", "relative"),
        ("flex-shrink", "0"),
        ("width", "36px"),
        ("height", sp!(20)),
        ("margin", "0"),
        ("border", "1px solid transparent"),
        ("border-radius", "999px"),
        ("background", "color-mix(in srgb, var(--color-text) 20%, var(--color-surface))"),
        ("cursor", "inherit"),
        ("transition", "background-color .15s"),
    ])
    .nest(
        "::before",
        StyleBlock::from_pairs(&[
            ("content", "''"),
            ("position", "absolute"),
            ("top", "1px"),
            ("left", "1px"),
            ("width", "16px"),
            ("height", "16px"),
            ("border-radius", "999px"),
            ("background", SURFACE),
            ("box-shadow", "0 1px 2px rgb(0 0 0 / 0.2)"),
            ("transition", "transform .15s"),
        ]),
    )
    .nest(":checked", StyleBlock::from_pairs(&[("background", PRIMARY)]))
    .nest(":checked::before", StyleBlock::from_pairs(&[("transform", "translateX(16px)")]))
    .nest(":focus-visible", focus_ring())
});

const CHECKBOX_PROPS: &[PropDef] = &[
    PropDef::new("label", PropKind::Any, "Text next to the box"),
    PropDef::new("bind", PropKind::StateKey, "State key holding `true` / `false`"),
    PropDef::new("checked", PropKind::Any, "Initially checked, when not bound"),
    PropDef::new("name", PropKind::String, "Form field name"),
    PropDef::new("disabled", PropKind::Any, "Boolean or expression"),
];

/// A checkbox or switch: a `<label>` row holding the control and its text.
fn checkable(node: &Node, cx: &mut RenderCx<'_>, base: &'static StyleBlock, role: Option<&str>) -> Result<(), RenderError> {
    let mut row = cx.element("label", node);
    row.base_style_ref(&CHOICE_ROW);
    cx.open(&row, node)?;
    let mut extra = Vec::new();
    if let Some(r) = role {
        extra.push(("role", lit(r)));
    }
    if node.prop("bind").is_none()
        && let Some(c) = node.prop("checked")
    {
        extra.push(("checked", c.clone()));
    }
    irender_html::Input::render_styled(&control(node, passthrough(node, "checkbox"), &extra), cx, base)?;
    if let Some(label) = node.prop("label") {
        let mut text = cx.part("span", node, "label");
        text.text(label);
        cx.open(&text, node)?;
        cx.close("span");
    }
    cx.close("label");
    Ok(())
}

pub struct Checkbox;

impl Component for Checkbox {
    fn name(&self) -> &'static str {
        "ui-checkbox"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A checkbox with a label.",
            props: CHECKBOX_PROPS,
            parts: &["control", "label"],
            children: false,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        checkable(node, cx, &CHECKBOX, None)
    }
}

pub struct Switch;

impl Component for Switch {
    fn name(&self) -> &'static str {
        "ui-switch"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "An on/off toggle with a label (a checkbox with `role=switch`).",
            props: CHECKBOX_PROPS,
            parts: &["control", "label"],
            children: false,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        checkable(node, cx, &SWITCH, Some("switch"))
    }
}

pub struct RadioGroup;

const RADIO_GROUP_PROPS: &[PropDef] = &[
    PropDef::new("options", PropKind::Any, "Array of values or `{ value, label, disabled }`; may use state").required(),
    PropDef::new("bind", PropKind::StateKey, "State key holding the chosen value"),
    PropDef::new("value", PropKind::Any, "Initially chosen value, when not bound"),
    PropDef::new("name", PropKind::String, "Form field name (default: the `bind` key)"),
    PropDef::new("orientation", PropKind::Enum(&["vertical", "horizontal"]), "Default `vertical`"),
    PropDef::new("disabled", PropKind::Any, "Disable every option"),
];

static RADIO_GROUP: LazyLock<[StyleBlock; 2]> = LazyLock::new(|| {
    [
        StyleBlock::from_pairs(&[("display", "grid"), ("gap", sp!(12))]),
        StyleBlock::from_pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("gap", sp!(16))]),
    ]
});

impl Component for RadioGroup {
    fn name(&self) -> &'static str {
        "ui-radio-group"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A set of radio buttons for one choice.",
            props: RADIO_GROUP_PROPS,
            parts: &["control", "label"],
            children: false,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let options = items(cx, node, "options")?;
        let orientation = choice(cx, node, "orientation", &["vertical", "horizontal"]);
        let chosen = node.prop("bind").is_none().then(|| cx.prop(node, "value")).flatten().map(|v| expr::stringify(&v));
        let disabled_all = cx.prop(node, "disabled").is_some_and(|d| expr::truthy(&d));
        // Radios need one shared name to be exclusive: the prop, the bound key, or one
        // derived from where the group sits in the document.
        let name = node
            .prop("name")
            .or(node.prop("bind"))
            .cloned()
            .unwrap_or_else(|| lit(format!("ui-radio-{}", node.path.replace(|c: char| !c.is_ascii_alphanumeric(), "-"))));

        // Each option, from expressions over it, so the same code renders the options
        // on the server and the template the browser rebuilds them from.
        let value = field_or(OPTION, "value", path(&[OPTION]));
        let as_text = |e: Expr| bin(BinOp::Add, Expr::Lit(json!("")), e);
        let label = expr_val(field_or(OPTION, "label", value.clone()));
        let disabled = if disabled_all { lit(true) } else { expr_val(path(&[OPTION, "disabled"])) };
        let checked = chosen.map(|c| expr_val(bin(BinOp::Eq, as_text(value.clone()), Expr::Lit(json!(c)))));
        let value = expr_val(value);
        let option = |cx: &mut RenderCx<'_>| {
            let mut props: Vec<(&str, Val)> = vec![("type", lit("radio")), ("value", value.clone()), ("name", name.clone())];
            if let Some(b) = node.prop("bind") {
                props.push(("bind", b.clone()));
            }
            props.push(("disabled", disabled.clone()));
            let extra: Vec<(&str, Val)> = checked.iter().map(|c| ("checked", c.clone())).collect();
            let mut row = irender_html::Element::new("label");
            row.base_style_ref(&CHOICE_ROW);
            cx.open(&row, node)?;
            irender_html::Input::render_styled(&control(node, props, &extra), cx, &CHECKBOX)?;
            let mut text = cx.part("span", node, "label");
            text.text(label.clone());
            cx.open(&text, node)?;
            cx.close("span");
            cx.close("label");
            Ok(())
        };

        let mut group = cx.element("div", node);
        group.base_style_ref(&RADIO_GROUP[orientation]).attr("role", "radiogroup");
        each_item(cx, node, group, options, OPTION, "ui_index", &value, option)?;
        cx.close("div");
        Ok(())
    }
}

/// The name of the current option while rendering a radio group's options.
const OPTION: &str = "ui_option";

// ---------------------------------------------------------------- slider

pub struct Slider;

const SLIDER_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key holding the number"),
    PropDef::new("min", PropKind::Any, "Default 0"),
    PropDef::new("max", PropKind::Any, "Default 100"),
    PropDef::new("step", PropKind::Any, "Default 1"),
    PropDef::new("name", PropKind::String, "Form field name"),
    PropDef::new("disabled", PropKind::Any, "Boolean or expression"),
];

static SLIDER: LazyLock<StyleBlock> = LazyLock::new(|| {
    native_scheme(StyleBlock::from_pairs(&[
        ("width", "100%"),
        ("height", sp!(20)),
        ("margin", "0"),
        ("accent-color", PRIMARY),
        ("cursor", "pointer"),
    ])
    .nest(":disabled", StyleBlock::from_pairs(&[("opacity", "0.5"), ("cursor", "not-allowed")]))
    .nest(":focus-visible", StyleBlock::from_pairs(&[("outline", "2px solid var(--color-ring, var(--color-muted))"), ("outline-offset", "2px")])))
});

impl Component for Slider {
    fn name(&self) -> &'static str {
        "ui-slider"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Pick a number by dragging.", props: SLIDER_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut extra = Vec::new();
        for (name, default) in [("min", 0), ("max", 100), ("step", 1)] {
            extra.push((name, node.prop(name).cloned().unwrap_or_else(|| lit(default))));
        }
        let mut input = control(node, passthrough(node, "range"), &extra);
        // The slider has no inner parts: the node's own attrs and style go on the input.
        for (k, v) in &node.attrs {
            input.attrs.insert(k.clone(), v.clone());
        }
        input.class = node.class.clone();
        input.style = node.style.clone();
        input.on = node.on.clone();
        irender_html::Input::render_styled(&input, cx, &SLIDER)
    }
}

// ---------------------------------------------------------------- button group

pub struct ButtonGroup;

const BUTTON_GROUP_PROPS: &[PropDef] = &[
    PropDef::new("orientation", PropKind::Enum(&["horizontal", "vertical"]), "Default `horizontal`"),
    PropDef::new("label", PropKind::Any, "Names the group for screen readers"),
];

static BUTTON_GROUP: LazyLock<[StyleBlock; 2]> = LazyLock::new(|| {
    // Neighbours share a border: inner corners go square and each overlaps the one before.
    let base = |direction: &str, first: [&str; 2], last: [&str; 2], overlap: &str| {
        let square = |corners: [&str; 2]| StyleBlock::from_pairs(&[(corners[0], "0"), (corners[1], "0")]);
        StyleBlock::from_pairs(&[("display", "inline-flex"), ("flex-direction", direction), ("align-items", "stretch")])
            .nest("& > :not(:first-child)", square(first).set(overlap, "-1px"))
            .nest("& > :not(:last-child)", square(last))
            // The focused one's ring isn't hidden under the next one.
            .nest("& > :focus-visible", StyleBlock::from_pairs(&[("position", "relative"), ("z-index", "1")]))
    };
    [
        base(
            "row",
            ["border-start-start-radius", "border-end-start-radius"],
            ["border-start-end-radius", "border-end-end-radius"],
            "margin-inline-start",
        ),
        base(
            "column",
            ["border-start-start-radius", "border-start-end-radius"],
            ["border-end-start-radius", "border-end-end-radius"],
            "margin-block-start",
        ),
    ]
});

impl Component for ButtonGroup {
    fn name(&self) -> &'static str {
        "ui-button-group"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Buttons joined into one control. Children are usually `ui-button`s; outline ones look best.",
            props: BUTTON_GROUP_PROPS,
            parts: &[],
            children: true,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let o = choice(cx, node, "orientation", &["horizontal", "vertical"]);
        let mut el = cx.element("div", node);
        el.base_style_ref(&BUTTON_GROUP[o]).attr("role", "group");
        if o == 1 {
            el.attr("aria-orientation", "vertical");
        }
        if let Some(label) = node.prop("label") {
            el.attr("aria-label", label.clone());
        }
        cx.open(&el, node)?;
        cx.children(node)?;
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- input group

pub struct InputGroup;

const INPUT_GROUP_PROPS: &[PropDef] = &[
    PropDef::new("prefix", PropKind::Any, "Text before the field, e.g. `https://` or `$`"),
    PropDef::new("suffix", PropKind::Any, "Text after the field, e.g. `.com` or `kg`"),
];

/// The fields inside a group, which give up their own border to the group's.
const GROUPED: &str = "& :is(input, textarea, select)";

static INPUT_GROUP: LazyLock<StyleBlock> = LazyLock::new(|| {
    let field = StyleBlock::from_pairs(&[
        ("flex", "1"),
        ("align-self", "stretch"),
        ("height", "auto"),
        ("border", "none"),
        ("border-radius", "0"),
        ("background", "transparent"),
        ("box-shadow", "none"),
    ]);
    StyleBlock::from_pairs(&[
        ("box-sizing", "border-box"),
        ("display", "flex"),
        ("align-items", "center"),
        ("width", "100%"),
        ("min-height", sp!(36)),
        ("border", "1px solid var(--color-border)"),
        ("border-radius", "var(--radius-md)"),
        ("color", TEXT),
        ("font-size", fs!(14)),
        ("box-shadow", "0 1px 2px rgb(0 0 0 / 0.05)"),
        ("overflow", "hidden"),
        ("transition", "border-color .15s, box-shadow .15s"),
    ])
    .nest(GROUPED, field)
    // Next to a prefix or suffix, the field's own padding would double the gap.
    .nest("& > span + :is(input, textarea, select)", StyleBlock::from_pairs(&[("padding-left", sp!(8))]))
    .nest("& > :is(input, textarea, select):has(+ span)", StyleBlock::from_pairs(&[("padding-right", sp!(8))]))
    .nest(format!("{GROUPED}:focus-visible"), StyleBlock::from_pairs(&[("box-shadow", "none")]))
    // The group shows the focus ring, around the field and its addons together.
    .nest(":has(:focus-visible)", focus_ring().set("border-color", RING))
    .nest(":has([aria-invalid=true])", StyleBlock::from_pairs(&[("border-color", DESTRUCTIVE)]))
    .nest(":has(:disabled)", StyleBlock::from_pairs(&[("opacity", "0.5")]))
    // Buttons sit inside the border, a little smaller than on their own.
    .nest("& > :is(button, a)", StyleBlock::from_pairs(&[("height", sp!(28)), ("margin", sp!(0, 4)), ("padding", sp!(0, 8))]))
});
static INPUT_GROUP_ADDON: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[("display", "flex"), ("align-items", "center"), ("padding", sp!(0, 12)), ("color", MUTED), ("white-space", "nowrap")])
});
static INPUT_GROUP_PREFIX: LazyLock<StyleBlock> = LazyLock::new(|| INPUT_GROUP_ADDON.clone().set("padding-right", "0"));
static INPUT_GROUP_SUFFIX: LazyLock<StyleBlock> = LazyLock::new(|| INPUT_GROUP_ADDON.clone().set("padding-left", "0"));

impl Component for InputGroup {
    fn name(&self) -> &'static str {
        "ui-input-group"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A field with text or buttons inside its border. Children are a `ui-input` (or `ui-textarea`, \
                  `ui-select`) and any `ui-button`s.",
            props: INPUT_GROUP_PROPS,
            parts: &["prefix", "suffix"],
            children: true,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut el = cx.element("div", node);
        el.base_style_ref(&INPUT_GROUP);
        cx.open(&el, node)?;
        let addon = |cx: &mut RenderCx<'_>, name: &str, base: &'static StyleBlock| -> Result<(), RenderError> {
            if let Some(t) = node.prop(name) {
                let mut part = cx.part("span", node, name);
                part.base_style_ref(base).text(t);
                cx.open(&part, node)?;
                cx.close("span");
            }
            Ok(())
        };
        addon(cx, "prefix", &INPUT_GROUP_PREFIX)?;
        cx.children(node)?;
        addon(cx, "suffix", &INPUT_GROUP_SUFFIX)?;
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- field

pub struct Field;

const FIELD_PROPS: &[PropDef] = &[
    PropDef::new("label", PropKind::Any, "The label"),
    PropDef::new("for", PropKind::Any, "`id` of the control it labels; default the first control inside it"),
    PropDef::new("description", PropKind::Any, "Help text under the control"),
    PropDef::new("error", PropKind::Any, "An error under the control; hidden while empty, so it can follow state"),
    PropDef::new("orientation", PropKind::Enum(&["vertical", "horizontal"]), "Default `vertical`; `horizontal` puts the control after the label"),
];

static FIELD: LazyLock<[StyleBlock; 2]> = LazyLock::new(|| {
    [
        StyleBlock::from_pairs(&[("display", "grid"), ("gap", sp!(8))]),
        // Label and control side by side; the text under them spans both columns.
        StyleBlock::from_pairs(&[
            ("display", "grid"),
            ("grid-template-columns", "1fr auto"),
            ("align-items", "center"),
            ("gap", sp!(8, 16)),
        ])
        .nest("& > p", StyleBlock::from_pairs(&[("grid-column", "1 / -1")])),
    ]
});
static FIELD_DESCRIPTION: LazyLock<StyleBlock> =
    LazyLock::new(|| StyleBlock::from_pairs(&[("margin", "0"), ("font-size", fs!(14)), ("color", MUTED)]));
static FIELD_ERROR: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[("margin", "0"), ("font-size", fs!(14)), ("color", DESTRUCTIVE)])
        .nest(":empty", StyleBlock::from_pairs(&[("display", "none")]))
});

impl Component for Field {
    fn name(&self) -> &'static str {
        "ui-field"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A label, a control, help text and an error, laid out together. Children are the control, which \
                  the label names and the help text and error describe.",
            props: FIELD_PROPS,
            parts: &["label", "description", "error"],
            children: true,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        // The control: the one `for` names, or the first. The label points at it, and it
        // points at the help text and error, so screen readers read them with it.
        let mut children = node.children.clone();
        let mut control_id = node.prop("for").cloned();
        let (mut description_id, mut error_id) = (None, None);
        if let Some(control) = find_control(&mut children, node.prop("for")) {
            let id = match focus_attr(control, "id") {
                Some(id) => id.clone(),
                None => {
                    let id = id_with(&instance_id(cx, node, "field"), "-control");
                    focus_attrs(control).insert("id".into(), id.clone());
                    id
                }
            };
            if node.prop("description").is_some() {
                description_id = Some(id_with(&id, "-description"));
            }
            if node.prop("error").is_some() {
                error_id = Some(id_with(&id, "-error"));
            }
            let ids: Vec<Val> = description_id.iter().chain(&error_id).cloned().collect();
            if !ids.is_empty() {
                describe(control, &ids);
            }
            control_id = Some(id);
        }

        let o = choice(cx, node, "orientation", &["vertical", "horizontal"]);
        let mut el = cx.element("div", node);
        el.base_style_ref(&FIELD[o]);
        cx.open(&el, node)?;
        if let Some(t) = node.prop("label") {
            let mut label = cx.part("label", node, "label");
            label.base_style_ref(&LABEL).text(t);
            if let Some(f) = control_id {
                label.attr("for", f);
            }
            cx.open(&label, node)?;
            cx.close("label");
        }
        children.iter().try_for_each(|c| cx.render(c))?;
        if let Some(d) = node.prop("description") {
            let mut p = cx.part("p", node, "description");
            p.base_style_ref(&FIELD_DESCRIPTION).text(d);
            if let Some(id) = description_id {
                p.attr("id", id);
            }
            cx.open(&p, node)?;
            cx.close("p");
        }
        if let Some(e) = node.prop("error") {
            let mut p = cx.part("p", node, "error");
            // Announced when it appears or changes.
            p.base_style_ref(&FIELD_ERROR).attr("aria-live", "polite").text(e);
            if let Some(id) = error_id {
                p.attr("id", id);
            }
            cx.open(&p, node)?;
            cx.close("p");
        }
        cx.close("div");
        Ok(())
    }
}

/// The control among `nodes`, depth first: the one whose focused element has `id`, or
/// without an `id` the first one.
fn find_control<'a>(nodes: &'a mut [Node], id: Option<&Val>) -> Option<&'a mut Node> {
    for n in nodes {
        let hit = match id {
            Some(id) => focus_attr(n, "id") == Some(id),
            None => CONTROLS.contains(&n.kind.as_str()),
        };
        if hit {
            return Some(n);
        }
        if let Some(found) = find_control(&mut n.children, id) {
            return Some(found);
        }
    }
    None
}
