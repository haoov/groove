//! Writes to a session's clusters: checked against the session and the context, said on the timeline once stored.

use groove_types::{Result, SessionId, Target, TimelineKind};
use serde_json::Value;

use crate::{AppState, Continuation, Services, Spawner};

/// A merge patch on one object, made on the object at `version` when one is given.
#[derive(Debug, Clone, PartialEq)]
pub struct Patched {
    pub target: Target,
    pub body: Value,
    pub version: Option<String>,
}

pub(super) fn patch(
    state: &mut AppState,
    spawner: &dyn Spawner,
    session: SessionId,
    patched: Patched,
) {
    if let Err(e) = writable(state, &session, &patched.target) {
        return state.failed(e);
    }
    let paths = state.env.kubeconfig.clone();
    spawner.spawn(Box::pin(async move {
        let Patched {
            target,
            body,
            version,
        } = patched;
        let subject = format!("{} · patch {body}", target.label());
        let stored = groove_cluster_service::patch(paths, target, body, version, false).await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| match stored {
                Ok(_) => crate::timeline::said(
                    services,
                    spawner,
                    &session,
                    TimelineKind::ClusterWrite,
                    subject,
                ),
                Err(e) => state.failed(e),
            },
        ) as Continuation
    }));
}

fn writable(state: &AppState, session: &SessionId, target: &Target) -> Result<()> {
    let read_only = state.config.known_cluster(&target.context)?.read_only;
    let open = state
        .session
        .get(session)
        .ok_or_else(|| groove_types::Error::not_found(crate::tools::NO_SESSION))?;
    groove_cluster_service::writable(target, read_only, &open.clusters)
}
