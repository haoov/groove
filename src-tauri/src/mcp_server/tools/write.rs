//! Tools that change something.
//!
//! Anything the user would want to see before it happens (git, MRs, tasks) goes
//! through the confirmation bridge via `post_and_wait`; annotations are local and
//! reversible, so they apply immediately and are pushed to the UI instead.

use tauri::Emitter;

use crate::approvals::ResolveOutcome;
use crate::core::db::models::SessionKind;
use crate::core::db::store;

use super::{str_field, McpState, ToolCallResponse};

/// Post a confirmation and block until the user decides. The sender is registered
/// BEFORE the row is posted, so a resolve cannot race it.
async fn post_and_wait(
    state: &McpState,
    op_type: &str,
    payload: serde_json::Value,
    task_id: Option<&str>,
) -> anyhow::Result<ResolveOutcome> {
    let (tx, rx) = tokio::sync::oneshot::channel::<ResolveOutcome>();
    let id = uuid::Uuid::new_v4().to_string();
    state.bridge.register_sender(&id, tx);

    if let Err(e) = state
        .bridge
        .post_with_id(&id, &state.pool, op_type, payload, "mcp", task_id)
        .await
    {
        state.bridge.remove_sender(&id);
        return Err(e);
    }

    rx.await
        .map_err(|_| anyhow::anyhow!("confirmation channel closed"))
}

/// Refuse a request identical to one already awaiting approval — a retry must not
/// queue a second copy.
async fn already_pending(
    state: &McpState,
    op_type: &str,
    task_id: Option<&str>,
    payload: &serde_json::Value,
) -> Option<ToolCallResponse> {
    state
        .bridge
        .has_identical_pending(&state.pool, op_type, task_id, payload)
        .await
        .then(|| {
            ToolCallResponse::err(
                "An identical request is already waiting for the user's approval. It stays \
                 queued until they decide — do not retry; continue with other work or ask \
                 the user.",
            )
        })
}

/// Git ops need `worktree_path` and `branch`, but MCP callers only know ids.
async fn enrich_worktree_fields(payload: &mut serde_json::Value, state: &McpState) {
    let Some(wt_id) = payload["worktree_id"].as_str().map(|s| s.to_string()) else {
        return;
    };
    if let Ok(wt) = store::worktrees::get(&state.pool, &wt_id).await {
        payload["worktree_path"] = serde_json::json!(wt.path);
        payload["branch"] = serde_json::json!(wt.branch);
        // The project name for display; the path's last segment is the branch leaf.
        if let Ok(Some(repo)) = store::repos::get_opt(&state.pool, &wt.repo_id).await {
            payload["repo"] = serde_json::json!(repo.project);
        }
    }
}

pub(super) async fn via_bridge(
    op_type: &str,
    mut payload: serde_json::Value,
    state: &McpState,
    mcp_session: &str,
) -> anyhow::Result<ToolCallResponse> {
    // An explorer is scratch: it converts to a task before anything is published.
    const NEEDS_BRANCH: [&str; 4] = [
        crate::approvals::ops::GIT_PUSH,
        crate::approvals::ops::GIT_PULL,
        crate::approvals::ops::GIT_REBASE,
        crate::approvals::ops::MR_CREATE,
    ];
    let is_explorer = match state.task_for(mcp_session) {
        Some(id) => matches!(
            store::sessions::kind_of(&state.pool, &id).await?,
            Some(SessionKind::Explorer)
        ),
        None => false,
    };
    if is_explorer && NEEDS_BRANCH.contains(&op_type) {
        return Ok(ToolCallResponse::err(
            "This is an explorer session — convert it to a task (create_task_from_explorer) before pushing, rebasing, or opening an MR.",
        ));
    }

    enrich_worktree_fields(&mut payload, state).await;
    // The dialog names where the MR lands.
    if op_type == crate::approvals::ops::MR_CREATE {
        if let Some(wt_id) = payload["worktree_id"].as_str().map(|s| s.to_string()) {
            if let Ok(target) = crate::forge::mr_target_for(&state.pool, &wt_id).await {
                payload["target_branch"] = serde_json::json!(target);
            }
        }
    }
    // An agent commits exactly its index — see commit_impl.
    if op_type == crate::approvals::ops::GIT_COMMIT {
        payload["index_only"] = serde_json::json!(true);
    }
    let task_id = state.task_for(mcp_session);
    bridged(state, op_type, payload, task_id.as_deref()).await
}

