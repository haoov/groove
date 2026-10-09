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

/// A string as YAML reads it back: bare, quoted, or a `|-` block when it spans lines.
fn text(one: &str, indent: usize) -> String {
    if one.contains('\n') && !one.contains('\r') {
        let mut block = String::from("|-");
        for line in one.trim_end_matches('\n').split('\n') {
            block.push('\n');
            if !line.is_empty() {
                block.push_str(&" ".repeat(indent));
                block.push_str(line);
            }
        }
        return block;
    }
    match bare(one) {
        true => one.to_string(),
        false => serde_json::to_string(one).unwrap_or_default(),
    }
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
        && !one.contains(['\t', '\r'])
        && !WORDS.contains(&one.to_lowercase().as_str())
        && one.parse::<f64>().is_err()
}

fn pad(out: &mut String, indent: usize) {
    out.push_str(&" ".repeat(indent));
}
