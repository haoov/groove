//! A maintenance task: marked by the user, spawning children.

use groove_types::{SessionId, SessionKind, Timestamp};

use super::Command;
use super::open::{Start, begun_with};
use super::rail::listed;
use crate::asker::Asker;
use crate::spawn::record;
use crate::{AppState, Services, Spawner};

const NOT_MAINTENANCE: &str =
    "only a task marked as maintenance spawns child sessions";

/// What a new child is: its title, its brief, and whether it takes the selection.
#[derive(Debug, Default)]
pub(crate) struct Child {
    pub title: Option<String>,
    pub brief: Option<String>,
    pub beside: bool,
}

/// The maintenance actions, then the clusters.
pub(super) fn dispatch(
    command: Command,
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
) {
    match command {
        Command::SetMaintenance { session, on } => {
            set_maintenance(state, services, spawner, &session, on)
        }
        Command::SpawnChild { parent } => {
            let child = Child::default();
            spawn_child(state, services, spawner, &parent, child, Asker::Ui)
        }
        other => super::clusters::dispatch(other, state, services, spawner),
    }
}

/// The task marked as maintenance or no longer; any other kind is refused.
pub fn set_maintenance(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    id: &SessionId,
    on: bool,
) {
    let Some(open) = state.session.get_mut(id) else {
        return;
    };
    if !matches!(open.session.kind, SessionKind::Task { .. }) {
        return state.failed(groove_types::Error::invalid(
            "only a task is marked as maintenance",
        ));
    }
    open.state.maintenance = on;
    let (service, id) = (services.session.clone(), id.clone());
    record(
        spawner,
        async move { service.set_maintenance(&id, on).await },
    );
}

/// An explorer spawned from a maintenance task, its agent started on the brief.
pub(crate) fn spawn_child(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    parent: &SessionId,
    child: Child,
    asker: Asker,
) {
    let Some(title) = maintained(state, parent) else {
        return asker.refused(state, NOT_MAINTENANCE);
    };
    let now = Timestamp::now();
    let session = groove_session_service::explorer(child.title.as_deref(), now);
    let start = Start {
        prompt: child.brief.as_deref().map(|brief| briefed(&title, brief)),
        beside: child.beside,
    };
    let auto = begun_with(state, spawner, session.clone(), now, start);
    if let Some(open) = state.session.get_mut(&session.id) {
        open.state.parent = Some(parent.clone());
    }
    if let Some(open) = state.session.get_mut(parent) {
        open.children.push(session.clone());
    }
    let said = format!("spawned {} as {}", session.title, session.id);
    let (service, parent) = (services.session.clone(), parent.clone());
    listed(spawner, async move {
        service.create_child(&session, &parent, now).await?;
        service.set_auto_approve(&session.id, auto).await
    });
    asker.done(|| said);
}

/// The parent's title, when it is on the rail and marked as maintenance.
fn maintained(state: &AppState, parent: &SessionId) -> Option<String> {
    let open = state.session.get(parent)?;
    open.state
        .maintenance
        .then(|| open.session.title.clone())
}

/// The child's first message: the brief, and what it does once the work is done.
fn briefed(parent: &str, brief: &str) -> String {
    format!(
        "Brief from {parent}:\n\n{brief}\n\nWhen the work is done, call log_work with what \
         you did, why, and the MR link, before the session closes."
    )
}
