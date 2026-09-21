//! The writes to a worktree's forge: an MR opened, written again, or closed.

use groove_types::{MrState, Repo, Result, Task, Worktree, WorktreeId};
use groove_workspace_service::{Delivered, Remote, Service, Text, text_of};

use crate::{AppState, Continuation, Services, Spawner};

/// Which write one job makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    Open,
    Edit,
    Close,
}

impl Act {
    fn label(self) -> &'static str {
        match self {
            Act::Open => "opening the merge request",
            Act::Edit => "writing the merge request",
            Act::Close => "closing the merge request",
        }
    }
}

/// One write against the selected worktree's forge, its answer written down.
pub(super) fn write(state: &mut AppState, services: &Services, spawner: &dyn Spawner, act: Act) {
    let Some(id) = crate::workspace::selected(state) else {
        return;
    };
    if state.workspace.poll.is_out(&id) || !allows(state, act) {
        return;
    }
    let Some((repo, worktree)) = crate::workspace::pair(state, &id) else {
        return;
    };
    let message = state.workspace.message.text();
    let text = text_of(&message, worked(state), &worktree.branch);
    if act == Act::Edit && text.title.is_empty() {
        return;
    }
    let remote = match Service::remote(&repo) {
        Ok(remote) => remote,
        Err(e) => return state.errors.push(e),
    };
    state.workspace.poll.sent(&id);
    let job = state.begin(act.label());
    let service = services.workspace.clone();
    spawner.spawn(Box::pin(async move {
        let wrote = made(&service, &remote, &repo, &worktree, &text, act).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            state.workspace.poll.answered(&id);
            landed(state, &id, wrote);
        }) as Continuation
    }));
}

/// Whether this write can be made: one open MR to write or close, none to offer one.
fn allows(state: &AppState, act: Act) -> bool {
    let open = state
        .workspace
        .delivery
        .mr
        .as_ref()
        .is_some_and(|mr| mr.state == MrState::Open);
    match act {
        Act::Open => !open,
        Act::Edit | Act::Close => open,
    }
}

async fn made(
    service: &Service,
    remote: &Remote,
    repo: &Repo,
    worktree: &Worktree,
    text: &Text,
    act: Act,
) -> Result<Delivered> {
    match act {
        Act::Open => service.open_mr(remote, repo, worktree, text).await,
        Act::Edit => service.edit_mr(remote, repo, worktree, text).await,
        Act::Close => service.close_mr(remote, repo, worktree).await,
    }
}

/// What the write answered, onto the rows and the slice, as a read would.
fn landed(state: &mut AppState, worktree: &WorktreeId, wrote: Result<Delivered>) {
    match wrote {
        Ok(delivered) => super::mr::took(state, worktree, delivered),
        Err(e) => state.errors.push(e),
    }
}

/// The task the selected session works, for the footer.
fn worked(state: &AppState) -> Option<&Task> {
    let open = state.session.selected()?;
    state.task.worked(&open.session)
}
