//! What the agent asks of the MR: opened, written again, closed, or commented on.

use groove_delivery_service::{MrAct, Text};

use super::Write;
use crate::asker::Asker;
use crate::delivery::{Say, mr, threads};
use crate::tools::NO_WORKTREE;
use crate::{AppState, Services, Spawner};

/// One MR opened, written again or closed, on the worktree the write names.
pub(super) fn mr(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    write: Write,
    act: MrAct,
) {
    let Some(worktree) = super::worktree_of(state, &write) else {
        return write.reply.failed(NO_WORKTREE);
    };
    let text = Text {
        title: write.text("title").unwrap_or_default().to_string(),
        body: write.text("description").unwrap_or_default().to_string(),
    };
    if act == MrAct::Open && text.title.is_empty() {
        return write.reply.failed("create_mr needs a title");
    }
    let asker = Asker::Agent(write.reply);
    mr::write(state, services, spawner, &worktree.id, text, act, asker);
}

/// A comment on the MR itself, under no line of it.
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
    threads::say_as(state, services, spawner, Say::Comment, body, whose, asker);
}
