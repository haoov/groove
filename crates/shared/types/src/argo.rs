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
    pub resources: Vec<AppResource>,
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
    /// Every result so far, hooks included.
    pub applied: Vec<SyncedResource>,
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
    pub group: String,
    pub kind: String,
    pub name: String,
    pub namespace: Option<String>,
    pub status: String,
    pub message: String,
    pub hook: bool,
    /// `PreSync`, `Sync`, `PostSync` or `SyncFail`.
    pub phase: String,
    /// A hook's own phase: `Running`, `Succeeded`, `Failed`.
    pub hook_phase: String,
}

impl SyncedResource {
    pub fn broke(&self) -> bool {
        self.status == "SyncFailed" || matches!(self.hook_phase.as_str(), "Failed" | "Error")
    }
}

/// One resource the Application manages, as its status lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppResource {
    pub group: String,
    pub kind: String,
    pub namespace: Option<String>,
    pub name: String,
    /// `Synced` or `OutOfSync`.
    pub sync: String,
    pub wave: i64,
    /// Gone from the sources, kept until a sync prunes it.
    pub prune: bool,
    /// Run by a sync's phase; its sync status counts for nothing.
    pub hook: bool,
}

impl AppPart {
    /// The last sync's result for `one`, hooks aside.
    pub fn result(&self, one: &AppResource) -> Option<&SyncedResource> {
        let applied = &self.operation.as_ref()?.applied;
        applied.iter().find(|done| !done.hook && one.is(done))
    }

    /// Out of sync, waiting to be pruned, or failed in the last sync.
    pub fn attention(&self, one: &AppResource) -> bool {
        one.out_of_sync() || one.prune || self.result(one).is_some_and(SyncedResource::broke)
    }
}

impl AppResource {
    pub fn out_of_sync(&self) -> bool {
        !self.hook && self.sync != "Synced"
    }

    pub fn is(&self, result: &SyncedResource) -> bool {
        let namespace = result.namespace.as_deref().unwrap_or_default();
        self.kind == result.kind
            && self.name == result.name
            && self.group == result.group
            && self.namespace.as_deref().unwrap_or_default() == namespace
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppCondition {
    /// `SyncError`, `ComparisonError`, `OrphanedResourceWarning`.
    pub kind: String,
    pub message: String,
    pub at: Option<Timestamp>,
}
