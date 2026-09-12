//! Time spent on a session: two counters, seconds measured and seconds logged.
//! Logging to the task source happens only on an explicit request.
use sqlx::SqlitePool;

use crate::core::db::models::{ActivityDay, TimeSummary};
use crate::core::db::store;
use crate::core::error::{AppError, AppResult, ErrorKind};

/// Largest tick accepted; a larger one is rejected as stale or replayed.
const MAX_TICK_SECONDS: i64 = 120;

fn today() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

/// Credit `seconds` of work to a session.
#[tauri::command]
pub async fn add_task_time(
    task_id: String,
    seconds: i64,
    pool: tauri::State<'_, SqlitePool>,
) -> AppResult<TimeSummary> {
    if !(0..=MAX_TICK_SECONDS).contains(&seconds) {
        return Err(AppError::invalid(format!(
            "implausible tick of {seconds}s ignored"
        )));
    }
    let day = today();
    store::time::add(&*pool, &task_id, &day, seconds).await?;
    Ok(store::time::summary(&*pool, &task_id, &day).await?)
}

#[tauri::command]
pub async fn get_task_time(
    task_id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> AppResult<TimeSummary> {
    Ok(store::time::summary(&*pool, &task_id, &today()).await?)
}

/// Tracked seconds per day across all sessions — feeds the Home activity heatmap.
#[tauri::command]
pub async fn get_activity_days(pool: tauri::State<'_, SqlitePool>) -> AppResult<Vec<ActivityDay>> {
    Ok(store::time::activity(&*pool).await?)
}

/// Record `hours` in the local ledger and in the provider's hours field when it has one.
async fn log_hours(
    hours: f64,
    session_id: &str,
    pool: &SqlitePool,
) -> anyhow::Result<serde_json::Value> {
    let (provider, key) = crate::provider::resolve(pool, session_id).await?;

    // The provider write goes first; only a landed write counts as logged.
    let written = provider.add_hours(&key, hours).await?;

    store::time::log(pool, session_id, (hours * 3600.0).round() as i64).await?;

    Ok(serde_json::json!({
        "before": written.as_ref().map(|w| w.before),
        "after": written.as_ref().map(|w| w.after),
        "added": hours,
    }))
}

/// UI path for logging hours.
#[tauri::command]
pub async fn log_task_hours(
    short_id: String,
    hours: f64,
    pool: tauri::State<'_, SqlitePool>,
) -> AppResult<serde_json::Value> {
    log_hours(hours, &short_id, &pool)
        .await
        .map_err(|e| AppError::from(e).with_kind(ErrorKind::Provider))
}

/// Confirmation-bridge path for `task.hours` (agent-initiated).
#[tracing::instrument(skip_all, fields(session = payload["task_id"].as_str()))]
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
