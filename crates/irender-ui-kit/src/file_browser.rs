//! `ui-file-browser`: folders and files to browse, one folder at a time, as a grid or a
//! list: a breadcrumb back up, search, sorting, selecting several, renaming, new folders,
//! deleting (with undo) and moving, by dragging onto a folder or the breadcrumb, by touch
//! too, or from a Move to menu. The files are a flat list in state, each with its folder
//! as `parent`; `file_browser.js` draws them and writes every change back.

use crate::{MUTED, PRIMARY, RING, SUBTLE, TEXT, forms, instance_id, lit, widget};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("file_browser.js");

const PROPS: &[PropDef] = &[
    PropDef::new(
        "bind",
        PropKind::StateKey,
        "State key holding the files and folders, a flat list: `{ id, name, parent, folder, size, modified, thumb, url }`; `parent` is the folder's `id` (none: the top), `folder: true` for a folder",
    )
    .required(),
    PropDef::new("bindFolder", PropKind::StateKey, "State key holding the open folder's `id` (null: the top), both ways"),
    PropDef::new("url", PropKind::String, "Load a folder's contents when it's first opened: `GET <url>?folder=<id>` (no `folder` for the top) answering a list of items; they're added to `bind`"),
    PropDef::new("uploadUrl", PropKind::String, "Files dropped from the desktop go up here, one `POST` each (form data `file` and `folder`), into the folder they're dropped on; an answer of an item takes the file's place"),
    PropDef::new("bindSelection", PropKind::StateKey, "State key that gets the `id`s selected"),
    PropDef::new("bindOpen", PropKind::StateKey, "State key that gets the file opened (a double click, or Enter); `open` fires too"),
    PropDef::new("view", PropKind::Enum(&["grid", "list"]), "How the folder shows to start with (default `grid`); a button switches"),
    PropDef::new("editable", PropKind::Bool, "New folders, renaming, deleting and moving (default true)"),
    PropDef::new("rootLabel", PropKind::Any, "The top folder's name in the breadcrumb (default `Files`)"),
    PropDef::new("height", PropKind::Any, "The files' area's height: px or a CSS length (default: as tall as they are)"),
    PropDef::new("locale", PropKind::String, "How dates and sizes are written (default: the document's `lang`)"),
    PropDef::new("label", PropKind::Any, "Names it for screen readers (default `Files`)"),
    PropDef::new("id", PropKind::Any, "Element id (default: derived from the document)"),
];

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

const BORDER: &str = "var(--border-width, 1px) solid var(--color-border)";
const TINT: &str = "color-mix(in srgb, var(--color-primary) 12%, var(--ui-bg, var(--color-surface, #fff)))";

