use crate::{Registry, escape};
use irender_core::{
    Action, Node, Policy, StyleBlock, StyleValue, Template, Theme, Val,
    expr::{self, Expr, Part, stringify, write_json_str},
    style,
};
use irender_style::{Layer, StyleCompiler};
use indexmap::IndexMap;
use serde_json::{Value as Json, json};
use std::{borrow::Cow, fmt::Write};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{path}: {message}")]
pub struct RenderError {
    pub path: String,
    pub message: String,
}

impl RenderError {
    pub fn new(node: &Node, message: impl Into<String>) -> Self {
        Self { path: node.path.clone(), message: message.into() }
    }
}

const VOID_TAGS: &[&str] =
    &["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr"];

/// An HTML element under construction. Start one with [`RenderCx::element`] (which
/// applies the node's `attrs`, `class`, `style` and `on`) or [`RenderCx::part`].
///
/// It borrows those from the node rather than copying them, so building one is cheap.
#[derive(Debug, Clone)]
pub struct Element<'n> {
    tag: String,
    /// The document's attributes. They always win over `attrs`.
    node_attrs: Option<&'n IndexMap<String, Val>>,
    /// Attributes set by the component.
    attrs: Vec<(String, Val)>,
    node_classes: &'n [Val],
    classes: Vec<Val>,
    styles: Vec<(Cow<'n, StyleBlock>, Layer)>,
    text: Option<Cow<'n, Val>>,
    on: Option<&'n IndexMap<String, Vec<Action>>>,
    bind: Option<String>,
    /// A list the browser rebuilds.
    list: Option<ClientList<'n>>,
    /// A `<select>`'s options, rebuilt in the browser when they use state.
    options: Option<&'n Val>,
    /// A client behaviour to start on this element, and its configuration.
    behavior: Option<(&'static str, Json)>,
}

impl<'n> Element<'n> {
    pub fn new(tag: impl Into<String>) -> Self {
        Self {
            tag: tag.into(),
            node_attrs: None,
            attrs: Vec::new(),
            node_classes: &[],
            classes: Vec::new(),
            styles: Vec::new(),
            text: None,
            on: None,
            bind: None,
            list: None,
            options: None,
            behavior: None,
        }
    }

    pub fn tag(&self) -> &str {
        &self.tag
    }

    pub fn is_void(&self) -> bool {
        VOID_TAGS.contains(&self.tag.to_ascii_lowercase().as_str())
    }

    fn document_sets(&self, name: &str) -> bool {
        self.node_attrs.is_some_and(|a| a.contains_key(name))
    }

    /// Sets an attribute unless the document already set it: document `attrs` always win.
    pub fn attr(&mut self, name: &str, value: impl Into<Val>) -> &mut Self {
        if !self.document_sets(name) && !self.attrs.iter().any(|(n, _)| n == name) {
            self.attrs.push((name.to_string(), value.into()));
        }
        self
    }

    /// Adds component default styles. Document styles always override these.
    pub fn base_style(&mut self, block: StyleBlock) -> &mut Self {
        self.styles.push((Cow::Owned(block), Layer::Base));
        self
    }

    /// Like [`Element::base_style`] for a block that outlives the element, e.g. a static.
    pub fn base_style_ref(&mut self, block: &'n StyleBlock) -> &mut Self {
        self.styles.push((Cow::Borrowed(block), Layer::Base));
        self
    }

    pub fn class(&mut self, class: impl Into<Val>) -> &mut Self {
        self.classes.push(class.into());
        self
    }

    /// Text content, escaped. Reactive templates update in place.
    pub fn text(&mut self, value: impl Into<Cow<'n, Val>>) -> &mut Self {
        self.text = Some(value.into());
        self
    }

    /// Two-way binds a form control's value to a `state` key.
    pub fn bind(&mut self, key: impl Into<String>) -> &mut Self {
        self.bind = Some(key.into());
        self
    }

    /// Marks the element as a list that the browser rebuilds from the `<template>`
    /// written by [`RenderCx::item_template`] (or [`RenderCx::item_template_with`])
    /// whenever `items` changes. The template must be the element's first child,
    /// followed only by the server-rendered items.
    pub fn list(&mut self, list: ClientList<'n>) -> &mut Self {
        self.list = Some(list);
        self
    }

    /// Rebuilds a `<select>`'s `<option>`s in the browser whenever `options` changes.
    /// Only needed when they use state; the server renders the initial options.
    pub(crate) fn options(&mut self, options: &'n Val) -> &mut Self {
        self.options = Some(options);
        self
    }

    /// Starts the client behaviour `name` on this element in the browser, passing it
    /// `config`. The behaviour's script must be added with [`RenderCx::global_js`].
    pub fn behavior(&mut self, name: &'static str, config: Json) -> &mut Self {
        self.behavior = Some((name, config));
        self
    }
}

/// A list the browser rebuilds (see [`Element::list`]).
#[derive(Debug, Clone)]
pub struct ClientList<'n> {
    pub items: &'n Val,
    /// The names of the current item and its index.
    pub item: String,
    pub index: String,
    /// Each item's key, in client form (see [`RenderCx::item_val_js`]). Items whose
    /// key survives a change keep their nodes.
    pub key: Option<Json>,
}

