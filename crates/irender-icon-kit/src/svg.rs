//! Reads an SVG file down to what an inline icon needs: the root's `viewBox` and
//! drawing defaults, and the shapes. Everything else (scripts, styles, links,
//! text, gradients, unknown elements and attributes) is dropped, so a pack loaded
//! from files can't put anything but shapes into a page.

use std::fmt::Write;

/// Shape and group elements an icon keeps.
const ELEMENTS: &[&str] = &["path", "circle", "ellipse", "rect", "line", "polyline", "polygon", "g"];

/// Drawing defaults kept from the root `<svg>`.
const ROOT_ATTRIBUTES: &[&str] =
    &["fill", "stroke", "stroke-width", "stroke-linecap", "stroke-linejoin", "stroke-miterlimit", "fill-rule", "clip-rule"];

/// Attributes kept on shapes.
const ATTRIBUTES: &[&str] = &[
    "d", "cx", "cy", "r", "rx", "ry", "x", "y", "width", "height", "x1", "y1", "x2", "y2", "points", "transform",
    "fill", "stroke", "stroke-width", "stroke-linecap", "stroke-linejoin", "stroke-miterlimit", "stroke-dasharray",
    "stroke-dashoffset", "fill-rule", "clip-rule", "opacity", "fill-opacity", "stroke-opacity",
];

/// An icon ready to inline.
#[derive(Debug, Clone, PartialEq)]
pub struct Icon {
    pub(crate) view_box: String,
    /// Drawing defaults from the root, such as `fill="none"`.
    pub(crate) attrs: Vec<(String, String)>,
    /// The shapes, as markup.
    pub(crate) body: String,
}

impl Icon {
    pub fn view_box(&self) -> &str {
        &self.view_box
    }

    /// The icon's shapes, as sanitized markup.
    pub fn body(&self) -> &str {
        &self.body
    }
}

pub(crate) fn parse(src: &str) -> Result<Icon, String> {
    let mut p = Parser { src, at: 0 };
    let mut root: Option<Vec<(String, String)>> = None;
    let mut body = String::new();
    // Open elements: whether each was kept.
    let mut open: Vec<bool> = Vec::new();
    // Inside a dropped element (or a shape's children): how deep.
    let mut skip = 0usize;

    while let Some(tag) = p.next_tag()? {
        match tag {
            Tag::Close(name) => {
                if root.is_none() {
                    return Err(format!("unexpected `</{name}>`"));
                }
                if skip > 0 {
                    skip -= 1;
                    continue;
                }
                match open.pop() {
                    Some(true) => body.push_str("</g>"),
                    Some(false) => {}
                    None => return finish(root.take().unwrap(), body),
                }
            }
            Tag::Open { name, attrs, closed } => {
                if root.is_none() {
                    if name != "svg" {
                        return Err(format!("expected `<svg>`, found `<{name}>`"));
                    }
                    if closed {
                        return finish(attrs, body);
                    }
                    root = Some(attrs);
                    continue;
                }
                if skip > 0 || !ELEMENTS.contains(&name.as_str()) {
                    skip += usize::from(!closed);
                    continue;
                }
                let _ = write!(body, "<{name}");
                for (k, v) in &attrs {
                    if ATTRIBUTES.contains(&k.as_str()) && safe_value(v) {
                        let _ = write!(body, " {k}=\"{v}\"");
                    }
                }
                if name == "g" {
                    body.push('>');
                    if closed {
                        body.push_str("</g>");
                    } else {
                        open.push(true);
                    }
                } else {
                    // Shapes keep no children (an `<animate>`, a `<title>`).
                    body.push_str("/>");
                    skip += usize::from(!closed);
                }
            }
        }
    }
    Err(if root.is_some() { "`<svg>` is never closed".into() } else { "no `<svg>` element".into() })
}

fn finish(root: Vec<(String, String)>, body: String) -> Result<Icon, String> {
    let get = |k: &str| root.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
    let view_box = match (get("viewBox"), get("width"), get("height")) {
        (Some(v), _, _) => v.split([' ', ',']).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" "),
        (None, Some(w), Some(h)) => format!("0 0 {} {}", w.trim_end_matches("px"), h.trim_end_matches("px")),
        _ => return Err("`<svg>` needs a `viewBox` (or `width` and `height`)".into()),
    };
    if view_box.split(' ').count() != 4 || !view_box.split(' ').all(|n| n.parse::<f64>().is_ok()) {
        return Err(format!("invalid viewBox `{view_box}`"));
    }
    if body.is_empty() {
        return Err("no shapes".into());
    }
    let attrs = root
        .into_iter()
        .filter(|(k, v)| ROOT_ATTRIBUTES.contains(&k.as_str()) && safe_value(v))
        .collect();
    Ok(Icon { view_box, attrs, body })
}

/// Numbers, path data, colours and transforms; nothing that references a URL or
/// needs escaping.
fn safe_value(v: &str) -> bool {
    !v.to_ascii_lowercase().contains("url(")
        && v.chars().all(|c| c.is_ascii_alphanumeric() || " .,-+#%()\n\t".contains(c))
}

enum Tag {
    Open { name: String, attrs: Vec<(String, String)>, closed: bool },
    Close(String),
}

