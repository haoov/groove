//! The rail's own rows: what opens a session, what closes it, what it remembers.

use groove_types::{Error, SessionId, Timestamp};

use super::{FIRST_SIZE, NO_PENDING};
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
                    Err(e) => return state.errors.push(e),
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
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                match result {
                    Ok(contents) => {
                        if let Some(open) = state.session.get_mut(&id) {
                            open.repos = contents.repos;
                            open.worktrees = contents.worktrees;
                            open.delivery = contents.delivery;
                            for (worktree, path) in contents.read {
                                open.mark(&worktree, &path, true);
                            }
                            if open.selected_worktree().is_none() {
                                open.state.selected_worktree =
                                    open.worktrees.first().map(|w| w.id.clone());
                            }
                        }
                    }
                    Err(e) => state.errors.push(e),
                }
                crate::workspace::follow(state, spawner);
            },
        ) as Continuation
    }));
}

pub fn open_explorer(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    title: Option<&str>,
) {
    let now = Timestamp::now();
    let session = groove_session_service::explorer(title, now);
    let id = session.id.clone();
    state.session.open(session.clone(), now);
    crate::workspace::follow(state, spawner);
    agent::start(state, spawner, id, FIRST_SIZE);
    let service = services.session.clone();
    record(spawner, NO_PENDING, async move {
        service.create_explorer(&session, now).await
    });
    list(services, spawner);
}

/// What git says about the selected worktree, read again for the overview.
pub fn refresh_status(state: &mut AppState, services: &Services, spawner: &dyn Spawner) {
    let Some(open) = state.session.selected() else {
        return;
    };
    let Some(worktree) = open.selected_worktree().cloned() else {
        return;
    };
    let session = open.session.id.clone();
    let service = services.session.clone();
    spawner.spawn(Box::pin(async move {
        let status = service.status(&worktree).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            let Ok(status) = status else { return };
            if let Some(open) = state.session.get_mut(&session) {
                open.told(&worktree.id, status);
            }
        }) as Continuation
    }));
}

pub fn rename_explorer(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    id: &SessionId,
    title: &str,
) {
    state.session.rename(id, title);
    let (service, id, title) = (services.session.clone(), id.clone(), title.to_string());
    record(spawner, NO_PENDING, async move {
        service.rename_explorer(&id, &title).await
    });
}

pub fn delete(state: &mut AppState, services: &Services, spawner: &dyn Spawner, id: &SessionId) {
    agent::end(state, id);
    let title = state
        .session
        .get(id)
        .map(|o| o.session.title.clone())
        .unwrap_or_default();
    state.session.close(id);
    crate::workspace::follow(state, spawner);
    let pending = state.begin(format!("deleting {title}"));
    let (service, at) = (services.session.clone(), id.clone());
    record(spawner, pending, async move { service.remove(&at).await });
    list(services, spawner);
}

/// Every session that lives on disk, for the board's Live column.
pub fn list(services: &Services, spawner: &dyn Spawner) {
    let service = services.session.clone();
    spawner.spawn(Box::pin(async move {
        let read = service.living().await;
        Box::new(
            move |state: &mut AppState, _: &Services, _: &dyn Spawner| match read {
                Ok(living) => state.session.living = living,
                Err(e) => state.errors.push(e),
            },
        ) as Continuation
    }));
}

/// A session picked: the one on the rail is selected, one closed comes back to it
/// with its agent.
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
    load_contents(services, spawner, id);
    agent::start(state, spawner, id.clone(), FIRST_SIZE);
    select(state, services, spawner, id);
    let (service, at) = (services.session.clone(), id.clone());
    record(spawner, NO_PENDING, async move {
        service.set_opened(&at, Some(now)).await
    });
}

pub fn select(state: &mut AppState, services: &Services, spawner: &dyn Spawner, id: &SessionId) {
    let now = Timestamp::now();
    state.session.select(id, now);
    crate::workspace::follow(state, spawner);
    let (service, id) = (services.session.clone(), id.clone());
    record(spawner, NO_PENDING, async move {
        service.set_seen(&id, now).await
    });
}

pub fn close(state: &mut AppState, services: &Services, spawner: &dyn Spawner, id: &SessionId) {
    agent::end(state, id);
    state.session.close(id);
    crate::workspace::follow(state, spawner);
    let (service, at) = (services.session.clone(), id.clone());
    record(spawner, NO_PENDING, async move {
        service.set_opened(&at, None).await
    });
    list(services, spawner);
}

/// Writes the row's selected worktree to its leaf.
pub(super) fn persist_selection(
    state: &AppState,
    services: &Services,
    spawner: &dyn Spawner,
    id: &SessionId,
) {
    let selected = state
        .session
        .get(id)
        .and_then(|o| o.state.selected_worktree.clone());
    let (service, id) = (services.session.clone(), id.clone());
    record(spawner, NO_PENDING, async move {
        service.set_selected_worktree(&id, selected.as_ref()).await
    });
}

/// A write whose only result is success or an error for the feed, ending `pending` when it lands.
pub(crate) fn record(
    spawner: &dyn Spawner,
    pending: u64,
    write: impl Future<Output = Result<(), Error>> + Send + 'static,
) {
    spawner.spawn(Box::pin(async move {
        let result = write.await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(pending);
            if let Err(e) = result {
                state.errors.push(e);
            }
        }) as Continuation
    }));
}
