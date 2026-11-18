//! Writes to a session's clusters: checked against the session and the context, said on the timeline once stored.

use groove_cluster_service::Allowed;
use groove_types::{Result, SessionId, Target, TimelineKind, Timestamp};
use serde_json::Value;

use crate::asker::Asker;
use crate::{AppState, Continuation, Services, Spawner};

/// A merge patch on one object, made on the object at `version` when one is given.
#[derive(Debug, Clone, PartialEq)]
pub struct Patched {
    pub target: Target,
    pub body: Value,
    pub version: Option<String>,
}

pub(crate) fn patch(
    state: &mut AppState,
    spawner: &dyn Spawner,
    session: SessionId,
    patched: Patched,
    asker: Asker,
) {
    let said = format!("patch {}", patched.body);
    stored((state, spawner), (session, said), patched, asker);
}

/// The workload's pods replaced, as `kubectl rollout restart` does.
pub(crate) fn restart(
    state: &mut AppState,
    spawner: &dyn Spawner,
    session: SessionId,
    target: Target,
) {
    let body = match groove_cluster_service::restart(&target.kind, Timestamp::now()) {
        Ok(body) => body,
        Err(e) => return Asker::Ui.failed(state, e),
    };
    let patched = Patched {
        target,
        body,
        version: None,
    };
    let said = (session, "restart".to_string());
    stored((state, spawner), said, patched, Asker::Ui);
}

/// The patch stored once the session may write there, `said` on its timeline.
fn stored(
    (state, spawner): (&mut AppState, &dyn Spawner),
    (session, said): (SessionId, String),
    patched: Patched,
    asker: Asker,
) {
    let checked = allowed(state, &session, &patched.target.context).and_then(|allowed| {
        groove_cluster_service::writable(&patched.target, allowed.read_only, &allowed.held)
    });
    if let Err(e) = checked {
        return asker.failed(state, e);
    }
    let paths = state.env.kubeconfig.clone();
    spawner.spawn(Box::pin(async move {
        let Patched {
            target,
            body,
            version,
        } = patched;
        let subject = format!("{} · {said}", target.label());
        let stored = groove_cluster_service::patch(paths, target, body, version, false).await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| match stored {
                Ok(_) => {
                    let kind = TimelineKind::ClusterWrite;
                    crate::timeline::said(services, spawner, &session, kind, subject.clone());
                    asker.done(|| format!("patched {subject}"));
                }
                Err(e) => asker.failed(state, e),
            },
        ) as Continuation
    }));
}

/// What `session` may write on `context`: a context Groove knows, and the clusters the session holds.
pub(crate) fn allowed(state: &AppState, session: &SessionId, context: &str) -> Result<Allowed> {
    let read_only = state.config.known_cluster(context)?.read_only;
    let open = state
        .session
        .get(session)
        .ok_or_else(|| groove_types::Error::not_found(crate::tools::NO_SESSION))?;
    Ok(Allowed {
        read_only,
        held: open.clusters.clone(),
    })
}
