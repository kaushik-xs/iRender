use irender_core::{
    Catalog, ComponentInfo, Diagnostics, Format, Ir, Policy, PropDef, PropKind, Severity, StyleValue, Template, Val,
    expr::{self, Expr, stringify},
    parse, resolve,
};
use serde_json::{Value as Json, json};
use std::path::Path;

// ---------------------------------------------------------------- expressions

fn eval(src: &str, data: Json) -> Json {
    Expr::parse(src).unwrap_or_else(|e| panic!("{src}: {e}")).eval(&|p| expr::lookup(&data, p))
}

#[test]
fn string_literals_and_escapes() {
    assert_eq!(eval(r#""a\"b""#, Json::Null), json!("a\"b"));
    assert_eq!(eval(r"'it\'s'", Json::Null), json!("it's"));
    assert_eq!(eval(r"'line\nbreak'", Json::Null), json!("line\nbreak"));
    assert_eq!(eval("'₹ ünïcode'", Json::Null), json!("₹ ünïcode"));
}

#[test]
fn coercions_match_the_documented_semantics() {
    assert_eq!(eval("null + 1", Json::Null), json!(1));
    assert_eq!(eval("'n=' + 3", Json::Null), json!("n=3"));
    assert_eq!(eval("1.5 + 'x'", Json::Null), json!("1.5x"));
    assert_eq!(eval("'a' * 2", Json::Null), Json::Null, "NaN becomes null");
    assert_eq!(eval("' 4 ' * 2", Json::Null), json!(8));
    assert_eq!(eval("true + true", Json::Null), json!(2));
    assert_eq!(eval("-x", json!({ "x": "3" })), json!(-3));
    assert_eq!(eval("-0", Json::Null), json!(0));
}

#[test]
fn equality_is_strict_across_types() {
    assert_eq!(eval("1 == '1'", Json::Null), json!(false));
    assert_eq!(eval("1 == 1.0", Json::Null), json!(true));
    assert_eq!(eval("null == missing", Json::Null), json!(true));
    assert_eq!(eval("a == b", json!({ "a": [1, 2], "b": [1, 2] })), json!(true));
    assert_eq!(eval("'a' != 'b'", Json::Null), json!(true));
}

#[test]
fn comparisons() {
    assert_eq!(eval("'apple' < 'banana'", Json::Null), json!(true));
    assert_eq!(eval("10 > 9", Json::Null), json!(true));
    assert_eq!(eval("'10' > 9", Json::Null), json!(true), "mixed compares as numbers");
    assert_eq!(eval("'x' < 1", Json::Null), json!(false), "NaN never compares");
    assert_eq!(eval("'x' >= 1", Json::Null), json!(false));
    assert_eq!(eval("2 <= 2 && 2 >= 2", Json::Null), json!(true));
}

#[test]
fn precedence_and_associativity() {
    assert_eq!(eval("1 || 0 && 0", Json::Null), json!(1), "&& binds tighter than ||");
    assert_eq!(eval("10 - 4 - 3", Json::Null), json!(3), "left associative");
    assert_eq!(eval("a ? 'x' : b ? 'y' : 'z'", json!({ "a": false, "b": true })), json!("y"));
    assert_eq!(eval("!a == b", json!({ "a": false, "b": true })), json!(true));
    assert_eq!(eval("2 * (3 + 4) % 5", Json::Null), json!(4));
}

#[test]
fn short_circuit_returns_operands() {
    assert_eq!(eval("name || 'anon'", json!({ "name": "" })), json!("anon"));
    assert_eq!(eval("user && user.name", json!({ "user": { "name": "Ada" } })), json!("Ada"));
    assert_eq!(eval("user && user.name", json!({})), Json::Null);
}

#[test]
fn lookup_edge_cases() {
    let data = json!({ "s": "a😀", "list": [10, 20], "obj": { "0": "zero", "k": { "deep": [1, [2, 3]] } } });
    assert_eq!(eval("s.length", data.clone()), json!(3), "UTF-16 length, like JS");
    assert_eq!(eval("list[5]", data.clone()), Json::Null);
    assert_eq!(eval("list.length.x", data.clone()), Json::Null);
    assert_eq!(eval("obj.0", data.clone()), json!("zero"));
    assert_eq!(eval("obj['k'].deep[1][0]", data.clone()), json!(2));
    assert_eq!(eval("list.foo", data.clone()), Json::Null);
    assert_eq!(eval("s.foo", data), Json::Null);
}

#[test]
fn rejects_anything_beyond_the_language() {
    for bad in ["a = 1", "f()", "a.b()", "[1, 2]", "{a: 1}", "a ? b", "'open", "a..b", "1 +", "@x", "a[b]"] {
        assert!(Expr::parse(bad).is_err(), "`{bad}` should not parse");
    }
}

#[test]
fn stringify_formats() {
    assert_eq!(stringify(&json!(3.0)), "3");
    assert_eq!(stringify(&json!(1.25)), "1.25");
    assert_eq!(stringify(&json!(-7)), "-7");
    assert_eq!(stringify(&Json::Null), "");
    assert_eq!(stringify(&json!({ "a": [1] })), r#"{"a":[1]}"#);
}

#[test]
fn templates() {
    let data = json!({ "a": 1, "b": "x" });
    let t = Template::parse("{{ a }}{{ b }} and {{ a + 1 }}").unwrap();
    assert_eq!(t.eval(&|p| expr::lookup(&data, p)), json!("1x and 2"));
    let plain = Template::parse("no holes } }} here").unwrap();
    assert_eq!(plain.eval(&|_| Json::Null), json!("no holes } }} here"));
    let err = Template::parse("ok {{ a + }}").unwrap_err();
    assert_eq!(err.offset, 10, "offsets point into the whole string, at the closing `}}`");
    assert!(Template::parse("{{ a").unwrap_err().message.contains("unclosed"));
    assert!(Template::parse("x").unwrap().as_single_expr().is_none());
}

#[test]
fn val_parsing() {
    assert_eq!(Val::parse(&json!("plain")).unwrap(), Val::from("plain"));
    assert_eq!(Val::parse(&json!(5)).unwrap(), Val::Lit(json!(5)));
    assert!(matches!(Val::parse(&json!("{{ x }}")).unwrap(), Val::Tpl(_)));
    let v = Val::parse(&json!("{{ a * b }}")).unwrap();
    let js = v.to_js(&|r| r == "b", &|_| json!(4));
    assert_eq!(js, json!({ "r": { "b": "*", "x": { "l": 4 }, "y": { "p": ["b"] } } }));
}

// ---------------------------------------------------------------- parsing

#[test]
fn parse_errors_carry_paths() {
    let err = parse(r#"{ "version": 1, "root": { "type": "x", "children": [ { "type": "y", "colour": 1 } ] } }"#, Format::Json)
        .unwrap_err();
    assert_eq!(err.0[0].path, "root.children[0].colour");
    assert!(err.0[0].message.contains("unknown field `colour`"), "{err}");

    let err = parse(r#"{ "version": 1, "root": { "type": "x" }, "extra": true }"#, Format::Json).unwrap_err();
    assert!(err.to_string().contains("unknown field `extra`"), "{err}");

    let err = parse(r#"{ "version": 1, "root": { "type": "x", "on": { "click": { "action": "teleport" } } } }"#, Format::Json)
        .unwrap_err();
    assert!(err.0[0].path.starts_with("root.on.click"), "{err}");

    let err = parse("{ not json", Format::Json).unwrap_err();
    assert_eq!(err.len(), 1);
}

#[test]
fn yaml_and_json_produce_the_same_document() {
    let json_doc = parse(r#"{ "version": 1, "state": { "n": 1 }, "root": { "type": "t", "props": { "v": "{{ n }}" } } }"#, Format::Json).unwrap();
    let yaml_doc = parse("version: 1\nstate: { n: 1 }\nroot:\n  type: t\n  props:\n    v: '{{ n }}'\n", Format::Yaml).unwrap();
    assert_eq!(serde_json::to_value(&json_doc).unwrap(), serde_json::to_value(&yaml_doc).unwrap());
}

#[test]
fn format_from_path() {
    assert_eq!(Format::from_path(Path::new("a/b.JSON")), Some(Format::Json));
    assert_eq!(Format::from_path(Path::new("x.yml")), Some(Format::Yaml));
    assert_eq!(Format::from_path(Path::new("x.toml")), None);
    assert_eq!(Format::from_path(Path::new("noext")), None);
}

// ---------------------------------------------------------------- resolve

struct Cat;

const BOX_PROPS: &[PropDef] = &[
    PropDef::new("title", PropKind::String, "").required(),
    PropDef::new("count", PropKind::Number, ""),
    PropDef::new("on", PropKind::Bool, ""),
    PropDef::new("tag", PropKind::Tag, ""),
];

impl Catalog for Cat {
    fn component(&self, name: &str) -> Option<ComponentInfo> {
        match name {
            "box" => Some(ComponentInfo { doc: "", props: BOX_PROPS, parts: &[], children: true, open_props: false }),
            "leaf" => Some(ComponentInfo { doc: "", props: &[], parts: &[], children: false, open_props: false }),
            "open" => Some(ComponentInfo { doc: "", props: &[], parts: &[], children: true, open_props: true }),
            _ => None,
        }
    }
}

fn check_with(doc: Json, policy: &Policy) -> Result<Ir, Diagnostics> {
    resolve(&parse(&doc.to_string(), Format::Json)?, &Cat, policy)
}

fn check(doc: Json) -> Result<Ir, Diagnostics> {
    check_with(doc, &Policy::trusted())
}

fn errors(doc: Json) -> Vec<String> {
    check(doc).unwrap_err().iter().map(|d| d.to_string()).collect()
}

fn doc(root: Json) -> Json {
    json!({ "version": 1, "state": { "n": 0 }, "root": root })
}

#[test]
fn valid_document_resolves_without_warnings() {
    let ir = check(doc(json!({ "type": "box", "props": { "title": "t", "count": 2, "on": true, "tag": "section" },
        "children": [{ "type": "leaf" }, { "type": "open", "props": { "anything": [1] } }] })))
    .unwrap();
    assert!(ir.warnings.is_empty());
    assert_eq!(ir.root.children[1].path, "root.children[1]");
}

#[test]
fn prop_types_are_checked_for_literals_only() {
    let errs = errors(doc(json!({ "type": "box", "props": { "title": 1, "count": "2", "on": "yes", "tag": "1h" } })));
    assert_eq!(
        errs,
        [
            "root.props.title: expected a string, got a number",
            "root.props.count: expected a number, got `2`",
            "root.props.on: expected a boolean, got `yes`",
            "root.props.tag: `1h` is not a valid tag name",
        ]
    );
    // Templates are not type-checked until render time.
    assert!(check(doc(json!({ "type": "box", "props": { "title": "{{ n }}", "count": "{{ n }}" } }))).is_ok());
}

#[test]
fn structure_errors() {
    assert_eq!(errors(doc(json!({ "type": "box" }))), ["root.props: missing required prop `title`"]);
    assert_eq!(
        errors(doc(json!({ "type": "leaf", "children": [{ "type": "leaf" }] }))),
        ["root.children: `leaf` does not accept children"]
    );
    assert_eq!(
        errors(doc(json!({ "type": "leaf", "parts": { "x": {} } }))),
        ["root.parts.x: `leaf` has no part `x` (it has no parts)"]
    );
    let errs = errors(json!({ "version": 2, "state": { "bad-key": 1 }, "root": { "type": "leaf" } }));
    assert_eq!(errs, ["version: unsupported version 2, expected 1", "state.bad-key: state keys must be identifiers (letters, digits, `_`)"]);
}

#[test]
fn attrs_and_classes() {
    let errs = errors(doc(json!({ "type": "leaf",
        "attrs": { "data-ok": 1, "a\"b": "x", "list": [1], "hidden": "{{ n > }}" }, "class": ["ok", "{{ }}"] })));
    assert_eq!(
        errs,
        [
            "root.attrs.a\"b: `a\"b` is not a valid attribute name",
            "root.attrs.list: attribute values must be strings, numbers, booleans or null",
            "root.attrs.hidden: invalid expression: unexpected end of expression (at offset 7)",
            "root.class[1]: invalid expression: unexpected end of expression (at offset 3)",
        ]
    );
}

#[test]
fn style_resolution() {
    let ir = check(doc(json!({ "type": "leaf", "style": {
        "fontSize": 14, "lineHeight": 1.4, "zIndex": 2, "margin": 0, "--gap": 4,
        "border": "1px solid $color.border", "price": null,
        "@media (min-width: 900px)": { "@supports (display: grid)": { "display": "grid" } },
        "&.active > span": { "color": "red" }, "[aria-busy=true]": { "cursor": "wait" }
    } })));
    let errs: Vec<_> = ir.unwrap_err().iter().map(|d| d.to_string()).collect();
    assert_eq!(errs, ["root.style.price: style values must be strings or numbers"]);

    let ir = check(doc(json!({ "type": "leaf", "style": {
        "fontSize": 14, "lineHeight": 1.4, "zIndex": 2, "margin": 0, "--gap": 4,
        "border": "1px solid $color.border", "width": "calc({{ n }}px + $space.md)"
    } })))
    .unwrap();
    let decls: Vec<_> = ir.root.style.decls.iter().map(|d| (d.property.as_str(), &d.value)).collect();
    let s = |v: &str| StyleValue::Static(v.into());
    assert_eq!(decls[0], ("font-size", &s("14px")));
    assert_eq!(decls[1], ("line-height", &s("1.4")));
    assert_eq!(decls[2], ("z-index", &s("2")));
    assert_eq!(decls[3], ("margin", &s("0")));
    assert_eq!(decls[4], ("--gap", &s("4")));
    assert_eq!(decls[5], ("border", &s("1px solid var(--color-border)")));
    let StyleValue::Dynamic { tpl, px } = decls[6].1 else { panic!() };
    assert!(px);
    assert_eq!(tpl.eval(&|_| json!(3)), json!("calc(3px + var(--space-md))"), "tokens in templates resolve too");
}

#[test]
fn style_errors() {
    let errs = errors(doc(json!({ "type": "leaf", "style": {
        ":hover": "red", "@print": { "color": "$nope" }, "@keyframes x": {}, "color": "red; top: 0",
        "width": "{{ n + }}", "height": "$"
    } })));
    assert_eq!(
        errs,
        [
            "root.style.:hover: a nested style block must be an object",
            "root.style.@print.color: unknown theme token `$nope`",
            "root.style.@keyframes x: unsupported at-rule `@keyframes` (use @media, @supports or @container)",
            "root.style.color: CSS value `red; top: 0` must not contain `{`, `}`, `;` or `</`",
            "root.style.width: invalid expression: unexpected end of expression (at offset 7)",
            "root.style.height: `$` must be followed by a token name (use `$$` for a literal `$`)",
        ]
    );
}

#[test]
fn actions() {
    let errs = errors(doc(json!({ "type": "leaf", "on": {
        "Click": { "action": "set_state", "key": "n", "value": 1 },
        "submit": [
            { "action": "submit", "endpoint": "/x", "method": "fetch", "into": "missing" },
            { "action": "navigate", "url": "{{ n }}" },
            { "action": "call", "name": "go", "args": { "x": "{{ n * }}" } }
        ]
    } })));
    assert_eq!(
        errs,
        [
            "root.on.Click: event names are lowercase DOM event names, e.g. `click`",
            "root.on.submit[0].method: unsupported method `FETCH`",
            "root.on.submit[0].into: `missing` is not declared in `state`",
            "root.on.submit[2].args.x: invalid expression: unexpected end of expression (at offset 7)",
        ]
    );
    let ir = check(doc(json!({ "type": "leaf", "on": { "click": [
        { "action": "submit", "endpoint": "/api", "body": { "n": "{{ n }}" }, "into": "n" },
        { "action": "set_state", "key": "n", "value": "{{ n + 1 }}" }
    ] } })))
    .unwrap();
    let actions = &ir.root.on["click"];
    assert!(matches!(&actions[0], irender_core::Action::Submit { method, into: Some(k), .. } if method == "POST" && k == "n"));
    assert!(matches!(&actions[1], irender_core::Action::SetState { key, value: Val::Tpl(_) } if key == "n"));
}

#[test]
fn untrusted_policy_blocks_scripts_urls_and_css() {
    let mut policy = Policy::untrusted();
    policy.allowed_css_properties = Some(["color".to_string()].into());
    let errs: Vec<_> = check_with(
        doc(json!({ "type": "box", "props": { "title": "t", "tag": "iframe" },
            "attrs": { "onerror": "x", "srcdoc": "<b>", "href": "data:text/html,x", "src": "https://ok.dev/a.png" },
            "style": { "color": "red", "padding": 1, "--free": "1", "background": "url(javascript:x)" },
            "on": { "click": { "action": "navigate", "url": "vbscript:x" } } })),
        &policy,
    )
    .unwrap_err()
    .iter()
    .map(|d| d.path.clone())
    .collect();
    assert_eq!(
        errs,
        [
            "root.props.tag",
            "root.on.click[0].url",
            "root.attrs.onerror",
            "root.attrs.srcdoc",
            "root.attrs.href",
            "root.style.padding",
            "root.style.background",
        ]
    );
}

// ---------------------------------------------------------------- theme

#[test]
fn theme_overrides_and_extensions() {
    let ir = check(json!({ "version": 1,
        "theme": {
            "tokens": { "color": { "primary": "#000", "brand": "#f00" }, "size": 3 },
            "breakpoints": { "tablet": "600px" },
            "variants": { "contrast": "[data-contrast] &" },
            "modes": { "contrast": { "color": { "primary": "#fff" } } }
        },
        "root": { "type": "leaf", "style": { "color": "$color.brand", "@tablet": { "@contrast": { "width": "$size" } } } } }))
    .unwrap();
    assert_eq!(ir.theme.tokens()["color.primary"], "#000");
    assert_eq!(ir.theme.tokens()["size"], "3");
    assert_eq!(ir.theme.tokens()["radius.md"], "8px", "defaults stay");
    assert_eq!(ir.theme.modes()["contrast"]["color.primary"], "#fff");
    assert_eq!(ir.theme.modes()["dark"]["color.surface"], "#111827", "default modes stay");
}

#[test]
fn theme_errors() {
    let errs = errors(json!({ "version": 1,
        "theme": {
            "tokens": { "bad name": "1", "x": true, "y": "a;b" },
            "variants": { "weird": "no-amp" },
            "modes": { "nope": {} }
        },
        "root": { "type": "leaf" } }));
    assert_eq!(
        errs,
        [
            "theme.tokens.bad name: token names may only contain letters, digits, `-` and `_`",
            "theme.tokens.x: a token must be a string, a number or an object of tokens",
            "theme.tokens.y: CSS value `a;b` must not contain `{`, `}`, `;` or `</`",
            "theme.variants.weird: a variant is an at-rule (`@media …`) or a selector containing `&`",
            "theme.modes.nope: `nope` is not a variant; declare it in `theme.variants`",
        ]
    );
}

#[test]
fn diagnostics_severity() {
    let mut d = Diagnostics::default();
    d.warn("a", "careful");
    assert!(!d.has_errors());
    assert_eq!(d.to_string(), "warning: a: careful");
    assert!(d.clone().into_result(()).is_ok());
    let strict = d.strict();
    assert!(strict.has_errors());
    assert_eq!(strict.0[0].severity, Severity::Error);
}
