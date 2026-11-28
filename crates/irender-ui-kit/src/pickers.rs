//! Choosing among things: `ui-query-builder` (rules nested with and / or),
//! `ui-transfer-list` (from one list to another) and `ui-tree-select` (a tree in a
//! popover). The first two are drawn by `pickers.js`; the last is a `ui-tree`.

use crate::{
    BG_POPOVER, BORDER, MUTED, PRIMARY, PRIMARY_FILL, SUBTLE, SURFACE, TEXT, forms, id_with, instance_id, lit, synthetic,
    widget::{self, flag, pairs},
};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("pickers.js");

fn control() -> StyleBlock {
    pairs(&[
        ("box-sizing", "border-box"),
        ("height", ctl!(32)),
        ("min-width", "0"),
        ("padding", pad!(0, 8)),
        ("border", "var(--border-width, 1px) solid var(--color-border)"),
        ("border-radius", "var(--radius-control, var(--radius-md))"),
        ("background", SURFACE),
        ("color", TEXT),
        ("font", "inherit"),
    ])
}

// ---------------------------------------------------------------- query builder

pub struct QueryBuilder;

const QUERY_TYPES: &[&str] = &["text", "number", "currency", "date", "select", "multiselect", "boolean"];
const QUERY_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key for the query: `{ op: and|or, not, rules: [{ field, operator, value, valueField } or another group] }`").required(),
    PropDef::new("fields", PropKind::Any, "What rules are about: `[{ field, label, type, options, operators, unit, column }]` (`column`: its name in `bindSql`); `type`: text, number, currency, date, select, multiselect or boolean").required(),
    PropDef::new("maxDepth", PropKind::Number, "How deep groups nest (default 3; 1 for none)"),
    PropDef::new("negate", PropKind::Bool, "Groups can be negated (`not`)"),
    PropDef::new("fieldCompare", PropKind::Bool, "A rule can compare a field with another field of its kind (`ends is after starts`), kept as its `valueField`"),
    PropDef::new("bindSql", PropKind::StateKey, "State key that gets the query as SQL, `{ where, params }`: the values in `params` behind placeholders, never in the text; rules not filled in left out"),
    PropDef::new("sql", PropKind::Enum(&["question", "numbered"]), "`bindSql`'s placeholders: `?` (default) or `$1`, `$2`"),
    PropDef::new("bindUrl", PropKind::StateKey, "State key that gets the query for a link: `status[is]=open&amount[gt]=100` while its rules are all joined by and, else `q=` and the query as JSON"),
    PropDef::new("bindText", PropKind::StateKey, "State key that gets the query in words"),
    PropDef::new("bindValid", PropKind::StateKey, "State key that gets true while every rule is filled in"),
    PropDef::new("rows", PropKind::Any, "Rows to try the query on; may use state"),
    PropDef::new("bindMatches", PropKind::StateKey, "With `rows`, state key that gets those that match"),
    PropDef::new("showText", PropKind::Bool, "The query in words under it"),
    PropDef::new("label", PropKind::Any, "Names it for screen readers (default `Conditions`)"),
    PropDef::new("disabled", PropKind::Any, "Shown, not changed"),
    PropDef::new("locale", PropKind::String, "How numbers and dates are written (default: the document's `lang`, else the browser's)"),
];

