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
