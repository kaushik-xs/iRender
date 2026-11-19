//! A document's interface text in its own language: the kits' fixed words ("Rows per
//! page", "Select all", "{n} selected") looked up by their English, with the
//! document's `labels` in place of any it gives.
//!
//! A label's key is the English text, with `{name}` where a value goes. A value is the
//! text to use, or for a count (`{n}`), its plural forms: `{ one, other }`, and `zero`,
//! `two`, `few` or `many` where the language has them (the browser picks with
//! `Intl.PluralRules` for `lang`; the server knows only `one` and `other`).

use crate::Diagnostics;
use indexmap::IndexMap;
use serde_json::{Value as Json, json};

/// Plural categories a label may have, as `Intl.PluralRules` names them.
pub const PLURALS: &[&str] = &["zero", "one", "two", "few", "many", "other"];

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Labels {
    /// The document's language (`lang`), e.g. `es` or `pt-BR`.
    pub lang: Option<String>,
    map: IndexMap<String, Label>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Label {
    Text(String),
    /// Plural forms by category; `other` is always there.
    Plural(IndexMap<String, String>),
}

impl Labels {
    /// Reads `lang` and `labels`, reporting what's wrong with them.
    pub fn resolve(lang: Option<&str>, labels: &IndexMap<String, Json>, diags: &mut Diagnostics) -> Self {
        let lang = lang.map(str::trim).filter(|l| !l.is_empty());
        if let Some(l) = lang
            && !l.split('-').all(|p| !p.is_empty() && p.len() <= 8 && p.chars().all(|c| c.is_ascii_alphanumeric()))
        {
            diags.push("lang", format!("`{l}` isn't a language tag, such as `es` or `pt-BR`"));
        }
        let mut map = IndexMap::new();
        for (key, value) in labels {
            let at = format!("labels.{key}");
            match value {
                Json::String(s) => {
                    map.insert(key.clone(), Label::Text(s.clone()));
                }
                Json::Object(o) => {
                    let mut forms = IndexMap::new();
                    for (k, v) in o {
                        match v {
                            Json::String(s) if PLURALS.contains(&k.as_str()) => {
                                forms.insert(k.clone(), s.clone());
                            }
                            Json::String(_) => diags.push(format!("{at}.{k}"), format!("a plural form is one of {}", PLURALS.join(", "))),
                            _ => diags.push(format!("{at}.{k}"), "a plural form is text"),
                        }
                    }
                    if !forms.contains_key("other") {
                        diags.push(at, "plural forms need `other`, the form for most counts");
                    } else {
                        map.insert(key.clone(), Label::Plural(forms));
                    }
                }
                _ => diags.push(at, "a label is text, or plural forms such as `{ one: …, other: … }`"),
            }
        }
        Labels { lang: lang.map(String::from), map }
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// `text`, in the document's words, with `{name}`s filled from `vars`.
    pub fn get(&self, text: &str, vars: &[(&str, &str)]) -> String {
        let s = match self.map.get(text) {
            Some(Label::Text(s)) => s.as_str(),
            Some(Label::Plural(forms)) => forms.get("other").map_or(text, String::as_str),
            None => text,
        };
        fill(s, vars)
    }

    /// A count: `one` or `other` in English (`{n} row`, `{n} rows`), looked up by the
    /// `other` form, with `{n}` and any `vars` filled in.
    pub fn count(&self, one: &str, other: &str, n: f64, vars: &[(&str, &str)]) -> String {
        let category = if n == 1.0 { "one" } else { "other" };
        let s = match self.map.get(other) {
            Some(Label::Plural(forms)) => forms.get(category).or_else(|| forms.get("other")).map_or(other, String::as_str),
            Some(Label::Text(s)) => s.as_str(),
            None if n == 1.0 => one,
            None => other,
        };
        let n = if n.fract() == 0.0 { format!("{}", n as i64) } else { n.to_string() };
        let mut all = vec![("n", n.as_str())];
        all.extend_from_slice(vars);
        fill(s, &all)
    }

    /// For the browser: `{ lang, labels }`, labels as the document gave them.
    pub fn to_json(&self) -> Json {
        let labels: serde_json::Map<String, Json> = self
            .map
            .iter()
            .map(|(k, v)| {
                (k.clone(), match v {
                    Label::Text(s) => Json::String(s.clone()),
                    Label::Plural(f) => Json::Object(f.iter().map(|(c, s)| (c.clone(), Json::String(s.clone()))).collect()),
                })
            })
            .collect();
        json!({ "lang": self.lang, "labels": labels })
    }
}

/// `{name}` replaced by its value; a `{name}` without one stays.
fn fill(s: &str, vars: &[(&str, &str)]) -> String {
    if vars.is_empty() || !s.contains('{') {
        return s.to_string();
    }
    let mut out = s.to_string();
    for (k, v) in vars {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(v: Json) -> (Labels, Diagnostics) {
        let mut d = Diagnostics::default();
        let map: IndexMap<String, Json> = serde_json::from_value(v).unwrap();
        (Labels::resolve(Some("es"), &map, &mut d), d)
    }

    #[test]
    fn looks_up_fills_and_counts() {
        let (l, d) = labels(json!({ "Rows per page": "Filas por página", "Page {page} of {pages}": "Página {page} de {pages}",
            "{n} rows": { "one": "{n} fila", "other": "{n} filas" } }));
        assert!(d.is_empty());
        assert_eq!(l.get("Rows per page", &[]), "Filas por página");
        assert_eq!(l.get("Select all", &[]), "Select all", "English when not given");
        assert_eq!(l.get("Page {page} of {pages}", &[("page", "2"), ("pages", "5")]), "Página 2 de 5");
        assert_eq!(l.count("{n} row", "{n} rows", 1.0, &[]), "1 fila");
        assert_eq!(l.count("{n} row", "{n} rows", 3.0, &[]), "3 filas");
        assert_eq!(l.count("{n} item", "{n} items", 1.0, &[]), "1 item");
        assert_eq!(l.count("{n} item", "{n} items", 2.5, &[]), "2.5 items");
    }

    #[test]
    fn reports_bad_labels() {
        let (_, d) = labels(json!({ "a": 3, "b": { "one": "x" }, "c": { "other": "y", "dual": "z" } }));
        let text = d.to_string();
        assert!(text.contains("labels.a: a label is text"), "{text}");
        assert!(text.contains("labels.b: plural forms need `other`"), "{text}");
        assert!(text.contains("labels.c.dual: a plural form is one of"), "{text}");
        let mut d = Diagnostics::default();
        Labels::resolve(Some("en_US"), &IndexMap::new(), &mut d);
        assert!(d.to_string().contains("isn't a language tag"));
    }
}
