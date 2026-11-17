//! A deliberately small expression language used inside `{{ … }}` templates.
//!
//! Supported: paths (`product.price`, `items[0]`, `row['first-name']`, `list.length`),
//! string / number / bool / null literals, `+ - * / %`, `== != < <= > >=`,
//! `&& || !`, `?:`, parentheses and the built-in functions in [`Func`]. Nothing else —
//! configs may be untrusted, so the language has no user-defined calls, assignment
//! or loops, and every built-in is bounded.
//!
//! The same AST is evaluated here (server-side) and by the browser runtime in
//! `irender-html/src/runtime.js`. Any change to the semantics must be mirrored there.

use serde_json::{Value as Json, json};
use std::{
    fmt,
    hash::{Hash, Hasher},
};

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Lit(Json),
    Path(Vec<String>),
    Unary(UnOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Cond(Box<Expr>, Box<Expr>, Box<Expr>),
    Call(Func, Vec<Expr>),
}

/// The built-in functions: a closed set, each with a bounded result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Func {
    /// `range(n)` → `[0, 1, …, n - 1]`; `range(start, end)` → `[start, …, end - 1]`.
    /// Arguments are truncated to integers; anything not a number counts as 0. At most
    /// [`MAX_RANGE`] items.
    Range,
}

/// The most items `range` returns.
pub const MAX_RANGE: usize = 1000;

