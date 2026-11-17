//! Compiles [`StyleBlock`]s into CSS.
//!
//! Every block becomes one class named after a hash of its CSS, so identical styles
//! share a class. Values computed at render time compile to `var(--i-dN)` and are set
//! per element (inline on the server, via `style.setProperty` in the browser).
//!
//! Precedence uses cascade layers, so it doesn't depend on rule order:
//! theme variables, then `i-base` (component defaults), then `i-instance` (the
//! document's `style`), which always wins.

use irender_core::{StyleBlock, StyleValue, Template, Theme, theme::Condition};
use indexmap::IndexMap;
use std::{
    collections::HashMap,
    fmt::Write,
    hash::{Hash, Hasher},
    rc::Rc,
};

#[cfg(feature = "lightningcss")]
mod validate;
#[cfg(feature = "lightningcss")]
pub use validate::{minify, validate, validate_declaration};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layer {
    /// Component defaults.
    Base,
    /// Styles from the document; always override `Base`.
    Instance,
}

/// A render-time value delivered through a CSS custom property.
#[derive(Debug, Clone, PartialEq)]
pub struct DynVar {
    pub name: String,
    pub tpl: Template,
    /// Append `px` when the value is a bare number.
    pub px: bool,
    /// The property it sets, when that declaration is on the element itself (no nested
    /// selector, condition or at-rule). A CSS-wide keyword (`inherit`, `unset`, …)
    /// can't pass through a custom property, so renderers set this property to it
    /// inline instead.
    pub property: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Compiled {
    pub class: String,
    pub vars: Vec<DynVar>,
}

/// Compiled CSS rules by class, detached from a compiler so they can cross threads.
#[derive(Debug, Clone, Default)]
pub struct Rules {
    base: IndexMap<String, String>,
    instance: IndexMap<String, String>,
    globals: IndexMap<&'static str, &'static str>,
}

pub struct StyleCompiler<'t> {
    theme: &'t Theme,
    base: IndexMap<String, String>,
    instance: IndexMap<String, String>,
    /// Page-level rules components need once, such as `@keyframes`, by name.
    globals: IndexMap<&'static str, &'static str>,
    /// Blocks already compiled, by content. A list renders the same blocks once per
    /// item; this turns every repeat into a hash and a comparison.
    cache: HashMap<u64, Vec<(StyleBlock, Layer, Rc<Compiled>)>>,
}

const PLACEHOLDER: &str = ".__irender__";

impl<'t> StyleCompiler<'t> {
    pub fn new(theme: &'t Theme) -> Self {
        Self { theme, base: IndexMap::new(), instance: IndexMap::new(), globals: IndexMap::new(), cache: HashMap::new() }
    }

    /// Adds a page-level rule, such as `@keyframes`, once per `name`. Later calls
    /// with the same name are ignored. The CSS is trusted: it comes from component
    /// code (it is `'static`), never from a document, and isn't validated.
    pub fn global(&mut self, name: &'static str, css: &'static str) {
        self.globals.entry(name).or_insert(css);
    }

    /// Returns the class for `block`, or `None` when the block is empty.
    pub fn compile(&mut self, block: &StyleBlock, layer: Layer) -> Option<Rc<Compiled>> {
        if block.is_empty() {
            return None;
        }
        let mut hasher = Fnv::default();
        block.hash(&mut hasher);
        layer.hash(&mut hasher);
        let key = hasher.finish();
        if let Some(hit) = self.cache.get(&key).and_then(|entries| {
            entries.iter().find(|(b, l, _)| *l == layer && b == block).map(|(.., c)| c.clone())
        }) {
            return Some(hit);
        }
        let compiled = Rc::new(self.compile_uncached(block, layer));
        self.cache.entry(key).or_default().push((block.clone(), layer, compiled.clone()));
        Some(compiled)
    }

