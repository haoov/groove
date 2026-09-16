use groove_types::{HookKind, SessionId, ToolCall};

const DETAIL_MAX: usize = 80;

/// The fields of a tool's input worth showing, in order.
const DETAILS: [&str; 4] = ["command", "file_path", "path", "pattern"];

/// One hook the agent posted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Post {
    pub session: SessionId,
    pub kind: HookKind,
    pub tool: Option<ToolCall>,
}

/// Claude's payload: the event's name, and the tool when the event has one.
pub(crate) fn post(session: &str, body: &[u8]) -> Option<Post> {
    let value: serde_json::Value = serde_json::from_slice(body).ok()?;
    let name = value.get("hook_event_name")?.as_str()?;
    Some(Post {
        session: SessionId::new(session),
        kind: HookKind::parse(name)?,
        tool: tool(&value),
    })
}

/// The tool's name, and the one field of its input worth a line.
fn tool(value: &serde_json::Value) -> Option<ToolCall> {
    let name = value.get("tool_name")?.as_str()?.to_string();
    let input = value.get("tool_input");
    let detail = DETAILS
        .iter()
        .find_map(|key| input?.get(key)?.as_str())
        .map(one_line);
    Some(ToolCall { name, detail })
}

/// The first line, capped.
fn one_line(text: &str) -> String {
    text.lines()
        .next()
        .unwrap_or_default()
        .chars()
        .take(DETAIL_MAX)
        .collect()
}
