//! A cluster context attached to a session, on a namespace or the whole cluster, or let go.

use groove_types::{Attached, Error, SessionId, Timestamp};

use groove_session_service::ClusterWrite;

use super::Command;
use crate::spawn::record;
use crate::{AppState, Services, Spawner};

pub(super) fn dispatch(
    command: Command,
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
) {
    match command {
        Command::AttachCluster { session, attached } => {
            attach(state, services, spawner, &session, attached)
        }
        Command::DetachCluster { session, attached } => {
            detach(state, services, spawner, &session, attached)
        }
        _ => {}
    }
}

/// A context Groove knows, held by the open session at once and written behind.
pub fn attach(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    session: &SessionId,
    attached: Attached,
) {
    if let Err(e) = state.config.known_cluster(&attached.context) {
        return state.failed(e);
    }
    let attached = Attached {
        namespace: attached.namespace.filter(|one| !one.trim().is_empty()),
        ..attached
    };
    let Some(open) = state.session.get_mut(session) else {
        return state.failed(Error::not_found(format!("no open session {session}")));
    };
    let replaced = open.attach(attached.clone());
    let mut writes: Vec<ClusterWrite> = replaced.into_iter().map(ClusterWrite::Detach).collect();
    writes.push(ClusterWrite::Attach(attached, Timestamp::now()));
    written(services, spawner, session, writes);
}

pub fn detach(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    session: &SessionId,
    attached: Attached,
) {
    let Some(open) = state.session.get_mut(session) else {
        return state.failed(Error::not_found(format!("no open session {session}")));
    };
    open.detach(&attached);
    written(
        services,
        spawner,
        session,
        vec![ClusterWrite::Detach(attached)],
    );
}

/// `writes` held behind the ones asked before, then flushed in a job.
fn written(services: &Services, spawner: &dyn Spawner, id: &SessionId, writes: Vec<ClusterWrite>) {
    services.session.hold_clusters(id, writes);
    let service = services.session.clone();
    record(spawner, async move { service.flush_clusters().await });
}
