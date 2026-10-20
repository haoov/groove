//! A tree parsed again in a job, and the blocks it opens, handed back to the document.

use std::sync::Arc;

use crate::{Block, Document, highlight, replay};

/// A tree parsed again for the text it belongs to, and the blocks of that text.
pub struct Settled {
    pub(crate) syntax: Option<highlight::Syntax>,
    pub(crate) seen: replay::Seen,
    pub blocks: Arc<Vec<Block>>,
}

impl Document {
    /// A job's tree, moved along the edits made since it read the text; false when it read another.
    pub fn install(&mut self, settled: Settled) -> bool {
        let Some(later) = self.edits.since(settled.seen) else {
            return false;
        };
        let mut syntax = settled.syntax;
        if let Some(tree) = syntax.as_mut() {
            later.iter().for_each(|edit| tree.shift(*edit));
        }
        self.syntax = syntax;
        self.edits.trim(settled.seen);
        true
    }

    /// The tree parsed again for the text it now holds, for a job to hand back.
    pub fn settled(mut self) -> Settled {
        let seen = self.edits.seen();
        self.reparse();
        Settled {
            blocks: Arc::new(self.blocks()),
            syntax: self.syntax,
            seen,
        }
    }
}
