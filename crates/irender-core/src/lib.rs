//! Core of iRender: build UI from configuration.
//!
//! The pipeline is:
//!
//! ```text
//! source (JSON / YAML) ──parse──▶ Document ──resolve──▶ Ir ──▶ renderer (irender-html, …)
//! ```
//!
//! - [`Document`] is the serde model of a config file. Any serde format works.
//! - [`resolve`] validates a document against a [`Catalog`] of components and a
//!   [`Policy`], parses every `{{ expression }}`, resolves `$theme.tokens` and
//!   reports *all* problems with their path (`root.children[2].props.min`).
//! - [`Ir`] is the validated tree that renderers consume.
//!
//! This crate knows nothing about HTML; renderers live in their own crates.

pub mod catalog;
pub mod diag;
pub mod expr;
pub mod fragment;
mod params;
pub mod ir;
pub mod app;
pub mod labels;
pub mod policy;
pub mod resolve;
pub mod spec;
pub mod style;
pub mod theme;
pub mod value;

pub use catalog::{Catalog, ComponentInfo, PropDef, PropKind};
pub use diag::{Diagnostic, Diagnostics, Severity};
pub use expr::{Expr, Part, Template};
pub use fragment::{FragmentDir, Fragments, NoFragments};
pub use ir::{Action, Ir, Node, PartStyle};
pub use app::AppSpec;
pub use labels::Labels;
pub use policy::Policy;
pub use resolve::{MAX_DEPTH, MAX_WAIT_MS, resolve, resolve_with};
pub use spec::{Document, Format, NodeSpec, parse, parse_as};
pub use style::{Decl, StyleBlock, StyleValue};
pub use theme::Theme;
pub use value::Val;
