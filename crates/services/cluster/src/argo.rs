//! Argo CD on the hub: an Application's health, drift, revisions and windows read, and its sync asked.

use std::path::PathBuf;

use groove_objects::{ArgoApp, ArgoSync, DriftFile, Stop, SyncResource, Treed};

/// An Application's resource health from Argo CD, followed until `stop`; each read goes through `send`.
pub fn argo_tree(
    paths: (Vec<PathBuf>, PathBuf),
    app: ArgoApp,
    stop: &Stop,
    send: impl Fn(Treed) + Send + Sync + 'static,
) -> impl std::future::Future<Output = ()> + Send + 'static {
    groove_objects::argo_tree(paths, app, stop, send)
}

/// An Application's resource health from Argo CD, read once.
pub async fn argo_tree_once(paths: (Vec<PathBuf>, PathBuf), app: ArgoApp) -> Treed {
    groove_objects::argo_tree_once(paths, app).await
}

/// A revision of an Application's source, read once from Argo CD.
pub async fn argo_revision_once(
    paths: (Vec<PathBuf>, PathBuf),
    app: ArgoApp,
    asked: (Option<usize>, String),
) -> std::result::Result<groove_types::RevisionMeta, String> {
    groove_objects::argo_revision_once(paths, app, asked).await
}

/// An Application's sync windows, read once from Argo CD.
pub async fn argo_windows_once(
    paths: (Vec<PathBuf>, PathBuf),
    app: ArgoApp,
) -> std::result::Result<groove_types::SyncWindows, String> {
    groove_objects::argo_windows_once(paths, app).await
}

/// An Application synced, once Argo CD takes the ask.
pub async fn argo_sync_once(
    paths: (Vec<PathBuf>, PathBuf),
    app: ArgoApp,
    sync: ArgoSync,
) -> std::result::Result<(), String> {
    groove_objects::argo_sync_once(paths, app, sync).await
}

/// An Application compared with its sources again; `hard` past Argo CD's cache.
pub async fn argo_refresh_once(
    paths: (Vec<PathBuf>, PathBuf),
    app: ArgoApp,
    hard: bool,
) -> std::result::Result<(), String> {
    groove_objects::argo_refresh_once(paths, app, hard).await
}

/// What differs, each file with the resource it stands for, read once.
pub async fn argo_drifted_once(
    paths: (Vec<PathBuf>, PathBuf),
    app: ArgoApp,
) -> std::result::Result<Vec<(SyncResource, DriftFile)>, String> {
    groove_objects::argo_drifted_once(paths, app).await
}

/// What differs between an Application's desired and live state, read once.
pub async fn argo_drift_once(
    paths: (Vec<PathBuf>, PathBuf),
    app: ArgoApp,
) -> std::result::Result<Vec<DriftFile>, String> {
    groove_objects::argo_drift_once(paths, app).await
}