impl Func {
    pub fn name(self) -> &'static str {
        match self {
            Func::Range => "range",
        }
    }

    fn from_name(name: &str) -> Option<Func> {
        match name {
            "range" => Some(Func::Range),
            _ => None,
        }
    }

    /// How many arguments it takes, at least and at most.
    fn arity(self) -> (usize, usize) {
        match self {
            Func::Range => (1, 2),
        }
    }

    fn call(self, args: &[Json]) -> Json {
        match self {
            Func::Range => {
                let int = |v: &Json| {
                    let n = to_num(v);
                    if n.is_finite() { n.trunc() } else { 0.0 }
                };
                let (start, end) = match args {
                    [n] => (0.0, int(n)),
                    [a, b, ..] => (int(a), int(b)),
                    [] => (0.0, 0.0),
                };
                let len = (end - start).clamp(0.0, MAX_RANGE as f64) as usize;
                Json::Array((0..len).map(|i| num(start + i as f64)).collect())
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnOp {
    Not,
    Neg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

impl UnOp {
    pub fn symbol(self) -> &'static str {
        match self {
            UnOp::Not => "!",
            UnOp::Neg => "-",
        }
    }
}

impl BinOp {
    pub fn symbol(self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Rem => "%",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::And => "&&",
            BinOp::Or => "||",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message} (at offset {offset})")]
pub struct ParseError {
    pub message: String,
    pub offset: usize,
}

impl ParseError {
    fn new(message: impl Into<String>, offset: usize) -> Self {
        Self { message: message.into(), offset }
    }
}

impl Expr {
    pub fn parse(src: &str) -> Result<Expr, ParseError> {
        let mut p = Parser { toks: lex(src)?, pos: 0 };
        let e = p.ternary()?;
        match p.peek() {
            Tok::End => Ok(e),
            _ => Err(p.err("unexpected token")),
        }
    }

    pub fn eval(&self, lookup: &dyn Fn(&[String]) -> Json) -> Json {
        match self {
            Expr::Lit(v) => v.clone(),
            Expr::Path(p) => lookup(p),
            Expr::Unary(UnOp::Not, a) => Json::Bool(!truthy(&a.eval(lookup))),
            Expr::Unary(UnOp::Neg, a) => num(-to_num(&a.eval(lookup))),
            Expr::Cond(c, a, b) => {
                if truthy(&c.eval(lookup)) {
                    a.eval(lookup)
                } else {
                    b.eval(lookup)
                }
            }
            Expr::Call(f, args) => f.call(&args.iter().map(|a| a.eval(lookup)).collect::<Vec<_>>()),
            Expr::Binary(op, l, r) => {
                let lv = l.eval(lookup);
                match op {
                    BinOp::And => return if truthy(&lv) { r.eval(lookup) } else { lv },
                    BinOp::Or => return if truthy(&lv) { lv } else { r.eval(lookup) },
                    _ => {}
                }
                let rv = r.eval(lookup);
                use std::cmp::Ordering::*;
                match op {
                    BinOp::Add if lv.is_string() || rv.is_string() => {
                        Json::String(stringify(&lv) + &stringify(&rv))
                    }
                    BinOp::Add => num(to_num(&lv) + to_num(&rv)),
                    BinOp::Sub => num(to_num(&lv) - to_num(&rv)),
                    BinOp::Mul => num(to_num(&lv) * to_num(&rv)),
                    BinOp::Div => num(to_num(&lv) / to_num(&rv)),
                    BinOp::Rem => num(to_num(&lv) % to_num(&rv)),
                    BinOp::Eq => Json::Bool(loose_eq(&lv, &rv)),
                    BinOp::Ne => Json::Bool(!loose_eq(&lv, &rv)),
                    BinOp::Lt => Json::Bool(compare(&lv, &rv) == Some(Less)),
                    BinOp::Le => Json::Bool(matches!(compare(&lv, &rv), Some(Less | Equal))),
                    BinOp::Gt => Json::Bool(compare(&lv, &rv) == Some(Greater)),
                    BinOp::Ge => Json::Bool(matches!(compare(&lv, &rv), Some(Greater | Equal))),
                    BinOp::And | BinOp::Or => unreachable!(),
                }
            }
        }
    }

    /// Calls `f` for every path in the expression.
    pub fn visit_paths(&self, f: &mut dyn FnMut(&[String])) {
        match self {
            Expr::Lit(_) => {}
            Expr::Path(p) => f(p),
            Expr::Unary(_, a) => a.visit_paths(f),
            Expr::Binary(_, l, r) => {
                l.visit_paths(f);
                r.visit_paths(f);
            }
            Expr::Cond(c, a, b) => {
                c.visit_paths(f);
                a.visit_paths(f);
                b.visit_paths(f);
            }
            Expr::Call(_, args) => args.iter().for_each(|a| a.visit_paths(f)),
        }
    }

    /// Replaces every path whose root is not `keep`-ed with its current value.
    ///
    /// Renderers use this to bake server-side data into client-side expressions so the
    /// browser only ever needs the reactive `state`.
    pub fn fold(&self, keep: &dyn Fn(&str) -> bool, lookup: &dyn Fn(&[String]) -> Json) -> Expr {
        match self {
            Expr::Path(p) if !keep(&p[0]) => Expr::Lit(lookup(p)),
            Expr::Path(_) | Expr::Lit(_) => self.clone(),
            Expr::Unary(op, a) => Expr::Unary(*op, Box::new(a.fold(keep, lookup))),
            Expr::Binary(op, l, r) => {
                Expr::Binary(*op, Box::new(l.fold(keep, lookup)), Box::new(r.fold(keep, lookup)))
            }
            Expr::Cond(c, a, b) => Expr::Cond(
                Box::new(c.fold(keep, lookup)),
                Box::new(a.fold(keep, lookup)),
                Box::new(b.fold(keep, lookup)),
            ),
            Expr::Call(f, args) => Expr::Call(*f, args.iter().map(|a| a.fold(keep, lookup)).collect()),
        }
    }

    /// Compact JSON form consumed by the browser runtime. Each node kind has a distinct
    /// discriminating key: `l` literal, `p` path, `u` unary, `b` binary, `c` conditional,
    /// `f` call (with its arguments in `a`).
    pub fn to_js(&self) -> Json {
        match self {
            Expr::Lit(v) => json!({ "l": v }),
            Expr::Path(p) => json!({ "p": p }),
            Expr::Unary(op, a) => json!({ "u": op.symbol(), "a": a.to_js() }),
            Expr::Binary(op, l, r) => json!({ "b": op.symbol(), "x": l.to_js(), "y": r.to_js() }),
            Expr::Cond(c, a, b) => json!({ "c": [c.to_js(), a.to_js(), b.to_js()] }),
            Expr::Call(f, args) => json!({ "f": f.name(), "a": args.iter().map(Expr::to_js).collect::<Vec<_>>() }),
        }
    }

    /// `self.fold(keep, lookup).to_js()` written straight to `out` as JSON text,
    /// without building intermediate values. Renderers use this on the hot path.
    pub fn write_js<W: fmt::Write + ?Sized>(
        &self,
        out: &mut W,
        keep: &dyn Fn(&str) -> bool,
        lookup: &dyn Fn(&[String]) -> Json,
    ) -> fmt::Result {
        match self {
            Expr::Lit(v) => write!(out, "{{\"l\":{v}}}"),
            Expr::Path(p) if !keep(&p[0]) => write!(out, "{{\"l\":{}}}", lookup(p)),
            Expr::Path(p) => {
                out.write_str("{\"p\":[")?;
                for (i, seg) in p.iter().enumerate() {
                    if i > 0 {
                        out.write_char(',')?;
                    }
                    write_json_str(out, seg)?;
                }
                out.write_str("]}")
            }
            Expr::Unary(op, a) => {
                write!(out, "{{\"u\":\"{}\",\"a\":", op.symbol())?;
                a.write_js(out, keep, lookup)?;
                out.write_char('}')
            }
            Expr::Binary(op, l, r) => {
                write!(out, "{{\"b\":\"{}\",\"x\":", op.symbol())?;
                l.write_js(out, keep, lookup)?;
                out.write_str(",\"y\":")?;
                r.write_js(out, keep, lookup)?;
                out.write_char('}')
            }
            Expr::Cond(c, a, b) => {
                out.write_str("{\"c\":[")?;
                c.write_js(out, keep, lookup)?;
                out.write_char(',')?;
                a.write_js(out, keep, lookup)?;
                out.write_char(',')?;
                b.write_js(out, keep, lookup)?;
                out.write_str("]}")
            }
            Expr::Call(f, args) => {
                write!(out, "{{\"f\":\"{}\",\"a\":[", f.name())?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        out.write_char(',')?;
                    }
                    a.write_js(out, keep, lookup)?;
                }
                out.write_str("]}")
            }
        }
    }
}

/// Writes `s` as a JSON string literal.
pub fn write_json_str<W: fmt::Write + ?Sized>(out: &mut W, s: &str) -> fmt::Result {
    out.write_char('"')?;
    let mut start = 0;
    for (i, c) in s.char_indices() {
        let escaped = match c {
            '"' => "\\\"",
            '\\' => "\\\\",
            '\n' => "\\n",
            '\r' => "\\r",
            '\t' => "\\t",
            c if (c as u32) < 0x20 => {
                out.write_str(&s[start..i])?;
                write!(out, "\\u{:04x}", c as u32)?;
                start = i + 1;
                continue;
            }
            _ => continue,
        };
        out.write_str(&s[start..i])?;
        out.write_str(escaped)?;
        start = i + 1;
    }
    out.write_str(&s[start..])?;
    out.write_char('"')
}

/// Hashes a JSON value consistently with its `PartialEq`.
pub fn hash_json<H: Hasher>(v: &Json, state: &mut H) {
    std::mem::discriminant(v).hash(state);
    match v {
        Json::Null => {}
        Json::Bool(b) => b.hash(state),
        Json::Number(n) => n.hash(state),
        Json::String(s) => s.hash(state),
        Json::Array(a) => {
            a.len().hash(state);
            a.iter().for_each(|v| hash_json(v, state));
        }
        Json::Object(o) => {
            o.len().hash(state);
            for (k, v) in o {
                k.hash(state);
                hash_json(v, state);
            }
        }
    }
}

impl Hash for Expr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Expr::Lit(v) => hash_json(v, state),
            Expr::Path(p) => p.hash(state),
            Expr::Unary(op, a) => {
                op.hash(state);
                a.hash(state);
            }
            Expr::Binary(op, l, r) => {
                op.hash(state);
                l.hash(state);
                r.hash(state);
            }
            Expr::Cond(c, a, b) => {
                c.hash(state);
                a.hash(state);
                b.hash(state);
            }
            Expr::Call(f, args) => {
                f.hash(state);
                args.hash(state);
            }
        }
    }
}

