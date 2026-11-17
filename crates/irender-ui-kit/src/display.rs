//! Components that show content: badge, card, alert, avatar, separator, skeleton,
//! kbd, progress, aspect ratio, table, typography, spinner, empty state and scroll
//! area.

use crate::{
    BORDER, BRAND_2, DESTRUCTIVE, MONO, MUTED, ON_BRAND_2, ON_DESTRUCTIVE, ON_PRIMARY, ON_SECONDARY, PRIMARY, SECONDARY, SPIN, SUBTLE,
    SURFACE, TEXT, choice, lit,
};
use irender_core::{ComponentInfo, Node, Part, PropDef, PropKind, StyleBlock, StyleValue, Val, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use std::sync::LazyLock;

/// Writes `<tag>` with text and no children.
fn leaf(cx: &mut RenderCx<'_>, node: &Node, tag: &str, base: &'static StyleBlock, text: Option<&Val>) -> Result<(), RenderError> {
    let mut el = cx.element(tag, node);
    el.base_style_ref(base);
    if let Some(t) = text {
        el.text(t);
    }
    cx.open(&el, node)?;
    cx.close(tag);
    Ok(())
}

/// Writes `<tag>` with optional text, then the node's children.
fn container(
    cx: &mut RenderCx<'_>,
    node: &Node,
    tag: &str,
    base: &'static StyleBlock,
    text: Option<&Val>,
) -> Result<(), RenderError> {
    let mut el = cx.element(tag, node);
    el.base_style_ref(base);
    if let Some(t) = text {
        el.text(t);
    }
    cx.open(&el, node)?;
    cx.children(node)?;
    cx.close(tag);
    Ok(())
}

/// A small text element inside a component, styled through `parts.<name>`.
fn part_text(cx: &mut RenderCx<'_>, node: &Node, tag: &str, part: &str, base: &'static StyleBlock, text: &Val) -> Result<(), RenderError> {
    let mut el = cx.part(tag, node, part);
    el.base_style_ref(base).text(text);
    cx.open(&el, node)?;
    cx.close(tag);
    Ok(())
}

const fn info(doc: &'static str, props: &'static [PropDef], parts: &'static [&'static str], children: bool) -> ComponentInfo {
    ComponentInfo { doc, props, parts, children, open_props: false }
}

// ---------------------------------------------------------------- badge

pub struct Badge;

const BADGE_VARIANTS: &[&str] = &["default", "secondary", "destructive", "outline", "brand"];
const BADGE_PROPS: &[PropDef] = &[
    PropDef::new("text", PropKind::Any, "Badge text").required(),
    PropDef::new("icon", PropKind::Any, "An icon name (with the icon kit) or a character, before the text"),
    PropDef::new("variant", PropKind::Enum(BADGE_VARIANTS), "Default `default`"),
];

static BADGE: LazyLock<[StyleBlock; 5]> = LazyLock::new(|| {
    let base = |bg: &str, fg: &str, border: &str| {
        StyleBlock::from_pairs(&[
            ("display", "inline-flex"),
            ("align-items", "center"),
            ("gap", sp!(4)),
            ("width", "fit-content"),
            ("padding", sp!(2, 8)),
            ("border", border),
            ("border-radius", "var(--radius-sm)"),
            ("font-size", fs!(12)),
            ("font-weight", "500"),
            ("line-height", "1.4"),
            ("white-space", "nowrap"),
            ("background", bg),
            ("color", fg),
        ])
    };
    [
        base(PRIMARY, ON_PRIMARY, "1px solid transparent"),
        base(SECONDARY, ON_SECONDARY, "1px solid transparent"),
        base(DESTRUCTIVE, ON_DESTRUCTIVE, "1px solid transparent"),
        base("transparent", TEXT, "1px solid var(--color-border)"),
        base(BRAND_2, ON_BRAND_2, "1px solid transparent"),
    ]
});

impl Component for Badge {
    fn name(&self) -> &'static str {
        "ui-badge"
    }
    fn info(&self) -> ComponentInfo {
        info("A small status label.", BADGE_PROPS, &[], false)
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let style = &BADGE[choice(cx, node, "variant", BADGE_VARIANTS)];
        let Some(i) = node.prop("icon") else { return leaf(cx, node, "span", style, node.prop("text")) };
        let mut el = cx.element("span", node);
        el.base_style_ref(style);
        cx.open(&el, node)?;
        crate::icon(cx, node, i, 12)?;
        if let Some(t) = node.prop("text") {
            cx.open(Element::new("span").text(t), node)?;
            cx.close("span");
        }
        cx.close("span");
        Ok(())
    }
}

