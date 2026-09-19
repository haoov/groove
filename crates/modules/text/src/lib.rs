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
    /// The grammar's own tree. An edit moves its nodes; a parse makes it right.
    syntax: Option<highlight::Syntax>,
}

/// A tree parsed again for the text it belongs to.
pub struct Settled {
    syntax: Option<highlight::Syntax>,
}

/// The colours over a range of lines, each at offsets from its own line's start.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Colours {
    first: usize,
    lines: Vec<Vec<Highlight>>,
}

impl Colours {
    pub fn of(&self, line: usize) -> &[Highlight] {
        let at = line.checked_sub(self.first);
        at.and_then(|at| self.lines.get(at))
            .map_or(&[], Vec::as_slice)
    }
}

impl Document {
    pub fn new(path: &str, text: &str) -> Self {
        let language = Language::of(path);
        let long = text.len() > MAX_HIGHLIGHT_BYTES;
        let rope = Rope::from_str(text);
        let syntax = match language {
            Some(language) if !long => highlight::Syntax::new(&rope, language),
            _ => None,
        };
        Self {
            text: rope,
            language,
            syntax,
        }
    }

    /// The same document with no tree, for text a reader only counts lines of.
    pub fn plain(path: &str, text: &str) -> Self {
        Self {
            text: Rope::from_str(text),
            language: Language::of(path),
            syntax: None,
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
        self.syntax.is_some()
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
        self.colours(at..at + 1).of(at).to_vec()
    }

    /// The lines the scopes around `line` begin on, outermost first. A scope that
    /// begins on `line` itself is left out.
    pub fn scopes(&self, line: usize) -> Vec<usize> {
        let (Some(syntax), Some(range)) = (&self.syntax, self.bytes_of(line)) else {
            return Vec::new();
        };
        syntax
            .scopes(range.start)
            .into_iter()
            .filter(|at| *at < line)
            .collect()
    }

    /// What the grammar says about `lines`, asked of the tree once.
    pub fn colours(&self, lines: Range<usize>) -> Colours {
        let first = lines.start;
        let last = lines.end.min(self.lines());
        let Some(syntax) = &self.syntax else {
            return Colours::default();
        };
        let (Some(from), Some(to)) = (self.bytes_of(first), self.bytes_of(last.wrapping_sub(1)))
        else {
            return Colours::default();
        };
        let found = syntax.spans(&self.text, from.start..to.end);
        Colours {
            first,
            lines: (first..last)
                .map(|at| match self.bytes_of(at) {
                    Some(line) => highlight::within(&found, line),
                    None => Vec::new(),
                })
                .collect(),
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

    /// Puts `text` in at `at` characters, and moves the tree along it.
    pub fn insert(&mut self, at: usize, text: &str) {
        let byte = self.text.char_to_byte(at);
        self.text.insert(at, text);
        let edit = highlight::inserted(&self.text, byte, text.len());
        self.shift(edit);
    }

    /// Takes `range` out, and moves the tree back over it.
    pub fn remove(&mut self, range: Range<usize>) {
        let (start, end) = (
            self.text.char_to_byte(range.start),
            self.text.char_to_byte(range.end),
        );
        let edit = highlight::removed(&self.text, start, end);
        self.text.remove(range);
        self.shift(edit);
    }

    fn shift(&mut self, edit: tree_sitter::InputEdit) {
        if let Some(syntax) = self.syntax.as_mut() {
            syntax.shift(edit);
        }
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

    pub fn install(&mut self, settled: Settled) {
        self.syntax = settled.syntax;
    }

    /// The tree parsed again for the text it now holds, for a job to hand back.
    pub fn settled(mut self) -> Settled {
        self.reparse();
        Settled {
            syntax: self.syntax,
        }
    }

    /// The same, on this thread.
    pub fn reparse(&mut self) {
        if let Some(syntax) = self.syntax.as_mut() {
            syntax.reparsed(&self.text);
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
