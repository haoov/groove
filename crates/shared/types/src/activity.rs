use crate::{ApprovalId, Timestamp};

/// The hook events the agent posts.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum HookKind {
    SessionStart,
    UserPromptSubmit,
    PreToolUse,
    PostToolUse,
    Notification,
    Stop,
}

impl HookKind {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "SessionStart" => HookKind::SessionStart,
            "UserPromptSubmit" => HookKind::UserPromptSubmit,
            "PreToolUse" => HookKind::PreToolUse,
            "PostToolUse" => HookKind::PostToolUse,
            "Notification" => HookKind::Notification,
            "Stop" => HookKind::Stop,
            _ => return None,
        })
    }
}

#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct ToolCall {
    pub name: String,
    /// One line of the tool's input, `cargo test --all`.
    pub detail: Option<String>,
}

#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum AgentStatus {
    Idle,
    Working,
    Done { seen: bool },
    Exited { code: i32 },
    Error { message: String },
}

/// A write waiting on the user, shown on the session's row.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Ask {
    pub id: ApprovalId,
    pub op: String,
    pub subject: String,
    /// The whole of what it writes, as the review sheet shows it.
    #[serde(default)]
    pub text: String,
    /// The worktree it writes in, when it writes in one.
    #[serde(default)]
    pub worktree: Option<crate::WorktreeId>,
}

/// The `agent` service's slice for one open session.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct SessionActivity {
    pub status: AgentStatus,
    pub tool: Option<ToolCall>,
    pub asks: Vec<Ask>,
    pub auto_approve: bool,
    pub changed_at: Timestamp,
    pub seen_at: Option<Timestamp>,
}

/// Colour and motion follow the class, nothing else.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AttentionClass {
    NeedsYou,
    ActWhenYouLook,
    Moving,
    Quiet,
}

impl SessionActivity {
    pub fn class(&self) -> AttentionClass {
        if !self.asks.is_empty() {
            return AttentionClass::NeedsYou;
        }
        match self.status {
            AgentStatus::Exited { .. } | AgentStatus::Error { .. } => AttentionClass::NeedsYou,
            AgentStatus::Done { seen: false } => AttentionClass::ActWhenYouLook,
            AgentStatus::Working => AttentionClass::Moving,
            AgentStatus::Idle | AgentStatus::Done { seen: true } => AttentionClass::Quiet,
        }
    }
}