// ---------------------------------------------------------------- card

pub struct Card;

const CARD_PROPS: &[PropDef] = &[
    PropDef::new("title", PropKind::Any, "Heading in the card's header"),
    PropDef::new("description", PropKind::Any, "Muted text under the title"),
    PropDef::new("tag", PropKind::Tag, "HTML tag (default `div`), e.g. `article`, `section`"),
];

static CARD: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("display", "flex"),
        ("flex-direction", "column"),
        ("gap", sp!(16)),
        ("padding", sp!(24)),
        ("border", "1px solid var(--color-border)"),
        ("border-radius", "var(--radius-lg)"),
        ("background", SURFACE),
        ("color", TEXT),
        ("box-shadow", "0 1px 2px rgb(0 0 0 / 0.05)"),
    ])
});
static CARD_HEADER: LazyLock<StyleBlock> =
    LazyLock::new(|| StyleBlock::from_pairs(&[("display", "grid"), ("gap", sp!(6))]));
static CARD_TITLE: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[("margin", "0"), ("font-size", fs!(16)), ("font-weight", "600"), ("line-height", "1.2")])
});
static CARD_DESCRIPTION: LazyLock<StyleBlock> =
    LazyLock::new(|| StyleBlock::from_pairs(&[("margin", "0"), ("font-size", fs!(14)), ("color", MUTED)]));

impl Component for Card {
    fn name(&self) -> &'static str {
        "ui-card"
    }
    fn info(&self) -> ComponentInfo {
        info(
            "A bordered panel with an optional title and description. Put a `ui-card-footer` last for actions.",
            CARD_PROPS,
            &["header", "title", "description"],
            true,
        )
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let tag = cx.prop_string(node, "tag").unwrap_or_else(|| "div".into());
        let mut el = cx.element(&tag, node);
        el.base_style_ref(&CARD);
        cx.open(&el, node)?;
        let (title, description) = (node.prop("title"), node.prop("description"));
        if title.is_some() || description.is_some() {
            let mut header = cx.part("div", node, "header");
            header.base_style_ref(&CARD_HEADER);
            cx.open(&header, node)?;
            if let Some(t) = title {
                part_text(cx, node, "h3", "title", &CARD_TITLE, t)?;
            }
            if let Some(d) = description {
                part_text(cx, node, "p", "description", &CARD_DESCRIPTION, d)?;
            }
            cx.close("div");
        }
        cx.children(node)?;
        cx.close(&tag);
        Ok(())
    }
}

pub struct CardFooter;

static CARD_FOOTER: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("display", "flex"),
        ("align-items", "center"),
        ("justify-content", "flex-end"),
        ("gap", sp!(8)),
        ("margin-top", "auto"),
        ("padding-top", sp!(8)),
    ])
});

impl Component for CardFooter {
    fn name(&self) -> &'static str {
        "ui-card-footer"
    }
    fn info(&self) -> ComponentInfo {
        info("A row of actions at the bottom of a `ui-card`.", &[], &[], true)
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        container(cx, node, "div", &CARD_FOOTER, None)
    }
}

// ---------------------------------------------------------------- alert

pub struct Alert;

const ALERT_VARIANTS: &[&str] = &["default", "destructive"];
const ALERT_PROPS: &[PropDef] = &[
    PropDef::new("title", PropKind::Any, "Bold first line"),
    PropDef::new("description", PropKind::Any, "Body text"),
    PropDef::new("icon", PropKind::Any, "An icon name (with the icon kit) or a character, shown on the left, e.g. `warning` or `⚠`"),
    PropDef::new("variant", PropKind::Enum(ALERT_VARIANTS), "Default `default`"),
];

