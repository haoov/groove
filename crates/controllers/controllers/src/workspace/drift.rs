//! An Argo CD Application's drift, read from Argo CD on the hub and shown as the diff's change.

use groove_cluster_service::ArgoApp;
use groove_workspace_service::DriftKey;

use crate::{AppState, Continuation, Services, Spawner};

/// One read at a time for an Application; a version shown, or failed, is not read again.
pub(super) fn show(state: &mut AppState, spawner: &dyn Spawner, (app, at): (DriftKey, String)) {
    let workspace = &mut state.workspace;
    let shown = workspace
        .drift
        .as_ref()
        .is_some_and(|one| one.app == app && one.at == at);
    let asked = (app.clone(), at.clone());
    let failed = workspace.drift_read.failed.as_ref();
    let failed = failed.is_some_and(|(one, version, _)| *one == app && *version == at);
    let asking = workspace.drift_read.asking.as_ref();
    let asking = asking.is_some_and(|(one, _)| *one == app);
    if shown || failed || asking {
        return;
    }
    let hub = state
        .config
        .clusters()
        .iter()
        .any(|one| one.argo_hub && one.context == app.0);
    if !hub {
        return;
    }
    workspace.drift_read.asking = Some(asked.clone());
    let paths = (state.env.kubeconfig.clone(), state.env.home.clone());
    spawner.spawn(Box::pin(read(paths, asked)));
}

/// The drift read from Argo CD and made ready off the main thread, then landed if still asked.
async fn read(
    paths: (Vec<std::path::PathBuf>, std::path::PathBuf),
    asked: (DriftKey, String),
) -> Continuation {
    let (hub, namespace, name) = asked.0.clone();
    let argo = ArgoApp {
        hub,
        namespace,
        name,
    };
    let read = match groove_cluster_service::argo_drift_once(paths, argo).await {
        Ok(pairs) => groove_workspace_service::prepared(pairs)
            .await
            .map_err(|e| e.message),
        Err(why) => Err(why),
    };
    Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
        let workspace = &mut state.workspace;
        if workspace.drift_read.asking.as_ref() != Some(&asked) {
            return;
        }
        workspace.drift_read.asking = None;
        match read {
            Ok(prepared) => {
                workspace.drift_read.failed = None;
                workspace.drifted(asked, prepared);
            }
            Err(why) => workspace.drift_read.failed = Some((asked.0, asked.1, why)),
        }
    })
}

/// The worktree's change read again in place of the drift.
pub(super) fn leave(state: &mut AppState, spawner: &dyn Spawner) {
    state.workspace.drift_read = Default::default();
    if state.workspace.undrift() {
        super::diff::reread(state, spawner);
    }
}