    fn compile_uncached(&mut self, block: &StyleBlock, layer: Layer) -> Compiled {
        let mut css = String::new();
        let mut vars = Vec::new();
        self.emit(block, PLACEHOLDER, &[], &mut css, &mut vars);
        let class = format!("i-{}", hash(&css));
        let css = css.replace(PLACEHOLDER, &format!(".{class}"));
        let rules = match layer {
            Layer::Base => &mut self.base,
            Layer::Instance => &mut self.instance,
        };
        rules.entry(class.clone()).or_insert(css);
        Compiled { class, vars }
    }

    fn emit(&self, block: &StyleBlock, selector: &str, wrappers: &[String], css: &mut String, vars: &mut Vec<DynVar>) {
        if !block.decls.is_empty() {
            for w in wrappers {
                let _ = writeln!(css, "{w} {{");
            }
            let _ = writeln!(css, "{selector} {{");
            for d in &block.decls {
                let value = match &d.value {
                    StyleValue::Static(v) => v.clone(),
                    StyleValue::Dynamic { tpl, px } => {
                        let name = format!("--i-d{}", vars.len());
                        let value = format!("var({name})");
                        let own = selector == PLACEHOLDER && wrappers.is_empty();
                        vars.push(DynVar { name, tpl: tpl.clone(), px: *px, property: own.then(|| d.property.clone()) });
                        value
                    }
                };
                let _ = writeln!(css, "  {}: {};", d.property, value);
            }
            css.push_str("}\n");
            for _ in wrappers {
                css.push_str("}\n");
            }
        }
        for (key, child) in &block.nested {
            if let Some(name) = key.strip_prefix('@') {
                let condition = if key.contains(' ') {
                    Some(Condition::AtRule(key.clone()))
                } else {
                    self.theme.condition(name)
                };
                match condition {
                    Some(Condition::AtRule(rule)) => {
                        let mut w = wrappers.to_vec();
                        w.push(rule);
                        self.emit(child, selector, &w, css, vars);
                    }
                    Some(Condition::Selector(s)) => {
                        self.emit(child, &s.replace('&', selector), wrappers, css, vars)
                    }
                    // Unknown conditions are rejected by `resolve`.
                    None => {}
                }
            } else if key.contains('&') {
                self.emit(child, &key.replace('&', selector), wrappers, css, vars);
            } else {
                self.emit(child, &format!("{selector}{key}"), wrappers, css, vars);
            }
        }
    }

    /// Takes the rules compiled so far, e.g. from a compiler used on another thread.
    pub fn into_rules(self) -> Rules {
        Rules { base: self.base, instance: self.instance, globals: self.globals }
    }

    /// Adds rules from another compiler. Rules already present keep their position, so
    /// merging per-chunk compilers in chunk order gives the same stylesheet as compiling
    /// everything in one compiler.
    pub fn merge(&mut self, rules: Rules) {
        for (class, css) in rules.base {
            self.base.entry(class).or_insert(css);
        }
        for (class, css) in rules.instance {
            self.instance.entry(class).or_insert(css);
        }
        for (name, css) in rules.globals {
            self.globals.entry(name).or_insert(css);
        }
    }

