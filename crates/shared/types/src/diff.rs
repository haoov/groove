use crate::{RepoId, Timestamp, WorktreeId};

/// What the change is read against.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffMode {
    /// Where the branch left the one it is based on.
    #[default]
    Base,
    /// What is not committed yet.
    Working,
}

impl DiffMode {
    pub const ALL: [DiffMode; 2] = [DiffMode::Base, DiffMode::Working];

    pub fn label(self) -> &'static str {
        match self {
            DiffMode::Base => "base",
            DiffMode::Working => "working",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffView {
    /// Both sides in one column.
    #[default]
    Inline,
    /// The old beside the new.
    Split,
}

impl DiffView {
    pub const ALL: [DiffView; 2] = [DiffView::Inline, DiffView::Split];

    pub fn label(self) -> &'static str {
        match self {
            DiffView::Inline => "inline",
            DiffView::Split => "split",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Untracked,
    /// In the worktree with nothing changed in it.
    Unchanged,
}

/// What a commit is known by: the first line of its message.
pub fn subject_of(message: &str) -> &str {
    message.lines().next().unwrap_or_default().trim()
}

/// Above this many bytes a file is listed and searched past, never read as text.
pub const TEXT_MAX_BYTES: u64 = 1 << 20;

impl FileStatus {
    /// What a change did to a path, from whether it stood before and stands after.
    pub fn of(before: bool, after: bool) -> Self {
        match (before, after) {
            (false, _) => FileStatus::Added,
            (true, false) => FileStatus::Deleted,
            (true, true) => FileStatus::Modified,
        }
    }
}

impl FileStatus {
    /// The one-letter mark the file list shows.
    pub fn letter(self) -> char {
        match self {
            FileStatus::Added => 'A',
            FileStatus::Modified => 'M',
            FileStatus::Deleted => 'D',
            FileStatus::Renamed => 'R',
            FileStatus::Untracked => '?',
            FileStatus::Unchanged => ' ',
        }
    }
}

/// What a row of a diff is: a line of the old file, of the new one, or of both.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RowKind {
    #[default]
    Context,
    Removed,
    Added,
    /// Lines neither side shows, and how many.
    Gap(u32),
}

/// What the change did to a line of the new file.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineMark {
    Added,
    Removed,
    Changed,
}

/// One run where the two sides differ: the old lines it took out, the new lines it put in.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Hunk {
    pub before: std::ops::Range<u32>,
    pub after: std::ops::Range<u32>,
}

/// One row of a diff: the line it is in each file, and what it is.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Row {
    pub old: Option<u32>,
    pub new: Option<u32>,
    pub kind: RowKind,
}

#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct FileDiff {
    pub path: String,
    pub added: u32,
    pub deleted: u32,
    pub status: FileStatus,
    /// `Some(true)` staged, `Some(false)` working tree only, `None` no local change.
    pub staged: Option<bool>,
}

#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct RepoDiff {
    pub worktree: WorktreeId,
    pub repo: RepoId,
    pub branch: String,
    pub files: Vec<FileDiff>,
}

#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct CommitEntry {
    pub sha: String,
    pub short_sha: String,
    pub message: String,
    pub author: String,
    pub at: Timestamp,
    /// Upstream history, not the task's own commit.
    pub is_base: bool,
}

/// One line's blame, `line` counted from 0; `uncommitted` marks git's all-zero sha.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct BlameLine {
    pub line: u32,
    pub sha: String,
    pub short_sha: String,
    pub author: String,
    pub at: Timestamp,
    pub summary: String,
    pub uncommitted: bool,
}
