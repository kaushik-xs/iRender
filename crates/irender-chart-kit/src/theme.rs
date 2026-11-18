//! The kit's series colours: eight categorical hues with their own steps for dark
//! mode, in an order whose neighbours stay apart for colour-blind readers.

use irender_core::{Document, theme::ThemeSpec};
use serde_json::{Value as Json, json};

/// The default series colours, light and dark. Each follows the palette when it has
/// series colours of its own (`color.palette-chart-N`: irender-ui-kit's coloured palettes
/// and custom colours set them), and is the kit's categorical hue otherwise. Picking a
/// chart scheme sets `color.chart-N` itself, so it wins over the palette's.
const DEFAULT_LIGHT: [&str; 8] = ["#2a78d6", "#eb6834", "#1baf7a", "#eda100", "#e87ba4", "#008300", "#6250d6", "#e34948"];
const DEFAULT_DARK: [&str; 8] = ["#3987e5", "#d95926", "#199e70", "#c98500", "#d55181", "#008300", "#9085e9", "#e66767"];

fn following_palette(colors: [&str; 8]) -> Json {
    let mut m = serde_json::Map::new();
    for (i, c) in colors.iter().enumerate() {
        m.insert(format!("chart-{}", i + 1), json!(format!("var(--color-palette-chart-{}, {c})", i + 1)));
    }
    Json::Object(m)
}

fn preset() -> Json {
    json!({
        "tokens": { "color": following_palette(DEFAULT_LIGHT) },
        "modes": { "dark": { "color": following_palette(DEFAULT_DARK) } },
        "options": { "chart": { "label": "Chart colours", "default": "default", "choices": schemes() } }
    })
}

/// A chart colour scheme: eight series colours, light then dark.
fn scheme(label: &str, light: [&str; 8], dark: [&str; 8]) -> Json {
    let colors = |c: [&str; 8]| {
        let mut m = serde_json::Map::new();
        for (i, v) in c.iter().enumerate() {
            m.insert(format!("chart-{}", i + 1), json!(v));
        }
        Json::Object(m)
    };
    json!({ "label": label, "tokens": { "color": colors(light) }, "modes": { "dark": { "color": colors(dark) } } })
}

/// The chart colour schemes `ui-theme-option` can pick (`option: chart`): the kit's
/// categorical hues, shades of one colour to match the UI kit's palettes, and greys.
fn schemes() -> Json {
    json!({
        // Follows the palette: its shades for a coloured one, the kit's hues for a neutral one.
        "default": { "label": "Default" },
        // The kit's categorical hues, whatever the palette.
        "hues": scheme("Hues", DEFAULT_LIGHT, DEFAULT_DARK),
        "blue": scheme("Blue",
            ["#2563eb", "#60a5fa", "#1e40af", "#93c5fd", "#1d4ed8", "#3b82f6", "#1e3a8a", "#bfdbfe"],
            ["#3b82f6", "#93c5fd", "#2563eb", "#bfdbfe", "#1d4ed8", "#60a5fa", "#1e40af", "#dbeafe"]),
        "rose": scheme("Rose",
            ["#e11d48", "#fb7185", "#9f1239", "#fda4af", "#be123c", "#f43f5e", "#881337", "#fecdd3"],
            ["#f43f5e", "#fda4af", "#e11d48", "#fecdd3", "#be123c", "#fb7185", "#9f1239", "#ffe4e6"]),
        "green": scheme("Green",
            ["#16a34a", "#4ade80", "#166534", "#86efac", "#15803d", "#22c55e", "#14532d", "#bbf7d0"],
            ["#22c55e", "#86efac", "#16a34a", "#bbf7d0", "#15803d", "#4ade80", "#166534", "#dcfce7"]),
        "orange": scheme("Orange",
            ["#ea580c", "#fb923c", "#9a3412", "#fdba74", "#c2410c", "#f97316", "#7c2d12", "#fed7aa"],
            ["#f97316", "#fdba74", "#ea580c", "#fed7aa", "#c2410c", "#fb923c", "#9a3412", "#ffedd5"]),
        "violet": scheme("Violet",
            ["#7c3aed", "#a78bfa", "#5b21b6", "#c4b5fd", "#6d28d9", "#8b5cf6", "#4c1d95", "#ddd6fe"],
            ["#8b5cf6", "#c4b5fd", "#7c3aed", "#ddd6fe", "#6d28d9", "#a78bfa", "#5b21b6", "#ede9fe"]),
        // Built from the custom colours picked with irender-ui-kit's `ui-theme-colors`.
        "custom": { "label": "Custom" },
        "mono": scheme("Mono",
            ["#18181b", "#71717a", "#3f3f46", "#a1a1aa", "#27272a", "#52525b", "#d4d4d8", "#09090b"],
            ["#fafafa", "#a1a1aa", "#d4d4d8", "#71717a", "#e4e4e7", "#52525b", "#f4f4f5", "#3f3f46"]),
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
        assert_eq!(color["chart-2"], "var(--color-palette-chart-2, #eb6834)");
        assert_eq!(color["primary"], "#000");
        assert_eq!(doc.theme.modes["dark"]["color"]["chart-1"], "var(--color-palette-chart-1, #3987e5)");
    }
}
