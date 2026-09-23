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

/// One skill an agent can be sent, from the core plugin or the user's own.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Skill {
    /// `groove:start-task` — what the agent is sent, and the ui's key.
    pub id: String,
    pub plugin: String,
    pub name: String,
    /// What it does and when to use it, as the agent's own menu shows it.
    pub description: String,
    /// The one line the ui shows under the label; never the description.
    pub hint: String,
    pub label: String,
    /// The kinds of session that offer it, by name; empty offers it to every kind.
    pub kinds: Vec<String>,
    /// A skill of the user's own, which they may write again or delete.
    pub editable: bool,
    /// When its file was last written.
    pub changed_at: Timestamp,
}

impl Skill {
    /// Whether a session of this kind offers it.
    pub fn offered_to(&self, kind: &SessionKind) -> bool {
        self.kinds.is_empty() || self.kinds.iter().any(|one| one == kind.name())
    }
}
