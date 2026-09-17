//! A file as a document: a rope of its text, the syntax spans over it, and search.
//! It holds no colours, only what a span means.

mod buffer;
mod highlight;
mod history;
mod language;
mod search;
mod tabs;

#[cfg(test)]
mod tests;

use std::borrow::Cow;
use std::ops::Range;

use groove_types::{Caret, Highlight, Indent};
use ropey::Rope;

pub use buffer::Buffer;
pub use language::Language;
pub use search::Found;
pub use tabs::{column_of, display_of, expand, spans_of};

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

    /// What one indent step writes here. A file Groove has no grammar for keeps the
    /// shape it already has: four spaces.
    pub fn indent(&self) -> Indent {
        match self.language {
            Some(language) => language.indent(),
            None => Indent::Spaces(4),
        }
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

    /// The characters before `caret`, clamped to a place the text has. A line past
    /// the last one is the end of the text, which is where a selection of all of it
    /// ends.
    pub fn char_of(&self, caret: Caret) -> usize {
        if caret.line >= self.lines() {
            return self.text.len_chars();
        }
        let start = self.text.line_to_char(caret.line);
        start + caret.column.min(self.line_chars(caret.line))
    }

    /// Where `at` characters in sits, as a reader counts it.
    pub fn caret_of(&self, at: usize) -> Caret {
        let at = at.min(self.text.len_chars());
        let line = self.text.char_to_line(at);
        Caret::new(line, at - self.text.line_to_char(line))
    }

    /// How long a line is, its break apart.
    pub fn line_chars(&self, line: usize) -> usize {
        match self.line(line) {
            Some(text) => text.chars().count(),
            None => 0,
        }
    }

    /// Puts `text` in at `at` characters, and moves the colours after it along.
    pub fn insert(&mut self, at: usize, text: &str) {
        let byte = self.text.char_to_byte(at);
        self.text.insert(at, text);
        highlight::moved(&mut self.spans, byte, 0, text.len());
    }

    /// Takes `range` out, and moves the colours after it back.
    pub fn remove(&mut self, range: Range<usize>) {
        let (start, end) = (
            self.text.char_to_byte(range.start),
            self.text.char_to_byte(range.end),
        );
        self.text.remove(range);
        highlight::moved(&mut self.spans, start, end - start, 0);
    }

    /// How many characters it holds.
    pub fn chars(&self) -> usize {
        self.text.len_chars()
    }

    /// The characters in `range`.
    pub fn slice(&self, range: Range<usize>) -> String {
        self.text.slice(range).to_string()
    }

    /// The whole text, for a write or a re-read of its colours.
    pub fn text(&self) -> String {
        self.text.to_string()
    }

    /// Takes colours read elsewhere, for the same text.
    pub fn set_spans(&mut self, spans: Vec<Highlight>) {
        self.spans = spans;
    }

    /// The colours of the whole text, read on this thread.
    pub fn colours(path: &str, text: &str) -> Vec<Highlight> {
        let language = Language::of(path);
        let long = text.len() > MAX_HIGHLIGHT_BYTES;
        match language {
            Some(language) if !long => highlight::spans(text, language),
            _ => Vec::new(),
        }
    }

    /// The colours read again, for a document that has been edited.
    pub fn recolour(&mut self) {
        let long = self.text.len_bytes() > MAX_HIGHLIGHT_BYTES;
        self.spans = match self.language {
            Some(language) if !long => highlight::spans(&self.text.to_string(), language),
            _ => Vec::new(),
        };
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