/// A string with `{{ expr }}` holes.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Part {
    Lit(String),
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Template {
    pub parts: Vec<Part>,
}

impl Template {
    pub fn is_template(s: &str) -> bool {
        s.contains("{{")
    }

    pub fn parse(src: &str) -> Result<Template, ParseError> {
        let mut parts = Vec::new();
        let mut rest = src;
        let mut base = 0;
        while let Some(start) = rest.find("{{") {
            if start > 0 {
                parts.push(Part::Lit(rest[..start].to_string()));
            }
            let inner = &rest[start + 2..];
            let end = inner
                .find("}}")
                .ok_or_else(|| ParseError::new("unclosed `{{`", base + start))?;
            let expr = Expr::parse(&inner[..end])
                .map_err(|e| ParseError::new(e.message, base + start + 2 + e.offset))?;
            parts.push(Part::Expr(expr));
            let consumed = start + 2 + end + 2;
            base += consumed;
            rest = &rest[consumed..];
        }
        if !rest.is_empty() {
            parts.push(Part::Lit(rest.to_string()));
        }
        Ok(Template { parts })
    }

    /// `"{{ x }}"` evaluates to the raw value of `x` (number, bool, …), not a string.
    pub fn as_single_expr(&self) -> Option<&Expr> {
        match self.parts.as_slice() {
            [Part::Expr(e)] => Some(e),
            _ => None,
        }
    }

