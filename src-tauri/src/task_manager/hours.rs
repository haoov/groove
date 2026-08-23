//! Time spent on a session: measured locally, logged to Notion on purpose.
//!
//! The tracker never writes to Notion by itself. "Hours spent" is a number other
//! people read, and a timer that quietly inflates it produces data nobody can
//! trust — so the app accumulates seconds per day, shows what it measured, and
//! waits for a figure you agree with. `time_entries` records what was measured,
//! `time_logs` what was written; the difference is what is left to log.
//! The Notion write itself lives in `notion::hours`.

use sqlx::SqlitePool;

use crate::core::db::models::{ActivityDay, TimeSummary};
use crate::core::db::store;

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

/// Tracked seconds per day across all sessions — feeds the Home activity heatmap.
#[tauri::command]
pub async fn get_activity_days(
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Vec<ActivityDay>, String> {
    store::time::activity(&*pool).await.map_err(|e| e.to_string())
}

/// Record `hours` in the local ledger, and in the provider's own field when it has
/// one. The external write goes first where it exists: only a write that landed
/// counts as logged, which is what keeps pressing the button twice from
/// double-counting. The unlogged remainder is clamped at read time.
async fn log_hours(
    hours: f64,
    session_id: &str,
    pool: &SqlitePool,
) -> anyhow::Result<serde_json::Value> {
    let (provider, key) = crate::provider::resolve(pool, session_id).await?;

    let written = match provider.capabilities().external_hours {
        true => Some(provider.add_hours(&key, hours).await?),
        false => None,
    };

    store::time::log(pool, session_id, (hours * 3600.0).round() as i64).await?;

    Ok(serde_json::json!({
        "before": written.as_ref().map(|w| w.before),
        "after": written.as_ref().map(|w| w.after),
        "added": hours,
    }))
}

/// UI path: you typed the number, so it happens.
#[tauri::command]
pub async fn log_task_hours(
    short_id: String,
    hours: f64,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<serde_json::Value, String> {
    log_hours(hours, &short_id, &pool).await.map_err(|e| e.to_string())
}

/// Confirmation-bridge path for `task.hours` (agent-initiated).
pub async fn log_hours_impl(
    payload: serde_json::Value,
    pool: &SqlitePool,
) -> anyhow::Result<serde_json::Value> {
    log_hours(
        payload["hours"].as_f64().unwrap_or(0.0),
        payload["task_id"].as_str().unwrap_or_default(),
        pool,
    )
    .await
}
