mod projects;
mod schema;

use crate::core::config;
use crate::provider::types::*;
use crate::provider::TaskProvider;

pub struct GithubProvider;

/// The owner/repo/number a GitHub key carries.
fn issue_of(key: &TaskKey) -> anyhow::Result<(&str, &str, &str, i64)> {
    match key {
        TaskKey::Github { host, owner, repo, number } => Ok((host, owner, repo, *number)),
        other => anyhow::bail!("not a GitHub task: {}", other.external_id()),
    }
}

impl GithubProvider {
    fn fetched(&self, cfg: &config::GithubConfig, item: &projects::BoardItem) -> FetchedTask {
        let status_field = &cfg.properties.status;
        let priority_field = cfg.properties.priority.as_deref();
        let field = |name: &str| {
            item.fields.iter().find(|f| f.name == name).map(|f| f.display.clone())
        };

        FetchedTask {
            key: TaskKey::Github {
                host: cfg.host.clone(),
                owner: item.owner.clone(),
                repo: item.repo.clone(),
                number: item.number,
            },
            title: item.title.clone(),
            // An issue on the board with no Status set is still queued.
            status: field(status_field).unwrap_or_else(|| "Backlog".to_string()),
            priority: priority_field.and_then(field),
            url: item.url.clone(),
            // No natural short id: the store mints gh-<repo>-<number>.
            natural_short_id: None,
            // Only the issue number goes in the branch — a branch is scoped to one
            // repo, so the rest of the id would be noise.
            branch_tag: Some(item.number.to_string()),
        }
    }

    /// The board item behind a task, and the board it came from. When an issue is
    /// on several configured boards the first one listed wins, so its fields do not
    /// flip between syncs.
    async fn item_for(
        &self,
        cfg: &config::GithubConfig,
        key: &TaskKey,
    ) -> anyhow::Result<(String, projects::BoardItem)> {
        let (_, owner, repo, number) = issue_of(key)?;
        for project in &cfg.projects {
            let items = projects::board_items(cfg, &project.id).await?;
            if let Some(item) =
                items.into_iter().find(|i| i.owner == owner && i.repo == repo && i.number == number)
            {
                return Ok((project.id.clone(), item));
            }
        }
        anyhow::bail!("{owner}/{repo}#{number} is not on any configured board")
    }
}

#[async_trait::async_trait]
impl TaskProvider for GithubProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Github
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            external_hours: false,
            template: false,
            create: false,
            editable_body: false,
            discard: false,
        }
    }

    fn task_url(&self, key: &TaskKey) -> String {
        match issue_of(key) {
            Ok((host, owner, repo, number)) => format!("https://{host}/{owner}/{repo}/issues/{number}"),
            Err(_) => key.external_id(),
        }
    }

    async fn list_tasks(&self) -> anyhow::Result<Vec<FetchedTask>> {
        let cfg = config::github()?;
        let mut out: Vec<FetchedTask> = Vec::new();

        for project in &cfg.projects {
            for item in projects::board_items(&cfg, &project.id).await? {
                let task = self.fetched(&cfg, &item);
                // First board listed wins, so a shared issue does not flip fields.
                if !out.iter().any(|t| t.key == task.key) {
                    out.push(task);
                }
            }
        }
        Ok(out)
    }

    async fn fetch_task(&self, key: &TaskKey) -> anyhow::Result<FetchedTask> {
        let cfg = config::github()?;
        let (_, item) = self.item_for(&cfg, key).await?;
        Ok(self.fetched(&cfg, &item))
    }

    async fn schema(&self, key: &TaskKey) -> anyhow::Result<TaskSchema> {
        let cfg = config::github()?;
        let (project_id, _) = self.item_for(&cfg, key).await?;
        schema::board_schema(&cfg, &project_id).await
    }

    async fn properties(&self, key: &TaskKey) -> anyhow::Result<Vec<PropertyValue>> {
        let cfg = config::github()?;
        let (project_id, item) = self.item_for(&cfg, key).await?;
        let schema = schema::board_schema(&cfg, &project_id).await?;

        Ok(schema
            .properties
            .iter()
            .map(|p| {
                let found = item.fields.iter().find(|f| f.name == p.name);
                let (value, display) = match (p.name.as_str(), found) {
                    // These live on the issue, not in a board field.
                    ("Labels", _) => (
                        serde_json::json!(item.labels),
                        item.labels.join(", "),
                    ),
                    ("Assignees", _) => (
                        serde_json::json!(item.assignees),
                        item.assignees.join(", "),
                    ),
                    (_, Some(f)) => (f.value.clone(), f.display.clone()),
                    (_, None) => (serde_json::Value::Null, String::new()),
                };
                PropertyValue { name: p.name.clone(), kind: p.kind.clone(), value, display }
            })
            .collect())
    }

    fn status_label(&self, intent: StatusIntent) -> Option<String> {
        let cfg = config::github().ok()?;
        let map = &cfg.status_map;
        Some(match intent {
            StatusIntent::Ready => map.ready.clone(),
            StatusIntent::InProgress => map.in_progress.clone(),
            StatusIntent::Done => map.done.clone(),
        })
    }

    async fn set_status(&self, _key: &TaskKey, _intent: StatusIntent) -> anyhow::Result<()> {
        anyhow::bail!("writing to a GitHub board is not implemented yet")
    }

    async fn discard(&self, _key: &TaskKey) -> anyhow::Result<()> {
        anyhow::bail!("closing a GitHub issue from here is not implemented yet")
    }

    async fn set_property(
        &self,
        _key: &TaskKey,
        _property: &str,
        _value: &serde_json::Value,
    ) -> anyhow::Result<PropertyWrite> {
        anyhow::bail!("writing to a GitHub board is not implemented yet")
    }

    async fn body_markdown(&self, key: &TaskKey) -> anyhow::Result<String> {
        let cfg = config::github()?;
        let (_, item) = self.item_for(&cfg, key).await?;
        Ok(item.body)
    }

    async fn replace_body(
        &self,
        _key: &TaskKey,
        _markdown: &str,
        _force: bool,
    ) -> anyhow::Result<BodyWrite> {
        anyhow::bail!("editing a GitHub issue body from here is not implemented yet")
    }
}