static ALERT: LazyLock<[StyleBlock; 2]> = LazyLock::new(|| {
    let base = |color: &str, border: &str| {
        StyleBlock::from_pairs(&[
            ("display", "grid"),
            ("grid-template-columns", "auto 1fr"),
            ("column-gap", sp!(12)),
            ("row-gap", sp!(2)),
            ("align-items", "start"),
            ("padding", sp!(12, 16)),
            ("border", border),
            ("border-radius", "var(--radius-lg)"),
            ("background", SURFACE),
            ("color", color),
            ("font-size", fs!(14)),
        ])
    };
    [
        base(TEXT, "1px solid var(--color-border)"),
        base(DESTRUCTIVE, "1px solid color-mix(in srgb, var(--color-destructive, #dc2626) 40%, transparent)"),
    ]
});
static ALERT_ICON: LazyLock<StyleBlock> =
    LazyLock::new(|| StyleBlock::from_pairs(&[("grid-row", "span 2"), ("line-height", "1.4")]));
static ALERT_TITLE: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[("grid-column", "2"), ("font-weight", "500"), ("line-height", "1.4")])
});
static ALERT_DESCRIPTION: LazyLock<StyleBlock> =
    LazyLock::new(|| StyleBlock::from_pairs(&[("grid-column", "2"), ("margin", "0"), ("opacity", "0.85")]));

impl Component for Alert {
    fn name(&self) -> &'static str {
        "ui-alert"
    }
    fn info(&self) -> ComponentInfo {
        info("A callout for important messages.", ALERT_PROPS, &["icon", "title", "description"], true)
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut el = cx.element("div", node);
        el.base_style_ref(&ALERT[choice(cx, node, "variant", ALERT_VARIANTS)]);
        el.attr("role", "alert");
        cx.open(&el, node)?;
        if let Some(icon) = node.prop("icon") {
            let mut part = cx.part("span", node, "icon");
            part.base_style_ref(&ALERT_ICON).attr("aria-hidden", "true");
            let name = crate::icon_name(cx, icon);
            if name.is_none() {
                part.text(icon);
            }
            cx.open(&part, node)?;
            if let Some(n) = name {
                crate::icon_kit(cx, node, &n, 16)?;
            }
            cx.close("span");
        }
        if let Some(t) = node.prop("title") {
            part_text(cx, node, "div", "title", &ALERT_TITLE, t)?;
        }
        if let Some(d) = node.prop("description") {
            part_text(cx, node, "p", "description", &ALERT_DESCRIPTION, d)?;
        }
        cx.children(node)?;
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- avatar

pub struct Avatar;

const AVATAR_PROPS: &[PropDef] = &[
    PropDef::new("src", PropKind::Any, "Image URL"),
    PropDef::new("alt", PropKind::Any, "Image description"),
    PropDef::new("fallback", PropKind::Any, "Text shown without an image, usually initials"),
    PropDef::new("size", PropKind::Any, "Number (px) or CSS length; default 40"),
];

static AVATAR: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("position", "relative"),
        ("display", "inline-flex"),
        ("align-items", "center"),
        ("justify-content", "center"),
        ("flex-shrink", "0"),
        ("overflow", "hidden"),
        ("border-radius", "999px"),
        ("background", SUBTLE),
        ("color", MUTED),
        ("font-size", fs!(14)),
        ("font-weight", "500"),
    ])
});
static AVATAR_IMAGE: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[("width", "100%"), ("height", "100%"), ("object-fit", "cover")])
});

impl Component for Avatar {
    fn name(&self) -> &'static str {
        "ui-avatar"
    }
    fn info(&self) -> ComponentInfo {
        info("A round image, or initials when there is none.", AVATAR_PROPS, &["image"], false)
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let size = node.prop("size").cloned().unwrap_or_else(|| lit(40));
        let mut sizing = StyleBlock::new();
        for property in ["width", "height"] {
            if let Some(v) = cx.style_value(node, property, &size)? {
                sizing.push(property, v);
            }
        }
        let has_image = cx.prop(node, "src").is_some_and(|s| !s.is_null() && s != "");
        let mut el = cx.element("span", node);
        el.base_style_ref(&AVATAR).base_style(sizing);
        if !has_image && let Some(f) = node.prop("fallback") {
            el.text(f);
        }
        cx.open(&el, node)?;
        if has_image {
            let mut img = cx.part("img", node, "image");
            img.base_style_ref(&AVATAR_IMAGE);
            img.attr("src", node.prop("src").cloned().unwrap_or_else(|| lit("")));
            let alt = node.prop("alt").or(node.prop("fallback")).cloned().unwrap_or_else(|| lit(""));
            img.attr("alt", alt);
            cx.open(&img, node)?;
        }
        cx.close("span");
        Ok(())
    }
}

