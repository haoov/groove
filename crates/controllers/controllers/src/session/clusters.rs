//! A cluster context attached to a session, on a namespace or the whole cluster, or let go.

use groove_types::{Attached, Error, SessionId, Timestamp};

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
    if state.config.cluster(&attached.context).is_none() {
        let why = format!(
            "Groove does not know the context `{}`: add it in Settings › Clusters first",
            attached.context
        );
        return state.failed(Error::invalid(why));
    }
    let attached = Attached {
        namespace: attached.namespace.filter(|one| !one.trim().is_empty()),
        ..attached
    };
    let Some(open) = state.session.get_mut(session) else {
        return state.failed(Error::not_found(format!("no open session {session}")));
    };
    let replaced = match open.attach(attached.clone()) {
        Ok(replaced) => replaced,
        Err(e) => return state.failed(e),
    };
    let (service, id, now) = (services.session.clone(), session.clone(), Timestamp::now());
    record(spawner, async move {
        for one in &replaced {
            service.detach_cluster(&id, one).await?;
        }
        service.attach_cluster(&id, &attached, now).await
    });
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
    let (service, id) = (services.session.clone(), session.clone());
    record(spawner, async move {
        service.detach_cluster(&id, &attached).await
    });
}
