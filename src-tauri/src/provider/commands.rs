//! The task commands, over whichever providers are configured.

use sqlx::SqlitePool;

use super::types::{FetchedTask, TaskKey};
use super::{enabled, mirror_row, resolve};
use super::types::{PropertyValue, PropertyOption, TaskSchema};
use crate::core::db::models::TaskView;
use crate::core::db::store;

/// The identifier a task is known by everywhere: the session id, its worktree
/// directory, and part of its branch. Minted once and never recomputed — it names
/// things that already exist on disk.
///
/// Providers with a natural id keep it. GitHub gets `gh-<repo>-<number>`, which is
/// unique within one owner; the owner is added only when that collides, so the
/// task that got there first is never re-keyed.
async fn mint_short_id(pool: &SqlitePool, task: &FetchedTask) -> String {
    let external_id = task.key.external_id();
    if let Ok(Some(existing)) = store::provider_tasks::get_by_external_id(pool, &external_id).await
    {
        return existing.short_id;
    }
    if let Some(natural) = &task.natural_short_id {
        return natural.clone();
    }

    let TaskKey::Github { owner, repo, number, .. } = &task.key else {
        return external_id;
    };
    let short = format!("gh-{}-{number}", segment(repo));
    match store::provider_tasks::get_by_short_id(pool, &short).await {
        Ok(Some(_)) => format!("gh-{}-{}-{number}", segment(owner), segment(repo)),
        _ => short,
    }
}

/// Lowercase, alphanumerics joined by single dashes, capped without cutting a word.
fn segment(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
        if out.len() >= 16 {
            break;
        }
    }
    out.trim_matches('-').to_string()
}

#[tauri::command]
pub async fn list_tasks(pool: tauri::State<'_, SqlitePool>) -> Result<Vec<TaskView>, String> {
    let mut out = Vec::new();

    // One provider being down must not blank the others' half of Home, so a
    // failure is logged and skipped. Same policy as the review queue.
    for provider in enabled() {
        let fetched = match provider.list_tasks().await {
            Ok(tasks) => tasks,
            Err(e) => {
                tracing::warn!("{} queue unavailable: {e}", provider.id().as_str());
                continue;
            }
        };

        let mut rows = Vec::with_capacity(fetched.len());
        for task in &fetched {
            let short_id = mint_short_id(&pool, task).await;
            rows.push(mirror_row(&short_id, task));
        }

        let keep: Vec<String> = rows.iter().map(|r| r.external_id.clone()).collect();
        let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
        store::provider_tasks::prune_missing(&mut *tx, provider.id().as_str(), &keep)
            .await
            .map_err(|e| e.to_string())?;
        for row in &rows {
            store::provider_tasks::upsert(&mut *tx, row).await.map_err(|e| e.to_string())?;
        }
        tx.commit().await.map_err(|e| e.to_string())?;

        out.extend(rows.into_iter().map(Into::into));
    }

    Ok(out)
}

#[tauri::command]
pub async fn sync_task(
    short_id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<TaskView, String> {
    async {
        let (provider, key) = resolve(&pool, &short_id).await?;
        let task = provider.fetch_task(&key).await?;
        let row = mirror_row(&short_id, &task);
        store::provider_tasks::upsert(&*pool, &row).await?;
        Ok(row.into())
    }
    .await
    .map_err(|e: anyhow::Error| e.to_string())
}

#[tauri::command]
pub async fn get_task_schema(
    short_id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<TaskSchema, String> {
    async {
        let (provider, key) = resolve(&pool, &short_id).await?;
        provider.schema(&key).await
    }
    .await
    .map_err(|e: anyhow::Error| e.to_string())
}

#[tauri::command]
pub async fn get_task_properties(
    short_id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Vec<PropertyValue>, String> {
    async {
        let (provider, key) = resolve(&pool, &short_id).await?;
        provider.properties(&key).await
    }
    .await
    .map_err(|e: anyhow::Error| e.to_string())
}

#[tauri::command]
pub async fn list_relation_options(
    short_id: String,
    property: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Vec<PropertyOption>, String> {
    async {
        let (provider, key) = resolve(&pool, &short_id).await?;
        provider.reference_options(&key, &property).await
    }
    .await
    .map_err(|e: anyhow::Error| e.to_string())
}
