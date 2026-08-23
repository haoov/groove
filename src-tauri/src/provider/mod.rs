//! Where tasks come from. One module per source, behind a shared trait.

pub mod notion;
pub mod types;

use types::*;

// Glob re-export is required: tauri::generate_handler! looks up __cmd__* symbols
// at the same path as the function.
pub use notion::*;

// Callers are routed onto this in the next step.
#[allow(dead_code)]
#[async_trait::async_trait]
pub(crate) trait TaskProvider: Send + Sync {
    fn id(&self) -> ProviderId;
    fn capabilities(&self) -> Capabilities;
    fn task_url(&self, key: &TaskKey) -> String;

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
    ) -> anyhow::Result<Vec<RelationOption>> {
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

    async fn add_hours(&self, key: &TaskKey, hours: f64) -> anyhow::Result<HoursWrite> {
        let _ = (key, hours);
        anyhow::bail!(
            "{} tasks have no hours field; the local ledger still recorded it",
            self.id().as_str()
        )
    }

    async fn template_markdown(&self) -> anyhow::Result<Option<String>> {
        Ok(None)
    }

    async fn create_task(&self, draft: &TaskDraft<'_>) -> anyhow::Result<FetchedTask> {
        let _ = draft;
        anyhow::bail!("{} cannot file new tasks", self.id().as_str())
    }
}

#[allow(dead_code)]
static NOTION: notion::NotionProvider = notion::NotionProvider;

/// A provider, if it is configured.
#[allow(dead_code)]
pub(crate) fn get(id: ProviderId) -> anyhow::Result<&'static dyn TaskProvider> {
    match id {
        ProviderId::Notion => Ok(&NOTION),
        ProviderId::Github => anyhow::bail!("the GitHub provider is not built yet"),
    }
}

/// Every configured provider, for the queue fan-out.
#[allow(dead_code)]
pub(crate) fn enabled() -> Vec<&'static dyn TaskProvider> {
    vec![&NOTION]
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
        board: None,
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
    let key = TaskKey::parse(&row.external_id)?;
    Ok((get(key.provider())?, key))
}
