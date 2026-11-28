//! `ui-combobox`: a searchable select in the style of react-select. Single or
//! multiple values, groups, disabled and fixed options, a clear button, options
//! loaded as you type, and creatable values, with the WAI-ARIA combobox pattern.
//!
//! The server renders the control with its current value; `combobox.js` (shipped
//! once per page through `global_js`) builds the menu and handles input.

use crate::{
    BG_POPOVER, BORDER, DESTRUCTIVE, MUTED, PRIMARY, RING, SPIN, TEXT, id_with, instance_id,
    lit, server_array,
};
use irender_core::{ComponentInfo, Expr, Node, PropDef, PropKind, StyleBlock, Val, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Map, Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("combobox.js");

pub struct Combobox;

const PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key holding the value (an array with `multiple`)"),
    PropDef::new(
        "options",
        PropKind::Any,
        "Values, `{ value, label, description, keywords, disabled }`, or groups `{ label, options }`; `keywords` (words or a list) find an option without being shown. From data, or a state path",
    ),
    PropDef::new("valueField", PropKind::String, "Field of an option object holding its value (default `value`)"),
    PropDef::new("labelField", PropKind::String, "Field holding its label (default `label`)"),
    PropDef::new("descriptionField", PropKind::String, "Field holding its description (default `description`)"),
    PropDef::new("imageField", PropKind::String, "Field holding a picture shown before its label: an avatar, a flag, a logo (default `image`)"),
    PropDef::new("keywordsField", PropKind::String, "Field holding words an option is found by, not shown (default `keywords`)"),
    PropDef::new("badgeField", PropKind::String, "Field holding short text shown after its label: a count, a status (default `badge`)"),
    PropDef::new("multiple", PropKind::Bool, "Choose several values, shown as removable chips"),
    PropDef::new("searchable", PropKind::Bool, "Type to filter (default true)"),
    PropDef::new("clearable", PropKind::Bool, "Show a button that clears the value"),
    PropDef::new("creatable", PropKind::Bool, "Offer to create a value from what was typed"),
    PropDef::new("searchUrl", PropKind::Any, "Load options as you type: GET `<url>?q=<text>`, answering an array"),
    PropDef::new("labels", PropKind::Any, "Options for values chosen before they're searched for, so they show their names: `[{ value, label }]`; may use state"),
    PropDef::new("lookup", PropKind::Bool, "With `searchUrl`: names of chosen values it doesn't know are asked for, GET `<url>?values=a,b`"),
    PropDef::new("minChars", PropKind::Number, "Characters typed before searching `searchUrl` (default 1)"),
    PropDef::new("debounce", PropKind::Number, "Milliseconds to wait after typing before searching (default 250)"),
    PropDef::new("closeOnSelect", PropKind::Bool, "Close the menu after choosing (default: unless `multiple`)"),
    PropDef::new("hideSelected", PropKind::Bool, "Leave chosen options out of the menu (default: with `multiple`)"),
    PropDef::new("fixed", PropKind::Any, "Values that can't be removed"),
    PropDef::new("value", PropKind::Any, "Initial value, when not bound"),
    PropDef::new("placeholder", PropKind::Any, "Shown while nothing is chosen (default `Select…`)"),
    PropDef::new("label", PropKind::Any, "Accessible name, when no visible label points at `parts.input`"),
    PropDef::new("name", PropKind::String, "Form field name: hidden inputs carry the value(s)"),
    PropDef::new("disabled", PropKind::Any, "Boolean or expression"),
    PropDef::new("loading", PropKind::Any, "Show the loading indicator, e.g. `{{ fetching }}`"),
    PropDef::new("noOptionsText", PropKind::Any, "Default `No options`"),
    PropDef::new("loadingText", PropKind::Any, "Default `Loading…`"),
    PropDef::new("createText", PropKind::Any, "Default `Create \"{input}\"`"),
    PropDef::new("id", PropKind::Any, "Element id (default: derived from the document)"),
];

fn focus_ring() -> StyleBlock {
    StyleBlock::from_pairs(&[
        ("border-color", RING),
        ("box-shadow", "0 0 0 3px color-mix(in srgb, var(--color-ring, var(--color-muted)) 50%, transparent)"),
    ])
}

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