/// A name in scope while rendering, e.g. a list's `item`.
#[derive(Debug, Clone)]
struct Scope {
    name: String,
    value: Json,
    /// Only known in the browser (an item of a list the browser rebuilds). Expressions
    /// that use it are kept for the runtime instead of being folded.
    client: bool,
    /// The index of a list the browser rebuilds (see [`RenderCx::client_key`]).
    index: bool,
}

/// Render state passed to components.
pub struct RenderCx<'a> {
    registry: &'a Registry,
    policy: &'a Policy,
    theme: &'a Theme,
    state: &'a IndexMap<String, Json>,
    data: &'a Json,
    scopes: Vec<Scope>,
    /// Values components pass to the components inside them (see [`RenderCx::with_context`]).
    contexts: Vec<(&'static str, Option<Val>)>,
    styles: StyleCompiler<'a>,
    out: String,
    interactive: bool,
    /// Client behaviour scripts, by name, in first-use order.
    scripts: IndexMap<&'static str, &'static str>,
    /// Scripts that run before the content is shown (see [`RenderCx::early_js`]).
    early: IndexMap<&'static str, &'static str>,
    /// Lists with at least this many items render in parallel.
    #[cfg_attr(not(feature = "parallel"), allow(dead_code))]
    parallel_min_items: Option<usize>,
}

impl<'a> RenderCx<'a> {
    pub(crate) fn new(
        registry: &'a Registry,
        policy: &'a Policy,
        theme: &'a Theme,
        state: &'a IndexMap<String, Json>,
        data: &'a Json,
        parallel_min_items: Option<usize>,
    ) -> Self {
        Self {
            registry,
            policy,
            theme,
            state,
            data,
            scopes: Vec::new(),
            contexts: Vec::new(),
            styles: StyleCompiler::new(theme),
            out: String::new(),
            interactive: false,
            scripts: IndexMap::new(),
            early: registry.early_scripts().clone(),
            parallel_min_items,
        }
    }

    /// The output: markup, stylesheet (with the theme's variables when `theme`),
    /// whether it's interactive, and its scripts.
    pub(crate) fn finish(self, theme: bool) -> (String, String, bool, Vec<&'static str>, Vec<&'static str>) {
        let (scripts, early) = (self.scripts.into_values().collect(), self.early.into_values().collect());
        let css = if theme { self.styles.css() } else { self.styles.rules_css() };
        (self.out, css, self.interactive, scripts, early)
    }

    pub fn theme(&self) -> &Theme {
        self.theme
    }

    /// What the document is allowed to do, for components that pass document
    /// content to client code.
    pub fn policy(&self) -> &Policy {
        self.policy
    }

    /// Resolves a path: loop scopes first, then `state`, then server `data`. Names
    /// only known in the browser are null.
    pub fn lookup(&self, path: &[String]) -> Json {
        let Some((root, rest)) = path.split_first() else { return Json::Null };
        let base = if let Some(s) = self.scope(root) {
            &s.value
        } else if let Some(v) = self.state.get(root) {
            v
        } else if let Some(v) = self.data.get(root) {
            v
        } else {
            return Json::Null;
        };
        expr::lookup(base, rest)
    }