    /// `:root` variables for every theme token, plus mode overrides.
    pub fn theme_css(&self) -> String {
        let mut css = String::new();
        let vars = |tokens: &IndexMap<String, String>, css: &mut String| {
            for (k, v) in tokens {
                let _ = writeln!(css, "  {}: {};", Theme::var_name(k), v);
            }
        };
        css.push_str(":root {\n");
        vars(self.theme.tokens(), &mut css);
        css.push_str("}\n");
        for (mode, tokens) in self.theme.modes() {
            match self.theme.condition(mode) {
                Some(Condition::AtRule(rule)) => {
                    let _ = writeln!(css, "{rule} {{\n:root {{");
                    vars(tokens, &mut css);
                    css.push_str("}\n}\n");
                }
                Some(Condition::Selector(s)) => {
                    // `[data-theme=dark] &` sets the variables on the matching ancestor itself.
                    let trimmed = s.trim_end();
                    let sel = match trimmed.strip_suffix('&') {
                        Some(prefix) if !prefix.trim().is_empty() => prefix.trim().to_string(),
                        _ => s.replace('&', ":root"),
                    };
                    let _ = writeln!(css, "{sel} {{");
                    vars(tokens, &mut css);
                    css.push_str("}\n");
                }
                None => {}
            }
        }
        // Theme options: each choice on `:root[data-theme-<option>=…]`, then its own
        // overrides per variant, after the base modes so a chosen palette's dark colours
        // win over the base dark ones.
        for (name, option) in self.theme.options() {
            let attr = irender_core::theme::ThemeOption::attribute(name);
            for (choice, c) in &option.choices {
                let on = format!(":root[{attr}=\"{choice}\"]");
                if !c.tokens.is_empty() {
                    let _ = writeln!(css, "{on} {{");
                    vars(&c.tokens, &mut css);
                    css.push_str("}\n");
                }
                for (mode, tokens) in &c.modes {
                    match self.theme.condition(mode) {
                        Some(Condition::AtRule(rule)) => {
                            let _ = writeln!(css, "{rule} {{\n{on} {{");
                            vars(tokens, &mut css);
                            css.push_str("}\n}\n");
                        }
                        Some(Condition::Selector(s)) => {
                            // `[data-theme=dark] &` applies to the root itself here.
                            let sel = match s.trim_end().strip_suffix('&') {
                                Some(prefix) if !prefix.trim().is_empty() => format!("{}[{attr}=\"{choice}\"]", prefix.trim()),
                                _ => s.replace('&', &on),
                            };
                            let _ = writeln!(css, "{sel} {{");
                            vars(tokens, &mut css);
                            css.push_str("}\n");
                        }
                        None => {}
                    }
                }
            }
        }
        css
    }

    /// The complete stylesheet.
    pub fn css(&self) -> String {
        let layer = |rules: &IndexMap<String, String>| rules.values().map(String::as_str).collect::<String>();
        let globals: String = self.globals.values().map(|css| format!("{}\n", css.trim_end())).collect();
        format!(
            "@layer i-base, i-instance;\n{}{globals}@layer i-base {{\n{}}}\n@layer i-instance {{\n{}}}\n",
            self.theme_css(),
            layer(&self.base),
            layer(&self.instance),
        )
    }
}

/// FNV-1a. Stable across Rust versions, unlike `DefaultHasher`, so anything derived
/// from it (class names, ids) stays the same after a toolchain upgrade.
pub struct Fnv(u64);

impl Default for Fnv {
    fn default() -> Self {
        Fnv(0xcbf2_9ce4_8422_2325)
    }
}

