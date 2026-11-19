//! `ui-file-upload`: files to attach, in a drop zone, as a grid or a list, kept in state
//! as `{ id, name, size, type, url, status, progress, error }`. `upload.js` checks each
//! file against `accept`, `maxSize` and `maxFiles`, sends it to `url` with its
//! progress, previews images, PDFs, spreadsheets (CSV and .xlsx) and text in a dialog,
//! and downloads them all as one zip, all without libraries.

use crate::overlay::{DIALOG_X, SHEETS};
use crate::{ACCENT, MUTED, PRIMARY, TEXT, forms, id_with, instance_id, lit};
use irender_core::{ComponentInfo, Node, PropDef, PropKind, StyleBlock, expr};
use irender_html::{Component, Element, RenderCx, RenderError};
use serde_json::{Value as Json, json};
use std::sync::LazyLock;

const SCRIPT: &str = include_str!("upload.js");

pub(crate) const VIEWS: &[&str] = &["grid", "list"];

pub struct FileUpload;

const PROPS: &[PropDef] = &[
    PropDef::new("bind", PropKind::StateKey, "State key for the files: `[{ id, name, size, type, url, status, progress, error }]`. Files already there (with a `url`) are shown too").required(),
    PropDef::new("url", PropKind::Any, "Where to upload: each file is POSTed as `multipart/form-data` (field `file`), and a JSON answer is merged into its entry, e.g. `{ id, url }`. Without it, files stay in the browser"),
    PropDef::new("params", PropKind::Any, "More form fields sent with each file, e.g. `{ folder: \"{{ claim.id }}\" }`"),
    PropDef::new("accept", PropKind::String, "Types allowed, as the file input's `accept`: `image/*,.pdf,.xlsx`"),
    PropDef::new("multiple", PropKind::Bool, "More than one file (default true)"),
    PropDef::new("maxFiles", PropKind::Number, "Most files in all"),
    PropDef::new("maxSize", PropKind::Any, "Largest file: bytes, or text like `10 MB` or `500 KB`"),
    PropDef::new("view", PropKind::Enum(VIEWS), "`grid` of cards (default) or `list` of rows"),
    PropDef::new("label", PropKind::Any, "The drop zone's text (default `Drop files here or click to browse`)"),
    PropDef::new("hint", PropKind::Any, "Smaller text under it, e.g. the types and size allowed"),
    PropDef::new("readOnly", PropKind::Bool, "Show the files only: no drop zone and no removing"),
    PropDef::new("removable", PropKind::Bool, "A remove button on each file (default true)"),
    PropDef::new("download", PropKind::Bool, "A Download all button that saves the files as one zip (default true)"),
    PropDef::new("zipName", PropKind::String, "The zip's name, without `.zip` (default `attachments`)"),
    PropDef::new("emptyText", PropKind::Any, "Shown under a read-only list with no files (default `No files yet.`)"),
    PropDef::new("disabled", PropKind::Any, "Boolean or expression"),
    PropDef::new("id", PropKind::Any, "The file input's id, for a label (default: derived from the document)"),
];

fn pairs(p: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(p)
}

