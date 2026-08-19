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
            crate::core::events::CONFIRMATION_REQUESTED,
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
                    match super::ops::execute(&confirmation.op_type, payload, pool, &self.inner.handle).await
                    {
                        Ok(r) => result = r,
                        Err(e) => op_error = Some(e.to_string()),
                    }
                }
                Err(e) => op_error = Some(format!("corrupt confirmation payload: {e}")),
            }
        }

        let _ = self.inner.handle.emit(
            crate::core::events::CONFIRMATION_RESOLVED,
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
            crate::core::events::CONFIRMATION_REQUESTED,
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

#[tauri::command]
pub async fn resolve_confirmation(
    id: String,
    approved: bool,
    payload_overrides: Option<serde_json::Value>,
    pool: tauri::State<'_, SqlitePool>,
    bridge: tauri::State<'_, Bridge>,
) -> Result<(), String> {
    bridge
        .resolve(&pool, &id, approved, payload_overrides)
        .await
        .map_err(|e| e.to_string())
}

