//! `ui-repeater`: rows of fields to add, fill in, reorder and remove, kept as a list of
//! objects in state. The fields are listed in `fields`; children are drawn after them with
//! the row in scope, and any native control of theirs marked `data-field` is wired too.
//! `repeater.js` moves values between the controls and state, works out computed fields,
//! totals and what's wrong, and adds, moves and removes rows.

use crate::{BORDER, DESTRUCTIVE, MUTED, SUBTLE, TEXT, each_item, expr_val, forms, items, lit};
use irender_core::{
    ComponentInfo, Node, Part, PropDef, PropKind, StyleBlock, Template, Val,
    expr::{self, Expr},
};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("repeater.js");
const ITEM: &str = "item";
const INDEX: &str = "index";
const TYPES: &[&str] = &["text", "multiline", "number", "currency", "date", "time", "email", "tel", "url", "select", "combobox", "checkbox", "computed"];

pub struct Repeater;

const PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key holding the rows, a list of objects").required(),
    PropDef::new(
        "fields",
        PropKind::Any,
        "The fields of a row: `[{ field, label, type, options, placeholder, width, required, min, max, pattern, unique, message, default, compute, format, currency, decimals, summary }]`. \
         `type`: text, multiline, number, currency, date, time, email, tel, url, select, combobox (typed, picked from `options` as you type), checkbox, or computed (`compute: qty * price`, over the row's fields and state)",
    )
    .required(),
    PropDef::new("layout", PropKind::Enum(&["table", "stack"]), "`table` (default): one line per row under a header (stacked on a narrow screen); `stack`: a card per row with labels"),
    PropDef::new("newItem", PropKind::Any, "What an added row starts with, after the fields' `default`s"),
    PropDef::new("min", PropKind::Number, "The fewest rows: they can't be removed below it, and it starts with that many"),
    PropDef::new("max", PropKind::Number, "The most rows"),
    PropDef::new("reorderable", PropKind::Bool, "Rows can be moved, by their grip or with Alt and the arrow keys on it"),
    PropDef::new("duplicable", PropKind::Bool, "A button that copies a row below itself"),
    PropDef::new("addLabel", PropKind::Any, "Default `Add row`"),
    PropDef::new("itemLabel", PropKind::String, "What a row is called to screen readers and on its card, e.g. `Contact` (default `Row`)"),
    PropDef::new("label", PropKind::Any, "Names the group for screen readers"),
    PropDef::new("emptyText", PropKind::Any, "Shown while there are no rows"),
    PropDef::new("disabled", PropKind::Any, "Boolean or expression: shown, not changed"),
    PropDef::new("bindErrors", PropKind::StateKey, "State key that gets what's wrong, `[{ index, field, message }]`"),
    PropDef::new("bindTotals", PropKind::StateKey, "State key that gets the fields' totals, `{ field: total }`, of those with `summary`"),
    PropDef::new("showErrors", PropKind::Any, "When to show messages: after a field was left (default), or true for all, e.g. `{{ tried }}`"),
    PropDef::new("locale", PropKind::String, "How numbers are written (default: the document's `lang`, else the browser's)"),
    PropDef::new("id", PropKind::Any, "Element id (default: derived from the document)"),
];

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