/// Bytes from `10 MB`, `500 KB`, `1.5 GB` or a plain number.
pub(crate) fn bytes(v: &Json) -> Option<f64> {
    if let Json::Number(n) = v {
        return n.as_f64().filter(|n| *n > 0.0);
    }
    let s = expr::stringify(v).trim().to_uppercase();
    let split = s.find(|c: char| c.is_ascii_alphabetic()).unwrap_or(s.len());
    let n: f64 = s[..split].trim().parse().ok()?;
    let unit = match s[split..].trim() {
        "" | "B" => 1.0,
        "KB" => 1024.0,
        "MB" => 1024.0 * 1024.0,
        "GB" => 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    (n > 0.0).then_some(n * unit)
}

static ROOT: LazyLock<StyleBlock> = LazyLock::new(|| {
    pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", sp!(12)), ("min-width", "0"), ("color", TEXT), ("font-size", fs!(14))])
        .nest("& [hidden]", pairs(&[("display", "none")]))
        // The drop zone: a label around the file input, so a click or Enter opens the picker.
        .nest(
            "& [data-up-zone]",
            pairs(&[
                ("position", "relative"),
                ("display", "flex"),
                ("flex-direction", "column"),
                ("align-items", "center"),
                ("justify-content", "center"),
                ("gap", sp!(6)),
                ("padding", sp!(24, 16)),
                ("border", "1.5px dashed var(--color-border)"),
                ("border-radius", "var(--radius-lg)"),
                ("text-align", "center"),
                ("cursor", "pointer"),
                ("transition", "border-color .15s, background .15s"),
            ]),
        )
        .nest("& [data-up-zone]:hover, & [data-up-zone][data-over]", pairs(&[("border-color", PRIMARY), ("background", ACCENT)]))
        .nest("& [data-up-zone]:has(input:focus-visible)", forms::focus_ring())
        .nest("& [data-up-zone]:has(input:disabled)", pairs(&[("opacity", "0.5"), ("cursor", "not-allowed")]))
        .nest(
            "& [data-up-zone] input",
            pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("opacity", "0"), ("pointer-events", "none")]),
        )
        .nest("& [data-up-zone] svg", pairs(&[("color", MUTED)]))
        .nest("& [data-up-label]", pairs(&[("font-weight", "500")]))
        .nest("& [data-up-hint]", pairs(&[("color", MUTED), ("font-size", fs!(13))]))
        .nest("& [data-up-errors]", pairs(&[("margin", "0"), ("padding-inline-start", sp!(18)), ("color", "var(--color-destructive, #dc2626)"), ("font-size", fs!(13))]))
        .nest("& [data-up-bar]", pairs(&[("display", "flex"), ("align-items", "center"), ("gap", sp!(8)), ("color", MUTED), ("font-size", fs!(13))]))
        .nest("& [data-up-bar] > :first-child", pairs(&[("margin-inline-end", "auto")]))
        .nest("& [data-up-empty]", pairs(&[("margin", "0"), ("color", MUTED)]))
        .nest(
            "& [data-up-live]",
            pairs(&[("position", "absolute"), ("width", "1px"), ("height", "1px"), ("overflow", "hidden"), ("clip-path", "inset(50%)"), ("white-space", "nowrap")]),
        )
});

