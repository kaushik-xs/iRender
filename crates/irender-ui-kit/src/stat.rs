//! `ui-stat` (a figure with its change, trend and target), `ui-description-list` +
//! `ui-description-item` (labels and values, copied or edited in place) and `ui-item` +
//! `ui-item-group` (a thing in a list: media, title, description, actions).

use crate::{
    BG_CARD, BORDER, MUTED, PRIMARY, PRIMARY_FILL, SUBTLE, TEXT, branch, each_item, expr_val, field_or, forms, icon, items, lit, path, status, synthetic, val_expr,
    widget::{self, flag, pairs},
};
use irender_core::{
    ComponentInfo, Node, PropDef, PropKind, StyleBlock,
    expr::{self, Expr},
};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("stat.js");

// ---------------------------------------------------------------- stat

pub struct Stat;

const FORMATS: &[&str] = &["number", "currency", "percent", "compact", "none"];
const STAT_PROPS: &[PropDef] = &[
    PropDef::new("label", PropKind::Any, "What the figure is").required(),
    PropDef::new("value", PropKind::Any, "The figure: a number, or text").required(),
    PropDef::new("format", PropKind::Enum(FORMATS), "How a number is written (default `number`; `compact`: 1.2M)"),
    PropDef::new("currency", PropKind::String, "An ISO code, e.g. `EUR`"),
    PropDef::new("decimals", PropKind::Number, "Digits after the point"),
    PropDef::new("unit", PropKind::Any, "After the figure, smaller, e.g. `kWh`"),
    PropDef::new("previous", PropKind::Any, "The figure before, to work out the change from"),
    PropDef::new("change", PropKind::Any, "The change itself, e.g. 0.12 for +12%"),
    PropDef::new("changeAs", PropKind::Enum(&["percent", "value"]), "The change as a percentage (default), or in the figure's own format"),
    PropDef::new("changeLabel", PropKind::Any, "After the change, e.g. `vs last month`"),
    PropDef::new("goodWhen", PropKind::Enum(&["up", "down", "neutral"]), "Which way is good, for the change's colour (default `up`)"),
    PropDef::new("trend", PropKind::Any, "A list of numbers drawn as a small line"),
    PropDef::new("target", PropKind::Any, "Progress to this is shown as a bar"),
    PropDef::new("targetLabel", PropKind::Any, "Default `{percent} of {target}`"),
    PropDef::new("description", PropKind::Any, "Smaller text at the foot"),
    PropDef::new("icon", PropKind::Any, "An icon name (with the icon kit) or a character"),
    PropDef::new("href", PropKind::Any, "The whole tile is a link"),
    PropDef::new("variant", PropKind::Enum(&["card", "plain"]), "Default `card`"),
    PropDef::new("loading", PropKind::Any, "Placeholders in place of the figures"),
    PropDef::new("locale", PropKind::String, "How numbers are written (default: the document's `lang`, else the browser's)"),
];

