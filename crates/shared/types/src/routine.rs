//! Routines: triggers and skills, bound to a session or standing alone.

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
    /// A reviewer left a review of comments on the session's MR.
    ReviewCommented,
    ReviewAsked,
    AgentFinished,
    TasksRead,
    TaskMoved,
}

impl Trigger {
    pub const ALL: [Trigger; 8] = [
        Trigger::Daily,
        Trigger::CiFailed,
        Trigger::ChangesRequested,
        Trigger::ReviewCommented,
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
            Trigger::ReviewCommented => "review-commented",
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
                | Trigger::ReviewCommented
                | Trigger::ReviewAsked
                | Trigger::AgentFinished
        )
    }
}

/// Where a routine runs: in an agent, or as an action of Groove's own.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RoutineKind {
    /// In the agent of the session the event is about.
    Bound,
    /// In a routine session of its own.
    Standalone,
    /// No agent of its own: Groove does its `do` itself.
    Action,
}

/// What an action routine does, built into Groove.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    /// Up Next's tasks that must start today, each opened with `groove:start-task`.
    StartDue,
}

impl Action {
    pub const ALL: [Action; 1] = [Action::StartDue];

    pub fn name(self) -> &'static str {
        match self {
            Action::StartDue => "start-due",
        }
    }

    pub fn named(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|one| one.name() == word)
    }
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
    /// What its agent is asked to do; empty, its one skill is sent alone.
    pub words: String,
    /// What an action routine does.
    pub action: Option<Action>,
}
