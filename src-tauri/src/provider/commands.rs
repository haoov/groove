//! The task commands, over whichever providers are configured.

use sqlx::SqlitePool;

use super::types::FetchedTask;
use super::types::{ProviderId, TaskDraft};
use super::{enabled, get, mirror_row, resolve};
use super::types::{PropertyValue, PropertyOption, TaskSchema};
use crate::core::db::models::TaskView;
use crate::core::db::store;

/// The identifier a task is known by everywhere: the session id, its worktree
/// directory, and part of its branch. Minted once and never recomputed — it names
/// things that already exist on disk.
///
/// A task keeps the id its source gave it, or takes the one its provider builds
/// (`TaskProvider::short_id`). A clash — which the UNIQUE column would otherwise
/// reject — gets a numeric suffix. This function names no provider: every format
/// lives with the provider that owns it.
pub(crate) async fn mint_short_id(
    pool: &SqlitePool,
    provider: &dyn super::TaskProvider,
    task: &FetchedTask,
    minted: &mut std::collections::HashSet<String>,
) -> anyhow::Result<String> {
    // An id already minted for this task is the only correct answer: it names a
    // branch and a directory that may already exist.
    let external_id = task.key.external_id();
    if let Some(existing) = store::provider_tasks::get_by_external_id(pool, &external_id).await? {
        return Ok(existing.short_id);
    }

    // The source's own id when it has one, else the one the provider builds.
    // Never the raw external_id: that is an API handle, not a name for a session.
    let wanted = match &task.natural_short_id {
        Some(natural) => natural.clone(),
        None => provider
            .short_id(task)
            .ok_or_else(|| anyhow::anyhow!("{} gave no short id for {external_id}", provider.id().as_str()))?,
    };

    // `short_id` is UNIQUE, and a clash used to surface as a constraint error that
    // aborted the whole sync. Suffixing is safe: nothing carries this id yet.
    let suffixed = (2..=99).map(|n| format!("{wanted}-{n}"));
    for candidate in std::iter::once(wanted.clone()).chain(suffixed) {
        if is_free(pool, minted, &candidate).await? {
            minted.insert(candidate.clone());
            return Ok(candidate);
        }
    }
    anyhow::bail!("cannot mint a short id for {wanted}: every form is taken")
}

/// `minted` covers the rest of this sync: two tasks can collide inside one batch,
/// before either has been written for the store to see.
async fn is_free(
    pool: &SqlitePool,
    minted: &std::collections::HashSet<String>,
    candidate: &str,
) -> anyhow::Result<bool> {
    if minted.contains(candidate) {
        return Ok(false);
    }
    Ok(store::provider_tasks::get_by_short_id(pool, candidate).await?.is_none())
}

/// Lowercase, alphanumerics joined by single dashes, truncated to 16 characters.
pub(crate) fn segment(text: &str) -> String {
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
            let short_id = mint_short_id(&pool, provider, task, &mut minted)
                .await
                .map_err(|e| e.to_string())?;
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
        Some(name) => ProviderId::parse(name),
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
    let short_id = mint_short_id(pool, provider, &filed, &mut Default::default()).await?;
    let row = mirror_row(&short_id, &filed);
    store::provider_tasks::upsert(pool, &row).await?;

    let mut out = filed_response(&short_id, &filed, row.synced_at);
    out["message"] = serde_json::json!(format!("Filed {short_id}"));
    Ok(out)
}