static STAT: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("position", "relative"), ("display", "flex"), ("flex-direction", "column"), ("gap", gap!(6)), ("min-width", "0"), ("color", TEXT), ("font-size", fs!(14)), ("text-decoration", "none")])
        .nest(
            "&[data-variant=card]",
            pairs(&[("padding", pad!(16, 20)), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-card, var(--radius-lg))"), ("--ui-bg", BG_CARD), ("background", BG_CARD), ("box-shadow", "var(--elevation-card, 0 1px 2px rgb(0 0 0 / 0.04))")]),
        )
        .nest("&[href]:hover", pairs(&[("border-color", "color-mix(in srgb, var(--color-text) 30%, var(--color-border))")]))
        .nest("&[href]:focus-visible", forms::focus_ring())
        .nest("& [data-stat-top]", pairs(&[("display", "flex"), ("align-items", "center"), ("justify-content", "space-between"), ("gap", gap!(8)), ("color", MUTED), ("font-weight", "var(--font-weight-medium, 500)")]))
        .nest("& [data-stat-main]", pairs(&[("display", "flex"), ("align-items", "flex-end"), ("justify-content", "space-between"), ("gap", gap!(12)), ("min-width", "0")]))
        .nest("& [data-stat-value]", pairs(&[("font-size", fs!(28)), ("font-weight", "var(--font-weight-semibold, 600)"), ("letter-spacing", "-0.02em"), ("line-height", "1.15"), ("font-variant-numeric", "tabular-nums"), ("min-width", "0"), ("overflow-wrap", "break-word")]))
        .nest("& [data-stat-unit]", pairs(&[("margin-inline-start", gap!(4)), ("font-size", fs!(14)), ("font-weight", "400"), ("letter-spacing", "0"), ("color", MUTED)]))
        .nest("& svg[data-stat-trend]", pairs(&[("flex", "1 1 32px"), ("min-width", "0"), ("max-width", "96px"), ("height", "32px"), ("overflow", "visible")]))
        .nest("& [data-stat-line]", pairs(&[("fill", "none"), ("stroke", "var(--stat, var(--color-primary))"), ("stroke-width", "2"), ("stroke-linecap", "round"), ("stroke-linejoin", "round"), ("vector-effect", "non-scaling-stroke")]))
        .nest("& [data-stat-area]", pairs(&[("fill", "var(--stat, var(--color-primary))"), ("opacity", "0.12")]))
        .nest("& [data-stat-foot]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("align-items", "center"), ("gap", gap!(2, 6)), ("font-size", fs!(13)), ("color", MUTED)]))
        .nest("& [data-stat-foot]:not(:has(> :not([hidden])))", pairs(&[("display", "none")]))
        .nest("& [data-stat-change]", pairs(&[("display", "inline-flex"), ("align-items", "center"), ("gap", gap!(2)), ("font-weight", "var(--font-weight-medium, 500)"), ("color", "var(--stat, inherit)"), ("font-variant-numeric", "tabular-nums")]))
        .nest("& [data-stat-change][hidden]", pairs(&[("display", "none")]))
        .nest("&[data-tone=good]", pairs(&[("--stat", "var(--color-success, #16a34a)")]))
        .nest("&[data-tone=bad]", pairs(&[("--stat", "var(--color-destructive, #dc2626)")]))
        .nest("& [data-stat-bar]", pairs(&[("height", "6px"), ("border-radius", "999px"), ("overflow", "hidden"), ("background", "color-mix(in srgb, var(--color-primary) 15%, transparent)")]))
        .nest("& [data-stat-bar] > span", pairs(&[("display", "block"), ("height", "100%"), ("border-radius", "999px"), ("background", PRIMARY_FILL), ("transition", "width .3s")]))
        .nest("&[aria-busy=true] :is([data-stat-value], [data-stat-change])", pairs(&[("color", "transparent"), ("border-radius", "var(--radius-sm)"), ("background", "color-mix(in srgb, var(--color-text) 10%, transparent)")]))
        .nest("&[aria-busy=true] svg[data-stat-trend]", pairs(&[("visibility", "hidden")]))
});

/// A number as the server writes it, until the browser formats it in its locale.
fn server_number(cx: &RenderCx<'_>, v: &Json) -> String {
    match v {
        Json::Number(n) => irender_core::labels::number(cx.labels().lang.as_deref(), n.as_f64().unwrap_or(0.0)),
        other => expr::stringify(other),
    }
}

