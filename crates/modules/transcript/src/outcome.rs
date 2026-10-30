//! What a tool call gave back: whether it failed, its output's tail, and the patch of a write.

use groove_types::chat::{Outcome, Patch};
use serde_json::Value;

/// The last lines of a command's output kept.
const OUTPUT_LINES: usize = 40;

/// What a call gave back: whether it failed, its output's tail, and the patch of a write.
pub(super) fn outcome(block: &Value, whole: Option<&Value>) -> Outcome {
    let failed = block.get("is_error").and_then(Value::as_bool) == Some(true)
        || whole.and_then(|one| one.get("interrupted")?.as_bool()) == Some(true);
    let output = whole
        .and_then(ran)
        .or_else(|| returned(block.get("content")?))
        .map(|text| tail(&text));
    Outcome {
        failed,
        output,
        patch: whole.and_then(patch),
    }
}

/// A command's stdout, then its stderr.
fn ran(whole: &Value) -> Option<String> {
    let out = whole.get("stdout")?.as_str()?;
    let err = whole
        .get("stderr")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let both: Vec<&str> = [out, err]
        .into_iter()
        .filter(|one| !one.is_empty())
        .collect();
    Some(both.join("\n"))
}

/// The text a tool result carries, a string or text blocks.
fn returned(content: &Value) -> Option<String> {
    match content {
        Value::String(text) => Some(text.clone()),
        Value::Array(blocks) => {
            let texts: Vec<&str> = blocks
                .iter()
                .filter_map(|one| one.get("text")?.as_str())
                .collect();
            (!texts.is_empty()).then(|| texts.join("\n"))
        }
        _ => None,
    }
}

fn tail(text: &str) -> String {
    let lines: Vec<&str> = text.trim_end().lines().collect();
    lines[lines.len().saturating_sub(OUTPUT_LINES)..].join("\n")
}

/// A write's lines gained and lost; a new file gains every line it holds.
fn patch(whole: &Value) -> Option<Patch> {
    let hunks = whole.get("structuredPatch")?.as_array()?;
    if whole.get("type").and_then(Value::as_str) == Some("create") {
        let content = whole.get("content")?.as_str()?;
        let added = content.lines().count() as u32;
        return Some(Patch {
            added,
            removed: 0,
            line: 1,
        });
    }
    let lines = hunks
        .iter()
        .filter_map(|one| one.get("lines")?.as_array())
        .flatten()
        .filter_map(Value::as_str);
    let (mut added, mut removed) = (0, 0);
    for one in lines {
        match one.chars().next() {
            Some('+') => added += 1,
            Some('-') => removed += 1,
            _ => {}
        }
    }
    let line = hunks.first()?.get("newStart")?.as_u64()? as u32;
    Some(Patch {
        added,
        removed,
        line,
    })
}