static BROWSER: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[
        ("display", "flex"),
        ("flex-direction", "column"),
        ("border", BORDER),
        ("border-radius", "var(--radius-card, var(--radius-lg))"),
        ("--ui-bg", "var(--color-card, var(--color-surface))"),
        ("background", "var(--ui-bg)"),
        ("color", TEXT),
        ("font-size", fs!(14)),
        ("container-type", "inline-size"),
    ])
    // ---- the bar: breadcrumb, search, buttons
    .nest("& [data-fm-bar]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("align-items", "center"), ("gap", gap!(8)), ("padding", pad!(10, 12)), ("border-bottom", BORDER)]))
    .nest("& [data-fm-crumbs]", pairs(&[("display", "flex"), ("flex-wrap", "wrap"), ("align-items", "center"), ("gap", gap!(2)), ("flex", "1"), ("min-width", "0"), ("margin", "0"), ("padding", "0"), ("list-style", "none")]))
    .nest("& [data-fm-crumbs] li + li::before", pairs(&[("content", "'›'"), ("margin", "0 4px"), ("color", MUTED)]))
    .nest(
        "& [data-fm-crumbs] button",
        pairs(&[("padding", pad!(3, 6)), ("border", "0"), ("border-radius", "var(--radius-sm)"), ("background", "transparent"), ("color", MUTED), ("font", "inherit"), ("cursor", "pointer")]),
    )
    .nest("& [data-fm-crumbs] button[aria-current]", pairs(&[("color", TEXT), ("font-weight", "var(--font-weight-semibold, 600)")]))
    .nest("& [data-fm-crumbs] button:hover", pairs(&[("background", SUBTLE)]))
    .nest(
        "& :is([data-fm-bar], [data-fm-undo]) > :is(button, input, select)",
        pairs(&[("height", ctl!(32)), ("border", BORDER), ("border-radius", "var(--radius-control, var(--radius-md))"), ("background", "transparent"), ("color", "inherit"), ("font", "inherit")]),
    )
    .nest("& [data-fm-bar] > button, & [data-fm-undo] button", pairs(&[("display", "inline-flex"), ("align-items", "center"), ("gap", gap!(6)), ("padding", pad!(0, 10)), ("cursor", "pointer")]))
    .nest("& [data-fm-bar] > button:hover, & [data-fm-undo] button:hover", pairs(&[("background", SUBTLE)]))
    .nest("& [data-fm-bar] > button:disabled", pairs(&[("opacity", "0.45"), ("cursor", "default"), ("background", "transparent")]))
    .nest("& [data-fm-bar] > button[aria-pressed=true]", pairs(&[("background", TINT), ("border-color", PRIMARY)]))
    .nest("& [data-fm-bar] input", pairs(&[("width", sp!(160)), ("padding", pad!(0, 10))]))
    .nest("& [data-fm-bar] select", pairs(&[("padding", pad!(0, 6))]))
    .nest("& :is(button, input, select):focus-visible", forms::focus_ring())
    .nest("& [data-fm-crumbs] li[data-drop] button", pairs(&[("background", TINT), ("outline", "2px dashed var(--color-primary)")]))
    // ---- the files
    .nest("& [data-fm-area]", pairs(&[("position", "relative"), ("overflow", "auto"), ("padding", pad!(12)), ("min-height", sp!(160))]))
    .nest("& [data-fm-files]", pairs(&[("outline", "none")]))
    .nest("& [data-fm-files][data-view=grid]", pairs(&[("display", "grid"), ("grid-template-columns", "repeat(auto-fill, minmax(112px, 1fr))"), ("gap", gap!(8))]))
    .nest("& [data-fm-files][data-view=list]", pairs(&[("display", "flex"), ("flex-direction", "column")]))
    .nest(
        "& [data-fm-item]",
        pairs(&[("position", "relative"), ("border", "var(--border-width, 1px) solid transparent"), ("border-radius", "var(--radius-control, var(--radius-md))"), ("cursor", "default"), ("user-select", "none")]),
    )
    .nest("& [data-fm-item]:hover", pairs(&[("background", SUBTLE)]))
    .nest("& [data-fm-item][aria-selected=true]", pairs(&[("background", TINT), ("border-color", "color-mix(in srgb, var(--color-primary) 40%, transparent)")]))
    .nest("& [data-fm-item][data-focus]", pairs(&[("box-shadow", &format!("0 0 0 2px {RING}"))]))
    .nest("& [data-fm-files]:not(:focus-within) [data-fm-item][data-focus]", pairs(&[("box-shadow", "none")]))
    .nest("& [data-fm-item][data-drop]", pairs(&[("background", TINT), ("outline", "2px dashed var(--color-primary)"), ("outline-offset", "-2px")]))
    // Files from the desktop over the open folder, and the ones on their way up.
    .nest("& [data-fm-area][data-drop]", pairs(&[("background", TINT), ("outline", "2px dashed var(--color-primary)"), ("outline-offset", "-4px")]))
    .nest("& [data-fm-item][data-fm-up=going]", pairs(&[("opacity", "0.6")]))
    .nest("& [data-fm-item][data-fm-up=failed] [data-fm-size]", pairs(&[("color", "var(--color-destructive, #dc2626)")]))
    .nest("& [data-fm-item][data-cut]", pairs(&[("opacity", "0.45")]))
    .nest("& [data-view=grid] [data-fm-item]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("align-items", "center"), ("gap", gap!(6)), ("padding", pad!(10, 6)), ("text-align", "center")]))
    .nest("& [data-view=grid] [data-fm-icon]", pairs(&[("width", "56px"), ("height", "56px")]))
    .nest("& [data-view=grid] [data-fm-thumb]", pairs(&[("width", "100%"), ("height", "72px"), ("object-fit", "cover"), ("border-radius", "var(--radius-sm)")]))
    .nest(
        "& [data-view=grid] [data-fm-name]",
        pairs(&[("max-width", "100%"), ("overflow", "hidden"), ("display", "-webkit-box"), ("-webkit-line-clamp", "2"), ("-webkit-box-orient", "vertical"), ("overflow-wrap", "anywhere"), ("font-size", fs!(13))]),
    )
    .nest("& [data-view=grid] [data-fm-meta]", pairs(&[("display", "none")]))
    .nest(
        "& [data-view=list] [data-fm-item]",
        pairs(&[("display", "grid"), ("grid-template-columns", "28px minmax(0, 1fr) 190px 90px"), ("align-items", "center"), ("gap", gap!(10)), ("padding", pad!(6, 8))]),
    )
    .nest("@container (max-width: 520px)", pairs(&[]).nest("& [data-view=list] [data-fm-item]", pairs(&[("grid-template-columns", "28px minmax(0, 1fr) 70px")])).nest("& [data-view=list] [data-fm-date]", pairs(&[("display", "none")])))
    .nest("& [data-view=list] [data-fm-icon]", pairs(&[("width", "24px"), ("height", "24px")]))
    .nest("& [data-view=list] [data-fm-thumb]", pairs(&[("width", "24px"), ("height", "24px"), ("object-fit", "cover"), ("border-radius", "4px")]))
    .nest("& [data-view=list] [data-fm-name]", pairs(&[("overflow", "hidden"), ("text-overflow", "ellipsis"), ("white-space", "nowrap")]))
    .nest("& [data-fm-meta]", pairs(&[("color", MUTED), ("font-size", fs!(13)), ("font-variant-numeric", "tabular-nums")]))
    .nest("& [data-fm-size]", pairs(&[("text-align", "end")]))
    .nest("& [data-fm-head]", pairs(&[("display", "grid"), ("grid-template-columns", "28px minmax(0, 1fr) 190px 90px"), ("gap", gap!(10)), ("padding", pad!(0, 9, 6)), ("color", MUTED), ("font-size", fs!(12)), ("font-weight", "var(--font-weight-medium, 500)")]))
    .nest("@container (max-width: 520px)", pairs(&[]).nest("& [data-fm-head]", pairs(&[("grid-template-columns", "28px minmax(0, 1fr) 70px")])).nest("& [data-fm-head] [data-fm-date]", pairs(&[("display", "none")])))
    .nest("& [data-fm-head] span:last-child", pairs(&[("text-align", "end")]))
    .nest("& [data-fm-icon]", pairs(&[("flex", "none")]))
    .nest(
        "& [data-fm-rename]",
        pairs(&[("width", "100%"), ("min-width", "0"), ("box-sizing", "border-box"), ("padding", pad!(2, 6)), ("border", &format!("1px solid {RING}")), ("border-radius", "var(--radius-sm)"), ("background", "var(--ui-bg)"), ("color", TEXT), ("font", "inherit"), ("text-align", "inherit")]),
    )
    .nest("& [data-fm-empty]", pairs(&[("margin", "0"), ("padding", pad!(32)), ("color", MUTED), ("text-align", "center"), ("grid-column", "1 / -1")]))
    // ---- the foot: counts, the undo bar, the Move to menu
    .nest("& [data-fm-foot]", pairs(&[("display", "flex"), ("align-items", "center"), ("justify-content", "space-between"), ("gap", gap!(8)), ("padding", pad!(8, 12)), ("border-top", BORDER), ("color", MUTED), ("font-size", fs!(13))]))
    .nest("& [data-fm-undo]", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", gap!(8)), ("color", TEXT)]))
    .nest("& [data-fm-undo][hidden]", pairs(&[("display", "none")]))
    .nest(
        "& [data-fm-menu]",
        pairs(&[
            ("inset", "auto"),
            ("min-width", sp!(200)),
            ("max-height", sp!(300)),
            ("overflow", "auto"),
            ("margin", "0"),
            ("padding", pad!(4)),
            ("border", BORDER),
            ("border-radius", "var(--radius-card, var(--radius-md))"),
            ("background", "var(--color-popover, var(--ui-bg))"),
            ("color", TEXT),
            ("box-shadow", "var(--elevation-popover, 0 6px 18px rgb(0 0 0 / 0.12))"),
        ]),
    )
    .nest(
        "& [data-fm-menu] button",
        pairs(&[("display", "block"), ("width", "100%"), ("padding", pad!(6, 10)), ("border", "0"), ("border-radius", "var(--radius-sm)"), ("background", "transparent"), ("color", "inherit"), ("font", "inherit"), ("text-align", "start"), ("cursor", "pointer")]),
    )
    .nest("& [data-fm-menu] button:is(:hover, :focus-visible)", pairs(&[("background", SUBTLE), ("outline", "none")]))
    .nest("& [data-fm-menu] button:disabled", pairs(&[("opacity", "0.4"), ("cursor", "default")]))
    .nest(
        "& [data-fm-live]",
        pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("overflow", "hidden"), ("clip-path", "inset(50%)"), ("white-space", "nowrap")]),
    )
});

