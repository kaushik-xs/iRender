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

/// A spacing value that follows the theme's `space.scale` (1 unless a theme option
/// sets it): `sp!(4, 12)` is `calc(4px * var(--space-scale, 1)) calc(12px * …)`. The
/// kit's paddings, gaps, margins and control heights use it, so a density option
/// makes every component more compact or roomier. `0` stays `0`.
macro_rules! sp {
    (0) => { "0" };
    (0, $($rest:literal),+) => { concat!("0 ", sp!($($rest),+)) };
    ($n:literal) => { concat!("calc(", $n, "px * var(--space-scale, 1))") };
    ($n:literal, $($rest:literal),+) => { concat!(sp!($n), " ", sp!($($rest),+)) };
}

/// A font size that follows the theme's `font.scale`: `fs!(14)`.
macro_rules! fs {
    ($n:literal) => { concat!("calc(", $n, "px * var(--font-scale, 1))") };
}

mod calendar;
mod carousel;
mod chat;
mod combobox;
mod command;
mod data_table;
mod disclosure;
mod display;
mod feedback;
mod forms;
mod navigation;
mod overlay;
mod sidebar;
mod theme;

pub use chat::{render_message_ui, render_ui, ui_document};
pub use theme::{apply_theme, theme};

use indexmap::IndexMap;
use irender_core::{
    Action, Node, Part, Template, Val,
    expr::{self, BinOp, Expr, UnOp},
};
use irender_html::{ClientList, Element, RenderCx, RenderError, Registry};
use serde_json::{Value as Json, json};

/// Adds every kit component to `registry`, and the script that applies the colour
/// scheme chosen with `ui-theme-toggle` to every page it renders.
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
        .register(display::Spinner)
        .register(display::Empty)
        .register(display::ScrollArea)
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
        .register(forms::ButtonGroup)
        .register(forms::InputGroup)
        .register(forms::Field)
        .register(disclosure::Accordion)
        .register(disclosure::AccordionItem)
        .register(disclosure::Collapsible)
        .register(disclosure::Tabs)
        .register(disclosure::TabPanel)
        .register(disclosure::Toggle)
        .register(disclosure::ToggleGroup)
        .register(disclosure::ThemeToggle)
        .register(disclosure::ThemeOption)
        .register(disclosure::ThemePanel)
        .register(disclosure::ThemeColors)
        .register(overlay::Dialog::DIALOG)
        .register(overlay::Dialog::ALERT)
        .register(overlay::Dialog::SHEET)
        .register(overlay::DialogClose)
        .register(overlay::DialogFooter)
        .register(overlay::Popover::POPOVER)
        .register(overlay::Popover::MENU)
        .register(overlay::SplitButton)
        .register(overlay::MenuItem)
        .register(overlay::MenuSeparator)
        .register(overlay::MenuLabel)
        .register(overlay::Tooltip)
        .register(overlay::HoverCard)
        .register(feedback::Toaster)
        .register(sidebar::SidebarLayout)
        .register(sidebar::Sidebar)
        .register(sidebar::SidebarHeader)
        .register(sidebar::SidebarContent)
        .register(sidebar::SidebarFooter)
        .register(sidebar::SidebarGroup)
        .register(sidebar::SidebarItem)
        .register(sidebar::SidebarTrigger)
        .register(combobox::Combobox)
        .register(command::Command)
        .register(command::CommandDialog)
        .register(command::CommandGroup)
        .register(command::CommandItem)
        .register(command::CommandSeparator)
        .register(chat::Conversation)
        .register(chat::Message)
        .register(chat::TypingIndicator)
        .register(chat::ChatInput)
        .register(carousel::Carousel)
        .register(calendar::Calendar)
        .register(calendar::DatePicker)
        .register(data_table::DataTable)
        .register(data_table::Child::COLUMN)
        .register(data_table::Child::TOOLBAR)
        .register(data_table::Child::BULK)
        .register(data_table::Child::DETAIL)
        // A Light or Dark choice made with `ui-theme-toggle` holds on every page, with a
        // toggle on it or not.
        .early_js("ui-theme", disclosure::THEME_EARLY_SCRIPT);
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
/// The second brand colour, for the `brand` variants; the primary when a theme has none.
const BRAND_2: &str = "var(--color-brand-2, var(--color-primary))";
const ON_BRAND_2: &str = "var(--color-on-brand-2, var(--color-on-primary))";
const DESTRUCTIVE: &str = "var(--color-destructive, #dc2626)";
const ON_DESTRUCTIVE: &str = "var(--color-on-destructive, #ffffff)";
const RING: &str = "var(--color-ring, var(--color-muted))";
const MONO: &str = "ui-monospace, SFMono-Regular, Menlo, monospace";