    fn scope(&self, name: &str) -> Option<&Scope> {
        self.scopes.iter().rev().find(|s| s.name == name)
    }

    fn is_reactive_root(&self, root: &str) -> bool {
        match self.scope(root) {
            Some(s) => s.client,
            None => self.state.contains_key(root),
        }
    }

    /// Whether `v` can change in the browser: it reads `state`, or an item of a list
    /// the browser rebuilds.
    pub fn is_reactive(&self, v: &Val) -> bool {
        match v {
            Val::Tpl(t) => self.is_reactive_tpl(t),
            Val::Lit(_) => false,
        }
    }

    fn is_reactive_tpl(&self, t: &Template) -> bool {
        t.references(&|r| self.is_reactive_root(r))
    }

    /// Writes the client-side form of `v`, with server data folded in.
    fn write_val_js(&self, out: &mut String, v: &Val) {
        let _ = v.write_js(out, &|r| self.is_reactive_root(r), &|p| self.lookup(p));
    }

    fn write_tpl_js(&self, out: &mut String, t: &Template) {
        let _ = t.write_js(out, &|r| self.is_reactive_root(r), &|p| self.lookup(p));
    }

    /// The client-side form of `v`, with server data folded in, for a behaviour to
    /// evaluate with `window.__irender.evaluate(js, state)`.
    pub fn val_js(&self, v: &Val) -> Json {
        v.to_js(&|r| self.is_reactive_root(r), &|p| self.lookup(p))
    }

    /// The value as of render time.
    pub fn eval(&self, v: &Val) -> Json {
        v.eval(&|p| self.lookup(p))
    }

    pub fn prop(&self, node: &Node, name: &str) -> Option<Json> {
        node.props.get(name).map(|v| self.eval(v))
    }

    /// A prop as text; `None` when missing or null.
    pub fn prop_string(&self, node: &Node, name: &str) -> Option<String> {
        self.prop(node, name).filter(|v| !v.is_null()).map(|v| stringify(&v))
    }

    /// Converts a prop into a style value: numbers gain `px`, `$tokens` resolve,
    /// templates become dynamic. `None` for null.
    pub fn style_value(&self, node: &Node, property: &str, v: &Val) -> Result<Option<StyleValue>, RenderError> {
        let value = match v {
            Val::Tpl(t) => StyleValue::Dynamic { tpl: t.clone(), px: !style::is_unitless(property) },
            Val::Lit(j) => {
                let Some(css) = style::value_to_css(property, j) else { return Ok(None) };
                let css = self.theme.substitute(&css).map_err(|e| RenderError::new(node, e))?;
                self.policy.check_css_value(&css).map_err(|e| RenderError::new(node, e))?;
                StyleValue::Static(css)
            }
        };
        Ok(Some(value))
    }