impl Component for Stat {
    fn name(&self) -> &'static str {
        "ui-stat"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A figure: its label, value, change from before, a small trend line and progress to a target. Children go at the foot.",
            props: STAT_PROPS,
            parts: &["label", "value"],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut config = widget::config(cx, node, &["value", "previous", "change", "trend", "target", "targetLabel"], &["format", "currency", "decimals", "changeAs", "goodWhen"], &[])?;
        config.insert("locale".into(), json!(crate::locale(cx, node)));
        widget::kit(cx);
        cx.global_js("ui-stat", SCRIPT);
        let mut el = cx.element(if node.prop("href").is_some() { "a" } else { "div" }, node);
        let plain = cx.prop_string(node, "variant").as_deref() == Some("plain");
        el.base_style_ref(&STAT).attr("data-variant", if plain { "plain" } else { "card" }).behavior("ui-stat", Json::Object(config));
        if let Some(h) = node.prop("href") {
            el.attr("href", h.clone());
        } else {
            el.attr("role", "group").attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit("")));
        }
        if let Some(l) = node.prop("loading") {
            el.attr("aria-busy", l.clone());
        }
        let tag = el.tag().to_string();
        cx.open(&el, node)?;
        let mut top = Element::new("div");
        top.attr("data-stat-top", true);
        cx.open(&top, node)?;
        let mut l = cx.part("span", node, "label");
        l.text(node.prop("label").cloned().unwrap_or_else(|| lit("")));
        cx.open(&l, node)?;
        cx.close("span");
        if let Some(i) = node.prop("icon") {
            icon(cx, node, i, 16)?;
        }
        cx.close("div");
        let mut main = Element::new("div");
        main.attr("data-stat-main", true);
        cx.open(&main, node)?;
        let now = cx.prop(node, "value").unwrap_or(Json::Null);
        let mut v = cx.part("div", node, "value");
        v.attr("data-stat-value", true);
        cx.open(&v, node)?;
        let mut figure = Element::new("span");
        figure.attr("data-stat-figure", true).text(lit(server_number(cx, &now)));
        cx.open(&figure, node)?;
        cx.close("span");
        if let Some(u) = node.prop("unit") {
            let mut s = Element::new("span");
            s.attr("data-stat-unit", true).text(u.clone());
            cx.open(&s, node)?;
            cx.close("span");
        }
        cx.close("div");
        if node.prop("trend").is_some() {
            cx.markup(r#"<svg data-stat-trend viewBox="0 0 96 32" preserveAspectRatio="none" aria-hidden="true"><path data-stat-area></path><path data-stat-line></path></svg>"#);
        }
        cx.close("div");
        if node.prop("target").is_some() {
            let mut b = Element::new("div");
            b.attr("data-stat-bar", true).attr("role", "progressbar").attr("aria-valuemin", "0").attr("aria-valuemax", "100").attr("aria-label", cx.label("Progress to the target"));
            cx.open(&b, node)?;
            cx.open(&Element::new("span"), node)?;
            cx.close("span");
            cx.close("div");
        }
        let mut foot = Element::new("div");
        foot.attr("data-stat-foot", true);
        cx.open(&foot, node)?;
        let mut c = Element::new("span");
        c.attr("data-stat-change", true).attr("hidden", true);
        cx.open(&c, node)?;
        cx.close("span");
        if let Some(t) = node.prop("changeLabel") {
            let mut s = Element::new("span");
            s.text(t.clone());
            cx.open(&s, node)?;
            cx.close("span");
        }
        if node.prop("target").is_some() {
            let mut s = Element::new("span");
            s.attr("data-stat-target", true);
            cx.open(&s, node)?;
            cx.close("span");
        }
        if let Some(d) = node.prop("description") {
            let mut s = Element::new("span");
            s.text(d.clone());
            cx.open(&s, node)?;
            cx.close("span");
        }
        cx.close("div");
        cx.children(node)?;
        cx.close(&tag);
        Ok(())
    }
}

// ---------------------------------------------------------------- description list

pub struct DescriptionList;
pub struct DescriptionItem;

