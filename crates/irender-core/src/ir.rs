//! The validated tree renderers consume.

use crate::{Diagnostics, Labels, StyleBlock, Theme, Val};
use indexmap::IndexMap;
use serde_json::Value as Json;

#[derive(Debug, Clone)]
pub struct Ir {
    pub version: u32,
    pub theme: Theme,
    /// The document's `lang` and `labels`.
    pub labels: Labels,
    /// The document's `app`.
    pub app: Option<crate::AppSpec>,
    pub state: IndexMap<String, Json>,
    pub root: Node,
    /// Non-fatal problems found while loading.
    pub warnings: Diagnostics,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub kind: String,
    /// Where this node came from, for error messages (`root.children[1]`).
    pub path: String,
    pub props: IndexMap<String, Val>,
    pub attrs: IndexMap<String, Val>,
    pub class: Vec<Val>,
    pub style: StyleBlock,
    pub parts: IndexMap<String, PartStyle>,
    pub on: IndexMap<String, Vec<Action>>,
    pub children: Vec<Node>,
}

impl Node {
    pub fn prop(&self, name: &str) -> Option<&Val> {
        self.props.get(name)
    }
}

#[derive(Debug, Clone, Default)]
pub struct PartStyle {
    pub attrs: IndexMap<String, Val>,
    pub class: Vec<Val>,
    pub style: StyleBlock,
}

#[derive(Debug, Clone)]
pub enum Action {
    SetState { key: String, value: Val },
    Navigate { url: Val },
    /// `invalidates`: source → ids (an id or a list), or `true` for all its records.
    Submit { endpoint: Val, method: String, body: IndexMap<String, Val>, into: Option<String>, invalidates: IndexMap<String, Val> },
    Call { name: String, args: IndexMap<String, Val> },
    Wait { ms: u64 },
}
