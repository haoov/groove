//! What an Argo CD Application says of itself: where it syncs from and to, how, and its last sync.

use crate::Timestamp;

#[derive(Debug, Clone, PartialEq)]
pub struct AppPart {
    /// When Argo CD last compared it with its sources, and to what: it moves when the drift may.
    pub compared: String,
    pub project: String,
    pub sync: String,
    pub health: String,
    pub sources: Vec<AppSource>,
    /// The revision it last synced to.
    pub synced: Option<String>,
    pub destination: Destination,
    /// Auto-sync, with prune and self-heal as set; none syncs by hand.
    pub automated: Option<(bool, bool)>,
    /// The sync options every sync applies: `ServerSideApply=true`.
    pub options: Vec<String>,
    pub retry: Option<Retry>,
    pub operation: Option<Operation>,
    pub conditions: Vec<AppCondition>,
    pub resources: Vec<AppResource>,
}

/// How a failed sync is tried again: `limit` times, waiting `backoff` grown by `factor` up to `max`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Retry {
    /// Below zero, without end.
    pub limit: i64,
    pub backoff: Option<String>,
    pub factor: Option<i64>,
    pub max: Option<String>,
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
    /// The revision its last successful sync applied.
    pub synced: Option<String>,
    /// The revision Argo CD last compared against: where `target` points now.
    pub compared: Option<String>,
}

impl AppSource {
    pub fn git(&self) -> bool {
        self.chart.is_none()
    }
}

/// The sync windows an Application's project puts on it, and whether one lets it sync now.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SyncWindows {
    pub can_sync: bool,
    pub assigned: Vec<SyncWindow>,
    pub active: Vec<SyncWindow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SyncWindow {
    /// `allow` or `deny`.
    pub kind: String,
    /// A cron schedule: `0 22 * * *`.
    pub schedule: String,
    pub duration: String,
    pub time_zone: String,
    /// A sync asked by hand passes it.
    pub manual: bool,
}

/// What Argo CD's repo server says of one revision.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RevisionMeta {
    pub author: String,
    pub date: Option<Timestamp>,
    /// Its first line only.
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Destination {
    pub server: Option<String>,
    pub name: Option<String>,
    pub namespace: Option<String>,
}

/// The address an Application's own cluster goes by.
const IN_CLUSTER: &str = "https://kubernetes.default.svc";

impl Destination {
    /// Whether it names the Application's own cluster.
    pub fn in_cluster(&self) -> bool {
        match (self.server.as_deref(), self.name.as_deref()) {
            (None, Some(name)) => name == "in-cluster",
            (Some(server), _) => server_of(server) == IN_CLUSTER,
            _ => false,
        }
    }
}

/// A server address as one form: scheme and host in lower case, no default port, no trailing slash.
pub fn server_of(server: &str) -> String {
    let server = server.trim_end_matches('/');
    let (scheme, rest) = server.split_once("://").unwrap_or(("https", server));
    let (host, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
    let (scheme, host) = (scheme.to_ascii_lowercase(), host.to_ascii_lowercase());
    let port = if scheme == "http" { ":80" } else { ":443" };
    let host = host.strip_suffix(port).unwrap_or(&host);
    format!("{scheme}://{host}{path}")
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
    /// Gone from the sources, kept until a sync prunes it; never a hook.
    pub prune: bool,
    /// Run by a sync's phase; its sync status counts for nothing.
    pub hook: bool,
}

impl AppPart {
    /// The commit the first git source on a branch synced to.
    pub fn synced_commit(&self) -> Option<&str> {
        let on_branch = |one: &&AppSource| one.chart.is_none() && !crate::is_sha(&one.target);
        let synced = self
            .sources
            .iter()
            .filter(on_branch)
            .filter_map(|one| one.synced.as_deref());
        synced.into_iter().find(|one| crate::is_sha(one))
    }

    /// The last sync's result for `one`, hooks aside.
    pub fn result(&self, one: &AppResource) -> Option<&SyncedResource> {
        let applied = &self.operation.as_ref()?.applied;
        applied.iter().find(|done| !done.hook && one.is(done))
    }

    /// Out of sync, to prune, failed in the last sync, or degraded: anything to see.
    pub fn notable(&self, one: &AppResource, health: &[ResourceHealth]) -> bool {
        let broke = self.result(one).is_some_and(SyncedResource::broke);
        one.out_of_sync() || one.prune || broke || health_of(health, one) == "Degraded"
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

/// A resource's health as Argo CD's resource tree holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceHealth {
    pub group: String,
    pub kind: String,
    pub namespace: Option<String>,
    pub name: String,
    /// `Healthy`, `Progressing`, `Degraded`, `Suspended`, `Missing` or `Unknown`; empty for none.
    pub status: String,
    pub reason: String,
}

/// What Argo CD says of `one`'s health; empty where it says nothing.
pub fn health_of<'a>(health: &'a [ResourceHealth], one: &AppResource) -> &'a str {
    let found = health.iter().find(|held| held.of(one));
    found.map_or("", |held| held.status.as_str())
}

impl ResourceHealth {
    pub fn of(&self, one: &AppResource) -> bool {
        self.kind == one.kind
            && self.name == one.name
            && self.group == one.group
            && self.namespace.as_deref().unwrap_or_default()
                == one.namespace.as_deref().unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppCondition {
    /// `SyncError`, `ComparisonError`, `OrphanedResourceWarning`.
    pub kind: String,
    pub message: String,
    pub at: Option<Timestamp>,
}

/// Where a resource's drift stands in the diff: `argo/shop/Deployment/web.yaml`.
pub fn drift_path(kind: &str, namespace: Option<&str>, name: &str) -> String {
    crate::object_path("argo", kind, namespace, name)
}

/// `auto · prune · heal` for an automated sync policy, `manual` without one.
pub fn sync_policy(automated: Option<(bool, bool)>) -> String {
    let Some((prune, heal)) = automated else {
        return "manual".to_string();
    };
    let mut said = "auto".to_string();
    if prune {
        said.push_str(" · prune");
    }
    if heal {
        said.push_str(" · heal");
    }
    said
}