const LIST_PROPS: &[PropDef] = &[
    PropDef::new("items", PropKind::Any, "Entries with the `ui-description-item` props, after any children; may use state"),
    PropDef::new("columns", PropKind::Number, "Across, on a wide screen (default 1)"),
    PropDef::new("layout", PropKind::Enum(&["stacked", "inline"]), "The value under its label (default) or beside it"),
    PropDef::new("divided", PropKind::Bool, "A line between entries"),
    PropDef::new("labelWidth", PropKind::Any, "With `inline`, the labels' width (default 160)"),
];
const EDITS: &[&str] = &["text", "multiline", "number", "date", "select"];
const ENTRY_PROPS: &[PropDef] = &[
    PropDef::new("label", PropKind::Any, "What it is").required(),
    PropDef::new("value", PropKind::Any, "The value; an empty one shows `—`. Children, if any, are shown instead"),
    PropDef::new("format", PropKind::Enum(&["number", "currency", "percent", "date", "datetime"]), "How the browser writes it"),
    PropDef::new("currency", PropKind::String, "An ISO code"),
    PropDef::new("href", PropKind::Any, "The value is a link"),
    PropDef::new("copy", PropKind::Bool, "A button copies the value"),
    PropDef::new("span", PropKind::Number, "Columns it takes"),
    PropDef::new("bind", PropKind::StateKey, "State key with the value, to edit it in place (then no `value`)"),
    PropDef::new("edit", PropKind::Enum(EDITS), "What it's edited with (default `text`)"),
    PropDef::new("options", PropKind::Any, "With `edit: select`: values or `{ value, label }`"),
    PropDef::new("placeholder", PropKind::Any, "Shown when a value that can be edited is empty (default `Add`)"),
    PropDef::new("locale", PropKind::String, "Default: the document's `lang`, else the browser's"),
];
const ENTRY: &str = "ui_entry";
const ENTRY_INDEX: &str = "ui_entry_index";

