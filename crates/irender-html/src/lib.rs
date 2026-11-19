//! Renders iRender documents to HTML on the server, with a small browser runtime
//! (~4.5 KB gzipped) for state, bindings and actions.
//!
//! ```no_run
//! use irender_html::Engine;
//! use irender_core::Format;
//!
//! let engine = Engine::new();
//! let ir = engine.load(r#"{"version":1,"root":{"type":"text","props":{"value":"Hi"}}}"#, Format::Json)?;
//! let page = engine.render(&ir, &serde_json::json!({}))?;
//! println!("{}", page.document("Hello"));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! The renderer writes to a `String` and has no web-framework dependency; serve the
//! output from Axum, Actix, a static file or anything else.

mod components;
mod escape;
mod render;

pub use components::{Button, ElementComponent, If, Input, List, Select, Stack, Text, Textarea};
pub use render::{ClientList, Element, RenderCx, RenderError};

use irender_core::{Catalog, ComponentInfo, Diagnostics, Document, Format, Fragments, Ir, NoFragments, Node, Policy};
use indexmap::IndexMap;
use serde_json::Value as Json;
use std::sync::Arc;

// One loaded document can be rendered from many threads at once: `Engine::render`
// takes `&self` and `&Ir`, and both are thread-safe. This fails to compile if a
// change ever makes them not so.
const _: () = {
    const fn thread_safe<T: Send + Sync>() {}
    thread_safe::<Engine>();
    thread_safe::<Ir>();
    thread_safe::<Page>();
};

/// Code for kits that show it, as `window.__irenderCode`: a highlighter for json, yaml,
/// js, rust, python, bash, html, css and sql, copying to the clipboard, a JSON check that
/// says where and why, and `enhance(el)`, which highlights the code fences of rendered
/// Markdown and gives each a copy button. Include it with
/// `cx.global_js(CODE_JS_KEY, CODE_JS)`, so a page has it once whichever kit asks.
pub const CODE_JS: &str = include_str!("code.js");
/// The key [`CODE_JS`] goes in a page under.
pub const CODE_JS_KEY: &str = "irender-code";

/// The highlighter's token colours, nested in `block`: hues mixed toward the text colour,
/// so they hold up on a dark theme too. Each can be set with `--code-<kind>`
/// (`--code-string`, `--code-keyword`, …).
pub fn code_token_styles(block: irender_core::StyleBlock) -> irender_core::StyleBlock {
    let tone = |name: &str, hue: &str| format!("var(--code-{name}, color-mix(in srgb, {hue} 78%, var(--color-text, currentColor)))");
    [
        ("c", "var(--code-comment, var(--color-muted, #71717a))".to_string()),
        ("s", tone("string", "#16a34a")),
        ("n", tone("number", "#c2410c")),
        ("l", tone("literal", "#c2410c")),
        ("k", tone("keyword", "#9333ea")),
        ("p", tone("key", "#0369a1")),
        ("t", tone("tag", "#be123c")),
        ("a", tone("attribute", "#b45309")),
        ("f", tone("function", "#2563eb")),
        ("v", tone("variable", "#be123c")),
    ]
    .into_iter()
    .fold(block, |b, (t, c)| b.nest(format!("& [data-t={t}]"), irender_core::StyleBlock::new().set("color", c)))
    .nest("& [data-t=c]", irender_core::StyleBlock::from_pairs(&[("font-style", "italic")]))
}

/// The browser runtime. Included once per page when the document is interactive.
pub const RUNTIME_JS: &str = include_str!("runtime.js");

/// A component renders one node into HTML.
pub trait Component: Send + Sync {
    fn name(&self) -> &'static str;
    fn info(&self) -> ComponentInfo;
    fn render(&self, node: &Node, cx: &mut RenderCx<'_>) -> Result<(), RenderError>;
}

/// The set of components a document may use.
#[derive(Clone, Default)]
pub struct Registry {
    components: IndexMap<&'static str, Arc<dyn Component>>,
    /// Early scripts added to every page, whatever components it uses.
    early: IndexMap<&'static str, &'static str>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// `element`, `text`, `stack`, `button`, `input`, `select`, `textarea`, `list` and `if`.
    pub fn with_builtins() -> Self {
        let mut r = Self::new();
        r.register(ElementComponent)
            .register(Text)
            .register(Stack)
            .register(Button)
            .register(Input)
            .register(Select)
            .register(Textarea)
            .register(List)
            .register(If);
        r
    }

