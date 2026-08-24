//! Where tasks come from. One module per source, behind a shared trait.

pub mod commands;
pub mod detect;
pub mod github;
pub mod notion;
pub mod types;

use types::*;

// Glob re-export is required: tauri::generate_handler! looks up __cmd__* symbols
// at the same path as the function.
pub use commands::*;
pub use github::setup::*;
pub use notion::*;

#[async_trait::async_trait]
pub(crate) trait TaskProvider: Send + Sync {
    fn id(&self) -> ProviderId;
    fn task_url(&self, key: &TaskKey) -> String;

    /// Short ids to try for a task with no natural id of its own, best first —
    /// the first one not already taken is minted.
    ///
    /// A short_id names a session, a worktree directory and part of a branch, so
    /// it must be filesystem-safe and stable. A provider whose tasks always carry
    /// `natural_short_id` never needs this.
    fn short_id_candidates(&self, task: &FetchedTask) -> Vec<String> {
        let _ = task;
        vec![]
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

    fn status_label(&self, intent: StatusIntent) -> Option<String>;
    async fn set_status(&self, key: &TaskKey, intent: StatusIntent) -> anyhow::Result<()>;
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

/// A provider, if it is configured. Asking for one that is not set up is the
/// error the caller wants, not a client that fails on its first call.
pub(crate) fn get(id: ProviderId) -> anyhow::Result<&'static dyn TaskProvider> {
    let configured = crate::core::config::get().is_some_and(|c| match id {
        ProviderId::Notion => c.notion.is_some(),
        ProviderId::Github => c.github.is_some(),
    });
    if !configured {
        anyhow::bail!("{} is not set up — add it in Settings", id.as_str());
    }
    match id {
        ProviderId::Notion => Ok(&NOTION),
        ProviderId::Github => Ok(&GITHUB),
    }
}

/// Every configured provider, for the queue fan-out.
pub(crate) fn enabled() -> Vec<&'static dyn TaskProvider> {
    let Some(cfg) = crate::core::config::get() else { return vec![] };
    let mut out: Vec<&'static dyn TaskProvider> = vec![];
    if cfg.notion.is_some() {
        out.push(&NOTION);
    }
    if cfg.github.is_some() {
        out.push(&GITHUB);
    }
    out
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
