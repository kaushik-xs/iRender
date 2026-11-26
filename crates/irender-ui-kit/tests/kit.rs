use irender_core::Format;
use irender_html::{Engine, Page, RenderError};
use serde_json::{Value as Json, json};

fn engine() -> Engine {
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    engine
}

fn render_with(root: Json, state: Json, data: Json) -> Result<Page, RenderError> {
    let engine = engine();
    let mut doc = irender_core::parse(&json!({ "version": 1, "state": state, "root": root }).to_string(), Format::Json)
        .unwrap_or_else(|d| panic!("{d}"));
    irender_ui_kit::apply_theme(&mut doc);
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    engine.render(&ir, &data)
}

fn render(root: Json) -> String {
    render_with(root, json!({}), json!({})).unwrap().body
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
fn every_component_is_prefixed_documented_and_renders() {
    let engine = engine();
    let minimal: &[(&str, Json)] = &[
        ("ui-badge", json!({ "text": "x" })),
        ("ui-filters", json!({ "bind": "k" })),
        ("ui-tree", json!({ "nodes": [] })),
        ("ui-file-upload", json!({ "bind": "k" })),
        ("ui-status-badge", json!({ "status": "Done" })),
        ("ui-progress-ring", json!({ "value": 50 })),
        ("ui-rating", json!({ "bind": "k" })),
        ("ui-password-checklist", json!({ "value": "" })),
        ("ui-kanban", json!({ "columns": [], "cards": [] })),
        ("ui-event-calendar", json!({ "events": [] })),
        ("ui-number-input", json!({ "bind": "k" })),
        ("ui-notifications", json!({ "items": [] })),
        ("ui-rail-layout", json!({ "bind": "k", "items": [] })),
        ("ui-code-block", json!({ "code": "" })),
        ("ui-json-editor", json!({ "bind": "k" })),
        ("ui-kbd", json!({ "text": "K" })),
        ("ui-progress", json!({ "value": 10 })),
        ("ui-aspect-ratio", json!({ "ratio": 2 })),
        ("ui-heading", json!({ "text": "H" })),
        ("ui-select", json!({ "options": [] })),
        ("ui-radio-group", json!({ "options": ["a"] })),
        ("ui-breadcrumb", json!({ "items": [] })),
        ("ui-pagination", json!({ "page": 1, "pages": 1, "href": "#{{ p }}" })),
        ("ui-accordion-item", json!({ "title": "T" })),
        ("ui-collapsible", json!({ "trigger": "T" })),
        ("ui-tabs", json!({ "bind": "k" })),
        ("ui-tab-panel", json!({ "value": "a" })),
        ("ui-toggle", json!({ "bind": "k" })),
        ("ui-toggle-group", json!({ "bind": "k", "options": ["a"] })),
        ("ui-popover", json!({ "trigger": "T" })),
        ("ui-dropdown-menu", json!({ "trigger": "T" })),
        ("ui-menu-item", json!({ "label": "L" })),
        ("ui-menu-label", json!({ "text": "L" })),
        ("ui-tooltip", json!({ "text": "T" })),
        ("ui-hover-card", json!({ "trigger": "T" })),
        ("ui-toaster", json!({ "bind": "k" })),
        ("ui-dialog-close", json!({ "label": "L" })),
        ("ui-combobox", json!({ "options": ["a"] })),
        ("ui-split-button", json!({ "label": "Save" })),
        ("ui-theme-option", json!({ "option": "palette" })),
        ("ui-sidebar-item", json!({ "label": "Home" })),
        ("ui-sidebar-action", json!({ "label": "More" })),
        ("ui-command-item", json!({ "label": "L" })),
        ("ui-chat-input", json!({ "messages": "k" })),
        ("ui-data-table", json!({ "rows": [], "columns": [{ "field": "a" }] })),
        ("ui-menu-sub", json!({ "label": "More" })),
        ("ui-illustration", json!({ "name": "not-found" })),
        ("ui-menu-checkbox", json!({ "label": "On", "bind": "k" })),
        ("ui-menu-radio-group", json!({ "bind": "k", "options": ["a"] })),
        ("ui-menubar-menu", json!({ "label": "File" })),
        ("ui-navigation-item", json!({ "label": "Products" })),
        ("ui-navigation-link", json!({ "label": "Docs", "href": "/docs" })),
        ("ui-step", json!({ "label": "One" })),
        ("ui-repeater", json!({ "bind": "k", "fields": [{ "field": "a" }] })),
        ("ui-import", json!({ "bind": "k", "fields": [{ "field": "a" }] })),
        ("ui-stat", json!({ "label": "Orders", "value": 3 })),
        ("ui-description-item", json!({ "label": "Owner", "value": "Ada" })),
        ("ui-comments", json!({ "bind": "k" })),
        ("ui-mention-input", json!({ "bind": "k", "people": [] })),
        ("ui-tags-input", json!({ "bind": "k" })),
        ("ui-masked-input", json!({ "bind": "k", "mask": "##" })),
        ("ui-phone-input", json!({ "bind": "k" })),
        ("ui-time-picker", json!({ "bind": "k" })),
        ("ui-lightbox", json!({ "images": [] })),
        ("ui-query-builder", json!({ "bind": "k", "fields": [{ "field": "a" }] })),
        ("ui-transfer-list", json!({ "bind": "k", "options": [] })),
        ("ui-tree-select", json!({ "bind": "k", "nodes": [] })),
        ("ui-pivot", json!({ "rows": [] })),
        ("ui-scheduler", json!({ "resources": [], "events": [] })),
        ("ui-qr-code", json!({ "value": "x" })),
        ("ui-local-store", json!({ "keys": ["k"] })),
        ("ui-app-shell", json!({ "bind": "k" })),
        ("ui-app-screen", json!({ "value": "a", "label": "A" })),
        ("ui-camera", json!({ "bind": "k" })),
        ("ui-permission-manager", json!({ "permissions": ["camera"] })),
        ("ui-photo-annotate", json!({ "src": "a.png" })),
        ("ui-location", json!({ "bind": "k" })),
        ("ui-geo-checkin", json!({ "bind": "k", "places": [] })),
        ("ui-route-tracker", json!({ "bind": "k" })),
        ("ui-app-badge", json!({ "count": 1 })),
        ("ui-share-target", json!({ "bind": "k" })),
        ("ui-contact-picker", json!({ "bind": "k" })),
        ("ui-passkey", json!({ "local": true })),
        ("ui-audio-recorder", json!({ "bind": "k" })),
        ("ui-video-recorder", json!({ "bind": "k" })),
        ("ui-screen-capture", json!({ "bind": "k" })),
        ("ui-voice-input", json!({ "bind": "k" })),
        ("ui-read-aloud", json!({ "text": "x" })),
        ("ui-wake-lock", json!({ "active": true })),
        ("ui-swipe-actions", json!({ "end": [{ "value": "a" }] })),
        ("ui-document-scanner", json!({ "bind": "k" })),
        ("ui-nfc", json!({ "bind": "k" })),
        ("ui-bluetooth", json!({ "read": [{ "key": "a", "service": "battery_service", "characteristic": "battery_level" }] })),
        ("ui-usb", json!({ "filters": [] })),
        ("ui-file-handler", json!({ "bind": "k" })),
        ("ui-barcode", json!({ "value": "x" })),
        ("ui-code-scanner", json!({ "bind": "k" })),
        ("ui-diff", json!({ "before": "a", "after": "b" })),
        ("ui-log", json!({ "lines": [] })),
        ("ui-code-editor", json!({ "bind": "k" })),
        ("ui-signature", json!({ "bind": "k" })),
        ("ui-image-cropper", json!({ "bind": "k" })),
        ("ui-plan", json!({ "items": [] })),
        ("ui-pdf", json!({ "src": "/a.pdf" })),
        ("ui-tour", json!({ "bind": "k", "steps": [] })),
        ("ui-session-timeout", json!({ "after": 10 })),
        ("ui-attachment", json!({ "files": [] })),
        ("ui-sources", json!({ "items": [] })),
        ("ui-tool-call", json!({ "title": "Search" })),
        ("ui-trace", json!({ "spans": [] })),
        ("ui-questionnaire", json!({ "bind": "k", "questions": [] })),
        ("ui-gauge", json!({ "value": 50 })),
        ("ui-sortable-list", json!({ "bind": "k" })),
        ("ui-form-builder", json!({ "bind": "k" })),
        ("ui-countdown", json!({ "to": "+1h" })),
        ("ui-iframe", json!({ "src": "https://example.com" })),
        ("ui-compare-slider", json!({ "before": "/a.jpg", "after": "/b.jpg" })),
        ("ui-presence", json!({ "people": [] })),
        ("ui-virtual-list", json!({ "items": [] })),
        ("ui-file-browser", json!({ "bind": "k" })),
        ("ui-approval", json!({ "bind": "k" })),
        ("ui-whiteboard", json!({ "bind": "k" })),
        ("ui-mindmap", json!({ "bind": "k" })),
    ];
    let mut kit = 0;
    let mut broken = Vec::new();
    for c in engine.registry.iter().filter(|c| c.name().starts_with("ui-")) {
        kit += 1;
        assert!(!c.info().doc.is_empty(), "{} has no doc", c.name());
        let props = minimal.iter().find(|(n, _)| *n == c.name()).map_or(json!({}), |(_, p)| p.clone());
        let mut root = json!({ "type": c.name(), "props": props });
        if c.name() == "ui-dialog-close" {
            root = json!({ "type": "ui-dialog", "children": [root] });
        }
        if c.name() == "ui-command-page" {
            root = json!({ "type": "ui-command", "children": [{ "type": "ui-command-page", "props": { "id": "p" } }] });
        }
        if c.name() == "ui-command-item" {
            root = json!({ "type": "ui-command", "children": [root] });
        }
        match c.name() {
            "ui-menubar-menu" => root = json!({ "type": "ui-menubar", "children": [root] }),
            "ui-navigation-item" | "ui-navigation-link" => root = json!({ "type": "ui-navigation-menu", "children": [root] }),
            "ui-menu-sub" => root = json!({ "type": "ui-dropdown-menu", "props": { "trigger": "T" }, "children": [root] }),
            "ui-filters" => root["children"] = json!([{ "type": "ui-filter", "props": { "field": "a" } }]),
            "ui-filter" => root = json!({ "type": "ui-filters", "props": { "bind": "k" }, "children": [{ "type": "ui-filter", "props": { "field": "a" } }] }),
            "ui-resizable" => root = json!({ "type": "ui-resizable", "children": [{ "type": "ui-resizable-panel" }, { "type": "ui-resizable-panel" }] }),
            "ui-resizable-panel" => root = json!({ "type": "ui-resizable", "children": [{ "type": "ui-resizable-panel" }, { "type": "ui-resizable-panel" }] }),
            "ui-stepper" => root = json!({ "type": "ui-stepper", "props": { "bind": "k" }, "children": [{ "type": "ui-step", "props": { "label": "One" } }] }),
            "ui-app-shell" | "ui-app-screen" => root = json!({ "type": "ui-app-shell", "props": { "bind": "k" }, "children": [{ "type": "ui-app-screen", "props": { "value": "a", "label": "A" } }] }),
            "ui-error-summary" | "ui-save-bar" => root = json!({ "type": "ui-form", "children": [root] }),
            "ui-tile" => root = json!({ "type": "ui-dashboard-grid", "children": [root] }),
            "ui-virtual-list" => root["children"] = json!([{ "type": "text", "props": { "value": "{{ item }}" } }]),
            "ui-rail-panel" => root = json!({ "type": "ui-rail-layout", "props": { "bind": "k", "items": [{ "id": "a", "label": "A" }] }, "children": [{ "type": "ui-rail-panel", "props": { "for": "a" } }] }),
            _ => {}
        }
        if c.name().starts_with("ui-data-table-") {
            root = json!({ "type": "ui-data-table", "props": { "rows": [], "columns": [{ "field": "a" }] }, "children": [root] });
        }
        if c.name() == "ui-sidebar-trigger" {
            root = json!({ "type": "ui-sidebar-layout", "props": { "bind": "k" }, "children": [root] });
        }
        let page = render_with(root.clone(), json!({ "k": null }), json!({}))
            .unwrap_or_else(|e| panic!("{}: {e}", c.name()));
        assert!(!page.body.is_empty(), "{}", c.name());
        // With `attrs: { id }` on the component, that's its id: on one element, and what
        // its buttons and labels point at.
        fn tag(v: &mut Json, name: &str) -> bool {
            if v["type"] == json!(name) {
                v["attrs"] = json!({ "id": "picked-id" });
                return true;
            }
            v.get_mut("children").and_then(Json::as_array_mut).is_some_and(|c| c.iter_mut().any(|k| tag(k, name)))
        }
        let mut with_id = root;
        assert!(tag(&mut with_id, c.name()), "{}", c.name());
        let body = render_with(with_id, json!({ "k": null }), json!({})).unwrap_or_else(|e| panic!("{}: {e}", c.name())).body;
        let ids: Vec<&str> = body.split(" id=\"").skip(1).map(|r| &r[..r.find('"').unwrap()]).collect();
        let n = ids.iter().filter(|i| **i == "picked-id").count();
        if n > 1 {
            broken.push(format!("{}: `picked-id` on {n} elements", c.name()));
        }
        for attr in ["popovertarget", "commandfor", "aria-controls", "aria-labelledby", "aria-describedby", "for", "list", "aria-activedescendant", "aria-owns"] {
            for r in body.split(&format!(" {attr}=\"")).skip(1) {
                for target in r[..r.find('"').unwrap()].split_whitespace() {
                    if !ids.contains(&target) {
                        broken.push(format!("{}: {attr}=\"{target}\" points at nothing", c.name()));
                    }
                }
            }
        }
    }
    assert!(broken.is_empty(), "\n{}", broken.join("\n"));
    assert_eq!(kit, 211);
    // The built-ins are untouched.
    for builtin in ["element", "text", "stack", "button", "input", "select", "textarea", "list", "if"] {
        assert!(engine.registry.get(builtin).is_some(), "{builtin}");
    }
}

#[test]
fn button_variants_sizes_and_links() {
    let page = render_with(
        json!({ "type": "element", "children": [
            { "type": "ui-button", "props": { "label": "A" } },
            { "type": "ui-button", "props": { "label": "A", "variant": "destructive", "size": "sm" } },
            { "type": "ui-button", "props": { "label": "Go", "href": "/next", "disabled": true } },
            { "type": "ui-button", "props": { "label": "{{ v }}", "variant": "{{ v }}" } }
        ] }),
        json!({}),
        json!({ "v": "nonsense" }),
    )
    .unwrap();
    let body = plain(&page.body);
    assert_eq!(
        body,
        concat!(
            r#"<div><button type="button">A</button><button type="button">A</button>"#,
            r#"<a href="/next" aria-disabled="true">Go</a><button type="button">nonsense</button></div>"#
        )
    );
    // Sizes follow the theme's `space.scale` (density) and `size.control`.
    for decl in ["background: var(--color-destructive, #dc2626);", "height: calc(32px * var(--space-scale, 1) * var(--size-control, 1));", "height: calc(36px * var(--space-scale, 1) * var(--size-control, 1));"] {
        assert!(page.css.contains(decl), "missing `{decl}`");
    }
}

#[test]
fn card_alert_and_typography_structure() {
    let body = plain(&render(json!({ "type": "ui-card", "props": { "title": "T", "description": "D" },
        "parts": { "title": { "attrs": { "id": "t" } } },
        "children": [{ "type": "ui-card-footer", "children": [{ "type": "ui-button", "props": { "label": "OK" } }] }] })));
    assert_eq!(body, r#"<div><div><h3 id="t">T</h3><p>D</p></div><div><button type="button">OK</button></div></div>"#);

    let body = plain(&render(json!({ "type": "ui-card", "props": { "icon": "★", "title": "T", "href": "/orders", "variant": "elevated" } })));
    assert_eq!(body, r#"<a href="/orders"><div><span aria-hidden="true" data-icon>★</span><h3>T</h3></div></a>"#);

    let body = plain(&render(json!({ "type": "ui-alert", "props": { "icon": "⚠", "title": "Oops", "variant": "destructive" } })));
    assert_eq!(body, r#"<div role="alert"><span aria-hidden="true" data-icon>⚠</span><div>Oops</div></div>"#);
    let body = plain(&render(json!({ "type": "ui-alert", "props": { "icon": "none", "title": "Done", "variant": "success" } })));
    assert_eq!(body, r#"<div role="alert"><div>Done</div></div>"#);
    let body = plain(&render(json!({ "type": "ui-alert", "props": { "title": "Note", "variant": "info" } })));
    assert!(body.starts_with(r#"<div role="alert"><span aria-hidden="true" data-icon><svg "#), "a variant brings its own icon: {body}");
    let body = render(json!({ "type": "ui-alert", "props": { "title": "Saved", "variant": "success", "dismissible": true } }));
    assert!(body.contains("data-dismissible") && body.contains(r#"aria-label="Dismiss" data-dismiss>✕</button>"#), "{body}");

    let body = plain(&render(json!({ "type": "element", "children": [
        { "type": "ui-heading", "props": { "text": "H", "level": 3 } },
        { "type": "ui-heading", "props": { "text": "H", "level": 9 } },
        { "type": "ui-text", "props": { "variant": "code", "text": "x" } },
        { "type": "ui-text", "props": { "variant": "blockquote", "text": "q" } } ] })));
    assert_eq!(body, "<div><h3>H</h3><h2>H</h2><code>x</code><blockquote>q</blockquote></div>");
}

#[test]
fn progress_follows_state_and_clamps_in_css() {
    let page = render_with(json!({ "type": "ui-progress", "props": { "value": "{{ done }}" } }), json!({ "done": 30 }), json!({}))
        .unwrap();
    assert!(page.body.contains(r#"role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow="30""#), "{}", page.body);
    // The value is a live CSS variable with `%` appended; CSS clamps it.
    assert!(page.body.contains(r#"style="--i-d0: 30%""#), "{}", page.body);
    assert!(page.body.contains("[{&quot;p&quot;:[&quot;done&quot;]},&quot;%&quot;]"), "{}", page.body);
    assert!(page.css.contains("--ui-value: var(--i-d0);"), "{}", page.css);
    assert!(page.css.contains("width: min(100%, max(0%, var(--ui-value)));"), "{}", page.css);
    let page = render_with(json!({ "type": "ui-progress", "props": { "value": 55 } }), json!({}), json!({})).unwrap();
    assert!(page.css.contains("--ui-value: 55%;"), "{}", page.css);
}

#[test]
fn checkbox_switch_and_radio_group_bind_like_the_builtins() {
    let body = plain(
        &render_with(
            json!({ "type": "element", "children": [
                { "type": "ui-checkbox", "props": { "bind": "ok", "label": "OK" } },
                { "type": "ui-switch", "props": { "bind": "on", "label": "On" }, "parts": { "control": { "attrs": { "id": "sw" } } } },
                { "type": "ui-radio-group", "props": { "bind": "plan",
                    "options": ["free", { "value": "pro", "label": "Pro" }, { "value": "team", "disabled": true }] } } ] }),
            json!({ "ok": true, "on": false, "plan": "pro" }),
            json!({}),
        )
        .unwrap()
        .body,
    );
    assert_eq!(
        body,
        concat!(
            r#"<div><label><input type="checkbox" checked data-i-bind="ok"><span>OK</span></label>"#,
            r#"<label><input id="sw" role="switch" type="checkbox" data-i-bind="on"><span>On</span></label>"#,
            r#"<div role="radiogroup">"#,
            r#"<label><input type="radio" name="plan" value="free" data-i-bind="plan"><span>free</span></label>"#,
            r#"<label><input type="radio" name="plan" value="pro" checked data-i-bind="plan"><span>Pro</span></label>"#,
            r#"<label><input type="radio" name="plan" disabled value="team" data-i-bind="plan"><span>team</span></label>"#,
            "</div></div>"
        )
    );
}

#[test]
fn unbound_radio_group_shares_a_name_and_checks_its_value() {
    let body = plain(&render(json!({ "type": "ui-radio-group", "props": { "options": ["a", "b"], "value": "b" } })));
    assert_eq!(
        body,
        concat!(
            r#"<div role="radiogroup"><label><input type="radio" name="ui-radio-root" value="a"><span>a</span></label>"#,
            r#"<label><input checked type="radio" name="ui-radio-root" value="b"><span>b</span></label></div>"#
        )
    );
}

#[test]
fn lists_from_state_render_now_and_rebuild_in_the_browser() {
    let page = render_with(
        json!({ "type": "element", "children": [
            { "type": "ui-radio-group", "props": { "bind": "pick", "options": "{{ opts }}" } },
            { "type": "ui-toggle-group", "props": { "bind": "pick", "options": "{{ opts }}" } },
            { "type": "ui-breadcrumb", "props": { "items": "{{ crumbs }}" } },
            { "type": "ui-pagination", "props": { "page": "{{ page }}", "pages": 5, "href": "#{{ p }}" } } ] }),
        json!({ "opts": ["a", { "value": "b", "label": "B", "disabled": true }], "pick": "b",
                "crumbs": [{ "label": "Home", "href": "/" }, { "label": "Docs" }], "page": 2 }),
        json!({}),
    )
    .unwrap();
    let body = plain(&page.body);
    // Each list carries its item template for the browser, and pagination a behaviour
    // that redraws its window…
    assert_eq!(body.matches("data-i-list=").count(), 3, "{body}");
    assert!(body.contains(r#"data-i-x="ui-pagination""#), "{body}");
    // …and the items for the state it starts with.
    assert!(body.contains(r#"value="a""#) && body.contains(r#"value="b""#), "{body}");
    assert!(body.contains(">B</span>") && body.contains(">B</button>"), "{body}");
    assert!(body.contains(r#">Home</a>"#) && body.contains(r#"aria-current="page""#), "{body}");
    assert!(body.contains(r##"href="#1""##) && body.contains(r##"href="#3""##), "{body}");
    // A breadcrumb from state has to say where its items are, to find the last.
    let err = render_with(
        json!({ "type": "ui-breadcrumb", "props": { "items": "{{ show ? crumbs : crumbs }}" } }),
        json!({ "show": true, "crumbs": [] }),
        json!({}),
    );
    assert!(err.is_err());
}

#[test]
fn breadcrumb_and_pagination() {
    let body = plain(&render(json!({ "type": "ui-breadcrumb",
        "props": { "items": [{ "label": "Home", "href": "/" }, { "label": "Docs", "href": "/docs" }, { "label": "Kit" }] } })));
    assert_eq!(
        body,
        concat!(
            r#"<nav aria-label="breadcrumb"><ol><li><a href="/">Home</a></li>"#,
            r#"<li role="presentation" aria-hidden="true">›</li><li><a href="/docs">Docs</a></li>"#,
            r#"<li role="presentation" aria-hidden="true">›</li><li><span aria-current="page">Kit</span></li></ol></nav>"#
        )
    );

    let page = render_with(
        json!({ "type": "ui-pagination", "props": { "page": "{{ page }}", "pages": 9, "href": "/orders?page={{ p }}&q={{ q }}" } }),
        json!({}),
        json!({ "page": 1, "q": "mugs" }),
    )
    .unwrap();
    let body = plain(&page.body);
    assert!(body.contains(r#"<a aria-disabled="true" aria-label="Go to previous page">‹ Previous</a>"#), "{body}");
    assert!(body.contains(r#"<a href="/orders?page=1&amp;q=mugs" aria-current="page">1</a>"#), "{body}");
    assert!(body.contains(r#"<a href="/orders?page=9&amp;q=mugs">9</a>"#), "{body}");
    assert!(body.contains(r#"<a href="/orders?page=2&amp;q=mugs" aria-label="Go to next page">Next ›</a>"#), "{body}");
}

/// The page numbers and gaps a pagination shows, e.g. `1 … 4 5 6 … 10`.
fn window(page: u64, pages: u64) -> String {
    let body = render(json!({ "type": "ui-pagination", "props": { "page": page, "pages": pages, "href": "#{{ p }}" } }));
    let text: Vec<String> = body
        .split('>')
        .filter_map(|s| s.strip_suffix("</a").or(s.strip_suffix("</span")))
        .filter(|t| !t.contains('›') && !t.contains('‹'))
        .map(str::to_string)
        .collect();
    text.join(" ")
}

#[test]
fn pagination_windows() {
    assert_eq!(window(1, 1), "1");
    assert_eq!(window(1, 3), "1 2 3");
    assert_eq!(window(1, 10), "1 2 … 10");
    assert_eq!(window(5, 10), "1 … 4 5 6 … 10");
    assert_eq!(window(3, 10), "1 2 3 4 … 10");
    assert_eq!(window(10, 10), "1 … 9 10");
    assert_eq!(window(4, 7), "1 2 3 4 5 6 7");
    // Out of range: kept within 1..pages.
    assert_eq!(window(99, 10), "1 … 9 10");
}

#[test]
fn untrusted_policy_drops_unsafe_urls() {
    // Like the built-ins: the trusted policy allows any URL, the untrusted one drops
    // unsafe ones at render, including URLs the kit builds from props.
    let mut engine = Engine::untrusted();
    irender_ui_kit::register(&mut engine.registry);
    let doc = json!({ "version": 1, "root": { "type": "element", "children": [
        { "type": "ui-button", "props": { "label": "x", "href": "{{ bad }}" } },
        { "type": "ui-breadcrumb", "props": { "items": [{ "label": "x", "href": "javascript:alert(1)" }, { "label": "y" }] } },
        { "type": "ui-pagination", "props": { "page": 1, "pages": 2, "href": "{{ bad }}" } },
        { "type": "ui-avatar", "props": { "src": "{{ bad }}" } },
        { "type": "ui-card", "props": { "title": "c", "href": "{{ bad }}" } }
    ] } });
    let ir = engine.load(&doc.to_string(), Format::Json).unwrap_or_else(|d| panic!("{d}"));
    let body = engine.render(&ir, &json!({ "bad": "javascript:alert(1)" })).unwrap().body;
    assert!(!body.contains("javascript:"), "{body}");
    assert!(plain(&body).starts_with("<div><a>x</a>"), "{body}");
}

#[test]
fn gallery_example_renders() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/ui-kit/gallery.yaml")).unwrap();
    let engine = engine();
    let mut doc = irender_core::parse(&src, Format::Yaml).unwrap_or_else(|d| panic!("{d}"));
    irender_ui_kit::apply_theme(&mut doc);
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    assert!(ir.warnings.is_empty(), "{}", ir.warnings);
    let page = engine.render(&ir, &json!({})).unwrap();
    assert!(page.interactive);
    assert!(page.css.contains("--color-destructive: #dc2626;"), "the preset's tokens are on :root");
}

// ---------------------------------------------------------------- phase 2

#[test]
fn dialog_uses_invoker_commands_and_scopes_its_close_buttons() {
    let body = plain(&render(json!({ "type": "ui-dialog",
        "props": { "trigger": "Edit", "title": "Edit profile", "description": "Make changes.", "id": "edit" },
        "children": [{ "type": "ui-dialog-footer", "children": [
            { "type": "ui-dialog-close", "props": { "label": "Save" },
              "on": { "click": { "action": "call", "name": "save" } } } ] }] })));
    assert!(body.starts_with(r#"<button command="show-modal" commandfor="edit" aria-haspopup="dialog" type="button">Edit</button>"#), "{body}");
    assert!(body.contains(r#"<dialog id="edit" closedby="any" aria-labelledby="edit-title" aria-describedby="edit-description">"#), "{body}");
    assert!(body.contains(r#"<h2 id="edit-title">Edit profile</h2><p id="edit-description">Make changes.</p>"#), "{body}");
    // The close button knows its dialog and keeps its own action.
    assert!(body.contains(r#"<button command="close" commandfor="edit" type="button" data-i-on="{&quot;click&quot;:[{&quot;a&quot;:&quot;call&quot;"#), "{body}");
    assert!(body.contains(r#"<button type="button" command="close" commandfor="edit" aria-label="Close">✕</button></dialog>"#), "{body}");

    let alert = plain(&render(json!({ "type": "ui-alert-dialog", "props": { "title": "Sure?", "id": "a" } })));
    assert!(alert.contains(r#"<dialog id="a" role="alertdialog" closedby="none""#), "{alert}");
    assert!(!alert.contains("aria-label=\"Close\""), "an alert dialog has no ✕: {alert}");

    let err = render_with(json!({ "type": "ui-dialog-close", "props": { "label": "x" } }), json!({}), json!({})).unwrap_err();
    assert!(err.message.contains("must be inside"), "{err}");
}

#[test]
fn ids_stay_unique_across_server_rendered_list_items() {
    let page = render_with(
        json!({ "type": "list", "props": { "items": "{{ rows }}" },
                "children": [{ "type": "ui-popover", "props": { "trigger": "{{ item }}" } }] }),
        json!({}),
        json!({ "rows": ["a", "b"] }),
    )
    .unwrap();
    let ids: Vec<&str> = page.body.match_indices(r#"popover="auto" id=""#).map(|(i, m)| {
        let rest = &page.body[i + m.len()..];
        &rest[..rest.find('"').unwrap()]
    }).collect();
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1], "{ids:?}");
    // The same document renders the same ids every time.
    let again = render_with(
        json!({ "type": "list", "props": { "items": "{{ rows }}" },
                "children": [{ "type": "ui-popover", "props": { "trigger": "{{ item }}" } }] }),
        json!({}),
        json!({ "rows": ["a", "b"] }),
    )
    .unwrap();
    assert_eq!(page.body, again.body);
}

#[test]
fn overlays_in_browser_rebuilt_lists_get_live_ids() {
    let row = |dialog: Json| json!({ "type": "list", "props": { "items": "{{ rows }}", "as": "r" },
        "children": [dialog, { "type": "ui-dropdown-menu", "props": { "trigger": "Menu" },
            "children": [{ "type": "ui-menu-item", "props": { "label": "Edit" } }] }] });
    let close = json!([{ "type": "ui-dialog-close", "props": { "label": "Done" } }]);
    let page = render_with(row(json!({ "type": "ui-dialog", "props": { "trigger": "Open" }, "children": close })),
        json!({ "rows": [{ "id": 7 }] }), json!({})).unwrap();
    let tpl = &page.body[page.body.find("<template>").unwrap()..page.body.find("</template>").unwrap()];
    let tpl = tpl.replace("&quot;", "\"");
    // Items share a scope key, so the index tells their ids apart; every reference follows.
    let live = |name: &str| tpl.matches(&format!(r#""n":"{name}","v":{{"t":["#)).count();
    assert_eq!(live("id"), 2, "dialog and menu: {tpl}");
    assert_eq!(live("commandfor"), 3, "trigger, close button and ✕: {tpl}");
    assert_eq!(live("popovertarget"), 2, "menu trigger and item: {tpl}");
    assert!(tpl.contains(r#"{"p":["index"]}"#), "{tpl}");

    // A templated `id` reaches the close button as a template too.
    let page = render_with(
        row(json!({ "type": "ui-dialog", "props": { "trigger": "Open", "id": "dlg-{{ r.id }}" }, "children": close })),
        json!({ "rows": [{ "id": 7 }] }), json!({})).unwrap();
    let tpl = page.body[page.body.find("<template>").unwrap()..].replace("&quot;", "\"");
    assert_eq!(tpl.matches(r#""n":"commandfor","v":{"t":["dlg-",{"p":["r","id"]}]}"#).count(), 3, "{tpl}");
    // The server-rendered item has plain ids.
    assert!(plain(&page.body).contains(r#"<dialog id="dlg-7""#), "{}", page.body);
}

#[test]
fn popover_anchors_to_its_trigger() {
    let page = render_with(
        json!({ "type": "ui-dropdown-menu", "props": { "trigger": "Open", "id": "m" }, "children": [
            { "type": "ui-menu-label", "props": { "text": "Account" } },
            { "type": "ui-menu-item", "props": { "label": "Profile", "shortcut": "⇧⌘P" } },
            { "type": "ui-menu-separator" },
            { "type": "ui-menu-item", "props": { "label": "Log out", "variant": "destructive" } } ] }),
        json!({}),
        json!({}),
    )
    .unwrap();
    let body = plain(&page.body);
    assert!(body.starts_with(r#"<button popovertarget="m" aria-haspopup="menu" type="button">Open</button><div popover="auto" id="m" role="menu" data-i-x="ui-dropdown-menu""#), "{body}");
    assert!(body.contains(r#"<button role="menuitem" tabindex="-1" type="button" popovertarget="m" popovertargetaction="hide"><span>Profile</span><span>⇧⌘P</span></button>"#), "{body}");
    for decl in ["anchor-name: --m;", "position-anchor: --m;", "position-area: bottom span-right;", "position-try-fallbacks: flip-block;"] {
        assert!(page.css.contains(decl), "missing `{decl}`");
    }
}

#[test]
fn tabs_toggle_and_group_drive_state() {
    let page = render_with(
        json!({ "type": "element", "children": [
            { "type": "ui-tabs", "props": { "bind": "tab", "id": "t" }, "children": [
                { "type": "ui-tab-panel", "props": { "value": "account", "label": "Account" },
                  "children": [{ "type": "text", "props": { "value": "A" } }] },
                { "type": "ui-tab-panel", "props": { "value": "password", "label": "Password" } } ] },
            { "type": "ui-toggle", "props": { "bind": "bold", "label": "B" } },
            { "type": "ui-toggle-group", "props": { "bind": "align", "options": ["left", "right"] } } ] }),
        json!({ "tab": "account", "bold": true, "align": "right" }),
        json!({}),
    )
    .unwrap();
    let body = plain(&page.body);
    assert!(body.contains(r#"<button type="button" role="tab" id="t-tab-0" aria-controls="t-panel-0" aria-selected="true""#), "{body}");
    assert!(body.contains(r#"aria-selected="false""#), "{body}");
    assert!(body.contains(r#"<div role="tabpanel" id="t-panel-0" aria-labelledby="t-tab-0" tabindex="0" data-i-b"#), "{body}");
    assert!(body.contains(r#"<div role="tabpanel" id="t-panel-1" aria-labelledby="t-tab-1" tabindex="0" hidden data-i-b"#), "{body}");
    assert!(body.contains(r#"<button type="button" aria-pressed="true""#), "{body}");
    // Pressing the pressed option of a group clears it.
    assert!(body.contains("&quot;c&quot;:[{&quot;b&quot;:&quot;==&quot;"), "{body}");
    let err = render_with(json!({ "type": "ui-tabs", "props": { "bind": "tab" }, "children": [{ "type": "element" }] }),
        json!({ "tab": 1 }), json!({})).unwrap_err();
    assert!(err.message.contains("must be `ui-tab-panel`"), "{err}");
}

#[test]
fn multiple_toggle_group_presses_each_option_in_the_list() {
    let page = render_with(
        json!({ "type": "ui-toggle-group", "props": { "bind": "marks", "multiple": true, "options": ["b", "i", "u"] } }),
        json!({ "marks": ["b", "u"] }),
        json!({}),
    )
    .unwrap();
    let body = plain(&page.body);
    let pressed: Vec<&str> = body.split("aria-pressed=\"").skip(1).map(|r| &r[..r.find('"').unwrap()]).collect();
    assert_eq!(pressed, ["true", "false", "true"], "{body}");
    // Pressed follows `includes`, and a press `toggle`s the option in the list.
    assert!(body.contains("{&quot;f&quot;:&quot;includes&quot;,&quot;a&quot;:[{&quot;p&quot;:[&quot;marks&quot;]},{&quot;l&quot;:&quot;i&quot;}]}"), "{body}");
    assert!(body.contains("{&quot;f&quot;:&quot;toggle&quot;,&quot;a&quot;:[{&quot;p&quot;:[&quot;marks&quot;]},{&quot;l&quot;:&quot;i&quot;}]}"), "{body}");
}

#[test]
fn split_button_joins_a_primary_action_to_a_menu() {
    let page = render_with(
        json!({ "type": "ui-split-button",
            "props": { "label": "Download PDF", "variant": "outline", "size": "sm", "id": "dl" },
            "attrs": { "data-test": "split" },
            "on": { "click": { "action": "set_state", "key": "last", "value": "pdf" } },
            "children": [ { "type": "ui-menu-item", "props": { "label": "Download Excel" },
                            "on": { "click": { "action": "set_state", "key": "last", "value": "xlsx" } } } ] }),
        json!({ "last": null }),
        json!({}),
    )
    .unwrap();
    let body = plain(&page.body);
    // The wrapper keeps the node's attrs; the primary button gets its click.
    assert!(body.starts_with(r#"<div data-test="split" role="group"><button type="button" data-i-on="#), "{body}");
    assert!(body.contains(r#">Download PDF</button>"#), "{body}");
    assert_eq!(body.matches("data-i-on=").count(), 2, "the primary action and the menu item: {body}");
    // The arrow opens the menu and says what it is.
    assert!(body.contains(r#"<button popovertarget="dl" aria-haspopup="menu" aria-label="More actions" type="button">▾</button>"#), "{body}");
    assert!(body.contains(r#"popover="auto" id="dl" role="menu""#), "{body}");
    assert!(body.contains(r#"role="menuitem""#), "{body}");
}

#[test]
fn sidebar_layout_panel_groups_and_items() {
    let page = render_with(
        json!({ "type": "ui-sidebar-layout", "props": { "bind": "open", "collapsible": "icon" }, "children": [
            { "type": "ui-sidebar", "children": [
                { "type": "ui-sidebar-content", "children": [
                    { "type": "ui-sidebar-group", "props": { "label": "Platform" }, "children": [
                        { "type": "ui-sidebar-item", "props": { "label": "Home", "icon": "⌂", "href": "/", "active": true, "badge": 3 } },
                        { "type": "ui-sidebar-item", "props": { "label": "Docs", "open": true }, "children": [
                            { "type": "ui-sidebar-item", "props": { "label": "Intro", "href": "/intro" } } ] } ] } ] } ] },
            { "type": "ui-sidebar-trigger" },
            { "type": "text", "props": { "value": "Main" } } ] }),
        json!({ "open": true }),
        json!({}),
    )
    .unwrap();
    let body = plain(&page.body);
    // The layout's state follows `bind`; the panel comes first, then the main area.
    assert!(body.contains(r#"data-sidebar-layout data-state="expanded" data-collapsible="icon" data-variant="sidebar" data-side="left""#), "{body}");
    // A behaviour makes it a sheet on phones and adds the keyboard shortcut.
    assert!(body.contains(r#"data-i-x="ui-sidebar" data-i-xc="{&quot;key&quot;:&quot;open&quot;}""#), "{body}");
    assert!(body.contains(r#"<aside data-sidebar aria-label="Sidebar">"#), "{body}");
    assert!(body.contains(r#"<main><button type="button""#), "the trigger is in the main area: {body}");
    // An item: a link with its icon, label and badge, current when active.
    assert!(
        body.contains(r#"<li><a data-sidebar-item data-tip="Home" href="/" aria-current="page"><span aria-hidden="true" data-icon>⌂</span><span data-label>Home</span><span data-badge>3</span></a></li>"#),
        "{body}"
    );
    // One with children opens a sub-menu.
    assert!(body.contains(r#"<li><details open><summary data-sidebar-item data-tip="Docs"><span data-label>Docs</span><span data-chevron>"#), "{body}");
    assert!(body.contains(r#"<li><a data-sidebar-item href="/intro"><span data-label>Intro</span></a></li></ul></details></li>"#), "{body}");
    // The trigger flips the bound key.
    assert!(body.contains(r#"aria-label="Toggle sidebar" data-sidebar-trigger"#), "{body}");
    assert!(body.contains("&quot;k&quot;:&quot;open&quot;"), "{body}");
    // A trigger outside a bound layout is an error, not a dead button.
    let err = render_with(json!({ "type": "ui-sidebar-trigger" }), json!({}), json!({})).unwrap_err();
    assert!(err.message.contains("inside a `ui-sidebar-layout` with `bind`"), "{err}");
}

#[test]
fn separator_label_button_and_badge_icons() {
    let body = plain(&render(json!({ "type": "element", "children": [
        { "type": "ui-separator", "props": { "label": "Or continue with" } },
        { "type": "ui-button", "props": { "label": "Add", "icon": "+" } },
        { "type": "ui-badge", "props": { "text": "+12%", "icon": "↗", "variant": "outline" } } ] })));
    assert!(body.contains("<div>Or continue with</div>"), "{body}");
    assert!(body.contains(r#"<button type="button"><span aria-hidden="true" data-icon>+</span><span>Add</span></button>"#), "{body}");
    assert!(body.contains(r#"<span><span aria-hidden="true" data-icon>↗</span><span>+12%</span></span>"#), "{body}");
    let body = plain(&render(json!({ "type": "ui-badge", "props": { "text": "Live", "dot": true, "variant": "success" } })));
    assert_eq!(body, r#"<span><span aria-hidden="true" data-dot></span><span>Live</span></span>"#);
}

#[test]
fn theme_option_lists_its_choices_with_swatches() {
    let page = render_with(json!({ "type": "ui-theme-option", "props": { "option": "palette", "swatch": "color.primary" } }), json!({}), json!({})).unwrap();
    let body = plain(&page.body);
    assert!(body.starts_with(r#"<div role="group" aria-label="Colour" data-i-x="ui-theme-option" data-i-xc="{&quot;option&quot;:&quot;palette&quot;,&quot;fallback&quot;:&quot;zinc&quot;,&quot;bind&quot;:null}">"#), "{body}");
    // The default is pressed until the behaviour reads the stored choice.
    // Zinc, the default, sets no primary of its own: its swatch is the base colour.
    assert!(body.contains(r#"<button type="button" data-choice="zinc" aria-pressed="true"><span aria-hidden="true"><span></span></span>Zinc</button>"#), "{body}");
    assert!(page.css.contains("background: #18181b;"), "zinc's swatch");
    assert!(body.contains(r#"<button type="button" data-choice="rose" aria-pressed="false"><span aria-hidden="true"><span></span></span>Rose</button>"#), "{body}");
    assert!(page.css.contains("background: #e11d48;"), "rose's swatch");
    // The kit's options are in the theme CSS, each with its dark colours.
    assert!(page.css.contains(r#":root[data-theme-palette="rose"], [data-theme-palette="rose"] {"#) && page.css.contains(r#":root[data-theme-density="compact"], [data-theme-density="compact"] {"#));
    let err = render_with(json!({ "type": "ui-theme-option", "props": { "option": "nope" } }), json!({}), json!({})).unwrap_err();
    assert!(err.message.contains("`nope` is not a theme option (options: `palette`, `radius`, `border`, `elevation`, `control-radius`, `card-radius`, `density`, `padding`, `gap`, `control-size`, `icon-size`, `text-size`, `font`, `heading-font`, `font-weight`, `line-height`, `letter-spacing`)"), "{err}");
}

#[test]
fn theme_panel_has_a_picker_per_option_and_the_mode() {
    let doc = json!({ "type": "ui-theme-panel" });
    let body = plain(&render(doc));
    let labels: Vec<&str> = body.split(r#"<div aria-hidden="true">"#).skip(1).map(|r| &r[..r.find('<').unwrap()]).collect();
    // The palette first and chart colours (not registered here) last.
    // Custom colours come with the palette, which has a `custom` choice.
    assert_eq!(labels, ["Colour", "Custom colours", "Radius", "Borders", "Shadows", "Control radius", "Card radius", "Density", "Padding", "Gaps", "Control size", "Icon size", "Text size", "Font", "Heading font", "Weight", "Line height", "Letter spacing", "Mode"], "{body}");
    assert!(body.contains(r#"data-i-xc="{&quot;option&quot;:&quot;palette&quot;"#) && body.contains(r#"data-i-x="ui-theme-toggle""#), "{body}");
    let some = plain(&render(json!({ "type": "ui-theme-panel", "props": { "options": ["radius"], "mode": false } })));
    assert_eq!(some.matches("data-i-x=").count(), 1, "{some}");
}

#[test]
fn custom_colours_and_the_brand_variant() {
    let page = render_with(
        json!({ "type": "element", "children": [
            { "type": "ui-theme-colors" },
            { "type": "ui-button", "props": { "label": "Go", "variant": "brand" } },
            { "type": "ui-badge", "props": { "text": "New", "variant": "brand" } },
            { "type": "ui-theme-option", "props": { "option": "palette", "swatch": "color.primary" } } ] }),
        json!({}),
        json!({}),
    )
    .unwrap();
    let body = plain(&page.body);
    // Each colour starts from the theme's, as a colour input and as hex.
    assert!(body.contains(r##"<label data-color="primary" data-probe="var(--color-primary)">Primary<input type="color" value="#18181b" aria-label="Primary, picked">"##), "{body}");
    assert!(body.contains(r##"<input type="text" value="#18181b" aria-label="Primary, as hex" spellcheck="false" maxlength="7" placeholder="#rrggbb">"##), "{body}");
    assert!(body.contains(r##"data-color="brand-2" data-probe="var(--color-brand-2, var(--color-primary))">Second colour"##), "{body}");
    // A gradient end starts empty, its picker on the colour it follows.
    assert!(body.contains(r##"<label data-color="primary-to">Primary gradient end<input type="color" value="#18181b" aria-label="Primary gradient end, picked"><input type="text" value=""##), "{body}");
    // Backgrounds, per mode, from the surface colour in themes without their own.
    assert!(body.contains(r##"<div>Backgrounds, <span data-mode>light mode</span></div>"##), "{body}");
    assert!(body.contains(r##"<label data-color="sheet" data-probe="var(--color-sheet, var(--color-dialog, var(--color-surface)))" data-per-mode>Sheets<input type="color" value="#ffffff""##), "{body}");
    // The second brand colour falls back to the primary in themes without one, and
    // brand fills take a gradient when the theme has one.
    assert!(page.css.contains("background: var(--gradient-brand-2, var(--color-brand-2, var(--gradient-primary, var(--color-primary))));"), "{}", page.css);
    let primary = render_with(json!({ "type": "ui-button", "props": { "label": "Go" } }), json!({}), json!({})).unwrap();
    assert!(primary.css.contains("background: var(--gradient-primary, var(--color-primary));"), "{}", primary.css);
    assert!(primary.css.contains("background: var(--gradient-primary-hover, var(--color-primary-hover));"), "{}", primary.css);
    let some = plain(&render(json!({ "type": "ui-theme-colors", "props": { "gradients": false, "backgrounds": false } })));
    assert_eq!(some.matches("<label").count(), 2, "{some}");
    // The custom choice's swatch is a rainbow.
    assert!(body.contains(r##"data-choice="custom" aria-pressed="false"><span aria-hidden="true"></span>Custom</button>"##), "{body}");
    assert!(page.css.contains("conic-gradient("));
    let err = render_with(json!({ "type": "ui-theme-colors", "props": { "option": "radius" } }), json!({}), json!({})).unwrap_err();
    assert!(err.message.contains("`radius` has no `custom` choice"), "{err}");
}

#[test]
fn accordion_items_share_a_name_unless_multiple() {
    let body = plain(&render(json!({ "type": "ui-accordion", "props": { "id": "faq" }, "children": [
        { "type": "ui-accordion-item", "props": { "title": "Q1", "open": true }, "children": [{ "type": "text", "props": { "value": "A1" } }] },
        { "type": "ui-accordion-item", "props": { "title": "Q2" } } ] })));
    assert_eq!(
        body,
        concat!(
            r#"<div id="faq"><details name="faq" open><summary>Q1</summary><div><span>A1</span></div></details>"#,
            r#"<details name="faq"><summary>Q2</summary><div></div></details></div>"#
        )
    );
    let body = plain(&render(json!({ "type": "ui-accordion", "props": { "multiple": true }, "children": [
        { "type": "ui-accordion-item", "props": { "title": "Q" } } ] })));
    assert!(!body.contains("name="), "{body}");
}

#[test]
fn wait_action_is_validated_and_serialized() {
    let ok = render_with(
        json!({ "type": "ui-button", "props": { "label": "x" }, "on": { "click": [
            { "action": "set_state", "key": "t", "value": { "title": "Saved" } }, { "action": "wait", "ms": 1500 },
            { "action": "set_state", "key": "t", "value": null } ] } }),
        json!({ "t": null }),
        json!({}),
    )
    .unwrap();
    assert!(ok.body.contains("{&quot;a&quot;:&quot;wait&quot;,&quot;ms&quot;:1500}"), "{}", ok.body);
    let doc = json!({ "version": 1, "root": { "type": "element", "on": { "click": { "action": "wait", "ms": 600000 } } } });
    let err = Engine::new().load(&doc.to_string(), Format::Json).unwrap_err();
    assert_eq!(err.to_string(), "root.on.click[0].ms: waits are at most 60000 ms");
}

#[test]
fn skeletons_pulse_with_one_keyframes_rule() {
    let page = render_with(
        json!({ "type": "element", "children": [
            { "type": "ui-skeleton" }, { "type": "ui-skeleton", "props": { "round": true } }, { "type": "ui-skeleton" } ] }),
        json!({}),
        json!({}),
    )
    .unwrap();
    assert_eq!(page.css.matches("@keyframes ui-pulse").count(), 1, "{}", page.css);
    assert!(page.css.contains("animation: ui-pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;"), "{}", page.css);
    assert!(page.css.contains("@media (prefers-reduced-motion: reduce)"), "{}", page.css);
    // Pages without a skeleton don't carry the rule.
    assert!(!render_with(json!({ "type": "ui-badge", "props": { "text": "x" } }), json!({}), json!({})).unwrap().css.contains("@keyframes"));
}

#[test]
fn field_groups_empty_spinner_and_scroll_area_structure() {
    let body = render(json!({ "type": "element", "children": [
        { "type": "ui-field", "props": { "label": "Email", "for": "e", "description": "Help", "error": "Bad" },
          "children": [ { "type": "ui-input-group", "props": { "prefix": "@", "suffix": ".com" },
            "children": [ { "type": "ui-input", "attrs": { "id": "e" } } ] } ] },
        { "type": "ui-button-group", "props": { "label": "View", "orientation": "vertical" } },
        { "type": "ui-empty", "props": { "icon": "!", "title": "None", "description": "Add one" } },
        { "type": "ui-spinner", "props": { "label": "Saving" } },
        { "type": "ui-scroll-area", "props": { "maxHeight": 100 } },
        { "type": "ui-scroll-area", "props": { "label": "Log" } } ] }));
    assert_eq!(
        plain(&body),
        concat!(
            r#"<div><div><label for="e">Email</label><div><span>@</span>"#,
            r#"<input id="e" aria-describedby="e-description e-error" type="text"><span>.com</span></div>"#,
            r#"<p id="e-description">Help</p><p aria-live="polite" id="e-error">Bad</p></div>"#,
            r#"<div role="group" aria-orientation="vertical" aria-label="View"></div>"#,
            r#"<div><div aria-hidden="true">!</div><h3>None</h3><p>Add one</p></div>"#,
            r#"<span role="status" aria-label="Saving"></span>"#,
            r#"<div tabindex="0"></div><div tabindex="0" role="region" aria-label="Log"></div></div>"#,
        )
    );
}

#[test]
fn fields_and_tooltips_describe_their_controls() {
    let body = plain(&render(json!({ "type": "element", "children": [
        // No `for`: the first control gets an id, inside the input group too.
        { "type": "ui-field", "props": { "label": "Site", "description": "Help" }, "children": [
            { "type": "ui-input-group", "children": [ { "type": "ui-input" } ] } ] },
        // A switch's id and description go on its inner input; one it already has is kept.
        { "type": "ui-field", "props": { "label": "On", "error": "{{ e }}", "for": "s" }, "children": [
            { "type": "ui-switch", "parts": { "control": { "attrs": { "id": "s", "aria-describedby": "note" } } } } ] },
        // The combobox's is its search input.
        { "type": "ui-field", "props": { "label": "City", "description": "Where" }, "children": [
            { "type": "ui-combobox", "props": { "options": ["a"] } } ] },
        { "type": "ui-tooltip", "props": { "text": "Tip", "id": "t" }, "children": [ { "type": "ui-button", "props": { "label": "B" } } ] } ] })));
    let ids = |s: &str| s.split(" aria-describedby=\"").skip(1).map(|r| r[..r.find('"').unwrap()].to_string()).collect::<Vec<_>>();
    assert_eq!(
        ids(&body),
        ["field-root-children-0-control-description", "note s-error", "field-root-children-2-control-description", "t"],
        "{body}"
    );
    for id in ["field-root-children-0-control", "s", "field-root-children-2-control"] {
        assert!(body.contains(&format!(r#"for="{id}""#)), "label for {id}: {body}");
        assert!(body.contains(&format!(r#"<input id="{id}""#)) || body.contains(&format!(r#" id="{id}""#)), "{id}: {body}");
    }
    assert!(body.contains(r#"<p id="field-root-children-0-control-description">Help</p>"#), "{body}");
    assert!(body.contains(r#"<p aria-live="polite" id="s-error" "#) || body.contains(r#"id="s-error""#), "{body}");
    assert!(body.contains(r#"role="tooltip" id="t">Tip</span>"#), "{body}");
}

#[test]
fn tooltips_in_browser_rebuilt_lists_describe_with_live_ids() {
    let page = render_with(
        json!({ "type": "list", "props": { "items": "{{ rows }}", "as": "r" }, "children": [
            { "type": "ui-tooltip", "props": { "text": "{{ r }}" }, "children": [ { "type": "ui-button", "props": { "label": "x" } } ] } ] }),
        json!({ "rows": ["a"] }),
        json!({}),
    )
    .unwrap();
    // Each rebuilt item's button points at that item's tip, not at one shared id.
    assert!(page.body.contains("aria-describedby"), "{}", page.body);
    assert!(!page.body.contains(r#"aria-describedby="tooltip-root-children-0""#), "{}", page.body);
}

#[test]
fn every_page_applies_the_stored_theme_once() {
    let page = render_with(json!({ "type": "ui-badge", "props": { "text": "x" } }), json!({}), json!({})).unwrap();
    assert_eq!(page.early_scripts.len(), 1);
    assert!(page.early_scripts[0].contains("irender-theme"));
    // A toggle on the page doesn't add it twice.
    let page = render_with(json!({ "type": "ui-theme-toggle" }), json!({}), json!({})).unwrap();
    assert_eq!(page.early_scripts.len(), 1);
}

#[test]
fn spinner_and_combobox_share_one_keyframes_rule() {
    let page = render_with(
        json!({ "type": "element", "children": [ { "type": "ui-spinner" }, { "type": "ui-combobox", "props": { "options": ["a"] } } ] }),
        json!({}),
        json!({}),
    )
    .unwrap();
    assert_eq!(page.css.matches("@keyframes ui-spin").count(), 1, "{}", page.css);
    assert!(page.css.contains("animation: ui-spin .7s linear infinite;"), "{}", page.css);
}

// ---------------------------------------------------------------- combobox

fn config(body: &str) -> Json {
    let start = body.find("data-i-xc=\"").unwrap() + 11;
    let raw = &body[start..start + body[start..].find('"').unwrap()];
    serde_json::from_str(&raw.replace("&quot;", "\"").replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">")).unwrap()
}

#[test]
fn combobox_renders_its_value_and_configures_the_behaviour() {
    let page = render_with(
        json!({ "type": "ui-combobox", "props": { "bind": "picked", "multiple": true, "clearable": true, "fixed": ["a"],
            "name": "pick", "id": "c", "label": "Letters",
            "options": ["a", { "value": "b", "label": "Bee", "description": "buzz" }, { "label": "More", "options": [{ "value": "c", "disabled": true }] }] } }),
        json!({ "picked": ["a", "b"] }),
        json!({}),
    )
    .unwrap();
    let body = plain(&page.body);
    // The value shows before any script runs: a fixed chip, a removable one, hidden inputs.
    assert!(body.contains(r#"<span data-ui-chip data-fixed><span>a</span></span>"#), "{body}");
    assert!(body.contains(r#"<span data-ui-chip><span>Bee</span><button type="button" tabindex="-1" data-ui-remove aria-label="Remove Bee">✕</button></span>"#), "{body}");
    assert!(body.contains(r#"<input type="hidden" name="pick" value="a"><input type="hidden" name="pick" value="b">"#), "{body}");
    assert!(body.contains(r#"role="combobox" aria-expanded="false" aria-controls="c-listbox" aria-autocomplete="list""#), "{body}");
    assert!(body.contains(r#"popover="manual" role="listbox" id="c-listbox" aria-multiselectable="true""#), "{body}");
    assert!(!body.contains("data-ui-clear hidden"), "clear shows: a removable value is chosen\n{body}");
    let cfg = config(&page.body);
    assert_eq!(cfg["key"], "picked");
    assert_eq!(cfg["hideSelected"], true);
    assert_eq!(cfg["closeOnSelect"], false);
    assert_eq!(cfg["options"][1], json!({ "value": "b", "label": "Bee", "description": "buzz" }));
    assert_eq!(cfg["options"][2], json!({ "group": "More", "options": [{ "value": "c", "label": "c", "disabled": true }] }));
    // Its script (and the anchor positioning fallback) and spinner keyframes ship once.
    assert_eq!(page.scripts.len(), 2);
    assert!(page.scripts.iter().any(|s| s.contains(r#"["ui-combobox"]"#)));
    assert!(page.scripts.iter().any(|s| s.contains("__irenderAnchors")));
    assert_eq!(page.css.matches("@keyframes ui-spin").count(), 1);
}

#[test]
fn combobox_options_from_state_are_a_path() {
    let page = render_with(
        json!({ "type": "ui-combobox", "props": { "bind": "city", "options": "{{ cities }}" } }),
        json!({ "city": "Pune", "cities": [{ "value": "Pune", "label": "Pune, MH" }] }),
        json!({}),
    )
    .unwrap();
    assert_eq!(config(&page.body)["optionsPath"], json!(["cities"]));
    assert!(page.body.contains(">Pune, MH</span>"), "the label comes from the current options\n{}", page.body);
    let err = render_with(json!({ "type": "ui-combobox", "props": { "options": "{{ a ? b : c }}" } }), json!({ "a": 1 }), json!({}))
        .unwrap_err();
    assert!(err.message.contains("must be a plain path"), "{err}");
}

#[test]
fn combobox_rejects_unsafe_search_urls() {
    for url in ["javascript:alert(1)", "//evil.example/x", "data:application/json,[]"] {
        let err = render_with(json!({ "type": "ui-combobox", "props": { "searchUrl": url } }), json!({}), json!({})).unwrap_err();
        assert!(err.message.contains("`searchUrl` must be a path or an http(s) URL"), "{url}: {err}");
    }
    let ok = render_with(json!({ "type": "ui-combobox", "props": { "searchUrl": "/api/s?kind={{ k }}" } }), json!({}), json!({ "k": "x" }));
    assert_eq!(config(&ok.unwrap().body)["searchUrl"], "/api/s?kind=x");
}

#[test]
fn examples_index_renders_with_a_jump_to_combobox() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples");
    let src = std::fs::read_to_string(format!("{dir}/index.yaml")).unwrap();
    let data: Json = serde_json::from_str(&std::fs::read_to_string(format!("{dir}/index.data.json")).unwrap()).unwrap();
    let engine = engine();
    let mut doc = irender_core::parse(&src, Format::Yaml).unwrap_or_else(|d| panic!("{d}"));
    irender_ui_kit::apply_theme(&mut doc);
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    assert!(ir.warnings.is_empty(), "{}", ir.warnings);
    let page = engine.render(&ir, &data).unwrap();
    let examples = data["examples"].as_array().unwrap();
    assert_eq!(page.body.matches("<article").count(), examples.len());
    // The theme is picked in the bar across the top (site-nav.yaml), not on the page.
    assert!(!page.body.contains("data-i-x=\"ui-theme-toggle\""), "the index leaves the theme to the bar");
    // The combobox lists every example by title, with its page as the value.
    let combobox = page.body.find("data-i-x=\"ui-combobox\"").unwrap();
    let options = &config(&page.body[combobox..])["options"];
    assert_eq!(options.as_array().unwrap().len(), examples.len());
    assert_eq!(options[0]["value"], examples[0]["href"]);
    assert_eq!(options[0]["label"], examples[0]["title"]);
    assert_eq!(options[0]["description"], examples[0]["summary"]);
}

/// The bar across the example pages: a Theme panel next to GitHub on pages with the
/// UI kit's theme, and none on the others.
#[test]
fn examples_bar_has_the_theme_where_pages_can_use_it() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples");
    let src = std::fs::read_to_string(format!("{dir}/site-nav.yaml")).unwrap();
    let engine = engine();
    let mut doc = irender_core::parse(&src, Format::Yaml).unwrap_or_else(|d| panic!("{d}"));
    irender_ui_kit::apply_theme(&mut doc);
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    assert!(ir.warnings.is_empty(), "{}", ir.warnings);
    let bar = |theme: bool| {
        let data = json!({ "root": "", "current": "index.html", "main": [{ "href": "ui-kit.html", "label": "UI kit" }], "more": [], "sticky": false, "theme": theme });
        engine.render(&ir, &data).unwrap().body
    };
    let with = bar(true);
    let panel = with.find("data-i-x=\"ui-theme-toggle\"").expect("a theme panel");
    assert!(panel < with.find("https://github.com/kaushik-xs/iRender").unwrap(), "the Theme button comes before GitHub");
    assert!(!bar(false).contains("data-i-x=\"ui-theme-toggle\""), "no theme panel without the theme");
}

#[test]
fn command_palette() {
    let body = render(json!({ "type": "ui-command", "props": { "id": "cmd", "placeholder": "Search…" }, "children": [
        { "type": "ui-command-group", "props": { "heading": "Settings" }, "children": [
            { "type": "ui-command-item", "props": { "label": "Profile", "shortcut": "⌘P", "keywords": ["account", "me"] } },
            { "type": "ui-command-item", "props": { "label": "Billing", "href": "/billing" } } ] },
        { "type": "ui-command-separator" } ] }));
    // The search box points at the list, which items join as options…
    assert!(body.contains(r#"role="combobox" aria-expanded="true" aria-controls="cmd-list""#), "{body}");
    assert!(body.contains(r#"role="listbox" id="cmd-list""#), "{body}");
    assert!(body.contains(r#"placeholder="Search…""#) && !body.contains("autofocus"), "{body}");
    assert!(body.contains(r#"<button role="option" tabindex="-1" aria-selected="false" data-ui-keywords="account me" type="button">"#), "{body}");
    assert!(body.contains(r#"<a role="option" tabindex="-1" aria-selected="false" href="/billing">"#), "{body}");
    // …under a heading that names the group.
    assert!(body.contains(r#"<div role="group" aria-labelledby="command-group-"#), "{body}");
    assert!(body.contains(r#"data-ui-empty hidden>No results found.</div>"#), "{body}");

    // The dialog opens from its trigger and focuses the search box.
    let body = render(json!({ "type": "ui-command-dialog", "props": { "trigger": "Open", "shortcut": "mod+k", "id": "dlg" },
        "children": [{ "type": "ui-command-item", "props": { "label": "New" } }] }));
    assert!(body.contains(r#"command="show-modal" commandfor="dlg""#), "{body}");
    assert!(body.contains(r#"<dialog"#) && body.contains(r#"aria-label="Command palette""#) && body.contains("autofocus"), "{body}");

    let err = render_with(json!({ "type": "ui-command-item", "props": { "label": "Lost" } }), json!({}), json!({})).unwrap_err();
    assert!(err.message.contains("must be inside"), "{err}");
}

#[test]
fn chat_messages_threads_and_input() {
    let page = render_with(
        json!({ "type": "ui-conversation", "props": { "messages": "{{ messages }}", "typing": "{{ waiting }}", "empty": "Ask anything" },
            "children": [{ "type": "ui-message", "props": { "role": "system", "content": "Today" } }] }),
        json!({ "waiting": false, "messages": [
            { "id": 1, "role": "user", "content": "Hi", "initials": "KS" },
            { "id": 2, "content": "Hello", "name": "Assistant", "time": "9:41",
              "replies": [{ "role": "user", "content": "One" }, { "role": "user", "content": "Two" }] }] }),
        json!({}),
    )
    .unwrap();
    let body = plain(&page.body);
    // A log the browser rebuilds from `messages`, keyed by id, after the children.
    assert!(body.contains(r#"role="log" aria-label="Conversation" tabindex="0""#), "{body}");
    assert!(body.contains(r#"<div data-role="system"><div data-body=""><div data-bubble="">Today</div>"#), "{body}");
    assert!(body.contains("data-i-list=") && body.contains("ui_message"), "{body}");
    // Each message now: its role, avatar, name and time, and a folded thread.
    assert!(body.contains(r#"<div data-role="user"><span aria-hidden="true">KS</span>"#), "{body}");
    assert!(body.contains(r#"<div data-role="assistant"><div data-body=""><div><span>Assistant</span><span>9:41</span></div><div data-bubble="">Hello</div><div data-ui="" data-i-x="ui-message-html" data-i-xc="{&quot;browser&quot;:null,&quot;url&quot;:null}"></div><details><summary>2 replies</summary>"#), "{body}");
    // The empty text and the typing indicator follow state.
    assert!(body.contains(r#"<template data-i-if="{&quot;r&quot;:{&quot;u&quot;:&quot;!&quot;,&quot;a&quot;:{&quot;p&quot;:[&quot;messages&quot;,&quot;length&quot;]}}}"><div>Ask anything</div>"#), "{body}");
    assert!(body.contains(r#"role="status" aria-label="Typing""#), "{body}");

    let body = plain(&render(json!({ "type": "ui-message", "props": { "content": "x", "replies": [{ "content": "y" }], "open": true } })));
    assert!(body.contains("<details open><summary>1 reply</summary>"), "{body}");

    let page = render_with(
        json!({ "type": "ui-chat-input", "props": { "messages": "messages", "bind": "draft", "endpoint": "/chat", "suggestions": ["Hi"] } }),
        json!({ "messages": [], "draft": "so far" }),
        json!({}),
    )
    .unwrap();
    let body = plain(&page.body);
    assert!(body.contains(r#"data-i-xc="{&quot;messages&quot;:&quot;messages&quot;,&quot;endpoint&quot;:&quot;/chat&quot;,&quot;body&quot;:null,&quot;headers&quot;:null,&quot;files&quot;:null,&quot;bind&quot;:&quot;draft&quot;,&quot;waiting&quot;:null,&quot;threads&quot;:null,&quot;thread&quot;:null}""#), "{body}");
    assert!(body.contains(r#"<button type="button" data-suggestion="">Hi</button>"#), "{body}");
    assert!(body.contains(r#"aria-label="Message" data-input="" data-i-bind="draft">so far</textarea>"#), "{body}");
    assert!(body.contains(r#"<button type="submit" aria-label="Send" data-send-button="">"#), "{body}");

    // Messages from state must be a path, so the empty text can count them.
    let err = render_with(json!({ "type": "ui-conversation", "props": { "messages": "{{ a || b }}", "empty": "-" } }), json!({ "a": [], "b": [] }), json!({}))
        .unwrap_err();
    assert!(err.message.contains("must be a path"), "{err}");
}

#[test]
fn message_ui_renders_into_html_and_shows_only_when_trusted() {
    let engine = engine();
    let mut doc = irender_core::parse(
        &json!({ "version": 1, "state": { "threads": [{ "id": "a", "messages": [
            { "role": "assistant", "content": "Here:", "ui": { "state": { "n": 1 }, "root": { "type": "ui-button", "props": { "label": "N {{ n }}" } } } },
            { "role": "assistant", "ui": { "type": "ui-badge", "props": { "text": "Node" } } },
            { "role": "user", "content": "kept", "ui": { "type": "nope" }, "html": "<b>as is</b>" },
            { "ui": { "type": "ui-badge", "props": { "text": "no role, not a message" } } }] }] },
            "root": { "type": "text", "props": { "value": "x" } } }).to_string(),
        Format::Json,
    )
    .unwrap();
    let embed = |ui: &Json| irender_ui_kit::render_ui(&engine, ui, irender_ui_kit::apply_theme);
    irender_ui_kit::render_message_ui(&mut doc, embed).unwrap();
    let messages = &doc.state["threads"][0]["messages"];
    // A document keeps its own state; a node becomes a document's root.
    let html = messages[0]["html"].as_str().unwrap();
    assert!(html.contains(r#"<div data-i-root style="display:contents">"#) && html.contains(r#"data-i-state>{"n":1}</script>"#), "{html}");
    assert!(html.contains("N 1</button>"), "{html}");
    // Styles without the theme's variables, so it takes the page's.
    assert!(html.starts_with("<style>") && !html.contains(":root"), "{html}");
    assert!(messages[1]["html"].as_str().unwrap().contains(">Node</span>"));
    assert_eq!(messages[2]["html"], "<b>as is</b>");
    assert!(messages[3].get("html").is_none());

    // A bad `ui` names its message.
    let mut bad = irender_core::parse(r#"{ "version": 1, "state": { "chat": [{ "role": "assistant", "ui": { "type": "nope" } }] }, "root": { "type": "text" } }"#, Format::Json).unwrap();
    let err = irender_ui_kit::render_message_ui(&mut bad, embed).unwrap_err();
    assert!(err.starts_with("state.chat[0].ui: ") && err.contains("unknown component type `nope`"), "{err}");

    // The message shows it in trusted documents only: markup can do what a script can.
    let message = json!({ "type": "ui-message", "props": { "content": "x", "html": "<b>hi</b>" } });
    let body = plain(&render(message.clone()));
    assert!(body.contains(r#"<div data-ui="" data-html="&lt;b&gt;hi&lt;/b&gt;" data-i-x="ui-message-html""#), "{body}");

    // A `ui` without `html` goes to the page, for `render` to render there.
    let body = plain(&render(json!({ "type": "ui-conversation", "props": {
        "messages": [{ "role": "assistant", "ui": { "type": "ui-badge", "props": { "text": "B" } } }],
        "render": { "browser": "/irender.wasm", "url": "/api/render" } } })));
    assert!(body.contains(r#"data-ui-json="{&quot;type&quot;:&quot;ui-badge&quot;,&quot;props&quot;:{&quot;text&quot;:&quot;B&quot;}}" data-i-x="ui-message-html" data-i-xc="{&quot;browser&quot;:&quot;/irender.wasm&quot;,&quot;url&quot;:&quot;/api/render&quot;}""#), "{body}");
    let mut untrusted = Engine::untrusted();
    irender_ui_kit::register(&mut untrusted.registry);
    let doc = irender_core::parse(&json!({ "version": 1, "root": message }).to_string(), Format::Json).unwrap();
    let body = untrusted.render(&untrusted.resolve(&doc).unwrap(), &json!({})).unwrap().body;
    assert!(!body.contains("data-html") && body.contains(">x</div>"), "{body}");
}

#[test]
fn carousel_structure() {
    let body = plain(&render(json!({ "type": "ui-carousel", "props": { "label": "Photos", "size": 240, "gap": 8 },
        "children": [{ "type": "text", "props": { "value": "A" } }] })));
    assert!(body.contains(r#"<section aria-roledescription="carousel" aria-label="Photos" data-i-x="ui-carousel""#), "{body}");
    assert!(body.contains(r#"<div tabindex="0" data-track=""><span>A</span></div>"#), "{body}");
    assert!(body.contains(r#"aria-label="Previous slide" data-prev="">‹</button>"#) && body.contains(r#"aria-label="Next slide" data-next="">›</button>"#), "{body}");
    let page = render_with(json!({ "type": "ui-carousel", "props": { "size": 240 } }), json!({}), json!({})).unwrap();
    assert!(page.css.contains("--ui-slide: 240px;"), "{}", page.css);
}

#[test]
fn data_table_renders_rows_sorted_and_paged_on_the_server() {
    let rows = json!([
        { "id": 1, "name": "Bolt", "qty": 30, "status": "OK" },
        { "id": 2, "name": "Nut", "qty": 5, "status": "Low" },
        { "id": 3, "name": "Washer", "qty": null, "status": "Out" }
    ]);
    let page = render_with(
        json!({ "type": "ui-data-table", "props": { "rows": "{{ rows }}", "sort": "qty", "pageSize": 2, "selectable": true,
            "bindSelection": "picked", "search": true, "id": "t" },
            "children": [
                { "type": "ui-data-table-column", "props": { "field": "name", "label": "Name", "filter": "text" } },
                { "type": "ui-data-table-column", "props": { "field": "qty", "format": "number", "summary": "sum" } },
                { "type": "ui-data-table-column", "props": { "field": "status", "cell": "badge", "tones": { "Low": "warning" } } },
                { "type": "ui-data-table-column", "props": { "id": "act", "label": "" }, "children": [
                    { "type": "ui-button", "props": { "label": "Open {{ row.name }}" } } ] }
            ] }),
        json!({ "picked": [2] }),
        json!({ "rows": rows }),
    )
    .unwrap();
    let body = plain(&page.body);
    // Ascending by quantity, blanks last, and only the first page shown.
    let names: Vec<&str> = body.match_indices("Open ").map(|(i, _)| &body[i + 5..i + 5 + body[i + 5..].find('<').unwrap()]).collect();
    assert_eq!(names, ["Nut", "Bolt", "Washer"]);
    assert_eq!(body.matches(r#"<tr data-dt-row="#).count(), 3);
    assert_eq!(body.matches(r#"<tr data-dt-row="2" hidden"#).count(), 1, "{body}");
    // The selection from state is checked, the sort marked, the tone chosen on the server.
    assert!(body.contains(r#"aria-label="Select Nut" checked"#), "{body}");
    assert!(body.contains(r#"<th data-dt-col="qty" scope="col" data-align="right" aria-sort="ascending">"#), "{body}");
    assert!(body.contains(r#"<span data-dt-badge="warning">Low</span>"#), "{body}");
    assert!(body.contains("1 of 3 row(s) selected.") && body.contains("Page 1 of 2"), "{body}");
    assert!(body.contains(r#"popovertarget="t-filter-name""#), "{body}");
    assert!(page.scripts.iter().any(|s| s.contains("ui-data-table")));

    // Rows from state: a list the browser rebuilds, with the cells as its template.
    let page = render_with(
        json!({ "type": "ui-data-table", "props": { "rows": "{{ rows }}", "filter": { "status": "{{ only }}" },
            "columns": [{ "field": "name" }] } }),
        json!({ "rows": rows, "only": null }),
        json!({}),
    )
    .unwrap();
    assert!(page.body.contains("data-i-list="), "{}", page.body);
    assert!(page.body.contains("<template><tr"), "{}", page.body);
    assert!(page.body.contains(r#"&quot;filter&quot;:{&quot;status&quot;:{&quot;$irender&quot;"#), "{}", page.body);
}

#[test]
fn data_table_virtual_and_remote_structure() {
    // A virtual table: its rows are a keyed list the behaviour fills, between spacers.
    let rows: Vec<Json> = (0..100).map(|i| json!({ "id": i })).collect();
    let body = render_with(
        json!({ "type": "ui-data-table", "props": { "rows": rows, "virtual": true, "paging": "scroll", "pageSize": 50, "columns": [{ "field": "id" }] } }),
        json!({}),
        json!({}),
    )
    .unwrap()
    .body;
    assert!(body.contains(r#"data-dt-pad="top""#) && body.contains(r#"data-dt-pad="bottom""#), "{body}");
    assert!(body.contains("&quot;k&quot;"), "a keyed list: {body}");
    assert_eq!(body.matches("<tr data-dt-row=").count(), 40, "only the first rows are rendered");
    assert!(body.contains("Loading more…") && body.contains("Showing 50 of 100"), "{body}");
    assert!(!body.contains("data-dt-go"), "no pager: {body}");

    // A remote table starts empty and says it is loading.
    let body = render_with(
        json!({ "type": "ui-data-table", "props": { "url": "/api/rows", "paging": "more", "columns": [{ "field": "id" }] } }),
        json!({}),
        json!({}),
    )
    .unwrap()
    .body;
    assert_eq!(body.matches("<tr data-dt-row=").count(), 0);
    assert!(body.contains(">Load more</button>") && body.contains("Loading…"), "{body}");
    assert!(body.contains("&quot;url&quot;:&quot;/api/rows&quot;"), "{body}");
}

#[test]
fn data_table_tree_and_pins_structure() {
    let body = render_with(
        json!({ "type": "ui-data-table", "props": { "rows": [{ "id": 1, "name": "A", "children": [{ "id": 2, "name": "B" }] }],
            "selectable": true, "columns": [{ "field": "name", "pin": "left", "format": "number" }, { "field": "id", "pin": "right" }] } }),
        json!({}),
        json!({}),
    )
    .unwrap()
    .body;
    assert!(body.contains(r#"role="treegrid""#), "{body}");
    // The first column is the tree column: the arrow, then the value in a span.
    assert!(body.contains(r#"<td data-dt-col="name" data-align="right" data-pin="left" data-dt-tree><div data-dt-node><button type="button" data-dt-twisty"#), "{body}");
    assert!(body.contains(r#"<span data-dt-fmt>A</span>"#), "{body}");
    assert!(body.contains(r#"<td data-dt-col="id" data-pin="right">"#), "{body}");
    // The checkbox column stays with the left-pinned ones.
    assert!(body.contains(r#"<th data-dt-box data-pin="left">"#), "{body}");
    assert!(body.contains("&quot;tree&quot;:{&quot;field&quot;:&quot;children&quot;"), "{body}");

    let err = render_with(
        json!({ "type": "ui-data-table", "props": { "rows": [], "columns": [{ "field": "a", "tree": true }, { "field": "b", "tree": true }] } }),
        json!({}),
        json!({}),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("only one column"), "{err}");
}

#[test]
fn data_table_mistakes() {
    let err = |root: Json| render_with(root, json!({}), json!({})).unwrap_err().to_string();
    let table = |children: Json, props: Json| {
        let mut p = json!({ "rows": [] });
        p.as_object_mut().unwrap().extend(props.as_object().unwrap().clone());
        json!({ "type": "ui-data-table", "props": p, "children": children })
    };
    assert!(err(table(json!([]), json!({}))).contains("needs columns"));
    assert!(err(table(json!([{ "type": "ui-button" }]), json!({}))).contains("not `ui-button`"));
    assert!(err(table(json!([{ "type": "ui-data-table-column", "props": { "filter": "select" } }]), json!({}))).contains("needs a `field`"));
    assert!(err(table(json!([]), json!({ "columns": [{ "field": "a" }, { "field": "a" }] }))).contains("two columns"));
    assert!(err(table(json!([]), json!({ "columns": [{ "field": "a", "cell": "sparkline" }] }))).contains("chart kit"));
    assert!(err(table(json!([]), json!({ "columns": [{ "field": "a", "cell": "badge", "tones": { "x": "pink" } }] }))).contains("tone"));
    assert!(err(json!({ "type": "ui-data-table-column", "props": { "field": "a" } })).contains("direct child"));
    let cols = json!([{ "field": "a" }]);
    assert!(err(json!({ "type": "ui-data-table", "props": { "columns": cols } })).contains("needs `rows`"));
    assert!(err(json!({ "type": "ui-data-table", "props": { "rows": [], "url": "/r", "columns": cols } })).contains("not both"));
    assert!(err(json!({ "type": "ui-data-table", "props": { "url": "javascript:alert(1)", "columns": cols } })).contains("http(s)"));
    assert!(err(json!({ "type": "ui-data-table", "props": { "url": "//evil.example/r", "columns": cols } })).contains("http(s)"));
}

#[test]
fn date_picker_structure_and_mistakes() {
    let page = render_with(
        json!({ "type": "ui-field", "props": { "label": "When" }, "children": [
            { "type": "ui-date-picker", "props": { "bind": "when", "mode": "range", "name": "when", "clearable": true } } ] }),
        json!({ "when": { "from": "2026-03-01", "to": "2026-03-09" } }),
        json!({}),
    )
    .unwrap();
    let body = plain(&page.body);
    // The field's label names the button that opens the calendar.
    assert!(body.contains(r#"<label for="field-root-control">"#), "{body}");
    assert!(body.contains(r#"<button id="field-root-control" type="button" data-cal-trigger popovertarget="date-picker-root-children-0-calendar" aria-haspopup="dialog">"#), "{body}");
    assert!(body.contains(r#"role="dialog""#) && body.contains(r#"aria-label="Choose dates""#), "{body}");
    assert!(body.contains(r#"<input type="hidden" name="when[from]" value="2026-03-01"><input type="hidden" name="when[to]" value="2026-03-09">"#), "{body}");
    assert!(body.contains("2026-03-01 – 2026-03-09"), "{body}");
    assert!(page.scripts.iter().any(|s| s.contains("ui-date-picker")));

    let err = |props: Json| render_with(json!({ "type": "ui-calendar", "props": props }), json!({}), json!({})).unwrap_err().to_string();
    // Times go with days, one or a range: not with several, nor with months.
    assert!(render_with(json!({ "type": "ui-calendar", "props": { "time": true, "mode": "range" } }), json!({}), json!({})).is_ok());
    assert!(err(json!({ "time": true, "mode": "multiple" })).contains("not `multiple`"));
    assert!(err(json!({ "time": true, "picker": "month" })).contains("`picker: day`"));
    assert!(err(json!({ "fiscalStart": 13 })).contains("1–12"));
    assert!(err(json!({ "weekStart": 7 })).contains("0 (Sunday)"));
    assert!(err(json!({ "presets": ["someday"] })).contains("unknown preset"));
    assert!(err(json!({ "presets": [{ "from": "today" }] })).contains("`{ label, from, to }`"));
    // A comparison goes with a range of days or months, not with one date or with times.
    assert!(err(json!({ "compare": true })).contains("`mode: range`"));
    assert!(err(json!({ "compare": true, "mode": "range", "time": true })).contains("without `time`"));
    assert!(err(json!({ "compare": ["nextYear"], "mode": "range" })).contains("`compare` kinds are previous, lastYear, custom"));
    assert!(err(json!({ "compare": "yes", "mode": "range" })).contains("`true` or a list"));
    let lone = render_with(json!({ "type": "ui-calendar", "props": { "bindCompare": "vs", "mode": "range" } }), json!({ "vs": null }), json!({}));
    assert!(lone.unwrap_err().to_string().contains("needs `compare`"));
}

#[test]
fn mindmap_structure_and_mistakes() {
    let page = render_with(
        json!({ "type": "ui-mindmap", "props": { "bind": "map", "id": "mm", "layout": "right", "height": 320 } }),
        json!({ "map": { "id": "c", "label": "Plan", "children": [{ "id": "a", "label": "Goals" }] } }),
        json!({}),
    )
    .unwrap();
    let body = plain(&page.body);
    // A toolbar, and a tree named by the central topic for the behaviour to draw.
    assert!(body.contains(r#"data-mm-bar role="toolbar" aria-label="Mind map tools""#), "{body}");
    assert!(body.contains(r#"data-mm-stage tabindex="0" role="tree" aria-label="Plan""#), "{body}");
    assert!(body.contains(r#"&quot;layout&quot;:&quot;right&quot;"#) && page.css.contains("320px"), "{body}");

    let err = |map: Json| render_with(json!({ "type": "ui-mindmap", "props": { "bind": "k" } }), json!({ "k": map }), json!({})).unwrap_err().to_string();
    assert!(err(json!({ "label": "x" })).contains("needs an `id` and a `label`"));
    assert!(err(json!({ "id": "c" })).contains("topic `c` needs a `label`"));
    assert!(err(json!({ "id": "c", "label": "x", "children": [{ "id": "c", "label": "y" }] })).contains("two topics have the `id` `c`"));
    assert!(err(json!({ "id": "c", "label": "x", "children": "no" })).contains("`children` must be a list"));
}

#[test]
fn whiteboard_structure_and_mistakes() {
    let page = render_with(
        json!({ "type": "ui-whiteboard", "props": { "bind": "board", "id": "wb", "tools": ["select", "pen"], "colors": ["red", "#00f"], "height": 300 } }),
        json!({ "board": [{ "id": "n", "type": "note", "x": 0, "y": 0, "w": 10, "h": 10 }] }),
        json!({}),
    )
    .unwrap();
    let body = plain(&page.body);
    // A toolbar and a focusable board the behaviour draws into.
    assert!(body.contains(r#"id="wb" data-tool="select""#) && body.contains(r#"data-wb-bar role="toolbar" aria-label="Whiteboard tools""#), "{body}");
    assert!(body.contains(r#"data-wb-stage tabindex="0" role="application" aria-roledescription="whiteboard" aria-label="Whiteboard""#), "{body}");
    assert!(body.contains(r#"&quot;tools&quot;:[&quot;select&quot;,&quot;pen&quot;]"#) && body.contains(r#"&quot;colors&quot;:[&quot;red&quot;,&quot;#00f&quot;]"#) && page.css.contains("300px"), "{body}");
    let ro = plain(&render_with(json!({ "type": "ui-whiteboard", "props": { "bind": "k", "readOnly": true } }), json!({ "k": [] }), json!({})).unwrap().body);
    assert!(ro.contains(r#"data-tool="hand" data-readonly"#), "{ro}");

    let err = |props: Json, state: Json| render_with(json!({ "type": "ui-whiteboard", "props": props }), state, json!({})).unwrap_err().to_string();
    assert!(err(json!({ "bind": "k" }), json!({ "k": [{ "id": 1, "type": "star" }] })).contains("item 1: `type` is pen, note"));
    assert!(err(json!({ "bind": "k" }), json!({ "k": [{ "type": "note" }] })).contains("item 1 needs an `id`"));
    assert!(err(json!({ "bind": "k", "tools": ["laser"] }), json!({ "k": [] })).contains("`tools` are select, hand"));
}

#[test]
fn approval_structure_and_mistakes() {
    let page = render_with(
        json!({ "type": "ui-approval", "props": { "bind": "flow", "me": "{{ me }}", "id": "ap" } }),
        json!({ "me": "a", "flow": [{ "title": "Lead", "rule": "any", "approvers": [{ "id": "a", "name": "Asha" }, { "id": "b", "name": "Bo", "decision": "approved" }] }] }),
        json!({}),
    )
    .unwrap();
    let body = plain(&page.body);
    // A named section the behaviour fills, `me` following state.
    assert!(body.contains(r#"id="ap" aria-label="Approval""#) && body.contains("<div data-ap-summary></div><div data-ap-view></div>"), "{body}");
    assert!(body.contains(r#"&quot;me&quot;:{&quot;$irender&quot;"#) && body.contains(r#"&quot;readOnly&quot;:false"#), "{body}");
    assert!(page.scripts.iter().any(|s| s.contains("ui-approval")));

    let err = |flow: Json| render_with(json!({ "type": "ui-approval", "props": { "bind": "k" } }), json!({ "k": flow }), json!({})).unwrap_err().to_string();
    assert!(err(json!("x")).contains("must hold a list of stages"));
    assert!(err(json!([{ "title": "A" }])).contains("stage 1 needs `approvers`"));
    assert!(err(json!([{ "approvers": [{ "id": "a", "name": "A" }], "rule": 3 }])).contains("`rule` is `all`, `any` or a number up to its approvers"));
    assert!(err(json!([{ "approvers": [{ "name": "A" }] }])).contains("needs an `id` and a `name`"));
    assert!(err(json!([{ "approvers": [{ "id": "a", "name": "A", "decision": "ok" }] }])).contains("a decision is approved, changes, rejected"));
}

#[test]
fn file_browser_structure_and_mistakes() {
    let page = render_with(
        json!({ "type": "ui-file-browser", "props": { "bind": "files", "bindFolder": "at", "id": "fm", "height": 300, "view": "list" } }),
        json!({ "files": [{ "id": "d", "name": "Docs", "folder": true }, { "id": "a", "name": "a.pdf", "parent": "d" }], "at": null }),
        json!({}),
    )
    .unwrap();
    let body = plain(&page.body);
    // A named region the behaviour fills: the bar, the files' area, the foot, a live region.
    assert!(body.contains(r#"id="fm" role="region" aria-label="Files""#), "{body}");
    assert!(body.contains("<div data-fm-bar></div>") && body.contains("<div data-fm-foot></div>") && body.contains("data-fm-live"), "{body}");
    assert!(body.contains(r#"&quot;view&quot;:&quot;list&quot;"#) && body.contains(r#"&quot;editable&quot;:true"#) && page.css.contains("300px"), "{body}");
    assert!(page.scripts.iter().any(|s| s.contains("ui-file-browser")) && page.scripts.iter().any(|s| s.contains("__irenderTouchDrag")));

    let err = |state: Json| render_with(json!({ "type": "ui-file-browser", "props": { "bind": "k" } }), state, json!({})).unwrap_err().to_string();
    assert!(err(json!({ "k": "nope" })).contains("must hold a list"));
    assert!(err(json!({ "k": [{ "id": 1 }] })).contains("item 1 needs an `id` and a `name`"));
    assert!(err(json!({ "k": [{ "id": 1, "name": "a" }, { "id": 1, "name": "b" }] })).contains("two items have the `id` `1`"));
}

#[test]
fn virtual_list_structure_and_mistakes() {
    let rows: Vec<Json> = (0..100).map(|i| json!({ "id": format!("r{i}"), "name": format!("Row {i}") })).collect();
    let page = render_with(
        json!({ "type": "ui-virtual-list", "props": { "items": "{{ rows }}", "itemHeight": 30, "label": "Rows" }, "children": [ { "type": "text", "props": { "value": "{{ item.name }}" } } ] }),
        json!({ "rows": rows }),
        json!({}),
    )
    .unwrap();
    let body = plain(&page.body);
    // A focusable scrolling area; a list the behaviour supplies, its first 30 written now.
    assert!(body.contains(r#"tabindex="0" data-vl aria-label="Rows" role="region""#), "{body}");
    assert!(body.contains(r#"<div data-vl-item="r0" role="listitem" aria-posinset="1"><span>Row 0</span></div>"#), "{body}");
    assert!(body.contains(r#"aria-posinset="30"><span>Row 29</span>"#) && !body.contains("Row 30<"), "{body}");
    // The rest stand in as space below: 70 rows of 30px.
    assert!(body.contains("<p data-vl-empty hidden>Nothing to show</p>") && page.css.contains("2100px"), "{body}");

    let err = |root: Json| render_with(root, json!({ "rows": [] }), json!({})).unwrap_err().to_string();
    assert!(err(json!({ "type": "ui-virtual-list", "props": { "items": "{{ rows }}" } })).contains("children are each item"));
    assert!(err(json!({ "type": "ui-virtual-list", "props": { "items": "{{ rows }}", "itemHeight": 1 }, "children": [ { "type": "text", "props": { "value": "x" } } ] })).contains("`itemHeight` is px"));
}

#[test]
fn presence_structure_and_mistakes() {
    let page = render_with(
        json!({ "type": "ui-presence", "props": { "people": "{{ team }}", "max": 1, "id": "p" } }),
        json!({ "team": [{ "id": "a", "name": "Asha Rao", "status": "active" }, { "id": "b", "name": "Bo", "avatar": "/bo.png" }] }),
        json!({}),
    )
    .unwrap();
    let body = plain(&page.body);
    // One button for the stack, named with the count; the list in a popover; no script.
    assert!(body.contains(r#"id="p" data-pr-stack popovertarget="p-list" aria-haspopup="dialog" aria-label="People here: 2""#), "{body}");
    assert!(body.contains(r#"data-status="active" data-color="1" title="Asha Rao">"#) && body.contains(r#"<img src="/bo.png" alt="">"#), "{body}");
    assert!(body.contains(r#"<span data-pr-more aria-hidden="true""#) && body.contains(r#"">+1</span>"#), "{body}");
    assert!(body.contains(r#"popover="auto" id="p-list" role="dialog" data-pr-pop"#) && body.contains("Active"), "{body}");
    assert!(!page.scripts.iter().any(|s| s.contains("ui-presence")), "no script without `announce`");

    let err = |props: Json, state: Json| render_with(json!({ "type": "ui-presence", "props": props }), state, json!({})).unwrap_err().to_string();
    assert!(err(json!({ "people": [{ "id": 1 }] }), json!({})).contains("person 1 needs a `name`"));
    assert!(err(json!({ "people": [], "max": 0 }), json!({})).contains("`max` is how many"));
    assert!(err(json!({ "people": [], "size": 100 }), json!({})).contains("`size` is from 16 to 64"));
}

#[test]
fn compare_slider_structure_and_mistakes() {
    let page = render_with(
        json!({ "type": "ui-compare-slider", "props": { "bind": "at", "before": "/a.jpg", "after": "/b.jpg", "beforeAlt": "Old", "afterAlt": "New", "ratio": "16 / 9", "caption": "Kitchen" } }),
        json!({ "at": 30 }),
        json!({}),
    )
    .unwrap();
    let body = plain(&page.body);
    // A figure: both pictures described, the labels, the divider as a range input.
    assert!(body.contains(r#"role="group" aria-label="Before and after""#), "{body}");
    assert!(body.contains(r#"<img src="/a.jpg" alt="Old" draggable="false" decoding="async" data-cmp-before>"#), "{body}");
    assert!(body.contains(r#"<span data-cmp-tag="after" aria-hidden="true">After</span>"#), "{body}");
    assert!(body.contains(r#"<input type="range" min="0" max="100" step="1" value="30" aria-label="Divider" aria-orientation="horizontal">"#), "{body}");
    assert!(body.contains("<figcaption>Kitchen</figcaption>") && body.contains("data-ratio"), "{body}");

    let err = |props: Json| render_with(json!({ "type": "ui-compare-slider", "props": props }), json!({}), json!({})).unwrap_err().to_string();
    assert!(err(json!({ "before": "a", "after": "b", "position": 120 })).contains("from 0 to 100"));
    assert!(err(json!({ "before": "a", "after": "b", "ratio": "wide" })).contains("`ratio` is width to height"));
}

#[test]
fn countdown_structure_and_mistakes() {
    let page = render_with(
        json!({ "type": "ui-countdown", "props": { "to": "{{ due }}", "from": "{{ opened }}", "warn": "1h", "danger": 900, "label": "Reply due" } }),
        json!({ "due": "2026-12-01T17:00:00Z", "opened": "2026-12-01T09:00:00Z" }),
        json!({}),
    )
    .unwrap();
    let body = plain(&page.body);
    // A timer named by its label, the deadline in a `<time>` until the behaviour counts.
    assert!(body.contains(r#"data-status="ok" role="timer" aria-label="Reply due""#), "{body}");
    assert!(body.contains(r#"<time data-cd-text datetime="2026-12-01T17:00:00Z">2026-12-01T17:00:00Z</time>"#), "{body}");
    assert!(body.contains("data-cd-bar") && body.contains(r#"&quot;warn&quot;:3600.0"#) && body.contains(r#"&quot;danger&quot;:900.0"#), "{body}");
    let tiles = plain(&render_with(json!({ "type": "ui-countdown", "props": { "to": "+1d", "variant": "tiles" } }), json!({}), json!({})).unwrap().body);
    assert_eq!(tiles.matches("data-cd-tile=").count(), 4, "{tiles}");

    let err = |props: Json| render_with(json!({ "type": "ui-countdown", "props": props }), json!({}), json!({})).unwrap_err().to_string();
    assert!(err(json!({ "to": "+1h", "warn": "soon" })).contains("`warn` is seconds"));
    assert!(err(json!({ "to": "+1h", "danger": "2w" })).contains("`danger` is seconds"));
    assert!(err(json!({ "to": "+1h", "warn": "10m", "danger": "1h" })).contains("`danger` comes after `warn`"));
}

#[test]
fn iframe_structure_and_mistakes() {
    let body = plain(&render_with(
        json!({ "type": "element", "children": [
            { "type": "ui-iframe", "props": { "src": "{{ url }}", "title": "Map", "ratio": "16 / 9" } },
            { "type": "ui-iframe", "props": { "src": "https://example.org", "sandbox": ["scripts", "allow-forms"], "allow": ["fullscreen", "camera"], "loading": "eager" } },
            { "type": "ui-iframe", "props": { "src": "https://example.org", "sandbox": false, "allow": "" } }
        ] }),
        json!({ "url": "https://example.com/map" }),
        json!({}),
    )
    .unwrap()
    .body);
    // Sandboxed and lazy by default, its `src` following state.
    assert!(body.contains(r#"<iframe src="https://example.com/map" title="Map" loading="lazy" referrerpolicy="strict-origin-when-cross-origin" sandbox="allow-scripts allow-same-origin allow-forms allow-popups allow-popups-to-escape-sandbox" allow="fullscreen" data-i-b="#), "{body}");
    assert!(body.contains(r#"loading="eager" referrerpolicy="strict-origin-when-cross-origin" sandbox="allow-scripts allow-forms" allow="fullscreen; camera"></iframe>"#), "{body}");
    assert!(body.contains(r#"<iframe src="https://example.org" title="Embedded page" loading="lazy" referrerpolicy="strict-origin-when-cross-origin"></iframe>"#), "{body}");

    let err = |props: Json| render_with(json!({ "type": "ui-iframe", "props": props }), json!({ "list": [] }), json!({})).unwrap_err().to_string();
    assert!(err(json!({ "src": "x", "sandbox": ["top-navigation"] })).contains("`sandbox` can't allow `top-navigation`"));
    assert!(err(json!({ "src": "x", "sandbox": "{{ list }}" })).contains("`sandbox` can't use state"));

    // The untrusted policy refuses frames, the kit's as well as a plain `iframe`.
    let mut untrusted = Engine::untrusted();
    irender_ui_kit::register(&mut untrusted.registry);
    let ir = untrusted.load(&json!({ "version": 1, "root": { "type": "ui-iframe", "props": { "src": "https://example.com" } } }).to_string(), Format::Json).unwrap();
    assert!(untrusted.render(&ir, &json!({})).unwrap_err().to_string().contains("tag `iframe` is not allowed"));
}

#[test]
fn form_builder_structure_and_mistakes() {
    let page = render_with(
        json!({ "type": "ui-form-builder", "props": { "bind": "fields", "id": "fb", "types": ["text", "section"], "endpoint": "/api/answers" } }),
        json!({ "fields": [] }),
        json!({}),
    )
    .unwrap();
    let body = plain(&page.body);
    // Three tabs and their panels, the first showing; the behaviour fills them.
    assert!(body.contains(r#"<button type="button" role="tab" id="fb-tab-build" aria-controls="fb-build" aria-selected="true" tabindex="0" data-fb-tab="build">Build</button>"#), "{body}");
    assert!(body.contains(r#"<div role="tabpanel" id="fb-document" aria-labelledby="fb-tab-document" data-fb-panel="document" hidden></div>"#), "{body}");
    assert!(body.contains(r#"&quot;types&quot;:[&quot;text&quot;,&quot;section&quot;]"#) && body.contains(r#"&quot;submitLabel&quot;:&quot;Submit&quot;"#), "{body}");
    assert!(page.scripts.iter().any(|s| s.contains("ui-form-builder")));

    let err = |props: Json, state: Json| render_with(json!({ "type": "ui-form-builder", "props": props }), state, json!({})).unwrap_err().to_string();
    assert!(err(json!({ "bind": "k", "types": ["slider"] }), json!({ "k": [] })).contains("`types` are text, textarea"));
    assert!(err(json!({ "bind": "k", "endpoint": "javascript:alert(1)" }), json!({ "k": [] })).contains("path on the site or an http(s) URL"));
    assert!(err(json!({ "bind": "k" }), json!({ "k": "nope" })).contains("must hold a list of fields"));
}

#[test]
fn sortable_list_structure_and_mistakes() {
    let page = render_with(
        json!({ "type": "ui-sortable-list", "props": { "bind": "tasks", "numbered": true, "removable": true, "id": "s" } }),
        json!({ "tasks": [{ "id": "a", "label": "Alpha" }, { "id": "b", "label": "Bravo" }] }),
        json!({}),
    )
    .unwrap();
    let body = plain(&page.body);
    // The runtime's keyed list of the state key: a template, then the items as they stand.
    assert!(body.contains(r#"<ol data-sl-items aria-label="List""#) && body.contains("<template>"), "{body}");
    assert!(body.contains(r#"<li data-sl-item="a"><button type="button" data-sl-grip aria-label="Move Alpha" aria-describedby="s-help">"#), "{body}");
    assert!(body.contains(r#"<span data-sl-place aria-hidden="true">2</span><div data-sl-body>Bravo</div><button type="button" data-sl-remove aria-label="Remove Bravo">✕</button>"#), "{body}");
    assert!(body.contains(r#"<p data-sl-empty>Nothing here yet</p>"#), "{body}");
    assert!(page.scripts.iter().any(|s| s.contains("ui-sortable-list")));

    // Plain values are their own keys and names.
    let plain_list = plain(&render_with(json!({ "type": "ui-sortable-list", "props": { "bind": "k" } }), json!({ "k": ["red", "blue"] }), json!({})).unwrap().body);
    assert!(plain_list.contains(r#"<li data-sl-item="blue"><button type="button" data-sl-grip aria-label="Move blue""#), "{plain_list}");

    let err = |state: Json| render_with(json!({ "type": "ui-sortable-list", "props": { "bind": "k" } }), state, json!({})).unwrap_err().to_string();
    assert!(err(json!({ "k": [{ "id": 1 }, { "name": "x" }] })).contains("item 2 has no `id`"));
    assert!(err(json!({ "k": "abc" })).contains("must hold a list"));
}

#[test]
fn gauge_structure_and_mistakes() {
    let page = render_with(
        json!({ "type": "ui-gauge", "props": { "value": "{{ oee }}", "unit": "%", "caption": "OEE", "target": 85,
            "bands": [{ "to": 60, "tone": "danger", "label": "Poor" }, { "to": 85, "tone": "warning", "label": "Fair" }, { "tone": "success", "label": "Good" }] } }),
        json!({ "oee": 72 }),
        json!({}),
    )
    .unwrap();
    let body = plain(&page.body);
    // A meter named by its caption, its band's tone and name worked out on the server.
    assert!(body.contains(r#"data-shape="arc" data-tone="warning" role="meter" aria-valuemin="0" aria-valuemax="100" aria-valuenow="72" aria-valuetext="72 %, Fair" aria-label="OEE""#), "{body}");
    // Three bands, the value's arc, the target; no script.
    assert_eq!(body.matches("data-band").count(), 3, "{body}");
    assert!(body.contains(r#"stroke-dasharray="0 60 25 200""#) && body.contains("data-arc") && body.contains("data-target"), "{body}");
    assert!(!body.contains("data-needle"), "{body}");
    assert!(page.scripts.is_empty() || !page.scripts.iter().any(|s| s.contains("ui-gauge")));

    // A fixed value: its tone and text are worked out once.
    let fixed = plain(&render_with(json!({ "type": "ui-gauge", "props": { "value": 30, "min": 20, "max": 120, "shape": "half", "needle": true, "bands": [{ "to": 80, "tone": "success" }, { "tone": "danger" }] } }), json!({}), json!({})).unwrap().body);
    assert!(fixed.contains(r#"data-shape="half" data-with-needle data-tone="success""#) && fixed.contains(r#"aria-valuetext="30""#) && fixed.contains("data-needle"), "{fixed}");

    let err = |props: Json| render_with(json!({ "type": "ui-gauge", "props": props }), json!({}), json!({})).unwrap_err().to_string();
    assert!(err(json!({ "value": 1, "min": 10, "max": 10 })).contains("`max` must be above `min`"));
    assert!(err(json!({ "value": 1, "size": 20 })).contains("48 to 640"));
    assert!(err(json!({ "value": 1, "bands": [{ "to": 50, "tone": "red" }] })).contains("no tone `red`"));
    assert!(err(json!({ "value": 1, "bands": [{ "tone": "danger" }, { "to": 90, "tone": "success" }] })).contains("only the last band"));
    assert!(err(json!({ "value": 1, "bands": [{ "to": 60, "tone": "danger" }, { "to": 40, "tone": "success" }] })).contains("must be above 60"));
}

#[test]
fn color_picker_structure_and_mistakes() {
    let page = render_with(
        json!({ "type": "ui-field", "props": { "label": "Brand" }, "children": [
            { "type": "ui-color-picker", "props": { "bind": "brand", "name": "brand", "clearable": true, "swatches": ["#1e3a8a", { "value": "#fff", "label": "White" }] } } ] }),
        json!({ "brand": "#1e3a8a" }),
        json!({}),
    )
    .unwrap();
    let body = plain(&page.body);
    // The field's label names the button that opens the panel.
    assert!(body.contains(r#"<label for="field-root-control">"#), "{body}");
    assert!(body.contains(r#"<button id="field-root-control" type="button" data-color-trigger popovertarget="color-picker-root-children-0-panel" aria-haspopup="dialog">"#), "{body}");
    assert!(body.contains(r##"<span data-color-value data-placeholder="Pick a colour">#1e3a8a</span>"##), "{body}");
    assert!(body.contains(r#"role="slider" tabindex="0" aria-label="Saturation and brightness""#), "{body}");
    assert!(body.contains(r#"aria-label="White" title="White" aria-pressed="false""#), "{body}");
    assert!(body.contains(r##"<input type="hidden" name="brand" value="#1e3a8a">"##), "{body}");
    // Colours reach CSS only through the behaviour.
    assert!(!body.contains("--swatch"), "{body}");
    assert!(page.scripts.iter().any(|s| s.contains("ui-color-picker")));

    // Inline, the field's label names its text field.
    let inline = plain(
        &render_with(
            json!({ "type": "ui-field", "props": { "label": "Tint" }, "children": [ { "type": "ui-color-picker", "props": { "bind": "tint", "inline": true, "alpha": true } } ] }),
            json!({ "tint": null }),
            json!({}),
        )
        .unwrap()
        .body,
    );
    assert!(inline.contains(r#"id="field-root-control""#) && inline.contains("data-color-text") && inline.contains(r#"aria-label="Opacity""#), "{inline}");
    assert!(!inline.contains("data-color-pop") && !inline.contains(r#"aria-label="Colour value""#), "{inline}");

    let err = |props: Json| render_with(json!({ "type": "ui-color-picker", "props": props }), json!({}), json!({})).unwrap_err().to_string();
    assert!(err(json!({ "custom": false })).contains("needs `swatches`"));
    assert!(err(json!({ "swatches": [1] })).contains("each swatch"));
    assert!(err(json!({ "swatches": "red" })).contains("must be a list"));
}

#[test]
fn date_picker_compare_structure() {
    let page = render_with(
        json!({ "type": "ui-date-picker", "props": { "bind": "when", "bindCompare": "vs", "compare": ["previous", "custom"], "mode": "range", "name": "when" } }),
        json!({ "when": { "from": "2026-03-01", "to": "2026-03-31" }, "vs": { "type": "previous", "from": "2026-01-29", "to": "2026-02-28" } }),
        json!({}),
    )
    .unwrap();
    let body = plain(&page.body);
    // A place for "vs …" after the value, filled by the behaviour, and the comparison in the form.
    assert!(body.contains(r#"2026-03-01 – 2026-03-31</span><span data-cal-vs hidden></span>"#), "{body}");
    assert!(body.contains(r#"<input type="hidden" name="when[compareFrom]" value="2026-01-29"><input type="hidden" name="when[compareTo]" value="2026-02-28">"#), "{body}");
    assert!(body.contains(r#""compare":["previous","custom"]"#) || body.contains(r#"&quot;compare&quot;:[&quot;previous&quot;,&quot;custom&quot;]"#), "{body}");
}

#[test]
fn context_menu_structure() {
    let page = render_with(
        json!({ "type": "ui-context-menu", "props": { "bind": "ctx", "id": "m" }, "children": [
            { "type": "element", "props": { "tag": "p" }, "children": [{ "type": "text", "props": { "value": "Area" } }] },
            { "type": "ui-menu-item", "props": { "label": "Open" } },
            { "type": "ui-menu-separator" } ] }),
        json!({ "ctx": null }),
        json!({}),
    )
    .unwrap();
    let body = plain(&page.body);
    // The area first, then the menu holding the menu's children, closing itself on a choice.
    assert!(body.contains(r#"<p><span>Area</span></p><div popover="auto" id="m" role="menu" aria-label="Actions""#), "{body}");
    assert!(body.contains(r#"role="menuitem" tabindex="-1" type="button" popovertarget="m" popovertargetaction="hide""#), "{body}");
    assert!(page.scripts.iter().any(|s| s.contains("ui-context-menu")) && page.scripts.iter().any(|s| s.contains("ui-dropdown-menu")));
}

#[test]
fn date_picker_with_input() {
    let body = plain(
        &render_with(
            json!({ "type": "ui-field", "props": { "label": "Start" }, "children": [
                { "type": "ui-date-picker", "props": { "bind": "start", "input": true, "clearable": true } } ] }),
            json!({ "start": "2026-03-15" }),
            json!({}),
        )
        .unwrap()
        .body,
    );
    // The label names the text field, which holds the value until the browser formats it.
    assert!(body.contains(r#"<label for="field-root-control">"#), "{body}");
    assert!(body.contains(r#"<input id="field-root-control" type="text" data-cal-input value="2026-03-15" autocomplete="off" spellcheck="false">"#), "{body}");
    assert!(body.contains(r#"aria-label="Choose a date""#) && body.contains("data-cal-trigger"), "{body}");
    let err = render_with(json!({ "type": "ui-date-picker", "props": { "input": true, "mode": "multiple" } }), json!({}), json!({})).unwrap_err().to_string();
    assert!(err.contains("not `multiple`"), "{err}");
}

#[test]
fn illustrations() {
    let page = render_with(
        json!({ "type": "element", "children": [
            { "type": "ui-illustration", "props": { "name": "no-data", "size": 160, "primary": "#0ea5e9", "label": "No data" } },
            { "type": "ui-empty", "props": { "illustration": "out-of-stock", "title": "Out of stock" } } ] }),
        json!({}),
        json!({}),
    )
    .unwrap();
    let body = plain(&page.body);
    assert!(body.contains(r#"<svg viewBox="0 0 240 180" width="240" height="180" role="img" aria-label="No data">"#), "{body}");
    assert!(body.contains(r#"<svg viewBox="0 0 240 180" width="240" height="180" aria-hidden="true" focusable="false">"#), "{body}");
    assert!(body.contains("data-f=\"p\""), "{body}");
    // Colours: the theme's unless given, with the size.
    assert!(page.css.contains("--ill-p: #0ea5e9;") && page.css.contains("width: 160px;"), "{}", page.css);
    assert!(page.css.contains("fill: var(--ill-p, var(--color-primary));"), "{}", page.css);
    assert!(page.css.contains("var(--ill-s, var(--color-brand-2, var(--color-chart-2, #f59e0b)))"), "{}", page.css);
}

#[test]
fn input_otp_structure_and_mistakes() {
    let body = plain(
        &render_with(
            json!({ "type": "ui-field", "props": { "label": "Code" }, "children": [
                { "type": "ui-input-otp", "props": { "bind": "code", "length": 4, "groups": [2, 2] } } ] }),
            json!({ "code": "12" }),
            json!({}),
        )
        .unwrap()
        .body,
    );
    assert!(body.contains(r#"<label for="field-root-control">"#), "{body}");
    assert!(body.contains(r#"<input id="field-root-control" type="text" data-otp-input value="12" maxlength="4" autocomplete="one-time-code" inputmode="numeric" autocapitalize="characters" spellcheck="false" pattern="[0-9]{4}">"#), "{body}");
    assert!(body.contains(r#"<div data-otp-slot>1</div><div data-otp-slot>2</div></div><div data-otp-sep aria-hidden="true"></div>"#), "{body}");
    assert_eq!(body.matches("data-otp-slot data-empty").count(), 2, "{body}");
    let err = |props: Json| render_with(json!({ "type": "ui-input-otp", "props": props }), json!({}), json!({})).unwrap_err().to_string();
    assert!(err(json!({ "length": 4, "groups": [3, 3] })).contains("add up"));
    assert!(err(json!({ "length": 40 })).contains("1 to 12"));
}

#[test]
fn filters_structure_and_mistakes() {
    let filter = |props: Json| json!({ "type": "ui-filter", "props": props });
    let root = json!({ "type": "ui-filters", "props": { "bind": "f", "title": "Refine" }, "children": [
        filter(json!({ "field": "status", "label": "Status", "section": "Order" })),
        filter(json!({ "field": "amount", "type": "currency", "operators": ["between", "gt"], "unit": "net" })) ] });
    let body = plain(&render_with(root, json!({ "f": {} }), json!({})).unwrap().body);
    assert!(body.contains(r#"data-fl-open aria-haspopup="dialog" aria-controls="filters-root""#), "{body}");
    assert!(body.contains(r#"<dialog id="filters-root" closedby="any" aria-labelledby="filters-root-title" data-fl-panel>"#), "{body}");
    assert!(body.contains(r#"<h2 id="filters-root-title">Refine</h2>"#), "{body}");
    // Fields as tabs, the first chosen; sections as headings.
    assert!(body.contains(r#"<div data-fl-section role="presentation">Order</div><button type="button" role="tab" id="filters-root-tab-0" aria-controls="filters-root-pane-0" aria-selected="true" tabindex="0" data-fl-tab="0"><span>Status</span>"#), "{body}");
    assert!(body.contains(r#"id="filters-root-tab-1" aria-controls="filters-root-pane-1" aria-selected="false" tabindex="-1" data-fl-tab="1"><span>amount</span>"#), "{body}");
    assert!(body.contains(r#"id="filters-root-pane-1" aria-labelledby="filters-root-tab-1" data-fl-pane="1" hidden>"#), "{body}");
    // Only the operators asked for, in that order; a list gets its value template.
    assert!(body.contains(r#"<option value="between">Between</option><option value="gt">More than</option></select>"#), "{body}");
    assert!(body.contains(r#"<option value="any">Is any of</option><option value="none">Is none of</option>"#), "{body}");
    assert!(body.contains("<template data-fl-choice>"), "{body}");
    // Amounts are text fields grouped as typed, with the currency's symbol before them.
    assert!(body.contains(r#"<span data-fl-field data-fl-field-a><span data-fl-prefix aria-hidden="true">$</span><input type="text" data-fl-a aria-label="amount value" inputmode="decimal" data-fl-num autocomplete="off">"#), "{body}");
    assert!(body.contains(r#"<span data-fl-unit>net</span>"#), "{body}");
    assert!(body.contains(r#"data-fl-apply>Apply</button>"#), "{body}");

    let err = |root: Json| render_with(root, json!({ "f": {} }), json!({})).unwrap_err().to_string();
    let filters = |children: Vec<Json>| json!({ "type": "ui-filters", "props": { "bind": "f" }, "children": children });
    assert!(err(filters(vec![])).contains("needs `ui-filter` children"));
    assert!(err(filters(vec![json!({ "type": "ui-button", "props": { "label": "x" } })])).contains("takes `ui-filter` children, not `ui-button`"));
    assert!(err(filters(vec![filter(json!({ "field": "a" })), filter(json!({ "field": "a" }))])).contains("two filters for `a`"));
    assert!(err(filters(vec![filter(json!({ "field": "d", "type": "date", "operators": ["gt"] }))])).contains("a `date` filter has no operator `gt` (it has between, on, before, after, empty, set)"));
    assert!(err(filters(vec![filter(json!({ "field": "n", "type": "number", "options": [1] }))])).contains("only `list` filters take `options`"));
    assert!(err(filter(json!({ "field": "a" }))).contains("must be a direct child of `ui-filters`"));
}

#[test]
fn tree_draws_the_starting_nodes() {
    let nodes = json!([
        { "id": "a", "label": "Alpha", "children": [ { "id": "b", "name": "Beta <b>", "children": [ { "id": "c", "label": "Gamma" } ] } ] },
        { "id": 2, "label": "Two" } ]);
    let page = render_with(
        json!({ "type": "ui-tree", "props": { "nodes": "{{ nodes }}", "bind": "sel", "checkable": true, "bindChecked": "on", "counts": true, "deletable": true, "movable": true } }),
        json!({ "nodes": nodes, "sel": "c", "on": ["c"] }),
        json!({}),
    )
    .unwrap();
    let body = plain(&page.body);
    // The selected node's parents start open, and it's the one tab stop. Everything
    // under Alpha is checked, so it shows checked.
    assert!(body.contains(r#"<li role="treeitem" data-tree-id="a" aria-level="1" aria-setsize="2" aria-posinset="1" aria-selected="false" tabindex="-1" aria-expanded="true" aria-checked="true">"#), "{body}");
    assert!(body.contains(r#"data-tree-id="2" aria-level="1" aria-setsize="2" aria-posinset="2" aria-selected="false" tabindex="-1" aria-checked="false">"#), "{body}");
    assert!(body.contains(r#"data-tree-id="c" aria-level="3" aria-setsize="1" aria-posinset="1" aria-selected="true" tabindex="0" aria-checked="true">"#), "{body}");
    // `name` works as the label, and text is escaped.
    assert!(body.contains("<span data-tree-label>Beta &lt;b&gt;</span>"), "{body}");
    assert!(body.contains(r#"<span data-tree-count aria-hidden="true">2</span>"#), "{body}");
    assert!(body.contains(r#"aria-label="Delete Two""#), "{body}");
    assert!(body.contains(r#"data-tree-row draggable="true""#), "{body}");
    assert!(body.contains(r#"<ul role="tree" aria-label="Tree">"#), "{body}");
    // With `checkable` and `deletable`, a bar for the checked nodes.
    assert!(body.contains("<span data-tree-checked>1 selected</span>"), "{body}");

    let closed = plain(&render(json!({ "type": "ui-tree", "props": { "nodes": [{ "id": "x", "label": "X", "children": [{ "id": "y", "label": "Y" }] }], "expanded": 0 } })));
    assert!(closed.contains(r#"aria-expanded="false""#) && closed.contains(r#"<ul role="group" hidden>"#), "{closed}");
    let all = plain(&render(json!({ "type": "ui-tree", "props": { "nodes": [{ "id": "x", "label": "X", "children": [{ "id": "y", "label": "Y" }] }], "expanded": true } })));
    assert!(all.contains(r#"aria-expanded="true""#), "{all}");
    let empty = plain(&render(json!({ "type": "ui-tree", "props": { "nodes": [], "emptyText": "No locations" } })));
    assert!(empty.contains(r#"<ul role="tree" aria-label="Tree" hidden></ul><p data-tree-empty>No locations</p>"#), "{empty}");
    let err = render_with(json!({ "type": "ui-tree", "props": { "nodes": "{{ n }}" } }), json!({ "n": 3 }), json!({})).unwrap_err().to_string();
    assert!(err.contains("`nodes` must be a list"), "{err}");
}

#[test]
fn file_upload_structure_and_mistakes() {
    let page = render_with(
        json!({ "type": "ui-file-upload", "props": { "bind": "files", "accept": "image/*,.pdf", "maxSize": "2 MB", "hint": "Up to 2 MB", "view": "list" } }),
        json!({ "files": [] }),
        json!({}),
    )
    .unwrap();
    let body = plain(&page.body);
    // A label around the input, so the zone opens the picker; the hint describes it.
    assert!(body.contains(r#"<label data-up-zone><input type="file" id="upload-root" data-up-input multiple accept="image/*,.pdf" aria-describedby="upload-root-hint">"#), "{body}");
    assert!(body.contains(r#"<span data-up-hint id="upload-root-hint">Up to 2 MB</span>"#), "{body}");
    assert!(body.contains(r#"<ul data-up-files data-view="list" aria-label="Files"></ul>"#), "{body}");
    assert!(body.contains(r#"<dialog data-up-preview closedby="any" aria-labelledby="upload-root-title">"#), "{body}");
    assert!(page.body.contains(r#"&quot;maxSize&quot;:2097152.0"#), "{}", page.body);

    // Read-only: no zone, and an empty note.
    let shown = plain(&render_with(json!({ "type": "ui-file-upload", "props": { "bind": "files", "readOnly": true, "multiple": false } }), json!({ "files": [] }), json!({})).unwrap().body);
    assert!(!shown.contains("data-up-zone") && shown.contains("<p data-up-empty>No files yet.</p>"), "{shown}");

    let err = |props: Json| render_with(json!({ "type": "ui-file-upload", "props": props }), json!({ "files": [] }), json!({})).unwrap_err().to_string();
    assert!(err(json!({ "bind": "files", "maxSize": "lots" })).contains("`maxSize` must be bytes"));
    assert!(err(json!({ "bind": "files", "maxFiles": 0 })).contains("`maxFiles` must be a whole number"));
}

#[test]
fn status_badge_ring_rating_checklist_and_timeline() {
    // Status badges: built-in tones (ignoring case and spacing), `tones`, `labels`.
    let badge = |props: Json| plain(&render(json!({ "type": "ui-status-badge", "props": props })));
    assert!(badge(json!({ "status": "In Review" })).contains(r#"<span data-tone="warning"><span data-dot aria-hidden="true"></span><span>In Review</span></span>"#));
    assert!(badge(json!({ "status": "PAID" })).contains(r#"data-tone="success""#));
    assert!(badge(json!({ "status": "on-hold" })).contains(r#"data-tone="warning""#));
    assert!(badge(json!({ "status": "Shipped", "tones": { "shipped": "info" } })).contains(r#"data-tone="info""#));
    assert!(badge(json!({ "status": "Mystery" })).contains(r#"data-tone="neutral""#));
    assert!(badge(json!({ "status": "in_progress", "labels": { "in_progress": "In progress" }, "dot": false })).contains(r#"<span data-tone="primary"><span>In progress</span></span>"#));

    // The ring: an arc of 100 units, the value in the middle.
    let ring = plain(&render(json!({ "type": "ui-progress-ring", "props": { "value": 62.4, "size": 40, "label": "Done", "tone": "success" } })));
    assert!(ring.contains(r#"<div role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow="62.4" aria-label="Done" data-tone="success">"#), "{ring}");
    assert!(ring.contains(r#"pathLength="100""#) && ring.contains(r#"<span data-ring-text aria-hidden="true">62%</span>"#), "{ring}");

    // A rating to pick: radios, the current one checked; to show: an image with its value.
    let pick = plain(&render_with(json!({ "type": "ui-rating", "props": { "bind": "r", "max": 3, "label": "Rate it" } }), json!({ "r": 2 }), json!({})).unwrap().body);
    assert!(pick.contains(r#"role="radiogroup" aria-label="Rate it""#), "{pick}");
    assert!(pick.contains(r#"<label data-on><input type="radio" name="rating-root" value="2" aria-label="2 stars" checked>"#), "{pick}");
    assert_eq!(pick.matches("<label").count(), 3, "{pick}");
    let shown = plain(&render(json!({ "type": "ui-rating", "props": { "value": 4.25 } })));
    assert!(shown.contains(r#"<span data-ro role="img" aria-label="Rating: 4.3 out of 5">"#) && shown.contains("<span data-score aria-hidden=\"true\">4.3</span>"), "{shown}");

    // The checklist: one item per rule, presets and patterns.
    let list = plain(&render(json!({ "type": "ui-password-checklist", "props": { "value": "", "rules": [{ "length": 12 }, "symbol", { "label": "No spaces", "pattern": "^\\S*$" }] } })));
    assert!(list.contains("<li>At least 12 characters<span data-state>, not met</span></li><li>A symbol, like ! or #"), "{list}");
    assert!(list.contains("<li>No spaces<span"), "{list}");

    // The timeline: initials from the name, a dot for an event, changes and a comment.
    let tl = plain(&render(json!({ "type": "ui-timeline", "children": [
        { "type": "ui-timeline-item", "props": { "who": "Asha Rao", "action": "edited", "when": "Mon", "changes": [{ "field": "Due", "from": "1 Nov", "to": "8 Nov" }], "comment": "Pushed a week" } },
        { "type": "ui-timeline-item", "props": { "action": "Build passed", "tone": "success" } } ] })));
    assert!(tl.contains(r#"<ol aria-label="Activity">"#), "{tl}");
    assert!(tl.contains(r#"<span data-mark aria-hidden="true"><span>AR</span></span>"#), "{tl}");
    assert!(tl.contains(r#"<li><span data-field>Due</span><span data-from>1 Nov</span><span data-arrow aria-label="to">→</span><span data-to>8 Nov</span></li>"#), "{tl}");
    assert!(tl.contains("<blockquote>Pushed a week</blockquote>"), "{tl}");
    assert!(tl.contains(r#"<li data-tl-item data-tone="success"><p data-tl-day role="heading" aria-level="3" hidden></p><span data-mark aria-hidden="true" data-dot>"#), "{tl}");
    // Without the feed's props, no script and no button.
    assert!(!tl.contains("data-i-x") && !tl.contains("data-tl-show"), "{tl}");

    // As a feed: each entry's time for the behaviour, a day heading and New to fill in, and
    // the button for the entries held back.
    let feed = plain(&render(json!({ "type": "ui-timeline", "props": { "group": "day", "relative": true, "since": "2026-11-20T00:00", "limit": 1 }, "children": [
        { "type": "ui-timeline-item", "props": { "who": "Asha Rao", "action": "paid", "when": "2026-11-21T10:00" } } ] })));
    assert!(feed.contains(r#"data-i-x="ui-timeline""#) && feed.contains(r#"<li data-tl-item data-when="2026-11-21T10:00">"#), "{feed}");
    assert!(feed.contains(r#"<time datetime="2026-11-21T10:00">2026-11-21T10:00</time><span data-tl-new hidden></span>"#), "{feed}");
    assert!(feed.contains(r#"<li data-tl-show hidden><button type="button"></button></li></ol>"#), "{feed}");
    let bad = render_with(json!({ "type": "ui-timeline", "props": { "limit": 0 } }), json!({}), json!({})).unwrap_err().to_string();
    assert!(bad.contains("`limit` is a whole number"), "{bad}");

    let err = |root: Json| render_with(root, json!({ "r": 0 }), json!({})).unwrap_err().to_string();
    assert!(err(json!({ "type": "ui-status-badge", "props": { "status": "x", "tones": { "x": "green" } } })).contains("has no tone `green`"));
    assert!(err(json!({ "type": "ui-progress-ring", "props": { "value": 1, "size": 4 } })).contains("`size` must be from 16 to 512"));
    assert!(err(json!({ "type": "ui-rating", "props": { "bind": "r", "max": 12 } })).contains("`max` must be a whole number from 1 to 10"));
    assert!(err(json!({ "type": "ui-rating", "props": { "bind": "r", "value": 3 } })).contains("takes `bind` (to pick one) or `value`"));
    assert!(err(json!({ "type": "ui-password-checklist", "props": { "value": "", "rules": ["long"] } })).contains("each rule is"));
}

#[test]
fn kanban_draws_the_starting_board() {
    let body = plain(&render_with(
        json!({ "type": "ui-kanban", "props": {
            "columns": [ { "id": "todo", "title": "To do", "tone": "info" }, { "id": "doing", "title": "Doing", "limit": 1 }, "done" ],
            "cards": "{{ cards }}", "bindCards": "cards", "addable": true } }),
        json!({ "cards": [
            { "id": 1, "column": "todo", "title": "Write <docs>", "tags": ["Docs", { "label": "Bug", "tone": "danger" }], "priority": "Urgent", "due": "Fri", "assignee": "Asha Rao" },
            { "id": 2, "column": "doing", "title": "Ship" }, { "id": 3, "column": "doing", "title": "Test" } ] }),
        json!({}),
    )
    .unwrap()
    .body);
    assert!(body.contains(r#"role="region" aria-label="Board" aria-roledescription="board""#), "{body}");
    assert!(body.contains(r#"<section data-kb-column="todo" aria-label="To do, 1 card" data-tone="info">"#), "{body}");
    // Over its limit: marked, and the count shows it.
    assert!(body.contains(r#"<section data-kb-column="doing" aria-label="Doing, 2 cards" data-over>"#) && body.contains("<span data-kb-count>2 / 1</span>"), "{body}");
    // Plain names work as columns; an empty one says so.
    assert!(body.contains(r#"<section data-kb-column="done" aria-label="done, 0 cards">"#) && body.contains("<p data-kb-empty>No cards</p>"), "{body}");
    // A card: the one tab stop, its label, tags in tones, priority, due and initials.
    assert!(body.contains(r#"<article data-kb-card="1" tabindex="0" aria-roledescription="card" aria-label="Write &lt;docs&gt;, Urgent priority, due Fri, assigned to Asha Rao" role="listitem" draggable="true">"#), "{body}");
    assert!(body.contains(r#"<span data-tone="neutral">Docs</span><span data-tone="danger">Bug</span>"#), "{body}");
    assert!(body.contains(r#"<span data-kb-priority data-tone="danger">Urgent</span><span data-kb-due>Fri</span><span data-kb-avatar title="Asha Rao">AR</span>"#), "{body}");
    assert_eq!(body.matches("data-kb-add").count(), 3, "{body}");

    let err = render_with(json!({ "type": "ui-kanban", "props": { "columns": [], "cards": [], "addable": true } }), json!({}), json!({})).unwrap_err().to_string();
    assert!(err.contains("`addable` needs `bindCards`"), "{err}");
}

#[test]
fn event_calendar_toolbar_and_mistakes() {
    let body = plain(&render(json!({ "type": "ui-event-calendar", "props": { "events": [], "views": ["week", "list"], "categories": { "A": "info" }, "label": "Shifts" } })));
    assert!(body.contains(r#"role="region" aria-label="Shifts""#), "{body}");
    assert!(body.contains(r#"data-ev-prev aria-label="Previous""#) && body.contains(r#"data-ev-next aria-label="Next""#) && body.contains("data-ev-today>Today</button>"), "{body}");
    // Only the views asked for, the first pressed; the body starts in it.
    assert!(body.contains(r#"<div data-ev-views role="group" aria-label="View"><button type="button" data-ev-view="week" aria-pressed="true">Week</button><button type="button" data-ev-view="list" aria-pressed="false">List</button></div>"#), "{body}");
    assert!(body.contains(r#"<div data-ev-legend aria-label="Categories"></div><div data-ev-body data-view="week"></div>"#), "{body}");
    // One view: no switch.
    let one = plain(&render(json!({ "type": "ui-event-calendar", "props": { "events": [], "views": ["list"] } })));
    assert!(!one.contains("data-ev-views"), "{one}");

    let err = |props: Json| render_with(json!({ "type": "ui-event-calendar", "props": props }), json!({}), json!({})).unwrap_err().to_string();
    assert!(err(json!({ "events": [], "views": ["decade"] })).contains("`views` has `decade`"));
    assert!(err(json!({ "events": [], "views": ["month"], "view": "week" })).contains("which `views` leaves out"));
    assert!(err(json!({ "events": [], "hours": [18, 9] })).contains("`hours` must be `[from, to]`"));
    assert!(err(json!({ "events": [], "weekStart": 3 })).contains("`weekStart` must be 0"));
}

#[test]
fn number_input_structure_and_mistakes() {
    let page = render_with(
        json!({ "type": "ui-field", "props": { "label": "Budget" }, "children": [
            { "type": "ui-number-input", "props": { "bind": "amount", "currency": "inr", "stepper": true, "suffix": "per month", "decimals": 0 } } ] }),
        json!({ "amount": 125000 }),
        json!({}),
    )
    .unwrap();
    let body = plain(&page.body);
    // The field's label points at the inner input; the currency's symbol goes first.
    assert!(body.contains(r#"<label for="field-root-control">"#), "{body}");
    assert!(body.contains(r#"<span data-num-prefix aria-hidden="true">₹</span><input id="field-root-control" type="text" inputmode="numeric" data-num-input autocomplete="off" value="125000" aria-description="in INR">"#), "{body}");
    assert!(body.contains(r#"<span data-num-suffix>per month</span>"#), "{body}");
    assert!(body.contains(r#"data-num-step="-1" tabindex="-1" aria-label="Decrease">−</button>"#), "{body}");
    assert!(page.body.contains("&quot;currency&quot;:&quot;INR&quot;"), "{}", page.body);

    let err = |props: Json| render_with(json!({ "type": "ui-number-input", "props": props }), json!({ "n": null }), json!({})).unwrap_err().to_string();
    assert!(err(json!({ "bind": "n", "min": 5, "max": 1 })).contains("`min` is more than `max`"));
    assert!(err(json!({ "bind": "n", "step": 0 })).contains("`step` must be more than 0"));
    assert!(err(json!({ "bind": "n", "currency": "rupees" })).contains("three-letter code"));
    assert!(err(json!({ "bind": "n", "decimals": 1.5 })).contains("`decimals` must be a whole number"));

    // Filters: a date filter's presets, a mistake in them, and the Views menu.
    let filters = |filter: Json| json!({ "type": "ui-filters", "props": { "bind": "f", "bindViews": "v" }, "children": [filter] });
    let fb = plain(&render_with(filters(json!({ "type": "ui-filter", "props": { "field": "d", "type": "date", "presets": ["last7", "thisYear"] } })), json!({ "f": {}, "v": [] }), json!({})).unwrap().body);
    assert!(fb.contains(r#"<div data-fl-presets role="group" aria-label="d presets"><button type="button" data-fl-preset="last7">Last 7 days</button><button type="button" data-fl-preset="thisYear">This year</button></div>"#), "{fb}");
    assert!(fb.contains(r#"data-fl-views-open aria-expanded="false""#) && fb.contains(r#"data-fl-view-name placeholder="Name these filters""#), "{fb}");
    let e = render_with(filters(json!({ "type": "ui-filter", "props": { "field": "d", "type": "date", "presets": ["fortnight"] } })), json!({ "f": {}, "v": [] }), json!({})).unwrap_err().to_string();
    assert!(e.contains("no date preset `fortnight`"), "{e}");
}

#[test]
fn notifications_bell_and_panel() {
    let body = plain(&render(json!({ "type": "ui-notifications", "props": { "items": [
        { "id": 1, "title": "A" }, { "id": 2, "title": "B", "read": true }, { "id": 3, "title": "C", "archived": true } ], "label": "Alerts" } })));
    // The bell counts what's unread and not archived; it opens the panel as a popover.
    assert!(body.contains(r#"<button type="button" data-nt-bell popovertarget="notifications-root" aria-haspopup="dialog" aria-label="Alerts, 1 unread">"#), "{body}");
    assert!(body.contains(r#"<span data-nt-count aria-hidden="true">1</span>"#), "{body}");
    assert!(body.contains(r#"popover="auto""#) && body.contains(r#"role="dialog" aria-labelledby="notifications-root-title""#), "{body}");
    assert!(body.contains(r#"<button type="button" data-nt-tab="unread" aria-pressed="true">Unread (1)</button><button type="button" data-nt-tab="all" aria-pressed="false">All</button>"#), "{body}");
    // Inline: no bell, a region; nothing unread disables Mark all as read.
    let inline = plain(&render(json!({ "type": "ui-notifications", "props": { "items": [], "inline": true, "tab": "all" } })));
    assert!(!inline.contains("data-nt-bell") && inline.contains(r#"role="region""#) && !inline.contains("popover="), "{inline}");
    assert!(inline.contains(r#"data-nt-mark-all disabled>Mark all as read</button>"#), "{inline}");
    let err = render_with(json!({ "type": "ui-notifications", "props": { "items": [], "tab": "new" } }), json!({}), json!({})).unwrap_err().to_string();
    assert!(err.contains("`tab` must be `unread` or `all`"), "{err}");
}

#[test]
fn rail_layout_structure_and_mistakes() {
    let root = |children: Json| json!({ "type": "ui-rail-layout", "props": { "bind": "s", "bindOpen": "o", "items": [
        { "id": "a", "label": "Alpha", "icon": "★", "badge": 2 }, { "id": "b", "label": "Beta" }, { "id": "c", "label": "Gear", "bottom": true } ] }, "children": children });
    let body = plain(&render_with(root(json!([ { "type": "ui-rail-panel", "props": { "for": "a" }, "children": [ { "type": "text", "props": { "value": "panel A" } } ] }, { "type": "text", "props": { "value": "main" } } ])), json!({ "s": "b", "o": true }), json!({})).unwrap().body);
    // Beta is shown and has no panel: the layout starts closed.
    assert!(body.contains(r#"data-closed"#), "{body}");
    assert!(body.contains(r#"<nav data-rail aria-label="Sections"><div role="tablist" aria-orientation="vertical" data-rail-tabs>"#), "{body}");
    // Tabs whose selection follows state (the expressions ride in `data-i-b`).
    assert!(body.contains(r#"<button data-rail-item="a" data-tip="Alpha" aria-label="Alpha" type="button" role="tab" id="rail-root-tab-a" aria-selected="false" tabindex="-1" aria-controls="rail-root-panel-a" data-i-b="#), "{body}");
    assert!(body.contains(r#"<span aria-hidden="true">★</span><span data-rail-badge>2</span></button>"#), "{body}");
    assert!(body.contains(r#"id="rail-root-tab-b" aria-selected="true" tabindex="0""#) && body.contains(r#"<span aria-hidden="true">B</span>"#), "{body}");
    assert!(body.contains(r#"<div data-rail-foot><button data-rail-item="c""#), "{body}");
    assert!(body.contains(r#"<section id="rail-root-panel-a" role="tabpanel" aria-labelledby="rail-root-tab-a" data-rail-panel="a" hidden"#), "{body}");
    assert!(body.contains(r#"<div data-rail-head><h2>Alpha</h2><button type="button" data-rail-fold aria-label="Hide Alpha""#), "{body}");
    assert!(body.contains("<main data-rail-main><span>main</span></main>"), "{body}");

    let err = |root: Json| render_with(root, json!({ "s": "a", "o": true }), json!({})).unwrap_err().to_string();
    assert!(err(root(json!([ { "type": "ui-rail-panel", "props": { "for": "z" } } ]))).contains("is for `z`, which isn't a rail item"));
    assert!(err(json!({ "type": "ui-rail-layout", "props": { "bind": "s", "items": [ { "id": "a", "label": "A" }, { "id": "a", "label": "B" } ] } })).contains("two rail items have the id `a`"));
    assert!(err(json!({ "type": "ui-rail-layout", "props": { "bind": "s", "items": [ { "id": "a" } ] } })).contains("needs a `label`"));
    assert!(err(json!({ "type": "ui-rail-panel", "props": { "for": "a" } })).contains("must be a direct child of `ui-rail-layout`"));

    // Resizable: the handle, and the width from state.
    let page = render_with(json!({ "type": "ui-rail-layout", "props": { "bind": "s", "resizable": true, "bindWidth": "w", "items": [ { "id": "a", "label": "A" } ] },
        "children": [ { "type": "ui-rail-panel", "props": { "for": "a" } } ] }), json!({ "s": "a", "o": true, "w": 320 }), json!({})).unwrap();
    let body = plain(&page.body);
    assert!(body.contains(r#"<div data-rail-resize role="separator" tabindex="0" aria-orientation="vertical" aria-label="Resize the panel" aria-controls="rail-root-panels" aria-valuemin="180" aria-valuemax="560" aria-valuenow="260"></div>"#), "{body}");
    assert!(body.contains(r#"style="--i-d0: 320px""#), "{body}");
    assert!(err(json!({ "type": "ui-rail-layout", "props": { "bind": "s", "resizable": true, "panelWidth": 100, "items": [ { "id": "a", "label": "A" } ] } })).contains("`minPanelWidth` ≤ `panelWidth` ≤ `maxPanelWidth`"));
}

#[test]
fn resizable_sizes_and_mistakes() {
    let panel = |p: Json| json!({ "type": "ui-resizable-panel", "props": p, "children": [ { "type": "text", "props": { "value": "x" } } ] });
    let group = |props: Json, kids: Vec<Json>| json!({ "type": "ui-resizable", "props": props, "children": kids });
    // Sizes not given share what's left; all of them scale to 100.
    let page = render_with(group(json!({ "handle": true }), vec![panel(json!({ "size": 40, "label": "Files" })), panel(json!({})), panel(json!({}))]), json!({}), json!({})).unwrap();
    let body = plain(&page.body);
    assert!(body.contains(r#"data-direction="horizontal""#), "{body}");
    assert!(page.css.contains("--rz-size: 40;") && page.css.contains("--rz-size: 30;"), "{}", page.css);
    assert!(body.contains(r#"data-rz-handle role="separator" tabindex="0" aria-orientation="vertical" aria-controls="rz-root-0" aria-label="Resize Files" aria-valuemin="10" aria-valuemax="100" aria-valuenow="40"><span data-rz-grip>"#), "{body}");
    assert!(body.contains(r#"aria-label="Resize panels""#), "{body}");
    let page = render_with(group(json!({}), vec![panel(json!({ "size": 60 })), panel(json!({ "size": 60 }))]), json!({}), json!({})).unwrap();
    assert!(page.css.contains("--rz-size: 50;") && !page.css.contains("--rz-size: 60;"), "{}", page.css);
    // Bound: the size follows state.
    let r = render_with(group(json!({ "bind": "s", "direction": "vertical" }), vec![panel(json!({})), panel(json!({}))]), json!({ "s": [25, 75] }), json!({})).unwrap();
    let body = plain(&r.body);
    assert!(body.contains(r#"aria-orientation="horizontal""#), "{body}");
    assert!(r.css.contains("--rz-size: var(--i-d"), "{}", r.css);
    assert!(body.contains(r#"style="--i-d0: 25""#) && body.contains(r#"style="--i-d0: 75""#), "{body}");

    let err = |root: Json| render_with(root, json!({}), json!({})).unwrap_err().to_string();
    assert!(err(group(json!({}), vec![panel(json!({}))])).contains("two or more"));
    assert!(err(group(json!({}), vec![panel(json!({})), json!({ "type": "text", "props": { "value": "x" } })])).contains("takes only `ui-resizable-panel` children"));
    assert!(err(group(json!({ "direction": "diagonal" }), vec![panel(json!({})), panel(json!({}))])).contains("`horizontal` or `vertical`"));
    assert!(err(group(json!({}), vec![panel(json!({ "min": 50, "max": 20 })), panel(json!({}))])).contains("`min` no more than `max`"));
    assert!(err(group(json!({}), vec![panel(json!({ "size": 0 })), panel(json!({ "size": 0 }))])).contains("add up to nothing"));
    assert!(err(panel(json!({}))).contains("must be a direct child of `ui-resizable`"));
}

#[test]
fn code_block_and_json_editor_markup_and_mistakes() {
    // The server draws plain lines: they read without the script.
    let page = render_with(json!({ "type": "ui-code-block", "props": { "code": "a < b\n\nc\n", "language": "js", "title": "x.js", "lineNumbers": true, "highlight": ["2-3"] } }), json!({}), json!({})).unwrap();
    let body = plain(&page.body);
    assert!(body.contains(r#"<figure data-numbered"#), "{body}");
    assert!(body.contains(r#"<figcaption data-code-head><span data-code-title>x.js</span><span data-code-lang>js</span><button type="button" data-code-copy aria-label="Copy code" title="Copy">"#), "{body}");
    assert!(body.contains(r#"<pre tabindex="0" aria-label="x.js"><code data-code-body><span data-ln>a &lt; b</span><span data-ln data-hl></span><span data-ln data-hl>c</span></code></pre><span data-code-live aria-live="polite"></span></figure>"#), "{body}");
    // An object shows as JSON; with no title or language, no bar, and the button alone.
    let body = plain(&render_with(json!({ "type": "ui-code-block", "props": { "code": "{{ v }}" } }), json!({ "v": { "a": [1] } }), json!({})).unwrap().body);
    assert!(body.contains(r#"<figure data-i-x="ui-code-block""#) && body.contains(r#"<button type="button" data-code-copy"#) && !body.contains("figcaption"), "{body}");
    assert!(body.contains(r#"<span data-ln>{</span><span data-ln>  &quot;a&quot;: [</span><span data-ln>    1</span><span data-ln>  ]</span><span data-ln>}</span>"#) || body.contains(r#"<span data-ln>  "a": [</span>"#), "{body}");

    // The editor: state's value, formatted with the indent asked for.
    let body = plain(&render_with(json!({ "type": "ui-json-editor", "props": { "bind": "v", "indent": 4 } }), json!({ "v": { "a": [1] } }), json!({})).unwrap().body);
    assert!(body.contains("{\n    &quot;a&quot;: [\n        1\n    ]\n}</textarea>") || body.contains("{\n    \"a\": [\n        1\n    ]\n}</textarea>"), "{body}");
    assert!(body.contains(r#"aria-describedby="json-root-error json-root-hint" aria-invalid="false" aria-label="JSON""#), "{body}");
    assert!(body.contains(r#"<div data-je-gutter><span>1</span><span>2</span><span>3</span><span>4</span><span>5</span></div>"#), "{body}");
    assert!(body.contains(r#"<p data-je-error id="json-root-error" aria-live="polite"></p>"#), "{body}");
    // In a field, the field names it.
    let body = plain(&render_with(json!({ "type": "ui-field", "props": { "label": "Payload" }, "children": [ { "type": "ui-json-editor", "props": { "bind": "v" } } ] }), json!({ "v": null }), json!({})).unwrap().body);
    assert!(body.contains(r#"<label for="field-root-control">Payload</label>"#) && body.contains(r#"<textarea id="field-root-control""#) && !body.contains(r#"aria-label="JSON""#), "{body}");

    let err = |root: Json| render_with(root, json!({ "v": null }), json!({})).unwrap_err().to_string();
    assert!(err(json!({ "type": "ui-code-block", "props": { "code": "x", "language": "cobol" } })).contains("`cobol` isn't one the highlighter knows"));
    assert!(err(json!({ "type": "ui-code-block", "props": { "code": "x", "highlight": ["4-2"] } })).contains("isn't a line number or a range"));
    assert!(err(json!({ "type": "ui-json-editor", "props": { "bind": "v", "type": "string" } })).contains("`object`, `array` or `any`"));
    assert!(err(json!({ "type": "ui-json-editor", "props": { "bind": "v", "rows": 1 } })).contains("`rows` must be from 2 to 200"));
}

#[test]
fn button_variant_from_state() {
    // A variant that reads state carries every look, picked by `data-variant`.
    let page = render_with(json!({ "type": "ui-button", "props": { "label": "A", "variant": "{{ on ? 'secondary' : 'ghost' }}" } }), json!({ "on": true }), json!({})).unwrap();
    assert!(plain(&page.body).contains(r#"data-variant="secondary""#), "{}", page.body);
    assert!(page.css.contains("[data-variant=ghost]") && page.css.contains("[data-variant=secondary]"), "{}", page.css);
    // A fixed one doesn't.
    let page = render_with(json!({ "type": "ui-button", "props": { "label": "A", "variant": "ghost" } }), json!({}), json!({})).unwrap();
    assert!(!page.body.contains("data-variant") && !page.css.contains("[data-variant="), "{}", page.body);
}

#[test]
fn data_table_bands_resizers_export_and_group_by() {
    let table = |props: Json| {
        let mut p = json!({ "rows": [ { "a": 1, "b": 2, "c": 3 } ], "title": "Q3 sales" });
        p.as_object_mut().unwrap().extend(props.as_object().unwrap().clone());
        json!({ "type": "ui-data-table", "props": p, "children": [
            { "type": "ui-data-table-column", "props": { "field": "a", "label": "A", "groupable": true } },
            { "type": "ui-data-table-column", "props": { "field": "b", "label": "B", "group": "Money" } },
            { "type": "ui-data-table-column", "props": { "field": "c", "label": "C", "group": "Money", "resizable": false } } ] })
    };
    let body = plain(&render_with(table(json!({ "resizable": true, "reorderable": true, "export": true })), json!({}), json!({})).unwrap().body);
    // The band row: an empty band over A, Money over B and C.
    assert!(body.contains(r#"<tr data-dt-bands><th data-dt-band="" colspan="1" aria-hidden="true"></th><th data-dt-band="Money" colspan="2" scope="colgroup">Money</th></tr>"#), "{body}");
    assert!(body.contains(r#"<span data-dt-resizer="a" role="separator" aria-orientation="vertical" tabindex="0" aria-label="Resize A"></span>"#), "{body}");
    assert!(!body.contains(r#"data-dt-resizer="c""#), "a column can opt out: {body}");
    assert!(body.contains(r#"<th data-dt-col="a" scope="col" data-dt-movable>"#), "{body}");
    assert!(body.contains(r#"data-dt-export"#) && body.contains(r#""export":"q3-sales.csv""#.replace('"', "&quot;").as_str()), "{body}");
    // Group by in View: None and the groupable columns.
    assert!(body.contains(r#"<div data-dt-pop-title data-dt-group-title>Group by</div>"#), "{body}");
    assert!(body.contains(r#"<input type="radio" name="data-table-root-group-by" value="" data-dt-group-by>None</label>"#), "{body}");
    assert!(body.contains(r#"value="a" data-dt-group-by>A</label>"#), "{body}");
    // A named file keeps its name, with .csv.
    let body = plain(&render_with(table(json!({ "export": "ledger" })), json!({}), json!({})).unwrap().body);
    assert!(body.contains("ledger.csv"), "{body}");

    // Rows from an API group on the server: the groups come as nested rows.
    let mut remote = table(json!({ "url": "/api/rows", "groupBy": "a" }));
    remote["props"].as_object_mut().unwrap().remove("rows");
    let body = plain(&render_with(remote.clone(), json!({}), json!({})).unwrap().body);
    assert!(body.contains("data-dt-tree") && body.contains("&quot;groupRemote&quot;:true"), "{body}");
    remote["props"]["virtual"] = json!(true);
    let err = render_with(remote, json!({}), json!({})).unwrap_err().to_string();
    assert!(err.contains("grouping rows doesn't work with `virtual` or nested rows"), "{err}");
}

#[test]
fn locale_defaults_to_the_documents_lang() {
    let page = |lang: Option<&str>, locale: Option<&str>| {
        let mut table = json!({ "type": "ui-data-table", "props": { "rows": [ { "a": 1 } ] }, "children": [ { "type": "ui-data-table-column", "props": { "field": "a" } } ] });
        if let Some(l) = locale {
            table["props"]["locale"] = json!(l);
        }
        let mut doc = json!({ "version": 1, "root": table });
        if let Some(l) = lang {
            doc["lang"] = json!(l);
        }
        let doc = irender_core::parse(&doc.to_string(), Format::Json).unwrap();
        let engine = engine();
        engine.render(&engine.resolve(&doc).unwrap(), &json!({})).unwrap().body
    };
    assert!(page(Some("de"), None).contains("&quot;locale&quot;:&quot;de&quot;"), "the document's language");
    assert!(page(Some("de"), Some("en-IN")).contains("&quot;locale&quot;:&quot;en-IN&quot;"), "the component's own wins");
    assert!(page(None, None).contains("&quot;locale&quot;:null"), "without either, the browser's");
}

#[test]
fn right_to_left_languages_set_dir() {
    let page = |lang: &str| {
        let doc = irender_core::parse(&json!({ "version": 1, "lang": lang, "state": { "on": true }, "root": { "type": "ui-switch", "props": { "bind": "on" } } }).to_string(), Format::Json).unwrap();
        let engine = engine();
        engine.render(&engine.resolve(&doc).unwrap(), &json!({})).unwrap()
    };
    for lang in ["ar", "he", "fa", "ur", "ar-EG"] {
        let p = page(lang);
        assert_eq!(p.dir, "rtl", "{lang}");
        assert!(p.document("t").contains(&format!(r#"<html lang="{lang}" dir="rtl">"#)));
        assert!(p.fragment().contains(r#"<div data-i-root dir="rtl""#));
        // The mirroring rules come along with the components that use them.
        assert!(p.css.contains("--ui-dir: -1") && p.css.contains("svg[data-flip]:dir(rtl)"), "{}", p.css);
    }
    let p = page("de");
    assert_eq!(p.dir, "ltr");
    assert!(p.document("t").contains(r#"<html lang="de">"#) && p.fragment().contains("<div data-i-root style"));
}

#[test]
fn dashboard_grid_places_tiles_and_mistakes() {
    let tile = |p: Json| json!({ "type": "ui-tile", "props": p, "children": [ { "type": "text", "props": { "value": "x" } } ] });
    let grid = |props: Json, kids: Vec<Json>| json!({ "type": "ui-dashboard-grid", "props": props, "children": kids });
    let kids = || vec![
        tile(json!({ "id": "a", "title": "Sales", "x": 0, "y": 3, "w": 8, "h": 2 })),
        tile(json!({ "id": "b", "x": 8, "y": 0, "w": 4, "h": 2, "locked": true })),
        tile(json!({ "title": "New" })),
    ];
    let page = render_with(grid(json!({ "columns": 12, "gap": 8 }), kids()), json!({}), json!({})).unwrap();
    let body = plain(&page.body);
    // a floats up to the top; the third takes the first free place, under a.
    assert!(page.css.contains("grid-column: 1 / span 8;\n  grid-row: 1 / span 2;"), "{}", page.css);
    assert!(page.css.contains("grid-column: 9 / span 4;\n  grid-row: 1 / span 2;"), "{}", page.css);
    assert!(page.css.contains("grid-column: 1 / span 4;\n  grid-row: 3 / span 2;"), "{}", page.css);
    assert!(page.css.contains("--dg-gap: 8px;") && page.css.contains("@container (max-width: 639.98px)"), "{}", page.css);
    assert!(body.contains(r#"data-dg-grid role="list" aria-label="Dashboard""#), "{body}");
    assert!(body.contains(r#"data-dg-tile data-id="a" role="listitem" aria-label="Sales""#), "{body}");
    assert!(body.contains(r#"data-dg-handle tabindex="0" role="button" aria-roledescription="movable tile" aria-label="Move Sales""#), "{body}");
    assert!(body.contains(r#"data-id="tile-3""#), "{body}");
    // The locked tile has nothing to drag or resize.
    let locked = &body[body.find(r#"data-id="b""#).unwrap()..body.find(r#"data-id="tile-3""#).unwrap()];
    assert!(!locked.contains("data-dg-handle") && !locked.contains("data-dg-resize"), "{locked}");

    // From state: the layout it names, the rest from props.
    let page = render_with(grid(json!({ "bind": "layout" }), kids()), json!({ "layout": [{ "id": "a", "x": 4, "y": 0, "w": 4, "h": 1 }] }), json!({})).unwrap();
    assert!(page.css.contains("grid-column: 5 / span 4;\n  grid-row: 1 / span 1;"), "{}", page.css);
    // Not editable: no handles at all.
    let body = plain(&render_with(grid(json!({ "editable": false }), kids()), json!({}), json!({})).unwrap().body);
    assert!(!body.contains("data-dg-handle") && !body.contains("data-dg-help"), "{body}");

    let err = |root: Json| render_with(root, json!({}), json!({})).unwrap_err().to_string();
    assert!(err(grid(json!({}), vec![json!({ "type": "text", "props": { "value": "x" } })])).contains("takes only `ui-tile` children"));
    assert!(err(grid(json!({}), vec![tile(json!({ "id": "a" })), tile(json!({ "id": "a" }))])).contains("two tiles have the id `a`"));
    assert!(err(grid(json!({ "columns": 0 }), vec![tile(json!({}))])).contains("`columns` must be from 1 to 48"));
    assert!(err(tile(json!({}))).contains("must be a direct child of `ui-dashboard-grid`"));
}

#[test]
fn data_table_views_menu_and_mistakes() {
    let table = |props: Json| {
        let mut p = json!({ "rows": [{ "id": 1, "name": "a" }] });
        p.as_object_mut().unwrap().extend(props.as_object().unwrap().clone());
        json!({ "type": "ui-data-table", "props": p, "children": [ { "type": "ui-data-table-column", "props": { "field": "name" } } ] })
    };
    // Ready-made views get ids, and the button names the view shown.
    let page = render_with(table(json!({ "views": [{ "name": "All" }, { "id": "mine", "name": "Mine" }], "bindCurrentView": "v" })), json!({ "v": "mine" }), json!({})).unwrap();
    let body = plain(&page.body);
    assert!(body.contains("data-dt-views-button") && body.contains(r#"<span data-dt-views-label>Mine</span>"#), "{body}");
    assert!(body.contains(r#"data-dt-views role="dialog" aria-label="Views""#), "{body}");
    assert!(body.contains(r#"&quot;id&quot;:&quot;preset-1&quot;"#) || body.contains(r#""id":"preset-1""#), "{body}");
    // `bindViews` alone shows the menu; nothing shows it without either.
    let body = plain(&render_with(table(json!({ "bindViews": "saved" })), json!({ "saved": [] }), json!({})).unwrap().body);
    assert!(body.contains("<span data-dt-views-label>Views</span>"), "{body}");
    assert!(!plain(&render(table(json!({})))).contains("data-dt-views"));
    let err = |root: Json| render_with(root, json!({}), json!({})).unwrap_err().to_string();
    assert!(err(table(json!({ "views": "nope" }))).contains("`views` must be a list"));
    assert!(err(table(json!({ "views": [1] }))).contains("each of `views` must be an object"));
}

#[test]
fn dashboard_grid_breakpoints_lay_out_each_size() {
    let tile = |p: Json| json!({ "type": "ui-tile", "props": p, "children": [ { "type": "text", "props": { "value": "x" } } ] });
    let kids = vec![
        tile(json!({ "id": "a", "x": 0, "y": 0, "w": 6, "h": 2 })),
        tile(json!({ "id": "b", "x": 6, "y": 0, "w": 6, "h": 2 })),
        tile(json!({ "id": "c", "x": 0, "y": 2, "w": 12, "h": 1, "layouts": { "sm": { "x": 0, "y": 0, "w": 2, "h": 1 } } })),
    ];
    let grid = json!({ "type": "ui-dashboard-grid", "props": { "breakpoints": { "md": { "width": 600, "columns": 6 }, "lg": { "width": 1000, "columns": 12 }, "sm": { "width": 400, "columns": 2 } } }, "children": kids });
    let page = render_with(grid.clone(), json!({}), json!({})).unwrap();
    let css = &page.css;
    // The widest by default; each smaller one under the width of the one above.
    assert!(css.contains("@container (max-width: 999.98px)") && css.contains("@container (max-width: 599.98px)"), "{css}");
    assert!(css.contains("--dg-cols: 12;") && css.contains("--dg-cols: 6;") && css.contains("--dg-cols: 2;"), "{css}");
    // md, worked out: widths halved, packed in reading order.
    let md = &css[css.find("@container (max-width: 999.98px)").unwrap()..];
    assert!(md.contains("grid-column: 4 / span 3;"), "{md}");
    // sm: c where its `layouts` says, first; a and b packed after it.
    assert!(css.contains("grid-column: 1 / span 2;\n  grid-row: 1 / span 1;"), "{css}");
    let config = |page: &Page| -> Json {
        let body = &page.body;
        let at = body.find("data-i-xc=\"").unwrap() + 11;
        serde_json::from_str(&body[at..at + body[at..].find('"').unwrap()].replace("&quot;", "\"").replace("&amp;", "&")).unwrap()
    };
    let c = config(&page);
    let sizes: Vec<String> = c["sizes"].as_array().unwrap().iter().map(|z| format!("{}:{}:{}", z["name"].as_str().unwrap(), z["width"], z["columns"])).collect();
    assert_eq!(sizes, ["lg:1000.0:12", "md:600.0:6", "sm:400.0:2"]);
    let at = |k: usize| c["sizes"][k]["items"].as_array().unwrap().iter().map(|i| format!("{}:{},{},{}x{}", i["id"].as_str().unwrap(), i["x"], i["y"], i["w"], i["h"])).collect::<Vec<_>>().join(" ");
    assert_eq!(at(1), "a:0,0,3x2 b:3,0,3x2 c:0,2,6x1");
    assert_eq!(at(2), "a:0,1,1x2 b:1,1,1x2 c:0,0,2x1");

    // `bind` with a layout per size; a list is the widest size's.
    let mut bound = grid.clone();
    bound["props"]["bind"] = json!("layout");
    let page = render_with(bound.clone(), json!({ "layout": { "md": [{ "id": "a", "x": 3, "y": 0, "w": 3, "h": 1 }] } }), json!({})).unwrap();
    assert!(config(&page)["sizes"][1]["items"][0]["x"] == json!(3), "{}", config(&page));
    let page = render_with(bound, json!({ "layout": [{ "id": "a", "x": 6, "y": 0, "w": 6, "h": 1 }] }), json!({})).unwrap();
    assert!(config(&page)["sizes"][0]["items"][0]["x"] == json!(6));

    let err = |b: Json| render_with(json!({ "type": "ui-dashboard-grid", "props": { "breakpoints": b }, "children": [tile(json!({}))] }), json!({}), json!({})).unwrap_err().to_string();
    assert!(err(json!({ "md": { "columns": 6 } })).contains("needs a `width`"));
    assert!(err(json!([600])).contains("must map names"));
}

#[test]
fn form_columns_and_sections_mix() {
    let field = |label: &str, span: Json| json!({ "type": "ui-field", "props": { "label": label, "span": span }, "children": [ { "type": "ui-input" } ] });
    let page = render_with(
        json!({ "type": "ui-form", "props": { "columns": 2 }, "children": [
            { "type": "ui-error-summary" },
            { "type": "ui-form-section", "props": { "title": "Address", "description": "Where to", "layout": "aside", "columns": 3, "span": "full" }, "children": [
                field("Street", json!(3)), field("City", json!(2)), field("Zip", json!(1)),
            ] },
            { "type": "ui-form-section", "props": { "columns": 2 }, "children": [ field("Wide", json!(5)) ] },
        ] }),
        json!({}),
        json!({}),
    )
    .unwrap();
    let body = plain(&page.body);
    // The form's grid holds the section, which spans it and has three columns of its own.
    assert!(body.contains(r#"<div data-form-columns><div data-form-grid data-cols="2">"#), "{body}");
    assert!(body.contains(r#"role="group" data-form-section data-layout="aside" aria-labelledby="#), "{body}");
    assert!(body.contains(r#"data-span="full"><div data-section-in><div data-section-head><h3 id="#), "{body}");
    assert!(body.contains(r#"data-cols="3""#), "{body}");
    // A span is kept to the columns there are: 3 of 3 and 5 of 2 are the full width.
    assert_eq!(body.matches(r#"data-span="full""#).count(), 3, "{body}");
    assert_eq!(body.matches(r#"data-span="2""#).count(), 1, "{body}");
    assert!(page.css.contains("--form-cols: 3;") && page.css.contains("@container (max-width: 399.98px)"), "{}", page.css);
    let bad = render_with(json!({ "type": "ui-form-section", "children": [ field("X", json!("wide")) ] }), json!({}), json!({}));
    assert!(bad.unwrap_err().to_string().contains("`span` must be a number of columns or `full`"));

    // A collapsible section folds under its heading: `<details>`, open unless `open` says not.
    let fold = |props: Json| plain(&render_with(json!({ "type": "ui-form-section", "props": props, "children": [ field("X", json!(1)) ] }), json!({ "shut": false }), json!({})).unwrap().body);
    let body = fold(json!({ "title": "More", "description": "Optional", "collapsible": true }));
    assert!(body.contains(r#"<details data-section-in open><summary data-section-head><h3 id="#), "{body}");
    assert!(body.contains("</p><span data-section-count></span></summary><div data-form-columns>") && body.contains("</details></div>"), "{body}");
    assert!(!fold(json!({ "title": "More", "collapsible": true, "open": "{{ shut }}" })).contains(" open>"));
    assert!(fold(json!({ "title": "More" })).contains("<div data-section-in><div data-section-head>"));
    let bad = render_with(json!({ "type": "ui-form-section", "props": { "collapsible": true }, "children": [] }), json!({}), json!({}));
    assert!(bad.unwrap_err().to_string().contains("needs a `title`"));
}
