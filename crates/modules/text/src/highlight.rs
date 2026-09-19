//! Colouring text by asking the grammar's highlight query what each node means.

use std::ops::Range;

use groove_types::{Capture, Highlight};
use ropey::Rope;
use tree_sitter::{
    InputEdit, Node, Parser, Point, QueryCursor, StreamingIterator, TextProvider, Tree,
};

use crate::language::Language;

/// A parsed document: the grammar's own tree over the text.
#[derive(Clone)]
pub(crate) struct Syntax {
    language: Language,
    tree: Tree,
}

impl Syntax {
    /// Parses `text` in a job, or nothing when the language has no grammar here.
    pub(crate) fn new(text: &Rope, language: Language) -> Option<Self> {
        let mut parser = parser(language)?;
        let tree = parse(&mut parser, text, None)?;
        Some(Self { language, tree })
    }

    /// Moves the tree's nodes along an edit, without parsing again.
    pub(crate) fn shift(&mut self, edit: InputEdit) {
        self.tree.edit(&edit);
    }

    /// Parses `text` again, from the tree it already has.
    pub(crate) fn reparsed(&mut self, text: &Rope) {
        let Some(mut parser) = parser(self.language) else {
            return;
        };
        if let Some(tree) = parse(&mut parser, text, Some(&self.tree)) {
            self.tree = tree;
        }
    }

    /// What every node the query names means, over `range` of the text.
    pub(crate) fn spans(&self, text: &Rope, range: Range<usize>) -> Vec<Highlight> {
        let Some((_, query)) = self.language.syntax() else {
            return Vec::new();
        };
        let mut cursor = QueryCursor::new();
        cursor.set_byte_range(range);
        let mut found: Vec<(usize, usize, Capture)> = Vec::new();
        let mut matches = cursor.matches(query, self.tree.root_node(), Chunks(text));
        while let Some(one) = matches.next() {
            for capture in one.captures() {
                let name = &query.capture_names()[capture.index as usize];
                let Some(meaning) = crate::language::capture(name) else {
                    continue;
                };
                let node = capture.node.byte_range();
                found.push((node.start, node.end, meaning));
            }
        }
        flatten(found)
    }
}

/// The insert of `inserted` bytes at `at`, read from the text it made.
pub(crate) fn inserted(text: &Rope, at: usize, inserted: usize) -> InputEdit {
    let start = point(text, at);
    InputEdit {
        start_byte: at,
        old_end_byte: at,
        new_end_byte: at + inserted,
        start_position: start,
        old_end_position: start,
        new_end_position: point(text, at + inserted),
    }
}

/// The removal of `at..end`, read from the text that still holds it.
pub(crate) fn removed(text: &Rope, at: usize, end: usize) -> InputEdit {
    let start = point(text, at);
    InputEdit {
        start_byte: at,
        old_end_byte: end,
        new_end_byte: at,
        start_position: start,
        old_end_position: point(text, end),
        new_end_position: start,
    }
}

fn point(text: &Rope, byte: usize) -> Point {
    let line = text.byte_to_line(byte);
    Point::new(line, byte - text.line_to_byte(line))
}

fn parser(language: Language) -> Option<Parser> {
    let (grammar, _) = language.syntax()?;
    let mut parser = Parser::new();
    parser.set_language(grammar).ok()?;
    Some(parser)
}

/// Parses from the rope's chunks.
fn parse(parser: &mut Parser, text: &Rope, old: Option<&Tree>) -> Option<Tree> {
    parser.parse_with_options(
        &mut |byte, _| match byte < text.len_bytes() {
            true => {
                let (chunk, start, _, _) = text.chunk_at_byte(byte);
                &chunk.as_bytes()[byte - start..]
            }
            false => &[],
        },
        old,
        None,
    )
}

/// The rope as the query reads it: one node's bytes, chunk by chunk.
struct Chunks<'a>(&'a Rope);

impl<'a> TextProvider<&'a [u8]> for Chunks<'a> {
    type I = Box<dyn Iterator<Item = &'a [u8]> + 'a>;

    fn text(&mut self, node: Node<'_>) -> Self::I {
        let slice = self.0.byte_slice(node.byte_range());
        Box::new(slice.chunks().map(str::as_bytes))
    }
}

/// The captures as ordered spans that never overlap: an inner one cuts the outer.
fn flatten(mut found: Vec<(usize, usize, Capture)>) -> Vec<Highlight> {
    found.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
    found.dedup_by(|later, kept| later.0 == kept.0 && later.1 == kept.1);
    let mut out: Vec<Highlight> = Vec::with_capacity(found.len());
    let mut open: Vec<(usize, Capture)> = Vec::new();
    let mut at = 0;
    for (start, end, capture) in found {
        while let Some((over, held)) = open.last().copied() {
            if over > start {
                break;
            }
            open.pop();
            push(&mut out, &mut at, over, held);
        }
        if let Some((_, held)) = open.last().copied() {
            push(&mut out, &mut at, start, held);
        }
        at = at.max(start);
        open.push((end, capture));
    }
    while let Some((over, held)) = open.pop() {
        push(&mut out, &mut at, over, held);
    }
    out
}

/// One span from `at` to `over`, when there is anything between them.
fn push(out: &mut Vec<Highlight>, at: &mut usize, over: usize, capture: Capture) {
    if *at >= over {
        return;
    }
    out.push(Highlight {
        range: *at..over,
        capture,
    });
    *at = over;
}

/// The spans inside `line`, moved to offsets from the line's own start.
pub(crate) fn within(spans: &[Highlight], line: Range<usize>) -> Vec<Highlight> {
    let first = spans.partition_point(|span| span.range.end <= line.start);
    spans[first..]
        .iter()
        .take_while(|span| span.range.start < line.end)
        .map(|span| Highlight {
            range: span.range.start.max(line.start) - line.start
                ..span.range.end.min(line.end) - line.start,
            capture: span.capture,
        })
        .filter(|span| !span.range.is_empty())
        .collect()
}
