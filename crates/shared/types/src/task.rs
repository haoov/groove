use crate::{Day, Error, ExternalId, Result, Timestamp};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderId {
    Notion,
    Github,
}

impl ProviderId {
    pub const ALL: [ProviderId; 2] = [ProviderId::Notion, ProviderId::Github];

    pub fn as_str(self) -> &'static str {
        match self {
            ProviderId::Notion => "notion",
            ProviderId::Github => "github",
        }
    }

    pub fn parse(name: &str) -> Result<Self> {
        Self::ALL
            .into_iter()
            .find(|p| p.as_str() == name)
            .ok_or_else(|| Error::invalid(format!("unknown task source {name}")))
    }
}

/// A task's identity at its source.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TaskKey {
    Notion {
        page_id: String,
    },
    Github {
        host: String,
        owner: String,
        repo: String,
        number: u64,
    },
}

impl TaskKey {
    pub fn provider(&self) -> ProviderId {
        match self {
            TaskKey::Notion { .. } => ProviderId::Notion,
            TaskKey::Github { .. } => ProviderId::Github,
        }
    }
}

/// A task as its provider last reported it.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Task {
    pub external_id: ExternalId,
    pub short_id: String,
    pub title: String,
    pub status: String,
    pub priority: Option<String>,
    pub synced_at: Timestamp,
    pub provider: ProviderId,
    pub url: Option<String>,
    pub board: Option<String>,
    pub branch_tag: Option<String>,
}

impl Task {
    /// What a branch name carries for this task.
    pub fn tag(&self) -> &str {
        self.branch_tag.as_deref().unwrap_or(&self.short_id)
    }
}

/// What the app asks for; the provider owns the label it writes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StatusIntent {
    Ready,
    InProgress,
    Done,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TimeSummary {
    pub tracked_seconds: i64,
    pub logged_seconds: i64,
    pub today_seconds: i64,
    pub unlogged_seconds: i64,
}

/// The dates a task carries as properties.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TaskDates {
    pub start: Option<Day>,
    pub due: Option<Day>,
    pub duration_days: Option<u32>,
}

/// How a task draws on the timeline band.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Span {
    Bar { from: Day, to: Day },
    Point(Day),
    Open { from: Day },
}

impl TaskDates {
    pub fn span(&self) -> Option<Span> {
        match (self.start, self.due, self.duration_days) {
            (Some(from), _, Some(days)) => Some(Span::Bar {
                from,
                to: from.plus_days(i64::from(days)),
            }),
            (Some(from), Some(to), None) => Some(Span::Bar { from, to }),
            (Some(from), None, None) => Some(Span::Open { from }),
            (None, Some(due), _) => Some(Span::Point(due)),
            (None, None, _) => None,
        }
    }
}
