//! Document → IR: validation, expression parsing and token resolution.

use crate::{
    Action, Catalog, ComponentInfo, Diagnostics, Document, Fragments, Ir, NoFragments, Node, PartStyle,
    Policy, PropDef, PropKind, Severity, StyleBlock, StyleValue, Template, Theme, Val,
    expr::Part,
    spec::{ActionSpec, NodeSpec, VERSION},
    style::{Decl, is_nested_key, is_unitless, number_to_css, to_kebab},
};
use indexmap::IndexMap;
use serde_json::Value as Json;
use std::borrow::Cow;

/// How deeply nodes may nest, counting the nodes that included fragments add. Deeper
/// trees are rejected so hostile input cannot overflow the stack while rendering.
pub const MAX_DEPTH: usize = 64;

/// The longest a `wait` action may pause.
pub const MAX_WAIT_MS: u64 = 60_000;

/// Validates `doc` and produces the IR. All problems are reported together.
pub fn resolve(doc: &Document, catalog: &dyn Catalog, policy: &Policy) -> Result<Ir, Diagnostics> {
    resolve_with(doc, catalog, policy, &NoFragments)
}

/// [`resolve`], with `include` nodes looked up in the document's `fragments` and then
/// in `fragments`.
pub fn resolve_with(
    doc: &Document,
    catalog: &dyn Catalog,
    policy: &Policy,
    fragments: &dyn Fragments,
) -> Result<Ir, Diagnostics> {
    let mut diags = Diagnostics::default();
    if doc.version != VERSION {
        diags.push("version", format!("unsupported version {}, expected {VERSION}", doc.version));
    }
    let theme = Theme::resolve(&doc.theme, &mut diags);
    for key in doc.state.keys() {
        if !is_ident(key) {
            diags.push(format!("state.{key}"), "state keys must be identifiers (letters, digits, `_`)");
        }
    }
    let mut r = Resolver {
        catalog,
        policy,
        theme: &theme,
        state: &doc.state,
        local: &doc.fragments,
        fragments,
        including: Vec::new(),
        depth: 0,
        diags,
    };
    let root = r.node(&doc.root, "root");
    let diags = r.diags;
    let warnings = Diagnostics(diags.iter().filter(|d| d.severity == Severity::Warning).cloned().collect());
    diags.into_result(Ir { version: doc.version, theme, state: doc.state.clone(), root, warnings })
}

fn is_ident(s: &str) -> bool {
    s.starts_with(|c: char| c.is_alphabetic() || c == '_')
        && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

fn json_type(v: &Json) -> &'static str {
    match v {
        Json::Null => "null",
        Json::Bool(_) => "a boolean",
        Json::Number(_) => "a number",
        Json::String(_) => "a string",
        Json::Array(_) => "an array",
        Json::Object(_) => "an object",
    }
}

struct Resolver<'a> {
    catalog: &'a dyn Catalog,
    policy: &'a Policy,
    theme: &'a Theme,
    state: &'a IndexMap<String, Json>,
    /// The document's own fragments.
    local: &'a IndexMap<String, NodeSpec>,
    fragments: &'a dyn Fragments,
    /// Fragments being expanded, outermost first, to catch include cycles.
    including: Vec<String>,
    depth: usize,
    diags: Diagnostics,
}

