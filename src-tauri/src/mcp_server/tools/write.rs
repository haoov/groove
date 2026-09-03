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

/// Refuse a request identical to one already awaiting approval.
///
/// The wait in `post_and_wait` has no deadline of its own, so a caller that gave
/// up — an MCP idle timeout, a reload, an agent simply trying again — leaves its
/// confirmation queued and approvable. Without this, the retry queues a SECOND
/// one, and for the create ops that means a duplicate task filed at the source.
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
        // The project name, for display: the path's last segment is the branch
        // leaf now that worktree dirs carry the branch's slashes.
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
    // Explorer sessions are scratch by policy: committing locally is fine, but
    // publishing (push, MR) means the work is real — convert to a task first so
    // it is a real task. Scoped to those ops specifically: task writes and
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
    // The dialog names where the MR lands.
    if op_type == crate::approvals::ops::MR_CREATE {
        if let Some(wt_id) = payload["worktree_id"].as_str().map(|s| s.to_string()) {
            if let Ok(target) = crate::forge::mr_target_for(&state.pool, &wt_id).await {
                payload["target_branch"] = serde_json::json!(target);
            }
        }
    }
    let task_id = state.task_for(mcp_session);

    // Asking twice for the same thing means the first call is still queued (the
    // user deferred it). Tell the caller to wait rather than posting a second row
    // the user would have to decide twice.
    if let Some(refusal) = already_pending(state, op_type, task_id.as_deref(), &payload).await {
        return Ok(refusal);
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

/// Draft + file a task from the active explorer session (gated by the
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

    // The source's own settings are read by the executor, not carried here: a
    // payload is persisted and emitted, and the token must never sit in one.
    let provider = crate::provider::commands::draft_provider(&input)?;

    // A GitHub issue needs a repo. Default to the explorer's own, which is what
    // the work was done in.
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

    if let Some(refusal) = already_pending(state, crate::approvals::ops::TASK_CREATE_FROM_EXPLORER, Some(explorer_id.as_str()), &payload).await {
        return Ok(refusal);
    }

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

/// File a task. Nothing is opened or provisioned — it lands in the queue for
/// later, which is what "file a task" means.
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

    if let Some(refusal) = already_pending(state, crate::approvals::ops::TASK_CREATE, task_id.as_deref(), &payload).await {
        return Ok(refusal);
    }

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
/// A second worktree on a repo the task already holds. Separate from
/// add_task_repo: the repo is not the question, the branch is.
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

    if let Some(refusal) = already_pending(state, crate::approvals::ops::TASK_ADD_WORKTREE, Some(&task_id), &payload).await {
        return Ok(refusal);
    }

    match post_and_wait(state, crate::approvals::ops::TASK_ADD_WORKTREE, payload, Some(&task_id)).await? {
        ResolveOutcome::Approved(result) => Ok(ToolCallResponse::ok(result)),
        ResolveOutcome::Rejected => {
            Ok(ToolCallResponse::err("Adding the worktree was rejected by the user"))
        }
        // Unknown repo, a branch already checked out, wrong session kind — each
        // message says what to do instead.
        ResolveOutcome::Failed(e) => {
            Ok(ToolCallResponse::err(format!("Could not add the worktree: {e}")))
        }
    }
}

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

    // Resolve the branch NOW rather than letting provisioning derive it: the
    // approval dialog has to name the branch it is about to create.
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

    if let Some(refusal) = already_pending(state, crate::approvals::ops::TASK_ADD_REPO, Some(&task_id), &payload).await {
        return Ok(refusal);
    }

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
    if let Some(refusal) = already_pending(state, op_type, Some(task_id), &payload).await {
        return Ok(refusal);
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