static LIST: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("display", "grid"), ("grid-template-columns", "repeat(var(--dl-cols, 1), minmax(0, 1fr))"), ("gap", gap!(16, 24)), ("margin", "0"), ("color", TEXT), ("font-size", fs!(14))])
        .nest("& > [data-entries]", pairs(&[("display", "contents")]))
        .nest("&[data-divided]", pairs(&[("gap", gap!(0, 24))]))
        .nest("&[data-divided] [data-dl-item]", pairs(&[("padding-block", pad!(10)), ("border-bottom", "var(--border-width, 1px) solid var(--color-border)")]))
        .nest("&[data-layout=inline] [data-dl-item]", pairs(&[("display", "grid"), ("grid-template-columns", "var(--dl-label, 160px) minmax(0, 1fr)"), ("gap", gap!(16)), ("align-items", "baseline")]))
        .nest(
            "@media (max-width: 640px)",
            pairs(&[("grid-template-columns", "minmax(0, 1fr)")])
                .nest("& [data-dl-item]", pairs(&[("grid-column", "auto !important")]))
                .nest("&[data-layout=inline] [data-dl-item]", pairs(&[("grid-template-columns", "minmax(0, 1fr)"), ("gap", gap!(2))])),
        )
});
static ENTRY_STYLE: LazyLock<StyleBlock> = LazyLock::new(|| {
    forms::native_scheme(pairs(&[("min-width", "0")]))
        .nest("& > dt", pairs(&[("color", MUTED), ("font-size", fs!(13))]))
        .nest("& > dd", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", gap!(4)), ("min-height", ctl!(28)), ("margin", "0"), ("overflow-wrap", "anywhere")]))
        .nest("& [data-dl-value]", pairs(&[("min-width", "0"), ("white-space", "pre-line")]))
        .nest("& [data-dl-value]:is(a)", pairs(&[("color", PRIMARY), ("text-underline-offset", "3px")]))
        .nest("& [data-dl-value][data-empty]", pairs(&[("color", MUTED)]))
        .nest("& [data-dl-tool]", widget::tool().set("opacity", "0").set("transition", "opacity .12s"))
        .nest("&:hover [data-dl-tool], & [data-dl-tool]:focus-visible", pairs(&[("opacity", "1")]))
        .nest("& [data-dl-tool]:hover", pairs(&[("background", SUBTLE), ("color", TEXT)]))
        .nest("& [data-dl-tool]:focus-visible", forms::focus_ring())
        .nest("@media (hover: none)", pairs(&[]).nest("& [data-dl-tool]", pairs(&[("opacity", "1")])))
        .nest("& [data-dl-edit]", pairs(&[("padding", pad!(2, 6)), ("margin-inline-start", "-7px"), ("border", "1px solid transparent"), ("border-radius", "var(--radius-sm)"), ("background", "none"), ("color", "inherit"), ("font", "inherit"), ("text-align", "start"), ("cursor", "text")]))
        .nest("& [data-dl-edit]:hover", pairs(&[("border-color", BORDER), ("background", SUBTLE)]))
        .nest("& [data-dl-edit]:focus-visible", forms::focus_ring())
        .nest("& [data-dl-input]", pairs(&[("width", "100%"), ("min-width", "0"), ("margin-inline-start", "-7px"), ("padding", pad!(2, 6)), ("border", "1px solid var(--color-ring, var(--color-muted))"), ("border-radius", "var(--radius-sm)"), ("background", crate::SURFACE), ("color", TEXT), ("font", "inherit"), ("outline", "none")]))
});

impl Component for DescriptionList {
    fn name(&self) -> &'static str {
        "ui-description-list"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Labels and their values, as at the top of a record's page. Children are `ui-description-item`s.", props: LIST_PROPS, parts: &[], children: true, open_props: false }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let cols = widget::number(cx, node, "columns").unwrap_or(1.0).clamp(1.0, 6.0) as u64;
        let mut s = StyleBlock::new().set("--dl-cols", cols.to_string());
        if let Some(w) = node.prop("labelWidth")
            && let Some(v) = cx.style_value(node, "width", w)?
        {
            s.push("--dl-label", v);
        }
        let mut dl = cx.element("dl", node);
        dl.base_style_ref(&LIST).base_style(s).attr("data-layout", cx.prop_string(node, "layout").unwrap_or_else(|| "stacked".into()));
        if flag(cx, node, "divided", false) {
            dl.attr("data-divided", true);
        }
        cx.open(&dl, node)?;
        cx.children(node)?;
        if node.prop("items").is_some() {
            let mut list = Element::new("div");
            list.attr("data-entries", true).attr("role", "presentation");
            let key = expr_val(field_or(ENTRY, "label", path(&[ENTRY_INDEX])));
            let props = ["label", "value", "href"].iter().map(|f| (*f, expr_val(path(&[ENTRY, f])))).collect();
            let entry = synthetic(node, "ui-description-item", props, Vec::new(), Vec::new());
            // Copy and format are the same for every item a list gives.
            each_item(cx, node, list, items(cx, node, "items")?, ENTRY, ENTRY_INDEX, &key, |cx| cx.render(&entry))?;
            cx.close("div");
        }
        cx.close("dl");
        Ok(())
    }
}

