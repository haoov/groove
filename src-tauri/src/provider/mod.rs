//! Task sources. One module per source, behind a shared trait.
//!
//! # Adding a provider
//!
//! 1. `types.rs`: a `ProviderId` variant + its `ALL` entry and `as_str` arm.
//! 2. `types.rs`: a `TaskKey` variant + its `parse`/`external_id`/`provider` arms.
//! 3. This file: the provider module, its static instance, and a `REGISTRY` row.
//! 4. `core/config/mod.rs`: a `Config` field. A token-bearing provider also
//!    needs a `ConfigView` mapping that strips the secret, and an accessor.
//! 5. `provider/<name>/setup.rs`: the setup payload struct + `build_config`;
//!    then the `set_task_source` match (task_manager/setup.rs) and
//!    `write_initial_config` gain their arms.
//! 6. `task_manager/setup.rs`: a `SetupRequest` field, named EXACTLY
//!    `ProviderId::as_str()`.
//! 7. Frontend: `src/setup/sources/index.tsx` (`SOURCES`) and
//!    `src/shared/lib/actions/taskProvider.ts` (`PROVIDERS`).
//! 8. `pnpm gen:types`.
//!
//! No edit needed: MCP tool prose, the DB schema, resolve/minting/mirroring,
//! approvals, the Home filter.

pub mod commands;
pub mod detect;
pub mod github;
pub mod notion;
pub mod types;
pub mod write;

use types::*;

// Keep the glob re-exports: tauri::generate_handler! resolves `__cmd__*` at the function's path.
pub use commands::*;
pub use github::setup::*;
pub use notion::setup::*;
pub use notion::users::*;
pub use write::*;

/// One task source. A defaulted method means the source lacks that feature.
#[async_trait::async_trait]
pub(crate) trait TaskProvider: Send + Sync {
    fn id(&self) -> ProviderId;
    fn task_url(&self, key: &TaskKey) -> String;

    /// Short id for a task whose source gives it none. Stable, unique within the
    /// provider, and safe in a git ref. `None` when every task has a `natural_short_id`.
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

    /// Write the status the intent maps to and return the label actually written.
    async fn set_status(&self, key: &TaskKey, intent: StatusIntent) -> anyhow::Result<String>;
    async fn discard(&self, key: &TaskKey) -> anyhow::Result<()>;

    async fn body_markdown(&self, key: &TaskKey) -> anyhow::Result<String>;
    async fn replace_body(
        &self,
        key: &TaskKey,
        markdown: &str,
        force: bool,
    ) -> anyhow::Result<BodyWrite>;

    /// Add to the source's own hours field. `Ok(None)` when it has none.
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

/// One row per provider. The only place that enumerates providers.
struct Entry {
    id: ProviderId,
    instance: &'static dyn TaskProvider,
    configured: fn(&crate::core::config::Config) -> bool,
}

/// Sized by `ProviderId::ALL`. Keep the order of `ALL`.
static REGISTRY: [Entry; ProviderId::ALL.len()] = [
    Entry {
        id: ProviderId::Notion,
        instance: &NOTION,
        configured: |c| c.notion.is_some(),
    },
    Entry {
        id: ProviderId::Github,
        instance: &GITHUB,
        configured: |c| c.github.is_some(),
    },
];

fn entry(id: ProviderId) -> &'static Entry {
    REGISTRY
        .iter()
        .find(|e| e.id == id)
        .expect("REGISTRY covers every ProviderId")
}

/// A configured provider. Errors when the provider is not set up.
pub(crate) fn get(id: ProviderId) -> anyhow::Result<&'static dyn TaskProvider> {
    let e = entry(id);
    let configured = crate::core::config::get().is_some_and(|c| (e.configured)(&c));
    if !configured {
        anyhow::bail!("{} is not set up — add it in Settings", id.as_str());
    }
    Ok(e.instance)
}

/// Every configured provider.
pub(crate) fn enabled() -> Vec<&'static dyn TaskProvider> {
    let Some(cfg) = crate::core::config::get() else {
        return vec![];
    };
    REGISTRY
        .iter()
        .filter(|e| (e.configured)(&cfg))
        .map(|e| e.instance)
        .collect()
}

/// At least one task source is set up.
pub(crate) fn has_task_source(cfg: &crate::core::config::Config) -> bool {
    REGISTRY.iter().any(|e| (e.configured)(cfg))
}

/// The provider names as prose: "notion or github".
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

/// The provider a task belongs to, and its key at the source. Reads the mirror
/// row, not the session.
pub(crate) async fn resolve(
    pool: &sqlx::SqlitePool,
    short_id: &str,
) -> anyhow::Result<(&'static dyn TaskProvider, TaskKey)> {
    let row = crate::core::db::store::provider_tasks::get_by_short_id(pool, short_id)
        .await?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "{short_id} is not a task — explorer and review sessions have no source"
            )
        })?;
    // The row's `provider` column decides the provider, never the id's shape.
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
            assert_eq!(
                REGISTRY[i].id, id,
                "REGISTRY must keep ProviderId::ALL's order"
            );
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
        assert!(!has_task_source(&cfg(
            r#"{ "git": { "worktree_root": "~/w" } }"#
        )));
    }

    /// A provider name outside `provider/` and `forge/` branches on the provider
    /// instead of going through `resolve()` / `get()`.
    #[test]
    fn only_provider_and_forge_name_a_provider() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut offenders = vec![];
        visit(&src, &mut offenders);
        assert!(
            offenders.is_empty(),
            "a provider is named outside provider/ and forge/: {offenders:?}"
        );

        fn visit(dir: &std::path::Path, offenders: &mut Vec<String>) {
            for entry in std::fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    visit(&path, offenders);
                    continue;
                }
                let shown = path.to_string_lossy().to_string();
                if path.extension().is_none_or(|e| e != "rs")
                    || shown.contains("/provider/")
                    || shown.contains("/forge/")
                    || shown.ends_with("/tests.rs")
                {
                    continue;
                }
                let text = std::fs::read_to_string(&path).unwrap();
                // A test module and a comment may both name one.
                for (at, line) in text
                    .split("#[cfg(test)]")
                    .next()
                    .unwrap_or("")
                    .lines()
                    .enumerate()
                {
                    let code = line.split("//").next().unwrap_or("");
                    if ProviderId::ALL
                        .iter()
                        .any(|id| code.contains(&format!("\"{}\"", id.as_str())))
                    {
                        offenders.push(format!("{shown}:{}", at + 1));
                    }
                }
            }
        }
    }

    #[test]
    fn names_prose_names_them_all() {
        let prose = names_prose();
        for id in ProviderId::ALL {
            assert!(
                prose.contains(id.as_str()),
                "{prose} misses {}",
                id.as_str()
            );
        }
    }
}
