//! The MR of a worktree: read when it is selected, then polled while it stays open.

use groove_types::{
    CiState, Mr, MrDelivery, MrState, Result, TimelineKind, Timestamp, WorktreeDelivery, WorktreeId,
};
use groove_workspace_service::Delivered;

use crate::{AppState, Continuation, Services, Spawner};

/// What one read of a worktree's MR answers.
type Answer = Result<Option<Delivered>>;

/// How often an open MR is read again.
pub const INTERVAL: i64 = 60;

/// What this tick reads: the worktree not yet asked about, then the open MRs when due.
pub fn wanted(state: &AppState, now: Timestamp) -> Vec<WorktreeId> {
    if !state.focused {
        return Vec::new();
    }
    let mut out = first(state);
    if state.workspace.poll.due(now, INTERVAL) {
        out.extend(again(state));
    }
    out.sort();
    out.dedup();
    out
}

/// The selected worktree, while nothing has asked its forge yet.
fn first(state: &AppState) -> Vec<WorktreeId> {
    crate::workspace::selected(state)
        .filter(|id| state.workspace.poll.asks(id))
        .into_iter()
        .collect()
}

/// Every worktree of an open session whose row says its MR is open.
fn again(state: &AppState) -> Vec<WorktreeId> {
    state
        .session
        .open
        .iter()
        .flat_map(|open| open.delivery.iter())
        .filter(|(_, delivery)| delivery.is_open())
        .map(|(id, _)| id.clone())
        .filter(|id| !state.workspace.poll.is_out(id))
        .collect()
}

/// Whether the window should keep waking for the poll.
pub fn polls(state: &AppState) -> bool {
    let open = state
        .session
        .open
        .iter()
        .flat_map(|open| open.delivery.iter())
        .any(|(_, delivery)| delivery.is_open());
    state.focused && open
}

/// One pass: what this tick wants, read one call each.
pub fn poll(state: &mut AppState, services: &Services, spawner: &dyn Spawner, now: Timestamp) {
    let wanted = wanted(state, now);
    if wanted.is_empty() {
        return;
    }
    if state.workspace.poll.due(now, INTERVAL) {
        state.workspace.poll.ran(now);
    }
    for worktree in wanted {
        read(state, services, spawner, &worktree);
    }
}

/// The selected worktree's MR, read again whatever the clock says.
pub fn refresh(state: &mut AppState, services: &Services, spawner: &dyn Spawner) {
    let Some(id) = crate::workspace::selected(state) else {
        return;
    };
    if state.workspace.poll.is_out(&id) {
        return;
    }
    state.workspace.poll.forget(&id);
    read(state, services, spawner, &id);
}

/// The MR rows the database already holds, onto the worktrees they belong to.
pub fn known(services: &Services, spawner: &dyn Spawner) {
    let service = services.workspace.clone();
    spawner.spawn(Box::pin(async move {
        let read = service.open().await;
        Box::new(
            move |state: &mut AppState, _: &Services, _: &dyn Spawner| match read {
                Ok(mrs) => remembered(state, mrs),
                Err(e) => state.errors.push(e),
            },
        ) as Continuation
    }));
}

/// Marks each worktree as having an open MR, so the poll knows to read it.
fn remembered(state: &mut AppState, mrs: Vec<Mr>) {
    for mr in mrs {
        let worktree = mr.worktree.clone();
        onto(state, &worktree, |row| {
            row.mr = Some(MrDelivery {
                forge: mr.forge,
                number: mr.remote_id.clone(),
                state: mr.state,
                url: mr.url.clone(),
                approved: false,
                changes_requested: false,
            });
        });
    }
}

/// One worktree's MR, read from its forge and written down.
fn read(state: &mut AppState, services: &Services, spawner: &dyn Spawner, id: &WorktreeId) {
    if state.workspace.poll.is_out(id) {
        return;
    }
    let Some((repo, worktree)) = crate::workspace::pair(state, id) else {
        return;
    };
    let remote = match groove_workspace_service::Service::remote(&repo) {
        Ok(remote) => remote,
        Err(e) => {
            state.workspace.poll.sent(id);
            state.workspace.poll.answered(id);
            return state.errors.push(e);
        }
    };
    state.workspace.poll.sent(id);
    let service = services.workspace.clone();
    spawner.spawn(Box::pin(async move {
        let read = service.read(&remote, &repo, &worktree).await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                state.workspace.poll.answered(&worktree.id);
                answered(state, services, spawner, &worktree.id, read);
            },
        ) as Continuation
    }));
}