// ---------------------------------------------------------------- separator

pub struct Separator;

const ORIENTATIONS: &[&str] = &["horizontal", "vertical"];
const SEPARATOR_PROPS: &[PropDef] = &[
    PropDef::new("orientation", PropKind::Enum(ORIENTATIONS), "Default `horizontal`"),
    PropDef::new("label", PropKind::Any, "Text in the middle of a horizontal line, e.g. `Or continue with`"),
    PropDef::new("decorative", PropKind::Bool, "Hide from assistive technology (default true)"),
];

static SEPARATOR: LazyLock<[StyleBlock; 2]> = LazyLock::new(|| {
    [
        StyleBlock::from_pairs(&[("flex-shrink", "0"), ("height", "1px"), ("width", "100%"), ("background", BORDER)]),
        StyleBlock::from_pairs(&[("flex-shrink", "0"), ("width", "1px"), ("align-self", "stretch"), ("background", BORDER)]),
    ]
});

static SEPARATOR_LABELLED: LazyLock<StyleBlock> = LazyLock::new(|| {
    let line = StyleBlock::from_pairs(&[("content", "\"\""), ("flex", "1"), ("height", "1px"), ("background", BORDER)]);
    StyleBlock::from_pairs(&[
        ("display", "flex"),
        ("align-items", "center"),
        ("gap", sp!(12)),
        ("width", "100%"),
        ("font-size", fs!(14)),
        ("color", MUTED),
        ("white-space", "nowrap"),
    ])
    .nest("::before", line.clone())
    .nest("::after", line)
});

impl Component for Separator {
    fn name(&self) -> &'static str {
        "ui-separator"
    }
    fn info(&self) -> ComponentInfo {
        info("A thin dividing line.", SEPARATOR_PROPS, &[], false)
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let o = choice(cx, node, "orientation", ORIENTATIONS);
        if o == 0
            && let Some(label) = node.prop("label")
        {
            // Lines either side of the text, which reads as part of the content.
            let mut el = cx.element("div", node);
            el.base_style_ref(&SEPARATOR_LABELLED).text(label);
            cx.open(&el, node)?;
            cx.close("div");
            return Ok(());
        }
        let decorative = cx.prop(node, "decorative").is_none_or(|d| expr::truthy(&d));
        let mut el = cx.element("div", node);
        el.base_style_ref(&SEPARATOR[o]);
        if decorative {
            el.attr("role", "none");
        } else {
            el.attr("role", "separator").attr("aria-orientation", ORIENTATIONS[o]);
        }
        cx.open(&el, node)?;
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- skeleton

pub struct Skeleton;

const SKELETON_PROPS: &[PropDef] = &[
    PropDef::new("width", PropKind::Any, "Number (px) or CSS length; default 100%"),
    PropDef::new("height", PropKind::Any, "Number (px) or CSS length; default 16"),
    PropDef::new("round", PropKind::Bool, "A circle instead of a rounded rectangle"),
];

/// The skeleton's pulse, added to the page once by the first skeleton.
const PULSE: &str = "@keyframes ui-pulse {\n  50% { opacity: 0.5; }\n}";

static SKELETON: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("background", SUBTLE),
        ("border-radius", "var(--radius-md)"),
        ("animation", "ui-pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite"),
    ])
    // People who ask their system for less motion get a still placeholder.
    .nest("@media (prefers-reduced-motion: reduce)", StyleBlock::from_pairs(&[("animation", "none")]))
});

