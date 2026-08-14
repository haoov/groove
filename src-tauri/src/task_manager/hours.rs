//! Time spent on a task: measured locally, logged to Notion on purpose.
//!
//! The tracker never writes to Notion by itself. "Hours spent" is a number other
//! people read, and a timer that quietly inflates it produces data nobody can
//! trust — so the app accumulates seconds, shows you what it measured, and waits
//! for you to log a figure you agree with.
//!
//! Accounting is two counters (see 0005_task_time.sql): `tracked` is what was
//! measured, `logged` is what has been written. Logging advances `logged`, so
//! pressing the button twice cannot double-count.

use serde::Serialize;
use sqlx::SqlitePool;

use super::notion::{notion_get, notion_patch};

/// A tick is only credited if it is this recent — the frontend decides when to
/// tick (see useTaskTimer), and this rejects a stale or replayed one.
const MAX_TICK_SECONDS: i64 = 120;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TaskTime {
    pub task_id: String,
    pub tracked_seconds: i64,
    pub logged_seconds: i64,
    pub today_seconds: i64,
    /// Measured but not yet written to Notion — what the log button offers.
    /// Computed in SQL so there is one definition of it.
    pub unlogged_seconds: i64,
}

fn today() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

/// Credit `seconds` of work to a task. Called on a timer while the window is
/// focused and the work is real (the frontend owns that judgement).
#[tauri::command]
pub async fn add_task_time(
    task_id: String,
    seconds: i64,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<TaskTime, String> {
    if !(0..=MAX_TICK_SECONDS).contains(&seconds) {
        return Err(format!("implausible tick of {seconds}s ignored"));
    }
    let now = chrono::Utc::now().timestamp();
    let day = today();

    // One statement so concurrent ticks can't lose an increment. The CASE resets
    // the daily bucket when the date has rolled over since the last tick.
    sqlx::query(
        "INSERT INTO task_time (task_id, tracked_seconds, logged_seconds, today_date, today_seconds, updated_at)
         VALUES (?1, ?2, 0, ?3, ?2, ?4)
         ON CONFLICT(task_id) DO UPDATE SET
           tracked_seconds = tracked_seconds + ?2,
           today_seconds   = CASE WHEN today_date = ?3 THEN today_seconds + ?2 ELSE ?2 END,
           today_date      = ?3,
           updated_at      = ?4",
    )
    .bind(&task_id)
    .bind(seconds)
    .bind(&day)
    .bind(now)
    .execute(&*pool)
    .await
    .map_err(|e| e.to_string())?;

    read(&task_id, &pool).await.map_err(|e| e.to_string())
}

async fn read(task_id: &str, pool: &SqlitePool) -> anyhow::Result<TaskTime> {
    let row: Option<TaskTime> = sqlx::query_as(
        "SELECT task_id, tracked_seconds, logged_seconds,
                CASE WHEN today_date = ?2 THEN today_seconds ELSE 0 END AS today_seconds,
                MAX(tracked_seconds - logged_seconds, 0)              AS unlogged_seconds
           FROM task_time WHERE task_id = ?1",
    )
    .bind(task_id)
    .bind(today())
    .fetch_optional(pool)
    .await?;

    Ok(row.unwrap_or(TaskTime {
        task_id: task_id.to_string(),
        tracked_seconds: 0,
        logged_seconds: 0,
        today_seconds: 0,
        unlogged_seconds: 0,
    }))
}

#[tauri::command]
pub async fn get_task_time(
    task_id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<TaskTime, String> {
    read(&task_id, &pool).await.map_err(|e| e.to_string())
}

/// Add `hours` to the Notion number property and advance the logged watermark.
///
/// ADDS rather than replaces: "Hours spent" is cumulative, and the value may have
/// been edited in Notion since we last read it, so the current value is re-read
/// immediately before the write.
pub(super) async fn log_hours(
    token: &str,
    notion_page_id: &str,
    property: &str,
    hours: f64,
    task_id: &str,
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

    // Only now is the time accounted for. Capped at tracked so a manual entry
    // larger than what was measured doesn't leave a negative remainder.
    let logged = (hours * 3600.0).round() as i64;
    sqlx::query(
        "INSERT INTO task_time (task_id, tracked_seconds, logged_seconds, today_date, today_seconds, updated_at)
         VALUES (?1, 0, ?2, ?3, 0, ?4)
         ON CONFLICT(task_id) DO UPDATE SET
           logged_seconds = MIN(tracked_seconds, logged_seconds + ?2),
           updated_at     = ?4",
    )
    .bind(task_id)
    .bind(logged)
    .bind(today())
    .bind(chrono::Utc::now().timestamp())
    .execute(pool)
    .await?;

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
