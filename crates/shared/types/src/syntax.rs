use std::ops::Range;

/// What a piece of code is, as a grammar reads it. Colour is the ui's to choose.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Capture {
    Keyword,
    Function,
    Type,
    String,
    Number,
    Comment,
    Constant,
    Attribute,
    Title,
    Literal,
    Link,
    /// A log line that names an error, and one that warns.
    Error,
    Warning,
}

/// One run of a line, in bytes from the line's start.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Highlight {
    pub range: Range<usize>,
    pub capture: Capture,
}

/// The spans inside `line`, in order, moved to offsets from the line's own start.
pub fn spans_within(spans: &[Highlight], line: Range<usize>) -> Vec<Highlight> {
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
