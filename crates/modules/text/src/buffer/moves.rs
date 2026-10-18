//! Where the lines went since a revision, for what a reader keeps by line to follow them.

use super::{Buffer, Touched};

/// How many changes a buffer keeps the moves of.
const KEPT: usize = 512;

impl Buffer {
    pub(super) fn logged(&mut self, touched: Touched) {
        self.moves.push((self.revision, touched));
        if self.moves.len() > KEPT {
            self.forgot = self.moves[KEPT / 2 - 1].0;
            self.moves.drain(..KEPT / 2);
        }
    }

    /// What each change since `revision` replaced, in order; `None` once they are forgotten.
    pub fn moves_since(&self, revision: u64) -> Option<Vec<Touched>> {
        if revision < self.forgot || revision > self.revision {
            return None;
        }
        let since = self.moves.iter().filter(|(at, _)| *at > revision);
        Some(since.map(|(_, touched)| *touched).collect())
    }
}
