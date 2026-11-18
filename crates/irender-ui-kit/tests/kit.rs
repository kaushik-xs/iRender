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
        ("ui-combobox", json!({ "options": ["a"] })),
        ("ui-split-button", json!({ "label": "Save" })),
        ("ui-theme-option", json!({ "option": "palette" })),
        ("ui-sidebar-item", json!({ "label": "Home" })),
        ("ui-command-item", json!({ "label": "L" })),
        ("ui-chat-input", json!({ "messages": "k" })),
        ("ui-data-table", json!({ "rows": [], "columns": [{ "field": "a" }] })),
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
        if c.name() == "ui-command-item" {
            root = json!({ "type": "ui-command", "children": [root] });
        }
        if c.name().starts_with("ui-data-table-") {
            root = json!({ "type": "ui-data-table", "props": { "rows": [], "columns": [{ "field": "a" }] }, "children": [root] });
        }
        if c.name() == "ui-sidebar-trigger" {
            root = json!({ "type": "ui-sidebar-layout", "props": { "bind": "k" }, "children": [root] });
        }
        let page = render_with(root, json!({ "k": null }), json!({}))
            .unwrap_or_else(|e| panic!("{}: {e}", c.name()));
        assert!(!page.body.is_empty(), "{}", c.name());
    }
    assert_eq!(kit, 79);
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
    // Sizes follow the theme's `space.scale` (density).
    for decl in ["background: var(--color-destructive, #dc2626);", "height: calc(32px * var(--space-scale, 1));", "height: calc(36px * var(--space-scale, 1));"] {
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
}

#[test]
fn theme_option_lists_its_choices_with_swatches() {
    let page = render_with(json!({ "type": "ui-theme-option", "props": { "option": "palette", "swatch": "color.primary" } }), json!({}), json!({})).unwrap();
    let body = plain(&page.body);
    assert!(body.starts_with(r#"<div role="group" aria-label="Colour" data-i-x="ui-theme-option" data-i-xc="{&quot;option&quot;:&quot;palette&quot;,&quot;fallback&quot;:&quot;zinc&quot;}">"#), "{body}");
    // The default is pressed until the behaviour reads the stored choice.
    // Zinc, the default, sets no primary of its own: its swatch is the base colour.
    assert!(body.contains(r#"<button type="button" data-choice="zinc" aria-pressed="true"><span aria-hidden="true"><span></span></span>Zinc</button>"#), "{body}");
    assert!(page.css.contains("background: #18181b;"), "zinc's swatch");
    assert!(body.contains(r#"<button type="button" data-choice="rose" aria-pressed="false"><span aria-hidden="true"><span></span></span>Rose</button>"#), "{body}");
    assert!(page.css.contains("background: #e11d48;"), "rose's swatch");
    // The kit's options are in the theme CSS, each with its dark colours.
    assert!(page.css.contains(r#":root[data-theme-palette="rose"] {"#) && page.css.contains(r#":root[data-theme-density="compact"] {"#));
    let err = render_with(json!({ "type": "ui-theme-option", "props": { "option": "nope" } }), json!({}), json!({})).unwrap_err();
    assert!(err.message.contains("`nope` is not a theme option (options: `palette`, `radius`, `density`, `text-size`, `font`)"), "{err}");
}

#[test]
fn theme_panel_has_a_picker_per_option_and_the_mode() {
    let doc = json!({ "type": "ui-theme-panel" });
    let body = plain(&render(doc));
    let labels: Vec<&str> = body.split(r#"<div aria-hidden="true">"#).skip(1).map(|r| &r[..r.find('<').unwrap()]).collect();
    // The palette first and chart colours (not registered here) last.
    // Custom colours come with the palette, which has a `custom` choice.
    assert_eq!(labels, ["Colour", "Custom colours", "Radius", "Density", "Text size", "Font", "Mode"], "{body}");
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
    assert!(body.contains(r##"<label data-color="primary">Primary<input type="color" value="#18181b" aria-label="Primary, picked">"##), "{body}");
    assert!(body.contains(r##"<input type="text" value="#18181b" aria-label="Primary, as hex" spellcheck="false" maxlength="7" placeholder="#rrggbb">"##), "{body}");
    assert!(body.contains(r##"data-color="brand-2">Second colour"##), "{body}");
    // The second brand colour falls back to the primary in themes without one.
    assert!(page.css.contains("background: var(--color-brand-2, var(--color-primary));"), "{}", page.css);
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
    // Its script and spinner keyframes ship once.
    assert_eq!(page.scripts.len(), 1);
    assert!(page.scripts[0].contains(r#"["ui-combobox"]"#));
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
    assert!(page.body.contains("data-i-x=\"ui-theme-toggle\""), "the index has a theme toggle");
    // The combobox lists every example by title, with its page as the value.
    let combobox = page.body.find("data-i-x=\"ui-combobox\"").unwrap();
    let options = &config(&page.body[combobox..])["options"];
    assert_eq!(options.as_array().unwrap().len(), examples.len());
    assert_eq!(options[0]["value"], examples[0]["href"]);
    assert_eq!(options[0]["label"], examples[0]["title"]);
    assert_eq!(options[0]["description"], examples[0]["summary"]);
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
    assert!(body.contains(r#"data-i-xc="{&quot;messages&quot;:&quot;messages&quot;,&quot;endpoint&quot;:&quot;/chat&quot;,&quot;bind&quot;:&quot;draft&quot;,&quot;waiting&quot;:null,&quot;threads&quot;:null,&quot;thread&quot;:null}""#), "{body}");
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
