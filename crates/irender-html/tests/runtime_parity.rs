//! The browser runtime re-implements the expression evaluator. This test runs the
//! same expressions through both and requires identical results. Needs `node` on
//! PATH; skipped (with a message) otherwise.

use irender_core::{Template, expr};
use serde_json::{Value as Json, json};
use std::process::Command;

/// (template, state, server data). Paths into `data` are folded into literals before
/// the browser sees them, exactly as the renderer does.
fn cases() -> Vec<(&'static str, Json, Json)> {
    let state = json!({
        "n": 3, "neg": -2, "f": 0.1, "s": "abc", "empty": "", "t": true, "no": false, "nil": null,
        "list": [1, "two", { "three": 3 }], "obj": { "a": { "b": [10, 20] }, "k-ey": "dash" }, "emoji": "a😀",
        "num_str": " 42 ", "big": 9007199254740991_i64, "key": "a", "i": 1, "names": { "region": "Region", "a": { "short": "A" } }
    });
    let data = json!({ "price": 1499, "name": "Mug", "rows": [{ "id": 7 }], "dim": "region" });
    let exprs = [
        // literals & paths
        "{{ 1 }}", "{{ -1.5 }}", "{{ 'x' }}", "{{ \"y\" }}", "{{ true }}", "{{ null }}",
        "{{ n }}", "{{ missing }}", "{{ missing.deep.path }}", "{{ list[1] }}", "{{ list.2.three }}",
        "{{ list.length }}", "{{ s.length }}", "{{ emoji.length }}", "{{ obj.a.b[1] }}", "{{ obj['k-ey'] }}",
        "{{ list }}", "{{ obj }}", "{{ list.9 }}", "{{ n.length }}", "{{ s.foo }}",
        // arithmetic & coercion
        "{{ n + 1 }}", "{{ n - 10 }}", "{{ n * f }}", "{{ f + 0.2 }}", "{{ n / 0 }}", "{{ -n / 0 }}",
        "{{ 0 / 0 }}", "{{ n % 2 }}", "{{ -7 % 3 }}", "{{ 7.5 % 2 }}", "{{ s * 2 }}", "{{ num_str * 2 }}",
        "{{ empty * 2 }}", "{{ t + t }}", "{{ nil + 1 }}", "{{ list + 1 }}", "{{ -s }}", "{{ -nil }}", "{{ -neg }}",
        "{{ s + n }}", "{{ n + s }}", "{{ s + nil }}", "{{ s + t }}", "{{ s + list }}", "{{ s + obj }}",
        "{{ f * 3 }}", "{{ 1 / 3 }}", "{{ big + 1 }}", "{{ 2.50 }}", "{{ 0.1 * 3 }}",
        // comparison & equality
        "{{ n == 3 }}", "{{ n == '3' }}", "{{ n != 3.0 }}", "{{ nil == missing }}", "{{ list == list }}",
        "{{ obj == obj }}", "{{ s < 'abd' }}", "{{ s > 'ab' }}", "{{ '10' < '9' }}", "{{ '10' < 9 }}",
        "{{ s < 1 }}", "{{ s >= 1 }}", "{{ nil < 1 }}", "{{ t > no }}", "{{ n <= 3 && n >= 3 }}",
        // logic
        "{{ !n }}", "{{ !empty }}", "{{ !list }}", "{{ !!s }}", "{{ empty || 'fallback' }}", "{{ s && n }}",
        "{{ no && missing.x }}", "{{ 0 || nil }}", "{{ f || 0 }}",
        "{{ n > 2 ? 'big' : 'small' }}", "{{ no ? 1 : nil ? 2 : 3 }}", "{{ (n + 1) * 2 }}", "{{ n + 1 * 2 }}",
        // built-in functions
        "{{ range(n) }}", "{{ range(1, n + 1) }}", "{{ range(s) }}", "{{ range(num_str) }}", "{{ range(-2) }}",
        "{{ range(f) }}", "{{ range(neg, 1) }}", "{{ range(big) }}", "{{ range(nil) }}", "{{ range(n) == range(0, 3) }}",
        "{{ range(0) }}", "{{ range(2.9, 5.1) }}", "x{{ range(2) }}y",
        "{{ includes(list, 1) }}", "{{ includes(list, 'two') }}", "{{ includes(list, obj) }}", "{{ includes(list, 1.0) }}",
        "{{ includes(s, 'a') }}", "{{ includes(nil, 1) }}", "{{ includes(list, list.2) }}", "{{ toggle(list, list.2) }}",
        "{{ toggle(list, 1) }}", "{{ toggle(list, 'x') }}", "{{ toggle(nil, n) }}", "{{ toggle(s, 1) }}",
        "{{ toggle(toggle(list, 'two'), 'two') }}",
        // computed keys
        "{{ obj[key] }}", "{{ obj[key].b[i] }}", "{{ obj[key]['b'][0] }}", "{{ list[i] }}", "{{ list[i + 1].three }}",
        "{{ names[dim] }}", "{{ names[key].short }}", "{{ names[missing] }}", "{{ list[f] }}", "{{ list[neg] }}",
        "{{ list[nil] }}", "{{ obj[t] }}", "{{ list['len' + 'gth'] }}", "{{ s[key] }}", "{{ rows[n - 3].id }}",
        "{{ names[n > 2 ? 'region' : 'a'] }}", "{{ list[range(3)] }}", "{{ obj[obj] }}",
        // templates (string results)
        "₹{{ price * n }}", "{{ n }}{{ s }}", "{{ name }} × {{ n }} = {{ price * n }}", "{{ nil }}-{{ t }}-{{ list }}",
        "{{ rows[0].id + n }}", "{{ f }}", "{{ neg }}px", "{{ big }}", "plain",
    ];
    exprs.iter().map(|e| (*e, state.clone(), data.clone())).collect()
}

