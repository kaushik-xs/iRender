//! The kit's neutral palette, after shadcn/ui's "zinc".

use irender_core::{Document, theme::ThemeSpec};
use serde_json::{Value as Json, json};

fn preset() -> Json {
    json!({
        "tokens": {
            "color": {
                "primary": "#18181b", "primary-hover": "#27272a", "on-primary": "#fafafa",
                "surface": "#ffffff", "text": "#09090b", "muted": "#71717a", "border": "#e4e4e7",
                "secondary": "#f4f4f5", "on-secondary": "#18181b", "accent": "#f4f4f5", "subtle": "#f4f4f5",
                "destructive": "#dc2626", "on-destructive": "#fafafa", "ring": "#a1a1aa"
            },
            "radius": { "sm": "6px", "md": "8px", "lg": "10px" }
        },
        "modes": {
            "dark": {
                "color": {
                    "primary": "#fafafa", "primary-hover": "#e4e4e7", "on-primary": "#18181b",
                    "surface": "#09090b", "text": "#fafafa", "muted": "#a1a1aa", "border": "#27272a",
                    "secondary": "#27272a", "on-secondary": "#fafafa", "accent": "#27272a", "subtle": "#27272a",
                    "destructive": "#ef4444", "on-destructive": "#fafafa", "ring": "#52525b"
                }
            }
        }
    })
}

/// The kit's palette as a document `theme`.
pub fn theme() -> ThemeSpec {
    serde_json::from_value(preset()).expect("the preset is a valid theme")
}

/// Merges the kit's palette under `doc`'s own theme: tokens the document sets
/// keep their values.
pub fn apply_theme(doc: &mut Document) {
    let mut merged = preset();
    let own = serde_json::to_value(&doc.theme).expect("a theme serializes");
    merge(&mut merged, own);
    doc.theme = serde_json::from_value(merged).expect("merging two themes gives a theme");
}

fn merge(base: &mut Json, over: Json) {
    match (base, over) {
        (Json::Object(base), Json::Object(over)) => {
            for (k, v) in over {
                merge(base.entry(k).or_insert(Json::Null), v);
            }
        }
        (base, over) => *base = over,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use irender_core::{Format, parse};

    #[test]
    fn document_tokens_win() {
        let mut doc = parse(
            r##"{ "version": 1, "theme": { "tokens": { "color": { "primary": "#e11d48" } },
                  "variants": { "brand": "[data-brand] &" } }, "root": { "type": "x" } }"##,
            Format::Json,
        )
        .unwrap();
        apply_theme(&mut doc);
        let color = &doc.theme.tokens["color"];
        assert_eq!(color["primary"], "#e11d48");
        assert_eq!(color["destructive"], "#dc2626");
        assert_eq!(doc.theme.modes["dark"]["color"]["surface"], "#09090b");
        assert_eq!(doc.theme.variants["brand"], "[data-brand] &");
    }
}