impl Resolver<'_> {
    fn node(&mut self, spec: &NodeSpec, path: &str) -> Node {
        if self.depth >= MAX_DEPTH {
            self.diags.push(path, format!("nodes nest more than {MAX_DEPTH} levels deep"));
            return placeholder(path);
        }
        self.depth += 1;
        let node = if spec.kind == "include" { self.include(spec, path) } else { self.component(spec, path) };
        self.depth -= 1;
        node
    }

    /// Replaces an `include` node with its fragment.
    fn include(&mut self, spec: &NodeSpec, path: &str) -> Node {
        if !spec.attrs.is_empty()
            || !spec.class.is_empty()
            || !spec.style.is_empty()
            || !spec.parts.is_empty()
            || !spec.on.is_empty()
            || !spec.children.is_empty()
            || spec.props.keys().any(|k| k != "name")
        {
            self.diags.push(path, "`include` only takes `props.name`; style the fragment's root instead");
        }
        let name = match spec.props.get("name") {
            Some(Json::String(n)) if !Template::is_template(n) => n.as_str(),
            Some(_) => {
                self.diags.push(format!("{path}.props.name"), "expected a fragment name (expressions are not allowed)");
                return placeholder(path);
            }
            None => {
                self.diags.push(format!("{path}.props"), "missing required prop `name`");
                return placeholder(path);
            }
        };
        if self.including.iter().any(|n| n == name) {
            let chain = self.including.iter().map(String::as_str).chain([name]).collect::<Vec<_>>().join(" → ");
            self.diags.push(format!("{path}.props.name"), format!("include cycle: {chain}"));
            return placeholder(path);
        }
        let fragment = match self.local.get(name) {
            Some(f) => Cow::Borrowed(f),
            None => match self.fragments.fragment(name) {
                Ok(Some(f)) => Cow::Owned(f),
                Ok(None) => {
                    self.diags.push(format!("{path}.props.name"), format!("unknown fragment `{name}`"));
                    return placeholder(path);
                }
                Err(e) => {
                    self.diags.push(format!("{path}.props.name"), format!("fragment `{name}`: {e}"));
                    return placeholder(path);
                }
            },
        };
        self.including.push(name.to_string());
        let node = self.node(&fragment, &format!("{path}@{name}"));
        self.including.pop();
        node
    }

    fn component(&mut self, spec: &NodeSpec, path: &str) -> Node {
        let info = self.catalog.component(&spec.kind);
        if info.is_none() {
            self.diags.push(format!("{path}.type"), format!("unknown component type `{}`", spec.kind));
        }

        let mut props = IndexMap::new();
        for (name, raw) in &spec.props {
            let p = format!("{path}.props.{name}");
            let def = info.and_then(|i| i.props.iter().find(|d| d.name == name));
            if let Some(i) = info
                && def.is_none()
                && !i.open_props
            {
                self.diags.push(&p, unknown_prop(&spec.kind, name, i));
            }
            let Some(val) = self.val(raw, &p) else { continue };
            if let (Some(def), Val::Lit(v)) = (def, &val) {
                self.check_prop(def, v, &p);
            }
            props.insert(name.clone(), val);
        }
        if let Some(i) = info {
            for d in i.props.iter().filter(|d| d.required && !spec.props.contains_key(d.name)) {
                self.diags.push(format!("{path}.props"), format!("missing required prop `{}`", d.name));
            }
        }

        let mut parts = IndexMap::new();
        for (name, part) in &spec.parts {
            let p = format!("{path}.parts.{name}");
            if let Some(i) = info
                && !i.parts.contains(&name.as_str())
            {
                let known = if i.parts.is_empty() {
                    "it has no parts".to_string()
                } else {
                    format!("parts: {}", i.parts.join(", "))
                };
                self.diags.push(&p, format!("`{}` has no part `{name}` ({known})", spec.kind));
            }
            let resolved = PartStyle {
                attrs: self.attrs(&part.attrs, &format!("{p}.attrs")),
                class: self.classes(&part.class, &format!("{p}.class")),
                style: self.style(part.style.iter(), &format!("{p}.style")),
            };
            parts.insert(name.clone(), resolved);
        }

        let mut on = IndexMap::new();
        for (event, actions) in &spec.on {
            let p = format!("{path}.on.{event}");
            if event.is_empty() || !event.chars().all(|c| c.is_ascii_lowercase()) {
                self.diags.push(&p, "event names are lowercase DOM event names, e.g. `click`");
            }
            let list = actions
                .as_slice()
                .iter()
                .enumerate()
                .filter_map(|(i, a)| self.action(a, &format!("{p}[{i}]")))
                .collect();
            on.insert(event.clone(), list);
        }

        if let Some(i) = info
            && !i.children
            && !spec.children.is_empty()
        {
            self.diags.push(format!("{path}.children"), format!("`{}` does not accept children", spec.kind));
        }

        Node {
            kind: spec.kind.clone(),
            path: path.to_string(),
            props,
            attrs: self.attrs(&spec.attrs, &format!("{path}.attrs")),
            class: self.classes(&spec.class, &format!("{path}.class")),
            style: self.style(spec.style.iter(), &format!("{path}.style")),
            parts,
            on,
            children: spec
                .children
                .iter()
                .enumerate()
                .map(|(i, c)| self.node(c, &format!("{path}.children[{i}]")))
                .collect(),
        }
    }

    fn val(&mut self, raw: &Json, path: &str) -> Option<Val> {
        Val::parse(raw).map_err(|e| self.diags.push(path, format!("invalid expression: {e}"))).ok()
    }

    fn check_prop(&mut self, def: &PropDef, v: &Json, path: &str) {
        let ok = match def.kind {
            PropKind::Any => true,
            PropKind::String => v.is_string(),
            PropKind::Number => v.is_number(),
            PropKind::Bool => v.is_boolean(),
            PropKind::Enum(opts) => v.as_str().is_some_and(|s| opts.contains(&s)),
            PropKind::StateKey => v.as_str().is_some_and(|s| self.state.contains_key(s)),
            PropKind::Tag => match v.as_str().map(|t| self.policy.check_tag(t)) {
                Some(Ok(())) => true,
                Some(Err(e)) => return self.diags.push(path, e),
                None => false,
            },
        };
        if !ok {
            let got = match v {
                Json::String(s) => format!("`{s}`"),
                _ => json_type(v).to_string(),
            };
            self.diags.push(path, format!("expected {}, got {got}", def.kind.describe()));
        }
    }

    fn attrs(&mut self, map: &IndexMap<String, Json>, path: &str) -> IndexMap<String, Val> {
        let mut out = IndexMap::new();
        for (name, raw) in map {
            let p = format!("{path}.{name}");
            if let Err(e) = self.policy.check_attr_name(name) {
                self.diags.push(&p, e);
                continue;
            }
            let Some(val) = self.val(raw, &p) else { continue };
            match &val {
                Val::Lit(Json::Array(_) | Json::Object(_)) => {
                    self.diags.push(&p, "attribute values must be strings, numbers, booleans or null");
                    continue;
                }
                Val::Lit(Json::String(s)) => {
                    if let Err(e) = self.policy.check_url_attr(name, s) {
                        self.diags.push(&p, e);
                        continue;
                    }
                }
                _ => {}
            }
            out.insert(name.clone(), val);
        }
        out
    }

    fn classes(&mut self, list: &[String], path: &str) -> Vec<Val> {
        list.iter()
            .enumerate()
            .filter_map(|(i, c)| self.val(&Json::String(c.clone()), &format!("{path}[{i}]")))
            .collect()
    }

    fn style<'m>(&mut self, entries: impl Iterator<Item = (&'m String, &'m Json)>, path: &str) -> StyleBlock {
        let mut block = StyleBlock::default();
        for (key, raw) in entries {
            let p = format!("{path}.{key}");
            if is_nested_key(key) {
                if let Err(e) = self.check_condition(key) {
                    self.diags.push(&p, e);
                    continue;
                }
                match raw {
                    Json::Object(obj) => {
                        let child = self.style(obj.iter(), &p);
                        block.nested.push((key.clone(), child));
                    }
                    _ => self.diags.push(&p, "a nested style block must be an object"),
                }
                continue;
            }
            let property = to_kebab(key);
            if let Err(e) = self.policy.check_css_property(&property) {
                self.diags.push(&p, e);
                continue;
            }
            let value = match raw {
                Json::Number(n) => StyleValue::Static(number_to_css(&property, n.as_f64().unwrap_or(0.0))),
                Json::String(s) if Template::is_template(s) => match self.dynamic_style(s) {
                    Ok(tpl) => StyleValue::Dynamic { tpl, px: !is_unitless(&property) },
                    Err(e) => {
                        self.diags.push(&p, e);
                        continue;
                    }
                },
                Json::String(s) => match self.theme.substitute(s) {
                    Ok(v) => StyleValue::Static(v),
                    Err(e) => {
                        self.diags.push(&p, e);
                        continue;
                    }
                },
                _ => {
                    self.diags.push(&p, "style values must be strings or numbers");
                    continue;
                }
            };
            if let StyleValue::Static(v) = &value
                && let Err(e) = self.policy.check_css_value(v)
            {
                self.diags.push(&p, e);
                continue;
            }
            block.decls.push(Decl { property, value });
        }
        block
    }

    /// A templated style value; tokens in its literal parts are resolved now.
    fn dynamic_style(&self, s: &str) -> Result<Template, String> {
        let mut tpl = Template::parse(s).map_err(|e| format!("invalid expression: {e}"))?;
        for part in &mut tpl.parts {
            if let Part::Lit(l) = part {
                *l = self.theme.substitute(l)?;
            }
        }
        Ok(tpl)
    }

    fn check_condition(&self, key: &str) -> Result<(), String> {
        let Some(name) = key.strip_prefix('@') else { return Ok(()) };
        if key.contains(' ') {
            let at = key.split(' ').next().unwrap_or_default();
            return match at {
                "@media" | "@supports" | "@container" => Ok(()),
                _ => Err(format!("unsupported at-rule `{at}` (use @media, @supports or @container)")),
            };
        }
        if self.theme.condition(name).is_some() {
            return Ok(());
        }
        let known: Vec<_> = self.theme.condition_names().map(|n| format!("@{n}")).collect();
        Err(format!("unknown condition `{key}` (known: {})", known.join(", ")))
    }

    fn state_key(&mut self, key: &str, path: &str) -> bool {
        let ok = self.state.contains_key(key);
        if !ok {
            self.diags.push(path, format!("`{key}` is not declared in `state`"));
        }
        ok
    }

    fn val_map(&mut self, map: &IndexMap<String, Json>, path: &str) -> IndexMap<String, Val> {
        map.iter()
            .filter_map(|(k, v)| Some((k.clone(), self.val(v, &format!("{path}.{k}"))?)))
            .collect()
    }

    fn url(&mut self, url: &str, path: &str) -> Option<Val> {
        let val = self.val(&Json::String(url.to_string()), path)?;
        if let Val::Lit(_) = val
            && !self.policy.url_allowed(url)
        {
            self.diags.push(path, format!("URL `{url}` is not allowed by the policy"));
            return None;
        }
        Some(val)
    }

    fn action(&mut self, spec: &ActionSpec, path: &str) -> Option<Action> {
        match spec {
            ActionSpec::SetState { key, value } => {
                let key_ok = self.state_key(key, &format!("{path}.key"));
                let value = self.val(value, &format!("{path}.value"))?;
                key_ok.then(|| Action::SetState { key: key.clone(), value })
            }
            ActionSpec::Navigate { url } => {
                Some(Action::Navigate { url: self.url(url, &format!("{path}.url"))? })
            }
            ActionSpec::Submit { endpoint, method, body, into } => {
                let method = method.to_ascii_uppercase();
                if !matches!(method.as_str(), "GET" | "POST" | "PUT" | "PATCH" | "DELETE") {
                    self.diags.push(format!("{path}.method"), format!("unsupported method `{method}`"));
                }
                if let Some(k) = into {
                    self.state_key(k, &format!("{path}.into"));
                }
                let body = self.val_map(body, &format!("{path}.body"));
                let endpoint = self.url(endpoint, &format!("{path}.endpoint"))?;
                Some(Action::Submit { endpoint, method, body, into: into.clone() })
            }
            ActionSpec::Call { name, args } => {
                let args = self.val_map(args, &format!("{path}.args"));
                Some(Action::Call { name: name.clone(), args })
            }
            ActionSpec::Wait { ms } => {
                if *ms > MAX_WAIT_MS {
                    self.diags.push(format!("{path}.ms"), format!("waits are at most {MAX_WAIT_MS} ms"));
                    return None;
                }
                Some(Action::Wait { ms: *ms })
            }
        }
    }
}

