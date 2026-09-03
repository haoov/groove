//! The MCP tool surface: the dispatch table plus the shared result type.
//!
//! Tools split by whether they need the user: `read` answers straight from the
//! DB/filesystem, `write` goes through the confirmation bridge. `dispatch` stays
//! a flat name → function table so adding a tool never grows a function body.

mod definitions;
mod read;
mod write;

pub(super) use definitions::mcp_tool_definitions;

use serde::Serialize;

use super::McpState;

// ─── Tool result ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub(super) struct ToolCallResponse {
    pub(super) content: Vec<ContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) is_error: Option<bool>,
}

#[derive(Debug, Serialize)]
pub(super) struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

impl ToolCallResponse {
    fn ok(value: serde_json::Value) -> Self {
        Self {
            content: vec![ContentBlock {
                content_type: "text".to_string(),
                text: serde_json::to_string_pretty(&value).unwrap_or_default(),
            }],
            is_error: None,
        }
    }

    fn err(msg: impl Into<String>) -> Self {
        Self {
            content: vec![ContentBlock {
                content_type: "text".to_string(),
                text: msg.into(),
            }],
            is_error: Some(true),
        }
    }
}

// ─── Dispatch ─────────────────────────────────────────────────────────────────

pub(super) async fn dispatch(
    name: &str,
    input: serde_json::Value,
    state: &McpState,
    // The calling connection — resolves to the agent's OWN task, not the focused one.
    mcp_session: &str,
) -> anyhow::Result<ToolCallResponse> {
    use crate::approvals::ops;
    match name {
        // Reads
        "get_active_task" => read::get_active_task(state, mcp_session).await,
        "list_tasks" => read::list_tasks(state).await,
        "list_repos" => read::list_repos(state, mcp_session).await,
        "get_worktrees" => read::get_worktrees(input, state, mcp_session).await,
        "get_task_diff" => read::get_task_diff(input, state).await,
        "get_commit_log" => read::get_commit_log(input, state).await,
        "get_mr_state" => read::get_mr_state(input, state).await,
        "get_annotations" => read::get_annotations(input, state).await,
        "get_task_time" => read::get_task_time(input, state, mcp_session).await,
        "get_open_file" => read::get_open_file(state, mcp_session).await,
        "get_file_content" => read::get_file_content(input).await,
        "get_task_body" => read::get_task_body(input, state).await,
        "get_task_template" => read::get_task_template(input, state, mcp_session).await,
        "read_user_skill" => read::read_user_skill(input).await,

        // Writes gated by the confirmation bridge
        "git_commit" => write::via_bridge(ops::GIT_COMMIT, input, state, mcp_session).await,
        "git_push" => write::via_bridge(ops::GIT_PUSH, input, state, mcp_session).await,
        "git_pull" => write::via_bridge(ops::GIT_PULL, input, state, mcp_session).await,
        "git_rebase" => write::via_bridge(ops::GIT_REBASE, input, state, mcp_session).await,
        "create_mr" => write::via_bridge(ops::MR_CREATE, input, state, mcp_session).await,
        "update_mr" => write::via_bridge(ops::MR_UPDATE, input, state, mcp_session).await,
        "close_mr" => write::via_bridge(ops::MR_CLOSE, input, state, mcp_session).await,
        "create_task_from_explorer" => {
            write::create_task_from_explorer(input, state, mcp_session).await
        }
        "create_task" => write::create_task(input, state, mcp_session).await,
        "update_task_property" => write::update_task_property(input, state, mcp_session).await,
        "log_task_hours" => write::log_task_hours(input, state, mcp_session).await,
        "update_task_body" => write::update_task_body(input, state, mcp_session).await,
        "add_task_repo" => write::add_task_repo(input, state, mcp_session).await,
        "add_task_worktree" => write::add_task_worktree(input, state, mcp_session).await,
        "save_user_skill" => write::via_bridge(ops::SKILL_SAVE, input, state, mcp_session).await,

        // Writes the user never has to approve (local, reversible, UI-visible)
        "create_annotation" => write::create_annotation(input, state).await,
        "update_annotation" => write::update_annotation(input, state).await,
        "resolve_annotation" => write::resolve_annotation(input, state).await,

        _ => Err(anyhow::anyhow!("Unknown tool: {name}")),
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn str_field(input: &serde_json::Value, key: &str) -> anyhow::Result<String> {
    input[key]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("missing required field: {key}"))
}
