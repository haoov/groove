//! The `alert` controller: incident.io's firing alerts, read on the clock, and a child per alert.

use groove_types::{Alert, Attached, SessionId, Timestamp};

use crate::asker::Asker;
use crate::session::Child;
use crate::{AppState, Continuation, Services, Spawner};

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// `alert.read`: the firing alerts read again, whatever the clock says.
    Read,
    /// `alert.investigate`: the child of a firing alert opened, or spawned on the alert.
    Investigate {
        parent: SessionId,
        dedup_key: String,
    },
}

impl Command {
    pub fn id(&self) -> &'static str {
        match self {
            Command::Read => "alert.read",
            Command::Investigate { .. } => "alert.investigate",
        }
    }
}

pub fn dispatch(
    command: Command,
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
) {
    match command {
        Command::Read => read(state, services, spawner, Timestamp::now()),
        Command::Investigate { parent, dedup_key } => {
            investigate(state, services, spawner, &parent, &dedup_key)
        }
    }
}

/// Whether the window should keep waking for the alerts: a maintenance task is on the rail.
pub fn polls(state: &AppState) -> bool {
    state.session.open.iter().any(|open| open.state.maintenance)
}

/// One tick: the alerts read when they are due.
pub fn poll(state: &mut AppState, services: &Services, spawner: &dyn Spawner, now: Timestamp) {
    if polls(state) && state.alert.due(now, state.config.alerts_poll()) {
        read(state, services, spawner, now);
    }
}

fn read(state: &mut AppState, services: &Services, spawner: &dyn Spawner, now: Timestamp) {
    state.alert.sent(now);
    let (config, service) = (state.config.alerts(), services.alert.clone());
    spawner.spawn(Box::pin(async move {
        let answered = service.read(&config).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.alert.read(answered, Timestamp::now());
        }) as Continuation
    }));
}

/// The child the parent spawned for this alert, opened; else a new one on the alert.
pub fn investigate(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    parent: &SessionId,
    dedup_key: &str,
) {
    let spawned = state.session.get(parent).and_then(|open| {
        let one = open.alerted.iter().find(|one| one.dedup_key == dedup_key)?;
        Some(one.session.clone())
    });
    if let Some(child) = spawned {
        return crate::session::open(state, services, spawner, &child);
    }
    let Some(alert) = state.alert.find(dedup_key).cloned() else {
        return state.failed(groove_types::Error::not_found(format!(
            "no firing alert {dedup_key}"
        )));
    };
    let child = Child {
        title: Some(alert.title.clone()),
        brief: Some(brief(&alert)),
        beside: false,
        skill: Some("groove:investigate"),
        attached: attached(state, &alert),
        alert: Some(alert),
    };
    crate::session::spawn_child(state, services, spawner, parent, child, Asker::Ui);
}

/// The alert's cluster and namespace, when Settings knows the cluster.
fn attached(state: &AppState, alert: &Alert) -> Option<Attached> {
    let context = alert.cluster.clone()?;
    state.config.known_cluster(&context).ok()?;
    Some(Attached {
        context,
        namespace: alert.namespace.clone(),
    })
}

/// What the child starts from: the alert, where it fires, and where to read more.
fn brief(alert: &Alert) -> String {
    let priority = alert.priority.as_deref().unwrap_or("no priority");
    let place = match (&alert.cluster, &alert.namespace) {
        (Some(cluster), Some(namespace)) => format!("{cluster} / {namespace}"),
        (Some(cluster), None) => cluster.clone(),
        _ => "no cluster named".to_string(),
    };
    let mut out = format!("{} is firing ({priority}) on {place}.", alert.title);
    if let Some(url) = &alert.source_url {
        out.push_str(&format!("\nAlert: {url}"));
    }
    if !alert.description.trim().is_empty() {
        out.push_str(&format!("\n\n{}", alert.description.trim()));
    }
    out
}