/// Only a live explorer converts.
async fn check_convertible(
    explorer_id: &str,
    state: &McpState,
) -> anyhow::Result<Option<ToolCallResponse>> {
    Ok(match store::sessions::kind_of(&state.pool, explorer_id).await? {
        None => Some(ToolCallResponse::err(format!(
            "no session {explorer_id} to convert"
        ))),
        Some(SessionKind::Explorer) => None,
        Some(SessionKind::Task) => Some(ToolCallResponse::err(format!(
            "This session is {explorer_id}, a real task — not an explorer. Only an \
             explorer converts; there is nothing here to convert."
        ))),
        Some(kind) => Some(ToolCallResponse::err(format!(
            "{explorer_id} is a {kind:?} session — only explorer sessions convert to tasks."
        ))),
    })
}

/// File a task from the explorer session, then rebind this connection to the new id.
pub(super) async fn create_task_from_explorer(
    input: serde_json::Value,
    state: &McpState,
    mcp_session: &str,
) -> anyhow::Result<ToolCallResponse> {
    let title = str_field(&input, "title")?;
    let body_markdown = input["body_markdown"].as_str().unwrap_or("").to_string();

    let explorer_id = state
        .task_for(mcp_session)
        .ok_or_else(|| anyhow::anyhow!("no active explorer session"))?;
    if let Some(refusal) = check_convertible(&explorer_id, state).await? {
        return Ok(refusal);
    }

    // A payload is persisted and emitted — it must never carry the source token.
    let provider = crate::provider::commands::draft_provider(&input)?;

    // A GitHub issue needs a repo; default to the explorer's own.
    let repo = match input["repo"].as_str() {
        Some(r) => Some(r.to_string()),
        None => store::repos::attached_to(&state.pool, &explorer_id)
            .await
            .ok()
            .and_then(|rs| rs.first().map(|r| format!("{}/{}", r.group_path, r.project))),
    };

    let payload = serde_json::json!({
        "explorer_id": explorer_id,
        "provider": provider.as_str(),
        "title": title,
        "body_markdown": body_markdown,
        "repo": repo,
    });

    let op = crate::approvals::ops::TASK_CREATE_FROM_EXPLORER;
    if let Some(refusal) = already_pending(state, op, Some(&explorer_id), &payload).await {
        return Ok(refusal);
    }

    // The explorer id this connection is bound to no longer exists once approved.
    let outcome = post_and_wait(state, op, payload, Some(&explorer_id)).await?;
    if let ResolveOutcome::Approved(task) = &outcome {
        if let Some(sid) = task["short_id"].as_str() {
            state.task_state.set_active_task_id(Some(sid.to_string()));
            state.rebind(mcp_session, sid);
        }
    }
    Ok(outcome_response(op, outcome))
}

/// Stamp `[claude]` into an annotation's Conventional Comment header, after the
/// decoration: `issue (non-blocking): …` → `issue (non-blocking)[claude]: …`
fn mark_as_agent(content: &str) -> String {
    if content.contains("[claude]") {
        return content.to_string();
    }
    // Only the first line carries the header; a body may legitimately contain ':'.
    match content.split_once(':') {
        Some((head, rest)) if !head.contains('\n') && !head.trim().is_empty() => {
            format!("{}[claude]:{rest}", head.trim_end())
        }
        // Not a Conventional Comment — prefix so the marker is never dropped.
        _ => format!("[claude] {content}"),
    }
}

pub(super) async fn create_annotation(
    input: serde_json::Value,
    state: &McpState,
) -> anyhow::Result<ToolCallResponse> {
    // A single line, or a [start_line, end_line] range.
    let start_line = input["start_line"]
        .as_i64()
        .unwrap_or_else(|| input["line_num"].as_i64().unwrap_or(0));
    let end_line = input["end_line"].as_i64().unwrap_or(start_line);

    let row = store::annotations::create(
        &state.pool,
        &str_field(&input, "task_id")?,
        &str_field(&input, "repo_id")?,
        &str_field(&input, "file_path")?,
        start_line,
        end_line,
        &mark_as_agent(&str_field(&input, "content")?),
        &str_field(&input, "author").unwrap_or_else(|_| "agent".to_string()),
    )
    .await?;

    // Push it to the UI so the gutter/panel updates live.
    let _ = state
        .bridge
        .app_handle()
        .emit(crate::core::events::ANNOTATION_CREATED, serde_json::to_value(&row)?);

    Ok(ToolCallResponse::ok(serde_json::to_value(row)?))
}

pub(super) async fn update_annotation(
    input: serde_json::Value,
    state: &McpState,
) -> anyhow::Result<ToolCallResponse> {
    let id = str_field(&input, "id")?;
    // The agent wrote this text, whoever drafted the note first.
    let content = mark_as_agent(&str_field(&input, "content")?);
    let row = store::annotations::update(&state.pool, &id, &content).await?;

    let _ = state
        .bridge
        .app_handle()
        .emit(crate::core::events::ANNOTATION_UPDATED, serde_json::to_value(&row)?);

    Ok(ToolCallResponse::ok(serde_json::to_value(row)?))
}