/// A light tint of the primary colour, and text in it that reads on that tint (the
/// primary, pulled toward the text colour so a pale primary stays legible).
const PRIMARY_TINT: &str = "color-mix(in srgb, var(--color-primary) 12%, var(--ui-bg, var(--color-background, var(--color-surface))))";
const PRIMARY_TEXT: &str = "color-mix(in srgb, var(--color-primary) 75%, var(--color-text))";

/// Everything is styled from the root, through attribute selectors, because the
/// behaviour creates most of the elements (chips, options) in the browser.
static ROOT: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("position", "relative"), ("width", "100%"), ("font-size", fs!(14)), ("color", TEXT)])
        .nest(
            "& [data-ui-control]",
            pairs(&[
                ("box-sizing", "border-box"),
                ("display", "flex"),
                ("align-items", "center"),
                ("gap", gap!(4)),
                ("min-height", ctl!(36)),
                ("padding-block", pad!(2, 2)), ("padding-inline", pad!(8, 4)),
                ("border", "var(--border-width, 1px) solid var(--color-border)"),
                ("border-radius", "var(--radius-control, var(--radius-md))"),
                ("background", "transparent"),
                ("box-shadow", "var(--elevation-control, 0 1px 2px rgb(0 0 0 / 0.05))"),
                ("cursor", "default"),
                ("transition", "border-color .15s, box-shadow .15s"),
            ]),
        )
        .nest("&[data-focused] [data-ui-control]", focus_ring())
        .nest("&[data-disabled] [data-ui-control]", pairs(&[("opacity", "0.5"), ("cursor", "not-allowed")]))
        .nest(
            "& [data-ui-values]",
            pairs(&[("display", "flex"), ("flex", "1"), ("flex-wrap", "wrap"), ("align-items", "center"), ("gap", gap!(4)), ("min-width", "0")]),
        )
        .nest(
            "& [data-ui-input]",
            pairs(&[
                ("flex", "1 1 40px"),
                ("min-width", ctl!(40)),
                ("height", ctl!(26)),
                ("margin", "0"),
                ("padding", pad!(0, 2)),
                ("border", "0"),
                ("outline", "none"),
                ("background", "transparent"),
                ("color", "inherit"),
                ("font", "inherit"),
            ]),
        )
        .nest("& [data-ui-input]::placeholder", pairs(&[("color", MUTED)]))
        .nest("& [data-ui-input]:read-only", pairs(&[("cursor", "default"), ("caret-color", "transparent")]))
        .nest(
            "& [data-ui-single]",
            pairs(&[("overflow", "hidden"), ("padding", pad!(0, 2)), ("white-space", "nowrap"), ("text-overflow", "ellipsis")]),
        )
        .nest("&[data-typing] [data-ui-single]", pairs(&[("display", "none")]))
        .nest(
            "& [data-ui-chip]",
            pairs(&[
                ("display", "inline-flex"),
                ("align-items", "center"),
                ("gap", gap!(2)),
                ("max-width", "100%"),
                ("height", ctl!(24)),
                ("padding-block", pad!(0, 0)), ("padding-inline", pad!(8, 2)),
                ("border-radius", "var(--radius-sm)"),
                // Tinted with the primary colour, like the selected options they stand for.
                ("background", PRIMARY_TINT),
                ("color", PRIMARY_TEXT),
                ("font-size", fs!(12)),
                ("font-weight", "var(--font-weight-medium, 500)"),
            ]),
        )
        .nest(
            "& [data-ui-chip][data-fixed]",
            pairs(&[("padding-inline-end", pad!(8)), ("background", "color-mix(in srgb, var(--color-muted) 30%, transparent)")]),
        )
        .nest(
            "& [data-ui-remove]",
            pairs(&[
                ("display", "inline-flex"),
                ("align-items", "center"),
                ("justify-content", "center"),
                ("width", "18px"),
                ("height", "18px"),
                ("padding", "0"),
                ("border", "0"),
                ("border-radius", "var(--radius-sm)"),
                ("background", "transparent"),
                ("color", "inherit"),
                ("font-size", fs!(10)),
                ("opacity", "0.6"),
                ("cursor", "pointer"),
            ]),
        )
        .nest(
            "& [data-ui-remove]:hover",
            pairs(&[("opacity", "1"), ("color", DESTRUCTIVE), ("background", "color-mix(in srgb, var(--color-destructive, #dc2626) 15%, transparent)")]),
        )
        .nest(
            "& [data-ui-indicators]",
            pairs(&[("display", "flex"), ("align-items", "center"), ("align-self", "stretch"), ("gap", gap!(2)), ("color", MUTED)]),
        )
        .nest("& [data-ui-indicators] button", indicator())
        .nest("& [data-ui-indicators] button:hover", pairs(&[("color", TEXT)]))
        .nest("& [data-ui-clear][hidden]", pairs(&[("display", "none")]))
        // A chevron drawn with borders: glyphs like ⌄ sit on the text baseline.
        .nest(
            "& [data-ui-toggle]::before",
            pairs(&[
                ("content", "''"),
                ("width", "6px"),
                ("height", "6px"),
                ("border-inline-end", "1.5px solid currentColor"),
                ("border-bottom", "1.5px solid currentColor"),
                ("transform", "translateY(-2px) rotate(45deg)"),
            ]),
        )
        .nest("&[data-open] [data-ui-toggle]", pairs(&[("transform", "rotate(180deg)")]))
        .nest("& [data-ui-sep]", pairs(&[("width", "1px"), ("align-self", "stretch"), ("margin", gap!(6, 2)), ("background", BORDER)]))
        .nest(
            "& [data-ui-spinner]",
            pairs(&[
                ("display", "none"),
                ("box-sizing", "border-box"),
                ("width", "14px"),
                ("height", "14px"),
                ("margin", gap!(0, 4)),
                ("border", "2px solid color-mix(in srgb, var(--color-primary) 25%, transparent)"),
                ("border-top-color", PRIMARY),
                ("border-radius", "999px"),
                ("animation", "ui-spin .7s linear infinite"),
            ]),
        )
        .nest("&[data-loading] [data-ui-spinner]", pairs(&[("display", "block")]))
        .nest("&[aria-busy=true] [data-ui-spinner]", pairs(&[("display", "block")]))
        .nest(
            "& [data-ui-menu]",
            pairs(&[
                ("box-sizing", "border-box"),
                ("width", "anchor-size(width)"),
                ("max-height", "300px"),
                ("margin", gap!(4, 0)),
                ("inset", "auto"),
                ("padding", pad!(4)),
                ("overflow-y", "auto"),
                ("border", "var(--border-width, 1px) solid var(--color-border)"),
                ("border-radius", "var(--radius-card, var(--radius-md))"),
                ("--ui-bg", BG_POPOVER),
                ("background", BG_POPOVER),
                ("color", TEXT),
                ("font-size", fs!(14)),
                ("box-shadow", "var(--elevation-popover, 0 6px 18px rgb(0 0 0 / 0.12))"),
            ]),
        )
        .nest(
            "& [role=option]",
            pairs(&[
                ("display", "grid"),
                ("grid-template-columns", "1fr auto"),
                ("column-gap", gap!(8)),
                ("padding", pad!(6, 8)),
                ("border-radius", "var(--radius-sm)"),
                ("cursor", "pointer"),
                ("user-select", "none"),
            ]),
        )
        .nest("& [role=option][data-active]", pairs(&[("background", "color-mix(in srgb, var(--color-primary) 10%, transparent)")]))
        .nest("& [role=option][aria-selected=true]", pairs(&[("font-weight", "var(--font-weight-medium, 500)")]))
        .nest(
            "& [role=option][aria-selected=true]::after",
            pairs(&[("content", "'✓'"), ("grid-column", "2"), ("grid-row", "1"), ("color", PRIMARY)]),
        )
        .nest("& [role=option][data-create]", pairs(&[("color", PRIMARY), ("font-weight", "var(--font-weight-medium, 500)")]))
        .nest("& [role=option][aria-disabled=true]", pairs(&[("opacity", "0.5"), ("cursor", "not-allowed")]))
        // A picture before the label and a badge after it.
        .nest("& [data-ui-option-label]", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", gap!(8)), ("min-width", "0")]))
        .nest("& [data-ui-option-label] > span:not([data-ui-badge])", pairs(&[("min-width", "0"), ("overflow", "hidden"), ("text-overflow", "ellipsis"), ("white-space", "nowrap")]))
        .nest("& :is([data-ui-option-label], [data-ui-chip]) > img", pairs(&[("flex", "none"), ("width", "20px"), ("height", "20px"), ("border-radius", "50%"), ("object-fit", "cover")]))
        .nest("& [data-ui-chip] > img", pairs(&[("width", "16px"), ("height", "16px"), ("margin-inline-end", "4px")]))
        .nest(
            "& [data-ui-badge]",
            pairs(&[("margin-inline-start", "auto"), ("padding", pad!(0, 6)), ("border-radius", "999px"), ("background", "color-mix(in srgb, var(--color-text) 8%, transparent)"), ("color", MUTED), ("font-size", fs!(11)), ("font-weight", "500"), ("line-height", "18px")]),
        )
        .nest("& [data-ui-description]", pairs(&[("grid-column", "1"), ("font-size", fs!(12)), ("font-weight", "400"), ("color", MUTED)]))
        .nest(
            "& [data-ui-group-label]",
            pairs(&[("padding", pad!(6, 8, 4)), ("font-size", fs!(12)), ("font-weight", "var(--font-weight-semibold, 600)"), ("color", MUTED)]),
        )
        .nest(
            "& [role=group]:not(:first-child)",
            pairs(&[("margin-top", gap!(4)), ("padding-top", pad!(4)), ("border-top", "var(--border-width, 1px) solid var(--color-border)")]),
        )
        .nest("& [data-ui-message]", pairs(&[("padding", pad!(8)), ("text-align", "center"), ("color", MUTED)]))
        .nest(
            "& [data-ui-live]",
            pairs(&[
                ("position", "absolute"),
                ("width", "1px"),
                ("height", "1px"),
                ("overflow", "hidden"),
                ("clip-path", "inset(50%)"),
                ("white-space", "nowrap"),
            ]),
        )
});

