use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter};

use crate::core::db::store;

/// Outcome of a resolved confirmation, delivered to the waiting MCP handler.
pub enum ResolveOutcome {
    Approved(serde_json::Value),
    Rejected,
    Failed(String),
}

#[derive(Clone)]
pub struct Bridge {
    inner: Arc<BridgeInner>,
}

struct BridgeInner {
    handle: AppHandle,
    // oneshot senders for MCP write tools — unblocked when confirmation resolves.
    senders: Mutex<HashMap<String, tokio::sync::oneshot::Sender<ResolveOutcome>>>,
}

impl Bridge {
    pub fn new(handle: AppHandle) -> Self {
        Self {
            inner: Arc::new(BridgeInner {
                handle,
                senders: Mutex::new(HashMap::new()),
            }),
        }
    }

    /// Insert a pending confirmation row and emit `confirmation_requested`. Returns the ID.
    pub async fn post(
        &self,
        pool: &SqlitePool,
        op_type: &str,
        payload: serde_json::Value,
        origin: &str,
        task_id: Option<&str>,
    ) -> anyhow::Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        self.post_with_id(&id, pool, op_type, payload, origin, task_id)
            .await?;
        Ok(id)
    }

    /// Like `post`, but with a caller-supplied ID so a waiting sender can be
    /// registered *before* the confirmation becomes visible (no resolve race).
    pub async fn post_with_id(
        &self,
        id: &str,
        pool: &SqlitePool,
        op_type: &str,
        payload: serde_json::Value,
        origin: &str,
        task_id: Option<&str>,
    ) -> anyhow::Result<()> {
        store::confirmations::insert(pool, id, task_id, op_type, &payload.to_string(), origin)
            .await?;

        self.inner.handle.emit(
            crate::events::CONFIRMATION_REQUESTED,
            serde_json::json!({
                "id": id,
                "session_id": task_id,
                "op_type": op_type,
                "payload": payload,
                "origin": origin,
            }),
        )?;

        Ok(())
    }

    /// True when an identical request is already awaiting the user's decision.
    /// Approvals can be deferred indefinitely, so a caller that asks twice (an
    /// agent retrying) must be told to wait instead of stacking a second row for
    /// the same action — the user would then have to decide it twice.
    pub async fn has_identical_pending(
        &self,
        pool: &SqlitePool,
        op_type: &str,
        task_id: Option<&str>,
        payload: &serde_json::Value,
    ) -> bool {
        store::confirmations::identical_pending(pool, op_type, task_id, &payload.to_string())
            .await
            .unwrap_or(false)
    }

    /// Register a oneshot sender so MCP write handlers can block until the user
    /// decides; it receives the op outcome (approved result / rejected / failed).
    pub fn register_sender(&self, id: &str, tx: tokio::sync::oneshot::Sender<ResolveOutcome>) {
        if let Ok(mut map) = self.inner.senders.lock() {
            map.insert(id.to_string(), tx);
        }
    }

    /// Drop a registered sender (e.g. when posting the confirmation failed).
    pub fn remove_sender(&self, id: &str) {
        if let Ok(mut map) = self.inner.senders.lock() {
            map.remove(id);
        }
    }

    /// Approve or reject a pending confirmation. On approval the underlying op runs first.
    /// Emits `confirmation_resolved` and unblocks any waiting MCP handler.
    pub async fn resolve(
        &self,
        pool: &SqlitePool,
        id: &str,
        approved: bool,
        overrides: Option<serde_json::Value>,
    ) -> anyhow::Result<()> {
        // Atomically claim the row so two concurrent resolves (e.g. double
        // Enter in the modal) can never execute the op twice.
        let Some(confirmation) = store::confirmations::claim(pool, id).await? else {
            return Err(anyhow::anyhow!("confirmation {id} not found (already resolved?)"));
        };

        // Run the op but do NOT early-return on failure: the pending row must be
        // deleted, the UI notified, and any waiting MCP handler unblocked either
        // way — otherwise a failed op leaves a stuck confirmation and a hung agent.
        let mut result = serde_json::Value::Null;
        let mut op_error: Option<String> = None;
        if approved {
            match serde_json::from_str::<serde_json::Value>(&confirmation.payload) {
                Ok(mut payload) => {
                    // Apply any field edits made in the confirmation modal (commit
                    // message, MR title/description) before executing the op.
                    if let (Some(obj), Some(over)) = (
                        payload.as_object_mut(),
                        overrides.as_ref().and_then(|v| v.as_object()),
                    ) {
                        for (k, v) in over {
                            obj.insert(k.clone(), v.clone());
                        }
                    }
                    match execute_op(&confirmation.op_type, payload, pool, &self.inner.handle).await
                    {
                        Ok(r) => result = r,
                        Err(e) => op_error = Some(e.to_string()),
                    }
                }
                Err(e) => op_error = Some(format!("corrupt confirmation payload: {e}")),
            }
        }

        let _ = self.inner.handle.emit(
            crate::events::CONFIRMATION_RESOLVED,
            serde_json::json!({
                "id": id,
                "session_id": confirmation.session_id,
                "approved": approved,
                "op_type": &confirmation.op_type,
                "result": result,
                "error": op_error,
            }),
        );

        let outcome = match (&op_error, approved) {
            (Some(e), _) => ResolveOutcome::Failed(e.clone()),
            (None, true) => ResolveOutcome::Approved(result),
            (None, false) => ResolveOutcome::Rejected,
        };
        if let Ok(mut map) = self.inner.senders.lock() {
            if let Some(tx) = map.remove(id) {
                let _ = tx.send(outcome);
            }
        }

        match op_error {
            Some(e) => Err(anyhow::anyhow!("{} failed: {e}", confirmation.op_type)),
            None => Ok(()),
        }
    }

    pub fn app_handle(&self) -> &AppHandle {
        &self.inner.handle
    }
}

