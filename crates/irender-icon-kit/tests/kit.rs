use irender_core::Format;
use irender_html::{Engine, RenderError};
use irender_icon_kit::{IconKit, Pack, Variant};
use serde_json::{Value as Json, json};

fn engine_with(kit: IconKit) -> Engine {
    let mut engine = Engine::new();
    irender_icon_kit::register_with(&mut engine.registry, kit);
    engine
}

fn render_in(engine: &Engine, root: Json, state: Json) -> Result<String, RenderError> {
    let doc = irender_core::parse(&json!({ "version": 1, "state": state, "root": root }).to_string(), Format::Json)
        .unwrap_or_else(|d| panic!("{d}"));
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    engine.render(&ir, &json!({ "status": "logistics:truck" })).map(|p| p.body)
}

fn render(props: Json) -> Result<String, RenderError> {
    render_in(&engine_with(IconKit::new()), json!({ "type": "ui-icon", "props": props }), json!({ "n": 1 }))
}

/// Removes generated class names so assertions don't depend on hashes.
fn plain(html: &str) -> String {
    let mut out = String::new();
    let mut rest = html;
    while let Some(i) = rest.find(" class=\"") {
        out.push_str(&rest[..i]);
        let after = &rest[i + 8..];
        rest = &after[after.find('"').unwrap() + 1..];
    }
    out + rest
}

