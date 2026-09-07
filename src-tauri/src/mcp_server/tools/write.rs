//! Tools that change something. Git, MR and task writes go through the confirmation
//! bridge; annotations apply immediately and are pushed to the UI.

use tauri::Emitter;

use crate::approvals::ResolveOutcome;
use crate::core::db::models::SessionKind;
use crate::core::db::store;

use super::{str_field, McpState, ToolCallResponse};

/// Post a confirmation and block until the user decides. Registers the sender
/// before posting; keep that order.
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

/// Ops that act on a worktree's files. `mr.update` and `mr.close` are absent: they
/// resolve everything from `mr_id`.
const WORKTREE_OPS: [&str; 5] = [
    crate::approvals::ops::GIT_COMMIT,
    crate::approvals::ops::GIT_PUSH,
    crate::approvals::ops::GIT_PULL,
    crate::approvals::ops::GIT_REBASE,
    crate::approvals::ops::MR_CREATE,
];

/// Set `worktree_path`, `branch` and `repo` from the row `worktree_id` names, and refuse a
/// worktree the bound session does not own. The caller's own copies of those three fields are
/// dropped first: they decide which directory git runs in and what the dialog says.
async fn bind_worktree(
    payload: &mut serde_json::Value,
    pool: &sqlx::SqlitePool,
    task_id: Option<&str>,
) -> Option<ToolCallResponse> {
    if let Some(obj) = payload.as_object_mut() {
        obj.remove("worktree_path");
        obj.remove("branch");
        obj.remove("repo");
    }

    let Some(wt_id) = payload["worktree_id"].as_str().map(|s| s.to_string()) else {
        return Some(ToolCallResponse::err(
            "worktree_id is required. Read it from get_active_task and pass the worktree to act on.",
        ));
    };
    let Ok(wt) = store::worktrees::get(pool, &wt_id).await else {
        return Some(ToolCallResponse::err(format!("no worktree {wt_id}")));
    };
    if let Some(id) = task_id {
        if wt.session_id != id {
            return Some(ToolCallResponse::err(format!(
                "worktree {wt_id} belongs to another session"
            )));
        }
    }

    payload["worktree_path"] = serde_json::json!(wt.path);
    payload["branch"] = serde_json::json!(wt.branch);
    if let Ok(Some(repo)) = store::repos::get_opt(pool, &wt.repo_id).await {
        payload["repo"] = serde_json::json!(repo.project);
    }
    None
}

pub(super) async fn via_bridge(
    op_type: &str,
    mut payload: serde_json::Value,
    state: &McpState,
    mcp_session: &str,
) -> anyhow::Result<ToolCallResponse> {
    // Ops an explorer session may not run.
    const NEEDS_BRANCH: [&str; 4] = [
        crate::approvals::ops::GIT_PUSH,
        crate::approvals::ops::GIT_PULL,
        crate::approvals::ops::GIT_REBASE,
        crate::approvals::ops::MR_CREATE,
    ];
    let task_id = state.task_for(mcp_session);
    let is_explorer = match task_id.as_deref() {
        Some(id) => matches!(
            store::sessions::kind_of(&state.pool, id).await?,
            Some(SessionKind::Explorer)
        ),
        None => false,
    };
    if is_explorer && NEEDS_BRANCH.contains(&op_type) {
        return Ok(ToolCallResponse::err(
            "This is an explorer session — convert it to a task (create_task_from_explorer) before pushing, rebasing, or opening an MR.",
        ));
    }

    if WORKTREE_OPS.contains(&op_type) {
        if let Some(refusal) = bind_worktree(&mut payload, &state.pool, task_id.as_deref()).await {
            return Ok(refusal);
        }
    }
    // The dialog names where the MR lands.
    if op_type == crate::approvals::ops::MR_CREATE {
        if let Some(wt_id) = payload["worktree_id"].as_str().map(|s| s.to_string()) {
            if let Ok(target) = crate::forge::mr_target_for(&state.pool, &wt_id).await {
                payload["target_branch"] = serde_json::json!(target);
            }
        }
    }
    // An agent commits exactly its index; see commit_impl.
    if op_type == crate::approvals::ops::GIT_COMMIT {
        payload["index_only"] = serde_json::json!(true);
    }
    bridged(state, op_type, payload, task_id.as_deref()).await
}

