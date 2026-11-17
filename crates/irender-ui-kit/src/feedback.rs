//! The toaster: one notification at a time, driven by a state key.

use crate::{ACCENT, DESTRUCTIVE, MUTED, SURFACE, TEXT, expr_val, lit, state_not, synthetic};
use irender_core::{Action, ComponentInfo, Node, PropDef, PropKind, StyleBlock, Val, expr::Expr};
use irender_html::{Component, RenderCx, RenderError};
use serde_json::Value as Json;
use std::sync::LazyLock;

pub struct Toaster;

const TOASTER_PROPS: &[PropDef] = &[PropDef::new(
    "bind",
    PropKind::StateKey,
    "State key holding the toast, `{ title, description, variant }`, or null for none",
)
.required()];

static REGION: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("position", "fixed"),
        ("right", "16px"),
        ("bottom", "16px"),
        ("z-index", "100"),
        ("width", "min(360px, calc(100% - 32px))"),
    ])
});
static TOAST: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("box-sizing", "border-box"),
        ("position", "relative"),
        ("display", "grid"),
        ("gap", "4px"),
        ("padding", "16px 40px 16px 16px"),
        ("border", "1px solid var(--color-border)"),
        ("border-radius", "var(--radius-lg)"),
        ("background", SURFACE),
        ("color", TEXT),
        ("font-size", "14px"),
        ("box-shadow", "0 10px 30px rgb(0 0 0 / 0.15)"),
    ])
    .nest("&[hidden]", StyleBlock::from_pairs(&[("display", "none")]))
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
        ("height", "24px"),
        ("border", "0"),
        ("border-radius", "var(--radius-sm)"),
        ("background", "transparent"),
        ("color", "inherit"),
        ("opacity", "0.6"),
        ("cursor", "pointer"),
    ])
    .nest(":hover", StyleBlock::from_pairs(&[("opacity", "1"), ("background", ACCENT), ("color", MUTED)]))
});

/// `{{ key.field }}`
fn field(key: &str, f: &str) -> Val {
    expr_val(Expr::Path(vec![key.to_string(), f.to_string()]))
}

impl Component for Toaster {
    fn name(&self) -> &'static str {
        "ui-toaster"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Shows the toast in `bind` in a corner of the screen. Set the key to show one, null to hide it; \
                  a `wait` action between the two hides it after a while.",
            props: TOASTER_PROPS,
            parts: &["toast"],
            children: false,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let Some(key) = cx.prop_string(node, "bind") else { return Err(RenderError::new(node, "`ui-toaster` needs `bind`")) };
        let mut region = cx.element("div", node);
        region.base_style_ref(&REGION).attr("role", "status").attr("aria-live", "polite");
        cx.open(&region, node)?;

        let mut toast = cx.part("div", node, "toast");
        toast.base_style_ref(&TOAST).attr("hidden", state_not(&key)).attr("data-variant", field(&key, "variant"));
        cx.open(&toast, node)?;
        for (f, style) in [("title", &*TITLE), ("description", &*DESCRIPTION)] {
            let mut el = irender_html::Element::new("div");
            el.base_style_ref(style).text(field(&key, f));
            cx.open(&el, node)?;
            cx.close("div");
        }
        let close = synthetic(
            node,
            "button",
            Vec::new(),
            vec![("type", lit("button")), ("aria-label", lit("Dismiss"))],
            vec![("click", vec![Action::SetState { key: key.clone(), value: Val::Lit(Json::Null) }])],
        );
        let mut b = cx.element("button", &close);
        b.base_style_ref(&CLOSE).text(lit("✕"));
        cx.open(&b, &close)?;
        cx.close("button");
        cx.close("div");
        cx.close("div");
        Ok(())
    }
}