pub struct FileBrowser;

impl Component for FileBrowser {
    fn name(&self) -> &'static str {
        "ui-file-browser"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Folders and files to browse one folder at a time, as a grid or a list: search, sort, select, rename, new folders, delete with undo, and move by dragging or a menu.",
            props: PROPS,
            parts: &[],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let key = cx.prop_string(node, "bind").filter(|k| !k.is_empty()).ok_or_else(|| RenderError::new(node, "`bind` is the state key holding the files"))?;
        let files = match cx.lookup(std::slice::from_ref(&key)) {
            Json::Array(a) => a,
            Json::Null => Vec::new(),
            _ => return Err(RenderError::new(node, format!("`{key}` must hold a list of files and folders"))),
        };
        let mut seen = std::collections::HashSet::new();
        for (i, f) in files.iter().enumerate() {
            let id = f.get("id").filter(|v| !v.is_null()).map(expr::stringify);
            let Some(id) = id else { return Err(RenderError::new(node, format!("item {} needs an `id` and a `name`", i + 1))) };
            if f.get("name").is_none_or(Json::is_null) {
                return Err(RenderError::new(node, format!("item {} needs an `id` and a `name`", i + 1)));
            }
            if !seen.insert(id.clone()) {
                return Err(RenderError::new(node, format!("two items have the `id` `{id}`")));
            }
        }
        let height = match cx.prop(node, "height").filter(|v| !v.is_null()) {
            None => None,
            Some(Json::Number(n)) => Some(format!("{n}px")),
            Some(v) => Some(expr::stringify(&v)),
        };
        let mut config = widget::config(cx, node, &[], &["view", "url", "uploadUrl"], &["bind", "bindFolder", "bindSelection", "bindOpen"])?;
        config.insert("editable".into(), json!(widget::flag(cx, node, "editable", true)));
        config.insert("rootLabel".into(), json!(cx.prop(node, "rootLabel").filter(|v| !v.is_null()).map_or_else(|| cx.label("Files"), |v| expr::stringify(&v))));
        config.insert("locale".into(), json!(crate::locale(cx, node)));
        let id = instance_id(cx, node, "file-browser");
        config.insert("id".into(), json!(expr::stringify(&cx.eval(&id))));
        widget::kit(cx);
        crate::touch_drag(cx);
        crate::anchors(cx);
        cx.global_js("ui-file-browser", SCRIPT);

        let mut root = cx.element("div", node);
        root.base_style_ref(&BROWSER)
            .behavior("ui-file-browser", Json::Object(config))
            .attr("id", id)
            .attr("role", "region")
            .attr("aria-label", node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Files"))));
        cx.open(&root, node)?;
        // The behaviour draws the bar, the files and the foot; the area keeps its height.
        for part in ["bar", "area", "foot"] {
            let mut d = Element::new("div");
            d.attr(format!("data-fm-{part}").as_str(), true);
            if part == "area"
                && let Some(h) = &height
            {
                d.base_style(StyleBlock::new().set("height", h.clone()));
            }
            cx.open(&d, node)?;
            cx.close("div");
        }
        let mut live = Element::new("div");
        live.attr("data-fm-live", true).attr("role", "status").attr("aria-live", "polite");
        cx.open(&live, node)?;
        cx.close("div");
        cx.close("div");
        Ok(())
    }
}