    pub fn eval(&self, lookup: &dyn Fn(&[String]) -> Json) -> Json {
        if let Some(e) = self.as_single_expr() {
            return e.eval(lookup);
        }
        let mut s = String::new();
        for part in &self.parts {
            match part {
                Part::Lit(l) => s.push_str(l),
                Part::Expr(e) => s.push_str(&stringify(&e.eval(lookup))),
            }
        }
        Json::String(s)
    }

    pub fn exprs(&self) -> impl Iterator<Item = &Expr> {
        self.parts.iter().filter_map(|p| match p {
            Part::Expr(e) => Some(e),
            Part::Lit(_) => None,
        })
    }

    /// Whether any path in the template has a root matching `pred`.
    pub fn references(&self, pred: &dyn Fn(&str) -> bool) -> bool {
        let mut found = false;
        for e in self.exprs() {
            e.visit_paths(&mut |p| found |= pred(&p[0]));
        }
        found
    }

    pub fn fold(&self, keep: &dyn Fn(&str) -> bool, lookup: &dyn Fn(&[String]) -> Json) -> Template {
        let parts = self
            .parts
            .iter()
            .map(|p| match p {
                Part::Lit(l) => Part::Lit(l.clone()),
                Part::Expr(e) => Part::Expr(e.fold(keep, lookup)),
            })
            .collect();
        Template { parts }
    }

    pub fn to_js(&self) -> Json {
        if let Some(e) = self.as_single_expr() {
            return json!({ "r": e.to_js() });
        }
        let parts: Vec<Json> = self
            .parts
            .iter()
            .map(|p| match p {
                Part::Lit(l) => Json::String(l.clone()),
                Part::Expr(e) => e.to_js(),
            })
            .collect();
        json!({ "t": parts })
    }

    /// `self.fold(keep, lookup).to_js()` written straight to `out` as JSON text.
    pub fn write_js<W: fmt::Write + ?Sized>(
        &self,
        out: &mut W,
        keep: &dyn Fn(&str) -> bool,
        lookup: &dyn Fn(&[String]) -> Json,
    ) -> fmt::Result {
        if let Some(e) = self.as_single_expr() {
            out.write_str("{\"r\":")?;
            e.write_js(out, keep, lookup)?;
            return out.write_char('}');
        }
        out.write_str("{\"t\":[")?;
        for (i, part) in self.parts.iter().enumerate() {
            if i > 0 {
                out.write_char(',')?;
            }
            match part {
                Part::Lit(l) => write_json_str(out, l)?,
                Part::Expr(e) => e.write_js(out, keep, lookup)?,
            }
        }
        out.write_str("]}")
    }
}

// ---- value semantics (mirrored in runtime.js) ----

