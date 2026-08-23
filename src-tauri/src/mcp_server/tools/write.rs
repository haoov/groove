//! Tools that change something.
//!
//! Anything the user would want to see before it happens (git, MRs, Notion) goes
//! through the confirmation bridge via `post_and_wait`; annotations are local and
//! reversible, so they apply immediately and are pushed to the UI instead.

use tauri::Emitter;

use crate::approvals::ResolveOutcome;
use crate::core::db::models::SessionKind;
use crate::core::db::store;

use super::{str_field, McpState, ToolCallResponse};

/// Post a confirmation and block until the user decides.
///
/// The sender is registered under a pre-generated id BEFORE the row is posted, so
/// a resolve can never race ahead of the registration.
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

/// Git ops need `worktree_path` and `branch`, but MCP callers only know ids.
async fn enrich_worktree_fields(payload: &mut serde_json::Value, state: &McpState) {
    let Some(wt_id) = payload["worktree_id"].as_str().map(|s| s.to_string()) else {
        return;
    };
    if let Ok(wt) = store::worktrees::get(&state.pool, &wt_id).await {
        payload["worktree_path"] = serde_json::json!(wt.path);
        payload["branch"] = serde_json::json!(wt.branch);
    }
}

pub(super) async fn via_bridge(
    op_type: &str,
    mut payload: serde_json::Value,
    state: &McpState,
    mcp_session: &str,
) -> anyhow::Result<ToolCallResponse> {
    // Explorer sessions are scratch by policy: committing locally is fine, but
    // publishing (push, MR) means the work is real — convert to a task first so
    // it exists in Notion. Scoped to those ops specifically: Notion writes and
    // task filing work fine from an explorer.
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
    let task_id = state.task_for(mcp_session);

    // Asking twice for the same thing means the first call is still queued (the
    // user deferred it). Tell the caller to wait rather than posting a second row
    // the user would have to decide twice.
    if state
        .bridge
        .has_identical_pending(&state.pool, op_type, task_id.as_deref(), &payload)
        .await
    {
        return Ok(ToolCallResponse::err(
            "An identical action is already waiting for the user's approval. It stays queued \
             until they decide — do not retry; continue with other work or ask the user.",
        ));
    }

    match post_and_wait(state, op_type, payload, task_id.as_deref()).await? {
        // Never hand back a bare `null` — it reads as "nothing happened".
        ResolveOutcome::Approved(value) if value.is_null() => Ok(ToolCallResponse::ok(
            serde_json::json!({ "ok": true, "op": op_type, "message": "Completed" }),
        )),
        ResolveOutcome::Approved(value) => Ok(ToolCallResponse::ok(value)),
        ResolveOutcome::Rejected => Ok(ToolCallResponse::err("Operation rejected by user")),
        ResolveOutcome::Failed(e) => Ok(ToolCallResponse::err(format!(
            "Operation approved but failed: {e}"
        ))),
    }
}

/// Reject anything that isn't a live explorer before asking the user to approve
/// anything, instead of re-pointing a real task's worktrees onto a brand-new one.
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
            "The focused session is {explorer_id}, a real task — not an explorer. \
             Ask the user to focus the explorer session they want converted."
        ))),
        Some(kind) => Some(ToolCallResponse::err(format!(
            "{explorer_id} is a {kind:?} session — only explorer sessions convert to tasks."
        ))),
    })
}

/// Draft + create a Notion task from the active explorer session (gated by the
/// confirmation bridge), then point the backend's active task at the new id so
/// subsequent agent tool calls resolve to the real task.
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

    let cfg = crate::core::config::require()?;
    let n = &cfg.notion;

    // NOTE: no notion token here — secrets are injected by `execute_op` at
    // execution time so they never sit in persisted/emitted confirmation payloads.
    let payload = serde_json::json!({
        "explorer_id": explorer_id,
        "database_id": n.database_id,
        "title": title,
        "body_markdown": body_markdown,
        "status_prop": n.properties.status,
        "status_value": n.status_map.ready,
        "assignee_prop": n.properties.assignee,
        "user_id": n.user_id,
        "sprint_prop": n.properties.sprint,
        "project_prop": n.properties.project,
        "project_id": n.default_project_id,
    });

    let outcome = post_and_wait(
        state,
        crate::approvals::ops::TASK_CREATE_FROM_EXPLORER,
        payload,
        Some(&explorer_id),
    )
    .await?;

    match outcome {
        ResolveOutcome::Approved(task) => {
            if let Some(sid) = task["short_id"].as_str() {
                state.task_state.set_active_task_id(Some(sid.to_string()));
                // The explorer id this connection was bound to no longer exists.
                state.rebind(mcp_session, sid);
            }
            Ok(ToolCallResponse::ok(task))
        }
        ResolveOutcome::Rejected => Ok(ToolCallResponse::err("Task creation rejected by user")),
        ResolveOutcome::Failed(e) => Ok(ToolCallResponse::err(format!(
            "Task creation approved but failed: {e}"
        ))),
    }
}

