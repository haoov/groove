//! The kubeconfig contexts on this machine, and whether each one signs in.

/// One context a kubeconfig names. Groove keeps its name; the credentials stay in the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KubeContext {
    pub name: String,
    pub cluster: String,
    pub server: Option<String>,
    pub namespace: Option<String>,
    pub auth: KubeAuth,
}

/// How a context signs in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KubeAuth {
    /// A plugin run for each token: `kubelogin` and its like.
    Exec {
        command: String,
    },
    Certificate,
    Token,
    None,
}

/// What the last check of a context found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Login {
    SignedIn {
        version: String,
    },
    /// A sign-in fixes it: the token expired, or the plugin failed.
    Refused(String),
    Unreachable(String),
    /// Anything else: the context is gone, or the server answered an error.
    Failed(String),
}

/// A context a session holds, on one namespace or, with none, the whole cluster.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Attached {
    pub context: String,
    pub namespace: Option<String>,
}

/// One kind a cluster serves at its preferred version.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KubeKind {
    /// Empty for the core group.
    pub group: String,
    pub version: String,
    pub kind: String,
    pub plural: String,
    pub namespaced: bool,
    /// It can be listed and watched.
    pub watchable: bool,
}

/// What one watcher reads: a kind of a context, in one namespace or across the cluster.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WatchKey {
    pub context: String,
    pub kind: KubeKind,
    pub namespace: Option<String>,
    /// A label selector the server filters on.
    pub selector: Option<String>,
}

/// A column of the server's Table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableColumn {
    pub name: String,
    /// 0 for what `kubectl get` shows, above for `-o wide`.
    pub priority: i32,
    /// The server writes the cell as an age.
    pub date: bool,
}

/// One object as a row of the Table: its metadata and its cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectRow {
    pub uid: String,
    pub name: String,
    pub namespace: Option<String>,
    pub version: String,
    pub cells: Vec<String>,
    pub aging: Vec<crate::Aging>,
}

/// The heading a kind stands under in the Resources sidebar, in the order they stand.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KindHeading {
    Builtin(Builtin),
    /// The API group a CRD adds its kinds to.
    Group(String),
    /// A built-in kind not namespaced: nodes, namespaces, volumes and the like.
    Cluster,
}

/// The headings of Kubernetes' own namespaced kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Builtin {
    Workloads,
    Network,
    Config,
    Storage,
    Rbac,
    /// What no other heading takes: events, leases and the like.
    Other,
}

impl KindHeading {
    pub fn label(&self) -> &str {
        match self {
            KindHeading::Builtin(Builtin::Workloads) => "Workloads",
            KindHeading::Builtin(Builtin::Network) => "Network",
            KindHeading::Builtin(Builtin::Config) => "Config",
            KindHeading::Builtin(Builtin::Storage) => "Storage",
            KindHeading::Builtin(Builtin::Rbac) => "RBAC",
            KindHeading::Builtin(Builtin::Other) => "Other",
            KindHeading::Group(group) => group,
            KindHeading::Cluster => "Cluster",
        }
    }
}

const WORKLOADS: [&str; 7] = [
    "Pod",
    "Deployment",
    "StatefulSet",
    "DaemonSet",
    "ReplicaSet",
    "Job",
    "CronJob",
];
const NETWORK: [&str; 4] = ["Service", "Ingress", "NetworkPolicy", "EndpointSlice"];
const CONFIG: [&str; 7] = [
    "ConfigMap",
    "Secret",
    "ServiceAccount",
    "ResourceQuota",
    "LimitRange",
    "HorizontalPodAutoscaler",
    "PodDisruptionBudget",
];
const RBAC: &str = "rbac.authorization.k8s.io";

impl KubeKind {
    pub fn heading(&self) -> KindHeading {
        let kind = self.kind.as_str();
        let builtin = |one| KindHeading::Builtin(one);
        match () {
            _ if !self.built_in() => KindHeading::Group(self.group.clone()),
            _ if self.group == RBAC => builtin(Builtin::Rbac),
            _ if !self.namespaced => KindHeading::Cluster,
            _ if WORKLOADS.contains(&kind) => builtin(Builtin::Workloads),
            _ if NETWORK.contains(&kind) => builtin(Builtin::Network),
            _ if CONFIG.contains(&kind) => builtin(Builtin::Config),
            _ if kind == "PersistentVolumeClaim" => builtin(Builtin::Storage),
            _ => builtin(Builtin::Other),
        }
    }

    /// Kubernetes' own: the core group, the unqualified groups and the `*.k8s.io` ones.
    fn built_in(&self) -> bool {
        !self.group.contains('.') || self.group.ends_with(".k8s.io")
    }
}
