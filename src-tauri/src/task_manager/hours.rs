//! Time spent on a session: measured locally, logged to Notion on purpose.
//!
//! The tracker never writes to Notion by itself. "Hours spent" is a number other
//! people read, and a timer that quietly inflates it produces data nobody can
//! trust — so the app accumulates seconds per day, shows what it measured, and
//! waits for a figure you agree with. `time_entries` records what was measured,
//! `time_logs` what was written; the difference is what is left to log.

use sqlx::SqlitePool;

use crate::core::db::models::TimeSummary;
use crate::core::db::store;
use super::notion::{notion_get, notion_patch};

/// A tick is only credited if it is this recent — the frontend decides when to
/// tick (see useTaskTimer), and this rejects a stale or replayed one.
const MAX_TICK_SECONDS: i64 = 120;

fn today() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

/// Credit `seconds` of work to a session. Called on a timer while the window is
/// focused and the work is real (the frontend owns that judgement).
#[tauri::command]
pub async fn add_task_time(
    task_id: String,
    seconds: i64,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<TimeSummary, String> {
    if !(0..=MAX_TICK_SECONDS).contains(&seconds) {
        return Err(format!("implausible tick of {seconds}s ignored"));
    }
    let day = today();
    store::time::add(&*pool, &task_id, &day, seconds)
        .await
        .map_err(|e| e.to_string())?;
    store::time::summary(&*pool, &task_id, &day)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_task_time(
    task_id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<TimeSummary, String> {
    store::time::summary(&*pool, &task_id, &today())
        .await
        .map_err(|e| e.to_string())
}

/// Add `hours` to the Notion number property and record the write.
///
/// ADDS rather than replaces: "Hours spent" is cumulative, and the value may
/// have been edited in Notion since we last read it, so the current value is
/// re-read immediately before the write.
pub(super) async fn log_hours(
    token: &str,
    notion_page_id: &str,
    property: &str,
    hours: f64,
    session_id: &str,
    pool: &SqlitePool,
) -> anyhow::Result<serde_json::Value> {
    if !(hours.is_finite() && hours > 0.0 && hours < 1000.0) {
        return Err(anyhow::anyhow!("{hours} is not a plausible number of hours"));
    }

    let page = notion_get(token, &format!("v1/pages/{notion_page_id}")).await?;
    let before = page["properties"][property]["number"].as_f64().unwrap_or(0.0);
    let after = ((before + hours) * 100.0).round() / 100.0;

    notion_patch(
        token,
        &format!("v1/pages/{notion_page_id}"),
        &serde_json::json!({ "properties": { property: { "number": after } } }),
    )
    .await?;

    // Only now is the time accounted for. The ledger records what was actually
    // sent; the unlogged remainder is clamped at read time, so a manual entry
    // larger than what was measured can never read negative.
    store::time::log(pool, session_id, (hours * 3600.0).round() as i64).await?;

    Ok(serde_json::json!({ "before": before, "after": after, "added": hours }))
}

/// UI path: you typed the number, so it happens.
#[tauri::command]
pub async fn log_task_hours(
    task_id: String,
    notion_page_id: String,
    hours: f64,
    app: tauri::AppHandle,
    task_state: tauri::State<'_, super::State>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<serde_json::Value, String> {
    let cfg = super::ensure_config(&app, &task_state).map_err(|e| e.to_string())?;
    let property = hours_property(&cfg.notion.token, &cfg.notion.database_id)
        .await
        .map_err(|e| e.to_string())?;
    log_hours(&cfg.notion.token, &notion_page_id, &property, hours, &task_id, &pool)
        .await
        .map_err(|e| e.to_string())
}

/// Confirmation-bridge path for `notion.hours` (agent-initiated).
pub async fn log_hours_impl(
    payload: serde_json::Value,
    pool: &SqlitePool,
) -> anyhow::Result<serde_json::Value> {
    let cfg = super::global_config().ok_or_else(|| anyhow::anyhow!("not configured"))?;
    let token = payload["token"].as_str().unwrap_or_default();
    let property = hours_property(token, &cfg.notion.database_id).await?;
    log_hours(
        token,
        payload["notion_page_id"].as_str().unwrap_or_default(),
        &property,
        payload["hours"].as_f64().unwrap_or(0.0),
        payload["task_id"].as_str().unwrap_or_default(),
        pool,
    )
    .await
}

/// The number property hours go into. Found in the schema rather than configured:
/// there is exactly one plausible name, and guessing wrong would silently write to
/// the wrong column.
async fn hours_property(token: &str, database_id: &str) -> anyhow::Result<String> {
    const CANDIDATES: [&str; 3] = ["Hours spent", "Hours", "Time spent"];
    let schema = super::schema::load(token, database_id).await?;
    for name in CANDIDATES {
        if let Some(p) = schema.property(name) {
            if p.kind == "number" {
                return Ok(p.name.clone());
            }
        }
    }
    Err(anyhow::anyhow!(
        "no number property named any of {CANDIDATES:?} in this database — nothing to log hours into"
    ))
}
