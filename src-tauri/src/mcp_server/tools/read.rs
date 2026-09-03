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


/// Every real task from the local mirror; explorers and reviews are not tasks.
pub(super) async fn list_tasks(state: &McpState) -> anyhow::Result<ToolCallResponse> {
    let tasks: Vec<crate::core::db::models::TaskView> = store::provider_tasks::all(&state.pool)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    Ok(ToolCallResponse::ok(
        serde_json::json!({ "count": tasks.len(), "tasks": tasks }),
    ))
}

/// Repos in the clone pool, flagged `attached` when on the caller's task.
pub(super) async fn list_repos(
    state: &McpState,
    mcp_session: &str,
) -> anyhow::Result<ToolCallResponse> {
    let main = crate::worktrees::list_main_repos()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    // Matched on local_path: the pool listing has no repo id until registered.
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
                // The clone itself, for reading a repo the session has not checked out.
                "local_path": r.local_path,
            })
        })
        .collect();

    Ok(ToolCallResponse::ok(
        serde_json::json!({ "count": repos.len(), "repos": repos }),
    ))
}

/// The task a read is about: the one named, else the caller's own.
fn task_or_own(input: &serde_json::Value, state: &McpState, mcp_session: &str) -> anyhow::Result<String> {
    input["task_id"]
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| state.task_for(mcp_session))
        .ok_or_else(|| anyhow::anyhow!("no task in scope"))
}

pub(super) async fn get_task_diff(
    input: serde_json::Value,
    state: &McpState,
    mcp_session: &str,
) -> anyhow::Result<ToolCallResponse> {
    let task_id = task_or_own(&input, state, mcp_session)?;
    let result = crate::review::get_task_diff_mcp(&task_id, &state.pool).await?;
    Ok(ToolCallResponse::ok(serde_json::to_value(result)?))
}

pub(super) async fn get_commit_log(
    input: serde_json::Value,
    state: &McpState,
    mcp_session: &str,
) -> anyhow::Result<ToolCallResponse> {
    const DEFAULT_LIMIT: u64 = 20;
    let task_id = task_or_own(&input, state, mcp_session)?;
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
    mcp_session: &str,
) -> anyhow::Result<ToolCallResponse> {
    let task_id = task_or_own(&input, state, mcp_session)?;
    let rows = store::annotations::for_session(&state.pool, &task_id, None).await?;
    Ok(ToolCallResponse::ok(serde_json::to_value(rows)?))
}

/// Time measured for a task, and how much reached the source. Hours as well as
/// seconds — `log_task_hours` takes hours.
pub(super) async fn get_task_time(
    input: serde_json::Value,
    state: &McpState,
    mcp_session: &str,
) -> anyhow::Result<ToolCallResponse> {
    let task_id = task_or_own(&input, state, mcp_session)?;

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let t = store::time::summary(&state.pool, &task_id, &today).await?;
    let hours = |secs: i64| (secs as f64 / 360.0).round() / 10.0;

    Ok(ToolCallResponse::ok(serde_json::json!({
        "task_id": task_id,
        "tracked_hours": hours(t.tracked_seconds),
        "logged_hours": hours(t.logged_seconds),
        "unlogged_hours": hours(t.unlogged_seconds),
        "tracked_seconds": t.tracked_seconds,
        "logged_seconds": t.logged_seconds,
        "unlogged_seconds": t.unlogged_seconds,
    })))
}

/// The task source's live schema — never cached.
pub(super) async fn get_task_schema(
    input: serde_json::Value,
    state: &McpState,
    mcp_session: &str,
) -> anyhow::Result<ToolCallResponse> {
    let task_id = task_or_own(&input, state, mcp_session)?;

    let schema = crate::provider::schema_for(&state.pool, &task_id).await?;
    Ok(ToolCallResponse::ok(serde_json::to_value(schema)?))
}

/// The MR's pipeline status and run URL, live from the forge.
pub(super) async fn get_mr_ci(
    input: serde_json::Value,
    state: &McpState,
) -> anyhow::Result<ToolCallResponse> {
    let mr_id = str_field(&input, "mr_id")?;
    Ok(ToolCallResponse::ok(
        crate::forge::mr_ci_for(&state.pool, &mr_id).await?,
    ))
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


pub(super) async fn get_task_body(
    input: serde_json::Value,
    state: &McpState,
) -> anyhow::Result<ToolCallResponse> {
    let task_id = str_field(&input, "task_id")?;
    let (provider, key) = crate::provider::resolve(&state.pool, &task_id).await?;
    let markdown = provider.body_markdown(&key).await?;
    Ok(ToolCallResponse::ok(serde_json::json!({ "markdown": markdown })))
}

/// The task template as markdown.
pub(super) async fn get_task_template(
    input: serde_json::Value,
    state: &McpState,
    mcp_session: &str,
) -> anyhow::Result<ToolCallResponse> {
    // Source: the one named, else the task's, else the only one configured. An
    // explorer has no source and falls through.
    let named = input["provider"]
        .as_str()
        .map(|_| crate::provider::commands::draft_provider(&input));
    let in_scope = match (&named, state.task_for(mcp_session)) {
        (None, Some(task_id)) => crate::provider::resolve(&state.pool, &task_id)
            .await
            .ok()
            .map(|(p, _)| Ok(p.id())),
        _ => None,
    };
    let provider = named
        .or(in_scope)
        .unwrap_or_else(|| crate::provider::commands::draft_provider(&serde_json::json!({})))
        .and_then(crate::provider::get);
    let provider = match provider {
        Ok(p) => p,
        Err(e) => return Ok(ToolCallResponse::err(e.to_string())),
    };

    match provider.template_markdown().await {
        Ok(Some(markdown)) => Ok(ToolCallResponse::ok(
            serde_json::json!({ "template_markdown": markdown }),
        )),
        // No template is an answer, not an error.
        Ok(None) => Ok(ToolCallResponse::ok(serde_json::json!({
            "template_markdown": "",
            "note": "this task source has no template — structure the body yourself",
        }))),
        Err(e) => Ok(ToolCallResponse::err(e.to_string())),
    }
}

/// The raw `SKILL.md` of one of the user's own skills, so an edit rewrites a
/// whole file instead of guessing at what the rest of it said.
pub(super) async fn read_user_skill(
    input: serde_json::Value,
) -> anyhow::Result<ToolCallResponse> {
    let name = str_field(&input, "name")?;
    Ok(match crate::skills::read_user_skill(&name) {
        Ok(body) => ToolCallResponse::ok(serde_json::json!({ "name": name, "body": body })),
        Err(e) => ToolCallResponse::err(e.to_string()),
    })
}
