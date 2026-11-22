//! `ui-filters`: filters for a list, set in a side panel and shown as chips. Each
//! `ui-filter` child is one field: a list of values to pick, a number or amount, a date
//! or text, each with its operators (is any of, between, before, contains, is empty, …).
//!
//! It writes what's applied to `bind` as `{ field: condition }`, in `ui-chart`'s
//! `filter` format, so the same object filters a chart, a `ui-data-table` (its
//! `filter`), or goes to an API. `filters.js` keeps a draft while the panel is open and
//! writes it on Apply, draws the chips, and lists a field's values from `rows` with how
//! many rows have each.

use crate::data_table::dynamic;
use crate::overlay::{DIALOG_TITLE, DIALOG_X, SHEETS};
use crate::{ACCENT, MUTED, PRIMARY_FILL, SECONDARY, TEXT, forms, id_with, instance_id, lit};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, Val, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("filters.js");

pub(crate) const TYPES: &[&str] = &["list", "number", "currency", "date", "text"];

/// Each type's operators: the name the behaviour uses, and its label.
// Labels: "Is any of", "Is none of", "Is empty", "Is not empty", "Contains", "Is", "Between", "On", "Before", "After", "Equals", "More than", "Less than"
fn operators(kind: &str) -> &'static [(&'static str, &'static str)] {
    match kind {
        "list" => &[("any", "Is any of"), ("none", "Is none of"), ("empty", "Is empty"), ("set", "Is not empty")],
        "text" => &[("contains", "Contains"), ("is", "Is"), ("empty", "Is empty"), ("set", "Is not empty")],
        "date" => &[("between", "Between"), ("on", "On"), ("before", "Before"), ("after", "After"), ("empty", "Is empty"), ("set", "Is not empty")],
        _ => &[("eq", "Equals"), ("gt", "More than"), ("lt", "Less than"), ("between", "Between"), ("empty", "Is empty"), ("set", "Is not empty")],
    }
}

pub struct Filters;

const PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key for the applied filters, `{ field: condition }`, as `ui-chart`'s `filter`").required(),
    PropDef::new("rows", PropKind::Any, "Rows the filters are for, e.g. `{{ orders }}`: list filters without `options` offer their field's values, each with a count"),
    PropDef::new("title", PropKind::Any, "The panel's heading (default `Filters`)"),
    PropDef::new("label", PropKind::Any, "The button's text (default `Filters`)"),
    PropDef::new("live", PropKind::Bool, "Apply each change at once, with no Apply button"),
    PropDef::new("chips", PropKind::Bool, "Show the applied filters as chips beside the button (default true)"),
    PropDef::new("currency", PropKind::String, "Currency code for `currency` filters, e.g. `INR` (default `USD`)"),
    PropDef::new("locale", PropKind::String, "Locale for numbers and dates in chips (default: the document's `lang`, else the browser's)"),
    PropDef::new("bindViews", PropKind::StateKey, "State key for saved views, `[{ name, filters }]`: a Views menu applies, saves and deletes them"),
    PropDef::new("today", PropKind::String, "The date date presets count from (default: today)"),
    PropDef::new(
        "searchFields",
        PropKind::Bool,
        "A box over the fields that finds one by its name or section (default: on past eight fields)",
    ),
    PropDef::new("fiscalStart", PropKind::Number, "The month a financial year starts, 1–12 (default 1): the quarter and year presets follow it"),
    PropDef::new("id", PropKind::Any, "The panel's element id (default: derived from the document)"),
];

/// Date presets, in order: the name, its label.
// Labels: "Today", "Yesterday", "Last 7 days", "Last 30 days", "This month", "Last month", "This quarter", "This year"
pub(crate) const PRESETS: &[(&str, &str)] = &[
    ("today", "Today"),
    ("yesterday", "Yesterday"),
    ("last7", "Last 7 days"),
    ("last30", "Last 30 days"),
    ("thisMonth", "This month"),
    ("lastMonth", "Last month"),
    ("thisQuarter", "This quarter"),
    ("thisYear", "This year"),
];
/// Presets a filter can ask for beyond the default ones. `quarters` is the quarter
/// today is in and the three before it, named by the script ("Q2 2026").
// Labels: "Last quarter", "Last year", "Year to date"
const MORE_PRESETS: &[(&str, &str)] = &[("lastQuarter", "Last quarter"), ("lastYear", "Last year"), ("yearToDate", "Year to date"), ("quarters", "")];

pub struct Filter;

const FILTER_PROPS: &[PropDef] = &[
    PropDef::new("field", PropKind::String, "The rows' field it filters").required(),
    PropDef::new("label", PropKind::Any, "Its name in the panel and chips (default: the field)"),
    PropDef::new("type", PropKind::Enum(TYPES), "`list` (default): values to pick; `number`, `currency`, `date` or `text`"),
    PropDef::new(
        "options",
        PropKind::Any,
        "A list filter's values: text, or `{ value, label }`. May use state. Default: the field's values in the filters' `rows`",
    ),
    PropDef::new(
        "optionsUrl",
        PropKind::Any,
        "A list filter's values from an API as they're searched for: GET `<url>?q=<text>`, answering texts or `{ value, label, count }`s. May use state",
    ),
    PropDef::new("operators", PropKind::Any, "Which operators to offer, by name, e.g. `[between, before, after]` (default: all of the type's)"),
    PropDef::new("section", PropKind::String, "A heading to group it under in the panel"),
    PropDef::new("unit", PropKind::String, "Shown after a number, e.g. `kg`"),
    PropDef::new("currency", PropKind::String, "This filter's currency, over the filters' `currency`"),
    PropDef::new(
        "presets",
        PropKind::Any,
        "A date filter's quick ranges: `true` (default: today, yesterday, last7, last30, thisMonth, lastMonth, thisQuarter, thisYear), `false`, or a list of those names",
    ),
];

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

