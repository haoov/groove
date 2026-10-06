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
