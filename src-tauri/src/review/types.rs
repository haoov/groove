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
    /// Working-tree staged state for this path: `Some(true)` = staged (index has
    /// changes), `Some(false)` = only working-tree changes, `None` = no local
    /// change (e.g. committed-only files shown in vs-main mode → no checkbox).
    #[serde(default)]
    pub staged: Option<bool>,
    /// Empty in the summary payload — line content is fetched lazily per file
    /// via `get_file_diff` when the file is actually displayed.
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
    /// True = upstream base history (reachable from origin/HEAD); false = the
    /// task's own commits (base..branch). The UI dims base commits and draws
    /// the divergence divider before the first one.
    #[serde(default)]
    pub is_base: bool,
}

/// One line's blame. `uncommitted` marks git's all-zero sha: the line exists only
/// on disk, so there is no commit to open.
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

