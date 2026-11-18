//! The toaster: a stack of notifications, each hiding on its own timer. Setting the
//! bound state key adds one; a client behaviour (toaster.js) does the rest.

use crate::{BG_POPOVER, ACCENT, DESTRUCTIVE, MUTED, TEXT, lit};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::json;
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("toaster.js");

pub struct Toaster;

const TOASTER_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key to set to a toast, `{ title, description, variant, duration }`, to show it")
        .required(),
    PropDef::new("duration", PropKind::Number, "Milliseconds before a toast hides (default 4000); 0 keeps it until dismissed"),
    PropDef::new("max", PropKind::Number, "Toasts shown at once (default 3); the oldest goes first"),
];

static REGION: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("position", "fixed"),
        ("right", "16px"),
        ("bottom", "16px"),
        ("z-index", "100"),
        ("width", "min(360px, calc(100% - 32px))"),
        ("display", "flex"),
        ("flex-direction", "column"),
        ("gap", sp!(8)),
    ])
});
static TOAST: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("box-sizing", "border-box"),
        ("position", "relative"),
        ("display", "grid"),
        ("gap", sp!(4)),
        ("padding", sp!(16, 40, 16, 16)),
        ("border", "1px solid var(--color-border)"),
        ("border-radius", "var(--radius-lg)"),
        ("--ui-bg", BG_POPOVER),
        ("background", BG_POPOVER),
        ("color", TEXT),
        ("font-size", fs!(14)),
        ("box-shadow", "0 10px 30px rgb(0 0 0 / 0.15)"),
    ])
    .nest(
        "&[data-variant=destructive]",
        StyleBlock::from_pairs(&[("background", DESTRUCTIVE), ("color", "var(--color-on-destructive, #fff)"), ("border-color", DESTRUCTIVE)]),
    )
});
static TITLE: LazyLock<StyleBlock> = LazyLock::new(|| StyleBlock::from_pairs(&[("font-weight", "600")]));
static DESCRIPTION: LazyLock<StyleBlock> = LazyLock::new(|| StyleBlock::from_pairs(&[("opacity", "0.9"), ("color", "inherit")]));
static CLOSE: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("position", "absolute"),
        ("top", "8px"),
        ("right", "8px"),
        ("width", "24px"),
        ("height", sp!(24)),
        ("border", "0"),
        ("border-radius", "var(--radius-sm)"),
        ("background", "transparent"),
        ("color", "inherit"),
        ("opacity", "0.6"),
        ("cursor", "pointer"),
    ])
    .nest(":hover", StyleBlock::from_pairs(&[("opacity", "1"), ("background", ACCENT), ("color", MUTED)]))
});

impl Component for Toaster {
    fn name(&self) -> &'static str {
        "ui-toaster"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Shows toasts in a corner of the screen. Set `bind` to a toast to add it to the stack; each one hides after \
                  its `duration`, and the key is cleared for the next.",
            props: TOASTER_PROPS,
            parts: &["toast"],
            children: false,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let Some(key) = cx.prop_string(node, "bind") else { return Err(RenderError::new(node, "`ui-toaster` needs `bind`")) };
        let number = |name: &str, default: f64| cx.prop(node, name).and_then(|v| v.as_f64()).filter(|n| *n >= 0.0).unwrap_or(default);
        let config = json!({ "key": key, "duration": number("duration", 4000.0), "max": number("max", 3.0).max(1.0) });
        cx.global_js("ui-toaster", SCRIPT);
        let mut region = cx.element("div", node);
        region.base_style_ref(&REGION).attr("role", "status").attr("aria-live", "polite").behavior("ui-toaster", config);
        cx.open(&region, node)?;

        // One toast, copied by the behaviour for each one it shows.
        cx.open(&Element::new("template"), node)?;
        let mut toast = cx.part("div", node, "toast");
        toast.base_style_ref(&TOAST);
        cx.open(&toast, node)?;
        for (f, style) in [("title", &*TITLE), ("description", &*DESCRIPTION)] {
            let mut el = Element::new("div");
            el.base_style_ref(style).attr(&format!("data-{f}"), lit(""));
            cx.open(&el, node)?;
            cx.close("div");
        }
        let mut close = Element::new("button");
        close
            .base_style_ref(&CLOSE)
            .attr("type", "button")
            .attr("aria-label", "Dismiss")
            .attr("data-dismiss", lit(""))
            .text(lit("✕"));
        cx.open(&close, node)?;
        cx.close("button");
        cx.close("div");
        cx.close("template");
        cx.close("div");
        Ok(())
    }
}
