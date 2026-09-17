//! Tabs as the surface shows them. A buffer keeps the tab a file has; a row draws it
//! run out to the next stop, so what is drawn, what a click lands on and where the
//! caret sits all count the same columns.

use groove_types::Highlight;

/// The line with every tab run out to the next stop.
pub fn expand(line: &str, width: usize) -> String {
    if !line.contains('\t') {
        return line.to_string();
    }
    let mut out = String::with_capacity(line.len() + width);
    for ch in line.chars() {
        match ch {
            '\t' => out.push_str(&" ".repeat(stop(out.chars().count(), width))),
            _ => out.push(ch),
        }
    }
    out
}

/// The column a character of the line is drawn at.
pub fn display_of(line: &str, column: usize, width: usize) -> usize {
    if !line.contains('\t') {
        return column;
    }
    let mut at = 0;
    for ch in line.chars().take(column) {
        at += match ch {
            '\t' => stop(at, width),
            _ => 1,
        };
    }
    at
}

/// The character a drawn column belongs to: the tab it lands inside counts as one.
pub fn column_of(line: &str, display: usize, width: usize) -> usize {
    if !line.contains('\t') {
        return display;
    }
    let mut at = 0;
    for (column, ch) in line.chars().enumerate() {
        if at >= display {
            return column;
        }
        at += match ch {
            '\t' => stop(at, width),
            _ => 1,
        };
    }
    line.chars().count()
}

/// The spans over the expanded line, for colouring what is drawn.
pub fn spans_of(spans: &[Highlight], line: &str, width: usize) -> Vec<Highlight> {
    if !line.contains('\t') {
        return spans.to_vec();
    }
    spans
        .iter()
        .map(|span| Highlight {
            range: byte_of(line, span.range.start, width)..byte_of(line, span.range.end, width),
            capture: span.capture,
        })
        .collect()
}

/// Where a byte of the line sits once the tabs before it are run out.
fn byte_of(line: &str, byte: usize, width: usize) -> usize {
    let mut at = 0;
    let mut column = 0;
    for (offset, ch) in line.char_indices() {
        if offset >= byte {
            return at;
        }
        let step = match ch {
            '\t' => stop(column, width),
            _ => ch.len_utf8(),
        };
        at += step;
        column += match ch {
            '\t' => stop(column, width),
            _ => 1,
        };
    }
    at + byte.saturating_sub(line.len())
}

/// How many columns a tab at `column` runs out to.
fn stop(column: usize, width: usize) -> usize {
    let width = width.max(1);
    width - column % width
}
