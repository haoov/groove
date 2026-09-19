use crate::{RepoId, Timestamp, WorktreeId};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffMode {
    #[default]
    Base,
    Working,
    VsRemote,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffView {
    /// The file as it is now, marked where the change touched it.
    File,
    /// Both sides in one column.
    #[default]
    Inline,
    /// The old beside the new.
    Split,
}

impl DiffView {
    pub const ALL: [DiffView; 3] = [DiffView::File, DiffView::Inline, DiffView::Split];

    pub fn label(self) -> &'static str {
        match self {
            DiffView::File => "file",
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

/// One line's blame; `uncommitted` marks git's all-zero sha.
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

/// The `(removed, added)` row pairs that get a word diff: a run of removed rows
/// followed by a run of added rows of the same length, paired in order. Anything
/// else is none.
pub fn word_diff_pairs(rows: &[Row]) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    let mut at = 0;
    while at < rows.len() {
        let gone = run(rows, at, RowKind::Removed);
        let came = run(rows, at + gone, RowKind::Added);
        if gone > 0 && gone == came {
            pairs.extend((0..gone).map(|k| (at + k, at + gone + k)));
        }
        at += (gone + came).max(1);
    }
    pairs
}

fn run(rows: &[Row], from: usize, kind: RowKind) -> usize {
    rows[from.min(rows.len())..]
        .iter()
        .take_while(|row| row.kind == kind)
        .count()
}