/// One turn, for spinners; added to the page once by whichever needs it first.
const SPIN: &str = "@keyframes ui-spin {\n  to { transform: rotate(1turn); }\n}";

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
    let id = id.replace("--", "-").trim_end_matches('-').to_string();
    // Items of a list the browser rebuilds share a scope key; their index tells them apart.
    match cx.client_key() {
        Some(mut t) => {
            t.parts.insert(0, Part::Lit(format!("{id}-")));
            Val::Tpl(t)
        }
        None => lit(id),
    }
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

/// The value a parent component passed to its children (the dialog a close button
/// belongs to, the accordion an item is in).
fn context(cx: &RenderCx<'_>, name: &str) -> Option<Val> {
    cx.context(name).cloned()
}

/// The kit's and the built-ins' form controls, which a `ui-field` labels and describes.
const CONTROLS: &[&str] = &[
    "input", "textarea", "select", "ui-input", "ui-textarea", "ui-select", "ui-checkbox", "ui-switch", "ui-slider",
    "ui-radio-group", "ui-combobox", "ui-date-picker",
];

/// The `attrs` that reach the element a control focuses: the inner `<input>` for kit
/// controls that wrap one, otherwise the node's own.
fn focus_attrs(node: &mut Node) -> &mut IndexMap<String, Val> {
    match node.kind.as_str() {
        "ui-checkbox" | "ui-switch" => &mut node.parts.entry("control".into()).or_default().attrs,
        "ui-combobox" => &mut node.parts.entry("input".into()).or_default().attrs,
        "ui-date-picker" => &mut node.parts.entry("trigger".into()).or_default().attrs,
        _ => &mut node.attrs,
    }
}

fn focus_attr<'a>(node: &'a Node, name: &str) -> Option<&'a Val> {
    match node.kind.as_str() {
        "ui-checkbox" | "ui-switch" => node.parts.get("control")?.attrs.get(name),
        "ui-combobox" => node.parts.get("input")?.attrs.get(name),
        "ui-date-picker" => node.parts.get("trigger")?.attrs.get(name),
        _ => node.attrs.get(name),
    }
}

/// Adds `ids` to the `aria-describedby` of the element `node` focuses, after any it has.
fn describe(node: &mut Node, ids: &[Val]) {
    let attrs = focus_attrs(node);
    let mut all: Vec<Val> = attrs.get("aria-describedby").cloned().into_iter().collect();
    all.extend_from_slice(ids);
    attrs.insert("aria-describedby".into(), join(&all));
}

/// Values joined with spaces: a template if any of them is one.
fn join(vals: &[Val]) -> Val {
    let mut parts = Vec::new();
    for (i, v) in vals.iter().enumerate() {
        if i > 0 {
            parts.push(Part::Lit(" ".into()));
        }
        match v {
            Val::Lit(j) => parts.push(Part::Lit(expr::stringify(j))),
            Val::Tpl(t) => parts.extend(t.parts.iter().cloned()),
        }
    }
    if parts.iter().all(|p| matches!(p, Part::Lit(_))) {
        return lit(parts.into_iter().map(|p| if let Part::Lit(s) = p { s } else { unreachable!() }).collect::<String>());
    }
    Val::Tpl(Template { parts })
}

/// The array prop `prop` as items to render: its value now, and the prop itself when
/// it uses state, so the browser can rebuild the items as it changes.
fn items<'n>(cx: &RenderCx<'_>, node: &'n Node, prop: &str) -> Result<(Vec<Json>, Option<&'n Val>), RenderError> {
    let values = match cx.prop(node, prop) {
        Some(Json::Array(items)) => items,
        None | Some(Json::Null) => Vec::new(),
        Some(_) => return Err(RenderError::new(node, format!("`{prop}` must evaluate to an array"))),
    };
    Ok((values, node.prop(prop).filter(|v| cx.is_reactive(v))))
}

