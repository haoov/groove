use crate::{Day, Error, ExternalId, Result, Timestamp};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderId {
    Github,
    Notion,
}

impl ProviderId {
    pub const ALL: [ProviderId; 2] = [ProviderId::Github, ProviderId::Notion];

    pub fn as_str(self) -> &'static str {
        match self {
            ProviderId::Github => "github",
            ProviderId::Notion => "notion",
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
    Github {
        host: String,
        owner: String,
        repo: String,
        number: u64,
    },
    Notion {
        page_id: String,
    },
}

impl TaskKey {
    /// How the key reads as one string: what the database and the MR footer carry.
    pub fn external_id(&self) -> ExternalId {
        match self {
            TaskKey::Github {
                host,
                owner,
                repo,
                number,
            } => ExternalId::new(format!("{host}/{owner}/{repo}#{number}")),
            TaskKey::Notion { page_id } => ExternalId::new(page_id.clone()),
        }
    }

    pub fn provider(&self) -> ProviderId {
        match self {
            TaskKey::Github { .. } => ProviderId::Github,
            TaskKey::Notion { .. } => ProviderId::Notion,
        }
    }

    /// The key an external id reads as: `<host>/<owner>/<repo>#<number>` is GitHub's,
    /// and anything without a number of its own is a Notion page.
    pub fn parse(id: &ExternalId) -> Result<Self> {
        let invalid = || Error::invalid(format!("not a task id: {}", id.as_str()));
        let Some((path, number)) = id.as_str().rsplit_once('#') else {
            return Ok(TaskKey::Notion {
                page_id: id.as_str().to_string(),
            });
        };
        let [repo, owner, host] = path.rsplitn(3, '/').collect::<Vec<&str>>()[..] else {
            return Err(invalid());
        };
        if host.is_empty() || owner.is_empty() || repo.is_empty() {
            return Err(invalid());
        }
        Ok(TaskKey::Github {
            host: host.to_string(),
            owner: owner.to_string(),
            repo: repo.to_string(),
            number: number.parse().map_err(|_| invalid())?,
        })
    }
}

/// How much a task matters, whatever its provider calls it.
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    High,
    Medium,
    Low,
}

impl Priority {
    pub const ALL: [Priority; 3] = [Priority::High, Priority::Medium, Priority::Low];

    pub fn label(self) -> &'static str {
        match self {
            Priority::High => "high",
            Priority::Medium => "medium",
            Priority::Low => "low",
        }
    }
}

/// A task as its provider last reported it.
#[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Task {
    pub external_id: ExternalId,
    pub short_id: String,
    pub title: String,
    /// The provider's own status label, and what it means to Groove.
    pub status: String,
    pub intent: Option<StatusIntent>,
    pub priority: Option<Priority>,
    pub dates: TaskDates,
    /// The hours the task is estimated at, and the hours logged against it.
    pub estimate: Option<f32>,
    pub logged: Option<f32>,
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
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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

impl TimeSummary {
    pub fn tracked_hours(self) -> f32 {
        hours(self.tracked_seconds)
    }

    pub fn unlogged_hours(self) -> f32 {
        hours(self.unlogged_seconds)
    }
}

/// Seconds as the hours a tenth is read in.
pub fn hours(seconds: i64) -> f32 {
    (seconds as f32 / 360.0).round() / 10.0
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
