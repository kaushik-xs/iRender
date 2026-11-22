mod support;

use irender_core::Format;
use irender_html::Engine;
use serde_json::{Value as Json, json};

fn render(engine: &Engine, doc: Json, data: Json) -> irender_html::Page {
    let ir = engine.load(&doc.to_string(), Format::Json).unwrap_or_else(|d| panic!("{d}"));
    engine.render(&ir, &data).unwrap()
}

#[test]
fn static_document_has_no_runtime() {
    let page = render(
        &Engine::new(),
        json!({ "version": 1, "root": { "type": "text", "props": { "tag": "h1", "value": "Hi {{ name }} <3" } } }),
        json!({ "name": "Ada" }),
    );
    assert!(!page.interactive);
    assert!(page.body.starts_with("<h1>Hi Ada &lt;3</h1>"), "{}", page.body);
    assert!(!page.fragment().contains("__irender"));
}

#[test]
fn attrs_override_component_defaults_and_styles_layer() {
    let page = render(
        &Engine::new(),
        json!({ "version": 1, "root": { "type": "button",
            "props": { "label": "Go", "type": "submit" },
            "attrs": { "type": "reset", "data-x": 1 },
            "style": { "background": "hotpink" } } }),
        json!({}),
    );
    assert!(page.body.contains(r#"type="reset""#), "{}", page.body);
    assert!(!page.body.contains(r#"type="submit""#));
    assert!(page.body.contains(r#"data-x="1""#));
    let base = page.css.find("@layer i-base {").unwrap();
    let instance = page.css.find("@layer i-instance {").unwrap();
    let pink = page.css.find("background: hotpink").unwrap();
    assert!(base < instance && instance < pink, "{}", page.css);
}

#[test]
fn reactive_values_bind_and_server_data_is_folded() {
    let page = render(
        &Engine::new(),
        json!({ "version": 1, "state": { "qty": 2 }, "root": { "type": "text",
            "props": { "value": "{{ price * qty }}" },
            "style": { "width": "{{ qty * 10 }}" } } }),
        json!({ "price": 5 }),
    );
    assert!(page.interactive);
    assert!(page.body.contains(">10</span>"), "{}", page.body);
    assert!(page.body.contains("--i-d0: 20px"), "{}", page.body);
    // `price` is baked in; only `qty` stays a path for the browser.
    assert!(page.body.contains("{&quot;l&quot;:5}"), "{}", page.body);
    assert!(page.body.contains("{&quot;p&quot;:[&quot;qty&quot;]}"), "{}", page.body);
    assert!(page.fragment().contains("data-i-state>{\"qty\":2}</script>"));
}

#[test]
fn list_scopes_shadow_state() {
    let page = render(
        &Engine::new(),
        json!({ "version": 1, "state": { "item": "state" }, "root": { "type": "list",
            "props": { "items": "{{ rows }}", "tag": "ul" },
            "children": [{ "type": "text", "props": { "tag": "li", "value": "{{ index }}:{{ item }}" } }] } }),
        json!({ "rows": ["a", "b"] }),
    );
    assert_eq!(page.body, "<ul><li>0:a</li><li>1:b</li></ul>");
}

#[test]
fn untrusted_documents_are_rejected_with_paths() {
    let doc = json!({ "version": 1, "root": { "type": "element", "props": { "tag": "script" },
        "attrs": { "onload": "x()" }, "style": { "colour": "red" } } });
    let err = Engine::untrusted().load(&doc.to_string(), Format::Json).unwrap_err();
    let text = err.to_string();
    assert!(text.contains("root.props.tag: tag `script` is not allowed"), "{text}");
    assert!(text.contains("root.attrs.onload"), "{text}");

}

#[test]
fn css_findings_warn_unless_strict() {
    let doc = json!({ "version": 1, "root": { "type": "element",
        "style": { "colour": "red", "color": "12px", "font-variant-numeric": "tabular-nums" } } })
    .to_string();
    let ir = Engine::new().load(&doc, Format::Json).unwrap();
    let warnings: Vec<_> = ir.warnings.iter().map(|w| w.to_string()).collect();
    assert_eq!(
        warnings,
        [
            "warning: root.style.colour: unknown CSS property `colour` (check the spelling)",
            "warning: root.style.color: invalid value `12px` for `color`",
        ]
    );
    // `font-variant-numeric` is valid CSS that lightningcss doesn't model; the
    // allowlist keeps it from warning, even under `strict_css`.
    let strict = Engine { strict_css: true, ..Engine::new() };
    assert_eq!(strict.load(&doc, Format::Json).unwrap_err().len(), 2);
}

#[test]
fn samples_render() {
    let engine = Engine::new();
    let (src, data) = support::product();
    let ir = engine.load(&src, Format::Json).unwrap_or_else(|d| panic!("{d}"));
    let page = engine.render(&ir, &data).unwrap();
    assert!(page.body.contains("₹1499"));
    assert!(page.body.contains("<li>Asha: Lovely glaze, pours evenly.</li>"));

    // The skill's documents that need no kit.
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../skills/irender/examples");
    for doc in ["counter.yaml", "wizard.yaml"] {
        let src = std::fs::read_to_string(format!("{dir}/{doc}")).unwrap_or_else(|e| panic!("{doc}: {e}"));
        let ir = engine.load(&src, Format::Yaml).unwrap_or_else(|d| panic!("{doc}:\n{d}"));
        assert!(engine.render(&ir, &json!({})).unwrap().interactive, "{doc}");
    }
}

#[test]
fn range_lists_on_the_server_and_in_the_browser() {
    let item = json!([{ "type": "text", "props": { "tag": "li", "value": "#{{ i }}" } }]);
    // From server data: the items are rendered and nothing is left for the browser.
    let page = render(
        &Engine::new(),
        json!({ "version": 1, "root": { "type": "list", "props": { "items": "{{ range(1, count + 1) }}", "as": "i", "tag": "ul" }, "children": item } }),
        json!({ "count": 3 }),
    );
    assert!(page.body.contains("<li>#1</li><li>#2</li><li>#3</li>"), "{}", page.body);
    assert!(!page.body.contains("data-i-list"));
    // From state: the browser gets the call, with the server data folded in.
    let page = render(
        &Engine::new(),
        json!({ "version": 1, "state": { "n": 2 }, "root": { "type": "list", "props": { "items": "{{ range(n + offset) }}", "as": "i", "tag": "ul" }, "children": item } }),
        json!({ "offset": 1 }),
    );
    assert!(page.body.contains("<li>#0</li><li>#1</li><li>#2</li>"), "{}", page.body);
    let attr = page.body.split("data-i-list=\"").nth(1).unwrap();
    let attr = attr[..attr.find('"').unwrap()].replace("&quot;", "\"");
    assert!(attr.contains(r#"{"f":"range","a":[{"b":"+","x":{"p":["n"]},"y":{"l":1}}]}"#), "{attr}");
}
