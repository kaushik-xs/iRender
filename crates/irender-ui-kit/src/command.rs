//! The command palette, in the style of shadcn/ui's Command (cmdk): a search box over
//! a list of actions. `ui-command` shows it in the page; `ui-command-dialog` opens it
//! as a modal, from a button or a keyboard shortcut.
//!
//! The server renders every item, so each keeps its own `on.click` actions;
//! `command.js` filters them as you type and moves the active one with the keyboard,
//! while focus stays in the search box (the WAI-ARIA combobox pattern).

use crate::{BG_DIALOG, BG_POPOVER, ACCENT, BORDER, MUTED, TEXT, context, icon, id_with, instance_id, lit, overlay};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, Val, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("command.js");

/// A magnifying glass for the search box, drawn like the icon kit's outline icons.
const SEARCH_ICON: &str = r#"<svg data-ui-search-icon width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/></svg>"#;

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

const SEARCH_PROPS: [PropDef; 6] = [
    PropDef::new("placeholder", PropKind::Any, "The search box's placeholder (default `Type a command or search…`)"),
    PropDef::new("label", PropKind::Any, "The search box's accessible name (default: the placeholder)"),
    PropDef::new("empty", PropKind::Any, "Shown when nothing matches (default `No results found.`)"),
    PropDef::new("id", PropKind::Any, "Element id (default: derived from the document)"),
    PropDef::new("rank", PropKind::Bool, "Put the best matches first, in each group and of the groups (default true); false keeps the items' order"),
    PropDef::new("bindSearch", PropKind::StateKey, "State key that holds the search text, both ways: to fetch items for it, say"),
];
const COMMAND_PROPS: &[PropDef] = &SEARCH_PROPS;
const DIALOG_PROPS: &[PropDef] = &[
    SEARCH_PROPS[0],
    SEARCH_PROPS[1],
    SEARCH_PROPS[2],
    SEARCH_PROPS[3],
    SEARCH_PROPS[4],
    SEARCH_PROPS[5],
    PropDef::new("trigger", PropKind::Any, "Text of a button that opens it"),
    PropDef::new("triggerVariant", PropKind::Enum(&["default", "secondary", "destructive", "outline", "ghost", "link", "brand"]), "The trigger button's variant (default `outline`)"),
    PropDef::new("shortcut", PropKind::String, "Keys that open and close it from anywhere on the page, e.g. `mod+k` (⌘K on Apple devices, Ctrl+K elsewhere)"),
    PropDef::new("title", PropKind::Any, "Its accessible name (default `Command palette`)"),
];

/// The letters that matched what was typed, marked by the behaviour with the CSS Custom
/// Highlight API (the label's text stays as the runtime wrote it).
const MATCH_CSS: &str = "::highlight(ui-command-match) {\n  background-color: color-mix(in srgb, var(--color-primary) 16%, transparent);\n  color: inherit;\n}";

