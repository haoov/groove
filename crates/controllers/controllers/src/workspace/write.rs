//! The writes to a worktree's forge: an MR opened, written again, or closed.

use groove_types::{MrState, Repo, Result, Task, Worktree, WorktreeId};

use crate::asker::Asker;
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
    /// Why the forge's own state leaves no room for it.
    fn refusal(self) -> &'static str {
        match self {
            Act::Open => "this worktree already has an open merge request",
            Act::Edit | Act::Close => "this worktree has no open merge request",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Act::Open => "opening the merge request",
            Act::Edit => "writing the merge request",
            Act::Close => "closing the merge request",
        }
    }
}

/// The selected worktree's merge request, from the surface's own box.
pub(super) fn here(state: &mut AppState, services: &Services, spawner: &dyn Spawner, act: Act) {
    let Some(worktree) = crate::workspace::worktree_now(state) else {
        return;
    };
    let message = state.workspace.message.text();
    let text = text_of(&message, worked(state), &worktree.branch);
    if act == Act::Edit && text.title.is_empty() {
        return;
    }
    write(state, services, spawner, worktree, text, act, Asker::Ui);
}

/// One write against a worktree's forge, its answer written down.
pub(crate) fn write(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    worktree: Worktree,
    text: Text,
    act: Act,
    asker: Asker,
) {
    let id = worktree.id.clone();
    if state.workspace.poll.is_out(&id) {
        return asker.refused("a read of this merge request is still out");
    }
    if !allows(state, act) {
        return asker.refused(act.refusal());
    }
    let Some((repo, worktree)) = crate::workspace::pair(state, &id) else {
        return asker.refused(crate::tools::NO_WORKTREE);
    };
    let remote = match Service::remote(&repo) {
        Ok(remote) => remote,
        Err(e) => return asker.failed(state, e),
    };
    state.workspace.poll.sent(&id);
    let job = state.begin(act.label());
    let service = services.workspace.clone();
    spawner.spawn(Box::pin(async move {
        let wrote = made(&service, &remote, &repo, &worktree, &text, act).await;
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                state.end(job);
                state.workspace.poll.answered(&id);
                let said = wrote.as_ref().ok().map(|one| one.mr.url.clone());
                landed(state, services, spawner, &id, wrote, asker, said);
            },
        ) as Continuation
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
fn landed(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    worktree: &WorktreeId,
    wrote: Result<Delivered>,
    asker: Asker,
    url: Option<String>,
) {
    match wrote {
        Ok(delivered) => {
            super::mr::took(state, services, spawner, worktree, delivered);
            asker.done(|| url.unwrap_or_default());
        }
        Err(e) => asker.failed(state, e),
    }
}

/// The task the selected session works, for the footer.
fn worked(state: &AppState) -> Option<&Task> {
    let open = state.session.selected()?;
    state.task.worked(&open.session)
}
