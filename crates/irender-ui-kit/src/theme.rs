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
/// density, padding, gaps, control and icon sizes, text size, font and the rest of
/// the typography, borders, shadows, and control and card radii. `ui-theme-option` picks them; a document can add
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
    let set = |label: &str, tokens: Json| json!({ "label": label, "tokens": tokens });
    const SANS: &str = "system-ui, -apple-system, 'Segoe UI', sans-serif";
    const SERIF: &str = "ui-serif, Georgia, Cambria, 'Times New Roman', serif";
    const MONO: &str = "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace";
    let weights = |label: &str, m: u32, s: u32, b: u32| set(label, json!({ "font": { "weight": { "medium": m, "semibold": s, "bold": b } } }));
    let elevation = |label: &str, control: &str, card: &str, raised: &str, popover: &str, dialog: &str| {
        set(label, json!({ "elevation": { "control": control, "card": card, "card-raised": raised, "popover": popover, "dialog": dialog } }))
    };
    json!({
        "palette": { "label": "Colour", "default": "zinc", "choices": palette },
        "radius": { "label": "Radius", "default": "0-5", "choices": {
            "0": radius("0", "0px", "0px", "0px"),
            "0-3": radius("0.3", "3px", "4px", "6px"),
            "0-5": radius("0.5", "6px", "8px", "10px"),
            "0-75": radius("0.75", "10px", "12px", "14px"),
            "1": radius("1.0", "12px", "16px", "20px"),
        } },
        // Shape: border width, shadows, and corner radii for controls and cards apart
        // from the overall radius, which they follow until picked.
        "border": { "label": "Borders", "default": "thin", "choices": {
            "thin": set("Thin", json!({ "border": { "width": "1px" } })),
            "medium": set("Medium", json!({ "border": { "width": "1.5px" } })),
            "thick": set("Thick", json!({ "border": { "width": "2px" } })),
        } },
        "elevation": { "label": "Shadows", "default": "default", "choices": {
            "flat": elevation("Flat", "none", "none", "none", "0 2px 8px rgb(0 0 0 / 0.08)", "0 4px 16px rgb(0 0 0 / 0.12)"),
            "default": elevation("Default", "0 1px 2px rgb(0 0 0 / 0.05)",
                "0 1px 2px rgb(0 0 0 / 0.04), 0 2px 6px -2px rgb(0 0 0 / 0.06)",
                "0 1px 2px rgb(0 0 0 / 0.05), 0 8px 24px -6px rgb(0 0 0 / 0.14)",
                "0 6px 18px rgb(0 0 0 / 0.12)", "0 10px 30px rgb(0 0 0 / 0.2)"),
            "raised": elevation("Raised", "0 1px 3px rgb(0 0 0 / 0.1)",
                "0 2px 4px rgb(0 0 0 / 0.06), 0 8px 20px -4px rgb(0 0 0 / 0.12)",
                "0 2px 4px rgb(0 0 0 / 0.08), 0 16px 36px -8px rgb(0 0 0 / 0.2)",
                "0 10px 28px rgb(0 0 0 / 0.18)", "0 16px 48px rgb(0 0 0 / 0.28)"),
        } },
        "control-radius": { "label": "Control radius", "default": "radius", "choices": {
            "radius": set("Same as radius", json!({ "radius": { "control": "var(--radius-md)" } })),
            "square": set("Square", json!({ "radius": { "control": "0px" } })),
            "small": set("Small", json!({ "radius": { "control": "4px" } })),
            "large": set("Large", json!({ "radius": { "control": "12px" } })),
            "round": set("Round", json!({ "radius": { "control": "18px" } })),
        } },
        "card-radius": { "label": "Card radius", "default": "radius", "choices": {
            "radius": set("Same as radius", json!({ "radius": { "card": "var(--radius-lg)" } })),
            "square": set("Square", json!({ "radius": { "card": "0px" } })),
            "small": set("Small", json!({ "radius": { "card": "6px" } })),
            "large": set("Large", json!({ "radius": { "card": "16px" } })),
            "x-large": set("Extra large", json!({ "radius": { "card": "24px" } })),
        } },
        "density": { "label": "Density", "default": "default", "choices": {
            "compact": scale("Compact", "space.scale", 0.8),
            "default": scale("Default", "space.scale", 1.0),
            "comfortable": scale("Comfortable", "space.scale", 1.2),
        } },
        // Finer than density, and on top of it: one kind of spacing or size each.
        "padding": { "label": "Padding", "default": "default", "choices": {
            "tight": scale("Tight", "space.inset", 0.75),
            "default": scale("Default", "space.inset", 1.0),
            "roomy": scale("Roomy", "space.inset", 1.25),
        } },
        "gap": { "label": "Gaps", "default": "default", "choices": {
            "tight": scale("Tight", "space.gap", 0.75),
            "default": scale("Default", "space.gap", 1.0),
            "roomy": scale("Roomy", "space.gap", 1.25),
        } },
        "control-size": { "label": "Control size", "default": "default", "choices": {
            "small": scale("Small", "size.control", 0.889),
            "default": scale("Default", "size.control", 1.0),
            "large": scale("Large", "size.control", 1.111),
        } },
        "icon-size": { "label": "Icon size", "default": "default", "choices": {
            "small": scale("Small", "size.icon", 0.875),
            "default": scale("Default", "size.icon", 1.0),
            "large": scale("Large", "size.icon", 1.25),
        } },
        "text-size": { "label": "Text size", "default": "default", "choices": {
            "small": scale("Small", "font.scale", 0.9),
            "default": scale("Default", "font.scale", 1.0),
            "large": scale("Large", "font.scale", 1.125),
        } },
        "font": { "label": "Font", "default": "sans", "choices": {
            "sans": font("Sans", SANS),
            "serif": font("Serif", SERIF),
            "mono": font("Mono", MONO),
        } },
        // Typography beyond the font: headings, weights, line height and tracking.
        // Headings keep the text's font until one is picked.
        "heading-font": { "label": "Heading font", "default": "text", "choices": {
            "text": set("Same as text", json!({})),
            "sans": set("Sans", json!({ "font": { "heading": SANS } })),
            "serif": set("Serif", json!({ "font": { "heading": SERIF } })),
            "mono": set("Mono", json!({ "font": { "heading": MONO } })),
        } },
        "font-weight": { "label": "Weight", "default": "default", "choices": {
            "lighter": weights("Lighter", 400, 500, 600),
            "default": weights("Default", 500, 600, 700),
            "bolder": weights("Bolder", 600, 700, 800),
        } },
        "line-height": { "label": "Line height", "default": "default", "choices": {
            "tight": scale("Tight", "font.leading", 1.35),
            // Nothing: the page's own line height until one is picked.
            "default": set("Default", json!({})),
            "relaxed": scale("Relaxed", "font.leading", 1.7),
        } },
        "letter-spacing": { "label": "Letter spacing", "default": "default", "choices": {
            "tight": set("Tight", json!({ "font": { "tracking": "-0.01em" } })),
            "default": set("Default", json!({})),
            "wide": set("Wide", json!({ "font": { "tracking": "0.025em" } })),
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
