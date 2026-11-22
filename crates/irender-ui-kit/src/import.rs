//! `ui-import`: rows from a spreadsheet, in four steps. A file is dropped (or cells
//! pasted), its columns are matched to the fields, the rows are put right in an editable
//! `ui-data-table`, and those that pass are handed to the page or posted to a URL in
//! batches. `import.js` reads, matches and sends; the table checks.

use crate::{BORDER, DESTRUCTIVE, MUTED, ON_PRIMARY, PRIMARY, PRIMARY_FILL, SUBTLE, TEXT, forms, instance_id, lit, synthetic};
use irender_core::{
    ComponentInfo, Node, PropDef, PropKind, StyleBlock,
    expr::{self, Expr},
};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("import.js");
const TYPES: &[&str] = &["text", "multiline", "number", "date", "select", "checkbox"];

pub struct Import;

const PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key holding the rows while they're reviewed").required(),
    PropDef::new("bindImported", PropKind::StateKey, "State key that gets the rows that passed, once imported (or sent)"),
    PropDef::new(
        "fields",
        PropKind::Any,
        "What each row has: `[{ field, label, type, options, required, min, max, pattern, unique, message, aliases, example, format }]`; `type`: text, multiline, number, date, select or checkbox; `aliases`: other names its column may have",
    )
    .required(),
    PropDef::new("url", PropKind::Any, "Post the rows here in batches, `{ rows }` as JSON; answering `{ errors: [{ row, field, message }] }` (`row` counts from 0 in the batch) sends those back to be put right"),
    PropDef::new("params", PropKind::Any, "With `url`, more to send with each batch; may use state"),
    PropDef::new("batchSize", PropKind::Number, "With `url`, rows per request (default 200)"),
    PropDef::new("accept", PropKind::String, "File types (default `.csv,.tsv,.txt,.xlsx`)"),
    PropDef::new("maxRows", PropKind::Number, "The most rows a file may have (default 10000)"),
    PropDef::new("skipInvalid", PropKind::Bool, "Rows that aren't right can be left out (default true; false: all must be right)"),
    PropDef::new("template", PropKind::Any, "Offers an empty file to fill in: `true` (default), its name ending in `.csv` or `.xlsx`, or false"),
    PropDef::new("title", PropKind::Any, "Names what's imported, e.g. `Contacts` (default `Rows`)"),
    PropDef::new("locale", PropKind::String, "How numbers and dates are read (default: the document's `lang`, else the browser's)"),
    PropDef::new("id", PropKind::Any, "Element id (default: derived from the document)"),
];

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