static QUERY: LazyLock<StyleBlock> = LazyLock::new(|| {
    forms::native_scheme(pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(8)), ("min-width", "0"), ("color", TEXT), ("font-size", fs!(14))]))
        .nest("& [data-q-group]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(8)), ("min-width", "0"), ("padding", pad!(10)), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-card, var(--radius-md))")]))
        .nest("& [data-q-group] [data-q-group]", pairs(&[("background", "color-mix(in srgb, var(--color-text) 3%, transparent)")]))
        .nest("& [data-q-bar]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("align-items", "center"), ("gap", gap!(6))]))
        .nest("& [data-q-list]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(6)), ("min-width", "0"), ("padding-inline-start", pad!(10)), ("border-inline-start", "2px solid var(--color-border)")]))
        .nest("& [data-q-rule]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("align-items", "center"), ("gap", gap!(6)), ("min-width", "0")]))
        .nest("& :is(select, input:not([type=checkbox]))", control())
        .nest("& :is(select, input):focus-visible", forms::focus_ring())
        .nest("& [data-q-rule] > select", pairs(&[("max-width", "220px")]))
        .nest("& [data-q-value]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("align-items", "center"), ("gap", gap!(6)), ("flex", "1 1 160px"), ("min-width", "0")]))
        .nest("& [data-q-value] > :is(input, select)", pairs(&[("flex", "1 1 90px"), ("max-width", "280px")]))
        .nest("& [data-q-value] small", pairs(&[("color", MUTED)]))
        .nest("& [data-q-empty]", pairs(&[("border-color", "var(--color-warning, #d97706) !important")]))
        .nest("& [data-q-chip]", pairs(&[("display", "inline-flex"), ("align-items", "center"), ("height", "26px"), ("padding", pad!(0, 10)), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "999px"), ("background", "none"), ("color", TEXT), ("font", "inherit"), ("font-size", fs!(13)), ("cursor", "pointer")]))
        .nest("& [data-q-chip][aria-pressed=true]", pairs(&[("border-color", "transparent"), ("background", PRIMARY_FILL), ("color", crate::ON_PRIMARY)]))
        .nest("& [data-q-seg]", pairs(&[("display", "inline-flex"), ("padding", "2px"), ("border-radius", "var(--radius-control, var(--radius-md))"), ("background", SUBTLE)]))
        .nest("& [data-q-seg] button", pairs(&[("height", "26px"), ("padding", pad!(0, 10)), ("border", "0"), ("border-radius", "var(--radius-sm)"), ("background", "none"), ("color", MUTED), ("font", "inherit"), ("font-size", fs!(13)), ("font-weight", "var(--font-weight-medium, 500)"), ("cursor", "pointer")]))
        .nest("& [data-q-seg] button[aria-pressed=true]", pairs(&[("background", SURFACE), ("color", TEXT), ("box-shadow", "0 1px 2px rgb(0 0 0 / 0.1)")]))
        .nest("& [data-q-add]", pairs(&[("display", "inline-flex"), ("align-items", "center"), ("gap", gap!(4)), ("height", "28px"), ("padding", pad!(0, 8)), ("border", "0"), ("border-radius", "var(--radius-sm)"), ("background", "none"), ("color", PRIMARY), ("font", "inherit"), ("font-size", fs!(13)), ("font-weight", "var(--font-weight-medium, 500)"), ("cursor", "pointer")]))
        .nest("& [data-q-add]:hover", pairs(&[("background", SUBTLE)]))
        .nest("& [data-q-tool]", widget::tool())
        .nest("& [data-q-tool]:hover", pairs(&[("background", SUBTLE), ("color", TEXT)]))
        .nest("& button:focus-visible", forms::focus_ring())
        .nest("& button:disabled", pairs(&[("opacity", "0.5"), ("cursor", "default")]))
        .nest("& [data-push]", pairs(&[("margin-inline-start", "auto")]))
        .nest("& [data-q-text]", pairs(&[("margin", "0"), ("color", MUTED), ("line-height", "1.5"), ("overflow-wrap", "anywhere")]))
        .nest("& [data-q-text] b", pairs(&[("color", TEXT), ("font-weight", "var(--font-weight-medium, 500)")]))
});

impl Component for QueryBuilder {
    fn name(&self) -> &'static str {
        "ui-query-builder"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Conditions put together with and / or, nested in groups: for segments, alerts, rules, access. State gets the query, its words, and the rows that match.", props: QUERY_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        if cx.prop_string(node, "bind").is_none() {
            return Err(RenderError::new(node, "`ui-query-builder` needs `bind`"));
        }
        let fields = crate::server_array(cx, node, "fields")?;
        for f in &fields {
            let name = f.get("field").map(expr::stringify).unwrap_or_default();
            let ty = f.get("type").filter(|t| !t.is_null()).map(expr::stringify).unwrap_or_else(|| "text".into());
            if name.is_empty() || !QUERY_TYPES.contains(&ty.as_str()) {
                return Err(RenderError::new(node, format!("each of `fields` needs a `field`, and a `type` that's one of {}", QUERY_TYPES.join(", "))));
            }
        }
        if fields.is_empty() {
            return Err(RenderError::new(node, "`ui-query-builder` needs `fields`"));
        }
        let mut config = widget::config(cx, node, &["rows", "disabled"], &["fields", "sql"], &["bind", "bindText", "bindValid", "bindMatches", "bindSql", "bindUrl"])?;
        config.insert("fieldCompare".into(), json!(flag(cx, node, "fieldCompare", false)));
        config.insert("maxDepth".into(), json!(widget::number(cx, node, "maxDepth").unwrap_or(3.0).clamp(1.0, 8.0)));
        config.insert("negate".into(), json!(flag(cx, node, "negate", false)));
        config.insert("showText".into(), json!(flag(cx, node, "showText", false)));
        config.insert("locale".into(), json!(crate::locale(cx, node)));
        widget::kit(cx);
        crate::cells(cx);
        cx.global_js("ui-pickers", SCRIPT);
        let mut el = cx.element("div", node);
        el.base_style_ref(&QUERY).attr("role", "group").attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Conditions")))).behavior("ui-query-builder", Json::Object(config));
        cx.open(&el, node)?;
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- transfer list

pub struct TransferList;

const TRANSFER_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key for the chosen values, a list in their order").required(),
    PropDef::new("options", PropKind::Any, "All of them: values or `{ value, label, description, group, disabled }`; may use state").required(),
    PropDef::new("titles", PropKind::Any, "The two lists' names (default `[Available, Chosen]`)"),
    PropDef::new("search", PropKind::Bool, "A search box over each list (default true)"),
    PropDef::new("reorderable", PropKind::Bool, "The chosen ones can be put in order, by dragging or with Alt and the arrow keys"),
    PropDef::new("max", PropKind::Number, "The most that can be chosen"),
    PropDef::new("height", PropKind::Any, "Each list's height (default 260)"),
    PropDef::new("disabled", PropKind::Any, "Shown, not changed"),
];