/// What the read brought back, onto the row and onto the selected worktree's slice.
fn answered(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    worktree: &WorktreeId,
    read: Answer,
) {
    let selected = state.workspace.holds(worktree);
    match read {
        Ok(Some(delivered)) => took(state, services, spawner, worktree, delivered),
        Ok(None) => {
            onto(state, worktree, |row| {
                *row = WorktreeDelivery {
                    status: row.status,
                    ..WorktreeDelivery::default()
                }
            });
            state.workspace.facts.remove(worktree);
            if selected {
                state.workspace.delivery.none();
            }
            crate::task::attention::reread(state, Timestamp::now());
        }
        Err(e) => {
            onto(state, worktree, |row| row.stale = true);
            if selected {
                state.workspace.delivery.aged();
            }
            state.errors.push(e);
        }
    }
}

/// One MR in hand, from a read or from a write: onto the rows, the facts, the slice.
pub(super) fn took(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    worktree: &WorktreeId,
    delivered: Delivered,
) {
    let before = stood(state, worktree);
    onto(state, worktree, |row| {
        row.mr = Some(delivered.shown());
        row.ci = delivered.ci();
        row.notes = delivered.notes();
        row.stale = false;
    });
    state
        .workspace
        .facts
        .insert(worktree.clone(), delivered.facts());
    if state.workspace.holds(worktree) {
        state.workspace.delivery.taken(delivered);
        state.workspace.remerge();
    }
    crate::task::attention::reread(state, Timestamp::now());
    moved(state, services, spawner, worktree, before);
}

/// What the row said before a read or a write landed on it.
fn stood(state: &AppState, worktree: &WorktreeId) -> (Option<MrState>, Option<CiState>) {
    let row = state
        .session
        .open
        .iter()
        .flat_map(|open| open.delivery.iter())
        .find(|(id, _)| id == worktree)
        .map(|(_, row)| row);
    (
        row.and_then(|row| row.mr.as_ref()).map(|mr| mr.state),
        row.and_then(|row| row.ci),
    )
}

/// The lines a read leaves: what the merge request became, and what its run said.
fn moved(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    worktree: &WorktreeId,
    before: (Option<MrState>, Option<CiState>),
) {
    let (was, ran) = before;
    let now = state
        .session
        .open
        .iter()
        .flat_map(|open| open.delivery.iter())
        .find(|(id, _)| id == worktree)
        .map(|(_, row)| row);
    let Some(row) = now else {
        return;
    };
    let (state_now, ci_now) = (row.mr.as_ref().map(|mr| mr.state), row.ci);
    let named = row
        .mr
        .as_ref()
        .map(|mr| format!("{}{}", mr.forge.sigil(), mr.number))
        .unwrap_or_default();
    if let Some(kind) = state_now.filter(|now| Some(*now) != was).and_then(became) {
        crate::timeline::log(state, services, spawner, kind, named.clone());
    }
    if let Some(ci) = ci_now.filter(|now| Some(*now) != ran && finished(*now)) {
        let said = format!("{named} {}", ci.label());
        crate::timeline::log(state, services, spawner, TimelineKind::Ci, said);
    }
}

/// Whether a run is over, so its result is worth a line.
fn finished(ci: CiState) -> bool {
    !matches!(ci, CiState::Pending | CiState::Running | CiState::Unknown)
}

/// The line an MR's own state is worth, once it is not the state it was.
fn became(state: MrState) -> Option<TimelineKind> {
    match state {
        MrState::Open => Some(TimelineKind::MrOpened),
        MrState::Merged => Some(TimelineKind::MrMerged),
        MrState::Closed => Some(TimelineKind::MrClosed),
    }
}

/// Writes on the row of whichever open session holds this worktree.
fn onto(state: &mut AppState, worktree: &WorktreeId, write: impl Fn(&mut WorktreeDelivery)) {
    for open in state.session.open.iter_mut() {
        if open.worktrees.iter().any(|one| &one.id == worktree) {
            write(open.row(worktree));
        }
    }
}
