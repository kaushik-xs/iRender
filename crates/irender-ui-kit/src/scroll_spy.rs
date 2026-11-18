//! `ui-scroll-spy`: marks the in-page link (`href="#id"`) for the section being read,
//! for a table of contents or a docs sidebar. `scroll_spy.js` gives that link
//! `aria-current="location"` as the page scrolls, and keeps it in view inside a list
//! that scrolls on its own.

use crate::{ACCENT, TEXT, lit};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, expr};
use irender_html::{Component, RenderCx, RenderError};
use serde_json::json;
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("scroll_spy.js");

pub struct ScrollSpy;

const PROPS: &[PropDef] = &[
    PropDef::new("offset", PropKind::Any, "Pixels covered at the top of the window, e.g. by a sticky header (default 0)"),
    PropDef::new("label", PropKind::Any, "Names the navigation for screen readers (default `On this page`)"),
    PropDef::new("tag", PropKind::Tag, "HTML tag (default `nav`)"),
];

static STYLE: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[])
        .nest("& a[aria-current=location]", StyleBlock::from_pairs(&[("color", TEXT), ("font-weight", "600"), ("background", ACCENT)]))
});

impl Component for ScrollSpy {
    fn name(&self) -> &'static str {
        "ui-scroll-spy"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Navigation whose in-page links (`href=\"#id\"`) follow the reading: the link for the section in view gets `aria-current=\"location\"` and stays in view. Children are the links, in any layout.",
            props: PROPS,
            parts: &[],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let offset = cx.prop(node, "offset").map_or(0.0, |v| expr::to_num(&v)).max(0.0);
        let tag = cx.prop_string(node, "tag").unwrap_or_else(|| "nav".into());
        cx.global_js("ui-scroll-spy", SCRIPT);
        let mut el = cx.element(&tag, node);
        el.base_style_ref(&STYLE)
            .behavior("ui-scroll-spy", json!({ "offset": offset }))
            .attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit("On this page")));
        cx.open(&el, node)?;
        cx.children(node)?;
        cx.close(&tag);
        Ok(())
    }
}