pub fn truthy(v: &Json) -> bool {
    match v {
        Json::Null => false,
        Json::Bool(b) => *b,
        Json::Number(n) => n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()),
        Json::String(s) => !s.is_empty(),
        Json::Array(_) | Json::Object(_) => true,
    }
}

pub fn to_num(v: &Json) -> f64 {
    match v {
        Json::Null => 0.0,
        Json::Bool(b) => f64::from(u8::from(*b)),
        Json::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Json::String(s) if s.trim().is_empty() => f64::NAN,
        Json::String(s) => s.trim().parse().unwrap_or(f64::NAN),
        Json::Array(_) | Json::Object(_) => f64::NAN,
    }
}

/// Number → JSON. Whole numbers become integers; NaN and infinities become `null`.
pub fn num(f: f64) -> Json {
    if !f.is_finite() {
        Json::Null
    } else if f.fract() == 0.0 && f.abs() < 9_007_199_254_740_992.0 {
        Json::from(f as i64)
    } else {
        serde_json::Number::from_f64(f).map_or(Json::Null, Json::Number)
    }
}

/// The text form of a value, as it appears when interpolated into a string.
pub fn stringify(v: &Json) -> String {
    match v {
        Json::Null => String::new(),
        Json::String(s) => s.clone(),
        Json::Bool(b) => b.to_string(),
        Json::Number(n) => match n.as_i64() {
            Some(i) => i.to_string(),
            None => match num(n.as_f64().unwrap_or(f64::NAN)) {
                Json::Number(n) if n.is_i64() => n.to_string(),
                _ => format!("{}", n.as_f64().unwrap_or(0.0)),
            },
        },
        Json::Array(_) | Json::Object(_) => v.to_string(),
    }
}

fn loose_eq(a: &Json, b: &Json) -> bool {
    match (a, b) {
        (Json::Number(x), Json::Number(y)) => x.as_f64() == y.as_f64(),
        _ => a == b,
    }
}

fn compare(a: &Json, b: &Json) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Json::String(x), Json::String(y)) => Some(x.cmp(y)),
        _ => to_num(a).partial_cmp(&to_num(b)),
    }
}

/// Walks `path` into `root`. Missing keys yield `null`; `.length` works on arrays and strings.
pub fn lookup(root: &Json, path: &[String]) -> Json {
    let mut cur = root;
    for (i, seg) in path.iter().enumerate() {
        let last = i + 1 == path.len();
        match cur {
            Json::Object(m) => match m.get(seg) {
                Some(v) => cur = v,
                None => return Json::Null,
            },
            Json::Array(a) if seg == "length" => {
                return if last { Json::from(a.len()) } else { Json::Null };
            }
            Json::Array(a) => match seg.parse::<usize>().ok().and_then(|n| a.get(n)) {
                Some(v) => cur = v,
                None => return Json::Null,
            },
            Json::String(s) if seg == "length" => {
                return if last { Json::from(s.encode_utf16().count()) } else { Json::Null };
            }
            _ => return Json::Null,
        }
    }
    cur.clone()
}

// ---- lexer ----

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Str(String),
    Ident(String),
    Op(&'static str),
    End,
}

const OPS2: [&str; 6] = ["==", "!=", "<=", ">=", "&&", "||"];
const OPS1: [&str; 16] = ["+", "-", "*", "/", "%", "<", ">", "!", "?", ":", "(", ")", ".", "[", "]", ","];

