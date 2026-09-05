use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
#[sqlx(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum SessionKind {
    Task,
    Explorer,
    Review,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct Session {
    pub id: String,
    pub kind: SessionKind,
    pub title: String,
    pub external_id: Option<String>,
    pub review_project: Option<String>,
    #[ts(type = "number | null")]
    pub review_iid: Option<i64>,
    #[ts(type = "number")]
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct ProviderTask {
    pub external_id: String,
    pub short_id: String,
    pub title: String,
    pub status: String,
    pub priority: Option<String>,
    #[ts(type = "number")]
    pub synced_at: i64,
    /// "notion" or "github".
    pub provider: String,
    pub url: Option<String>,
    /// The Projects v2 board that supplied the fields, when several could have.
    pub board: Option<String>,
    /// Appended to branch names; None means use short_id.
    pub branch_tag: Option<String>,
}

/// The task shape the frontend and MCP tools consume. Real tasks come from the
/// mirror; synthetic sessions synthesize status/priority.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct TaskView {
    pub short_id: String,
    /// Opaque handle for the provider's own API. Never parsed by the frontend.
    pub external_id: String,
    /// Which source the task came from. None for a session with no task behind it
    /// (explorer, review) — naming one would be a guess, and it used to guess
    /// "notion".
    pub provider: Option<String>,
    /// Deep link to the page or issue.
    pub external_url: Option<String>,
    pub title: String,
    pub status: String,
    pub priority: Option<String>,
    #[ts(type = "number")]
    pub last_synced_at: i64,
}

impl From<ProviderTask> for TaskView {
    fn from(t: ProviderTask) -> Self {
        Self {
            short_id: t.short_id,
            external_id: t.external_id,
            provider: Some(t.provider),
            external_url: t.url,
            title: t.title,
            status: t.status,
            priority: t.priority,
            last_synced_at: t.synced_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct Repo {
    pub id: String,
    pub host: String,
    pub group_path: String,
    pub project: String,
    pub local_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct Worktree {
    pub id: String,
    pub session_id: String,
    pub repo_id: String,
    pub branch: String,
    pub path: String,
    /// The branch this work merges into: diff/log base and MR target.
    pub base_ref: Option<String>,
    #[ts(type = "number")]
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct Mr {
    pub id: String,
    pub worktree_id: String,
    pub platform: String,
    pub remote_id: String,
    pub url: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct Annotation {
    pub id: String,
    pub session_id: String,
    pub repo_id: String,
    pub file_path: String,
    /// Range on the new (working-tree) side; single-line has start == end.
    #[ts(type = "number")]
    pub start_line: i64,
    #[ts(type = "number")]
    pub end_line: i64,
    pub content: String,
    pub author: String,
    pub status: String,
    #[ts(type = "number")]
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct PendingConfirmation {
    pub id: String,
    pub session_id: Option<String>,
    pub op_type: String,
    pub payload: String,
    pub origin: String,
    #[ts(type = "number")]
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct TimeSummary {
    pub session_id: String,
    #[ts(type = "number")]
    pub tracked_seconds: i64,
    #[ts(type = "number")]
    pub logged_seconds: i64,
    #[ts(type = "number")]
    pub today_seconds: i64,
    /// Measured but not yet logged at the source — what the log button offers.
    #[ts(type = "number")]
    pub unlogged_seconds: i64,
}