fn indicator() -> StyleBlock {
    pairs(&[
        ("display", "inline-flex"),
        ("align-items", "center"),
        ("justify-content", "center"),
        ("width", "26px"),
        ("height", ctl!(26)),
        ("padding", "0"),
        ("border", "0"),
        ("border-radius", "var(--radius-sm)"),
        ("background", "transparent"),
        ("color", "inherit"),
        ("font-size", fs!(12)),
        ("cursor", "pointer"),
        ("transition", "transform .15s, color .15s"),
    ])
}

/// Which fields of an option object hold its value, label and description, like
/// react-select's `getOptionValue` / `getOptionLabel`.
#[derive(Clone)]
struct Fields {
    value: String,
    label: String,
    description: String,
    image: String,
    badge: String,
    keywords: String,
}

impl Default for Fields {
    fn default() -> Self {
        Self { value: "value".into(), label: "label".into(), description: "description".into(), image: "image".into(), badge: "badge".into(), keywords: "keywords".into() }
    }
}

/// `{ value, label, description?, disabled? }`, or a group `{ group, options }`.
fn normalize(o: &Json, f: &Fields) -> Json {
    match o {
        Json::Object(m) if m.get("options").is_some_and(Json::is_array) => {
            let options = m["options"].as_array().into_iter().flatten().map(|o| normalize(o, f)).collect::<Vec<_>>();
            json!({ "group": expr::stringify(m.get("label").unwrap_or(&Json::Null)), "options": options })
        }
        Json::Object(m) => {
            let value = m.get(&f.value).cloned().unwrap_or(Json::Null);
            let label = m.get(&f.label).filter(|l| !l.is_null()).map(expr::stringify).unwrap_or_else(|| expr::stringify(&value));
            let mut out = Map::new();
            out.insert("value".into(), value);
            out.insert("label".into(), Json::String(label));
            for (field, name) in [(&f.description, "description"), (&f.image, "image"), (&f.badge, "badge")] {
                if let Some(d) = m.get(field).filter(|d| !d.is_null() && *d != "") {
                    out.insert(name.into(), Json::String(expr::stringify(d)));
                }
            }
            if m.get("disabled").is_some_and(expr::truthy) {
                out.insert("disabled".into(), Json::Bool(true));
            }
            // Words it's found by, not shown.
            match m.get(&f.keywords) {
                Some(Json::Array(k)) if !k.is_empty() => {
                    out.insert("keywords".into(), Json::String(k.iter().map(expr::stringify).collect::<Vec<_>>().join(" ")));
                }
                Some(Json::String(k)) if !k.is_empty() => {
                    out.insert("keywords".into(), Json::String(k.clone()));
                }
                _ => {}
            }
            Json::Object(out)
        }
        other => json!({ "value": other, "label": expr::stringify(other) }),
    }
}