static ROOT: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("align-items", "center"), ("gap", gap!(8))])
        .nest("& [data-fl-views-wrap]", pairs(&[("position", "relative")]))
        .nest(
            "& [data-fl-views]",
            pairs(&[
                ("position", "absolute"),
                ("z-index", "20"),
                ("top", "calc(100% + 4px)"),
                ("inset-inline-start", "0"),
                ("display", "flex"),
                ("flex-direction", "column"),
                ("gap", gap!(6)),
                ("width", "260px"),
                ("padding", pad!(6)),
                ("border", "var(--border-width, 1px) solid var(--color-border)"),
                ("border-radius", "var(--radius-control, var(--radius-md))"),
                ("background", "var(--color-popover, var(--color-surface, #fff))"),
                ("box-shadow", "0 8px 24px rgb(0 0 0 / 0.12)"),
            ]),
        )
        .nest("& [data-fl-views][hidden]", pairs(&[("display", "none")]))
        .nest("& [data-fl-view-list]", pairs(&[("margin", "0"), ("padding", "0"), ("list-style", "none"), ("display", "flex"), ("flex-direction", "column"), ("gap", "1px")]))
        .nest("& [data-fl-view-list] li", pairs(&[("display", "flex"), ("align-items", "center")]))
        .nest(
            "& [data-fl-view-list] button",
            pairs(&[("border", "0"), ("border-radius", "var(--radius-sm)"), ("background", "transparent"), ("color", TEXT), ("font", "inherit"), ("font-size", fs!(14)), ("cursor", "pointer")]),
        )
        .nest("& [data-fl-view-apply]", pairs(&[("flex", "1 1 auto"), ("min-width", "0"), ("padding", pad!(6, 8)), ("text-align", "start"), ("overflow", "hidden"), ("text-overflow", "ellipsis"), ("white-space", "nowrap")]))
        .nest("& [data-fl-view-apply][aria-current=true]", pairs(&[("font-weight", "var(--font-weight-semibold, 600)")]))
        .nest("& [data-fl-view-apply][aria-current=true]::after", pairs(&[("content", "' ✓'"), ("color", MUTED)]))
        .nest("& [data-fl-view-delete]", pairs(&[("flex", "none"), ("padding", pad!(4, 8)), ("color", MUTED)]))
        .nest("& [data-fl-view-list] button:hover", pairs(&[("background", ACCENT)]))
        .nest("& [data-fl-view-list] button:focus-visible", forms::focus_ring())
        .nest("& [data-fl-view-empty]", pairs(&[("padding", pad!(6, 8)), ("color", MUTED), ("font-size", fs!(13))]))
        .nest("& [data-fl-view-form]", pairs(&[("display", "flex"), ("gap", gap!(6)), ("padding-top", pad!(6)), ("border-top", "var(--border-width, 1px) solid var(--color-border)")]))
        .nest("& [data-fl-view-form] input", pairs(&[("height", "32px")]))
});

/// Count badges, on the button and beside each field in the panel.
fn badge() -> StyleBlock {
    pairs(&[
        ("display", "inline-flex"),
        ("align-items", "center"),
        ("justify-content", "center"),
        ("min-width", "18px"),
        ("height", "18px"),
        ("padding", pad!(0, 5)),
        ("box-sizing", "border-box"),
        ("border-radius", "999px"),
        ("background", PRIMARY_FILL),
        ("color", "var(--color-on-primary, #fff)"),
        ("font-size", fs!(11)),
        ("font-weight", "var(--font-weight-semibold, 600)"),
        ("font-variant-numeric", "tabular-nums"),
    ])
    .nest("&[hidden]", pairs(&[("display", "none")]))
}

static CHIPS: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("display", "contents")])
        .nest(
            "& [data-fl-chip]",
            pairs(&[
                ("display", "inline-flex"),
                ("align-items", "center"),
                ("max-width", "280px"),
                ("height", ctl!(32)),
                ("border", "var(--border-width, 1px) solid var(--color-border)"),
                ("border-radius", "999px"),
                ("background", SECONDARY),
                ("font-size", fs!(13)),
                ("overflow", "hidden"),
            ]),
        )
        .nest(
            "& [data-fl-chip] > button",
            pairs(&[
                ("display", "inline-flex"),
                ("align-items", "center"),
                ("gap", gap!(4)),
                ("min-width", "0"),
                ("height", "100%"),
                ("border", "0"),
                ("background", "transparent"),
                ("color", TEXT),
                ("font", "inherit"),
                ("cursor", "pointer"),
            ]),
        )
        .nest("& [data-fl-chip] > button:first-child", pairs(&[("padding-block", pad!(0, 0)), ("padding-inline", pad!(12, 4))]))
        .nest("& [data-fl-chip] > button:last-child", pairs(&[("padding-block", pad!(0, 0)), ("padding-inline", pad!(6, 10)), ("color", MUTED)]))
        .nest("& [data-fl-chip] > button:hover", pairs(&[("background", ACCENT), ("color", TEXT)]))
        .nest("& [data-fl-chip] > button:focus-visible", forms::focus_ring().set("outline-offset", "-2px"))
        .nest("& [data-fl-chip] b", pairs(&[("font-weight", "400"), ("color", MUTED), ("white-space", "nowrap")]))
        .nest("& [data-fl-chip] span", pairs(&[("overflow", "hidden"), ("text-overflow", "ellipsis"), ("white-space", "nowrap"), ("font-weight", "var(--font-weight-medium, 500)")]))
});

