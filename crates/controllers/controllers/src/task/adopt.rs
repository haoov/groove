//! An explorer promoted to the session of the task its agent filed, on the same conversation.

use std::time::Duration;

use groove_task_service::Fetched;
use groove_types::{Session, SessionId, SessionKind, StatusIntent, TaskKey, Timestamp, Worktree};

use crate::asker::Asker;
use crate::{AppState, Continuation, Services, Spawner};

pub(crate) const NOT_EXPLORER: &str = "only an explorer is made the session of a task";

/// How long the agent has to take the answer in before it is stopped.
#[cfg(not(test))]
const GRACE: Duration = Duration::from_millis(1500);
#[cfg(test)]
const GRACE: Duration = Duration::from_millis(10);

/// The task read at its source, then this explorer promoted to the session that works it.
pub fn adopt(
    state: &mut AppState,
    spawner: &dyn Spawner,
    session: &SessionId,
    key: TaskKey,
    asker: Asker,
) {
    let explorer = state
        .session
        .get(session)
        .is_some_and(|open| matches!(open.session.kind, SessionKind::Explorer));
    if !explorer {
        return asker.refused(NOT_EXPLORER);
    }
    let sources = state.task.sources(state.config.config.as_ref());
    let job = state.begin(format!("reading {}", key.external_id()));
    let session = session.clone();
    spawner.spawn(Box::pin(async move {
        let read = groove_task_service::fetch(&sources, &key).await;
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                state.end(job);
                match read {
                    Ok(read) => told(state, spawner, session, read, asker),
                    Err(e) => asker.failed(state, e),
                }
            },
        ) as Continuation
    }));
}

/// The agent told what is about to happen, then given the time to take it in.
fn told(
    state: &mut AppState,
    spawner: &dyn Spawner,
    explorer: SessionId,
    read: Fetched,
    asker: Asker,
) {
    if let Some(other) = super::working(state, &read.task).filter(|one| *one != explorer) {
        return asker.refused(format!("{other} already works {}", read.task.short_id));
    }
    let (short, title) = (read.task.short_id.clone(), read.task.title.clone());
    asker.done(|| {
        format!(
            "this session becomes {short}: {title}. Groove stops you now, moves the \
             worktrees and starts you again on this same conversation. Stop here."
        )
    });
    spawner.spawn(Box::pin(async move {
        tokio::time::sleep(GRACE).await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                promoted(state, services, spawner, explorer, read);
            },
        ) as Continuation
    }));
}

/// The agent stopped, then the worktrees moved and the rows handed over.
fn promoted(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    explorer: SessionId,
    read: Fetched,
) {
    let Some(worktrees) = state
        .session
        .get(&explorer)
        .map(|open| open.worktrees.clone())
    else {
        return;
    };
    crate::agent::end(state, &explorer);
    crate::shell::end(state, &explorer);
    let (service, task) = (services.session.clone(), read.task.clone());
    let job = state.begin(format!("making {explorer} {}", task.short_id));
    spawner.spawn(Box::pin(async move {
        let done = service
            .promote(&explorer, &worktrees, &task, Timestamp::now())
            .await;
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                state.end(job);
                match done {
                    Ok((session, worktrees)) => {
                        became(state, spawner, &explorer, session, worktrees, read)
                    }
                    Err(e) => {
                        state.failed(e);
                        crate::agent::start(state, spawner, explorer, crate::session::FIRST_SIZE);
                    }
                }
            },
        ) as Continuation
    }));
}

/// The rail's row is the task's now, and its agent starts on the explorer's conversation.
fn became(
    state: &mut AppState,
    spawner: &dyn Spawner,
    explorer: &SessionId,
    session: Session,
    worktrees: Vec<Worktree>,
    read: Fetched,
) {
    let launch_dir = crate::agent::launch_dir(state);
    if let Err(e) =
        groove_agent_service::hand_over(&launch_dir, explorer.as_str(), session.id.as_str())
    {
        state.failed(e);
    }
    crate::agent::forget(state, explorer);
    let id = session.id.clone();
    state.session.promoted(explorer, session, worktrees);
    let external_id = read.task.external_id.clone();
    state.task.synced(read);
    super::status::set(state, spawner, &external_id, StatusIntent::InProgress);
    crate::workspace::follow(state, spawner);
    crate::agent::start(state, spawner, id, crate::session::FIRST_SIZE);
}