#[test]
fn renders_an_inline_svg_in_the_text_colour() {
    let html = plain(&render(json!({ "name": "logistics:truck" })).unwrap());
    assert!(html.starts_with(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="#
    ), "{html}");
    assert!(html.ends_with("</svg>"));
    assert!(html.contains(r#"<circle cx="7" cy="18" r="2"/>"#));
}

#[test]
fn bare_names_search_every_pack() {
    assert_eq!(plain(&render(json!({ "name": "forklift" })).unwrap()), plain(&render(json!({ "name": "logistics:forklift" })).unwrap()));
}

#[test]
fn label_and_size() {
    let page = {
        let engine = engine_with(IconKit::new());
        let doc = irender_core::parse(
            &json!({ "version": 1, "root": { "type": "ui-icon",
                "props": { "name": "check", "label": "Done", "size": 20 } } })
            .to_string(),
            Format::Json,
        )
        .unwrap();
        engine.render(&engine.resolve(&doc).unwrap(), &json!({})).unwrap()
    };
    let html = plain(&page.body);
    assert!(html.contains(r#"stroke-width="2""#), "{html}");
    assert!(html.contains(r#"role="img" aria-label="Done""#) && !html.contains("aria-hidden"));
    assert!(page.css.contains("width:20px") || page.css.contains("width: 20px"), "{}", page.css);
    let default = engine_with(IconKit::new());
    let doc = irender_core::parse(r#"{ "version": 1, "root": { "type": "ui-icon", "props": { "name": "check" } } }"#, Format::Json).unwrap();
    let css = default.render(&default.resolve(&doc).unwrap(), &json!({})).unwrap().css;
    assert!(css.contains("1em"), "{css}");
}

#[test]
fn the_stroke_width_is_fixed() {
    let doc = irender_core::parse(
        r#"{ "version": 1, "root": { "type": "ui-icon", "props": { "name": "check", "stroke": 1 } } }"#,
        Format::Json,
    )
    .unwrap();
    let err = engine_with(IconKit::new()).resolve(&doc).unwrap_err().to_string();
    assert!(err.contains("stroke"), "{err}");
}

#[test]
fn document_attrs_and_style_still_apply() {
    let html = render_in(
        &engine_with(IconKit::new()),
        json!({ "type": "ui-icon", "props": { "name": "check" }, "attrs": { "id": "ok", "data-n": "{{ n }}" } }),
        json!({ "n": 1 }),
    )
    .unwrap();
    assert!(html.contains(r#"id="ok""#) && html.contains(r#"data-n="1""#), "{html}");
}

#[test]
fn names_from_server_data_work() {
    let html = render(json!({ "name": "{{ status }}" })).unwrap();
    assert!(html.contains(r#"<circle cx="17" cy="18" r="2"/>"#), "{html}");
}

#[test]
fn names_that_read_state_are_an_error() {
    let err = render(json!({ "name": "{{ n == 1 ? 'check' : 'close' }}" })).unwrap_err();
    assert!(err.to_string().contains("can't use state"), "{err}");
}

#[test]
fn unknown_names_suggest_close_ones() {
    let err = render(json!({ "name": "trcuk" })).unwrap_err().to_string();
    assert!(err.contains("unknown icon `trcuk`") && err.contains("`logistics:truck`"), "{err}");
    let err = render(json!({ "name": "nope:truck" })).unwrap_err().to_string();
    assert!(err.contains("no icon pack `nope`") && err.contains("logistics"), "{err}");
    let err = render(json!({ "name": "core:truck" })).unwrap_err().to_string();
    assert!(err.contains("unknown icon `core:truck`"), "{err}");
}

#[test]
fn own_packs_are_sanitized_and_can_come_first() {
    let mut pack = Pack::new("brand", "Brand").unwrap();
    pack.add_svg(
        "truck",
        r#"<svg viewBox="0 0 16 16" fill="currentColor" onload="x()"><script>x()</script><rect width="16" height="8" onclick="y()"/></svg>"#,
    )
    .unwrap();
    let kit = IconKit::empty().with(pack).with_builtin("logistics").unwrap();
    let engine = engine_with(kit);
    let html = plain(&render_in(&engine, json!({ "type": "ui-icon", "props": { "name": "truck" } }), json!({})).unwrap());
    assert_eq!(
        html,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true"><rect width="16" height="8"/></svg>"#
    );
    assert!(render_in(&engine, json!({ "type": "ui-icon", "props": { "name": "logistics:truck" } }), json!({})).is_ok());
    assert!(render_in(&engine, json!({ "type": "ui-icon", "props": { "name": "check" } }), json!({})).is_err());
}

#[test]
fn pack_names_are_checked() {
    assert!(Pack::new("Brand", "x").is_err());
    let mut pack = Pack::new("brand", "Brand").unwrap();
    assert!(pack.add_svg("Logo", r#"<svg viewBox="0 0 1 1"><path d="M0 0"/></svg>"#).is_err());
    assert!(pack.add_svg("logo", "<svg><path d=\"M0 0\"/></svg>").is_err());
}

#[test]
fn packs_load_from_a_directory() {
    let dir = std::env::temp_dir().join(format!("irender-icons-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("Box Truck.svg"), r#"<svg viewBox="0 0 24 24"><path d="M1 1h2"/></svg>"#).unwrap();
    std::fs::write(dir.join("notes.txt"), "ignored").unwrap();
    let pack = Pack::from_dir("brand", &dir).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(pack.names().collect::<Vec<_>>(), ["box-truck"]);
}

#[test]
fn filled_variant() {
    let outline = plain(&render(json!({ "name": "logistics:truck" })).unwrap());
    let filled = plain(&render(json!({ "name": "logistics:truck", "variant": "filled" })).unwrap());
    assert!(filled.contains(r#"fill="currentColor" stroke="currentColor""#), "{filled}");
    assert_ne!(outline, filled);
    // Icons with nothing to fill use the outline drawing for both.
    let check = |v: &str| plain(&render(json!({ "name": "check", "variant": v })).unwrap());
    assert_eq!(check("outline"), check("filled"));
    let err = render(json!({ "name": "check", "variant": "{{ n == 1 ? 'filled' : 'outline' }}" })).unwrap_err();
    assert!(err.to_string().contains("`variant` can't use state"), "{err}");
}

#[test]
fn own_packs_without_a_filled_style_say_so() {
    let mut pack = Pack::new("brand", "Brand").unwrap();
    pack.add_svg("logo", r#"<svg viewBox="0 0 24 24"><path d="M1 1h2"/></svg>"#).unwrap();
    pack.add_svg("dash", r#"<svg viewBox="0 0 24 24"><path d="M1 1h4"/></svg>"#).unwrap();
    pack.add_svg_filled("dash", r#"<svg viewBox="0 0 24 24" fill="currentColor"><rect width="4" height="2"/></svg>"#).unwrap();
    let kit = IconKit::empty().with(pack);
    assert!(kit.find("brand:dash", Variant::Filled).is_ok());
    assert_eq!(kit.find("logo", Variant::Filled).unwrap_err().to_string(), "`brand:logo` has no filled style");
}

#[test]
fn packs_load_filled_icons_from_a_directory() {
    let dir = std::env::temp_dir().join(format!("irender-icons-filled-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("filled")).unwrap();
    std::fs::write(dir.join("box.svg"), r#"<svg viewBox="0 0 24 24"><path d="M1 1h2"/></svg>"#).unwrap();
    std::fs::write(dir.join("line.svg"), r#"<svg viewBox="0 0 24 24"><path d="M1 1h2"/></svg>"#).unwrap();
    std::fs::write(dir.join("filled/box.svg"), r#"<svg viewBox="0 0 24 24"><rect width="2" height="2"/></svg>"#).unwrap();
    std::fs::write(dir.join("filled/_same.txt"), "line\n").unwrap();
    let pack = Pack::from_dir("brand", &dir).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(pack.has_filled("box") && pack.has_filled("line"));
    assert_eq!(pack.get_variant("box", Variant::Filled).unwrap().body(), r#"<rect width="2" height="2"/>"#);
}

#[test]
fn every_builtin_icon_follows_the_standard_and_renders() {
    let engine = engine_with(IconKit::new());
    let mut total = 0;
    let packs: Vec<_> = IconKit::builtin().collect();
    assert_eq!(packs[0].id(), "core");
    assert_eq!(packs.last().unwrap().id(), "brands");
    for pack in packs {
        // Logo marks keep their owners' drawing, padded by a wider viewBox.
        let view_box = if pack.id() == "brands" { "-2 -2 28 28" } else { "0 0 24 24" };
        assert!(!pack.title().is_empty() && !pack.description().is_empty(), "{}", pack.id());
        assert!(!pack.is_empty(), "{}", pack.id());
        for name in pack.names() {
            let icon = pack.get(name).unwrap();
            assert_eq!(icon.view_box(), view_box, "{}:{name}", pack.id());
            assert!(!icon.body().contains("fill=") && !icon.body().contains("stroke"), "{}:{name}", pack.id());
            let full = format!("{}:{name}", pack.id());
            render_in(&engine, json!({ "type": "ui-icon", "props": { "name": full } }), json!({})).unwrap();
            // Every icon has a filled style: its own drawing, or the outline when there's nothing to fill.
            let filled = pack.get_variant(name, Variant::Filled).unwrap_or_else(|| panic!("{full} has no filled style"));
            assert_eq!(filled.view_box(), view_box, "{full}");
            let body = filled.body().replace(r#"fill="none""#, "").replace(r#"fill-rule="evenodd""#, "");
            assert!(!body.contains("fill") && !body.contains("stroke"), "{full} (filled)");
            render_in(&engine, json!({ "type": "ui-icon", "props": { "name": full, "variant": "filled" } }), json!({})).unwrap();
            total += 1;
        }
    }
    assert!(total > 0);
}

#[test]
fn works_under_the_untrusted_policy() {
    let mut engine = Engine::untrusted();
    irender_icon_kit::register(&mut engine.registry);
    assert!(render_in(&engine, json!({ "type": "ui-icon", "props": { "name": "home", "label": "Home" } }), json!({})).is_ok());
}
