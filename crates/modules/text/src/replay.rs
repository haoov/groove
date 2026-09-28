//! The edits a document took since a job read it, for that job's tree to take too.

use std::sync::atomic::{AtomicU64, Ordering};

use tree_sitter::InputEdit;

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Every edit since the oldest read still out, numbered from `base`.
#[derive(Clone)]
pub(crate) struct Replay {
    /// Which document the log belongs to; a copy keeps it.
    id: u64,
    base: u64,
    edits: Vec<InputEdit>,
}

/// Where a read stood in one document's log.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Seen {
    id: u64,
    at: u64,
}

impl Default for Replay {
    fn default() -> Self {
        Self {
            id: NEXT.fetch_add(1, Ordering::Relaxed),
            base: 0,
            edits: Vec::new(),
        }
    }
}

impl Replay {
    pub(crate) fn record(&mut self, edit: InputEdit) {
        self.edits.push(edit);
    }

    pub(crate) fn seen(&self) -> Seen {
        Seen {
            id: self.id,
            at: self.base + self.edits.len() as u64,
        }
    }

    /// The edits after `seen`, when it is a point of this log still held.
    pub(crate) fn since(&self, seen: Seen) -> Option<&[InputEdit]> {
        let from = seen.at.checked_sub(self.base)?;
        let from = usize::try_from(from).ok()?;
        (seen.id == self.id)
            .then(|| self.edits.get(from..))
            .flatten()
    }

    /// Forgets what comes before `seen`, which no read needs any more.
    pub(crate) fn trim(&mut self, seen: Seen) {
        let Some(upto) = seen.at.checked_sub(self.base) else {
            return;
        };
        let upto = usize::try_from(upto)
            .unwrap_or(usize::MAX)
            .min(self.edits.len());
        self.edits.drain(..upto);
        self.base += upto as u64;
    }
}
