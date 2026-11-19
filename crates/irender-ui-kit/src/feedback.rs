//! The toaster: a stack of notifications, each hiding on its own timer. Setting the
//! bound state key adds one; a client behaviour (toaster.js) does the rest.

use crate::{BG_POPOVER, MUTED, TEXT, lit};
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
        ("inset-inline-end", "16px"),
        ("bottom", "16px"),
        ("z-index", "100"),
        ("width", "min(360px, calc(100% - 32px))"),
        ("display", "flex"),
        ("flex-direction", "column"),
        ("gap", sp!(8)),
    ])
});
const TOAST_IN: &str = "@keyframes ui-toast-in {\n  from { opacity: 0; transform: translateY(8px) scale(0.98); }\n}";

static TOAST: LazyLock<StyleBlock> = LazyLock::new(|| {
    let mut b = StyleBlock::from_pairs(&[
        ("--tone", MUTED),
        ("--wash", "0%"),
        ("--tint", "0%"),
        ("box-sizing", "border-box"),
        ("position", "relative"),
        ("overflow", "hidden"),
        ("display", "grid"),
        ("grid-template-columns", "1fr"),
        ("column-gap", sp!(12)),
        ("row-gap", sp!(2)),
        ("align-items", "start"),
        ("padding-block", sp!(14, 14)), ("padding-inline", sp!(16, 40)),
        ("border", "1px solid color-mix(in srgb, var(--tone) var(--tint), var(--color-border))"),
        ("border-radius", "var(--radius-lg)"),
        ("--ui-bg", &format!("color-mix(in srgb, var(--tone) var(--wash), {BG_POPOVER})")),
        ("background", "var(--ui-bg)"),
        ("color", TEXT),
        ("font-size", fs!(14)),
        ("line-height", "20px"),
        ("box-shadow", "0 1px 2px rgb(0 0 0 / 0.06), 0 12px 32px -8px rgb(0 0 0 / 0.25)"),
        ("animation", "ui-toast-in 200ms cubic-bezier(0.2, 0.8, 0.2, 1)"),
    ])
    // The text sits in the last column, beside the variant's icon when it has one.
    .nest("& > div", StyleBlock::from_pairs(&[("grid-column", "-2")]))
    .nest(
        "& [data-icon]",
        StyleBlock::from_pairs(&[
            ("display", "none"),
            ("grid-column", "1"),
            ("grid-row", "1 / span 2"),
            ("height", "20px"),
            ("align-items", "center"),
            ("color", "var(--tone)"),
        ]),
    )
    .nest(
        "& [data-progress]",
        StyleBlock::from_pairs(&[
            ("position", "absolute"),
            ("inset-inline", "0"),
            ("bottom", "0"),
            ("height", "3px"),
            ("background", "var(--tone)"),
            ("opacity", "0.45"),
            ("transform-origin", "left"),
        ]),
    )
    .nest(":dir(rtl) [data-progress]", StyleBlock::from_pairs(&[("transform-origin", "right")]))
    .nest("@media (prefers-reduced-motion: reduce)", StyleBlock::from_pairs(&[("animation", "none")]));
    for (name, color, _) in crate::NOTICES {
        b = b
            .nest(
                format!("&[data-variant={name}]"),
                StyleBlock::from_pairs(&[("--tone", color), ("--wash", "6%"), ("--tint", "30%"), ("grid-template-columns", "auto 1fr")]),
            )
            .nest(format!("&[data-variant={name}] [data-icon={name}]"), StyleBlock::from_pairs(&[("display", "flex")]));
    }
    b
});
static TITLE: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[("font-weight", "600"), ("color", "color-mix(in srgb, var(--tone) calc(var(--tint) + 5%), var(--color-text))")])
});
static DESCRIPTION: LazyLock<StyleBlock> =
    LazyLock::new(|| StyleBlock::from_pairs(&[("color", "color-mix(in srgb, var(--color-text) 78%, var(--ui-bg))")]));
static CLOSE: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("position", "absolute"),
        ("top", "12px"),
        ("inset-inline-end", "10px"),
        ("width", "24px"),
        ("height", sp!(24)),
        ("display", "inline-flex"),
        ("align-items", "center"),
        ("justify-content", "center"),
        ("padding", "0"),
        ("border", "0"),
        ("border-radius", "var(--radius-sm)"),
        ("background", "transparent"),
        ("color", "inherit"),
        ("font-size", fs!(12)),
        ("opacity", "0.6"),
        ("cursor", "pointer"),
    ])
    .nest(":hover", StyleBlock::from_pairs(&[("opacity", "1"), ("background", "color-mix(in srgb, var(--tone) 14%, transparent)")]))
    .nest(":focus-visible", StyleBlock::from_pairs(&[("opacity", "1"), ("outline", "2px solid var(--color-ring)"), ("outline-offset", "1px")]))
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
        cx.global_css("ui-toast-in", TOAST_IN);
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
        // Each variant's icon, shown by the toast's `data-variant`, then the bar that
        // counts down its time.
        for (name, _, paths) in crate::NOTICES {
            cx.markup(&format!(r#"<span aria-hidden="true" data-icon="{name}">{}</span>"#, crate::notice_icon(paths, "")));
        }
        cx.markup(r#"<span aria-hidden="true" data-progress></span>"#);
        let mut close = Element::new("button");
        close
            .base_style_ref(&CLOSE)
            .attr("type", "button")
            .attr("aria-label", cx.label("Dismiss"))
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
