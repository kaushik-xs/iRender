use irender_core::Format;
use irender_html::{Engine, Page, RenderError};
use irender_twin_kit::TwinKit;
use serde_json::{Value as Json, json};

fn engine(untrusted: bool) -> Engine {
    let mut engine = if untrusted { Engine::untrusted() } else { Engine::new() };
    irender_ui_kit::register(&mut engine.registry);
    irender_twin_kit::register(&mut engine.registry);
    engine
}

fn render_on(engine: &Engine, root: Json, state: Json, data: Json) -> Result<Page, RenderError> {
    let doc = irender_core::parse(&json!({ "version": 1, "state": state, "root": root }).to_string(), Format::Json).unwrap_or_else(|d| panic!("{d}"));
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    engine.render(&ir, &data)
}

fn render(root: Json, state: Json) -> Result<Page, RenderError> {
    render_on(&engine(false), root, state, json!({}))
}

fn twin(props: Json) -> Json {
    json!({ "type": "ui-twin", "props": props })
}

/// The twin's behaviour config.
fn config(page: &Page) -> Json {
    let rest = page.body.split(" data-i-xc=\"").nth(1).unwrap();
    let text = rest[..rest.find('"').unwrap()].replace("&quot;", "\"").replace("&#39;", "'").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&");
    serde_json::from_str(&text).unwrap()
}

#[test]
fn a_world_from_configuration() {
    let page = render(
        twin(json!({
            "models": { "pump": "models/pump.glb" },
            "objects": [{ "id": "p1", "model": "pump", "position": [1, 2], "temp": 71 }],
            "readouts": [{ "field": "temp", "unit": "°C", "warn": 70 }],
        })),
        json!({}),
    )
    .unwrap();
    let c = config(&page);
    assert_eq!(c["props"]["models"]["pump"], "models/pump.glb");
    assert_eq!(c["props"]["objects"][0]["temp"], 71);
    assert_eq!(c["reactive"], false);
    assert_eq!(c["popup"], true);
    assert_eq!(c["source"]["three"], format!("https://cdn.jsdelivr.net/npm/three@{}/+esm", irender_twin_kit::THREE_VERSION));
    assert!(c["source"]["addons"].as_str().unwrap().contains("/examples/jsm/{name}/+esm"));
    // The region, the view the keyboard turns and what it says it does, the list of objects.
    assert!(page.body.contains(r#"role="region" aria-label="3D view""#), "{}", page.body);
    assert!(page.body.contains(r#"data-twin-canvas tabindex="0" role="img""#), "{}", page.body);
    assert!(page.body.contains("Arrow keys turn the view"));
    assert!(page.body.contains(r#"data-twin-list aria-label="Objects""#));
    assert!(page.body.contains(r#"aria-label="Fit view""#));
    // three.js is loaded by the script, when a twin is near the view.
    assert!(!page.document("t").contains("<script src"));
    assert!(!render(twin(json!({ "controls": false })), json!({})).unwrap().body.contains("data-twin-controls"));
}

#[test]
fn state_stays_live_and_data_is_folded() {
    let root = twin(json!({
        "objects": "{{ rows }}",
        "world": { "views": [{ "id": "a", "position": ["{{ x }}", 5, 5] }] },
        "live": { "url": "api/plant", "every": "{{ every }}" },
        "bind": "picked",
        "bindLive": "values",
        "bindObjects": "rows",
    }));
    let page = render_on(&engine(false), root, json!({ "rows": [], "picked": null, "every": 2000, "values": {} }), json!({ "x": 4 })).unwrap();
    let c = config(&page);
    assert_eq!(c["props"]["objects"], json!({ "$irender": { "r": { "p": ["rows"] } } }));
    assert_eq!(c["props"]["world"]["views"][0]["position"], json!([4, 5, 5]));
    assert_eq!(c["props"]["live"]["every"], json!({ "$irender": { "r": { "p": ["every"] } } }));
    assert_eq!((c["bind"].as_str(), c["bindLive"].as_str(), c["bindObjects"].as_str()), (Some("picked"), Some("values"), Some("rows")));
    assert_eq!(c["reactive"], true);
}

#[test]
fn popups_and_panels() {
    let root = json!({ "type": "ui-twin", "children": [
        { "type": "ui-twin-popup", "children": [{ "type": "text", "props": { "value": "{{ object.title }}" } }] },
        { "type": "ui-twin-panel", "props": { "position": "bottom-right" }, "children": [{ "type": "text", "props": { "value": "Legend" } }] },
    ] });
    let page = render(root, json!({})).unwrap();
    assert!(page.body.contains("data-twin-proto"), "{}", page.body);
    assert!(page.body.contains(r#"data-position="bottom-right""#));
    assert!(page.body.contains("Legend"));

    let two = json!({ "type": "ui-twin", "children": [{ "type": "ui-twin-popup" }, { "type": "ui-twin-popup" }] });
    assert!(render(two, json!({})).unwrap_err().to_string().contains("one `ui-twin-popup`"));
    let other = json!({ "type": "ui-twin", "children": [{ "type": "ui-button" }] });
    assert!(render(other, json!({})).unwrap_err().to_string().contains("`ui-twin-popup` and `ui-twin-panel`"));
    let alone = json!({ "type": "ui-twin-panel" });
    assert!(render(alone, json!({})).unwrap_err().to_string().contains("goes inside a `ui-twin`"));
}

#[test]
fn models_are_checked() {
    let err = render(twin(json!({ "models": { "pump": 3 } })), json!({})).unwrap_err().to_string();
    assert!(err.contains("`models.pump` must be a url"), "{err}");
    // Documents the policy doesn't trust can't load a model, a world or a feed from a script URL.
    for props in [
        json!({ "models": { "pump": "javascript:alert(1)" } }),
        json!({ "models": { "pump": { "url": "javascript:alert(1)" } } }),
        json!({ "src": "javascript:alert(1)" }),
        json!({ "live": { "stream": "javascript:alert(1)" } }),
    ] {
        let err = render_on(&engine(true), twin(props.clone()), json!({}), json!({})).unwrap_err().to_string();
        assert!(err.contains("not allowed by the policy"), "{props}: {err}");
    }
    assert_eq!(config(&render_on(&engine(true), twin(json!({})), json!({}), json!({})).unwrap())["safe"], true);
    assert_eq!(config(&render(twin(json!({})), json!({})).unwrap())["safe"], false);
}

#[test]
fn three_from_elsewhere() {
    let mut engine = Engine::new();
    let kit = TwinKit { three: "/static/three.module.js".into(), addons: "/static/jsm/{name}".into(), draco: "/static/draco/".into(), basis: "/static/basis/".into() };
    irender_twin_kit::register_with(&mut engine.registry, kit);
    let c = config(&render_on(&engine, twin(json!({})), json!({}), json!({})).unwrap());
    assert_eq!(c["source"], json!({ "three": "/static/three.module.js", "addons": "/static/jsm/{name}", "draco": "/static/draco/", "basis": "/static/basis/" }));
}
