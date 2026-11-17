use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// The document cannot be used.
    Error,
    /// Probably a mistake, but the document still works (e.g. a CSS property the
    /// validator doesn't recognise).
    Warning,
}

/// A single problem in a document, located by its path (`root.children[0].props.label`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub path: String,
    pub message: String,
    pub severity: Severity,
}

/// Every problem found in a document. Validation never stops at the first error.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Diagnostics(pub Vec<Diagnostic>);

impl Diagnostics {
    pub fn single(path: impl Into<String>, message: impl Into<String>) -> Self {
        let mut d = Self::default();
        d.push(path, message);
        d
    }

    /// Adds an error.
    pub fn push(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.0.push(Diagnostic { path: path.into(), message: message.into(), severity: Severity::Error });
    }

    pub fn warn(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.0.push(Diagnostic { path: path.into(), message: message.into(), severity: Severity::Warning });
    }

    pub fn has_errors(&self) -> bool {
        self.0.iter().any(|d| d.severity == Severity::Error)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.0.iter()
    }

    pub fn extend(&mut self, other: Diagnostics) {
        self.0.extend(other.0);
    }

    /// `Ok(value)` when there are no errors, otherwise `Err(self)`. Warnings are dropped;
    /// take them out first if you need them.
    pub fn into_result<T>(self, value: T) -> Result<T, Diagnostics> {
        if self.has_errors() { Err(self) } else { Ok(value) }
    }

    /// Turns every warning into an error.
    pub fn strict(mut self) -> Self {
        for d in &mut self.0 {
            d.severity = Severity::Error;
        }
        self
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.severity == Severity::Warning {
            f.write_str("warning: ")?;
        }
        if self.path.is_empty() {
            f.write_str(&self.message)
        } else {
            write!(f, "{}: {}", self.path, self.message)
        }
    }
}

impl fmt::Display for Diagnostics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, d) in self.0.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{d}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Diagnostics {}
