//! A firing alert, as incident.io reports it, and the child Groove spawned for one.

use crate::{SessionId, Timestamp};

/// One firing alert, its attributes read by the names the config gives.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Alert {
    pub id: String,
    pub title: String,
    pub description: String,
    /// The alert's identity across firings: one child per key.
    pub dedup_key: String,
    pub source_url: Option<String>,
    pub priority: Option<String>,
    pub cluster: Option<String>,
    pub namespace: Option<String>,
    pub created_at: Timestamp,
}

impl Alert {
    /// Critical first, then on call, then warning, then the rest.
    pub fn rank(&self) -> u8 {
        match self
            .priority
            .as_deref()
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("critical") => 0,
            Some("on call") => 1,
            Some("warning") => 2,
            _ => 3,
        }
    }

    pub fn critical(&self) -> bool {
        self.rank() == 0
    }
}

/// The child spawned for an alert, with the alert as it stood then.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct AlertChild {
    pub session: SessionId,
    pub parent: SessionId,
    pub dedup_key: String,
    pub title: String,
    pub priority: Option<String>,
    pub cluster: Option<String>,
    pub namespace: Option<String>,
}

/// Which incident.io attributes Groove reads, and the alerts it keeps.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct AlertsConfig {
    #[serde(default = "cluster")]
    pub cluster: String,
    #[serde(default = "namespace")]
    pub namespace: String,
    #[serde(default = "priority")]
    pub priority: String,
    /// Only the alerts whose attribute of this name holds one of these values.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub only: Option<AlertFilter>,
}

impl Default for AlertsConfig {
    fn default() -> Self {
        Self {
            cluster: cluster(),
            namespace: namespace(),
            priority: priority(),
            only: None,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct AlertFilter {
    pub attribute: String,
    pub values: Vec<String>,
}

fn cluster() -> String {
    "Cluster".into()
}

fn namespace() -> String {
    "Namespace".into()
}

fn priority() -> String {
    "Priority".into()
}