/// The files: cards in a grid, or rows in a list. Shared parts: an icon tinted by kind,
/// a progress bar and an error line.
static FILES: LazyLock<StyleBlock> = LazyLock::new(|| {
    let kind = |c: &str| pairs(&[("color", c)]);
    pairs(&[("margin", "0"), ("padding", "0"), ("list-style", "none")])
        .nest("&[data-view=grid]", pairs(&[("display", "grid"), ("grid-template-columns", "repeat(auto-fill, minmax(150px, 1fr))"), ("gap", sp!(12))]))
        .nest("&[data-view=list]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("border", "1px solid var(--color-border)"), ("border-radius", "var(--radius-lg)")]))
        .nest("& [data-up-file]", pairs(&[("position", "relative"), ("min-width", "0")]))
        .nest(
            "&[data-view=grid] [data-up-file]",
            pairs(&[("display", "flex"), ("flex-direction", "column"), ("border", "1px solid var(--color-border)"), ("border-radius", "var(--radius-lg)"), ("overflow", "hidden")]),
        )
        .nest(
            "&[data-view=list] [data-up-file]",
            pairs(&[("display", "flex"), ("align-items", "center"), ("gap", sp!(12)), ("padding", sp!(8, 12)), ("border-top", "1px solid var(--color-border)")]),
        )
        .nest("&[data-view=list] [data-up-file]:first-child", pairs(&[("border-top", "0")]))
        // The part that opens the preview: the thumbnail and name.
        .nest(
            "& [data-up-open]",
            pairs(&[
                ("display", "flex"),
                ("min-width", "0"),
                ("padding", "0"),
                ("border", "0"),
                ("background", "transparent"),
                ("color", "inherit"),
                ("font", "inherit"),
                ("text-align", "start"),
                ("cursor", "pointer"),
            ]),
        )
        .nest("& [data-up-open]:focus-visible", forms::focus_ring().set("outline-offset", "-2px"))
        .nest("&[data-view=grid] [data-up-open]", pairs(&[("flex-direction", "column"), ("width", "100%")]))
        .nest("&[data-view=list] [data-up-open]", pairs(&[("flex", "1 1 auto"), ("align-items", "center"), ("gap", sp!(12))]))
        .nest(
            "& [data-up-thumb]",
            pairs(&[("display", "flex"), ("align-items", "center"), ("justify-content", "center"), ("flex", "none"), ("background", "color-mix(in srgb, var(--color-text) 4%, transparent)"), ("overflow", "hidden")]),
        )
        .nest("&[data-view=grid] [data-up-thumb]", pairs(&[("height", "110px"), ("border-bottom", "1px solid var(--color-border)")]))
        .nest("&[data-view=list] [data-up-thumb]", pairs(&[("width", "36px"), ("height", "36px"), ("border-radius", "var(--radius-sm)")]))
        .nest("& [data-up-thumb] img", pairs(&[("width", "100%"), ("height", "100%"), ("object-fit", "cover")]))
        .nest("& [data-up-thumb] [data-ext]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("align-items", "center"), ("gap", "2px"), ("font-size", fs!(10)), ("font-weight", "700"), ("letter-spacing", ".04em")]))
        .nest("&[data-view=list] [data-up-thumb] [data-ext] b", pairs(&[("display", "none")]))
        .nest("& [data-kind=pdf] [data-ext]", kind("#dc2626"))
        .nest("& [data-kind=sheet] [data-ext]", kind("#16a34a"))
        .nest("& [data-kind=doc] [data-ext]", kind("#2563eb"))
        .nest("& [data-kind=image] [data-ext]", kind("#9333ea"))
        .nest("& [data-kind=archive] [data-ext]", kind("#d97706"))
        .nest("& :is([data-kind=text], [data-kind=other]) [data-ext]", kind(MUTED))
        .nest("& [data-up-meta]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", "2px"), ("min-width", "0")]))
        .nest("&[data-view=grid] [data-up-meta]", pairs(&[("padding", sp!(8, 10))]))
        .nest("& [data-up-name]", pairs(&[("overflow", "hidden"), ("text-overflow", "ellipsis"), ("white-space", "nowrap"), ("font-weight", "500")]))
        .nest("& [data-up-size]", pairs(&[("color", MUTED), ("font-size", fs!(12)), ("font-variant-numeric", "tabular-nums")]))
        .nest("& [data-up-error]", pairs(&[("color", "var(--color-destructive, #dc2626)"), ("font-size", fs!(12))]))
        .nest(
            "& progress",
            pairs(&[("appearance", "none"), ("width", "100%"), ("height", "4px"), ("border", "0"), ("border-radius", "999px"), ("background", "color-mix(in srgb, var(--color-text) 10%, transparent)"), ("overflow", "hidden")]),
        )
        .nest("& progress::-webkit-progress-bar", pairs(&[("background", "transparent")]))
        .nest("& progress::-webkit-progress-value", pairs(&[("background", PRIMARY), ("border-radius", "999px")]))
        .nest("& progress::-moz-progress-bar", pairs(&[("background", PRIMARY), ("border-radius", "999px")]))
        .nest("& [data-up-actions]", pairs(&[("display", "flex"), ("gap", sp!(2)), ("flex", "none")]))
        .nest("&[data-view=grid] [data-up-actions]", pairs(&[("position", "absolute"), ("top", sp!(6)), ("inset-inline-end", sp!(6))]))
        .nest(
            "& [data-up-actions] button",
            pairs(&[
                ("display", "inline-flex"),
                ("align-items", "center"),
                ("justify-content", "center"),
                ("width", "28px"),
                ("height", "28px"),
                ("padding", "0"),
                ("border", "0"),
                ("border-radius", "var(--radius-sm)"),
                ("background", "transparent"),
                ("color", MUTED),
                ("cursor", "pointer"),
            ]),
        )
        .nest(
            "&[data-view=grid] [data-up-actions] button",
            pairs(&[("background", "color-mix(in srgb, var(--color-surface, #fff) 85%, transparent)"), ("box-shadow", "0 1px 2px rgb(0 0 0 / 0.12)")]),
        )
        .nest("& [data-up-actions] button:hover", pairs(&[("color", TEXT), ("background", ACCENT)]))
        .nest("& [data-up-actions] button:focus-visible", forms::focus_ring())
        .nest("& [data-up-remove]:hover", pairs(&[("color", "var(--color-destructive, #dc2626)")]))
});