    /// An element carrying the node's `attrs`, `class`, `style` and `on`.
    pub fn element<'n>(&self, tag: &str, node: &'n Node) -> Element<'n> {
        let mut el = Element::new(tag);
        el.node_attrs = Some(&node.attrs);
        el.node_classes = &node.class;
        el.styles.push((Cow::Borrowed(&node.style), Layer::Instance));
        el.on = Some(&node.on);
        el
    }

    /// An inner element carrying the overrides from the node's `parts.<name>`.
    pub fn part<'n>(&self, tag: &str, node: &'n Node, name: &str) -> Element<'n> {
        let mut el = Element::new(tag);
        if let Some(p) = node.parts.get(name) {
            el.node_attrs = Some(&p.attrs);
            el.node_classes = &p.class;
            el.styles.push((Cow::Borrowed(&p.style), Layer::Instance));
        }
        el
    }

    /// Writes the start tag (and text content, if any).
    pub fn open(&mut self, el: &Element<'_>, node: &Node) -> Result<(), RenderError> {
        self.policy.check_tag(&el.tag).map_err(|e| RenderError::new(node, e))?;
        // Raw JSON of this element's bindings, comma-separated.
        let mut bindings = String::new();
        let mut classes = String::new();
        let mut inline = String::new();
        let sep = |s: &mut String, with: &str| {
            if !s.is_empty() {
                s.push_str(with);
            }
        };

        for (block, layer) in &el.styles {
            let Some(compiled) = self.styles.compile(block, *layer) else { continue };
            sep(&mut classes, " ");
            classes.push_str(&compiled.class);
            for var in &compiled.vars {
                let current = var.tpl.eval(&|p| self.lookup(p));
                sep(&mut inline, "; ");
                // Every var is set on its element, `initial` when there's no value, so
                // none is ever inherited from an ancestor's var of the same name.
                match css_var_value(&current, var.px) {
                    Some(css) if is_css_wide_keyword(&css) => match &var.property {
                        Some(p) => {
                            let _ = write!(inline, "{}: initial; {p}: {}", var.name, css.trim());
                        }
                        None => {
                            let _ = write!(inline, "{}: initial", var.name);
                        }
                    },
                    Some(css) => {
                        let _ = write!(inline, "{}: {css}", var.name);
                    }
                    None => {
                        let _ = write!(inline, "{}: initial", var.name);
                    }
                }
                if self.is_reactive_tpl(&var.tpl) {
                    sep(&mut bindings, ",");
                    let _ = write!(bindings, "{{\"k\":\"var\",\"n\":\"{}\",\"px\":{},", var.name, var.px);
                    if let Some(p) = &var.property {
                        let _ = write!(bindings, "\"p\":\"{p}\",");
                    }
                    bindings.push_str("\"v\":");
                    self.write_tpl_js(&mut bindings, &var.tpl);
                    bindings.push('}');
                }
            }
        }
        for c in el.node_classes.iter().chain(&el.classes) {
            let current = stringify(&self.eval(c));
            if self.is_reactive(c) {
                sep(&mut bindings, ",");
                bindings.push_str("{\"k\":\"class\",\"i\":");
                let _ = write_json_str(&mut bindings, &current);
                bindings.push_str(",\"v\":");
                self.write_val_js(&mut bindings, c);
                bindings.push('}');
            }
            let current = current.trim();
            if !current.is_empty() {
                sep(&mut classes, " ");
                classes.push_str(current);
            }
        }

        let _ = write!(self.out, "<{}", el.tag);
        if !classes.is_empty() {
            self.out.push_str(" class=\"");
            escape::push_attr(&mut self.out, &classes);
            self.out.push('"');
        }
        let attrs = el.node_attrs.into_iter().flatten().chain(el.attrs.iter().map(|(n, v)| (n, v)));
        if !inline.is_empty() {
            if let Some((_, user)) = attrs.clone().find(|(n, _)| *n == "style") {
                let user = stringify(&self.eval(user));
                if !user.is_empty() {
                    inline.push_str("; ");
                    inline.push_str(&user);
                }
            }
            self.out.push_str(" style=\"");
            escape::push_attr(&mut self.out, &inline);
            self.out.push('"');
        }
        for (name, v) in attrs {
            if name == "style" && !inline.is_empty() {
                continue;
            }
            if self.is_reactive(v) {
                sep(&mut bindings, ",");
                bindings.push_str("{\"k\":\"attr\",\"n\":");
                let _ = write_json_str(&mut bindings, name);
                bindings.push_str(",\"v\":");
                self.write_val_js(&mut bindings, v);
                bindings.push('}');
            }
            let value = self.eval(v);
            self.write_attr(name, &value);
        }
        if let Some(on) = el.on.filter(|on| !on.is_empty()) {
            let mut js = String::from("{");
            for (i, (event, actions)) in on.iter().enumerate() {
                if i > 0 {
                    js.push(',');
                }
                let _ = write_json_str(&mut js, event);
                js.push_str(":[");
                for (j, action) in actions.iter().enumerate() {
                    if j > 0 {
                        js.push(',');
                    }
                    self.write_action_js(&mut js, action);
                }
                js.push(']');
            }
            js.push('}');
            self.out.push_str(" data-i-on=\"");
            escape::push_attr(&mut self.out, &js);
            self.out.push('"');
            self.interactive = true;
        }
        if let Some(key) = &el.bind {
            self.out.push_str(" data-i-bind=\"");
            escape::push_attr(&mut self.out, key);
            self.out.push('"');
            self.interactive = true;
        }
        if let Some(list) = &el.list {
            let mut js = String::from("{\"v\":");
            self.write_val_js(&mut js, list.items);
            js.push_str(",\"as\":");
            let _ = write_json_str(&mut js, &list.item);
            js.push_str(",\"ix\":");
            let _ = write_json_str(&mut js, &list.index);
            if let Some(key) = &list.key {
                let _ = write!(js, ",\"k\":{key}");
            }
            js.push('}');
            self.out.push_str(" data-i-list=\"");
            escape::push_attr(&mut self.out, &js);
            self.out.push('"');
            self.interactive = true;
        }
        let text = el.text.as_deref().map(|t| {
            if self.is_reactive(t) {
                sep(&mut bindings, ",");
                bindings.push_str("{\"k\":\"text\",\"v\":");
                self.write_val_js(&mut bindings, t);
                bindings.push('}');
            }
            self.eval(t)
        });
        if let Some(options) = el.options.filter(|o| self.is_reactive(o)) {
            sep(&mut bindings, ",");
            bindings.push_str("{\"k\":\"opts\",\"v\":");
            self.write_val_js(&mut bindings, options);
            bindings.push('}');
        }
        if let Some((name, config)) = &el.behavior {
            self.out.push_str(" data-i-x=\"");
            escape::push_attr(&mut self.out, name);
            self.out.push_str("\" data-i-xc=\"");
            escape::push_attr(&mut self.out, &config.to_string());
            self.out.push('"');
            self.interactive = true;
        }
        if !bindings.is_empty() {
            self.out.push_str(" data-i-b=\"[");
            escape::push_attr(&mut self.out, &bindings);
            self.out.push_str("]\"");
            self.interactive = true;
        }
        self.out.push('>');
        if let Some(t) = text
            && !el.is_void()
        {
            match &t {
                Json::String(s) => escape::push_text(&mut self.out, s),
                other => escape::push_text(&mut self.out, &stringify(other)),
            }
        }
        Ok(())
    }

    pub fn close(&mut self, tag: &str) {
        let _ = write!(self.out, "</{tag}>");
    }

    /// Writes escaped text.
    pub fn text(&mut self, s: &str) {
        escape::push_text(&mut self.out, s);
    }

    /// Writes markup as is, such as an icon's SVG shapes. For component code only,
    /// with markup the component built or sanitized itself: never pass document
    /// content through it.
    pub fn markup(&mut self, html: &str) {
        self.out.push_str(html);
    }

    /// Whether a component named `name` is registered, e.g. for a kit that uses
    /// another kit's component when it's there.
    pub fn has_component(&self, name: &str) -> bool {
        self.registry.get(name).is_some()
    }

    pub fn render(&mut self, node: &Node) -> Result<(), RenderError> {
        let registry = self.registry;
        let component = registry
            .get(&node.kind)
            .ok_or_else(|| RenderError::new(node, format!("unknown component type `{}`", node.kind)))?;
        component.render(node, self)
    }

    pub fn children(&mut self, node: &Node) -> Result<(), RenderError> {
        node.children.iter().try_for_each(|c| self.render(c))
    }

    /// Renders the node's children once per item, with the item and its index in scope
    /// under the given names.
    ///
    /// With the `parallel` feature and [`Engine::parallel_min_items`](crate::Engine)
    /// set, large lists are split into chunks rendered on rayon's thread pool. The
    /// output is identical to rendering them in order.
    pub fn each(&mut self, node: &Node, items: Vec<Json>, item: &str, index: &str) -> Result<(), RenderError> {
        #[cfg(feature = "parallel")]
        if self.parallel_min_items.is_some_and(|min| items.len() >= min) {
            return self.each_parallel(node, &items, item, index);
        }
        for (i, value) in items.into_iter().enumerate() {
            self.with_scope(vec![(item.to_string(), value), (index.to_string(), json!(i))], |cx| cx.children(node))?;
        }
        Ok(())
    }

    /// Writes the node's children once into a `<template>`, with `item` and `index`
    /// left for the browser to fill in. The runtime clones it per item when it
    /// rebuilds a list (see [`Element::list`]).
    pub(crate) fn item_template(&mut self, node: &Node, item: &str, index: &str) -> Result<(), RenderError> {
        self.item_template_with(item, index, |cx| cx.children(node))
    }

    /// Writes a list's item `<template>` from whatever `f` renders, with `item` and
    /// `index` left for the browser to fill in. For components that render their own
    /// items rather than their children (see [`Element::list`]).
    pub fn item_template_with(
        &mut self,
        item: &str,
        index: &str,
        f: impl FnOnce(&mut Self) -> Result<(), RenderError>,
    ) -> Result<(), RenderError> {
        self.out.push_str("<template>");
        self.in_client_item(item, index, f)?;
        self.out.push_str("</template>");
        Ok(())
    }

    /// The client-side form of `v` inside an item of a list the browser rebuilds,
    /// such as a list's `key`.
    pub fn item_val_js(&mut self, v: &Val, item: &str, index: &str) -> Json {
        self.in_client_item(item, index, |cx| Ok(cx.val_js(v))).unwrap_or(Json::Null)
    }

    /// Runs `f` with `item` and `index` in scope as names only the browser knows.
    fn in_client_item<T>(
        &mut self,
        item: &str,
        index: &str,
        f: impl FnOnce(&mut Self) -> Result<T, RenderError>,
    ) -> Result<T, RenderError> {
        let n = self.scopes.len();
        for (name, is_index) in [(item, false), (index, true)] {
            self.scopes.push(Scope { name: name.to_string(), value: Json::Null, client: true, index: is_index });
        }
        let result = f(self);
        self.scopes.truncate(n);
        result
    }

    /// Renders the node's children where the browser can show and hide them as `when`
    /// changes. The content sits between two `<template>` markers, so this works in any
    /// parent, tables included. Hidden content is rendered inside the first marker.
    pub(crate) fn conditional(&mut self, node: &Node, when: &Val) -> Result<(), RenderError> {
        self.conditional_with(when, |cx| cx.children(node))
    }

    /// Like `if` with a `when` that uses state: renders whatever `f` renders where the
    /// browser can show and hide it as `when` changes.
    pub fn conditional_with(
        &mut self,
        when: &Val,
        f: impl FnOnce(&mut Self) -> Result<(), RenderError>,
    ) -> Result<(), RenderError> {
        let show = expr::truthy(&self.eval(when));
        let mut js = String::new();
        self.write_val_js(&mut js, when);
        self.out.push_str("<template data-i-if=\"");
        escape::push_attr(&mut self.out, &js);
        self.out.push_str("\">");
        if show {
            self.out.push_str("</template>");
            f(self)?;
        } else {
            f(self)?;
            self.out.push_str("</template>");
        }
        self.out.push_str("<template data-i-fi></template>");
        self.interactive = true;
        Ok(())
    }

    #[cfg(feature = "parallel")]
    fn each_parallel(&mut self, node: &Node, items: &[Json], item: &str, index: &str) -> Result<(), RenderError> {
        use rayon::prelude::*;
        // `self` holds a non-thread-safe style cache, so workers get the shared,
        // read-only parts and build their own context.
        let (registry, policy, theme, state, data) = (self.registry, self.policy, self.theme, self.state, self.data);
        let min = self.parallel_min_items;
        let (scopes, contexts) = (&self.scopes, &self.contexts);
        // A few chunks per thread so uneven items still balance across threads.
        let chunk = items.len().div_ceil(rayon::current_num_threads() * 4).max(16);
        let chunks: Vec<_> = items
            .par_chunks(chunk)
            .enumerate()
            .map(|(c, chunk_items)| {
                let mut cx = RenderCx::new(registry, policy, theme, state, data, min);
                cx.scopes = scopes.clone();
                cx.contexts = contexts.clone();
                for (j, value) in chunk_items.iter().enumerate() {
                    let vars = vec![(item.to_string(), value.clone()), (index.to_string(), json!(c * chunk + j))];
                    cx.with_scope(vars, |cx| cx.children(node))?;
                }
                Ok((cx.out, cx.styles.into_rules(), cx.interactive, cx.scripts, cx.early))
            })
            .collect();
        for result in chunks {
            let (out, rules, interactive, scripts, early) = result?;
            for (name, js) in scripts {
                self.scripts.entry(name).or_insert(js);
            }
            for (name, js) in early {
                self.early.entry(name).or_insert(js);
            }
            self.out.push_str(&out);
            self.styles.merge(rules);
            self.interactive |= interactive;
        }
        Ok(())
    }

    /// Adds a page-level CSS rule, such as `@keyframes`, once per page however many
    /// components ask for it. For component code only: the CSS is `'static`, so a
    /// document can never supply it, and it isn't validated.
    pub fn global_css(&mut self, name: &'static str, css: &'static str) {
        self.styles.global(name, css);
    }

    /// Adds a client behaviour's script once per page. The script registers the
    /// behaviour, and the runtime starts it on elements marked with
    /// [`Element::behavior`]:
    ///
    /// ```js
    /// (window.__irenderBehaviors ||= {})["name"] = (el, api) => {
    ///   // api.config, api.state, api.set(key, value), api.emit(event, detail),
    ///   // api.list(listEl, items, offset) to supply a list's items itself
    ///   return { update(state) { /* after every state change */ } };
    /// };
    /// ```
    ///
    /// For component code only: the script is `'static`, so a document can never
    /// supply one. It must not contain `</script`.
    pub fn global_js(&mut self, name: &'static str, js: &'static str) {
        debug_assert!(!js.to_ascii_lowercase().contains("</script"), "behaviour `{name}` contains `</script`");
        self.scripts.entry(name).or_insert(js);
        self.interactive = true;
    }

    /// Adds a script once per page that runs right after the page's `<style>`, before
    /// any of its content is parsed or shown. For the little that must happen before
    /// the first paint, such as applying a stored colour scheme; everything else
    /// belongs in a behaviour ([`global_js`](Self::global_js)). The same rules apply:
    /// component code only, and no `</script`.
    pub fn early_js(&mut self, name: &'static str, js: &'static str) {
        debug_assert!(!js.to_ascii_lowercase().contains("</script"), "script `{name}` contains `</script`");
        self.early.entry(name).or_insert(js);
    }

    /// A short key that differs between the items of a server-rendered list (it hashes
    /// the names in scope and their values), and is empty outside lists. Components
    /// add it to element ids so repeated instances stay unique.
    pub fn scope_key(&self) -> String {
        use std::hash::{Hash, Hasher};
        if self.scopes.is_empty() {
            return String::new();
        }
        let mut h = irender_style::Fnv::default();
        for s in &self.scopes {
            s.name.hash(&mut h);
            expr::hash_json(&s.value, &mut h);
        }
        format!("{:x}", h.finish() & 0xffff_ffff)
    }

    /// The indexes of the lists the browser rebuilds around this point, as a template
    /// such as `{{ index }}` (`None` outside them). [`scope_key`](Self::scope_key)
    /// can't tell those items apart, so components add this to element ids too.
    pub fn client_key(&self) -> Option<Template> {
        let mut parts = Vec::new();
        for s in self.scopes.iter().filter(|s| s.index) {
            if !parts.is_empty() {
                parts.push(Part::Lit("-".into()));
            }
            parts.push(Part::Expr(Expr::Path(vec![s.name.clone()])));
        }
        (!parts.is_empty()).then_some(Template { parts })
    }

    /// Runs `f` with `value` available to the components inside under `name`, e.g. the
    /// id of the dialog a close button belongs to. Unlike a scope, the value isn't a
    /// name documents can read, and it stays a template when it depends on an item
    /// only the browser knows. `None` hides an outer value of the same name.
    pub fn with_context<T>(
        &mut self,
        name: &'static str,
        value: Option<Val>,
        f: impl FnOnce(&mut Self) -> Result<T, RenderError>,
    ) -> Result<T, RenderError> {
        self.contexts.push((name, value));
        let result = f(self);
        self.contexts.pop();
        result
    }

    /// The innermost value passed down under `name` by [`with_context`](Self::with_context).
    pub fn context(&self, name: &str) -> Option<&Val> {
        self.contexts.iter().rev().find(|(n, _)| *n == name).and_then(|(_, v)| v.as_ref())
    }

    /// Runs `f` with extra names in scope (e.g. a list's `item` and `index`).
    pub fn with_scope<T>(
        &mut self,
        vars: Vec<(String, Json)>,
        f: impl FnOnce(&mut Self) -> Result<T, RenderError>,
    ) -> Result<T, RenderError> {
        let n = vars.len();
        self.scopes.extend(vars.into_iter().map(|(name, value)| Scope { name, value, client: false, index: false }));
        let result = f(self);
        self.scopes.truncate(self.scopes.len() - n);
        result
    }

    fn write_attr(&mut self, name: &str, value: &Json) {
        // ARIA states are the strings "true" / "false", not HTML boolean attributes.
        if let Json::Bool(b) = value
            && is_aria(name)
        {
            return self.write_attr_str(name, if *b { "true" } else { "false" });
        }
        match value {
            Json::Null | Json::Bool(false) => {}
            Json::Bool(true) => {
                self.out.push(' ');
                self.out.push_str(name);
            }
            Json::String(s) => self.write_attr_str(name, s),
            other => self.write_attr_str(name, &stringify(other)),
        }
    }

    fn write_attr_str(&mut self, name: &str, value: &str) {
        if Policy::is_url_attr(name) && !self.policy.url_allowed(value) {
            return;
        }
        let _ = write!(self.out, " {name}=\"");
        escape::push_attr(&mut self.out, value);
        self.out.push('"');
    }

    fn write_action_js(&self, out: &mut String, action: &Action) {
        let map = |out: &mut String, m: &IndexMap<String, Val>| {
            out.push('{');
            for (i, (k, v)) in m.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                let _ = write_json_str(out, k);
                out.push(':');
                self.write_val_js(out, v);
            }
            out.push('}');
        };
        match action {
            Action::SetState { key, value } => {
                out.push_str("{\"a\":\"set\",\"k\":");
                let _ = write_json_str(out, key);
                out.push_str(",\"v\":");
                self.write_val_js(out, value);
            }
            Action::Navigate { url } => {
                out.push_str("{\"a\":\"nav\",\"u\":");
                self.write_val_js(out, url);
            }
            Action::Submit { endpoint, method, body, into } => {
                out.push_str("{\"a\":\"submit\",\"e\":");
                self.write_val_js(out, endpoint);
                out.push_str(",\"m\":");
                let _ = write_json_str(out, method);
                out.push_str(",\"b\":");
                map(out, body);
                out.push_str(",\"into\":");
                match into {
                    Some(k) => {
                        let _ = write_json_str(out, k);
                    }
                    None => out.push_str("null"),
                }
            }
            Action::Call { name, args } => {
                out.push_str("{\"a\":\"call\",\"n\":");
                let _ = write_json_str(out, name);
                out.push_str(",\"args\":");
                map(out, args);
            }
            Action::Wait { ms } => {
                let _ = write!(out, "{{\"a\":\"wait\",\"ms\":{ms}");
            }
        }
        out.push('}');
    }
}

fn is_aria(name: &str) -> bool {
    name.get(..5).is_some_and(|p| p.eq_ignore_ascii_case("aria-"))
}

/// A server-rendered custom property value. Values that could inject other
/// declarations are dropped.
/// `inherit`, `initial`, `unset`, `revert` or `revert-layer`: a value that applies to
/// the custom property itself, so it can't be passed through one.
fn is_css_wide_keyword(css: &str) -> bool {
    ["inherit", "initial", "unset", "revert", "revert-layer"].iter().any(|k| css.trim().eq_ignore_ascii_case(k))
}

fn css_var_value(v: &Json, px: bool) -> Option<String> {
    let s = match v {
        Json::Null => return None,
        Json::Number(_) if px => format!("{}px", stringify(v)),
        _ => stringify(v),
    };
    irender_core::policy::css_value_is_contained(&s).ok().map(|_| s)
}
