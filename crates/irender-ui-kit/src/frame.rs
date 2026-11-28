//! `ui-iframe`: another page shown inside this one, sandboxed, lazy and sized. `src` and
//! `title` follow state through the runtime's attribute bindings, which drop an unsafe URL
//! as they do for any `src`; `frame.js` tells when it's loaded (or failed), sizes it to
//! its page with `height: auto`, and passes messages both ways. Under the untrusted
//! policy it's refused, unless the policy allows sandboxed frames: then its address is a
//! fixed `https:` one, its sandbox can't take `same-origin` or a way out of it, and it
//! may use only a few features.

use crate::{BG_UNDER, BORDER, choice, lit, server_array, widget};
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

/// Under the untrusted policy's sandboxed frames: what `sandbox` may still allow (none of
/// them lets the page out of its sandbox or at this one's origin), and its default.
const POLICED_ALLOWANCES: &[&str] = &["scripts", "forms", "popups", "presentation", "pointer-lock", "orientation-lock"];
const POLICED_SANDBOX: &str = "allow-scripts allow-forms allow-popups";
/// Attributes `attrs` may not set on it then: they'd widen, or go round, the props' checks.
const POLICED_ATTRS: &[&str] = &["sandbox", "allow", "allowfullscreen", "allowpaymentrequest", "src", "csp", "credentialless", "referrerpolicy"];
/// And the browser features it may ask for.
const POLICED_FEATURES: &[&str] = &["fullscreen", "picture-in-picture", "autoplay", "encrypted-media"];

