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
fn examples_render() {
    let engine = Engine::new();
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples");
    let src = std::fs::read_to_string(format!("{dir}/product.json")).unwrap();
    let data: Json = serde_json::from_str(&std::fs::read_to_string(format!("{dir}/product.data.json")).unwrap()).unwrap();
    let ir = engine.load(&src, Format::Json).unwrap_or_else(|d| panic!("{d}"));
    let page = engine.render(&ir, &data).unwrap();
    assert!(page.body.contains("₹1499"));
    assert!(page.body.contains("<li>Asha: Lovely glaze, pours evenly.</li>"));

    let src = std::fs::read_to_string(format!("{dir}/counter.yaml")).unwrap();
    let ir = engine.load(&src, Format::Yaml).unwrap_or_else(|d| panic!("{d}"));
    assert!(engine.render(&ir, &json!({})).unwrap().interactive);
}

/// Every document under `examples/` loads without errors and renders with its data.
/// The ones built with irender-ui-kit (the gallery and the index) are checked in its tests.
#[test]
fn examples_load_and_render() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples");
    let read = |path: &str| std::fs::read_to_string(format!("{dir}/{path}")).unwrap_or_else(|e| panic!("{path}: {e}"));
    let cases: &[(&str, Option<&str>, Option<&str>)] = &[
        ("counter.yaml", None, None),
        ("product.json", Some("product.data.json"), None),
        ("wizard.yaml", None, None),
        ("dashboard/dashboard.yaml", Some("dashboard/dashboard.data.json"), Some("dashboard/fragments")),
        ("search/search.json", None, None),
    ];
    for (doc, data, fragments) in cases {
        let mut engine = Engine::new();
        if let Some(f) = fragments {
            engine.fragments = Some(std::sync::Arc::new(irender_core::FragmentDir(format!("{dir}/{f}").into())));
        }
        let format = Format::from_path(std::path::Path::new(doc)).unwrap();
        let ir = engine.load(&read(doc), format).unwrap_or_else(|d| panic!("{doc}:\n{d}"));
        let data = data.map_or(json!({}), |d| serde_json::from_str(&read(d)).unwrap());
        let page = engine.render(&ir, &data).unwrap_or_else(|e| panic!("{doc}: {e}"));
        assert!(page.interactive, "{doc}");
    }
}
