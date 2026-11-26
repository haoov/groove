//! A reply typed inside the card of the thread it answers.

use groove_controllers::AppState;

use super::Block;
use crate::views::session::Noting;

/// The thread a reply answers takes the reply's rows in place of its buttons.
pub(super) fn answering(app: &AppState, noting: &Noting, (blocks, typing): (&mut [Block], usize)) {
    let shown = &app.delivery.shown;
    let block = blocks.iter_mut().find(|block| {
        shown
            .get(block.at)
            .is_some_and(|note| note.thread == noting.thread)
    });
    if let Some(block) = block {
        block.rows += typing;
        block.typing = typing;
        block.acts = false;
    }
}
