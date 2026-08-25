//! Where tasks come from. One module per source, behind a shared trait.

pub mod commands;
pub mod detect;
pub mod github;
pub mod notion;
pub mod types;
pub mod write;

use types::*;

// Glob re-export is required: tauri::generate_handler! looks up __cmd__* symbols
// at the same path as the function.
pub use commands::*;
pub use github::setup::*;
pub use notion::users::*;
pub use write::*;

/// One task source.
///
/// The defaulted methods ARE the capability signal: `reference_options → []`,
/// `template_markdown`/`add_hours` → `Ok(None)`, `create_task` → error. A
/// provider without the feature inherits the default and the app treats the
/// answer as "this source has none", never as a failure. `replace_body` may
/// ignore `force` when its body format loses nothing in a round trip.
#[async_trait::async_trait]
pub(crate) trait TaskProvider: Send + Sync {
    fn id(&self) -> ProviderId;
    fn task_url(&self, key: &TaskKey) -> String;

    /// The short id for a task whose source gives it none of its own.
    ///
    /// A short_id is the session's primary key and can become part of a branch
    /// name, so it must be stable and safe in a git ref. Build it so it cannot
    /// collide within this provider; a clash is still deduplicated, but the id
    /// carries the suffix forever. `None` means every task here arrives with a
    /// `natural_short_id`.
    fn short_id(&self, task: &FetchedTask) -> Option<String> {
        let _ = task;
        None
    }

    async fn list_tasks(&self) -> anyhow::Result<Vec<FetchedTask>>;
    async fn fetch_task(&self, key: &TaskKey) -> anyhow::Result<FetchedTask>;

    async fn schema(&self, key: &TaskKey) -> anyhow::Result<TaskSchema>;
    async fn properties(&self, key: &TaskKey) -> anyhow::Result<Vec<PropertyValue>>;
    async fn set_property(
        &self,
        key: &TaskKey,
        property: &str,
        value: &serde_json::Value,
    ) -> anyhow::Result<PropertyWrite>;

    /// Choices for a reference-valued property. Empty when there are none.
    async fn reference_options(
        &self,
        key: &TaskKey,
        property: &str,
    ) -> anyhow::Result<Vec<PropertyOption>> {
        let _ = (key, property);
        Ok(vec![])
    }

    /// Write the status the intent maps to, and return the label ACTUALLY
    /// written — the caller mirrors that, never its own guess of it.
    async fn set_status(&self, key: &TaskKey, intent: StatusIntent) -> anyhow::Result<String>;
    async fn discard(&self, key: &TaskKey) -> anyhow::Result<()>;

    async fn body_markdown(&self, key: &TaskKey) -> anyhow::Result<String>;
    async fn replace_body(
        &self,
        key: &TaskKey,
        markdown: &str,
        force: bool,
    ) -> anyhow::Result<BodyWrite>;

    /// Add to the source's own hours field. `Ok(None)` means it has none — time is
    /// still tracked locally, so that is an answer, not a failure.
    async fn add_hours(&self, key: &TaskKey, hours: f64) -> anyhow::Result<Option<HoursWrite>> {
        let _ = (key, hours);
        Ok(None)
    }

    async fn template_markdown(&self) -> anyhow::Result<Option<String>> {
        Ok(None)
    }

    async fn create_task(&self, draft: &TaskDraft<'_>) -> anyhow::Result<FetchedTask> {
        let _ = draft;
        anyhow::bail!("{} cannot file new tasks", self.id().as_str())
    }
}

static NOTION: notion::NotionProvider = notion::NotionProvider;
static GITHUB: github::GithubProvider = github::GithubProvider;

/// One row per provider: its instance, and whether the config carries it. THE
/// enumeration point — nothing else may list providers.
struct Entry {
    id: ProviderId,
    instance: &'static dyn TaskProvider,
    configured: fn(&crate::core::config::Config) -> bool,
}

/// Sized by `ProviderId::ALL`, so a new variant refuses to compile until its
/// row exists. Keep the order of `ALL`.
static REGISTRY: [Entry; ProviderId::ALL.len()] = [
    Entry { id: ProviderId::Notion, instance: &NOTION, configured: |c| c.notion.is_some() },
    Entry { id: ProviderId::Github, instance: &GITHUB, configured: |c| c.github.is_some() },
];

fn entry(id: ProviderId) -> &'static Entry {
    REGISTRY.iter().find(|e| e.id == id).expect("REGISTRY covers every ProviderId")
}

