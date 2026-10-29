//! One transcript line read into what the conversation hears; a record kind Groove does not know says nothing.

use groove_types::Timestamp;
use groove_types::chat::Heard;
use serde_json::Value;

const DETAIL_MAX: usize = 120;

/// The fields of a tool's input worth a line, in order.
const DETAILS: [&str; 9] = [
    "command",
    "file_path",
    "path",
    "pattern",
    "message",
    "title",
    "query",
    "url",
    "description",
];

const PASTED: &str = "<pasted_content";
const PASTED_END: &str = "</pasted_content";

/// The tools that only look.
const LOOKS: [&str; 4] = ["Read", "Grep", "Glob", "LS"];

/// The tools that write a file.
const EDITS: [&str; 3] = ["Edit", "MultiEdit", "NotebookEdit"];

/// The tools that say nothing worth a row.
const SILENT: [&str; 1] = ["ToolSearch"];

/// What one line says; nothing for a kind not known, a side chain or Claude's own meta prompt.
pub fn line(text: &str) -> Vec<Heard> {
    let Ok(record) = serde_json::from_str::<Value>(text) else {
        return Vec::new();
    };
    let flag = |key: &str| record.get(key).and_then(Value::as_bool) == Some(true);
    if flag("isSidechain") || flag("isMeta") {
        return Vec::new();
    }
    let heard = heard(&record);
    let at = record.get("timestamp").and_then(Value::as_str);
    match at.and_then(|one| Timestamp::parse(one).ok()) {
        Some(at) if !heard.is_empty() => std::iter::once(Heard::Clock(at)).chain(heard).collect(),
        _ => heard,
    }
}

/// What a record says, by its kind.
fn heard(record: &Value) -> Vec<Heard> {
    let content = record.pointer("/message/content");
    match (record.get("type").and_then(Value::as_str), content) {
        (Some("user"), Some(Value::String(text))) => prompt(text).into_iter().collect(),
        (Some("user"), Some(Value::Array(blocks))) => user(blocks, record),
        (Some("assistant"), Some(Value::Array(blocks))) => blocks.iter().filter_map(said).collect(),
        (Some("attachment"), _) => queued(record).into_iter().collect(),
        (Some("queue-operation"), _) => waiting(record).into_iter().collect(),
        _ => Vec::new(),
    }
}

/// The user's words; a slash command by its name; Claude's own tags say nothing.
fn prompt(text: &str) -> Option<Heard> {
    if let Some(name) = between(text, "<command-name>", "</command-name>") {
        let args = between(text, "<command-args>", "</command-args>").unwrap_or_default();
        return Some(Heard::You(format!("{name} {args}").trim().to_string()));
    }
    let text = unpasted(text);
    let text = text.trim();
    (!text.is_empty() && !text.starts_with('<')).then(|| Heard::You(text.to_string()))
}

/// A message typed while the agent works, which Claude keeps as an attachment.
fn queued(record: &Value) -> Option<Heard> {
    let attachment = record.get("attachment")?;
    if attachment.get("type")?.as_str()? != "queued_command" {
        return None;
    }
    prompt(attachment.get("prompt")?.as_str()?)
}

/// A message waiting for the agent's turn, or taken off the wait.
fn waiting(record: &Value) -> Option<Heard> {
    let content = record.get("content").and_then(Value::as_str);
    let said = || match prompt(content?)? {
        Heard::You(text) => Some(text),
        _ => None,
    };
    match record.get("operation")?.as_str()? {
        "enqueue" => said().map(Heard::Queued),
        "remove" | "dequeue" => Some(Heard::Dequeued(said())),
        "popAll" => Some(Heard::Recalled),
        _ => None,
    }
}

/// The text with each pasted block in place of its tags.
fn unpasted(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some((before, after)) = rest.split_once(PASTED) {
        let Some((_, inside)) = after.split_once('>') else {
            break;
        };
        let Some((pasted, past)) = inside.split_once(PASTED_END) else {
            break;
        };
        out.push_str(before);
        out.push_str(pasted.trim());
        rest = past.split_once('>').map_or("", |(_, tail)| tail);
    }
    out.push_str(rest);
    out
}

fn between<'a>(text: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let (_, rest) = text.split_once(open)?;
    Some(rest.split_once(close)?.0.trim())
}

/// A user record: typed words, or what tools gave back.
fn user(blocks: &[Value], record: &Value) -> Vec<Heard> {
    let results = blocks
        .iter()
        .filter(|one| kind(one) == "tool_result")
        .count();
    let whole = record.get("toolUseResult").filter(|_| results == 1);
    blocks
        .iter()
        .filter_map(|block| match kind(block) {
            "text" => prompt(block.get("text")?.as_str()?),
            "tool_result" => Some(Heard::Result {
                id: field(block, "tool_use_id")?,
                outcome: super::outcome::outcome(block, whole),
            }),
            _ => None,
        })
        .collect()
}

/// One block of the agent's message.
fn said(block: &Value) -> Option<Heard> {
    match kind(block) {
        "text" => {
            let text = block.get("text")?.as_str()?.trim();
            (!text.is_empty()).then(|| Heard::Said(text.to_string()))
        }
        "thinking" | "redacted_thinking" => Some(Heard::Thought),
        "tool_use" => used(block),
        _ => None,
    }
}

/// A tool call, by what it does.
fn used(block: &Value) -> Option<Heard> {
    let (id, name) = (field(block, "id")?, field(block, "name")?);
    let input = block.get("input");
    let text = |key: &str| Some(input?.get(key)?.as_str()?.to_string());
    let name = name.as_str();
    Some(match name {
        "Write" => Heard::Edit {
            id,
            path: text("file_path")?,
            created: true,
        },
        _ if EDITS.contains(&name) => Heard::Edit {
            id,
            path: text("file_path").or_else(|| text("notebook_path"))?,
            created: false,
        },
        "Bash" => Heard::Ran {
            id,
            command: text("command")?,
            about: text("description"),
        },
        _ if LOOKS.contains(&name) => Heard::Look {
            id,
            at: looked_at(input).unwrap_or_else(|| name.to_string()),
        },
        _ if SILENT.contains(&name) => return None,
        _ => Heard::Tool {
            id,
            name: name.to_string(),
            detail: detail(input),
        },
    })
}

/// A look's subject: a file by its name, else its pattern.
fn looked_at(input: Option<&Value>) -> Option<String> {
    let input = input?;
    if let Some(path) = input.get("file_path").and_then(Value::as_str) {
        return Some(path.rsplit('/').next().unwrap_or(path).to_string());
    }
    ["pattern", "path"]
        .iter()
        .find_map(|key| input.get(key)?.as_str())
        .map(str::to_string)
}

fn detail(input: Option<&Value>) -> Option<String> {
    let found = DETAILS.iter().find_map(|key| input?.get(key)?.as_str())?;
    let first = found.lines().next().unwrap_or_default();
    Some(first.chars().take(DETAIL_MAX).collect())
}

fn kind(block: &Value) -> &str {
    block
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default()
}

fn field(block: &Value, key: &str) -> Option<String> {
    Some(block.get(key)?.as_str()?.to_string())
}
