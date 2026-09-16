//! A file as a document: a rope of its text, the syntax spans over it, and search.
//! It holds no colours, only what a span means.

mod highlight;
mod language;
mod search;

#[cfg(test)]
mod tests;

use std::borrow::Cow;
use std::ops::Range;

use groove_types::Highlight;
use ropey::Rope;

pub use language::Language;
pub use search::Found;

/// Above this a document keeps its text and gives up its colour.
pub const MAX_HIGHLIGHT_BYTES: usize = 1 << 20;

impl std::fmt::Debug for Document {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Document")
            .field("lines", &self.lines())
            .field("language", &self.language)
            .finish()
    }
}

#[derive(Clone)]
pub struct Document {
    text: Rope,
    language: Option<Language>,
    spans: Vec<Highlight>,
}

impl Document {
    pub fn new(path: &str, text: &str) -> Self {
        let language = Language::of(path);
        let long = text.len() > MAX_HIGHLIGHT_BYTES;
        let spans = match language {
            Some(language) if !long => highlight::spans(text, language),
            _ => Vec::new(),
        };
        Self {
            text: Rope::from_str(text),
            language,
            spans,
        }
    }

    pub fn language(&self) -> Option<Language> {
        self.language
    }

    pub fn is_highlighted(&self) -> bool {
        !self.spans.is_empty()
    }

    /// Lines as a reader counts them: a trailing newline ends the last one, and an
    /// empty file has none.
    pub fn lines(&self) -> usize {
        if self.text.len_bytes() == 0 {
            return 0;
        }
        let lines = self.text.len_lines();
        match self.ends_open() {
            true => lines,
            false => lines.saturating_sub(1),
        }
    }

    pub fn bytes(&self) -> usize {
        self.text.len_bytes()
    }

    pub fn line(&self, at: usize) -> Option<Cow<'_, str>> {
        (at < self.lines()).then(|| {
            let line = self.text.line(at);
            match line.as_str() {
                Some(text) => Cow::Borrowed(ended(text)),
                None => Cow::Owned(ended(&line.to_string()).to_string()),
            }
        })
    }

    /// The line's coloured runs, at offsets from its own start.
    pub fn spans(&self, at: usize) -> Vec<Highlight> {
        match self.bytes_of(at) {
            Some(range) => highlight::within(&self.spans, range),
            None => Vec::new(),
        }
    }

    /// Every line holding `query`, with where in the line it was found.
    pub fn search(&self, query: &str) -> Vec<Found> {
        (0..self.lines())
            .filter_map(|at| self.line(at).map(|text| (at, text)))
            .flat_map(|(at, text)| search::found(at, &text, query))
            .collect()
    }

    fn bytes_of(&self, at: usize) -> Option<Range<usize>> {
        let text = self.line(at)?;
        let start = self.text.line_to_byte(at);
        Some(start..start + text.len())
    }

    fn ends_open(&self) -> bool {
        let last = self.text.len_chars();
        last == 0 || self.text.char(last.saturating_sub(1)) != '\n'
    }
}

/// The line without the break that ends it.
fn ended(line: &str) -> &str {
    let line = line.strip_suffix('\n').unwrap_or(line);
    line.strip_suffix('\r').unwrap_or(line)
}
