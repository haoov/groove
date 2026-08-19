use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum SessionKind {
    Task,
    Explorer,
    Review,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum SessionState {
    Open,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Session {
    pub id: String,
    pub kind: SessionKind,
    pub state: SessionState,
    pub title: String,
    pub notion_page_id: Option<String>,
    pub review_project: Option<String>,
    pub review_iid: Option<i64>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NotionTask {
    pub page_id: String,
    pub short_id: String,
    pub title: String,
    pub status: String,
    pub priority: Option<String>,
    pub synced_at: i64,
}

/// The task shape the frontend and MCP tools consume. Real tasks come from the
/// mirror; synthetic sessions synthesize status/priority.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskView {
    pub short_id: String,
    pub notion_page_id: String,
    pub title: String,
    pub status: String,
    pub priority: Option<String>,
    pub last_synced_at: i64,
}

impl From<NotionTask> for TaskView {
    fn from(t: NotionTask) -> Self {
        Self {
            short_id: t.short_id,
            notion_page_id: t.page_id,
            title: t.title,
            status: t.status,
            priority: t.priority,
            last_synced_at: t.synced_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Repo {
    pub id: String,
    pub host: String,
    pub group_path: String,
    pub project: String,
    pub local_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Worktree {
    pub id: String,
    pub session_id: String,
    pub repo_id: String,
    pub branch: String,
    pub path: String,
    /// Review sessions: the MR's target branch — diff/log base becomes
    /// `origin/<base_ref>` instead of the repo default.
    pub base_ref: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Mr {
    pub id: String,
    pub worktree_id: String,
    pub platform: String,
    pub remote_id: String,
    pub url: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Annotation {
    pub id: String,
    pub session_id: String,
    pub repo_id: String,
    pub file_path: String,
    /// Range on the new (working-tree) side; single-line has start == end.
    pub start_line: i64,
    pub end_line: i64,
    pub content: String,
    pub author: String,
    pub status: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PendingConfirmation {
    pub id: String,
    pub session_id: Option<String>,
    pub op_type: String,
    pub payload: String,
    pub origin: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TimeSummary {
    pub session_id: String,
    pub tracked_seconds: i64,
    pub logged_seconds: i64,
    pub today_seconds: i64,
    /// Measured but not yet written to Notion — what the log button offers.
    pub unlogged_seconds: i64,
}
