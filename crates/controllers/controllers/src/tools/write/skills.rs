//! One skill of the user's own, written by their agent.

use super::Write;
use crate::asker::Asker;
use crate::{AppState, Spawner};

pub(super) fn save(state: &mut AppState, spawner: &dyn Spawner, write: Write) {
    let Some(name) = write.text("name").map(str::to_string) else {
        return write.reply.failed("save_user_skill needs a name");
    };
    let Some(body) = write.text("body").map(str::to_string) else {
        return write.reply.failed("save_user_skill needs a body");
    };
    let instead_of = write.text("previous").map(str::to_string);
    let asker = Asker::Agent(write.reply);
    crate::agent::skills::save(state, spawner, name, body, instead_of, asker);
}
