//! Taking a session away: its agent, its worktrees, its row, and only then the rest.

use groove_types::{Error, SessionId};

use super::FIRST_SIZE;
use super::rail::list;
use crate::{AppState, Continuation, Services, Spawner, agent};

/// The session taken away: its agent, its worktrees, its row; unforced, lost work refuses it.
pub fn delete(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    id: &SessionId,
    force: bool,
) {
    let running = state.agent.agent(id).is_some();
    agent::end(state, id);
    crate::shell::end(state, id);
    let title = state
        .session
        .get(id)
        .map(|o| o.session.title.clone())
        .unwrap_or_default();
    let pending = state.begin(format!("deleting {title}"));
    let (service, at) = (services.session.clone(), id.clone());
    spawner.spawn(Box::pin(async move {
        let removed = service.remove(&at, force).await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                state.end(pending);
                match removed {
                    Ok(()) => gone(state, spawner, &at),
                    Err(e) => kept(state, spawner, &at, running, e),
                }
                list(services, spawner);
            },
        ) as Continuation
    }));
}

/// The session off the rail, with its launch files and its lines.
fn gone(state: &mut AppState, spawner: &dyn Spawner, id: &SessionId) {
    agent::forget(state, id);
    state.session.feed.retain(|line| &line.session != id);
    if let Some(closed) = state.session.close(id) {
        for worktree in &closed.worktrees {
            state.workspace.forget(&worktree.id);
        }
    }
    crate::workspace::follow(state, spawner);
}

/// A refused delete leaves the session as it was, its agent started again.
fn kept(state: &mut AppState, spawner: &dyn Spawner, id: &SessionId, running: bool, e: Error) {
    state.failed(e);
    if running {
        agent::start(state, spawner, id.clone(), FIRST_SIZE);
    }
}

/// The session and its worktrees gone from this machine; a dirty one is refused.
pub fn delete_local(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    id: &SessionId,
) {
    delete(state, services, spawner, id, false);
    crate::task::load(state, services, spawner);
}

/// Uncommitted and unpushed work goes with it.
pub fn force_delete(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    id: &SessionId,
) {
    delete(state, services, spawner, id, true);
    crate::task::load(state, services, spawner);
}
