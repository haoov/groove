//! The clock on the task being worked, and the hours it hands the source.

use groove_task_service::{IDLE, sources};
use groove_types::{AgentStatus, ExternalId, SessionKind, TaskKey, Timestamp, hours};

use crate::{AppState, Continuation, Services, Spawner};

/// How often the ledger takes what the clock measured.
const WRITE: i64 = 60;

/// The clock: it credits the task of the session being worked, and the ledger takes
/// what it measured every `WRITE` seconds.
pub fn tick(state: &mut AppState, services: &Services, spawner: &dyn Spawner, now: Timestamp) {
    state.task.timer.on(worked(state, now), now);
    if !state.task.timer.due(now, WRITE) {
        return;
    }
    super::attention::reread(state, now);
    let owed = state.task.timer.taken(now);
    if owed.is_empty() {
        return;
    }
    let (service, day) = (services.task.clone(), now.day());
    spawner.spawn(Box::pin(async move {
        let wrote = service.credit(owed, day).await;
        let read = match wrote {
            Ok(()) => service.time().await,
            Err(e) => Err(e),
        };
        Box::new(
            move |state: &mut AppState, _: &Services, _: &dyn Spawner| match read {
                Ok(time) => state.task.time = time.into_iter().collect(),
                Err(e) => state.failed(e),
            },
        ) as Continuation
    }));
}

/// The task the window is working: the selected session's, while the user is there.
fn worked(state: &AppState, now: Timestamp) -> Option<ExternalId> {
    if !state.focused {
        return None;
    }
    let open = state.session.selected()?;
    let SessionKind::Task { external_id } = &open.session.kind else {
        return None;
    };
    let acted = now.seconds() - state.acted_at.seconds() <= IDLE;
    let busy = state
        .agent
        .agent(&open.session.id)
        .is_some_and(|agent| agent.activity.status == AgentStatus::Working);
    (acted || busy).then(|| external_id.clone())
}

/// The hours the clock measured, to the source and then to the ledger.
pub(super) fn log_hours(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    id: &ExternalId,
) {
    let Some(time) = state.task.measured(id) else {
        return;
    };
    let seconds = time.unlogged_seconds;
    if seconds <= 0 {
        return;
    }
    let key = match TaskKey::parse(id) {
        Ok(key) => key,
        Err(e) => return state.failed(e),
    };
    let sources = sources(state.config.config.as_ref());
    let (service, id) = (services.task.clone(), id.clone());
    let job = state.begin(format!("logging {}h to {id}", hours(seconds)));
    spawner.spawn(Box::pin(async move {
        let wrote = groove_task_service::log_hours(&sources, &key, hours(seconds)).await;
        let read = match wrote {
            Ok(_) => service.logged(&id, seconds).await.and(service.time().await),
            Err(e) => Err(e),
        };
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                state.end(job);
                match read {
                    Ok(time) => state.task.time = time.into_iter().collect(),
                    Err(e) => return state.failed(e),
                }
                super::sync(state, spawner, key);
            },
        ) as Continuation
    }));
}
