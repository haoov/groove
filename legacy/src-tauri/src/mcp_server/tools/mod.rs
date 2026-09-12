//! The MCP tool surface: the dispatch table and the shared result type. `read`
//! answers from the DB and filesystem; `write` goes through the confirmation bridge.

mod args;
mod definitions;
mod read;
mod write;

pub(crate) use args::{InvalidParams, UnknownTool};
pub(crate) use definitions::mcp_tool_definitions;

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
    // The calling connection; resolves to the agent's own task, not the focused one.
    mcp_session: &str,
) -> anyhow::Result<ToolCallResponse> {
    use crate::approvals::ops;
    // `dispatched_names` reads these arms; keep one name literal per arm.
    match name {
        // Reads
        "get_active_task" => read::get_active_task(input, state, mcp_session).await,
        "list_tasks" => read::list_tasks(input, state).await,
        "list_repos" => read::list_repos(input, state, mcp_session).await,
        "get_task_diff" => read::get_task_diff(input, state, mcp_session).await,
        "get_commit_log" => read::get_commit_log(input, state, mcp_session).await,
        "get_mr_state" => read::get_mr_state(input, state).await,
        "get_mr_ci" => read::get_mr_ci(input, state).await,
        "get_mr_threads" => read::get_mr_threads(input, state).await,
        "get_annotations" => read::get_annotations(input, state, mcp_session).await,
        "get_task_time" => read::get_task_time(input, state, mcp_session).await,
        "get_open_file" => read::get_open_file(input, state, mcp_session).await,
        "get_task_body" => read::get_task_body(input, state, mcp_session).await,
        "get_task_schema" => read::get_task_schema(input, state, mcp_session).await,
        "list_relation_options" => read::list_relation_options(input, state, mcp_session).await,
        "get_task_template" => read::get_task_template(input, state, mcp_session).await,
        "read_user_skill" => read::read_user_skill(input).await,

        // Writes gated by the confirmation bridge
        "git_commit" => {
            bridged::<args::CommitArgs>(ops::GIT_COMMIT, input, state, mcp_session).await
        }
        "git_push" => bridged::<args::WorktreeArgs>(ops::GIT_PUSH, input, state, mcp_session).await,
        "git_pull" => bridged::<args::WorktreeArgs>(ops::GIT_PULL, input, state, mcp_session).await,
        "git_rebase" => {
            bridged::<args::RebaseArgs>(ops::GIT_REBASE, input, state, mcp_session).await
        }
        "create_mr" => {
            bridged::<args::CreateMrArgs>(ops::MR_CREATE, input, state, mcp_session).await
        }
        "update_mr" => {
            bridged::<args::UpdateMrArgs>(ops::MR_UPDATE, input, state, mcp_session).await
        }
        "close_mr" => bridged::<args::MrArgs>(ops::MR_CLOSE, input, state, mcp_session).await,
        "create_task_from_explorer" => {
            write::create_task_from_explorer(input, state, mcp_session).await
        }
        "create_task" => write::create_task(input, state, mcp_session).await,
        "update_task_property" => write::update_task_property(input, state, mcp_session).await,
        "log_task_hours" => write::log_task_hours(input, state, mcp_session).await,
        "finish_task" => write::finish_task(input, state, mcp_session).await,
        "update_task_body" => write::update_task_body(input, state, mcp_session).await,
        "add_task_repo" => write::add_task_repo(input, state, mcp_session).await,
        "add_task_worktree" => write::add_task_worktree(input, state, mcp_session).await,
        "save_user_skill" => {
            bridged::<args::SaveSkillArgs>(ops::SKILL_SAVE, input, state, mcp_session).await
        }

        // Writes without approval
        "create_annotation" => write::create_annotation(input, state).await,
        "update_annotation" => write::update_annotation(input, state).await,
        "resolve_annotation" => write::resolve_annotation(input, state).await,

        _ => Err(anyhow::Error::new(UnknownTool(name.to_string()))),
    }
}

/// Type a gated write's arguments, then hand the bridge the payload they describe.
async fn bridged<T: args::ToolArgs + serde::Serialize>(
    op_type: &str,
    input: serde_json::Value,
    state: &McpState,
    mcp_session: &str,
) -> anyhow::Result<ToolCallResponse> {
    write::via_bridge(op_type, args::payload::<T>(input)?, state, mcp_session).await
}

// ─── Definitions and dispatch agree ───────────────────────────────────────────

/// Every tool the dispatcher answers, read from this file's own match arms.
#[cfg(test)]
fn dispatched_names() -> Vec<String> {
    let body = include_str!("mod.rs")
        .split_once("match name {")
        .expect("the dispatch match")
        .1
        .split_once("_ => Err(")
        .expect("the fallback arm")
        .0;
    body.lines()
        .filter_map(|line| line.split_once("=>"))
        .flat_map(|(head, _)| {
            head.split('"')
                .skip(1)
                .step_by(2)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect()
}

#[cfg(test)]
mod dispatch_tests {
    use super::{dispatched_names, mcp_tool_definitions};

    fn advertised() -> Vec<String> {
        mcp_tool_definitions()
            .iter()
            .map(|t| t["name"].as_str().expect("a tool name").to_string())
            .collect()
    }

    #[test]
    fn every_advertised_tool_has_a_dispatch_arm() {
        let dispatched = dispatched_names();
        assert!(dispatched.len() > 20, "the match arms did not parse");
        for name in advertised() {
            assert!(
                dispatched.contains(&name),
                "tools/list advertises {name}, dispatch has no arm for it"
            );
        }
    }

    #[test]
    fn every_dispatch_arm_is_advertised() {
        let advertised = advertised();
        for name in dispatched_names() {
            assert!(
                advertised.contains(&name),
                "dispatch answers {name}, tools/list does not advertise it"
            );
        }
    }

    #[test]
    fn no_tool_is_advertised_twice() {
        let mut seen = advertised();
        seen.sort();
        let count = seen.len();
        seen.dedup();
        assert_eq!(seen.len(), count, "a tool name is advertised twice");
    }
}