static PANEL: LazyLock<StyleBlock> = LazyLock::new(|| {
    SHEETS[0]
        .clone()
        .set("width", "min(640px, 96vw)")
        .set("padding", "0")
        .nest("&[open]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", "0")]))
});
static HEAD: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("display", "flex"), ("align-items", "center"), ("gap", gap!(8)), ("padding-block", pad!(20, 16)), ("padding-inline", pad!(24, 56)), ("border-bottom", "var(--border-width, 1px) solid var(--color-border)")])
});
static BODY: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("flex", "1 1 auto"), ("min-height", "0"), ("display", "grid"), ("grid-template-columns", "minmax(150px, 36%) minmax(0, 1fr)")])
        // On a phone: the fields in a row of tabs above their controls.
        .nest(
            "@media (max-width: 560px)",
            pairs(&[("grid-template-columns", "minmax(0, 1fr)"), ("grid-template-rows", "auto minmax(0, 1fr)")])
                .nest("& [data-fl-fields]", pairs(&[("border-inline-end", "0"), ("border-bottom", "var(--border-width, 1px) solid var(--color-border)")]))
                .nest("& [data-fl-fields] > [role=tablist]", pairs(&[("flex-direction", "row"), ("overflow-x", "auto")]))
                .nest("& [role=tablist] > [role=tab]", pairs(&[("width", "auto"), ("flex", "none")]))
                .nest("& [data-fl-section]", pairs(&[("display", "none")])),
        )
});
static TABS: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("display", "flex"),
        ("flex-direction", "column"),
        ("gap", gap!(2)),
        ("padding", pad!(12)),
        ("border-inline-end", "var(--border-width, 1px) solid var(--color-border)"),
        ("overflow-y", "auto"),
    ])
    .nest(
        "& [data-fl-section]",
        pairs(&[("padding", pad!(12, 8, 4)), ("color", MUTED), ("font-size", fs!(12)), ("font-weight", "var(--font-weight-medium, 500)")]),
    )
    .nest("& [data-fl-section]:first-child", pairs(&[("padding-top", pad!(4))]))
    .nest("& [role=tablist]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(2))]))
    .nest("& [data-fl-find]", pairs(&[("margin-bottom", gap!(8)), ("flex-shrink", "0")]))
    .nest("& [data-fl-unfound]", pairs(&[("padding", pad!(8)), ("color", MUTED), ("font-size", fs!(13))]))
    .nest(
        "& [role=tab]",
        pairs(&[
            ("display", "flex"),
            ("align-items", "center"),
            ("justify-content", "space-between"),
            ("gap", gap!(8)),
            ("width", "100%"),
            ("padding", pad!(8)),
            ("border", "0"),
            ("border-radius", "var(--radius-control, var(--radius-md))"),
            ("background", "transparent"),
            ("color", TEXT),
            ("font", "inherit"),
            ("font-size", fs!(14)),
            ("text-align", "start"),
            ("cursor", "pointer"),
        ]),
    )
    .nest("& [role=tab]:hover", pairs(&[("background", ACCENT)]))
    .nest("& [role=tab][aria-selected=true]", pairs(&[("background", ACCENT), ("font-weight", "var(--font-weight-semibold, 600)")]))
    .nest("& [role=tab]:focus-visible", forms::focus_ring())
    .nest("& [data-fl-n]", badge())
    // Last, over the tabs' own `display`: fields the box leaves out.
    .nest("& [role=tab][hidden], & [data-fl-section][hidden], & [data-fl-unfound][hidden]", pairs(&[("display", "none")]))
});
static PANE: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(12)), ("min-width", "0"), ("padding", pad!(16, 20)), ("overflow-y", "auto")])
        .nest("&[hidden]", pairs(&[("display", "none")]))
        .nest("& h3", pairs(&[("margin", "0"), ("font-size", fs!(15)), ("font-weight", "var(--font-weight-semibold, 600)")]))
        .nest("& [data-fl-values]", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", gap!(8))]))
        .nest("& [data-fl-values][hidden], & [data-fl-and][hidden], & [data-fl-b][hidden]", pairs(&[("display", "none")]))
        .nest("& [data-fl-values] [data-fl-field]", pairs(&[("position", "relative"), ("flex", "1 1 0"), ("display", "flex"), ("min-width", "0")]))
        .nest("& [data-fl-values] [data-fl-field][hidden]", pairs(&[("display", "none")]))
        .nest("& [data-fl-values] input", pairs(&[("flex", "1 1 0"), ("min-width", "0")]))
        .nest("& [data-fl-num]", pairs(&[("text-align", "end"), ("font-variant-numeric", "tabular-nums")]))
        .nest("& [data-fl-prefix]", pairs(&[("position", "absolute"), ("inset-inline-start", pad!(10)), ("top", "50%"), ("transform", "translateY(-50%)"), ("color", MUTED), ("font-size", fs!(14)), ("pointer-events", "none")]))
        .nest("& [data-fl-prefix] + input", pairs(&[("padding-inline-start", pad!(26))]))
        // With the calendar there, the date fields only keep its values.
        .nest("& [data-fl-dates]", pairs(&[("display", "none")]))
        .nest("& [data-fl-cal][hidden]", pairs(&[("display", "none")]))
        .nest("& [data-fl-presets]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("gap", gap!(6))]))
        .nest(
            "& [data-fl-preset]",
            pairs(&[
                ("padding", pad!(4, 10)),
                ("border", "var(--border-width, 1px) solid var(--color-border)"),
                ("border-radius", "999px"),
                ("background", "transparent"),
                ("color", TEXT),
                ("font", "inherit"),
                ("font-size", fs!(13)),
                ("cursor", "pointer"),
            ]),
        )
        .nest("& [data-fl-preset]:hover", pairs(&[("background", ACCENT)]))
        .nest("& [data-fl-preset][aria-pressed=true]", pairs(&[("border-color", "var(--color-primary)"), ("background", "color-mix(in srgb, var(--color-primary) 10%, transparent)"), ("font-weight", "var(--font-weight-medium, 500)")]))
        .nest("& [data-fl-preset]:focus-visible", forms::focus_ring())
        .nest("& [data-fl-and], & [data-fl-unit]", pairs(&[("color", MUTED), ("font-size", fs!(13)), ("white-space", "nowrap")]))
        .nest("& [data-fl-error]", pairs(&[("margin", "0"), ("color", "var(--color-destructive, #dc2626)"), ("font-size", fs!(13))]))
        .nest("& [data-fl-options]", pairs(&[("display", "flex"), ("flex-direction", "column")]))
        .nest("& [data-fl-options][hidden], & [data-fl-search][hidden], & [data-fl-op][hidden]", pairs(&[("display", "none")]))
        .nest(
            "& [data-fl-option]",
            pairs(&[
                ("display", "flex"),
                ("align-items", "center"),
                ("gap", gap!(10)),
                ("padding", pad!(8, 4)),
                ("border-bottom", "var(--border-width, 1px) solid var(--color-border)"),
                ("font-size", fs!(14)),
                ("cursor", "pointer"),
            ]),
        )
        .nest("& [data-fl-option]:hover", pairs(&[("background", ACCENT)]))
        .nest("& [data-fl-option] > span:nth-of-type(1)", pairs(&[("flex", "1 1 auto"), ("min-width", "0"), ("overflow-wrap", "anywhere")]))
        .nest("& [data-fl-option] > span:nth-of-type(2)", pairs(&[("color", MUTED), ("font-size", fs!(12)), ("font-variant-numeric", "tabular-nums")]))
        .nest("& [data-fl-none]", pairs(&[("margin", "0"), ("padding", pad!(8, 4)), ("color", MUTED), ("font-size", fs!(14))]))
});
static FOOT: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("display", "flex"),
        ("align-items", "center"),
        ("gap", gap!(8)),
        ("padding", pad!(12, 24)),
        ("border-top", "var(--border-width, 1px) solid var(--color-border)"),
    ])
    .nest("& > :first-child", pairs(&[("margin-inline-end", "auto")]))
});

