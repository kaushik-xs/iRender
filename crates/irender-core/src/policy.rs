use std::collections::BTreeSet;

/// Attributes whose value is a URL and must pass [`Policy::url_allowed`].
pub const URL_ATTRIBUTES: &[&str] =
    &["href", "src", "action", "formaction", "poster", "cite", "data", "xlink:href"];

/// What a document is allowed to do.
///
/// Use [`Policy::trusted`] for configs written by your own team and
/// [`Policy::untrusted`] for anything written by users, tenants or an LLM.
/// Some checks (valid tag/attribute names, CSS values that could break out of a
/// declaration) are always on, whatever the policy.
#[derive(Debug, Clone)]
pub struct Policy {
    /// Allow `onclick`-style attributes. Interactivity should go through `on` actions.
    pub allow_event_attributes: bool,
    /// Allow `javascript:`, `data:` and other non-http(s) URLs and CSS `expression()`.
    pub allow_unsafe_urls: bool,
    pub blocked_tags: BTreeSet<String>,
    pub blocked_attributes: BTreeSet<String>,
    /// When set, only these CSS properties may be used (custom properties `--*` always may).
    pub allowed_css_properties: Option<BTreeSet<String>>,
    pub blocked_css_properties: BTreeSet<String>,
}

impl Default for Policy {
    fn default() -> Self {
        Self::trusted()
    }
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

impl Policy {
    pub fn trusted() -> Self {
        Self {
            allow_event_attributes: true,
            allow_unsafe_urls: true,
            blocked_tags: BTreeSet::new(),
            blocked_attributes: BTreeSet::new(),
            allowed_css_properties: None,
            blocked_css_properties: BTreeSet::new(),
        }
    }

    pub fn untrusted() -> Self {
        Self {
            allow_event_attributes: false,
            allow_unsafe_urls: false,
            blocked_tags: set(&[
                "script", "style", "iframe", "frame", "frameset", "object", "embed", "applet",
                "link", "meta", "base", "template",
            ]),
            blocked_attributes: set(&["srcdoc"]),
            allowed_css_properties: None,
            blocked_css_properties: set(&["behavior", "-moz-binding"]),
        }
    }

    pub fn check_tag(&self, tag: &str) -> Result<(), String> {
        let valid = tag.starts_with(|c: char| c.is_ascii_alphabetic())
            && tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
        if !valid {
            return Err(format!("`{tag}` is not a valid tag name"));
        }
        if self.blocked_tags.contains(&tag.to_ascii_lowercase()) {
            return Err(format!("tag `{tag}` is not allowed by the policy"));
        }
        Ok(())
    }

    pub fn check_attr_name(&self, name: &str) -> Result<(), String> {
        let valid = name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_' || c == ':')
            && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':' | '.'));
        if !valid {
            return Err(format!("`{name}` is not a valid attribute name"));
        }
        let lower = name.to_ascii_lowercase();
        if !self.allow_event_attributes && lower.starts_with("on") {
            return Err(format!("event attribute `{name}` is not allowed; use `on` actions"));
        }
        if self.blocked_attributes.contains(&lower) {
            return Err(format!("attribute `{name}` is not allowed by the policy"));
        }
        // The runtime reads its bindings and actions from these; a document that wrote
        // them could forge actions that skip validation.
        if lower.starts_with("data-i-") {
            return Err(format!("`{name}` is reserved for the iRender runtime"));
        }
        Ok(())
    }

    pub fn is_url_attr(name: &str) -> bool {
        URL_ATTRIBUTES.contains(&name.to_ascii_lowercase().as_str())
    }

    pub fn url_allowed(&self, url: &str) -> bool {
        self.allow_unsafe_urls || is_safe_url(url)
    }

    pub fn check_url_attr(&self, name: &str, value: &str) -> Result<(), String> {
        if Self::is_url_attr(name) && !self.url_allowed(value) {
            return Err(format!("URL `{value}` is not allowed by the policy"));
        }
        Ok(())
    }

    pub fn check_css_property(&self, property: &str) -> Result<(), String> {
        if property.starts_with("--") {
            return Ok(());
        }
        if self.blocked_css_properties.contains(property)
            || self.allowed_css_properties.as_ref().is_some_and(|a| !a.contains(property))
        {
            return Err(format!("CSS property `{property}` is not allowed by the policy"));
        }
        Ok(())
    }

    pub fn check_css_value(&self, value: &str) -> Result<(), String> {
        css_value_is_contained(value)?;
        if !self.allow_unsafe_urls {
            let lower = value.to_ascii_lowercase();
            if ["javascript:", "vbscript:", "expression("].iter().any(|bad| lower.contains(bad)) {
                return Err(format!("CSS value `{value}` is not allowed by the policy"));
            }
        }
        Ok(())
    }
}

/// Always enforced: a CSS value must not be able to end its declaration or rule.
pub fn css_value_is_contained(value: &str) -> Result<(), String> {
    if value.contains(['{', '}', ';']) || value.contains("</") {
        return Err(format!("CSS value `{value}` must not contain `{{`, `}}`, `;` or `</`"));
    }
    Ok(())
}

/// Relative URLs and http(s), mailto and tel URLs.
pub fn is_safe_url(url: &str) -> bool {
    let url = url.trim_start();
    let scheme_end = url.find(':');
    let path_start = url.find(['/', '?', '#']);
    match (scheme_end, path_start) {
        (Some(c), Some(p)) if p < c => true,
        (Some(c), _) => {
            let scheme = url[..c].to_ascii_lowercase();
            matches!(scheme.as_str(), "http" | "https" | "mailto" | "tel")
        }
        (None, _) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls() {
        assert!(is_safe_url("/a/b?x=1:2"));
        assert!(is_safe_url("https://x.dev"));
        assert!(is_safe_url("mailto:a@b.c"));
        assert!(!is_safe_url("javascript:alert(1)"));
        assert!(!is_safe_url("  JavaScript:alert(1)"));
        assert!(!is_safe_url("data:text/html,hi"));
    }

    #[test]
    fn untrusted_blocks_dangerous_things() {
        let p = Policy::untrusted();
        assert!(p.check_tag("script").is_err());
        assert!(p.check_attr_name("onclick").is_err());
        assert!(p.check_url_attr("href", "javascript:x").is_err());
        assert!(p.check_css_value("url(javascript:x)").is_err());
        assert!(Policy::trusted().check_css_value("red; position: fixed").is_err());
    }
}