#[test]
fn rust_and_browser_evaluators_agree() {
    let node = std::env::var("NODE").unwrap_or_else(|_| "node".into());
    if Command::new(&node).arg("--version").output().is_err() {
        eprintln!("skipping runtime parity: `{node}` not found (set NODE=/path/to/node)");
        return;
    }

    let mut inputs = Vec::new();
    let mut expected = Vec::new();
    for (src, state, data) in cases() {
        let tpl = Template::parse(src).unwrap_or_else(|e| panic!("{src}: {e}"));
        let is_state = |root: &str| state.get(root).is_some() || data.get(root).is_none();
        let lookup = |p: &[String]| {
            let source = if state.get(&p[0]).is_some() { &state } else { &data };
            expr::lookup(source, p)
        };
        expected.push(tpl.eval(&lookup));
        let js = tpl.fold(&is_state, &lookup).to_js();
        inputs.push(json!({ "template": js, "state": state }));
    }

    let runtime = concat!(env!("CARGO_MANIFEST_DIR"), "/src/runtime.js");
    let script = r#"
        const fs = require("fs");
        globalThis.window = {};
        globalThis.document = { readyState: "complete", querySelectorAll: () => [] };
        eval(fs.readFileSync(process.argv[1], "utf8"));
        const cases = JSON.parse(fs.readFileSync(0, "utf8"));
        process.stdout.write(JSON.stringify(cases.map(c => window.__irender.evaluate(c.template, c.state))));
    "#;
    let mut child = Command::new(&node)
        .args(["-e", script, runtime])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    {
        use std::io::Write;
        child.stdin.take().unwrap().write_all(Json::Array(inputs).to_string().as_bytes()).unwrap();
    }
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "node failed:\n{}", String::from_utf8_lossy(&out.stderr));
    let actual: Vec<Json> = serde_json::from_slice(&out.stdout).unwrap();

    let mismatches: Vec<String> = cases()
        .iter()
        .zip(expected.iter().zip(&actual))
        .filter(|(_, (rust, js))| !same(rust, js))
        .map(|((src, ..), (rust, js))| format!("  {src:<40} rust={rust}  js={js}"))
        .collect();
    assert!(mismatches.is_empty(), "evaluators disagree:\n{}", mismatches.join("\n"));
}

/// JSON equality where `3` and `3.0` are the same number.
fn same(a: &Json, b: &Json) -> bool {
    match (a, b) {
        (Json::Number(x), Json::Number(y)) => x.as_f64() == y.as_f64(),
        (Json::Array(x), Json::Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(a, b)| same(a, b)),
        (Json::Object(x), Json::Object(y)) => x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w))),
        _ => a == b,
    }
}
