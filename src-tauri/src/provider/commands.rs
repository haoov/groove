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
pub(crate) async fn mint_short_id(pool: &SqlitePool, task: &FetchedTask) -> String {
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
    let short_id = mint_short_id(pool, &filed).await;
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

/// File a task from the UI composer. Direct, not gated: you typed it and pressed
/// the button. It is NOT opened or provisioned — it lands in the queue.
#[tauri::command]
pub async fn create_task(
    title: String,
    body_markdown: String,
    provider: Option<String>,
    repo: Option<String>,
    properties: Option<std::collections::HashMap<String, serde_json::Value>>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<serde_json::Value, String> {
    async {
        if title.trim().is_empty() {
            anyhow::bail!("a task needs a title");
        }
        let payload = serde_json::json!({
            "title": title.trim(),
            "body_markdown": body_markdown,
            "provider": provider,
            "repo": repo,
        });
        let created = create_task_impl(payload, &pool).await?;

        // Creation takes a fixed set of fields; anything else the composer set is a
        // follow-up write. The task is already filed by then, so one that will not
        // take is reported rather than failing the whole call.
        let mut created = created;
        if let Some(props) = properties.filter(|p| !p.is_empty()) {
            let short_id = created["short_id"].as_str().unwrap_or_default().to_string();
            let (provider, key) = resolve(&pool, &short_id).await?;
            let mut warnings = vec![];
            for (name, value) in props {
                if let Err(e) = provider.set_property(&key, &name, &value).await {
                    warnings.push(format!("{name}: {e}"));
                }
            }
            if !warnings.is_empty() {
                created["warnings"] = serde_json::json!(warnings);
            }
        }
        Ok(created)
    }
    .await
    .map_err(|e: anyhow::Error| e.to_string())
}

/// The configured task template as markdown, for the composer to start from.
/// Empty when the source has none — a blank body is a fine default.
#[tauri::command]
pub async fn get_task_template_markdown(provider: Option<String>) -> Result<String, String> {
    async {
        let id = match provider.as_deref() {
            Some("github") => ProviderId::Github,
            Some("notion") | None => ProviderId::Notion,
            Some(other) => anyhow::bail!("unknown task source {other}"),
        };
        Ok(get(id)?.template_markdown().await?.unwrap_or_default())
    }
    .await
    .map_err(|e: anyhow::Error| e.to_string())
}
