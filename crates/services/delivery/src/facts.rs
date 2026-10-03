//! What the routines' triggers read of the MRs: each run's state, the reviews given and asked.

use std::collections::{BTreeMap, BTreeSet};

use groove_types::{CiState, ReviewState, WorktreeId};

use crate::State;

/// When the poll ticks, how often it reads everything, and how old a read may grow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clock {
    pub now: groove_types::Timestamp,
    pub every: i64,
    pub stale_after: i64,
}

impl State {
    /// Whether none of these worktrees still carries an open merge request.
    pub fn all_landed(&self, worktrees: &[groove_types::Worktree]) -> bool {
        worktrees.iter().all(|one| !self.is_open(&one.id))
    }

    /// One tick of the poll: old reads aged, the worktrees to read now, the pass marked when it ran.
    pub fn tick(
        &mut self,
        focused: bool,
        (shown, living): (&[WorktreeId], &[WorktreeId]),
        clock: Clock,
    ) -> Vec<WorktreeId> {
        self.aged_out(clock.now, clock.stale_after);
        let wanted = self.wanted(focused, shown, living, (clock.now, clock.every));
        if !wanted.is_empty() && self.poll.due(clock.now, clock.every) {
            self.poll.ran(clock.now);
        }
        wanted
    }

    /// The state of each worktree's run, where its MR reported one.
    pub fn ci_of(&self, worktrees: &[WorktreeId]) -> BTreeMap<WorktreeId, CiState> {
        let ci = |one: &WorktreeId| Some((one.clone(), self.held(one)?.ci()?));
        worktrees.iter().filter_map(ci).collect()
    }

    /// For each worktree whose MR was read: whether changes are asked, and whether a review commented.
    pub fn reviewed(
        &self,
        worktrees: &[WorktreeId],
    ) -> (BTreeMap<WorktreeId, bool>, BTreeMap<WorktreeId, bool>) {
        let (mut changes, mut commented) = (BTreeMap::new(), BTreeMap::new());
        for one in worktrees {
            let Some(read) = self.held(one).and_then(|held| held.read.as_ref()) else {
                continue;
            };
            let reviewers = &read.details.reviewers;
            let said = reviewers.iter().any(|r| r.state == ReviewState::Commented);
            changes.insert(one.clone(), read.details.changes_requested());
            commented.insert(one.clone(), said);
        }
        (changes, commented)
    }

    /// The sessions of the reviews the queue asks of the user, once the queue is read.
    pub fn asked(&self) -> Option<BTreeSet<String>> {
        let waiting = |one: &&groove_types::ReviewMr| {
            matches!(one.review, None | Some(ReviewState::Requested))
        };
        let asked = self
            .reviews
            .iter()
            .filter(waiting)
            .map(|one| one.session_id());
        self.reviews_read.then(|| asked.collect())
    }
}