struct Parser<'a> {
    src: &'a str,
    at: usize,
}

impl Parser<'_> {
    fn rest(&self) -> &str {
        &self.src[self.at..]
    }

    fn skip_past(&mut self, end: &str) -> Result<(), String> {
        let i = self.rest().find(end).ok_or_else(|| format!("missing `{end}`"))?;
        self.at += i + end.len();
        Ok(())
    }

    fn next_tag(&mut self) -> Result<Option<Tag>, String> {
        loop {
            let Some(i) = self.rest().find('<') else { return Ok(None) };
            self.at += i;
            let rest = self.rest();
            if rest.starts_with("<!--") {
                self.skip_past("-->")?;
            } else if rest.starts_with("<![CDATA[") {
                self.skip_past("]]>")?;
            } else if rest.starts_with("<?") {
                self.skip_past("?>")?;
            } else if rest.starts_with("<!") {
                self.skip_past(">")?;
            } else if let Some(rest) = rest.strip_prefix("</") {
                let end = rest.find('>').ok_or("unclosed tag")?;
                let name = rest[..end].trim().to_string();
                self.at += 2 + end + 1;
                return Ok(Some(Tag::Close(local(&name))));
            } else {
                self.at += 1;
                return self.open_tag().map(Some);
            }
        }
    }

    fn open_tag(&mut self) -> Result<Tag, String> {
        let name = self.name();
        if name.is_empty() {
            return Err("invalid tag".into());
        }
        let mut attrs = Vec::new();
        loop {
            self.spaces();
            let rest = self.rest();
            if rest.starts_with("/>") {
                self.at += 2;
                return Ok(Tag::Open { name: local(&name), attrs, closed: true });
            }
            if rest.starts_with('>') {
                self.at += 1;
                return Ok(Tag::Open { name: local(&name), attrs, closed: false });
            }
            let key = self.name();
            if key.is_empty() {
                return Err(format!("invalid attribute in `<{name}>`"));
            }
            self.spaces();
            if !self.rest().starts_with('=') {
                return Err(format!("attribute `{key}` in `<{name}>` has no value"));
            }
            self.at += 1;
            self.spaces();
            let quote = self.rest().chars().next().filter(|c| *c == '"' || *c == '\'');
            let Some(quote) = quote else { return Err(format!("attribute `{key}` must be quoted")) };
            self.at += 1;
            let end = self.rest().find(quote).ok_or("unclosed attribute value")?;
            let value = self.rest()[..end].trim().to_string();
            self.at += end + 1;
            attrs.push((key, value));
        }
    }

    fn name(&mut self) -> String {
        let n = self.rest().find(|c: char| !(c.is_ascii_alphanumeric() || "-_:.".contains(c))).unwrap_or(self.rest().len());
        let name = self.rest()[..n].to_string();
        self.at += n;
        name
    }

    fn spaces(&mut self) {
        let n = self.rest().len() - self.rest().trim_start().len();
        self.at += n;
    }
}

/// `svg:path` → `path`.
fn local(name: &str) -> String {
    name.rsplit(':').next().unwrap_or(name).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_shapes_and_defaults() {
        let icon = parse(
            r#"<?xml version="1.0"?><!-- by hand --><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"
                viewBox="0 0 24 24" fill="none" stroke="currentColor" class="x" onload="alert(1)">
                <title>Truck</title><path d="M3 4h2"/><g stroke-width="1"><circle cx="9" cy="19" r="1.5"></circle></g></svg>"#,
        )
        .unwrap();
        assert_eq!(icon.view_box, "0 0 24 24");
        assert_eq!(icon.attrs, vec![("fill".into(), "none".into()), ("stroke".into(), "currentColor".into())]);
        assert_eq!(icon.body, r#"<path d="M3 4h2"/><g stroke-width="1"><circle cx="9" cy="19" r="1.5"/></g>"#);
    }

    #[test]
    fn drops_everything_that_is_not_a_shape() {
        let icon = parse(
            r##"<svg viewBox="0 0 10 10"><script>alert(1)</script><style>*{}</style>
                <a href="javascript:x"><path d="M0 0"/></a><foreignObject><div>x</div></foreignObject>
                <rect x="1" y="1" width="2" height="2" fill="url(#g)" onclick="x" style="a:b" href="y"/>
                <use href="#a"/><image href="x.png"/></svg>"##,
        )
        .unwrap();
        assert_eq!(icon.body, r#"<rect x="1" y="1" width="2" height="2"/>"#);
    }

    #[test]
    fn view_box_from_size() {
        let icon = parse(r#"<svg width="16px" height="16px"><path d="M1 1"/></svg>"#).unwrap();
        assert_eq!(icon.view_box, "0 0 16 16");
    }

    #[test]
    fn rejects_broken_files() {
        assert!(parse("<div/>").is_err());
        assert!(parse(r#"<svg viewBox="0 0 24 24"><path d="M0 0"/>"#).is_err());
        assert!(parse(r#"<svg><path d="M0 0"/></svg>"#).is_err());
        assert!(parse(r#"<svg viewBox="0 0 24 24"></svg>"#).is_err());
        assert!(parse(r#"<svg viewBox="0 0 24 24"><path d=M0/></svg>"#).is_err());
    }
}