static PREVIEW: LazyLock<StyleBlock> = LazyLock::new(|| {
    SHEETS[0]
        .clone()
        .set("width", "min(960px, 100vw)")
        .set("padding", "0")
        .nest("&[open]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("gap", "0")]))
        .nest(
            "& [data-up-head]",
            pairs(&[("display", "flex"), ("align-items", "center"), ("gap", sp!(8)), ("padding-block", sp!(14, 14)), ("padding-inline", sp!(20, 56)), ("border-bottom", "1px solid var(--color-border)")]),
        )
        .nest("& [data-up-title]", pairs(&[("margin-block", "0 0"), ("margin-inline", "0 auto"), ("min-width", "0"), ("overflow", "hidden"), ("text-overflow", "ellipsis"), ("white-space", "nowrap"), ("font-size", fs!(16)), ("font-weight", "600")]))
        .nest("& [data-up-count]", pairs(&[("color", MUTED), ("font-size", fs!(13)), ("white-space", "nowrap")]))
        .nest(
            "& [data-up-body]",
            pairs(&[("flex", "1 1 auto"), ("min-height", "0"), ("display", "flex"), ("align-items", "center"), ("justify-content", "center"), ("overflow", "auto"), ("background", "color-mix(in srgb, var(--color-text) 3%, transparent)")]),
        )
        .nest("& [data-up-body] img", pairs(&[("max-width", "100%"), ("max-height", "100%"), ("object-fit", "contain")]))
        .nest("& [data-up-body] iframe", pairs(&[("width", "100%"), ("height", "100%"), ("border", "0"), ("background", "#fff")]))
        .nest("& [data-up-body] pre", pairs(&[("align-self", "stretch"), ("width", "100%"), ("margin", "0"), ("padding", sp!(16)), ("box-sizing", "border-box"), ("white-space", "pre-wrap"), ("font-size", fs!(13))]))
        .nest("& [data-up-body] [data-up-sheet]", pairs(&[("align-self", "stretch"), ("width", "100%"), ("overflow", "auto")]))
        .nest("& [data-up-body] table", pairs(&[("border-collapse", "collapse"), ("font-size", fs!(13)), ("background", "var(--color-surface, #fff)")]))
        .nest("& [data-up-body] :is(th, td)", pairs(&[("padding", sp!(4, 10)), ("border", "1px solid var(--color-border)"), ("white-space", "nowrap"), ("text-align", "start")]))
        .nest("& [data-up-body] th", pairs(&[("position", "sticky"), ("top", "0"), ("background", ACCENT), ("color", MUTED), ("font-weight", "500")]))
        .nest("& [data-up-body] [data-up-none]", pairs(&[("display", "flex"), ("flex-direction", "column"), ("align-items", "center"), ("gap", sp!(12)), ("color", MUTED)]))
        .nest("& [data-up-body] [data-up-note]", pairs(&[("position", "sticky"), ("inset-inline-start", "0"), ("padding", sp!(8, 12)), ("color", MUTED), ("font-size", fs!(12))]))
});

// Decorative: the zone and buttons around them are labelled.
const UPLOAD: &str = r#"<svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 15V4M7.5 8.5 12 4l4.5 4.5"/><path d="M4 15v3.5A1.5 1.5 0 0 0 5.5 20h13a1.5 1.5 0 0 0 1.5-1.5V15"/></svg>"#;
const DOWNLOAD: &str = r#"<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M8 2.5v8M4.5 7 8 10.5 11.5 7M3 13.5h10"/></svg>"#;

impl Component for FileUpload {
    fn name(&self) -> &'static str {
        "ui-file-upload"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo {
            doc: "Files to attach: a drop zone, uploads with progress, the files as a grid or a list, a preview of images, PDFs, \
                  spreadsheets and text, and Download all as a zip. The files are kept in state under `bind`.",
            props: PROPS,
            parts: &["zone", "files"],
            children: false,
            open_props: false,
        }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let key = cx.prop_string(node, "bind").ok_or_else(|| RenderError::new(node, "`ui-file-upload` needs `bind`"))?;
        let flag = |cx: &RenderCx<'_>, name: &str, default: bool| cx.prop(node, name).map_or(default, |v| expr::truthy(&v));
        let read_only = flag(cx, "readOnly", false);
        let multiple = flag(cx, "multiple", true);
        let removable = flag(cx, "removable", true) && !read_only;
        let download = flag(cx, "download", true);
        let max_size = match cx.prop(node, "maxSize") {
            None | Some(Json::Null) => None,
            Some(v) => Some(bytes(&v).ok_or_else(|| RenderError::new(node, "`maxSize` must be bytes, or text like `10 MB` or `500 KB`"))?),
        };
        let max_files = cx.prop(node, "maxFiles").map(|v| expr::to_num(&v));
        if max_files.is_some_and(|n| n < 1.0 || n.fract() != 0.0) {
            return Err(RenderError::new(node, "`maxFiles` must be a whole number from 1"));
        }
        let view = cx.prop_string(node, "view").unwrap_or_else(|| "grid".into());
        let accept = cx.prop_string(node, "accept");
        let id = instance_id(cx, node, "upload");

        cx.global_js("ui-file-upload", SCRIPT);
        let params = match node.prop("params") {
            Some(p) => crate::data_table::dynamic(cx, node, "params", p)?,
            None => Json::Null,
        };
        let url = match node.prop("url") {
            Some(u) => crate::data_table::dynamic(cx, node, "url", u)?,
            None => Json::Null,
        };
        let config = json!({
            "bind": key, "url": url, "params": params, "accept": accept, "multiple": multiple,
            "maxFiles": max_files, "maxSize": max_size, "view": view, "removable": removable, "readOnly": read_only,
            "zipName": cx.prop_string(node, "zipName").unwrap_or_else(|| cx.label("attachments")),
        });
        let mut root = cx.element("div", node);
        root.base_style_ref(&ROOT).behavior("ui-file-upload", config);
        cx.open(&root, node)?;

        if !read_only {
            let mut zone = cx.part("label", node, "zone");
            zone.attr("data-up-zone", true);
            cx.open(&zone, node)?;
            let mut input = Element::new("input");
            input.attr("type", "file").attr("id", id.clone()).attr("data-up-input", true);
            if multiple {
                input.attr("multiple", true);
            }
            if let Some(a) = &accept {
                input.attr("accept", a.clone());
            }
            if let Some(d) = node.prop("disabled") {
                input.attr("disabled", d.clone());
            }
            if node.prop("hint").is_some() {
                input.attr("aria-describedby", id_with(&id, "-hint"));
            }
            cx.open(&input, node)?;
            cx.markup(UPLOAD);
            let mut label = Element::new("span");
            label.attr("data-up-label", true).text(node.prop("label").cloned().unwrap_or_else(|| lit(cx.label("Drop files here or click to browse"))));
            cx.open(&label, node)?;
            cx.close("span");
            if let Some(h) = node.prop("hint") {
                let mut hint = Element::new("span");
                hint.attr("data-up-hint", true).attr("id", id_with(&id, "-hint")).text(h.clone());
                cx.open(&hint, node)?;
                cx.close("span");
            }
            cx.close("label");
            let mut errors = Element::new("ul");
            errors.attr("data-up-errors", true).attr("role", "alert").attr("hidden", true);
            cx.open(&errors, node)?;
            cx.close("ul");
        }

        let mut bar = Element::new("div");
        bar.attr("data-up-bar", true).attr("hidden", true);
        cx.open(&bar, node)?;
        let mut count = Element::new("span");
        count.attr("data-up-total", true);
        cx.open(&count, node)?;
        cx.close("span");
        if download {
            let mut b = Element::new("button");
            forms::button_look(&mut b, "outline", "sm");
            b.attr("type", "button").attr("data-up-zip", true);
            cx.open(&b, node)?;
            cx.markup(DOWNLOAD);
            let mut t = Element::new("span");
            t.text(lit(cx.label("Download all")));
            cx.open(&t, node)?;
            cx.close("span");
            cx.close("button");
        }
        cx.close("div");

        let mut files = cx.part("ul", node, "files");
        files.base_style_ref(&FILES).attr("data-up-files", true).attr("data-view", view).attr("aria-label", cx.label("Files"));
        cx.open(&files, node)?;
        cx.close("ul");
        if read_only {
            let mut empty = Element::new("p");
            empty.attr("data-up-empty", true).text(node.prop("emptyText").cloned().unwrap_or_else(|| lit(cx.label("No files yet."))));
            cx.open(&empty, node)?;
            cx.close("p");
        }

        // The preview: one file at a time, with the arrows to the others.
        let mut dialog = Element::new("dialog");
        dialog.base_style_ref(&PREVIEW).attr("data-up-preview", true).attr("closedby", "any").attr("aria-labelledby", id_with(&id, "-title"));
        cx.open(&dialog, node)?;
        let mut head = Element::new("div");
        head.attr("data-up-head", true);
        cx.open(&head, node)?;
        let mut title = Element::new("h2");
        title.attr("data-up-title", true).attr("id", id_with(&id, "-title"));
        cx.open(&title, node)?;
        cx.close("h2");
        let mut n = Element::new("span");
        n.attr("data-up-count", true);
        cx.open(&n, node)?;
        cx.close("span");
        // Labels: "Previous file", "Next file"
        for (attr, label, text) in [("data-up-prev", "Previous file", "‹"), ("data-up-next", "Next file", "›")] {
            let mut b = Element::new("button");
            forms::button_look(&mut b, "outline", "sm");
            b.attr("type", "button").attr(attr, true).attr("aria-label", cx.label(label)).text(lit(text));
            cx.open(&b, node)?;
            cx.close("button");
        }
        let mut save = Element::new("a");
        forms::button_look(&mut save, "outline", "sm");
        save.attr("data-up-save", true).attr("download", "").attr("href", "#");
        cx.open(&save, node)?;
        cx.markup(DOWNLOAD);
        let mut t = Element::new("span");
        t.text(lit(cx.label("Download")));
        cx.open(&t, node)?;
        cx.close("span");
        cx.close("a");
        cx.close("div");
        let mut body = Element::new("div");
        body.attr("data-up-body", true);
        cx.open(&body, node)?;
        cx.close("div");
        let mut x = Element::new("button");
        x.base_style_ref(&DIALOG_X).attr("type", "button").attr("data-up-close", true).attr("aria-label", cx.label("Close")).text(lit("✕"));
        cx.open(&x, node)?;
        cx.close("button");
        cx.close("dialog");

        let mut live = Element::new("div");
        live.attr("data-up-live", true).attr("role", "status").attr("aria-live", "polite");
        cx.open(&live, node)?;
        cx.close("div");
        cx.close("div");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::bytes;
    use serde_json::json;

    #[test]
    fn sizes() {
        assert_eq!(bytes(&json!(2048)), Some(2048.0));
        assert_eq!(bytes(&json!("10 MB")), Some(10.0 * 1024.0 * 1024.0));
        assert_eq!(bytes(&json!("500kb")), Some(500.0 * 1024.0));
        assert_eq!(bytes(&json!("1.5 GB")), Some(1.5 * 1024.0 * 1024.0 * 1024.0));
        assert_eq!(bytes(&json!("12")), Some(12.0));
        assert_eq!(bytes(&json!("ten MB")), None);
        assert_eq!(bytes(&json!("5 TB")), None);
        assert_eq!(bytes(&json!(0)), None);
    }
}
