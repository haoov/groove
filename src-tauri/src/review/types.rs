use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct DiffResult {
    pub task_id: String,
    pub repos: Vec<RepoDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct RepoDiff {
    pub worktree_id: String,
    pub repo_id: String,
    pub branch: String,
    pub fetch_status: String,
    pub files: Vec<FileDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct FileDiff {
    pub path: String,
    #[ts(type = "number")]
    pub added: i64,
    #[ts(type = "number")]
    pub deleted: i64,
    /// Git status letter: "A" added, "M" modified, "D" deleted (summary only).
    pub status: String,
    /// `Some(true)` staged, `Some(false)` working-tree only, `None` no local change.
    #[serde(default)]
    pub staged: Option<bool>,
    /// Empty in the summary payload; filled by `get_file_diff`.
    pub hunks: Vec<Hunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct Hunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct DiffLine {
    #[ts(type = "number")]
    pub num: i64,
    pub content: String,
    #[serde(rename = "type")]
    pub line_type: String, // "add" | "del" | "ctx"
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct CommitEntry {
    pub sha: String,
    pub short_sha: String,
    pub message: String,
    pub author: String,
    #[ts(type = "number")]
    pub timestamp: i64,
    /// True for upstream base history, false for the task's own commits.
    #[serde(default)]
    pub is_base: bool,
}

/// One line's blame; `uncommitted` marks git's all-zero sha.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct BlameLine {
    pub line: u32,
    pub sha: String,
    pub short_sha: String,
    pub author: String,
    #[ts(type = "number")]
    pub time: i64,
    pub summary: String,
    pub uncommitted: bool,
}
