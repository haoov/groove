use std::fmt;

use super::properties::{PriorityMap, PropertyNames, StatusMap};

/// Stands in for every secret in a `Debug` rendering.
pub const REDACTED: &str = "<redacted>";

#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NotionConfig {
    pub token: String,
    pub database_id: String,
    pub user_id: String,
    /// Required: the people property that says a page is yours, the relation to its sprint.
    #[serde(default)]
    pub assignee: Option<String>,
    #[serde(default)]
    pub sprint: Option<String>,
    /// The status of the running sprint.
    #[serde(default)]
    pub sprint_status: Option<String>,
    pub properties: PropertyNames,
    pub status_map: StatusMap,
    #[serde(default)]
    pub priority_map: PriorityMap,
    pub filters: FilterConfig,
    #[serde(default)]
    pub task_template_page_id: Option<String>,
    #[serde(default)]
    pub default_project_id: Option<String>,
}

impl NotionConfig {
    pub fn assignee(&self) -> Option<&str> {
        crate::mapping::given(self.assignee.as_deref())
    }

    pub fn sprint(&self) -> Option<&str> {
        crate::mapping::given(self.sprint.as_deref())
    }

    /// The required names not given yet; the source reads no task without them.
    pub fn unmapped(&self) -> Vec<&'static str> {
        let required = [("assignee", self.assignee()), ("sprint", self.sprint())];
        let missing = required.into_iter().filter(|(_, name)| name.is_none());
        missing.map(|(label, _)| label).collect()
    }

    /// The status a sprint carries while it is the one running.
    pub fn sprint_status(&self) -> &str {
        self.sprint_status.as_deref().unwrap_or("Current")
    }
}

impl fmt::Debug for NotionConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NotionConfig")
            .field("token", &REDACTED)
            .field("database_id", &self.database_id)
            .field("user_id", &self.user_id)
            .field("sprint", &self.sprint)
            .field("properties", &self.properties)
            .field("status_map", &self.status_map)
            .field("priority_map", &self.priority_map)
            .field("filters", &self.filters)
            .field("task_template_page_id", &self.task_template_page_id)
            .field("default_project_id", &self.default_project_id)
            .finish()
    }
}

impl NotionConfig {
    /// One user's tasks of one database, every name a gap.
    pub fn bare(token: &str, database_id: &str, user_id: &str) -> Self {
        Self {
            token: token.trim().to_string(),
            database_id: database_id.trim().to_string(),
            user_id: user_id.trim().to_string(),
            assignee: None,
            sprint: None,
            sprint_status: None,
            properties: PropertyNames::default(),
            status_map: StatusMap::default(),
            priority_map: PriorityMap::default(),
            filters: FilterConfig {
                exclude_statuses: Vec::new(),
            },
            task_template_page_id: None,
            default_project_id: None,
        }
    }
}

/// What `NotionConfig` shows: everything but the token, the assignee and the sprint status.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct NotionView {
    pub database_id: String,
    pub user_id: String,
    pub sprint: Option<String>,
    pub properties: PropertyNames,
    pub status_map: StatusMap,
    pub priority_map: PriorityMap,
    pub filters: FilterConfig,
    pub task_template_page_id: Option<String>,
    pub default_project_id: Option<String>,
}

impl From<NotionConfig> for NotionView {
    fn from(c: NotionConfig) -> Self {
        Self {
            database_id: c.database_id,
            user_id: c.user_id,
            sprint: c.sprint,
            properties: c.properties,
            status_map: c.status_map,
            priority_map: c.priority_map,
            filters: c.filters,
            task_template_page_id: c.task_template_page_id,
            default_project_id: c.default_project_id,
        }
    }
}

/// A token of your own; without one the host's token comes from `gh auth token`.
#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GithubConfig {
    pub host: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    pub properties: PropertyNames,
    pub status_map: StatusMap,
    #[serde(default)]
    pub priority_map: PriorityMap,
}

impl GithubConfig {
    /// A source on `host`, the token read from `gh`, every name a gap.
    pub fn bare(host: &str) -> Self {
        Self {
            host: host.trim().to_string(),
            token: None,
            properties: PropertyNames::default(),
            status_map: StatusMap::default(),
            priority_map: PriorityMap::default(),
        }
    }
}

impl fmt::Debug for GithubConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GithubConfig")
            .field("host", &self.host)
            .field("token", &self.token.as_ref().map(|_| REDACTED))
            .field("properties", &self.properties)
            .field("status_map", &self.status_map)
            .field("priority_map", &self.priority_map)
            .finish()
    }
}

/// Everything of `GithubConfig` except the token.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct GithubView {
    pub host: String,
    pub properties: PropertyNames,
    pub status_map: StatusMap,
    pub priority_map: PriorityMap,
}

impl From<GithubConfig> for GithubView {
    fn from(c: GithubConfig) -> Self {
        Self {
            host: c.host,
            properties: c.properties,
            status_map: c.status_map,
            priority_map: c.priority_map,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FilterConfig {
    #[serde(deserialize_with = "super::names::one_or_many")]
    pub exclude_statuses: Vec<String>,
}
