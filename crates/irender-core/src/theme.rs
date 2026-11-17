use crate::{Diagnostics, expr::stringify, policy::css_value_is_contained};
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

/// The `theme` section of a document. Everything here is merged over the defaults.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct ThemeSpec {
    /// Design tokens. Nested objects flatten with `.`: `{"color": {"primary": "#4f46e5"}}`
    /// is used in styles as `$color.primary` and compiles to `var(--color-primary)`.
    pub tokens: IndexMap<String, Json>,
    /// Token overrides applied under a variant, e.g. `{"dark": {"color": {"surface": "#111"}}}`.
    pub modes: IndexMap<String, IndexMap<String, Json>>,
    /// Breakpoints usable as `@name` in styles; the value is a min-width (`"768px"`).
    pub breakpoints: IndexMap<String, String>,
    /// Variants usable as `@name` in styles: an at-rule (`"@media print"`) or a
    /// selector containing `&` (`"[data-theme=dark] &"`).
    pub variants: IndexMap<String, String>,
}

/// A resolved theme: flat tokens plus the named conditions styles can use.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    tokens: IndexMap<String, String>,
    modes: IndexMap<String, IndexMap<String, String>>,
    breakpoints: IndexMap<String, String>,
    variants: IndexMap<String, String>,
}

/// What a style key like `@md` or `@dark` expands to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    /// Wrap the rule: `@media (min-width: 768px)`.
    AtRule(String),
    /// Rewrite the selector; `&` stands for the element.
    Selector(String),
}

fn map(pairs: &[(&str, &str)]) -> IndexMap<String, String> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

impl Default for Theme {
    fn default() -> Self {
        let tokens = map(&[
            ("color.primary", "#4f46e5"),
            ("color.primary-hover", "#4338ca"),
            ("color.on-primary", "#ffffff"),
            ("color.surface", "#ffffff"),
            ("color.text", "#111827"),
            ("color.muted", "#6b7280"),
            ("color.border", "#e5e7eb"),
            ("radius.sm", "4px"),
            ("radius.md", "8px"),
            ("radius.lg", "12px"),
            ("space.xs", "4px"),
            ("space.sm", "8px"),
            ("space.md", "16px"),
            ("space.lg", "24px"),
            ("space.xl", "32px"),
            ("font.body", "system-ui, -apple-system, 'Segoe UI', sans-serif"),
            ("font.size.sm", "14px"),
            ("font.size.md", "16px"),
            ("font.size.lg", "20px"),
        ]);
        let mut modes = IndexMap::new();
        modes.insert(
            "dark".to_string(),
            map(&[
                ("color.surface", "#111827"),
                ("color.text", "#f9fafb"),
                ("color.muted", "#9ca3af"),
                ("color.border", "#374151"),
            ]),
        );
        Self {
            tokens,
            modes,
            breakpoints: map(&[("sm", "640px"), ("md", "768px"), ("lg", "1024px"), ("xl", "1280px")]),
            variants: map(&[
                ("dark", "@media (prefers-color-scheme: dark)"),
                ("print", "@media print"),
                ("motion-safe", "@media (prefers-reduced-motion: no-preference)"),
                ("motion-reduce", "@media (prefers-reduced-motion: reduce)"),
            ]),
        }
    }
}

impl Theme {
    pub fn resolve(spec: &ThemeSpec, diags: &mut Diagnostics) -> Theme {
        let mut theme = Theme::default();
        for (k, v) in &spec.tokens {
            flatten(k, v, &mut theme.tokens, "theme.tokens", diags);
        }
        for (k, v) in &spec.breakpoints {
            check_value(v, &format!("theme.breakpoints.{k}"), diags);
            theme.breakpoints.insert(k.clone(), v.clone());
        }
        for (k, v) in &spec.variants {
            let path = format!("theme.variants.{k}");
            if !v.starts_with('@') && !v.contains('&') {
                diags.push(&path, "a variant is an at-rule (`@media …`) or a selector containing `&`");
            }
            check_value(v, &path, diags);
            theme.variants.insert(k.clone(), v.clone());
        }
        for (mode, tokens) in &spec.modes {
            let path = format!("theme.modes.{mode}");
            if !theme.variants.contains_key(mode) {
                diags.push(&path, format!("`{mode}` is not a variant; declare it in `theme.variants`"));
            }
            let target = theme.modes.entry(mode.clone()).or_default();
            for (k, v) in tokens {
                flatten(k, v, target, &path, diags);
            }
        }
        theme
    }

