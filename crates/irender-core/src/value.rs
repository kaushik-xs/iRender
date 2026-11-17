use crate::expr::{ParseError, Template};
use serde_json::{Value as Json, json};
use std::{borrow::Cow, fmt};

/// A config value: either a literal or a `{{ template }}` evaluated at render time.
#[derive(Debug, Clone, PartialEq)]
pub enum Val {
    Lit(Json),
    Tpl(Template),
}

impl Val {
    /// Strings containing `{{` become templates; everything else is a literal.
    pub fn parse(v: &Json) -> Result<Val, ParseError> {
        match v {
            Json::String(s) if Template::is_template(s) => Template::parse(s).map(Val::Tpl),
            _ => Ok(Val::Lit(v.clone())),
        }
    }

    pub fn eval(&self, lookup: &dyn Fn(&[String]) -> Json) -> Json {
        match self {
            Val::Lit(v) => v.clone(),
            Val::Tpl(t) => t.eval(lookup),
        }
    }

    pub fn as_lit(&self) -> Option<&Json> {
        match self {
            Val::Lit(v) => Some(v),
            Val::Tpl(_) => None,
        }
    }

    /// Client-side form, with every non-`keep` path folded into a literal.
    pub fn to_js(&self, keep: &dyn Fn(&str) -> bool, lookup: &dyn Fn(&[String]) -> Json) -> Json {
        match self {
            Val::Lit(v) => json!({ "r": { "l": v } }),
            Val::Tpl(t) => t.fold(keep, lookup).to_js(),
        }
    }

    /// [`Val::to_js`] written straight to `out` as JSON text.
    pub fn write_js<W: fmt::Write + ?Sized>(
        &self,
        out: &mut W,
        keep: &dyn Fn(&str) -> bool,
        lookup: &dyn Fn(&[String]) -> Json,
    ) -> fmt::Result {
        match self {
            Val::Lit(v) => write!(out, "{{\"r\":{{\"l\":{v}}}}}"),
            Val::Tpl(t) => t.write_js(out, keep, lookup),
        }
    }
}

impl From<Val> for Cow<'_, Val> {
    fn from(v: Val) -> Self {
        Cow::Owned(v)
    }
}

impl<'a> From<&'a Val> for Cow<'a, Val> {
    fn from(v: &'a Val) -> Self {
        Cow::Borrowed(v)
    }
}

impl From<Json> for Val {
    fn from(v: Json) -> Self {
        Val::Lit(v)
    }
}

impl From<&str> for Val {
    fn from(s: &str) -> Self {
        Val::Lit(Json::String(s.to_string()))
    }
}

impl From<String> for Val {
    fn from(s: String) -> Self {
        Val::Lit(Json::String(s))
    }
}

impl From<bool> for Val {
    fn from(b: bool) -> Self {
        Val::Lit(Json::Bool(b))
    }
}
