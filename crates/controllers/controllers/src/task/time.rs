//! The clock on the task being worked, and the hours it hands the source.

use groove_task_service::working;
use groove_types::{AgentStatus, ExternalId, TaskKey, Timestamp, hours};

use crate::asker::Asker;
use crate::{AppState, Continuation, Services, Spawner};

/// How often the ledger takes what the clock measured.
const WRITE: i64 = 60;

/// The clock on the task being worked; the ledger takes what it measured every `WRITE` seconds.
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
                Ok(time) => state.task.timed(time),
                Err(e) => state.failed(e),
            },
        ) as Continuation
    }));
}

/// The task the window is working: the selected session's, while the user is there.
fn worked(state: &AppState, now: Timestamp) -> Option<ExternalId> {
    let open = state.session.selected()?;
    let task = open.session.kind.task()?;
    let busy = state
        .agent
        .activity(&open.session.id)
        .is_some_and(|activity| activity.status == AgentStatus::Working);
    working(state.focused, state.acted_at, busy, now).then(|| task.clone())
}

/// The hours the clock measured, to the source and then to the ledger.
pub(crate) fn log_hours(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    id: &ExternalId,
    asker: Asker,
) {
    let Some(time) = state.task.measured(id) else {
        return asker.refused("nothing has been measured for it yet");
    };
    let seconds = time.unlogged_seconds;
    if seconds <= 0 {
        return asker.done(|| "the source already has every hour of it".into());
    }
    let key = match TaskKey::parse(id) {
        Ok(key) => key,
        Err(e) => return asker.failed(state, e),
    };
    let sources = state.task.sources(state.config.config.as_ref());
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
                let time = match read {
                    Ok(time) => time,
                    Err(e) => return asker.failed(state, e),
                };
                state.task.timed(time);
                super::sync(state, spawner, key);
                asker.done(|| format!("logged {}h", hours(seconds)));
            },
        ) as Continuation
    }));
}
