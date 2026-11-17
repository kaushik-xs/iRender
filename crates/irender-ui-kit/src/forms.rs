//! Buttons and form controls. The inputs reuse the built-ins' state binding and
//! only change how they look.

use crate::{
    ACCENT, DESTRUCTIVE, MUTED, ON_DESTRUCTIVE, ON_PRIMARY, ON_SECONDARY, PRIMARY, PRIMARY_HOVER, RING, SECONDARY,
    SURFACE, TEXT, choice, field, lit, server_array,
};
use indexmap::IndexMap;
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, Val, expr};
use irender_html::{Component, RenderCx, RenderError};
use serde_json::Value as Json;
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
        ("gap", "8px"),
        ("flex-shrink", "0"),
        ("white-space", "nowrap"),
        ("border", "1px solid transparent"),
        ("border-radius", "var(--radius-md)"),
        ("font-family", "inherit"),
        ("font-size", "14px"),
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
        StyleBlock::from_pairs(&[("height", "36px"), ("padding", "0 16px")]),
        StyleBlock::from_pairs(&[("height", "32px"), ("padding", "0 12px"), ("gap", "6px")]),
        StyleBlock::from_pairs(&[("height", "40px"), ("padding", "0 24px")]),
        StyleBlock::from_pairs(&[("height", "36px"), ("width", "36px"), ("padding", "0")]),
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
        if let Some(label) = node.prop("label") {
            el.text(label);
        }
        cx.open(&el, node)?;
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
        ("gap", "8px"),
        ("font-size", "14px"),
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
fn native_scheme(b: StyleBlock) -> StyleBlock {
    b.set("color-scheme", "light").nest("@dark", StyleBlock::from_pairs(&[("color-scheme", "dark")]))
}

fn field_base(height: Option<&str>) -> StyleBlock {
    let mut b = StyleBlock::from_pairs(&[
        // Padding and border count inside `width: 100%` and `height`.
        ("box-sizing", "border-box"),
        ("width", "100%"),
        ("min-width", "0"),
        ("padding", "4px 12px"),
        ("border", "1px solid var(--color-border)"),
        ("border-radius", "var(--radius-md)"),
        ("background", "transparent"),
        ("color", TEXT),
        ("font-family", "inherit"),
        ("font-size", "14px"),
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
    LazyLock::new(|| field_base(None).set("min-height", "64px").set("padding", "8px 12px").set("resize", "vertical"));
static SELECT: LazyLock<StyleBlock> = LazyLock::new(|| field_base(Some("36px")).set("padding", "4px 8px").set("cursor", "pointer"));

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
        ("gap", "8px"),
        ("font-size", "14px"),
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
        ("height", "20px"),
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
    PropDef::new("options", PropKind::Any, "Array of values or `{ value, label, disabled }`; fixed on the server").required(),
    PropDef::new("bind", PropKind::StateKey, "State key holding the chosen value"),
    PropDef::new("value", PropKind::Any, "Initially chosen value, when not bound"),
    PropDef::new("name", PropKind::String, "Form field name (default: the `bind` key)"),
    PropDef::new("orientation", PropKind::Enum(&["vertical", "horizontal"]), "Default `vertical`"),
    PropDef::new("disabled", PropKind::Any, "Disable every option"),
];

static RADIO_GROUP: LazyLock<[StyleBlock; 2]> = LazyLock::new(|| {
    [
        StyleBlock::from_pairs(&[("display", "grid"), ("gap", "12px")]),
        StyleBlock::from_pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("gap", "16px")]),
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
        let options = server_array(cx, node, "options")?;
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

        let mut group = cx.element("div", node);
        group.base_style_ref(&RADIO_GROUP[orientation]).attr("role", "radiogroup");
        cx.open(&group, node)?;
        for o in &options {
            let (value, label) = match o {
                Json::Object(_) => (field(o, "value").unwrap_or_default(), field(o, "label").or_else(|| field(o, "value"))),
                other => (expr::stringify(other), Some(expr::stringify(other))),
            };
            let disabled = disabled_all || o.get("disabled").is_some_and(expr::truthy);
            let mut props: Vec<(&str, Val)> = vec![("type", lit("radio")), ("value", lit(value.clone())), ("name", name.clone())];
            if let Some(b) = node.prop("bind") {
                props.push(("bind", b.clone()));
            }
            if disabled {
                props.push(("disabled", lit(true)));
            }
            let mut extra = Vec::new();
            if chosen.as_deref() == Some(value.as_str()) {
                extra.push(("checked", lit(true)));
            }

            let mut row = irender_html::Element::new("label");
            row.base_style_ref(&CHOICE_ROW);
            cx.open(&row, node)?;
            irender_html::Input::render_styled(&control(node, props, &extra), cx, &CHECKBOX)?;
            let mut text = cx.part("span", node, "label");
            text.text(lit(label.unwrap_or_default()));
            cx.open(&text, node)?;
            cx.close("span");
            cx.close("label");
        }
        cx.close("div");
        Ok(())
    }
}

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
        ("height", "20px"),
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
