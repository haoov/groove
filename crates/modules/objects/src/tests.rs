mod argo;
mod describe;
mod discovery;
mod follow;
mod logs;
mod retry;
mod watch;
mod yaml;

pub(crate) fn pods() -> groove_types::KubeKind {
    groove_types::KubeKind {
        group: String::new(),
        version: "v1".into(),
        kind: "Pod".into(),
        plural: "pods".into(),
        namespaced: true,
        watchable: true,
    }
}
