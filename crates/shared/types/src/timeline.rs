use crate::{SessionId, Timestamp};

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineKind {
    TurnStart,
    TurnEnd,
    Tool,
    Write,
    Commit,
    Push,
    MrOpened,
    MrUpdated,
    MrMerged,
    MrClosed,
    Ci,
    Note,
}

impl TimelineKind {
    pub fn label(self) -> &'static str {
        match self {
            TimelineKind::TurnStart => "turn started",
            TimelineKind::TurnEnd => "turn ended",
            TimelineKind::Tool => "tool",
            TimelineKind::Write => "write",
            TimelineKind::Commit => "commit",
            TimelineKind::Push => "push",
            TimelineKind::MrOpened => "MR opened",
            TimelineKind::MrUpdated => "MR updated",
            TimelineKind::MrMerged => "MR merged",
            TimelineKind::MrClosed => "MR closed",
            TimelineKind::Ci => "CI",
            TimelineKind::Note => "note",
        }
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
