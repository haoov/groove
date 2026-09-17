use std::ops::Range;

use groove_types::{Capture, Highlight};
use tree_sitter_highlight::{Highlight as Index, HighlightEvent, Highlighter};

use crate::language::Language;

/// The coloured runs of `text`, in order, none of them overlapping.
pub(crate) fn spans(text: &str, language: Language) -> Vec<Highlight> {
    let Some(config) = language.config() else {
        return Vec::new();
    };
    let mut highlighter = Highlighter::new();
    let Ok(events) = highlighter.highlight(config, text.as_bytes(), None, None, |_| None) else {
        return Vec::new();
    };
    let mut spans = Vec::new();
    let mut open: Vec<Capture> = Vec::new();
    for event in events {
        match event {
            Ok(HighlightEvent::HighlightStart(Index(index))) => {
                open.extend(crate::language::capture(index));
            }
            Ok(HighlightEvent::HighlightEnd) => {
                open.pop();
            }
            Ok(HighlightEvent::Source { start, end }) => {
                if let Some(capture) = open.last() {
                    spans.push(Highlight {
                        range: start..end,
                        capture: *capture,
                    });
                }
            }
            Err(_) => return spans,
        }
    }
    spans
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

/// Moves spans along an edit at `at` that took `removed` bytes out and put
/// `inserted` in. A span the edit lands inside grows or shrinks with it.
pub(crate) fn moved(spans: &mut Vec<Highlight>, at: usize, removed: usize, inserted: usize) {
    let end = at + removed;
    for span in spans.iter_mut() {
        span.range.start = start_of(span.range.start, at, end, inserted);
        span.range.end = end_of(span.range.end, at, end, inserted);
    }
    spans.retain(|span| !span.range.is_empty());
}

/// Text put in where a span starts belongs before it, so the span moves along.
fn start_of(byte: usize, at: usize, end: usize, inserted: usize) -> usize {
    match byte {
        _ if byte < at => byte,
        _ if byte >= end => byte - (end - at) + inserted,
        _ => at,
    }
}

/// Text put in where a span ends belongs after it, so the span keeps its end.
fn end_of(byte: usize, at: usize, end: usize, inserted: usize) -> usize {
    match byte {
        _ if byte <= at => byte,
        _ if byte >= end => byte - (end - at) + inserted,
        _ => at,
    }
}
