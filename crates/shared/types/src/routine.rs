//! Routines: triggers, skills and a scope, bound to a session or standing alone.

/// What starts a routine besides its button.
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum Trigger {
    /// The first time the app opens each day.
    Daily,
    CiFailed,
    ChangesRequested,
    ReviewAsked,
    AgentFinished,
    TasksRead,
    TaskMoved,
}

impl Trigger {
    pub const ALL: [Trigger; 7] = [
        Trigger::Daily,
        Trigger::CiFailed,
        Trigger::ChangesRequested,
        Trigger::ReviewAsked,
        Trigger::AgentFinished,
        Trigger::TasksRead,
        Trigger::TaskMoved,
    ];

    /// The word a routine file names it by.
    pub fn name(self) -> &'static str {
        match self {
            Trigger::Daily => "daily",
            Trigger::CiFailed => "ci-failed",
            Trigger::ChangesRequested => "changes-requested",
            Trigger::ReviewAsked => "review-asked",
            Trigger::AgentFinished => "agent-finished",
            Trigger::TasksRead => "tasks-read",
            Trigger::TaskMoved => "task-moved",
        }
    }

    pub fn named(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|one| one.name() == word)
    }

    /// An event about one session, which only a bound routine answers to.
    pub fn of_a_session(self) -> bool {
        matches!(
            self,
            Trigger::CiFailed
                | Trigger::ChangesRequested
                | Trigger::ReviewAsked
                | Trigger::AgentFinished
        )
    }
}

/// Where a routine's skill runs.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RoutineKind {
    /// In the agent of the session the event is about.
    Bound,
    /// In a routine session of its own.
    Standalone,
}

/// One routine as its file describes it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Routine {
    /// `shared:<name>` or `user:<name>`.
    pub id: String,
    pub name: String,
    pub description: String,
    /// The skills its agent may use, by id.
    pub skills: Vec<String>,
    pub kind: RoutineKind,
    /// The triggers it answers to, its button aside.
    pub on: Vec<Trigger>,
    /// What it may do without asking: writes for a bound routine, tools for a standalone one.
    pub scope: Vec<String>,
    /// What its agent is asked to do; empty, its one skill is sent alone.
    pub words: String,
}
