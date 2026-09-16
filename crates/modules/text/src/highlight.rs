use std::ops::Range;

use tree_sitter_highlight::{Highlight, HighlightEvent, Highlighter};

use crate::language::{Capture, Language};

/// One coloured run of the document, in bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub range: Range<usize>,
    pub capture: Capture,
}

/// The coloured runs of `text`, in order, none of them overlapping.
pub(crate) fn spans(text: &str, language: Language) -> Vec<Span> {
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
            Ok(HighlightEvent::HighlightStart(Highlight(index))) => {
                open.extend(Capture::at(index));
            }
            Ok(HighlightEvent::HighlightEnd) => {
                open.pop();
            }
            Ok(HighlightEvent::Source { start, end }) => {
                if let Some(capture) = open.last() {
                    spans.push(Span {
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
pub(crate) fn within(spans: &[Span], line: Range<usize>) -> Vec<Span> {
    let first = spans.partition_point(|span| span.range.end <= line.start);
    spans[first..]
        .iter()
        .take_while(|span| span.range.start < line.end)
        .map(|span| Span {
            range: span.range.start.max(line.start) - line.start
                ..span.range.end.min(line.end) - line.start,
            capture: span.capture,
        })
        .filter(|span| !span.range.is_empty())
        .collect()
}
