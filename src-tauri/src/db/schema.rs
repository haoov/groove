use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Task {
    pub short_id: String,
    pub notion_page_id: String,
    pub title: String,
    pub status: String,
    pub priority: Option<String>,
    pub last_synced_at: i64,
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
    pub task_id: String,
    pub repo_id: String,
    pub branch: String,
    pub path: String,
    pub is_active: i64,
    pub created_at: i64,
    /// Review sessions: the MR's target branch — diff/log base becomes
    /// `origin/<base_ref>` instead of the repo default. NULL = default behavior.
    pub base_ref: Option<String>,
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
pub struct PendingConfirmation {
    pub id: String,
    pub task_id: Option<String>,
    pub op_type: String,
    pub payload: String,
    pub origin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Annotation {
    pub id: String,
    pub task_id: String,
    pub repo_id: String,
    pub file_path: String,
    /// Anchor line (== `start_line`); kept for back-compat with existing queries/rendering.
    pub line_num: i64,
    /// First line of the annotated range (new-side line number).
    pub start_line: i64,
    /// Last line of the annotated range; single-line annotations have start == end.
    pub end_line: i64,
    pub content: String,
    pub author: String,
    pub status: String,
    pub created_at: i64,
}