/// Opens `el` and renders `render` once per item with the item and its index in
/// scope as `item` and `index`. With `live`, the browser rebuilds the items whenever
/// it changes, reusing those whose `key` it has seen. The caller closes `el`.
#[allow(clippy::too_many_arguments)]
fn each_item<'n>(
    cx: &mut RenderCx<'_>,
    node: &'n Node,
    mut el: Element<'n>,
    (values, live): (Vec<Json>, Option<&'n Val>),
    item: &str,
    index: &str,
    key: &Val,
    render: impl Fn(&mut RenderCx<'_>) -> Result<(), RenderError>,
) -> Result<(), RenderError> {
    if let Some(items) = live {
        let key = cx.item_val_js(key, item, index);
        el.list(ClientList { items, item: item.into(), index: index.into(), key: Some(key) });
    }
    cx.open(&el, node)?;
    if live.is_some() {
        cx.item_template_with(item, index, &render)?;
    }
    for (i, value) in values.into_iter().enumerate() {
        cx.with_scope(vec![(item.into(), value), (index.into(), json!(i))], &render)?;
    }
    Ok(())
}

/// Renders `f` while `when` holds: decided on the server, or by the browser when
/// `when` uses state (or an item of a list the browser rebuilds).
fn branch(
    cx: &mut RenderCx<'_>,
    when: &Val,
    f: impl FnOnce(&mut RenderCx<'_>) -> Result<(), RenderError>,
) -> Result<(), RenderError> {
    if cx.is_reactive(when) {
        cx.conditional_with(when, f)
    } else if expr::truthy(&cx.eval(when)) {
        f(cx)
    } else {
        Ok(())
    }
}

/// A value as an expression: a literal, a template's one expression, or the parts of
/// a longer template joined as text.
fn val_expr(v: &Val) -> Expr {
    match v {
        Val::Lit(j) => Expr::Lit(j.clone()),
        Val::Tpl(t) => match t.parts.as_slice() {
            [Part::Expr(e)] => e.clone(),
            parts => parts.iter().fold(Expr::Lit(json!("")), |text, p| {
                let p = match p {
                    Part::Lit(s) => Expr::Lit(json!(s)),
                    Part::Expr(e) => e.clone(),
                };
                bin(BinOp::Add, text, p)
            }),
        },
    }
}

fn bin(op: BinOp, a: Expr, b: Expr) -> Expr {
    Expr::Binary(op, Box::new(a), Box::new(b))
}

fn cond(c: Expr, a: Expr, b: Expr) -> Expr {
    Expr::Cond(Box::new(c), Box::new(a), Box::new(b))
}

fn path(segments: &[&str]) -> Expr {
    Expr::Path(segments.iter().map(|s| s.to_string()).collect())
}

fn num(n: impl Into<Json>) -> Expr {
    Expr::Lit(n.into())
}

/// `var.field` when the item is an object that has it, otherwise `fallback`: options
/// and items may be plain values or objects.
fn field_or(var: &str, field: &str, fallback: Expr) -> Expr {
    let f = path(&[var, field]);
    cond(bin(BinOp::Ne, f.clone(), Expr::Lit(Json::Null)), f, fallback)
}

/// The icon name `value` holds, when the icon kit draws it: the kit is registered and
/// the value is a name like `settings` or `it:database`. Known on the server, written
/// in the document or from server data (a list item's, say), not from state.
fn icon_name(cx: &RenderCx<'_>, value: &Val) -> Option<Val> {
    if cx.is_reactive(value) || !cx.has_component("ui-icon") {
        return None;
    }
    let Json::String(n) = cx.eval(value) else { return None };
    let name = !n.is_empty() && n.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == ':');
    name.then(|| lit(n))
}

/// The icon kit's `ui-icon` for `name`, decorative.
fn icon_kit(cx: &mut RenderCx<'_>, node: &Node, name: &Val, size: u32) -> Result<(), RenderError> {
    let mut i = synthetic(node, "ui-icon", vec![("name", name.clone()), ("size", lit(size))], Vec::new(), Vec::new());
    i.style = irender_core::StyleBlock::from_pairs(&[("flex-shrink", "0")]);
    cx.render(&i)
}

/// An icon in front of a label: a `ui-icon` when the icon kit is registered and the
/// value is an icon name (`settings`, `it:database`), otherwise the value as text,
/// e.g. an emoji. Decorative either way: the label next to it names the thing.
fn icon(cx: &mut RenderCx<'_>, node: &Node, value: &Val, size: u32) -> Result<(), RenderError> {
    if let Some(name) = icon_name(cx, value) {
        return icon_kit(cx, node, &name, size);
    }
    let mut span = Element::new("span");
    span.attr("aria-hidden", "true").attr("data-icon", true).text(value.clone());
    cx.open(&span, node)?;
    cx.close("span");
    Ok(())
}
