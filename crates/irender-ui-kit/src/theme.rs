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
        "options": options(),
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

/// Primary colours for a coloured palette, light then dark, and the series colours charts
/// take with it (`palette-chart-1` … `8`, shades of the same colour, as the chart kit's
/// scheme of that name), unless a chart colour scheme is picked.
fn accent(light: [&str; 3], dark: [&str; 3], chart_light: [&str; 8], chart_dark: [&str; 8]) -> Json {
    let colors = |p: [&str; 3], c: [&str; 8]| {
        let mut m = json!({ "primary": p[0], "primary-hover": p[1], "on-primary": p[2], "ring": p[0] });
        for (i, v) in c.iter().enumerate() {
            m[format!("palette-chart-{}", i + 1)] = json!(v);
        }
        m
    };
    json!({ "tokens": { "color": colors(light, chart_light) }, "modes": { "dark": { "color": colors(dark, chart_dark) } } })
}

/// A neutral palette: every colour, light then dark.
fn neutral(light: [&str; 10], dark: [&str; 10]) -> Json {
    let colors = |c: [&str; 10]| {
        json!({
            "primary": c[0], "primary-hover": c[1], "on-primary": c[2], "surface": c[3], "text": c[4],
            "muted": c[5], "border": c[6], "secondary": c[7], "on-secondary": c[0], "accent": c[7],
            "subtle": c[7], "ring": c[8], "sidebar": c[9]
        })
    };
    json!({ "tokens": { "color": colors(light) }, "modes": { "dark": { "color": colors(dark) } } })
}

/// The theme options the kit offers: palettes after shadcn/ui's themes, corner radius,
/// density, text size and font. `ui-theme-option` picks them; a document can add
/// choices or options of its own under `theme.options`.
fn options() -> Json {
    let mut palette = json!({
        "zinc": { "label": "Zinc" },
        "slate": neutral(
            ["#0f172a", "#1e293b", "#f8fafc", "#ffffff", "#020617", "#64748b", "#e2e8f0", "#f1f5f9", "#94a3b8", "#f8fafc"],
            ["#f8fafc", "#e2e8f0", "#0f172a", "#020617", "#f8fafc", "#94a3b8", "#1e293b", "#1e293b", "#475569", "#0f172a"],
        ),
        "stone": neutral(
            ["#1c1917", "#292524", "#fafaf9", "#ffffff", "#0c0a09", "#78716c", "#e7e5e4", "#f5f5f4", "#a8a29e", "#fafaf9"],
            ["#fafaf9", "#e7e5e4", "#1c1917", "#0c0a09", "#fafaf9", "#a8a29e", "#292524", "#292524", "#57534e", "#1c1917"],
        ),
        "rose": accent(["#e11d48", "#be123c", "#fff1f2"], ["#e11d48", "#f43f5e", "#fff1f2"],
            ["#e11d48", "#fb7185", "#9f1239", "#fda4af", "#be123c", "#f43f5e", "#881337", "#fecdd3"],
            ["#f43f5e", "#fda4af", "#e11d48", "#fecdd3", "#be123c", "#fb7185", "#9f1239", "#ffe4e6"]),
        "blue": accent(["#2563eb", "#1d4ed8", "#eff6ff"], ["#3b82f6", "#60a5fa", "#0b1120"],
            ["#2563eb", "#60a5fa", "#1e40af", "#93c5fd", "#1d4ed8", "#3b82f6", "#1e3a8a", "#bfdbfe"],
            ["#3b82f6", "#93c5fd", "#2563eb", "#bfdbfe", "#1d4ed8", "#60a5fa", "#1e40af", "#dbeafe"]),
        "green": accent(["#16a34a", "#15803d", "#f0fdf4"], ["#22c55e", "#4ade80", "#052e16"],
            ["#16a34a", "#4ade80", "#166534", "#86efac", "#15803d", "#22c55e", "#14532d", "#bbf7d0"],
            ["#22c55e", "#86efac", "#16a34a", "#bbf7d0", "#15803d", "#4ade80", "#166534", "#dcfce7"]),
        "orange": accent(["#ea580c", "#c2410c", "#fff7ed"], ["#f97316", "#fb923c", "#431407"],
            ["#ea580c", "#fb923c", "#9a3412", "#fdba74", "#c2410c", "#f97316", "#7c2d12", "#fed7aa"],
            ["#f97316", "#fdba74", "#ea580c", "#fed7aa", "#c2410c", "#fb923c", "#9a3412", "#ffedd5"]),
        "violet": accent(["#7c3aed", "#6d28d9", "#f5f3ff"], ["#8b5cf6", "#a78bfa", "#f5f3ff"],
            ["#7c3aed", "#a78bfa", "#5b21b6", "#c4b5fd", "#6d28d9", "#8b5cf6", "#4c1d95", "#ddd6fe"],
            ["#8b5cf6", "#c4b5fd", "#7c3aed", "#ddd6fe", "#6d28d9", "#a78bfa", "#5b21b6", "#ede9fe"]),
    });
    // Colours someone types or picks with `ui-theme-colors`, kept in the browser.
    palette["custom"] = json!({});
    for (name, choice) in palette.as_object_mut().unwrap() {
        let mut label = name.clone();
        label[..1].make_ascii_uppercase();
        choice["label"] = json!(label);
    }
    let radius = |label: &str, sm: &str, md: &str, lg: &str| json!({ "label": label, "tokens": { "radius": { "sm": sm, "md": md, "lg": lg } } });
    let scale = |label: &str, token: &str, v: f64| {
        let (group, key) = token.split_once('.').unwrap();
        json!({ "label": label, "tokens": { group: { key: v } } })
    };
    let font = |label: &str, family: &str| json!({ "label": label, "tokens": { "font": { "body": family } } });
    json!({
        "palette": { "label": "Colour", "default": "zinc", "choices": palette },
        "radius": { "label": "Radius", "default": "0-5", "choices": {
            "0": radius("0", "0px", "0px", "0px"),
            "0-3": radius("0.3", "3px", "4px", "6px"),
            "0-5": radius("0.5", "6px", "8px", "10px"),
            "0-75": radius("0.75", "10px", "12px", "14px"),
            "1": radius("1.0", "12px", "16px", "20px"),
        } },
        "density": { "label": "Density", "default": "default", "choices": {
            "compact": scale("Compact", "space.scale", 0.8),
            "default": scale("Default", "space.scale", 1.0),
            "comfortable": scale("Comfortable", "space.scale", 1.2),
        } },
        "text-size": { "label": "Text size", "default": "default", "choices": {
            "small": scale("Small", "font.scale", 0.9),
            "default": scale("Default", "font.scale", 1.0),
            "large": scale("Large", "font.scale", 1.125),
        } },
        "font": { "label": "Font", "default": "sans", "choices": {
            "sans": font("Sans", "system-ui, -apple-system, 'Segoe UI', sans-serif"),
            "serif": font("Serif", "ui-serif, Georgia, Cambria, 'Times New Roman', serif"),
            "mono": font("Mono", "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace"),
        } },
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
