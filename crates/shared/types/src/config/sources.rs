use std::fmt;

use crate::{Priority, StatusIntent};

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
        named(&self.assignee)
    }

    pub fn sprint(&self) -> Option<&str> {
        named(&self.sprint)
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

/// Everything of `NotionConfig` except the token.
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

/// The provider's own name for each property Groove reads; an empty name is a gap.
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

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PropertyNames {
    pub status: String,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub start: Option<String>,
    #[serde(default)]
    pub due: Option<String>,
    #[serde(default)]
    pub estimate: Option<String>,
    #[serde(default)]
    pub estimate_unit: EstimateUnit,
    /// Where `task.log_hours` adds what Groove measured.
    #[serde(default)]
    pub logged: Option<String>,
    #[serde(default)]
    pub project: Option<String>,
}

/// What the source's estimate counts in; Groove holds it in hours.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EstimateUnit {
    #[default]
    Hours,
    Days,
}

/// The hours a day of estimate stands for.
const DAY_HOURS: f32 = 8.0;

impl EstimateUnit {
    pub const ALL: [EstimateUnit; 2] = [EstimateUnit::Hours, EstimateUnit::Days];

    pub fn label(self) -> &'static str {
        match self {
            EstimateUnit::Hours => "hours",
            EstimateUnit::Days => "days",
        }
    }

    pub fn parse(label: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|one| one.label() == label)
    }

    /// A value the source holds, in hours.
    pub fn hours(self, value: f32) -> f32 {
        match self {
            EstimateUnit::Hours => value,
            EstimateUnit::Days => value * DAY_HOURS,
        }
    }
}

/// Which status values mean what; the first of each is the one Groove writes.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StatusMap {
    pub ready: Vec<String>,
    pub in_progress: Vec<String>,
    pub done: Vec<String>,
}

impl StatusMap {
    /// What the provider's own status label means to Groove.
    pub fn intent(&self, label: &str) -> Option<StatusIntent> {
        let holds = |names: &[String]| names.iter().any(|name| name == label);
        match label {
            _ if holds(&self.done) => Some(StatusIntent::Done),
            _ if holds(&self.in_progress) => Some(StatusIntent::InProgress),
            _ if holds(&self.ready) => Some(StatusIntent::Ready),
            _ => None,
        }
    }

    /// The label Groove writes for an intent.
    pub fn label(&self, intent: StatusIntent) -> Option<&str> {
        let names = match intent {
            StatusIntent::Ready => &self.ready,
            StatusIntent::InProgress => &self.in_progress,
            StatusIntent::Done => &self.done,
        };
        names.first().map(String::as_str)
    }
}

/// Which of the provider's priority values mean what.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PriorityMap {
    pub high: Vec<String>,
    pub medium: Vec<String>,
    pub low: Vec<String>,
}

impl PriorityMap {
    pub fn level(&self, label: &str) -> Option<Priority> {
        let holds = |names: &[String]| names.iter().any(|name| name == label);
        match label {
            _ if holds(&self.high) => Some(Priority::High),
            _ if holds(&self.medium) => Some(Priority::Medium),
            _ if holds(&self.low) => Some(Priority::Low),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FilterConfig {
    pub exclude_statuses: Vec<String>,
}

/// A name the file gives, blank being none.
fn named(one: &Option<String>) -> Option<&str> {
    one.as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
}
