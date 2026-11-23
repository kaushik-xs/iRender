//! `ui-form-builder`: a form put together in the browser. A palette of field types is
//! added (clicked or dragged) to a list of fields that's put in order by dragging or the
//! keyboard, each field's label, key, help, choices and rules set in an inspector; a
//! preview tries the form out, and the form comes out as an iRender document (JSON or
//! YAML) of the kit's own form: `ui-form` with its rules, `ui-field`s, sections and a
//! submit button.
//!
//! The server renders the frame (the tabs and their panels, a live region);
//! `form_builder.js` draws the builder from the fields in state and writes them back,
//! with the document, as they change.

use crate::{MUTED, PRIMARY, RING, SUBTLE, TEXT, forms, instance_id, lit};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("form_builder.js");

/// The kinds of field the palette offers, in its order.
pub(crate) const TYPES: &[&str] = &["text", "textarea", "email", "number", "date", "select", "radio", "checkboxes", "checkbox", "section"];

const PROPS: &[PropDef] = &[
    PropDef::new(
        "bind",
        PropKind::StateKey,
        "State key holding the fields, in order: `{ id, type, label, description, placeholder, required, options, min, max }`",
    )
    .required(),
    PropDef::new("bindDocument", PropKind::StateKey, "State key that gets the form as an iRender document, kept up to date"),
    PropDef::new("types", PropKind::Any, "The field types offered, from `text`, `textarea`, `email`, `number`, `date`, `select`, `radio`, `checkboxes`, `checkbox`, `section` (default all)"),
    PropDef::new("submitLabel", PropKind::Any, "The generated form's submit button (default `Submit`)"),
    PropDef::new("endpoint", PropKind::String, "Where the generated form posts its answers (as JSON); without it, the form thanks the person instead"),
    PropDef::new("label", PropKind::Any, "Names the builder for screen readers (default `Form builder`)"),
    PropDef::new("id", PropKind::Any, "Element id (default: derived from the document)"),
];

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

const BORDER: &str = "var(--border-width, 1px) solid var(--color-border)";
const TINT: &str = "color-mix(in srgb, var(--color-primary) 10%, var(--ui-bg, var(--color-background, var(--color-surface))))";