/// Only a live explorer converts.
async fn check_convertible(
    explorer_id: &str,
    state: &McpState,
) -> anyhow::Result<Option<ToolCallResponse>> {
    Ok(
        match store::sessions::kind_of(&state.pool, explorer_id).await? {
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
        },
    )
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

    // The payload is persisted and emitted; it must never carry the source token.
    let provider = crate::provider::commands::draft_provider(&input)?;

    // Default `repo` to the explorer's first attached repo.
    let repo = match input["repo"].as_str() {
        Some(r) => Some(r.to_string()),
        None => store::repos::attached_to(&state.pool, &explorer_id)
            .await
            .ok()
            .and_then(|rs| {
                rs.first()
                    .map(|r| format!("{}/{}", r.group_path, r.project))
            }),
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

    // After approval the explorer id is gone; rebind the connection to the new task.
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
    // Only the first line is the header.
    match content.split_once(':') {
        Some((head, rest)) if !head.contains('\n') && !head.trim().is_empty() => {
            format!("{}[claude]:{rest}", head.trim_end())
        }
        // Not a Conventional Comment: prefix the marker.
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

    let _ = state.bridge.app_handle().emit(
        crate::core::events::ANNOTATION_CREATED,
        serde_json::to_value(&row)?,
    );

    Ok(ToolCallResponse::ok(serde_json::to_value(row)?))
}

pub(super) async fn update_annotation(
    input: serde_json::Value,
    state: &McpState,
) -> anyhow::Result<ToolCallResponse> {
    let id = str_field(&input, "id")?;
    let content = mark_as_agent(&str_field(&input, "content")?);
    let row = store::annotations::update(&state.pool, &id, &content).await?;

    let _ = state.bridge.app_handle().emit(
        crate::core::events::ANNOTATION_UPDATED,
        serde_json::to_value(&row)?,
    );

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
    bridged(
        state,
        crate::approvals::ops::TASK_CREATE,
        payload,
        task_id.as_deref(),
    )
    .await
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
    bridged(
        state,
        crate::approvals::ops::TASK_ADD_WORKTREE,
        payload,
        Some(&task_id),
    )
    .await
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

    // Resolve the branch for the approval dialog.
    let branch = match input["branch"]
        .as_str()
        .map(str::trim)
        .filter(|b| !b.is_empty())
    {
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
    bridged(
        state,
        crate::approvals::ops::TASK_ADD_REPO,
        payload,
        Some(&task_id),
    )
    .await
}

/// The task a write applies to: the one named, else the caller's own. It must have a source.
async fn task_target(
    state: &McpState,
    mcp_session: &str,
    input: &serde_json::Value,
) -> anyhow::Result<Result<String, ToolCallResponse>> {
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
    bridged(
        state,
        crate::approvals::ops::TASK_PROPERTY,
        payload,
        Some(&task_id),
    )
    .await
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
    bridged(
        state,
        crate::approvals::ops::TASK_HOURS,
        payload,
        Some(&task_id),
    )
    .await
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
    bridged(
        state,
        crate::approvals::ops::TASK_FINISH,
        payload,
        Some(&task_id),
    )
    .await
}

/// Replace the task's page body with markdown. Refuses a lossy page unless `force`.
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
    bridged(
        state,
        crate::approvals::ops::TASK_BODY,
        payload,
        Some(&task_id),
    )
    .await
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

/// One outcome, one wording. A null result becomes an explicit success.
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
    use super::{bind_worktree, mark_as_agent};
    use crate::core::db::models::Repo;
    use crate::core::db::{store, test_pool};

    #[tokio::test]
    async fn a_bridged_write_reaches_only_its_own_worktree() {
        let pool = test_pool().await;
        let repo = Repo {
            id: "g/a".into(),
            host: "gitlab.example.com".into(),
            group_path: "g".into(),
            project: "a".into(),
            local_path: "/pool/g/a".into(),
        };
        store::repos::upsert(&pool, &repo).await.unwrap();
        store::sessions::create_explorer(&pool, "s-1", "One")
            .await
            .unwrap();
        store::sessions::create_explorer(&pool, "s-2", "Two")
            .await
            .unwrap();
        store::repos::attach(&pool, "s-1", "g/a").await.unwrap();
        let wt = store::worktrees::upsert(&pool, "s-1", "g/a", "fix/x", "/wt/s-1/a/fix/x")
            .await
            .unwrap();

        // A caller-supplied path, branch and repo are replaced by the row's own.
        let mut forged = serde_json::json!({
            "worktree_id": wt.id,
            "worktree_path": "/etc",
            "branch": "main",
            "repo": "somewhere-else",
        });
        assert!(bind_worktree(&mut forged, &pool, Some("s-1"))
            .await
            .is_none());
        assert_eq!(forged["worktree_path"], "/wt/s-1/a/fix/x");
        assert_eq!(forged["branch"], "fix/x");
        assert_eq!(forged["repo"], "a");

        // No id: the path is not taken from the caller instead.
        let mut no_id = serde_json::json!({ "worktree_path": "/etc" });
        assert!(bind_worktree(&mut no_id, &pool, Some("s-1"))
            .await
            .is_some());
        assert!(no_id["worktree_path"].is_null());

        // Another session's worktree is refused.
        let mut other = serde_json::json!({ "worktree_id": wt.id });
        assert!(bind_worktree(&mut other, &pool, Some("s-2"))
            .await
            .is_some());

        let mut unknown = serde_json::json!({ "worktree_id": "nope" });
        assert!(bind_worktree(&mut unknown, &pool, Some("s-1"))
            .await
            .is_some());
    }

    #[test]
    fn stamps_after_the_decoration() {
        assert_eq!(
            mark_as_agent("issue (non-blocking): `parse_ref` panics."),
            "issue (non-blocking)[claude]: `parse_ref` panics."
        );
    }

    #[test]
    fn stamps_a_bare_label() {
        assert_eq!(
            mark_as_agent("suggestion: extract this."),
            "suggestion[claude]: extract this."
        );
    }

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

    #[test]
    fn unparseable_content_still_gets_marked() {
        assert_eq!(
            mark_as_agent("this just panics"),
            "[claude] this just panics"
        );
    }
}