// Decorative: the button around it is labelled.
const SLIDERS: &str = r#"<svg width="14" height="14" style="width:calc(14px * var(--size-icon, 1));height:calc(14px * var(--size-icon, 1))" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" aria-hidden="true"><path d="M2.5 4.5h6M11.5 4.5h2M2.5 11.5h2M7.5 11.5h6"/><circle cx="10" cy="4.5" r="1.5"/><circle cx="6" cy="11.5" r="1.5"/></svg>"#;

/// One `ui-filter`, read from the child.
struct Def {
    field: String,
    label: String,
    kind: String,
    ops: Vec<(&'static str, &'static str)>,
    section: Option<String>,
    options: Json,
    options_url: Json,
    unit: Option<String>,
    currency: Option<String>,
    presets: Vec<(&'static str, &'static str)>,
}

fn defs(cx: &RenderCx<'_>, node: &Node) -> Result<Vec<Def>, RenderError> {
    let mut out = Vec::new();
    for child in &node.children {
        if child.kind != "ui-filter" {
            return Err(RenderError::new(child, format!("`ui-filters` takes `ui-filter` children, not `{}`", child.kind)));
        }
        let field = cx.prop_string(child, "field").filter(|f| !f.is_empty()).ok_or_else(|| RenderError::new(child, "a filter needs a `field`"))?;
        if out.iter().any(|d: &Def| d.field == field) {
            return Err(RenderError::new(child, format!("two filters for `{field}`")));
        }
        let kind = cx.prop_string(child, "type").unwrap_or_else(|| "list".into());
        let all = operators(&kind);
        let ops = match cx.prop(child, "operators") {
            None | Some(Json::Null) => all.to_vec(),
            Some(Json::Array(names)) => {
                let mut ops = Vec::new();
                for n in &names {
                    let n = irender_core::expr::stringify(n);
                    let op = all.iter().find(|(o, _)| *o == n).ok_or_else(|| {
                        let names: Vec<_> = all.iter().map(|(o, _)| *o).collect();
                        RenderError::new(child, format!("a `{kind}` filter has no operator `{n}` (it has {})", names.join(", ")))
                    })?;
                    ops.push(*op);
                }
                if ops.is_empty() {
                    return Err(RenderError::new(child, "`operators` must name at least one"));
                }
                ops
            }
            Some(_) => return Err(RenderError::new(child, "`operators` must be a list of names")),
        };
        if kind != "list" && (child.prop("options").is_some() || child.prop("optionsUrl").is_some()) {
            return Err(RenderError::new(child, "only `list` filters take `options` or `optionsUrl`"));
        }
        crate::widget::check_url(cx, child, "optionsUrl")?;
        let presets = match cx.prop(child, "presets") {
            _ if kind != "date" => Vec::new(),
            None | Some(Json::Null) | Some(Json::Bool(true)) => PRESETS.to_vec(),
            Some(Json::Bool(false)) => Vec::new(),
            Some(Json::Array(names)) => names
                .iter()
                .map(|n| {
                    let n = irender_core::expr::stringify(n);
                    PRESETS.iter().chain(MORE_PRESETS).find(|(p, _)| *p == n).copied().ok_or_else(|| {
                        let all: Vec<_> = PRESETS.iter().chain(MORE_PRESETS).map(|(p, _)| *p).collect();
                        RenderError::new(child, format!("no date preset `{n}` (presets: {})", all.join(", ")))
                    })
                })
                .collect::<Result<_, _>>()?,
            Some(_) => return Err(RenderError::new(child, "`presets` must be true, false or a list of preset names")),
        };
        out.push(Def {
            label: cx.prop_string(child, "label").unwrap_or_else(|| field.clone()),
            options_url: child.prop("optionsUrl").map(|o| dynamic(cx, child, "optionsUrl", o)).transpose()?.unwrap_or(Json::Null),
            options: child.prop("options").map(|o| dynamic(cx, child, "options", o)).transpose()?.unwrap_or(Json::Null),
            section: cx.prop_string(child, "section"),
            unit: cx.prop_string(child, "unit"),
            currency: cx.prop_string(child, "currency"),
            presets,
            field,
            kind,
            ops,
        });
    }
    if out.is_empty() {
        return Err(RenderError::new(node, "`ui-filters` needs `ui-filter` children"));
    }
    Ok(out)
}