pub(super) async fn resolve_annotation(
    input: serde_json::Value,
    state: &McpState,
) -> anyhow::Result<ToolCallResponse> {
    let id = input["id"].as_str().unwrap_or("").to_string();
    if id.is_empty() {
        return Err(anyhow::anyhow!("missing id"));
    }
    store::annotations::resolve(&state.pool, &id).await?;
    let _ = state.bridge.app_handle().emit(
        crate::core::events::ANNOTATION_RESOLVED,
        serde_json::json!({ "id": id }),
    );
    Ok(ToolCallResponse::ok(serde_json::json!({
        "ok": true, "message": format!("Annotation {id} resolved"),
    })))
}

// ─── Task writes ──────────────────────────────────────────────────────────────

/// File a task into the queue. Nothing is opened or provisioned.
pub(super) async fn create_task(
    input: serde_json::Value,
    state: &McpState,
    mcp_session: &str,
) -> anyhow::Result<ToolCallResponse> {
    let title = str_field(&input, "title")?;
    let payload = serde_json::json!({
        "title": title,
        "body_markdown": input["body_markdown"].as_str().unwrap_or(""),
        "provider": input["provider"].as_str(),
        "repo": input["repo"].as_str(),
    });
    let task_id = state.task_for(mcp_session);
    bridged(state, crate::approvals::ops::TASK_CREATE, payload, task_id.as_deref()).await
}

/// A second worktree on a repo the task already holds.
pub(super) async fn add_task_worktree(
    input: serde_json::Value,
    state: &McpState,
    mcp_session: &str,
) -> anyhow::Result<ToolCallResponse> {
    let branch = str_field(&input, "branch")?;
    let Some(task_id) = input["task_id"]
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| state.task_for(mcp_session))
    else {
        return Ok(ToolCallResponse::err(
            "No task to add the worktree to — open a task session first.",
        ));
    };

    let payload = serde_json::json!({
        "task_id": task_id,
        "branch": branch,
        "repo": input["repo"].as_str(),
        "target_branch": input["target_branch"].as_str(),
    });
    bridged(state, crate::approvals::ops::TASK_ADD_WORKTREE, payload, Some(&task_id)).await
}

/// Attach a cloned repo to a task and provision its worktree. Defaults to the caller's task.
pub(super) async fn add_task_repo(
    input: serde_json::Value,
    state: &McpState,
    mcp_session: &str,
) -> anyhow::Result<ToolCallResponse> {
    let repo = str_field(&input, "repo")?;
    let Some(task_id) = input["task_id"]
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| state.task_for(mcp_session))
    else {
        return Ok(ToolCallResponse::err(
            "No task to add the repo to — open a task session first.",
        ));
    };

    // Resolve the branch now so the approval dialog can name it.
    let branch = match input["branch"].as_str().map(str::trim).filter(|b| !b.is_empty()) {
        Some(b) => b.to_string(),
        None => crate::worktrees::default_branch_for(&task_id, &state.pool)
            .await
            .map_err(|e| anyhow::anyhow!("cannot work out a branch for {task_id}: {e}"))?,
    };

    let payload = serde_json::json!({
        "task_id": task_id,
        "repo": repo,
        "branch": branch,
        "target_branch": input["target_branch"].as_str(),
    });
    bridged(state, crate::approvals::ops::TASK_ADD_REPO, payload, Some(&task_id)).await
}

/// The task a write applies to: the one named, else the caller's own. It must have a
/// source behind it.
async fn task_target(
    state: &McpState,
    mcp_session: &str,
    input: &serde_json::Value,
) -> anyhow::Result<Result<String, ToolCallResponse>> {
    // An explicit task_id wins, so an agent can update a task it isn't sitting in.
    let task_id = match input["task_id"].as_str() {
        Some(id) => id.to_string(),
        None => match state.task_for(mcp_session) {
            Some(id) => id,
            None => return Ok(Err(ToolCallResponse::err("no task in scope"))),
        },
    };
    let has_source = store::sessions::get_opt(&state.pool, &task_id)
        .await?
        .and_then(|s| s.external_id)
        .is_some();
    match has_source {
        true => Ok(Ok(task_id)),
        false => Ok(Err(ToolCallResponse::err(format!(
            "{task_id} has no task behind it — explorer and review sessions aren't tasks"
        )))),
    }
}