static TRANSFER: LazyLock<StyleBlock> = LazyLock::new(|| {
    forms::native_scheme(pairs(&[("display", "grid"), ("grid-template-columns", "minmax(0, 1fr) auto minmax(0, 1fr)"), ("gap", gap!(10)), ("align-items", "center"), ("color", TEXT), ("font-size", fs!(14))]))
        .nest("& [data-t-list]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("min-width", "0"), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-card, var(--radius-md))"), ("overflow", "hidden")]))
        .nest("& [data-t-head]", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", gap!(8)), ("padding", pad!(8, 12)), ("border-bottom", "var(--border-width, 1px) solid var(--color-border)"), ("font-weight", "var(--font-weight-medium, 500)")]))
        .nest("& [data-t-head] small", pairs(&[("margin-inline-start", "auto"), ("color", MUTED), ("font-weight", "400")]))
        .nest("& [data-t-search]", pairs(&[("padding", pad!(6)), ("border-bottom", "var(--border-width, 1px) solid var(--color-border)")]))
        .nest("& [data-t-search] input", control().set("width", "100%"))
        .nest("& [data-t-search] input:focus-visible", forms::focus_ring())
        .nest("& [role=listbox]", pairs(&[("height", "var(--t-h, 260px)"), ("overflow-y", "auto"), ("padding", pad!(4)), ("outline", "none")]))
        .nest("& [role=listbox]:focus-visible", pairs(&[("box-shadow", "inset 0 0 0 2px var(--color-ring, var(--color-muted))")]))
        .nest("& [role=option]", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", gap!(8)), ("padding", pad!(6, 8)), ("border-radius", "var(--radius-sm)"), ("cursor", "pointer"), ("user-select", "none")]))
        .nest("& [role=option]:hover", pairs(&[("background", SUBTLE)]))
        .nest("& [role=option][aria-selected=true]", pairs(&[("background", "color-mix(in srgb, var(--color-primary) 12%, transparent)")]))
        .nest("& [role=option][aria-disabled=true]", pairs(&[("opacity", "0.5"), ("cursor", "default")]))
        .nest("& [role=listbox]:focus-visible [data-on]", pairs(&[("box-shadow", "inset 0 0 0 1px var(--color-ring, var(--color-muted))")]))
        .nest("& [role=option] span", pairs(&[("min-width", "0"), ("overflow-wrap", "anywhere")]))
        .nest("& [role=option] small", pairs(&[("display", "block"), ("color", MUTED), ("font-size", fs!(12))]))
        .nest("& [role=option][data-drop]", pairs(&[("box-shadow", "inset 0 2px 0 var(--color-primary)")]))
        .nest("& [data-t-group]", pairs(&[("padding", pad!(6, 8, 2)), ("color", MUTED), ("font-size", fs!(12)), ("font-weight", "var(--font-weight-medium, 500)")]))
        .nest("& [data-t-none]", pairs(&[("padding", pad!(16, 8)), ("color", MUTED), ("text-align", "center")]))
        .nest("& [data-t-moves]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(6))]))
        .nest("& [data-t-moves] button", pairs(&[("display", "grid"), ("place-items", "center"), ("width", "34px"), ("height", "34px"), ("padding", "0"), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-control, var(--radius-md))"), ("background", SURFACE), ("color", TEXT), ("cursor", "pointer")]))
        .nest("& [data-t-moves] button:hover", pairs(&[("background", SUBTLE)]))
        .nest("& [data-t-moves] button:focus-visible", forms::focus_ring())
        .nest("& [data-t-moves] button:disabled", pairs(&[("opacity", "0.4"), ("cursor", "default")]))
        .nest("@media (max-width: 640px)", pairs(&[("grid-template-columns", "minmax(0, 1fr)")]).nest("& [data-t-moves]", pairs(&[("flex-direction", "row"), ("justify-content", "center")])).nest("& [data-t-moves] svg", pairs(&[("rotate", "90deg")])))
});

impl Component for TransferList {
    fn name(&self) -> &'static str {
        "ui-transfer-list"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Two lists: what there is and what's chosen, moved between with buttons, a double click, Enter or dragging.", props: TRANSFER_PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        if cx.prop_string(node, "bind").is_none() {
            return Err(RenderError::new(node, "`ui-transfer-list` needs `bind`"));
        }
        let mut config = widget::config(cx, node, &["options", "disabled", "titles"], &["max"], &["bind"])?;
        config.insert("search".into(), json!(flag(cx, node, "search", true)));
        config.insert("reorderable".into(), json!(flag(cx, node, "reorderable", false)));
        widget::kit(cx);
        cx.global_js("ui-pickers", SCRIPT);
        let mut s = StyleBlock::new();
        if let Some(h) = node.prop("height")
            && let Some(v) = cx.style_value(node, "height", h)?
        {
            s.push("--t-h", v);
        }
        let mut el = cx.element("div", node);
        el.base_style_ref(&TRANSFER).base_style(s).behavior("ui-transfer-list", Json::Object(config));
        cx.open(&el, node)?;
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- tree select

pub struct TreeSelect;

const TREE_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key for the chosen node's id, or with `multiple`, a list of ids").required(),
    PropDef::new("nodes", PropKind::Any, "As `ui-tree`'s: `[{ id, label, children }]`; may use state").required(),
    PropDef::new("multiple", PropKind::Bool, "Check several; checking a node checks those under it"),
    PropDef::new("leavesOnly", PropKind::Bool, "Only nodes without children can be chosen (single choice)"),
    PropDef::new("path", PropKind::Bool, "Shows the chosen node with those above it"),
    PropDef::new("search", PropKind::Bool, "A search box over the tree (default true)"),
    PropDef::new("expanded", PropKind::Any, "As `ui-tree`'s"),
    PropDef::new("clearable", PropKind::Bool, "A button clears the choice (default true)"),
    PropDef::new("placeholder", PropKind::Any, "Default `Select…`"),
    PropDef::new("label", PropKind::Any, "Accessible name, when no visible label names it"),
    PropDef::new("disabled", PropKind::Any, "Boolean or expression"),
    PropDef::new("invalid", PropKind::Any, "Mark it wrong"),
    PropDef::new("height", PropKind::Any, "Most the tree grows to (default 280)"),
];

static TREE: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("position", "relative"), ("display", "flex"), ("width", "100%"), ("min-width", "0"), ("font-size", fs!(14))])
        .nest(
            "& > [data-ts-button]",
            pairs(&[("display", "flex"), ("align-items", "center"), ("gap", gap!(4)), ("flex-wrap", "wrap"), ("box-sizing", "border-box"), ("width", "100%"), ("min-height", ctl!(36)), ("padding", pad!(3, 56, 3, 12)), ("border", "var(--border-width, 1px) solid var(--color-border)"), ("border-radius", "var(--radius-control, var(--radius-md))"), ("background", "transparent"), ("color", TEXT), ("font", "inherit"), ("text-align", "start"), ("cursor", "pointer"), ("box-shadow", "var(--elevation-control, 0 1px 2px rgb(0 0 0 / 0.05))")]),
        )
        .nest("&:dir(rtl) > [data-ts-button]", pairs(&[("padding", pad!(3, 12, 3, 56))]))
        .nest("& > [data-ts-button]:focus-visible", forms::focus_ring())
        .nest("& > [data-ts-button]:disabled", pairs(&[("opacity", "0.5"), ("cursor", "not-allowed")]))
        .nest("& > [data-ts-button][aria-invalid=true]", pairs(&[("border-color", "var(--color-destructive, #dc2626)")]))
        .nest("& [data-ts-none]", pairs(&[("color", MUTED)]))
        .nest("& [data-ts-above]", pairs(&[("color", MUTED)]))
        .nest("& [data-ts-tag]", pairs(&[("padding", pad!(1, 8)), ("border-radius", "var(--radius-sm)"), ("background", SUBTLE), ("font-size", fs!(13))]))
        .nest("& > [data-ts-ends]", pairs(&[("position", "absolute"), ("inset-inline-end", "6px"), ("top", "0"), ("height", ctl!(36)), ("display", "flex"), ("align-items", "center"), ("color", MUTED), ("pointer-events", "none")]))
        .nest("& [data-ts-clear]", widget::tool().set("pointer-events", "auto").set("min-width", "24px").set("height", "24px"))
        .nest("& [data-ts-clear][hidden]", pairs(&[("display", "none")]))
        .nest("& [data-ts-clear]:hover", pairs(&[("background", SUBTLE), ("color", TEXT)]))
        .nest(
            "& > [popover]",
            pairs(&[("box-sizing", "border-box"), ("inset", "auto"), ("margin", "6px 0"), ("width", "max(anchor-size(width), 240px)"), ("max-width", "calc(100vw - 16px)"), ("padding", pad!(8)), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-card, var(--radius-md))"), ("background", BG_POPOVER), ("color", TEXT), ("box-shadow", "var(--elevation-popover, 0 6px 18px rgb(0 0 0 / 0.12))"), ("position-area", "bottom span-right"), ("position-try-fallbacks", "flip-block, flip-inline")]).nest("&:dir(rtl)", pairs(&[("position-area", "bottom span-left")])),
        )
});