static BUILDER: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("display", "flex"),
        ("flex-direction", "column"),
        ("gap", gap!(12)),
        ("container-type", "inline-size"),
        ("color", TEXT),
        ("font-size", fs!(14)),
        ("--ui-bg", "var(--color-card, var(--color-surface))"),
    ])
    // ---- the tabs
    .nest("& [data-fb-tabs]", pairs(&[("display", "inline-flex"), ("gap", gap!(2)), ("width", "fit-content"), ("padding", pad!(3)), ("border-radius", "var(--radius-control, var(--radius-md))"), ("background", SUBTLE)]))
    .nest(
        "& [data-fb-tabs] button",
        pairs(&[("height", ctl!(30)), ("padding", pad!(0, 12)), ("border", "0"), ("border-radius", "var(--radius-sm)"), ("background", "transparent"), ("color", MUTED), ("font", "inherit"), ("font-weight", "var(--font-weight-medium, 500)"), ("cursor", "pointer")]),
    )
    .nest("& [data-fb-tabs] button[aria-selected=true]", pairs(&[("background", "var(--ui-bg)"), ("color", TEXT), ("box-shadow", "var(--elevation-control, 0 1px 2px rgb(0 0 0 / 0.08))")]))
    .nest("& [data-fb-tabs] button:focus-visible", forms::focus_ring())
    .nest("& [role=tabpanel][hidden]", pairs(&[("display", "none")]))
    // ---- build: palette, fields, inspector
    .nest("& [data-fb-build]", pairs(&[("display", "grid"), ("grid-template-columns", "176px minmax(0, 1fr) 280px"), ("gap", gap!(16)), ("align-items", "start")]))
    .nest("@container (max-width: 820px)", pairs(&[]).nest("& [data-fb-build]", pairs(&[("grid-template-columns", "minmax(0, 1fr)")])))
    .nest("& [data-fb-heading]", pairs(&[("margin", "0 0 8px"), ("color", MUTED), ("font-size", fs!(12)), ("font-weight", "var(--font-weight-medium, 500)"), ("text-transform", "uppercase"), ("letter-spacing", "0.04em")]))
    .nest("& [data-fb-palette]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(4))]))
    .nest(
        "& [data-fb-palette] button",
        pairs(&[
            ("display", "flex"),
            ("align-items", "center"),
            ("gap", gap!(8)),
            ("height", ctl!(32)),
            ("padding", pad!(0, 10)),
            ("border", BORDER),
            ("border-radius", "var(--radius-control, var(--radius-md))"),
            ("background", "var(--ui-bg)"),
            ("color", "inherit"),
            ("font", "inherit"),
            ("text-align", "start"),
            ("cursor", "grab"),
        ]),
    )
    .nest("& [data-fb-palette] button:hover", pairs(&[("background", SUBTLE)]))
    .nest("& [data-fb-palette] button:focus-visible", forms::focus_ring())
    .nest("& [data-fb-glyph]", pairs(&[("display", "inline-grid"), ("place-items", "center"), ("flex", "none"), ("width", "22px"), ("height", "22px"), ("border-radius", "var(--radius-sm)"), ("background", SUBTLE), ("color", MUTED), ("font-size", fs!(11)), ("font-weight", "var(--font-weight-semibold, 600)")]))
    .nest(
        "& [data-fb-canvas]",
        pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(6)), ("min-height", sp!(200)), ("padding", pad!(10)), ("border", "var(--border-width, 1px) dashed var(--color-border)"), ("border-radius", "var(--radius-card, var(--radius-lg))")]),
    )
    .nest("& [data-fb-canvas][data-fb-over]", pairs(&[("border-color", PRIMARY), ("background", TINT)]))
    .nest("& [data-fb-empty]", pairs(&[("margin", "auto"), ("padding", pad!(24)), ("color", MUTED), ("text-align", "center")]))
    .nest(
        "& [data-fb-card]",
        pairs(&[
            ("position", "relative"),
            ("display", "flex"),
            ("align-items", "center"),
            ("gap", gap!(6)),
            ("padding", pad!(6, 6, 6, 4)),
            ("border", BORDER),
            ("border-radius", "var(--radius-control, var(--radius-md))"),
            ("background", "var(--ui-bg)"),
        ]),
    )
    .nest("& [data-fb-card][data-section]", pairs(&[("margin-top", gap!(8)), ("background", SUBTLE)]))
    .nest("& [data-fb-card][aria-current=true]", pairs(&[("border-color", PRIMARY), ("box-shadow", "0 0 0 1px var(--color-primary)")]))
    .nest("& [data-fb-card][data-dragging]", pairs(&[("opacity", "0.45")]))
    .nest(
        "& [data-fb-card][data-drop]::before",
        pairs(&[("content", "''"), ("position", "absolute"), ("left", "0"), ("right", "0"), ("height", "2px"), ("border-radius", "1px"), ("background", PRIMARY), ("pointer-events", "none")]),
    )
    .nest("& [data-fb-card][data-drop=before]::before", pairs(&[("top", "-5px")]))
    .nest("& [data-fb-card][data-drop=after]::before", pairs(&[("bottom", "-5px")]))
    .nest(
        "& [data-fb-card] button",
        pairs(&[("display", "inline-flex"), ("align-items", "center"), ("justify-content", "center"), ("flex", "none"), ("min-width", "26px"), ("height", "28px"), ("padding", pad!(0, 4)), ("border", "0"), ("border-radius", "var(--radius-sm)"), ("background", "transparent"), ("color", MUTED), ("font", "inherit"), ("cursor", "pointer")]),
    )
    .nest("& [data-fb-card] button:hover", pairs(&[("background", SUBTLE), ("color", TEXT)]))
    .nest("& [data-fb-card] button:focus-visible", forms::focus_ring())
    .nest("& [data-fb-card] [data-fb-grip]", pairs(&[("cursor", "grab"), ("touch-action", "none")]))
    .nest(
        "& [data-fb-card] [data-fb-pick]",
        pairs(&[("flex", "1"), ("min-width", "0"), ("justify-content", "flex-start"), ("gap", gap!(8)), ("color", TEXT), ("text-align", "start"), ("overflow", "hidden"), ("white-space", "nowrap"), ("text-overflow", "ellipsis")]),
    )
    .nest("& [data-fb-card] [data-fb-pick]:hover", pairs(&[("background", "transparent")]))
    .nest("& [data-fb-meta]", pairs(&[("color", MUTED), ("font-size", fs!(12))]))
    .nest("& [data-fb-required]", pairs(&[("color", "var(--color-destructive, #dc2626)")]))
    .nest("& [data-fb-inspector]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(12)), ("padding", pad!(14)), ("border", BORDER), ("border-radius", "var(--radius-card, var(--radius-lg))")]))
    // ---- the controls the builder and the preview draw
    .nest("& [data-fb-field]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(6))]))
    .nest("& [data-fb-field] > label, & [data-fb-legend]", pairs(&[("font-weight", "var(--font-weight-medium, 500)")]))
    .nest("& [data-fb-help]", pairs(&[("color", MUTED), ("font-size", fs!(13))]))
    .nest("& [data-fb-error]", pairs(&[("color", "var(--color-destructive, #dc2626)"), ("font-size", fs!(13))]))
    .nest(
        "& :is(input:not([type=checkbox], [type=radio]), textarea, select)",
        pairs(&[
            ("box-sizing", "border-box"),
            ("width", "100%"),
            ("min-height", ctl!(36)),
            ("padding", pad!(6, 10)),
            ("border", BORDER),
            ("border-radius", "var(--radius-control, var(--radius-md))"),
            ("background", "transparent"),
            ("color", TEXT),
            ("font", "inherit"),
        ]),
    )
    .nest("& :is(input:not([type=checkbox], [type=radio]), textarea, select):focus-visible", pairs(&[("outline", "none"), ("border-color", RING), ("box-shadow", "0 0 0 3px color-mix(in srgb, var(--color-ring, var(--color-muted)) 50%, transparent)")]))
    .nest("& [aria-invalid=true]", pairs(&[("border-color", "var(--color-destructive, #dc2626) !important")]))
    .nest("& input[type=checkbox], & input[type=radio]", pairs(&[("width", "16px"), ("height", "16px"), ("margin", "0"), ("accent-color", "var(--color-primary)")]))
    .nest("& [data-fb-check]", pairs(&[("display", "inline-flex"), ("align-items", "center"), ("gap", gap!(8)), ("cursor", "pointer")]))
    .nest("& [data-fb-legend]", pairs(&[("padding", "0"), ("margin-bottom", gap!(4))]))
    .nest("& [data-fb-choices]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(6)), ("margin", "0"), ("padding", "0"), ("border", "0")]))
    .nest("& [data-fb-row]", pairs(&[("display", "grid"), ("grid-template-columns", "1fr 1fr"), ("gap", gap!(8))]))
    // ---- preview and document
    .nest("& [data-fb-preview]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", gap!(16)), ("max-width", sp!(560)), ("padding", pad!(20)), ("border", BORDER), ("border-radius", "var(--radius-card, var(--radius-lg))")]))
    .nest("& [data-fb-section-title]", pairs(&[("margin", concat!(gap!(8), " 0 0")), ("font-size", fs!(16)), ("font-weight", "var(--font-weight-semibold, 600)")]))
    .nest("& [data-fb-actions]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("align-items", "center"), ("gap", gap!(8))]))
    .nest(
        "& [data-fb-button]",
        pairs(&[
            ("display", "inline-flex"),
            ("align-items", "center"),
            ("gap", gap!(6)),
            ("height", ctl!(34)),
            ("padding", pad!(0, 14)),
            ("border", BORDER),
            ("border-radius", "var(--radius-control, var(--radius-md))"),
            ("background", "transparent"),
            ("color", "inherit"),
            ("font", "inherit"),
            ("font-weight", "var(--font-weight-medium, 500)"),
            ("cursor", "pointer"),
        ]),
    )
    .nest("& [data-fb-button][data-primary]", pairs(&[("border-color", "transparent"), ("background", "var(--fill-primary, var(--color-primary))"), ("color", "var(--color-on-primary)")]))
    .nest("& [data-fb-button][aria-pressed=true]", pairs(&[("background", TINT), ("border-color", PRIMARY)]))
    .nest("& [data-fb-button]:hover:not([data-primary])", pairs(&[("background", SUBTLE)]))
    .nest("& [data-fb-button]:focus-visible", forms::focus_ring())
    .nest("& [data-fb-done]", pairs(&[("padding", pad!(12)), ("border-radius", "var(--radius-control, var(--radius-md))"), ("background", "color-mix(in srgb, var(--color-success, #16a34a) 12%, transparent)")]))
    .nest(
        "& pre",
        pairs(&[
            ("margin", "0"),
            ("max-height", sp!(520)),
            ("overflow", "auto"),
            ("padding", pad!(14)),
            ("border", BORDER),
            ("border-radius", "var(--radius-card, var(--radius-lg))"),
            ("background", "var(--color-code, var(--color-muted-surface, color-mix(in srgb, var(--color-text) 4%, transparent)))"),
            ("font-family", "var(--font-mono, ui-monospace, monospace)"),
            ("font-size", fs!(12.5)),
            ("line-height", "1.55"),
            ("tab-size", "2"),
        ]),
    )
    .nest(
        "& [data-fb-live]",
        pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("overflow", "hidden"), ("clip-path", "inset(50%)"), ("white-space", "nowrap")]),
    )
});