/// The label of `value` among `options` (groups included), or the value as text.
fn label_of(options: &[Json], value: &Json) -> String {
    options
        .iter()
        .flat_map(|o| match o.get("options").and_then(Json::as_array) {
            Some(group) => group.iter().collect::<Vec<_>>(),
            None => vec![o],
        })
        .find(|o| o.get("value") == Some(value))
        .and_then(|o| o.get("label").and_then(Json::as_str).map(String::from))
        .unwrap_or_else(|| expr::stringify(value))
}

/// Only same-site paths and http(s) URLs can be searched. `//host/…` (and `/\host`,
/// which browsers read the same way) would quietly send keystrokes to another site.
pub(crate) fn safe_search_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    if lower.contains('\\') || lower.chars().any(char::is_control) {
        return false;
    }
    if lower.starts_with("https://") || lower.starts_with("http://") {
        return true;
    }
    if lower.starts_with("//") {
        return false;
    }
    // A relative path has no scheme: no `:` before the first `/`, `?` or `#`.
    let end = lower.find(['/', '?', '#']).unwrap_or(lower.len());
    !lower[..end].contains(':')
}

impl Component for Combobox {
    fn name(&self) -> &'static str {
        "ui-combobox"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A searchable select: one or many values, groups, async options, creatable values, full keyboard support.",
            props: PROPS,
            parts: &["control", "input", "menu"],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        crate::anchors(cx);
        let flag = |cx: &RenderCx<'_>, name: &str, default: bool| cx.prop(node, name).map_or(default, |v| expr::truthy(&v));
        // A prop's text, or the default's label.
        // Labels: "Select…", "No options", "Loading…", "Create “{input}”"
        let text = |cx: &RenderCx<'_>, name: &str, default: &str| cx.prop_string(node, name).unwrap_or_else(|| cx.label(default));
        let multiple = flag(cx, "multiple", false);
        let key = cx.prop_string(node, "bind");