impl Component for DescriptionItem {
    fn name(&self) -> &'static str {
        "ui-description-item"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "A label and its value in a `ui-description-list`; with `bind`, the value is edited in place.", props: ENTRY_PROPS, parts: &[], children: true, open_props: false }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let bind = cx.prop_string(node, "bind");
        if bind.is_some() && node.prop("value").is_some() {
            return Err(RenderError::new(node, "give `value` to show one, or `bind` to edit one, not both"));
        }
        let copy = flag(cx, node, "copy", false);
        let format = cx.prop_string(node, "format");
        let value = match &bind {
            Some(k) => expr_val(Expr::Path(vec![k.clone()])),
            None => node.prop("value").cloned().unwrap_or_else(|| lit(Json::Null)),
        };
        let mut el = cx.element("div", node);
        el.attr("data-dl-item", true).base_style_ref(&ENTRY_STYLE);
        if let Some(n) = widget::number(cx, node, "span") {
            el.base_style(StyleBlock::new().set("grid-column", format!("span {}", n.clamp(1.0, 6.0) as u64)));
        }
        let scripted = bind.is_some() || copy || format.is_some();
        if scripted {
            let mut config = widget::config(cx, node, &["options", "placeholder"], &["format", "currency", "edit"], &["bind"])?;
            if bind.is_none() {
                config.insert("value".into(), crate::data_table::dynamic(cx, node, "value", &value)?);
            }
            config.insert("locale".into(), json!(crate::locale(cx, node)));
            crate::cells(cx);
            widget::kit(cx);
            cx.global_js("ui-stat", SCRIPT);
            el.behavior("ui-description-item", Json::Object(config));
        }
        cx.open(&el, node)?;
        let mut dt = Element::new("dt");
        let label = node.prop("label").cloned().unwrap_or_else(|| lit(""));
        dt.text(label.clone());
        cx.open(&dt, node)?;
        cx.close("dt");
        cx.open(&Element::new("dd"), node)?;
        if !node.children.is_empty() {
            cx.children(node)?;
        } else {
            let href = node.prop("href").filter(|h| cx.is_reactive(h) || expr::truthy(&cx.eval(h)));
            let mut v = Element::new(if bind.is_some() { "button" } else if href.is_some() { "a" } else { "span" });
            let shown = crate::cond(crate::bin(expr::BinOp::Or, val_expr(&value), crate::bin(expr::BinOp::Eq, val_expr(&value), crate::num(0))), val_expr(&value), Expr::Lit(json!("—")));
            v.attr("data-dl-value", true).text(expr_val(shown));
            if let Some(h) = href {
                v.attr("href", h.clone());
            }
            if bind.is_some() {
                v.attr("type", "button").attr("data-dl-edit", true);
            }
            let tag = v.tag().to_string();
            cx.open(&v, node)?;
            cx.close(&tag);
        }
        if copy {
            let mut b = Element::new("button");
            b.attr("type", "button").attr("data-dl-tool", true).attr("data-dl-copy", true).attr("aria-label", cx.label_with("Copy {name}", &[("name", &expr::stringify(&cx.eval(&label)))]));
            cx.open(&b, node)?;
            cx.close("button");
        }
        cx.close("dd");
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- item

pub struct ItemGroup;
pub struct Item;

const GROUP_PROPS: &[PropDef] = &[
    PropDef::new("label", PropKind::Any, "Names the list for screen readers"),
    PropDef::new("variant", PropKind::Enum(&["divided", "cards", "plain"]), "Lines between (default), a card each, or neither"),
];
const ITEM_PROPS: &[PropDef] = &[
    PropDef::new("title", PropKind::Any, "Its name"),
    PropDef::new("description", PropKind::Any, "Smaller text under it"),
    PropDef::new("icon", PropKind::Any, "An icon name (with the icon kit) or a character, in a tinted square"),
    PropDef::new("image", PropKind::Any, "An image URL in its place"),
    PropDef::new("initials", PropKind::Any, "Or initials in a circle"),
    PropDef::new("meta", PropKind::Any, "Muted text at the end, e.g. a time or a size"),
    PropDef::new("tone", PropKind::Enum(status::TONES), "The icon's colour"),
    PropDef::new("href", PropKind::Any, "The whole item is a link; its children still work"),
    PropDef::new("selected", PropKind::Any, "Shown as the current one"),
    PropDef::new("disabled", PropKind::Any, "Shown as not available"),
    PropDef::new("size", PropKind::Enum(&["default", "sm"]), "Default `default`"),
    PropDef::new("variant", PropKind::Enum(&["plain", "outline", "muted"]), "Default `plain`"),
];

