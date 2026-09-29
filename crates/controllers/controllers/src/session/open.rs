//! A session coming and going: back at start, new, picked, closed.

use groove_types::{Session, SessionId, Timestamp};

use super::FIRST_SIZE;
use super::rail::{list, listed};
use crate::spawn::record;
use crate::{AppState, Continuation, Services, Spawner, agent};

/// Reads the opened rows; the continuation puts them on the rail, loads each, starts each agent.
pub fn restore(services: &Services, spawner: &dyn Spawner) {
    let service = services.session.clone();
    spawner.spawn(Box::pin(async move {
        let result = service.opened().await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                let rows = match result {
                    Ok(rows) => rows,
                    Err(e) => return state.failed(e),
                };
                let last_seen = rows
                    .iter()
                    .max_by_key(|(_, st)| st.seen_at)
                    .map(|(s, _)| s.id.clone());
                for (session, session_state) in rows {
                    let id = session.id.clone();
                    state.session.restore(session, session_state);
                    load_contents(services, spawner, &id);
                    agent::start(state, spawner, id, FIRST_SIZE);
                }
                state.session.selected = last_seen;
                super::feed::read(state, services, spawner);
                crate::task::follow(state, spawner);
            },
        ) as Continuation
    }));
}

/// The session's recorded repos and worktrees into its row.
fn load_contents(services: &Services, spawner: &dyn Spawner, id: &SessionId) {
    let (service, id) = (services.session.clone(), id.clone());
    spawner.spawn(Box::pin(async move {
        let result = service.contents(&id).await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                match result {
                    Ok(contents) => {
                        if let Some(open) = state.session.get_mut(&id) {
                            open.repos = contents.repos;
                            open.worktrees = contents.worktrees;
                            open.status = contents.status;
                            for (worktree, path) in contents.read {
                                open.mark(&worktree, &path, true);
                            }
                            if open.selected_worktree().is_none() {
                                open.state.selected_worktree =
                                    open.worktrees.first().map(|w| w.id.clone());
                            }
                        }
                    }
                    Err(e) => state.failed(e),
                }
                crate::workspace::follow(state, spawner);
                crate::delivery::known(services, spawner);
            },
        ) as Continuation
    }));
}

/// A new session on the rail, its agent started; returns whether its writes run unasked.
pub(crate) fn begun(
    state: &mut AppState,
    spawner: &dyn Spawner,
    session: Session,
    now: Timestamp,
) -> bool {
    let id = session.id.clone();
    let auto = state.config.auto_approve_default();
    state.session.open(session, now);
    if let Some(open) = state.session.get_mut(&id) {
        open.state.auto_approve = auto;
    }
    crate::workspace::follow(state, spawner);
    agent::start(state, spawner, id, FIRST_SIZE);
    auto
}

pub fn open_explorer(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    title: Option<&str>,
) {
    let now = Timestamp::now();
    let session = groove_session_service::explorer(title, now);
    let auto = begun(state, spawner, session.clone(), now);
    let service = services.session.clone();
    listed(spawner, async move {
        service.create_explorer(&session, now).await?;
        service.set_auto_approve(&session.id, auto).await
    });
}

/// A session picked: selected when on the rail, brought back with its agent when closed.
pub fn open(state: &mut AppState, services: &Services, spawner: &dyn Spawner, id: &SessionId) {
    if state.session.get(id).is_some() {
        return select(state, services, spawner, id);
    }
    let now = Timestamp::now();
    let Some(living) = state
        .session
        .living
        .iter()
        .find(|living| living.session.id == *id)
        .cloned()
    else {
        return;
    };
    state.session.open(living.session, now);
    super::feed::read(state, services, spawner);
    load_contents(services, spawner, id);
    agent::start(state, spawner, id.clone(), FIRST_SIZE);
    select(state, services, spawner, id);
    let (service, at) = (services.session.clone(), id.clone());
    record(
        spawner,
        async move { service.set_opened(&at, Some(now)).await },
    );
}

pub fn select(state: &mut AppState, services: &Services, spawner: &dyn Spawner, id: &SessionId) {
    let now = Timestamp::now();
    state.session.select(id, now);
    state.agent.saw(id, now);
    crate::workspace::follow(state, spawner);
    crate::task::follow(state, spawner);
    let (service, id) = (services.session.clone(), id.clone());
    record(spawner, async move { service.set_seen(&id, now).await });
}

pub fn close(state: &mut AppState, services: &Services, spawner: &dyn Spawner, id: &SessionId) {
    agent::end(state, id);
    crate::shell::end(state, id);
    state.session.close(id);
    crate::workspace::follow(state, spawner);
    let (service, at) = (services.session.clone(), id.clone());
    record(spawner, async move { service.set_opened(&at, None).await });
    list(services, spawner);
}
