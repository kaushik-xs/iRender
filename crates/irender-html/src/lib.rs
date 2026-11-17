//! Renders iRender documents to HTML on the server, with a small browser runtime
//! (~4 KB gzipped) for state, bindings and actions.
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
pub use render::{Element, RenderCx, RenderError};

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
        #[cfg(feature = "parallel")]
        let parallel = self.parallel_min_items;
        #[cfg(not(feature = "parallel"))]
        let parallel = None;
        let mut cx = RenderCx::new(&self.registry, &self.policy, &ir.theme, &ir.state, data, parallel);
        cx.render(&ir.root)?;
        let (body, css, interactive) = cx.finish();
        #[cfg(feature = "lightningcss")]
        let css = if self.minify {
            irender_style::minify(&css).map_err(|e| RenderError { path: String::new(), message: e })?
        } else {
            css
        };
        let state = Json::Object(ir.state.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
        Ok(Page { css, body, state, interactive })
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
}

impl Page {
    /// `<style>`, the root element, initial state and (if needed) the runtime.
    pub fn fragment(&self) -> String {
        let mut s = format!(
            "<style>\n{}</style>\n<div data-i-root style=\"display:contents\">{}",
            self.css, self.body
        );
        if self.interactive {
            s.push_str(&format!(
                "<script type=\"application/json\" data-i-state>{}</script>",
                escape::script_json(&self.state)
            ));
        }
        s.push_str("</div>\n");
        if self.interactive {
            s.push_str(&format!("<script>\n{RUNTIME_JS}</script>\n"));
        }
        s
    }

    pub fn document(&self, title: &str) -> String {
        format!(
            "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
             <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
             <title>{}</title>\n</head>\n<body>\n{}</body>\n</html>\n",
            escape::text(title),
            self.fragment()
        )
    }
}