impl Component for Skeleton {
    fn name(&self) -> &'static str {
        "ui-skeleton"
    }
    fn info(&self) -> ComponentInfo {
        info("A pulsing placeholder block shown while content loads.", SKELETON_PROPS, &[], false)
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut sizing = StyleBlock::new();
        for (property, default) in [("width", lit("100%")), ("height", lit(16))] {
            let v = node.prop(property).cloned().unwrap_or(default);
            if let Some(v) = cx.style_value(node, property, &v)? {
                sizing.push(property, v);
            }
        }
        if cx.prop(node, "round").is_some_and(|r| expr::truthy(&r)) {
            sizing = sizing.set("border-radius", "999px");
        }
        cx.global_css("ui-pulse", PULSE);
        let mut el = cx.element("div", node);
        el.base_style_ref(&SKELETON).base_style(sizing).attr("aria-hidden", "true");
        cx.open(&el, node)?;
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- kbd

pub struct Kbd;

const KBD_PROPS: &[PropDef] = &[PropDef::new("text", PropKind::Any, "The key, e.g. `⌘K`").required()];

static KBD: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("display", "inline-flex"),
        ("align-items", "center"),
        ("padding", "1px 6px"),
        ("border", "1px solid var(--color-border)"),
        ("border-bottom-width", "2px"),
        ("border-radius", "var(--radius-sm)"),
        ("background", SUBTLE),
        ("color", TEXT),
        ("font-family", MONO),
        ("font-size", fs!(12)),
        ("font-weight", "500"),
    ])
});

impl Component for Kbd {
    fn name(&self) -> &'static str {
        "ui-kbd"
    }
    fn info(&self) -> ComponentInfo {
        info("A keyboard key.", KBD_PROPS, &[], false)
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        leaf(cx, node, "kbd", &KBD, node.prop("text"))
    }
}

// ---------------------------------------------------------------- progress

pub struct Progress;

const PROGRESS_PROPS: &[PropDef] =
    &[PropDef::new("value", PropKind::Any, "0–100. Updates live when it uses state.").required()];

static PROGRESS: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("position", "relative"),
        ("height", "8px"),
        ("width", "100%"),
        ("overflow", "hidden"),
        ("border-radius", "999px"),
        ("background", "color-mix(in srgb, var(--color-primary) 20%, transparent)"),
    ])
});
static PROGRESS_INDICATOR: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("height", "100%"),
        // `--ui-value` is set per element; clamping keeps bad values in range.
        ("width", "min(100%, max(0%, var(--ui-value)))"),
        ("background", PRIMARY),
        ("transition", "width .2s"),
    ])
});

impl Component for Progress {
    fn name(&self) -> &'static str {
        "ui-progress"
    }
    fn info(&self) -> ComponentInfo {
        info("A progress bar.", PROGRESS_PROPS, &["indicator"], false)
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let value = node.prop("value").cloned().unwrap_or_else(|| lit(0));
        // The value as a percentage: a template gains a `%`, a literal becomes one.
        let percent = match &value {
            Val::Tpl(t) => {
                let mut tpl = t.clone();
                tpl.parts.push(Part::Lit("%".into()));
                StyleValue::Dynamic { tpl, px: false }
            }
            Val::Lit(v) => StyleValue::Static(format!("{}%", expr::to_num(v))),
        };
        let mut el = cx.element("div", node);
        el.base_style_ref(&PROGRESS);
        el.attr("role", "progressbar").attr("aria-valuemin", lit(0)).attr("aria-valuemax", lit(100));
        el.attr("aria-valuenow", value.clone());
        cx.open(&el, node)?;
        let mut bar = cx.part("div", node, "indicator");
        let mut v = StyleBlock::new();
        v.push("--ui-value", percent);
        bar.base_style_ref(&PROGRESS_INDICATOR).base_style(v);
        cx.open(&bar, node)?;
        cx.close("div");
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- aspect ratio

pub struct AspectRatio;

const ASPECT_PROPS: &[PropDef] =
    &[PropDef::new("ratio", PropKind::Any, "Width / height: a number (`1.5`) or `\"16 / 9\"`").required()];

static ASPECT: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[("position", "relative"), ("width", "100%"), ("overflow", "hidden")]).nest(
        "& > *",
        StyleBlock::from_pairs(&[("width", "100%"), ("height", "100%"), ("object-fit", "cover")]),
    )
});

