//! A document's interface text in its own language: the kits' fixed words ("Rows per
//! page", "Select all", "{n} selected") looked up by their English, with the
//! document's `labels` in place of any it gives.
//!
//! A label's key is the English text, with `{name}` where a value goes. A value is the
//! text to use, or for a count (`{n}`), its plural forms: `{ one, other }`, and `zero`,
//! `two`, `few` or `many` where the language has them. The browser picks with
//! `Intl.PluralRules` for `lang`; the server follows the same rules for common languages
//! ([`plural`]) and writes the count as the browser would ([`number`]).

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

    /// `rtl` for a language written right to left (Arabic, Hebrew, Persian, Urdu, …),
    /// else `ltr`.
    pub fn dir(&self) -> &'static str {
        match base(self.lang.as_deref()).as_str() {
            "ar" | "he" | "iw" | "fa" | "ur" | "ps" | "yi" | "dv" | "ug" | "ckb" | "sd" | "syr" => "rtl",
            _ => "ltr",
        }
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
    /// `other` form, with `{n}` (written in the language's way) and any `vars` filled in.
    pub fn count(&self, one: &str, other: &str, n: f64, vars: &[(&str, &str)]) -> String {
        let category = plural(self.lang.as_deref(), n);
        let s = match self.map.get(other) {
            Some(Label::Plural(forms)) => forms.get(category).or_else(|| forms.get("other")).map_or(other, String::as_str),
            Some(Label::Text(s)) => s.as_str(),
            None if n == 1.0 => one,
            None => other,
        };
        let n = number(self.lang.as_deref(), n);
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

/// The language part of a tag: `pt` for `pt-BR`.
fn base(lang: Option<&str>) -> String {
    lang.unwrap_or("en").split('-').next().unwrap_or("en").to_ascii_lowercase()
}

/// The plural category of `n` in `lang`, as `Intl.PluralRules` gives it, for the
/// languages people most often write documents in; others use English's `one` and
/// `other`. Fractions are `other`, except where a language says otherwise.
pub fn plural(lang: Option<&str>, n: f64) -> &'static str {
    let whole = n.fract() == 0.0 && n >= 0.0;
    let i = n.abs().trunc() as u64;
    let (m10, m100) = (i % 10, i % 100);
    let few = whole && (2..=4).contains(&m10) && !(12..=14).contains(&m100);
    match base(lang).as_str() {
        // No plural forms.
        "ja" | "zh" | "ko" | "th" | "vi" | "id" | "ms" => "other",
        // 0 and 1 (and what's between) are one.
        "fr" | "hi" | "bn" | "fa" | "pa" | "gu" | "mr" | "kn" | "am" | "zu" => {
            if n.abs() < 2.0 {
                "one"
            } else {
                "other"
            }
        }
        "pt" if lang.is_some_and(|l| !l.eq_ignore_ascii_case("pt-PT")) => {
            if n.abs() < 2.0 {
                "one"
            } else {
                "other"
            }
        }
        "ru" | "uk" | "be" => match () {
            _ if !whole => "other",
            _ if m10 == 1 && m100 != 11 => "one",
            _ if few => "few",
            _ => "many",
        },
        "pl" => match () {
            _ if !whole => "other",
            _ if i == 1 => "one",
            _ if few => "few",
            _ => "many",
        },
        "cs" | "sk" => match () {
            _ if !whole => "many",
            _ if i == 1 => "one",
            _ if (2..=4).contains(&i) => "few",
            _ => "other",
        },
        "ar" => match () {
            _ if !whole => "other",
            _ if i == 0 => "zero",
            _ if i == 1 => "one",
            _ if i == 2 => "two",
            _ if (3..=10).contains(&m100) => "few",
            _ if (11..=99).contains(&m100) => "many",
            _ => "other",
        },
        "he" => match () {
            _ if whole && i == 1 => "one",
            _ if whole && i == 2 => "two",
            _ => "other",
        },
        _ => {
            if whole && i == 1 {
                "one"
            } else {
                "other"
            }
        }
    }
}