/// Re-emit all rows that survived an app crash so the user can still act on
/// them, oldest first.
pub async fn surface_pending(pool: &SqlitePool, handle: &AppHandle) {
    for row in store::confirmations::all(pool).await.unwrap_or_default() {
        let payload: serde_json::Value =
            serde_json::from_str(&row.payload).unwrap_or_default();
        let _ = handle.emit(
            crate::events::CONFIRMATION_REQUESTED,
            serde_json::json!({
                "id": row.id,
                "session_id": row.session_id,
                "op_type": row.op_type,
                "payload": payload,
                "origin": row.origin,
            }),
        );
    }
}

/// Inject the Notion token (and any other config-derived secrets) into an op
/// payload at execution time. Secrets are deliberately NOT stored in
/// `pending_confirmations` rows or emitted in confirmation events.
fn inject_notion_secrets(
    payload: &mut serde_json::Value,
    handle: &AppHandle,
) -> anyhow::Result<()> {
    use tauri::Manager;
    let task_state = handle.state::<crate::task_manager::State>();
    let cfg = crate::task_manager::ensure_config(handle, &task_state)?;
    payload["token"] = serde_json::json!(cfg.notion.token);
    if payload.get("status_prop_name").and_then(|v| v.as_str()).is_none() {
        payload["status_prop_name"] = serde_json::json!(cfg.notion.properties.status);
    }
    Ok(())
}

/// The success payload every write op returns. A bare `null` is what an agent
/// gets when a tool call does nothing, so ops that "just succeed" must still say
/// so explicitly — otherwise the model reads success as failure and retries.
fn op_ok(op: &str, message: impl Into<String>) -> serde_json::Value {
    serde_json::json!({ "ok": true, "op": op, "message": message.into() })
}

/// Repo name for readable messages: the last segment of the worktree path.
fn repo_of(payload: &serde_json::Value) -> String {
    payload["worktree_path"]
        .as_str()
        .unwrap_or("")
        .rsplit('/')
        .find(|s| !s.is_empty())
        .unwrap_or("the repo")
        .to_string()
}

fn branch_of(payload: &serde_json::Value) -> String {
    payload["branch"].as_str().unwrap_or("HEAD").to_string()
}