/// Stamp `[claude]` into an annotation's Conventional Comment header.
///
/// Done here rather than asked of the agent: authorship is a fact the app knows,
/// and the `author` column does not survive promotion to a GitLab comment — that
/// posts under the user's own account, so without this the reader cannot tell who
/// wrote it. Sits after the decoration so the label still parses:
///   `issue (non-blocking): …` → `issue (non-blocking)[claude]: …`
fn mark_as_agent(content: &str) -> String {
    if content.contains("[claude]") {
        return content.to_string();
    }
    // Only the first line carries the header; a body may legitimately contain ':'.
    match content.split_once(':') {
        Some((head, rest)) if !head.contains('\n') && !head.trim().is_empty() => {
            format!("{}[claude]:{rest}", head.trim_end())
        }
        // Not a Conventional Comment (the hook will reject it anyway) — prefix it
        // so the marker is never silently dropped.
        _ => format!("[claude] {content}"),
    }
}

pub(super) async fn create_annotation(
    input: serde_json::Value,
    state: &McpState,
) -> anyhow::Result<ToolCallResponse> {
    // Agents annotate a single line (or an optional [start_line, end_line] range).
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

// ─── Notion task writes ───────────────────────────────────────────────────────

/// File a task in Notion. Nothing is opened or provisioned — it lands in the
/// queue for later, which is what "file a task" means.
pub(super) async fn create_task(
    input: serde_json::Value,
    state: &McpState,
    mcp_session: &str,
) -> anyhow::Result<ToolCallResponse> {
    let title = str_field(&input, "title")?;
    let body_markdown = input["body_markdown"].as_str().unwrap_or("");
    let cfg = crate::core::config::require()?;

    // Same payload the UI composer builds; the token is injected at execution.
    let payload = crate::provider::notion::new_task_payload(&cfg.notion, &title, body_markdown);
    let task_id = state.task_for(mcp_session);

    match post_and_wait(state, crate::approvals::ops::TASK_CREATE, payload, task_id.as_deref()).await? {
        ResolveOutcome::Approved(task) => Ok(ToolCallResponse::ok(task)),
        ResolveOutcome::Rejected => Ok(ToolCallResponse::err("Task creation rejected by user")),
        ResolveOutcome::Failed(e) => Ok(ToolCallResponse::err(format!(
            "Task creation approved but failed: {e}"
        ))),
    }
}

/// Attach a repo already cloned under MAIN to a task, and provision its worktree.
///
/// Defaults to the caller's OWN task, like the other task-scoped writes — an agent
/// adding a repo means its own session, not whatever the user is looking at.
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

    let payload = serde_json::json!({
        "task_id": task_id,
        "repo": repo,
        "branch": input["branch"].as_str(),
    });

    match post_and_wait(state, crate::approvals::ops::TASK_ADD_REPO, payload, Some(&task_id)).await? {
        ResolveOutcome::Approved(result) => Ok(ToolCallResponse::ok(result)),
        ResolveOutcome::Rejected => Ok(ToolCallResponse::err("Adding the repo was rejected by the user")),
        // The resolution errors (unknown repo, ambiguous name, wrong session kind)
        // all surface here, and each one tells the agent what to do instead.
        ResolveOutcome::Failed(e) => Ok(ToolCallResponse::err(format!("Could not add the repo: {e}"))),
    }
}

/// The task a write applies to. The provider resolves it at execution time, so
/// the payload only ever carries the short id.
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

/// Set one property, whatever its type. The schema decides how the value is
/// interpreted, so this covers Priority, Platform Components, Tags and anything
/// added to the database later.
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
    bridged(state, crate::approvals::ops::TASK_PROPERTY, payload, &task_id).await
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
    bridged(state, crate::approvals::ops::TASK_HOURS, payload, &task_id).await
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
    bridged(state, crate::approvals::ops::TASK_BODY, payload, &task_id).await
}

/// Post a pre-built payload and map the outcome — the tail every gated write
/// shares once its payload is assembled.
async fn bridged(
    state: &McpState,
    op_type: &str,
    payload: serde_json::Value,
    task_id: &str,
) -> anyhow::Result<ToolCallResponse> {
    if state
        .bridge
        .has_identical_pending(&state.pool, op_type, Some(task_id), &payload)
        .await
    {
        return Ok(ToolCallResponse::err(
            "An identical change is already waiting for the user's approval — do not retry.",
        ));
    }
    match post_and_wait(state, op_type, payload, Some(task_id)).await? {
        ResolveOutcome::Approved(v) if v.is_null() => Ok(ToolCallResponse::ok(
            serde_json::json!({ "ok": true, "op": op_type }),
        )),
        ResolveOutcome::Approved(v) => Ok(ToolCallResponse::ok(v)),
        ResolveOutcome::Rejected => Ok(ToolCallResponse::err("Rejected by user")),
        ResolveOutcome::Failed(e) => Ok(ToolCallResponse::err(format!("Approved but failed: {e}"))),
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

    /// Not a Conventional Comment — the hook rejects it, but the marker must not
    /// vanish in the meantime.
    #[test]
    fn unparseable_content_still_gets_marked() {
        assert_eq!(mark_as_agent("this just panics"), "[claude] this just panics");
    }

    #[test]
    fn multiline_header_is_not_assumed() {
        let out = mark_as_agent("no header here\nsecond: line");
        assert!(out.starts_with("[claude] "), "{out}");
    }
}