/// A provider, if it is configured. Asking for one that is not set up is the
/// error the caller wants, not a client that fails on its first call.
pub(crate) fn get(id: ProviderId) -> anyhow::Result<&'static dyn TaskProvider> {
    let e = entry(id);
    let configured = crate::core::config::get().is_some_and(|c| (e.configured)(&c));
    if !configured {
        anyhow::bail!("{} is not set up — add it in Settings", id.as_str());
    }
    Ok(e.instance)
}

/// Every configured provider, for the queue fan-out.
pub(crate) fn enabled() -> Vec<&'static dyn TaskProvider> {
    let Some(cfg) = crate::core::config::get() else { return vec![] };
    REGISTRY
        .iter()
        .filter(|e| (e.configured)(&cfg))
        .map(|e| e.instance)
        .collect()
}

/// At least one task source is set up. The one guard for "would leave none".
pub(crate) fn has_task_source(cfg: &crate::core::config::Config) -> bool {
    REGISTRY.iter().any(|e| (e.configured)(cfg))
}

/// The provider names as prose — "notion or github" — for tool descriptions
/// that must name the real set, not a stale copy of it.
pub(crate) fn names_prose() -> String {
    let names: Vec<&str> = ProviderId::ALL.iter().map(|p| p.as_str()).collect();
    match names.as_slice() {
        [] => String::new(),
        [one] => (*one).to_string(),
        [head @ .., last] => format!("{} or {last}", head.join(", ")),
    }
}

/// The mirror row for a task the provider just reported.
pub(crate) fn mirror_row(
    short_id: &str,
    task: &FetchedTask,
) -> crate::core::db::models::ProviderTask {
    crate::core::db::models::ProviderTask {
        external_id: task.key.external_id(),
        short_id: short_id.to_string(),
        title: task.title.clone(),
        status: task.status.clone(),
        priority: task.priority.clone(),
        synced_at: chrono::Utc::now().timestamp(),
        provider: task.key.provider().as_str().to_string(),
        url: Some(task.url.clone()),
        board: task.board.clone(),
        branch_tag: task.branch_tag.clone(),
    }
}

/// The provider a task belongs to, and its key at the source.
///
/// Reads the mirror rather than the session, so it answers for a task that has
/// never been opened. Explorer and review sessions have no mirror row.
pub(crate) async fn resolve(
    pool: &sqlx::SqlitePool,
    short_id: &str,
) -> anyhow::Result<(&'static dyn TaskProvider, TaskKey)> {
    let row = crate::core::db::store::provider_tasks::get_by_short_id(pool, short_id)
        .await?
        .ok_or_else(|| {
            anyhow::anyhow!("{short_id} is not a task — explorer and review sessions have no source")
        })?;
    // The row's own column decides the provider. Deriving it from the id's shape
    // instead would guess, and the shapes are not reserved.
    let id = ProviderId::parse(&row.provider)?;
    let key = TaskKey::parse(id, &row.external_id)?;
    Ok((get(id)?, key))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(json: &str) -> crate::core::config::Config {
        serde_json::from_str(json).expect("must parse")
    }

    const NOTION_ONLY: &str = r#"{
      "notion": {
        "token": "t", "database_id": "db", "user_id": "u",
        "properties": { "status": "Status" },
        "status_map": { "ready": "Ready", "in_progress": "Doing", "done": "Done" },
        "filters": { "exclude_statuses": [], "filter_by_assignee": true }
      },
      "git": { "worktree_root": "~/w" }
    }"#;

    #[test]
    fn the_registry_covers_every_provider_in_order() {
        for (i, id) in ProviderId::ALL.into_iter().enumerate() {
            assert_eq!(REGISTRY[i].id, id, "REGISTRY must keep ProviderId::ALL's order");
        }
    }

    #[test]
    fn has_task_source_reads_any_configured_provider() {
        assert!(has_task_source(&cfg(NOTION_ONLY)));
        assert!(has_task_source(&cfg(
            r#"{ "github": { "host": "github.com",
                 "properties": { "status": "Status" },
                 "status_map": { "ready": "Ready", "in_progress": "Doing", "done": "Done" } },
                 "git": { "worktree_root": "~/w" } }"#
        )));
        assert!(!has_task_source(&cfg(r#"{ "git": { "worktree_root": "~/w" } }"#)));
    }

    /// Tool descriptions interpolate this — it must name every provider.
    #[test]
    fn names_prose_names_them_all() {
        let prose = names_prose();
        for id in ProviderId::ALL {
            assert!(prose.contains(id.as_str()), "{prose} misses {}", id.as_str());
        }
    }
}
