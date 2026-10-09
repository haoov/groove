//! One object read whole, as a resource tab draws it: what every kind has, then its kind's own part.

use std::sync::Arc;

use crate::{KubeKind, Timestamp};

/// What a watch of whole objects reads: a kind, in one namespace or all, narrowed on the server.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FollowKey {
    pub context: String,
    pub kind: KubeKind,
    pub namespace: Option<String>,
    /// A field selector: `metadata.name=api-0`, `involvedObject.uid=…`.
    pub fields: Option<String>,
}

impl FollowKey {
    /// The one object named `name`.
    pub fn named(context: &str, kind: &KubeKind, namespace: Option<&str>, name: &str) -> Self {
        Self {
            context: context.to_string(),
            kind: kind.clone(),
            namespace: namespace.map(String::from),
            fields: Some(format!("metadata.name={name}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Described {
    pub uid: String,
    pub name: String,
    pub namespace: Option<String>,
    pub labels: Vec<(String, String)>,
    pub annotations: Vec<(String, String)>,
    pub created: Option<Timestamp>,
    pub owners: Vec<Owner>,
    /// Ready and desired replicas, for a kind that runs some.
    pub replicas: Option<(i64, i64)>,
    /// The labels a Service picks its pods by.
    pub selector: Vec<(String, String)>,
    pub pod: Option<Box<PodPart>>,
    pub event: Option<EventRow>,
    /// The object as YAML; empty where only a part of it is read.
    pub yaml: Arc<str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Owner {
    /// `apps/v1`, or `v1` for the core group.
    pub api_version: String,
    pub kind: String,
    pub name: String,
}

impl Owner {
    pub fn group(&self) -> &str {
        self.api_version
            .split_once('/')
            .map_or("", |(group, _)| group)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PodPart {
    /// What `kubectl get` says it is: `Running`, `CrashLoopBackOff`, `Completed`.
    pub status: String,
    pub node: Option<String>,
    pub ip: Option<String>,
    pub qos: Option<String>,
    pub priority: Option<(String, i64)>,
    pub started: Option<Timestamp>,
    pub conditions: Vec<Condition>,
    /// The init containers first.
    pub containers: Vec<Container>,
    /// What it reads: `("secret", "paxone-tls")`, `("sa", "paxone")`.
    pub uses: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Condition {
    pub kind: String,
    pub met: bool,
    pub reason: Option<String>,
    pub since: Option<Timestamp>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Container {
    pub name: String,
    pub image: String,
    pub init: bool,
    pub state: State,
    pub ready: bool,
    pub restarts: u32,
    pub last: Option<Ended>,
    pub ports: Vec<String>,
    pub liveness: Option<String>,
    pub readiness: Option<String>,
    pub mounts: Vec<String>,
    /// Request and limit, as written: `250m`, `1Gi`.
    pub cpu: (Option<String>, Option<String>),
    pub memory: (Option<String>, Option<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Running { since: Option<Timestamp> },
    Waiting { reason: String },
    Terminated { reason: String, exit: i32 },
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ended {
    pub reason: String,
    pub exit: i32,
    pub at: Option<Timestamp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventRow {
    pub warning: bool,
    pub reason: String,
    pub count: u32,
    pub last: Option<Timestamp>,
    pub message: String,
    /// The object it is about: `pod/api-0`.
    pub about: String,
}

/// Usage measured by metrics-server, per container: cores and bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct Usage {
    pub containers: Vec<(String, f64, f64)>,
}
