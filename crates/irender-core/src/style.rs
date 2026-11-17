use crate::expr::{Template, num, stringify};
use serde_json::Value as Json;
use std::hash::{Hash, Hasher};

/// A resolved style object: declarations plus nested blocks keyed by
/// `:pseudo`, `&selector`, `@breakpoint`, `@variant` or a raw `@media …`.
///
/// `Hash` is consistent with `PartialEq`, so renderers can cache compiled blocks by content.
#[derive(Debug, Clone, Default, PartialEq, Hash)]
pub struct StyleBlock {
    pub decls: Vec<Decl>,
    pub nested: Vec<(String, StyleBlock)>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Decl {
    pub property: String,
    pub value: StyleValue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StyleValue {
    /// Final CSS text; theme tokens are already `var(--…)`.
    Static(String),
    /// Evaluated at render time and delivered through a CSS custom property.
    /// `px` means a bare number result gets a `px` unit.
    Dynamic { tpl: Template, px: bool },
}

impl Hash for StyleValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            StyleValue::Static(s) => s.hash(state),
            StyleValue::Dynamic { tpl, px } => {
                tpl.hash(state);
                px.hash(state);
            }
        }
    }
}

impl StyleBlock {
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds a block of static declarations.
    pub fn from_pairs(pairs: &[(&str, &str)]) -> Self {
        pairs.iter().fold(Self::new(), |b, (k, v)| b.set(k, *v))
    }

    pub fn is_empty(&self) -> bool {
        self.decls.is_empty() && self.nested.iter().all(|(_, b)| b.is_empty())
    }

    pub fn set(mut self, property: &str, value: impl Into<String>) -> Self {
        self.push(property, StyleValue::Static(value.into()));
        self
    }

    pub fn push(&mut self, property: &str, value: StyleValue) {
        self.decls.push(Decl { property: to_kebab(property), value });
    }

    pub fn nest(mut self, key: impl Into<String>, block: StyleBlock) -> Self {
        self.nested.push((key.into(), block));
        self
    }
}

pub fn is_nested_key(key: &str) -> bool {
    key.starts_with([':', '&', '@', '['])
}

/// `backgroundColor` → `background-color`, `WebkitLineClamp` → `-webkit-line-clamp`.
/// Already-kebab and custom (`--x`) properties are unchanged.
pub fn to_kebab(property: &str) -> String {
    if property.starts_with("--") || !property.contains(|c: char| c.is_ascii_uppercase()) {
        return property.to_string();
    }
    let mut out = String::with_capacity(property.len() + 4);
    for (i, c) in property.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 || matches!(property.get(..6), Some("Webkit")) || property.starts_with("Moz") {
                out.push('-');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

const UNITLESS: &[&str] = &[
    "animation-iteration-count", "aspect-ratio", "column-count", "columns", "fill-opacity",
    "flex", "flex-grow", "flex-shrink", "font-weight", "grid-area", "grid-column",
    "grid-column-end", "grid-column-start", "grid-row", "grid-row-end", "grid-row-start",
    "line-clamp", "-webkit-line-clamp", "line-height", "opacity", "order", "orphans", "scale",
    "stop-opacity", "stroke-opacity", "tab-size", "widows", "z-index", "zoom",
];

/// Whether a bare number for this property should stay unitless rather than gain `px`.
pub fn is_unitless(property: &str) -> bool {
    property.starts_with("--") || UNITLESS.contains(&property)
}

pub fn number_to_css(property: &str, n: f64) -> String {
    let s = stringify(&num(n));
    if n == 0.0 || is_unitless(property) { s } else { format!("{s}px") }
}

/// CSS text for a literal value: numbers gain `px` where appropriate, strings pass through.
pub fn value_to_css(property: &str, v: &Json) -> Option<String> {
    match v {
        Json::Number(n) => Some(number_to_css(property, n.as_f64()?)),
        Json::String(s) => Some(s.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kebab() {
        assert_eq!(to_kebab("backgroundColor"), "background-color");
        assert_eq!(to_kebab("WebkitLineClamp"), "-webkit-line-clamp");
        assert_eq!(to_kebab("border-top"), "border-top");
        assert_eq!(to_kebab("--myVar"), "--myVar");
    }

    #[test]
    fn numbers() {
        assert_eq!(number_to_css("padding", 12.0), "12px");
        assert_eq!(number_to_css("opacity", 0.5), "0.5");
        assert_eq!(number_to_css("margin", 0.0), "0");
    }
}
