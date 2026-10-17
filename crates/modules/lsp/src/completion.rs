//! A completion answer read as items, snippets typed as plain text.

use std::path::Path;

use groove_types::Completion;
use serde_json::Value;

use crate::documents::Documents;

/// The items of a list or a bare array, in the server's order.
pub(crate) fn items(answer: &Value, path: &Path, documents: &Documents) -> Vec<Completion> {
    let found = match answer {
        Value::Array(found) => found,
        Value::Object(list) => match list.get("items") {
            Some(Value::Array(found)) => found,
            _ => return Vec::new(),
        },
        _ => return Vec::new(),
    };
    found
        .iter()
        .filter_map(|one| item(one, path, documents))
        .collect()
}

fn item(found: &Value, path: &Path, documents: &Documents) -> Option<Completion> {
    let text = |key: &str| found.get(key).and_then(Value::as_str).map(String::from);
    let label = text("label")?;
    let edit = found.get("textEdit");
    let typed = edit
        .and_then(|edit| edit.get("newText"))
        .and_then(Value::as_str)
        .map(String::from)
        .or_else(|| text("insertText"))
        .unwrap_or_else(|| label.clone());
    let snippet = found.get("insertTextFormat").and_then(Value::as_u64) == Some(2);
    let range = edit.and_then(|edit| edit.get("range").or_else(|| edit.get("replace")));
    let start = range.and_then(|range| range.get("start"));
    Some(Completion {
        detail: text("detail"),
        insert: if snippet { plain(&typed) } else { typed },
        from: start.map(|start| documents.caret(path, start)),
        filter: text("filterText").unwrap_or_else(|| label.clone()),
        sort: text("sortText").unwrap_or_else(|| label.clone()),
        label,
    })
}

/// A snippet as the text it types: `${1:x}` is `x`, `${1|a,b|}` is `a`, a bare `$1` is nothing.
pub(crate) fn plain(snippet: &str) -> String {
    let mut out = String::new();
    let mut chars = snippet.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => out.extend(chars.next()),
            '$' if chars.peek() == Some(&'{') => {
                chars.next();
                out.push_str(&placeholder(&mut chars));
            }
            '$' if chars
                .peek()
                .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_') =>
            {
                while chars
                    .next_if(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .is_some()
                {}
            }
            other => out.push(other),
        }
    }
    out
}

/// The body of a `${…}` up to its closing brace, as the text it types.
fn placeholder(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut body = String::new();
    let mut depth = 0;
    for c in chars.by_ref() {
        match c {
            '{' => depth += 1,
            '}' if depth == 0 => break,
            '}' => depth -= 1,
            _ => {}
        }
        body.push(c);
    }
    let rest = body.trim_start_matches(|c: char| c.is_ascii_alphanumeric() || c == '_');
    if let Some(choices) = rest.strip_prefix('|') {
        return choices.split([',', '|']).next().unwrap_or_default().into();
    }
    rest.strip_prefix(':').map(plain).unwrap_or_default()
}
