//! A maintenance task: marked by the user, spawning children.

use groove_session_service::ClusterWrite;
use groove_types::{Alert, AlertChild, Attached, SessionId, SessionKind, Timestamp};

use super::Command;
use super::open::{Start, begun_with};
use super::rail::listed;
use crate::asker::Asker;
use crate::spawn::record;
use crate::{AppState, Services, Spawner};

const NOT_MAINTENANCE: &str = "only a task marked as maintenance spawns child sessions";

/// What a new child is: its title, its brief, and whether it takes the selection.
#[derive(Debug, Default)]
pub(crate) struct Child {
    pub title: Option<String>,
    pub brief: Option<String>,
    pub beside: bool,
    /// The skill its first message invokes, with the brief as its arguments.
    pub skill: Option<&'static str>,
    pub alert: Option<Alert>,
    pub attached: Option<Attached>,
}

/// The maintenance and log actions, then the clusters.
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
            spawn_child(state, services, spawner, &parent, child, Asker::Ui);
        }
        Command::RemoveLogEntry { session, entry } => {
            super::log::remove(state, services, spawner, &session, entry)
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
    crate::alert::poll(state, services, spawner, Timestamp::now());
    let (service, id) = (services.session.clone(), id.clone());
    record(
        spawner,
        async move { service.set_maintenance(&id, on).await },
    );
}

/// An explorer spawned from a maintenance task, its agent started on the brief; its id.
pub(crate) fn spawn_child(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    parent: &SessionId,
    child: Child,
    asker: Asker,
) -> Option<SessionId> {
    let Some(title) = maintained(state, parent) else {
        asker.refused(state, NOT_MAINTENANCE);
        return None;
    };
    let now = Timestamp::now();
    let session = groove_session_service::explorer(child.title.as_deref(), now);
    let prompt = child.brief.as_deref().map(|brief| briefed(&title, brief));
    let prompt = match child.skill {
        Some(skill) => prompt.map(|said| format!("/{skill} {said}")),
        None => prompt,
    };
    let start = Start {
        prompt,
        beside: child.beside,
    };
    let auto = begun_with(state, spawner, session.clone(), now, start);
    let alerted = child
        .alert
        .as_ref()
        .map(|one| alert_child(one, &session.id, parent));
    let writes = held(
        state,
        &session.id,
        parent,
        (&session, &alerted),
        child.attached,
    );
    let said = format!("spawned {} as {}", session.title, session.id);
    let (service, parent, id) = (services.session.clone(), parent.clone(), session.id.clone());
    listed(spawner, async move {
        service
            .create_child(&session, &parent, alerted.as_ref(), now)
            .await?;
        service.hold_clusters(&session.id, writes);
        service.flush_clusters().await?;
        service.set_auto_approve(&session.id, auto).await
    });
    asker.done(|| said);
    Some(id)
}

/// The child under its parent on the rail, the cluster it holds; the writes that cluster needs.
fn held(
    state: &mut AppState,
    id: &SessionId,
    parent: &SessionId,
    (session, alerted): (&groove_types::Session, &Option<AlertChild>),
    attached: Option<Attached>,
) -> Vec<ClusterWrite> {
    let mut writes = Vec::new();
    if let Some(open) = state.session.get_mut(id) {
        open.state.parent = Some(parent.clone());
        if let Some(attached) = attached {
            open.attach(attached.clone());
            writes.push(ClusterWrite::Attach(attached, Timestamp::now()));
        }
    }
    if let Some(open) = state.session.get_mut(parent) {
        open.children.push(session.clone());
        open.alerted.extend(alerted.clone());
    }
    writes
}

/// The alert a child is spawned for, as the parent remembers it.
fn alert_child(one: &Alert, session: &SessionId, parent: &SessionId) -> AlertChild {
    AlertChild {
        session: session.clone(),
        parent: parent.clone(),
        dedup_key: one.dedup_key.clone(),
        title: one.title.clone(),
        priority: one.priority.clone(),
        cluster: one.cluster.clone(),
        namespace: one.namespace.clone(),
    }
}

/// The parent's title, when it is on the rail and marked as maintenance.
fn maintained(state: &AppState, parent: &SessionId) -> Option<String> {
    let open = state.session.get(parent)?;
    open.state.maintenance.then(|| open.session.title.clone())
}

/// The child's first message: the brief, under its parent's title.
fn briefed(parent: &str, brief: &str) -> String {
    format!("Brief from {parent}:\n\n{brief}")
}
