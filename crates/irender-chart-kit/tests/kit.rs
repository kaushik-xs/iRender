use irender_chart_kit::{ChartKit, Renderer};
use irender_core::Format;
use irender_html::{Engine, Page, RenderError};
use serde_json::{Value as Json, json};

fn engine(untrusted: bool) -> Engine {
    let mut engine = if untrusted { Engine::untrusted() } else { Engine::new() };
    irender_ui_kit::register(&mut engine.registry);
    irender_chart_kit::register(&mut engine.registry);
    engine
}

fn render_on(engine: &Engine, root: Json, state: Json, data: Json) -> Result<Page, RenderError> {
    let mut doc = irender_core::parse(&json!({ "version": 1, "state": state, "root": root }).to_string(), Format::Json)
        .unwrap_or_else(|d| panic!("{d}"));
    irender_chart_kit::apply_theme(&mut doc);
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    engine.render(&ir, &data)
}

fn render_with(root: Json, state: Json, data: Json) -> Result<Page, RenderError> {
    render_on(&engine(false), root, state, data)
}

fn unescape(s: &str) -> String {
    s.replace("&quot;", "\"").replace("&#39;", "'").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

/// The behaviour configs of the page's charts, in order.
fn configs(page: &Page) -> Vec<Json> {
    page.body
        .split(" data-i-xc=\"")
        .skip(1)
        .map(|rest| serde_json::from_str(&unescape(&rest[..rest.find('"').unwrap()])).unwrap())
        .collect()
}

fn config(root: Json, state: Json, data: Json) -> Json {
    configs(&render_with(root, state, data).unwrap()).remove(0)
}

#[test]
fn renders_a_sized_box_that_starts_the_behaviour() {
    let page = render_with(
        json!({ "type": "stack", "children": [
            { "type": "ui-chart", "props": { "type": "bar", "data": [{ "day": "Mon", "n": 1 }], "height": 200 } },
            { "type": "ui-sparkline", "props": { "data": [1, 2, 3] } } ] }),
        json!({}),
        json!({}),
    )
    .unwrap();
    assert_eq!(page.body.matches("data-i-x=\"ui-chart\"").count(), 2);
    assert!(page.interactive);
    // One script for every chart on the page, and no ECharts inline.
    assert_eq!(page.scripts.len(), 1);
    assert!(page.scripts[0].contains("__irenderBehaviors"));
    assert!(page.css.contains("height: 200px"), "{}", page.css);
    assert!(page.css.contains("height: 40px"), "{}", page.css);
    let [chart, spark] = <[Json; 2]>::try_from(configs(&page)).unwrap();
    assert_eq!(chart["src"], irender_chart_kit::ECHARTS_CDN);
    assert_eq!(chart["renderer"], "canvas");
    assert_eq!(chart["sparkline"], false);
    assert_eq!(chart["reactive"], false);
    assert_eq!(chart["safe"], false);
    assert_eq!(chart["props"]["data"], json!([{ "day": "Mon", "n": 1 }]));
    assert_eq!(spark["sparkline"], true);
}

#[test]
fn server_data_is_folded_in_and_state_stays_live() {
    let c = config(
        json!({ "type": "ui-chart", "props": { "type": "{{ kind }}", "data": "{{ rows }}", "y": "{{ metric }}" } }),
        json!({ "kind": "line" }),
        json!({ "rows": [[1, 2]], "metric": "n" }),
    );
    assert_eq!(c["reactive"], true);
    assert_eq!(c["props"]["data"], json!([[1, 2]]));
    assert_eq!(c["props"]["y"], "n");
    assert!(c["props"]["type"]["$irender"].is_object(), "{c}");
}

#[test]
fn templates_inside_an_option_work_at_any_depth() {
    let c = config(
        json!({ "type": "ui-chart", "props": { "option": {
            "title": { "text": "{{ heading }}" },
            "series": [{ "type": "bar", "data": "{{ values }}", "name": "{{ name }}" }] } } }),
        json!({ "values": [1, 2] }),
        json!({ "heading": "Sales", "name": "2026" }),
    );
    let option = &c["props"]["option"];
    assert_eq!(option["title"]["text"], "Sales");
    assert_eq!(option["series"][0]["name"], "2026");
    assert!(option["series"][0]["data"]["$irender"].is_object());
    assert_eq!(c["reactive"], true);
}

#[test]
fn a_chart_needs_a_type_or_an_option() {
    let err = render_with(json!({ "type": "ui-chart", "props": { "data": [] } }), json!({}), json!({})).unwrap_err();
    assert!(err.to_string().contains("needs a `type` or an `option`"), "{err}");
    let err = render_with(json!({ "type": "ui-chart", "props": { "option": [1] } }), json!({}), json!({})).unwrap_err();
    assert!(err.to_string().contains("`option` must be an object"), "{err}");
}

#[test]
fn unknown_types_and_props_are_load_errors() {
    let engine = engine(false);
    let doc = json!({ "version": 1, "root": { "type": "ui-chart", "props": { "type": "pyramid", "colour": "red" } } });
    let err = engine.load(&doc.to_string(), Format::Json).unwrap_err().to_string();
    assert!(err.contains("one of: line, area, bar"), "{err}");
    assert!(err.contains("no prop `colour`"), "{err}");
}

#[test]
fn bind_must_be_a_state_key() {
    let engine = engine(false);
    let doc = json!({ "version": 1, "root": { "type": "ui-chart", "props": { "type": "bar", "bind": "picked" } } });
    assert!(engine.load(&doc.to_string(), Format::Json).is_err());
    let c = config(json!({ "type": "ui-chart", "props": { "type": "bar", "bind": "picked" } }), json!({ "picked": null }), json!({}));
    assert_eq!(c["bind"], "picked");
}

#[test]
fn untrusted_documents_get_safe_tooltips_and_checked_map_urls() {
    let untrusted = engine(true);
    let page = render_on(&untrusted, json!({ "type": "ui-chart", "props": { "type": "bar" } }), json!({}), json!({})).unwrap();
    assert_eq!(configs(&page)[0]["safe"], true);

    let map = |url: &str| json!({ "type": "ui-chart", "props": { "option": {}, "maps": { "world": url } } });
    let err = render_on(&untrusted, map("javascript:alert(1)"), json!({}), json!({})).unwrap_err();
    assert!(err.to_string().contains("not allowed by the policy"), "{err}");
    let page = render_on(&untrusted, map("/geo/world.json"), json!({}), json!({})).unwrap();
    assert_eq!(configs(&page)[0]["maps"], json!({ "world": "/geo/world.json" }));
}

#[test]
fn register_with_sets_the_source_and_renderer() {
    let mut engine = Engine::new();
    irender_chart_kit::register_with(
        &mut engine.registry,
        ChartKit { src: "/static/echarts.min.js".into(), renderer: Renderer::Svg },
    );
    let page = render_on(&engine, json!({ "type": "ui-chart", "props": { "type": "pie" } }), json!({}), json!({})).unwrap();
    let c = &configs(&page)[0];
    assert_eq!(c["src"], "/static/echarts.min.js");
    assert_eq!(c["renderer"], "svg");
    let page = render_on(&engine, json!({ "type": "ui-chart", "props": { "type": "pie", "renderer": "canvas" } }), json!({}), json!({}))
        .unwrap();
    assert_eq!(configs(&page)[0]["renderer"], "canvas");
}

#[test]
fn the_palette_becomes_css_variables_in_both_modes() {
    let page = render_with(json!({ "type": "ui-chart", "props": { "type": "bar" } }), json!({}), json!({})).unwrap();
    assert!(page.css.contains("--color-chart-1: #2a78d6"), "{}", page.css);
    assert!(page.css.contains("--color-chart-1: #3987e5"), "{}", page.css);
}

#[test]
fn every_component_is_documented() {
    let engine = engine(false);
    for name in ["ui-chart", "ui-sparkline", "ui-live-data"] {
        let c = engine.registry.get(name).unwrap_or_else(|| panic!("{name} is registered"));
        let info = c.info();
        assert!(!info.doc.is_empty());
        assert!(info.props.iter().all(|p| !p.doc.is_empty()), "{name}");
    }
}

#[test]
fn bindings_and_pipeline_props_reach_the_browser() {
    let c = config(
        json!({ "type": "ui-chart", "props": {
            "type": "bar", "data": "{{ rows }}", "drill": ["region", "country"], "aggregate": "sum", "sort": "-amount", "limit": 5,
            "filter": { "date": "{{ range }}", "product": ["A", "B"] }, "zoom": "slider", "brush": true,
            "bind": "region", "bindRow": "row", "bindZoom": "range", "bindLegend": "visible", "bindBrush": "picked", "bindDrill": "path" } }),
        json!({ "region": null, "row": null, "range": null, "visible": null, "picked": [], "path": [] }),
        json!({ "rows": [] }),
    );
    for (key, state) in [("bind", "region"), ("bindRow", "row"), ("bindZoom", "range"), ("bindLegend", "visible"), ("bindBrush", "picked"), ("bindDrill", "path")] {
        assert_eq!(c[key], state, "{key}");
    }
    let p = &c["props"];
    assert_eq!(p["drill"], json!(["region", "country"]));
    assert_eq!(p["aggregate"], "sum");
    assert_eq!(p["filter"]["product"], json!(["A", "B"]));
    assert!(p["filter"]["date"]["$irender"].is_object(), "{p}");
    assert_eq!(c["reactive"], true);
}

#[test]
fn bindings_must_be_state_keys_and_aggregates_known() {
    let engine = engine(false);
    for props in [json!({ "type": "line", "bindZoom": "range" }), json!({ "type": "bar", "aggregate": "median" })] {
        let doc = json!({ "version": 1, "root": { "type": "ui-chart", "props": props } });
        assert!(engine.load(&doc.to_string(), Format::Json).is_err(), "{props}");
    }
}

#[test]
fn live_data_renders_a_hidden_source() {
    let page = render_with(
        json!({ "type": "ui-live-data", "props": { "url": "/api/ticks", "every": 1000, "mode": "append", "window": 50,
            "params": { "from": "{{ since }}" }, "paused": "{{ !live }}", "into": "ticks", "status": "feed" } }),
        json!({ "ticks": [], "feed": null, "since": null, "live": true }),
        json!({}),
    )
    .unwrap();
    assert!(page.body.contains("data-i-x=\"ui-live-data\""));
    assert!(page.body.contains(" hidden"), "{}", page.body);
    let c = &configs(&page)[0];
    assert_eq!(c["into"], "ticks");
    assert_eq!(c["status"], "feed");
    assert_eq!(c["props"]["url"], "/api/ticks");
    assert_eq!(c["props"]["mode"], "append");
    assert!(c["props"]["paused"]["$irender"].is_object());
    assert!(c["props"]["params"]["from"]["$irender"].is_object());

    let engine = engine(false);
    let doc = json!({ "version": 1, "root": { "type": "ui-live-data", "props": { "url": "/x" } } });
    let err = engine.load(&doc.to_string(), Format::Json).unwrap_err().to_string();
    assert!(err.contains("missing required prop `into`"), "{err}");
}

#[test]
fn live_sources_must_pass_the_policy() {
    let untrusted = engine(true);
    let chart = json!({ "type": "ui-chart", "props": { "type": "line", "live": { "url": "javascript:alert(1)" } } });
    let err = render_on(&untrusted, chart, json!({}), json!({})).unwrap_err();
    assert!(err.to_string().contains("not allowed by the policy"), "{err}");
    let source = json!({ "type": "ui-live-data", "props": { "url": "data:application/json,[]", "into": "x" } });
    let err = render_on(&untrusted, source, json!({ "x": null }), json!({})).unwrap_err();
    assert!(err.to_string().contains("not allowed by the policy"), "{err}");
    let ok = json!({ "type": "ui-chart", "props": { "type": "line", "live": { "url": "/api/live", "every": 2000 } } });
    let page = render_on(&untrusted, ok, json!({}), json!({})).unwrap();
    assert_eq!(configs(&page)[0]["props"]["live"]["url"], "/api/live");
}

#[test]
fn a_gantt_starts_as_tall_as_its_rows_and_grows_in_the_browser() {
    let rows = json!([{ "name": "a", "start": "2026-01-05", "end": "2026-01-09" }, { "name": "b", "start": "2026-01-12" }]);
    let tall = |props: Json| {
        let page = render_with(json!({ "type": "ui-chart", "props": props }), json!({}), json!({})).unwrap();
        // The first `height:` that isn't the end of `line-height:`.
        let at = page.css.match_indices("height:").map(|(i, _)| i).find(|&i| !page.css[..i].ends_with('-'));
        let rule = at.map(|i| &page.css[i + 7..]).map(|h| h[..h.find(';').unwrap_or(h.len())].trim().trim_end_matches('}').to_string());
        (rule, configs(&page).remove(0)["autoHeight"].clone())
    };
    // A heading band, two rows of 34px, and the foot.
    assert_eq!(tall(json!({ "type": "gantt", "data": rows })), (Some("124px".into()), json!(true)));
    // Room for the legend or zoom buttons, the lines' names and the slider.
    assert_eq!(
        tall(json!({ "type": "gantt", "data": rows, "series": "team", "markers": ["2026-01-07"], "zoom": "slider" })),
        (Some("206px".into()), json!(true))
    );
    // Rows from the starting state count too; a set height stays.
    let page = render_with(json!({ "type": "ui-chart", "props": { "type": "gantt", "data": "{{ rows }}" } }), json!({ "rows": rows }), json!({})).unwrap();
    assert!(page.css.contains("height: 124px"), "{}", page.css);
    assert_eq!(tall(json!({ "type": "gantt", "data": rows, "height": 200 })), (Some("200px".into()), json!(false)));
    // `visibleRows` caps it: one row's worth, the other scrolled to; more than there are changes nothing.
    assert_eq!(tall(json!({ "type": "gantt", "data": rows, "visibleRows": 1 })), (Some("90px".into()), json!(true)));
    assert_eq!(tall(json!({ "type": "gantt", "data": rows, "visibleRows": 5 })), (Some("124px".into()), json!(true)));
}

#[test]
fn a_pareto_threshold_reaches_the_browser() {
    let c = config(json!({ "type": "ui-chart", "props": { "type": "pareto", "data": [], "threshold": "{{ t }}", "limit": 5 } }), json!({ "t": 70 }), json!({}));
    assert_eq!(c["props"]["type"], "pareto");
    assert!(c["props"]["threshold"]["$irender"].is_object(), "{c}");
    assert_eq!(c["props"]["limit"], 5);
}
