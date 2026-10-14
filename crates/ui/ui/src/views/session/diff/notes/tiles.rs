//! The tile a line a note opens on carries: its open notes counted, or a resolved one.

use groove_controllers::AppState;
use groove_types::Note;

use super::{Inline, Slot};
use crate::components::Tile;

/// A line's tile, and the resolved note a click on it opens.
pub(crate) type Tiled = (Tile, Option<groove_types::NoteOrigin>);

impl Inline {
    /// What each row of `slots` carries in its marks' column, and whether a note stands on it.
    pub(crate) fn marks(&self, app: &AppState, slots: &[Slot]) -> Vec<(bool, Option<Tiled>)> {
        let noted = self.noted(slots);
        slots
            .iter()
            .zip(noted)
            .map(|(slot, noted)| match slot {
                Slot::Code(at) => (noted, self.tile(app, *at)),
                _ => (noted, None),
            })
            .collect()
    }

    fn tile(&self, app: &AppState, code: usize) -> Option<Tiled> {
        let notes: Vec<&Note> = self
            .blocks
            .iter()
            .filter(|block| block.from == code)
            .filter_map(|block| app.delivery.shown.get(block.at?))
            .collect();
        let open = notes.iter().filter(|note| !note.resolved).count();
        match (open, notes.first()) {
            (_, None) => None,
            (0, Some(first)) => Some((Tile::Resolved, Some(first.origin.clone()))),
            (open, Some(_)) => Some((Tile::Open(open), None)),
        }
    }
}
