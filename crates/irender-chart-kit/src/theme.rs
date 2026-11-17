//! The kit's series colours: eight categorical hues with their own steps for dark
//! mode, in an order whose neighbours stay apart for colour-blind readers.

use irender_core::{Document, theme::ThemeSpec};
use serde_json::{Value as Json, json};

fn preset() -> Json {
    json!({
        "tokens": {
            "color": {
                "chart-1": "#2a78d6", "chart-2": "#eb6834", "chart-3": "#1baf7a", "chart-4": "#eda100",
                "chart-5": "#e87ba4", "chart-6": "#008300", "chart-7": "#6250d6", "chart-8": "#e34948"
            }
        },
        "modes": {
            "dark": {
                "color": {
                    "chart-1": "#3987e5", "chart-2": "#d95926", "chart-3": "#199e70", "chart-4": "#c98500",
                    "chart-5": "#d55181", "chart-6": "#008300", "chart-7": "#9085e9", "chart-8": "#e66767"
                }
            }
        }
    })
}

/// The kit's colours as a document `theme`.
pub fn theme() -> ThemeSpec {
    serde_json::from_value(preset()).expect("the preset is a valid theme")
}

/// Merges the kit's colours under `doc`'s own theme: tokens the document sets keep
/// their values. Works alongside `irender_ui_kit::apply_theme`, in either order.
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
            r##"{ "version": 1, "theme": { "tokens": { "color": { "chart-1": "#e11d48", "primary": "#000" } } },
                  "root": { "type": "x" } }"##,
            Format::Json,
        )
        .unwrap();
        apply_theme(&mut doc);
        let color = &doc.theme.tokens["color"];
        assert_eq!(color["chart-1"], "#e11d48");
        assert_eq!(color["chart-2"], "#eb6834");
        assert_eq!(color["primary"], "#000");
        assert_eq!(doc.theme.modes["dark"]["color"]["chart-1"], "#3987e5");
    }
}