impl Component for AspectRatio {
    fn name(&self) -> &'static str {
        "ui-aspect-ratio"
    }
    fn info(&self) -> ComponentInfo {
        info("Keeps its content at a fixed width-to-height ratio.", ASPECT_PROPS, &[], true)
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let ratio = match node.prop("ratio") {
            Some(Val::Tpl(t)) => StyleValue::Dynamic { tpl: t.clone(), px: false },
            Some(Val::Lit(v)) => StyleValue::Static(expr::stringify(v)),
            None => StyleValue::Static("1".into()),
        };
        if let StyleValue::Static(s) = &ratio {
            irender_core::policy::css_value_is_contained(s).map_err(|e| RenderError::new(node, e))?;
        }
        let mut r = StyleBlock::new();
        r.push("aspect-ratio", ratio);
        let mut el = cx.element("div", node);
        el.base_style_ref(&ASPECT).base_style(r);
        cx.open(&el, node)?;
        cx.children(node)?;
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- table

pub struct Table;

const TABLE_PROPS: &[PropDef] = &[PropDef::new("caption", PropKind::Any, "Caption shown under the table")];

static TABLE_CONTAINER: LazyLock<StyleBlock> =
    LazyLock::new(|| StyleBlock::from_pairs(&[("position", "relative"), ("width", "100%"), ("overflow-x", "auto")]));
static TABLE: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("width", "100%"),
        ("border-collapse", "collapse"),
        ("caption-side", "bottom"),
        ("font-size", fs!(14)),
        ("color", TEXT),
    ])
    .nest("& caption", StyleBlock::from_pairs(&[("margin-top", sp!(16)), ("color", MUTED)]))
    .nest(
        "& th",
        StyleBlock::from_pairs(&[
            ("height", sp!(40)),
            ("padding", sp!(0, 8)),
            ("text-align", "left"),
            ("vertical-align", "middle"),
            ("font-weight", "500"),
            ("white-space", "nowrap"),
            ("border-bottom", "1px solid var(--color-border)"),
        ]),
    )
    .nest(
        "& td",
        StyleBlock::from_pairs(&[("padding", sp!(8)), ("vertical-align", "middle"), ("border-bottom", "1px solid var(--color-border)")]),
    )
    .nest("& tbody tr:last-child td", StyleBlock::from_pairs(&[("border-bottom", "0")]))
    .nest("& tbody tr:hover", StyleBlock::from_pairs(&[("background", SUBTLE)]))
    .nest(
        "& tfoot td",
        StyleBlock::from_pairs(&[("background", SUBTLE), ("font-weight", "500"), ("border-top", "1px solid var(--color-border)")]),
    )
});

impl Component for Table {
    fn name(&self) -> &'static str {
        "ui-table"
    }
    fn info(&self) -> ComponentInfo {
        info(
            "A styled table in a scrolling container. Children are the table's rows and sections \
             (`thead`, `tbody`, a `list` with `tag: tbody`, …).",
            TABLE_PROPS,
            &["table"],
            true,
        )
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut el = cx.element("div", node);
        el.base_style_ref(&TABLE_CONTAINER);
        cx.open(&el, node)?;
        let mut table = cx.part("table", node, "table");
        table.base_style_ref(&TABLE);
        cx.open(&table, node)?;
        if let Some(c) = node.prop("caption") {
            let mut caption = irender_html::Element::new("caption");
            caption.text(c);
            cx.open(&caption, node)?;
            cx.close("caption");
        }
        cx.children(node)?;
        cx.close("table");
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- typography

pub struct Heading;

const HEADING_PROPS: &[PropDef] = &[
    PropDef::new("text", PropKind::Any, "The heading").required(),
    PropDef::new("level", PropKind::Any, "1–4 (default 2)"),
];

static HEADINGS: LazyLock<[StyleBlock; 4]> = LazyLock::new(|| {
    let base = |size: &str, weight: &str| {
        StyleBlock::from_pairs(&[
            ("margin", "0"),
            ("color", TEXT),
            ("font-size", size),
            ("font-weight", weight),
            ("line-height", "1.2"),
            ("letter-spacing", "-0.02em"),
            ("scroll-margin-top", "80px"),
        ])
    };
    [
        base("36px", "800"),
        base("30px", "600").set("padding-bottom", sp!(8)).set("border-bottom", "1px solid var(--color-border)"),
        base("24px", "600"),
        base("20px", "600"),
    ]
});

impl Component for Heading {
    fn name(&self) -> &'static str {
        "ui-heading"
    }
    fn info(&self) -> ComponentInfo {
        info("A heading, `h1`–`h4`.", HEADING_PROPS, &[], false)
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let level = cx.prop(node, "level").map(|l| expr::to_num(&l)).filter(|l| (1.0..=4.0).contains(l));
        let level = level.map_or(2, |l| l as usize);
        leaf(cx, node, &format!("h{level}"), &HEADINGS[level - 1], node.prop("text"))
    }
}

