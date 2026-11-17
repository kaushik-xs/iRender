//! CSS validation and minification backed by lightningcss.

use irender_core::{Diagnostics, Ir, Node, StyleBlock, StyleValue};
use lightningcss::{
    properties::{Property, PropertyId},
    stylesheet::{MinifyOptions, ParserOptions, PrinterOptions, StyleSheet},
};

/// Standard properties that lightningcss doesn't model. Their names are accepted, but
/// their values can't be checked. Keep sorted: it is binary-searched.
const UNMODELLED_PROPERTIES: &[&str] = &[
    "anchor-name", "background-blend-mode", "border-collapse", "break-after", "break-before", "break-inside",
    "caption-side", "clear", "clip", "column-count", "column-fill", "column-rule", "column-span",
    "column-width", "columns", "contain", "contain-intrinsic-size", "content", "content-visibility",
    "counter-increment", "counter-reset", "counter-set", "empty-cells", "field-sizing", "float",
    "font-feature-settings", "font-kerning", "font-language-override", "font-optical-sizing",
    "font-size-adjust", "font-synthesis", "font-variant", "font-variant-alternates",
    "font-variant-east-asian", "font-variant-ligatures", "font-variant-numeric", "font-variant-position",
    "font-variation-settings", "forced-color-adjust", "hanging-punctuation", "hyphenate-character",
    "image-orientation", "initial-letter", "interpolate-size", "isolation", "line-clamp", "math-depth",
    "math-style", "object-fit", "object-position", "offset", "offset-distance", "offset-path", "orphans",
    "outline-offset", "overflow-anchor", "overflow-clip-margin", "overscroll-behavior",
    "overscroll-behavior-x", "overscroll-behavior-y", "page", "page-break-after", "page-break-before",
    "page-break-inside", "paint-order", "pointer-events", "position-anchor", "position-area", "position-try",
    "quotes", "ruby-align", "ruby-position", "scroll-behavior", "scroll-snap-align", "scroll-snap-stop",
    "scroll-snap-type", "scrollbar-color", "scrollbar-gutter", "scrollbar-width", "shape-image-threshold",
    "shape-margin", "shape-outside", "table-layout", "text-box", "text-combine-upright", "text-orientation",
    "text-underline-offset", "text-underline-position", "text-wrap", "text-wrap-mode", "text-wrap-style",
    "touch-action", "transition-behavior", "view-timeline", "white-space-collapse", "widows", "will-change",
    "writing-mode", "zoom",
];

/// Checks that `property` exists and `value` is valid for it.
///
/// Custom properties (`--x`) and vendor-prefixed properties are accepted as-is, and
/// so is any value of a property in [`UNMODELLED_PROPERTIES`]. Values containing
/// `var()` can only be checked once the variable is known, so they are accepted too.
pub fn validate_declaration(property: &str, value: &str) -> Result<(), String> {
    if property.starts_with('-') || UNMODELLED_PROPERTIES.binary_search(&property).is_ok() {
        return Ok(());
    }
    let id = PropertyId::from(property);
    if let PropertyId::Custom(_) = id {
        return Err(format!("unknown CSS property `{property}` (check the spelling)"));
    }
    // lightningcss leaves CSS-wide keywords unparsed, but every property accepts them.
    let keyword = ["inherit", "initial", "unset", "revert", "revert-layer"]
        .iter()
        .any(|k| value.trim().eq_ignore_ascii_case(k));
    match Property::parse_string(id, value, ParserOptions::default()) {
        Ok(Property::Unparsed(_)) if !value.contains("var(") && !keyword => {
            Err(format!("invalid value `{value}` for `{property}`"))
        }
        Ok(_) => Ok(()),
        Err(_) => Err(format!("invalid value `{value}` for `{property}`")),
    }
}

/// Checks every static declaration in the tree, and the property name of dynamic ones.
///
/// Findings are warnings: lightningcss doesn't know every valid property or the newest
/// value syntax, and a gap in the validator must not stop a style from being used.
/// Use [`Diagnostics::strict`] to treat them as errors.
pub fn validate(ir: &Ir) -> Diagnostics {
    let mut diags = Diagnostics::default();
    walk(&ir.root, &mut diags);
    diags
}

fn walk(node: &Node, diags: &mut Diagnostics) {
    block(&node.style, &format!("{}.style", node.path), diags);
    for (name, part) in &node.parts {
        block(&part.style, &format!("{}.parts.{name}.style", node.path), diags);
    }
    for child in &node.children {
        walk(child, diags);
    }
}

fn block(b: &StyleBlock, path: &str, diags: &mut Diagnostics) {
    for d in &b.decls {
        let value = match &d.value {
            StyleValue::Static(v) => v.as_str(),
            // Every property accepts `var()`, so this checks only the name.
            StyleValue::Dynamic { .. } => "var(--i-d)",
        };
        if let Err(e) = validate_declaration(&d.property, value) {
            diags.warn(format!("{path}.{}", d.property), e);
        }
    }
    for (key, child) in &b.nested {
        block(child, &format!("{path}.{key}"), diags);
    }
}

pub fn minify(css: &str) -> Result<String, String> {
    let mut sheet = StyleSheet::parse(css, ParserOptions::default()).map_err(|e| e.to_string())?;
    sheet.minify(MinifyOptions::default()).map_err(|e| e.to_string())?;
    let out = sheet
        .to_css(PrinterOptions { minify: true, ..PrinterOptions::default() })
        .map_err(|e| e.to_string())?;
    Ok(out.code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declarations() {
        assert!(validate_declaration("color", "red").is_ok());
        assert!(validate_declaration("padding", "8px 12px").is_ok());
        assert!(validate_declaration("color", "var(--color-primary)").is_ok());
        assert!(validate_declaration("--anything", "whatever").is_ok());
        assert!(validate_declaration("-webkit-line-clamp", "3").is_ok());
        for keyword in ["inherit", "initial", "unset", "revert", "revert-layer", "INHERIT"] {
            assert!(validate_declaration("font-size", keyword).is_ok(), "{keyword}");
            assert!(validate_declaration("font", keyword).is_ok(), "{keyword}");
        }
        assert!(validate_declaration("colr", "red").is_err());
        assert!(validate_declaration("color", "12px").is_err());
        assert!(validate_declaration("display", "flexy").is_err());
    }

    #[test]
    fn unmodelled_properties_are_known() {
        assert!(UNMODELLED_PROPERTIES.windows(2).all(|w| w[0] < w[1]), "keep the list sorted and unique");
        for (property, value) in [("border-collapse", "collapse"), ("font-variant-numeric", "tabular-nums"),
                                  ("pointer-events", "none"), ("content", "'→'"), ("float", "left")] {
            assert!(validate_declaration(property, value).is_ok(), "{property}");
        }
        // Every entry is one lightningcss really doesn't know; drop any it learns.
        for property in UNMODELLED_PROPERTIES {
            assert!(matches!(PropertyId::from(*property), PropertyId::Custom(_)), "{property} is modelled now");
        }
        assert!(validate_declaration("border-colapse", "collapse").is_err());
    }

    #[test]
    fn minifies_layers() {
        let out = minify("@layer a {\n.x {\n  color: red;\n}\n}\n").unwrap();
        assert_eq!(out, "@layer a{.x{color:red}}");
    }
}
