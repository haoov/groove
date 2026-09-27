//! What a session's row remembers, and the board's list of every session.

use groove_types::{Error, SessionId, Timestamp};

use crate::spawn::record;
use crate::{AppState, Continuation, Services, Spawner};

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
    record(spawner, async move {
        service.rename_explorer(&id, &title).await
    });
}

/// Every session that lives on disk, for the board's Live column.
pub fn list(services: &Services, spawner: &dyn Spawner) {
    let service = services.session.clone();
    spawner.spawn(Box::pin(async move {
        let read = service.living().await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            match read {
                Ok(living) => state.session.living = living,
                Err(e) => state.failed(e),
            }
            crate::task::attention::reread(state, Timestamp::now());
        }) as Continuation
    }));
}

/// Writes the session's auto-approve flag to its leaf.
pub(crate) fn set_auto_approve(
    services: &Services,
    spawner: &dyn Spawner,
    id: &SessionId,
    on: bool,
) {
    let (service, id) = (services.session.clone(), id.clone());
    record(
        spawner,
        async move { service.set_auto_approve(&id, on).await },
    );
}

/// Writes the row's selected worktree to its leaf.
pub(crate) fn persist_selection(
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
    record(spawner, async move {
        service.set_selected_worktree(&id, selected.as_ref()).await
    });
}

/// A write, then the board's own list read again once it has landed.
pub(crate) fn listed(
    spawner: &dyn Spawner,
    write: impl Future<Output = Result<(), Error>> + Send + 'static,
) {
    spawner.spawn(Box::pin(async move {
        let result = write.await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                if let Err(e) = result {
                    state.failed(e);
                }
                list(services, spawner);
            },
        ) as Continuation
    }));
}
