//! How a task source's own properties read as Groove's: their names, units, statuses and priorities.

use crate::{Priority, StatusIntent};

/// The provider's own name for each property Groove reads; an empty name is a gap.
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
    #[serde(deserialize_with = "super::names::one_or_many")]
    pub ready: Vec<String>,
    #[serde(deserialize_with = "super::names::one_or_many")]
    pub in_progress: Vec<String>,
    #[serde(deserialize_with = "super::names::one_or_many")]
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
    #[serde(deserialize_with = "super::names::one_or_many")]
    pub high: Vec<String>,
    #[serde(deserialize_with = "super::names::one_or_many")]
    pub medium: Vec<String>,
    #[serde(deserialize_with = "super::names::one_or_many")]
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
