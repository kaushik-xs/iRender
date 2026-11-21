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
    /// Sets of token overrides people choose between on the page, such as a palette,
    /// a corner radius or a density: `{"palette": {"choices": {"rose": {"tokens": …}}}}`.
    /// A choice applies through `data-theme-<option>` on `<html>`, or on any element
    /// for the part of the page inside it; the default one is part of the base
    /// tokens, so a page with nothing chosen looks like it.
    pub options: IndexMap<String, OptionSpec>,
}

/// One theme option: its choices, and which one applies until another is picked.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct OptionSpec {
    /// What a picker calls the option, e.g. `Colour`. Default: its name.
    pub label: Option<String>,
    /// The choice that applies until someone picks another. Default: the first.
    pub default: Option<String>,
    pub choices: IndexMap<String, ChoiceSpec>,
}

/// One choice of a theme option.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct ChoiceSpec {
    /// What a picker calls it, e.g. `Rose`. Default: its name.
    pub label: Option<String>,
    /// Token overrides while it's chosen, nested like `theme.tokens`.
    pub tokens: IndexMap<String, Json>,
    /// Token overrides while it's chosen and a variant applies, like `theme.modes`:
    /// `{"dark": {"color": {"primary": "#fb7185"}}}`.
    pub modes: IndexMap<String, IndexMap<String, Json>>,
}

/// A resolved theme: flat tokens plus the named conditions styles can use.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    tokens: IndexMap<String, String>,
    modes: IndexMap<String, IndexMap<String, String>>,
    breakpoints: IndexMap<String, String>,
    variants: IndexMap<String, String>,
    options: IndexMap<String, ThemeOption>,
}

/// A resolved theme option (see [`ThemeSpec::options`]).
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeOption {
    pub label: String,
    pub default: String,
    pub choices: IndexMap<String, ThemeChoice>,
}

/// A resolved choice: flat tokens and per-variant overrides.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeChoice {
    pub label: String,
    pub tokens: IndexMap<String, String>,
    pub modes: IndexMap<String, IndexMap<String, String>>,
}

