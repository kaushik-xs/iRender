//! What the components drawn by a behaviour share: reading their props into its config.

use irender_core::{Node, StyleBlock, expr};
use irender_html::{RenderCx, RenderError};
use serde_json::{Map, Value as Json, json};

pub(crate) fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

/// Adds `kit.js` before a behaviour's own script.
pub(crate) fn kit(cx: &mut RenderCx<'_>) {
    crate::flips(cx);
    cx.global_js("ui-kit", include_str!("kit.js"));
}

/// The props as the behaviour's config: those in `live` may follow state, `keys` are state
/// keys, and the rest are read now. Missing ones are left out.
pub(crate) fn config(cx: &RenderCx<'_>, node: &Node, live: &[&str], fixed: &[&str], keys: &[&str]) -> Result<Map<String, Json>, RenderError> {
    let mut out = Map::new();
    for name in live {
        if let Some(v) = node.prop(name) {
            out.insert(name.to_string(), crate::data_table::dynamic(cx, node, name, v)?);
        }
    }
    for name in fixed {
        if let Some(v) = cx.prop(node, name).filter(|v| !v.is_null()) {
            out.insert(name.to_string(), v);
        }
    }
    for name in keys {
        if let Some(v) = cx.prop_string(node, name) {
            out.insert(name.to_string(), json!(v));
        }
    }
    Ok(out)
}

/// A boolean prop.
pub(crate) fn flag(cx: &RenderCx<'_>, node: &Node, name: &str, default: bool) -> bool {
    cx.prop(node, name).filter(|v| !v.is_null()).map_or(default, |v| expr::truthy(&v))
}

/// A number prop.
pub(crate) fn number(cx: &RenderCx<'_>, node: &Node, name: &str) -> Option<f64> {
    cx.prop(node, name).filter(|v| !v.is_null()).map(|v| expr::to_num(&v))
}

/// A small icon button's look.
pub(crate) fn tool() -> StyleBlock {
    pairs(&[
        ("display", "inline-grid"),
        ("place-items", "center"),
        ("flex", "none"),
        ("min-width", "28px"),
        ("height", "28px"),
        ("padding", "0"),
        ("border", "0"),
        ("border-radius", "var(--radius-sm)"),
        ("background", "none"),
        ("color", crate::MUTED),
        ("font", "inherit"),
        ("cursor", "pointer"),
    ])
}

/// A URL prop given in the document, checked: a path or http(s).
pub(crate) fn check_url(cx: &RenderCx<'_>, node: &Node, name: &str) -> Result<(), RenderError> {
    if let Some(u) = node.prop(name).filter(|u| !cx.is_reactive(u)) {
        let u = expr::stringify(&cx.eval(u));
        if !u.is_empty() && !crate::combobox::safe_search_url(&u) {
            return Err(RenderError::new(node, format!("`{name}` must be a path or an http(s) URL, got `{u}`")));
        }
    }
    Ok(())
}