/// Styled from the root, like the combobox: the behaviour shows and hides the parts.
fn palette_style(framed: bool) -> StyleBlock {
    let mut root = pairs(&[
        ("box-sizing", "border-box"),
        ("display", "flex"),
        ("flex-direction", "column"),
        ("width", "100%"),
        ("overflow", "hidden"),
        ("--ui-bg", BG_POPOVER),
        ("background", BG_POPOVER),
        ("color", TEXT),
        ("font-size", fs!(14)),
    ]);
    if framed {
        root = root.set("border", "1px solid var(--color-border)").set("border-radius", "var(--radius-lg)");
    }
    root.nest(
            "& [data-ui-search]",
            pairs(&[("display", "flex"), ("align-items", "center"), ("gap", sp!(8)), ("padding", sp!(0, 12)), ("border-bottom", "1px solid var(--color-border)")]),
        )
        .nest("& [data-ui-search-icon]", pairs(&[("flex-shrink", "0"), ("color", MUTED)]))
        .nest(
            "& [data-ui-command-input]",
            pairs(&[
                ("flex", "1"),
                ("min-width", "0"),
                ("height", sp!(44)),
                ("margin", "0"),
                ("padding", "0"),
                ("border", "0"),
                ("outline", "none"),
                ("background", "transparent"),
                ("color", "inherit"),
                ("font", "inherit"),
            ]),
        )
        // The open page's crumb: its title, a click (or Backspace) going back.
        .nest(
            "& [data-ui-crumb]",
            pairs(&[("flex", "none"), ("padding", sp!(2, 8)), ("border", "0"), ("border-radius", "var(--radius-sm)"), ("background", ACCENT), ("color", TEXT), ("font", "inherit"), ("font-size", fs!(12)), ("font-weight", "500"), ("cursor", "pointer"), ("white-space", "nowrap")]),
        )
        .nest("& [data-ui-crumb]::before", pairs(&[("content", "'‹ '"), ("color", MUTED)]))
        .nest("& [data-ui-command-input]::placeholder", pairs(&[("color", MUTED)]))
        .nest("& [role=listbox]", pairs(&[("position", "relative"), ("max-height", "300px"), ("overflow-y", "auto"), ("overscroll-behavior", "contain"), ("padding", sp!(4))]))
        .nest("& [data-ui-heading]", pairs(&[("padding", sp!(6, 8)), ("font-size", fs!(12)), ("font-weight", "500"), ("color", MUTED)]))
        .nest(
            "& [role=option]",
            pairs(&[
                ("box-sizing", "border-box"),
                ("display", "flex"),
                ("align-items", "center"),
                ("gap", sp!(8)),
                ("width", "100%"),
                ("padding", sp!(6, 8)),
                ("border", "0"),
                ("border-radius", "var(--radius-sm)"),
                ("background", "transparent"),
                ("color", TEXT),
                ("font", "inherit"),
                ("text-align", "start"),
                ("text-decoration", "none"),
                ("cursor", "default"),
                ("user-select", "none"),
            ]),
        )
        .nest("& [role=option][aria-selected=true]", pairs(&[("background", ACCENT)]))
        .nest("& [role=option]:disabled", pairs(&[("opacity", "0.5")]))
        .nest(
            "& [data-ui-shortcut]",
            pairs(&[("margin-inline-start", "auto"), ("padding-inline-start", sp!(16)), ("font-size", fs!(12)), ("letter-spacing", "0.1em"), ("color", MUTED)]),
        )
        .nest("& [role=separator]", pairs(&[("height", "1px"), ("margin", sp!(4, -4)), ("background", BORDER)]))
        .nest("& [data-ui-empty]", pairs(&[("padding", sp!(24, 8)), ("text-align", "center"), ("color", MUTED)]))
        .nest(
            "& [data-ui-live]",
            pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("overflow", "hidden"), ("clip-path", "inset(50%)"), ("white-space", "nowrap")]),
        )
        // Columns, so the behaviour can put the best matches first with `order`.
        .nest("& :is([role=listbox], [role=group], :has(> [role=option]))", pairs(&[("display", "flex"), ("flex-direction", "column")]))
        .nest("& [data-ui-heading]", pairs(&[("order", "-1")]))
        // Last, so it beats the `display` of items and the rest at the same specificity.
        .nest("& [hidden]", pairs(&[("display", "none")]))
}

static FRAMED: LazyLock<StyleBlock> = LazyLock::new(|| palette_style(true));
static BARE: LazyLock<StyleBlock> = LazyLock::new(|| palette_style(false));
static DIALOG: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("box-sizing", "border-box"),
        ("width", "min(512px, calc(100% - 32px))"),
        // Near the top, as palettes open, so the list grows down.
        ("margin", "12vh auto auto"),
        ("padding", "0"),
        ("border", "1px solid var(--color-border)"),
        ("border-radius", "var(--radius-lg)"),
        ("--ui-bg", BG_DIALOG),
        ("background", BG_DIALOG),
        ("color", TEXT),
        ("box-shadow", "0 10px 30px rgb(0 0 0 / 0.2)"),
        ("overflow", "hidden"),
    ])
    .nest("::backdrop", pairs(&[("background", "rgb(0 0 0 / 0.5)")]))
});

