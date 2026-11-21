mod support;

use irender_core::{ComponentInfo, Format, Node, PropDef, PropKind, Val};
use irender_html::{Component, Engine, Page, RenderCx, RenderError};
use serde_json::{Value as Json, json};

fn render_with(engine: &Engine, root: Json, state: Json, data: Json) -> Result<Page, RenderError> {
    let doc = json!({ "version": 1, "state": state, "root": root });
    let ir = engine.load(&doc.to_string(), Format::Json).unwrap_or_else(|d| panic!("{d}"));
    engine.render(&ir, &data)
}

fn render(root: Json) -> Page {
    render_with(&Engine::new(), root, json!({}), json!({})).unwrap()
}

/// Removes generated class names so assertions don't depend on hashes.
fn strip_classes(html: &str) -> String {
    let mut out = String::new();
    let mut rest = html;
    while let Some(i) = rest.find(" class=\"") {
        out.push_str(&rest[..i]);
        let after = &rest[i + 8..];
        let end = after.find('"').unwrap();
        let kept: Vec<_> = after[..end].split(' ').filter(|c| !c.starts_with("i-")).collect();
        if !kept.is_empty() {
            out.push_str(&format!(" class=\"{}\"", kept.join(" ")));
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

// ---------------------------------------------------------------- built-ins

#[test]
fn element_renders_any_tag_and_void_elements() {
    let page = render(json!({ "type": "element", "props": { "tag": "section" },
        "attrs": { "aria-label": "Main", "data-n": 3, "hidden": false, "inert": true },
        "children": [{ "type": "element", "props": { "tag": "img" }, "attrs": { "src": "/a.png", "alt": "A" } }] }));
    assert_eq!(page.body, r#"<section aria-label="Main" data-n="3" inert><img src="/a.png" alt="A"></section>"#);
}

#[test]
fn void_element_with_children_is_a_render_error() {
    let err = render_with(
        &Engine::new(),
        json!({ "type": "element", "props": { "tag": "br" }, "children": [{ "type": "element" }] }),
        json!({}),
        json!({}),
    )
    .unwrap_err();
    assert_eq!(err.to_string(), "root: `<br>` cannot have children");
}

#[test]
fn text_escapes_and_defaults_to_span() {
    let page = render(json!({ "type": "text", "props": { "value": "<b>&\"</b>" }, "attrs": { "title": "\"q\" <x>" } }));
    assert_eq!(page.body, r#"<span title="&quot;q&quot; &lt;x&gt;">&lt;b&gt;&amp;"&lt;/b&gt;</span>"#);
}

#[test]
fn stack_maps_props_to_flex_styles() {
    let page = render(json!({ "type": "stack",
        "props": { "direction": "row", "gap": "$space.sm", "align": "center", "justify": "space-between", "wrap": true } }));
    for decl in ["display: flex;", "flex-direction: row;", "gap: var(--space-sm);", "align-items: center;",
                 "justify-content: space-between;", "flex-wrap: wrap;"] {
        assert!(page.css.contains(decl), "missing `{decl}` in\n{}", page.css);
    }
    let page = render(json!({ "type": "stack", "props": { "gap": 12 } }));
    assert!(page.css.contains("flex-direction: column;") && page.css.contains("gap: calc(12px * var(--space-scale, 1) * var(--space-gap, 1));"), "{}", page.css);
}

#[test]
fn button_variants_parts_and_defaults() {
    let page = render(json!({ "type": "button", "props": { "label": "Save", "variant": "ghost" },
        "parts": { "label": { "attrs": { "data-part": "label" }, "style": { "text-transform": "uppercase" } } } }));
    assert_eq!(
        strip_classes(&page.body),
        r#"<button type="button"><span data-part="label">Save</span></button>"#
    );
    assert!(page.css.contains("background: transparent;"), "ghost variant");
    assert!(page.css.contains("text-transform: uppercase;"), "part style");
    let label_classes = page.body.split("<span class=\"").nth(1).unwrap();
    assert!(label_classes.starts_with("i-"), "part style becomes a class on the part");

    let page = render(json!({ "type": "button", "props": { "type": "submit", "disabled": true }, "children": [{ "type": "text", "props": { "value": "Go" } }] }));
    assert_eq!(strip_classes(&page.body), r#"<button type="submit" disabled><span>Go</span></button>"#);
    assert!(page.css.contains("background: var(--color-primary);"), "primary is the default");
}

#[test]
fn input_binding_and_static_value() {
    let page = render_with(&Engine::new(),
        json!({ "type": "input", "props": { "bind": "q", "placeholder": "Search", "type": "search" } }),
        json!({ "q": "mugs" }), json!({})).unwrap();
    assert_eq!(strip_classes(&page.body), r#"<input type="search" placeholder="Search" value="mugs" data-i-bind="q">"#);
    assert!(page.interactive);

    let page = render(json!({ "type": "input", "props": { "value": "x", "name": "n" } }));
    assert_eq!(strip_classes(&page.body), r#"<input type="text" name="n" value="x">"#);
    assert!(!page.interactive);
}

#[test]
fn list_renders_items_with_scopes() {
    let page = render_with(&Engine::new(),
        json!({ "type": "list", "props": { "items": "{{ users }}", "as": "u", "index": "i" },
            "children": [{ "type": "text", "props": { "value": "{{ i + 1 }}. {{ u.name }}" } }] }),
        json!({}), json!({ "users": [{ "name": "Ada" }, { "name": "Linus" }] })).unwrap();
    assert_eq!(page.body, "<div><span>1. Ada</span><span>2. Linus</span></div>");

    let empty = render(json!({ "type": "list", "props": { "items": "{{ nothing }}" }, "children": [{ "type": "element" }] }));
    assert_eq!(empty.body, "<div></div>");

    let err = render_with(&Engine::new(), json!({ "type": "list", "props": { "items": 3 } }), json!({}), json!({})).unwrap_err();
    assert_eq!(err.to_string(), "root: `items` must evaluate to an array");
}

#[test]
fn nested_lists_shadow_outer_names() {
    let page = render_with(&Engine::new(),
        json!({ "type": "list", "props": { "items": "{{ groups }}" }, "children": [
            { "type": "list", "props": { "items": "{{ item.xs }}" }, "children": [
                { "type": "text", "props": { "value": "{{ item }}" } } ] } ] }),
        json!({}), json!({ "groups": [{ "xs": [1, 2] }, { "xs": [3] }] })).unwrap();
    assert_eq!(page.body, "<div><div><span>1</span><span>2</span></div><div><span>3</span></div></div>");
}

// ---------------------------------------------------------------- bindings & actions

fn bindings(body: &str) -> Vec<Json> {
    body.match_indices("data-i-b=\"")
        .map(|(i, _)| {
            let rest = &body[i + 10..];
            let raw = &rest[..rest.find('"').unwrap()];
            serde_json::from_str(&raw.replace("&quot;", "\"").replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&#39;", "'")).unwrap()
        })
        .collect()
}

fn attr_json(body: &str, name: &str) -> Json {
    let start = body.find(&format!("{name}=\"")).unwrap() + name.len() + 2;
    let raw = &body[start..start + body[start..].find('"').unwrap()];
    let raw = raw.replace("&quot;", "\"").replace("&lt;", "<").replace("&gt;", ">").replace("&#39;", "'");
    serde_json::from_str(&raw.replace("&amp;", "&")).unwrap()
}

#[test]
fn only_state_dependent_values_become_bindings() {
    let page = render_with(&Engine::new(),
        json!({ "type": "element", "attrs": { "data-a": "{{ user.id }}", "data-b": "{{ n }}" },
            "class": ["static", "{{ user.role }}", "{{ n > 1 ? 'many' : 'one' }}"] }),
        json!({ "n": 2 }), json!({ "user": { "id": 7, "role": "admin" } })).unwrap();
    assert_eq!(page.body.split('>').next().unwrap().matches("data-i-b").count(), 1);
    assert!(page.body.starts_with(r#"<div class="static admin many" data-a="7" data-b="2""#), "{}", page.body);
    let b = &bindings(&page.body)[0];
    assert_eq!(b.as_array().unwrap().len(), 2, "{b}");
    assert_eq!(b[0]["k"], "class");
    assert_eq!(b[0]["i"], "many", "initial class so the runtime can swap it");
    assert_eq!(b[1], json!({ "k": "attr", "n": "data-b", "v": { "r": { "p": ["n"] } } }));
}

#[test]
fn dynamic_styles_use_css_variables() {
    let page = render_with(&Engine::new(),
        json!({ "type": "element", "attrs": { "style": "outline: 1px solid red" },
            "style": { "width": "{{ pct }}%", "height": "{{ h }}", "opacity": "{{ o }}", "color": "{{ bad }}" } }),
        json!({ "pct": 40, "o": 0.5 }), json!({ "h": 10, "bad": "red; position: fixed" })).unwrap();
    let open = page.body.split('>').next().unwrap();
    // A dropped value is still set, to `initial`, so no ancestor's var shows through.
    assert!(open.contains(r#"style="--i-d0: 40%; --i-d1: 10px; --i-d2: 0.5; --i-d3: initial; outline: 1px solid red""#), "{open}");
    assert!(!open.contains("position"), "values that could inject declarations are dropped");
    assert!(page.css.contains("width: var(--i-d0);") && page.css.contains("height: var(--i-d1);"));
    let kinds: Vec<_> = bindings(&page.body)[0].as_array().unwrap().iter().map(|b| (b["n"].clone(), b["px"].clone())).collect();
    assert_eq!(kinds, [(json!("--i-d0"), json!(true)), (json!("--i-d2"), json!(false))], "only state-backed vars bind");
}

#[test]
fn dynamic_styles_without_a_value_or_with_a_keyword_never_inherit_a_var() {
    let page = render_with(&Engine::new(),
        json!({ "type": "element", "style": {
            "color": "{{ none }}",
            "border-color": "{{ keyword }}",
            ":hover": { "background": "{{ keyword }}" } } }),
        json!({}), json!({ "none": null, "keyword": "inherit" })).unwrap();
    let open = page.body.split('>').next().unwrap();
    // No value: `initial`, not left unset for an ancestor's `--i-d0` to fill. A keyword
    // goes on the property itself; under `:hover` it can't, so that var is emptied too.
    assert!(open.contains(r#"style="--i-d0: initial; --i-d1: initial; border-color: inherit; --i-d2: initial""#), "{open}");
}

#[test]
fn actions_serialize_with_server_data_folded() {
    let page = render_with(&Engine::new(),
        json!({ "type": "button", "props": { "label": "Go" }, "on": {
            "click": [
                { "action": "set_state", "key": "n", "value": "{{ n + step }}" },
                { "action": "submit", "endpoint": "/api/{{ id }}", "method": "put", "body": { "n": "{{ n }}", "k": 1 }, "into": "res" },
                { "action": "navigate", "url": "/done?n={{ n }}" },
                { "action": "call", "name": "track", "args": { "id": "{{ id }}" } }
            ] } }),
        json!({ "n": 0, "res": null }), json!({ "id": 9, "step": 5 })).unwrap();
    let on = attr_json(&page.body, "data-i-on");
    assert_eq!(on, json!({ "click": [
        { "a": "set", "k": "n", "v": { "r": { "b": "+", "x": { "p": ["n"] }, "y": { "l": 5 } } } },
        { "a": "submit", "e": { "t": ["/api/", { "l": 9 }] }, "m": "PUT",
          "b": { "n": { "r": { "p": ["n"] } }, "k": { "r": { "l": 1 } } }, "into": "res" },
        { "a": "nav", "u": { "t": ["/done?n=", { "p": ["n"] }] } },
        { "a": "call", "n": "track", "args": { "id": { "r": { "l": 9 } } } }
    ] }));
}

// ---------------------------------------------------------------- safety

#[test]
fn unsafe_urls_from_data_are_dropped_at_render() {
    let root = json!({ "type": "element", "props": { "tag": "a" }, "attrs": { "href": "{{ link }}" } });
    let data = json!({ "link": "javascript:alert(1)" });
    let page = render_with(&Engine::untrusted(), root.clone(), json!({}), data.clone()).unwrap();
    assert_eq!(page.body, "<a></a>");
    let page = render_with(&Engine::new(), root, json!({}), data).unwrap();
    assert_eq!(page.body, r#"<a href="javascript:alert(1)"></a>"#, "trusted documents may use any URL");
}

#[test]
fn state_json_cannot_close_the_script_tag() {
    let page = render_with(&Engine::new(), json!({ "type": "text", "props": { "value": "{{ s }}" } }),
        json!({ "s": "</script><script>alert(1)</script>" }), json!({})).unwrap();
    let fragment = page.fragment();
    assert_eq!(fragment.matches("</script>").count(), 2, "only our own two script tags close");
    assert!(fragment.contains(r"</script>"));
}

#[test]
fn document_wraps_fragment() {
    let page = render(json!({ "type": "text", "props": { "value": "x" } }));
    let doc = page.document("A <b> title");
    assert!(doc.starts_with("<!doctype html>"));
    assert!(doc.contains("<title>A &lt;b&gt; title</title>"));
    assert!(doc.contains("<style>body{margin:0;background:var(--color-background,var(--color-surface))}</style>"), "no light frame in dark mode");
    assert!(!page.fragment().contains("body{"), "a fragment leaves the host's body alone");
    assert!(doc.contains("<div data-i-root style=\"display:contents\"><span>x</span></div>"));
    assert!(!doc.contains("data-i-state"), "static pages ship no state or runtime");
}

// ---------------------------------------------------------------- engine

#[test]
fn output_is_deterministic() {
    let (src, data) = support::product();
    let engine = Engine::new();
    let ir = engine.load(&src, Format::Json).unwrap();
    let a = engine.render(&ir, &data).unwrap().document("t");
    let b = engine.render(&engine.load(&src, Format::Json).unwrap(), &data).unwrap().document("t");
    assert_eq!(a, b);
}

#[test]
fn identical_styles_share_one_class() {
    let item = json!({ "type": "element", "style": { "color": "red", ":hover": { "color": "blue" } } });
    let page = render(json!({ "type": "element", "children": [item.clone(), item.clone(), item] }));
    assert_eq!(page.css.matches("color: red;").count(), 1);
    let classes: Vec<_> = page.body.match_indices("class=\"").map(|(i, _)| &page.body[i..i + 20]).collect();
    assert_eq!(classes.len(), 3);
    assert!(classes.windows(2).all(|w| w[0] == w[1]));
}

#[test]
fn minified_css_is_smaller_and_keeps_layers() {
    let (src, data) = support::product();
    let ir = Engine::new().load(&src, Format::Json).unwrap();
    let plain = Engine::new().render(&ir, &data).unwrap();
    let min = Engine { minify: true, ..Engine::new() }.render(&ir, &data).unwrap();
    assert!(min.css.len() < plain.css.len(), "{} vs {}", min.css.len(), plain.css.len());
    assert!(!min.css.contains('\n'));
    // lightningcss may fold the `@layer a, b;` statement into the first block; what
    // matters is that `i-base` is still declared before `i-instance`.
    let base = min.css.find("@layer i-base").unwrap();
    let instance = min.css.find("@layer i-instance").unwrap();
    assert!(base < instance, "{}", min.css);
    assert!(min.css.contains(":root{--color-primary:#0f766e"), "theme variables survive");
    assert_eq!(min.body, plain.body);
}

#[test]
fn deeply_nested_documents_fail_cleanly() {
    let (ok, _) = support::deep(50);
    assert!(Engine::new().load(&ok, Format::Json).is_ok());
    let (too_deep, _) = support::deep(100);
    let err = Engine::new().load(&too_deep, Format::Json).unwrap_err();
    assert!(err.to_string().contains("recursion limit exceeded"), "{err}");
}

struct Badge;

const BADGE_PROPS: &[PropDef] = &[
    PropDef::new("text", PropKind::Any, "").required(),
    PropDef::new("tone", PropKind::Enum(&["info", "warn"]), ""),
];

impl Component for Badge {
    fn name(&self) -> &'static str {
        "badge"
    }

    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "", props: BADGE_PROPS, parts: &[], children: false, open_props: false }
    }

    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        let tone = cx.prop_string(node, "tone").unwrap_or_else(|| "info".into());
        let mut el = cx.element("mark", node);
        el.attr("data-tone", tone).class("badge");
        el.text(node.prop("text").cloned().unwrap_or(Val::from("")));
        cx.open(&el, node)?;
        cx.close("mark");
        Ok(())
    }
}

#[test]
fn custom_components_validate_and_render() {
    let mut engine = Engine::new();
    engine.registry.register(Badge);
    let page = render_with(&engine, json!({ "type": "badge", "props": { "text": "{{ n }} new" }, "attrs": { "data-tone": "x" } }),
        json!({ "n": 3 }), json!({})).unwrap();
    assert!(page.body.starts_with(r#"<mark class="badge" data-tone="x" data-i-b="#), "document attrs win: {}", page.body);
    assert!(page.body.ends_with(">3 new</mark>"));

    let doc = json!({ "version": 1, "root": { "type": "badge", "props": { "tone": "loud" } } });
    let err = engine.load(&doc.to_string(), Format::Json).unwrap_err();
    assert_eq!(err.to_string(), "root.props.tone: expected one of: info, warn, got `loud`\nroot.props: missing required prop `text`");
}

#[test]
fn one_document_renders_from_many_threads() {
    let (src, data) = support::table(50);
    let engine = Engine::new();
    let ir = engine.load(&src, Format::Json).unwrap();
    let expected = engine.render(&ir, &data).unwrap().document("t");
    std::thread::scope(|s| {
        let handles: Vec<_> = (0..8)
            .map(|_| s.spawn(|| (0..20).map(|_| engine.render(&ir, &data).unwrap().document("t")).collect::<Vec<_>>()))
            .collect();
        for h in handles {
            assert!(h.join().unwrap().iter().all(|html| *html == expected));
        }
    });
}

#[cfg(feature = "parallel")]
mod parallel {
    use super::*;

    fn both(src: &str, data: &Json) -> (Result<String, RenderError>, Result<String, RenderError>) {
        let sequential = Engine::new();
        let parallel = Engine { parallel_min_items: Some(1), ..Engine::new() };
        let ir = sequential.load(src, Format::Json).unwrap_or_else(|d| panic!("{d}"));
        let render = |e: &Engine| e.render(&ir, data).map(|p| p.document("t"));
        (render(&sequential), render(&parallel))
    }

    #[test]
    fn output_is_identical_to_sequential() {
        for (src, data) in [support::table(1000), support::table(3), support::product(), support::unique_styles(300)] {
            let (a, b) = both(&src, &data);
            assert_eq!(a.unwrap(), b.unwrap());
        }
    }

    #[test]
    fn nested_lists_and_outer_scopes() {
        let doc = json!({ "version": 1, "state": { "pick": 1 }, "root": {
            "type": "list", "props": { "items": "{{ groups }}", "as": "g", "index": "gi" },
            "children": [{ "type": "list", "props": { "items": "{{ g.xs }}" }, "children": [
                { "type": "text", "props": { "value": "{{ gi }}.{{ index }}={{ item }}{{ pick == item ? '*' : '' }}" },
                  "style": { "color": "{{ item % 2 == 0 ? 'red' : 'blue' }}" } } ] }] } });
        let groups: Vec<Json> = (0..200).map(|g| json!({ "xs": (0..(g % 7)).collect::<Vec<_>>() })).collect();
        let (a, b) = both(&doc.to_string(), &json!({ "groups": groups }));
        let a = a.unwrap();
        assert!(a.contains(">195.5=5</span>") && a.contains(">2.1=1*</span>"), "{a}");
        assert_eq!(a, b.unwrap());
    }

    #[test]
    fn first_error_in_document_order_wins() {
        let doc = json!({ "version": 1, "root": { "type": "list", "props": { "items": "{{ rows }}" }, "children": [
            { "type": "list", "props": { "items": "{{ item.xs }}" }, "attrs": { "data-row": "{{ index }}" } } ] } });
        let rows: Vec<Json> =
            (0..1000).map(|i| if i == 300 || i == 700 { json!({ "xs": i }) } else { json!({ "xs": [] }) }).collect();
        let (a, b) = both(&doc.to_string(), &json!({ "rows": rows }));
        assert_eq!(a.unwrap_err(), b.unwrap_err());
    }
}

// ---------------------------------------------------------------- if

#[test]
fn if_without_state_is_decided_on_the_server() {
    let root = json!({ "type": "element", "children": [
        { "type": "if", "props": { "when": "{{ user.admin }}" },
          "children": [{ "type": "text", "props": { "value": "admin" } }] },
        { "type": "if", "props": { "when": "{{ !user.admin }}" },
          "children": [{ "type": "text", "props": { "value": "guest" } }] }
    ] });
    let page = render_with(&Engine::new(), root, json!({}), json!({ "user": { "admin": true } })).unwrap();
    assert_eq!(page.body, "<div><span>admin</span></div>");
    assert!(!page.interactive);
}

#[test]
fn if_with_state_renders_between_markers() {
    let shown = |open: bool| {
        let root = json!({ "type": "if", "props": { "when": "{{ open }}" },
            "children": [{ "type": "text", "props": { "value": "{{ name }}" } }] });
        render_with(&Engine::new(), root, json!({ "open": open }), json!({ "name": "Ada" })).unwrap()
    };
    let page = shown(true);
    assert_eq!(page.body, r#"<template data-i-if="{&quot;r&quot;:{&quot;p&quot;:[&quot;open&quot;]}}"></template><span>Ada</span><template data-i-fi></template>"#);
    assert!(page.interactive);
    let page = shown(false);
    assert_eq!(page.body, r#"<template data-i-if="{&quot;r&quot;:{&quot;p&quot;:[&quot;open&quot;]}}"><span>Ada</span></template><template data-i-fi></template>"#);
}

#[test]
fn if_requires_when() {
    let doc = json!({ "version": 1, "root": { "type": "if" } });
    let err = Engine::new().load(&doc.to_string(), Format::Json).unwrap_err();
    assert_eq!(err.to_string(), "root.props: missing required prop `when`");
}

/// `if`, an else-if and an `else` over `n`: big, small, or zero.
fn branches() -> Json {
    let text = |v: &str| json!([{ "type": "text", "props": { "value": v } }]);
    json!({ "type": "element", "children": [
        { "type": "if", "props": { "when": "{{ n > 2 }}" }, "children": text("big") },
        { "type": "else", "props": { "when": "{{ n > 0 }}" }, "children": text("small") },
        { "type": "else", "children": text("zero") } ] })
}

/// The page as shown: hidden content (inside `<template>`s) and markers removed.
fn shown(html: &str) -> String {
    let mut out = String::new();
    let mut rest = html;
    while let Some(i) = rest.find("<template") {
        out.push_str(&rest[..i]);
        rest = &rest[i + rest[i..].find("</template>").unwrap() + 11..];
    }
    out + rest
}

#[test]
fn else_shows_when_no_earlier_branch_does() {
    // Decided on the server: only the matching branch is written.
    for (n, body) in [(3, "big"), (1, "small"), (0, "zero")] {
        let page = render_with(&Engine::new(), branches(), json!({}), json!({ "n": n })).unwrap();
        assert_eq!(page.body, format!("<div><span>{body}</span></div>"));
    }
    // From state: every branch has markers, and exactly one is shown.
    for (n, body) in [(3, "big"), (1, "small"), (0, "zero")] {
        let page = render_with(&Engine::new(), branches(), json!({ "n": n }), json!({})).unwrap();
        assert_eq!(shown(&page.body), format!("<div><span>{body}</span></div>"), "{}", page.body);
        assert_eq!(page.body.matches("data-i-fi").count(), 3);
    }
    // The browser gets each branch's full condition.
    let page = render_with(&Engine::new(), branches(), json!({ "n": 0 }), json!({})).unwrap();
    let conds: Vec<Json> = page.body.match_indices("data-i-if=\"").map(|(i, _)| attr_json(&page.body[i..], "data-i-if")).collect();
    let (big, small) = (json!({ "b": ">", "x": { "p": ["n"] }, "y": { "l": 2 } }), json!({ "b": ">", "x": { "p": ["n"] }, "y": { "l": 0 } }));
    assert_eq!(conds[1], json!({ "r": { "b": "&&", "x": { "u": "!", "a": big }, "y": small } }));
    assert_eq!(conds[2], json!({ "r": { "b": "&&", "x": { "u": "!", "a": big }, "y": { "u": "!", "a": small } } }));
}

#[test]
fn else_after_a_text_template_condition() {
    // `when` with text around its expression is a string: truthy unless empty.
    let root = json!({ "type": "element", "children": [
        { "type": "if", "props": { "when": "{{ name }}{{ suffix }}" }, "children": [{ "type": "text", "props": { "value": "named" } }] },
        { "type": "else", "children": [{ "type": "text", "props": { "value": "anonymous" } }] } ] });
    let page = render_with(&Engine::new(), root.clone(), json!({}), json!({ "name": "", "suffix": "" })).unwrap();
    assert_eq!(page.body, "<div><span>anonymous</span></div>");
    let page = render_with(&Engine::new(), root, json!({}), json!({ "name": "", "suffix": "x" })).unwrap();
    assert_eq!(page.body, "<div><span>named</span></div>");
}

#[test]
fn else_must_follow_an_if() {
    let err = |root: Json| load(&Engine::new(), json!({ "version": 1, "root": root })).unwrap_err().to_string();
    let text = json!({ "type": "text", "props": { "value": "x" } });
    let msg = "`else` must directly follow an `if`, or an `else` with `when`";
    assert_eq!(err(json!({ "type": "else" })), format!("root.type: {msg}"));
    assert_eq!(err(json!({ "type": "element", "children": [text, { "type": "else" }] })), format!("root.children[1].type: {msg}"));
    // A plain `else` ends the chain.
    assert_eq!(
        err(json!({ "type": "element", "children": [
            { "type": "if", "props": { "when": true } }, { "type": "else" }, { "type": "else" }] })),
        format!("root.children[2].type: {msg}")
    );
    assert_eq!(
        err(json!({ "type": "element", "children": [
            { "type": "if", "props": { "when": true } }, { "type": "else", "attrs": { "id": "x" } }] })),
        "root.children[1]: `else` only takes `props.when` and children"
    );
}

// ---------------------------------------------------------------- reactive lists

#[test]
fn list_over_state_adds_an_item_template() {
    let root = json!({ "type": "list", "props": { "items": "{{ todos }}", "as": "todo", "tag": "ul" },
        "children": [{ "type": "text", "props": { "tag": "li", "value": "{{ index + 1 }}. {{ todo.title }} {{ suffix }}" } }] });
    let page = render_with(&Engine::new(), root, json!({ "todos": [{ "title": "a" }] }), json!({ "suffix": "!" })).unwrap();
    let body = &page.body;
    // The browser rebuilds from the template, where `todo` and `index` stay paths and
    // server data is folded in. The server still renders the initial items.
    assert!(body.starts_with("<ul data-i-list=\""), "{body}");
    assert_eq!(attr_json(body, "data-i-list"), json!({ "v": { "r": { "p": ["todos"] } }, "as": "todo", "ix": "index" }));
    let tpl = &body[body.find("<template>").unwrap()..body.find("</template>").unwrap()];
    assert!(tpl.contains(r#"<li data-i-b="#), "{tpl}");
    assert!(tpl.contains("&quot;todo&quot;,&quot;title&quot;") && tpl.contains("{&quot;l&quot;:&quot;!&quot;}"), "{tpl}");
    assert!(body.ends_with("</template><li>1. a !</li></ul>"), "{body}");
}

#[test]
fn keyed_list_writes_its_key_for_the_browser() {
    let root = json!({ "type": "list", "props": { "items": "{{ todos }}", "as": "todo", "key": "{{ todo.id }}-{{ prefix }}" },
        "children": [{ "type": "text", "props": { "value": "{{ todo.id }}" } }] });
    let page = render_with(&Engine::new(), root, json!({ "todos": [{ "id": 1 }] }), json!({ "prefix": "p" })).unwrap();
    // `todo` stays a path; server data is folded in.
    assert_eq!(attr_json(&page.body, "data-i-list")["k"], json!({ "t": [{ "p": ["todo", "id"] }, "-", { "l": "p" }] }));
    assert!(page.body.ends_with("</template><span>1</span></div>"), "{}", page.body);

    // A list over server data renders on the server only, so the key is unused.
    let root = json!({ "type": "list", "props": { "items": "{{ rows }}", "key": "{{ item }}" },
        "children": [{ "type": "text", "props": { "value": "{{ item }}" } }] });
    let page = render_with(&Engine::new(), root, json!({}), json!({ "rows": [1] })).unwrap();
    assert_eq!(page.body, "<div><span>1</span></div>");
}

#[test]
fn list_over_server_data_has_no_template() {
    let root = json!({ "type": "list", "props": { "items": "{{ rows }}" },
        "children": [{ "type": "text", "props": { "value": "{{ item }}" } }] });
    let page = render_with(&Engine::new(), root, json!({}), json!({ "rows": [1, 2] })).unwrap();
    assert_eq!(page.body, "<div><span>1</span><span>2</span></div>");
}

#[test]
fn nested_list_over_a_client_item_is_reactive_too() {
    let root = json!({ "type": "list", "props": { "items": "{{ groups }}", "as": "g" },
        "children": [{ "type": "list", "props": { "items": "{{ g.tags }}" },
            "children": [{ "type": "if", "props": { "when": "{{ item != 'x' }}" },
                "children": [{ "type": "text", "props": { "value": "{{ item }}" } }] }] }] });
    let page = render_with(&Engine::new(), root, json!({ "groups": [{ "tags": ["a", "x"] }] }), json!({})).unwrap();
    // Server-rendered items fold `g` and `item`, so they need no runtime of their own.
    assert!(page.body.ends_with("</template><div><span>a</span></div></div>"), "{}", page.body);
    // Inside the outer template the inner list depends on `g`, which only the browser knows.
    assert_eq!(page.body.matches("data-i-list").count(), 2, "{}", page.body);
    assert!(page.body.contains("data-i-if"), "{}", page.body);
}

// ---------------------------------------------------------------- include

fn load(engine: &Engine, doc: Json) -> Result<irender_core::Ir, irender_core::Diagnostics> {
    engine.load(&doc.to_string(), Format::Json)
}

#[test]
fn include_inlines_document_fragments_in_scope() {
    let doc = json!({ "version": 1,
        "fragments": { "row": { "type": "text", "props": { "tag": "li", "value": "{{ item.name }}" } } },
        "root": { "type": "list", "props": { "items": "{{ rows }}", "tag": "ul" },
                  "children": [{ "type": "include", "props": { "name": "row" } }] } });
    let engine = Engine::new();
    let ir = load(&engine, doc).unwrap();
    assert_eq!(ir.root.children[0].path, "root.children[0]@row");
    let page = engine.render(&ir, &json!({ "rows": [{ "name": "a" }, { "name": "b" }] })).unwrap();
    assert_eq!(page.body, "<ul><li>a</li><li>b</li></ul>");
}

#[test]
fn include_uses_the_engine_source_after_the_document() {
    let mut engine = Engine::new();
    engine.fragments = Some(std::sync::Arc::new(|name: &str| -> Result<Option<irender_core::NodeSpec>, String> {
        match name {
            "banner" => Ok(Some(serde_json::from_value(json!({ "type": "text", "props": { "value": "from host" } })).unwrap())),
            "broken" => Err("upstream timed out".into()),
            _ => Ok(None),
        }
    }));
    let doc = |name: &str| json!({ "version": 1, "root": { "type": "include", "props": { "name": name } } });
    let ir = load(&engine, doc("banner")).unwrap();
    assert_eq!(engine.render(&ir, &json!({})).unwrap().body, "<span>from host</span>");

    let err = load(&engine, doc("broken")).unwrap_err();
    assert_eq!(err.to_string(), "root.props.name: fragment `broken`: upstream timed out");
    let err = load(&engine, doc("nope")).unwrap_err();
    assert_eq!(err.to_string(), "root.props.name: unknown fragment `nope`");

    // The document's own fragment wins.
    let local = json!({ "version": 1, "fragments": { "banner": { "type": "text", "props": { "value": "local" } } },
        "root": { "type": "include", "props": { "name": "banner" } } });
    assert_eq!(engine.render(&load(&engine, local).unwrap(), &json!({})).unwrap().body, "<span>local</span>");
}

#[test]
fn include_errors_are_located() {
    let doc = json!({ "version": 1,
        "fragments": {
            "a": { "type": "element", "children": [{ "type": "include", "props": { "name": "b" } }] },
            "b": { "type": "include", "props": { "name": "a" } },
            "bad": { "type": "text", "props": { "value": 1, "size": 2 } }
        },
        "root": { "type": "element", "children": [
            { "type": "include", "props": { "name": "a" } },
            { "type": "include", "props": { "name": "bad" } },
            { "type": "include", "props": { "name": "{{ which }}" } },
            { "type": "include", "props": { "name": "bad" }, "style": { "color": "red" } }
        ] } });
    let err = load(&Engine::new(), doc).unwrap_err().to_string();
    let lines: Vec<_> = err.lines().collect();
    assert_eq!(lines, [
        "root.children[0]@a.children[0]@b.props.name: include cycle: a → b → a",
        "root.children[1]@bad.props.size: `text` has no prop `size` (props: value, tag; use `attrs` for HTML attributes)",
        "root.children[2].props.name: expected a fragment name (expressions are not allowed)",
        "root.children[3]: `include` only takes `props.name` and `props.with`; style the fragment's root instead",
        "root.children[3]@bad.props.size: `text` has no prop `size` (props: value, tag; use `attrs` for HTML attributes)",
    ], "{err}");
}

#[test]
fn fragments_cannot_nest_past_the_depth_limit() {
    // Each fragment is shallow, but they chain deeper than a single document could.
    let fragments: serde_json::Map<String, Json> = (0..80)
        .map(|i| {
            let next = json!({ "type": "include", "props": { "name": format!("f{}", i + 1) } });
            (format!("f{i}"), json!({ "type": "element", "children": [next] }))
        })
        .chain([("f80".to_string(), json!({ "type": "element" }))])
        .collect();
    let doc = json!({ "version": 1, "fragments": fragments, "root": { "type": "include", "props": { "name": "f0" } } });
    let err = load(&Engine::new(), doc).unwrap_err().to_string();
    assert!(err.ends_with(&format!("nodes nest more than {} levels deep", irender_core::MAX_DEPTH)), "{err}");
}

#[test]
fn fragment_dir_reads_json_and_yaml() {
    let dir = std::env::temp_dir().join(format!("irender-fragments-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("cards")).unwrap();
    std::fs::write(dir.join("header.json"), r#"{ "type": "text", "props": { "value": "Header" } }"#).unwrap();
    std::fs::write(dir.join("cards/promo.yaml"), "type: text\nprops:\n  value: Promo\n").unwrap();
    let mut engine = Engine::new();
    engine.fragments = Some(std::sync::Arc::new(irender_core::FragmentDir(dir.clone())));
    let doc = json!({ "version": 1, "root": { "type": "element", "children": [
        { "type": "include", "props": { "name": "header" } },
        { "type": "include", "props": { "name": "cards/promo" } } ] } });
    let body = engine.render(&load(&engine, doc).unwrap(), &json!({})).unwrap().body;
    let escape = load(&engine, json!({ "version": 1, "root": { "type": "include", "props": { "name": "../etc" } } }));
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(body, "<div><span>Header</span><span>Promo</span></div>");
    assert!(escape.unwrap_err().to_string().contains("invalid fragment name `../etc`"));
}

#[test]
fn documents_cannot_write_runtime_attributes() {
    let forged = r#"{"mount":[{"a":"submit","e":{"r":{"l":"https://evil.example"}},"m":"POST","b":{},"into":null}]}"#;
    let doc = json!({ "version": 1, "root": { "type": "element", "attrs": { "data-i-on": forged, "data-I-list": "{}" },
        "parts": {} } });
    for engine in [Engine::new(), Engine::untrusted()] {
        let err = engine.load(&doc.to_string(), Format::Json).unwrap_err().to_string();
        assert_eq!(err.lines().count(), 2, "{err}");
        assert!(err.contains("`data-i-on` is reserved for the iRender runtime"), "{err}");
    }
}

#[test]
fn aria_booleans_are_strings() {
    let root = json!({ "type": "element", "attrs": {
        "aria-expanded": "{{ open }}", "aria-hidden": false, "ARIA-busy": true, "hidden": false, "inert": true } });
    let page = render_with(&Engine::new(), root, json!({ "open": false }), json!({})).unwrap();
    assert!(page.body.starts_with(r#"<div aria-expanded="false" aria-hidden="false" ARIA-busy="true" inert data-i-b="#), "{}", page.body);
}

// ---------------------------------------------------------------- form controls

#[test]
fn select_renders_options_and_the_bound_value() {
    let root = json!({ "type": "select", "props": { "bind": "country", "placeholder": "Choose…",
        "options": ["in", { "value": "de", "label": "Germany" }, { "value": "us", "label": null, "disabled": true }, 3] } });
    let page = render_with(&Engine::new(), root, json!({ "country": "de" }), json!({})).unwrap();
    assert_eq!(
        strip_classes(&page.body),
        concat!(
            r#"<select data-i-bind="country">"#,
            r#"<option value="" disabled data-i-ph>Choose…</option>"#,
            r#"<option value="in">in</option>"#,
            r#"<option selected value="de">Germany</option>"#,
            r#"<option disabled value="us">us</option>"#,
            r#"<option value="3">3</option>"#,
            "</select>"
        )
    );
    // Nothing chosen yet: the placeholder is selected.
    let root = json!({ "type": "select", "props": { "bind": "n", "placeholder": "Pick", "options": [1, 2] } });
    let page = render_with(&Engine::new(), root, json!({ "n": null }), json!({})).unwrap();
    assert!(page.body.contains(r#"<option value="" disabled selected data-i-ph>Pick</option>"#), "{}", page.body);
}

#[test]
fn select_options_from_state_are_rebuilt_in_the_browser() {
    let root = json!({ "type": "select", "props": { "bind": "city", "options": "{{ cities }}" } });
    let page = render_with(&Engine::new(), root, json!({ "city": "b", "cities": ["a", "b"] }), json!({})).unwrap();
    assert_eq!(bindings(&page.body), [json!([{ "k": "opts", "v": { "r": { "p": ["cities"] } } }])]);
    assert!(page.body.contains(r#"<option selected value="b">b</option>"#), "{}", page.body);
    // Options from server data are fixed: no binding.
    let root = json!({ "type": "select", "props": { "value": 2, "options": "{{ sizes }}" } });
    let page = render_with(&Engine::new(), root, json!({}), json!({ "sizes": [1, 2] })).unwrap();
    assert!(!page.body.contains("data-i-b"), "{}", page.body);
    assert!(page.body.contains(r#"<option selected value="2">2</option>"#), "{}", page.body);
}

#[test]
fn select_options_must_be_an_array() {
    let root = json!({ "type": "select", "props": { "options": "{{ nope }}" } });
    assert!(render_with(&Engine::new(), root.clone(), json!({}), json!({ "nope": null })).is_ok());
    let err = render_with(&Engine::new(), root, json!({}), json!({ "nope": "x" })).unwrap_err();
    assert_eq!(err.to_string(), "root: `options` must evaluate to an array");
}

#[test]
fn textarea_holds_escaped_text() {
    let root = json!({ "type": "textarea", "props": { "bind": "notes", "rows": 5, "placeholder": "Notes" } });
    let page = render_with(&Engine::new(), root, json!({ "notes": "a < b\n</textarea>" }), json!({})).unwrap();
    assert_eq!(
        strip_classes(&page.body),
        "<textarea rows=\"5\" placeholder=\"Notes\" data-i-bind=\"notes\">a &lt; b\n&lt;/textarea&gt;</textarea>"
    );
    let page = render(json!({ "type": "textarea", "props": { "value": "hi" } }));
    assert_eq!(strip_classes(&page.body), r#"<textarea rows="3">hi</textarea>"#);
}

#[test]
fn bound_radios_and_checkboxes_render_checked() {
    let radio = |value: Json| json!({ "type": "input", "props": { "type": "radio", "bind": "size", "value": value } });
    let root = json!({ "type": "element", "children": [radio(json!("s")), radio(json!("m")), radio(json!(2)),
        { "type": "input", "props": { "type": "checkbox", "bind": "gift" } }] });
    let page = render_with(&Engine::new(), root, json!({ "size": "m", "gift": true }), json!({})).unwrap();
    assert_eq!(
        strip_classes(&page.body),
        concat!(
            r#"<div><input type="radio" value="s" name="size" data-i-bind="size">"#,
            r#"<input type="radio" value="m" checked name="size" data-i-bind="size">"#,
            r#"<input type="radio" value="2" name="size" data-i-bind="size">"#,
            r#"<input type="checkbox" checked data-i-bind="gift"></div>"#
        )
    );
    // Numbers in state match radios by their text, as in the browser.
    let page = render_with(&Engine::new(), radio(json!(2)), json!({ "size": 2 }), json!({})).unwrap();
    assert!(page.body.contains("checked"), "{}", page.body);
}

#[test]
fn a_bound_radio_needs_a_value() {
    let root = json!({ "type": "input", "props": { "type": "radio", "bind": "size" } });
    let err = render_with(&Engine::new(), root, json!({ "size": "m" }), json!({})).unwrap_err();
    assert_eq!(err.to_string(), "root: a bound radio needs a `value`");
}

// ---------------------------------------------------------------- include parameters

#[test]
fn include_parameters_substitute_into_the_fragment() {
    let doc = json!({ "version": 1, "state": { "qty": 2 },
        "fragments": {
            "row": { "type": "text", "props": { "tag": "li", "value": "{{ label }}: {{ who.name }} × {{ n }}" },
                     "class": ["{{ tone }}"] },
            "badge": { "type": "include", "props": { "name": "row",
                       "with": { "label": "[{{ label }}]", "who": "{{ who }}", "n": "{{ n * 10 }}", "tone": "{{ tone }}" } } }
        },
        "root": { "type": "list", "props": { "items": "{{ users }}", "as": "u", "tag": "ul" }, "children": [
            { "type": "include", "props": { "name": "row", "with": { "label": "User", "who": "{{ u }}", "n": "{{ qty }}", "tone": "plain" } } },
            { "type": "include", "props": { "name": "badge", "with": { "label": "Badge", "who": "{{ u }}", "n": 1, "tone": "{{ u.tone }}" } } }
        ] } });
    let engine = Engine::new();
    let ir = engine.load(&doc.to_string(), Format::Json).unwrap_or_else(|d| panic!("{d}"));
    assert!(ir.warnings.is_empty(), "{}", ir.warnings);
    let page = engine.render(&ir, &json!({ "users": [{ "name": "Ada", "tone": "gold" }] })).unwrap();
    let body = strip_classes(&page.body);
    // `qty` is state, so the first row still updates live; the second only uses data.
    assert!(body.starts_with(r#"<ul><li class="plain" data-i-b="#), "{body}");
    assert!(body.contains(">User: Ada × 2</li>"), "{body}");
    assert!(body.ends_with(r#"<li class="gold">[Badge]: Ada × 10</li></ul>"#), "{body}");
    let live = attr_json(&page.body, "data-i-b");
    assert_eq!(live[0]["v"]["t"][4], json!({ "p": ["qty"] }), "{live}");
}

#[test]
fn include_parameters_work_in_lists_the_browser_rebuilds() {
    let doc = json!({ "version": 1, "state": { "todos": [{ "title": "a" }] },
        "fragments": { "item": { "type": "text", "props": { "tag": "li", "value": "{{ title }}" } } },
        "root": { "type": "list", "props": { "items": "{{ todos }}", "as": "t", "tag": "ul" },
                  "children": [{ "type": "include", "props": { "name": "item", "with": { "title": "{{ t.title }}" } } }] } });
    let engine = Engine::new();
    let page = engine.render(&engine.load(&doc.to_string(), Format::Json).unwrap(), &json!({})).unwrap();
    // The template keeps `t.title` for the browser, exactly as if written inline.
    let tpl = &page.body[page.body.find("<template>").unwrap()..page.body.find("</template>").unwrap()];
    assert!(tpl.contains("&quot;p&quot;:[&quot;t&quot;,&quot;title&quot;]"), "{tpl}");
    assert!(page.body.ends_with("</template><li>a</li></ul>"), "{}", page.body);
}

#[test]
fn literal_arguments_are_checked_like_literals() {
    let doc = |with: Json, root_props: Json| json!({ "version": 1, "state": { "email": "" },
        "fragments": {
            "link": { "type": "element", "props": { "tag": "a" }, "attrs": { "href": "{{ url }}" },
                      "style": { "color": "{{ tone }}" } },
            "field": { "type": "input", "props": { "bind": "{{ key }}" } }
        },
        "root": { "type": "include", "props": { "name": root_props["name"].clone(), "with": with } } });
    let load = |engine: &Engine, d: Json| engine.load(&d.to_string(), Format::Json);

    // A token passed in becomes a static style; a URL stays a literal attribute.
    let ir = load(&Engine::new(), doc(json!({ "url": "/home", "tone": "$color.primary" }), json!({ "name": "link" }))).unwrap();
    let page = Engine::new().render(&ir, &json!({})).unwrap();
    assert_eq!(strip_classes(&page.body), r#"<a href="/home"></a>"#);
    assert!(page.css.contains("color: var(--color-primary);"), "{}", page.css);

    // The untrusted policy still rejects an unsafe URL passed as a parameter.
    let err = load(&Engine::untrusted(), doc(json!({ "url": "javascript:alert(1)", "tone": "red" }), json!({ "name": "link" })))
        .unwrap_err()
        .to_string();
    assert!(err.contains("root@link.attrs.href"), "{err}");

    // A state key passed in is still checked against `state`.
    assert!(load(&Engine::new(), doc(json!({ "key": "email" }), json!({ "name": "field" }))).is_ok());
    let err = load(&Engine::new(), doc(json!({ "key": "nope" }), json!({ "name": "field" }))).unwrap_err().to_string();
    assert_eq!(err, "root@field.props.bind: expected a key declared in `state`, got `nope`");
}

#[test]
fn include_parameter_errors_are_located() {
    let doc = json!({ "version": 1,
        "fragments": {
            "f": { "type": "text", "props": { "value": "{{ total.x }}" } },
            "g": { "type": "list", "props": { "items": "{{ rows }}" },
                   "children": [{ "type": "text", "props": { "value": "{{ item }}" } }] },
            "h": { "type": "text", "props": { "value": "fixed" } }
        },
        "root": { "type": "element", "children": [
            { "type": "include", "props": { "name": "f", "with": { "total": "{{ a + b }}" } } },
            { "type": "include", "props": { "name": "g", "with": { "item": 1, "rows": [] } } },
            { "type": "include", "props": { "name": "h", "with": ["not", "an", "object"] } },
            { "type": "include", "props": { "name": "h", "with": { "bad name": 1 } } }
        ] } });
    let err = Engine::new().load(&doc.to_string(), Format::Json).unwrap_err().to_string();
    let lines: Vec<_> = err.lines().collect();
    assert_eq!(lines, [
        "root.children[0]@f.props.value: `total` is a computed value, so `total.x` has no fields; pass a path or a literal",
        "root.children[1]@g.props: this list's `item` hides the fragment parameter `item`; rename one of them",
        "root.children[2].props.with: `with` must be an object of parameter names to values",
        "root.children[3].props.with.bad name: parameter names must be identifiers (letters, digits, `_`)",
    ], "{err}");
}

#[test]
fn unused_parameters_warn() {
    let doc = json!({ "version": 1,
        "fragments": { "h": { "type": "text", "props": { "value": "{{ title }}" } } },
        "root": { "type": "include", "props": { "name": "h", "with": { "title": "T", "titel": "typo" } } } });
    let ir = Engine::new().load(&doc.to_string(), Format::Json).unwrap();
    assert_eq!(ir.warnings.to_string(), "warning: root.props.with.titel: fragment `h` never uses parameter `titel`");
}

// ---------------------------------------------------------------- component behaviours

/// A component with a client behaviour that counts its starts and updates.
struct Counter;

const COUNTER_JS: &str = r#"(window.__irenderBehaviors ||= {}).counter = (el, api) => {
  el.dataset.starts = String(+(el.dataset.starts || 0) + 1);
  el.dataset.step = String(api.config.step);
  el.addEventListener("click", () => api.set("n", api.state.n + api.config.step));
  return { update(state) { el.textContent = "n=" + state.n; } };
};"#;

impl Component for Counter {
    fn name(&self) -> &'static str {
        "counter"
    }
    fn info(&self) -> ComponentInfo {
        const PROPS: &[PropDef] = &[PropDef::new("step", PropKind::Number, "")];
        ComponentInfo { doc: "", props: PROPS, parts: &[], children: false, open_props: false }
    }
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        cx.global_js("counter", COUNTER_JS);
        let step = cx.prop(node, "step").unwrap_or(json!(1));
        let mut el = cx.element("button", node);
        el.behavior("counter", json!({ "step": step }));
        cx.open(&el, node)?;
        cx.close("button");
        Ok(())
    }
}

pub fn behaviour_doc() -> Json {
    json!({ "type": "element", "children": [
        { "type": "counter", "props": { "step": 2 }, "attrs": { "id": "a" } },
        { "type": "counter", "attrs": { "id": "b" } },
        { "type": "if", "props": { "when": "{{ show }}" }, "children": [{ "type": "counter", "attrs": { "id": "c" } }] }
    ] })
}

#[test]
fn behaviours_mark_elements_and_ship_their_script_once() {
    let mut engine = Engine::new();
    engine.registry.register(Counter);
    let page = render_with(&engine, behaviour_doc(), json!({ "n": 0, "show": false }), json!({})).unwrap();
    assert!(page.interactive);
    assert_eq!(page.scripts, vec![COUNTER_JS], "one script however many counters");
    assert!(page.body.contains(r#"<button id="a" data-i-x="counter" data-i-xc="{&quot;step&quot;:2}">"#), "{}", page.body);
    let html = page.fragment();
    let (script, runtime) = (html.find("__irenderBehaviors").unwrap(), html.find("iRender browser runtime").unwrap());
    assert!(script < runtime, "behaviours register before the runtime starts them");
    // Documents can't mark elements themselves.
    let forged = json!({ "version": 1, "root": { "type": "element", "attrs": { "data-i-x": "counter" } } });
    assert!(engine.load(&forged.to_string(), Format::Json).unwrap_err().to_string().contains("reserved"));
}

struct EarlyUser;

impl Component for EarlyUser {
    fn name(&self) -> &'static str {
        "early-user"
    }
    fn info(&self) -> ComponentInfo {
        ComponentInfo { doc: "", props: &[], parts: &[], children: false, open_props: false }
    }
    fn render(&self, _node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError> {
        cx.early_js("scheme", "window.scheme = 1;");
        Ok(())
    }
}

#[test]
fn registry_early_scripts_go_on_every_page_once() {
    let mut engine = Engine::new();
    engine.registry.register(EarlyUser).early_js("scheme", "window.scheme = 1;").early_js("other", "window.other = 2;");
    // On a page that uses none of the registry's components…
    let page = render_with(&engine, json!({ "type": "text", "props": { "value": "x" } }), json!({}), json!({})).unwrap();
    assert_eq!(page.early_scripts, ["window.scheme = 1;", "window.other = 2;"]);
    let html = page.fragment();
    assert!(html.starts_with(&format!("<style>\n{}</style>\n<script>\nwindow.scheme = 1;</script>\n", page.css)), "{html}");
    // …and once on a page whose component adds the same one.
    let page = render_with(&engine, json!({ "type": "early-user" }), json!({}), json!({})).unwrap();
    assert_eq!(page.early_scripts, ["window.scheme = 1;", "window.other = 2;"]);
    // An engine without them adds none.
    assert!(render(json!({ "type": "text", "props": { "value": "x" } })).early_scripts.is_empty());
}
