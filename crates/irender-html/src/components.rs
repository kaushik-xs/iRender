//! Built-in components. Each one only sets defaults: a document can override any
//! attribute through `attrs`, any style through `style`, and inner elements through
//! `parts`.

use crate::{Component, RenderCx, RenderError};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, Val, expr};
use serde_json::Value as Json;
use std::sync::LazyLock;

/// `element`: any HTML tag. The escape hatch for anything the others don't cover.
pub struct ElementComponent;

const ELEMENT_PROPS: &[PropDef] = &[PropDef::new("tag", PropKind::Tag, "HTML tag name (default `div`)")];

impl Component for ElementComponent {
    fn name(&self) -> &'static str {
        "element"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Any HTML element. Set attributes with `attrs`.",
            props: ELEMENT_PROPS,
            parts: &[],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let tag = cx.prop_string(node, "tag").unwrap_or_else(|| "div".into());
        let el = cx.element(&tag, node);
        if el.is_void() && !node.children.is_empty() {
            return Err(RenderError::new(node, format!("`<{tag}>` cannot have children")));
        }
        cx.open(&el, node)?;
        if !el.is_void() {
            cx.children(node)?;
            cx.close(&tag);
        }
        Ok(())
    }
}

/// `text`: a piece of text in a `span` (or any tag).
pub struct Text;

const TEXT_PROPS: &[PropDef] = &[
    PropDef::new("value", PropKind::Any, "The text. Templates update live when they use `state`.").required(),
    PropDef::new("tag", PropKind::Tag, "HTML tag (default `span`), e.g. `h1`, `p`, `li`"),
];

impl Component for Text {
    fn name(&self) -> &'static str {
        "text"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Text content.", props: TEXT_PROPS, parts: &[], children: false, open_props: false }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let tag = cx.prop_string(node, "tag").unwrap_or_else(|| "span".into());
        let mut el = cx.element(&tag, node);
        if let Some(v) = node.prop("value") {
            el.text(v);
        }
        cx.open(&el, node)?;
        cx.close(&tag);
        Ok(())
    }
}

/// `stack`: a flex container.
pub struct Stack;

const STACK_PROPS: &[PropDef] = &[
    PropDef::new("direction", PropKind::Enum(&["row", "column", "row-reverse", "column-reverse"]), "Default `column`"),
    PropDef::new("gap", PropKind::Any, "Number (px), CSS length or `$token`"),
    PropDef::new("align", PropKind::Any, "`align-items`"),
    PropDef::new("justify", PropKind::Any, "`justify-content`"),
    PropDef::new("wrap", PropKind::Bool, "Wrap children onto multiple lines"),
    PropDef::new("tag", PropKind::Tag, "HTML tag (default `div`)"),
];

impl Component for Stack {
    fn name(&self) -> &'static str {
        "stack"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Flex layout container.",
            props: STACK_PROPS,
            parts: &[],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let tag = cx.prop_string(node, "tag").unwrap_or_else(|| "div".into());
        let mut base = StyleBlock::from_pairs(&[("display", "flex")]);
        let direction = node.prop("direction").cloned().unwrap_or_else(|| Val::from("column"));
        let mapped = [
            ("flex-direction", Some(&direction)),
            ("gap", node.prop("gap")),
            ("align-items", node.prop("align")),
            ("justify-content", node.prop("justify")),
        ];
        for (property, v) in mapped {
            if let Some(v) = v
                && let Some(value) = cx.style_value(node, property, v)?
            {
                base.push(property, value);
            }
        }
        if cx.prop(node, "wrap").is_some_and(|w| expr::truthy(&w)) {
            base = base.set("flex-wrap", "wrap");
        }
        let mut el = cx.element(&tag, node);
        el.base_style(base);
        cx.open(&el, node)?;
        cx.children(node)?;
        cx.close(&tag);
        Ok(())
    }
}

/// `button`: a styled `<button>` with a `label` part.
pub struct Button;

const BUTTON_PROPS: &[PropDef] = &[
    PropDef::new("label", PropKind::Any, "Button text, rendered in the `label` part"),
    PropDef::new("variant", PropKind::Enum(&["primary", "secondary", "ghost"]), "Default `primary`"),
    PropDef::new("type", PropKind::Enum(&["button", "submit", "reset"]), "Default `button`"),
    PropDef::new("disabled", PropKind::Any, "Boolean or expression"),
];

static BUTTON_BASE: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("display", "inline-flex"),
        ("align-items", "center"),
        ("justify-content", "center"),
        ("gap", "8px"),
        ("padding", "8px 16px"),
        ("border", "1px solid transparent"),
        ("border-radius", "var(--radius-md)"),
        ("font", "inherit"),
        ("font-weight", "500"),
        ("line-height", "1.25"),
        ("cursor", "pointer"),
        ("transition", "background-color .15s, border-color .15s, color .15s"),
    ])
    .nest(
        ":focus-visible",
        StyleBlock::from_pairs(&[("outline", "2px solid var(--color-primary)"), ("outline-offset", "2px")]),
    )
    .nest(":disabled", StyleBlock::from_pairs(&[("opacity", "0.5"), ("cursor", "not-allowed")]))
});

static BUTTON_VARIANTS: LazyLock<[StyleBlock; 3]> =
    LazyLock::new(|| [button_variant("primary"), button_variant("secondary"), button_variant("ghost")]);

type Pairs = &'static [(&'static str, &'static str)];