/// Stands in for a node that failed to resolve; the IR is discarded anyway.
fn placeholder(path: &str) -> Node {
    Node {
        kind: "element".into(),
        path: path.to_string(),
        props: IndexMap::new(),
        attrs: IndexMap::new(),
        class: Vec::new(),
        style: StyleBlock::default(),
        parts: IndexMap::new(),
        on: IndexMap::new(),
        children: Vec::new(),
    }
}

fn unknown_prop(kind: &str, name: &str, info: ComponentInfo) -> String {
    let known: Vec<_> = info.props.iter().map(|d| d.name).collect();
    if known.is_empty() {
        format!("`{kind}` has no prop `{name}` (it takes no props; use `attrs` for HTML attributes)")
    } else {
        format!("`{kind}` has no prop `{name}` (props: {}; use `attrs` for HTML attributes)", known.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Format, parse};

    struct Cat;
    impl Catalog for Cat {
        fn component(&self, name: &str) -> Option<ComponentInfo> {
            const PROPS: &[PropDef] = &[
                PropDef::new("label", PropKind::String, ""),
                PropDef::new("size", PropKind::Enum(&["sm", "lg"]), ""),
                PropDef::new("bind", PropKind::StateKey, ""),
            ];
            (name == "box").then_some(ComponentInfo {
                doc: "",
                props: PROPS,
                parts: &["icon"],
                children: true,
                open_props: false,
            })
        }
    }

    fn check(json: &str, policy: &Policy) -> Result<Ir, Diagnostics> {
        resolve(&parse(json, Format::Json)?, &Cat, policy)
    }

    #[test]
    fn reports_every_problem_with_its_path() {
        let err = check(
            r#"{ "version": 1, "state": { "n": 0 }, "root": { "type": "box",
                "props": { "label": 3, "size": "xl", "bind": "nope", "extra": 1 },
                "style": { "padding": "$space.huge", "@tablet": { "color": "red" } },
                "parts": { "badge": {} },
                "on": { "click": { "action": "set_state", "key": "m", "value": "{{ n + }}" } },
                "children": [ { "type": "nope" } ] } }"#,
            &Policy::trusted(),
        )
        .unwrap_err();
        let paths: Vec<_> = err.iter().map(|d| d.path.as_str()).collect();
        assert_eq!(
            paths,
            [
                "root.props.label",
                "root.props.size",
                "root.props.bind",
                "root.props.extra",
                "root.parts.badge",
                "root.on.click[0].key",
                "root.on.click[0].value",
                "root.style.padding",
                "root.style.@tablet",
                "root.children[0].type",
            ],
            "{err}"
        );
    }

    #[test]
    fn resolves_tokens_numbers_and_templates() {
        let ir = check(
            r#"{ "version": 1, "state": { "w": 10 }, "root": { "type": "box",
                "style": { "padding": 12, "opacity": 0.5, "color": "$color.primary",
                           "width": "{{ w }}", ":hover": { "marginTop": 4 } } } }"#,
            &Policy::trusted(),
        )
        .unwrap();
        let s = &ir.root.style;
        assert_eq!(s.decls[0].value, StyleValue::Static("12px".into()));
        assert_eq!(s.decls[1].value, StyleValue::Static("0.5".into()));
        assert_eq!(s.decls[2].value, StyleValue::Static("var(--color-primary)".into()));
        assert!(matches!(s.decls[3].value, StyleValue::Dynamic { px: true, .. }));
        assert_eq!(s.nested[0].1.decls[0].property, "margin-top");
    }

    #[test]
    fn untrusted_policy() {
        let err = check(
            r#"{ "version": 1, "root": { "type": "box",
                "attrs": { "onclick": "x()", "href": "javascript:x()" },
                "on": { "click": { "action": "navigate", "url": "javascript:x()" } } } }"#,
            &Policy::untrusted(),
        )
        .unwrap_err();
        assert_eq!(err.len(), 3, "{err}");
    }
}
