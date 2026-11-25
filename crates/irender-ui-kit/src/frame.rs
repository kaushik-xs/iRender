//! `ui-iframe`: another page shown inside this one, sandboxed, lazy and sized. No
//! script: `src` and `title` follow state through the runtime's attribute bindings,
//! which drop an unsafe URL as they do for any `src`.

use crate::{BG_UNDER, BORDER, choice, lit, server_array};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, StyleValue, Val, expr};
use irender_html::{Component, RenderCx, RenderError};
use serde_json::Value as Json;
use std::sync::LazyLock;

pub struct Iframe;

/// What `sandbox` may allow, without the `allow-` the attribute's tokens start with.
const ALLOWANCES: &[&str] = &[
    "scripts",
    "same-origin",
    "forms",
    "popups",
    "popups-to-escape-sandbox",
    "modals",
    "downloads",
    "presentation",
    "pointer-lock",
    "orientation-lock",
    "top-navigation-by-user-activation",
    "storage-access-by-user-activation",
];
/// Enough for most embeds (maps, videos, forms, dashboards) to work, while the frame
/// can't navigate this page or open dialogs over it.
const DEFAULT_SANDBOX: &str = "allow-scripts allow-same-origin allow-forms allow-popups allow-popups-to-escape-sandbox";
const LOADING: &[&str] = &["lazy", "eager"];

const PROPS: &[PropDef] = &[
    PropDef::new("src", PropKind::Any, "The page's address; may use state").required(),
    PropDef::new("title", PropKind::Any, "Names the frame for screen readers (default `Embedded page`)"),
    PropDef::new("height", PropKind::Any, "Number (px) or CSS length (default 400); ignored with `ratio`"),
    PropDef::new("ratio", PropKind::Any, "Width / height instead of a height: `\"16 / 9\"`, `1.5`"),
    PropDef::new("sandbox", PropKind::Any, "What the page may do: a list from `scripts`, `same-origin`, `forms`, `popups`, `popups-to-escape-sandbox`, `modals`, `downloads`, `presentation`, `pointer-lock`, `orientation-lock`, `top-navigation-by-user-activation`, `storage-access-by-user-activation`; `[]` for nothing, `false` for no sandbox (default `[scripts, same-origin, forms, popups, popups-to-escape-sandbox]`)"),
    PropDef::new("allow", PropKind::Any, "Browser features it may use: a list like `[fullscreen, clipboard-write, camera]`, or the `allow` attribute's text (default `[fullscreen]`)"),
    PropDef::new("loading", PropKind::Enum(LOADING), "Default `lazy`: loaded as it nears the screen"),
    PropDef::new("referrerPolicy", PropKind::Any, "Default `strict-origin-when-cross-origin`"),
    PropDef::new("bordered", PropKind::Bool, "A border and rounded corners (default true)"),
];

static FRAME: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("display", "block"),
        ("box-sizing", "border-box"),
        ("width", "100%"),
        ("max-width", "100%"),
        ("border", "0"),
        // Shows while the page loads, and behind one that's transparent.
        ("background", BG_UNDER),
        ("color-scheme", "normal"),
    ])
});
static BORDERED: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("border", "var(--border-width, 1px) solid"),
        ("border-color", BORDER),
        ("border-radius", "var(--radius-card, var(--radius-md))"),
    ])
});

/// The `sandbox` attribute, or `None` for none at all.
fn sandbox(cx: &RenderCx<'_>, node: &Node) -> Result<Option<String>, RenderError> {
    match cx.prop(node, "sandbox") {
        None | Some(Json::Null) | Some(Json::Bool(true)) => return Ok(Some(DEFAULT_SANDBOX.into())),
        Some(Json::Bool(false)) => return Ok(None),
        _ => {}
    }
    let mut tokens = Vec::new();
    for item in server_array(cx, node, "sandbox")? {
        let name = expr::stringify(&item);
        let name = name.strip_prefix("allow-").unwrap_or(&name);
        if !ALLOWANCES.contains(&name) {
            return Err(RenderError::new(node, format!("`sandbox` can't allow `{name}`; it takes {}", ALLOWANCES.join(", "))));
        }
        tokens.push(format!("allow-{name}"));
    }
    Ok(Some(tokens.join(" ")))
}

fn allow(cx: &RenderCx<'_>, node: &Node) -> Result<String, RenderError> {
    match cx.prop(node, "allow") {
        None | Some(Json::Null) => Ok("fullscreen".into()),
        Some(Json::String(s)) => Ok(s),
        Some(_) => Ok(server_array(cx, node, "allow")?.iter().map(expr::stringify).collect::<Vec<_>>().join("; ")),
    }
}

impl Component for Iframe {
    fn name(&self) -> &'static str {
        "ui-iframe"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Another page shown inside this one: sandboxed, loaded as it nears the screen, at a height or a ratio.",
            props: PROPS,
            parts: &[],
            children: false,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let Some(src) = node.prop("src") else {
            return Err(RenderError::new(node, "`ui-iframe` needs `src`"));
        };
        let sandbox = sandbox(cx, node)?;
        let allow = allow(cx, node)?;

        let mut size = StyleBlock::new();
        match node.prop("ratio") {
            Some(Val::Tpl(t)) => size.push("aspect-ratio", StyleValue::Dynamic { tpl: t.clone(), px: false }),
            Some(Val::Lit(v)) if !v.is_null() => {
                let ratio = expr::stringify(v);
                irender_core::policy::css_value_is_contained(&ratio).map_err(|e| RenderError::new(node, e))?;
                size.push("aspect-ratio", StyleValue::Static(ratio));
            }
            _ => {
                let height = node.prop("height").cloned().unwrap_or_else(|| lit(400));
                if let Some(v) = cx.style_value(node, "height", &height)? {
                    size.push("height", v);
                }
            }
        }

        let mut el = cx.element("iframe", node);
        el.base_style_ref(&FRAME);
        if cx.prop(node, "bordered") != Some(Json::Bool(false)) {
            el.base_style_ref(&BORDERED);
        }
        el.base_style(size)
            .attr("src", src.clone())
            .attr("title", node.prop("title").cloned().unwrap_or_else(|| lit(cx.label("Embedded page"))))
            .attr("loading", LOADING[choice(cx, node, "loading", LOADING)])
            .attr("referrerpolicy", node.prop("referrerPolicy").cloned().unwrap_or_else(|| lit("strict-origin-when-cross-origin")));
        if let Some(s) = sandbox {
            el.attr("sandbox", s);
        }
        if !allow.is_empty() {
            el.attr("allow", allow);
        }
        cx.open(&el, node)?;
        cx.close("iframe");
        Ok(())
    }
}
