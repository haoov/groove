use std::path::PathBuf;

use crate::{RepoId, SessionId, Timestamp, WorktreeId};

#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Repo {
    pub id: RepoId,
    pub host: String,
    pub group_path: String,
    pub project: String,
    pub local_path: String,
}

impl Repo {
    /// `<host>/<group>/<project>`, the pool's name for it.
    pub fn slug(&self) -> String {
        format!("{}/{}/{}", self.host, self.group_path, self.project)
    }
}

#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Worktree {
    pub id: WorktreeId,
    pub session: SessionId,
    pub repo: RepoId,
    pub branch: String,
    pub path: String,
    /// The branch this work merges into: diff base and MR target.
    pub base_ref: Option<String>,
    pub created_at: Timestamp,
}

/// Git counts for one worktree.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct WorktreeStatus {
    pub modified: u32,
    pub staged: u32,
    pub ahead: u32,
    pub behind: u32,
}

impl WorktreeStatus {
    pub fn is_clean(&self) -> bool {
        self.modified == 0 && self.staged == 0
    }

    pub fn is_pushed(&self) -> bool {
        self.ahead == 0
    }
}

/// A clone in the pool, named by its path under `main/`.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct PoolEntry {
    /// `<host>/<group…>/<project>`
    pub slug: String,
    pub path: PathBuf,
}

impl PoolEntry {
    /// Whether this clone is the forge's own `<group…>/<project>`, whole segments only.
    pub fn holds(&self, project: &str) -> bool {
        self.slug == project || self.slug.ends_with(&format!("/{project}"))
    }
}

/// What a new worktree should be.
#[derive(Clone, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct WorktreeSpec {
    /// `None`: the session's default name.
    pub branch: Option<String>,
    /// The branch to cut from and merge into. `None`: the repo's default.
    pub target: Option<String>,
    /// A review: check out this branch of origin instead of cutting a new one.
    pub track_remote: Option<String>,
}
