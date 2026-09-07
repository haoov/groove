//! Provider-agnostic task reads and writes. Everything goes through `resolve`;
//! nothing in this file may name a provider.

use sqlx::SqlitePool;

use crate::core::db::store;
use crate::core::error::{AppError, AppResult, ErrorKind};

#[tauri::command]
pub async fn update_task_property(
    short_id: String,
    property: String,
    value: serde_json::Value,
    pool: tauri::State<'_, SqlitePool>,
) -> AppResult<String> {
    write_property(&short_id, &property, &value, &pool)
        .await
        .map_err(|e| AppError::from(e).with_kind(ErrorKind::Provider))
}

/// Confirmation-bridge path for `task.property` (agent-initiated).
pub async fn update_property_impl(
    payload: serde_json::Value,
    pool: &SqlitePool,
) -> anyhow::Result<serde_json::Value> {
    let field = |k: &str| payload[k].as_str().unwrap_or_default().to_string();
    let display = write_property(
        &field("task_id"),
        &field("property"),
        &payload["value"],
        pool,
    )
    .await?;
    Ok(serde_json::json!({ "property": field("property"), "value": display }))
}

/// Set one property through the task's provider, then re-mirror the task.
pub(crate) async fn write_property(
    short_id: &str,
    property: &str,
    value: &serde_json::Value,
    pool: &SqlitePool,
) -> anyhow::Result<String> {
    let (provider, key) = crate::provider::resolve(pool, short_id).await?;
    let written = provider.set_property(&key, property, value).await?;
    if let Ok(task) = provider.fetch_task(&key).await {
        let _ = store::provider_tasks::upsert(pool, &crate::provider::mirror_row(short_id, &task))
            .await;
    }
    Ok(written.display)
}

/// The task body as markdown.
#[tauri::command]
pub async fn get_task_body_markdown(
    short_id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> AppResult<String> {
    async {
        let (provider, key) = crate::provider::resolve(&pool, &short_id).await?;
        provider.body_markdown(&key).await
    }
    .await
    .map_err(|e| AppError::from(e).with_kind(ErrorKind::Provider))
}

/// Queue a body replacement from the UI through the confirmation bridge.
#[tauri::command]
pub async fn request_task_body_update(
    short_id: String,
    markdown: String,
    force: bool,
    bridge: tauri::State<'_, crate::approvals::Bridge>,
    pool: tauri::State<'_, SqlitePool>,
) -> AppResult<String> {
    Ok(bridge
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
        .await?)
}

/// Confirmation-bridge path for `task.body`.
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
