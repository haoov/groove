//! The documents open on a server: their text, to count columns, and their version.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use groove_types::Caret;
use serde_json::{Value, json};

use crate::position::Encoding;

pub(crate) struct Documents {
    pub(crate) encoding: Encoding,
    open: HashMap<PathBuf, Open>,
}

struct Open {
    version: i64,
    text: String,
    starts: Vec<usize>,
}

impl Default for Documents {
    fn default() -> Self {
        Self {
            encoding: Encoding::Utf16,
            open: HashMap::new(),
        }
    }
}

impl Documents {
    pub(crate) fn open(&mut self, path: &Path, text: String) {
        self.open.insert(path.to_path_buf(), Open::new(1, text));
    }

    /// The document's new version, or `None` when it is not open.
    pub(crate) fn change(&mut self, path: &Path, text: String) -> Option<i64> {
        let open = self.open.get_mut(path)?;
        *open = Open::new(open.version + 1, text);
        Some(open.version)
    }

    pub(crate) fn close(&mut self, path: &Path) -> bool {
        self.open.remove(path).is_some()
    }

    pub(crate) fn is_open(&self, path: &Path) -> bool {
        self.open.contains_key(path)
    }

    /// `at` as the server counts it.
    pub(crate) fn position(&self, path: &Path, at: Caret) -> Value {
        let line = self.line(path, at.line).unwrap_or_default();
        json!({ "line": at.line, "character": self.encoding.units(line, at.column) })
    }

    /// A server's position as a caret; a file not open counts its units as characters.
    pub(crate) fn caret(&self, path: &Path, position: &Value) -> Caret {
        let number = |key: &str| position.get(key).and_then(Value::as_u64).unwrap_or(0) as usize;
        let (line, units) = (number("line"), number("character"));
        let column = match self.line(path, line) {
            Some(text) => self.encoding.column(text, units),
            None => units,
        };
        Caret::new(line, column)
    }

    fn line(&self, path: &Path, line: usize) -> Option<&str> {
        let open = self.open.get(path)?;
        let start = *open.starts.get(line)?;
        let end = open
            .starts
            .get(line + 1)
            .copied()
            .unwrap_or(open.text.len());
        Some(open.text[start..end].trim_end_matches(['\n', '\r']))
    }
}

impl Open {
    fn new(version: i64, text: String) -> Self {
        let starts = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(at, _)| at + 1))
            .collect();
        Self {
            version,
            text,
            starts,
        }
    }
}