pub struct FormBuilder;

impl Component for FormBuilder {
    fn name(&self) -> &'static str {
        "ui-form-builder"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A form put together in the browser: field types added from a palette, put in order, set up in an inspector, tried in a preview, and given out as an iRender document of the kit's own form.",
            props: PROPS,
            parts: &[],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let key = cx.prop_string(node, "bind").filter(|k| !k.is_empty()).ok_or_else(|| RenderError::new(node, "`bind` is the state key holding the fields"))?;
        match cx.lookup(std::slice::from_ref(&key)) {
            Json::Array(_) | Json::Null => {}
            _ => return Err(RenderError::new(node, format!("`{key}` must hold a list of fields"))),
        }
        let types: Vec<String> = match cx.prop(node, "types") {
            None | Some(Json::Null) => TYPES.iter().map(|t| t.to_string()).collect(),
            Some(Json::Array(list)) if !list.is_empty() => {
                let mut out = Vec::new();
                for t in &list {
                    match t.as_str() {
                        Some(t) if TYPES.contains(&t) => out.push(t.to_string()),
                        _ => return Err(RenderError::new(node, format!("`types` are {}, got `{}`", TYPES.join(", "), expr::stringify(t)))),
                    }
                }
                out
            }
            Some(_) => return Err(RenderError::new(node, "`types` must be a list of field types")),
        };
        if let Some(e) = cx.prop_string(node, "endpoint")
            && !(e.starts_with('/') || e.starts_with("https://") || e.starts_with("http://"))
        {
            return Err(RenderError::new(node, "`endpoint` must be a path on the site or an http(s) URL"));
        }
        cx.global_js("ui-form-builder", SCRIPT);
        let id = instance_id(cx, node, "form-builder");
        let config = json!({
            "bind": key,
            "bindDocument": cx.prop_string(node, "bindDocument"),
            "types": types,
            "submitLabel": cx.prop(node, "submitLabel").filter(|v| !v.is_null()).map(|v| expr::stringify(&v)).unwrap_or_else(|| cx.label("Submit")),
            "endpoint": cx.prop_string(node, "endpoint"),
            "id": expr::stringify(&cx.eval(&id)),
        });
        let mut root = cx.element("div", node);
        root.base_style_ref(&BUILDER)
            .behavior("ui-form-builder", config)
            .attr("id", id.clone())
            .attr("role", "group")
            .attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Form builder"))));
        cx.open(&root, node)?;

