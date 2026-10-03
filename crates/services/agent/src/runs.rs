//! The routines' runs: what runs, what waits its turn, what ran on each session, what was last seen.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use groove_types::{CiState, ExternalId, SessionId, Timestamp, Trigger, WorktreeId};

/// One run of a routine on one session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub routine: String,
    pub session: SessionId,
    /// `None` when its button started it.
    pub trigger: Option<Trigger>,
    /// What the event says, handed to the routine.
    pub about: String,
    /// When its words reached the agent.
    pub sent_at: Option<Timestamp>,
    /// Its agent has worked since.
    pub went: bool,
}

/// One event a look found, about a session or about none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fired {
    pub trigger: Trigger,
    pub session: Option<SessionId>,
    pub about: String,
}

/// What the last look saw, for the next one to tell what changed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Seen {
    pub selected: Option<SessionId>,
    pub ci: BTreeMap<WorktreeId, CiState>,
    pub changes: BTreeMap<WorktreeId, bool>,
    /// The review sessions the queue asks of the user, once it is read.
    pub asked: Option<BTreeSet<String>>,
    pub working: BTreeSet<SessionId>,
    /// Each task's short id and status.
    pub tasks: BTreeMap<ExternalId, (String, String)>,
    pub reading: bool,
    /// A read of the tasks has come back since the app started.
    pub tasks_read: bool,
}

/// The `runs` part of the agent slice.
#[derive(Debug, Default)]
pub struct Runs {
    pub running: Vec<Run>,
    pub waiting: VecDeque<Run>,
    pub seen: Option<Seen>,
    /// Each routine that ran on a session since the user last selected it.
    ran: BTreeSet<(String, SessionId)>,
    /// The daily trigger was weighed this run of the app.
    pub dawned: bool,
}

impl Runs {
    pub fn ran(&self, routine: &str, session: &SessionId) -> bool {
        self.ran.contains(&(routine.to_string(), session.clone()))
    }

    /// Whether this routine runs or waits on this session already.
    pub fn holds(&self, routine: &str, session: &SessionId) -> bool {
        let one = |run: &Run| run.routine == routine && &run.session == session;
        self.running.iter().any(one) || self.waiting.iter().any(one)
    }

    /// Queued, and counted as run on its session.
    pub fn queue(&mut self, run: Run) {
        self.ran.insert((run.routine.clone(), run.session.clone()));
        self.waiting.push_back(run);
    }

    /// Every routine's count on this session back to zero.
    pub fn looked(&mut self, session: &SessionId) {
        self.ran.retain(|(_, on)| on != session);
    }

    /// Whether a run is on this session's agent now.
    pub fn busy(&self, session: &SessionId) -> bool {
        self.running.iter().any(|run| &run.session == session)
    }

    /// Every run on this session dropped, as when the session goes.
    pub fn drop_session(&mut self, session: &SessionId) {
        self.running.retain(|run| &run.session != session);
        self.waiting.retain(|run| &run.session != session);
        self.looked(session);
    }
}
