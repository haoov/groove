use std::fmt;

/// Stands in for every secret in a `Debug` rendering.
pub const REDACTED: &str = "<redacted>";

#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NotionConfig {
    pub token: String,
    pub database_id: String,
    pub user_id: String,
    pub properties: PropertyNames,
    pub status_map: StatusMap,
    pub filters: FilterConfig,
    #[serde(default)]
    pub task_template_page_id: Option<String>,
    #[serde(default)]
    pub default_project_id: Option<String>,
}

impl fmt::Debug for NotionConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NotionConfig")
            .field("token", &REDACTED)
            .field("database_id", &self.database_id)
            .field("user_id", &self.user_id)
            .field("properties", &self.properties)
            .field("status_map", &self.status_map)
            .field("filters", &self.filters)
            .field("task_template_page_id", &self.task_template_page_id)
            .field("default_project_id", &self.default_project_id)
            .finish()
    }
}

/// Everything of `NotionConfig` except the token.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct NotionView {
    pub database_id: String,
    pub user_id: String,
    pub properties: PropertyNames,
    pub status_map: StatusMap,
    pub filters: FilterConfig,
    pub task_template_page_id: Option<String>,
    pub default_project_id: Option<String>,
}

impl From<NotionConfig> for NotionView {
    fn from(c: NotionConfig) -> Self {
        Self {
            database_id: c.database_id,
            user_id: c.user_id,
            properties: c.properties,
            status_map: c.status_map,
            filters: c.filters,
            task_template_page_id: c.task_template_page_id,
            default_project_id: c.default_project_id,
        }
    }
}

/// The token comes from `gh auth token`, never from here.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GithubConfig {
    pub host: String,
    pub properties: GithubPropertyNames,
    pub status_map: StatusMap,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GithubPropertyNames {
    pub status: String,
    pub priority: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PropertyNames {
    pub status: String,
    pub priority: Option<String>,
    pub sprint: Option<String>,
    pub project: Option<String>,
    pub assignee: Option<String>,
}

/// The three status values the app writes.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StatusMap {
    pub ready: String,
    pub in_progress: String,
    pub done: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FilterConfig {
    pub exclude_statuses: Vec<String>,
    #[serde(default = "default_true")]
    pub filter_by_assignee: bool,
}

fn default_true() -> bool {
    true
}
