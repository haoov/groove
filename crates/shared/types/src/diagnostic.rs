//! What a language server says about a file, and where it points.

use std::path::PathBuf;

use crate::Caret;

/// How bad a diagnostic is, worst first.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Severity {
    Error,
    Warning,
    Information,
    Hint,
}

/// One finding on a range of a file, in the server's words.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Diagnostic {
    pub start: Caret,
    pub end: Caret,
    pub severity: Severity,
    pub message: String,
    /// The tool that found it, as `ty` or `yaml-schema: kubernetes`.
    pub source: Option<String>,
    /// The rule it broke, as `invalid-return-type`.
    pub code: Option<String>,
}

/// A range in a file, as a definition answers it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Location {
    pub path: PathBuf,
    pub start: Caret,
    pub end: Caret,
}

/// One thing a server offers to type at the caret.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Completion {
    pub label: String,
    pub detail: Option<String>,
    /// What accepting it types, snippet marks taken out.
    pub insert: String,
    /// Where the text it replaces starts; the word before the caret when the server names none.
    pub from: Option<Caret>,
    /// What the typed word is matched against.
    pub filter: String,
    pub sort: String,
}