static ROOT: LazyLock<StyleBlock> = LazyLock::new(|| {
    let hidden = || pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("overflow", "hidden"), ("clip-path", "inset(50%)"), ("white-space", "nowrap")]);
    pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(8)), ("min-width", "0"), ("color", TEXT), ("font-size", fs!(14))])
        .nest("& [data-rep-rows]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(8)), ("min-width", "0")]))
        .nest("& [data-rep-line]", pairs(&[("display", "grid"), ("grid-template-columns", "var(--rep-cols)"), ("gap", gap!(8)), ("align-items", "start"), ("min-width", "0")]))
        .nest("& [data-rep-head]", pairs(&[("color", MUTED), ("font-size", fs!(13)), ("font-weight", "var(--font-weight-medium, 500)")]))
        .nest("& [data-rep-head] [data-end]", pairs(&[("text-align", "end")]))
        .nest("& [data-rep-cell]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(4)), ("min-width", "0")]))
        .nest("& [data-rep-cell] > label", pairs(&[("font-size", fs!(13)), ("font-weight", "var(--font-weight-medium, 500)")]))
        .nest("& [data-rep-cell] > label[data-required]::after", pairs(&[("content", "' *'"), ("color", DESTRUCTIVE)]))
        .nest("& [data-rep-cell] :is(input, select, textarea)", pairs(&[("width", "100%"), ("min-width", "0")]))
        .nest("& [data-rep-cell] input[type=checkbox]", pairs(&[("width", "16px"), ("margin-block", ctl!(10))]))
        .nest("& [data-rep-cell] :is([data-kind=number], [data-kind=currency])", pairs(&[("text-align", "end"), ("font-variant-numeric", "tabular-nums")]))
        .nest("& [data-rep-cell] [aria-invalid=true]", pairs(&[("border-color", DESTRUCTIVE)]))
        .nest("& output", pairs(&[("display", "flex"), ("align-items", "center"), ("justify-content", "flex-end"), ("min-height", ctl!(36)), ("font-variant-numeric", "tabular-nums"), ("overflow-wrap", "anywhere")]))
        .nest("& [data-rep-error]", pairs(&[("color", DESTRUCTIVE), ("font-size", fs!(12))]))
        .nest("& [data-rep-error]:empty", pairs(&[("display", "none")]))
        .nest("& [data-rep-tools]", pairs(&[("display", "flex"), ("align-items", "center"), ("justify-content", "flex-end"), ("gap", gap!(2)), ("min-height", ctl!(36))]))
        .nest(
            "& :is([data-rep-tools] button, [data-rep-grip])",
            pairs(&[
                ("display", "grid"),
                ("place-items", "center"),
                ("width", "28px"),
                ("height", "28px"),
                ("padding", "0"),
                ("border", "0"),
                ("border-radius", "var(--radius-sm)"),
                ("background", "none"),
                ("color", MUTED),
                ("cursor", "pointer"),
            ]),
        )
        .nest("& [data-rep-grip]", pairs(&[("cursor", "grab"), ("touch-action", "none"), ("width", "20px"), ("margin-top", ctl!(4))]))
        .nest("& :is([data-rep-tools] button, [data-rep-grip]):hover", pairs(&[("background", SUBTLE), ("color", TEXT)]))
        .nest("& :is([data-rep-tools] button, [data-rep-grip]):focus-visible", forms::focus_ring())
        .nest("& [data-rep-tools] button:disabled", pairs(&[("opacity", "0.35"), ("pointer-events", "none")]))
        .nest("& [data-rep-row][data-moving]", pairs(&[("opacity", "0.6")]))
        .nest("& [data-rep-empty]", pairs(&[("padding", pad!(16)), ("border", "var(--border-width, 1px) dashed var(--color-border)"), ("border-radius", "var(--radius-card, var(--radius-md))"), ("color", MUTED), ("text-align", "center")]))
        .nest("& [data-rep-empty][hidden]", pairs(&[("display", "none")]))
        .nest("& [data-rep-foot]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("align-items", "center"), ("justify-content", "space-between"), ("gap", gap!(8, 16))]))
        .nest("& [data-rep-totals]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("gap", gap!(4, 20)), ("margin-inline-start", "auto"), ("font-variant-numeric", "tabular-nums")]))
        .nest("& [data-rep-totals] span", pairs(&[("color", MUTED), ("margin-inline-end", gap!(6))]))
        .nest("& [data-rep-totals] b", pairs(&[("font-weight", "var(--font-weight-semibold, 600)")]))
        .nest("& [data-rep-live]", hidden())
        .nest("&[data-layout=table] [data-rep-row] [data-rep-cell] > label", hidden())
        .nest("& [data-rep-title]", pairs(&[("display", "none")]))
        .nest("&[data-layout=stack] [data-rep-head]", pairs(&[("display", "none")]))
        .nest(
            "&[data-layout=stack] [data-rep-row]",
            pairs(&[("padding", pad!(12)), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-card, var(--radius-md))")]),
        )
        .nest("&[data-layout=stack] [data-rep-line]", pairs(&[("grid-template-columns", "repeat(auto-fit, minmax(min(180px, 100%), 1fr))")]))
        .nest("&[data-layout=stack] [data-rep-tools], &[data-layout=stack] [data-rep-grip-cell]", pairs(&[("order", "-1")]))
        .nest("&[data-layout=stack] [data-rep-grip-cell]", pairs(&[("display", "none")]))
        .nest("&[data-layout=stack] [data-rep-tools]", pairs(&[("grid-column", "1 / -1"), ("min-height", "0")]))
        .nest("&[data-layout=stack] [data-rep-title]", pairs(&[("display", "block"), ("margin-inline-end", "auto"), ("font-weight", "var(--font-weight-semibold, 600)")]))
        .nest("&[data-layout=stack] output", pairs(&[("justify-content", "flex-start")]))
        // Narrow: every row a card, as `stack`.
        .nest(
            "@media (max-width: 640px)",
            pairs(&[])
                .nest("&[data-layout=table] [data-rep-head]", pairs(&[("display", "none")]))
                .nest(
                    "&[data-layout=table] [data-rep-row]",
                    pairs(&[("padding", pad!(12)), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-card, var(--radius-md))")]),
                )
                .nest("&[data-layout=table] [data-rep-line]", pairs(&[("grid-template-columns", "minmax(0, 1fr)")]))
                .nest("&[data-layout=table] [data-rep-row] [data-rep-cell] > label", pairs(&[("position", "static"), ("width", "auto"), ("height", "auto"), ("clip-path", "none")]))
                .nest("& [data-rep-tools]", pairs(&[("order", "-1")]))
                .nest("& [data-rep-grip-cell]", pairs(&[("display", "none")]))
                .nest("& [data-rep-title]", pairs(&[("display", "block"), ("margin-inline-end", "auto"), ("font-weight", "var(--font-weight-semibold, 600)")]))
                .nest("& output", pairs(&[("justify-content", "flex-start")])),
        )
});

fn svg(paths: &str) -> String {
    format!(r#"<svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">{paths}</svg>"#)
}

impl Component for Repeater {
    fn name(&self) -> &'static str {
        "ui-repeater"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Rows of fields to add, fill in, reorder and remove: line items, contacts, rules. State gets a list of objects. \
                  Children are drawn after the fields, with the row as `item` and its place as `index`.",
            props: PROPS,
            parts: &["rows"],
            children: true,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let key = cx.prop_string(node, "bind").ok_or_else(|| RenderError::new(node, "`ui-repeater` needs `bind`"))?;
        let fields = read_fields(cx, node)?;
        let flag = |cx: &RenderCx<'_>, n: &str| cx.prop(node, n).is_some_and(|v| expr::truthy(&v));
        let number = |cx: &RenderCx<'_>, n: &str| cx.prop(node, n).filter(|v| !v.is_null()).map(|v| expr::to_num(&v));
        let (min, max) = (number(cx, "min").unwrap_or(0.0).max(0.0), number(cx, "max"));
        if max.is_some_and(|m| m < min) {
            return Err(RenderError::new(node, "`min` is more than `max`"));
        }
        let stack = cx.prop_string(node, "layout").as_deref() == Some("stack");
        let reorder = flag(cx, "reorderable");
        let copies = flag(cx, "duplicable");
        let item_label = cx.prop_string(node, "itemLabel").unwrap_or_else(|| cx.label("Row"));
        let shown = node.prop("showErrors").map(|v| crate::data_table::dynamic(cx, node, "showErrors", v)).transpose()?;
        let disabled = node.prop("disabled").map(|v| crate::data_table::dynamic(cx, node, "disabled", v)).transpose()?;
        crate::cells(cx);
        cx.global_js("ui-repeater", SCRIPT);
        let config = json!({
            "bind": key, "fields": fields, "min": min, "max": max, "newItem": cx.prop(node, "newItem").filter(|v| v.is_object()),
            "errors": cx.prop_string(node, "bindErrors"), "totals": cx.prop_string(node, "bindTotals"),
            "showErrors": shown, "disabled": disabled, "locale": crate::locale(cx, node), "itemLabel": item_label,
        });
        let summed: Vec<&Json> = fields.iter().filter(|f| !f["summary"].is_null()).collect();

        let mut root = cx.element("div", node);
        root.base_style_ref(&ROOT).attr("role", "group").attr("data-layout", if stack { "stack" } else { "table" }).behavior("ui-repeater", config);
        if let Some(id) = node.prop("id") {
            root.attr("id", id.clone());
        }
        if let Some(l) = node.prop("label") {
            root.attr("aria-label", l.clone());
        }
        // A column per field, then the document's own and the row's buttons.
        let mut cols: Vec<String> = Vec::new();
        if reorder {
            cols.push("20px".into());
        }
        for f in &fields {
            let w = match &f["width"] {
                Json::Null => match f["type"].as_str() {
                    Some("checkbox") => "64px".to_string(),
                    Some("number" | "currency" | "date" | "time" | "computed") => "minmax(96px, 0.6fr)".into(),
                    _ => "minmax(0, 1fr)".into(),
                },
                Json::Number(n) => format!("{n}px"),
                v => expr::stringify(v),
            };
            if w.chars().any(|c| matches!(c, ';' | '{' | '}' | '<')) {
                return Err(RenderError::new(node, format!("`width` can't be `{w}`")));
            }
            cols.push(w);
        }
        if !node.children.is_empty() {
            cols.push("auto".into());
        }
        cols.push(format!("{}px", 32 + if copies { 30 } else { 0 }));
        root.base_style(StyleBlock::new().set("--rep-cols", cols.join(" ")));
        cx.open(&root, node)?;

        if !stack {
            let mut head = Element::new("div");
            head.attr("data-rep-line", true).attr("data-rep-head", true).attr("aria-hidden", "true");
            cx.open(&head, node)?;
            if reorder {
                cx.open(&Element::new("span"), node)?;
                cx.close("span");
            }
            for f in &fields {
                let mut s = Element::new("span");
                if matches!(f["type"].as_str(), Some("number" | "currency" | "computed")) {
                    s.attr("data-end", true);
                }
                let star = if f["required"] == json!(true) { " *" } else { "" };
                s.text(lit(format!("{}{star}", expr::stringify(&f["label"]))));
                cx.open(&s, node)?;
                cx.close("span");
            }
            cx.close("div");
        }

        let bound = Node { props: [("rows".to_string(), expr_val(Expr::Path(vec![key.clone()])))].into_iter().collect(), ..node.clone() };
        let rows = items(cx, &bound, "rows")?;
        let mut body = cx.part("div", node, "rows");
        body.attr("data-rep-rows", true).attr("role", "list");
        let grip = svg(r#"<path d="M6 3.5h.01M10 3.5h.01M6 8h.01M10 8h.01M6 12.5h.01M10 12.5h.01" stroke-width="2"/>"#);
        // Rows are their places: a moved row's fields take its values.
        let place = expr_val(Expr::Path(vec![INDEX.into()]));
        each_item(cx, &bound, body, rows, ITEM, INDEX, &place, |cx| {
            let mut row = Element::new("div");
            row.attr("data-rep-row", expr_val(Expr::Path(vec![INDEX.into()]))).attr("role", "listitem");
            cx.open(&row, node)?;
            let mut line = Element::new("div");
            line.attr("data-rep-line", true);
            cx.open(&line, node)?;
            if reorder {
                let mut cell = Element::new("div");
                cell.attr("data-rep-grip-cell", true);
                cx.open(&cell, node)?;
                let mut b = Element::new("button");
                b.attr("type", "button").attr("data-rep-grip", true).attr("aria-label", cx.label("Move (Alt and the arrow keys)"));
                cx.open(&b, node)?;
                cx.markup(&grip);
                cx.close("button");
                cx.close("div");
            }
            for f in &fields {
                field(cx, node, f)?;
            }
            if !node.children.is_empty() {
                let mut cell = Element::new("div");
                cell.attr("data-rep-cell", true);
                cx.open(&cell, node)?;
                cx.children(node)?;
                cx.close("div");
            }
            let mut tools = Element::new("div");
            tools.attr("data-rep-tools", true);
            cx.open(&tools, node)?;
            let mut title = Element::new("span");
            title.attr("data-rep-title", true).text(Val::Tpl(Template {
                parts: vec![Part::Lit(format!("{item_label} ")), Part::Expr(crate::bin(expr::BinOp::Add, Expr::Path(vec![INDEX.into()]), crate::num(1)))],
            }));
            cx.open(&title, node)?;
            cx.close("span");
            let tool = |cx: &mut RenderCx<'_>, attr: &str, label: String, paths: &str| -> Result<(), RenderError> {
                let mut b = Element::new("button");
                b.attr("type", "button").attr(attr, true).attr("aria-label", label.clone()).attr("title", label);
                cx.open(&b, node)?;
                cx.markup(&svg(paths));
                cx.close("button");
                Ok(())
            };
            if copies {
                tool(cx, "data-rep-copy", cx.label("Duplicate"), r#"<rect x="5.5" y="5.5" width="8" height="8" rx="1.5"/><path d="M10.5 3.2A1.5 1.5 0 0 0 9.2 2.5H4A1.5 1.5 0 0 0 2.5 4v5.2c0 .6.3 1 .7 1.3"/>"#)?;
            }
            tool(cx, "data-rep-remove", cx.label("Remove"), r#"<path d="M3 4.5h10M6.5 4.5V3h3v1.5M4.5 4.5l.6 8.5h5.8l.6-8.5M7 7v4M9 7v4"/>"#)?;
            cx.close("div");
            cx.close("div");
            cx.close("div");
            Ok(())
        })?;
        cx.close("div");

        if let Some(t) = node.prop("emptyText") {
            let mut e = Element::new("div");
            e.attr("data-rep-empty", true).attr("hidden", true).text(t.clone());
            cx.open(&e, node)?;
            cx.close("div");
        }
        let mut foot = Element::new("div");
        foot.attr("data-rep-foot", true);
        cx.open(&foot, node)?;
        let mut add = Element::new("button");
        forms::button_look(&mut add, "outline", "sm");
        add.attr("type", "button").attr("data-rep-add", true);
        cx.open(&add, node)?;
        cx.markup(&svg(r#"<path d="M8 3v10M3 8h10"/>"#));
        let mut l = Element::new("span");
        l.text(node.prop("addLabel").cloned().unwrap_or_else(|| lit(cx.label("Add row"))));
        cx.open(&l, node)?;
        cx.close("span");
        cx.close("button");
        if !summed.is_empty() {
            let mut t = Element::new("div");
            t.attr("data-rep-totals", true).attr("aria-live", "polite");
            cx.open(&t, node)?;
            for f in &summed {
                let mut d = Element::new("div");
                d.attr("data-total", expr::stringify(&f["field"]));
                cx.open(&d, node)?;
                let mut s = Element::new("span");
                s.text(f["label"].clone().into_val());
                cx.open(&s, node)?;
                cx.close("span");
                cx.open(&Element::new("b"), node)?;
                cx.close("b");
                cx.close("div");
            }
            cx.close("div");
        }
        cx.close("div");
        let mut live = Element::new("div");
        live.attr("data-rep-live", true).attr("role", "status");
        cx.open(&live, node)?;
        cx.close("div");
        cx.close("div");
        Ok(())
    }
}

trait IntoVal {
    fn into_val(self) -> Val;
}
impl IntoVal for Json {
    fn into_val(self) -> Val {
        Val::Lit(self)
    }
}

/// `fields` checked, with `compute` as the browser's expression.
fn read_fields(cx: &RenderCx<'_>, node: &Node) -> Result<Vec<Json>, RenderError> {
    let list = crate::server_array(cx, node, "fields")?;
    if list.is_empty() {
        return Err(RenderError::new(node, "`ui-repeater` needs `fields`"));
    }
    let mut out: Vec<Json> = Vec::new();
    for f in list {
        let Json::Object(mut f) = f else { return Err(RenderError::new(node, "each of `fields` must be an object: `{ field, label, type }`")) };
        let name = f.get("field").filter(|v| !v.is_null()).map(expr::stringify).unwrap_or_default();
        if name.is_empty() || name.contains('.') {
            return Err(RenderError::new(node, "each of `fields` needs a `field`, a plain name"));
        }
        if out.iter().any(|o| o["field"] == json!(name)) {
            return Err(RenderError::new(node, format!("two fields are called `{name}`")));
        }
        let compute = f.get("compute").filter(|v| !v.is_null()).map(expr::stringify);
        let ty = f.get("type").filter(|v| !v.is_null()).map(expr::stringify).unwrap_or_else(|| if compute.is_some() { "computed" } else { "text" }.into());
        if !TYPES.contains(&ty.as_str()) {
            return Err(RenderError::new(node, format!("field `type` must be one of {}, got `{ty}`", TYPES.join(", "))));
        }
        if (ty == "computed") != compute.is_some() {
            return Err(RenderError::new(node, format!("field `{name}`: `compute` goes with `type: computed`")));
        }
        if let Some(c) = compute {
            let e = Expr::parse(&c).map_err(|e| RenderError::new(node, format!("field `{name}`: can't read `compute`: {e}")))?;
            f.insert("compute".into(), Template { parts: vec![Part::Expr(e)] }.to_js());
        }
        if let Some(s) = f.get("summary").filter(|v| !v.is_null()).map(expr::stringify)
            && !["sum", "avg", "min", "max", "count"].contains(&s.as_str())
        {
            return Err(RenderError::new(node, format!("field `summary` must be sum, avg, min, max or count, got `{s}`")));
        }
        if let Some(t) = f.get("format").filter(|v| !v.is_null()).map(expr::stringify)
            && !["number", "currency", "percent"].contains(&t.as_str())
        {
            return Err(RenderError::new(node, format!("field `format` must be number, currency or percent, got `{t}`")));
        }
        if !matches!(f.get("label"), Some(v) if !v.is_null()) {
            f.insert("label".into(), json!(name));
        }
        f.insert("type".into(), json!(ty));
        out.push(Json::Object(f));
    }
    Ok(out)
}

/// One field of a row: its label, its control and its message. The label and message are
/// tied to the control by the script, which numbers the rows.
fn field(cx: &mut RenderCx<'_>, node: &Node, f: &Json) -> Result<(), RenderError> {
    let ty = f["type"].as_str().unwrap_or("text");
    let name = expr::stringify(&f["field"]);
    let mut cell = Element::new("div");
    cell.attr("data-rep-cell", true);
    cx.open(&cell, node)?;
    let mut label = Element::new("label");
    label.text(lit(expr::stringify(&f["label"])));
    if f["required"] == json!(true) {
        label.attr("data-required", true);
    }
    cx.open(&label, node)?;
    cx.close("label");
    let mut el = Element::new(match ty {
        "multiline" => "textarea",
        "select" => "select",
        "computed" => "output",
        _ => "input",
    });
    el.attr("data-field", name).attr("data-kind", ty);
    match ty {
        "multiline" => {
            el.base_style_ref(forms::input_look()).base_style(StyleBlock::from_pairs(&[("height", "auto"), ("min-height", "64px"), ("padding", "8px 12px")])).attr("rows", "2");
        }
        "select" => {
            el.base_style_ref(forms::select_look());
        }
        "computed" => {}
        "checkbox" => {
            el.attr("type", "checkbox").base_style_ref(forms::checkbox_look());
        }
        "number" | "currency" => {
            el.attr("type", "text").attr("inputmode", "decimal").base_style_ref(forms::input_look());
        }
        // The options as the browser's own suggestions, shared by the rows (repeater.js).
        "combobox" => {
            el.attr("type", "text").attr("autocomplete", "off").base_style_ref(forms::input_look());
        }
        _ => {
            el.attr("type", ty.to_string()).base_style_ref(forms::input_look());
        }
    }
    if ty != "computed" {
        if let Some(p) = f["placeholder"].as_str() {
            el.attr("placeholder", p.to_string());
        }
        if f["required"] == json!(true) {
            el.attr("aria-required", "true");
        }
    }
    cx.open(&el, node)?;
    if ty == "select" {
        let option = |cx: &mut RenderCx<'_>, value: &str, text: &str| -> Result<(), RenderError> {
            let mut o = Element::new("option");
            o.attr("value", value.to_string()).text(lit(text.to_string()));
            cx.open(&o, node)?;
            cx.close("option");
            Ok(())
        };
        option(cx, "", f["placeholder"].as_str().unwrap_or(""))?;
        for (i, o) in f["options"].as_array().into_iter().flatten().enumerate() {
            let text = crate::field(o, "label").or_else(|| crate::field(o, "value")).unwrap_or_else(|| expr::stringify(o));
            // By place, so a number stays a number.
            option(cx, &i.to_string(), &text)?;
        }
    }
    if !el.is_void() {
        let tag = el.tag().to_string();
        cx.close(&tag);
    }
    let mut e = Element::new("span");
    e.attr("data-rep-error", true);
    cx.open(&e, node)?;
    cx.close("span");
    cx.close("div");
    Ok(())
}
