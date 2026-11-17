/// The type a component prop accepts. Only literal values are checked; templates are
/// checked at render time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropKind {
    Any,
    String,
    Number,
    Bool,
    Enum(&'static [&'static str]),
    /// The name of a key declared in the document's `state`.
    StateKey,
    /// An HTML tag name, checked against the [`Policy`](crate::Policy).
    Tag,
}

impl PropKind {
    pub fn describe(self) -> String {
        match self {
            PropKind::Any => "any value".into(),
            PropKind::String => "a string".into(),
            PropKind::Number => "a number".into(),
            PropKind::Bool => "a boolean".into(),
            PropKind::Enum(opts) => format!("one of: {}", opts.join(", ")),
            PropKind::StateKey => "a key declared in `state`".into(),
            PropKind::Tag => "an HTML tag name".into(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PropDef {
    pub name: &'static str,
    pub kind: PropKind,
    pub required: bool,
    pub doc: &'static str,
}

impl PropDef {
    pub const fn new(name: &'static str, kind: PropKind, doc: &'static str) -> Self {
        Self { name, kind, required: false, doc }
    }

    pub const fn required(mut self) -> Self {
        self.required = true;
        self
    }
}

/// What a component accepts, used to validate documents.
#[derive(Debug, Clone, Copy)]
pub struct ComponentInfo {
    pub doc: &'static str,
    pub props: &'static [PropDef],
    /// Named inner elements that can be styled through `parts`.
    pub parts: &'static [&'static str],
    pub children: bool,
    /// Accept props that are not listed in `props`.
    pub open_props: bool,
}

/// A set of known components. Renderers implement this for their registries.
pub trait Catalog {
    fn component(&self, name: &str) -> Option<ComponentInfo>;
}
