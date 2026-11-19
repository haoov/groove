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
    stored((state, spawner), (session, said), changed(patched), asker);
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
    stored((state, spawner), said, changed(patched), Asker::Ui);
}

/// The workload's replicas set to `replicas`, as `kubectl scale` does.
pub(crate) fn scale(
    state: &mut AppState,
    spawner: &dyn Spawner,
    (session, mut target): (SessionId, Target),
    replicas: u32,
) {
    let body = match groove_cluster_service::scale(&mut target, replicas) {
        Ok(body) => body,
        Err(e) => return Asker::Ui.failed(state, e),
    };
    let patched = Patched {
        target,
        body,
        version: None,
    };
    let said = (session, format!("scale to {replicas}"));
    stored((state, spawner), said, changed(patched), Asker::Ui);
}

fn changed(patched: Patched) -> (Target, Change, Option<String>) {
    let Patched {
        target,
        body,
        version,
    } = patched;
    (target, Change::Patch(body), version)
}

/// What a write does to its object; an edit ends the drafts of its object once it lands.
enum Change {
    Patch(Value),
    Edit(Value),
    Delete,
}

/// The user's YAML edit, made on the version its draft started from.
pub(crate) fn edit(
    state: &mut AppState,
    spawner: &dyn Spawner,
    session: SessionId,
    edited: Patched,
) {
    let said = (session, format!("edit {}", edited.body));
    let Patched {
        target,
        body,
        version,
    } = edited;
    let change = (target, Change::Edit(body), version);
    stored((state, spawner), said, change, Asker::Ui);
}

/// The object deleted, on `version` when one is given.
pub(crate) fn delete(
    (state, spawner): (&mut AppState, &dyn Spawner),
    (session, target): (SessionId, Target),
    version: Option<String>,
    asker: Asker,
) {
    let said = (session, "delete".to_string());
    let change = (target, Change::Delete, version);
    stored((state, spawner), said, change, asker);
}

/// The change made once the session may write there, `said` on its timeline.
fn stored(
    (state, spawner): (&mut AppState, &dyn Spawner),
    (session, said): (SessionId, String),
    (target, change, version): (Target, Change, Option<String>),
    asker: Asker,
) {
    let checked = allowed(state, &session, &target.context).and_then(|allowed| {
        groove_cluster_service::writable(&target, allowed.read_only, &allowed.held)
    });
    if let Err(e) = checked {
        return asker.failed(state, e);
    }
    let paths = state.env.kubeconfig.clone();
    spawner.spawn(Box::pin(async move {
        let subject = format!("{} · {said}", target.label());
        let landed = matches!(change, Change::Edit(_)).then(|| target.clone());
        let done = match change {
            Change::Patch(body) | Change::Edit(body) => {
                let patched = groove_cluster_service::patch(paths, target, body, version, false);
                patched.await.map(drop)
            }
            Change::Delete => groove_cluster_service::delete(paths, target, version).await,
        };
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| match done {
                Ok(()) => {
                    if let Some(target) = &landed {
                        state.cluster.store.follows.yamls.landed(target);
                    }
                    let kind = TimelineKind::ClusterWrite;
                    crate::timeline::said(services, spawner, &session, kind, subject.clone());
                    asker.done(|| format!("done: {subject}"));
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
