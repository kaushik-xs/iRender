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

mod display;
mod forms;
mod navigation;
mod theme;

pub use theme::{apply_theme, theme};

use irender_core::{Node, Val, expr};
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
        .register(navigation::Pagination);
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
