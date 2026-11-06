//! What the transcript says of the agents sent to the background: their launch, their report, their end.

use groove_types::chat::{Heard, Sent};
use serde_json::Value;

use crate::parse::{between, field};

/// The status a background agent's call returns with at once; its end comes as a notification.
pub(crate) const LAUNCHED: &str = "async_launched";

/// A background agent's call: its own id, which its end names.
pub(crate) fn launched(blocks: &[Value], whole: Option<&Value>) -> Option<Heard> {
    let id = blocks.iter().find_map(|one| field(one, "tool_use_id"))?;
    let agent = whole?.get("agentId")?.as_str()?.to_string();
    Some(Heard::Background { id, agent })
}

/// A background agent's end, by its call or its own id.
pub(crate) fn notified(text: &str) -> Option<Heard> {
    let notice = between(text, "<task-notification>", "</task-notification>")?;
    let tag = |name: &str| between(notice, &format!("<{name}>"), &format!("</{name}>"));
    let state = match tag("status").unwrap_or_default() {
        "completed" => Sent::Done,
        "failed" => Sent::Failed,
        _ => Sent::Stopped,
    };
    Some(Heard::Ended {
        call: tag("tool-use-id").map(str::to_string),
        agent: tag("task-id")?.to_string(),
        state,
        summary: tag("result").or(tag("summary")).map(str::to_string),
    })
}

/// A subagent's report, handed back as a message from it.
pub(crate) fn reported(text: &str) -> Option<Heard> {
    let (_, rest) = text.split_once("<agent-message from=\"")?;
    let (agent, rest) = rest.split_once('"')?;
    let body = rest.split_once(REPORT).map_or(rest, |(_, body)| body);
    let body = body.split("</agent-message>").next().unwrap_or(body);
    let lines: Vec<&str> = body
        .lines()
        .map(|one| one.strip_prefix("  ").unwrap_or(one))
        .collect();
    let report = lines.join("\n").trim().to_string();
    let agent = agent.to_string();
    (!report.is_empty()).then_some(Heard::Reported { agent, report })
}

/// What opens a subagent's report, past the frame Claude Code puts around it.
const REPORT: &str = "The report follows:";