impl Component for TreeSelect {
    fn name(&self) -> &'static str {
        "ui-tree-select"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "A choice from a hierarchy: a field that opens a searchable tree, for one node or several.", props: TREE_PROPS, parts: &["trigger"], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let key = cx.prop_string(node, "bind").ok_or_else(|| RenderError::new(node, "`ui-tree-select` needs `bind`"))?;
        let multiple = flag(cx, node, "multiple", false);
        let mut config = widget::config(cx, node, &["nodes", "placeholder"], &[], &["bind"])?;
        for f in ["leavesOnly", "path"] {
            config.insert(f.into(), json!(flag(cx, node, f, false)));
        }
        config.insert("multiple".into(), json!(multiple));
        widget::kit(cx);
        crate::anchors(cx);
        cx.global_js("ui-pickers", SCRIPT);
        let id = instance_id(cx, node, "tree-select");
        let pop = id_with(&id, "-tree");
        let anchor = format!("--{}", expr::stringify(&cx.eval(&id)));
        let mut el = cx.element("div", node);
        el.base_style_ref(&TREE).behavior("ui-tree-select", Json::Object(config));
        cx.open(&el, node)?;
        let mut b = cx.part("button", node, "trigger");
        b.attr("type", "button")
            .attr("id", id.clone())
            .attr("data-ts-button", true)
            .attr("popovertarget", pop.clone())
            .attr("aria-haspopup", "tree")
            .base_style(StyleBlock::new().set("anchor-name", anchor.clone()));
        for (prop, attr) in [("label", "aria-label"), ("disabled", "disabled"), ("invalid", "aria-invalid")] {
            if let Some(v) = node.prop(prop) {
                b.attr(attr, v.clone());
            }
        }
        cx.open(&b, node)?;
        cx.close("button");
        let mut ends = Element::new("span");
        ends.attr("data-ts-ends", true);
        cx.open(&ends, node)?;
        if flag(cx, node, "clearable", true) {
            let mut c = Element::new("button");
            c.attr("type", "button").attr("data-ts-clear", true).attr("hidden", true).attr("aria-label", cx.label("Clear"));
            cx.open(&c, node)?;
            cx.close("button");
        }
        cx.markup(r#"<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 6l4 4 4-4"/></svg>"#);
        cx.close("span");
        let mut p = Element::new("div");
        p.attr("popover", "auto").attr("id", pop).base_style(StyleBlock::new().set("position-anchor", anchor));
        cx.open(&p, node)?;
        let mut props = vec![
            ("nodes", node.prop("nodes").cloned().unwrap_or_else(|| lit(json!([])))),
            ("search", lit(flag(cx, node, "search", true))),
            ("label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Choices")))),
            ("height", node.prop("height").cloned().unwrap_or_else(|| lit(280))),
        ];
        if multiple {
            props.push(("checkable", lit(true)));
            props.push(("bindChecked", lit(key)));
        } else if !flag(cx, node, "leavesOnly", false) {
            props.push(("bind", lit(key)));
        }
        if let Some(e) = node.prop("expanded") {
            props.push(("expanded", e.clone()));
        }
        cx.render(&synthetic(node, "ui-tree", props, Vec::new(), Vec::new()))?;
        cx.close("div");
        cx.close("div");
        Ok(())
    }
}