impl ThemeOption {
    /// The attribute on `<html>` that holds the choice: `data-theme-palette`.
    pub fn attribute(name: &str) -> String {
        format!("data-theme-{name}")
    }
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
            // Spacing and font sizes follow `space.scale` and `font.scale`, which a
            // density or text size option sets.
            ("space.scale", "1"),
            ("space.xs", "calc(4px * var(--space-scale, 1))"),
            ("space.sm", "calc(8px * var(--space-scale, 1))"),
            ("space.md", "calc(16px * var(--space-scale, 1))"),
            ("space.lg", "calc(24px * var(--space-scale, 1))"),
            ("space.xl", "calc(32px * var(--space-scale, 1))"),
            ("font.body", "system-ui, -apple-system, 'Segoe UI', sans-serif"),
            ("font.scale", "1"),
            ("font.size.sm", "calc(14px * var(--font-scale, 1))"),
            ("font.size.md", "calc(16px * var(--font-scale, 1))"),
            ("font.size.lg", "calc(20px * var(--font-scale, 1))"),
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
            options: IndexMap::new(),
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
        for (name, spec) in &spec.options {
            if let Some(option) = resolve_option(name, spec, &theme.variants, diags) {
                // The default choice is the base: a page with nothing picked shows it.
                let default = &option.choices[&option.default];
                theme.tokens.extend(default.tokens.clone());
                for (mode, tokens) in &default.modes {
                    theme.modes.entry(mode.clone()).or_default().extend(tokens.clone());
                }
                theme.options.insert(name.clone(), option);
            }
        }
        theme
    }

    /// The theme options, by name (see [`ThemeSpec::options`]).
    pub fn options(&self) -> &IndexMap<String, ThemeOption> {
        &self.options
    }

    pub fn tokens(&self) -> &IndexMap<String, String> {
        &self.tokens
    }

    pub fn modes(&self) -> &IndexMap<String, IndexMap<String, String>> {
        &self.modes
    }

    pub fn has_token(&self, name: &str) -> bool {
        self.tokens.contains_key(name)
            || self.modes.values().any(|m| m.contains_key(name))
            || self.options.values().flat_map(|o| o.choices.values()).any(|c| {
                c.tokens.contains_key(name) || c.modes.values().any(|m| m.contains_key(name))
            })
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

/// Option and choice names end up in attribute names and values, so they're kept plain.
fn plain_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn resolve_option(
    name: &str,
    spec: &OptionSpec,
    variants: &IndexMap<String, String>,
    diags: &mut Diagnostics,
) -> Option<ThemeOption> {
    let path = format!("theme.options.{name}");
    if !plain_name(name) {
        diags.push(&path, "option names may only contain lowercase letters, digits and `-`");
        return None;
    }
    if spec.choices.is_empty() {
        diags.push(format!("{path}.choices"), "an option needs at least one choice");
        return None;
    }
    let mut choices = IndexMap::new();
    for (choice, c) in &spec.choices {
        let cpath = format!("{path}.choices.{choice}");
        if !plain_name(choice) {
            diags.push(&cpath, "choice names may only contain lowercase letters, digits and `-`");
            continue;
        }
        let mut tokens = IndexMap::new();
        for (k, v) in &c.tokens {
            flatten(k, v, &mut tokens, &format!("{cpath}.tokens"), diags);
        }
        let mut modes: IndexMap<String, IndexMap<String, String>> = IndexMap::new();
        for (mode, over) in &c.modes {
            let mpath = format!("{cpath}.modes.{mode}");
            if !variants.contains_key(mode) {
                diags.push(&mpath, format!("`{mode}` is not a variant; declare it in `theme.variants`"));
            }
            let target = modes.entry(mode.clone()).or_default();
            for (k, v) in over {
                flatten(k, v, target, &mpath, diags);
            }
        }
        let label = c.label.clone().unwrap_or_else(|| choice.clone());
        choices.insert(choice.clone(), ThemeChoice { label, tokens, modes });
    }
    let default = match &spec.default {
        Some(d) if choices.contains_key(d) => d.clone(),
        Some(d) => {
            diags.push(format!("{path}.default"), format!("`{d}` is not one of its choices"));
            return None;
        }
        None => choices.keys().next()?.clone(),
    };
    Some(ThemeOption { label: spec.label.clone().unwrap_or_else(|| name.to_string()), default, choices })
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

    #[test]
    fn options_resolve_with_the_default_in_the_base() {
        let spec: ThemeSpec = serde_json::from_value(json!({
            "tokens": { "color": { "primary": "#000" } },
            "options": {
                "palette": { "label": "Colour", "default": "rose", "choices": {
                    "zinc": { "tokens": { "color": { "primary": "#18181b" } } },
                    "rose": { "label": "Rose", "tokens": { "color": { "primary": "#e11d48" } },
                              "modes": { "dark": { "color": { "primary": "#fb7185" } } } } } },
                "radius": { "choices": { "none": { "tokens": { "radius": { "md": "0px" } } } } }
            }
        }))
        .unwrap();
        let mut d = Diagnostics::default();
        let t = Theme::resolve(&spec, &mut d);
        assert!(d.is_empty(), "{d}");
        let palette = &t.options()["palette"];
        assert_eq!((palette.label.as_str(), palette.default.as_str()), ("Colour", "rose"));
        assert_eq!(palette.choices["zinc"].label, "zinc", "a choice's label defaults to its name");
        // The default choice is the base, in its modes too.
        assert_eq!(t.tokens()["color.primary"], "#e11d48");
        assert_eq!(t.modes()["dark"]["color.primary"], "#fb7185");
        assert_eq!(t.options()["radius"].default, "none", "the first choice when none is named");
        assert!(t.has_token("radius.md"));
        assert_eq!(ThemeOption::attribute("palette"), "data-theme-palette");
    }

    #[test]
    fn option_mistakes_are_reported_with_their_paths() {
        let spec: ThemeSpec = serde_json::from_value(json!({ "options": {
            "Palette": { "choices": { "a": {} } },
            "empty": { "choices": {} },
            "pick": { "default": "c", "choices": { "a": {} } },
            "modes": { "choices": { "a": { "modes": { "night": {} } }, "B C": {} } }
        } }))
        .unwrap();
        let mut d = Diagnostics::default();
        Theme::resolve(&spec, &mut d);
        let text = d.to_string();
        for expected in [
            "theme.options.Palette: option names may only contain lowercase letters, digits and `-`",
            "theme.options.empty.choices: an option needs at least one choice",
            "theme.options.pick.default: `c` is not one of its choices",
            "theme.options.modes.choices.a.modes.night: `night` is not a variant",
            "theme.options.modes.choices.B C: choice names may only contain",
        ] {
            assert!(text.contains(expected), "missing `{expected}` in:\n{text}");
        }
    }
}
