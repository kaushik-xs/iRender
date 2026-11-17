use irender_core::{Diagnostics, StyleBlock, StyleValue, Template, Theme, theme::ThemeSpec};
use irender_style::{Layer, StyleCompiler};
use serde_json::json;

fn block(pairs: &[(&str, &str)]) -> StyleBlock {
    StyleBlock::from_pairs(pairs)
}

fn theme(spec: serde_json::Value) -> Theme {
    let spec: ThemeSpec = serde_json::from_value(spec).unwrap();
    let mut d = Diagnostics::default();
    let t = Theme::resolve(&spec, &mut d);
    assert!(d.is_empty(), "{d}");
    t
}

/// The CSS for one compiled block, with the class replaced by `.X`.
fn css_of(theme: &Theme, b: &StyleBlock) -> String {
    let mut c = StyleCompiler::new(theme);
    let class = c.compile(b, Layer::Instance).unwrap().class.clone();
    let css = c.css();
    let start = css.find("@layer i-instance {\n").unwrap() + 20;
    css[start..css.len() - 2].trim_end().replace(&format!(".{class}"), ".X")
}

#[test]
fn empty_blocks_produce_no_class() {
    let t = Theme::default();
    let mut c = StyleCompiler::new(&t);
    assert!(c.compile(&StyleBlock::new(), Layer::Instance).is_none());
    assert!(c.compile(&StyleBlock::new().nest(":hover", StyleBlock::new()), Layer::Base).is_none());
}

#[test]
fn class_names_are_stable() {
    // Pinned so class names stay the same across releases (good for caching and snapshots).
    let t = Theme::default();
    let mut c = StyleCompiler::new(&t);
    let class = c.compile(&block(&[("color", "red")]), Layer::Instance).unwrap().class.clone();
    assert_eq!(class, "i-edmjinugoisj");
}

#[test]
fn nesting_combinations() {
    let t = Theme::default();
    let b = block(&[("color", "red")])
        .nest("@md", block(&[]).nest(":hover", block(&[("color", "blue")])))
        .nest("@media (orientation: portrait)", block(&[]).nest("@supports (display: grid)", block(&[("display", "grid")])))
        .nest("&:not(.x) + &", block(&[("margin", "0")]))
        .nest("::before", block(&[("content", "'*'")]))
        .nest("[aria-current=page]", block(&[("font-weight", "700")]));
    assert_eq!(
        css_of(&t, &b),
        "\
.X {
  color: red;
}
@media (min-width: 768px) {
.X:hover {
  color: blue;
}
}
@media (orientation: portrait) {
@supports (display: grid) {
.X {
  display: grid;
}
}
}
.X:not(.x) + .X {
  margin: 0;
}
.X::before {
  content: '*';
}
.X[aria-current=page] {
  font-weight: 700;
}"
    );
}

#[test]
fn selector_variants_rewrite_the_selector() {
    let t = theme(json!({ "variants": { "rtl": "[dir=rtl] &", "open": "&[open]" } }));
    let b = block(&[]).nest("@rtl", block(&[("margin-left", "0")])).nest("@open", block(&[("display", "block")]));
    assert_eq!(css_of(&t, &b), "[dir=rtl] .X {\n  margin-left: 0;\n}\n.X[open] {\n  display: block;\n}");
}

#[test]
fn dynamic_vars_are_numbered_across_nested_blocks() {
    let t = Theme::default();
    let tpl = |s: &str| StyleValue::Dynamic { tpl: Template::parse(s).unwrap(), px: true };
    let mut root = block(&[("color", "red")]);
    root.push("width", tpl("{{ w }}"));
    let mut hover = StyleBlock::new();
    hover.push("height", tpl("{{ h }}"));
    let root = root.nest(":hover", hover);
    let mut c = StyleCompiler::new(&t);
    let out = c.compile(&root, Layer::Instance).unwrap();
    let names: Vec<_> = out.vars.iter().map(|v| v.name.as_str()).collect();
    assert_eq!(names, ["--i-d0", "--i-d1"]);
    assert!(c.css().contains("height: var(--i-d1);"));
}

#[test]
fn layers_separate_component_and_document_styles() {
    let t = Theme::default();
    let mut c = StyleCompiler::new(&t);
    let base = c.compile(&block(&[("color", "gray")]), Layer::Base).unwrap();
    let inst = c.compile(&block(&[("color", "black")]), Layer::Instance).unwrap();
    let css = c.css();
    let base_at = css.find(&format!(".{} {{", base.class)).unwrap();
    let inst_at = css.find(&format!(".{} {{", inst.class)).unwrap();
    let base_layer = css.find("@layer i-base {").unwrap();
    let inst_layer = css.find("@layer i-instance {").unwrap();
    assert!(base_layer < base_at && base_at < inst_layer && inst_layer < inst_at);
}

#[test]
fn theme_css_for_at_rule_and_selector_modes() {
    let t = theme(json!({
        "tokens": { "brand": "#123" },
        "variants": { "hc": "[data-contrast=high] &", "print": "@media print" },
        "modes": { "hc": { "brand": "#000" }, "print": { "brand": "black" } }
    }));
    let css = StyleCompiler::new(&t).theme_css();
    assert!(css.contains(":root {\n  --color-primary: #4f46e5;"), "{css}");
    assert!(css.contains("  --brand: #123;\n"), "{css}");
    assert!(css.contains("[data-contrast=high] {\n  --brand: #000;\n}"), "{css}");
    assert!(css.contains("@media print {\n:root {\n  --brand: black;\n}\n}"), "{css}");
}

#[cfg(feature = "lightningcss")]
#[test]
fn compiled_css_round_trips_through_lightningcss() {
    let t = Theme::default();
    let mut c = StyleCompiler::new(&t);
    c.compile(&block(&[("color", "red")]).nest("@md", block(&[("padding", "1px")])), Layer::Instance);
    c.compile(&block(&[("display", "flex")]).nest(":hover", block(&[("opacity", ".5")])), Layer::Base);
    let min = irender_style::minify(&c.css()).unwrap();
    assert!(min.contains("@media (width>=768px)") || min.contains("@media (min-width:768px)"), "{min}");
    assert!(!min.contains('\n'));
}
