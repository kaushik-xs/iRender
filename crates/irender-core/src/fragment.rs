//! Sources for `include` nodes. An include is replaced by its fragment when the
//! document is resolved, so the fragment is validated like the rest of the document
//! and sees the same scope (e.g. a list's `item`) as the place that includes it.

use crate::{Format, spec::NodeSpec};
use indexmap::IndexMap;
use std::{collections::HashMap, fs, path::PathBuf};

/// Supplies the fragments that documents include. Implement it to build fragments
/// from a database, an API response or anything else known at load time.
pub trait Fragments: Send + Sync {
    /// The fragment called `name`, or `Ok(None)` if there is none.
    fn fragment(&self, name: &str) -> Result<Option<NodeSpec>, String>;
}

/// No fragments besides the document's own.
pub struct NoFragments;

impl Fragments for NoFragments {
    fn fragment(&self, _: &str) -> Result<Option<NodeSpec>, String> {
        Ok(None)
    }
}

impl Fragments for IndexMap<String, NodeSpec> {
    fn fragment(&self, name: &str) -> Result<Option<NodeSpec>, String> {
        Ok(self.get(name).cloned())
    }
}

impl Fragments for HashMap<String, NodeSpec> {
    fn fragment(&self, name: &str) -> Result<Option<NodeSpec>, String> {
        Ok(self.get(name).cloned())
    }
}

impl<F> Fragments for F
where
    F: Fn(&str) -> Result<Option<NodeSpec>, String> + Send + Sync,
{
    fn fragment(&self, name: &str) -> Result<Option<NodeSpec>, String> {
        self(name)
    }
}

/// Fragments stored as files: `name` is read from `<dir>/<name>.json` (or `.yaml` /
/// `.yml`). Names may contain `/` for subdirectories but cannot leave `dir`.
pub struct FragmentDir(pub PathBuf);

impl Fragments for FragmentDir {
    fn fragment(&self, name: &str) -> Result<Option<NodeSpec>, String> {
        if !is_fragment_name(name) {
            return Err(format!(
                "invalid fragment name `{name}` (use letters, digits, `_`, `-` and `/` between segments)"
            ));
        }
        for ext in ["json", "yaml", "yml"] {
            let path = self.0.join(format!("{name}.{ext}"));
            let Some(format) = Format::from_path(&path) else { continue };
            if !path.is_file() {
                continue;
            }
            let src = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            return crate::parse_as(&src, format).map(Some).map_err(|d| format!("{}: {d}", path.display()));
        }
        Ok(None)
    }
}

fn is_fragment_name(name: &str) -> bool {
    name.split('/').all(|seg| {
        !seg.is_empty() && seg.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fragment_names_cannot_escape_the_directory() {
        for ok in ["header", "cards/product", "a-b_c1"] {
            assert!(is_fragment_name(ok), "{ok}");
        }
        for bad in ["", "../x", "a/../b", "/abs", "a//b", "a/", "x.json", "a b"] {
            assert!(!is_fragment_name(bad), "{bad}");
        }
    }
}
