/// Appends `s` to `out`, replacing each byte for which `map` returns an entity.
#[inline]
fn push_escaped(out: &mut String, s: &str, map: impl Fn(u8) -> Option<&'static str>) {
    let mut start = 0;
    for (i, b) in s.bytes().enumerate() {
        if let Some(entity) = map(b) {
            out.push_str(&s[start..i]);
            out.push_str(entity);
            start = i + 1;
        }
    }
    out.push_str(&s[start..]);
}

/// Appends escaped text content.
pub fn push_text(out: &mut String, s: &str) {
    push_escaped(out, s, |b| match b {
        b'&' => Some("&amp;"),
        b'<' => Some("&lt;"),
        b'>' => Some("&gt;"),
        _ => None,
    });
}

/// Appends an escaped double-quoted attribute value.
pub fn push_attr(out: &mut String, s: &str) {
    push_escaped(out, s, |b| match b {
        b'&' => Some("&amp;"),
        b'<' => Some("&lt;"),
        b'>' => Some("&gt;"),
        b'"' => Some("&quot;"),
        b'\'' => Some("&#39;"),
        _ => None,
    });
}

/// Escapes text content.
pub fn text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    push_text(&mut out, s);
    out
}

/// JSON safe to embed in a `<script>` element.
pub fn script_json(v: &serde_json::Value) -> String {
    v.to_string().replace('<', "\\u003c")
}

#[cfg(test)]
mod tests {
    #[test]
    fn escaping() {
        let mut out = String::new();
        super::push_attr(&mut out, "a&b<c>\"d'é");
        assert_eq!(out, "a&amp;b&lt;c&gt;&quot;d&#39;é");
        assert_eq!(super::text("x<y & ü"), "x&lt;y &amp; ü");
    }
}
