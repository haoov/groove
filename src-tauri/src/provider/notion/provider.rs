use crate::core::config;
use crate::provider::types::*;
use crate::provider::TaskProvider;

pub struct NotionProvider;

/// The page id a Notion key carries.
fn page_of(key: &TaskKey) -> anyhow::Result<&str> {
    match key {
        TaskKey::Notion { page_id } => Ok(page_id),
        other => anyhow::bail!("not a Notion task: {}", other.external_id()),
    }
}

impl NotionProvider {
    fn fetched(&self, task: &crate::core::db::models::ProviderTask) -> FetchedTask {
        let key = TaskKey::Notion { page_id: task.external_id.clone() };
        FetchedTask {
            url: self.task_url(&key),
            key,
            title: task.title.clone(),
            status: task.status.clone(),
            priority: task.priority.clone(),
            natural_short_id: Some(task.short_id.clone()),
            branch_tag: None,
        }
    }
}

#[async_trait::async_trait]
impl TaskProvider for NotionProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Notion
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            external_hours: true,
            template: true,
            create: true,
            editable_body: true,
            discard: true,
        }
    }

    fn task_url(&self, key: &TaskKey) -> String {
        match key {
            TaskKey::Notion { page_id } => {
                format!("https://www.notion.so/{}", page_id.replace('-', ""))
            }
            other => other.external_id(),
        }
    }

    async fn list_tasks(&self) -> anyhow::Result<Vec<FetchedTask>> {
        Ok(super::tasks::fetch_queue().await?.iter().map(|t| self.fetched(t)).collect())
    }

    async fn fetch_task(&self, key: &TaskKey) -> anyhow::Result<FetchedTask> {
        let task = super::tasks::fetch_page(page_of(key)?).await?;
        Ok(self.fetched(&task))
    }

    async fn schema(&self, _key: &TaskKey) -> anyhow::Result<TaskSchema> {
        let cfg = config::notion()?;
        super::schema::load(&cfg.token, &cfg.database_id).await
    }

    async fn properties(&self, key: &TaskKey) -> anyhow::Result<Vec<PropertyValue>> {
        let cfg = config::notion()?;
        super::properties::read_all(&cfg, page_of(key)?).await
    }

    async fn set_property(
        &self,
        key: &TaskKey,
        property: &str,
        value: &serde_json::Value,
    ) -> anyhow::Result<PropertyWrite> {
        let cfg = config::notion()?;
        let display =
            super::properties::patch_property(&cfg, page_of(key)?, property, value).await?;
        Ok(PropertyWrite { display })
    }

    async fn reference_options(
        &self,
        _key: &TaskKey,
        property: &str,
    ) -> anyhow::Result<Vec<PropertyOption>> {
        let cfg = config::notion()?;
        let schema = super::schema::load(&cfg.token, &cfg.database_id).await?;
        match schema.relation_target(property) {
            Some(db) => super::schema::relation_options(&cfg.token, db).await,
            None => Ok(vec![]),
        }
    }

    fn status_label(&self, intent: StatusIntent) -> Option<String> {
        let cfg = config::notion().ok()?;
        let map = &cfg.status_map;
        Some(match intent {
            StatusIntent::Ready => map.ready.clone(),
            StatusIntent::InProgress => map.in_progress.clone(),
            StatusIntent::Done => map.done.clone(),
        })
    }

    async fn set_status(&self, key: &TaskKey, intent: StatusIntent) -> anyhow::Result<()> {
        let cfg = config::notion()?;
        let label = self
            .status_label(intent)
            .ok_or_else(|| anyhow::anyhow!("no status configured for {intent:?}"))?;
        super::tasks::set_status(
            &cfg.token,
            page_of(key)?,
            &cfg.properties.status,
            &label,
        )
        .await
    }

    async fn discard(&self, key: &TaskKey) -> anyhow::Result<()> {
        let cfg = config::notion()?;
        super::tasks::trash(&cfg.token, page_of(key)?).await
    }

    async fn body_markdown(&self, key: &TaskKey) -> anyhow::Result<String> {
        let cfg = config::notion()?;
        let blocks = super::get_task_body_impl(page_of(key)?, &cfg.token).await?;
        Ok(super::markdown::blocks_to_markdown(&blocks))
    }

    async fn replace_body(
        &self,
        key: &TaskKey,
        markdown: &str,
        force: bool,
    ) -> anyhow::Result<BodyWrite> {
        let cfg = config::notion()?;
        super::body::replace(&cfg.token, page_of(key)?, markdown, force).await
    }

    async fn add_hours(&self, key: &TaskKey, hours: f64) -> anyhow::Result<HoursWrite> {
        let cfg = config::notion()?;
        let property =
            super::hours::hours_property(&cfg.token, &cfg.database_id).await?;
        let (before, after) =
            super::hours::add_hours(&cfg.token, page_of(key)?, &property, hours).await?;
        Ok(HoursWrite { before, after })
    }

    async fn template_markdown(&self) -> anyhow::Result<Option<String>> {
        let cfg = config::notion()?;
        match cfg.task_template_page_id.as_deref() {
            Some(id) => Ok(Some(super::body::template_markdown(id, &cfg.token).await?)),
            None => Ok(None),
        }
    }

    async fn create_task(&self, draft: &TaskDraft<'_>) -> anyhow::Result<FetchedTask> {
        let cfg = config::notion()?;
        // Notion files into its one database; the draft's repo means nothing here.
        let payload = super::new_task_payload(&cfg, draft.title, draft.body_markdown);
        let req = super::NewTask::from_payload(&payload)?;
        let (page_id, short_id) = super::create::create_page(&cfg.token, &req).await?;
        let key = TaskKey::Notion { page_id };
        Ok(FetchedTask {
            url: self.task_url(&key),
            key,
            title: draft.title.to_string(),
            status: cfg.status_map.ready.clone(),
            priority: None,
            natural_short_id: Some(short_id),
            branch_tag: None,
        })
    }
}
