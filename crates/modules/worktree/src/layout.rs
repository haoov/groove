//! Where the pool, the worktrees and a session's directory live under the root.

use std::path::{Path, PathBuf};

/// Where things live under the worktree root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    pub root: PathBuf,
}

impl Layout {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// `<root>/main`, the clones.
    pub fn main(&self) -> PathBuf {
        self.root.join("main")
    }

    /// `<root>/main/<host>/<group>/<project>`
    pub fn repo_dir(&self, host: &str, group: &str, project: &str) -> PathBuf {
        self.main().join(host).join(group).join(project)
    }

    /// `<root>/worktrees/<session>`
    pub fn session_dir(&self, session: &str) -> PathBuf {
        self.root.join("worktrees").join(session)
    }

    /// `<session dir>/<project>/<branch>`, the branch's slashes kept as directories.
    pub fn worktree_dir(&self, session: &str, project: &str, branch: &str) -> PathBuf {
        self.session_dir(session)
            .join(worktree_leaf(project, branch))
    }
}

/// `<project>/<branch>`. Never flatten the slashes: `fix/parser` and `fix-parser` collide.
pub fn worktree_leaf(project: &str, branch: &str) -> PathBuf {
    Path::new(project).join(branch)
}
