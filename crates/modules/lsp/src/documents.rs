//! The documents open on a server: their text, to count columns, and their version.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use groove_types::Caret;
use serde_json::{Value, json};

use crate::position::Encoding;

pub(crate) struct Documents {
    pub(crate) encoding: Encoding,
    /// The server takes a change as the range it replaced, not the whole text.
    pub(crate) incremental: bool,
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
            incremental: false,
            open: HashMap::new(),
        }
    }
}

impl Documents {
    pub(crate) fn open(&mut self, path: &Path, text: String) {
        self.open.insert(path.to_path_buf(), Open::new(1, text));
    }

    /// The document's new version and its `contentChanges`, or `None` when it is not open.
    pub(crate) fn change(&mut self, path: &Path, text: String) -> Option<(i64, Value)> {
        let open = self.open.get_mut(path)?;
        let changes = match self.incremental {
            true => json!([open.edit(&text, self.encoding)]),
            false => json!([{ "text": text }]),
        };
        *open = Open::new(open.version + 1, text);
        Some((open.version, changes))
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
    /// The one range of this text `text` differs in, and what replaces it.
    fn edit(&self, text: &str, encoding: Encoding) -> Value {
        let (old, new) = (self.text.as_bytes(), text.as_bytes());
        let mut head = old.iter().zip(new).take_while(|(a, b)| a == b).count();
        while !self.text.is_char_boundary(head) || !text.is_char_boundary(head) {
            head -= 1;
        }
        let most = old.len().min(new.len()) - head;
        let same = old.iter().rev().zip(new.iter().rev()).take(most);
        let mut tail = same.take_while(|(a, b)| a == b).count();
        while !self.text.is_char_boundary(old.len() - tail)
            || !text.is_char_boundary(new.len() - tail)
        {
            tail -= 1;
        }
        let range = json!({
            "start": self.position(head, encoding),
            "end": self.position(old.len() - tail, encoding),
        });
        json!({ "range": range, "text": &text[head..new.len() - tail] })
    }

    /// Byte `at` of the text as the server counts it.
    fn position(&self, at: usize, encoding: Encoding) -> Value {
        let line = self.starts.partition_point(|start| *start <= at) - 1;
        let before = &self.text[self.starts[line]..at];
        json!({ "line": line, "character": encoding.units(before, usize::MAX) })
    }

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