pub struct Text;

const TEXT_VARIANTS: &[&str] = &["p", "lead", "large", "small", "muted", "code", "blockquote"];
const TEXT_PROPS: &[PropDef] = &[
    PropDef::new("text", PropKind::Any, "The text"),
    PropDef::new("variant", PropKind::Enum(TEXT_VARIANTS), "Default `p`"),
    PropDef::new("tag", PropKind::Tag, "Override the variant's tag"),
];

static TEXTS: LazyLock<[StyleBlock; 7]> = LazyLock::new(|| {
    let base = |pairs: &[(&str, &str)]| {
        let mut all = vec![("margin", "0")];
        all.extend_from_slice(pairs);
        StyleBlock::from_pairs(&all)
    };
    [
        base(&[("line-height", "1.75")]),
        base(&[("font-size", fs!(20)), ("color", MUTED)]),
        base(&[("font-size", fs!(18)), ("font-weight", "600")]),
        base(&[("font-size", fs!(14)), ("font-weight", "500"), ("line-height", "1")]),
        base(&[("font-size", fs!(14)), ("color", MUTED)]),
        base(&[
            ("padding", sp!(2, 5)),
            ("border-radius", "var(--radius-sm)"),
            ("background", SUBTLE),
            ("font-family", MONO),
            ("font-size", "0.875em"),
            ("font-weight", "600"),
        ]),
        base(&[("padding-left", sp!(24)), ("border-left", "2px solid var(--color-border)"), ("font-style", "italic")]),
    ]
});
const TEXT_TAGS: [&str; 7] = ["p", "p", "div", "small", "p", "code", "blockquote"];

impl Component for Text {
    fn name(&self) -> &'static str {
        "ui-text"
    }
    fn info(&self) -> ComponentInfo {
        info("Body text in one of several typographic styles.", TEXT_PROPS, &[], true)
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let v = choice(cx, node, "variant", TEXT_VARIANTS);
        let tag = cx.prop_string(node, "tag").unwrap_or_else(|| TEXT_TAGS[v].into());
        container(cx, node, &tag, &TEXTS[v], node.prop("text"))
    }
}

// ---------------------------------------------------------------- spinner

pub struct Spinner;

const SPINNER_PROPS: &[PropDef] = &[
    PropDef::new("size", PropKind::Any, "Number (px) or CSS length; default 16"),
    PropDef::new("label", PropKind::Any, "What is loading, for screen readers; default `Loading`"),
];

static SPINNER: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("display", "inline-block"),
        ("flex-shrink", "0"),
        ("box-sizing", "border-box"),
        ("border", "2px solid currentColor"),
        ("border-right-color", "transparent"),
        ("border-radius", "999px"),
        ("animation", "ui-spin .7s linear infinite"),
    ])
    // Slower rather than still: a still ring wouldn't say that something is happening.
    .nest("@media (prefers-reduced-motion: reduce)", StyleBlock::from_pairs(&[("animation-duration", "2s")]))
});

impl Component for Spinner {
    fn name(&self) -> &'static str {
        "ui-spinner"
    }
    fn info(&self) -> ComponentInfo {
        info("A spinning ring that says something is loading. Takes the text colour.", SPINNER_PROPS, &[], false)
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let size = node.prop("size").cloned().unwrap_or_else(|| lit(16));
        let mut sizing = StyleBlock::new();
        for property in ["width", "height"] {
            if let Some(v) = cx.style_value(node, property, &size)? {
                sizing.push(property, v);
            }
        }
        cx.global_css("ui-spin", SPIN);
        let mut el = cx.element("span", node);
        el.base_style_ref(&SPINNER).base_style(sizing).attr("role", "status");
        el.attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit("Loading")));
        cx.open(&el, node)?;
        cx.close("span");
        Ok(())
    }
}

// ---------------------------------------------------------------- empty

pub struct Empty;

const EMPTY_PROPS: &[PropDef] = &[
    PropDef::new("title", PropKind::Any, "What is empty, e.g. `No projects yet`"),
    PropDef::new("description", PropKind::Any, "What to do about it"),
    PropDef::new("icon", PropKind::Any, "A character or emoji shown above the title"),
];

