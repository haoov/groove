//! The writes to a worktree's MR: opened, written again, or closed.

use groove_delivery_service::{MrAct, Text, text_of};
use groove_types::WorktreeId;

use super::Whose;
use crate::asker::Asker;
use crate::{AppState, Continuation, Services, Spawner};

fn label(act: MrAct) -> &'static str {
    match act {
        MrAct::Open => "opening the merge request",
        MrAct::Edit => "writing the merge request",
        MrAct::Close => "closing the merge request",
    }
}

/// The selected worktree's MR, from the commit box.
pub(super) fn here(state: &mut AppState, services: &Services, spawner: &dyn Spawner, act: MrAct) {
    let Some(worktree) = state.session.selected_worktree().cloned() else {
        return;
    };
    let task = state
        .session
        .selected()
        .and_then(|open| state.task.worked(&open.session));
    let text = text_of(&state.workspace.message.text(), task, &worktree.branch);
    if act == MrAct::Edit && text.title.is_empty() {
        return;
    }
    write(state, services, spawner, &worktree.id, text, act, Asker::Ui);
}

/// One write on a worktree's MR, its answer taken as a read would be.
pub(crate) fn write(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    worktree: &WorktreeId,
    text: Text,
    act: MrAct,
    asker: Asker,
) {
    let Some(whose) = Whose::of(state, worktree) else {
        return asker.refused(crate::tools::NO_WORKTREE);
    };
    if let Err(why) = state.delivery.allows(worktree, act) {
        return asker.refused(why);
    }
    state.delivery.poll.sent(worktree);
    let job = state.begin(label(act));
    let service = services.delivery.clone();
    spawner.spawn(Box::pin(async move {
        let (repo, at) = (&whose.repo, &whose.worktree);
        let wrote = match act {
            MrAct::Open => service.open_mr(repo, at, &text).await,
            MrAct::Edit => service.edit_mr(repo, at, &text).await,
            MrAct::Close => service.close_mr(repo, at).await,
        };
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| {
                state.end(job);
                state.delivery.poll.answered(&whose.worktree.id);
                match wrote {
                    Ok(mut delivered) => {
                        if let Some(e) = delivered.unassigned.take() {
                            state.failed(e);
                        }
                        let url = delivered.mr.url.clone();
                        super::poll::took(state, services, spawner, &whose, delivered);
                        asker.done(|| url);
                    }
                    Err(e) => asker.failed(state, e),
                }
            },
        ) as Continuation
    }));
}
