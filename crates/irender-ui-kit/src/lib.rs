//! A component kit for iRender, styled after shadcn/ui.
//!
//! ```no_run
//! use irender_core::Format;
//! use irender_html::Engine;
//!
//! let mut engine = Engine::new();
//! irender_ui_kit::register(&mut engine.registry);
//!
//! let mut doc = irender_core::parse(r#"{ "version": 1, "root": { "type": "ui-button",
//!     "props": { "label": "Save", "variant": "outline" } } }"#, Format::Json)?;
//! irender_ui_kit::apply_theme(&mut doc); // optional: the kit's neutral palette
//! let page = engine.render(&engine.resolve(&doc)?, &serde_json::json!({}))?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Every component is named `ui-…` so it never collides with the built-ins or your
//! own components. Like the built-ins, each only sets defaults: a document can
//! still override any attribute through `attrs`, any style through `style`, and
//! inner elements through `parts`.
//!
//! Colours come from theme tokens. The ones the default theme lacks
//! (`color.secondary`, `color.accent`, `color.destructive`, …) fall back to values
//! derived from `color.text` and `color.surface`, so the kit looks right with any
//! theme, dark mode included. [`apply_theme`] adds the kit's own palette.

mod disclosure;
mod display;
mod feedback;
mod forms;
mod navigation;
mod overlay;
mod theme;

pub use theme::{apply_theme, theme};

use indexmap::IndexMap;
use irender_core::{
    Action, Node, Part, Template, Val,
    expr::{self, BinOp, Expr, UnOp},
};
use irender_html::{RenderCx, RenderError, Registry};
use serde_json::Value as Json;

/// Adds every kit component to `registry`.
pub fn register(registry: &mut Registry) {
    registry
        .register(forms::Button)
        .register(display::Badge)
        .register(display::Card)
        .register(display::CardFooter)
        .register(display::Alert)
        .register(display::Avatar)
        .register(display::Separator)
        .register(display::Skeleton)
        .register(display::Kbd)
        .register(display::Progress)
        .register(display::AspectRatio)
        .register(display::Table)
        .register(display::Heading)
        .register(display::Text)
        .register(forms::Label)
        .register(forms::Input)
        .register(forms::Textarea)
        .register(forms::Select)
        .register(forms::Checkbox)
        .register(forms::Switch)
        .register(forms::RadioGroup)
        .register(navigation::Breadcrumb)
        .register(navigation::Pagination)
        .register(forms::Slider)
        .register(disclosure::Accordion)
        .register(disclosure::AccordionItem)
        .register(disclosure::Collapsible)
        .register(disclosure::Tabs)
        .register(disclosure::TabPanel)
        .register(disclosure::Toggle)
        .register(disclosure::ToggleGroup)
        .register(overlay::Dialog::DIALOG)
        .register(overlay::Dialog::ALERT)
        .register(overlay::Dialog::SHEET)
        .register(overlay::DialogClose)
        .register(overlay::DialogFooter)
        .register(overlay::Popover::POPOVER)
        .register(overlay::Popover::MENU)
        .register(overlay::MenuItem)
        .register(overlay::MenuSeparator)
        .register(overlay::MenuLabel)
        .register(overlay::Tooltip)
        .register(overlay::HoverCard)
        .register(feedback::Toaster);
}

// Colours. Tokens the default theme has are used directly; the others fall back
// to mixes of text and surface so they follow light and dark mode on their own.
const PRIMARY: &str = "var(--color-primary)";
const PRIMARY_HOVER: &str = "var(--color-primary-hover)";
const ON_PRIMARY: &str = "var(--color-on-primary)";
const SURFACE: &str = "var(--color-surface)";
const TEXT: &str = "var(--color-text)";
const MUTED: &str = "var(--color-muted)";
const BORDER: &str = "var(--color-border)";
const SECONDARY: &str = "var(--color-secondary, color-mix(in srgb, var(--color-text) 6%, var(--color-surface)))";
const ON_SECONDARY: &str = "var(--color-on-secondary, var(--color-text))";
const ACCENT: &str = "var(--color-accent, color-mix(in srgb, var(--color-text) 6%, var(--color-surface)))";
const SUBTLE: &str = "var(--color-subtle, color-mix(in srgb, var(--color-text) 6%, var(--color-surface)))";
const DESTRUCTIVE: &str = "var(--color-destructive, #dc2626)";
const ON_DESTRUCTIVE: &str = "var(--color-on-destructive, #ffffff)";
const RING: &str = "var(--color-ring, var(--color-muted))";
const MONO: &str = "ui-monospace, SFMono-Regular, Menlo, monospace";

