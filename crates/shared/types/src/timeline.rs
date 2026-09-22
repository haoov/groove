use crate::{SessionId, Timestamp};

/// What a line of the log is. Only what a session did to its own work belongs here:
/// git, the forge, its repos and its worktrees, and the turns around them. A tool the
/// agent ran on a file is its status, not its history.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineKind {
    TurnStart,
    TurnEnd,
    Commit,
    Push,
    Pull,
    Rebase,
    MrOpened,
    MrUpdated,
    MrMerged,
    MrClosed,
    Ci,
    Review,
    Note,
    RepoAdded,
    WorktreeAdded,
    WorktreeRemoved,
}

impl TimelineKind {
    /// Every kind, for a reader that walks them all.
    pub const ALL: [TimelineKind; 16] = [
        TimelineKind::TurnStart,
        TimelineKind::TurnEnd,
        TimelineKind::Commit,
        TimelineKind::Push,
        TimelineKind::Pull,
        TimelineKind::Rebase,
        TimelineKind::MrOpened,
        TimelineKind::MrUpdated,
        TimelineKind::MrMerged,
        TimelineKind::MrClosed,
        TimelineKind::Ci,
        TimelineKind::Review,
        TimelineKind::Note,
        TimelineKind::RepoAdded,
        TimelineKind::WorktreeAdded,
        TimelineKind::WorktreeRemoved,
    ];

    pub fn label(self) -> &'static str {
        match self {
            TimelineKind::TurnStart => "turn started",
            TimelineKind::TurnEnd => "turn ended",
            TimelineKind::Commit => "commit",
            TimelineKind::Push => "push",
            TimelineKind::Pull => "pull",
            TimelineKind::Rebase => "rebase",
            TimelineKind::MrOpened => "MR opened",
            TimelineKind::MrUpdated => "MR updated",
            TimelineKind::MrMerged => "MR merged",
            TimelineKind::MrClosed => "MR closed",
            TimelineKind::Ci => "CI",
            TimelineKind::Review => "review",
            TimelineKind::Note => "note",
            TimelineKind::RepoAdded => "repo added",
            TimelineKind::WorktreeAdded => "worktree added",
            TimelineKind::WorktreeRemoved => "worktree removed",
        }
    }

    /// The word the database holds it as.
    pub fn word(self) -> &'static str {
        match self {
            TimelineKind::TurnStart => "turn_start",
            TimelineKind::TurnEnd => "turn_end",
            TimelineKind::Commit => "commit",
            TimelineKind::Push => "push",
            TimelineKind::Pull => "pull",
            TimelineKind::Rebase => "rebase",
            TimelineKind::MrOpened => "mr_opened",
            TimelineKind::MrUpdated => "mr_updated",
            TimelineKind::MrMerged => "mr_merged",
            TimelineKind::MrClosed => "mr_closed",
            TimelineKind::Ci => "ci",
            TimelineKind::Review => "review",
            TimelineKind::Note => "note",
            TimelineKind::RepoAdded => "repo_added",
            TimelineKind::WorktreeAdded => "worktree_added",
            TimelineKind::WorktreeRemoved => "worktree_removed",
        }
    }

    /// The kind a stored word names, or nothing for one this version never wrote.
    pub fn parse(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.word() == word)
    }
}

/// One line of a session's log.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct TimelineEvent {
    pub session: SessionId,
    pub at: Timestamp,
    pub kind: TimelineKind,
    pub subject: String,
    pub payload: serde_json::Value,
}