    /// Adds a component, replacing any existing one with the same name.
    pub fn register(&mut self, component: impl Component + 'static) -> &mut Self {
        self.components.insert(component.name(), Arc::new(component));
        self
    }

    /// Adds a script to every page rendered with this registry, run like
    /// [`RenderCx::early_js`]: right after the page's `<style>`, before its content is
    /// shown. For a kit whose setting applies to the whole page whatever is on it, such
    /// as a stored colour scheme. A component's `early_js` with the same name adds it
    /// only once.
    pub fn early_js(&mut self, name: &'static str, js: &'static str) -> &mut Self {
        debug_assert!(!js.to_ascii_lowercase().contains("</script"), "script `{name}` contains `</script`");
        self.early.insert(name, js);
        self
    }

    pub(crate) fn early_scripts(&self) -> &IndexMap<&'static str, &'static str> {
        &self.early
    }

    pub fn get(&self, name: &str) -> Option<&Arc<dyn Component>> {
        self.components.get(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Arc<dyn Component>> {
        self.components.values()
    }
}

impl Catalog for Registry {
    fn component(&self, name: &str) -> Option<ComponentInfo> {
        self.get(name).map(|c| c.info())
    }
}

/// Loads, validates and renders documents.
#[derive(Clone)]
pub struct Engine {
    pub registry: Registry,
    pub policy: Policy,
    /// Minify the generated CSS (needs the `lightningcss` feature).
    pub minify: bool,
    /// Treat CSS validation warnings (unknown property, invalid value) as errors.
    pub strict_css: bool,
    /// Render lists with at least this many items on several threads. `None` (the
    /// default) renders everything on the calling thread.
    ///
    /// This lowers the latency of large pages, but uses more total CPU. A busy server
    /// already uses every core across requests, so enable it when large pages are rare
    /// and their latency matters. Output is identical either way.
    #[cfg(feature = "parallel")]
    pub parallel_min_items: Option<usize>,
    /// Where `include` nodes find fragments the document doesn't define itself, e.g.
    /// [`FragmentDir`](irender_core::FragmentDir) or your own [`Fragments`] that builds
    /// them from an API. Includes are expanded when a document is loaded.
    pub fragments: Option<Arc<dyn Fragments>>,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    /// Built-in components and a trusted policy.
    pub fn new() -> Self {
        Self {
            registry: Registry::with_builtins(),
            policy: Policy::trusted(),
            minify: false,
            strict_css: false,
            #[cfg(feature = "parallel")]
            parallel_min_items: None,
            fragments: None,
        }
    }

    /// Built-in components and [`Policy::untrusted`], for user- or LLM-written documents.
    pub fn untrusted() -> Self {
        Self { policy: Policy::untrusted(), ..Self::new() }
    }

    pub fn load(&self, src: &str, format: Format) -> Result<Ir, Diagnostics> {
        self.resolve(&irender_core::parse(src, format)?)
    }

    /// Validates a parsed document. Non-fatal findings end up in [`Ir::warnings`].
    pub fn resolve(&self, doc: &Document) -> Result<Ir, Diagnostics> {
        let fragments = self.fragments.as_deref().unwrap_or(&NoFragments);
        #[allow(unused_mut)]
        let mut ir = irender_core::resolve_with(doc, &self.registry, &self.policy, fragments)?;
        #[cfg(feature = "lightningcss")]
        {
            let css = irender_style::validate(&ir);
            let css = if self.strict_css { css.strict() } else { css };
            if css.has_errors() {
                return Err(css);
            }
            ir.warnings.extend(css);
        }
        Ok(ir)
    }

    /// Renders the document. `data` is read-only server data available to expressions
    /// alongside `state`.
    pub fn render(&self, ir: &Ir, data: &Json) -> Result<Page, RenderError> {
        self.render_page(ir, data, true)
    }

    /// Renders `ir` as a piece of another iRender page, such as the UI in a chat
    /// message. Its styles leave out the theme's variables, so it takes the host page's
    /// theme. Insert its [`Page::embedded`] markup into the page and start it with
    /// `window.__irender.mount(element)`.
    pub fn render_embedded(&self, ir: &Ir, data: &Json) -> Result<Page, RenderError> {
        self.render_page(ir, data, false)
    }

    fn render_page(&self, ir: &Ir, data: &Json, theme: bool) -> Result<Page, RenderError> {
        #[cfg(feature = "parallel")]
        let parallel = self.parallel_min_items;
        #[cfg(not(feature = "parallel"))]
        let parallel = None;
        let mut cx = RenderCx::new(&self.registry, &self.policy, &ir.theme, &ir.labels, &ir.state, data, parallel);
        cx.render(&ir.root)?;
        let (body, css, interactive, scripts, early_scripts) = cx.finish(theme);
        #[cfg(feature = "lightningcss")]
        let css = if self.minify {
            irender_style::minify(&css).map_err(|e| RenderError { path: String::new(), message: e })?
        } else {
            css
        };
        let state = Json::Object(ir.state.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
        let labels = (!ir.labels.is_empty() || ir.labels.lang.is_some()).then(|| ir.labels.to_json());
        Ok(Page { css, body, state, interactive, scripts, early_scripts, labels, lang: ir.labels.lang.clone() })
    }
}

/// Rendered output. Use [`Page::document`] for a full page or [`Page::fragment`] to
/// embed it in your own layout.
#[derive(Debug, Clone)]
pub struct Page {
    pub css: String,
    pub body: String,
    pub state: Json,
    /// Whether the page has bindings or actions and therefore needs the runtime.
    pub interactive: bool,
    /// Client behaviour scripts the page's components need (see
    /// [`RenderCx::global_js`]), included before the runtime.
    pub scripts: Vec<&'static str>,
    /// Scripts that run before the content is shown (see [`RenderCx::early_js`]),
    /// written right after the `<style>`.
    pub early_scripts: Vec<&'static str>,
    /// The document's `lang` and `labels` for the browser (`{ lang, labels }`), when it
    /// has either.
    pub labels: Option<Json>,
    /// The document's language: [`Page::document`]'s `<html lang>`.
    pub lang: Option<String>,
}

impl Page {
    /// `<style>`, early scripts, the root element, initial state and (if needed) the
    /// runtime.
    pub fn fragment(&self) -> String {
        let mut s = format!("<style>\n{}</style>\n", self.css);
        for js in &self.early_scripts {
            s.push_str(&format!("<script>\n{js}</script>\n"));
        }
        s.push_str(&format!("<div data-i-root style=\"display:contents\">{}", self.body));
        if self.interactive {
            s.push_str(&format!(
                "<script type=\"application/json\" data-i-state>{}</script>",
                escape::script_json(&self.state)
            ));
            s.push_str(&self.labels_json());
        }
        s.push_str("</div>\n");
        for js in &self.scripts {
            s.push_str(&format!("<script>\n{js}</script>\n"));
        }
        if self.interactive {
            s.push_str(&format!("<script>\n{RUNTIME_JS}</script>\n"));
        }
        s
    }

    /// Markup to insert into a page that already runs iRender: the styles, the
    /// behaviour scripts and a root with its own state, without the runtime or early
    /// scripts. Its `<script>`s only run when added as new script elements, which
    /// `window.__irender.mount` doesn't do; see `Engine::render_embedded`.
    pub fn embedded(&self) -> String {
        let mut s = format!("<style>\n{}</style>\n<div data-i-root style=\"display:contents\">{}", self.css, self.body);
        if self.interactive {
            s.push_str(&format!("<script type=\"application/json\" data-i-state>{}</script>", escape::script_json(&self.state)));
            s.push_str(&self.labels_json());
        }
        s.push_str("</div>\n");
        for js in &self.scripts {
            s.push_str(&format!("<script>\n{js}</script>\n"));
        }
        s
    }

    /// The labels for the runtime, beside the state.
    fn labels_json(&self) -> String {
        self.labels.as_ref().map_or_else(String::new, |l| format!("<script type=\"application/json\" data-i-labels>{}</script>", escape::script_json(l)))
    }

    /// A full HTML page around [`Page::fragment`]. The `body` loses the browser's
    /// default margin and takes the theme's page colour (`color.background`, else
    /// `color.surface`), so dark mode reaches the edges of the window.
    pub fn document(&self, title: &str) -> String {
        format!(
            "<!doctype html>\n<html lang=\"{}\">\n<head>\n<meta charset=\"utf-8\">\n\
             <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
             <title>{}</title>\n<style>body{{margin:0;background:var(--color-background,var(--color-surface))}}</style>\n\
             </head>\n<body>\n{}</body>\n</html>\n",
            escape::text(self.lang.as_deref().unwrap_or("en")),
            escape::text(title),
            self.fragment()
        )
    }
}