/// The search box, the list with the node's children, and the empty message, inside
/// `el`, which runs the behaviour. The caller closes `el` (a `div`).
fn palette(cx: &mut RenderCx<'_>, node: &Node, mut el: Element<'_>, id: &Val, shortcut: Option<String>, dialog: bool) -> Result<(), RenderError> {
    cx.global_js("ui-command", SCRIPT);
    let rank = cx.prop(node, "rank").is_none_or(|v| irender_core::expr::truthy(&v));
    el.attr("data-ui-command", true).behavior("ui-command", json!({ "shortcut": shortcut, "rank": rank, "bindSearch": cx.prop_string(node, "bindSearch") }));
    cx.global_css("ui-command-match", MATCH_CSS);
    cx.open(&el, node)?;

    let placeholder = node.prop("placeholder").cloned().unwrap_or_else(|| lit(cx.label("Type a command or search…")));
    let list = id_with(id, "-list");
    let mut search = cx.part("div", node, "search");
    search.attr("data-ui-search", true);
    cx.open(&search, node)?;
    cx.markup(SEARCH_ICON);
    // The page shown (an item's `page`), and the way back from it.
    let mut crumb = Element::new("button");
    crumb.attr("type", "button").attr("data-ui-crumb", true).attr("hidden", true);
    cx.open(&crumb, node)?;
    cx.close("button");
    let mut input = cx.part("input", node, "input");
    input
        .attr("data-ui-command-input", true)
        .attr("type", "text")
        .attr("role", "combobox")
        .attr("aria-expanded", "true")
        .attr("aria-controls", list.clone())
        .attr("aria-autocomplete", "list")
        .attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| placeholder.clone()))
        .attr("autocomplete", "off")
        .attr("spellcheck", "false")
        .attr("placeholder", placeholder);
    if dialog {
        // Opening the dialog focuses it; a palette in the page leaves focus alone.
        input.attr("autofocus", true);
    }
    cx.open(&input, node)?;
    cx.close("div");

    let mut listbox = cx.part("div", node, "list");
    listbox.attr("role", "listbox").attr("id", list);
    cx.open(&listbox, node)?;
    cx.with_context("ui-command", Some(id.clone()), |cx| cx.children(node))?;
    cx.close("div");

    let mut empty = cx.part("div", node, "empty");
    empty.attr("data-ui-empty", true).attr("hidden", true).text(node.prop("empty").cloned().unwrap_or_else(|| lit(cx.label("No results found."))));
    cx.open(&empty, node)?;
    cx.close("div");
    let mut live = Element::new("div");
    live.attr("data-ui-live", true).attr("role", "status").attr("aria-live", "polite");
    cx.open(&live, node)?;
    cx.close("div");
    Ok(())
}

pub struct Command;

impl Component for Command {
    fn name(&self) -> &'static str {
        "ui-command"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A search box over a list of actions, filtered as you type; Up and Down move, Enter runs the active \
                  one. Children are `ui-command-group`, `ui-command-item` and `ui-command-separator`.",
            props: COMMAND_PROPS,
            parts: &["search", "input", "list", "empty"],
            children: true,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let id = instance_id(cx, node, "command");
        let mut el = cx.element("div", node);
        el.base_style_ref(&FRAMED).attr("id", id.clone());
        palette(cx, node, el, &id, None, false)?;
        cx.close("div");
        Ok(())
    }
}

pub struct CommandDialog;

impl Component for CommandDialog {
    fn name(&self) -> &'static str {
        "ui-command-dialog"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A `ui-command` in a modal, opened by its trigger, a `shortcut`, or a button with `commandfor` its id. \
                  Running an item closes it.",
            props: DIALOG_PROPS,
            parts: &["trigger", "search", "input", "list", "empty"],
            children: true,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let id = instance_id(cx, node, "command");
        overlay::trigger_button(
            node,
            cx,
            vec![("command", lit("show-modal")), ("commandfor", id.clone()), ("aria-haspopup", lit("dialog"))],
            StyleBlock::new(),
        )?;
        let mut dialog = cx.element("dialog", node);
        dialog
            .base_style_ref(&DIALOG)
            .attr("id", id.clone())
            .attr("closedby", "any")
            .attr("aria-label", node.prop("title").cloned().unwrap_or_else(|| lit(cx.label("Command palette"))));
        cx.open(&dialog, node)?;
        let mut el = Element::new("div");
        el.base_style_ref(&BARE);
        palette(cx, node, el, &id, cx.prop_string(node, "shortcut"), true)?;
        cx.close("div");
        cx.close("dialog");
        Ok(())
    }
}

pub struct CommandGroup;

impl Component for CommandGroup {
    fn name(&self) -> &'static str {
        "ui-command-group"
    }
    fn info(&self) -> ComponentInfo {
        const PROPS: &[PropDef] = &[
            PropDef::new("heading", PropKind::Any, "Its heading"),
            PropDef::new("id", PropKind::Any, "Element id (default: derived from the document)"),
        ];
        ComponentInfo {
            doc: "Items under a heading in a command palette. It hides while none of its items match.",
            props: PROPS,
            parts: &["heading"],
            children: true,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut el = cx.element("div", node);
        el.attr("role", "group");
        let heading = node.prop("heading").map(|h| (h, id_with(&instance_id(cx, node, "command-group"), "-heading")));
        if let Some((_, hid)) = &heading {
            el.attr("aria-labelledby", hid.clone());
        }
        cx.open(&el, node)?;
        if let Some((text, hid)) = heading {
            let mut h = cx.part("div", node, "heading");
            h.attr("data-ui-heading", true).attr("id", hid).text(text);
            cx.open(&h, node)?;
            cx.close("div");
        }
        cx.children(node)?;
        cx.close("div");
        Ok(())
    }
}

