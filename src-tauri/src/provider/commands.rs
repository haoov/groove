//! The task commands, over whichever providers are configured.

use sqlx::SqlitePool;

use super::types::{FetchedTask, TaskKey};
use super::types::{ProviderId, TaskDraft};
use super::{enabled, get, mirror_row, resolve};
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
pub(crate) async fn mint_short_id(
    pool: &SqlitePool,
    task: &FetchedTask,
    minted: &mut std::collections::HashSet<String>,
) -> anyhow::Result<String> {
    // A source with an id of its own keeps it, and that answer needs no query.
    if let Some(natural) = &task.natural_short_id {
        return Ok(natural.clone());
    }

    // An id already minted for this task is the only correct answer: it names a
    // branch and a directory that may already exist.
    let external_id = task.key.external_id();
    if let Some(existing) = store::provider_tasks::get_by_external_id(pool, &external_id).await? {
        return Ok(existing.short_id);
    }

    let TaskKey::Github { owner, repo, number, .. } = &task.key else {
        return Ok(external_id);
    };

    // `minted` covers the rest of this sync: two issues can collide inside one
    // batch, before either has been written for the store to see.
    let short = format!("gh-{}-{number}", segment(repo));
    let taken = minted.contains(&short)
        || store::provider_tasks::get_by_short_id(pool, &short).await?.is_some();
    let chosen = match taken {
        true => format!("gh-{}-{}-{number}", segment(owner), segment(repo)),
        false => short,
    };
    minted.insert(chosen.clone());
    Ok(chosen)
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
    let mut failed: Vec<String> = vec![];

    // One provider being down must not blank the others' half of Home, so a
    // failure is logged and skipped. Same policy as the review queue.
    for provider in enabled() {
        let fetched = match provider.list_tasks().await {
            Ok(tasks) => tasks,
            Err(e) => {
                // Keep the others, and keep this one's last known tasks: showing
                // fewer rows with no explanation reads as "you have none".
                tracing::warn!("{} queue unavailable: {e}", provider.id().as_str());
                failed.push(format!("{}: {e}", provider.id().as_str()));
                if let Ok(stale) = store::provider_tasks::for_provider(&*pool, provider.id().as_str()).await {
                    out.extend(stale.into_iter().map(Into::into));
                }
                continue;
            }
        };

        let mut rows = Vec::with_capacity(fetched.len());
        let mut minted = std::collections::HashSet::new();
        for task in &fetched {
            let short_id = mint_short_id(&pool, task, &mut minted).await.map_err(|e| e.to_string())?;
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

    // Every source down is a failure, not an empty queue.
    if out.is_empty() && !failed.is_empty() {
        return Err(failed.join("; "));
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

/// Which source a draft is filed at. Missing means the only one configured; with
/// both set up the caller has to say.
pub(crate) fn draft_provider(payload: &serde_json::Value) -> anyhow::Result<ProviderId> {
    match payload["provider"].as_str() {
        Some("github") => Ok(ProviderId::Github),
        Some("notion") => Ok(ProviderId::Notion),
        Some(other) => anyhow::bail!("unknown task source {other}"),
        None => match enabled().as_slice() {
            [only] => Ok(only.id()),
            [] => anyhow::bail!("no task source is set up"),
            _ => anyhow::bail!("more than one task source is set up — say which to file in"),
        },
    }
}

/// Confirmation-bridge path for `task.create`: file the task and nothing else.
///
/// Deliberately does NOT open a session or provision worktrees — filing a task you
/// intend to pick up later should not clone repositories.
pub async fn create_task_impl(
    payload: serde_json::Value,
    pool: &SqlitePool,
) -> anyhow::Result<serde_json::Value> {
    let provider = get(draft_provider(&payload)?)?;
    let draft = TaskDraft {
        title: payload["title"].as_str().unwrap_or("Untitled task"),
        body_markdown: payload["body_markdown"].as_str().unwrap_or(""),
        repo: payload["repo"].as_str(),
    };

    let filed = provider.create_task(&draft).await?;
    let short_id = mint_short_id(pool, &filed, &mut Default::default()).await?;
    let row = mirror_row(&short_id, &filed);
    store::provider_tasks::upsert(pool, &row).await?;

    Ok(serde_json::json!({
        "short_id": short_id,
        "external_id": filed.key.external_id(),
        "provider": filed.key.provider().as_str(),
        "external_url": filed.url,
        "title": filed.title,
        "status": filed.status,
        "priority": null,
        "last_synced_at": row.synced_at,
        "message": format!("Filed {short_id}"),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::db::test_pool;
    use crate::provider::types::TaskKey;

    fn gh(owner: &str, repo: &str, number: i64) -> FetchedTask {
        FetchedTask {
            key: TaskKey::Github {
                host: "github.com".into(),
                owner: owner.into(),
                repo: repo.into(),
                number,
            },
            title: String::new(),
            status: String::new(),
            priority: None,
            url: String::new(),
            natural_short_id: None,
            branch_tag: Some(number.to_string()),
            board: None,
        }
    }

    /// Two repos that truncate to the same segment collide. Nothing is written to
    /// the store until the batch ends, so the store cannot be what catches it.
    #[tokio::test]
    async fn a_collision_inside_one_sync_still_gets_two_ids() {
        let pool = test_pool().await;
        let mut minted = std::collections::HashSet::new();

        let first = mint_short_id(&pool, &gh("acme", "kubernetes-operator", 42), &mut minted)
            .await
            .unwrap();
        let second = mint_short_id(&pool, &gh("beta", "kubernetes-operations", 42), &mut minted)
            .await
            .unwrap();

        assert_ne!(first, second, "two tasks must never share a short_id");
        assert_eq!(first, "gh-kubernetes-opera-42", "the first keeps the short form");
        assert!(second.contains("beta"), "the collider takes the owner: {second}");
    }
}
