//! The serde model of a config file. Any serde format can produce it.

use crate::{Diagnostics, theme::ThemeSpec};
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value as Json;
use std::path::Path;

pub const VERSION: u32 = 1;

/// A UI document: theme, initial client state and a tree of nodes.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Document {
    /// Document format version. Currently `1`.
    pub version: u32,
    #[serde(default)]
    pub theme: ThemeSpec,
    /// Initial client-side state. Expressions that reference these keys are reactive.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub state: IndexMap<String, Json>,
    /// Named subtrees that nodes pull in with `{ "type": "include", "props": { "name": … } }`.
    /// Looked up before the engine's [`Fragments`](crate::Fragments) source.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub fragments: IndexMap<String, NodeSpec>,
    pub root: NodeSpec,
}

/// One element of the UI tree.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NodeSpec {
    /// Component name, e.g. `stack`, `text`, `button`, or `element` for any HTML tag.
    #[serde(rename = "type")]
    pub kind: String,
    /// Component props. Strings may contain `{{ expressions }}`.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub props: IndexMap<String, Json>,
    /// Any HTML attribute (`id`, `aria-*`, `data-*`, …). Wins over attributes the component sets.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub attrs: IndexMap<String, Json>,
    /// Extra class names.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub class: Vec<String>,
    /// Any CSS property. Nest `:hover`, `&.x`, `@md`, `@dark` or `@media …` blocks.
    /// Values may use `$theme.tokens` and `{{ expressions }}`.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    #[schemars(with = "IndexMap<String, Json>")]
    pub style: IndexMap<String, Json>,
    /// Attributes, classes and styles for the component's inner elements.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub parts: IndexMap<String, PartSpec>,
    /// DOM event name → action(s).
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub on: IndexMap<String, OneOrMany<ActionSpec>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<NodeSpec>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PartSpec {
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub attrs: IndexMap<String, Json>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub class: Vec<String>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub style: IndexMap<String, Json>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum OneOrMany<T> {
    One(T),
    Many(Vec<T>),
}

impl<T> OneOrMany<T> {
    pub fn as_slice(&self) -> &[T] {
        match self {
            OneOrMany::One(t) => std::slice::from_ref(t),
            OneOrMany::Many(v) => v,
        }
    }
}

/// A declarative action. Configs never contain code; `call` hands off to the host app.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum ActionSpec {
    /// Set a `state` key to a value (usually an expression).
    SetState { key: String, value: Json },
    /// Go to a URL.
    Navigate { url: String },
    /// Send `body` as JSON (or a query string for GET). Optionally store the JSON response in `into`.
    Submit {
        endpoint: String,
        #[serde(default = "default_method")]
        method: String,
        #[serde(default)]
        body: IndexMap<String, Json>,
        #[serde(default)]
        into: Option<String>,
    },
    /// Dispatch an `irender:call` DOM event with `{ name, args, state }` for the host app to handle.
    Call {
        name: String,
        #[serde(default)]
        args: IndexMap<String, Json>,
    },
}

fn default_method() -> String {
    "POST".into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Json,
    #[cfg(feature = "yaml")]
    Yaml,
}

impl Format {
    pub fn from_path(path: &Path) -> Option<Format> {
        match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
            "json" => Some(Format::Json),
            #[cfg(feature = "yaml")]
            "yaml" | "yml" => Some(Format::Yaml),
            _ => None,
        }
    }
}

/// Parses a document. Errors carry the path of the offending field.
pub fn parse(src: &str, format: Format) -> Result<Document, Diagnostics> {
    parse_as(src, format)
}

/// Parses any deserializable type (e.g. a data file) with path-aware errors.
pub fn parse_as<T: DeserializeOwned>(src: &str, format: Format) -> Result<T, Diagnostics> {
    fn diag<E: std::fmt::Display>(e: serde_path_to_error::Error<E>) -> Diagnostics {
        let path = e.path().to_string();
        let path = if path == "." { String::new() } else { path };
        Diagnostics::single(path, e.inner().to_string())
    }
    match format {
        Format::Json => {
            let de = &mut serde_json::Deserializer::from_str(src);
            serde_path_to_error::deserialize(de).map_err(diag)
        }
        #[cfg(feature = "yaml")]
        Format::Yaml => {
            let de = serde_yaml_ng::Deserializer::from_str(src);
            serde_path_to_error::deserialize(de).map_err(diag)
        }
    }
}