fn button_variant(variant: &str) -> StyleBlock {
    let (rest, hover): (Pairs, Pairs) = match variant {
        "secondary" => (
            &[("background", "var(--color-surface)"), ("color", "var(--color-text)"), ("border-color", "var(--color-border)")],
            &[("border-color", "var(--color-muted)")],
        ),
        "ghost" => (
            &[("background", "transparent"), ("color", "var(--color-primary)")],
            &[("background", "color-mix(in srgb, var(--color-primary) 10%, transparent)")],
        ),
        _ => (
            &[("background", "var(--color-primary)"), ("color", "var(--color-on-primary)")],
            &[("background", "var(--color-primary-hover)")],
        ),
    };
    StyleBlock::from_pairs(rest).nest(":hover:not(:disabled)", StyleBlock::from_pairs(hover))
}

impl Component for Button {
    fn name(&self) -> &'static str {
        "button"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A button. Wire it up with `on.click`.",
            props: BUTTON_PROPS,
            parts: &["label"],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let variant = match cx.prop_string(node, "variant").as_deref() {
            Some("secondary") => &BUTTON_VARIANTS[1],
            Some("ghost") => &BUTTON_VARIANTS[2],
            _ => &BUTTON_VARIANTS[0],
        };
        let mut el = cx.element("button", node);
        el.base_style_ref(&BUTTON_BASE).base_style_ref(variant);
        el.attr("type", cx.prop_string(node, "type").unwrap_or_else(|| "button".into()));
        if let Some(d) = node.prop("disabled") {
            el.attr("disabled", d.clone());
        }
        cx.open(&el, node)?;
        if let Some(label) = node.prop("label") {
            let mut part = cx.part("span", node, "label");
            part.text(label);
            cx.open(&part, node)?;
            cx.close("span");
        }
        cx.children(node)?;
        cx.close("button");
        Ok(())
    }
}

/// `input`: a text-like `<input>`, optionally bound to state.
pub struct Input;

static INPUT_BASE: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("padding", "8px 12px"),
        ("border", "1px solid var(--color-border)"),
        ("border-radius", "var(--radius-md)"),
        ("font", "inherit"),
        ("background", "var(--color-surface)"),
        ("color", "var(--color-text)"),
    ])
    .nest(":focus", StyleBlock::from_pairs(&[("outline", "2px solid var(--color-primary)"), ("outline-offset", "-1px")]))
});

const INPUT_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key kept in sync with the value"),
    PropDef::new("type", PropKind::String, "Input type (default `text`)"),
    PropDef::new("placeholder", PropKind::Any, ""),
    PropDef::new("name", PropKind::String, "Form field name"),
    PropDef::new("value", PropKind::Any, "Initial value when not bound"),
    PropDef::new("disabled", PropKind::Any, "Boolean or expression"),
];

impl Component for Input {
    fn name(&self) -> &'static str {
        "input"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Form input.", props: INPUT_PROPS, parts: &[], children: false, open_props: false }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut el = cx.element("input", node);
        el.base_style_ref(&INPUT_BASE);
        el.attr("type", cx.prop_string(node, "type").unwrap_or_else(|| "text".into()));
        for name in ["placeholder", "name", "disabled"] {
            if let Some(v) = node.prop(name) {
                el.attr(name, v.clone());
            }
        }
        if let Some(key) = cx.prop_string(node, "bind") {
            el.attr("value", cx.lookup(std::slice::from_ref(&key)));
            el.bind(key);
        } else if let Some(v) = node.prop("value") {
            el.attr("value", v.clone());
        }
        cx.open(&el, node)
    }
}

/// `list`: repeats its children for each item of an array.
pub struct List;

const LIST_PROPS: &[PropDef] = &[
    PropDef::new("items", PropKind::Any, "An array: `{{ data.path }}`, or `{{ state_key }}` to rebuild in the browser")
        .required(),
    PropDef::new("as", PropKind::String, "Name of the current item (default `item`)"),
    PropDef::new("index", PropKind::String, "Name of the current index (default `index`)"),
    PropDef::new("tag", PropKind::Tag, "HTML tag (default `div`), e.g. `ul`"),
];

impl Component for List {
    fn name(&self) -> &'static str {
        "list"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Renders its children once per item. Rebuilt in the browser when `items` uses state.",
            props: LIST_PROPS,
            parts: &[],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let items = match cx.prop(node, "items") {
            Some(Json::Array(items)) => items,
            None | Some(Json::Null) => Vec::new(),
            Some(_) => return Err(RenderError::new(node, "`items` must evaluate to an array")),
        };
        let item = cx.prop_string(node, "as").unwrap_or_else(|| "item".into());
        let index = cx.prop_string(node, "index").unwrap_or_else(|| "index".into());
        let tag = cx.prop_string(node, "tag").unwrap_or_else(|| "div".into());
        let mut el = cx.element(&tag, node);
        let reactive = node.prop("items").filter(|v| cx.is_reactive(v));
        if let Some(v) = reactive {
            el.list(v, item.clone(), index.clone());
        }
        cx.open(&el, node)?;
        if reactive.is_some() {
            cx.item_template(node, &item, &index)?;
        }
        cx.each(node, items, &item, &index)?;
        cx.close(&tag);
        Ok(())
    }
}

/// `if`: renders its children only when `when` is truthy.
pub struct If;

const IF_PROPS: &[PropDef] =
    &[PropDef::new("when", PropKind::Any, "Condition. Shown and hidden in the browser when it uses state.").required()];

impl Component for If {
    fn name(&self) -> &'static str {
        "if"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Renders its children when `when` is truthy. Adds no element of its own.",
            props: IF_PROPS,
            parts: &[],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let Some(when) = node.prop("when") else { return Ok(()) };
        if cx.is_reactive(when) {
            return cx.conditional(node, when);
        }
        if expr::truthy(&cx.eval(when)) {
            cx.children(node)?;
        }
        Ok(())
    }
}
