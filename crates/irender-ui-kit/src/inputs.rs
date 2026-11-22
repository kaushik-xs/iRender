//! Inputs with a shape of their own: `ui-tags-input`, `ui-masked-input`, `ui-phone-input`
//! and `ui-time-picker`. Each is a box around a text field, driven by `inputs.js`.

use crate::{
    BG_POPOVER, BORDER, MUTED, SUBTLE, TEXT, forms, instance_id,
    widget::{self, flag, pairs},
};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Map, Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("inputs.js");

static BOX: LazyLock<StyleBlock> = LazyLock::new(|| {
    forms::native_scheme(pairs(&[
        ("position", "relative"),
        ("display", "flex"),
        ("align-items", "center"),
        ("box-sizing", "border-box"),
        ("width", "100%"),
        ("min-height", ctl!(36)),
        ("border", "var(--border-width, 1px) solid var(--color-border)"),
        ("border-radius", "var(--radius-control, var(--radius-md))"),
        ("background", "transparent"),
        ("color", TEXT),
        ("font-size", fs!(14)),
        ("box-shadow", "var(--elevation-control, 0 1px 2px rgb(0 0 0 / 0.05))"),
    ]))
    .nest("&:focus-within", forms::focus_ring().set("border-color", crate::RING))
    .nest("&:has([aria-invalid=true])", pairs(&[("border-color", "var(--color-destructive, #dc2626)")]))
    .nest("&:has(> input:disabled)", pairs(&[("opacity", "0.5"), ("cursor", "not-allowed")]))
    .nest(
        "& input",
        pairs(&[("flex", "1 1 80px"), ("min-width", "0"), ("height", ctl!(34)), ("padding", pad!(0, 12)), ("border", "0"), ("outline", "none"), ("background", "transparent"), ("color", "inherit"), ("font", "inherit")]),
    )
    .nest("& input::placeholder", pairs(&[("color", MUTED)]))
    // Tags.
    .nest("&[data-tags]", pairs(&[("flex-wrap", "wrap"), ("gap", gap!(4)), ("padding", pad!(3))]))
    .nest("&[data-tags] input", pairs(&[("height", ctl!(28)), ("padding", pad!(0, 8))]))
    .nest("& [data-tag]", pairs(&[("display", "inline-flex"), ("align-items", "center"), ("gap", gap!(2)), ("max-width", "100%"), ("height", ctl!(26)), ("padding-inline", pad!(8, 2)), ("border-radius", "var(--radius-sm)"), ("background", SUBTLE), ("font-size", fs!(13)), ("outline", "none")]))
    .nest("& [data-tag] > span", pairs(&[("min-width", "0"), ("overflow", "hidden"), ("text-overflow", "ellipsis"), ("white-space", "nowrap")]))
    .nest("& [data-tag]:focus", pairs(&[("box-shadow", "inset 0 0 0 2px var(--color-ring, var(--color-muted))")]))
    .nest("& [data-tag][data-bad]", pairs(&[("background", "color-mix(in srgb, var(--color-destructive, #dc2626) 14%, transparent)"), ("color", "var(--color-destructive, #dc2626)")]))
    .nest("& [data-tag] button", widget::tool().set("min-width", "20px").set("height", "20px"))
    .nest("& [data-tag] button:hover", pairs(&[("color", TEXT)]))
    .nest("& [data-tags-list]", pairs(&[("display", "contents")]))
    // The list under a box: suggestions, countries, times.
    .nest(
        "& [data-in-pop]",
        pairs(&[
            ("position", "absolute"),
            ("z-index", "30"),
            ("top", "calc(100% + 4px)"),
            ("inset-inline-start", "0"),
            ("box-sizing", "border-box"),
            ("min-width", "min(100%, 160px)"),
            ("max-width", "min(320px, 90vw)"),
            ("max-height", "260px"),
            ("overflow-y", "auto"),
            ("padding", pad!(4)),
            ("border", "var(--border-width, 1px) solid"),
            ("border-color", BORDER),
            ("border-radius", "var(--radius-md)"),
            ("background", BG_POPOVER),
            ("box-shadow", "var(--elevation-popover, 0 6px 18px rgb(0 0 0 / 0.12))"),
        ]),
    )
    .nest("& [data-in-pop][hidden]", pairs(&[("display", "none")]))
    .nest("& [data-in-pop][data-up]", pairs(&[("top", "auto"), ("bottom", "calc(100% + 4px)")]))
    .nest("& [role=option]", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", gap!(8)), ("padding", pad!(6, 8)), ("border-radius", "var(--radius-sm)"), ("white-space", "nowrap"), ("cursor", "pointer")]))
    .nest("& [role=option][data-on]", pairs(&[("background", SUBTLE)]))
    .nest("& [role=option][aria-selected=true]", pairs(&[("font-weight", "var(--font-weight-semibold, 600)")]))
    .nest("& [role=option] small", pairs(&[("margin-inline-start", "auto"), ("padding-inline-start", pad!(12)), ("color", MUTED), ("font-size", fs!(12))]))
    .nest("& [data-in-none]", pairs(&[("padding", pad!(8)), ("color", MUTED)]))
    // Phone: the country before the number.
    .nest(
        "& [data-country]",
        pairs(&[("display", "inline-flex"), ("align-items", "center"), ("gap", gap!(4)), ("align-self", "stretch"), ("padding", pad!(0, 8)), ("border", "0"), ("border-inline-end", "var(--border-width, 1px) solid var(--color-border)"), ("background", "none"), ("color", TEXT), ("font", "inherit"), ("white-space", "nowrap"), ("cursor", "pointer")]),
    )
    .nest("& [data-country] small", pairs(&[("color", MUTED), ("font-size", fs!(13)), ("font-variant-numeric", "tabular-nums")]))
    .nest("& [data-country]:focus-visible", forms::focus_ring().set("outline-offset", "-2px"))
    .nest("& [data-in-search]", pairs(&[("position", "sticky"), ("top", "-4px"), ("z-index", "1"), ("box-sizing", "border-box"), ("width", "100%"), ("margin-bottom", gap!(4)), ("border-bottom", "var(--border-width, 1px) solid var(--color-border) !important"), ("background", BG_POPOVER), ("flex", "none")]))
    // Time: the clock that opens the list.
    .nest("& [data-in-tool]", widget::tool().set("margin-inline-end", "4px"))
    .nest("& [data-in-tool]:hover", pairs(&[("background", SUBTLE), ("color", TEXT)]))
    .nest("& [data-in-tool]:focus-visible", forms::focus_ring())
});

/// The text field of one of these, with the document's own props on it.
fn field<'n>(cx: &mut RenderCx<'_>, node: &'n Node, prefix: &str, ty: &str) -> Element<'n> {
    let mut i = cx.part("input", node, "input");
    i.attr("type", ty.to_string()).attr("id", instance_id(cx, node, prefix)).attr("autocomplete", "off");
    for (prop, attr) in [("placeholder", "placeholder"), ("label", "aria-label"), ("disabled", "disabled"), ("invalid", "aria-invalid"), ("name", "name")] {
        if let Some(v) = node.prop(prop) {
            i.attr(attr, v.clone());
        }
    }
    i
}

fn open(cx: &mut RenderCx<'_>, node: &Node, name: &'static str, mut config: Map<String, Json>, tags: bool) -> Result<(), RenderError> {
    config.insert("locale".into(), json!(crate::locale(cx, node)));
    widget::kit(cx);
    cx.global_js("ui-inputs", SCRIPT);
    let mut el = cx.element("div", node);
    el.base_style_ref(&BOX).behavior(name, Json::Object(config));
    if tags {
        el.attr("data-tags", true);
    }
    cx.open(&el, node)
}

fn need_bind(cx: &RenderCx<'_>, node: &Node) -> Result<String, RenderError> {
    cx.prop_string(node, "bind").ok_or_else(|| RenderError::new(node, format!("`{}` needs `bind`", node.kind)))
}

// ---------------------------------------------------------------- tags

pub struct TagsInput;

const TAGS_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key holding the tags, a list of texts").required(),
    PropDef::new("suggestions", PropKind::Any, "Tags offered as it's typed in; may use state"),
    PropDef::new("only", PropKind::Bool, "Only suggestions can be added"),
    PropDef::new("max", PropKind::Number, "The most tags"),
    PropDef::new("pattern", PropKind::String, "A regular expression a tag must match, or it's marked wrong (and isn't kept, with `strict`)"),
    PropDef::new("strict", PropKind::Bool, "A tag that doesn't match `pattern` isn't added"),
    PropDef::new("lowercase", PropKind::Bool, "Tags are kept in lower case"),
    PropDef::new("placeholder", PropKind::Any, "Shown while there are none"),
    PropDef::new("label", PropKind::Any, "Accessible name, when no visible label names it"),
    PropDef::new("name", PropKind::String, "Form field name"),
    PropDef::new("disabled", PropKind::Any, "Boolean or expression"),
    PropDef::new("invalid", PropKind::Any, "Mark it wrong"),
];