static GROUP: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("display", "flex"), ("flex-direction", "column"), ("min-width", "0")])
        .nest("&[data-variant=divided]", pairs(&[("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-card, var(--radius-lg))"), ("overflow", "hidden")]))
        .nest("&[data-variant=divided] [data-ui-item]", pairs(&[("border-radius", "0")]))
        .nest("&[data-variant=divided] [data-ui-item] + [data-ui-item]", pairs(&[("border-top", "var(--border-width, 1px) solid var(--color-border)")]))
        .nest("&[data-variant=divided]:not(:has([data-ui-item]))", pairs(&[("display", "none")]))
        .nest("&[data-variant=cards]", pairs(&[("gap", gap!(8))]))
        .nest("&[data-variant=cards] [data-ui-item]", pairs(&[("border", "var(--border-width, 1px) solid var(--color-border)")]))
});
static ITEM: LazyLock<StyleBlock> = LazyLock::new(|| {
    status::tone_rules(
        pairs(&[
            ("--tone", MUTED),
            ("position", "relative"),
            ("display", "flex"),
            ("align-items", "center"),
            ("gap", gap!(12)),
            ("min-width", "0"),
            ("padding", pad!(12, 16)),
            ("border-radius", "var(--radius-card, var(--radius-md))"),
            ("color", TEXT),
            ("font-size", fs!(14)),
        ]),
        "&",
    )
    .nest("&[data-size=sm]", pairs(&[("padding", pad!(8, 12)), ("gap", gap!(10))]))
    .nest("&[data-variant=outline]", pairs(&[("border", "var(--border-width, 1px) solid var(--color-border)")]))
    .nest("&[data-variant=muted]", pairs(&[("background", SUBTLE)]))
    .nest("&:is([data-link], [data-i-on]):hover", pairs(&[("background", SUBTLE)]))
    .nest("&[data-i-on]", pairs(&[("cursor", "pointer")]))
    .nest("&[aria-current=true]", pairs(&[("background", "color-mix(in srgb, var(--color-primary) 9%, transparent)")]))
    .nest("&[aria-disabled=true]", pairs(&[("opacity", "0.5")]))
    .nest("&:focus-visible", forms::focus_ring())
    .nest("& [data-item-media]", pairs(&[("flex", "none"), ("display", "grid"), ("place-items", "center"), ("width", "40px"), ("height", "40px"), ("overflow", "hidden"), ("border-radius", "var(--radius-md)"), ("background", "color-mix(in srgb, var(--tone) 14%, transparent)"), ("color", "var(--tone)"), ("font-size", fs!(14)), ("font-weight", "var(--font-weight-medium, 500)")]))
    .nest("&[data-size=sm] [data-item-media]", pairs(&[("width", "32px"), ("height", "32px")]))
    .nest("& [data-item-media][data-round]", pairs(&[("border-radius", "50%")]))
    .nest("& [data-item-media] img", pairs(&[("width", "100%"), ("height", "100%"), ("object-fit", "cover")]))
    .nest("& [data-item-text]", pairs(&[("flex", "1 1 auto"), ("min-width", "0")]))
    .nest("& [data-item-title]", pairs(&[("display", "block"), ("color", "inherit"), ("font-weight", "var(--font-weight-medium, 500)"), ("text-decoration", "none"), ("overflow-wrap", "anywhere")]))
    // The link covers the item; what's inside it that acts sits above.
    .nest("& a[data-item-title]::after", pairs(&[("content", "''"), ("position", "absolute"), ("inset", "0"), ("border-radius", "inherit")]))
    .nest("& a[data-item-title]:focus-visible", pairs(&[("outline", "none")]))
    .nest("&:has(a[data-item-title]:focus-visible)", forms::focus_ring())
    .nest("& [data-item-description]", pairs(&[("display", "block"), ("color", MUTED), ("font-size", fs!(13)), ("overflow-wrap", "anywhere")]))
    .nest("& [data-item-meta]", pairs(&[("flex", "none"), ("color", MUTED), ("font-size", fs!(13)), ("white-space", "nowrap")]))
    .nest("& [data-item-actions]", pairs(&[("position", "relative"), ("z-index", "1"), ("flex", "none"), ("display", "flex"), ("align-items", "center"), ("gap", gap!(8))]))
    .nest("@media (max-width: 480px)", pairs(&[("flex-wrap", "wrap")]).nest("& [data-item-actions]", pairs(&[("width", "100%"), ("justify-content", "flex-end")])).nest("& [data-item-text]", pairs(&[("flex", "1 1 0")])))
});