impl Component for Filters {
    fn name(&self) -> &'static str {
        "ui-filters"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Filters for a list: a button that opens a side panel of fields, each with its operators, and the applied filters as \
                  chips. Writes `{ field: condition }` to `bind`, for a chart's or data table's `filter`. Children are `ui-filter`s.",
            props: PROPS,
            parts: &["button", "panel"],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let key = cx.prop_string(node, "bind").ok_or_else(|| RenderError::new(node, "`ui-filters` needs `bind`"))?;
        let defs = defs(cx, node)?;
        let live = cx.prop(node, "live").is_some_and(|v| irender_core::expr::truthy(&v));
        let chips = cx.prop(node, "chips").is_none_or(|v| irender_core::expr::truthy(&v));
        let title = node.prop("title").cloned().unwrap_or_else(|| lit(cx.label("Filters")));
        let id = instance_id(cx, node, "filters");

        cx.global_js("ui-filters", SCRIPT);
        let fiscal = cx.prop(node, "fiscalStart").filter(|v| !v.is_null()).map_or(1.0, |v| irender_core::expr::to_num(&v));
        if !(1.0..=12.0).contains(&fiscal) || fiscal.fract() != 0.0 {
            return Err(RenderError::new(node, "`fiscalStart` must be a month, 1–12"));
        }
        let config = json!({
            "bind": key,
            "live": live,
            "rows": node.prop("rows").map(|r| dynamic(cx, node, "rows", r)).transpose()?,
            "currency": cx.prop_string(node, "currency"),
            "locale": crate::locale(cx, node),
            "bindViews": cx.prop_string(node, "bindViews"),
            "today": cx.prop_string(node, "today"),
            "fiscalStart": fiscal,
            "defs": defs.iter().map(|d| json!({
                "field": d.field, "label": d.label, "section": d.section, "type": d.kind, "options": d.options, "optionsUrl": d.options_url, "unit": d.unit, "currency": d.currency,
                "ops": d.ops.iter().map(|(o, _)| *o).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        });
        let mut root = cx.element("div", node);
        root.base_style_ref(&ROOT).behavior("ui-filters", config);
        cx.open(&root, node)?;

        let mut button = cx.part("button", node, "button");
        forms::button_look(&mut button, "outline", "sm");
        button.attr("type", "button").attr("data-fl-open", true).attr("aria-haspopup", "dialog").attr("aria-controls", id.clone());
        cx.open(&button, node)?;
        cx.markup(SLIDERS);
        let mut label = Element::new("span");
        label.text(node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Filters"))));
        cx.open(&label, node)?;
        cx.close("span");
        let mut count = Element::new("span");
        count.base_style(badge()).attr("data-fl-count", true).attr("hidden", true);
        cx.open(&count, node)?;
        cx.close("span");
        cx.close("button");
        if cx.prop_string(node, "bindViews").is_some() {
            views_menu(cx, node, &id)?;
        }
        if chips {
            let mut list = Element::new("div");
            list.base_style_ref(&CHIPS).attr("data-fl-chips", true);
            cx.open(&list, node)?;
            cx.close("div");
            let mut clear = Element::new("button");
            forms::button_look(&mut clear, "ghost", "sm");
            clear.attr("type", "button").attr("data-fl-clear", true).attr("hidden", true).text(lit(cx.label("Clear all")));
            cx.open(&clear, node)?;
            cx.close("button");
        }

        // The panel: fields on the left, the chosen one's controls on the right.
        let mut panel = cx.part("dialog", node, "panel");
        panel.base_style_ref(&PANEL).attr("id", id.clone()).attr("closedby", "any").attr("aria-labelledby", id_with(&id, "-title")).attr("data-fl-panel", true);
        cx.open(&panel, node)?;
        let mut head = Element::new("div");
        head.base_style_ref(&HEAD);
        cx.open(&head, node)?;
        let mut h = Element::new("h2");
        h.base_style_ref(&DIALOG_TITLE).attr("id", id_with(&id, "-title")).text(title);
        cx.open(&h, node)?;
        cx.close("h2");
        cx.close("div");

        let mut body = Element::new("div");
        body.base_style_ref(&BODY);
        cx.open(&body, node)?;
        // The fields, with a box that finds one when there are many.
        let mut fields = Element::new("div");
        fields.base_style_ref(&TABS).attr("data-fl-fields", true);
        cx.open(&fields, node)?;
        if cx.prop(node, "searchFields").map_or(defs.len() > 8, |v| irender_core::expr::truthy(&v)) {
            let mut find = Element::new("input");
            find.base_style_ref(forms::input_look())
                .attr("type", "search")
                .attr("data-fl-find", true)
                .attr("autocomplete", "off")
                .attr("placeholder", cx.label("Find a field…"))
                .attr("aria-label", cx.label("Find a field"));
            cx.open(&find, node)?;
            let mut none = Element::new("p");
            none.attr("data-fl-unfound", true).attr("hidden", true).text(lit(cx.label("No fields match")));
            cx.open(&none, node)?;
            cx.close("p");
        }
        let mut tabs = Element::new("div");
        tabs.attr("role", "tablist").attr("aria-orientation", "vertical").attr("aria-label", cx.label("Fields"));
        cx.open(&tabs, node)?;
        let mut section: Option<&str> = None;
        for (i, d) in defs.iter().enumerate() {
            if d.section.is_some() && d.section.as_deref() != section {
                section = d.section.as_deref();
                let mut s = Element::new("div");
                s.attr("data-fl-section", true).attr("role", "presentation").text(lit(d.section.clone().unwrap_or_default()));
                cx.open(&s, node)?;
                cx.close("div");
            }
            let mut tab = Element::new("button");
            tab.attr("type", "button")
                .attr("role", "tab")
                .attr("id", id_with(&id, &format!("-tab-{i}")))
                .attr("aria-controls", id_with(&id, &format!("-pane-{i}")))
                .attr("aria-selected", if i == 0 { "true" } else { "false" })
                .attr("tabindex", lit(if i == 0 { 0 } else { -1 }))
                .attr("data-fl-tab", lit(i));
            cx.open(&tab, node)?;
            let mut name = Element::new("span");
            name.text(lit(d.label.clone()));
            cx.open(&name, node)?;
            cx.close("span");
            let mut n = Element::new("span");
            n.attr("data-fl-n", true).attr("hidden", true);
            cx.open(&n, node)?;
            cx.close("span");
            cx.close("button");
        }
        cx.close("div");
        cx.close("div");

        for (i, d) in defs.iter().enumerate() {
            pane(cx, node, &id, i, d, cx.prop_string(node, "currency").as_deref(), crate::locale(cx, node).as_deref())?;
        }
        cx.close("div");

        let mut foot = Element::new("div");
        foot.base_style_ref(&FOOT);
        cx.open(&foot, node)?;
        let mut reset = Element::new("button");
        forms::button_look(&mut reset, "ghost", "sm");
        reset.attr("type", "button").attr("data-fl-reset", true).text(lit(cx.label("Clear all")));
        cx.open(&reset, node)?;
        cx.close("button");
        if live {
            let mut done = Element::new("button");
            forms::button_look(&mut done, "default", "sm");
            done.attr("type", "button").attr("command", "close").attr("commandfor", id.clone()).text(lit(cx.label("Done")));
            cx.open(&done, node)?;
            cx.close("button");
        } else {
            let mut cancel = Element::new("button");
            forms::button_look(&mut cancel, "outline", "sm");
            cancel.attr("type", "button").attr("command", "close").attr("commandfor", id.clone()).text(lit(cx.label("Cancel")));
            cx.open(&cancel, node)?;
            cx.close("button");
            let mut apply = Element::new("button");
            forms::button_look(&mut apply, "default", "sm");
            apply.attr("type", "button").attr("data-fl-apply", true).text(lit(cx.label("Apply")));
            cx.open(&apply, node)?;
            cx.close("button");
        }
        cx.close("div");

        let mut x = Element::new("button");
        x.base_style_ref(&DIALOG_X).attr("type", "button").attr("command", "close").attr("commandfor", id.clone()).attr("aria-label", cx.label("Close")).text(lit("✕"));
        cx.open(&x, node)?;
        cx.close("button");
        cx.close("dialog");
        let mut live = Element::new("div");
        live.attr("data-fl-live", true)
            .attr("role", "status")
            .attr("aria-live", "polite")
            .base_style(pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("overflow", "hidden"), ("clip-path", "inset(50%)"), ("white-space", "nowrap")]));
        cx.open(&live, node)?;
        cx.close("div");
        cx.close("div");
        Ok(())
    }
}

/// A field's controls: its operator, then its values (a list to pick from, or one or
/// two inputs). The behaviour fills the list and shows what the operator needs.
fn pane(cx: &mut RenderCx<'_>, node: &Node, id: &Val, i: usize, d: &Def, currency: Option<&str>, locale: Option<&str>) -> Result<(), RenderError> {
    let mut p = Element::new("div");
    p.base_style_ref(&PANE)
        .attr("role", "tabpanel")
        .attr("id", id_with(id, &format!("-pane-{i}")))
        .attr("aria-labelledby", id_with(id, &format!("-tab-{i}")))
        .attr("data-fl-pane", lit(i));
    if i > 0 {
        p.attr("hidden", true);
    }
    cx.open(&p, node)?;
    let mut h = Element::new("h3");
    h.text(lit(d.label.clone()));
    cx.open(&h, node)?;
    cx.close("h3");

    let mut op = Element::new("select");
    op.base_style_ref(forms::select_look()).attr("data-fl-op", true).attr("aria-label", cx.label_with("{name} condition", &[("name", &d.label)]));
    if d.ops.len() == 1 {
        op.attr("hidden", true);
    }
    cx.open(&op, node)?;
    for (value, label) in &d.ops {
        let mut o = Element::new("option");
        o.attr("value", *value).text(lit(cx.label(label)));
        cx.open(&o, node)?;
        cx.close("option");
    }
    cx.close("select");

    if d.kind == "list" {
        let mut search = Element::new("input");
        search
            .base_style_ref(forms::input_look())
            .attr("type", "search")
            .attr("data-fl-search", true)
            .attr("hidden", true)
            .attr("placeholder", cx.label_with("Search {name}", &[("name", &d.label.to_lowercase())]))
            .attr("aria-label", cx.label_with("Search {name}", &[("name", &d.label.to_lowercase())]));
        cx.open(&search, node)?;
        let mut list = Element::new("div");
        list.attr("data-fl-options", true).attr("role", "group").attr("aria-label", d.label.clone());
        cx.open(&list, node)?;
        cx.close("div");
        // What the browser copies for each value: a checkbox, the label and its count.
        let mut tpl = Element::new("template");
        tpl.attr("data-fl-choice", true);
        cx.open(&tpl, node)?;
        let mut row = Element::new("label");
        cx.open(row.attr("data-fl-option", true), node)?;
        let mut cb = Element::new("input");
        cb.base_style_ref(forms::checkbox_look()).attr("type", "checkbox").attr("data-fl-pick", true);
        cx.open(&cb, node)?;
        cx.open(&Element::new("span"), node)?;
        cx.close("span");
        cx.open(&Element::new("span"), node)?;
        cx.close("span");
        cx.close("label");
        cx.close("template");
    } else {
        // Numbers are text fields the behaviour groups as they're typed; dates keep their
        // values in date fields the calendar below fills.
        let numeric = matches!(d.kind.as_str(), "number" | "currency");
        let ty = match d.kind.as_str() {
            "date" => "date",
            _ => "text",
        };
        let mut values = Element::new("div");
        values.attr("data-fl-values", true);
        if d.kind == "date" {
            values.attr("data-fl-dates", true);
        }
        cx.open(&values, node)?;
        let symbol = (d.kind == "currency").then(|| crate::number::symbol(&d.currency.clone().or(currency.map(str::to_string)).unwrap_or_else(|| "USD".into()).to_uppercase()));
        // Labels: "{name} value", "{name} upper value"
        for (which, label) in [("a", "{name} value"), ("b", "{name} upper value")] {
            if which == "b" {
                let mut and = Element::new("span");
                and.attr("data-fl-and", true).text(lit(cx.label("and")));
                cx.open(&and, node)?;
                cx.close("span");
            }
            let mut field = Element::new("span");
            field.attr("data-fl-field", true).attr(&format!("data-fl-field-{which}"), true);
            cx.open(&field, node)?;
            if let Some(sym) = &symbol {
                let mut s = Element::new("span");
                s.attr("data-fl-prefix", true).attr("aria-hidden", "true").text(lit(sym.clone()));
                cx.open(&s, node)?;
                cx.close("span");
            }
            let mut input = Element::new("input");
            input.base_style_ref(forms::input_look()).attr("type", ty).attr(&format!("data-fl-{which}"), true).attr("aria-label", cx.label_with(label, &[("name", &d.label)]));
            if numeric {
                input.attr("inputmode", "decimal").attr("data-fl-num", true).attr("autocomplete", "off");
            }
            cx.open(&input, node)?;
            cx.close("span");
        }
        if let Some(u) = &d.unit {
            let mut unit = Element::new("span");
            unit.attr("data-fl-unit", true).text(lit(u.clone()));
            cx.open(&unit, node)?;
            cx.close("span");
        }
        cx.close("div");
        if d.kind == "date" {
            // The kit's calendar: a range for Between, one day for the others.
            let base = expr::stringify(&cx.eval(id));
            for (mode, ops) in [("range", "between"), ("single", "on,before,after")] {
                if !ops.split(',').any(|o| d.ops.iter().any(|(x, _)| *x == o)) {
                    continue;
                }
                let mut props = vec![
                    ("mode", lit(mode)),
                    ("id", lit(format!("{base}-cal-{i}-{mode}"))),
                    ("label", lit(if mode == "range" { cx.label_with("{name} range", &[("name", &d.label)]) } else { cx.label_with("{name} date", &[("name", &d.label)]) })),
                ];
                if let Some(l) = locale {
                    props.push(("locale", lit(l.to_string())));
                }
                let mut cal = crate::synthetic(node, "ui-calendar", props, vec![("data-fl-cal", lit(mode))], Vec::new());
                cal.style = StyleBlock::from_pairs(&[("align-self", "flex-start")]);
                cx.render(&cal)?;
            }
        }
        // Presets set a range, so only where the filter offers Between.
        if !d.presets.is_empty() && d.ops.iter().any(|(o, _)| *o == "between") {
            let mut box_ = Element::new("div");
            box_.attr("data-fl-presets", true).attr("role", "group").attr("aria-label", cx.label_with("{name} presets", &[("name", &d.label)]));
            cx.open(&box_, node)?;
            for (name, label) in &d.presets {
                // The four quarters are named by the script, which knows today.
                let names: Vec<(String, String)> =
                    if *name == "quarters" { (0..4).map(|n| (format!("quarter{n}"), String::new())).collect() } else { vec![(name.to_string(), cx.label(label))] };
                for (name, label) in names {
                    let mut b = Element::new("button");
                    b.attr("type", "button").attr("data-fl-preset", name).text(lit(label));
                    cx.open(&b, node)?;
                    cx.close("button");
                }
            }
            cx.close("div");
        }
    }
    let mut error = Element::new("p");
    error.attr("data-fl-error", true).attr("role", "alert").attr("hidden", true);
    cx.open(&error, node)?;
    cx.close("p");
    cx.close("div");
    Ok(())
}

/// The Views button and its menu: saved views (the browser lists them) and a name box
/// to save the filters as one.
fn views_menu(cx: &mut RenderCx<'_>, node: &Node, id: &Val) -> Result<(), RenderError> {
    let mut wrap = Element::new("div");
    wrap.attr("data-fl-views-wrap", true);
    cx.open(&wrap, node)?;
    let mut b = Element::new("button");
    forms::button_look(&mut b, "outline", "sm");
    b.attr("type", "button")
        .attr("data-fl-views-open", true)
        .attr("aria-expanded", "false")
        .attr("aria-controls", id_with(id, "-views"))
        .attr("aria-haspopup", "true");
    cx.open(&b, node)?;
    let mut t = Element::new("span");
    t.text(lit(cx.label("Views")));
    cx.open(&t, node)?;
    cx.close("span");
    cx.markup(r#"<svg width="12" height="12" style="width:calc(12px * var(--size-icon, 1));height:calc(12px * var(--size-icon, 1))" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 6l4 4 4-4"/></svg>"#);
    cx.close("button");
    let mut menu = Element::new("div");
    menu.attr("data-fl-views", true).attr("id", id_with(id, "-views")).attr("hidden", true);
    cx.open(&menu, node)?;
    let mut list = Element::new("ul");
    list.attr("data-fl-view-list", true).attr("aria-label", cx.label("Saved views"));
    cx.open(&list, node)?;
    cx.close("ul");
    let mut form = Element::new("div");
    form.attr("data-fl-view-form", true);
    cx.open(&form, node)?;
    let mut name = Element::new("input");
    name.base_style_ref(forms::input_look()).attr("type", "text").attr("data-fl-view-name", true).attr("placeholder", cx.label("Name these filters")).attr("aria-label", cx.label("View name"));
    cx.open(&name, node)?;
    let mut save = Element::new("button");
    forms::button_look(&mut save, "default", "sm");
    save.attr("type", "button").attr("data-fl-view-save", true).text(lit(cx.label("Save")));
    cx.open(&save, node)?;
    cx.close("button");
    cx.close("div");
    cx.close("div");
    cx.close("div");
    Ok(())
}

impl Component for Filter {
    fn name(&self) -> &'static str {
        "ui-filter"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "One field of the `ui-filters` it is in: its type, label and operators. Read by `ui-filters`, never rendered on its own.",
            props: FILTER_PROPS,
            parts: &[],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, _cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        Err(RenderError::new(node, "`ui-filter` must be a direct child of `ui-filters`"))
    }
}