static EMPTY: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("display", "flex"),
        ("flex-direction", "column"),
        ("align-items", "center"),
        ("justify-content", "center"),
        ("gap", sp!(12)),
        ("padding", sp!(48, 24)),
        ("border", "1px dashed var(--color-border)"),
        ("border-radius", "var(--radius-lg)"),
        ("color", TEXT),
        ("text-align", "center"),
    ])
});
static EMPTY_ICON: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("display", "flex"),
        ("align-items", "center"),
        ("justify-content", "center"),
        ("width", "40px"),
        ("height", sp!(40)),
        ("border-radius", "var(--radius-md)"),
        ("background", SUBTLE),
        ("font-size", fs!(20)),
    ])
});
static EMPTY_TITLE: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[("margin", "0"), ("font-size", fs!(18)), ("font-weight", "500"), ("line-height", "1.2")])
});
static EMPTY_DESCRIPTION: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[("margin", "0"), ("max-width", "40ch"), ("font-size", fs!(14)), ("color", MUTED)])
});

impl Component for Empty {
    fn name(&self) -> &'static str {
        "ui-empty"
    }
    fn info(&self) -> ComponentInfo {
        info(
            "A placeholder for a page or list with nothing in it yet. Children follow the text, usually a button.",
            EMPTY_PROPS,
            &["icon", "title", "description"],
            true,
        )
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut el = cx.element("div", node);
        el.base_style_ref(&EMPTY);
        cx.open(&el, node)?;
        if let Some(icon) = node.prop("icon") {
            let mut part = cx.part("div", node, "icon");
            part.base_style_ref(&EMPTY_ICON).attr("aria-hidden", "true").text(icon);
            cx.open(&part, node)?;
            cx.close("div");
        }
        if let Some(t) = node.prop("title") {
            part_text(cx, node, "h3", "title", &EMPTY_TITLE, t)?;
        }
        if let Some(d) = node.prop("description") {
            part_text(cx, node, "p", "description", &EMPTY_DESCRIPTION, d)?;
        }
        cx.children(node)?;
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- scroll area

pub struct ScrollArea;

const SCROLL_AREA_PROPS: &[PropDef] = &[
    PropDef::new("height", PropKind::Any, "Number (px) or CSS length"),
    PropDef::new("maxHeight", PropKind::Any, "Number (px) or CSS length: grows with its content up to this"),
    PropDef::new("label", PropKind::Any, "Names the region for screen readers"),
];

static SCROLL_AREA: LazyLock<StyleBlock> = LazyLock::new(|| {
    StyleBlock::from_pairs(&[
        ("overflow", "auto"),
        ("overscroll-behavior", "contain"),
        ("scrollbar-width", "thin"),
        ("scrollbar-color", "color-mix(in srgb, var(--color-text) 25%, transparent) transparent"),
        ("border-radius", "var(--radius-md)"),
    ])
    .nest(
        ":focus-visible",
        StyleBlock::from_pairs(&[
            ("outline", "none"),
            ("box-shadow", "0 0 0 3px color-mix(in srgb, var(--color-ring, var(--color-muted)) 50%, transparent)"),
        ]),
    )
});

impl Component for ScrollArea {
    fn name(&self) -> &'static str {
        "ui-scroll-area"
    }
    fn info(&self) -> ComponentInfo {
        info(
            "A box that scrolls its children, with a thin scrollbar in the theme's colours. \
             It can be focused, so the keyboard scrolls it too.",
            SCROLL_AREA_PROPS,
            &[],
            true,
        )
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut sizing = StyleBlock::new();
        for (prop, property) in [("height", "height"), ("maxHeight", "max-height")] {
            if let Some(v) = node.prop(prop)
                && let Some(v) = cx.style_value(node, property, v)?
            {
                sizing.push(property, v);
            }
        }
        let mut el = cx.element("div", node);
        el.base_style_ref(&SCROLL_AREA).base_style(sizing).attr("tabindex", "0");
        if let Some(label) = node.prop("label") {
            el.attr("role", "region").attr("aria-label", label.clone());
        }
        cx.open(&el, node)?;
        cx.children(node)?;
        cx.close("div");
        Ok(())
    }
}
