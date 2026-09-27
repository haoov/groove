//! Every worktree's MR, the sessions' notes, the review queue, and the rules on them.

use std::collections::BTreeMap;

use groove_types::{
    Annotation, AnnotationId, AnnotationStatus, CiState, Mr, MrState, Note, RepoId, ReviewMr,
    SessionId, TimelineKind, Timestamp, WorktreeDelivery, WorktreeId, WorktreeStatus,
};

use crate::{Delivered, Held, INTERVAL, Polling, merged};

#[derive(Debug, Default)]
pub struct State {
    held: BTreeMap<WorktreeId, Held>,
    pub poll: Polling,
    notes: BTreeMap<SessionId, Vec<Annotation>>,
    /// The selected session's notes beside the selected worktree's threads.
    pub shown: Vec<Note>,
    /// What the forges ask this user to review.
    pub reviews: Vec<ReviewMr>,
}

/// What an MR write does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MrAct {
    Open,
    Edit,
    Close,
}

/// A line one read is worth on its session's log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub kind: TimelineKind,
    pub subject: String,
}

impl State {
    pub fn held(&self, worktree: &WorktreeId) -> Option<&Held> {
        self.held.get(worktree)
    }

    pub fn is_open(&self, worktree: &WorktreeId) -> bool {
        self.held(worktree).is_some_and(Held::is_open)
    }

    /// Whether the worktree has an MR, whatever its state.
    pub fn has_mr(&self, worktree: &WorktreeId) -> bool {
        self.held(worktree).is_some_and(|one| one.mr.is_some())
    }

    /// A worktree's row: git's status beside what the forge said.
    pub fn row(&self, worktree: &WorktreeId, status: WorktreeStatus) -> WorktreeDelivery {
        let held = self.held(worktree);
        WorktreeDelivery {
            status,
            mr: held.and_then(Held::shown),
            ci: held.and_then(Held::ci),
            notes: held.map_or(0, Held::open_threads),
            stale: held.is_some_and(|one| one.stale),
        }
    }

    /// What a read or a write answered, and the lines its changes are worth.
    pub fn took(&mut self, worktree: &WorktreeId, delivered: Delivered) -> Vec<Line> {
        let before = self.held(worktree).map(|one| (one.state(), one.ci()));
        let held = Held::from(delivered);
        let lines = lines(before.unwrap_or_default(), &held);
        self.held.insert(worktree.clone(), held);
        lines
    }

    /// The forge has no MR for the worktree.
    pub fn gone(&mut self, worktree: &WorktreeId) {
        self.held.remove(worktree);
    }

    /// A read that failed leaves what stands, and ages it.
    pub fn aged(&mut self, worktree: &WorktreeId) {
        if let Some(held) = self.held.get_mut(worktree) {
            held.stale = true;
        }
    }

    /// The rows the database holds, for worktrees the forge has not answered for yet.
    pub fn remembered(&mut self, mrs: Vec<Mr>) {
        for mr in mrs {
            let held = self.held.entry(mr.worktree.clone()).or_default();
            if held.mr.is_none() {
                held.mr = Some(mr);
            }
        }
    }

    /// What this tick reads: the selected worktree once, then `living`'s open MRs when due.
    pub fn wanted(
        &self,
        focused: bool,
        selected: Option<&WorktreeId>,
        living: &[WorktreeId],
        now: Timestamp,
    ) -> Vec<WorktreeId> {
        if !focused {
            return Vec::new();
        }
        let first = selected.filter(|id| self.poll.asks(id)).cloned();
        let mut out: Vec<WorktreeId> = first.into_iter().collect();
        if self.poll.due(now, INTERVAL) {
            let open = living.iter().filter(|id| self.is_open(id));
            out.extend(open.filter(|id| !self.poll.is_out(id)).cloned());
        }
        out.sort();
        out.dedup();
        out
    }

    /// Whether the window should keep waking for the poll.
    pub fn polls(&self, focused: bool, living: &[WorktreeId]) -> bool {
        focused && living.iter().any(|id| self.is_open(id))
    }

    /// Whether this write can be made on this worktree's MR now.
    pub fn allows(&self, worktree: &WorktreeId, act: MrAct) -> Result<(), &'static str> {
        if self.poll.is_out(worktree) {
            return Err("a read of this merge request is still out");
        }
        match (act, self.is_open(worktree)) {
            (MrAct::Open, true) => Err("this worktree already has an open merge request"),
            (MrAct::Edit | MrAct::Close, false) => Err("this worktree has no open merge request"),
            _ => Ok(()),
        }
    }
}

impl State {
    pub fn noted(&mut self, session: SessionId, notes: Vec<Annotation>) {
        self.notes.insert(session, notes);
    }

    /// A read of the session's notes is out: nothing held yet, and nothing to ask again.
    pub fn reading(&mut self, session: &SessionId) {
        self.notes.entry(session.clone()).or_default();
    }

    /// The session's notes, once they have been read.
    pub fn notes_of(&self, session: &SessionId) -> Option<&[Annotation]> {
        self.notes.get(session).map(Vec::as_slice)
    }

    pub fn note(&self, session: &SessionId, id: &AnnotationId) -> Option<&Annotation> {
        self.notes_of(session)?.iter().find(|one| &one.id == id)
    }

    /// Whether an open note of the session already stands on any of those lines.
    pub fn covered(&self, session: &SessionId, path: &str, lines: (u32, u32)) -> bool {
        let own = self.notes_of(session).unwrap_or_default();
        merged(own, &[])
            .iter()
            .any(|note| !note.resolved && note.over(path, lines))
    }

    /// The session's open notes on this repo, which the forge has not been told about.
    pub fn unposted(&self, session: &SessionId, repo: &RepoId) -> Vec<Annotation> {
        let own = self.notes_of(session).unwrap_or_default();
        own.iter()
            .filter(|note| &note.repo == repo && note.status == AnnotationStatus::Open)
            .cloned()
            .collect()
    }

    /// The notes the surface shows: this session's, beside this worktree's threads.
    pub fn show(&mut self, session: Option<&SessionId>, worktree: Option<&WorktreeId>) {
        let own = session
            .and_then(|one| self.notes_of(one))
            .unwrap_or_default();
        let threads = worktree
            .and_then(|one| self.held(one))
            .map(Held::threads)
            .unwrap_or_default();
        self.shown = merged(own, threads);
    }
}

/// What a change of state or of run is worth on the log.
fn lines(before: (Option<MrState>, Option<CiState>), held: &Held) -> Vec<Line> {
    let named = held.shown().map(|mr| mr.named()).unwrap_or_default();
    let mut out = Vec::new();
    if let Some(state) = held.state().filter(|now| Some(*now) != before.0) {
        out.push(Line {
            kind: became(state),
            subject: named.clone(),
        });
    }
    if let Some(ci) = held
        .ci()
        .filter(|now| Some(*now) != before.1 && finished(*now))
    {
        out.push(Line {
            kind: TimelineKind::Ci,
            subject: format!("{named} {}", ci.label()),
        });
    }
    out
}

/// Whether a run is over.
fn finished(ci: CiState) -> bool {
    !matches!(ci, CiState::Pending | CiState::Running | CiState::Unknown)
}

fn became(state: MrState) -> TimelineKind {
    match state {
        MrState::Open => TimelineKind::MrOpened,
        MrState::Merged => TimelineKind::MrMerged,
        MrState::Closed => TimelineKind::MrClosed,
    }
}