fn lex(src: &str) -> Result<Vec<(Tok, usize)>, ParseError> {
    let mut out: Vec<(Tok, usize)> = Vec::new();
    let mut chars = src.char_indices().peekable();
    while let Some(&(i, c)) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }
        if c.is_ascii_digit() {
            // After `.` only an integer is allowed, so `rows.1.2` is a path, not `rows . 1.2`.
            let int_only = matches!(out.last(), Some((Tok::Op("."), _)));
            let mut end = i;
            let mut seen_dot = false;
            while let Some(&(j, d)) = chars.peek() {
                if d.is_ascii_digit() {
                    end = j + 1;
                    chars.next();
                } else if d == '.'
                    && !int_only
                    && !seen_dot
                    && src[j + 1..].starts_with(|x: char| x.is_ascii_digit())
                {
                    seen_dot = true;
                    end = j + 1;
                    chars.next();
                } else {
                    break;
                }
            }
            let n = src[i..end].parse().map_err(|_| ParseError::new("invalid number", i))?;
            out.push((Tok::Num(n), i));
            continue;
        }
        if c == '\'' || c == '"' {
            chars.next();
            let mut s = String::new();
            loop {
                match chars.next() {
                    None => return Err(ParseError::new("unterminated string", i)),
                    Some((_, q)) if q == c => break,
                    Some((_, '\\')) => match chars.next() {
                        Some((_, 'n')) => s.push('\n'),
                        Some((_, 't')) => s.push('\t'),
                        Some((_, e)) => s.push(e),
                        None => return Err(ParseError::new("unterminated string", i)),
                    },
                    Some((_, ch)) => s.push(ch),
                }
            }
            out.push((Tok::Str(s), i));
            continue;
        }
        if c.is_alphabetic() || c == '_' || c == '$' {
            let mut end = i;
            while let Some(&(j, d)) = chars.peek() {
                if d.is_alphanumeric() || d == '_' || d == '$' {
                    end = j + d.len_utf8();
                    chars.next();
                } else {
                    break;
                }
            }
            out.push((Tok::Ident(src[i..end].to_string()), i));
            continue;
        }
        let rest = &src[i..];
        if let Some(op) = OPS2.iter().find(|op| rest.starts_with(**op)) {
            out.push((Tok::Op(op), i));
            chars.next();
            chars.next();
            continue;
        }
        if let Some(op) = OPS1.iter().find(|op| rest.starts_with(**op)) {
            out.push((Tok::Op(op), i));
            chars.next();
            continue;
        }
        return Err(ParseError::new(format!("unexpected character `{c}`"), i));
    }
    out.push((Tok::End, src.len()));
    Ok(out)
}

// ---- parser ----

struct Parser {
    toks: Vec<(Tok, usize)>,
    pos: usize,
}

fn bin_op(o: &str) -> Option<(BinOp, u8)> {
    Some(match o {
        "||" => (BinOp::Or, 1),
        "&&" => (BinOp::And, 2),
        "==" => (BinOp::Eq, 3),
        "!=" => (BinOp::Ne, 3),
        "<" => (BinOp::Lt, 4),
        "<=" => (BinOp::Le, 4),
        ">" => (BinOp::Gt, 4),
        ">=" => (BinOp::Ge, 4),
        "+" => (BinOp::Add, 5),
        "-" => (BinOp::Sub, 5),
        "*" => (BinOp::Mul, 6),
        "/" => (BinOp::Div, 6),
        "%" => (BinOp::Rem, 6),
        _ => return None,
    })
}

impl Parser {
    fn peek(&self) -> &Tok {
        &self.toks[self.pos].0
    }

    fn offset(&self) -> usize {
        self.toks[self.pos].1
    }

