//! What an Argo CD Application says of itself: where it syncs from and to, how, and its last sync.

use crate::Timestamp;

#[derive(Debug, Clone, PartialEq)]
pub struct AppPart {
    pub project: String,
    pub sync: String,
    pub health: String,
    pub sources: Vec<AppSource>,
    /// The revision it last synced to.
    pub synced: Option<String>,
    pub destination: Destination,
    /// Auto-sync, with prune and self-heal as set; none syncs by hand.
    pub automated: Option<(bool, bool)>,
    pub operation: Option<Operation>,
    pub conditions: Vec<AppCondition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppSource {
    pub repo: String,
    pub path: Option<String>,
    pub chart: Option<String>,
    /// The branch, tag or chart version it follows.
    pub target: String,
    /// The Helm value files it renders with.
    pub values: Vec<String>,
    /// The name another source reads its files by: `values` for `$values/…`.
    pub reference: Option<String>,
    /// The revision it last synced to.
    pub synced: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Destination {
    pub server: Option<String>,
    pub name: Option<String>,
    pub namespace: Option<String>,
}

/// The sync running, or the last one: its phase, what it said, and what it could not apply.
#[derive(Debug, Clone, PartialEq)]
pub struct Operation {
    /// `Running`, `Succeeded`, `Failed`, `Error`, `Terminating`.
    pub phase: String,
    pub message: String,
    pub revision: Option<String>,
    pub started: Option<Timestamp>,
    pub finished: Option<Timestamp>,
    pub retries: u32,
    /// Who asked: a user's name, or `auto-sync`.
    pub by: String,
    pub failed: Vec<SyncedResource>,
}

impl Operation {
    pub fn running(&self) -> bool {
        matches!(self.phase.as_str(), "Running" | "Terminating")
    }

    pub fn failed(&self) -> bool {
        matches!(self.phase.as_str(), "Failed" | "Error")
    }
}

/// One resource of a sync's result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncedResource {
    pub kind: String,
    pub name: String,
    pub namespace: Option<String>,
    pub status: String,
    pub message: String,
    pub hook: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppCondition {
    /// `SyncError`, `ComparisonError`, `OrphanedResourceWarning`.
    pub kind: String,
    pub message: String,
    pub at: Option<Timestamp>,
}
