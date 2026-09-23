//! What the agent asks the forge for: a merge request opened, written again, closed,
//! or commented on.

use groove_workspace_service::Text;

use super::Write;
use crate::asker::Asker;
use crate::tools::NO_WORKTREE;
use crate::{AppState, Services, Spawner};

pub(super) use crate::workspace::write::Act;

/// One merge request opened, written again or closed, on the worktree it names.
pub(super) fn mr(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    write: Write,
    act: Act,
) {
    let Some(worktree) = super::worktree_of(state, &write) else {
        return write.reply.failed(NO_WORKTREE);
    };
    let text = Text {
        title: write.text("title").unwrap_or_default().to_string(),
        body: write.text("description").unwrap_or_default().to_string(),
    };
    if act == Act::Open && text.title.is_empty() {
        return write.reply.failed("create_mr needs a title");
    }
    let asker = Asker::Agent(write.reply);
    crate::workspace::write::write(state, services, spawner, worktree, text, act, asker);
}

/// A comment on the merge request itself, under no line of it.
pub(super) fn comment(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    write: Write,
) {
    let Some(body) = write.text("body").map(str::to_string) else {
        return write.reply.failed("comment_mr needs a body");
    };
    let Some(whose) = super::notes::whose(state, &write) else {
        return write.reply.failed(NO_WORKTREE);
    };
    let asker = Asker::Agent(write.reply);
    let said = crate::workspace::forge::Say::Comment;
    crate::workspace::forge::say(state, services, spawner, said, body, whose, asker);
}