    fn next(&mut self) -> Tok {
        let t = self.toks[self.pos].0.clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn eat(&mut self, op: &str) -> bool {
        if matches!(self.peek(), Tok::Op(o) if *o == op) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, op: &str) -> Result<(), ParseError> {
        if self.eat(op) { Ok(()) } else { Err(self.err(format!("expected `{op}`"))) }
    }

    fn err(&self, message: impl Into<String>) -> ParseError {
        ParseError::new(message, self.offset())
    }

    fn ternary(&mut self) -> Result<Expr, ParseError> {
        let cond = self.binary(1)?;
        if !self.eat("?") {
            return Ok(cond);
        }
        let a = self.ternary()?;
        self.expect(":")?;
        let b = self.ternary()?;
        Ok(Expr::Cond(Box::new(cond), Box::new(a), Box::new(b)))
    }

    fn binary(&mut self, min: u8) -> Result<Expr, ParseError> {
        let mut left = self.unary()?;
        while let Tok::Op(o) = self.peek() {
            let Some((op, prec)) = bin_op(o) else { break };
            if prec < min {
                break;
            }
            self.pos += 1;
            let right = self.binary(prec + 1)?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expr, ParseError> {
        if self.eat("!") {
            return Ok(Expr::Unary(UnOp::Not, Box::new(self.unary()?)));
        }
        if self.eat("-") {
            return Ok(Expr::Unary(UnOp::Neg, Box::new(self.unary()?)));
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<Expr, ParseError> {
        let at = self.offset();
        match self.next() {
            Tok::Num(n) => Ok(Expr::Lit(num(n))),
            Tok::Str(s) => Ok(Expr::Lit(Json::String(s))),
            Tok::Ident(id) => match id.as_str() {
                "true" => Ok(Expr::Lit(Json::Bool(true))),
                "false" => Ok(Expr::Lit(Json::Bool(false))),
                "null" => Ok(Expr::Lit(Json::Null)),
                _ if matches!(self.peek(), Tok::Op("(")) => self.call(&id, at),
                _ => self.path(id),
            },
            Tok::Op("(") => {
                let e = self.ternary()?;
                self.expect(")")?;
                Ok(e)
            }
            Tok::End => Err(ParseError::new("unexpected end of expression", at)),
            _ => Err(ParseError::new("unexpected token", at)),
        }
    }

    fn call(&mut self, name: &str, at: usize) -> Result<Expr, ParseError> {
        let f = Func::from_name(name)
            .ok_or_else(|| ParseError::new(format!("unknown function `{name}`; the only one is `range`"), at))?;
        self.expect("(")?;
        let mut args = Vec::new();
        if !self.eat(")") {
            loop {
                args.push(self.ternary()?);
                if self.eat(")") {
                    break;
                }
                self.expect(",")?;
            }
        }
        let (min, max) = f.arity();
        if args.len() < min || args.len() > max {
            let takes = if min == max { min.to_string() } else { format!("{min} or {max}") };
            return Err(ParseError::new(format!("`{name}` takes {takes} argument(s)"), at));
        }
        Ok(Expr::Call(f, args))
    }

    fn path(&mut self, root: String) -> Result<Expr, ParseError> {
        let index = |n: f64| (n.fract() == 0.0 && n >= 0.0).then(|| (n as u64).to_string());
        let mut segs = vec![root];
        loop {
            if self.eat(".") {
                let at = self.offset();
                match self.next() {
                    Tok::Ident(s) => segs.push(s),
                    Tok::Num(n) if index(n).is_some() => segs.extend(index(n)),
                    _ => return Err(ParseError::new("expected a property name after `.`", at)),
                }
            } else if self.eat("[") {
                let at = self.offset();
                match self.next() {
                    Tok::Str(s) => segs.push(s),
                    Tok::Num(n) if index(n).is_some() => segs.extend(index(n)),
                    _ => return Err(ParseError::new("expected a string or index inside `[]`", at)),
                }
                self.expect("]")?;
            } else {
                return Ok(Expr::Path(segs));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(src: &str, data: Json) -> Json {
        Expr::parse(src).unwrap().eval(&|p| lookup(&data, p))
    }

    #[test]
    fn arithmetic_and_precedence() {
        assert_eq!(eval("1 + 2 * 3", Json::Null), json!(7));
        assert_eq!(eval("(1 + 2) * 3", Json::Null), json!(9));
        assert_eq!(eval("7 % 4 - -1", Json::Null), json!(4));
        assert_eq!(eval("1 / 0", Json::Null), Json::Null);
        assert_eq!(eval("0.1 + 0.2", Json::Null), json!(0.30000000000000004));
    }

    #[test]
    fn paths_and_strings() {
        let data = json!({ "p": { "name": "Mug", "tags": ["a", "b"], "first-name": "X" } });
        assert_eq!(eval("p.name + '!'", data.clone()), json!("Mug!"));
        assert_eq!(eval("p.tags[1]", data.clone()), json!("b"));
        assert_eq!(eval("p.tags.0", data.clone()), json!("a"));
        assert_eq!(eval("p.tags.length", data.clone()), json!(2));
        assert_eq!(eval("p['first-name']", data.clone()), json!("X"));
        assert_eq!(eval("p.missing.deep", data), Json::Null);
    }

    #[test]
    fn logic_and_ternary() {
        let data = json!({ "qty": 3 });
        assert_eq!(eval("qty > 1 && qty < 5", data.clone()), json!(true));
        assert_eq!(eval("qty == 1 ? 'one' : 'many'", data.clone()), json!("many"));
        assert_eq!(eval("missing || 'fallback'", data.clone()), json!("fallback"));
        assert_eq!(eval("!qty", data), json!(false));
    }

    #[test]
    fn templates() {
        let data = json!({ "n": 2, "ok": true });
        let t = Template::parse("₹{{ n * 10 }} total").unwrap();
        assert_eq!(t.eval(&|p| lookup(&data, p)), json!("₹20 total"));
        let raw = Template::parse("{{ ok }}").unwrap();
        assert_eq!(raw.eval(&|p| lookup(&data, p)), json!(true));
        assert!(Template::parse("{{ n ").is_err());
        assert!(Template::parse("{{ }}").is_err());
    }

    #[test]
    fn fold_keeps_only_reactive_roots() {
        let data = json!({ "price": 5, "qty": 2 });
        let e = Expr::parse("price * qty").unwrap();
        let folded = e.fold(&|r| r == "qty", &|p| lookup(&data, p));
        assert_eq!(folded.to_js(), json!({ "b": "*", "x": { "l": 5 }, "y": { "p": ["qty"] } }));
    }

    #[test]
    fn write_js_matches_fold_to_js() {
        let data = json!({ "price": 5, "name": "a\"b\\c\n\u{1}", "obj": { "k": [1, null] } });
        let keep = |r: &str| r == "qty";
        let lookup = |p: &[String]| lookup(&data, p);
        for src in [
            "{{ price * qty }}",
            "x{{ qty }}y",
            "{{ !qty ? name : obj }}",
            "{{ -qty + obj.k[1] }}",
            "\"quoted\" {{ qty['we\\ird'] }}",
            "{{ qty && 'a\\tb' || 1.5 }}",
        ] {
            let t = Template::parse(src).unwrap();
            let mut out = String::new();
            t.write_js(&mut out, &keep, &lookup).unwrap();
            let parsed: Json = serde_json::from_str(&out).unwrap_or_else(|e| panic!("{src}: {e}\n{out}"));
            assert_eq!(parsed, t.fold(&keep, &lookup).to_js(), "{src}");
        }
    }

    #[test]
    fn parse_errors() {
        assert!(Expr::parse("a +").is_err());
        assert!(Expr::parse("a b").is_err());
        assert!(Expr::parse("a.").is_err());
        assert!(Expr::parse("foo(1)").is_err());
        assert!(Expr::parse("range()").is_err());
        assert!(Expr::parse("range(1, 2, 3)").is_err());
        assert!(Expr::parse("range(1,)").is_err());
        assert!(Expr::parse("a, b").is_err());
    }

    #[test]
    fn range() {
        let data = json!({ "n": 3, "s": "2", "big": 1e9 });
        assert_eq!(eval("range(n)", data.clone()), json!([0, 1, 2]));
        assert_eq!(eval("range(1, n + 1)", data.clone()), json!([1, 2, 3]));
        assert_eq!(eval("range(s)", data.clone()), json!([0, 1]));
        assert_eq!(eval("range(2.9)", data.clone()), json!([0, 1]));
        assert_eq!(eval("range(-1)", data.clone()), json!([]));
        assert_eq!(eval("range(5, 2)", data.clone()), json!([]));
        assert_eq!(eval("range(missing)", data.clone()), json!([]));
        assert!(Expr::parse("range(n).length").is_err(), "`.` follows paths only");
        assert_eq!(eval("range(big)", data.clone()).as_array().unwrap().len(), MAX_RANGE);
        assert_eq!(eval("range(n) == range(0, 3)", data), json!(true));
        let e = Expr::parse("range(n + 1)").unwrap();
        assert_eq!(e.to_js(), json!({ "f": "range", "a": [{ "b": "+", "x": { "p": ["n"] }, "y": { "l": 1 } }] }));
    }
}