impl Component for TagsInput {
    fn name(&self) -> &'static str {
        "ui-tags-input"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Tags typed one after another: Enter or a comma adds one, a pasted list adds them all, Backspace takes the last.", props: TAGS_PROPS, parts: &["input"], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let key = need_bind(cx, node)?;
        let mut config = widget::config(cx, node, &["suggestions", "placeholder"], &["max", "pattern"], &["bind"])?;
        for f in ["only", "strict", "lowercase"] {
            config.insert(f.into(), json!(flag(cx, node, f, false)));
        }
        open(cx, node, "ui-tags-input", config, true)?;
        let mut list = Element::new("span");
        list.attr("data-tags-list", true).attr("role", "list");
        cx.open(&list, node)?;
        if let Json::Array(tags) = cx.lookup(&[key]) {
            for t in tags {
                let mut s = Element::new("span");
                s.attr("data-tag", true).attr("role", "listitem").text(crate::lit(expr::stringify(&t)));
                cx.open(&s, node)?;
                cx.close("span");
            }
        }
        cx.close("span");
        let mut i = field(cx, node, "tags", "text");
        i.attr("role", "combobox").attr("aria-expanded", "false");
        cx.open(&i, node)?;
        pop(cx, node)?;
        cx.close("div");
        Ok(())
    }
}

