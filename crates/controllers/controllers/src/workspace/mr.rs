//! The MR of a worktree: read when it is selected, then polled while it stays open.

use groove_types::{
    Mr, MrDelivery, Repo, Result, Timestamp, Worktree, WorktreeDelivery, WorktreeId,
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
    out.dedup();
    out
}

/// The selected worktree, while nothing has asked its forge yet.
fn first(state: &AppState) -> Vec<WorktreeId> {
    let selected = state
        .session
        .selected()
        .and_then(|open| open.selected_worktree())
        .map(|worktree| worktree.id.clone());
    selected
        .filter(|id| state.workspace.poll.asks(id) && readable(state, id))
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
        .filter(|id| !state.workspace.poll.is_out(id) && readable(state, id))
        .collect()
}

/// Whether the forge of this worktree's repo is one Groove reads.
fn readable(state: &AppState, id: &WorktreeId) -> bool {
    host_of(state, id).is_some_and(groove_workspace_service::Service::reads)
}

/// The host the repo of this worktree lives on.
fn host_of<'a>(state: &'a AppState, id: &WorktreeId) -> Option<&'a str> {
    for open in state.session.open.iter() {
        let Some(worktree) = open.worktrees.iter().find(|one| &one.id == id) else {
            continue;
        };
        let repo = open.repos.iter().find(|repo| repo.id == worktree.repo)?;
        return Some(repo.host.as_str());
    }
    None
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
    let Some((repo, worktree)) = pair(state, id) else {
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
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.workspace.poll.answered(&worktree.id);
            answered(state, &worktree.id, read);
        }) as Continuation
    }));
}

/// What the read brought back, onto the row and onto the selected worktree's slice.
fn answered(state: &mut AppState, worktree: &WorktreeId, read: Answer) {
    let selected = state.workspace.holds(worktree);
    match read {
        Ok(Some(delivered)) => {
            onto(state, worktree, |row| {
                row.mr = Some(delivered.shown());
                row.ci = delivered.ci();
                row.notes = delivered.notes();
                row.stale = false;
            });
            if selected {
                state.workspace.delivery.taken(delivered);
            }
        }
        Ok(None) => {
            onto(state, worktree, |row| {
                *row = WorktreeDelivery {
                    status: row.status,
                    ..WorktreeDelivery::default()
                }
            });
            if selected {
                state.workspace.delivery.none();
            }
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

/// Writes on the row of whichever open session holds this worktree.
fn onto(state: &mut AppState, worktree: &WorktreeId, write: impl Fn(&mut WorktreeDelivery)) {
    for open in state.session.open.iter_mut() {
        if open.worktrees.iter().any(|one| &one.id == worktree) {
            write(open.row(worktree));
        }
    }
}

/// The repo and the worktree one id names, in whichever open session holds it.
fn pair(state: &AppState, id: &WorktreeId) -> Option<(Repo, Worktree)> {
    for open in state.session.open.iter() {
        let Some(worktree) = open.worktrees.iter().find(|one| &one.id == id) else {
            continue;
        };
        let repo = open.repos.iter().find(|repo| repo.id == worktree.repo)?;
        return Some((repo.clone(), worktree.clone()));
    }
    None
}
