use irender_core::Format;
use irender_html::{Engine, Page, RenderError};
use serde_json::{Value as Json, json};

fn render_with(root: Json, state: Json) -> Result<Page, RenderError> {
    let mut engine = Engine::new();
    irender_ui_kit::register(&mut engine.registry);
    irender_markdown_kit::register(&mut engine.registry);
    let mut doc = irender_core::parse(&json!({ "version": 1, "state": state, "root": root }).to_string(), Format::Json)
        .unwrap_or_else(|d| panic!("{d}"));
    irender_ui_kit::apply_theme(&mut doc);
    let ir = engine.resolve(&doc).unwrap_or_else(|d| panic!("{d}"));
    engine.render(&ir, &json!({}))
}

/// Removes generated class names so assertions don't depend on hashes.
fn plain(html: &str) -> String {
    let mut out = String::new();
    let mut rest = html;
    while let Some(i) = rest.find(" class=\"i-") {
        out.push_str(&rest[..i]);
        let after = &rest[i + 8..];
        rest = &after[after.find('"').unwrap() + 1..];
    }
    out + rest
}

#[test]
fn markdown_renders_on_the_server_and_follows_state() {
    let page = render_with(json!({ "type": "ui-markdown", "props": { "text": "# Hi\n\n**bold** <b>raw</b>" } }), json!({})).unwrap();
    assert_eq!(plain(&page.body), r#"<div data-markdown><h1>Hi</h1>
<p><strong>bold</strong> &lt;b&gt;raw&lt;/b&gt;</p></div>"#);
    assert!(!page.interactive, "fixed text needs no script");

    let page = render_with(json!({ "type": "ui-markdown", "props": { "text": "{{ note }}" } }), json!({ "note": "_it_" })).unwrap();
    let body = plain(&page.body);
    assert!(body.starts_with(r#"<div data-markdown data-i-x="ui-markdown""#), "{body}");
    assert!(body.contains("><p><em>it</em></p></div>"), "the text it starts with: {body}");
    assert!(page.scripts.iter().any(|s| s.contains("__irenderMarkdown")), "the renderer comes along");
}

#[test]
fn editor_structure() {
    let page = render_with(
        json!({ "type": "ui-markdown-editor", "props": { "bind": "doc", "mode": "tabs", "maxLength": 100, "placeholder": "Write…" } }),
        json!({ "doc": "- one" }),
    )
    .unwrap();
    let body = plain(&page.body);
    assert!(body.contains(r#"role="toolbar" aria-label="Formatting""#), "{body}");
    assert!(body.contains(r#"<button type="button" data-md="bold" aria-label="Bold" title="Bold (⌘B)">B</button>"#), "{body}");
    assert!(body.contains(r#"<button type="button" role="tab" data-md-view="preview" aria-selected="false">Preview</button>"#), "{body}");
    // The text area is the built-in one, bound to state.
    assert!(body.contains(r#"<textarea aria-label="Markdown" spellcheck="true" maxlength="100" rows="10" placeholder="Write…" data-i-bind="doc">- one</textarea>"#), "{body}");
    // The preview starts with what's there.
    assert!(body.contains("<ul>\n<li>one</li>\n</ul>"), "{body}");
    assert!(body.contains(r#"<span data-md-count aria-live="polite">5/100</span>"#), "{body}");

    let split = plain(&render_with(json!({ "type": "ui-markdown-editor", "props": { "bind": "doc", "mode": "split", "toolbar": false } }), json!({ "doc": "" })).unwrap().body);
    assert!(!split.contains("role=\"toolbar\"") && !split.contains("data-md-view"), "no toolbar and no tabs: {split}");
    assert!(split.contains(r#"data-mode="split""#) && split.contains("Nothing to preview"), "{split}");

    // By default the text is edited formatted, with its Markdown a tab away.
    let rich = plain(&render_with(json!({ "type": "ui-markdown-editor", "props": { "bind": "doc", "placeholder": "Write…" } }), json!({ "doc": "**hi**" })).unwrap().body);
    assert!(rich.contains(r#"data-mode="rich" data-view="rich""#), "{rich}");
    assert!(rich.contains(r#"<button type="button" role="tab" data-md-view="rich" aria-selected="true">Rich text</button>"#), "{rich}");
    assert!(rich.contains(r#"<button type="button" role="tab" data-md-view="source" aria-selected="false">Markdown</button>"#), "{rich}");
    assert!(
        rich.contains(r#"data-md-rich contenteditable="true" role="textbox" aria-multiline="true" aria-label="Text" spellcheck="true" data-placeholder="Write…"><p><strong>hi</strong></p></div>"#),
        "{rich}"
    );
    assert!(rich.contains(r#"<div data-md-linkbar hidden><input type="url" aria-label="Link address""#), "{rich}");
    // Table, Image and Rule, and the bar an image's address is typed in.
    assert!(rich.contains(r#"data-md="table" aria-label="Table""#) && rich.contains(r#"data-md="rule" aria-label="Horizontal rule""#), "{rich}");
    assert!(rich.contains(r#"<div data-md-imagebar hidden><input type="url" data-md-image-src aria-label="Image address""#), "{rich}");
    assert!(!rich.contains("data-md-preview"), "{rich}");
    let empty = plain(&render_with(json!({ "type": "ui-markdown-editor", "props": { "bind": "doc", "disabled": true } }), json!({ "doc": "" })).unwrap().body);
    assert!(empty.contains(r#"contenteditable="false""#) && empty.contains(r#"data-empty><p><br></p></div>"#), "{empty}");

    // Without `bind` the document doesn't load.
    let mut engine = Engine::new();
    irender_markdown_kit::register(&mut engine.registry);
    let err = engine.load(r#"{ "version": 1, "root": { "type": "ui-markdown-editor" } }"#, Format::Json).unwrap_err();
    assert!(err.to_string().contains("missing required prop `bind`"), "{err}");
}

#[test]
fn code_fences_bring_the_highlighter() {
    // Fixed Markdown with a fence: the behaviour (with no text to follow) and the code script.
    let page = render_with(json!({ "type": "ui-markdown", "props": { "text": "```rust\nfn main() {}\n```" } }), json!({})).unwrap();
    let body = plain(&page.body);
    assert!(body.contains(r#"data-i-x="ui-markdown" data-i-xc="{&quot;text&quot;:null}""#), "{body}");
    assert!(body.contains(r#"<pre><code class="language-rust">fn main() {}</code></pre>"#), "drawn plain on the server: {body}");
    assert!(page.scripts.iter().any(|s| s.contains("__irenderCode")), "the highlighter comes along");
    assert!(!page.scripts.iter().any(|s| s.contains("window.__irenderMarkdown = ")), "the renderer doesn't: nothing to re-render");
    // From state: both, since a fence may come later.
    let page = render_with(json!({ "type": "ui-markdown", "props": { "text": "{{ note }}" } }), json!({ "note": "_it_" })).unwrap();
    assert!(page.scripts.iter().any(|s| s.contains("__irenderCode")) && page.scripts.iter().any(|s| s.contains("window.__irenderMarkdown = ")));
}
