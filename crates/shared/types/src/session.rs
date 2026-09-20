use crate::{ExternalId, SessionId, Timestamp, WorktreeId};

/// What a session is about. The variant carries the identity that kind needs.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum SessionKind {
    Task { external_id: ExternalId },
    Explorer,
    Review { project: String, iid: u64 },
}

impl SessionKind {
    /// The task this session works, if it works one.
    pub fn task(&self) -> Option<&ExternalId> {
        match self {
            SessionKind::Task { external_id } => Some(external_id),
            _ => None,
        }
    }

    /// Whether this session works that task.
    pub fn works(&self, task: &ExternalId) -> bool {
        self.task() == Some(task)
    }

    pub fn name(&self) -> &'static str {
        match self {
            SessionKind::Task { .. } => "task",
            SessionKind::Explorer => "explorer",
            SessionKind::Review { .. } => "review",
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub title: String,
    pub kind: SessionKind,
    pub created_at: Timestamp,
}

/// The volatile part of a session, one leaf row per session.
#[derive(Clone, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SessionState {
    pub opened_at: Option<Timestamp>,
    pub seen_at: Option<Timestamp>,
    pub auto_approve: bool,
    pub selected_worktree: Option<WorktreeId>,
}
