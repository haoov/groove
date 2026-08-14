use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffResult {
    pub task_id: String,
    pub repos: Vec<RepoDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoDiff {
    pub repo_id: String,
    pub branch: String,
    pub fetch_status: String,
    pub files: Vec<FileDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: String,
    pub added: i64,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub num: i64,
    pub content: String,
    #[serde(rename = "type")]
    pub line_type: String, // "add" | "del" | "ctx"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitEntry {
    pub sha: String,
    pub short_sha: String,
    pub message: String,
    pub author: String,
    pub timestamp: i64,
    /// True = upstream base history (reachable from origin/HEAD); false = the
    /// task's own commits (base..branch). The UI dims base commits and draws
    /// the divergence divider before the first one.
    #[serde(default)]
    pub is_base: bool,
}

/// One line's blame. `uncommitted` marks git's all-zero sha: the line exists only
/// on disk, so there is no commit to open.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlameLine {
    pub line: u32,
    pub sha: String,
    pub short_sha: String,
    pub author: String,
    pub time: i64,
    pub summary: String,
    pub uncommitted: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BranchSpec {
    pub repo_id: String,
    pub branch_name: Option<String>, // None → derive from task short_id
}

/// A clone living under `<worktree_root>/MAIN` — the repo pool the pickers list.
#[derive(Debug, Clone, Serialize)]
pub struct MainRepo {
    pub url: String,
    pub local_path: String,
    /// Path relative to MAIN, e.g. "DevOps/mayo" — the picker's display name.
    pub slug: String,
}
