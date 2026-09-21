//! The writes to a worktree's forge: an MR opened, written again, or closed.

use groove_types::{Repo, Result, Task, Worktree, WorktreeId};
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
    let Some((repo, worktree)) = selected(state) else {
        return;
    };
    if act != Act::Open && state.workspace.delivery.mr.is_none() {
        return;
    }
    let message = state.workspace.message.text();
    let text = text_of(&message, worked(state), &worktree.branch);
    if act == Act::Edit && text.title.is_empty() {
        return;
    }
    let remote = match Service::remote(&repo) {
        Ok(remote) => remote,
        Err(e) => return state.errors.push(e),
    };
    let id = worktree.id.clone();
    if state.workspace.poll.is_out(&id) {
        return;
    }
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

/// The selected worktree and its repo.
fn selected(state: &AppState) -> Option<(Repo, Worktree)> {
    let open = state.session.selected()?;
    let worktree = open.selected_worktree()?;
    let repo = open.repos.iter().find(|repo| repo.id == worktree.repo)?;
    Some((repo.clone(), worktree.clone()))
}
