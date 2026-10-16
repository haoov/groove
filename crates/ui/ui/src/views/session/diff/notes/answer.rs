//! A reply typed inside the card of the thread it answers.

use groove_controllers::AppState;
use groove_types::NoteOrigin;

use super::Block;
use crate::views::session::{Noting, Writing};

/// The thread a reply answers takes the reply's rows in place of its buttons; returns that note.
pub(super) fn answering(
    app: &AppState,
    noting: &Noting,
    (blocks, typing): (&mut [Block], usize),
) -> Option<usize> {
    let Writing::Reply(thread) = &noting.writing else {
        return None;
    };
    let shown = &app.delivery.shown;
    let block = blocks.iter_mut().find(|block| {
        block
            .at
            .and_then(|at| shown.get(at))
            .is_some_and(|note| note.origin == NoteOrigin::Thread(thread.clone()))
    })?;
    block.rows += typing;
    block.typing = typing;
    block.acts = false;
    block.at
}
