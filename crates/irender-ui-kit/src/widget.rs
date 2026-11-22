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

/// What the device components have in common, drawn by their behaviours: rows of controls
/// (`data-w-row`), buttons (`data-w-btn`, `=primary`, `=danger` or `=ghost`), notes, errors
/// and lists.
pub(crate) fn device(b: StyleBlock) -> StyleBlock {
    use crate::{BORDER, MUTED, ON_PRIMARY, PRIMARY, PRIMARY_FILL, SUBTLE, SURFACE, TEXT, forms};
    forms::native_scheme(b)
        .nest("& [hidden]", pairs(&[("display", "none !important")]))
        .nest("& [data-w-row]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("align-items", "center"), ("gap", gap!(8)), ("min-width", "0")]))
        .nest("& [data-w-push]", pairs(&[("margin-inline-start", "auto")]))
        .nest("& [data-w-btn]", pairs(&[("box-sizing", "border-box"), ("display", "inline-flex"), ("align-items", "center"), ("justify-content", "center"), ("gap", gap!(6)), ("min-width", ctl!(36)), ("height", ctl!(36)), ("padding", pad!(0, 14)), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-control, var(--radius-md))"), ("background", SURFACE), ("color", TEXT), ("font", "inherit"), ("font-size", fs!(14)), ("font-weight", "var(--font-weight-medium, 500)"), ("text-decoration", "none"), ("white-space", "nowrap"), ("cursor", "pointer")]))
        .nest("& [data-w-btn]:hover", pairs(&[("background", SUBTLE)]))
        .nest("& [data-w-btn=primary]", pairs(&[("border-color", "transparent"), ("background", PRIMARY_FILL), ("color", ON_PRIMARY)]))
        .nest("& [data-w-btn=primary]:hover", pairs(&[("background", crate::PRIMARY_FILL_HOVER)]))
        .nest("& [data-w-btn=danger]", pairs(&[("border-color", "transparent"), ("background", crate::DESTRUCTIVE), ("color", crate::ON_DESTRUCTIVE)]))
        .nest("& [data-w-btn=ghost]", pairs(&[("border-color", "transparent"), ("background", "none"), ("color", MUTED)]))
        .nest("& [data-w-btn=ghost]:hover", pairs(&[("background", SUBTLE), ("color", TEXT)]))
        .nest("& [data-w-btn][data-icon-only]", pairs(&[("padding", "0")]))
        .nest("& [data-w-btn][aria-pressed=true]", pairs(&[("border-color", PRIMARY), ("background", "color-mix(in srgb, var(--color-primary) 12%, transparent)"), ("color", TEXT)]))
        .nest("& [data-w-btn]:disabled", pairs(&[("opacity", "0.5"), ("cursor", "default")]))
        .nest("& :is(button, a, input, select, textarea, [tabindex]):focus-visible", forms::focus_ring())
        .nest("& [data-w-note]", pairs(&[("margin", "0"), ("color", MUTED), ("font-size", fs!(13)), ("line-height", "1.45")]))
        .nest("& [data-w-note]:empty", pairs(&[("display", "none")]))
        .nest("& [data-w-error]", pairs(&[("margin", "0"), ("color", crate::DESTRUCTIVE), ("font-size", fs!(13)), ("line-height", "1.45")]))
        .nest("& [data-w-error]:empty", pairs(&[("display", "none")]))
        .nest("& [data-w-card]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(10)), ("box-sizing", "border-box"), ("min-width", "0"), ("padding", pad!(14)), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-card, var(--radius-lg))"), ("background", crate::BG_CARD)]))
        .nest("& [data-w-title]", pairs(&[("margin", "0"), ("font-size", fs!(14)), ("font-weight", "var(--font-weight-semibold, 600)")]))
        .nest("& [data-w-list]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("margin", "0"), ("padding", "0"), ("list-style", "none"), ("min-width", "0")]))
        .nest("& [data-w-list] > li", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", gap!(10)), ("min-width", "0"), ("padding", pad!(9, 0)), ("border-top", "1px solid var(--color-border)")]))
        .nest("& [data-w-list] > li:first-child", pairs(&[("border-top", "0")]))
        .nest("& [data-w-main]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("flex", "1 1 0"), ("min-width", "0")]))
        .nest("& [data-w-main] > *", pairs(&[("overflow-wrap", "anywhere")]))
        .nest("& [data-w-tag]", pairs(&[("display", "inline-flex"), ("align-items", "center"), ("gap", gap!(5)), ("flex", "none"), ("padding", pad!(2, 8)), ("border-radius", "999px"), ("background", "color-mix(in srgb, var(--w-tone, var(--color-muted)) 14%, transparent)"), ("color", "color-mix(in srgb, var(--w-tone, var(--color-muted)) 78%, var(--color-text))"), ("font-size", fs!(12)), ("font-weight", "var(--font-weight-medium, 500)"), ("white-space", "nowrap")]))
        .nest("& [data-tone=good]", pairs(&[("--w-tone", "var(--color-success, #16a34a)")]))
        .nest("& [data-tone=warn]", pairs(&[("--w-tone", "var(--color-warning, #d97706)")]))
        .nest("& [data-tone=bad]", pairs(&[("--w-tone", "var(--color-destructive, #dc2626)")]))
        .nest("& [data-tone=info]", pairs(&[("--w-tone", "var(--color-info, #2563eb)")]))
        .nest("& [data-w-field]", pairs(&[("box-sizing", "border-box"), ("min-width", "0"), ("height", ctl!(36)), ("padding", pad!(0, 10)), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-control, var(--radius-md))"), ("background", SURFACE), ("color", TEXT), ("font", "inherit"), ("font-size", fs!(14))]))
        .nest("& dialog", pairs(&[("box-sizing", "border-box"), ("width", "min(400px, calc(100vw - 32px))"), ("max-height", "calc(100dvh - 32px)"), ("padding", pad!(20)), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-card, var(--radius-lg))"), ("background", crate::BG_DIALOG), ("color", TEXT), ("font-family", "var(--font-body, system-ui, sans-serif)"), ("font-size", fs!(14)), ("line-height", "1.5"), ("box-shadow", "var(--elevation-dialog, 0 16px 40px rgb(0 0 0 / 0.25))")]))
        .nest("& dialog::backdrop", pairs(&[("background", "rgb(0 0 0 / 0.55)")]))
        .nest("& dialog h2", pairs(&[("margin", "0 0 10px"), ("font-size", fs!(17)), ("font-weight", "var(--font-weight-semibold, 600)")]))
        .nest("& dialog ol", pairs(&[("margin", "0 0 16px"), ("padding-inline-start", "20px"), ("display", "flex"), ("flex-direction", "column"), ("gap", gap!(6))]))
        .nest("& progress", pairs(&[("width", "100%"), ("height", "6px"), ("accent-color", PRIMARY)]))
}

/// A column, as wide as there's room for.
pub(crate) fn column(gap: &'static str, max: &'static str) -> StyleBlock {
    pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap), ("width", "100%"), ("max-width", max), ("min-width", "0"), ("color", crate::TEXT), ("font-size", fs!(14))])
}

/// Adds `device.js` (and `kit.js`) before a behaviour's own script.
pub(crate) fn device_kit(cx: &mut RenderCx<'_>) {
    kit(cx);
    cx.global_js("ui-device", include_str!("device.js"));
}

/// A component its behaviour draws: an element of `tag` with the behaviour `name`.
pub(crate) fn drawn<'n>(cx: &mut RenderCx<'_>, node: &'n Node, tag: &str, style: &'n StyleBlock, name: &'static str, config: Map<String, Json>, attrs: Vec<(&str, irender_core::Val)>) -> Result<(), RenderError> {
    let mut el = cx.element(tag, node);
    el.base_style_ref(style).behavior(name, Json::Object(config));
    for (k, v) in attrs {
        el.attr(k, v);
    }
    cx.open(&el, node)?;
    cx.close(tag);
    Ok(())
}

/// A required state key.
pub(crate) fn need(cx: &RenderCx<'_>, node: &Node, prop: &str) -> Result<String, RenderError> {
    cx.prop_string(node, prop).ok_or_else(|| RenderError::new(node, format!("`{}` needs `{prop}`", node.kind)))
}

/// Booleans into a config, with their defaults.
pub(crate) fn flags(cx: &RenderCx<'_>, node: &Node, config: &mut Map<String, Json>, list: &[(&str, bool)]) {
    for (f, d) in list {
        config.insert(f.to_string(), json!(flag(cx, node, f, *d)));
    }
}
