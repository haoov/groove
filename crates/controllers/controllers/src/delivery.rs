//! The `delivery` controller: one function per user action on the `delivery` service.

pub(crate) mod mr;
pub(crate) mod notes;
pub(crate) mod poll;
mod queue;
pub(crate) mod threads;

use groove_delivery_service::MrAct;
use groove_types::{Repo, SessionId, Worktree, WorktreeId};

use crate::{AppState, Services, Spawner};

pub use notes::NoteAct;
pub use poll::{known, poll, polls};
pub use threads::{Say, ThreadAct};

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// `delivery.refresh_mr`: the selected worktree's MR, read again now.
    RefreshMr,
    /// `delivery.browse_mr`: an MR's page, in the browser.
    BrowseMr { url: String },
    /// `delivery.review_queue`: the MRs the forges ask this user to review.
    ReviewQueue,
    /// `delivery.create_mr`: the worktree's branch offered to its base.
    CreateMr,
    /// `delivery.update_mr`: its title and body written again from the box.
    UpdateMr,
    /// `delivery.close_mr`: closed, with nothing merged.
    CloseMr,
    /// `delivery.get_notes`: the selected session's notes.
    GetNotes,
    /// One note of the selected session made, written again, resolved or taken away.
    Note(NoteAct),
    /// One note posted, or one of the MR's threads answered or resolved.
    Thread(ThreadAct),
    /// What the commit box says on the merge request: a comment, or a verdict.
    Say(Say),
}

impl Command {
    pub fn id(&self) -> &'static str {
        match self {
            Command::RefreshMr => "delivery.refresh_mr",
            Command::BrowseMr { .. } => "delivery.browse_mr",
            Command::ReviewQueue => "delivery.review_queue",
            Command::CreateMr => "delivery.create_mr",
            Command::UpdateMr => "delivery.update_mr",
            Command::CloseMr => "delivery.close_mr",
            Command::GetNotes => "delivery.get_notes",
            Command::Note(act) => act.id(),
            Command::Thread(act) => act.id(),
            Command::Say(say) => say.id(),
        }
    }

    /// Whether it writes on what the surface shows, which a commit never allows.
    fn writes(&self) -> bool {
        matches!(self, Command::Note(_) | Command::Thread(_))
    }
}

pub fn dispatch(
    command: Command,
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
) {
    if state.workspace.readonly() && command.writes() {
        return;
    }
    match command {
        Command::RefreshMr => poll::refresh_selected(state, services, spawner),
        Command::BrowseMr { url } => crate::spawn::record(spawner, async move {
            groove_delivery_service::browse(&url).await
        }),
        Command::ReviewQueue => queue::read(state, services, spawner),
        Command::CreateMr => mr::here(state, services, spawner, MrAct::Open),
        Command::UpdateMr => mr::here(state, services, spawner, MrAct::Edit),
        Command::CloseMr => mr::here(state, services, spawner, MrAct::Close),
        Command::GetNotes => notes::list_selected(state, services, spawner),
        Command::Note(act) => notes::here(state, services, spawner, act),
        Command::Thread(act) => threads::here(state, services, spawner, act),
        Command::Say(say) => threads::say_here(state, services, spawner, say),
    }
}

/// The session a write belongs to, and the worktree and repo it lands on.
#[derive(Debug, Clone)]
pub(crate) struct Whose {
    pub session: SessionId,
    pub repo: Repo,
    pub worktree: Worktree,
}

impl Whose {
    /// The worktree by id, in whichever session holds it.
    pub(crate) fn of(state: &AppState, worktree: &WorktreeId) -> Option<Self> {
        let (open, repo, worktree) = state.session.find(worktree)?;
        Some(Self {
            session: open.session.id.clone(),
            repo: repo.clone(),
            worktree: worktree.clone(),
        })
    }

    /// The selected session's selected worktree.
    pub(crate) fn selected(state: &AppState) -> Option<Self> {
        let id = state.session.selected_worktree()?.id.clone();
        Self::of(state, &id)
    }
}