/// The index of the prop's value in `options`, or 0 (the default) when it's
/// missing or unknown.
fn choice(cx: &RenderCx<'_>, node: &Node, prop: &str, options: &[&str]) -> usize {
    cx.prop_string(node, prop).and_then(|v| options.iter().position(|o| *o == v)).unwrap_or(0)
}

/// An array prop that the server expands into markup. It can't follow state, so
/// a prop that reads state is an error rather than silently frozen.
fn server_array(cx: &RenderCx<'_>, node: &Node, prop: &str) -> Result<Vec<Json>, RenderError> {
    if node.prop(prop).is_some_and(|v| cx.is_reactive(v)) {
        return Err(RenderError::new(node, format!("`{prop}` can't use state; it is expanded on the server")));
    }
    match cx.prop(node, prop) {
        Some(Json::Array(items)) => Ok(items),
        None | Some(Json::Null) => Ok(Vec::new()),
        Some(_) => Err(RenderError::new(node, format!("`{prop}` must evaluate to an array"))),
    }
}

/// A field of an object item, as text.
fn field(item: &Json, key: &str) -> Option<String> {
    item.get(key).filter(|v| !v.is_null()).map(expr::stringify)
}

fn lit(v: impl Into<Json>) -> Val {
    Val::Lit(v.into())
}

/// The element id for this instance of a component: the `id` prop if given
/// (templates allowed, e.g. `confirm-{{ item.id }}` inside a list), otherwise one
/// derived from the node's place in the document and the list item it is in.
fn instance_id(cx: &RenderCx<'_>, node: &Node, prefix: &str) -> Val {
    if let Some(id) = node.prop("id") {
        return id.clone();
    }
    let path: String = node.path.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
    let key = cx.scope_key();
    let id = if key.is_empty() { format!("{prefix}-{path}") } else { format!("{prefix}-{path}-{key}") };
    lit(id.replace("--", "-").trim_end_matches('-').to_string())
}

/// `id` followed by `suffix`, as a value that follows a templated id.
fn id_with(id: &Val, suffix: &str) -> Val {
    match id {
        Val::Lit(v) => lit(format!("{}{suffix}", expr::stringify(v))),
        Val::Tpl(t) => {
            let mut t = t.clone();
            t.parts.push(Part::Lit(suffix.into()));
            Val::Tpl(t)
        }
    }
}

/// `prefix` followed by `id`, e.g. a CSS anchor name.
fn id_after(prefix: &str, id: &Val) -> Val {
    match id {
        Val::Lit(v) => lit(format!("{prefix}{}", expr::stringify(v))),
        Val::Tpl(t) => {
            let mut t = t.clone();
            t.parts.insert(0, Part::Lit(prefix.into()));
            Val::Tpl(t)
        }
    }
}

fn expr_val(e: Expr) -> Val {
    Val::Tpl(Template { parts: vec![Part::Expr(e)] })
}

fn state(key: &str) -> Expr {
    Expr::Path(vec![key.to_string()])
}

/// `{{ key == value }}`: live in the browser because it reads state.
fn state_eq(key: &str, value: Json) -> Val {
    expr_val(Expr::Binary(BinOp::Eq, Box::new(state(key)), Box::new(Expr::Lit(value))))
}

/// `{{ !key }}`
fn state_not(key: &str) -> Val {
    expr_val(Expr::Unary(UnOp::Not, Box::new(state(key))))
}

/// A node the kit renders itself, carrying its own attributes and actions. The
/// path stays the original node's so errors point at the document.
fn synthetic(node: &Node, kind: &str, props: Vec<(&str, Val)>, attrs: Vec<(&str, Val)>, on: Vec<(&str, Vec<Action>)>) -> Node {
    Node {
        kind: kind.into(),
        path: node.path.clone(),
        props: props.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        attrs: attrs.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        class: Vec::new(),
        style: Default::default(),
        parts: IndexMap::new(),
        on: on.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        children: Vec::new(),
    }
}

/// The value of a name a parent component put in scope for its children (the
/// dialog a close button belongs to, the accordion an item is in).
fn context(cx: &RenderCx<'_>, name: &str) -> Option<Val> {
    let v = cx.lookup(&[name.to_string()]);
    (!v.is_null()).then_some(Val::Lit(v))
}