const PROPS: &[PropDef] = &[
    PropDef::new("src", PropKind::Any, "The page's address; may use state (under the untrusted policy, with sandboxed frames allowed: a fixed `https:` one)").required(),
    PropDef::new("title", PropKind::Any, "Names the frame for screen readers (default `Embedded page`)"),
    PropDef::new("height", PropKind::Any, "Number (px) or CSS length (default 400), or `auto`: as tall as its page says, by a message `{ height }` from it (a page of this site is measured); ignored with `ratio`"),
    PropDef::new("minHeight", PropKind::Number, "With `height: auto`: the height before its page says, and at least (default 150)"),
    PropDef::new("ratio", PropKind::Any, "Width / height instead of a height: `\"16 / 9\"`, `1.5`"),
    PropDef::new("sandbox", PropKind::Any, "What the page may do: a list from `scripts`, `same-origin`, `forms`, `popups`, `popups-to-escape-sandbox`, `modals`, `downloads`, `presentation`, `pointer-lock`, `orientation-lock`, `top-navigation-by-user-activation`, `storage-access-by-user-activation`; `[]` for nothing, `false` for no sandbox (default `[scripts, same-origin, forms, popups, popups-to-escape-sandbox]`)"),
    PropDef::new("allow", PropKind::Any, "Browser features it may use: a list like `[fullscreen, clipboard-write, camera]`, or the `allow` attribute's text (default `[fullscreen]`)"),
    PropDef::new("loading", PropKind::Enum(LOADING), "Default `lazy`: loaded as it nears the screen"),
    PropDef::new("referrerPolicy", PropKind::Any, "Default `strict-origin-when-cross-origin`"),
    PropDef::new("bordered", PropKind::Bool, "A border and rounded corners (default true)"),
    PropDef::new("timeout", PropKind::Number, "Seconds to wait for its page before it's `failed` (default 20; 0 waits for ever)"),
    PropDef::new("bindStatus", PropKind::StateKey, "State key that gets `loading`, `ready` or `failed`; `load` and `failed` events fire too"),
    PropDef::new("bindMessage", PropKind::StateKey, "State key that gets each message its page posts (a `message` event fires too)"),
    PropDef::new("send", PropKind::Any, "Posted to its page, to its origin only, once it's loaded and each time this changes; may use state"),
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

/// The `sandbox` attribute, or `None` for none at all. `policed`: under the untrusted
/// policy's sandboxed frames, which narrow what it takes.
fn sandbox(cx: &RenderCx<'_>, node: &Node, policed: bool) -> Result<Option<String>, RenderError> {
    match cx.prop(node, "sandbox") {
        None | Some(Json::Null) | Some(Json::Bool(true)) => return Ok(Some((if policed { POLICED_SANDBOX } else { DEFAULT_SANDBOX }).into())),
        Some(Json::Bool(false)) if policed => return Err(RenderError::new(node, "the policy keeps a frame sandboxed: `sandbox: false` isn't allowed")),
        Some(Json::Bool(false)) => return Ok(None),
        _ => {}
    }
    let allowed = if policed { POLICED_ALLOWANCES } else { ALLOWANCES };
    let mut tokens = Vec::new();
    for item in server_array(cx, node, "sandbox")? {
        let name = expr::stringify(&item);
        let name = name.strip_prefix("allow-").unwrap_or(&name);
        if !allowed.contains(&name) {
            let under = if policed && ALLOWANCES.contains(&name) { " under this policy" } else { "" };
            return Err(RenderError::new(node, format!("`sandbox` can't allow `{name}`{under}; it takes {}", allowed.join(", "))));
        }
        tokens.push(format!("allow-{name}"));
    }
    Ok(Some(tokens.join(" ")))
}

fn allow(cx: &RenderCx<'_>, node: &Node, policed: bool) -> Result<String, RenderError> {
    let list: Vec<String> = match cx.prop(node, "allow") {
        None | Some(Json::Null) => return Ok("fullscreen".into()),
        Some(Json::String(s)) if !policed => return Ok(s),
        Some(Json::String(s)) => s.split(';').map(|f| f.trim().to_string()).filter(|f| !f.is_empty()).collect(),
        Some(_) => server_array(cx, node, "allow")?.iter().map(expr::stringify).collect(),
    };
    // Under the policy: a feature on its own, for this frame's page only (no origins after it).
    if policed && let Some(bad) = list.iter().find(|f| !POLICED_FEATURES.contains(&f.as_str())) {
        return Err(RenderError::new(node, format!("`allow` can't ask for `{bad}` under this policy; it takes {}", POLICED_FEATURES.join(", "))));
    }
    Ok(list.join("; "))
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
        // The untrusted policy refuses frames, unless it allows sandboxed ones: then the
        // address is fixed and `https:`, and the sandbox and features narrowed.
        let policed = cx.policy().frames_policed();
        if policed {
            match src {
                Val::Lit(Json::String(u)) if u.trim().get(..8).is_some_and(|p| p.eq_ignore_ascii_case("https://")) && u.trim().len() > 8 => {}
                Val::Lit(_) => return Err(RenderError::new(node, "under this policy a frame's `src` is an `https:` address")),
                Val::Tpl(_) => return Err(RenderError::new(node, "under this policy a frame's `src` is a fixed `https:` address, not one from state")),
            }
            // `attrs` go on the `<iframe>` too: none of them may widen what the props allow.
            if let Some(bad) = node.attrs.keys().find(|k| POLICED_ATTRS.contains(&k.to_ascii_lowercase().as_str())) {
                return Err(RenderError::new(node, format!("under this policy a frame's `{bad}` can't be set in `attrs`: use its props, which keep it in the sandbox")));
            }
        }
        let sandbox = sandbox(cx, node, policed)?;
        let allow = allow(cx, node, policed)?;
        let auto = cx.prop_string(node, "height").as_deref() == Some("auto");
        let mut config = widget::config(cx, node, &["send"], &["timeout", "minHeight"], &["bindStatus", "bindMessage"])?;
        if auto {
            config.insert("auto".into(), Json::Bool(true));
        }

        let mut size = StyleBlock::new();
        match node.prop("ratio") {
            Some(Val::Tpl(t)) => size.push("aspect-ratio", StyleValue::Dynamic { tpl: t.clone(), px: false }),
            Some(Val::Lit(v)) if !v.is_null() => {
                let ratio = expr::stringify(v);
                irender_core::policy::css_value_is_contained(&ratio).map_err(|e| RenderError::new(node, e))?;
                size.push("aspect-ratio", StyleValue::Static(ratio));
            }
            _ if auto => {
                let min = widget::number(cx, node, "minHeight").unwrap_or(150.0).max(0.0);
                size.push("height", StyleValue::Static(format!("{min}px")));
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
        widget::kit(cx);
        cx.global_js("ui-iframe", include_str!("frame.js"));
        el.attr("data-loading", true).behavior("ui-iframe", Json::Object(config));
        cx.open_sandboxed_frame(&el, node)?;
        cx.close("iframe");
        Ok(())
    }
}