/// What a caller gets back for a task that was just filed. Shared with the
/// explorer conversion, which is this plus adopting the session onto the result —
/// two copies of these keys had already drifted apart.
pub(crate) fn filed_response(
    short_id: &str,
    filed: &FetchedTask,
    synced_at: i64,
) -> serde_json::Value {
    serde_json::json!({
        "short_id": short_id,
        "external_id": filed.key.external_id(),
        "provider": filed.key.provider().as_str(),
        "external_url": filed.url,
        "title": filed.title,
        "status": filed.status,
        "priority": null,
        "last_synced_at": synced_at,
    })
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

    /// A task carrying its source's own id, to exercise the natural path.
    fn natural(short_id: &str) -> FetchedTask {
        FetchedTask {
            key: TaskKey::Notion { page_id: "24f1a2b3c4d56789abcdef0123456789".into() },
            title: String::new(),
            status: String::new(),
            priority: None,
            url: String::new(),
            natural_short_id: Some(short_id.to_string()),
            branch_tag: None,
            board: None,
        }
    }

    /// The owner is in the id, so the common case — one repo name under two
    /// owners — cannot collide at all.
    #[tokio::test]
    async fn two_owners_sharing_a_repo_name_do_not_collide() {
        let pool = test_pool().await;
        let github = crate::provider::github::GithubProvider;
        let mut minted = std::collections::HashSet::new();

        let first = mint_short_id(&pool, &github, &gh("acme", "api", 42), &mut minted).await.unwrap();
        let second = mint_short_id(&pool, &github, &gh("beta", "api", 42), &mut minted).await.unwrap();

        assert_eq!(first, "gh-acme-api-42");
        assert_eq!(second, "gh-beta-api-42");
    }

    /// Truncation can still collide. Nothing is written to the store until the
    /// batch ends, so the store cannot be what catches it.
    #[tokio::test]
    async fn a_collision_inside_one_sync_still_gets_two_ids() {
        let pool = test_pool().await;
        let github = crate::provider::github::GithubProvider;
        let mut minted = std::collections::HashSet::new();

        let long = "kubernetes-operator-controller";
        let first = mint_short_id(&pool, &github, &gh("acme", long, 42), &mut minted).await.unwrap();
        let second = mint_short_id(&pool, &github, &gh("acme", &format!("{long}-extra"), 42), &mut minted)
            .await
            .unwrap();

        assert_ne!(first, second, "two tasks must never share a short_id");
        assert_eq!(second, format!("{first}-2"), "the collider is suffixed");
    }

    /// `short_id` is UNIQUE, so a natural id that clashes must be disambiguated.
    /// It used to be returned as-is and surface as a constraint error that failed
    /// the entire sync.
    #[tokio::test]
    async fn a_clashing_natural_id_is_disambiguated() {
        let pool = test_pool().await;
        let notion = crate::provider::notion::NotionProvider;
        let mut minted = std::collections::HashSet::new();

        let first = mint_short_id(&pool, &notion, &natural("PLAT-42"), &mut minted).await.unwrap();
        assert_eq!(first, "PLAT-42");

        // A different task at the same id: the second must not reuse it.
        let mut other = natural("PLAT-42");
        other.key = TaskKey::Notion { page_id: "0123456789abcdef0123456789abcdef".into() };
        let second = mint_short_id(&pool, &notion, &other, &mut minted).await.unwrap();
        assert_eq!(second, "PLAT-42-2", "the collider is suffixed, not repeated");
    }

    /// A provider that supplies neither a natural id nor one of its own must fail
    /// loudly. Falling back to the raw external_id would make an API handle the
    /// session's primary key, and part of a branch name.
    #[tokio::test]
    async fn no_short_id_is_an_error_not_a_raw_external_id() {
        struct Silent;
        #[async_trait::async_trait]
        impl crate::provider::TaskProvider for Silent {
            fn id(&self) -> ProviderId { ProviderId::Github }
            fn task_url(&self, _: &TaskKey) -> String { String::new() }
            async fn list_tasks(&self) -> anyhow::Result<Vec<FetchedTask>> { Ok(vec![]) }
            async fn fetch_task(&self, _: &TaskKey) -> anyhow::Result<FetchedTask> {
                anyhow::bail!("no")
            }
            async fn schema(&self, _: &TaskKey) -> anyhow::Result<TaskSchema> { anyhow::bail!("no") }
            async fn properties(&self, _: &TaskKey) -> anyhow::Result<Vec<PropertyValue>> { Ok(vec![]) }
            async fn set_property(
                &self, _: &TaskKey, _: &str, _: &serde_json::Value,
            ) -> anyhow::Result<super::super::types::PropertyWrite> { anyhow::bail!("no") }
            fn status_label(&self, _: super::super::types::StatusIntent) -> Option<String> { None }
            async fn set_status(
                &self, _: &TaskKey, _: super::super::types::StatusIntent,
            ) -> anyhow::Result<()> { Ok(()) }
            async fn discard(&self, _: &TaskKey) -> anyhow::Result<()> { Ok(()) }
            async fn body_markdown(&self, _: &TaskKey) -> anyhow::Result<String> { Ok(String::new()) }
            async fn replace_body(
                &self, _: &TaskKey, _: &str, _: bool,
            ) -> anyhow::Result<super::super::types::BodyWrite> { anyhow::bail!("no") }
        }

        let pool = test_pool().await;
        let err = mint_short_id(&pool, &Silent, &gh("acme", "repo", 7), &mut Default::default())
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("gave no short id"), "{err}");
    }
}