        // The tabs, and a panel for each that the behaviour fills.
        let base = expr::stringify(&cx.eval(&id));
        let mut tabs = Element::new("div");
        tabs.attr("role", "tablist").attr("data-fb-tabs", true).attr("aria-label", cx.label("View"));
        cx.open(&tabs, node)?;
        let names = [("build", cx.label("Build")), ("preview", cx.label("Preview")), ("document", cx.label("Document"))];
        for (i, (k, text)) in names.iter().enumerate() {
            let mut b = Element::new("button");
            b.attr("type", "button")
                .attr("role", "tab")
                .attr("id", format!("{base}-tab-{k}"))
                .attr("aria-controls", format!("{base}-{k}"))
                .attr("aria-selected", if i == 0 { "true" } else { "false" })
                .attr("tabindex", if i == 0 { "0" } else { "-1" })
                .attr("data-fb-tab", *k)
                .text(lit(text.clone()));
            cx.open(&b, node)?;
            cx.close("button");
        }
        cx.close("div");
        for (i, (k, _)) in names.iter().enumerate() {
            let mut p = Element::new("div");
            p.attr("role", "tabpanel").attr("id", format!("{base}-{k}")).attr("aria-labelledby", format!("{base}-tab-{k}")).attr("data-fb-panel", *k).attr("hidden", i != 0);
            cx.open(&p, node)?;
            cx.close("div");
        }
        let mut live = Element::new("div");
        live.attr("data-fb-live", true).attr("role", "status").attr("aria-live", "polite");
        cx.open(&live, node)?;
        cx.close("div");
        cx.close("div");
        Ok(())
    }
}