        // Options: fixed at render, or a state path the behaviour reads live.
        let (options, options_path) = match node.prop("options") {
            Some(Val::Tpl(t)) if cx.is_reactive(&Val::Tpl(t.clone())) => match t.as_single_expr() {
                Some(Expr::Path(path)) => {
                    let current = match cx.prop(node, "options") {
                        Some(Json::Array(items)) => items,
                        _ => Vec::new(),
                    };
                    (current, Some(path.clone()))
                }
                _ => return Err(RenderError::new(node, "`options` that use state must be a plain path, like `{{ cities }}`")),
            },
            _ => (server_array(cx, node, "options")?, None),
        };
        let fields = Fields {
            value: cx.prop_string(node, "valueField").unwrap_or_else(|| "value".into()),
            label: cx.prop_string(node, "labelField").unwrap_or_else(|| "label".into()),
            description: cx.prop_string(node, "descriptionField").unwrap_or_else(|| "description".into()),
            image: cx.prop_string(node, "imageField").unwrap_or_else(|| "image".into()),
            badge: cx.prop_string(node, "badgeField").unwrap_or_else(|| "badge".into()),
            keywords: cx.prop_string(node, "keywordsField").unwrap_or_else(|| "keywords".into()),
        };
        let options: Vec<Json> = options.iter().map(|o| normalize(o, &fields)).collect();

        let search_url = cx.prop_string(node, "searchUrl").filter(|u| !u.is_empty());
        if let Some(u) = &search_url
            && !safe_search_url(u)
        {
            return Err(RenderError::new(node, format!("`searchUrl` must be a path or an http(s) URL, got `{u}`")));
        }

        let current = match &key {
            Some(k) => cx.lookup(std::slice::from_ref(k)),
            None => cx.prop(node, "value").unwrap_or(Json::Null),
        };
        let selected: Vec<Json> = match (&current, multiple) {
            (Json::Array(items), true) => items.clone(),
            (Json::Null, _) => Vec::new(),
            (Json::String(s), false) if s.is_empty() => Vec::new(),
            (v, _) => vec![v.clone()],
        };
        let fixed = server_array(cx, node, "fixed")?;
        let name = cx.prop_string(node, "name");
        let placeholder = text(cx, "placeholder", "Select…");