/// `n` as `Intl.NumberFormat` writes it in `lang` (at most three decimals), for the
/// languages [`plural`] knows; others group with a comma, as English does.
pub fn number(lang: Option<&str>, n: f64) -> String {
    // Group separator, decimal mark, and the fewest digits before a group is made.
    let (group, decimal, min) = match base(lang).as_str() {
        "de" | "nl" | "id" | "tr" | "da" | "el" => (".", ",", 4),
        "it" => (".", ",", 5),
        "pt" if lang.is_some_and(|l| !l.eq_ignore_ascii_case("pt-PT")) => (".", ",", 4),
        "es" => (".", ",", 5),
        "pt" => ("\u{a0}", ",", 5),
        "pl" => ("\u{a0}", ",", 5),
        "fr" => ("\u{202f}", ",", 4),
        "ru" | "uk" | "cs" | "sk" | "sv" | "nb" | "no" | "fi" => ("\u{a0}", ",", 4),
        "be" | "hu" => ("\u{a0}", ",", 5),
        _ => (",", ".", 4),
    };
    let neg = n < 0.0;
    let s = format!("{:.3}", n.abs());
    let (int, frac) = s.split_once('.').unwrap_or((&s, ""));
    let frac = frac.trim_end_matches('0');
    let indian = matches!(base(lang).as_str(), "hi" | "bn" | "mr" | "gu" | "pa" | "kn") || lang.is_some_and(|l| l.eq_ignore_ascii_case("en-IN"));
    let grouped = if int.len() < min {
        int.to_string()
    } else if indian {
        // 12,34,567: three digits, then twos.
        let (head, last3) = int.split_at(int.len() - 3);
        let mut parts: Vec<&str> = Vec::new();
        let mut rest = head;
        while rest.len() > 2 {
            let (a, b) = rest.split_at(rest.len() - 2);
            parts.insert(0, b);
            rest = a;
        }
        if !rest.is_empty() {
            parts.insert(0, rest);
        }
        parts.push(last3);
        parts.join(group)
    } else {
        let bytes: Vec<char> = int.chars().collect();
        let mut out = String::new();
        for (k, c) in bytes.iter().enumerate() {
            if k > 0 && (bytes.len() - k).is_multiple_of(3) {
                out.push_str(group);
            }
            out.push(*c);
        }
        out
    };
    format!("{}{grouped}{}{}", if neg { "-" } else { "" }, if frac.is_empty() { "" } else { decimal }, frac)
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
        assert_eq!(l.count("{n} item", "{n} items", 2.5, &[]), "2,5 items", "es writes a decimal comma");
    }

    #[test]
    fn plural_rules_and_numbers_match_the_browsers() {
        let cats = |lang: &str, ns: &[f64]| ns.iter().map(|n| plural(Some(lang), *n)).collect::<Vec<_>>().join(" ");
        assert_eq!(cats("en", &[0.0, 1.0, 2.0, 1.5]), "other one other other");
        assert_eq!(cats("fr", &[0.0, 1.0, 1.5, 2.0]), "one one one other");
        assert_eq!(cats("pl", &[1.0, 2.0, 5.0, 12.0, 22.0, 25.0, 1.5]), "one few many many few many other");
        assert_eq!(cats("ru", &[1.0, 21.0, 11.0, 3.0, 13.0, 5.0]), "one one many few many many");
        assert_eq!(cats("cs", &[1.0, 3.0, 5.0, 1.5]), "one few other many");
        assert_eq!(cats("ar", &[0.0, 1.0, 2.0, 3.0, 11.0, 100.0]), "zero one two few many other");
        assert_eq!(cats("ja", &[1.0, 2.0]), "other other");
        assert_eq!(cats("pt-BR", &[0.0, 1.0, 2.0]), "one one other");
        assert_eq!(cats("pt-PT", &[0.0, 1.0, 2.0]), "other one other");
        let num = |lang: Option<&str>, n: f64| number(lang, n);
        assert_eq!(num(None, 1234567.5), "1,234,567.5");
        assert_eq!(num(Some("en"), 999.0), "999");
        assert_eq!(num(Some("de"), 1234.25), "1.234,25");
        assert_eq!(num(Some("es"), 1234.0), "1234", "Spanish groups from five digits");
        assert_eq!(num(Some("es"), 12345.0), "12.345");
        assert_eq!(num(Some("fr"), 1234.0), "1\u{202f}234");
        assert_eq!(num(Some("pl"), 1234.0), "1234");
        assert_eq!(num(Some("ru"), 1234.0), "1\u{a0}234");
        assert_eq!(num(Some("hi"), 1234567.0), "12,34,567");
        assert_eq!(num(Some("en-IN"), 12345.0), "12,345");
        assert_eq!(num(Some("en"), -0.125), "-0.125");
        assert_eq!(num(Some("it"), 1234.0), "1234");
        assert_eq!(num(Some("hu"), 12345.0), "12\u{a0}345");
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