static ROOT: LazyLock<StyleBlock> = LazyLock::new(|| {
    forms::native_scheme(pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(16)), ("min-width", "0"), ("color", TEXT), ("font-size", fs!(14))]))
        .nest("& [hidden]", pairs(&[("display", "none !important")]))
        .nest("& > ol", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("gap", gap!(8, 20)), ("margin", "0"), ("padding", "0"), ("list-style", "none"), ("counter-reset", "imp")]))
        .nest("& > ol > li", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", gap!(8)), ("color", MUTED), ("font-weight", "var(--font-weight-medium, 500)"), ("counter-increment", "imp")]))
        .nest(
            "& > ol > li::before",
            pairs(&[
                ("content", "counter(imp)"),
                ("display", "grid"),
                ("place-items", "center"),
                ("box-sizing", "border-box"),
                ("width", "24px"),
                ("height", "24px"),
                ("border-radius", "50%"),
                ("border", "1.5px solid var(--color-border)"),
                ("font-size", fs!(12)),
            ]),
        )
        .nest("& > ol > li[data-done]::before", pairs(&[("content", "'✓'"), ("background", PRIMARY_FILL), ("color", ON_PRIMARY), ("border-color", "transparent")]))
        .nest("& > ol > li[aria-current]", pairs(&[("color", TEXT)]))
        .nest("& > ol > li[aria-current]::before", pairs(&[("border-color", PRIMARY), ("color", PRIMARY)]))
        .nest("& [data-imp-step]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(12)), ("min-width", "0")]))
        .nest("& [data-imp-step]:focus", pairs(&[("outline", "none")]))
        .nest(
            "& [data-imp-zone]",
            pairs(&[
                ("position", "relative"),
                ("display", "flex"),
                ("flex-direction", "column"),
                ("align-items", "center"),
                ("gap", gap!(6)),
                ("padding", pad!(40, 20)),
                ("border", "1.5px dashed var(--color-border)"),
                ("border-radius", "var(--radius-card, var(--radius-lg))"),
                ("text-align", "center"),
                ("cursor", "pointer"),
                ("transition", "background-color .15s, border-color .15s"),
            ]),
        )
        .nest("& [data-imp-zone]:is(:hover, [data-over])", pairs(&[("background", SUBTLE), ("border-color", PRIMARY)]))
        .nest("& [data-imp-zone]:focus-within", forms::focus_ring())
        .nest("& [data-imp-zone] input", pairs(&[("position", "absolute"), ("inset", "0"), ("opacity", "0"), ("cursor", "pointer")]))
        .nest("& [data-imp-zone] strong", pairs(&[("font-weight", "var(--font-weight-medium, 500)")]))
        .nest("& [data-imp-muted]", pairs(&[("color", MUTED), ("font-size", fs!(13))]))
        .nest("& [data-imp-error]", pairs(&[("color", DESTRUCTIVE)]))
        .nest("& [data-imp-bar]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("align-items", "center"), ("gap", gap!(8))]))
        .nest("& [data-imp-bar] > [data-push]", pairs(&[("margin-inline-start", "auto")]))
        .nest("& [data-imp-bar] label", pairs(&[("display", "inline-flex"), ("align-items", "center"), ("gap", gap!(6))]))
        .nest("& [data-imp-map]", pairs(&[("display", "grid"), ("grid-template-columns", "minmax(140px, 1fr) minmax(160px, 1fr) minmax(0, 1.4fr)"), ("border", "var(--border-width, 1px) solid"), ("border-color", BORDER), ("border-radius", "var(--radius-card, var(--radius-md))"), ("overflow", "hidden")]))
        .nest("& [data-imp-map] > *", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", gap!(6)), ("min-width", "0"), ("padding", pad!(8, 12)), ("border-bottom", "var(--border-width, 1px) solid var(--color-border)")]))
        .nest("& [data-imp-map] > :nth-last-child(-n+3)", pairs(&[("border-bottom", "0")]))
        .nest("& [data-imp-map] > [data-head]", pairs(&[("color", MUTED), ("font-size", fs!(13)), ("font-weight", "var(--font-weight-medium, 500)"), ("background", SUBTLE)]))
        .nest("& [data-imp-map] b", pairs(&[("font-weight", "var(--font-weight-medium, 500)")]))
        .nest("& [data-imp-map] [data-req]", pairs(&[("color", DESTRUCTIVE)]))
        .nest("& [data-imp-map] [data-sample]", pairs(&[("display", "block"), ("color", MUTED), ("white-space", "nowrap"), ("overflow", "hidden"), ("text-overflow", "ellipsis"), ("align-content", "center")]))
        .nest(
            "& [data-imp-map] select",
            pairs(&[("width", "100%"), ("min-width", "0"), ("height", ctl!(34)), ("padding", pad!(0, 8)), ("border", "var(--border-width, 1px) solid var(--color-border)"), ("border-radius", "var(--radius-control, var(--radius-md))"), ("background", crate::SURFACE), ("color", TEXT), ("font", "inherit")]),
        )
        .nest("& [data-imp-map] select:focus-visible", forms::focus_ring())
        .nest("& progress", pairs(&[("width", "100%"), ("accent-color", PRIMARY)]))
        .nest("& [data-imp-done]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("align-items", "center"), ("gap", gap!(8)), ("padding", pad!(32, 16)), ("text-align", "center")]))
        .nest("& [data-imp-done] strong", pairs(&[("font-size", fs!(18)), ("font-weight", "var(--font-weight-semibold, 600)")]))
        .nest("& [data-imp-done] svg", pairs(&[("color", "var(--color-success, #16a34a)")]))
        .nest("@media (max-width: 560px)", pairs(&[]).nest("& [data-imp-map]", pairs(&[("grid-template-columns", "minmax(0, 1fr) minmax(0, 1fr)")])).nest("& [data-imp-map] > :nth-child(3n)", pairs(&[("display", "none")])))
});

const UPLOAD: &str = r#"<svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="4" y="3.5" width="16" height="17" rx="2"/><path d="M4 9h16M4 14.5h16M10 9v11.5"/></svg>"#;
const DONE: &str = r#"<svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="9"/><path d="m8.5 12.5 2.5 2.5 4.5-5"/></svg>"#;

impl Component for Import {
    fn name(&self) -> &'static str {
        "ui-import"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Rows from a spreadsheet (CSV, Excel, or pasted cells): its columns are matched to `fields`, the rows put right in an editable table, \
                  and the good ones handed over (an `import` event and `bindImported`) or posted to `url`.",
            props: PROPS,
            parts: &[],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let key = cx.prop_string(node, "bind").ok_or_else(|| RenderError::new(node, "`ui-import` needs `bind`"))?;
        let mut fields = Vec::new();
        for f in crate::server_array(cx, node, "fields")? {
            let Json::Object(mut f) = f else { return Err(RenderError::new(node, "each of `fields` must be an object")) };
            let name = f.get("field").map(expr::stringify).unwrap_or_default();
            if name.is_empty() || name.contains('.') || name == "_row" {
                return Err(RenderError::new(node, "each of `fields` needs a `field`, a plain name (not `_row`)"));
            }
            let ty = f.get("type").filter(|v| !v.is_null()).map(expr::stringify).unwrap_or_else(|| "text".into());
            if !TYPES.contains(&ty.as_str()) {
                return Err(RenderError::new(node, format!("field `type` must be one of {}, got `{ty}`", TYPES.join(", "))));
            }
            f.insert("type".into(), json!(ty));
            if f.get("label").is_none_or(|l| l.is_null()) {
                f.insert("label".into(), json!(name));
            }
            fields.push(Json::Object(f));
        }
        if fields.is_empty() {
            return Err(RenderError::new(node, "`ui-import` needs `fields`"));
        }
        if let Some(u) = node.prop("url").filter(|u| !cx.is_reactive(u)) {
            let u = expr::stringify(&cx.eval(u));
            if !crate::combobox::safe_search_url(&u) {
                return Err(RenderError::new(node, format!("`url` must be a path or an http(s) URL, got `{u}`")));
            }
        }
        let title = cx.prop_string(node, "title").unwrap_or_else(|| cx.label("Rows"));
        let template = match cx.prop(node, "template") {
            Some(Json::Bool(false)) => None,
            Some(Json::String(s)) if s.ends_with(".csv") || s.ends_with(".xlsx") => Some(s),
            Some(Json::String(s)) => return Err(RenderError::new(node, format!("`template` must end in .csv or .xlsx, got `{s}`"))),
            _ => Some("template.csv".into()),
        };
        let skip = cx.prop(node, "skipInvalid").is_none_or(|v| expr::truthy(&v));
        let accept = cx.prop_string(node, "accept").unwrap_or_else(|| ".csv,.tsv,.txt,.xlsx".into());
        let num = |cx: &RenderCx<'_>, n: &str, d: f64| cx.prop(node, n).filter(|v| !v.is_null()).map_or(d, |v| expr::to_num(&v)).max(1.0);
        let max_rows = num(cx, "maxRows", 10_000.0);
        crate::cells(cx);
        crate::sheets(cx);
        cx.global_js("ui-import", SCRIPT);
        let config = json!({
            "bind": key, "imported": cx.prop_string(node, "bindImported"), "fields": fields, "skip": skip, "template": template,
            "url": node.prop("url").map(|u| crate::data_table::dynamic(cx, node, "url", u)).transpose()?,
            "params": node.prop("params").map(|u| crate::data_table::dynamic(cx, node, "params", u)).transpose()?,
            "batch": num(cx, "batchSize", 200.0), "maxRows": max_rows, "locale": crate::locale(cx, node),
        });
        let mut root = cx.element("div", node);
        root.base_style_ref(&ROOT).attr("role", "group").attr("aria-label", cx.label_with("Import {name}", &[("name", &title)])).behavior("ui-import", config);
        if node.prop("id").is_some() {
            root.attr("id", instance_id(cx, node, "import"));
        }
        cx.open(&root, node)?;
        cx.open(&Element::new("ol"), node)?;
        // Labels: "Upload", "Match columns", "Review", "Import"
        for (i, s) in ["Upload", "Match columns", "Review", "Import"].iter().enumerate() {
            let mut li = Element::new("li");
            if i == 0 {
                li.attr("aria-current", "step");
            }
            li.text(lit(cx.label(s)));
            cx.open(&li, node)?;
            cx.close("li");
        }
        cx.close("ol");

        let step = |cx: &mut RenderCx<'_>, name: &str, first: bool| -> Result<(), RenderError> {
            let mut d = Element::new("div");
            d.attr("data-imp-step", name).attr("tabindex", "-1").attr("hidden", !first);
            cx.open(&d, node)
        };
        let text = |cx: &mut RenderCx<'_>, tag: &str, attr: &str, t: String| -> Result<(), RenderError> {
            let mut e = Element::new(tag);
            e.attr(attr, true).text(lit(t));
            cx.open(&e, node)?;
            cx.close(tag);
            Ok(())
        };
        let button = |cx: &mut RenderCx<'_>, label: String, variant: &str, attr: &str| -> Result<(), RenderError> {
            let b = synthetic(node, "ui-button", vec![("label", lit(label)), ("variant", lit(variant))], vec![(attr, lit(true))], Vec::new());
            cx.render(&b)
        };
        let bar = |cx: &mut RenderCx<'_>| -> Result<(), RenderError> {
            let mut d = Element::new("div");
            d.attr("data-imp-bar", true);
            cx.open(&d, node)
        };

        // 1. A file, or pasted cells.
        step(cx, "upload", true)?;
        let mut zone = Element::new("label");
        zone.attr("data-imp-zone", true);
        cx.open(&zone, node)?;
        cx.markup(UPLOAD);
        text(cx, "strong", "data-imp-zone-title", cx.label("Drop a file here, or click to choose one"))?;
        let kinds = accept.split(',').map(|k| k.trim().trim_start_matches('.').to_uppercase()).collect::<Vec<_>>().join(", ");
        let limit = irender_core::labels::number(cx.labels().lang.as_deref(), max_rows);
        text(cx, "span", "data-imp-muted", cx.label_with("{types}, up to {max} rows. Or paste cells copied from a spreadsheet.", &[("types", &kinds), ("max", &limit)]))?;
        let mut input = Element::new("input");
        input.attr("type", "file").attr("accept", accept).attr("aria-label", cx.label("Choose a file"));
        cx.open(&input, node)?;
        cx.close("label");
        bar(cx)?;
        if template.is_some() {
            button(cx, cx.label("Download a template"), "link", "data-imp-template")?;
        }
        cx.close("div");
        let mut err = Element::new("p");
        err.attr("data-imp-error", true).attr("role", "alert").attr("hidden", true).base_style(pairs(&[("margin", "0")]));
        cx.open(&err, node)?;
        cx.close("p");
        cx.close("div");

        // 2. Its columns, matched.
        step(cx, "match", false)?;
        bar(cx)?;
        text(cx, "span", "data-imp-file", String::new())?;
        for (attr, label) in [("data-imp-sheets", cx.label("Sheet")), ("data-imp-header", cx.label("The first row has the column names"))] {
            let mut l = Element::new("label");
            l.attr(attr, true);
            if attr == "data-imp-sheets" {
                l.attr("hidden", true);
            }
            cx.open(&l, node)?;
            if attr == "data-imp-sheets" {
                cx.text(&label);
                let mut s = Element::new("select");
                s.base_style_ref(forms::select_look()).base_style(pairs(&[("width", "auto")]));
                cx.open(&s, node)?;
                cx.close("select");
            } else {
                let mut c = Element::new("input");
                c.attr("type", "checkbox").base_style_ref(forms::checkbox_look());
                cx.open(&c, node)?;
                cx.text(&label);
            }
            cx.close("label");
        }
        cx.close("div");
        let mut map = Element::new("div");
        map.attr("data-imp-map", true);
        cx.open(&map, node)?;
        cx.close("div");
        let mut p = Element::new("p");
        p.attr("data-imp-missing", true).attr("data-imp-error", true).attr("role", "status").base_style(pairs(&[("margin", "0")]));
        cx.open(&p, node)?;
        cx.close("p");
        bar(cx)?;
        button(cx, cx.label("Back"), "outline", "data-imp-back")?;
        cx.open(Element::new("span").attr("data-push", true), node)?;
        cx.close("span");
        button(cx, cx.label("Review rows"), "default", "data-imp-review")?;
        cx.close("div");
        cx.close("div");

        // 3. The rows, put right.
        step(cx, "review", false)?;
        text(cx, "div", "data-imp-counts", String::new())?;
        let mut columns = vec![json!({ "field": "_row", "label": cx.label("Row"), "hideable": false, "pin": "left", "width": 64, "align": "right" })];
        for f in &fields {
            let mut c = f.as_object().cloned().unwrap_or_default();
            let ty = c.remove("type").unwrap_or(json!("text"));
            for k in ["aliases", "example", "default"] {
                c.remove(k);
            }
            c.insert("edit".into(), ty);
            columns.push(Json::Object(c));
        }
        let mut table = synthetic(
            node,
            "ui-data-table",
            vec![
                ("rows", crate::expr_val(Expr::Path(vec![key.clone()]))),
                ("rowKey", lit("_row")),
                ("columns", lit(Json::Array(columns))),
                ("deletable", lit(true)),
                ("search", lit(true)),
                ("pageSize", lit(10)),
                ("height", lit(480)),
                ("title", lit(title.clone())),
            ],
            Vec::new(),
            Vec::new(),
        );
        if let Some(l) = node.prop("locale") {
            table.props.insert("locale".into(), l.clone());
        }
        cx.render(&table)?;
        bar(cx)?;
        button(cx, cx.label("Back"), "outline", "data-imp-back")?;
        cx.open(Element::new("span").attr("data-push", true), node)?;
        cx.close("span");
        button(cx, cx.label("Import"), "default", "data-imp-go")?;
        cx.close("div");
        cx.close("div");

        // 4. Sent.
        step(cx, "done", false)?;
        let mut d = Element::new("div");
        d.attr("data-imp-sending", true).attr("role", "status");
        cx.open(&d, node)?;
        text(cx, "p", "data-imp-progress", String::new())?;
        let mut pr = Element::new("progress");
        pr.attr("max", "100").attr("value", "0").attr("aria-label", cx.label("Importing"));
        cx.open(&pr, node)?;
        cx.close("progress");
        cx.close("div");
        let mut d = Element::new("div");
        d.attr("data-imp-done", true).attr("hidden", true);
        cx.open(&d, node)?;
        cx.markup(DONE);
        text(cx, "strong", "data-imp-result", String::new())?;
        text(cx, "span", "data-imp-muted", String::new())?;
        button(cx, cx.label("Import another file"), "outline", "data-imp-again")?;
        cx.close("div");
        cx.close("div");
        cx.close("div");
        Ok(())
    }
}
