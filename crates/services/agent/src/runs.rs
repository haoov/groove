//! The routines' runs: what runs, what waits its turn, what ran on each session, what was last seen.

mod rules;
mod seen;
mod turn;

use std::collections::{BTreeSet, VecDeque};

use groove_types::{SessionId, Timestamp, Trigger};

pub use rules::{Place, Sessions, target};
pub use seen::Seen;
pub use turn::Ending;

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

impl Run {
    /// Not yet sent to its agent.
    pub fn new(routine: &str, session: SessionId, trigger: Option<Trigger>, about: &str) -> Self {
        Self {
            routine: routine.to_string(),
            session,
            trigger,
            about: about.to_string(),
            sent_at: None,
            went: false,
        }
    }
}

/// One event a look found, about a session or about none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fired {
    pub trigger: Trigger,
    pub session: Option<SessionId>,
    pub about: String,
}

/// The `runs` part of the agent slice.
#[derive(Debug, Default)]
pub struct Runs {
    pub running: Vec<Run>,
    pub waiting: VecDeque<Run>,
    pub seen: Option<Seen>,
    /// Each routine queued on a session since the user last selected it.
    queued: BTreeSet<(String, SessionId)>,
    /// The daily trigger was weighed this run of the app.
    pub dawned: bool,
}

impl Runs {
    /// Whether this routine was queued on this session since the user last selected it.
    pub fn queued_since_seen(&self, routine: &str, session: &SessionId) -> bool {
        self.queued
            .contains(&(routine.to_string(), session.clone()))
    }

    /// Whether this routine runs or waits on this session already.
    pub fn holds(&self, routine: &str, session: &SessionId) -> bool {
        let one = |run: &Run| run.routine == routine && &run.session == session;
        self.running.iter().any(one) || self.waiting.iter().any(one)
    }

    /// Queued, and counted as queued on its session.
    pub fn queue(&mut self, run: Run) {
        self.queued
            .insert((run.routine.clone(), run.session.clone()));
        self.waiting.push_back(run);
    }

    /// The user selected the session: every routine may be queued on it again.
    pub fn seen_session(&mut self, session: &SessionId) {
        self.queued.retain(|(_, on)| on != session);
    }

    /// Whether a run is on this session's agent now.
    pub fn busy(&self, session: &SessionId) -> bool {
        self.running.iter().any(|run| &run.session == session)
    }

    /// Every run on this session dropped, as when the session goes.
    pub fn drop_session(&mut self, session: &SessionId) {
        self.running.retain(|run| &run.session != session);
        self.waiting.retain(|run| &run.session != session);
        self.seen_session(session);
    }
}