fn pop(cx: &mut RenderCx<'_>, node: &Node) -> Result<(), RenderError> {
    let mut p = Element::new("div");
    p.attr("data-in-pop", true).attr("role", "listbox").attr("hidden", true);
    cx.open(&p, node)?;
    cx.close("div");
    Ok(())
}

// ---------------------------------------------------------------- masked

pub struct MaskedInput;

const MASKED_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key for the value").required(),
    PropDef::new(
        "mask",
        PropKind::Any,
        "The shape: `#` a digit, `A` a letter (made upper case), `a` a letter, `*` a letter or digit, `\\\\` before one taken literally; anything else is put in by itself. \
         A list of masks uses the first long enough. Also `card`, `iban`, `uuid` and `ipv4`",
    )
    .required(),
    PropDef::new("raw", PropKind::Bool, "State gets only what was typed, without the mask's own characters"),
    PropDef::new("bindComplete", PropKind::StateKey, "State key that gets true once the mask is filled"),
    PropDef::new("placeholder", PropKind::Any, "Shown while it's empty (default: the mask with `_`)"),
    PropDef::new("label", PropKind::Any, "Accessible name, when no visible label names it"),
    PropDef::new("name", PropKind::String, "Form field name"),
    PropDef::new("disabled", PropKind::Any, "Boolean or expression"),
    PropDef::new("invalid", PropKind::Any, "Mark it wrong"),
];

impl Component for MaskedInput {
    fn name(&self) -> &'static str {
        "ui-masked-input"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "Text in a fixed shape, like a card number, a postcode or a policy number, put in shape as it's typed.", props: MASKED_PROPS, parts: &["input"], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        need_bind(cx, node)?;
        match cx.prop(node, "mask") {
            Some(Json::String(s)) if !s.is_empty() => {}
            Some(Json::Array(a)) if !a.is_empty() && a.iter().all(|m| m.as_str().is_some_and(|s| !s.is_empty())) => {}
            _ => return Err(RenderError::new(node, "`mask` must be a text or a list of texts")),
        }
        let mut config = widget::config(cx, node, &["mask", "placeholder"], &[], &["bind", "bindComplete"])?;
        config.insert("raw".into(), json!(flag(cx, node, "raw", false)));
        open(cx, node, "ui-masked-input", config, false)?;
        let i = field(cx, node, "masked", "text");
        cx.open(&i, node)?;
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- phone

pub struct PhoneInput;

const PHONE_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key for the number, with its country code: `+919876543210` (null when empty)").required(),
    PropDef::new("country", PropKind::String, "The country to start with, e.g. `IN` (default: from `locale`)"),
    PropDef::new("countries", PropKind::Any, "Only these countries, e.g. `[IN, AE, GB]`"),
    PropDef::new("bindCountry", PropKind::StateKey, "State key that gets the country, e.g. `IN`"),
    PropDef::new("bindValid", PropKind::StateKey, "State key that gets true while it's a number of a length the country uses"),
    PropDef::new("placeholder", PropKind::Any, "Shown while it's empty (default: a number of the country's)"),
    PropDef::new("label", PropKind::Any, "Accessible name, when no visible label names it"),
    PropDef::new("name", PropKind::String, "Form field name"),
    PropDef::new("disabled", PropKind::Any, "Boolean or expression"),
    PropDef::new("invalid", PropKind::Any, "Mark it wrong"),
    PropDef::new("locale", PropKind::String, "The countries' names (default: the document's `lang`, else the browser's)"),
];

