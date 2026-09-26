//! Reading a worktree's MR: the poll's tick, a read asked for now, and what it answered.

use groove_delivery_service::{Delivered, INTERVAL};
use groove_types::{Result, Timestamp, WorktreeId};

use super::Whose;
use crate::{AppState, Continuation, Services, Spawner};

/// One tick: every worktree the service wants read, one call each.
pub fn poll(state: &mut AppState, services: &Services, spawner: &dyn Spawner, now: Timestamp) {
    let selected = state.session.selected_worktree().map(|one| one.id.clone());
    let living = state.session.worktrees();
    let wanted = state
        .delivery
        .wanted(state.focused, selected.as_ref(), &living, now);
    if wanted.is_empty() {
        return;
    }
    if state.delivery.poll.due(now, INTERVAL) {
        state.delivery.poll.ran(now);
    }
    for worktree in wanted {
        read(state, services, spawner, &worktree);
    }
}

/// Whether the window should keep waking for the poll.
pub fn polls(state: &AppState) -> bool {
    state
        .delivery
        .polls(state.focused, &state.session.worktrees())
}

/// The selected worktree's MR, read again whatever the clock says.
pub(super) fn refresh_selected(state: &mut AppState, services: &Services, spawner: &dyn Spawner) {
    if let Some(id) = state.session.selected_worktree().map(|one| one.id.clone()) {
        refresh(state, services, spawner, &id);
    }
}

/// One worktree's MR, read again whatever the clock says.
pub(crate) fn refresh(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    worktree: &WorktreeId,
) {
    state.delivery.poll.forget(worktree);
    read(state, services, spawner, worktree);
}

/// The MR rows the database already holds.
pub fn known(services: &Services, spawner: &dyn Spawner) {
    let service = services.delivery.clone();
    spawner.spawn(Box::pin(async move {
        let read = service.open().await;
        Box::new(
            move |state: &mut AppState, _: &Services, _: &dyn Spawner| match read {
                Ok(mrs) => state.delivery.remembered(mrs),
                Err(e) => state.failed(e),
            },
        ) as Continuation
    }));
}

/// One worktree's MR, read from its forge and written down.
fn read(state: &mut AppState, services: &Services, spawner: &dyn Spawner, worktree: &WorktreeId) {
    if state.delivery.poll.is_out(worktree) {
        return;
    }
    let Some(whose) = Whose::of(state, worktree) else {
        return;
    };
    state.delivery.poll.sent(worktree);
    let service = services.delivery.clone();
    spawner.spawn(Box::pin(async move {
        let read = service.read(&whose.repo, &whose.worktree).await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                state.delivery.poll.answered(&whose.worktree.id);
                answered(state, services, spawner, &whose, read);
            },
        ) as Continuation
    }));
}

/// What a read brought back, onto the service's state.
fn answered(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    whose: &Whose,
    read: Result<Option<Delivered>>,
) {
    match read {
        Ok(Some(delivered)) => return took(state, services, spawner, whose, delivered),
        Ok(None) => state.delivery.gone(&whose.worktree.id),
        Err(e) => {
            state.delivery.aged(&whose.worktree.id);
            state.failed(e);
        }
    }
    moved(state);
}

/// One MR in hand, from a read or a write: onto the state, its lines onto its session's log.
pub(crate) fn took(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    whose: &Whose,
    delivered: Delivered,
) {
    for line in state.delivery.took(&whose.worktree.id, delivered) {
        let (kind, at) = (line.kind, &whose.worktree.id);
        crate::tools::logged(services, spawner, &whose.session, kind, &line.subject, at);
    }
    moved(state);
}

/// What reads the MRs: the surface's notes and the attention rules.
fn moved(state: &mut AppState) {
    super::notes::show(state);
    crate::task::attention::reread(state, Timestamp::now());
}