        let id = instance_id(cx, node, "combobox");
        let list_id = id_with(&id, "-listbox");
        let anchor = format!("--{}", expr::stringify(&cx.eval(&id)));
        let config = json!({
            "key": key,
            "multiple": multiple,
            "searchable": flag(cx, "searchable", true),
            "clearable": flag(cx, "clearable", false),
            "creatable": flag(cx, "creatable", false),
            "closeOnSelect": flag(cx, "closeOnSelect", !multiple),
            "hideSelected": flag(cx, "hideSelected", multiple),
            "options": options,
            "optionsPath": options_path,
            // Options from state or `searchUrl` are normalized in the browser with these.
            "fields": { "value": fields.value, "label": fields.label, "description": fields.description, "image": fields.image, "badge": fields.badge, "keywords": fields.keywords },
            "searchUrl": search_url,
            "labels": node.prop("labels").map(|l| crate::data_table::dynamic(cx, node, "labels", l)).transpose()?,
            "lookup": cx.prop(node, "lookup").is_some_and(|v| expr::truthy(&v)),
            "minChars": cx.prop(node, "minChars").map_or(1.0, |v| expr::to_num(&v)).max(0.0),
            "debounce": cx.prop(node, "debounce").map_or(250.0, |v| expr::to_num(&v)).clamp(0.0, 5000.0),
            "fixed": fixed,
            "value": if key.is_none() { current.clone() } else { Json::Null },
            "name": name,
            "text": {
                "placeholder": placeholder,
                "noOptions": text(cx, "noOptionsText", "No options"),
                "loading": text(cx, "loadingText", "Loading…"),
                "create": text(cx, "createText", "Create \u{201c}{input}\u{201d}"),
                "error": cx.label("Couldn't load options"),
                "remove": cx.label("Remove {label}"),
            },
        });
        cx.global_js("ui-combobox", SCRIPT);
        cx.global_css("ui-spin", SPIN);

        let mut root = cx.element("div", node);
        root.base_style_ref(&ROOT).behavior("ui-combobox", config);
        if !selected.is_empty() {
            root.attr("data-has-value", true);
        }
        if let Some(l) = node.prop("loading") {
            root.attr("aria-busy", l.clone());
        }
        cx.open(&root, node)?;

        let mut control = cx.part("div", node, "control");
        let mut anchored = StyleBlock::new();
        anchored = anchored.set("anchor-name", anchor.clone());
        control.base_style(anchored).attr("data-ui-control", true);
        cx.open(&control, node)?;

        let mut values = Element::new("div");
        values.attr("data-ui-values", true);
        cx.open(&values, node)?;
        if multiple {
            for v in &selected {
                let mut chip = Element::new("span");
                chip.attr("data-ui-chip", true);
                let is_fixed = fixed.contains(v);
                if is_fixed {
                    chip.attr("data-fixed", true);
                }
                cx.open(&chip, node)?;
                let label = label_of(&options, v);
                let mut t = Element::new("span");
                t.text(lit(label.clone()));
                cx.open(&t, node)?;
                cx.close("span");
                if !is_fixed {
                    let mut x = Element::new("button");
                    x.attr("type", "button").attr("tabindex", lit(-1)).attr("data-ui-remove", true);
                    x.attr("aria-label", cx.label_with("Remove {label}", &[("label", &label)])).text(lit("✕"));
                    cx.open(&x, node)?;
                    cx.close("button");
                }
                cx.close("span");
            }
        } else if let Some(v) = selected.first() {
            let mut single = Element::new("span");
            single.attr("data-ui-single", true).text(lit(label_of(&options, v)));
            cx.open(&single, node)?;
            cx.close("span");
        }
        let mut input = cx.part("input", node, "input");
        input
            .attr("type", "text")
            .attr("data-ui-input", true)
            .attr("role", "combobox")
            .attr("aria-expanded", "false")
            .attr("aria-controls", list_id.clone())
            .attr("aria-autocomplete", "list")
            .attr("aria-haspopup", "listbox")
            .attr("autocomplete", "off")
            .attr("autocapitalize", "off")
            .attr("spellcheck", "false");
        if selected.is_empty() {
            input.attr("placeholder", placeholder);
        }
        if !flag(cx, "searchable", true) {
            input.attr("readonly", true);
        }
        if let Some(l) = node.prop("label") {
            input.attr("aria-label", l.clone());
        }
        if let Some(d) = node.prop("disabled") {
            input.attr("disabled", d.clone());
        }
        cx.open(&input, node)?;
        cx.close("div");

