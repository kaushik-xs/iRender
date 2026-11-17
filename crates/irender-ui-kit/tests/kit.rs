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
    ];
    let mut kit = 0;
    for c in engine.registry.iter().filter(|c| c.name().starts_with("ui-")) {
        kit += 1;
        assert!(!c.info().doc.is_empty(), "{} has no doc", c.name());
        let props = minimal.iter().find(|(n, _)| *n == c.name()).map_or(json!({}), |(_, p)| p.clone());
        let mut root = json!({ "type": c.name(), "props": props });
        if c.name() == "ui-dialog-close" {
            root = json!({ "type": "ui-dialog", "children": [root] });
        }
        let page = render_with(root, json!({ "k": null }), json!({}))
            .unwrap_or_else(|e| panic!("{}: {e}", c.name()));
        assert!(!page.body.is_empty(), "{}", c.name());
    }
    assert_eq!(kit, 44);
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
    for decl in ["background: var(--color-destructive, #dc2626);", "height: 32px;", "height: 36px;"] {
        assert!(page.css.contains(decl), "missing `{decl}`");
    }
}

#[test]
fn card_alert_and_typography_structure() {
    let body = plain(&render(json!({ "type": "ui-card", "props": { "title": "T", "description": "D" },
        "parts": { "title": { "attrs": { "id": "t" } } },
        "children": [{ "type": "ui-card-footer", "children": [{ "type": "ui-button", "props": { "label": "OK" } }] }] })));
    assert_eq!(body, r#"<div><div><h3 id="t">T</h3><p>D</p></div><div><button type="button">OK</button></div></div>"#);

    let body = plain(&render(json!({ "type": "ui-alert", "props": { "icon": "⚠", "title": "Oops", "variant": "destructive" } })));
    assert_eq!(body, r#"<div role="alert"><span aria-hidden="true">⚠</span><div>Oops</div></div>"#);

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
fn server_expanded_lists_reject_state() {
    for (kind, props) in [
        ("ui-radio-group", json!({ "options": "{{ opts }}" })),
        ("ui-breadcrumb", json!({ "items": "{{ opts }}" })),
    ] {
        let err = render_with(json!({ "type": kind, "props": props }), json!({ "opts": [] }), json!({})).unwrap_err();
        assert!(err.message.contains("can't use state"), "{kind}: {err}");
    }
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
        { "type": "ui-avatar", "props": { "src": "{{ bad }}" } }
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
    assert!(body.starts_with(r#"<button popovertarget="m" aria-haspopup="menu" type="button">Open</button><div popover="auto" id="m">"#), "{body}");
    assert!(body.contains(r#"<button type="button" popovertarget="m" popovertargetaction="hide"><span>Profile</span><span>⇧⌘P</span></button>"#), "{body}");
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
