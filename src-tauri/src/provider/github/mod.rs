mod cache;
mod fields;
mod issues;
mod projects;
mod schema;
pub mod setup;

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
            // No natural short id: short_id() below builds one.
            natural_short_id: None,
            // Only the issue number goes in the branch — a branch is scoped to one
            // repo, so the rest of the id would be noise.
            branch_tag: Some(item.number.to_string()),
            board: Some(item.board.clone()),
        }
    }

    /// The board item behind a task.
    ///
    /// The queue cache is the fast path, but it only holds OPEN issues ASSIGNED
    /// to you — a task whose issue was closed or reassigned must stay operable
    /// for its open session, so a miss falls back to fetching the issue itself.
    async fn item_for(
        &self,
        cfg: &config::GithubConfig,
        key: &TaskKey,
    ) -> anyhow::Result<projects::BoardItem> {
        let (_, owner, repo, number) = issue_of(key)?;
        let cached = projects::cached_issues(&cfg.host)
            .await?
            .into_iter()
            .find(|i| i.owner == owner && i.repo == repo && i.number == number);
        if let Some(item) = cached {
            return Ok(item);
        }
        projects::fetch_item(&cfg.host, owner, repo, number)
            .await?
            .ok_or_else(|| anyhow::anyhow!("{owner}/{repo}#{number} is not on any project board"))
    }
}