        let mut indicators = Element::new("div");
        indicators.attr("data-ui-indicators", true);
        cx.open(&indicators, node)?;
        let mut spinner = Element::new("span");
        spinner.attr("data-ui-spinner", true).attr("aria-hidden", "true");
        cx.open(&spinner, node)?;
        cx.close("span");
        let mut clear = Element::new("button");
        clear.attr("type", "button").attr("tabindex", lit(-1)).attr("data-ui-clear", true).attr("aria-label", cx.label("Clear"));
        let clearable = flag(cx, "clearable", false) && selected.iter().any(|v| !fixed.contains(v));
        if !clearable {
            clear.attr("hidden", true);
        }
        clear.text(lit("✕"));
        cx.open(&clear, node)?;
        cx.close("button");
        let mut sep = Element::new("span");
        sep.attr("data-ui-sep", true).attr("aria-hidden", "true");
        cx.open(&sep, node)?;
        cx.close("span");
        let mut toggle = Element::new("button");
        toggle.attr("type", "button").attr("tabindex", lit(-1)).attr("data-ui-toggle", true).attr("aria-hidden", "true");
        cx.open(&toggle, node)?;
        cx.close("button");
        cx.close("div");
        cx.close("div");

        let mut menu = cx.part("div", node, "menu");
        let placement = crate::with_area(StyleBlock::new().set("position-anchor", anchor).set("position-try-fallbacks", "flip-block"), "bottom span-right");
        menu.base_style(placement)
            .attr("data-ui-menu", true)
            .attr("popover", "manual")
            .attr("role", "listbox")
            .attr("id", list_id);
        if multiple {
            menu.attr("aria-multiselectable", "true");
        }
        cx.open(&menu, node)?;
        cx.close("div");

        let mut live = Element::new("div");
        live.attr("data-ui-live", true).attr("role", "status").attr("aria-live", "polite");
        cx.open(&live, node)?;
        cx.close("div");

        let mut hidden = Element::new("span");
        hidden.attr("data-ui-hidden", true);
        cx.open(&hidden, node)?;
        if let Some(n) = &name {
            let as_text = |v: &Json| match v {
                Json::Null => String::new(),
                Json::Object(_) | Json::Array(_) => v.to_string(),
                other => expr::stringify(other),
            };
            let first = [selected.first().cloned().unwrap_or(Json::Null)];
            for v in if multiple { &selected[..] } else { &first[..] } {
                let mut h = Element::new("input");
                h.attr("type", "hidden").attr("name", n.clone()).attr("value", as_text(v));
                cx.open(&h, node)?;
            }
        }
        cx.close("span");
        cx.close("div");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_urls() {
        for ok in ["/api/search", "api/search", "https://example.com/s", "http://localhost:8000/s?x=1"] {
            assert!(safe_search_url(ok), "{ok}");
        }
        for ok in ["search?x=a:b", "./s", "../api/s"] {
            assert!(safe_search_url(ok), "{ok}");
        }
        for bad in ["javascript:alert(1)", "data:application/json,[]", "//evil.example/s", "file:///etc/passwd",
                    "/\\evil.example/s", "\\\\evil.example", " //evil.example", "java\tscript:x", "JAVASCRIPT:x"] {
            assert!(!safe_search_url(bad), "{bad}");
        }
    }

    #[test]
    fn options_normalize() {
        let f = Fields::default();
        assert_eq!(normalize(&json!("a"), &f), json!({ "value": "a", "label": "a" }));
        assert_eq!(normalize(&json!(3), &f), json!({ "value": 3, "label": "3" }));
        assert_eq!(
            normalize(&json!({ "value": "de", "label": "Germany", "disabled": true, "description": null }), &f),
            json!({ "value": "de", "label": "Germany", "disabled": true })
        );
        assert_eq!(
            normalize(&json!({ "label": "Fruit", "options": ["apple"] }), &f),
            json!({ "group": "Fruit", "options": [{ "value": "apple", "label": "apple" }] })
        );
        let custom = Fields { value: "href".into(), label: "title".into(), description: "summary".into(), ..Fields::default() };
        assert_eq!(
            normalize(&json!({ "href": "a.html", "title": "A", "summary": "About A" }), &custom),
            json!({ "value": "a.html", "label": "A", "description": "About A" })
        );
        let opts = vec![normalize(&json!({ "label": "G", "options": [{ "value": 1, "label": "One" }] }), &f)];
        assert_eq!(label_of(&opts, &json!(1)), "One");
        assert_eq!(label_of(&opts, &json!(2)), "2");
    }
}