impl Component for PhoneInput {
    fn name(&self) -> &'static str {
        "ui-phone-input"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "A phone number with its country, picked from a searched list or from a pasted `+…` number, grouped as it's typed.", props: PHONE_PROPS, parts: &["input"], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        need_bind(cx, node)?;
        let config = widget::config(cx, node, &["placeholder"], &["country", "countries"], &["bind", "bindCountry", "bindValid"])?;
        open(cx, node, "ui-phone-input", config, false)?;
        let mut b = Element::new("button");
        b.attr("type", "button").attr("data-country", true).attr("aria-haspopup", "listbox").attr("aria-expanded", "false").attr("aria-label", cx.label("Country"));
        if let Some(d) = node.prop("disabled") {
            b.attr("disabled", d.clone());
        }
        cx.open(&b, node)?;
        cx.close("button");
        let mut i = field(cx, node, "phone", "tel");
        i.attr("autocomplete", "tel").attr("inputmode", "tel");
        cx.open(&i, node)?;
        pop(cx, node)?;
        cx.close("div");
        Ok(())
    }
}

// ---------------------------------------------------------------- time

pub struct TimePicker;

const TIME_PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key for the time, `HH:MM` (or `HH:MM:SS`; null when empty)").required(),
    PropDef::new("step", PropKind::Number, "Minutes between the times listed (default 30)"),
    PropDef::new("min", PropKind::Any, "The first time, e.g. `09:00`; may use state"),
    PropDef::new("max", PropKind::Any, "The last time, e.g. `17:30`"),
    PropDef::new("disabledTimes", PropKind::Any, "Times that can't be picked: `[\"12:00\", { from: \"13:00\", to: \"14:00\" }]`; may use state"),
    PropDef::new("seconds", PropKind::Bool, "Times have seconds"),
    PropDef::new("hour12", PropKind::Bool, "12-hour with am and pm, or 24-hour (default: the locale's way)"),
    PropDef::new("durationFrom", PropKind::Any, "Shows how long each time is after this one, e.g. `{{ start }}`"),
    PropDef::new("placeholder", PropKind::Any, "Shown while it's empty"),
    PropDef::new("label", PropKind::Any, "Accessible name, when no visible label names it"),
    PropDef::new("name", PropKind::String, "Form field name"),
    PropDef::new("disabled", PropKind::Any, "Boolean or expression"),
    PropDef::new("invalid", PropKind::Any, "Mark it wrong"),
    PropDef::new("locale", PropKind::String, "How times are written (default: the document's `lang`, else the browser's)"),
];

impl Component for TimePicker {
    fn name(&self) -> &'static str {
        "ui-time-picker"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "A time of day, typed any way (`9`, `930`, `9:30pm`, `21.30`) or picked from a list.", props: TIME_PROPS, parts: &["input"], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        need_bind(cx, node)?;
        let step = widget::number(cx, node, "step").unwrap_or(30.0);
        if !(1.0..=720.0).contains(&step) {
            return Err(RenderError::new(node, "`step` must be from 1 to 720 minutes"));
        }
        let mut config = widget::config(cx, node, &["min", "max", "disabledTimes", "durationFrom"], &["hour12"], &["bind"])?;
        config.insert("step".into(), json!(step));
        config.insert("seconds".into(), json!(flag(cx, node, "seconds", false)));
        open(cx, node, "ui-time-picker", config, false)?;
        let mut i = field(cx, node, "time", "text");
        i.attr("role", "combobox").attr("aria-expanded", "false").attr("aria-haspopup", "listbox");
        cx.open(&i, node)?;
        let mut b = Element::new("button");
        b.attr("type", "button").attr("data-in-tool", true).attr("tabindex", "-1").attr("aria-label", cx.label("Show times"));
        cx.open(&b, node)?;
        cx.close("button");
        pop(cx, node)?;
        cx.close("div");
        Ok(())
    }
}