impl Component for ItemGroup {
    fn name(&self) -> &'static str {
        "ui-item-group"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "A list of `ui-item`s (a `list` works as its child).", props: GROUP_PROPS, parts: &[], children: true, open_props: false }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut el = cx.element("div", node);
        el.base_style_ref(&GROUP).attr("role", "list").attr("data-variant", cx.prop_string(node, "variant").unwrap_or_else(|| "divided".into()));
        if let Some(l) = node.prop("label") {
            el.attr("aria-label", l.clone());
        }
        cx.open(&el, node)?;
        cx.with_context("ui-item-group", Some(lit(true)), |cx| cx.children(node))?;
        cx.close("div");
        Ok(())
    }
}

impl Component for Item {
    fn name(&self) -> &'static str {
        "ui-item"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "A thing in a list: an icon, image or initials, a title and description, and its children (badges, buttons) at the end.", props: ITEM_PROPS, parts: &["title", "description", "media"], children: true, open_props: false }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut el = cx.element("div", node);
        el.base_style_ref(&ITEM)
            .attr("data-ui-item", true)
            .attr("data-size", cx.prop_string(node, "size").unwrap_or_else(|| "default".into()))
            .attr("data-variant", cx.prop_string(node, "variant").unwrap_or_else(|| "plain".into()));
        if crate::context(cx, "ui-item-group").is_some() {
            el.attr("role", "listitem");
        }
        if let Some(t) = node.prop("tone") {
            el.attr("data-tone", t.clone());
        }
        for (prop, attr) in [("selected", "aria-current"), ("disabled", "aria-disabled")] {
            if let Some(v) = node.prop(prop) {
                el.attr(attr, v.clone());
            }
        }
        let href = node.prop("href");
        if href.is_some() {
            el.attr("data-link", true);
        }
        cx.open(&el, node)?;
        let media = node.prop("image").map(|v| ("image", v)).or(node.prop("icon").map(|v| ("icon", v))).or(node.prop("initials").map(|v| ("initials", v)));
        if let Some((kind, v)) = media {
            let mut m = cx.part("span", node, "media");
            m.attr("data-item-media", true).attr("aria-hidden", "true");
            if kind == "initials" {
                m.attr("data-round", true).text(v.clone());
            }
            cx.open(&m, node)?;
            match kind {
                "image" => {
                    let mut img = Element::new("img");
                    img.attr("src", v.clone()).attr("alt", "");
                    cx.open(&img, node)?;
                }
                "icon" => icon(cx, node, v, 20)?,
                _ => {}
            }
            cx.close("span");
        }
        let mut text = Element::new("div");
        text.attr("data-item-text", true);
        cx.open(&text, node)?;
        if let Some(t) = node.prop("title") {
            let tag = if href.is_some() { "a" } else { "span" };
            let mut s = cx.part(tag, node, "title");
            s.attr("data-item-title", true).text(t.clone());
            if let Some(h) = href {
                s.attr("href", h.clone());
            }
            cx.open(&s, node)?;
            cx.close(tag);
        }
        if let Some(d) = node.prop("description") {
            branch(cx, d, |cx| {
                let mut s = cx.part("span", node, "description");
                s.attr("data-item-description", true).text(d.clone());
                cx.open(&s, node)?;
                cx.close("span");
                Ok(())
            })?;
        }
        cx.close("div");
        if let Some(m) = node.prop("meta") {
            let mut s = Element::new("span");
            s.attr("data-item-meta", true).text(m.clone());
            cx.open(&s, node)?;
            cx.close("span");
        }
        if !node.children.is_empty() {
            let mut a = Element::new("div");
            a.attr("data-item-actions", true);
            cx.open(&a, node)?;
            cx.children(node)?;
            cx.close("div");
        }
        cx.close("div");
        Ok(())
    }
}
