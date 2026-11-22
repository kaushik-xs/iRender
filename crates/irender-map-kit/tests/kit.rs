use irender_core::Format;
use irender_html::{Engine, Page, RenderError};
use irender_map_kit::MapKit;
use serde_json::{Value as Json, json};

fn engine(untrusted: bool, kit: MapKit) -> Engine {
    let mut engine = if untrusted { Engine::untrusted() } else { Engine::new() };
    irender_ui_kit::register(&mut engine.registry);
    irender_map_kit::register_with(&mut engine.registry, kit);
    engine
}

fn render_on(engine: &Engine, root: Json, state: Json, data: Json) -> Result<Page, RenderError> {
    let doc = irender_core::parse(&json!({ "version": 1, "state": state, "root": root }).to_string(), Format::Json).unwrap_or_else(|d| panic!("{d}"));
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    engine.render(&ir, &data)
}

fn render(root: Json, state: Json) -> Result<Page, RenderError> {
    render_on(&engine(false, MapKit::default()), root, state, json!({}))
}

fn map(props: Json) -> Json {
    json!({ "type": "ui-map", "props": props })
}

/// The map's behaviour config.
fn config(page: &Page) -> Json {
    let rest = page.body.split(" data-i-xc=\"").nth(1).unwrap();
    let text = rest[..rest.find('"').unwrap()].replace("&quot;", "\"").replace("&#39;", "'").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&");
    serde_json::from_str(&text).unwrap()
}

#[test]
fn a_map_on_openstreetmap() {
    let page = render(map(json!({ "markers": [{ "id": "a", "lat": 1, "lng": 2, "title": "A" }] })), json!({})).unwrap();
    let c = config(&page);
    assert_eq!(c["provider"], "osm");
    assert_eq!(c["source"]["tiles"]["url"], "https://tile.openstreetmap.org/{z}/{x}/{y}.png");
    assert_eq!(c["source"]["tiles"]["attribution"], "© OpenStreetMap contributors");
    assert_eq!(c["props"]["markers"][0]["title"], "A");
    assert_eq!(c["reactive"], false);
    assert!(page.body.contains(r#"role="region" aria-label="Map""#), "{}", page.body);
    // The libraries are loaded by the script, when a map is near the view.
    assert!(!page.document("t").contains("<script src"));
    // My location is asked for.
    assert!(!page.body.contains(r#"data-map-action="locate""#));
    let page = render(map(json!({ "locate": true })), json!({})).unwrap();
    assert!(page.body.contains(r#"aria-label="My location""#));
    let page = render(map(json!({ "controls": false })), json!({})).unwrap();
    assert!(!page.body.contains("data-map-controls"));
}

#[test]
fn state_stays_live_and_data_is_folded() {
    let root = map(json!({ "markers": "{{ rows }}", "center": ["{{ lat }}", 5], "bind": "picked", "shapes": [{ "type": "circle", "center": [1, 2], "radius": "{{ r }}" }] }));
    let page = render_on(&engine(false, MapKit::default()), root, json!({ "rows": [], "picked": null, "r": 10 }), json!({ "lat": 4 })).unwrap();
    let c = config(&page);
    assert_eq!(c["props"]["markers"], json!({ "$irender": { "r": { "p": ["rows"] } } }));
    assert_eq!(c["props"]["center"], json!([4, 5]));
    assert_eq!(c["props"]["shapes"][0]["radius"], json!({ "$irender": { "r": { "p": ["r"] } } }));
    assert_eq!(c["bind"], "picked");
    assert_eq!(c["reactive"], true);
}

#[test]
fn google_needs_a_key() {
    let google = |extra: Json| {
        let mut props = json!({ "provider": "google" });
        props.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        map(props)
    };
    let err = render(google(json!({})), json!({})).unwrap_err().to_string();
    assert!(err.contains("needs an API key"), "{err}");
    assert_eq!(config(&render(google(json!({ "apiKey": "doc" })), json!({})).unwrap())["source"]["key"], "doc");
    let kit = MapKit { google_key: Some("host".into()), ..MapKit::default() };
    let page = render_on(&engine(false, kit.clone()), google(json!({})), json!({}), json!({})).unwrap();
    assert_eq!(config(&page)["source"]["key"], "host");
    // A document's own wins, and an empty one from data counts as none.
    let page = render_on(&engine(false, kit), google(json!({ "apiKey": "{{ key }}" })), json!({}), json!({ "key": "" })).unwrap();
    assert_eq!(config(&page)["source"]["key"], "host");
}

#[test]
fn popups_and_panels() {
    let root = json!({ "type": "ui-map", "props": { "markers": [] }, "children": [
        { "type": "ui-map-popup", "children": [{ "type": "text", "props": { "value": "{{ marker.title }}!" } }] },
        { "type": "ui-map-panel", "props": { "position": "top-right" }, "children": [{ "type": "text", "props": { "value": "Legend" } }] } ] });
    let page = render(root, json!({})).unwrap();
    assert!(page.body.contains("data-map-proto"), "{}", page.body);
    assert!(page.body.contains("<template>"), "{}", page.body);
    assert!(page.body.contains(r#"data-position="top-right"><span>Legend</span>"#), "{}", page.body);

    let err = render(json!({ "type": "ui-map", "children": [{ "type": "text", "props": { "value": "x" } }] }), json!({})).unwrap_err().to_string();
    assert!(err.contains("a map's children are"), "{err}");
    let err = render(json!({ "type": "ui-map-popup" }), json!({})).unwrap_err().to_string();
    assert!(err.contains("goes inside a `ui-map`"), "{err}");
}

#[test]
fn tile_urls_pass_the_policy() {
    let tiles = |url: &str| map(json!({ "tiles": { "url": url } }));
    let untrusted = engine(true, MapKit::default());
    let err = render_on(&untrusted, tiles("javascript:alert(1)"), json!({}), json!({})).unwrap_err().to_string();
    assert!(err.contains("not allowed by the policy"), "{err}");
    let page = render_on(&untrusted, tiles("https://tiles.example/{z}/{x}/{y}.png"), json!({}), json!({})).unwrap();
    assert_eq!(config(&page)["safe"], true);
    let err = render_on(&untrusted, map(json!({ "overlays": [{ "type": "image", "url": "javascript:alert(1)", "bounds": [[1, 2], [3, 4]] }] })), json!({}), json!({})).unwrap_err().to_string();
    assert!(err.contains("`overlays`: URL not allowed"), "{err}");
    let err = render(map(json!({ "tiles": {} })), json!({})).unwrap_err().to_string();
    assert!(err.contains("`tiles` needs a `url`"), "{err}");
}