/// Set one property; the schema decides how the value is interpreted.
pub(super) async fn update_task_property(
    input: serde_json::Value,
    state: &McpState,
    mcp_session: &str,
) -> anyhow::Result<ToolCallResponse> {
    let property = str_field(&input, "property")?;
    let task_id = match task_target(state, mcp_session, &input).await? {
        Ok(id) => id,
        Err(refusal) => return Ok(refusal),
    };
    let payload = serde_json::json!({
        "task_id": task_id,
        "property": property,
        "value": input["value"].clone(),
    });
    bridged(state, crate::approvals::ops::TASK_PROPERTY, payload, Some(&task_id)).await
}

/// Add hours to the task's "Hours spent". Adds — never replaces.
pub(super) async fn log_task_hours(
    input: serde_json::Value,
    state: &McpState,
    mcp_session: &str,
) -> anyhow::Result<ToolCallResponse> {
    let hours = input["hours"]
        .as_f64()
        .ok_or_else(|| anyhow::anyhow!("hours must be a number"))?;
    let task_id = match task_target(state, mcp_session, &input).await? {
        Ok(id) => id,
        Err(refusal) => return Ok(refusal),
    };
    let payload = serde_json::json!({ "task_id": task_id, "hours": hours });
    bridged(state, crate::approvals::ops::TASK_HOURS, payload, Some(&task_id)).await
}

/// Mark the task done and tear its workspace down — every worktree of the session.
pub(super) async fn finish_task(
    input: serde_json::Value,
    state: &McpState,
    mcp_session: &str,
) -> anyhow::Result<ToolCallResponse> {
    let task_id = match task_target(state, mcp_session, &input).await? {
        Ok(id) => id,
        Err(refusal) => return Ok(refusal),
    };
    let payload = serde_json::json!({ "task_id": task_id });
    bridged(state, crate::approvals::ops::TASK_FINISH, payload, Some(&task_id)).await
}

/// Replace the task's page body with markdown. Refuses when the page holds blocks
/// markdown can't rebuild, unless `force` says the loss is accepted.
pub(super) async fn update_task_body(
    input: serde_json::Value,
    state: &McpState,
    mcp_session: &str,
) -> anyhow::Result<ToolCallResponse> {
    let markdown = str_field(&input, "markdown")?;
    let task_id = match task_target(state, mcp_session, &input).await? {
        Ok(id) => id,
        Err(refusal) => return Ok(refusal),
    };
    let payload = serde_json::json!({
        "task_id": task_id,
        "markdown": markdown,
        "force": input["force"].as_bool().unwrap_or(false),
    });
    bridged(state, crate::approvals::ops::TASK_BODY, payload, Some(&task_id)).await
}

/// The tail every gated write shares: refuse a duplicate, post, wait, map.
async fn bridged(
    state: &McpState,
    op_type: &str,
    payload: serde_json::Value,
    task_id: Option<&str>,
) -> anyhow::Result<ToolCallResponse> {
    if let Some(refusal) = already_pending(state, op_type, task_id, &payload).await {
        return Ok(refusal);
    }
    let outcome = post_and_wait(state, op_type, payload, task_id).await?;
    Ok(outcome_response(op_type, outcome))
}

/// One outcome, one wording. A null result becomes an explicit success — the model
/// reads a bare null as failure.
fn outcome_response(op_type: &str, outcome: ResolveOutcome) -> ToolCallResponse {
    match outcome {
        ResolveOutcome::Approved(v) if v.is_null() => ToolCallResponse::ok(
            serde_json::json!({ "ok": true, "op": op_type, "message": "Completed" }),
        ),
        ResolveOutcome::Approved(v) => ToolCallResponse::ok(v),
        ResolveOutcome::Rejected => ToolCallResponse::err("Rejected by the user"),
        ResolveOutcome::Failed(e) => ToolCallResponse::err(format!("Approved but failed: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::mark_as_agent;

    #[test]
    fn stamps_after_the_decoration() {
        assert_eq!(
            mark_as_agent("issue (non-blocking): `parse_ref` panics."),
            "issue (non-blocking)[claude]: `parse_ref` panics."
        );
    }

    #[test]
    fn stamps_a_bare_label() {
        assert_eq!(mark_as_agent("suggestion: extract this."), "suggestion[claude]: extract this.");
    }

    /// The body often contains a colon; only the header may be rewritten.
    #[test]
    fn only_the_header_is_touched() {
        let out = mark_as_agent("issue: broke at 10:32, see log:line 4");
        assert_eq!(out, "issue[claude]: broke at 10:32, see log:line 4");
    }

    #[test]
    fn never_stamps_twice() {
        let once = mark_as_agent("issue: x");
        assert_eq!(mark_as_agent(&once), once);
    }

    /// Unparseable content still gets the marker.
    #[test]
    fn unparseable_content_still_gets_marked() {
        assert_eq!(mark_as_agent("this just panics"), "[claude] this just panics");
    }
}