#[async_trait::async_trait]
impl TaskProvider for GithubProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Github
    }

    fn task_url(&self, key: &TaskKey) -> String {
        match issue_of(key) {
            Ok((host, owner, repo, number)) => format!("https://{host}/{owner}/{repo}/issues/{number}"),
            Err(_) => key.external_id(),
        }
    }

    /// `gh-<owner>-<repo>-<number>` — the owner is always in it. Leaving it out
    /// reads better but collides whenever two owners share a repo name, and the
    /// loser then carries a numeric suffix for the rest of its life.
    fn short_id(&self, task: &FetchedTask) -> Option<String> {
        let TaskKey::Github { owner, repo, number, .. } = &task.key else {
            return None;
        };
        let seg = crate::provider::commands::segment;
        Some(format!("gh-{}-{}-{number}", seg(owner), seg(repo)))
    }

    async fn list_tasks(&self) -> anyhow::Result<Vec<FetchedTask>> {
        let cfg = config::github()?;
        Ok(projects::assigned_issues(&cfg.host)
            .await?
            .iter()
            .map(|item| self.fetched(&cfg, item))
            .collect())
    }

    async fn fetch_task(&self, key: &TaskKey) -> anyhow::Result<FetchedTask> {
        let cfg = config::github()?;
        let item = self.item_for(&cfg, key).await?;
        Ok(self.fetched(&cfg, &item))
    }

    async fn schema(&self, key: &TaskKey) -> anyhow::Result<TaskSchema> {
        let cfg = config::github()?;
        let item = self.item_for(&cfg, key).await?;
        schema::board_schema(&cfg, &item.project_id).await
    }

    async fn properties(&self, key: &TaskKey) -> anyhow::Result<Vec<PropertyValue>> {
        let cfg = config::github()?;
        let item = self.item_for(&cfg, key).await?;
        let schema = schema::board_schema(&cfg, &item.project_id).await?;

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

    async fn set_status(&self, key: &TaskKey, intent: StatusIntent) -> anyhow::Result<String> {
        let cfg = config::github()?;
        let item = self.item_for(&cfg, key).await?;
        // The board's own columns, not the config: every board names them
        // differently, and this is the write a user makes every day.
        let label = fields::status_for(&cfg, &item.project_id, intent).await;
        if label.is_empty() {
            // Name the board's real columns: the fix is one config line, and
            // without the list the user cannot know what to put there.
            let columns = fields::field_def(&cfg.host, &item.project_id, &cfg.properties.status)
                .await
                .map(|d| d.options.into_iter().map(|(n, _)| n).collect::<Vec<_>>())
                .unwrap_or_default();
            return Err(fields::no_status_error(intent, &columns));
        }
        fields::set_field(
            &cfg,
            &item.project_id,
            &item.item_id,
            &cfg.properties.status,
            &serde_json::json!(label),
        )
        .await?;
        Ok(label)
    }

    async fn discard(&self, key: &TaskKey) -> anyhow::Result<()> {
        let cfg = config::github()?;
        let (_, owner, repo, number) = issue_of(key)?;
        issues::close_not_planned(&cfg, owner, repo, number).await
    }

    async fn set_property(
        &self,
        key: &TaskKey,
        property: &str,
        value: &serde_json::Value,
    ) -> anyhow::Result<PropertyWrite> {
        let cfg = config::github()?;
        let item = self.item_for(&cfg, key).await?;
        let display =
            fields::set_field(&cfg, &item.project_id, &item.item_id, property, value).await?;
        Ok(PropertyWrite { display })
    }

    async fn body_markdown(&self, key: &TaskKey) -> anyhow::Result<String> {
        let cfg = config::github()?;
        let item = self.item_for(&cfg, key).await?;
        Ok(item.body)
    }

    /// An issue body is markdown already, so nothing can be lost in the round trip
    /// and `force` has nothing to decide.
    async fn replace_body(
        &self,
        key: &TaskKey,
        markdown: &str,
        _force: bool,
    ) -> anyhow::Result<BodyWrite> {
        let cfg = config::github()?;
        let (_, owner, repo, number) = issue_of(key)?;
        issues::set_body(&cfg, owner, repo, number, markdown).await?;
        Ok(BodyWrite { blocks_written: markdown.lines().count() })
    }

    async fn add_hours(&self, key: &TaskKey, hours: f64) -> anyhow::Result<Option<HoursWrite>> {
        let cfg = config::github()?;
        let item = self.item_for(&cfg, key).await?;
        let schema = schema::board_schema(&cfg, &item.project_id).await?;
        let Some(name) = schema.hours_property else {
            return Ok(None);
        };

        let before = item
            .fields
            .iter()
            .find(|f| f.name == name)
            .and_then(|f| f.value.as_f64())
            .unwrap_or(0.0);
        let after = before + hours;
        fields::set_field(&cfg, &item.project_id, &item.item_id, &name, &serde_json::json!(after))
            .await?;
        Ok(Some(HoursWrite { before, after }))
    }

    /// File an issue in the draft's repo and put it on the first configured board.
    /// A task that is not on a board would not come back from `list_tasks`.
    async fn create_task(&self, draft: &TaskDraft<'_>) -> anyhow::Result<FetchedTask> {
        let cfg = config::github()?;
        let slug = draft
            .repo
            .ok_or_else(|| anyhow::anyhow!("filing a GitHub issue needs a repo to file it in"))?;
        // The last two segments: a caller passes either `owner/repo` or a repo id,
        // which carries the host in front of it.
        let mut parts = slug.rsplit('/');
        let (Some(repo), Some(owner)) = (parts.next(), parts.next()) else {
            anyhow::bail!("expected owner/repo, got {slug}");
        };
        let project_id = issues::repo_board(&cfg, owner, repo).await?;

        let (number, url, node_id) =
            issues::create(&cfg, owner, repo, draft.title, draft.body_markdown).await?;
        let item_id = issues::add_to_board(&cfg, &project_id, &node_id).await?;

        let status = fields::status_for(&cfg, &project_id, StatusIntent::Ready).await;
        if !status.is_empty() {
            fields::set_field(
                &cfg,
                &project_id,
                &item_id,
                &cfg.properties.status,
                &serde_json::json!(status),
            )
            .await?;
        }

        Ok(FetchedTask {
            key: TaskKey::Github {
                host: cfg.host.clone(),
                owner: owner.to_string(),
                repo: repo.to_string(),
                number,
            },
            title: draft.title.to_string(),
            status,
            priority: None,
            url,
            natural_short_id: None,
            branch_tag: Some(number.to_string()),
            board: None,
        })
    }
}

#[cfg(test)]
mod tests {
    /// The same split create_task does. A caller passes either `owner/repo` or a
    /// repo id, which is `host/owner/repo` — taking the first two segments made
    /// every GitHub conversion ask GitHub for a repo called `owner/repo` owned by
    /// the hostname.
    fn owner_repo(slug: &str) -> Option<(&str, &str)> {
        let mut parts = slug.rsplit('/');
        match (parts.next(), parts.next()) {
            (Some(repo), Some(owner)) => Some((owner, repo)),
            _ => None,
        }
    }

    #[test]
    fn a_repo_id_and_a_bare_slug_both_resolve() {
        assert_eq!(owner_repo("haoov/groove"), Some(("haoov", "groove")));
        assert_eq!(owner_repo("github.com/haoov/groove"), Some(("haoov", "groove")));
        assert_eq!(owner_repo("groove"), None);
    }
}
