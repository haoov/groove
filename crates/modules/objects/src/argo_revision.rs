//! What Argo CD says once of an Application: a revision of a source, and the sync windows on it.

use std::path::PathBuf;

use groove_types::{RevisionMeta, SyncWindows};

use crate::argo_tree::{ArgoApp, Opened};

/// `revision` of `app`'s source `source` of several, read once.
pub async fn argo_revision_once(
    (paths, home): (Vec<PathBuf>, PathBuf),
    app: ArgoApp,
    (source, revision): (Option<usize>, String),
) -> Result<RevisionMeta, String> {
    let mut opened = Opened::of((&paths, &home), &app).await?;
    let path = groove_argocd::revision_path(&app.namespace, &app.name, &revision, source);
    let body = opened.get(&path).await?;
    groove_argocd::revision(&body).map_err(|why| opened.refused(&why))
}

/// `app`'s sync windows, read once.
pub async fn argo_windows_once(
    (paths, home): (Vec<PathBuf>, PathBuf),
    app: ArgoApp,
) -> Result<SyncWindows, String> {
    let mut opened = Opened::of((&paths, &home), &app).await?;
    let body = opened
        .get(&groove_argocd::windows_path(&app.namespace, &app.name))
        .await?;
    groove_argocd::windows(&body).map_err(|why| opened.refused(&why))
}