impl Hasher for Fnv {
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 ^= u64::from(*b);
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

/// Class-name hash of the CSS text, in base36.
fn hash(s: &str) -> String {
    let mut fnv = Fnv::default();
    fnv.write(s.as_bytes());
    let mut h = fnv.finish();
    let mut out = Vec::new();
    while h > 0 {
        out.push(b"0123456789abcdefghijklmnopqrstuvwxyz"[(h % 36) as usize]);
        h /= 36;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedupes_and_nests() {
        let theme = Theme::default();
        let mut c = StyleCompiler::new(&theme);
        let block = StyleBlock::from_pairs(&[("color", "red")])
            .nest(":hover", StyleBlock::from_pairs(&[("color", "blue")]))
            .nest("@md", StyleBlock::from_pairs(&[("padding", "8px")]))
            .nest("& > span", StyleBlock::from_pairs(&[("margin", "0")]));
        let a = c.compile(&block, Layer::Instance).unwrap();
        let b = c.compile(&block, Layer::Instance).unwrap();
        assert_eq!(a.class, b.class);
        let css = c.css();
        let cls = &a.class;
        assert!(css.contains(&format!(".{cls}:hover {{\n  color: blue;")), "{css}");
        assert!(css.contains(&format!("@media (min-width: 768px) {{\n.{cls} {{\n  padding: 8px;")), "{css}");
        assert!(css.contains(&format!(".{cls} > span {{")), "{css}");
        assert_eq!(css.matches(&format!(".{cls} {{\n  color: red;")).count(), 1);
    }

    #[test]
    fn cache_returns_the_same_compiled_block() {
        let theme = Theme::default();
        let mut c = StyleCompiler::new(&theme);
        let block = StyleBlock::from_pairs(&[("color", "red")]);
        let a = c.compile(&block, Layer::Instance).unwrap();
        let b = c.compile(&block.clone(), Layer::Instance).unwrap();
        assert!(Rc::ptr_eq(&a, &b), "equal content hits the cache");
        let base = c.compile(&block, Layer::Base).unwrap();
        assert_eq!(base.class, a.class, "same class name in either layer");
        assert!(c.css().contains("@layer i-base {\n.") && c.css().matches("color: red;").count() == 2);
        let other = c.compile(&StyleBlock::from_pairs(&[("color", "blue")]), Layer::Instance).unwrap();
        assert_ne!(other.class, a.class);
    }

    #[test]
    fn dynamic_values_become_vars() {
        let theme = Theme::default();
        let mut c = StyleCompiler::new(&theme);
        let mut block = StyleBlock::new();
        block.push("width", StyleValue::Dynamic { tpl: Template::parse("{{ w }}").unwrap(), px: true });
        let out = c.compile(&block, Layer::Instance).unwrap();
        assert_eq!(out.vars[0].name, "--i-d0");
        assert!(c.css().contains("width: var(--i-d0);"));
    }

    #[test]
    fn layers_and_theme() {
        let theme = Theme::default();
        let c = StyleCompiler::new(&theme);
        let css = c.css();
        assert!(css.starts_with("@layer i-base, i-instance;"));
        assert!(css.contains("--color-primary: #4f46e5;"));
        assert!(css.contains("@media (prefers-color-scheme: dark) {\n:root {\n  --color-surface: #111827;"));
    }

    #[test]
    fn global_rules_appear_once_in_first_use_order_and_survive_merges() {
        let theme = Theme::default();
        let mut a = StyleCompiler::new(&theme);
        a.global("spin", "@keyframes spin { to { rotate: 1turn; } }");
        a.global("pulse", "@keyframes pulse { 50% { opacity: .5; } }");
        a.global("spin", "@keyframes spin { ignored }");
        let mut b = StyleCompiler::new(&theme);
        b.global("pulse", "@keyframes pulse { 50% { opacity: .5; } }");
        b.global("fade", "@keyframes fade { from { opacity: 0; } }");
        a.merge(b.into_rules());
        let css = a.css();
        assert_eq!(css.matches("@keyframes").count(), 3, "{css}");
        let (spin, pulse, fade) = (css.find("spin").unwrap(), css.find("pulse").unwrap(), css.find("fade").unwrap());
        assert!(spin < pulse && pulse < fade, "{css}");
        // Outside the layers, after the theme, and the layer order statement stays first.
        assert!(css.starts_with("@layer i-base, i-instance;"));
        assert!(css.find(":root").unwrap() < spin && fade < css.find("@layer i-base {").unwrap(), "{css}");
    }

    #[test]
    fn theme_options_compile_to_root_attribute_rules() {
        let spec: irender_core::theme::ThemeSpec = serde_json::from_value(serde_json::json!({ "options": { "palette": { "choices": {
            "zinc": { "tokens": { "color": { "primary": "#18181b" } } },
            "rose": { "tokens": { "color": { "primary": "#e11d48" } }, "modes": { "dark": { "color": { "primary": "#fb7185" } } } }
        } } } }))
        .unwrap();
        let theme = Theme::resolve(&spec, &mut irender_core::Diagnostics::default());
        let css = StyleCompiler::new(&theme).theme_css();
        assert!(css.contains(":root[data-theme-palette=\"rose\"] {\n  --color-primary: #e11d48;\n}"), "{css}");
        // Its dark colours come after the base dark mode, so they win in dark mode.
        let dark = css.find("@media (prefers-color-scheme: dark) {\n:root[data-theme-palette=\"rose\"] {\n  --color-primary: #fb7185;").unwrap();
        assert!(dark > css.find("@media (prefers-color-scheme: dark) {\n:root {").unwrap(), "{css}");
    }
}
