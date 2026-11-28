//! `ui-pivot`: rows summed up by fields down the side and across the top, drawn and
//! recomputed by `pivot.js` as the rows, the layout or the filters change.

use crate::{
    BG_POPOVER, BG_UNDER, BORDER, MUTED, PRIMARY, SUBTLE, SURFACE, TEXT, forms, lit,
    widget::{self, flag, pairs},
};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, expr};
use irender_html::{Component, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("pivot.js");
const AGGS: &[&str] = &["sum", "avg", "min", "max", "count", "distinct", "median", "first", "last"];

pub struct Pivot;

const PROPS: &[PropDef] = &[
    PropDef::new("rows", PropKind::Any, "The rows to sum up; may use state").required(),
    PropDef::new("fields", PropKind::Any, "The fields it can use: `[{ field, label, type, part }]` (default: those of the first row). `part` groups dates by `year`, `quarter`, `month`, `week` or `day`"),
    PropDef::new("down", PropKind::Any, "Fields down the side, outermost first"),
    PropDef::new("across", PropKind::Any, "Fields across the top"),
    PropDef::new("values", PropKind::Any, "What's in the cells: `[{ field, agg, label, format, currency, decimals, show, calc }]`; `agg`: sum (default), avg, min, max, count, distinct, median, first or last; `show`: value (default), row, column or total, as a share of it, or `running`, added to the ones above it in its group; `calc` (in place of `field`) works it out from the cell's sums, `{{ (sum.revenue - sum.cost) / sum.revenue }}`, `count` being its rows"),
    PropDef::new("filter", PropKind::Any, "Filters from outside, e.g. a `ui-filters`' `bind`: `{ field: condition }`"),
    PropDef::new("bind", PropKind::StateKey, "State key for the layout people pick, `{ down, across, values, sort, closed }`, to keep it"),
    PropDef::new("editable", PropKind::Bool, "A panel to move fields between down, across and values, and pick how they're summed"),
    PropDef::new("totals", PropKind::Enum(&["both", "rows", "columns", "none"]), "Grand totals (default `both`)"),
    PropDef::new("subtotals", PropKind::Bool, "Outer groups fold, and show their totals (default true)"),
    PropDef::new("heat", PropKind::Bool, "Cells tinted by their value"),
    PropDef::new("bindCell", PropKind::StateKey, "State key that gets a clicked cell: `{ down, across, value, rows }`, the rows behind it"),
    PropDef::new("export", PropKind::Any, "An Export button: `true`, or a file name ending in `.csv` or `.xlsx`"),
    PropDef::new("height", PropKind::Any, "Most it grows to before it scrolls, its headings kept in view"),
    PropDef::new("title", PropKind::Any, "Names the table"),
    PropDef::new("emptyText", PropKind::Any, "Default `No rows.`"),
    PropDef::new("locale", PropKind::String, "Default: the document's `lang`, else the browser's"),
];

static ROOT: LazyLock<StyleBlock> = LazyLock::new(|| {
    let field = || forms::native_scheme(pairs(&[("height", ctl!(30)), ("padding", pad!(0, 6)), ("border", "var(--border-width, 1px) solid var(--color-border)"), ("border-radius", "var(--radius-control, var(--radius-md))"), ("background", SURFACE), ("color", TEXT), ("font", "inherit")]));
    pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(10)), ("min-width", "0"), ("color", TEXT), ("font-size", fs!(14))])
        .nest("& [data-pv-bar]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("align-items", "flex-start"), ("gap", gap!(8, 16))]))
        .nest("& [data-pv-zone]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("align-items", "center"), ("gap", gap!(6)), ("min-height", ctl!(34)), ("padding", pad!(4, 8)), ("border", "var(--border-width, 1px) dashed var(--color-border)"), ("border-radius", "var(--radius-control, var(--radius-md))")]))
        .nest("& [data-pv-zone][data-over]", pairs(&[("border-color", PRIMARY), ("background", "color-mix(in srgb, var(--color-primary) 8%, transparent)")]))
        .nest("& [data-pv-zone] > small", pairs(&[("color", MUTED)]))
        .nest("& [data-pv-chip]", pairs(&[("display", "inline-flex"), ("align-items", "center"), ("gap", gap!(2)), ("height", ctl!(26)), ("padding-inline", pad!(8, 2)), ("border-radius", "var(--radius-sm)"), ("background", SUBTLE), ("font-size", fs!(13)), ("cursor", "grab")]))
        .nest("& [data-pv-chip] :is(select)", pairs(&[("border", "0"), ("background", "none"), ("color", MUTED), ("font", "inherit"), ("font-size", fs!(12)), ("max-width", "84px")]))
        .nest("& [data-pv-chip] button", widget::tool().set("min-width", "22px").set("height", "22px"))
        .nest("& [data-pv-chip] button:hover", pairs(&[("color", TEXT)]))
        .nest("& [data-pv-zone] > select", field().set("border", "0").set("background", "none").set("color", PRIMARY).set("width", "auto"))
        .nest("& :is(button, select):focus-visible", forms::focus_ring())
        .nest("& [data-pv-export]", field().set("padding", "0 10px").set("margin-inline-start", "auto").set("cursor", "pointer"))
        .nest("& [data-pv-scroll]", pairs(&[("overflow", "auto"), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-control, var(--radius-md))")]))
        .nest("& table", pairs(&[("border-collapse", "separate"), ("border-spacing", "0"), ("min-width", "100%")]))
        .nest("& :is(th, td)", pairs(&[("padding", pad!(6, 10)), ("border-bottom", "var(--border-width, 1px) solid var(--color-border)"), ("white-space", "nowrap"), ("text-align", "end"), ("font-variant-numeric", "tabular-nums")]))
        .nest("& th", pairs(&[("font-weight", "var(--font-weight-medium, 500)"), ("text-align", "start")]))
        .nest("& thead th", pairs(&[("position", "sticky"), ("top", "0"), ("z-index", "1"), ("background", "color-mix(in srgb, var(--color-text) 4%, var(--ui-bg, var(--color-background, var(--color-surface))))")]))
        .nest("& thead th[data-col]", pairs(&[("text-align", "end")]))
        .nest("& thead th[data-span]", pairs(&[("text-align", "center"), ("border-inline-start", "var(--border-width, 1px) solid var(--color-border)")]))
        .nest("& tbody th, & tfoot th", pairs(&[("position", "sticky"), ("inset-inline-start", "0"), ("z-index", "1"), ("background", BG_UNDER)]))
        .nest("& thead th[data-corner]", pairs(&[("inset-inline-start", "0"), ("z-index", "2")]))
        .nest("& [data-depth] > th", pairs(&[("padding-inline-start", "calc(10px + var(--d, 0) * 18px)")]))
        .nest("& [data-sub] :is(th, td)", pairs(&[("font-weight", "var(--font-weight-semibold, 600)")]))
        .nest("& [data-total], & tfoot :is(th, td)", pairs(&[("font-weight", "var(--font-weight-semibold, 600)"), ("background", "color-mix(in srgb, var(--color-text) 4%, var(--ui-bg, var(--color-background, var(--color-surface))))")]))
        .nest("& tfoot :is(th, td)", pairs(&[("position", "sticky"), ("bottom", "0"), ("border-bottom", "0"), ("border-top", "var(--border-width, 1px) solid var(--color-border)")]))
        .nest("& tfoot th", pairs(&[("z-index", "2")]))
        .nest("& [data-pv-fold], & [data-pv-sort]", pairs(&[("display", "inline-flex"), ("align-items", "center"), ("gap", gap!(4)), ("padding", "0"), ("border", "0"), ("background", "none"), ("color", "inherit"), ("font", "inherit"), ("cursor", "pointer")]))
        .nest("& [data-pv-fold] svg", pairs(&[("color", MUTED), ("transition", "transform .15s")]))
        .nest("& [data-pv-fold][aria-expanded=true] svg", pairs(&[("transform", "rotate(90deg)")]))
        .nest("& [data-pv-sort] svg", pairs(&[("color", MUTED)]))
        .nest("& td[data-cell]", pairs(&[("background", "color-mix(in srgb, var(--color-primary) calc(var(--h, 0) * 45%), transparent)")]))
        .nest("& td[data-click]", pairs(&[("cursor", "pointer")]))
        .nest("& td[data-click]:is(:hover, :focus-visible)", pairs(&[("outline", "2px solid var(--color-ring, var(--color-muted))"), ("outline-offset", "-2px")]))
        .nest("& td[aria-selected=true]", pairs(&[("box-shadow", "inset 0 0 0 2px var(--color-primary)")]))
        .nest("& [data-pv-none]", pairs(&[("margin", "0"), ("padding", pad!(24)), ("color", MUTED), ("text-align", "center")]))
        .nest("& [data-pv-pop]", pairs(&[("background", BG_POPOVER)]))
});

fn check(cx: &RenderCx<'_>, node: &Node) -> Result<(), RenderError> {
    if let Some(Json::Array(vs)) = cx.prop(node, "values") {
        for v in vs {
            if let Some(a) = v.get("agg").filter(|a| !a.is_null()).map(expr::stringify)
                && !AGGS.contains(&a.as_str())
            {
                return Err(RenderError::new(node, format!("`agg` must be one of {}, got `{a}`", AGGS.join(", "))));
            }
        }
    }
    Ok(())
}

impl Component for Pivot {
    fn name(&self) -> &'static str {
        "ui-pivot"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "A pivot table: rows summed up by fields down and across, with subtotals that fold, shares, a heat tint, sorting, and the rows behind a cell.", props: PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        check(cx, node)?;
        let mut config = widget::config(cx, node, &["rows", "filter", "emptyText"], &["fields", "down", "across", "values", "totals"], &["bind", "bindCell"])?;
        for (f, d) in [("editable", false), ("subtotals", true), ("heat", false)] {
            config.insert(f.into(), json!(flag(cx, node, f, d)));
        }
        let export = match cx.prop(node, "export") {
            None | Some(Json::Null) | Some(Json::Bool(false)) => None,
            Some(Json::String(s)) if s.ends_with(".csv") || s.ends_with(".xlsx") => Some(s),
            _ => Some("pivot.xlsx".into()),
        };
        config.insert("export".into(), json!(export));
        // Calculated values, compiled for the browser: by their text, so a layout kept in
        // state finds them too. They see the cell's sums by field (`sum`) and its rows (`count`).
        let mut calcs = serde_json::Map::new();
        if let Some(Json::Array(vs)) = cx.prop(node, "values") {
            for v in vs {
                let Some(Json::String(text)) = v.get("calc") else { continue };
                let src = if text.contains("{{") { text.clone() } else { format!("{{{{ {text} }}}}") };
                let t = irender_core::Template::parse(&src).map_err(|e| RenderError::new(node, format!("`calc` `{text}`: {e}")))?;
                calcs.insert(text.clone(), cx.item_val_js(&irender_core::Val::Tpl(t), "sum", "count"));
            }
        }
        config.insert("calcs".into(), Json::Object(calcs));
        config.insert("title".into(), json!(cx.prop_string(node, "title")));
        config.insert("locale".into(), json!(crate::locale(cx, node)));
        widget::kit(cx);
        crate::sheets(cx);
        cx.global_js("ui-pivot", SCRIPT);
        let mut s = StyleBlock::new();
        if let Some(h) = node.prop("height")
            && let Some(v) = cx.style_value(node, "max-height", h)?
        {
            s.push("--pv-h", v);
        }
        let mut el = cx.element("div", node);
        el.base_style_ref(&ROOT).base_style(s).base_style(StyleBlock::new().nest("& [data-pv-scroll]", pairs(&[("max-height", "var(--pv-h, none)")])));
        el.attr("role", "group").attr("aria-label", node.prop("title").cloned().unwrap_or_else(|| lit(cx.label("Pivot table")))).behavior("ui-pivot", Json::Object(config));
        cx.open(&el, node)?;
        cx.close("div");
        Ok(())
    }
}