pub struct CommandPage;

const PAGE_PROPS: &[PropDef] = &[
    PropDef::new("id", PropKind::String, "What an item's `page` opens it by").required(),
    PropDef::new("title", PropKind::Any, "Shown in the search box while it's open (default: the id)"),
];

impl Component for CommandPage {
    fn name(&self) -> &'static str {
        "ui-command-page"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A page of a command palette: items (and groups) shown in place of the rest when an item with its `page` is chosen. \
                  Backspace in the empty search box goes back.",
            props: PAGE_PROPS,
            parts: &[],
            children: true,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        if context(cx, "ui-command").is_none() {
            return Err(RenderError::new(node, "`ui-command-page` must be inside a `ui-command` or `ui-command-dialog`"));
        }
        let id = cx.prop_string(node, "id").unwrap_or_default();
        let mut el = Element::new("div");
        el.attr("data-ui-page", id.clone()).attr("data-ui-page-title", cx.prop_string(node, "title").unwrap_or(id)).attr("hidden", true);
        cx.open(&el, node)?;
        cx.children(node)?;
        cx.close("div");
        Ok(())
    }
}

pub struct CommandItem;

const ITEM_PROPS: &[PropDef] = &[
    PropDef::new("label", PropKind::Any, "Item text, matched as you type").required(),
    PropDef::new("keywords", PropKind::Any, "More words that find it: a string or a list"),
    PropDef::new("icon", PropKind::Any, "An icon name when the icon kit is registered (`settings`), or text such as an emoji"),
    PropDef::new("shortcut", PropKind::Any, "Keyboard hint shown on the right, e.g. `⌘P`"),
    PropDef::new("href", PropKind::Any, "Make it a link"),
    PropDef::new("disabled", PropKind::Any, "Boolean or expression"),
    PropDef::new("page", PropKind::String, "Opens the `ui-command-page` with this `id`, and the palette stays open (Backspace in the empty search box goes back)"),
];

impl Component for CommandItem {
    fn name(&self) -> &'static str {
        "ui-command-item"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "An action in a command palette: clicking it, or Enter while it is active, runs its `on.click`.",
            props: ITEM_PROPS,
            parts: &["shortcut"],
            children: false,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        if context(cx, "ui-command").is_none() {
            return Err(RenderError::new(node, "`ui-command-item` must be inside a `ui-command` or `ui-command-dialog`"));
        }
        let href = node.prop("href");
        let tag = if href.is_some() { "a" } else { "button" };
        let mut el = cx.element(tag, node);
        // Never focused: focus stays in the search box, which points at the active item.
        el.attr("role", "option").attr("tabindex", "-1").attr("aria-selected", "false");
        match cx.prop(node, "keywords") {
            Some(Json::Array(words)) => {
                el.attr("data-ui-keywords", words.iter().map(expr::stringify).collect::<Vec<_>>().join(" "));
            }
            Some(Json::Null) | None => {}
            Some(words) => {
                el.attr("data-ui-keywords", expr::stringify(&words));
            }
        }
        match href {
            Some(h) => {
                el.attr("href", h.clone());
            }
            None => {
                el.attr("type", "button");
                if let Some(d) = node.prop("disabled") {
                    el.attr("disabled", d.clone());
                }
            }
        }
        if let Some(p) = cx.prop_string(node, "page") {
            el.attr("data-ui-open-page", p).attr("aria-haspopup", "true");
        }
        cx.open(&el, node)?;
        if let Some(i) = node.prop("icon") {
            icon(cx, node, i, 16)?;
        }
        let mut label = Element::new("span");
        label.attr("data-ui-label", true).text(node.prop("label").cloned().unwrap_or_else(|| lit("")));
        cx.open(&label, node)?;
        cx.close("span");
        if let Some(s) = node.prop("shortcut") {
            let mut k = cx.part("span", node, "shortcut");
            k.attr("data-ui-shortcut", true).text(s);
            cx.open(&k, node)?;
            cx.close("span");
        }
        cx.close(tag);
        Ok(())
    }
}

pub struct CommandSeparator;

impl Component for CommandSeparator {
    fn name(&self) -> &'static str {
        "ui-command-separator"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "A line between groups in a command palette, hidden while searching.",
            props: &[],
            parts: &[],
            children: false,
            open_props: false,
        }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let mut el = cx.element("div", node);
        el.attr("role", "separator");
        cx.open(&el, node)?;
        cx.close("div");
        Ok(())
    }
}