/// Dispatch an approved write op to its implementation. Returns op-specific metadata.
async fn execute_op(
    op_type: &str,
    mut payload: serde_json::Value,
    pool: &SqlitePool,
    handle: &AppHandle,
) -> anyhow::Result<serde_json::Value> {
    let repo = repo_of(&payload);
    let branch = branch_of(&payload);

    match op_type {
        crate::ops::GIT_COMMIT => {
            let subject = payload["message"]
                .as_str()
                .unwrap_or("")
                .lines()
                .next()
                .unwrap_or("")
                .to_string();
            crate::git_engine::commit_impl(payload, pool).await?;
            Ok(op_ok(op_type, format!("Committed \"{subject}\" on {branch} in {repo}")))
        }
        crate::ops::GIT_PUSH => {
            crate::git_engine::push_impl(payload).await?;
            Ok(op_ok(op_type, format!("Pushed {branch} to origin in {repo}")))
        }
        crate::ops::GIT_PULL => {
            crate::git_engine::pull_impl(payload).await?;
            Ok(op_ok(op_type, format!("Pulled origin/{branch} into {repo}")))
        }
        crate::ops::GIT_REBASE => {
            // rebase_impl already reports status/files (the UI keys on
            // `status == "conflict"`); enrich it rather than replace it.
            let mut v = crate::git_engine::rebase_impl(payload).await?;
            let conflicted = v["status"].as_str() == Some("conflict");
            let files = v["files"].as_array().map(|a| a.len()).unwrap_or(0);
            if let Some(obj) = v.as_object_mut() {
                obj.insert("ok".into(), serde_json::json!(true));
                obj.insert("op".into(), serde_json::json!(op_type));
                obj.insert(
                    "message".into(),
                    serde_json::json!(if conflicted {
                        format!("Rebase of {branch} stopped on conflicts in {files} file(s) — resolve them, then continue the rebase")
                    } else {
                        format!("Rebased {branch} onto its base branch in {repo}")
                    }),
                );
            }
            Ok(v)
        }
        crate::ops::GIT_DISCARD => {
            let file = payload["file_path"].as_str().unwrap_or("the file").to_string();
            crate::git_engine::discard_impl(payload).await?;
            Ok(op_ok(op_type, format!("Discarded local changes in {file} ({repo})")))
        }
        crate::ops::GIT_DISCARD_ALL => {
            crate::git_engine::discard_all_impl(payload).await?;
            Ok(op_ok(op_type, format!("Discarded ALL local changes in {repo}")))
        }
        crate::ops::MR_CREATE => {
            let worktree_id = payload["worktree_id"].as_str().unwrap_or("").to_string();
            crate::mr_manager::create_mr_impl(payload, pool).await?;
            // Hand back the MR the op just recorded — the agent's next step is
            // almost always "give me the link".
            let latest = store::mrs::latest_for_worktree(pool, &worktree_id)
                .await
                .ok()
                .flatten();
            Ok(match latest {
                Some(mr) => serde_json::json!({
                    "ok": true,
                    "op": op_type,
                    "message": format!("Merge request !{} created from {branch}", mr.remote_id),
                    "iid": mr.remote_id,
                    "url": mr.url,
                }),
                None => op_ok(op_type, format!("Merge request created from {branch}")),
            })
        }
        crate::ops::MR_UPDATE => {
            crate::mr_manager::update_mr_impl(payload, pool).await?;
            Ok(op_ok(op_type, "Merge request title/description updated"))
        }
        crate::ops::MR_CLOSE => {
            crate::mr_manager::close_mr_impl(payload, pool).await?;
            Ok(op_ok(op_type, "Merge request closed"))
        }
        crate::ops::NOTION_STATUS => {
            let status = payload["status"].as_str().unwrap_or("").to_string();
            inject_notion_secrets(&mut payload, handle)?;
            crate::task_manager::update_notion_status_impl(payload).await?;
            Ok(op_ok(op_type, format!("Notion status set to \"{status}\"")))
        }
        crate::ops::NOTION_PROPERTY => {
            inject_notion_secrets(&mut payload, handle)?;
            let out = crate::task_manager::update_property_impl(payload, pool).await?;
            let prop = out["property"].as_str().unwrap_or("property").to_string();
            let value = out["value"].as_str().unwrap_or("").to_string();
            Ok(op_ok(op_type, if value.is_empty() {
                format!("Cleared {prop}")
            } else {
                format!("{prop} set to \"{value}\"")
            }))
        }
        crate::ops::NOTION_HOURS => {
            inject_notion_secrets(&mut payload, handle)?;
            let out = crate::task_manager::log_hours_impl(payload, pool).await?;
            let (before, after) = (out["before"].as_f64().unwrap_or(0.0), out["after"].as_f64().unwrap_or(0.0));
            Ok(op_ok(op_type, format!("Hours spent {before} → {after}")))
        }
        crate::ops::NOTION_BODY => {
            inject_notion_secrets(&mut payload, handle)?;
            // Carries its own message (block counts).
            crate::task_manager::update_body_impl(payload, pool).await
        }
        crate::ops::TASK_CREATE => {
            inject_notion_secrets(&mut payload, handle)?;
            crate::task_manager::create_task_impl(payload, pool).await
        }
        crate::ops::TASK_ADD_REPO => {
            // No Notion secrets: this is local git and DB work only. The handle is
            // for the workspace_ready refresh the add ends with.
            crate::task_manager::add_repo_impl(payload, pool, handle).await
        }
        crate::ops::TASK_CREATE_FROM_EXPLORER => {
            // Already returns the created task (short_id, page id, …).
            inject_notion_secrets(&mut payload, handle)?;
            crate::task_manager::create_task_from_explorer_impl(payload, pool).await
        }
        _ => Err(anyhow::anyhow!("unknown op_type: {op_type}")),
    }
}

