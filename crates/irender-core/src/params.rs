//! Parameters of `include`: `{ "name": "card", "with": { "title": "{{ p.name }}" } }`.
//!
//! A parameter is substituted into the fragment's expressions when the document is
//! resolved: the fragment's `{{ title }}` becomes the caller's `{{ p.name }}`. The
//! result is exactly what writing the expression inline would give, so server data
//! is still folded, state still updates live, and list items still resolve, both on
//! the server and in lists the browser rebuilds.

use crate::{
    Val,
    expr::{self, Expr, Part, Template},
};
use indexmap::{IndexMap, IndexSet};
use serde_json::Value as Json;

/// The parameters of the fragment being resolved, and which of them it used.
#[derive(Default)]
pub(crate) struct Params {
    args: IndexMap<String, Expr>,
    used: IndexSet<String>,
}

impl Params {
    pub(crate) fn new(args: IndexMap<String, Expr>) -> Self {
        Self { args, used: IndexSet::new() }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.args.is_empty()
    }

    pub(crate) fn contains(&self, name: &str) -> bool {
        self.args.contains_key(name)
    }

    /// Parameters the fragment never referred to.
    pub(crate) fn unused(&self) -> impl Iterator<Item = &str> {
        self.args.keys().filter(|k| !self.used.contains(*k)).map(String::as_str)
    }

    /// Substitutes the parameters into `e`.
    pub(crate) fn expr(&mut self, e: &Expr) -> Result<Expr, String> {
        Ok(match e {
            Expr::Path(segs) => match self.args.get(&segs[0]) {
                None => e.clone(),
                Some(arg) => {
                    self.used.insert(segs[0].clone());
                    let rest = &segs[1..];
                    match arg {
                        _ if rest.is_empty() => arg.clone(),
                        Expr::Path(base) => Expr::Path(base.iter().chain(rest).cloned().collect()),
                        Expr::Lit(v) => Expr::Lit(expr::lookup(v, rest)),
                        _ => {
                            return Err(format!(
                                "`{}` is a computed value, so `{}` has no fields; pass a path or a literal",
                                segs[0],
                                segs.join(".")
                            ));
                        }
                    }
                }
            },
            Expr::Lit(_) => e.clone(),
            Expr::Unary(op, a) => Expr::Unary(*op, Box::new(self.expr(a)?)),
            Expr::Binary(op, a, b) => Expr::Binary(*op, Box::new(self.expr(a)?), Box::new(self.expr(b)?)),
            Expr::Cond(c, a, b) => Expr::Cond(Box::new(self.expr(c)?), Box::new(self.expr(a)?), Box::new(self.expr(b)?)),
            Expr::Call(f, args) => Expr::Call(*f, args.iter().map(|a| self.expr(a)).collect::<Result<_, _>>()?),
            Expr::Index(v, k) => Expr::Index(Box::new(self.expr(v)?), Box::new(self.expr(k)?)),
        })
    }

    pub(crate) fn template(&mut self, t: &Template) -> Result<Template, String> {
        let parts = t
            .parts
            .iter()
            .map(|p| match p {
                Part::Lit(s) => Ok(Part::Lit(s.clone())),
                Part::Expr(e) => self.expr(e).map(Part::Expr),
            })
            .collect::<Result<_, String>>()?;
        Ok(Template { parts })
    }
}

/// A template as one expression: itself when it is a single `{{ … }}`, otherwise
/// its parts joined as strings (`'' + a + ' and ' + b`, which is how templates join).
pub(crate) fn to_expr(t: Template) -> Expr {
    let mut parts = t.parts.into_iter();
    if parts.len() == 1 {
        return match parts.next() {
            Some(Part::Expr(e)) => e,
            Some(Part::Lit(s)) => Expr::Lit(Json::String(s)),
            None => unreachable!(),
        };
    }
    parts.fold(Expr::Lit(Json::String(String::new())), |acc, p| {
        let next = match p {
            Part::Lit(s) => Expr::Lit(Json::String(s)),
            Part::Expr(e) => e,
        };
        Expr::Binary(expr::BinOp::Add, Box::new(acc), Box::new(next))
    })
}

/// A template whose every part is now literal becomes a literal value, so the
/// usual checks (prop types, state keys, URL policy, CSS) apply to it.
pub(crate) fn simplify(t: Template) -> Val {
    let literal = |p: &Part| matches!(p, Part::Lit(_) | Part::Expr(Expr::Lit(_)));
    if !t.parts.iter().all(literal) {
        return Val::Tpl(t);
    }
    if let [Part::Expr(Expr::Lit(v))] = t.parts.as_slice() {
        return Val::Lit(v.clone());
    }
    let text: String = t
        .parts
        .iter()
        .map(|p| match p {
            Part::Lit(s) => s.clone(),
            Part::Expr(Expr::Lit(v)) => expr::stringify(v),
            Part::Expr(_) => unreachable!(),
        })
        .collect();
    Val::Lit(Json::String(text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Arguments as `with` holds them: strings may be templates, anything else is literal.
    fn params(pairs: &[(&str, Json)]) -> Params {
        let arg = |v: &Json| match v {
            Json::String(s) => to_expr(Template::parse(s).unwrap_or_else(|e| panic!("{s}: {e}"))),
            other => Expr::Lit(other.clone()),
        };
        Params::new(pairs.iter().map(|(k, v)| (k.to_string(), arg(v))).collect())
    }

    fn sub(p: &mut Params, src: &str) -> Result<Val, String> {
        p.template(&Template::parse(src).unwrap()).map(simplify)
    }

    fn eval(v: &Val, data: Json) -> Json {
        v.eval(&|path| expr::lookup(&data, path))
    }

    #[test]
    fn paths_extend_and_literals_fold() {
        let mut p = params(&[("who", json!("{{ user }}")), ("n", json!(3)), ("label", json!("Hi {{ user.name }}!"))]);
        let data = json!({ "user": { "name": "Ada", "age": 36 } });
        assert_eq!(eval(&sub(&mut p, "{{ who.name }} is {{ who.age + n }}").unwrap(), data.clone()), json!("Ada is 39"));
        // A mixed-text argument joins as text, even around numbers.
        assert_eq!(eval(&sub(&mut p, "{{ label }}").unwrap(), data.clone()), json!("Hi Ada!"));
        assert_eq!(sub(&mut p, "{{ n }}").unwrap(), Val::Lit(json!(3)));
        assert_eq!(sub(&mut p, "n = {{ n }}").unwrap(), Val::Lit(json!("n = 3")));
        // Names that aren't parameters are left for render time.
        assert!(matches!(sub(&mut p, "{{ other }}").unwrap(), Val::Tpl(_)));
        assert_eq!(p.unused().count(), 0);
    }

    #[test]
    fn literal_objects_have_fields_but_computed_values_do_not() {
        let mut p = params(&[("cfg", json!({ "size": "sm" }))]);
        assert_eq!(sub(&mut p, "{{ cfg.size }}").unwrap(), Val::Lit(json!("sm")));
        let mut p = params(&[("total", json!("{{ a + b }}"))]);
        assert_eq!(
            sub(&mut p, "{{ total.x }}").unwrap_err(),
            "`total` is a computed value, so `total.x` has no fields; pass a path or a literal"
        );
    }

    #[test]
    fn number_parts_join_as_text() {
        let e = to_expr(Template::parse("{{ 1 }}{{ 2 }}").unwrap());
        assert_eq!(e.eval(&|_| Json::Null), json!("12"));
    }
}
