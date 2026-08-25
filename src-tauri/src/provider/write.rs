//! Task reads and writes that work for ANY provider: everything here goes
//! through `resolve`, so nothing in this file may name one. These lived under
//! `notion/` from before tasks could come from anywhere — which routed every
//! provider's writes through a module named notion.

use sqlx::SqlitePool;

use crate::core::db::store;

#[tauri::command]
pub async fn update_task_property(
    short_id: String,
    property: String,
    value: serde_json::Value,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<String, String> {
    write_property(&short_id, &property, &value, &pool)
        .await
        .map_err(|e| e.to_string())
}

/// Confirmation-bridge path for `task.property` (agent-initiated).
pub async fn update_property_impl(
    payload: serde_json::Value,
    pool: &SqlitePool,
) -> anyhow::Result<serde_json::Value> {
    let field = |k: &str| payload[k].as_str().unwrap_or_default().to_string();
    let display =
        write_property(&field("task_id"), &field("property"), &payload["value"], pool).await?;
    Ok(serde_json::json!({ "property": field("property"), "value": display }))
}

/// Set one property through the task's own provider, then re-mirror it: Home and
/// the queue read status and priority from the local row.
pub(crate) async fn write_property(
    short_id: &str,
    property: &str,
    value: &serde_json::Value,
    pool: &SqlitePool,
) -> anyhow::Result<String> {
    let (provider, key) = crate::provider::resolve(pool, short_id).await?;
    let written = provider.set_property(&key, property, value).await?;
    if let Ok(task) = provider.fetch_task(&key).await {
        let _ =
            store::provider_tasks::upsert(pool, &crate::provider::mirror_row(short_id, &task))
                .await;
    }
    Ok(written.display)
}

/// The ticket body as markdown — what the overview renders. Going through
/// markdown (rather than a bespoke block renderer) means one renderer for task
/// and MR descriptions, and markdown typed literally into the source displays
/// as intended.
#[tauri::command]
pub async fn get_task_body_markdown(
    short_id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<String, String> {
    async {
        let (provider, key) = crate::provider::resolve(&pool, &short_id).await?;
        provider.body_markdown(&key).await
    }
    .await
    .map_err(|e: anyhow::Error| e.to_string())
}

/// Queue a body replacement from the UI.
///
/// Goes through the confirmation bridge even though a human asked for it: this is
/// the one write that can delete blocks the app never displayed, so it gets a
/// preview and an explicit yes like an agent's write would.
#[tauri::command]
pub async fn request_task_body_update(
    short_id: String,
    markdown: String,
    force: bool,
    bridge: tauri::State<'_, crate::approvals::Bridge>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<String, String> {
    bridge
        .post(
            &pool,
            crate::approvals::ops::TASK_BODY,
            serde_json::json!({
                "task_id": short_id,
                "markdown": markdown,
                "force": force,
            }),
            "ui",
            Some(&short_id),
        )
        .await
        .map_err(|e| e.to_string())
}

/// Confirmation-bridge path for `task.body`.
///
/// Gated even when it comes from the UI: it is the one write that can destroy
/// content the app never showed you.
pub async fn update_body_impl(
    payload: serde_json::Value,
    pool: &SqlitePool,
) -> anyhow::Result<serde_json::Value> {
    let task_id = payload["task_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing task_id"))?;
    let markdown = payload["markdown"].as_str().unwrap_or_default();
    let force = payload["force"].as_bool().unwrap_or(false);

    let (provider, key) = crate::provider::resolve(pool, task_id).await?;
    let written = provider.replace_body(&key, markdown, force).await?;
    Ok(serde_json::json!({
        "ok": true,
        "blocks_written": written.blocks_written,
        "message": format!("Task body updated ({} blocks)", written.blocks_written),
    }))
}
