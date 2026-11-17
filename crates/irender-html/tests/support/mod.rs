//! Documents shared by the tests and the benchmarks.
#![allow(dead_code)]

use serde_json::{Value as Json, json};

pub fn examples_dir() -> String {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples").to_string()
}

/// The product card example and its data.
pub fn product() -> (String, Json) {
    let dir = examples_dir();
    let doc = std::fs::read_to_string(format!("{dir}/product.json")).unwrap();
    let data = std::fs::read_to_string(format!("{dir}/product.data.json")).unwrap();
    (doc, serde_json::from_str(&data).unwrap())
}

/// A table of `rows` rows from server data. Every row has four cells, a templated
/// class and one reactive cell, so it exercises scopes, folding and bindings.
pub fn table(rows: usize) -> (String, Json) {
    let doc = json!({
        "version": 1,
        "state": { "selected": 0 },
        "root": {
            "type": "element",
            "props": { "tag": "table" },
            "style": { "border-collapse": "collapse", "width": "100%", "font-family": "$font.body" },
            "children": [{
                "type": "list",
                "props": { "items": "{{ rows }}", "as": "row", "tag": "tbody" },
                "children": [{
                    "type": "element",
                    "props": { "tag": "tr" },
                    "class": ["{{ index % 2 == 0 ? 'even' : 'odd' }}"],
                    "style": { "border-bottom": "1px solid $color.border", ":hover": { "background": "$color.border" } },
                    "on": { "click": { "action": "set_state", "key": "selected", "value": "{{ row.id }}" } },
                    "children": [
                        { "type": "text", "props": { "tag": "td", "value": "{{ row.id }}" }, "style": { "padding": 8 } },
                        { "type": "text", "props": { "tag": "td", "value": "{{ row.name }}" }, "style": { "padding": 8 } },
                        { "type": "text", "props": { "tag": "td", "value": "₹{{ row.price * row.qty }}" },
                          "style": { "padding": 8, "text-align": "right" } },
                        { "type": "text", "props": { "tag": "td", "value": "{{ selected == row.id ? '✓' : '' }}" },
                          "style": { "padding": 8, "color": "$color.primary" } }
                    ]
                }]
            }]
        }
    });
    let data = json!({
        "rows": (0..rows)
            .map(|i| json!({ "id": i, "name": format!("Item {i}"), "price": 100 + i % 50, "qty": 1 + i % 4 }))
            .collect::<Vec<_>>()
    });
    (doc.to_string(), data)
}

/// `n` static cards, each with *different* styles: the worst case for the style
/// compiler because nothing deduplicates.
pub fn unique_styles(n: usize) -> (String, Json) {
    let children: Vec<Json> = (0..n)
        .map(|i| {
            json!({
                "type": "stack",
                "props": { "gap": i % 24 },
                "style": { "padding": i, "margin": format!("{}px auto", i % 40), ":hover": { "opacity": 0.9 } },
                "children": [{ "type": "text", "props": { "value": format!("Card {i}") } }]
            })
        })
        .collect();
    let doc = json!({ "version": 1, "root": { "type": "element", "children": children } });
    (doc.to_string(), json!({}))
}

/// A chain of `depth` nested elements.
pub fn deep(depth: usize) -> (String, Json) {
    let mut node = json!({ "type": "text", "props": { "value": "leaf" } });
    for i in 0..depth {
        node = json!({ "type": "element", "style": { "padding-left": i % 3 }, "children": [node] });
    }
    (json!({ "version": 1, "root": node }).to_string(), json!({}))
}

/// Number of elements in rendered HTML (start tags).
pub fn count_elements(html: &str) -> usize {
    html.match_indices('<').filter(|(i, _)| !html[i + 1..].starts_with('/')).count()
}
