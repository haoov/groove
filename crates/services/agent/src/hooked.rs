//! What a hook says the agent is doing.

use groove_types::{AgentStatus, HookKind, SessionActivity, Timestamp, ToolCall};

use crate::ASKS_THE_USER;

/// What a hook says the agent is doing; a question or a prompt waits on the user.
pub(crate) fn hook(
    activity: &mut SessionActivity,
    kind: HookKind,
    tool: Option<ToolCall>,
    at: Timestamp,
) {
    let asks = tool.as_ref().is_some_and(|one| one.name == ASKS_THE_USER);
    let status = match kind {
        HookKind::SessionStart => AgentStatus::Idle,
        HookKind::PreToolUse if asks => AgentStatus::Asking,
        HookKind::UserPromptSubmit | HookKind::PreToolUse | HookKind::PostToolUse => {
            AgentStatus::Working
        }
        HookKind::Stop => AgentStatus::Done { seen: false },
        HookKind::Notification => AgentStatus::Asking,
    };
    activity.tool = match kind {
        HookKind::PreToolUse => tool,
        _ => None,
    };
    if activity.status != status {
        activity.changed_at = at;
    }
    activity.status = status;
}
