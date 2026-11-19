//! The carousel: a row of slides that scrolls sideways, a slide at a time, with
//! previous and next buttons. The browser does the scrolling and snapping; a small
//! behaviour (carousel.js) drives the buttons and the arrow keys.

use crate::{BORDER, SURFACE, TEXT, lit};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock};
use irender_html::{Component, RenderCx, RenderError};
use serde_json::json;
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("carousel.js");

pub struct Carousel;

const CAROUSEL_PROPS: &[PropDef] = &[
    PropDef::new("label", PropKind::Any, "Names the carousel for screen readers (default `Carousel`)"),
    PropDef::new("size", PropKind::Any, "Each slide's width: number (px) or CSS length; default `min(280px, 85%)`"),
    PropDef::new("gap", PropKind::Any, "Space between slides: number (px) or CSS length; default 12"),
];

static CAROUSEL: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[("position", "relative"), ("min-width", "0"), ("--ui-slide", "min(280px, 85%)"), ("--ui-slide-gap", sp!(12))])
});
static TRACK: LazyLock<StyleBlock> = LazyLock::new(|| {
    // A grid, so slides inside a `display: contents` wrapper (a `list`) are sized too.
    StyleBlock::from_pairs(&[
        ("display", "grid"),
        ("grid-auto-flow", "column"),
        ("grid-auto-columns", "var(--ui-slide)"),
        ("gap", "var(--ui-slide-gap)"),
        ("overflow-x", "auto"),
        ("overscroll-behavior-x", "contain"),
        ("scroll-snap-type", "x mandatory"),
        ("scrollbar-width", "none"),
        ("border-radius", "var(--radius-lg)"),
    ])
    .nest("&::-webkit-scrollbar", StyleBlock::from_pairs(&[("display", "none")]))
    .nest("& [data-slide]", StyleBlock::from_pairs(&[("min-width", "0"), ("scroll-snap-align", "start")]))
    .nest(":focus-visible", StyleBlock::from_pairs(&[("outline", "2px solid var(--color-ring, var(--color-muted))"), ("outline-offset", "2px")]))
});
static ARROW: LazyLock<[StyleBlock; 2]> = LazyLock::new(|| {
    let arrow = |side: &str| {
        StyleBlock::from_pairs(&[
            ("position", "absolute"),
            ("top", "50%"),
            (side, sp!(8)),
            ("transform", "translateY(-50%)"),
            ("display", "inline-flex"),
            ("align-items", "center"),
            ("justify-content", "center"),
            ("width", "32px"),
            ("height", "32px"),
            ("padding", "0"),
            ("border", "1px solid"),
            ("border-color", BORDER),
            ("border-radius", "999px"),
            ("background", SURFACE),
            ("color", TEXT),
            ("font-size", fs!(18)),
            ("line-height", "1"),
            ("box-shadow", "0 2px 8px rgb(0 0 0 / 0.15)"),
            ("cursor", "pointer"),
        ])
        .nest(":disabled", StyleBlock::from_pairs(&[("display", "none")]))
        .nest(":focus-visible", StyleBlock::from_pairs(&[("outline", "2px solid var(--color-ring, var(--color-muted))"), ("outline-offset", "2px")]))
    };
    [arrow("left"), arrow("right")]
});

impl Component for Carousel {
    fn name(&self) -> &'static str {
        "ui-carousel"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Slides in a row that scrolls sideways and snaps to each one: images, cards, anything. Its buttons, \
                  Left and Right on the focused track, and swiping move a slide at a time. Children are the slides.",
            props: CAROUSEL_PROPS,
            parts: &["track", "previous", "next"],
            children: true,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut sizing = StyleBlock::new();
        // Converted as the lengths they are, so numbers gain `px`.
        for (prop, as_property, var) in [("size", "width", "--ui-slide"), ("gap", "gap", "--ui-slide-gap")] {
            if let Some(v) = node.prop(prop)
                && let Some(v) = cx.style_value(node, as_property, v)?
            {
                sizing.push(var, v);
            }
        }
        cx.global_js("ui-carousel", SCRIPT);
        let mut el = cx.element("section", node);
        el.base_style_ref(&CAROUSEL)
            .base_style(sizing)
            .attr("aria-roledescription", cx.label("carousel"))
            .attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Carousel"))))
            .behavior("ui-carousel", json!({}));
        cx.open(&el, node)?;
        let mut track = cx.part("div", node, "track");
        track.base_style_ref(&TRACK).attr("tabindex", "0").attr("data-track", lit(""));
        cx.open(&track, node)?;
        cx.children(node)?;
        cx.close("div");
        for (i, (part, label, glyph, marker)) in
            [("previous", "Previous slide", "‹", "data-prev"), ("next", "Next slide", "›", "data-next")].into_iter().enumerate()
        {
            let mut b = cx.part("button", node, part);
            b.base_style_ref(&ARROW[i]).attr("type", "button").attr("aria-label", label).attr(marker, lit("")).text(lit(glyph));
            cx.open(&b, node)?;
            cx.close("button");
        }
        cx.close("section");
        Ok(())
    }
}
