//! An object as YAML in block style, the way `kubectl get -o yaml` writes it.

use serde_json::Value;

pub fn yaml(value: &Value) -> String {
    let mut out = String::new();
    match value {
        Value::Object(map) if !map.is_empty() => mapping(&mut out, map, 0),
        Value::Array(items) if !items.is_empty() => sequence(&mut out, items, 0),
        other => {
            out.push_str(&scalar(other, 0));
            out.push('\n');
        }
    }
    out
}

fn mapping(out: &mut String, map: &serde_json::Map<String, Value>, indent: usize) {
    for (at, (key, value)) in map.iter().enumerate() {
        if at > 0 || !out.ends_with("- ") {
            pad(out, indent);
        }
        out.push_str(&text(key, indent));
        out.push(':');
        nested(out, value, indent);
    }
}

fn sequence(out: &mut String, items: &[Value], indent: usize) {
    for item in items {
        pad(out, indent);
        out.push_str("- ");
        match item {
            Value::Object(map) if !map.is_empty() => mapping(out, map, indent + 2),
            Value::Array(inner) if !inner.is_empty() => {
                out.push('\n');
                sequence(out, inner, indent + 2);
            }
            other => {
                out.push_str(&scalar(other, indent + 2));
                out.push('\n');
            }
        }
    }
}

/// What follows `key:`: a scalar on the line, or the block under it.
fn nested(out: &mut String, value: &Value, indent: usize) {
    match value {
        Value::Object(map) if !map.is_empty() => {
            out.push('\n');
            mapping(out, map, indent + 2);
        }
        Value::Array(items) if !items.is_empty() => {
            out.push('\n');
            sequence(out, items, indent);
        }
        other => {
            out.push(' ');
            out.push_str(&scalar(other, indent + 2));
            out.push('\n');
        }
    }
}

fn scalar(value: &Value, indent: usize) -> String {
    match value {
        Value::Null => "null".into(),
        Value::Bool(one) => one.to_string(),
        Value::Number(one) => one.to_string(),
        Value::String(one) => text(one, indent),
        Value::Object(_) => "{}".into(),
        Value::Array(_) => "[]".into(),
    }
}

/// A string as YAML reads it back: bare, quoted, or a block when it spans lines.
fn text(one: &str, indent: usize) -> String {
    if one.contains('\n') && !one.contains('\r') && !one.trim_matches('\n').is_empty() {
        return block(one, indent);
    }
    match bare(one) {
        true => one.to_string(),
        false => serde_json::to_string(one).unwrap_or_default(),
    }
}

/// A `|` block that keeps its trailing line breaks, with an indent of 2 when it starts with a space.
fn block(one: &str, indent: usize) -> String {
    let body = one.strip_suffix('\n').unwrap_or(one);
    let first = body.split('\n').find(|line| !line.is_empty());
    let mut out = String::from("|");
    if first.is_some_and(|line| line.starts_with(' ')) {
        out.push('2');
    }
    out.push_str(match one.len() - one.trim_end_matches('\n').len() {
        0 => "-",
        1 => "",
        _ => "+",
    });
    for line in body.split('\n') {
        out.push('\n');
        if !line.is_empty() {
            out.push_str(&" ".repeat(indent));
            out.push_str(line);
        }
    }
    out
}

/// Whether a string reads back as itself without quotes.
fn bare(one: &str) -> bool {
    const WORDS: [&str; 12] = [
        "true", "false", "null", "~", "yes", "no", "on", "off", "y", "n", ".inf", ".nan",
    ];
    let starts = |c: char| ",[]{}#&*!|>'\"%@` ".contains(c);
    let lone = ["-", "?", ":"]
        .iter()
        .any(|mark| one == *mark || one.starts_with(&format!("{mark} ")));
    !one.is_empty()
        && !lone
        && !one.starts_with(starts)
        && !one.ends_with([' ', ':'])
        && !one.contains(": ")
        && !one.contains(" #")
        && !one.contains(['\t', '\r', '\n'])
        && !WORDS.contains(&one.to_lowercase().as_str())
        && one.parse::<f64>().is_err()
}

fn pad(out: &mut String, indent: usize) {
    out.push_str(&" ".repeat(indent));
}