    pub fn tokens(&self) -> &IndexMap<String, String> {
        &self.tokens
    }

    pub fn modes(&self) -> &IndexMap<String, IndexMap<String, String>> {
        &self.modes
    }

    pub fn has_token(&self, name: &str) -> bool {
        self.tokens.contains_key(name) || self.modes.values().any(|m| m.contains_key(name))
    }

    /// `color.primary` → `--color-primary`.
    pub fn var_name(token: &str) -> String {
        format!("--{}", token.replace('.', "-"))
    }

    /// Looks up `@name`: breakpoints first, then variants.
    pub fn condition(&self, name: &str) -> Option<Condition> {
        if let Some(min) = self.breakpoints.get(name) {
            return Some(Condition::AtRule(format!("@media (min-width: {min})")));
        }
        self.variants.get(name).map(|v| {
            if v.starts_with('@') {
                Condition::AtRule(v.clone())
            } else {
                Condition::Selector(v.clone())
            }
        })
    }

    pub fn condition_names(&self) -> impl Iterator<Item = &str> {
        self.breakpoints.keys().chain(self.variants.keys()).map(String::as_str)
    }

    /// Replaces `$token.name` with `var(--token-name)`. `$$` is a literal `$`.
    pub fn substitute(&self, value: &str) -> Result<String, String> {
        if !value.contains('$') {
            return Ok(value.to_string());
        }
        let mut out = String::with_capacity(value.len() + 16);
        let mut chars = value.char_indices().peekable();
        while let Some((i, c)) = chars.next() {
            if c != '$' {
                out.push(c);
                continue;
            }
            if let Some(&(_, '$')) = chars.peek() {
                chars.next();
                out.push('$');
                continue;
            }
            let start = i + 1;
            let mut end = start;
            while let Some(&(j, d)) = chars.peek() {
                if d.is_ascii_alphanumeric() || matches!(d, '_' | '-' | '.') {
                    end = j + 1;
                    chars.next();
                } else {
                    break;
                }
            }
            // A trailing `.` is punctuation, not part of the token name.
            let name = value[start..end].trim_end_matches('.');
            if name.is_empty() {
                return Err("`$` must be followed by a token name (use `$$` for a literal `$`)".into());
            }
            if !self.has_token(name) {
                return Err(format!("unknown theme token `${name}`"));
            }
            out.push_str(&format!("var({})", Self::var_name(name)));
            out.push_str(&value[start + name.len()..end]);
        }
        Ok(out)
    }
}

fn check_value(v: &str, path: &str, diags: &mut Diagnostics) {
    if let Err(e) = css_value_is_contained(v) {
        diags.push(path, e);
    }
}

fn flatten(key: &str, v: &Json, out: &mut IndexMap<String, String>, path: &str, diags: &mut Diagnostics) {
    let path = format!("{path}.{key}");
    if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        diags.push(&path, "token names may only contain letters, digits, `-` and `_`");
        return;
    }
    match v {
        Json::Object(children) => {
            for (k, child) in children {
                let mut nested = IndexMap::new();
                flatten(k, child, &mut nested, &path, diags);
                out.extend(nested.into_iter().map(|(n, v)| (format!("{key}.{n}"), v)));
            }
        }
        Json::String(_) | Json::Number(_) => {
            let s = stringify(v);
            check_value(&s, &path, diags);
            out.insert(key.to_string(), s);
        }
        _ => diags.push(&path, "a token must be a string, a number or an object of tokens"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tokens_flatten_and_substitute() {
        let spec: ThemeSpec = serde_json::from_value(json!({
            "tokens": { "color": { "brand": { "500": "#0f766e" } } }
        }))
        .unwrap();
        let mut d = Diagnostics::default();
        let t = Theme::resolve(&spec, &mut d);
        assert!(d.is_empty(), "{d}");
        assert_eq!(
            t.substitute("1px solid $color.brand.500.").unwrap(),
            "1px solid var(--color-brand-500)."
        );
        assert_eq!(t.substitute("$$5").unwrap(), "$5");
        assert!(t.substitute("$nope").is_err());
    }
}
