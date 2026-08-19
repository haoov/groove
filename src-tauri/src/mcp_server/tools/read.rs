//! Read-only tools: no confirmation, no side effects.

use crate::core::db::store;

use super::{str_field, McpState, ToolCallResponse};

pub(super) async fn get_active_task(
    state: &McpState,
    mcp_session: &str,
) -> anyhow::Result<ToolCallResponse> {
    let Some(task_id) = state.task_for(mcp_session) else {
        return Ok(ToolCallResponse::ok(serde_json::json!({ "active_task": null })));
    };

    let task = store::sessions::view_opt(&state.pool, &task_id).await?;
    let worktrees = store::worktrees::for_session(&state.pool, &task_id).await?;
    let repos = store::repos::attached_to(&state.pool, &task_id).await?;

    Ok(ToolCallResponse::ok(serde_json::json!({
        "active_task": task,
        "worktrees": worktrees,
        "repos": repos,
    })))
}

pub(super) async fn get_worktrees(
    input: serde_json::Value,
    state: &McpState,
    mcp_session: &str,
) -> anyhow::Result<ToolCallResponse> {
    let task_id = input["task_id"]
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| state.task_for(mcp_session));

    let Some(task_id) = task_id else {
        return Ok(ToolCallResponse::ok(serde_json::json!({ "worktrees": [] })));
    };

    let worktrees = store::worktrees::for_session(&state.pool, &task_id).await?;

    Ok(ToolCallResponse::ok(serde_json::to_value(worktrees)?))
}

/// Every real task the app knows about, from the local mirror.
///
/// The desk agent has no worktrees and no diff, so without this it cannot answer
/// "what am I working on". Synthetic rows are excluded: an explorer, a review or
/// the desk itself is not something picked off a queue.
pub(super) async fn list_tasks(state: &McpState) -> anyhow::Result<ToolCallResponse> {
    let tasks: Vec<crate::core::db::models::TaskView> = store::notion_tasks::all(&state.pool)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(ToolCallResponse::ok(
        serde_json::json!({ "count": tasks.len(), "tasks": tasks }),
    ))
}

/// Repos in the clone pool, flagged with whether they are on the caller's task.
///
/// `add_task_repo` takes a slug or project name, so the agent needs to see the
/// real names rather than guess them.
pub(super) async fn list_repos(
    state: &McpState,
    mcp_session: &str,
) -> anyhow::Result<ToolCallResponse> {
    let main = crate::worktrees::list_main_repos()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    // Attached repos are matched on local_path: the pool listing has no repo id
    // until a repo is registered, and registering is `add_task_repo`'s job.
    let attached: Vec<String> = match state.task_for(mcp_session) {
        Some(task_id) => store::repos::attached_paths(&state.pool, &task_id).await?,
        None => vec![],
    };

    let repos: Vec<serde_json::Value> = main
        .into_iter()
        .map(|r| {
            let project = r.slug.rsplit('/').next().unwrap_or(&r.slug).to_string();
            serde_json::json!({
                "slug": r.slug,
                "project": project,
                "attached": attached.contains(&r.local_path),
            })
        })
        .collect();

    Ok(ToolCallResponse::ok(
        serde_json::json!({ "count": repos.len(), "repos": repos }),
    ))
}

pub(super) async fn get_task_diff(
    input: serde_json::Value,
    state: &McpState,
) -> anyhow::Result<ToolCallResponse> {
    let task_id = str_field(&input, "task_id")?;
    let result = crate::review::get_task_diff_mcp(&task_id, &state.pool).await?;
    Ok(ToolCallResponse::ok(serde_json::to_value(result)?))
}

pub(super) async fn get_commit_log(
    input: serde_json::Value,
    state: &McpState,
) -> anyhow::Result<ToolCallResponse> {
    const DEFAULT_LIMIT: u64 = 20;
    let task_id = str_field(&input, "task_id")?;
    let limit = input["limit"].as_u64().unwrap_or(DEFAULT_LIMIT) as u32;
    let log = crate::review::get_commit_log_mcp(&task_id, limit, &state.pool).await?;
    Ok(ToolCallResponse::ok(serde_json::to_value(log)?))
}

pub(super) async fn get_mr_state(
    input: serde_json::Value,
    state: &McpState,
) -> anyhow::Result<ToolCallResponse> {
    let worktree_id = str_field(&input, "worktree_id")?;
    let mr = store::mrs::latest_for_worktree(&state.pool, &worktree_id).await?;
    Ok(ToolCallResponse::ok(serde_json::to_value(mr)?))
}

pub(super) async fn get_annotations(
    input: serde_json::Value,
    state: &McpState,
) -> anyhow::Result<ToolCallResponse> {
    let task_id = str_field(&input, "task_id")?;
    let rows = store::annotations::for_session(&state.pool, &task_id, None).await?;
    Ok(ToolCallResponse::ok(serde_json::to_value(rows)?))
}

pub(super) async fn get_open_file(
    state: &McpState,
    mcp_session: &str,
) -> anyhow::Result<ToolCallResponse> {
    let active_id = state.task_for(mcp_session);
    let open_file = state.editor_state.get_open_file();
    Ok(ToolCallResponse::ok(
        serde_json::json!({ "active_task_id": active_id, "open_file": open_file }),
    ))
}

pub(super) async fn get_file_content(
    input: serde_json::Value,
) -> anyhow::Result<ToolCallResponse> {
    // 1 MiB — the agent has shell access for anything bigger.
    const MAX_FILE_BYTES: u64 = 1_048_576;

    let file_path = str_field(&input, "file_path")?;
    let meta = tokio::fs::metadata(&file_path).await?;
    if meta.len() > MAX_FILE_BYTES {
        return Ok(ToolCallResponse::err(format!(
            "{file_path} is {} bytes (cap {MAX_FILE_BYTES}); read it via the shell instead",
            meta.len()
        )));
    }
    let content = tokio::fs::read_to_string(&file_path).await?;
    Ok(ToolCallResponse::ok(serde_json::json!({ "content": content })))
}

pub(super) async fn get_task_body(input: serde_json::Value) -> anyhow::Result<ToolCallResponse> {
    let notion_page_id = str_field(&input, "notion_page_id")?;
    let cfg = crate::core::config::require()?;
    let blocks =
        crate::notion::get_task_body_impl(&notion_page_id, &cfg.notion.token).await?;
    Ok(ToolCallResponse::ok(
        serde_json::json!({ "blocks": blocks, "count": blocks.len() }),
    ))
}

pub(super) async fn get_task_template() -> anyhow::Result<ToolCallResponse> {
    let cfg = crate::core::config::require()?;
    let page_id = cfg
        .notion
        .task_template_page_id
        .clone()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("notion.task_template_page_id is not set in config"))?;

    // Markdown is what the agent mirrors (and what create_task_from_explorer takes
    // back) — raw block JSON was easy to misread. The impl validates the configured
    // id and returns actionable errors (database id, no access, empty).
    match crate::notion::body::template_markdown(&page_id, &cfg.notion.token).await {
        Ok(markdown) => Ok(ToolCallResponse::ok(
            serde_json::json!({ "template_markdown": markdown }),
        )),
        Err(e) => Ok(ToolCallResponse::err(e.to_string())),
    }
}
