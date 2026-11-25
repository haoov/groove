//! A sync and a refresh of one Application, asked of Argo CD on the hub.

use std::path::PathBuf;

use groove_argocd::Resource;

use crate::argo_tree::{ArgoApp, Opened};

/// A sync of the whole Application, or of `resources` alone; `prune` deletes what left the sources.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ArgoSync {
    pub prune: bool,
    pub resources: Vec<Resource>,
}

/// `app` synced once Argo CD takes the ask; the sync itself runs on after.
pub async fn argo_sync_once(
    (paths, home): (Vec<PathBuf>, PathBuf),
    app: ArgoApp,
    sync: ArgoSync,
) -> Result<(), String> {
    let mut opened = Opened::of((&paths, &home), &app).await?;
    let body = groove_argocd::sync_body(&app.namespace, sync.prune, &sync.resources);
    let answer = opened
        .post(&groove_argocd::sync_path(&app.name), body)
        .await?;
    groove_argocd::taken(&answer).map_err(|why| opened.refused(&why))
}

/// `app` compared with its sources again; `hard` reads them past Argo CD's cache.
pub async fn argo_refresh_once(
    (paths, home): (Vec<PathBuf>, PathBuf),
    app: ArgoApp,
    hard: bool,
) -> Result<(), String> {
    let mut opened = Opened::of((&paths, &home), &app).await?;
    let path = groove_argocd::refresh_path(&app.namespace, &app.name, hard);
    let answer = opened.get(&path).await?;
    groove_argocd::taken(&answer).map_err(|why| opened.refused(&why))
}
